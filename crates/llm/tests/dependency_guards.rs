//! Dependency guard tests for openwand-llm.

use std::process::Command;

const LLM_FORBIDDEN: &[&str] = &[
    // Internal crates LLM must NOT depend on
    "openwand-session",
    "openwand-trace",
    "openwand-memory",
    "openwand-policy",
    "openwand-tools",
    "openwand-mcp-pool",
    "openwand-store",
    "openwand-skills",
    "openwand-goals",
    "openwand-content",
    // External crates banned from LLM
    "loro",
    "rmcp",
    "dioxus",
    "axum",
    "reqwest",
];

fn get_all_deps() -> Vec<String> {
    let output = Command::new("cargo")
        .args([
            "tree",
            "-p",
            "openwand-llm",
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
fn llm_no_forbidden_internal_dependencies() {
    let deps = get_all_deps();

    for forbidden in LLM_FORBIDDEN {
        let found = deps
            .iter()
            .any(|dep| dep == *forbidden || dep.starts_with(&format!("{forbidden}-")));
        assert!(
            !found,
            "openwand-llm must not depend on '{forbidden}'.\nAll deps: {:?}",
            deps
        );
    }
}

#[test]
fn llm_no_rig_types_escape() {
    // Verify rig-core is NOT a dependency in commit 12.
    // When rig adapter is added later, this test should be updated to
    // verify rig types don't appear in public API instead.
    let deps = get_all_deps();
    let has_rig = deps.iter().any(|dep| dep == "rig-core" || dep.starts_with("rig-"));
    assert!(
        !has_rig,
        "openwand-llm should not depend on rig-core yet (commit 12 DTOs only).\n\
         If rig adapter was added, update this test to verify public API instead.\n\
         All deps: {:?}",
        deps
    );
}
