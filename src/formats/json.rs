use std::collections::BTreeMap;

use crate::error::{AuditError, Result};
use crate::models::{
    CollectReport, DiffReport, InventoryReport, LibraryEntry, RequiredReport, SCHEMA_VERSION,
};

pub fn required_to_json(report: &RequiredReport) -> String {
    let mut output = JsonWriter::object();
    output.number_field("schema_version", SCHEMA_VERSION as u64, true);
    output.string_field("arch", &report.arch, true);
    output.string_field("executable", &report.executable, true);
    output.string_array_field("command", &report.command, true);
    output.string_object_field("environment", &report.environment, true);
    output.string_array_field("static_needed", &report.static_needed, true);
    output.string_array_field("runtime_requested", &report.runtime_requested, true);
    output.string_array_field("loaded_paths", &report.loaded_paths, true);
    output.string_array_field("diagnostics", &report.diagnostics, false);
    output.finish()
}

pub fn inventory_to_json(report: &InventoryReport) -> String {
    let mut output = JsonWriter::object();
    output.number_field("schema_version", SCHEMA_VERSION as u64, true);
    output.string_field("arch", &report.arch, true);
    output.library_array_field("libraries", &report.libraries, false);
    output.finish()
}

pub fn diff_to_json(report: &DiffReport) -> String {
    let mut output = JsonWriter::object();
    output.number_field("schema_version", SCHEMA_VERSION as u64, true);
    output.string_field("arch", &report.arch, true);
    output.string_array_field("missing_before_ignore", &report.missing_before_ignore, true);
    output.ignored_array_field("ignored", &report.ignored, true);
    output.string_array_field("bundle_candidates", &report.bundle_candidates, false);
    output.finish()
}

pub fn collect_to_json(report: &CollectReport) -> String {
    let mut output = JsonWriter::object();
    output.number_field("schema_version", SCHEMA_VERSION as u64, true);
    output.string_field("arch", &report.arch, true);
    output.library_array_field("copied", &report.copied, true);
    output.string_array_field("unresolved", &report.unresolved, false);
    output.finish()
}

pub fn parse_required_json(input: &str) -> Result<RequiredReport> {
    Ok(RequiredReport {
        arch: extract_json_string_field(input, "arch")?,
        executable: extract_json_string_field(input, "executable")?,
        command: extract_json_string_array_field(input, "command")?,
        environment: BTreeMap::new(),
        static_needed: extract_json_string_array_field(input, "static_needed")?,
        runtime_requested: extract_json_string_array_field(input, "runtime_requested")?,
        loaded_paths: extract_json_string_array_field(input, "loaded_paths")?,
        diagnostics: extract_json_string_array_field(input, "diagnostics")?,
    })
}

pub fn parse_inventory_json(input: &str) -> Result<InventoryReport> {
    let arch = extract_json_string_field(input, "arch")?;
    let libraries_block = extract_json_array_block(input, "libraries")?;
    let mut libraries = Vec::new();

    for object in split_json_objects(libraries_block) {
        libraries.push(LibraryEntry {
            soname: extract_json_string_field(object, "soname")?,
            path: extract_json_string_field(object, "path")?,
        });
    }

    Ok(InventoryReport { arch, libraries })
}

pub fn parse_diff_json(input: &str) -> Result<DiffReport> {
    Ok(DiffReport {
        arch: extract_json_string_field(input, "arch")?,
        missing_before_ignore: extract_json_string_array_field(input, "missing_before_ignore")?,
        ignored: Vec::new(),
        bundle_candidates: extract_json_string_array_field(input, "bundle_candidates")?,
    })
}

struct JsonWriter {
    output: String,
}

impl JsonWriter {
    fn object() -> Self {
        Self {
            output: "{\n".to_string(),
        }
    }

    fn finish(mut self) -> String {
        self.output.push_str("}\n");
        self.output
    }

    fn number_field(&mut self, key: &str, value: u64, trailing_comma: bool) {
        self.push_indent(2);
        self.output.push_str(&format!("\"{key}\": {value}"));
        self.finish_field(trailing_comma);
    }

