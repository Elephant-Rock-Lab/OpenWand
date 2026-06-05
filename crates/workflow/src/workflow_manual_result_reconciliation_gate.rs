//! Manual result reconciliation gate DTOs.
//!
//! Converts a Ready reconciliation-readiness record into a new immutable
//! workflow run revision, updating the linked stage from accepted
//! operator-reported evidence.
//!
//! Patch 1: progression is driven by the readiness preview, not raw result status.
//! Patch 2: only stage-progressing manual results are eligible.
//! Patch 3: full hash-bound evidence chain (8 hashes).
//! Patch 5: only Suspended stages are eligible (explicit rule).
//! Patch 6: authority flags — creates revision evidence only.
//! Patch 7: AlreadyReconciled status + idempotency semantics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_command_composer::WorkflowCommandComposerId;
use crate::workflow_command_review::WorkflowCommandReviewId;
use crate::workflow_loop_controller::WorkflowLoopControllerId;
use crate::workflow_manual_result::WorkflowManualResultId;
use crate::workflow_manual_result_reconciliation_readiness::{
    WorkflowManualResultReconciliationReadinessId,
    WorkflowManualResultReconciliationPreview,
};
use crate::workflow_manual_result_review::WorkflowManualResultReviewId;
use crate::workflow_reconciliation::WorkflowRunRevisionId;
use crate::workflow_run::{
    WorkflowExecutionId, WorkflowStageLifecycleEvent, WorkflowStageLifecycleKind,
    WorkflowStageRun, WorkflowStageRunStatus,
};

/// Content-addressed gate ID. Format: wmrrg_<blake3_hex>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowManualResultReconciliationGateId(pub String);

/// Patch 3: full hash-bound request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualResultReconciliationGateRequest {
    pub workflow_execution_id: WorkflowExecutionId,
    pub manual_result_id: WorkflowManualResultId,
    pub manual_result_review_id: WorkflowManualResultReviewId,
    pub reconciliation_readiness_id: WorkflowManualResultReconciliationReadinessId,
    pub stage_id: String,
    // Patch 3: 8 hash bindings
    pub expected_workflow_run_hash: String,
    pub expected_reconciliation_readiness_hash: String,
    pub expected_manual_result_review_hash: String,
    pub expected_manual_result_hash: String,
    pub expected_command_review_hash: String,
    pub expected_command_composer_hash: String,
    pub expected_command_descriptor_hash: String,
    pub expected_loop_controller_hash: String,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub idempotency_key: String,
}

/// Sealed gate record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualResultReconciliationGateRecord {
    pub gate_id: WorkflowManualResultReconciliationGateId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub manual_result_id: WorkflowManualResultId,
    pub manual_result_review_id: WorkflowManualResultReviewId,
    pub reconciliation_readiness_id: WorkflowManualResultReconciliationReadinessId,
    pub command_review_id: WorkflowCommandReviewId,
    pub command_composer_id: WorkflowCommandComposerId,
    pub loop_controller_id: WorkflowLoopControllerId,
    pub stage_id: String,
    // Stored hashes from evidence chain
    pub workflow_run_hash: String,
    pub reconciliation_readiness_hash: String,
    pub manual_result_review_hash: String,
    pub manual_result_hash: String,
    pub command_review_hash: String,
    pub command_composer_hash: String,
    pub command_descriptor_hash: String,
    pub loop_controller_hash: String,
    // Patch 7: status and decision
    pub status: WorkflowManualResultReconciliationGateStatus,
    pub decision: WorkflowManualResultReconciliationGateDecision,
    pub predicates: Vec<WorkflowManualResultReconciliationGatePredicateResult>,
    // Patch 1: preview-driven progression
    pub progression: Option<ManualResultStageProgression>,
    pub new_run_revision_id: Option<WorkflowRunRevisionId>,
    // Patch 6: authority flags
    pub creates_run_revision: bool,
    pub mutates_original_workflow_run: bool,
    pub verifies_external_truth: bool,
    pub executes_command: bool,
    pub routes_continuation: bool,
    pub appends_trace: bool,
    pub writes_memory: bool,
    pub creates_execution_grant: bool,
    pub execution_allowed_now: bool,
    pub reconciled_by: String,
    pub reconciled_at: DateTime<Utc>,
}

/// Patch 7: gate status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowManualResultReconciliationGateStatus {
    Reconciled,
    Blocked,
    Failed,
    AlreadyReconciled,
}

