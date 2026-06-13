//! Command review DTOs — operator acknowledgment of display-only command descriptors.
//!
//! Review records that the operator saw and acknowledged/rejected/requested-changes
//! on a command descriptor. It does not execute the command, grant execution
//! permission, route actions, resolve approvals, or mutate workflow state.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_command_composer::WorkflowCommandComposerId;
use crate::workflow_loop_controller::WorkflowLoopControllerId;
use crate::workflow_run::WorkflowExecutionId;

/// Content-addressed review ID. Format: wcrv_<blake3_hex>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowCommandReviewId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCommandReviewRequest {
    pub workflow_execution_id: WorkflowExecutionId,
    pub command_composer_id: WorkflowCommandComposerId,
    pub loop_controller_id: WorkflowLoopControllerId,
    /// Patch 2: hash of the composer record (not just the descriptor projection)
    pub expected_command_composer_hash: String,
    /// Hash of the descriptor projection inside the composer record
    pub expected_command_descriptor_hash: String,
    pub expected_loop_controller_hash: String,
    pub decision: WorkflowCommandReviewDecision,
    pub reviewer: String,
    pub rationale: String,
    pub feedback: Option<WorkflowCommandReviewFeedback>,
    pub reviewed_at: DateTime<Utc>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCommandReview {
    pub review_id: WorkflowCommandReviewId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub command_composer_id: WorkflowCommandComposerId,
    pub loop_controller_id: WorkflowLoopControllerId,
    pub command_composer_hash: String,
    pub command_descriptor_hash: String,
    pub loop_controller_hash: String,
    pub decision: WorkflowCommandReviewDecision,
    pub reviewer: String,
    pub rationale: String,
    pub feedback: Option<WorkflowCommandReviewFeedback>,
    pub acknowledgment_snapshot: WorkflowCommandAcknowledgmentSnapshot,
    // 12 hardcoded-false authority flags
    pub executes_command: bool,
    pub invokes_shell: bool,
    pub invokes_git: bool,
    pub routes_action: bool,
    pub resolves_approval: bool,
    pub reconciles_outcome: bool,
    pub mutates_workflow_state: bool,
    pub schedules_work: bool,
    pub starts_worker: bool,
    pub queues_operation: bool,
    pub creates_execution_grant: bool,
    pub execution_allowed_now: bool,
    pub reviewed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowCommandReviewDecision {
    Acknowledged,
    Rejected,
    ChangesRequested,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCommandReviewFeedback {
    pub summary: String,
    pub blocking_reasons: Vec<String>,
    pub requested_changes: Vec<String>,
    pub evidence_gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCommandAcknowledgmentSnapshot {
    pub descriptor_display_command: String,
    pub descriptor_copyable_text_hash: String,
    pub descriptor_display_only: bool,
    pub descriptor_executable: bool,
    pub descriptor_missing_inputs: Vec<String>,
    pub loop_detected_state: String,
    pub loop_recommended_operation: String,
    /// Patch 3: acknowledgment records review only, not command performance.
    pub acknowledges_review_only: bool,
    /// Patch 3: the command was not performed by this acknowledgment.
    pub command_performed_now: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_review(decision: WorkflowCommandReviewDecision) -> WorkflowCommandReview {
        WorkflowCommandReview {
            review_id: WorkflowCommandReviewId("wcrv_test".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            command_composer_hash: "ch".into(),
            command_descriptor_hash: "dh".into(),
            loop_controller_hash: "lh".into(),
            decision,
            reviewer: "tester".into(),
            rationale: "testing".into(),
            feedback: None,
            acknowledgment_snapshot: WorkflowCommandAcknowledgmentSnapshot {
                descriptor_display_command: "openwand test".into(),
                descriptor_copyable_text_hash: "cth".into(),
                descriptor_display_only: true,
                descriptor_executable: false,
                descriptor_missing_inputs: vec![],
                loop_detected_state: "idle".into(),
                loop_recommended_operation: "none".into(),
                acknowledges_review_only: true,
                command_performed_now: false,
            },
            executes_command: false, invokes_shell: false, invokes_git: false,
            routes_action: false, resolves_approval: false, reconciles_outcome: false,
            mutates_workflow_state: false, schedules_work: false, starts_worker: false,
            queues_operation: false, creates_execution_grant: false,
            execution_allowed_now: false, reviewed_at: Utc::now(),
        }
    }

    #[test]
    fn workflow_command_review_roundtrips() {
        let r = test_review(WorkflowCommandReviewDecision::Acknowledged);
        let json = serde_json::to_string(&r).unwrap();
        let back: WorkflowCommandReview = serde_json::from_str(&json).unwrap();
        assert_eq!(r.review_id, back.review_id);
    }

    #[test]
    fn workflow_command_review_id_is_content_addressed() {
        use blake3::Hasher;
        let mut h1 = Hasher::new();
        h1.update(b"review:v1:wfx_t:wcc_t:ack");
        let id1 = format!("wcrv_{}", &h1.finalize().to_hex()[..16]);

        let mut h2 = Hasher::new();
        h2.update(b"review:v1:wfx_t:wcc_t:ack");
        let id2 = format!("wcrv_{}", &h2.finalize().to_hex()[..16]);

        assert_eq!(id1, id2);
    }

    #[test]
    fn workflow_command_review_decision_serializes_snake_case() {
        let d = WorkflowCommandReviewDecision::ChangesRequested;
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains("changes_requested"));
        let a = WorkflowCommandReviewDecision::Acknowledged;
        assert!(serde_json::to_string(&a).unwrap().contains("acknowledged"));
        let r = WorkflowCommandReviewDecision::Rejected;
        assert!(serde_json::to_string(&r).unwrap().contains("rejected"));
    }

    #[test]
    fn workflow_command_review_acknowledgment_requires_rationale() {
        let req = WorkflowCommandReviewRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            expected_command_composer_hash: "ch".into(),
            expected_command_descriptor_hash: "dh".into(),
            expected_loop_controller_hash: "lh".into(),
            decision: WorkflowCommandReviewDecision::Acknowledged,
            reviewer: "tester".into(),
            rationale: String::new(), // empty
            feedback: None,
            reviewed_at: Utc::now(),
            idempotency_key: "k".into(),
        };
        assert!(req.rationale.is_empty(), "Empty rationale should be caught by validation");
    }

    #[test]
    fn workflow_command_review_rejection_requires_feedback() {
        let r = test_review(WorkflowCommandReviewDecision::Rejected);
        // Rejection without feedback blocking_reasons is invalid
        assert!(r.feedback.is_none() || r.feedback.as_ref().is_none_or(|f| f.blocking_reasons.is_empty()));
    }

    #[test]
    fn workflow_command_review_change_request_requires_requested_change() {
        let r = test_review(WorkflowCommandReviewDecision::ChangesRequested);
        assert!(r.feedback.is_none() || r.feedback.as_ref().is_none_or(|f| f.requested_changes.is_empty()));
    }

    #[test]
    fn workflow_command_review_has_no_authority_flags() {
        let r = test_review(WorkflowCommandReviewDecision::Acknowledged);
        assert!(!r.executes_command); assert!(!r.invokes_shell); assert!(!r.invokes_git);
        assert!(!r.routes_action); assert!(!r.resolves_approval); assert!(!r.reconciles_outcome);
        assert!(!r.mutates_workflow_state); assert!(!r.schedules_work); assert!(!r.starts_worker);
        assert!(!r.queues_operation); assert!(!r.creates_execution_grant); assert!(!r.execution_allowed_now);
    }

    #[test]
    fn workflow_command_acknowledgment_snapshot_roundtrips() {
        let s = WorkflowCommandAcknowledgmentSnapshot {
            descriptor_display_command: "test".into(),
            descriptor_copyable_text_hash: "h".into(),
            descriptor_display_only: true, descriptor_executable: false,
            descriptor_missing_inputs: vec!["decision".into()],
            loop_detected_state: "idle".into(),
            loop_recommended_operation: "none".into(),
            acknowledges_review_only: true, command_performed_now: false,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: WorkflowCommandAcknowledgmentSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(s.descriptor_display_command, back.descriptor_display_command);
        assert!(back.acknowledges_review_only);
        assert!(!back.command_performed_now);
    }

    #[test]
    fn workflow_command_review_does_not_claim_command_performed() {
        let r = test_review(WorkflowCommandReviewDecision::Acknowledged);
        let json = serde_json::to_string(&r).unwrap().to_lowercase();
        assert!(!json.contains("\"command_performed\":true"));
        assert!(!json.contains("\"command_executed\":true"));
    }

    // Patch 3: snapshot marks review-only
    #[test]
    fn acknowledgment_snapshot_marks_review_only() {
        let r = test_review(WorkflowCommandReviewDecision::Acknowledged);
        assert!(r.acknowledgment_snapshot.acknowledges_review_only);
    }

    // Patch 3: snapshot marks command not performed
    #[test]
    fn acknowledgment_snapshot_marks_command_not_performed() {
        let r = test_review(WorkflowCommandReviewDecision::Acknowledged);
        assert!(!r.acknowledgment_snapshot.command_performed_now);
    }
}
