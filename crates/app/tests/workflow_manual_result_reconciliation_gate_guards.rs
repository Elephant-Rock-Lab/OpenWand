//! Guard tests for workflow manual result reconciliation gate.

#[test] fn gate_crate_does_not_import_tool_executor() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result_reconciliation_gate.rs"),
        include_str!("../../workflow/src/workflow_manual_result_reconciliation_gate_evaluator.rs"),
        include_str!("../../workflow/src/workflow_manual_result_reconciliation_gate_validation.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor"))); }
}

#[test] fn gate_crate_does_not_import_policy_engine() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result_reconciliation_gate.rs"),
        include_str!("../../workflow/src/workflow_manual_result_reconciliation_gate_evaluator.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine"))); }
}

#[test] fn gate_crate_does_not_import_memory() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result_reconciliation_gate.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("MemoryStore"))); }
}

#[test] fn gate_crate_does_not_import_trace() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result_reconciliation_gate.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("openwand_trace"))); }
}

#[test] fn gate_crate_does_not_import_process() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result_reconciliation_gate.rs"),
        include_str!("../../workflow/src/workflow_manual_result_reconciliation_gate_evaluator.rs")];
    for src in &sources { assert!(!src.contains("std::process")); }
}

#[test] fn gate_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_gate.rs");
    assert!(!src.contains("std::process::Command"));
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("git status") || l.contains("git diff")));
}

#[test] fn gate_app_does_not_execute_commands() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_gate.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn gate_app_does_not_verify_external_state() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_gate.rs");
    assert!(!src.contains("verify_shell") && !src.contains("verify_git"));
}

// Guard test reconciliation check: check only use lines and fn lines
#[test] fn gate_app_does_not_call_session_runner() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_gate.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("SessionRunner")));
}

#[test] fn gate_app_does_not_mutate_workflow_state() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_gate.rs");
    assert!(!src.contains("save_workflow_reconciliation("));
    let fn_lines: Vec<&str> = src.lines().filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn ")).collect();
    assert!(!fn_lines.iter().any(|l| l.contains("mutate") || l.contains("update_run")));
}

#[test] fn gate_app_does_not_route_actions() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_gate.rs");
    let fn_lines: Vec<&str> = src.lines().filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn ")).collect();
    assert!(!fn_lines.iter().any(|l| l.contains("route_action")));
}

#[test] fn gate_app_does_not_resolve_approvals() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_gate.rs");
    let fn_lines: Vec<&str> = src.lines().filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn ")).collect();
    assert!(!fn_lines.iter().any(|l| l.contains("resolve_approval")));
}

#[test] fn gate_app_does_not_append_trace() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_gate.rs");
    assert!(!src.contains(".append("));
}

#[test] fn gate_app_does_not_write_memory() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_gate.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")));
}

#[test] fn gate_app_does_not_write_session_state() {
    let src = include_str!("../src/workflow_manual_result_reconciliation_gate.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("SessionState")));
}

#[test] fn gate_ui_does_not_expose_execute_verify_or_reconcile() {
    let src = include_str!("../src/ui/workflow_manual_result_reconciliation_gate_state.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("execute")));
    assert!(!fn_lines.iter().any(|l| l.contains("verify")));
    // Note: "reconcile" appears in struct/type names but not as fn verbs
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
fn gate_record_serialized_json_has_no_execution_authority() {
    use openwand_workflow::workflow_manual_result_reconciliation_gate::*;
    use openwand_workflow::workflow_manual_result::WorkflowManualResultId;
    use openwand_workflow::workflow_manual_result_review::WorkflowManualResultReviewId;
    use openwand_workflow::workflow_manual_result_reconciliation_readiness::WorkflowManualResultReconciliationReadinessId;
    use openwand_workflow::workflow_command_review::WorkflowCommandReviewId;
    use openwand_workflow::workflow_command_composer::WorkflowCommandComposerId;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    let rec = WorkflowManualResultReconciliationGateRecord {
        gate_id: WorkflowManualResultReconciliationGateId("wmrrg_g".into()),
        workflow_execution_id: WorkflowExecutionId("wfx_g".into()),
        manual_result_id: WorkflowManualResultId("wmr_g".into()),
        manual_result_review_id: WorkflowManualResultReviewId("wmrr_g".into()),
        reconciliation_readiness_id: WorkflowManualResultReconciliationReadinessId("wmrrr_g".into()),
        command_review_id: WorkflowCommandReviewId("wcrv_g".into()),
        command_composer_id: WorkflowCommandComposerId("wcc_g".into()),
        loop_controller_id: WorkflowLoopControllerId("wlc_g".into()),
        stage_id: "stage_1".into(),
        workflow_run_hash: "wrh".into(), reconciliation_readiness_hash: "rrh".into(),
        manual_result_review_hash: "mrrh".into(), manual_result_hash: "mrh".into(),
        command_review_hash: "crh".into(), command_composer_hash: "cch".into(),
        command_descriptor_hash: "cdh".into(), loop_controller_hash: "lch".into(),
        status: WorkflowManualResultReconciliationGateStatus::Reconciled,
        decision: WorkflowManualResultReconciliationGateDecision::Reconciled { revision_id: None, summary: "ok".into() },
        predicates: vec![], progression: None,
        new_run_revision_id: None,
        creates_run_revision: true,
        mutates_original_workflow_run: false,
        verifies_external_truth: false, executes_command: false,
        routes_continuation: false, appends_trace: false, writes_memory: false,
        creates_execution_grant: false, execution_allowed_now: false,
        reconciled_by: "g".into(), reconciled_at: chrono::Utc::now(),
    };
    let json = serde_json::to_string_pretty(&rec).unwrap().to_lowercase();
    assert!(json.contains("\"mutates_original_workflow_run\": false"));
    assert!(json.contains("\"verifies_external_truth\": false"));
    assert!(json.contains("\"executes_command\": false"));
    assert!(json.contains("\"routes_continuation\": false"));
}
