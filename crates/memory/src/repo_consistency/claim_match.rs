//! Deterministic claim matching grammar v0.
//!
//! Intentionally narrow. No fuzzy natural-language matching.
//! Unsupported claims become Unverifiable, not hallucinated failures.

use serde::{Deserialize, Serialize};

/// A parsed repo claim pattern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoClaimPattern {
    CrateExists { crate_name: String },
    WorkspaceContainsCrate { crate_name: String },
    FileExists { path: String },
    ModuleExists { module_path: String },
    CrateDependsOn { crate_name: String, dependency: String },
    Unsupported,
}

/// Parse a claim text into a RepoClaimPattern.
/// Deterministic v0 grammar — no fuzzy matching.
pub fn parse_claim(text: &str) -> RepoClaimPattern {
    let lower = text.to_lowercase();
    let trimmed = lower.trim();

    // Pattern: "crate X exists" or "X crate exists"
    if let Some(name) = extract_crate_exists(trimmed) {
        return RepoClaimPattern::CrateExists { crate_name: name };
    }

    // Pattern: "workspace contains crate X"
    if let Some(name) = extract_workspace_contains(trimmed) {
        return RepoClaimPattern::WorkspaceContainsCrate { crate_name: name };
    }

    // Pattern: "file X exists"
    if let Some(path) = extract_file_exists(trimmed) {
        return RepoClaimPattern::FileExists { path };
    }

    // Pattern: "module X exists" or "module X"
    if let Some(path) = extract_module_exists(trimmed) {
        return RepoClaimPattern::ModuleExists { module_path: path };
    }

    // Pattern: "crate X depends on Y" or "X depends on Y"
    if let Some((crate_name, dep)) = extract_depends_on(trimmed) {
        return RepoClaimPattern::CrateDependsOn {
            crate_name,
            dependency: dep,
        };
    }

    RepoClaimPattern::Unsupported
}

