use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use runtimescaner::{
    collect_libraries, collect_to_json, diff_reports, diff_to_json, inventory_to_json,
    parse_diff_json, parse_ignore_toml, parse_inventory_json, parse_required_json,
    required_to_json, run_inventory, run_required, AuditError, RequiredOptions, Result,
};

pub fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_help();
        return Ok(());
    };

    match command.as_str() {
        "required" => run_required_command(args.collect()),
        "inventory" => run_inventory_command(args.collect()),
        "diff" => run_diff_command(args.collect()),
        "collect" => run_collect_command(args.collect()),
        "-h" | "--help" | "help" => {
            print_help();
            Ok(())
        }
        unknown => Err(AuditError::message(format!("unknown command: {unknown}"))),
    }
}

fn run_required_command(args: Vec<String>) -> Result<()> {
    let options = RequiredCommand::parse(args)?;
    let report = run_required(&RequiredOptions {
        executable: options.executable,
        args: options.command_args,
        envs: options.envs,
        timeout: options.timeout,
        cwd: options.cwd,
    })?;
    write_output(options.out, required_to_json(&report))
}

fn run_inventory_command(args: Vec<String>) -> Result<()> {
    let options = InventoryCommand::parse(args)?;
    let report = run_inventory(&options.extra_dirs)?;
    write_output(options.out, inventory_to_json(&report))
}

fn run_diff_command(args: Vec<String>) -> Result<()> {
    let options = DiffCommand::parse(args)?;
    let required = parse_required_json(&read_file(&options.required_path)?)?;
    let inventory = parse_inventory_json(&read_file(&options.inventory_path)?)?;
    let ignore_rules = match options.ignore_path {
        Some(path) => parse_ignore_toml(&read_file(&path)?),
        None => Vec::new(),
    };

    let report = diff_reports(
        &required,
        &inventory,
        &ignore_rules,
        options.allow_arch_mismatch,
    )?;
    write_output(options.out, diff_to_json(&report))
}

fn run_collect_command(args: Vec<String>) -> Result<()> {
    let options = CollectCommand::parse(args)?;
    let missing_report = parse_diff_json(&read_file(&options.missing_path)?)?;
    let report = collect_libraries(
        &missing_report.bundle_candidates,
        &options.search_dirs,
        &options.libdir,
    )?;
    write_output(options.out, collect_to_json(&report))
}

struct RequiredCommand {
    executable: PathBuf,
    command_args: Vec<String>,
    envs: BTreeMap<String, String>,
    timeout: Option<Duration>,
    cwd: Option<PathBuf>,
    out: Option<PathBuf>,
}

impl RequiredCommand {
    fn parse(args: Vec<String>) -> Result<Self> {
        let mut parser = ArgParser::new(args);
        let mut command = Self {
            executable: PathBuf::new(),
            command_args: Vec::new(),
            envs: BTreeMap::new(),
            timeout: None,
            cwd: None,
            out: None,
        };
        let mut has_executable = false;

        while let Some(arg) = parser.next() {
            match arg.as_str() {
                "--exec" => {
                    command.executable = PathBuf::from(parser.value("--exec")?);
                    has_executable = true;
                }
                "--arg" => command.command_args.push(parser.value("--arg")?),
                "--env" => {
                    let (name, value) = parse_env_assignment(&parser.value("--env")?)?;
                    command.envs.insert(name, value);
                }
                "--timeout" => command.timeout = Some(parse_duration(&parser.value("--timeout")?)?),
                "--cwd" => command.cwd = Some(PathBuf::from(parser.value("--cwd")?)),
                "--out" => command.out = Some(PathBuf::from(parser.value("--out")?)),
                other => {
                    return Err(AuditError::message(format!(
                        "unknown required option: {other}"
                    )))
                }
            }
        }

        if !has_executable {
            return Err(AuditError::message("required needs --exec"));
        }
        Ok(command)
    }
}

struct InventoryCommand {
    extra_dirs: Vec<PathBuf>,
    out: Option<PathBuf>,
}

impl InventoryCommand {
    fn parse(args: Vec<String>) -> Result<Self> {
        let mut parser = ArgParser::new(args);
        let mut command = Self {
            extra_dirs: Vec::new(),
            out: None,
        };

        while let Some(arg) = parser.next() {
            match arg.as_str() {
                "--lib-dir" | "--libdir" => {
                    command.extra_dirs.push(PathBuf::from(parser.value(&arg)?));
                }
                "--out" => command.out = Some(PathBuf::from(parser.value("--out")?)),
                other => {
                    return Err(AuditError::message(format!(
                        "unknown inventory option: {other}"
                    )))
                }
            }
        }

        Ok(command)
    }
}

struct DiffCommand {
    required_path: PathBuf,
    inventory_path: PathBuf,
    ignore_path: Option<PathBuf>,
    allow_arch_mismatch: bool,
    out: Option<PathBuf>,
}

