//! Manual result review DTOs — operator acceptance of reported manual result evidence.
//!
//! Review records that a human reviewer accepted, rejected, or requested changes
//! on a reported manual result. It does not verify external state, reconcile
//! workflow state, execute commands, or mutate any existing records.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_command_composer::WorkflowCommandComposerId;
use crate::workflow_command_review::WorkflowCommandReviewId;
use crate::workflow_loop_controller::WorkflowLoopControllerId;
use crate::workflow_manual_result::WorkflowManualResultId;
use crate::workflow_run::WorkflowExecutionId;

/// Content-addressed review ID. Format: wmrr_<blake3_hex>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowManualResultReviewId(pub String);

/// Request to review a reported manual result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualResultReviewRequest {
    pub workflow_execution_id: WorkflowExecutionId,
    pub manual_result_id: WorkflowManualResultId,
    pub command_review_id: WorkflowCommandReviewId,
    pub command_composer_id: WorkflowCommandComposerId,
    pub loop_controller_id: WorkflowLoopControllerId,
    // Patch 2: full evidence chain hashes
    pub expected_manual_result_hash: String,
    pub expected_command_review_hash: String,
    pub expected_command_composer_hash: String,
    pub expected_command_descriptor_hash: String,
    pub expected_loop_controller_hash: String,
    pub decision: WorkflowManualResultReviewDecision,
    pub reviewer: String,
    pub rationale: String,
    pub feedback: Option<WorkflowManualResultReviewFeedback>,
    pub reviewed_at: DateTime<Utc>,
    pub idempotency_key: String,
}

/// Sealed review record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualResultReview {
    pub review_id: WorkflowManualResultReviewId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub manual_result_id: WorkflowManualResultId,
    pub command_review_id: WorkflowCommandReviewId,
    pub command_composer_id: WorkflowCommandComposerId,
    pub loop_controller_id: WorkflowLoopControllerId,
    // Patch 2: stored hashes from evidence chain
    pub manual_result_hash: String,
    pub command_review_hash: String,
    pub command_composer_hash: String,
    pub command_descriptor_hash: String,
    pub loop_controller_hash: String,
    pub decision: WorkflowManualResultReviewDecision,
    pub reviewer: String,
    pub rationale: String,
    pub feedback: Option<WorkflowManualResultReviewFeedback>,
    // Patch 3: acceptance semantics snapshot
    pub acceptance_snapshot: WorkflowManualResultReviewAcceptanceSnapshot,
    // 12 hardcoded-false authority flags
    pub verifies_external_state: bool,
    pub reconciles_workflow_state: bool,
    pub mutates_workflow_state: bool,
    pub executes_command: bool,
    pub invokes_shell: bool,
    pub invokes_git: bool,
    pub routes_action: bool,
    pub resolves_approval: bool,
    pub appends_trace: bool,
    pub writes_memory: bool,
    pub creates_execution_grant: bool,
    pub execution_allowed_now: bool,
    pub reviewed_at: DateTime<Utc>,
}

/// Review decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowManualResultReviewDecision {
    Accepted,
    Rejected,
    ChangesRequested,
}

/// Structured feedback for rejection or change request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualResultReviewFeedback {
    pub summary: String,
    pub blocking_reasons: Vec<String>,
    pub requested_changes: Vec<String>,
    pub evidence_gaps: Vec<String>,
}

