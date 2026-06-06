//! UI audit packet distribution state — display-only helpers.

use openwand_workflow::workflow_audit_packet_distribution::*;

#[derive(Debug, Clone)]
pub struct DistributionSummaryRow {
    pub distribution_id: String,
    pub review_id: String,
    pub destination_kind: String,
    pub destination_label: String,
    pub reported_distribution: bool,
    pub proof_of_delivery: bool,
}

pub fn distribution_summary(rec: &AuditPacketDistribution) -> DistributionSummaryRow {
    DistributionSummaryRow {
        distribution_id: rec.distribution_id.0.clone(),
        review_id: rec.review_id.0.clone(),
        destination_kind: format!("{:?}", rec.destination.destination_kind),
        destination_label: rec.destination.label.clone(),
        reported_distribution: rec.reported_distribution,
        proof_of_delivery: rec.proof_of_delivery,
    }
}

pub fn distribution_safety_warning() -> String {
    "Audit packet distribution is reported metadata only. \
     It does not prove delivery, confirm receipt, verify the destination, \
     upload files, send messages, or integrate with external systems.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_audit_packet_review::AuditPacketReviewId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;

    fn test_distribution() -> AuditPacketDistribution {
        let req = AuditPacketDistributionRequest {
            review_id: AuditPacketReviewId("wapr_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            expected_review_hash: "rh".into(),
            audit_packet_hash: "ph".into(),
            chain_hash: "ch".into(),
            inspection_id: "weci_t".into(),
            destination: AuditPacketDistributionDestination {
                destination_kind: AuditPacketDestinationKind::FileShare,
                label: "Shared".into(),
                reference: "ref".into(),
                operator_supplied_hash: None,
                notes: vec![],
            },
            distribution_notes: vec![],
            idempotency_key: "k".into(),
        };
        build_audit_packet_distribution(req)
    }

    #[test]
    fn summary_row_extracts_fields() {
        let rec = test_distribution();
        let row = distribution_summary(&rec);
        assert!(row.distribution_id.starts_with("wapd_"));
        assert!(row.reported_distribution);
        assert!(!row.proof_of_delivery);
    }

    #[test]
    fn safety_warning_does_not_overclaim() {
        let w = distribution_safety_warning();
        assert!(w.contains("reported metadata"));
        assert!(w.contains("does not prove delivery"));
    }
}
