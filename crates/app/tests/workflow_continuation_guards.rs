//! Guard tests for workflow continuation.

#[test] fn continuation_crate_does_not_import_tool_executor() {
    let sources = [include_str!("../../workflow/src/workflow_continuation.rs"),
        include_str!("../../workflow/src/workflow_next_action_selector.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor"))); }
}

#[test] fn continuation_crate_does_not_import_policy_engine_for_execution() {
    let sources = [include_str!("../../workflow/src/workflow_continuation.rs"),
        include_str!("../../workflow/src/workflow_next_action_selector.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine"))); }
}

#[test] fn continuation_crate_does_not_import_memory_projection_store() {
    let sources = [include_str!("../../workflow/src/workflow_continuation.rs"),
        include_str!("../../workflow/src/workflow_next_action_selector.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("MemoryStore"))); }
}

#[test] fn continuation_crate_does_not_import_trace_append() {
    let sources = [include_str!("../../workflow/src/workflow_continuation.rs"),
        include_str!("../../workflow/src/workflow_next_action_selector.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("openwand_trace"))); }
}

#[test] fn continuation_crate_does_not_import_process_command() {
    let sources = [include_str!("../../workflow/src/workflow_continuation.rs"),
        include_str!("../../workflow/src/workflow_next_action_selector.rs")];
    for src in &sources { assert!(!src.contains("std::process")); }
}

#[test] fn continuation_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_continuation.rs");
    assert!(!src.contains("std::process::Command")); assert!(!src.contains("git "));
}

#[test] fn continuation_app_does_not_route_actions() {
    let src = include_str!("../src/workflow_continuation.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("route_action")));
    assert!(!src.contains("WorkflowActionRouteRequest"));
}

#[test] fn continuation_app_does_not_resolve_approvals() {
    let src = include_str!("../src/workflow_continuation.rs");
    assert!(!src.contains("resolve_approval")); assert!(!src.contains("ApprovalDecision"));
}

#[test] fn continuation_app_does_not_reconcile_outcomes() {
    let src = include_str!("../src/workflow_continuation.rs");
    assert!(!src.contains("evaluate_reconciliation")); assert!(!src.contains("reconcile"));
}

#[test] fn continuation_app_does_not_execute_tools() {
    let src = include_str!("../src/workflow_continuation.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn continuation_app_does_not_append_trace_directly() {
    let src = include_str!("../src/workflow_continuation.rs");
    assert!(!src.contains(".append(")); assert!(!src.contains("AppendTraceEntry"));
}

#[test] fn continuation_app_does_not_write_memory() {
    let src = include_str!("../src/workflow_continuation.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")));
}

#[test] fn continuation_app_does_not_write_session_state_directly() {
    let src = include_str!("../src/workflow_continuation.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("SessionState")));
}

#[test] fn continuation_ui_does_not_expose_route_resolve_reconcile_retry_resume() {
    let src = include_str!("../src/ui/workflow_continuation_state.rs");
    assert!(!src.contains("route_action")); assert!(!src.contains("resolve_approval"));
    assert!(!src.contains("reconcile")); assert!(!src.contains("retry"));
    assert!(!src.contains("resume")); assert!(!src.contains("execute_tool"));
}
