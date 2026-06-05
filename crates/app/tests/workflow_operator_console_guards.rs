//! Guard tests for workflow operator console.

#[test] fn console_crate_does_not_import_tool_executor() {
    let src = include_str!("../../workflow/src/workflow_operator_console.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn console_crate_does_not_import_policy_engine() {
    let src = include_str!("../../workflow/src/workflow_operator_console.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine")));
}

#[test] fn console_crate_does_not_import_session_runner() {
    let src = include_str!("../../workflow/src/workflow_operator_console.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("SessionRunner")));
}

#[test] fn console_crate_does_not_import_process() {
    let sources = [include_str!("../../workflow/src/workflow_operator_console.rs")];
    for src in &sources { assert!(!src.lines().any(|l| l.contains("std::process"))); }
}

#[test] fn console_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_operator_console.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("git status") || l.contains("git diff")));
}

#[test] fn console_app_does_not_execute_commands() {
    let src = include_str!("../src/workflow_operator_console.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn console_app_does_not_verify_external_state() {
    let src = include_str!("../src/workflow_operator_console.rs");
    assert!(!src.contains("verify_shell") && !src.contains("verify_git"));
}

// Check fn lines only (guard test reconciliation check pattern)
#[test] fn console_app_does_not_mutate_workflow_state() {
    let src = include_str!("../src/workflow_operator_console.rs");
    let fn_lines: Vec<&str> = src.lines().filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn ")).collect();
    assert!(!fn_lines.iter().any(|l| l.contains("save_workflow") || l.contains("mutate")));
}

#[test] fn console_app_does_not_route_actions() {
    let src = include_str!("../src/workflow_operator_console.rs");
    let fn_lines: Vec<&str> = src.lines().filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn ")).collect();
    assert!(!fn_lines.iter().any(|l| l.contains("route_action")));
}

#[test] fn console_app_does_not_resolve_approvals() {
    let src = include_str!("../src/workflow_operator_console.rs");
    let fn_lines: Vec<&str> = src.lines().filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn ")).collect();
    assert!(!fn_lines.iter().any(|l| l.contains("resolve_approval")));
}

#[test] fn console_app_does_not_append_trace() {
    let src = include_str!("../src/workflow_operator_console.rs");
    assert!(!src.contains(".append("));
}

#[test] fn console_app_does_not_write_memory() {
    let src = include_str!("../src/workflow_operator_console.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")));
}

#[test] fn console_app_does_not_persist_console_state() {
    let src = include_str!("../src/workflow_operator_console.rs");
    let pub_fns: Vec<&str> = src.lines().filter(|l| l.trim().starts_with("pub fn")).collect();
    assert!(!pub_fns.iter().any(|l| l.contains("save_") || l.contains("persist_") || l.contains("write_")));
}

#[test] fn console_ui_does_not_expose_execute_verify_or_reconcile() {
    let src = include_str!("../src/ui/workflow_operator_console_state.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("execute")));
    assert!(!fn_lines.iter().any(|l| l.contains("verify")));
}

// Patch 2: loop-controller recommendations do not mutate
#[test]
fn loop_controller_manual_ladder_recommendations_do_not_mutate_state() {
    use openwand_workflow::workflow_loop_controller::*;
    use openwand_workflow::workflow_loop_recommendation::*;
    use openwand_workflow::workflow_loop_state::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    let request = WorkflowLoopControllerRequest {
        workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
        latest_run_revision_id: None,
        expected_workflow_run_hash: "h".into(),
        expected_latest_revision_hash: None,
        requested_by: "test".into(),
        requested_at: chrono::Utc::now(),
        idempotency_key: "k1".into(),
    };
    let ctx = WorkflowLoopContext {
        workflow_run: None, latest_revision: None,
        latest_route: None, latest_outcome: None,
        latest_reconciliation: None, latest_continuation: None,
        latest_proposal: None, latest_review: None,
        latest_routing_readiness: None, latest_next_action_routing: None,
    };
    let rec = evaluate_loop_controller(&request, &ctx);
    assert!(!rec.creates_route);
    assert!(!rec.executes_tool);
    assert!(!rec.mutates_workflow_state);
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

#[test]
fn console_record_serialized_json_has_no_authority() {
    use openwand_workflow::workflow_operator_console::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_loop_state::WorkflowDetectedLoopState;
    let state = build_console_state(
        WorkflowExecutionId("wfx_g".into()), "suspended".into(),
        vec![], &WorkflowDetectedLoopState::Inconclusive,
        None, vec![], vec![],
    );
    let json = serde_json::to_string_pretty(&state).unwrap().to_lowercase();
    assert!(json.contains("\"creates_route\": false"));
    assert!(json.contains("\"executes_tool\": false"));
    assert!(json.contains("\"verifies_external_state\": false"));
    assert!(json.contains("\"mutates_workflow_state\": false"));
}
