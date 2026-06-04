//! Guard tests for workflow manual result.

#[test] fn manual_result_crate_does_not_import_tool_executor() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result.rs"),
        include_str!("../../workflow/src/workflow_manual_result_validation.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor"))); }
}

#[test] fn manual_result_crate_does_not_import_policy_engine_for_execution() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result.rs"),
        include_str!("../../workflow/src/workflow_manual_result_validation.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine"))); }
}

#[test] fn manual_result_crate_does_not_import_memory_projection_store() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("MemoryStore"))); }
}

#[test] fn manual_result_crate_does_not_import_trace_append() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("openwand_trace"))); }
}

#[test] fn manual_result_crate_does_not_import_process_command() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result.rs"),
        include_str!("../../workflow/src/workflow_manual_result_validation.rs")];
    for src in &sources { assert!(!src.contains("std::process")); }
}

#[test] fn manual_result_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_manual_result.rs");
    assert!(!src.contains("std::process::Command")); assert!(!src.contains("git "));
}

#[test] fn manual_result_app_does_not_execute_commands() {
    let src = include_str!("../src/workflow_manual_result.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn manual_result_app_does_not_verify_shell_or_git_state() {
    let src = include_str!("../src/workflow_manual_result.rs");
    assert!(!src.contains("git status")); assert!(!src.contains("git diff"));
    assert!(!src.contains("git log"));
    let fn_lines: Vec<&str> = src.lines().filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn ")).collect();
    assert!(!fn_lines.iter().any(|l| l.contains("git status") || l.contains("git diff") || l.contains("git log")));
}

#[test] fn manual_result_app_does_not_read_artifact_files() {
    let src = include_str!("../src/workflow_manual_result.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("std::fs::read")));
    assert!(!src.contains("read_artifact")); assert!(!src.contains("read_file"));
}

#[test] fn manual_result_app_does_not_fetch_artifact_urls() {
    let src = include_str!("../src/workflow_manual_result.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("reqwest") || l.contains("http")));
}

#[test] fn manual_result_app_does_not_route_actions() {
    let src = include_str!("../src/workflow_manual_result.rs");
    assert!(!src.contains("route_action")); assert!(!src.contains("evaluate_action_route"));
}

#[test] fn manual_result_app_does_not_resolve_approvals() {
    let src = include_str!("../src/workflow_manual_result.rs");
    assert!(!src.contains("resolve_approval")); assert!(!src.contains("ApprovalDecision"));
}

#[test] fn manual_result_app_does_not_reconcile_outcomes() {
    let src = include_str!("../src/workflow_manual_result.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("evaluate_reconciliation")));
}

#[test] fn manual_result_app_does_not_append_trace_directly() {
    let src = include_str!("../src/workflow_manual_result.rs");
    assert!(!src.contains(".append(")); assert!(!src.contains("AppendTraceEntry"));
}

#[test] fn manual_result_app_does_not_write_memory() {
    let src = include_str!("../src/workflow_manual_result.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")));
}

#[test] fn manual_result_app_does_not_write_session_state_directly() {
    let src = include_str!("../src/workflow_manual_result.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("SessionState")));
}

#[test] fn manual_result_ui_does_not_expose_execute_verify_route_resolve_reconcile() {
    let src = include_str!("../src/ui/workflow_manual_result_state.rs");
    let fn_lines: Vec<&str> = src.lines().filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn ")).collect();
    assert!(!fn_lines.iter().any(|l| l.contains("execute"))); assert!(!fn_lines.iter().any(|l| l.contains("verify")));
    assert!(!fn_lines.iter().any(|l| l.contains("route"))); assert!(!fn_lines.iter().any(|l| l.contains("resolve")));
    assert!(!fn_lines.iter().any(|l| l.contains("reconcile")));
}

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
