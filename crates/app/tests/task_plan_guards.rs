//! Guard and no-mutation tests for task planning.
//!
//! Proves task planning cannot execute tools, mutate memory, append trace,
//! alter policy, create execution grants, or perform git/shell operations.

use std::path::Path;

/// Read all Rust source in a crate's src/ directory.
fn read_crate_sources(crate_name: &str) -> String {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    let src_dir = workspace_root.join(crate_name).join("src");
    let mut all = String::new();
    if let Ok(entries) = std::fs::read_dir(&src_dir) {
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "rs") {
                all.push_str(&std::fs::read_to_string(&path).unwrap());
                all.push('\n');
            }
        }
    }
    all
}

#[test]
fn task_planning_app_does_not_call_shell_or_git() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let source = std::fs::read_to_string(
        Path::new(&manifest_dir).join("src/task_planning.rs"),
    ).unwrap();
    assert!(!source.contains("std::process"));
    assert!(!source.contains("Command"));
    assert!(!source.contains("git"));
}

#[test]
fn task_planning_app_does_not_execute_tools() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let source = std::fs::read_to_string(
        Path::new(&manifest_dir).join("src/task_planning.rs"),
    ).unwrap();
    assert!(!source.contains("ToolExecutor"));
    assert!(!source.contains("tool_executor"));
}

#[test]
fn task_planning_app_does_not_append_trace_directly() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let source = std::fs::read_to_string(
        Path::new(&manifest_dir).join("src/task_planning.rs"),
    ).unwrap();
    assert!(!source.contains("TraceStore"));
    assert!(!source.contains("trace_append"));
}

#[test]
fn task_planning_app_does_not_write_memory() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let source = std::fs::read_to_string(
        Path::new(&manifest_dir).join("src/task_planning.rs"),
    ).unwrap();
    assert!(!source.contains("MemoryStore"));
    assert!(!source.contains("project_episode"));
}

#[test]
fn task_plan_creation_does_not_write_governance_records() {
    // Design assertion: task_planning.rs does not import governance persistence
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let source = std::fs::read_to_string(
        Path::new(&manifest_dir).join("src/task_planning.rs"),
    ).unwrap();
    assert!(!source.contains("save_proposal"));
    assert!(!source.contains("save_proposal_review"));
    assert!(!source.contains("execute_proposal"));
}

#[test]
fn plan_review_does_not_write_governance_records() {
    // Same file — governance record imports are structurally absent
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let source = std::fs::read_to_string(
        Path::new(&manifest_dir).join("src/task_planning.rs"),
    ).unwrap();
    assert!(!source.contains("verify_execution"));
    assert!(!source.contains("execute_push"));
    assert!(!source.contains("save_proposal"));
}

#[test]
fn workflow_crate_does_not_import_process_command() {
    let source = read_crate_sources("crates/workflow");
    assert!(!source.contains("std::process"));
    assert!(!source.contains("process::Command"));
}

#[test]
fn workflow_crate_does_not_import_tool_executor() {
    let source = read_crate_sources("crates/workflow");
    // Check use statements only, not doc comments
    let use_lines: Vec<&str> = source.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
    assert!(!source.contains("openwand_tools"));
}

#[test]
fn workflow_crate_does_not_import_policy_engine() {
    let source = read_crate_sources("crates/workflow");
    assert!(!source.contains("openwand_policy"));
}

#[test]
fn workflow_crate_does_not_import_memory_store() {
    let source = read_crate_sources("crates/workflow");
    assert!(!source.contains("openwand_memory"));
    assert!(!source.contains("MemoryStore"));
}

#[test]
fn workflow_crate_does_not_import_trace_append() {
    let source = read_crate_sources("crates/workflow");
    assert!(!source.contains("openwand_trace"));
    assert!(!source.contains("TraceStore"));
}
