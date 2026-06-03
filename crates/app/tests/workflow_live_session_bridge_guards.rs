//! Guard tests for live session bridge.

#[test] fn live_bridge_does_not_import_llm_client() {
    let src = include_str!("../src/workflow_session_bridge.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("LlmClient") || l.contains("llm_client")));
}

#[test] fn live_bridge_does_not_import_tool_executor_execute() {
    let src = include_str!("../src/workflow_session_bridge.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
    // Also verify no .execute( calls
    assert!(!src.contains(".execute("));
}

#[test] fn live_bridge_does_not_import_policy_engine_for_direct_eval() {
    let src = include_str!("../src/workflow_session_bridge.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine")));
}

#[test] fn live_bridge_does_not_import_trace_append() {
    let src = include_str!("../src/workflow_session_bridge.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("AppendTraceEntry")));
    assert!(!src.contains(".append("));
}

#[test] fn live_bridge_does_not_import_memory_projection_store() {
    let src = include_str!("../src/workflow_session_bridge.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("MemoryStore") || l.contains("MemoryReadStore")));
}

#[test] fn live_bridge_does_not_import_process_command() {
    let src = include_str!("../src/workflow_session_bridge.rs");
    assert!(!src.contains("std::process"));
}

#[test] fn live_bridge_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_session_bridge.rs");
    assert!(!src.contains("std::process::Command"));
    assert!(!src.contains("git "));
}

#[test] fn live_bridge_does_not_construct_tool_result_directly() {
    let src = include_str!("../src/workflow_session_bridge.rs");
    assert!(!src.contains("ToolResult"));
}

#[test] fn live_bridge_does_not_construct_approval_record_directly() {
    let src = include_str!("../src/workflow_session_bridge.rs");
    assert!(!src.contains("ApprovalDecision"));
    assert!(!src.contains("resolve_approval"));
}

#[test] fn live_bridge_does_not_construct_trace_event_directly() {
    let src = include_str!("../src/workflow_session_bridge.rs");
    assert!(!src.contains("StoredEvent::"));
    assert!(!src.contains("TraceEvent"));
}

#[test] fn live_bridge_test_constructor_is_test_only() {
    // Patch 1: from_harness is documented as test-only.
    // Production code must use LiveSessionBridge::new().
    // This guard verifies the module doc comment states this.
    let src = include_str!("../src/workflow_session_bridge.rs");
    assert!(src.contains("Production code must use `LiveSessionBridge::new()`"),
        "from_harness must be documented as test-only");
}