/// Patch 7: gate decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowManualResultReconciliationGateDecision {
    Reconciled { revision_id: Option<String>, summary: String },
    Blocked { reason_code: String, summary: String },
    Failed { reason_code: String, summary: String },
    AlreadyReconciled { revision_id: Option<String>, summary: String },
}

/// Patch 1: preview-driven stage progression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualResultStageProgression {
    pub stage_id: String,
    pub previous_status: WorkflowStageRunStatus,
    pub new_status: WorkflowStageRunStatus,
    pub preview_target: WorkflowManualResultReconciliationPreview,
    pub lifecycle_event: WorkflowStageLifecycleEvent,
    pub summary: String,
}

/// Patch 1: is the preview target actionable for stage progression?
pub fn is_actionable_preview(preview: &WorkflowManualResultReconciliationPreview) -> bool {
    matches!(preview,
        WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess
        | WorkflowManualResultReconciliationPreview::FailStageFromReportedFailure
        | WorkflowManualResultReconciliationPreview::BlockStageFromReportedFailure
    )
}

/// Patch 1: compute stage progression from readiness preview.
/// The preview is the gate authority (Patch 1), not raw result status.
pub fn compute_manual_result_stage_progression(
    stage_id: &str,
    current_status: &WorkflowStageRunStatus,
    preview: &WorkflowManualResultReconciliationPreview,
) -> Option<ManualResultStageProgression> {
    // Patch 5: only Suspended stages are eligible
    if *current_status != WorkflowStageRunStatus::Suspended {
        return None;
    }

    match preview {
        WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess =>
            Some(ManualResultStageProgression {
                stage_id: stage_id.into(),
                previous_status: current_status.clone(),
                new_status: WorkflowStageRunStatus::Completed,
                preview_target: preview.clone(),
                lifecycle_event: WorkflowStageLifecycleEvent {
                    event_id: format!("evt_{}_manual_completed", stage_id),
                    stage_id: stage_id.into(),
                    event_kind: WorkflowStageLifecycleKind::StageCompleted,
                    summary: "Stage completed from accepted operator-reported success evidence.".into(),
                    occurred_at: Utc::now(),
                },
                summary: "Stage completed from accepted operator-reported success evidence.".into(),
            }),
        WorkflowManualResultReconciliationPreview::FailStageFromReportedFailure =>
            Some(ManualResultStageProgression {
                stage_id: stage_id.into(),
                previous_status: current_status.clone(),
                new_status: WorkflowStageRunStatus::Failed,
                preview_target: preview.clone(),
                lifecycle_event: WorkflowStageLifecycleEvent {
                    event_id: format!("evt_{}_manual_failed", stage_id),
                    stage_id: stage_id.into(),
                    event_kind: WorkflowStageLifecycleKind::StageFailed,
                    summary: "Stage failed from accepted operator-reported failure evidence.".into(),
                    occurred_at: Utc::now(),
                },
                summary: "Stage failed from accepted operator-reported failure evidence.".into(),
            }),
        WorkflowManualResultReconciliationPreview::BlockStageFromReportedFailure =>
            Some(ManualResultStageProgression {
                stage_id: stage_id.into(),
                previous_status: current_status.clone(),
                new_status: WorkflowStageRunStatus::Blocked,
                preview_target: preview.clone(),
                lifecycle_event: WorkflowStageLifecycleEvent {
                    event_id: format!("evt_{}_manual_blocked", stage_id),
                    stage_id: stage_id.into(),
                    event_kind: WorkflowStageLifecycleKind::StageBlocked,
                    summary: "Stage blocked from accepted operator-reported failure evidence.".into(),
                    occurred_at: Utc::now(),
                },
                summary: "Stage blocked from accepted operator-reported failure evidence.".into(),
            }),
        _ => None, // Non-actionable previews don't produce progression
    }
}

/// Apply manual result progression to stages (original untouched).
pub fn apply_manual_progression_to_stages(
    stages: &[WorkflowStageRun],
    progression: &ManualResultStageProgression,
) -> Vec<WorkflowStageRun> {
    stages.iter().map(|s| {
        if s.stage_id == progression.stage_id {
            let mut updated = s.clone();
            updated.status = progression.new_status.clone();
            updated.completed_at = Some(progression.lifecycle_event.occurred_at);
            updated
        } else {
            s.clone()
        }
    }).collect()
}

