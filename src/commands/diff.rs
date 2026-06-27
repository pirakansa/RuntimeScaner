use std::collections::BTreeSet;

use crate::error::{AuditError, Result};
use crate::models::{DiffReport, IgnoreRule, IgnoredLibrary, InventoryReport, RequiredReport};
use crate::util::glob_matches;

pub fn diff_reports(
    required: &RequiredReport,
    inventory: &InventoryReport,
    ignore_rules: &[IgnoreRule],
    allow_arch_mismatch: bool,
) -> Result<DiffReport> {
    validate_architecture(required, inventory, allow_arch_mismatch)?;

    let available = available_sonames(inventory);
    let required_sonames = required_sonames(required);
    let mut missing_before_ignore = Vec::new();
    let mut ignored = Vec::new();
    let mut bundle_candidates = Vec::new();

    for soname in required_sonames {
        if available.contains(soname.as_str()) {
            continue;
        }

        missing_before_ignore.push(soname.clone());
        if let Some(rule) = matching_ignore_rule(ignore_rules, &soname) {
            ignored.push(IgnoredLibrary {
                soname,
                reason: rule.reason.clone(),
            });
        } else {
            bundle_candidates.push(soname);
        }
    }

    Ok(DiffReport {
        arch: inventory.arch.clone(),
        missing_before_ignore,
        ignored,
        bundle_candidates,
    })
}

pub fn parse_ignore_toml(input: &str) -> Vec<IgnoreRule> {
    let mut rules = Vec::new();
    let mut pattern = None;
    let mut reason = None;

    for line in input.lines().map(str::trim) {
        if line == "[[ignore]]" {
            push_completed_rule(&mut rules, &mut pattern, &mut reason);
            continue;
        }

        if let Some(value) = parse_toml_string_assignment(line, "pattern") {
            pattern = Some(value);
        } else if let Some(value) = parse_toml_string_assignment(line, "reason") {
            reason = Some(value);
        }
    }

    push_completed_rule(&mut rules, &mut pattern, &mut reason);
    rules
}

fn validate_architecture(
    required: &RequiredReport,
    inventory: &InventoryReport,
    allow_arch_mismatch: bool,
) -> Result<()> {
    if required.arch != inventory.arch && !allow_arch_mismatch {
        return Err(AuditError::message(format!(
            "architecture mismatch: required is {}, inventory is {}",
            required.arch, inventory.arch
        )));
    }
    Ok(())
}

fn available_sonames(inventory: &InventoryReport) -> BTreeSet<&str> {
    inventory
        .libraries
        .iter()
        .map(|library| library.soname.as_str())
        .collect()
}

fn required_sonames(required: &RequiredReport) -> BTreeSet<String> {
    required
        .static_needed
        .iter()
        .chain(required.runtime_requested.iter())
        .cloned()
        .collect()
}

fn matching_ignore_rule<'a>(
    ignore_rules: &'a [IgnoreRule],
    soname: &str,
) -> Option<&'a IgnoreRule> {
    ignore_rules
        .iter()
        .find(|rule| glob_matches(&rule.pattern, soname))
}

fn push_completed_rule(
    rules: &mut Vec<IgnoreRule>,
    pattern: &mut Option<String>,
    reason: &mut Option<String>,
) {
    if let (Some(pattern), Some(reason)) = (pattern.take(), reason.take()) {
        rules.push(IgnoreRule { pattern, reason });
    }
}

fn parse_toml_string_assignment(line: &str, key: &str) -> Option<String> {
    let (left, right) = line.split_once('=')?;
    if left.trim() != key {
        return None;
    }
    let value = right.trim();
    if !value.starts_with('"') || !value.ends_with('"') {
        return None;
    }
    Some(value[1..value.len() - 1].replace("\\\"", "\""))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::models::LibraryEntry;

    #[test]
    fn diff_separates_ignored_missing_libraries() {
        let required = RequiredReport {
            arch: "x86_64".to_string(),
            executable: "./app".to_string(),
            command: vec!["./app".to_string()],
            environment: BTreeMap::new(),
            static_needed: vec!["libc.so.6".to_string()],
            runtime_requested: vec!["libXcursor.so.1".to_string(), "libGL.so.1".to_string()],
            loaded_paths: Vec::new(),
            diagnostics: Vec::new(),
        };
        let inventory = InventoryReport {
            arch: "x86_64".to_string(),
            libraries: vec![LibraryEntry {
                soname: "libc.so.6".to_string(),
                path: "/lib/libc.so.6".to_string(),
            }],
        };
        let ignore = vec![IgnoreRule {
            pattern: "libGL*.so*".to_string(),
            reason: "server-owned GPU stack".to_string(),
        }];

        let report = diff_reports(&required, &inventory, &ignore, false).unwrap();

        assert_eq!(
            report.missing_before_ignore,
            vec!["libGL.so.1", "libXcursor.so.1"]
        );
        assert_eq!(
            report.ignored,
            vec![IgnoredLibrary {
                soname: "libGL.so.1".to_string(),
                reason: "server-owned GPU stack".to_string(),
            }]
        );
        assert_eq!(report.bundle_candidates, vec!["libXcursor.so.1"]);
    }

    #[test]
    fn diff_rejects_architecture_mismatch_by_default() {
        let required = RequiredReport {
            arch: "x86_64".to_string(),
            executable: "./app".to_string(),
            command: vec!["./app".to_string()],
            environment: BTreeMap::new(),
            static_needed: Vec::new(),
            runtime_requested: Vec::new(),
            loaded_paths: Vec::new(),
            diagnostics: Vec::new(),
        };
        let inventory = InventoryReport {
            arch: "aarch64".to_string(),
            libraries: Vec::new(),
        };

        let error = diff_reports(&required, &inventory, &[], false).unwrap_err();

        assert!(error.to_string().contains("architecture mismatch"));
    }

    #[test]
    fn parses_ignore_rules() {
        let input = "[[ignore]]\n\
                     pattern = \"libGL*.so*\"\n\
                     reason = \"server-owned\"\n";

        let rules = parse_ignore_toml(input);

        assert_eq!(
            rules,
            vec![IgnoreRule {
                pattern: "libGL*.so*".to_string(),
                reason: "server-owned".to_string(),
            }]
        );
    }
}
