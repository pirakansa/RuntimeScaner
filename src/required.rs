use std::collections::BTreeSet;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::{AuditError, Result};
use crate::models::{RequiredOptions, RequiredReport};
use crate::util::{current_arch, sorted_unique};

pub fn run_required(options: &RequiredOptions) -> Result<RequiredReport> {
    if !options.executable.exists() {
        return Err(AuditError::message(format!(
            "executable not found: {}",
            options.executable.display()
        )));
    }

    let static_needed = read_static_needed(&options.executable)?;
    let loader_output = run_with_loader_debug(options)?;
    let mut diagnostics = parse_diagnostics(&loader_output.stderr);
    append_exit_diagnostics(&loader_output, &mut diagnostics);
    let (runtime_requested, loaded_paths) = parse_ld_debug(&loader_output.stderr);

    Ok(RequiredReport {
        arch: current_arch(),
        executable: options.executable.display().to_string(),
        command: command_line(options),
        environment: options.envs.clone(),
        static_needed,
        runtime_requested,
        loaded_paths,
        diagnostics: sorted_unique(diagnostics),
    })
}

pub fn read_static_needed(executable: &Path) -> Result<Vec<String>> {
    let output = Command::new("readelf")
        .arg("-d")
        .arg(executable)
        .output()
        .map_err(|error| AuditError::io("readelf unavailable", error))?;

    if !output.status.success() {
        return Err(AuditError::CommandFailed {
            program: "readelf".to_string(),
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let needed = stdout
        .lines()
        .filter(|line| line.contains("(NEEDED)"))
        .filter_map(extract_bracketed)
        .collect::<BTreeSet<_>>();

    if needed.is_empty() && stdout.contains("There is no dynamic section") {
        return Err(AuditError::message(
            "executable is not dynamically linked or has no dynamic section",
        ));
    }

    Ok(needed.into_iter().collect())
}

pub fn parse_ld_debug(stderr: &str) -> (Vec<String>, Vec<String>) {
    let mut requested = BTreeSet::new();
    let mut loaded = BTreeSet::new();

    for line in stderr.lines() {
        if let Some(soname) = requested_library(line) {
            requested.insert(soname.to_string());
        }

        if let Some(path) = initialized_library_path(line) {
            loaded.insert(path.to_string());
        }
    }

    (
        requested.into_iter().collect(),
        loaded.into_iter().collect(),
    )
}

fn command_line(options: &RequiredOptions) -> Vec<String> {
    let mut command = vec![options.executable.display().to_string()];
    command.extend(options.args.clone());
    command
}

fn append_exit_diagnostics(loader_output: &LoaderOutput, diagnostics: &mut Vec<String>) {
    if loader_output.timed_out {
        diagnostics.push("target command timed out".to_string());
    } else if !loader_output.success {
        diagnostics.push(format!(
            "target command exited unsuccessfully with status {}",
            loader_output
                .status
                .map(|code| code.to_string())
                .unwrap_or_else(|| "terminated by signal".to_string())
        ));
    }
}

fn run_with_loader_debug(options: &RequiredOptions) -> Result<LoaderOutput> {
    let mut command = Command::new(&options.executable);
    command
        .args(&options.args)
        .env("LD_DEBUG", "libs,files")
        .envs(&options.envs)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = &options.cwd {
        command.current_dir(cwd);
    }

    let mut child = command.spawn().map_err(|error| {
        AuditError::io(
            format!("cannot run executable {}", options.executable.display()),
            error,
        )
    })?;

    let deadline = options.timeout.map(|timeout| Instant::now() + timeout);
    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| AuditError::io("cannot wait for target command", error))?
        {
            break status;
        }

        if let Some(deadline) = deadline {
            if Instant::now() >= deadline {
                timed_out = true;
                child
                    .kill()
                    .map_err(|error| AuditError::io("cannot kill timed out target", error))?;
                break child.wait().map_err(|error| {
                    AuditError::io("cannot wait for killed target command", error)
                })?;
            }
        }

        thread::sleep(Duration::from_millis(20));
    };

    let output = child
        .wait_with_output()
        .map_err(|error| AuditError::io("cannot collect target command output", error))?;

    Ok(LoaderOutput {
        success: status.success(),
        status: status.code(),
        timed_out,
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

#[derive(Debug)]
struct LoaderOutput {
    success: bool,
    status: Option<i32>,
    timed_out: bool,
    stderr: String,
}

fn parse_diagnostics(stderr: &str) -> Vec<String> {
    stderr
        .lines()
        .filter(|line| {
            line.contains("not found")
                || line.contains("cannot open")
                || line.contains("could not be loaded")
                || line.contains("panicked at")
        })
        .map(|line| line.trim().to_string())
        .collect()
}

fn requested_library(line: &str) -> Option<&str> {
    let start = line.find("find library=")?;
    let rest = &line[start + "find library=".len()..];
    let soname = rest
        .split([' ', ';', '\t'])
        .next()
        .unwrap_or_default()
        .trim();
    (!soname.is_empty()).then_some(soname)
}

fn initialized_library_path(line: &str) -> Option<&str> {
    let start = line.find("calling init:")?;
    let path = line[start + "calling init:".len()..].trim();
    path.starts_with('/').then_some(path)
}

fn extract_bracketed(line: &str) -> Option<String> {
    let start = line.find('[')? + 1;
    let end = line[start..].find(']')? + start;
    Some(line[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ld_debug_requested_and_loaded_libraries() {
        let input = "123: find library=libXcursor.so.1 [0]; searching\n\
                     123: calling init: /lib/aarch64-linux-gnu/libX11.so.6\n";

        let (requested, loaded) = parse_ld_debug(input);

        assert_eq!(requested, vec!["libXcursor.so.1"]);
        assert_eq!(loaded, vec!["/lib/aarch64-linux-gnu/libX11.so.6"]);
    }
}