/// Patch 3: acceptance semantics snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualResultReviewAcceptanceSnapshot {
    pub accepts_reported_evidence: bool,
    pub verifies_external_state: bool,
    pub reconciles_workflow_state: bool,
    pub result_verified_by_openwand: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_review(decision: WorkflowManualResultReviewDecision) -> WorkflowManualResultReview {
        WorkflowManualResultReview {
            review_id: WorkflowManualResultReviewId("wmrr_test".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            manual_result_id: WorkflowManualResultId("wmr_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            manual_result_hash: "mrh".into(),
            command_review_hash: "crh".into(),
            command_composer_hash: "cch".into(),
            command_descriptor_hash: "cdh".into(),
            loop_controller_hash: "lch".into(),
            reviewer: "reviewer".into(),
            rationale: "ok".into(),
            feedback: None,
            acceptance_snapshot: WorkflowManualResultReviewAcceptanceSnapshot {
                accepts_reported_evidence: matches!(&decision, WorkflowManualResultReviewDecision::Accepted),
                verifies_external_state: false,
                reconciles_workflow_state: false,
                result_verified_by_openwand: false,
            },
            decision,
            verifies_external_state: false,
            reconciles_workflow_state: false,
            mutates_workflow_state: false,
            executes_command: false,
            invokes_shell: false,
            invokes_git: false,
            routes_action: false,
            resolves_approval: false,
            appends_trace: false,
            writes_memory: false,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        }
    }

    #[test]
    fn manual_result_review_roundtrips() {
        let r = test_review(WorkflowManualResultReviewDecision::Accepted);
        let json = serde_json::to_string(&r).unwrap();
        let back: WorkflowManualResultReview = serde_json::from_str(&json).unwrap();
        assert_eq!(r.review_id, back.review_id);
    }

    #[test]
    fn manual_result_review_id_is_content_addressed() {
        let id = WorkflowManualResultReviewId("wmrr_deadbeef1234".into());
        assert!(id.0.starts_with("wmrr_"));
    }

    #[test]
    fn manual_result_review_decision_serializes_snake_case() {
        assert!(serde_json::to_string(&WorkflowManualResultReviewDecision::Accepted).unwrap().contains("accepted"));
        assert!(serde_json::to_string(&WorkflowManualResultReviewDecision::Rejected).unwrap().contains("rejected"));
        assert!(serde_json::to_string(&WorkflowManualResultReviewDecision::ChangesRequested).unwrap().contains("changes_requested"));
    }

    #[test]
    fn manual_result_review_request_has_all_evidence_chain_hashes() {
        let req = WorkflowManualResultReviewRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            manual_result_id: WorkflowManualResultId("wmr_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            expected_manual_result_hash: "mrh".into(),
            expected_command_review_hash: "crh".into(),
            expected_command_composer_hash: "cch".into(),
            expected_command_descriptor_hash: "cdh".into(),
            expected_loop_controller_hash: "lch".into(),
            decision: WorkflowManualResultReviewDecision::Accepted,
            reviewer: "r".into(),
            rationale: "ok".into(),
            feedback: None,
            reviewed_at: Utc::now(),
            idempotency_key: "k".into(),
        };
        assert!(!req.expected_manual_result_hash.is_empty());
        assert!(!req.expected_command_review_hash.is_empty());
        assert!(!req.expected_command_composer_hash.is_empty());
        assert!(!req.expected_command_descriptor_hash.is_empty());
        assert!(!req.expected_loop_controller_hash.is_empty());
    }

    // Patch 3: explicit acceptance semantics
    #[test]
    fn accepted_review_accepts_reported_evidence_only() {
        let r = test_review(WorkflowManualResultReviewDecision::Accepted);
        assert!(r.acceptance_snapshot.accepts_reported_evidence);
        assert!(!r.acceptance_snapshot.verifies_external_state);
        assert!(!r.acceptance_snapshot.reconciles_workflow_state);
        assert!(!r.acceptance_snapshot.result_verified_by_openwand);
    }

    #[test]
    fn accepted_review_does_not_verify_external_state() {
        let r = test_review(WorkflowManualResultReviewDecision::Accepted);
        assert!(!r.verifies_external_state);
    }

    #[test]
    fn accepted_review_does_not_reconcile_workflow_state() {
        let r = test_review(WorkflowManualResultReviewDecision::Accepted);
        assert!(!r.reconciles_workflow_state);
        assert!(!r.mutates_workflow_state);
    }

    #[test]
    fn accepted_review_does_not_mark_result_true() {
        let r = test_review(WorkflowManualResultReviewDecision::Accepted);
        assert!(!r.acceptance_snapshot.result_verified_by_openwand);
    }

    #[test]
    fn manual_result_review_has_no_execution_authority() {
        let r = test_review(WorkflowManualResultReviewDecision::Accepted);
        assert!(!r.verifies_external_state);
        assert!(!r.reconciles_workflow_state);
        assert!(!r.mutates_workflow_state);
        assert!(!r.executes_command);
        assert!(!r.invokes_shell);
        assert!(!r.invokes_git);
        assert!(!r.routes_action);
        assert!(!r.resolves_approval);
        assert!(!r.appends_trace);
        assert!(!r.writes_memory);
        assert!(!r.creates_execution_grant);
        assert!(!r.execution_allowed_now);
    }

    #[test]
    fn feedback_roundtrips() {
        let fb = WorkflowManualResultReviewFeedback {
            summary: "issues".into(),
            blocking_reasons: vec!["risk".into()],
            requested_changes: vec!["fix".into()],
            evidence_gaps: vec!["trace".into()],
        };
        let json = serde_json::to_string(&fb).unwrap();
        let back: WorkflowManualResultReviewFeedback = serde_json::from_str(&json).unwrap();
        assert_eq!(1, back.blocking_reasons.len());
        assert_eq!(1, back.requested_changes.len());
        assert_eq!(1, back.evidence_gaps.len());
    }

    #[test]
    fn rejected_review_does_not_accept_reported_evidence() {
        let r = test_review(WorkflowManualResultReviewDecision::Rejected);
        assert!(!r.acceptance_snapshot.accepts_reported_evidence);
    }

    #[test]
    fn changes_requested_review_does_not_accept_reported_evidence() {
        let r = test_review(WorkflowManualResultReviewDecision::ChangesRequested);
        assert!(!r.acceptance_snapshot.accepts_reported_evidence);
    }

    #[test]
    fn acceptance_snapshot_roundtrips() {
        let snap = WorkflowManualResultReviewAcceptanceSnapshot {
            accepts_reported_evidence: true,
            verifies_external_state: false,
            reconciles_workflow_state: false,
            result_verified_by_openwand: false,
        };
        let json = serde_json::to_string(&snap).unwrap();
        let back: WorkflowManualResultReviewAcceptanceSnapshot = serde_json::from_str(&json).unwrap();
        assert!(back.accepts_reported_evidence);
        assert!(!back.verifies_external_state);
    }
}
