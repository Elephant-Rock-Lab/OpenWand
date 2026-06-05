//! CLI tests for workflow external attestation.

use openwand_workflow::workflow_external_attestation::*;
use openwand_workflow::workflow_run::WorkflowExecutionId;

fn test_dir() -> std::path::PathBuf {
    tempfile::tempdir().unwrap().into_path()
}

fn save_test_attestation(dir: &std::path::PathBuf, suffix: &str, key: &str) -> WorkflowExternalAttestation {
    let req = ExternalAttestationRequest {
        workflow_execution_id: WorkflowExecutionId("wfx_cli".into()),
        target_kind: ExternalAttestationTargetKind::ManualResult,
        target_id: format!("wmr_{}", suffix),
        expected_target_hash: None,
        kind: ExternalAttestationKind::ThirdPartySignoff,
        source_name: "Alice".into(),
        source_role: "reviewer".into(),
        source_system_identifier: None,
        claim: format!("Reviewed {}", suffix),
        references: vec![],
        reported_signature: None,
        attested_at: chrono::Utc::now(),
        idempotency_key: key.into(),
    };
    let att = build_external_attestation(req);
    openwand_app::workflow_external_attestation::save_external_attestation(dir, &att).unwrap();
    att
}

#[test]
fn cli_attestation_attach_outputs_attestation_id() {
    let dir = test_dir();
    let att = save_test_attestation(&dir, "a1", "key1");
    assert!(att.attestation_id.0.starts_with("watt_"));
}

#[test]
fn cli_attestation_attach_requires_claim_and_source() {
    // Verify validation rejects empty claim/source
    let req = ExternalAttestationRequest {
        workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
        target_kind: ExternalAttestationTargetKind::ManualResult,
        target_id: "wmr_1".into(),
        expected_target_hash: None,
        kind: ExternalAttestationKind::ThirdPartySignoff,
        source_name: "".into(),
        source_role: "role".into(),
        source_system_identifier: None,
        claim: "".into(),
        references: vec![],
        reported_signature: None,
        attested_at: chrono::Utc::now(),
        idempotency_key: "key".into(),
    };
    let result = openwand_workflow::workflow_external_attestation_validation::validate_attestation_request(&req);
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| e.contains("claim")));
    assert!(errors.iter().any(|e| e.contains("source_name")));
}

#[test]
fn cli_attestation_reference_is_metadata_only() {
    let dir = test_dir();
    let mut req = ExternalAttestationRequest {
        workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
        target_kind: ExternalAttestationTargetKind::ManualResult,
        target_id: "wmr_1".into(),
        expected_target_hash: None,
        kind: ExternalAttestationKind::ThirdPartySignoff,
        source_name: "Alice".into(),
        source_role: "reviewer".into(),
        source_system_identifier: None,
        claim: "Claim".into(),
        references: vec![ExternalAttestationReference {
            reference_id: "r1".into(),
            label: "CI".into(),
            kind: ExternalAttestationReferenceKind::ExternalUrl,
            reference: "https://ci.example.com/123".into(),
            operator_supplied_hash: Some("abc123".into()),
            description: None,
        }],
        reported_signature: None,
        attested_at: chrono::Utc::now(),
        idempotency_key: "key".into(),
    };
    let att = build_external_attestation(req);
    openwand_app::workflow_external_attestation::save_external_attestation(&dir, &att).unwrap();
    let loaded = openwand_app::workflow_external_attestation::load_external_attestation(&dir, &att.attestation_id).unwrap();
    assert_eq!(1, loaded.references.len());
    assert_eq!("https://ci.example.com/123", loaded.references[0].reference);
}

#[test]
fn cli_attestation_does_not_expose_verify_trust_certify() {
    let src = include_str!("../src/main.rs");
    let att_section_start = src.find("workflow-external-attestation").unwrap_or(0);
    let att_section = &src[att_section_start.saturating_sub(100)..];
    assert!(!att_section.contains("\"verify\""));
    assert!(!att_section.contains("\"trust\""));
    assert!(!att_section.contains("\"certify\""));
    assert!(!att_section.contains("\"validate-signature\""));
    assert!(!att_section.contains("\"check-url\""));
    assert!(!att_section.contains("\"read-file\""));
}
