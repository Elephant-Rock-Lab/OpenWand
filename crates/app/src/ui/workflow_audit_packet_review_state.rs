//! UI audit packet review state — display-only helpers.

use openwand_workflow::workflow_audit_packet_review::*;

#[derive(Debug, Clone)]
pub struct ReviewSummaryRow {
    pub review_id: String,
    pub inspection_id: String,
    pub reviewer: String,
    pub decision: String,
    pub scope: String,
    pub caveat_count: usize,
}

pub fn review_summary(rec: &AuditPacketReview) -> ReviewSummaryRow {
    ReviewSummaryRow {
        review_id: rec.review_id.0.clone(),
        inspection_id: rec.inspection_id.clone(),
        reviewer: rec.reviewer.clone(),
        decision: format!("{:?}", rec.decision),
        scope: rec.scope.clone(),
        caveat_count: rec.caveats.len(),
    }
}

pub fn review_safety_warning() -> String {
    "Audit packet review is not truth certification. \
     Review does not verify packet contents, certify external truth, \
     prove delivery, modify the audit packet, or promote trust.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;

    fn test_review() -> AuditPacketReview {
        let req = AuditPacketReviewRequest {
            inspection_id: "weci_t".into(),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            expected_audit_packet_hash: "h1".into(),
            expected_chain_hash: "h2".into(),
            reviewer: "alice".into(),
            decision: AuditPacketReviewDecision::ReviewedWithCaveats,
            scope: "test".into(),
            caveats: vec!["caveat1".into()],
            idempotency_key: "k".into(),
        };
        build_audit_packet_review(req)
    }

    #[test]
    fn summary_row_extracts_fields() {
        let rec = test_review();
        let row = review_summary(&rec);
        assert!(row.review_id.starts_with("wapr_"));
        assert_eq!("alice", row.reviewer);
        assert_eq!(1, row.caveat_count);
    }

    #[test]
    fn safety_warning_does_not_overclaim() {
        let w = review_safety_warning();
        assert!(w.contains("not truth certification"));
        assert!(w.contains("does not verify"));
    }
}
