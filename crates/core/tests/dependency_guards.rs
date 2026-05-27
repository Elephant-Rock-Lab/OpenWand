//! Dependency guard tests for openwand-core.
//!
//! Makes the Wave 00 dependency decisions executable.
//! Uses `cargo tree` shell-out — no extra dev-dependencies needed.

use std::process::Command;

/// Forbidden crates that openwand-core must never depend on (including transitive).
const CORE_FORBIDDEN: &[&str] = &[
    // Internal crates
    "openwand-trace",
    "openwand-memory",
    "openwand-session",
    "openwand-policy",
    "openwand-tools",
    "openwand-store",
    "openwand-llm",
    "openwand-mcp-pool",
    "openwand-skills",
    "openwand-goals",
    "openwand-content",
    // External crates banned from core
    "loro",
    "rig-core",
    "rmcp",
    "tokio",
    "blake3",
    "uuid",
    "thiserror",
    "async-trait",
    "futures",
    "reqwest",
    "axum",
    "dioxus",
];

/// Allowed direct dependencies for openwand-core.
const CORE_ALLOWED_DIRECT: &[&str] = &[
    "serde",
    "serde_json",
    "chrono",
    "ulid",
];

fn get_all_deps(crate_name: &str, include_dev: bool) -> Vec<String> {
    let mut args = vec!["tree", "-p", crate_name, "--prefix", "none"];
    if !include_dev {
        args.push("--no-dev-dependencies");
    }

    let output = Command::new("cargo")
        .args(&args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run `cargo tree`");

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .map(|s| {
            // "serde v1.0.228" → "serde", "serde_derive v1.0.228 (proc-macro)" → "serde_derive"
            s.to_string()
        })
        .collect()
}

fn get_direct_deps(crate_name: &str) -> Vec<String> {
    let output = Command::new("cargo")
        .args(["tree", "-p", crate_name, "--depth", "1", "--prefix", "none", "--no-dev-dependencies"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run `cargo tree`");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // First line is the crate itself, remaining lines are direct deps
    stdout
        .lines()
        .skip(1) // skip the crate itself
        .filter(|line| !line.is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn core_has_no_forbidden_dependencies() {
    let deps = get_all_deps("openwand-core", false);

    for forbidden in CORE_FORBIDDEN {
        let found = deps.iter().any(|dep| {
            dep == *forbidden || dep.starts_with(&format!("{forbidden}-"))
        });
        assert!(
            !found,
            "openwand-core must not depend on '{forbidden}'.\n\
             All deps: {:?}",
            deps
        );
    }
}

#[test]
fn core_direct_deps_are_locked() {
    let direct = get_direct_deps("openwand-core");

    let unexpected: Vec<&str> = direct
        .iter()
        .filter(|dep| !CORE_ALLOWED_DIRECT.contains(&dep.as_str()))
        .map(|s| s.as_str())
        .collect();

    assert!(
        unexpected.is_empty(),
        "openwand-core has unexpected direct dependencies: {:?}\n\
         Allowed: {:?}\n\
         If these are intentional, add them to CORE_ALLOWED_DIRECT in this test.",
        unexpected, CORE_ALLOWED_DIRECT
    );
}
