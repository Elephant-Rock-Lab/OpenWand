//! Guard tests for workflow manual result review.

#[test] fn review_crate_does_not_import_tool_executor() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result_review.rs"),
        include_str!("../../workflow/src/workflow_manual_result_review_validation.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor"))); }
}

#[test] fn review_crate_does_not_import_policy_engine_for_execution() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result_review.rs"),
        include_str!("../../workflow/src/workflow_manual_result_review_validation.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("PolicyEngine"))); }
}

#[test] fn review_crate_does_not_import_memory_projection_store() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result_review.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("MemoryStore"))); }
}

#[test] fn review_crate_does_not_import_trace_append() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result_review.rs")];
    for src in &sources { let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("TraceStore") || l.contains("openwand_trace"))); }
}

#[test] fn review_crate_does_not_import_process_command() {
    let sources = [include_str!("../../workflow/src/workflow_manual_result_review.rs"),
        include_str!("../../workflow/src/workflow_manual_result_review_validation.rs")];
    for src in &sources { assert!(!src.contains("std::process")); }
}

#[test] fn review_app_does_not_call_shell_or_git() {
    let src = include_str!("../src/workflow_manual_result_review.rs");
    assert!(!src.contains("std::process::Command"));
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("git status") || l.contains("git diff") || l.contains("git log")));
}

#[test] fn review_app_does_not_execute_commands() {
    let src = include_str!("../src/workflow_manual_result_review.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("ToolExecutor")));
}

#[test] fn review_app_does_not_verify_external_state() {
    let src = include_str!("../src/workflow_manual_result_review.rs");
    assert!(!src.contains("verify_shell") && !src.contains("verify_git"));
    assert!(!src.contains("check_artifact") && !src.contains("read_artifact"));
}

#[test] fn review_app_does_not_reconcile_workflow_state() {
    let src = include_str!("../src/workflow_manual_result_review.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("evaluate_reconciliation")));
    let fn_lines: Vec<&str> = src.lines().filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn ")).collect();
    assert!(!fn_lines.iter().any(|l| l.contains("reconcile")));
}

#[test] fn review_app_does_not_mutate_manual_result_records() {
    let src = include_str!("../src/workflow_manual_result_review.rs");
    assert!(!src.contains("save_manual_result("));
    assert!(!src.contains("update_manual_result"));
}

#[test] fn review_app_does_not_route_actions() {
    let src = include_str!("../src/workflow_manual_result_review.rs");
    assert!(!src.contains("route_action")); assert!(!src.contains("evaluate_action_route"));
}

#[test] fn review_app_does_not_resolve_approvals() {
    let src = include_str!("../src/workflow_manual_result_review.rs");
    assert!(!src.contains("resolve_approval")); assert!(!src.contains("ApprovalDecision"));
}

#[test] fn review_app_does_not_append_trace_directly() {
    let src = include_str!("../src/workflow_manual_result_review.rs");
    assert!(!src.contains(".append(")); assert!(!src.contains("AppendTraceEntry"));
}

#[test] fn review_app_does_not_write_memory() {
    let src = include_str!("../src/workflow_manual_result_review.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("memory") || l.contains("MemoryStore")));
}

#[test] fn review_app_does_not_write_session_state_directly() {
    let src = include_str!("../src/workflow_manual_result_review.rs");
    let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
    assert!(!use_lines.iter().any(|l| l.contains("SessionState")));
}

#[test] fn review_ui_does_not_expose_execute_verify_reconcile_resolve() {
    let src = include_str!("../src/ui/workflow_manual_result_review_state.rs");
    let fn_lines: Vec<&str> = src.lines()
        .filter(|l| l.trim().starts_with("pub fn") || l.trim().starts_with("fn "))
        .collect();
    assert!(!fn_lines.iter().any(|l| l.contains("execute")));
    assert!(!fn_lines.iter().any(|l| l.contains("verify")));
    assert!(!fn_lines.iter().any(|l| l.contains("reconcile")));
    assert!(!fn_lines.iter().any(|l| l.contains("resolve")));
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

// Patch 3: serialized JSON guard — acceptance semantics
#[test]
fn review_serialized_json_contains_no_verification_or_reconciliation_claims() {
    use openwand_workflow::workflow_manual_result_review::*;
    use openwand_workflow::workflow_manual_result::WorkflowManualResultId;
    use openwand_workflow::workflow_command_review::WorkflowCommandReviewId;
    use openwand_workflow::workflow_command_composer::WorkflowCommandComposerId;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    let rec = WorkflowManualResultReview {
        review_id: WorkflowManualResultReviewId("wmrr_g".into()),
        workflow_execution_id: WorkflowExecutionId("wfx_g".into()),
        manual_result_id: WorkflowManualResultId("wmr_g".into()),
        command_review_id: WorkflowCommandReviewId("wcrv_g".into()),
        command_composer_id: WorkflowCommandComposerId("wcc_g".into()),
        loop_controller_id: WorkflowLoopControllerId("wlc_g".into()),
        manual_result_hash: "mrh".into(), command_review_hash: "crh".into(),
        command_composer_hash: "cch".into(), command_descriptor_hash: "cdh".into(),
        loop_controller_hash: "lch".into(),
        decision: WorkflowManualResultReviewDecision::Accepted,
        reviewer: "g".into(), rationale: "g".into(), feedback: None,
        acceptance_snapshot: WorkflowManualResultReviewAcceptanceSnapshot {
            accepts_reported_evidence: true,
            verifies_external_state: false,
            reconciles_workflow_state: false,
            result_verified_by_openwand: false,
        },
        verifies_external_state: false, reconciles_workflow_state: false,
        mutates_workflow_state: false, executes_command: false,
        invokes_shell: false, invokes_git: false,
        routes_action: false, resolves_approval: false,
        appends_trace: false, writes_memory: false,
        creates_execution_grant: false, execution_allowed_now: false,
        reviewed_at: chrono::Utc::now(),
    };
    let json = serde_json::to_string_pretty(&rec).unwrap().to_lowercase();
    // The acceptance_snapshot should show accepts_reported_evidence: true
    // but verifies_external_state and reconciles_workflow_state must be false
    assert!(json.contains("accepts_reported_evidence"));
    assert!(json.contains("\"verifies_external_state\": false"));
    assert!(json.contains("\"reconciles_workflow_state\": false"));
}