impl DiffCommand {
    fn parse(args: Vec<String>) -> Result<Self> {
        let mut parser = ArgParser::new(args);
        let mut required_path = None;
        let mut inventory_path = None;
        let mut ignore_path = None;
        let mut allow_arch_mismatch = false;
        let mut out = None;

        while let Some(arg) = parser.next() {
            match arg.as_str() {
                "--required" => required_path = Some(PathBuf::from(parser.value("--required")?)),
                "--inventory" => inventory_path = Some(PathBuf::from(parser.value("--inventory")?)),
                "--ignore" => ignore_path = Some(PathBuf::from(parser.value("--ignore")?)),
                "--allow-arch-mismatch" => allow_arch_mismatch = true,
                "--out" => out = Some(PathBuf::from(parser.value("--out")?)),
                other => return Err(AuditError::message(format!("unknown diff option: {other}"))),
            }
        }

        Ok(Self {
            required_path: required_path
                .ok_or_else(|| AuditError::message("diff needs --required"))?,
            inventory_path: inventory_path
                .ok_or_else(|| AuditError::message("diff needs --inventory"))?,
            ignore_path,
            allow_arch_mismatch,
            out,
        })
    }
}

struct CollectCommand {
    missing_path: PathBuf,
    search_dirs: Vec<PathBuf>,
    libdir: PathBuf,
    out: Option<PathBuf>,
}

impl CollectCommand {
    fn parse(args: Vec<String>) -> Result<Self> {
        let mut parser = ArgParser::new(args);
        let mut missing_path = None;
        let mut search_dirs = Vec::new();
        let mut libdir = None;
        let mut out = None;

        while let Some(arg) = parser.next() {
            match arg.as_str() {
                "--missing" => missing_path = Some(PathBuf::from(parser.value("--missing")?)),
                "--search-dir" => search_dirs.push(PathBuf::from(parser.value("--search-dir")?)),
                "--lib-dir" | "--libdir" => libdir = Some(PathBuf::from(parser.value(&arg)?)),
                "--out" => out = Some(PathBuf::from(parser.value("--out")?)),
                other => {
                    return Err(AuditError::message(format!(
                        "unknown collect option: {other}"
                    )))
                }
            }
        }

        Ok(Self {
            missing_path: missing_path
                .ok_or_else(|| AuditError::message("collect needs --missing"))?,
            libdir: libdir.ok_or_else(|| AuditError::message("collect needs --libdir"))?,
            search_dirs,
            out,
        })
    }
}

fn parse_env_assignment(assignment: &str) -> Result<(String, String)> {
    let (name, value) = assignment
        .split_once('=')
        .ok_or_else(|| AuditError::message("--env must use NAME=VALUE"))?;
    Ok((name.to_string(), value.to_string()))
}

fn read_file(path: &PathBuf) -> Result<String> {
    fs::read_to_string(path)
        .map_err(|error| AuditError::io(format!("cannot read {}", path.display()), error))
}

fn write_output(path: Option<PathBuf>, contents: String) -> Result<()> {
    match path {
        Some(path) => fs::write(&path, contents)
            .map_err(|error| AuditError::io(format!("cannot write {}", path.display()), error)),
        None => {
            print!("{contents}");
            Ok(())
        }
    }
}

fn parse_duration(value: &str) -> Result<Duration> {
    if let Some(milliseconds) = value.strip_suffix("ms") {
        let milliseconds = milliseconds
            .parse::<u64>()
            .map_err(|_| AuditError::message(format!("invalid duration: {value}")))?;
        return Ok(Duration::from_millis(milliseconds));
    }

    if let Some(seconds) = value.strip_suffix('s') {
        let seconds = seconds
            .parse::<u64>()
            .map_err(|_| AuditError::message(format!("invalid duration: {value}")))?;
        return Ok(Duration::from_secs(seconds));
    }

    Err(AuditError::message(format!(
        "invalid duration: {value}; use values like 5s or 500ms"
    )))
}

fn print_help() {
    println!(
        "RuntimeScaner\n\
         \n\
         Commands:\n\
           required  --exec <path> [--arg <value>] [--env NAME=VALUE] [--timeout 5s] [--cwd <dir>] [--out <file>]\n\
           inventory [--lib-dir <dir>] [--out <file>]\n\
           diff      --required <file> --inventory <file> [--ignore <file>] [--allow-arch-mismatch] [--out <file>]\n\
           collect   --missing <file> --libdir <dir> [--search-dir <dir>] [--out <file>]"
    );
}

struct ArgParser {
    args: Vec<String>,
    index: usize,
}

impl ArgParser {
    fn new(args: Vec<String>) -> Self {
        Self { args, index: 0 }
    }

    fn next(&mut self) -> Option<String> {
        let value = self.args.get(self.index).cloned()?;
        self.index += 1;
        Some(value)
    }

    fn value(&mut self, option: &str) -> Result<String> {
        self.next()
            .ok_or_else(|| AuditError::message(format!("{option} needs a value")))
    }
}
