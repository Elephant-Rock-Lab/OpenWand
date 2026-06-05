//! UI external attestation state — display-only helpers.

use openwand_workflow::workflow_external_attestation::*;

#[derive(Debug, Clone)]
pub struct AttestationSummaryRow {
    pub attestation_id: String,
    pub target_kind: String,
    pub target_id: String,
    pub kind: String,
    pub source_name: String,
    pub claim: String,
    pub verified: bool,
}

pub fn attestation_summary(att: &WorkflowExternalAttestation) -> AttestationSummaryRow {
    AttestationSummaryRow {
        attestation_id: att.attestation_id.0.clone(),
        target_kind: format!("{:?}", att.target.target_kind),
        target_id: att.target.target_id.clone(),
        kind: format!("{:?}", att.kind),
        source_name: att.source.name.clone(),
        claim: att.claim.clone(),
        verified: att.verified_by_openwand,
    }
}

pub fn attestation_safety_warning() -> String {
    "External attestation is reported evidence. \
     It is not verification, not trust promotion, not reconciliation, \
     and does not certify external truth.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;

    fn test_att() -> WorkflowExternalAttestation {
        let req = ExternalAttestationRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            target_kind: ExternalAttestationTargetKind::ManualResult,
            target_id: "wmr_t".into(),
            expected_target_hash: None,
            kind: ExternalAttestationKind::CodeReviewApproval,
            source_name: "Bob".into(),
            source_role: "reviewer".into(),
            source_system_identifier: None,
            claim: "LGTM".into(),
            references: vec![],
            reported_signature: None,
            attested_at: chrono::Utc::now(),
            idempotency_key: "k1".into(),
        };
        build_external_attestation(req)
    }

    #[test]
    fn summary_row_extracts_fields() {
        let att = test_att();
        let row = attestation_summary(&att);
        assert!(row.attestation_id.starts_with("watt_"));
        assert_eq!("Bob", row.source_name);
        assert_eq!("LGTM", row.claim);
        assert!(!row.verified);
    }

    #[test]
    fn safety_warning_does_not_overclaim() {
        let w = attestation_safety_warning();
        assert!(w.contains("reported evidence"));
        assert!(w.contains("not verification"));
        assert!(w.contains("does not certify"));
    }
}
