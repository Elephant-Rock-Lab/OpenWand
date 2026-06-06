//! CLI tests for audit packet review and distribution.

use openwand_workflow::workflow_audit_packet_review::*;
use openwand_workflow::workflow_audit_packet_distribution::*;
use openwand_workflow::workflow_run::WorkflowExecutionId;

fn test_dir() -> std::path::PathBuf {
    tempfile::tempdir().unwrap().into_path()
}

fn save_test_review(dir: &std::path::PathBuf) -> AuditPacketReview {
    let req = AuditPacketReviewRequest {
        inspection_id: "weci_test".into(),
        workflow_execution_id: WorkflowExecutionId("wfx_cli".into()),
        expected_audit_packet_hash: "pkt_hash".into(),
        expected_chain_hash: "chain_hash".into(),
        reviewer: "alice".into(),
        decision: AuditPacketReviewDecision::ReviewedWithCaveats,
        scope: "Full review".into(),
        caveats: vec![],
        idempotency_key: "k1".into(),
    };
    let rec = build_audit_packet_review(req);
    openwand_app::workflow_audit_packet_review::save_audit_packet_review(dir, &rec).unwrap();
    rec
}

fn save_test_distribution(dir: &std::path::PathBuf, review_id: &AuditPacketReviewId) -> AuditPacketDistribution {
    let req = AuditPacketDistributionRequest {
        review_id: review_id.clone(),
        workflow_execution_id: WorkflowExecutionId("wfx_cli".into()),
        expected_review_hash: "rev_hash".into(),
        audit_packet_hash: "pkt_hash".into(),
        chain_hash: "chain_hash".into(),
        inspection_id: "weci_test".into(),
        destination: AuditPacketDistributionDestination {
            destination_kind: AuditPacketDestinationKind::Archive,
            label: "Archive".into(),
            reference: "ref".into(),
            operator_supplied_hash: None,
            notes: vec![],
        },
        distribution_notes: vec![],
        idempotency_key: "dk1".into(),
    };
    let rec = build_audit_packet_distribution(req);
    openwand_app::workflow_audit_packet_distribution::save_audit_packet_distribution(dir, &rec).unwrap();
    rec
}

#[test]
fn cli_audit_packet_review_record_outputs_review_id() {
    let dir = test_dir();
    let rec = save_test_review(&dir);
    assert!(rec.review_id.0.starts_with("wapr_"));
}

#[test]
fn cli_audit_packet_review_show_roundtrips() {
    let dir = test_dir();
    let rec = save_test_review(&dir);
    let loaded = openwand_app::workflow_audit_packet_review::load_audit_packet_review(
        &dir, &rec.review_id,
    ).unwrap();
    assert_eq!(rec.review_id, loaded.review_id);
}

#[test]
fn cli_audit_packet_distribution_record_outputs_distribution_id() {
    let dir = test_dir();
    let rev = save_test_review(&dir);
    let dist = save_test_distribution(&dir, &rev.review_id);
    assert!(dist.distribution_id.0.starts_with("wapd_"));
}

#[test]
fn cli_audit_packet_distribution_requires_review_reference() {
    let dir = test_dir();
    let rev = save_test_review(&dir);
    let dist = save_test_distribution(&dir, &rev.review_id);
    assert_eq!(rev.review_id, dist.review_id);
}

// Patch 7: no forbidden verbs
#[test]
fn cli_audit_packet_review_does_not_expose_certify_verify_trust() {
    let src = include_str!("../src/main.rs");
    let apr_start = src.find("enum AuditPacketReviewCommands").unwrap_or(0);
    let apr_end = src.find("fn cmd_audit_packet_review").unwrap_or(src.len());
    let section = &src[apr_start..apr_end];
    let lower = section.to_lowercase();
    let forbidden = ["certify", "verify", "trust", "promote", "approve-truth"];
    for word in &forbidden {
        assert!(!lower.contains(word), "Review CLI contains forbidden term: {}", word);
    }
}

#[test]
fn cli_audit_packet_distribution_does_not_expose_send_upload_archive_now() {
    let src = include_str!("../src/main.rs");
    let apd_start = src.find("enum AuditPacketDistributionCommands").unwrap_or(0);
    let apd_end = src.find("fn cmd_audit_packet_distribution").unwrap_or(src.len());
    let section = &src[apd_start..apd_end];
    let lower = section.to_lowercase();
    let forbidden = ["send-email", "upload", "archive-now", "prove-delivery", "confirm-receipt"];
    for word in &forbidden {
        assert!(!lower.contains(word), "Distribution CLI contains forbidden term: {}", word);
    }
}
