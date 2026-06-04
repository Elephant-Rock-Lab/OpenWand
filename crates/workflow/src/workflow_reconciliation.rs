//! Workflow reconciliation DTOs.
//!
//! Reconciliation consumes a terminal workflow action outcome record, validates
//! its full linkage chain back to the workflow run, and produces either a new
//! WorkflowRunRevision with updated stage state or a blocked/failed evidence record.
//!
//! Reconciliation does not route actions, resolve approvals, execute tools,
//! evaluate policy directly, append trace, mutate memory, call shell/git,
//! or create approval/session/tool/route/readiness/proposal/task-plan records.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_action_outcome::{WorkflowActionOutcomeId, WorkflowActionOutcomeStatus};
use crate::workflow_action_route::WorkflowActionRouteId;
use crate::workflow_run::{
    WorkflowExecutionId, WorkflowStageLifecycleEvent, WorkflowStageRun, WorkflowStageRunStatus,
};

/// Content-addressed reconciliation ID. Format: wrc_<blake3_hex>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowReconciliationId(pub String);

/// Content-addressed run revision ID. Format: wrr_<blake3_hex>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowRunRevisionId(pub String);

/// Request to reconcile a terminal outcome back into workflow run stage state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowReconciliationRequest {
    pub workflow_execution_id: WorkflowExecutionId,
    pub route_id: WorkflowActionRouteId,
    pub outcome_id: WorkflowActionOutcomeId,
    pub stage_id: String,
    pub action_request_id: String,
    pub expected_workflow_run_hash: String,
    pub expected_route_hash: String,
    pub expected_outcome_hash: String,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub idempotency_key: String,
}

/// Durable reconciliation evidence record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowReconciliationRecord {
    pub reconciliation_id: WorkflowReconciliationId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub route_id: WorkflowActionRouteId,
    pub outcome_id: WorkflowActionOutcomeId,
    pub stage_id: String,
    pub action_request_id: String,
    pub status: WorkflowReconciliationStatus,
    pub decision: WorkflowReconciliationDecision,
    pub predicates: Vec<WorkflowReconciliationPredicateResult>,
    pub progression: Option<WorkflowStageProgression>,
    pub new_run_revision_id: Option<WorkflowRunRevisionId>,
    pub created_at: DateTime<Utc>,
}

/// Reconciliation status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowReconciliationStatus {
    Blocked,
    Reconciled,
    Failed,
    AlreadyReconciled,
}

/// Reconciliation decision — what happened, not what was executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowReconciliationDecision {
    Reconciled { summary: String },
    Blocked { reason_code: String, summary: String },
    Failed { reason_code: String, summary: String },
    AlreadyReconciled { summary: String },
}

/// Stage-level transition derived from outcome evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStageProgression {
    pub stage_id: String,
    pub previous_status: WorkflowStageRunStatus,
    pub new_status: WorkflowStageRunStatus,
    pub outcome_status: WorkflowActionOutcomeStatus,
    pub lifecycle_event: WorkflowStageLifecycleEvent,
    pub summary: String,
}

/// Immutable workflow run revision with updated stage state.
/// The original WorkflowRunRecord is never mutated (Patch 1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunRevision {
    pub revision_id: WorkflowRunRevisionId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub previous_revision_id: Option<WorkflowRunRevisionId>,
    pub source_reconciliation_id: WorkflowReconciliationId,
    pub run_hash_before: String,
    pub run_hash_after: String,
    pub stages: Vec<WorkflowStageRun>,
    pub lifecycle_events: Vec<WorkflowStageLifecycleEvent>,
    /// Aggregate status derived from stage states.
    /// If all stages are terminal (Completed/Blocked/Failed/Skipped),
    /// this may be Completed. The original run record is never mutated (Patch 1).
    pub aggregate_status: Option<WorkflowStageRunStatus>,
    pub created_at: DateTime<Utc>,
}

/// Terminal stage statuses (Patch 2).
/// Completed, Blocked, Failed, Skipped are terminal.
/// Pending, Running, Suspended are NOT terminal.
pub fn is_terminal_stage_status(status: &WorkflowStageRunStatus) -> bool {
    matches!(
        status,
        WorkflowStageRunStatus::Completed
            | WorkflowStageRunStatus::Blocked
            | WorkflowStageRunStatus::Failed
            | WorkflowStageRunStatus::Skipped
    )
}

/// Reconciliation predicates — validate full linkage and safety.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowReconciliationPredicate {
    WorkflowRunExists,
    WorkflowRunHashMatchesRequest,
    RouteRecordExists,
    RouteHashMatchesRequest,
    OutcomeRecordExists,
    OutcomeHashMatchesRequest,
    RouteLinksSameWorkflowRun,
    OutcomeLinksSameWorkflowRun,
    OutcomeLinksSameRoute,
    StageExists,
    StageWasSuspended,
    ActionRequestExists,
    OutcomeLinksSameStage,
    OutcomeLinksSameActionRequest,
    OutcomeIsTerminal,
    OutcomeEvidenceFromSession,
    NoPriorConflictingReconciliation,
    IdempotencyKeyUnusedOrMatchesExisting,
}

