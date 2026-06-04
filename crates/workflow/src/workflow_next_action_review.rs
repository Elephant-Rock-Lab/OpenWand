//! Next-action review DTOs.
//!
//! Review is a decision, not a route. It creates no route records, execution grants,
//! session turns, approval requests, tool calls, trace events, or workflow mutations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_continuation::WorkflowNextActionProposalId;
use crate::workflow_reconciliation::WorkflowRunRevisionId;

/// Content-addressed review ID. Format: wnar_<blake3_hex>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowNextActionReviewId(pub String);

/// Human review decision on a next-action proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNextActionReview {
    pub review_id: WorkflowNextActionReviewId,
    pub proposal_id: WorkflowNextActionProposalId,
    pub proposal_hash: String,
    pub source_run_revision_id: WorkflowRunRevisionId,
    pub source_run_revision_hash: String,
    pub decision: WorkflowNextActionReviewDecision,
    pub reviewer: String,
    pub rationale: String,
    pub feedback: Option<WorkflowNextActionFeedback>,
    /// Always false — reviews never create routes.
    pub creates_route: bool,
    /// Always false — reviews never route actions.
    pub routes_action_now: bool,
    /// Always false — reviews never execute tools.
    pub executes_tool_now: bool,
    /// Always false — reviews never mutate workflow state.
    pub mutates_workflow_state_now: bool,
    pub reviewed_at: DateTime<Utc>,
}

/// Review decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNextActionReviewDecision {
    Approved,
    Rejected,
    ChangesRequested,
}

/// Structured feedback for rejection or change request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNextActionFeedback {
    pub summary: String,
    pub blocking_reasons: Vec<String>,
    pub requested_changes: Vec<String>,
    pub evidence_gaps: Vec<String>,
}

/// Content-addressed review hash for linkage verification.
pub fn review_hash_for(proposal_id: &str, decision: &WorkflowNextActionReviewDecision, reviewer: &str) -> String {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(b"review:v1:");
    hasher.update(proposal_id.as_bytes());
    hasher.update(b":");
    hasher.update(format!("{:?}", decision).as_bytes());
    hasher.update(b":");
    hasher.update(reviewer.as_bytes());
    hasher.finalize().to_hex().to_string()
}

/// Validate review constraints.
pub fn validate_review(
    reviewer: &str,
    rationale: &str,
    decision: &WorkflowNextActionReviewDecision,
    feedback: Option<&WorkflowNextActionFeedback>,
) -> Result<(), String> {
    if reviewer.is_empty() {
        return Err("reviewer is required".into());
    }
    if rationale.is_empty() {
        return Err("rationale is required".into());
    }
    match decision {
        WorkflowNextActionReviewDecision::Rejected => {
            if feedback.map_or(true, |f| f.blocking_reasons.is_empty()) {
                return Err("rejection requires feedback.blocking_reasons".into());
            }
        }
        WorkflowNextActionReviewDecision::ChangesRequested => {
            if feedback.map_or(true, |f| f.requested_changes.is_empty()) {
                return Err("changes requested requires feedback.requested_changes".into());
            }
        }
        WorkflowNextActionReviewDecision::Approved => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_action_review_roundtrips() {
        let review = WorkflowNextActionReview {
            review_id: WorkflowNextActionReviewId("wnar_abc".into()),
            proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
            proposal_hash: "ph".into(),
            source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            source_run_revision_hash: "rh".into(),
            decision: WorkflowNextActionReviewDecision::Approved,
            reviewer: "alice".into(),
            rationale: "safe".into(),
            feedback: None,
            creates_route: false, routes_action_now: false,
            executes_tool_now: false, mutates_workflow_state_now: false,
            reviewed_at: Utc::now(),
        };
        let json = serde_json::to_string(&review).unwrap();
        let back: WorkflowNextActionReview = serde_json::from_str(&json).unwrap();
        assert_eq!(review.review_id, back.review_id);
    }

    #[test]
    fn next_action_review_id_is_content_addressed() {
        let id = WorkflowNextActionReviewId("wnar_deadbeef".into());
        assert!(id.0.starts_with("wnar_"));
    }

    #[test]
    fn next_action_review_decision_serializes_snake_case() {
        let json = serde_json::to_string(&WorkflowNextActionReviewDecision::ChangesRequested).unwrap();
        assert!(json.contains("changes_requested"));
    }

    #[test]
    fn next_action_review_approval_requires_rationale() {
        assert!(validate_review("alice", "", &WorkflowNextActionReviewDecision::Approved, None).is_err());
        assert!(validate_review("alice", "ok", &WorkflowNextActionReviewDecision::Approved, None).is_ok());
    }

    #[test]
    fn next_action_review_rejection_requires_feedback() {
        assert!(validate_review("alice", "no", &WorkflowNextActionReviewDecision::Rejected, None).is_err());
        let fb = WorkflowNextActionFeedback {
            summary: "unsafe".into(), blocking_reasons: vec!["risk".into()],
            requested_changes: vec![], evidence_gaps: vec![],
        };
        assert!(validate_review("alice", "no", &WorkflowNextActionReviewDecision::Rejected, Some(&fb)).is_ok());
    }

    #[test]
    fn next_action_review_change_request_requires_requested_change() {
        assert!(validate_review("alice", "fix", &WorkflowNextActionReviewDecision::ChangesRequested, None).is_err());
        let fb = WorkflowNextActionFeedback {
            summary: "needs work".into(), blocking_reasons: vec![],
            requested_changes: vec!["add evidence".into()], evidence_gaps: vec![],
        };
        assert!(validate_review("alice", "fix", &WorkflowNextActionReviewDecision::ChangesRequested, Some(&fb)).is_ok());
    }

    #[test]
    fn next_action_review_does_not_create_route_or_execute() {
        let review = WorkflowNextActionReview {
            review_id: WorkflowNextActionReviewId("wnar_t".into()),
            proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
            proposal_hash: "ph".into(),
            source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            source_run_revision_hash: "rh".into(),
            decision: WorkflowNextActionReviewDecision::Approved,
            reviewer: "test".into(), rationale: "ok".into(), feedback: None,
            creates_route: false, routes_action_now: false,
            executes_tool_now: false, mutates_workflow_state_now: false,
            reviewed_at: Utc::now(),
        };
        assert!(!review.creates_route);
        assert!(!review.routes_action_now);
        assert!(!review.executes_tool_now);
        assert!(!review.mutates_workflow_state_now);
    }

    #[test]
    fn next_action_feedback_roundtrips() {
        let fb = WorkflowNextActionFeedback {
            summary: "issues".into(), blocking_reasons: vec!["risk".into()],
            requested_changes: vec!["fix".into()], evidence_gaps: vec!["trace".into()],
        };
        let json = serde_json::to_string(&fb).unwrap();
        let back: WorkflowNextActionFeedback = serde_json::from_str(&json).unwrap();
        assert_eq!(1, back.blocking_reasons.len());
        assert_eq!(1, back.requested_changes.len());
    }
}
