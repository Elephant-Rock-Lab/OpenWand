//! Manual result reconciliation readiness DTOs.
//!
//! Determines whether an accepted manual result review is ready to be reconciled
//! into workflow run revision evidence. Does NOT reconcile, verify external state,
//! create run revisions, or mutate workflow state.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_command_composer::WorkflowCommandComposerId;
use crate::workflow_command_review::WorkflowCommandReviewId;
use crate::workflow_loop_controller::WorkflowLoopControllerId;
use crate::workflow_manual_result::WorkflowManualResultId;
use crate::workflow_manual_result_review::WorkflowManualResultReviewId;
use crate::workflow_run::WorkflowExecutionId;

/// Content-addressed readiness ID. Format: wmrrr_<blake3_hex>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowManualResultReconciliationReadinessId(pub String);

/// Request to evaluate reconciliation readiness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualResultReconciliationReadinessRequest {
    pub workflow_execution_id: WorkflowExecutionId,
    pub manual_result_id: WorkflowManualResultId,
    pub manual_result_review_id: WorkflowManualResultReviewId,
    pub command_review_id: WorkflowCommandReviewId,
    pub command_composer_id: WorkflowCommandComposerId,
    pub loop_controller_id: WorkflowLoopControllerId,
    // Patch 1: full evidence chain hashes
    pub expected_manual_result_review_hash: String,
    pub expected_manual_result_hash: String,
    pub expected_command_review_hash: String,
    pub expected_command_composer_hash: String,
    pub expected_command_descriptor_hash: String,
    pub expected_loop_controller_hash: String,
    pub evaluator: String,
    pub evaluated_at: DateTime<Utc>,
    pub idempotency_key: String,
}

/// Sealed readiness evidence record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualResultReconciliationReadinessRecord {
    pub readiness_id: WorkflowManualResultReconciliationReadinessId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub manual_result_id: WorkflowManualResultId,
    pub manual_result_review_id: WorkflowManualResultReviewId,
    pub command_review_id: WorkflowCommandReviewId,
    pub command_composer_id: WorkflowCommandComposerId,
    pub loop_controller_id: WorkflowLoopControllerId,
    pub manual_result_review_hash: String,
    pub manual_result_hash: String,
    pub command_review_hash: String,
    pub command_composer_hash: String,
    pub command_descriptor_hash: String,
    pub loop_controller_hash: String,
    pub status: WorkflowManualResultReconciliationReadinessStatus,
    pub decision: WorkflowManualResultReconciliationReadinessDecision,
    pub predicates: Vec<WorkflowManualResultReconciliationReadinessPredicateResult>,
    // Patch 3: reconciliation preview — what reconciliation would do
    pub reconciliation_preview: Option<WorkflowManualResultReconciliationPreview>,
    // Patch 4: hardcoded-false authority flags
    pub verifies_external_state: bool,
    pub reconciles_now: bool,
    pub mutates_workflow_state: bool,
    pub creates_run_revision: bool,
    pub appends_trace: bool,
    pub writes_memory: bool,
    pub routes_action: bool,
    pub resolves_approval: bool,
    pub creates_execution_grant: bool,
    pub execution_allowed_now: bool,
    pub evaluator: String,
    pub evaluated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowManualResultReconciliationReadinessStatus {
    Ready,
    Blocked,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowManualResultReconciliationReadinessDecision {
    Ready { summary: String },
    Blocked { reason_code: String, summary: String },
    Inconclusive { reason_code: String, summary: String },
}

/// Patch 3: preview of what reconciliation would target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "target", rename_all = "snake_case")]
pub enum WorkflowManualResultReconciliationPreview {
    CompleteStageFromReportedSuccess,
    BlockStageFromReportedFailure,
    FailStageFromReportedFailure,
    PartialResultRequiresReview,
    NotPerformedBlocksReconciliation,
    InconclusiveBlocksReconciliation,
}

/// Readiness predicates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowManualResultReconciliationReadinessPredicate {
    ManualResultRecordExists,
    ManualResultReviewExists,
    ReviewDecisionIsAccepted,
    ManualResultWasReportedByOperator,
    ManualResultNotVerifiedByOpenwand,
    ReviewAcceptsReportedEvidenceOnly,
    ManualResultReviewHashMatchesRequest,
    ManualResultHashMatchesRequest,
    CommandReviewHashMatchesRequest,
    CommandComposerHashMatchesRequest,
    CommandDescriptorHashMatchesRequest,
    LoopControllerHashMatchesRequest,
    ManualResultReviewIsLatest,
    NoPriorConflictingReconciliationReadiness,
}

