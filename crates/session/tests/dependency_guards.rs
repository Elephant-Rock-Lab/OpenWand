//! Dependency guard: verify session crate's dependency boundaries.
//! Session MUST depend on: core, trace, llm, tools, policy, memory, store (via StoredEvent)
//! Session MUST NOT depend on: mcp-pool, rmcp

use cargo_metadata::MetadataCommand;
use std::path::Path;

#[test]
fn session_no_mcp_pool_dependency() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("session crate should be inside crates/ inside workspace root");

    let meta = MetadataCommand::new()
        .manifest_path(workspace_root.join("Cargo.toml"))
        .exec()
        .expect("cargo metadata should work");

    let session_id = meta
        .packages
        .iter()
        .find(|p| p.name == "openwand-session")
        .expect("openwand-session package should exist")
        .id
        .clone();

    let resolve = meta.resolve.expect("should have resolve");
    let session_node = resolve
        .nodes
        .iter()
        .find(|n| n.id == session_id)
        .expect("session should be in resolve graph");

    let deps: Vec<String> = session_node
        .deps
        .iter()
        .map(|d| {
            meta.packages
                .iter()
                .find(|p| p.id == d.pkg)
                .map(|p| p.name.clone())
                .unwrap_or_default()
        })
        .collect();

    assert!(
        !deps.iter().any(|d| d == "openwand-mcp-pool"),
        "session MUST NOT depend on mcp-pool. Found deps: {deps:?}"
    );
    assert!(
        !deps.iter().any(|d| d == "rmcp"),
        "session MUST NOT depend on rmcp. Found deps: {deps:?}"
    );
}

#[test]
fn session_has_required_dependencies() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("session crate should be inside crates/ inside workspace root");

    let meta = MetadataCommand::new()
        .manifest_path(workspace_root.join("Cargo.toml"))
        .exec()
        .expect("cargo metadata should work");

    let session_id = meta
        .packages
        .iter()
        .find(|p| p.name == "openwand-session")
        .expect("openwand-session package should exist")
        .id
        .clone();

    let resolve = meta.resolve.expect("should have resolve");
    let session_node = resolve
        .nodes
        .iter()
        .find(|n| n.id == session_id)
        .expect("session should be in resolve graph");

    let deps: Vec<String> = session_node
        .deps
        .iter()
        .map(|d| {
            meta.packages
                .iter()
                .find(|p| p.id == d.pkg)
                .map(|p| p.name.clone())
                .unwrap_or_default()
        })
        .collect();

    let required = [
        "openwand-core",
        "openwand-trace",
        "openwand-llm",
        "openwand-tools",
        "openwand-policy",
        "openwand-memory",
        "openwand-store",
    ];

    for req in &required {
        assert!(
            deps.iter().any(|d| d == *req),
            "session MUST depend on {req}. Found deps: {deps:?}"
        );
    }
}
