//! Guard tests for workflow next-action routing.

#[test] fn next_action_routing_crate_does_not_import_tool_executor() {
    let src = include_str!("../../workflow/src/workflow_next_action_routing_gate.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn next_action_routing_crate_does_not_import_policy_engine_for_execution() {
    let src = include_str!("../../workflow/src/workflow_next_action_routing_gate.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine")));
}

#[test] fn next_action_routing_crate_does_not_import_memory_projection_store() {
    let src = include_str!("../../workflow/src/workflow_next_action_routing_gate.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("MemoryStore")));
}

#[test] fn next_action_routing_crate_does_not_import_trace_append() {
    let src = include_str!("../../workflow/src/workflow_next_action_routing_gate.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("openwand_trace")));
}

#[test] fn next_action_routing_crate_does_not_import_process_command() {
    let src = include_str!("../../workflow/src/workflow_next_action_routing_gate.rs");
    assert!(!src.contains("std::process"));
}

#[test] fn next_action_routing_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_next_action_routing.rs");
    assert!(!src.contains("std::process::Command"));
    assert!(!src.contains("git "));
}

#[test] fn next_action_routing_app_does_not_execute_tools() {
    let src = include_str!("../src/workflow_next_action_routing.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn next_action_routing_app_does_not_resolve_approvals() {
    let src = include_str!("../src/workflow_next_action_routing.rs");
    assert!(!src.contains("resolve_approval"));
    assert!(!src.contains("ApprovalDecision"));
}

#[test] fn next_action_routing_app_does_not_append_trace_directly() {
    let src = include_str!("../src/workflow_next_action_routing.rs");
    assert!(!src.contains(".append("));
    assert!(!src.contains("AppendTraceEntry"));
}

#[test] fn next_action_routing_app_does_not_write_memory() {
    let src = include_str!("../src/workflow_next_action_routing.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")));
}

#[test] fn next_action_routing_app_does_not_write_session_state_directly() {
    let src = include_str!("../src/workflow_next_action_routing.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("SessionState")));
}

// Patch 3: proves adapter delegates to existing path
#[test] fn next_action_routing_app_uses_existing_workflow_action_routing_path() {
    let src = include_str!("../src/workflow_next_action_routing.rs");
    // Must reference the existing routing module
    assert!(src.contains("crate::workflow_action_routing::save_workflow_action_route"),
        "Must use existing route persistence path");
    assert!(src.contains("evaluate_action_route"),
        "Must use existing route evaluation gate");
}

// Patch 3: proves no duplication of route persistence logic
#[test] fn next_action_routing_app_does_not_duplicate_action_route_persistence_logic() {
    let src = include_str!("../src/workflow_next_action_routing.rs");
    let route_persist = include_str!("../src/workflow_action_routing.rs");
    // The next-action routing module should NOT have its own version of
    // "records_dir" or "action_routes_root" — it delegates
    assert!(!src.contains("action_routes_root"), "Must not duplicate route persistence helpers");
    // But it must persist routing records under its own namespace
    assert!(src.contains("workflow_next_action_routing"));
}

#[test] fn next_action_routing_ui_does_not_expose_resolve_reconcile_retry_resume() {
    let src = include_str!("../src/ui/workflow_next_action_routing_state.rs");
    assert!(!src.contains("resolve_approval")); assert!(!src.contains("reconcile"));
    assert!(!src.contains("retry")); assert!(!src.contains("resume"));
    assert!(!src.contains("execute_tool")); assert!(!src.contains("shell"));
}
