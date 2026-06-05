//! CLI tests for workflow verification readiness.

use openwand_workflow::workflow_verification_readiness::*;
use openwand_workflow::workflow_run::WorkflowExecutionId;

fn test_dir() -> std::path::PathBuf {
    tempfile::tempdir().unwrap().into_path()
}

fn save_test_readiness(dir: &std::path::PathBuf, suffix: &str, key: &str) -> VerificationReadinessRecord {
    let request = VerificationReadinessRequest {
        target_kind: VerificationReadinessTargetKind::ManualResult,
        target_id: format!("wmr_{}", suffix),
        workflow_execution_id: WorkflowExecutionId("wfx_cli".into()),
        expected_target_hash: format!("hash_{}", suffix),
        idempotency_key: key.into(),
    };
    let rec = evaluate_readiness_metadata_only(
        &request, "reported_succeeded", &request.expected_target_hash, "wfx_cli",
    );
    openwand_app::workflow_verification_readiness::save_verification_readiness(dir, &rec).unwrap();
    rec
}

#[test]
fn cli_verification_readiness_evaluate_outputs_readiness_id() {
    let dir = test_dir();
    let rec = save_test_readiness(&dir, "e1", "key1");
    assert!(rec.readiness_id.0.starts_with("wvr_"));
}

#[test]
fn cli_verification_readiness_show_roundtrips_record() {
    let dir = test_dir();
    let rec = save_test_readiness(&dir, "s1", "key1");
    let loaded = openwand_app::workflow_verification_readiness::load_verification_readiness(
        &dir, &rec.readiness_id,
    ).unwrap();
    assert_eq!(rec.readiness_id, loaded.readiness_id);
    assert_eq!(VerificationReadinessStatus::Ready, loaded.status);
}

#[test]
fn cli_verification_readiness_latest_by_target_returns_latest() {
    let dir = test_dir();
    save_test_readiness(&dir, "l1", "key1");
    let results = openwand_app::workflow_verification_readiness::readiness_by_target_id(
        &dir, "wmr_l1",
    ).unwrap();
    assert_eq!(1, results.len());
}

#[test]
fn cli_verification_readiness_does_not_expose_verify_trust_certify_schedule() {
    let src = include_str!("../src/main.rs");
    let section_start = src.find("workflow-verification-readiness").unwrap_or(0);
    let section = &src[section_start.saturating_sub(100)..];
    assert!(!section.contains("\"verify\""));
    assert!(!section.contains("\"trust\""));
    assert!(!section.contains("\"certify\""));
    assert!(!section.contains("\"schedule\""));
    assert!(!section.contains("\"fetch\""));
    assert!(!section.contains("\"read-artifact\""));
}