fn extract_crate_exists(text: &str) -> Option<String> {
    // "crate X exists"
    if text.starts_with("crate ") && text.ends_with(" exists") {
        let name = text.trim_start_matches("crate ").trim_end_matches(" exists").trim();
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    // "X crate exists"
    if text.ends_with(" crate exists") {
        let name = text.trim_end_matches(" crate exists").trim();
        if !name.is_empty() && !name.contains(' ') {
            return Some(name.to_string());
        }
    }
    None
}

fn extract_workspace_contains(text: &str) -> Option<String> {
    if text.starts_with("workspace contains crate ") {
        let name = text.trim_start_matches("workspace contains crate ").trim();
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    None
}

fn extract_file_exists(text: &str) -> Option<String> {
    if text.starts_with("file ") && text.ends_with(" exists") {
        let path = text.trim_start_matches("file ").trim_end_matches(" exists").trim();
        if !path.is_empty() {
            return Some(path.to_string());
        }
    }
    None
}

fn extract_module_exists(text: &str) -> Option<String> {
    if text.starts_with("module ") && text.ends_with(" exists") {
        let path = text.trim_start_matches("module ").trim_end_matches(" exists").trim();
        if !path.is_empty() {
            return Some(path.to_string());
        }
    }
    None
}

fn extract_depends_on(text: &str) -> Option<(String, String)> {
    if let Some(rest) = text.strip_prefix("crate ") {
        // "crate X depends on Y"
        if let Some(parts) = rest.split_once(" depends on ") {
            let crate_name = parts.0.trim();
            let dep = parts.1.trim();
            if !crate_name.is_empty() && !dep.is_empty() {
                return Some((crate_name.to_string(), dep.to_string()));
            }
        }
    }
    None
}

/// Check if a claim pattern is supported by the repo observations.
pub fn match_claim(
    pattern: &RepoClaimPattern,
    crate_names: &[String],
    src_files: &[String],
    dependencies: &[(String, String)], // (crate_name, dep_name)
) -> bool {
    match pattern {
        RepoClaimPattern::CrateExists { crate_name } => {
            crate_names.iter().any(|n| {
                n.eq_ignore_ascii_case(crate_name)
                    || n.ends_with(&format!("-{}", crate_name))
            })
        }
        RepoClaimPattern::WorkspaceContainsCrate { crate_name } => {
            crate_names.iter().any(|n| {
                n.eq_ignore_ascii_case(crate_name)
                    || n.ends_with(&format!("-{}", crate_name))
            })
        }
        RepoClaimPattern::FileExists { path } => {
            let normalized = path.replace('\\', "/");
            src_files.iter().any(|f| f.replace('\\', "/") == normalized)
        }
        RepoClaimPattern::ModuleExists { module_path } => {
            // Module path like "core::events" maps to events.rs or events/mod.rs
            // Strip crate prefix if present
            let parts: Vec<&str> = module_path.split("::").collect();
            let last = parts.last().copied().unwrap_or(module_path.as_str());
            let module_file = last.to_string() + ".rs";
            let module_dir = last.to_string() + "/mod.rs";
            src_files.iter().any(|f| {
                let normalized = f.replace('\\', "/");
                normalized.ends_with(&module_file) || normalized.ends_with(&module_dir)
            })
        }
        RepoClaimPattern::CrateDependsOn { crate_name, dependency } => {
            dependencies.iter().any(|(cn, dn)| {
                cn.eq_ignore_ascii_case(crate_name) && dn.eq_ignore_ascii_case(dependency)
            })
        }
        RepoClaimPattern::Unsupported => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_exists_claim_supported_by_workspace() {
        let pattern = parse_claim("crate core exists");
        assert_eq!(
            RepoClaimPattern::CrateExists { crate_name: "core".to_string() },
            pattern
        );
        assert!(match_claim(&pattern, &["core".to_string()], &[], &[]));
    }

    #[test]
    fn workspace_contains_claim_supported_by_workspace() {
        let pattern = parse_claim("workspace contains crate memory");
        assert_eq!(
            RepoClaimPattern::WorkspaceContainsCrate { crate_name: "memory".to_string() },
            pattern
        );
        assert!(match_claim(&pattern, &["memory".to_string()], &[], &[]));
    }

    #[test]
    fn file_exists_claim_supported_by_repo() {
        let pattern = parse_claim("file crates/core/src/lib.rs exists");
        assert!(matches!(pattern, RepoClaimPattern::FileExists { .. }));
        assert!(match_claim(
            &pattern,
            &[],
            &["crates/core/src/lib.rs".to_string()],
            &[]
        ));
    }

    #[test]
    fn module_exists_claim_supported_by_src_file() {
        let pattern = parse_claim("module core::events exists");
        assert!(matches!(pattern, RepoClaimPattern::ModuleExists { .. }));
        assert!(match_claim(
            &pattern,
            &[],
            &["crates/core/src/events.rs".to_string()],
            &[]
        ));
    }

    #[test]
    fn dependency_claim_supported_by_cargo_toml() {
        let pattern = parse_claim("crate core depends on serde");
        assert_eq!(
            RepoClaimPattern::CrateDependsOn {
                crate_name: "core".to_string(),
                dependency: "serde".to_string()
            },
            pattern
        );
        assert!(match_claim(
            &pattern,
            &[],
            &[],
            &[("core".to_string(), "serde".to_string())]
        ));
    }

    #[test]
    fn missing_file_claim_marked_missing_in_repo() {
        let pattern = parse_claim("file nonexistent.rs exists");
        assert!(!match_claim(&pattern, &[], &["other.rs".to_string()], &[]));
    }

    #[test]
    fn missing_crate_claim_marked_missing_in_repo() {
        let pattern = parse_claim("crate nonexistent exists");
        assert!(!match_claim(&pattern, &["core".to_string()], &[], &[]));
    }

    #[test]
    fn unsupported_claim_marked_unverifiable() {
        let pattern = parse_claim("the project uses a microservices architecture");
        assert_eq!(RepoClaimPattern::Unsupported, pattern);
    }

    #[test]
    fn unsupported_claim_does_not_fail_report() {
        let pattern = parse_claim("something completely unparseable");
        assert_eq!(RepoClaimPattern::Unsupported, pattern);
        // Unsupported claims should not crash — they just become Unverifiable
        assert!(!match_claim(&pattern, &[], &[], &[]));
    }
}
