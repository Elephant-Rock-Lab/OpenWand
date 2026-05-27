//! Dependency guard tests for openwand-trace.
//!
//! Makes the Wave 00 dependency decisions executable.
//! Uses `cargo tree` shell-out — no extra dev-dependencies needed.

use std::process::Command;

/// Crates that openwand-trace must never depend on.
const TRACE_FORBIDDEN: &[&str] = &[
    // Internal crates — trace is generic, must not know about domain
    "openwand-core",
    "openwand-session",
    "openwand-policy",
    "openwand-tools",
    "openwand-store",
    "openwand-llm",
    "openwand-mcp-pool",
    "openwand-memory",
    "openwand-skills",
    "openwand-goals",
    "openwand-content",
    // External crates banned from trace
    "loro",
    "rig-core",
    "rmcp",
    "dioxus",
    "reqwest",
    "axum",
];

fn get_trace_deps() -> Vec<String> {
    let output = Command::new("cargo")
        .args(["tree", "-p", "openwand-trace", "--prefix", "none", "--no-dev-dependencies"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run `cargo tree`");

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .map(|s| {
            // Strip version suffix: "serde v1.0.228" → "serde"
            s.split_whitespace()
                .next()
                .unwrap_or("")
                .to_string()
        })
        .collect()
}

#[test]
fn trace_has_no_forbidden_dependencies() {
    let deps = get_trace_deps();

    for forbidden in TRACE_FORBIDDEN {
        let found = deps.iter().any(|dep| {
            dep == *forbidden || dep.starts_with(&format!("{forbidden}-"))
        });
        assert!(
            !found,
            "openwand-trace must not depend on '{forbidden}'. Found deps: {:?}",
            deps
        );
    }
}

#[test]
fn trace_does_not_depend_on_core() {
    let deps = get_trace_deps();

    let has_core = deps.iter().any(|dep| dep == "openwand-core");
    assert!(
        !has_core,
        "openwand-trace must remain generic — no dependency on openwand-core.\n\
         Found deps: {:?}",
        deps
    );
}