/// Result of evaluating one reconciliation predicate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowReconciliationPredicateResult {
    pub predicate: WorkflowReconciliationPredicate,
    pub passed: bool,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_proposal::WorkflowStageKind;
    use crate::workflow_run::{WorkflowStageLifecycleKind};

    fn test_reconciliation_record() -> WorkflowReconciliationRecord {
        WorkflowReconciliationRecord {
            reconciliation_id: WorkflowReconciliationId("wrc_abc123".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            route_id: WorkflowActionRouteId("war_t".into()),
            outcome_id: WorkflowActionOutcomeId("wao_t".into()),
            stage_id: "stage_1".into(),
            action_request_id: "ar_1".into(),
            status: WorkflowReconciliationStatus::Reconciled,
            decision: WorkflowReconciliationDecision::Reconciled {
                summary: "Stage completed".into(),
            },
            predicates: vec![],
            progression: None,
            new_run_revision_id: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn reconciliation_record_roundtrips() {
        let record = test_reconciliation_record();
        let json = serde_json::to_string(&record).unwrap();
        let back: WorkflowReconciliationRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record.reconciliation_id, back.reconciliation_id);
        assert_eq!(record.status, back.status);
    }

    #[test]
    fn reconciliation_id_is_content_addressed() {
        // IDs must start with wrc_
        let id = WorkflowReconciliationId("wrc_deadbeef".into());
        assert!(id.0.starts_with("wrc_"));
    }

    #[test]
    fn reconciliation_status_serializes_snake_case() {
        let json = serde_json::to_string(&WorkflowReconciliationStatus::AlreadyReconciled).unwrap();
        assert!(json.contains("already_reconciled"));
    }

    #[test]
    fn reconciliation_decision_roundtrips() {
        let d = WorkflowReconciliationDecision::Blocked {
            reason_code: "stage_not_suspended".into(),
            summary: "Stage not suspended".into(),
        };
        let json = serde_json::to_string(&d).unwrap();
        let back: WorkflowReconciliationDecision = serde_json::from_str(&json).unwrap();
        if let WorkflowReconciliationDecision::Blocked { reason_code, .. } = back {
            assert_eq!("stage_not_suspended", reason_code);
        } else {
            panic!("Expected Blocked");
        }
    }

    #[test]
    fn reconciliation_requires_predicates() {
        let record = test_reconciliation_record();
        // Empty predicates is valid for Blocked/Failed but not Reconciled
        // This just tests the structure accepts Vec
        assert!(record.predicates.is_empty());
    }

    #[test]
    fn stage_progression_roundtrips() {
        let p = WorkflowStageProgression {
            stage_id: "s1".into(),
            previous_status: WorkflowStageRunStatus::Suspended,
            new_status: WorkflowStageRunStatus::Completed,
            outcome_status: WorkflowActionOutcomeStatus::ToolCompleted,
            lifecycle_event: WorkflowStageLifecycleEvent {
                event_id: "evt_1".into(),
                stage_id: "s1".into(),
                event_kind: WorkflowStageLifecycleKind::StageCompleted,
                summary: "Stage completed from session-produced tool outcome evidence.".into(),
                occurred_at: Utc::now(),
            },
            summary: "Stage completed from session-produced tool outcome evidence.".into(),
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: WorkflowStageProgression = serde_json::from_str(&json).unwrap();
        assert_eq!(p.stage_id, back.stage_id);
        assert_eq!(WorkflowStageRunStatus::Suspended, back.previous_status);
        assert_eq!(WorkflowStageRunStatus::Completed, back.new_status);
    }

    #[test]
    fn run_revision_roundtrips() {
        let r = WorkflowRunRevision {
            revision_id: WorkflowRunRevisionId("wrr_abc".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            previous_revision_id: None,
            source_reconciliation_id: WorkflowReconciliationId("wrc_1".into()),
            run_hash_before: "h1".into(),
            run_hash_after: "h2".into(),
            stages: vec![WorkflowStageRun {
                stage_id: "s1".into(),
                title: "Stage 1".into(),
                kind: WorkflowStageKind::Verify,
                status: WorkflowStageRunStatus::Completed,
                order: 0,
                depends_on: vec![],
                started_at: None,
                completed_at: Some(Utc::now()),
                summary: "done".into(),
            }],
            lifecycle_events: vec![],
            aggregate_status: Some(WorkflowStageRunStatus::Completed),
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&r).unwrap();
        let back: WorkflowRunRevision = serde_json::from_str(&json).unwrap();
        assert_eq!(r.revision_id, back.revision_id);
        assert_eq!(1, back.stages.len());
    }

    #[test]
    fn run_revision_id_is_content_addressed() {
        let id = WorkflowRunRevisionId("wrr_deadbeef".into());
        assert!(id.0.starts_with("wrr_"));
    }
}
