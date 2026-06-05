//! CLI tests for workflow evidence chain inspector.
//! Uses direct function calls instead of binary execution.

use openwand_workflow::workflow_run::*;
use openwand_workflow::workflow_readiness::WorkflowReadinessId;
use openwand_workflow::workflow_proposal::WorkflowProposalId;
use openwand_workflow::workflow_proposal_review::WorkflowProposalReviewId;
use openwand_workflow::plan::TaskPlanId;

fn test_dir() -> std::path::PathBuf {
    tempfile::tempdir().unwrap().into_path()
}

fn save_minimal_run(dir: &std::path::Path, id: &str) {
    let run = WorkflowRunRecord {
        execution_id: WorkflowExecutionId(id.into()),
        readiness_id: WorkflowReadinessId("wfrd_r1".into()),
        proposal_id: WorkflowProposalId("wfp_p1".into()),
        proposal_review_id: WorkflowProposalReviewId("wfr_rev1".into()),
        source_task_plan_id: TaskPlanId("tpl_tp1".into()),
        status: WorkflowRunStatus::Running,
        decision: WorkflowExecutionDecision::RunCreated,
        predicates: vec![],
        run_snapshot: WorkflowRunSnapshot {
            readiness_id: "wfrd_r1".into(),
            proposal_id: "wfp_p1".into(),
            proposal_hash: "h".into(),
            source_task_plan_hash: "h".into(),
            readiness_status_at_execution: "ready".into(),
            proposal_review_decision_at_execution: "approved".into(),
        },
        stages: vec![],
        lifecycle_events: vec![],
        action_requests: vec![],
        abort_snapshot: WorkflowAbortSnapshot {
            abort_notes_available: false,
            rollback_notes_available: false,
            recovery_notes: vec![],
        },
        created_at: chrono::Utc::now(),
        completed_at: None,
    };
    openwand_app::workflow_execution::save_workflow_run(dir, &run).unwrap();
}

#[test]
fn cli_export_packet_writes_expected_packet_file() {
    let dir = test_dir();
    save_minimal_run(&dir, "wfx_cli_pkt");
    let out = dir.join("packet.json");
    openwand_app::workflow_evidence_chain_inspector::export_audit_packet(
        &dir, &WorkflowExecutionId("wfx_cli_pkt".into()), &out,
    ).unwrap();
    assert!(out.exists());
    let content = std::fs::read_to_string(&out).unwrap();
    assert!(content.contains("recorded_evidence"));
    assert!(content.contains("wfx_cli_pkt"));
}

#[test]
fn cli_export_packet_does_not_write_eval_reports_by_default() {
    let dir = test_dir();
    save_minimal_run(&dir, "wfx_no_eval");
    let out = dir.join("custom").join("pkt.json");
    openwand_app::workflow_evidence_chain_inspector::export_audit_packet(
        &dir, &WorkflowExecutionId("wfx_no_eval".into()), &out,
    ).unwrap();
    assert!(!dir.join("workflow_evidence_chain_inspector").exists());
}

#[test]
fn cli_inspect_returns_state_with_weci_id() {
    let dir = test_dir();
    save_minimal_run(&dir, "wfx_inspect");
    let state = openwand_app::workflow_evidence_chain_inspector::assemble_evidence_chain(
        &dir, &WorkflowExecutionId("wfx_inspect".into()), false,
    ).unwrap();
    assert!(state.inspection_id.starts_with("weci_"));
    assert_eq!("wfx_inspect", state.workflow_execution_id);
}

#[test]
fn cli_evidence_chain_does_not_expose_verify_certify_execute_reconcile() {
    let src = include_str!("../src/main.rs");
    let chain_section_start = src.find("workflow-evidence-chain").unwrap_or(0);
    let chain_section = &src[chain_section_start.saturating_sub(100)..];
    assert!(!chain_section.contains("\"verify\""));
    assert!(!chain_section.contains("\"certify\""));
    assert!(!chain_section.contains("\"execute\""));
    assert!(!chain_section.contains("\"reconcile\""));
}
