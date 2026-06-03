//! Guard tests for workflow action outcome.

#[test] fn outcome_crate_does_not_import_tool_executor() {
    let sources = [include_str!("../../workflow/src/workflow_action_outcome.rs"),
        include_str!("../../workflow/src/workflow_action_outcome_gate.rs"),
        include_str!("../../workflow/src/workflow_action_outcome_validation.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor"))); }
}

#[test] fn outcome_crate_does_not_import_policy_engine_for_execution() {
    let sources = [include_str!("../../workflow/src/workflow_action_outcome.rs"),
        include_str!("../../workflow/src/workflow_action_outcome_gate.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine"))); }
}

#[test] fn outcome_crate_does_not_import_memory_projection_store() {
    let src = include_str!("../../workflow/src/workflow_action_outcome.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("MemoryStore")));
}

#[test] fn outcome_crate_does_not_import_trace_append() {
    let src = include_str!("../../workflow/src/workflow_action_outcome.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("openwand_trace")));
}

#[test] fn outcome_crate_does_not_import_process_command() {
    let sources = [include_str!("../../workflow/src/workflow_action_outcome.rs"),
        include_str!("../../workflow/src/workflow_action_outcome_gate.rs")];
    for src in &sources { assert!(!src.contains("std::process")); }
}

#[test] fn outcome_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_action_outcome.rs");
    assert!(!src.contains("std::process::Command")); assert!(!src.contains("git "));
}

#[test] fn outcome_app_does_not_execute_tools_directly() {
    let src = include_str!("../src/workflow_action_outcome.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn outcome_app_does_not_create_approval_records_directly() {
    let src = include_str!("../src/workflow_action_outcome.rs");
    assert!(!src.contains("create_approval")); assert!(!src.contains("save_approval"));
}

#[test] fn outcome_app_does_not_mutate_pending_approval_state_directly() {
    let src = include_str!("../src/workflow_action_outcome.rs");
    assert!(!src.contains("pending_approval.lock")); assert!(!src.contains("pending_approval.take"));
}

#[test] fn outcome_app_does_not_append_trace_directly() {
    let src = include_str!("../src/workflow_action_outcome.rs");
    assert!(!src.contains(".append(")); assert!(!src.contains("AppendTraceEntry"));
}

#[test] fn outcome_app_does_not_write_memory() {
    let src = include_str!("../src/workflow_action_outcome.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")));
}

#[test] fn outcome_ui_does_not_expose_direct_approval_or_tool_execution() {
    let src = include_str!("../src/ui/workflow_action_outcome_state.rs");
    assert!(!src.contains("approve_direct")); assert!(!src.contains("reject_direct"));
    assert!(!src.contains("execute_tool")); assert!(!src.contains("run_tool"));
}
