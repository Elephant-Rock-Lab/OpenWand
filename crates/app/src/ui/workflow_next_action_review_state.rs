//! UI next-action review state — display-only helpers.
//!
//! Coverage gap closure (Wave 50A, FIX-05, KNOWN_GAPS gap 3).

use openwand_workflow::workflow_next_action_review::*;

#[derive(Debug, Clone)]
pub struct ReviewSummaryRow {
    pub review_id: String,
    pub proposal_id: String,
    pub decision: String,
    pub reviewer: String,
    pub rationale: String,
    pub has_feedback: bool,
}

pub fn review_summary(rec: &WorkflowNextActionReview) -> ReviewSummaryRow {
    ReviewSummaryRow {
        review_id: rec.review_id.0.clone(),
        proposal_id: rec.proposal_id.0.clone(),
        decision: format!("{:?}", rec.decision),
        reviewer: rec.reviewer.clone(),
        rationale: rec.rationale.clone(),
        has_feedback: rec.feedback.is_some(),
    }
}

pub fn next_action_review_safety_warning() -> String {
    "Next-action review is a decision, not a route. \
     It creates no route records, execution grants, session turns, \
     approval requests, tool calls, trace events, or workflow mutations.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_continuation::WorkflowNextActionProposalId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;

    fn test_review() -> WorkflowNextActionReview {
        WorkflowNextActionReview {
            review_id: WorkflowNextActionReviewId("wnar_t".into()),
            proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
            proposal_hash: "h".into(),
            source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            source_run_revision_hash: "h".into(),
            decision: WorkflowNextActionReviewDecision::Approved,
            reviewer: "alice".into(),
            rationale: "test rationale".into(),
            feedback: None,
            creates_route: false,
            routes_action_now: false,
            executes_tool_now: false,
            mutates_workflow_state_now: false,
            reviewed_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn summary_row_extracts_fields() {
        let rec = test_review();
        let row = review_summary(&rec);
        assert_eq!("wnar_t", row.review_id);
        assert_eq!("wnap_t", row.proposal_id);
        assert_eq!("alice", row.reviewer);
        assert!(!row.has_feedback);
    }

    #[test]
    fn safety_warning_does_not_overclaim() {
        let w = next_action_review_safety_warning();
        assert!(w.contains("not a route"));
        assert!(w.contains("creates no route"));
    }
}
