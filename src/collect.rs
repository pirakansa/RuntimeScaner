use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AuditError, Result};
use crate::models::{CollectReport, LibraryEntry};
use crate::util::current_arch;

pub fn collect_libraries(
    missing: &[String],
    search_dirs: &[PathBuf],
    output_dir: &Path,
) -> Result<CollectReport> {
    fs::create_dir_all(output_dir).map_err(|error| {
        AuditError::io(
            format!("cannot create output directory {}", output_dir.display()),
            error,
        )
    })?;

    let mut copied = Vec::new();
    let mut unresolved = Vec::new();

    for soname in missing {
        match collect_one_library(soname, search_dirs, output_dir)? {
            Some(library) => copied.push(library),
            None => unresolved.push(soname.clone()),
        }
    }

    Ok(CollectReport {
        arch: current_arch(),
        copied,
        unresolved,
    })
}

pub fn resolve_library(soname: &str, search_dirs: &[PathBuf]) -> Option<PathBuf> {
    for dir in search_dirs {
        let candidate = dir.join(soname);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn collect_one_library(
    soname: &str,
    search_dirs: &[PathBuf],
    output_dir: &Path,
) -> Result<Option<LibraryEntry>> {
    let Some(source) = resolve_library(soname, search_dirs) else {
        return Ok(None);
    };

    let destination = output_dir.join(soname);
    fs::copy(&source, &destination).map_err(|error| {
        AuditError::io(
            format!(
                "cannot copy {} to {}",
                source.display(),
                destination.display()
            ),
            error,
        )
    })?;

    Ok(Some(LibraryEntry {
        soname: soname.to_string(),
        path: destination.display().to_string(),
    }))
}
