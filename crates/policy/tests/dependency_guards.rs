//! Dependency guard tests for openwand-policy.

use std::process::Command;

const POLICY_FORBIDDEN: &[&str] = &[
    // Internal crates policy must NOT depend on
    "openwand-tools",
    "openwand-session",
    "openwand-trace",
    "openwand-memory",
    "openwand-llm",
    "openwand-store",
    "openwand-mcp-pool",
    "openwand-skills",
    "openwand-goals",
    "openwand-content",
    // External crates banned from policy
    "loro",
    "rig-core",
    "rmcp",
    "dioxus",
    "reqwest",
    "axum",
    "ring",
];

fn get_all_deps() -> Vec<String> {
    let output = Command::new("cargo")
        .args([
            "tree",
            "-p",
            "openwand-policy",
            "--prefix",
            "none",
            "--no-dev-dependencies",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run `cargo tree`");

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .map(|s| s.to_string())
        .collect()
}

#[test]
fn policy_no_forbidden_internal_dependencies() {
    let deps = get_all_deps();

    for forbidden in POLICY_FORBIDDEN {
        let found = deps
            .iter()
            .any(|dep| dep == *forbidden || dep.starts_with(&format!("{forbidden}-")));
        assert!(
            !found,
            "openwand-policy must not depend on '{forbidden}'.\nAll deps: {:?}",
            deps
        );
    }
}
