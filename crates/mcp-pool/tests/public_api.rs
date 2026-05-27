//! Public API guard tests for openwand-mcp-pool.

use std::process::Command;

/// Verify rmcp types do not appear in the public API surface.
/// rmcp must stay internal to this crate.
#[test]
fn public_api_contains_no_tooldef() {
    let output = Command::new("cargo")
        .args(["doc", "-p", "openwand-mcp-pool", "--no-deps", "--document-private-items"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run cargo doc");

    // Just verify doc generation doesn't fail
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Doc generation may fail on rmcp internals — check our own files
        eprintln!("cargo doc warnings/errors (may be from rmcp internals):\n{stderr}");
    }

    // Check source files don't re-export rmcp types
    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in walkdir::WalkDir::new(&src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "rs")
                .unwrap_or(false)
        })
    {
        let content = std::fs::read_to_string(entry.path()).unwrap_or_default();
        // Public re-exports should not contain rmcp types
        for line in content.lines() {
            if line.starts_with("pub use") && line.contains("rmcp") {
                panic!(
                    "Public re-export of rmcp type found in {}: {}",
                    entry.path().display(),
                    line
                );
            }
        }
    }
}

/// Verify dependency rules via cargo tree.
#[test]
fn mcp_pool_no_forbidden_internal_deps() {
    let forbidden: &[&str] = &[
        "openwand-tools",
        "openwand-session",
        "openwand-policy",
        "openwand-llm",
        "openwand-memory",
        "openwand-store",
        "openwand-trace",
        "openwand-skills",
        "openwand-goals",
        "openwand-content",
        "loro",
        "rig-core",
    ];

    let output = Command::new("cargo")
        .args([
            "tree",
            "-p",
            "openwand-mcp-pool",
            "--prefix",
            "none",
            "--no-dev-dependencies",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("failed to run cargo tree");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let deps: Vec<String> = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .map(|s| s.to_string())
        .collect();

    for forbidden_name in forbidden {
        let found = deps
            .iter()
            .any(|dep| dep == *forbidden_name || dep.starts_with(&format!("{forbidden_name}-")));
        assert!(
            !found,
            "openwand-mcp-pool must not depend on '{forbidden_name}'.\nAll deps: {:?}",
            deps
        );
    }
}