    fn string_field(&mut self, key: &str, value: &str, trailing_comma: bool) {
        self.push_indent(2);
        self.output
            .push_str(&format!("\"{key}\": \"{}\"", json_escape(value)));
        self.finish_field(trailing_comma);
    }

    fn string_array_field(&mut self, key: &str, values: &[String], trailing_comma: bool) {
        self.push_indent(2);
        self.output.push_str(&format!("\"{key}\": ["));
        if !values.is_empty() {
            self.output.push('\n');
            for (index, value) in values.iter().enumerate() {
                self.push_indent(4);
                self.output.push_str(&format!("\"{}\"", json_escape(value)));
                self.finish_array_item(index + 1 != values.len());
            }
            self.push_indent(2);
        }
        self.output.push(']');
        self.finish_field(trailing_comma);
    }

    fn string_object_field(
        &mut self,
        key: &str,
        values: &BTreeMap<String, String>,
        trailing_comma: bool,
    ) {
        self.push_indent(2);
        self.output.push_str(&format!("\"{key}\": {{"));
        if !values.is_empty() {
            self.output.push('\n');
            for (index, (name, value)) in values.iter().enumerate() {
                self.push_indent(4);
                self.output.push_str(&format!(
                    "\"{}\": \"{}\"",
                    json_escape(name),
                    json_escape(value)
                ));
                self.finish_array_item(index + 1 != values.len());
            }
            self.push_indent(2);
        }
        self.output.push('}');
        self.finish_field(trailing_comma);
    }

    fn library_array_field(&mut self, key: &str, values: &[LibraryEntry], trailing_comma: bool) {
        self.push_indent(2);
        self.output.push_str(&format!("\"{key}\": [\n"));
        for (index, library) in values.iter().enumerate() {
            self.push_library_object(library, index + 1 != values.len());
        }
        self.push_indent(2);
        self.output.push(']');
        self.finish_field(trailing_comma);
    }

    fn ignored_array_field(
        &mut self,
        key: &str,
        values: &[crate::models::IgnoredLibrary],
        trailing_comma: bool,
    ) {
        self.push_indent(2);
        self.output.push_str(&format!("\"{key}\": [\n"));
        for (index, ignored) in values.iter().enumerate() {
            self.push_indent(4);
            self.output.push_str("{\n");
            self.push_keyed_string("soname", &ignored.soname, 6, true);
            self.push_keyed_string("reason", &ignored.reason, 6, false);
            self.push_indent(4);
            self.output.push('}');
            self.finish_array_item(index + 1 != values.len());
        }
        self.push_indent(2);
        self.output.push(']');
        self.finish_field(trailing_comma);
    }

    fn push_library_object(&mut self, library: &LibraryEntry, trailing_comma: bool) {
        self.push_indent(4);
        self.output.push_str("{\n");
        self.push_keyed_string("soname", &library.soname, 6, true);
        self.push_keyed_string("path", &library.path, 6, false);
        self.push_indent(4);
        self.output.push('}');
        self.finish_array_item(trailing_comma);
    }

    fn push_keyed_string(&mut self, key: &str, value: &str, indent: usize, trailing_comma: bool) {
        self.push_indent(indent);
        self.output
            .push_str(&format!("\"{key}\": \"{}\"", json_escape(value)));
        self.finish_field(trailing_comma);
    }

    fn finish_array_item(&mut self, trailing_comma: bool) {
        if trailing_comma {
            self.output.push(',');
        }
        self.output.push('\n');
    }

    fn finish_field(&mut self, trailing_comma: bool) {
        if trailing_comma {
            self.output.push(',');
        }
        self.output.push('\n');
    }

    fn push_indent(&mut self, indent: usize) {
        self.output.push_str(&" ".repeat(indent));
    }
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character => escaped.push(character),
        }
    }
    escaped
}

fn extract_json_string_field(input: &str, key: &str) -> Result<String> {
    let value = extract_json_field_value(input, key)?;
    parse_json_string(value)
        .map(|(value, _)| value)
        .ok_or_else(|| AuditError::message(format!("invalid JSON string field: {key}")))
}

