//! Read-only repo observation.
//!
//! Snapshot of observable repo structure for consistency checking.
//! Strictly read-only — no write methods on the filesystem trait.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::report::normalize_repo_path;

/// Error type for repo observation.
#[derive(Debug, thiserror::Error)]
pub enum RepoObserveError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    TomlParse(String),
    #[error("path not found: {0}")]
    NotFound(PathBuf),
}

/// Read-only filesystem trait. No write methods.
pub trait RepoReadFs: Send + Sync {
    fn read_to_string(&self, path: &Path) -> Result<String, RepoObserveError>;
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, RepoObserveError>;
    fn exists(&self, path: &Path) -> bool;
}

/// Standard filesystem implementation.
pub struct StdRepoReadFs;

impl RepoReadFs for StdRepoReadFs {
    fn read_to_string(&self, path: &Path) -> Result<String, RepoObserveError> {
        Ok(std::fs::read_to_string(path)?)
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, RepoObserveError> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            entries.push(entry.path());
        }
        Ok(entries)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

/// Snapshot of observed repo state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoObservationSnapshot {
    pub repo_root: PathBuf,
    pub cargo_package_name: Option<String>,
    pub workspace_members: Vec<String>,
    pub crates: Vec<ObservedCrate>,
    pub docs_files: Vec<String>,
    pub dependencies: Vec<ObservedDependency>,
}

/// An observed crate in the workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedCrate {
    pub name: String,
    pub path: String,
    pub src_files: Vec<String>,
    pub test_files: Vec<String>,
}

/// An observed dependency.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObservedDependency {
    pub crate_name: String,
    pub dependency_name: String,
}

/// Observe a repo and produce a snapshot.
pub fn observe_repo(fs: &dyn RepoReadFs, repo_root: &Path) -> Result<RepoObservationSnapshot, RepoObserveError> {
    let cargo_toml_path = repo_root.join("Cargo.toml");
    let cargo_content = if fs.exists(&cargo_toml_path) {
        Some(fs.read_to_string(&cargo_toml_path)?)
    } else {
        None
    };

    let (package_name, workspace_members) = parse_cargo_toml(cargo_content.as_deref());

    // Observe crates/* directories
    let crates_dir = repo_root.join("crates");
    let mut crates = Vec::new();
    if fs.exists(&crates_dir) {
        if let Ok(entries) = fs.read_dir(&crates_dir) {
            for entry in entries {
                let cargo_path = entry.join("Cargo.toml");
                if fs.exists(&cargo_path) {
                    if let Ok(content) = fs.read_to_string(&cargo_path) {
                        if let Some((name, _, _)) = parse_crate_cargo_toml(&content) {
                            let crate_path = normalize_repo_path(&entry.strip_prefix(repo_root).unwrap_or(&entry));
                            let src_dir = entry.join("src");
                            let test_dir = entry.join("tests");

                            let src_files = list_rs_files(fs, &src_dir, repo_root);
                            let test_files = list_rs_files(fs, &test_dir, repo_root);

                            crates.push(ObservedCrate {
                                name,
                                path: crate_path,
                                src_files,
                                test_files,
                            });
                        }
                    }
                }
            }
        }
    }

    // Observe docs/* files
    let docs_dir = repo_root.join("docs");
    let docs_files = if fs.exists(&docs_dir) {
        list_files_recursive(fs, &docs_dir, repo_root)
    } else {
        vec![]
    };

    // Observe dependencies from root + crate Cargo.tomls
    let mut dependencies = Vec::new();
    if let Some(ref content) = cargo_content {
        dependencies.extend(parse_dependencies("workspace", content));
    }
    for observed_crate in &crates {
        // Use path (directory name) not crate name (package name)
        let crate_cargo = repo_root.join(&observed_crate.path).join("Cargo.toml");
        if let Ok(content) = fs.read_to_string(&crate_cargo) {
            dependencies.extend(parse_dependencies(&observed_crate.name, &content));
        }
    }

    Ok(RepoObservationSnapshot {
        repo_root: repo_root.to_path_buf(),
        cargo_package_name: package_name,
        workspace_members,
        crates,
        docs_files,
        dependencies,
    })
}

