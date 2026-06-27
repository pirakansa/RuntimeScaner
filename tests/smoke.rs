use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn help_lists_audit_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_runtimescaner"))
        .arg("--help")
        .output()
        .expect("binary should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("required"));
    assert!(stdout.contains("inventory"));
    assert!(stdout.contains("diff"));
    assert!(stdout.contains("collect"));
}

#[test]
fn required_emits_static_needed_libraries() {
    let output = Command::new(env!("CARGO_BIN_EXE_runtimescaner"))
        .args(["required", "--exec", "/bin/true", "--timeout", "1s"])
        .output()
        .expect("required should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""static_needed": ["#));
    assert!(stdout.contains(r#""libc.so.6""#));
}

#[test]
fn inventory_includes_explicit_library_directory_entries() {
    let temp_dir = unique_temp_dir("runtimescaner-inventory-cli");
    let lib_dir = temp_dir.join("lib");
    fs::create_dir_all(&lib_dir).expect("library dir should be created");
    fs::write(lib_dir.join("libcli-demo.so.1"), b"demo").expect("library should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_runtimescaner"))
        .args(["inventory", "--libdir", lib_dir.to_str().unwrap()])
        .output()
        .expect("inventory should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""soname": "libcli-demo.so.1""#));
    assert!(stdout.contains(&lib_dir.join("libcli-demo.so.1").display().to_string()));

    fs::remove_dir_all(temp_dir).expect("temp dir should be removed");
}

#[test]
fn diff_reports_missing_and_ignored_libraries() {
    let temp_dir = unique_temp_dir("runtimescaner-diff");
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let required = temp_dir.join("required.json");
    let inventory = temp_dir.join("inventory.json");
    let ignore = temp_dir.join("ignore.toml");
    fs::write(
        &required,
        r#"{
  "schema_version": 1,
  "arch": "x86_64",
  "executable": "./app",
  "command": ["./app"],
  "environment": {},
  "static_needed": ["libc.so.6"],
  "runtime_requested": ["libGL.so.1", "libXcursor.so.1"],
  "loaded_paths": [],
  "diagnostics": []
}
"#,
    )
    .expect("required file should be written");
    fs::write(
        &inventory,
        r#"{
  "schema_version": 1,
  "arch": "x86_64",
  "libraries": [
    {"soname": "libc.so.6", "path": "/lib/libc.so.6"}
  ]
}
"#,
    )
    .expect("inventory file should be written");
    fs::write(
        &ignore,
        r#"[[ignore]]
pattern = "libGL*.so*"
reason = "server-owned GPU stack"
"#,
    )
    .expect("ignore file should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_runtimescaner"))
        .args([
            "diff",
            "--required",
            required.to_str().unwrap(),
            "--inventory",
            inventory.to_str().unwrap(),
            "--ignore",
            ignore.to_str().unwrap(),
        ])
        .output()
        .expect("diff should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""missing_before_ignore""#));
    assert!(stdout.contains(r#""soname": "libGL.so.1""#));
    assert!(stdout.contains(r#""bundle_candidates": ["#));
    assert!(stdout.contains(r#""libXcursor.so.1""#));

    fs::remove_dir_all(temp_dir).expect("temp dir should be removed");
}

#[test]
fn diff_allows_architecture_mismatch_when_requested() {
    let temp_dir = unique_temp_dir("runtimescaner-diff-arch");
    fs::create_dir_all(&temp_dir).expect("temp dir should be created");

    let required = temp_dir.join("required.json");
    let inventory = temp_dir.join("inventory.json");
    fs::write(
        &required,
        r#"{
  "schema_version": 1,
  "arch": "x86_64",
  "executable": "./app",
  "command": ["./app"],
  "environment": {},
  "static_needed": ["libmissing.so.1"],
  "runtime_requested": [],
  "loaded_paths": [],
  "diagnostics": []
}
"#,
    )
    .expect("required file should be written");
    fs::write(
        &inventory,
        r#"{
  "schema_version": 1,
  "arch": "aarch64",
  "libraries": []
}
"#,
    )
    .expect("inventory file should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_runtimescaner"))
        .args([
            "diff",
            "--required",
            required.to_str().unwrap(),
            "--inventory",
            inventory.to_str().unwrap(),
            "--allow-arch-mismatch",
        ])
        .output()
        .expect("diff should run");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""arch": "aarch64""#));
    assert!(stdout.contains(r#""libmissing.so.1""#));

    fs::remove_dir_all(temp_dir).expect("temp dir should be removed");
}

#[test]
fn collect_copies_resolved_bundle_candidates() {
    let temp_dir = unique_temp_dir("runtimescaner-collect");
    let search_dir = temp_dir.join("search");
    let output_dir = temp_dir.join("out");
    fs::create_dir_all(&search_dir).expect("search dir should be created");
    fs::write(search_dir.join("libdemo.so.1"), b"demo").expect("library should be written");
    let missing = temp_dir.join("missing.json");
    fs::write(
        &missing,
        r#"{
  "schema_version": 1,
  "arch": "x86_64",
  "missing_before_ignore": ["libdemo.so.1"],
  "ignored": [],
  "bundle_candidates": ["libdemo.so.1", "libmissing.so.1"]
}
"#,
    )
    .expect("missing file should be written");

    let output = Command::new(env!("CARGO_BIN_EXE_runtimescaner"))
        .args([
            "collect",
            "--missing",
            missing.to_str().unwrap(),
            "--search-dir",
            search_dir.to_str().unwrap(),
            "--libdir",
            output_dir.to_str().unwrap(),
        ])
        .output()
        .expect("collect should run");

    assert!(output.status.success());
    assert_eq!(
        fs::read(output_dir.join("libdemo.so.1")).expect("copied library should exist"),
        b"demo"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(r#""libmissing.so.1""#));

    fs::remove_dir_all(temp_dir).expect("temp dir should be removed");
}

fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}
