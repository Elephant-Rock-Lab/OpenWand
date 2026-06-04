//! Guard tests for workflow loop controller.

#[test] fn loop_controller_crate_does_not_import_tool_executor() {
    let sources = [include_str!("../../workflow/src/workflow_loop_controller.rs"),
        include_str!("../../workflow/src/workflow_loop_state.rs"),
        include_str!("../../workflow/src/workflow_loop_recommendation.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor"))); }
}

#[test] fn loop_controller_crate_does_not_import_policy_engine_for_execution() {
    let sources = [include_str!("../../workflow/src/workflow_loop_controller.rs"),
        include_str!("../../workflow/src/workflow_loop_state.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine"))); }
}

#[test] fn loop_controller_crate_does_not_import_memory_projection_store() {
    let sources = [include_str!("../../workflow/src/workflow_loop_controller.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("MemoryStore"))); }
}

#[test] fn loop_controller_crate_does_not_import_trace_append() {
    let sources = [include_str!("../../workflow/src/workflow_loop_controller.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("openwand_trace"))); }
}

#[test] fn loop_controller_crate_does_not_import_process_command() {
    let sources = [include_str!("../../workflow/src/workflow_loop_controller.rs"),
        include_str!("../../workflow/src/workflow_loop_recommendation.rs"),
        include_str!("../../workflow/src/workflow_loop_state.rs")];
    for src in &sources { assert!(!src.contains("std::process")); }
}

#[test] fn loop_controller_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_loop_controller.rs");
    assert!(!src.contains("std::process::Command")); assert!(!src.contains("git "));
}

#[test] fn loop_controller_app_does_not_route_actions() {
    let src = include_str!("../src/workflow_loop_controller.rs");
    assert!(!src.contains("route_action")); assert!(!src.contains("evaluate_action_route"));
}

#[test] fn loop_controller_app_does_not_resolve_approvals() {
    let src = include_str!("../src/workflow_loop_controller.rs");
    assert!(!src.contains("resolve_approval")); assert!(!src.contains("ApprovalDecision"));
}

#[test] fn loop_controller_app_does_not_reconcile_outcomes() {
    let src = include_str!("../src/workflow_loop_controller.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("evaluate_reconciliation")));
}

#[test] fn loop_controller_app_does_not_execute_tools() {
    let src = include_str!("../src/workflow_loop_controller.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn loop_controller_app_does_not_append_trace_directly() {
    let src = include_str!("../src/workflow_loop_controller.rs");
    assert!(!src.contains(".append(")); assert!(!src.contains("AppendTraceEntry"));
}

#[test] fn loop_controller_app_does_not_write_memory() {
    let src = include_str!("../src/workflow_loop_controller.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")));
}

#[test] fn loop_controller_app_does_not_write_session_state_directly() {
    let src = include_str!("../src/workflow_loop_controller.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("SessionState")));
}

#[test] fn loop_controller_ui_does_not_expose_route_resolve_reconcile_retry_resume() {
    let src = include_str!("../src/ui/workflow_loop_controller_state.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("route_action")));
    assert!(!use_lines.iter().any(|l| l.contains("resolve_approval")));
    // Check function definitions, not strings
    let fn_lines: Vec<&str> = src.lines().filter(|l| l.contains("fn ") && l.contains("reconcile")).collect();
    assert!(fn_lines.is_empty());
    assert!(!src.contains("fn retry")); assert!(!src.contains("fn resume"));
    assert!(!src.contains("fn execute_tool"));
}

// Patch 1: dep guard
#[test]
fn workflow_crate_dependency_guard_still_allows_only_6_deps() {
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()
        .join("workflow").join("Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path).unwrap();
    let allowed = ["serde", "serde_json", "blake3", "chrono", "thiserror", "tracing"];
    let mut dep_count = 0u32;
    let mut in_deps = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed == "[dependencies]" { in_deps = true; continue; }
        if trimmed.starts_with('[') { in_deps = false; continue; }
        if !in_deps || trimmed.is_empty() || trimmed.starts_with('#') { continue; }
        let name = trimmed.split('=').next().unwrap().trim();
        assert!(allowed.contains(&name), "Unexpected dependency: {}", name);
        dep_count += 1;
    }
    assert_eq!(6, dep_count, "Workflow crate must have exactly 6 dependencies");
}
