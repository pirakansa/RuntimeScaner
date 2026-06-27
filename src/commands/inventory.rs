use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::error::{AuditError, Result};
use crate::models::{InventoryReport, LibraryEntry};
use crate::util::current_arch;

pub fn run_inventory(extra_dirs: &[PathBuf]) -> Result<InventoryReport> {
    let mut libraries = parse_ldconfig()?;
    libraries.extend(scan_library_dirs(extra_dirs)?);
    libraries.sort_by(|left, right| {
        left.soname
            .cmp(&right.soname)
            .then_with(|| left.path.cmp(&right.path))
    });
    libraries.dedup();

    Ok(InventoryReport {
        arch: current_arch(),
        libraries,
    })
}

pub fn parse_ldconfig() -> Result<Vec<LibraryEntry>> {
    let output = Command::new("ldconfig")
        .arg("-p")
        .output()
        .map_err(|error| AuditError::io("ldconfig unavailable", error))?;

    if !output.status.success() {
        return Err(AuditError::CommandFailed {
            program: "ldconfig".to_string(),
            status: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_ldconfig_output(&stdout))
}

pub fn parse_ldconfig_output(output: &str) -> Vec<LibraryEntry> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let (soname, path) = trimmed.split_once("=>")?;
            let soname = soname.split_whitespace().next()?.to_string();
            let path = path.trim().to_string();
            Some(LibraryEntry { soname, path })
        })
        .collect()
}

pub fn scan_library_dirs(dirs: &[PathBuf]) -> Result<Vec<LibraryEntry>> {
    let mut entries = Vec::new();
    for dir in dirs {
        let read_dir = fs::read_dir(dir)
            .map_err(|error| AuditError::io(format!("cannot read {}", dir.display()), error))?;
        for entry in read_dir {
            let entry = entry
                .map_err(|error| AuditError::io(format!("cannot read {}", dir.display()), error))?;
            let path = entry.path();
            if is_shared_object_name(path.file_name()) {
                entries.push(LibraryEntry {
                    soname: path
                        .file_name()
                        .and_then(OsStr::to_str)
                        .unwrap_or_default()
                        .to_string(),
                    path: path.display().to_string(),
                });
            }
        }
    }
    Ok(entries)
}

fn is_shared_object_name(file_name: Option<&OsStr>) -> bool {
    let Some(name) = file_name.and_then(OsStr::to_str) else {
        return false;
    };
    name.ends_with(".so") || name.contains(".so.")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn parses_ldconfig_output() {
        let input = "libX11.so.6 (libc6,x86-64) => /lib/libX11.so.6\n";

        let libraries = parse_ldconfig_output(input);

        assert_eq!(
            libraries,
            vec![LibraryEntry {
                soname: "libX11.so.6".to_string(),
                path: "/lib/libX11.so.6".to_string(),
            }]
        );
    }

    #[test]
    fn scans_explicit_library_directories() {
        let temp_dir = unique_temp_dir("runtimescaner-inventory");
        fs::create_dir_all(&temp_dir).expect("temp dir should be created");
        fs::write(temp_dir.join("libdemo.so"), b"demo").expect("library should be written");
        fs::write(temp_dir.join("libdemo.so.1"), b"demo").expect("library should be written");
        fs::write(temp_dir.join("README.txt"), b"not a library").expect("file should be written");

        let mut libraries = scan_library_dirs(std::slice::from_ref(&temp_dir)).unwrap();
        libraries.sort_by(|left, right| left.soname.cmp(&right.soname));

        assert_eq!(
            libraries,
            vec![
                LibraryEntry {
                    soname: "libdemo.so".to_string(),
                    path: temp_dir.join("libdemo.so").display().to_string(),
                },
                LibraryEntry {
                    soname: "libdemo.so.1".to_string(),
                    path: temp_dir.join("libdemo.so.1").display().to_string(),
                },
            ]
        );

        fs::remove_dir_all(temp_dir).expect("temp dir should be removed");
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
    }
}
