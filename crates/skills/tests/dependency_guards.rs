//! Dependency guard tests for openwand-skills crate.
//!
//! Proves the skills crate remains a leaf crate with only
//! manifest-related dependencies.

use std::path::Path;

/// Read all Rust source files in the skills crate and check for forbidden imports.
fn read_all_sources() -> String {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let src_dir = Path::new(&crate_dir).join("src");
    let mut all_source = String::new();
    for entry in std::fs::read_dir(src_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "rs") {
            all_source.push_str(&std::fs::read_to_string(&path).unwrap());
            all_source.push('\n');
        }
    }
    all_source
}

/// Check that a forbidden dependency pattern does not appear in source.
fn assert_no_import(source: &str, forbidden: &[&str], reason: &str) {
    for pattern in forbidden {
        let patterns = [
            format!("use {pattern}"),
            format!("use {pattern}::"),
            format!("extern crate {pattern}"),
        ];
        for p in &patterns {
            assert!(
                !source.contains(p.as_str()),
                "Skills crate must not import {reason}: found '{p}'"
            );
        }
    }
}

#[test]
fn skills_crate_does_not_import_tool_executor() {
    let source = read_all_sources();
    assert_no_import(&source, &["openwand_tools", "openwand_session", "openwand_policy"], "tool executor");
}

#[test]
fn skills_crate_does_not_import_policy_engine() {
    let source = read_all_sources();
    assert_no_import(&source, &["openwand_policy"], "policy engine");
}

#[test]
fn skills_crate_does_not_import_memory_projection_store() {
    let source = read_all_sources();
    assert_no_import(&source, &["openwand_memory"], "memory projection/store");
}

#[test]
fn skills_crate_does_not_import_trace_append() {
    let source = read_all_sources();
    assert_no_import(&source, &["openwand_trace", "openwand_store"], "trace append");
}

#[test]
fn skills_crate_dependency_guard_allows_only_manifest_dependencies() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let cargo_toml = std::fs::read_to_string(Path::new(&crate_dir).join("Cargo.toml")).unwrap();

    let allowed = [
        "serde",
        "serde_json",
        "toml",
        "thiserror",
        "tracing",
        "openwand-skills", // self-reference in package name
    ];

    // Check [dependencies] section
    let in_deps = cargo_toml.contains("[dependencies]");
    assert!(in_deps, "Cargo.toml must have [dependencies]");

    // Parse dependency lines (simple check)
    let deps_start = cargo_toml.find("[dependencies]").unwrap();
    let deps_end = cargo_toml
        .find("\n[")
        .map(|p| if p > deps_start { p } else { cargo_toml.len() })
        .unwrap_or(cargo_toml.len());
    let deps_section = &cargo_toml[deps_start..deps_end];

    for line in deps_section.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') || line.contains("No dependency on") {
            continue;
        }
        let dep_name = line.split('=').next().unwrap_or("").trim();
        if dep_name.is_empty() {
            continue;
        }
        assert!(
            allowed.contains(&dep_name),
            "Skills crate has forbidden dependency: '{dep_name}'\nAllowed: {allowed:?}"
        );
    }
}
