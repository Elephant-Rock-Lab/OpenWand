//! Workflow proposal review DTOs.
//!
//! A review is evidence, not an execution grant.
//! creates_execution_grant is always false.
//! execution_allowed_now is always false.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::plan::TaskPlanId;
use crate::workflow_proposal::WorkflowProposalId;

/// Content-addressed review ID. Format: `wfr_<blake3_hex>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowProposalReviewId(pub String);

impl WorkflowProposalReviewId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A human review of a workflow proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowProposalReview {
    pub review_id: WorkflowProposalReviewId,
    pub proposal_id: WorkflowProposalId,
    pub source_task_plan_id: TaskPlanId,
    pub proposal_hash: String,
    pub decision: WorkflowProposalReviewDecision,
    pub reviewer: String,
    pub rationale: String,
    pub feedback: Option<WorkflowProposalFeedback>,
    /// Always false. A review is evidence, not an execution grant.
    pub creates_execution_grant: bool,
    /// Always false. Approval does not authorize execution now.
    pub execution_allowed_now: bool,
    pub reviewed_at: DateTime<Utc>,
}

/// Review decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowProposalReviewDecision {
    Approved,
    Rejected,
    ChangesRequested,
}

/// Structured feedback on a workflow proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowProposalFeedback {
    pub summary: String,
    pub blocking_reasons: Vec<String>,
    pub requested_changes: Vec<String>,
    pub evidence_gaps: Vec<String>,
}

/// Compute content-addressed review ID.
pub fn workflow_review_id_for(
    proposal_id: &WorkflowProposalId,
    decision: &WorkflowProposalReviewDecision,
    rationale: &str,
) -> WorkflowProposalReviewId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(proposal_id.0.as_bytes());
    let decision_str = serde_json::to_string(decision).unwrap_or_default();
    hasher.update(decision_str.as_bytes());
    hasher.update(rationale.as_bytes());
    let hash = hasher.finalize();
    WorkflowProposalReviewId(format!("wfr_{}", hash.to_hex()))
}

