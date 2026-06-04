//! Guard tests for workflow routing readiness.

#[test] fn routing_readiness_crate_does_not_import_tool_executor() {
    let sources = [include_str!("../../workflow/src/workflow_routing_readiness.rs"),
        include_str!("../../workflow/src/workflow_routing_readiness_gate.rs"),
        include_str!("../../workflow/src/workflow_next_action_review.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor"))); }
}

#[test] fn routing_readiness_crate_does_not_import_policy_engine_for_execution() {
    let sources = [include_str!("../../workflow/src/workflow_routing_readiness.rs"),
        include_str!("../../workflow/src/workflow_routing_readiness_gate.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine"))); }
}

#[test] fn routing_readiness_crate_does_not_import_memory_projection_store() {
    let sources = [include_str!("../../workflow/src/workflow_routing_readiness.rs"),
        include_str!("../../workflow/src/workflow_routing_readiness_gate.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("MemoryStore"))); }
}

#[test] fn routing_readiness_crate_does_not_import_trace_append() {
    let sources = [include_str!("../../workflow/src/workflow_routing_readiness.rs"),
        include_str!("../../workflow/src/workflow_routing_readiness_gate.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("openwand_trace"))); }
}

#[test] fn routing_readiness_crate_does_not_import_process_command() {
    let sources = [include_str!("../../workflow/src/workflow_routing_readiness.rs"),
        include_str!("../../workflow/src/workflow_routing_readiness_gate.rs"),
        include_str!("../../workflow/src/workflow_next_action_review.rs")];
    for src in &sources { assert!(!src.contains("std::process")); }
}

#[test] fn routing_readiness_app_does_not_call_shell_or_git() {
    let srcs = [include_str!("../src/workflow_routing_readiness.rs"),
        include_str!("../src/workflow_next_action_review.rs")];
    for src in &srcs { assert!(!src.contains("std::process::Command")); assert!(!src.contains("git ")); }
}

#[test] fn routing_readiness_app_does_not_route_actions() {
    let srcs = [include_str!("../src/workflow_routing_readiness.rs"),
        include_str!("../src/workflow_next_action_review.rs")];
    for src in &srcs { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("route_action"))); }
}

#[test] fn routing_readiness_app_does_not_resolve_approvals() {
    let srcs = [include_str!("../src/workflow_routing_readiness.rs"),
        include_str!("../src/workflow_next_action_review.rs")];
    for src in &srcs { assert!(!src.contains("resolve_approval")); assert!(!src.contains("ApprovalDecision")); }
}

#[test] fn routing_readiness_app_does_not_reconcile_outcomes() {
    let srcs = [include_str!("../src/workflow_routing_readiness.rs"),
        include_str!("../src/workflow_next_action_review.rs")];
    for src in &srcs { assert!(!src.contains("evaluate_reconciliation")); }
}

#[test] fn routing_readiness_app_does_not_execute_tools() {
    let srcs = [include_str!("../src/workflow_routing_readiness.rs"),
        include_str!("../src/workflow_next_action_review.rs")];
    for src in &srcs { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor"))); }
}

#[test] fn routing_readiness_app_does_not_append_trace_directly() {
    let srcs = [include_str!("../src/workflow_routing_readiness.rs"),
        include_str!("../src/workflow_next_action_review.rs")];
    for src in &srcs { assert!(!src.contains(".append(")); assert!(!src.contains("AppendTraceEntry")); }
}

#[test] fn routing_readiness_app_does_not_write_memory() {
    let srcs = [include_str!("../src/workflow_routing_readiness.rs"),
        include_str!("../src/workflow_next_action_review.rs")];
    for src in &srcs { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore"))); }
}

#[test] fn routing_readiness_app_does_not_write_session_state_directly() {
    let srcs = [include_str!("../src/workflow_routing_readiness.rs"),
        include_str!("../src/workflow_next_action_review.rs")];
    for src in &srcs { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("SessionState"))); }
}

#[test] fn routing_readiness_ui_does_not_expose_route_resolve_reconcile_retry_resume() {
    let src = include_str!("../src/ui/workflow_routing_readiness_state.rs");
    assert!(!src.contains("route_action")); assert!(!src.contains("resolve_approval"));
    assert!(!src.contains("reconcile")); assert!(!src.contains("retry"));
    assert!(!src.contains("resume")); assert!(!src.contains("execute_tool"));
}

// Patch 3: workflow crate dep guard
#[test]
fn workflow_crate_dependency_guard_still_allows_only_6_deps() {
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap() // crates
        .join("workflow").join("Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path).unwrap();
    let allowed = ["serde", "serde_json", "blake3", "chrono", "thiserror", "tracing"];
    let mut dep_count = 0u32;
    let mut in_deps = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed == "[dependencies]" { in_deps = true; continue; }
        if trimmed.starts_with('[') { in_deps = false; continue; }
        if !in_deps { continue; }
        if trimmed.is_empty() || trimmed.starts_with('#') { continue; }
        let name = trimmed.split('=').next().unwrap().trim();
        assert!(allowed.contains(&name), "Unexpected dependency: {}", name);
        dep_count += 1;
    }
    assert_eq!(6, dep_count, "Workflow crate must have exactly 6 dependencies");
}