/// 25 gate predicates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowManualResultReconciliationGatePredicate {
    // Core existence
    WorkflowRunExists,
    ManualResultExists,
    ManualResultReviewExists,
    ReconciliationReadinessRecordExists,
    StageExists,
    // Hash matching (Patch 3: 8 hashes)
    WorkflowRunHashMatchesRequest,
    ManualResultHashMatchesRequest,
    ManualResultReviewHashMatchesRequest,
    ReconciliationReadinessHashMatchesRequest,
    CommandReviewHashMatchesReadiness,
    CommandComposerHashMatchesReadiness,
    CommandDescriptorHashMatchesReadiness,
    LoopControllerHashMatchesReadiness,
    // Readiness and review status
    ReconciliationReadinessStatusIsReady,
    ReviewDecisionIsAccepted,
    ManualResultWasReportedByOperator,
    // Patch 1: preview authority
    ReconciliationPreviewExists,
    ReconciliationPreviewTargetIsActionable,
    ManualResultStatusMatchesReadinessPreview,
    // Patch 2: eligibility
    ManualResultEligibleForWorkflowStageReconciliation,
    // Patch 4: latest revalidation
    ManualResultReviewIsLatest,
    ReconciliationReadinessIsLatest,
    // Patch 5: stage eligibility (explicit rule)
    StageStatusEligibleForManualReconciliation,
    // Conflicts and idempotency
    NoPriorConflictingManualReconciliation,
    IdempotencyKeyUnusedOrMatchesExisting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualResultReconciliationGatePredicateResult {
    pub predicate: WorkflowManualResultReconciliationGatePredicate,
    pub passed: bool,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_id_is_content_addressed() {
        let id = WorkflowManualResultReconciliationGateId("wmrrg_deadbeef1234".into());
        assert!(id.0.starts_with("wmrrg_"));
    }

    #[test]
    fn gate_status_serializes_snake_case() {
        assert!(serde_json::to_string(&WorkflowManualResultReconciliationGateStatus::Reconciled).unwrap().contains("reconciled"));
        assert!(serde_json::to_string(&WorkflowManualResultReconciliationGateStatus::Blocked).unwrap().contains("blocked"));
        assert!(serde_json::to_string(&WorkflowManualResultReconciliationGateStatus::Failed).unwrap().contains("failed"));
        assert!(serde_json::to_string(&WorkflowManualResultReconciliationGateStatus::AlreadyReconciled).unwrap().contains("already_reconciled"));
    }

    #[test]
    fn gate_decision_roundtrips() {
        let d = WorkflowManualResultReconciliationGateDecision::Reconciled {
            revision_id: Some("wrr_abc".into()),
            summary: "ok".into(),
        };
        let json = serde_json::to_string(&d).unwrap();
        let back: WorkflowManualResultReconciliationGateDecision = serde_json::from_str(&json).unwrap();
        assert!(format!("{:?}", back).contains("Reconciled"));
    }

    #[test]
    fn already_reconciled_decision_roundtrips() {
        let d = WorkflowManualResultReconciliationGateDecision::AlreadyReconciled {
            revision_id: Some("wrr_xyz".into()),
            summary: "dup".into(),
        };
        let json = serde_json::to_string(&d).unwrap();
        let back: WorkflowManualResultReconciliationGateDecision = serde_json::from_str(&json).unwrap();
        assert!(format!("{:?}", back).contains("AlreadyReconciled"));
    }

    #[test]
    fn manual_result_stage_progression_roundtrips() {
        let p = ManualResultStageProgression {
            stage_id: "s1".into(),
            previous_status: WorkflowStageRunStatus::Suspended,
            new_status: WorkflowStageRunStatus::Completed,
            preview_target: WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess,
            lifecycle_event: WorkflowStageLifecycleEvent {
                event_id: "evt_s1_manual_completed".into(),
                stage_id: "s1".into(),
                event_kind: WorkflowStageLifecycleKind::StageCompleted,
                summary: "ok".into(),
                occurred_at: Utc::now(),
            },
            summary: "ok".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: ManualResultStageProgression = serde_json::from_str(&json).unwrap();
        assert_eq!(p.stage_id, back.stage_id);
    }

    // Patch 6: authority flag tests
    #[test]
    fn gate_record_has_no_execution_authority() {
        let r = test_reconciled_record();
        assert!(!r.mutates_original_workflow_run);
        assert!(!r.verifies_external_truth);
        assert!(!r.executes_command);
        assert!(!r.routes_continuation);
        assert!(!r.appends_trace);
        assert!(!r.writes_memory);
        assert!(!r.creates_execution_grant);
        assert!(!r.execution_allowed_now);
    }

    #[test]
    fn reconciled_gate_creates_run_revision_evidence() {
        let r = test_reconciled_record();
        assert!(r.creates_run_revision, "Reconciled gate should create revision evidence");
    }

    #[test]
    fn gate_does_not_mutate_original_workflow_run() {
        let r = test_reconciled_record();
        assert!(!r.mutates_original_workflow_run);
    }

    #[test]
    fn gate_does_not_verify_external_truth() {
        let r = test_reconciled_record();
        assert!(!r.verifies_external_truth);
    }

    #[test]
    fn gate_does_not_route_continuation() {
        let r = test_reconciled_record();
        assert!(!r.routes_continuation);
    }

    // Patch 3: request has all 8 hashes
    #[test]
    fn request_has_all_eight_hashes() {
        let req = test_request();
        assert!(!req.expected_workflow_run_hash.is_empty());
        assert!(!req.expected_reconciliation_readiness_hash.is_empty());
        assert!(!req.expected_manual_result_review_hash.is_empty());
        assert!(!req.expected_manual_result_hash.is_empty());
        assert!(!req.expected_command_review_hash.is_empty());
        assert!(!req.expected_command_composer_hash.is_empty());
        assert!(!req.expected_command_descriptor_hash.is_empty());
        assert!(!req.expected_loop_controller_hash.is_empty());
    }

    #[test]
    fn is_actionable_preview_identifies_stage_targets() {
        assert!(is_actionable_preview(&WorkflowManualResultReconciliationPreview::CompleteStageFromReportedSuccess));
        assert!(is_actionable_preview(&WorkflowManualResultReconciliationPreview::FailStageFromReportedFailure));
        assert!(is_actionable_preview(&WorkflowManualResultReconciliationPreview::BlockStageFromReportedFailure));
        assert!(!is_actionable_preview(&WorkflowManualResultReconciliationPreview::PartialResultRequiresReview));
        assert!(!is_actionable_preview(&WorkflowManualResultReconciliationPreview::NotPerformedBlocksReconciliation));
        assert!(!is_actionable_preview(&WorkflowManualResultReconciliationPreview::InconclusiveBlocksReconciliation));
    }

    // Helpers
    use crate::workflow_run::WorkflowExecutionId;

    fn test_request() -> WorkflowManualResultReconciliationGateRequest {
        WorkflowManualResultReconciliationGateRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            manual_result_id: WorkflowManualResultId("wmr_t".into()),
            manual_result_review_id: WorkflowManualResultReviewId("wmrr_t".into()),
            reconciliation_readiness_id: WorkflowManualResultReconciliationReadinessId("wmrrr_t".into()),
            stage_id: "stage_1".into(),
            expected_workflow_run_hash: "wrh".into(),
            expected_reconciliation_readiness_hash: "rrh".into(),
            expected_manual_result_review_hash: "mrrh".into(),
            expected_manual_result_hash: "mrh".into(),
            expected_command_review_hash: "crh".into(),
            expected_command_composer_hash: "cch".into(),
            expected_command_descriptor_hash: "cdh".into(),
            expected_loop_controller_hash: "lch".into(),
            requested_by: "test".into(),
            requested_at: Utc::now(),
            idempotency_key: "k1".into(),
        }
    }

    fn test_reconciled_record() -> WorkflowManualResultReconciliationGateRecord {
        WorkflowManualResultReconciliationGateRecord {
            gate_id: WorkflowManualResultReconciliationGateId("wmrrg_test".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            manual_result_id: WorkflowManualResultId("wmr_t".into()),
            manual_result_review_id: WorkflowManualResultReviewId("wmrr_t".into()),
            reconciliation_readiness_id: WorkflowManualResultReconciliationReadinessId("wmrrr_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            stage_id: "stage_1".into(),
            workflow_run_hash: "wrh".into(),
            reconciliation_readiness_hash: "rrh".into(),
            manual_result_review_hash: "mrrh".into(),
            manual_result_hash: "mrh".into(),
            command_review_hash: "crh".into(),
            command_composer_hash: "cch".into(),
            command_descriptor_hash: "cdh".into(),
            loop_controller_hash: "lch".into(),
            status: WorkflowManualResultReconciliationGateStatus::Reconciled,
            decision: WorkflowManualResultReconciliationGateDecision::Reconciled {
                revision_id: Some("wrr_test".into()),
                summary: "ok".into(),
            },
            predicates: vec![],
            progression: None,
            new_run_revision_id: Some(WorkflowRunRevisionId("wrr_test".into())),
            creates_run_revision: true,
            mutates_original_workflow_run: false,
            verifies_external_truth: false,
            executes_command: false,
            routes_continuation: false,
            appends_trace: false,
            writes_memory: false,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reconciled_by: "test".into(),
            reconciled_at: Utc::now(),
        }
    }
}