/// Validate a workflow proposal review.
pub fn validate_workflow_proposal_review(review: &WorkflowProposalReview) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if review.reviewer.trim().is_empty() {
        errors.push("reviewer must not be empty".into());
    }

    if review.rationale.trim().is_empty() {
        errors.push("rationale must not be empty".into());
    }

    if review.creates_execution_grant {
        errors.push("creates_execution_grant must be false".into());
    }

    if review.execution_allowed_now {
        errors.push("execution_allowed_now must be false".into());
    }

    match review.decision {
        WorkflowProposalReviewDecision::Rejected => {
            if let Some(ref feedback) = review.feedback {
                if feedback.blocking_reasons.is_empty() {
                    errors.push("rejection requires at least one blocking reason".into());
                }
            } else {
                errors.push("rejection requires feedback with blocking reasons".into());
            }
        }
        WorkflowProposalReviewDecision::ChangesRequested => {
            if let Some(ref feedback) = review.feedback {
                if feedback.requested_changes.is_empty() {
                    errors.push("change request requires at least one requested change".into());
                }
            } else {
                errors.push("change request requires feedback with requested changes".into());
            }
        }
        WorkflowProposalReviewDecision::Approved => {}
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::TaskPlanId;

    fn test_proposal_id() -> WorkflowProposalId {
        WorkflowProposalId("wfp_test123".into())
    }

    fn test_plan_id() -> TaskPlanId {
        TaskPlanId("tpl_test".into())
    }

    fn approved_review() -> WorkflowProposalReview {
        let proposal_id = test_proposal_id();
        let review_id = workflow_review_id_for(
            &proposal_id,
            &WorkflowProposalReviewDecision::Approved,
            "Looks good",
        );
        WorkflowProposalReview {
            review_id,
            proposal_id,
            source_task_plan_id: test_plan_id(),
            proposal_hash: "phash".into(),
            decision: WorkflowProposalReviewDecision::Approved,
            reviewer: "test-reviewer".into(),
            rationale: "Looks good".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        }
    }

    #[test]
    fn workflow_review_roundtrips() {
        let review = approved_review();
        let json = serde_json::to_string(&review).unwrap();
        let back: WorkflowProposalReview = serde_json::from_str(&json).unwrap();
        assert_eq!(review.review_id, back.review_id);
        assert_eq!(review.proposal_id, back.proposal_id);
    }

    #[test]
    fn workflow_review_id_is_content_addressed() {
        let id = workflow_review_id_for(
            &test_proposal_id(),
            &WorkflowProposalReviewDecision::Approved,
            "test",
        );
        assert!(id.as_str().starts_with("wfr_"));
        // Same inputs → same ID
        let id2 = workflow_review_id_for(
            &test_proposal_id(),
            &WorkflowProposalReviewDecision::Approved,
            "test",
        );
        assert_eq!(id, id2);
    }

    #[test]
    fn workflow_review_approval_requires_rationale() {
        let mut review = approved_review();
        review.rationale = "  ".into();
        let errors = validate_workflow_proposal_review(&review).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("rationale")));
    }

    #[test]
    fn workflow_review_does_not_create_execution_grant() {
        let review = approved_review();
        assert!(!review.creates_execution_grant);
    }

    #[test]
    fn workflow_review_does_not_allow_execution_now() {
        let review = approved_review();
        assert!(!review.execution_allowed_now);
    }

    #[test]
    fn workflow_review_rejection_requires_blocking_reason() {
        let proposal_id = test_proposal_id();
        let review_id = workflow_review_id_for(
            &proposal_id,
            &WorkflowProposalReviewDecision::Rejected,
            "Bad",
        );
        let review = WorkflowProposalReview {
            review_id,
            proposal_id,
            source_task_plan_id: test_plan_id(),
            proposal_hash: "phash".into(),
            decision: WorkflowProposalReviewDecision::Rejected,
            reviewer: "r".into(),
            rationale: "Bad".into(),
            feedback: None, // missing
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        let errors = validate_workflow_proposal_review(&review).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("blocking")));
    }

    #[test]
    fn workflow_change_request_requires_requested_change() {
        let proposal_id = test_proposal_id();
        let review_id = workflow_review_id_for(
            &proposal_id,
            &WorkflowProposalReviewDecision::ChangesRequested,
            "Fix",
        );
        let review = WorkflowProposalReview {
            review_id,
            proposal_id,
            source_task_plan_id: test_plan_id(),
            proposal_hash: "phash".into(),
            decision: WorkflowProposalReviewDecision::ChangesRequested,
            reviewer: "r".into(),
            rationale: "Fix".into(),
            feedback: Some(WorkflowProposalFeedback {
                summary: "Needs work".into(),
                blocking_reasons: vec![],
                requested_changes: vec![], // empty
                evidence_gaps: vec![],
            }),
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        let errors = validate_workflow_proposal_review(&review).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("requested change")));
    }

    #[test]
    fn workflow_feedback_roundtrips() {
        let feedback = WorkflowProposalFeedback {
            summary: "Test feedback".into(),
            blocking_reasons: vec!["Reason 1".into()],
            requested_changes: vec!["Change 1".into()],
            evidence_gaps: vec!["Gap 1".into()],
        };
        let json = serde_json::to_string(&feedback).unwrap();
        let back: WorkflowProposalFeedback = serde_json::from_str(&json).unwrap();
        assert_eq!(feedback.summary, back.summary);
        assert_eq!(1, back.blocking_reasons.len());
    }

    #[test]
    fn workflow_review_decision_serializes_snake_case() {
        let decision = WorkflowProposalReviewDecision::ChangesRequested;
        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("changes_requested"));
        assert!(!json.contains("ChangesRequested"));
    }

    #[test]
    fn workflow_review_hash_changes_with_different_decision() {
        let id_approved = workflow_review_id_for(
            &test_proposal_id(),
            &WorkflowProposalReviewDecision::Approved,
            "test",
        );
        let id_rejected = workflow_review_id_for(
            &test_proposal_id(),
            &WorkflowProposalReviewDecision::Rejected,
            "test",
        );
        assert_ne!(id_approved, id_rejected);
    }
}