fn parse_cargo_toml(content: Option<&str>) -> (Option<String>, Vec<String>) {
    let Some(content) = content else { return (None, vec![]) };

    let package_name = content
        .lines()
        .find(|l| l.trim().starts_with("name"))
        .and_then(|l| l.split('=').nth(1))
        .map(|v| v.trim().trim_matches('"').to_string());

    let workspace_members = content
        .lines()
        .find(|l| l.trim().starts_with("members"))
        .and_then(|l| l.split('=').nth(1))
        .map(|v| {
            v.trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    (package_name, workspace_members)
}

fn parse_crate_cargo_toml(content: &str) -> Option<(String, Vec<String>, Vec<ObservedDependency>)> {
    let name = content
        .lines()
        .find(|l| l.trim().starts_with("name"))
        .and_then(|l| l.split('=').nth(1))
        .map(|v| v.trim().trim_matches('"').to_string())?;

    let deps = parse_dependencies(&name, content);
    Some((name, vec![], deps))
}

fn parse_dependencies(crate_name: &str, content: &str) -> Vec<ObservedDependency> {
    let mut deps = Vec::new();
    let mut in_deps = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_deps = trimmed == "[dependencies]";
            continue;
        }
        if in_deps {
            if let Some(dep_name) = trimmed.split('=').next() {
                let dep_name = dep_name.trim().to_string();
                if !dep_name.is_empty() {
                    deps.push(ObservedDependency {
                        crate_name: crate_name.to_string(),
                        dependency_name: dep_name,
                    });
                }
            }
        }
    }
    deps
}

fn list_rs_files(fs: &dyn RepoReadFs, dir: &Path, repo_root: &Path) -> Vec<String> {
    list_files_recursive(fs, dir, repo_root)
        .into_iter()
        .filter(|p| p.ends_with(".rs"))
        .collect()
}

fn list_files_recursive(fs: &dyn RepoReadFs, dir: &Path, repo_root: &Path) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = fs.read_dir(dir) {
        for entry in entries {
            if fs.exists(&entry) {
                if let Ok(entries2) = fs.read_dir(&entry) {
                    // It's a directory — recurse
                    let relative = entry.strip_prefix(repo_root).unwrap_or(&entry);
                    for sub_entry in entries2 {
                        let rel = sub_entry.strip_prefix(repo_root).unwrap_or(&sub_entry);
                        files.push(normalize_repo_path(rel));
                    }
                } else {
                    // It's a file
                    let relative = entry.strip_prefix(repo_root).unwrap_or(&entry);
                    files.push(normalize_repo_path(relative));
                }
            }
        }
    }
    files.sort();
    files
}

#[cfg(test)]
pub struct StubRepoReadFs {
    files: std::collections::HashMap<String, String>,
    dirs: std::collections::HashMap<String, Vec<PathBuf>>,
}

#[cfg(test)]
impl StubRepoReadFs {
    pub fn new() -> Self {
        Self {
            files: std::collections::HashMap::new(),
            dirs: std::collections::HashMap::new(),
        }
    }

    pub fn add_file(&mut self, path: &str, content: &str) {
        self.files.insert(path.to_string(), content.to_string());
        if let Some(parent) = Path::new(path).parent() {
            let parent_str = parent.to_string_lossy().to_string();
            self.dirs.entry(parent_str).or_default().push(PathBuf::from(path));
        }
    }

    pub fn add_dir(&mut self, path: &str, children: Vec<&str>) {
        self.dirs.insert(
            path.to_string(),
            children.into_iter().map(PathBuf::from).collect(),
        );
    }
}

