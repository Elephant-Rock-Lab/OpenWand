//! Guard tests for workflow manual result reconciliation readiness.

#[test] fn readiness_crate_does_not_import_tool_executor() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result_reconciliation_readiness.rs"),
        include_str!("../../workflow/src/workflow_manual_result_reconciliation_readiness_evaluator.rs"),
        include_str!("../../workflow/src/workflow_manual_result_reconciliation_readiness_validation.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor"))); }
}

#[test] fn readiness_crate_does_not_import_policy_engine() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result_reconciliation_readiness.rs"),
        include_str!("../../workflow/src/workflow_manual_result_reconciliation_readiness_evaluator.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine"))); }
}

#[test] fn readiness_crate_does_not_import_memory() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result_reconciliation_readiness.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("MemoryStore"))); }
}

#[test] fn readiness_crate_does_not_import_trace() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result_reconciliation_readiness.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("openwand_trace"))); }
}

#[test] fn readiness_crate_does_not_import_process() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result_reconciliation_readiness.rs"),
        include_str!("../../workflow/src/workflow_manual_result_reconciliation_readiness_evaluator.rs")];
    for src in &sources { assert!(!src.contains("std::process")); }
}

#[test] fn readiness_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_readiness.rs");
    assert!(!src.contains("std::process::Command"));
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("git status") || l.contains("git diff")));
}

#[test] fn readiness_app_does_not_execute_commands() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_readiness.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn readiness_app_does_not_verify_external_state() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_readiness.rs");
    assert!(!src.contains("verify_shell") && !src.contains("verify_git"));
}

#[test] fn readiness_app_does_not_reconcile() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_readiness.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("evaluate_reconciliation")));
    let fn_lines: Vec<&str> = src.lines().filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn ")).collect();
    assert!(!fn_lines.iter().any(|l| l.contains("reconcile")));
}

#[test] fn readiness_app_does_not_mutate_workflow_state() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_readiness.rs");
    assert!(!src.contains("save_workflow_reconciliation("));
    assert!(!src.contains("save_workflow_run_revision("));
}

#[test] fn readiness_app_does_not_route_actions() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_readiness.rs");
    assert!(!src.contains("route_action"));
}

#[test] fn readiness_app_does_not_resolve_approvals() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_readiness.rs");
    assert!(!src.contains("resolve_approval"));
}

#[test] fn readiness_app_does_not_append_trace() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_readiness.rs");
    assert!(!src.contains(".append("));
}

#[test] fn readiness_app_does_not_write_memory() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_readiness.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")));
}

#[test] fn readiness_app_does_not_write_session_state() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_readiness.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("SessionState")));
}

#[test] fn readiness_ui_does_not_expose_execute_verify_reconcile() {
    let src = include_str!("../src/ui/workflow_manual_result_reconciliation_readiness_state.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("execute")));
    assert!(!fn_lines.iter().any(|l| l.contains("verify")));
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

#[test]
fn readiness_record_serialized_json_has_no_reconciliation_authority() {
    use openwand_workflow::workflow_manual_result_reconciliation_readiness::*;
    use openwand_workflow::workflow_manual_result::WorkflowManualResultId;
    use openwand_workflow::workflow_manual_result_review::WorkflowManualResultReviewId;
    use openwand_workflow::workflow_command_review::WorkflowCommandReviewId;
    use openwand_workflow::workflow_command_composer::WorkflowCommandComposerId;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    let rec = WorkflowManualResultReconciliationReadinessRecord {
        readiness_id: WorkflowManualResultReconciliationReadinessId("wmrrr_g".into()),
        workflow_execution_id: WorkflowExecutionId("wfx_g".into()),
        manual_result_id: WorkflowManualResultId("wmr_g".into()),
        manual_result_review_id: WorkflowManualResultReviewId("wmrr_g".into()),
        command_review_id: WorkflowCommandReviewId("wcrv_g".into()),
        command_composer_id: WorkflowCommandComposerId("wcc_g".into()),
        loop_controller_id: WorkflowLoopControllerId("wlc_g".into()),
        manual_result_review_hash: "rrh".into(), manual_result_hash: "mrh".into(),
        command_review_hash: "crh".into(), command_composer_hash: "cch".into(),
        command_descriptor_hash: "cdh".into(), loop_controller_hash: "lch".into(),
        status: WorkflowManualResultReconciliationReadinessStatus::Ready,
        decision: WorkflowManualResultReconciliationReadinessDecision::Ready { summary: "ok".into() },
        predicates: vec![], reconciliation_preview: None,
        verifies_external_state: false, reconciles_now: false,
        mutates_workflow_state: false, creates_run_revision: false,
        appends_trace: false, writes_memory: false,
        routes_action: false, resolves_approval: false,
        creates_execution_grant: false, execution_allowed_now: false,
        evaluator: "g".into(), evaluated_at: chrono::Utc::now(),
    };
    let json = serde_json::to_string_pretty(&rec).unwrap().to_lowercase();
    assert!(json.contains("\"reconciles_now\": false"));
    assert!(json.contains("\"creates_run_revision\": false"));
    assert!(json.contains("\"verifies_external_state\": false"));
}