fn extract_json_string_array_field(input: &str, key: &str) -> Result<Vec<String>> {
    let block = extract_json_array_block(input, key)?;
    let mut values = Vec::new();
    let mut rest = block.trim();
    while !rest.is_empty() {
        if rest.starts_with(',') {
            rest = rest[1..].trim_start();
            continue;
        }
        let Some((value, remaining)) = parse_json_string(rest) else {
            break;
        };
        values.push(value);
        rest = remaining.trim_start();
    }
    Ok(values)
}

fn extract_json_array_block<'a>(input: &'a str, key: &str) -> Result<&'a str> {
    let value = extract_json_field_value(input, key)?;
    if !value.starts_with('[') {
        return Err(AuditError::message(format!(
            "JSON field is not an array: {key}"
        )));
    }

    find_matching_array_end(value, key).map(|end| &value[1..end])
}

fn extract_json_field_value<'a>(input: &'a str, key: &str) -> Result<&'a str> {
    let marker = format!("\"{key}\"");
    let start = input
        .find(&marker)
        .ok_or_else(|| AuditError::message(format!("missing JSON field: {key}")))?;
    let after_key = &input[start + marker.len()..];
    let colon = after_key
        .find(':')
        .ok_or_else(|| AuditError::message(format!("invalid JSON field: {key}")))?;
    Ok(after_key[colon + 1..].trim_start())
}

fn find_matching_array_end(value: &str, key: &str) -> Result<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in value.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(index);
                }
            }
            _ => {}
        }
    }

    Err(AuditError::message(format!(
        "unterminated JSON array field: {key}"
    )))
}

fn split_json_objects(input: &str) -> Vec<&str> {
    let mut objects = Vec::new();
    let mut start = None;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, character) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        match character {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    start = Some(index);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(start) = start.take() {
                        objects.push(&input[start..=index]);
                    }
                }
            }
            _ => {}
        }
    }

    objects
}

fn parse_json_string(input: &str) -> Option<(String, &str)> {
    let input = input.trim_start();
    if !input.starts_with('"') {
        return None;
    }

    let mut value = String::new();
    let mut escaped = false;
    for (index, character) in input[1..].char_indices() {
        if escaped {
            match character {
                '"' => value.push('"'),
                '\\' => value.push('\\'),
                'n' => value.push('\n'),
                'r' => value.push('\r'),
                't' => value.push('\t'),
                other => value.push(other),
            }
            escaped = false;
            continue;
        }

        match character {
            '\\' => escaped = true,
            '"' => return Some((value, &input[index + 2..])),
            other => value.push(other),
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_generated_required_json() {
        let report = RequiredReport {
            arch: "x86_64".to_string(),
            executable: "./app".to_string(),
            command: vec!["./app".to_string()],
            environment: BTreeMap::new(),
            static_needed: vec!["libc.so.6".to_string()],
            runtime_requested: vec!["libX11.so.6".to_string()],
            loaded_paths: vec!["/lib/libX11.so.6".to_string()],
            diagnostics: Vec::new(),
        };

        let parsed = parse_required_json(&required_to_json(&report)).unwrap();

        assert_eq!(parsed.arch, report.arch);
        assert_eq!(parsed.static_needed, report.static_needed);
        assert_eq!(parsed.runtime_requested, report.runtime_requested);
    }

    #[test]
    fn parses_generated_inventory_json() {
        let report = InventoryReport {
            arch: "x86_64".to_string(),
            libraries: vec![LibraryEntry {
                soname: "libdemo.so.1".to_string(),
                path: "/lib/libdemo.so.1".to_string(),
            }],
        };

        let parsed = parse_inventory_json(&inventory_to_json(&report)).unwrap();

        assert_eq!(parsed, report);
    }

    #[test]
    fn parses_generated_diff_json_bundle_candidates() {
        let report = DiffReport {
            arch: "x86_64".to_string(),
            missing_before_ignore: vec!["libdemo.so.1".to_string()],
            ignored: Vec::new(),
            bundle_candidates: vec!["libdemo.so.1".to_string()],
        };

        let parsed = parse_diff_json(&diff_to_json(&report)).unwrap();

        assert_eq!(parsed.arch, report.arch);
        assert_eq!(parsed.missing_before_ignore, report.missing_before_ignore);
        assert_eq!(parsed.bundle_candidates, report.bundle_candidates);
    }
}