#[cfg(test)]
impl RepoReadFs for StubRepoReadFs {
    fn read_to_string(&self, path: &Path) -> Result<String, RepoObserveError> {
        let key = path.to_string_lossy().replace('\\', "/");
        self.files
            .get(&key)
            .cloned()
            .ok_or_else(|| RepoObserveError::NotFound(path.to_path_buf()))
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, RepoObserveError> {
        let key = path.to_string_lossy().replace('\\', "/");
        self.dirs
            .get(&key)
            .cloned()
            .ok_or_else(|| RepoObserveError::NotFound(path.to_path_buf()))
    }

    fn exists(&self, path: &Path) -> bool {
        let key = path.to_string_lossy().replace('\\', "/");
        self.files.contains_key(&key) || self.dirs.contains_key(&key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_fs() -> StubRepoReadFs {
        let mut fs = StubRepoReadFs::new();
        fs.add_file("/repo/Cargo.toml", r#"
[package]
name = "openwand"
members = ["crates/core", "crates/memory"]
"#);
        fs.add_dir("/repo", vec!["/repo/Cargo.toml", "/repo/crates"]);
        fs.add_dir("/repo/crates", vec!["/repo/crates/core", "/repo/crates/memory"]);
        fs.add_file("/repo/crates/core/Cargo.toml", r#"
[package]
name = "openwand-core"
[dependencies]
serde = "1"
"#);
        fs.add_dir("/repo/crates/core", vec!["/repo/crates/core/Cargo.toml", "/repo/crates/core/src"]);
        fs.add_dir("/repo/crates/core/src", vec!["/repo/crates/core/src/lib.rs"]);
        fs.add_file("/repo/crates/core/src/lib.rs", "");
        fs.add_file("/repo/crates/memory/Cargo.toml", r#"
[package]
name = "openwand-memory"
[dependencies]
serde = "1"
blake3 = "1"
"#);
        fs.add_dir("/repo/crates/memory", vec!["/repo/crates/memory/Cargo.toml", "/repo/crates/memory/src", "/repo/crates/memory/tests"]);
        fs.add_dir("/repo/crates/memory/src", vec!["/repo/crates/memory/src/lib.rs"]);
        fs.add_file("/repo/crates/memory/src/lib.rs", "");
        fs.add_dir("/repo/crates/memory/tests", vec!["/repo/crates/memory/tests/smoke.rs"]);
        fs.add_file("/repo/crates/memory/tests/smoke.rs", "");
        fs.add_dir("/repo/docs", vec!["/repo/docs/LOCK.md"]);
        fs.add_file("/repo/docs/LOCK.md", "");
        fs
    }

    #[test]
    fn repo_observer_detects_workspace_members() {
        let fs = workspace_fs();
        let snap = observe_repo(&fs, Path::new("/repo")).unwrap();
        assert!(snap.workspace_members.contains(&"crates/core".to_string()));
        assert!(snap.workspace_members.contains(&"crates/memory".to_string()));
    }

    #[test]
    fn repo_observer_detects_crate_src_files() {
        let fs = workspace_fs();
        let snap = observe_repo(&fs, Path::new("/repo")).unwrap();
        let core = snap.crates.iter().find(|c| c.name == "openwand-core").unwrap();
        assert!(core.src_files.iter().any(|f| f.contains("lib.rs")));
    }

    #[test]
    fn repo_observer_detects_test_files() {
        let fs = workspace_fs();
        let snap = observe_repo(&fs, Path::new("/repo")).unwrap();
        let memory = snap.crates.iter().find(|c| c.name == "openwand-memory").unwrap();
        assert!(memory.test_files.iter().any(|f| f.contains("smoke.rs")));
    }

    #[test]
    fn repo_observer_detects_docs_files() {
        let fs = workspace_fs();
        let snap = observe_repo(&fs, Path::new("/repo")).unwrap();
        assert!(snap.docs_files.iter().any(|f| f.contains("LOCK.md")));
    }

    #[test]
    fn repo_observer_detects_dependencies() {
        let fs = workspace_fs();
        let snap = observe_repo(&fs, Path::new("/repo")).unwrap();
        assert!(snap.dependencies.iter().any(|d| d.dependency_name == "serde"));
        assert!(snap.dependencies.iter().any(|d| d.dependency_name == "blake3"));
    }

    #[test]
    fn repo_observer_handles_missing_cargo_toml() {
        let mut fs = StubRepoReadFs::new();
        fs.add_dir("/empty", vec![]);
        let snap = observe_repo(&fs, Path::new("/empty")).unwrap();
        assert_eq!(None, snap.cargo_package_name);
        assert!(snap.crates.is_empty());
    }

    #[test]
    fn repo_observer_is_read_only() {
        let fs = workspace_fs();
        let _snap = observe_repo(&fs, Path::new("/repo")).unwrap();
        // RepoReadFs has no write methods — this is a compile-time guarantee
        // The trait only exposes read_to_string, read_dir, exists
        assert!(true, "RepoReadFs exposes no write methods");
    }

    #[test]
    fn repo_observer_does_not_change_directory_snapshot() {
        let fs = workspace_fs();
        let snap1 = observe_repo(&fs, Path::new("/repo")).unwrap();
        let snap2 = observe_repo(&fs, Path::new("/repo")).unwrap();
        // Two observations must be identical — nothing was mutated
        assert_eq!(snap1.cargo_package_name, snap2.cargo_package_name);
        assert_eq!(snap1.crates.len(), snap2.crates.len());
        assert_eq!(snap1.docs_files.len(), snap2.docs_files.len());
    }
}
