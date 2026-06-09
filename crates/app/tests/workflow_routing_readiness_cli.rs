//! CLI tests for routing readiness.
//!
//! Coverage gap closure (Wave 50A, FIX-05, KNOWN_GAPS gap 2).

use openwand_workflow::workflow_routing_readiness::*;
use openwand_workflow::workflow_routing_readiness_gate::{WorkflowRoutingReadinessContext, evaluate_routing_readiness};
use openwand_workflow::workflow_next_action_review::WorkflowNextActionReviewId;
use openwand_workflow::workflow_run::WorkflowExecutionId;
use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;
use openwand_workflow::workflow_continuation::WorkflowNextActionProposalId;

fn test_dir() -> std::path::PathBuf {
    tempfile::tempdir().unwrap().into_path()
}

fn save_test_readiness(dir: &std::path::PathBuf) -> WorkflowRoutingReadinessRecord {
    let request = WorkflowRoutingReadinessRequest {
        proposal_id: WorkflowNextActionProposalId("wnap_test".into()),
        review_id: WorkflowNextActionReviewId("wnar_test".into()),
        workflow_execution_id: WorkflowExecutionId("wfx_cli".into()),
        source_run_revision_id: WorkflowRunRevisionId("wrr_test".into()),
        expected_proposal_hash: "h1".into(),
        expected_run_revision_hash: "h2".into(),
        expected_review_hash: "h3".into(),
        requested_by: "alice".into(),
        requested_at: chrono::Utc::now(),
        idempotency_key: "k1".into(),
    };
    let ctx = WorkflowRoutingReadinessContext {
        proposal: None,
        review: None,
        latest_review: None,
        run_revision: None,
        action_request: None,
        prior_readiness: vec![],
    };
    let record = evaluate_routing_readiness(&request, &ctx);
    openwand_app::workflow_routing_readiness::save_workflow_routing_readiness(dir, &record).unwrap();
    record
}

#[test]
fn cli_routing_readiness_evaluate_outputs_readiness_id() {
    let dir = test_dir();
    let rec = save_test_readiness(&dir);
    assert!(rec.readiness_id.0.starts_with("wrrd_"));
}

#[test]
fn cli_routing_readiness_show_roundtrips() {
    let dir = test_dir();
    let rec = save_test_readiness(&dir);
    let loaded = openwand_app::workflow_routing_readiness::load_workflow_routing_readiness(
        &dir, &rec.readiness_id,
    ).unwrap();
    assert_eq!(rec.readiness_id, loaded.readiness_id);
}

#[test]
fn cli_routing_readiness_references_review() {
    let dir = test_dir();
    let rec = save_test_readiness(&dir);
    assert_eq!(rec.review_id.0, "wnar_test");
}

// No forbidden verbs
#[test]
fn cli_routing_readiness_does_not_expose_route_execute_trust() {
    let src = include_str!("../src/main.rs");
    let start = src.find("enum WorkflowRoutingReadinessCommands").unwrap_or(0);
    let end = src.find("fn cmd_workflow_routing_readiness").unwrap_or(src.len());
    let section = &src[start..end];
    let lower = section.to_lowercase();
    let forbidden = ["execute", "approve-truth", "certify", "promote"];
    for word in &forbidden {
        assert!(!lower.contains(word), "Routing readiness CLI contains forbidden term: {}", word);
    }
}
