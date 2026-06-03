//! Guard tests for workflow action routing.

#[test] fn action_route_crate_does_not_import_tool_executor() {
    let sources = [include_str!("../../workflow/src/workflow_action_route.rs"),
        include_str!("../../workflow/src/workflow_action_route_gate.rs"),
        include_str!("../../workflow/src/workflow_action_route_validation.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor") || l.contains("tool_executor"))); }
}

#[test] fn action_route_crate_does_not_import_policy_engine_for_execution() {
    let sources = [include_str!("../../workflow/src/workflow_action_route.rs"),
        include_str!("../../workflow/src/workflow_action_route_gate.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine") || l.contains("policy_engine"))); }
}

#[test] fn action_route_crate_does_not_import_memory_projection_store() {
    let sources = [include_str!("../../workflow/src/workflow_action_route.rs"),
        include_str!("../../workflow/src/workflow_action_route_gate.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("MemoryStore") || l.contains("memory_store"))); }
}

#[test] fn action_route_crate_does_not_import_trace_append() {
    let sources = [include_str!("../../workflow/src/workflow_action_route.rs"),
        include_str!("../../workflow/src/workflow_action_route_gate.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("openwand_trace"))); }
}

#[test] fn action_route_crate_does_not_import_process_command() {
    let sources = [include_str!("../../workflow/src/workflow_action_route.rs"),
        include_str!("../../workflow/src/workflow_action_route_gate.rs")];
    for src in &sources { assert!(!src.contains("std::process")); }
}

#[test] fn action_route_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_action_routing.rs");
    assert!(!src.contains("std::process::Command"));
    assert!(!src.contains("git "));
}

#[test] fn action_route_app_does_not_execute_tools_directly() {
    let src = include_str!("../src/workflow_action_routing.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn action_route_app_does_not_create_approval_records_directly() {
    let src = include_str!("../src/workflow_action_routing.rs");
    assert!(!src.contains("create_approval"));
    assert!(!src.contains("save_approval"));
}

#[test] fn action_route_app_does_not_append_trace_directly() {
    let src = include_str!("../src/workflow_action_routing.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("openwand_trace")));
}

#[test] fn action_route_app_does_not_write_memory() {
    let src = include_str!("../src/workflow_action_routing.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")));
}

#[test] fn action_route_ui_does_not_expose_approve_reject_retry_resume() {
    let src = include_str!("../src/ui/workflow_action_routing_state.rs");
    assert!(!src.contains("approve_tool"));
    assert!(!src.contains("reject_tool"));
    assert!(!src.contains("retry"));
    assert!(!src.contains("resume"));
}

#[test] fn action_route_does_not_create_tool_result_directly() {
    let src = include_str!("../src/workflow_action_routing.rs");
    assert!(!src.contains("ToolResult"));
    assert!(!src.contains("tool_result"));
}
