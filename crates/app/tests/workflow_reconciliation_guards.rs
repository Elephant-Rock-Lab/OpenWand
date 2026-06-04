//! Guard tests for workflow reconciliation.

#[test] fn reconciliation_crate_does_not_import_tool_executor() {
    let sources = [include_str!("../../workflow/src/workflow_reconciliation.rs"),
        include_str!("../../workflow/src/workflow_reconciliation_gate.rs"),
        include_str!("../../workflow/src/workflow_stage_progression.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor"))); }
}

#[test] fn reconciliation_crate_does_not_import_policy_engine_for_execution() {
    let sources = [include_str!("../../workflow/src/workflow_reconciliation.rs"),
        include_str!("../../workflow/src/workflow_reconciliation_gate.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine"))); }
}

#[test] fn reconciliation_crate_does_not_import_memory_projection_store() {
    let sources = [include_str!("../../workflow/src/workflow_reconciliation.rs"),
        include_str!("../../workflow/src/workflow_reconciliation_gate.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("MemoryStore"))); }
}

#[test] fn reconciliation_crate_does_not_import_trace_append() {
    let sources = [include_str!("../../workflow/src/workflow_reconciliation.rs"),
        include_str!("../../workflow/src/workflow_reconciliation_gate.rs"),
        include_str!("../../workflow/src/workflow_stage_progression.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("openwand_trace"))); }
}

#[test] fn reconciliation_crate_does_not_import_process_command() {
    let sources = [include_str!("../../workflow/src/workflow_reconciliation.rs"),
        include_str!("../../workflow/src/workflow_reconciliation_gate.rs"),
        include_str!("../../workflow/src/workflow_stage_progression.rs")];
    for src in &sources { assert!(!src.contains("std::process")); }
}

#[test] fn reconciliation_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_reconciliation.rs");
    assert!(!src.contains("std::process::Command")); assert!(!src.contains("git "));
}

#[test] fn reconciliation_app_does_not_route_actions() {
    let src = include_str!("../src/workflow_reconciliation.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("route_action")));
    assert!(!src.contains("WorkflowActionRouteRequest"));
}

#[test] fn reconciliation_app_does_not_resolve_approvals() {
    let src = include_str!("../src/workflow_reconciliation.rs");
    assert!(!src.contains("resolve_approval")); assert!(!src.contains("ApprovalDecision"));
}

#[test] fn reconciliation_app_does_not_execute_tools() {
    let src = include_str!("../src/workflow_reconciliation.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn reconciliation_app_does_not_append_trace_directly() {
    let src = include_str!("../src/workflow_reconciliation.rs");
    assert!(!src.contains(".append(")); assert!(!src.contains("AppendTraceEntry"));
}

#[test] fn reconciliation_app_does_not_write_memory() {
    let src = include_str!("../src/workflow_reconciliation.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")));
}

#[test] fn reconciliation_app_does_not_write_session_state_directly() {
    let src = include_str!("../src/workflow_reconciliation.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("SessionState")));
}

#[test] fn reconciliation_ui_does_not_expose_route_resolve_retry_resume() {
    let src = include_str!("../src/ui/workflow_reconciliation_state.rs");
    assert!(!src.contains("route_action")); assert!(!src.contains("resolve_approval"));
    assert!(!src.contains("retry")); assert!(!src.contains("resume"));
    assert!(!src.contains("execute_tool")); assert!(!src.contains("run_tool"));
}