/// Result of evaluating one readiness predicate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualResultReconciliationReadinessPredicateResult {
    pub predicate: WorkflowManualResultReconciliationReadinessPredicate,
    pub passed: bool,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_id_is_content_addressed() {
        let id = WorkflowManualResultReconciliationReadinessId("wmrrr_deadbeef1234".into());
        assert!(id.0.starts_with("wmrrr_"));
    }

    #[test]
    fn readiness_status_serializes_snake_case() {
        assert!(serde_json::to_string(&WorkflowManualResultReconciliationReadinessStatus::Ready).unwrap().contains("ready"));
        assert!(serde_json::to_string(&WorkflowManualResultReconciliationReadinessStatus::Blocked).unwrap().contains("blocked"));
        assert!(serde_json::to_string(&WorkflowManualResultReconciliationReadinessStatus::Inconclusive).unwrap().contains("inconclusive"));
    }

    #[test]
    fn readiness_decision_roundtrips() {
        let d = WorkflowManualResultReconciliationReadinessDecision::Ready { summary: "ok".into() };
        let json = serde_json::to_string(&d).unwrap();
        let back: WorkflowManualResultReconciliationReadinessDecision = serde_json::from_str(&json).unwrap();
        assert!(format!("{:?}", back).contains("Ready"));
    }

    #[test]
    fn reconciliation_preview_roundtrips() {
        let p = WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess;
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("complete_stage_from_reported_success"));
    }

    // Patch 4: no authority tests
    #[test]
    fn readiness_record_has_no_reconciliation_authority() {
        let r = test_ready_record();
        assert!(!r.verifies_external_state);
        assert!(!r.reconciles_now);
        assert!(!r.mutates_workflow_state);
        assert!(!r.creates_run_revision);
        assert!(!r.appends_trace);
        assert!(!r.writes_memory);
        assert!(!r.routes_action);
        assert!(!r.resolves_approval);
        assert!(!r.creates_execution_grant);
        assert!(!r.execution_allowed_now);
    }

    #[test]
    fn ready_readiness_does_not_create_run_revision() {
        let r = test_ready_record();
        assert!(!r.creates_run_revision);
    }

    #[test]
    fn ready_readiness_does_not_verify_external_state() {
        let r = test_ready_record();
        assert!(!r.verifies_external_state);
    }

    #[test]
    fn ready_readiness_does_not_allow_execution_now() {
        let r = test_ready_record();
        assert!(!r.execution_allowed_now);
    }

    fn test_ready_record() -> WorkflowManualResultReconciliationReadinessRecord {
        WorkflowManualResultReconciliationReadinessRecord {
            readiness_id: WorkflowManualResultReconciliationReadinessId("wmrrr_test".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            manual_result_id: WorkflowManualResultId("wmr_t".into()),
            manual_result_review_id: WorkflowManualResultReviewId("wmrr_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            manual_result_review_hash: "rrh".into(),
            manual_result_hash: "mrh".into(),
            command_review_hash: "crh".into(),
            command_composer_hash: "cch".into(),
            command_descriptor_hash: "cdh".into(),
            loop_controller_hash: "lch".into(),
            status: WorkflowManualResultReconciliationReadinessStatus::Ready,
            decision: WorkflowManualResultReconciliationReadinessDecision::Ready { summary: "All predicates pass".into() },
            predicates: vec![],
            reconciliation_preview: Some(WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess),
            verifies_external_state: false,
            reconciles_now: false,
            mutates_workflow_state: false,
            creates_run_revision: false,
            appends_trace: false,
            writes_memory: false,
            routes_action: false,
            resolves_approval: false,
            creates_execution_grant: false,
            execution_allowed_now: false,
            evaluator: "test".into(),
            evaluated_at: Utc::now(),
        }
    }
}
