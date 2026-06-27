use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

pub const SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredReport {
    pub arch: String,
    pub executable: String,
    pub command: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub static_needed: Vec<String>,
    pub runtime_requested: Vec<String>,
    pub loaded_paths: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryEntry {
    pub soname: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryReport {
    pub arch: String,
    pub libraries: Vec<LibraryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoreRule {
    pub pattern: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoredLibrary {
    pub soname: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffReport {
    pub arch: String,
    pub missing_before_ignore: Vec<String>,
    pub ignored: Vec<IgnoredLibrary>,
    pub bundle_candidates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectReport {
    pub arch: String,
    pub copied: Vec<LibraryEntry>,
    pub unresolved: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RequiredOptions {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub envs: BTreeMap<String, String>,
    pub timeout: Option<Duration>,
    pub cwd: Option<PathBuf>,
}
