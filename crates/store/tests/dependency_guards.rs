//! Dependency guard: verify store crate's dependency boundaries.
//! Store MUST NOT depend on session, policy, tools, llm, memory, loro, rig, rmcp.

use cargo_metadata::MetadataCommand;
use std::path::Path;

#[test]
fn store_forbidden_dependencies() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("store crate should be inside crates/ inside workspace root");

    let meta = MetadataCommand::new()
        .manifest_path(workspace_root.join("Cargo.toml"))
        .exec()
        .expect("cargo metadata should work");

    let store_id = meta
        .packages
        .iter()
        .find(|p| p.name == "openwand-store")
        .expect("openwand-store package should exist")
        .id
        .clone();

    let resolve = meta.resolve.expect("should have resolve");
    let store_node = resolve
        .nodes
        .iter()
        .find(|n| n.id == store_id)
        .expect("store should be in resolve graph");

    let deps: Vec<String> = store_node
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

    let forbidden = [
        "openwand-session",
        "openwand-policy",
        "openwand-tools",
        "openwand-llm",
        "openwand-memory",
        "loro",
        "rig-core",
        "rmcp",
    ];

    for name in &forbidden {
        assert!(
            !deps.iter().any(|d| d == *name),
            "store MUST NOT depend on {name}. Found deps: {deps:?}"
        );
    }
}

#[test]
fn store_required_dependencies() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("store crate should be inside crates/ inside workspace root");

    let meta = MetadataCommand::new()
        .manifest_path(workspace_root.join("Cargo.toml"))
        .exec()
        .expect("cargo metadata should work");

    let store_id = meta
        .packages
        .iter()
        .find(|p| p.name == "openwand-store")
        .expect("openwand-store package should exist")
        .id
        .clone();

    let resolve = meta.resolve.expect("should have resolve");
    let store_node = resolve
        .nodes
        .iter()
        .find(|n| n.id == store_id)
        .expect("store should be in resolve graph");

    let deps: Vec<String> = store_node
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

    let required = ["openwand-core", "openwand-trace"];

    for name in &required {
        assert!(
            deps.iter().any(|d| d == *name),
            "store MUST depend on {name}. Found deps: {deps:?}"
        );
    }
}
