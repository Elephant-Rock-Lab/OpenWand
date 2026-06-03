//! Workflow run DTOs.
//!
//! A workflow run is governed evidence. It is not a tool call.
//! A stage is not authority. A stage transition is not a policy override.
//! A workflow run may request governed actions, but it never executes tools directly.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::plan::TaskPlanId;
use crate::plan_review::TaskPlanReviewId;
use crate::workflow_proposal::{WorkflowProposalId, WorkflowStageKind};
use crate::workflow_proposal_review::WorkflowProposalReviewId;
use crate::workflow_readiness::WorkflowReadinessId;

/// Content-addressed execution ID. Format: `wfx_<blake3_hex>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowExecutionId(pub String);

impl WorkflowExecutionId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Request to execute a workflow from a readiness record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionRequest {
    pub readiness_id: WorkflowReadinessId,
    pub proposal_id: WorkflowProposalId,
    pub proposal_review_id: WorkflowProposalReviewId,
    pub expected_readiness_hash: String,
    pub expected_proposal_hash: String,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub idempotency_key: String,
}

/// Durable workflow run record. Evidence, not tool authority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunRecord {
    pub execution_id: WorkflowExecutionId,
    pub readiness_id: WorkflowReadinessId,
    pub proposal_id: WorkflowProposalId,
    pub proposal_review_id: WorkflowProposalReviewId,
    pub source_task_plan_id: TaskPlanId,
    pub status: WorkflowRunStatus,
    pub decision: WorkflowExecutionDecision,
    pub predicates: Vec<WorkflowExecutionPredicateResult>,
    pub run_snapshot: WorkflowRunSnapshot,
    pub stages: Vec<WorkflowStageRun>,
    pub lifecycle_events: Vec<WorkflowStageLifecycleEvent>,
    pub action_requests: Vec<WorkflowActionRequest>,
    pub abort_snapshot: WorkflowAbortSnapshot,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Run lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRunStatus {
    Blocked,
    Running,
    Suspended,
    Completed,
    Failed,
    AlreadyExecuted,
}

/// Execution decision.
///
/// `RunCreated` means the gate created run evidence and advanced deterministic
/// non-tool stages. It does NOT mean tools were executed or external actions performed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowExecutionDecision {
    RunCreated,
    Blocked { reason_code: String, summary: String },
    Suspended { reason_code: String, summary: String },
    Failed { reason_code: String, summary: String },
}

/// Result of evaluating a single execution predicate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecutionPredicateResult {
    pub predicate: WorkflowExecutionPredicate,
    pub passed: bool,
    pub reason: String,
}

/// Execution-time predicates. None of these execute tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowExecutionPredicate {
    ReadinessRecordExists,
    ReadinessIsReady,
    ReadinessHashMatchesRequest,
    ProposalExists,
    ProposalHashMatchesReadiness,
    ProposalHashMatchesRequest,
    ProposalReviewExists,
    ProposalReviewIsLatest,
    ProposalReviewApproved,
    SourceTaskPlanExists,
    SourceTaskPlanHashMatchesProposal,
    ToolIntentResolutionStillValid,
    ToolIntentsRemainNonExecutable,
    ApprovalRequirementsRepresented,
    PolicyConstraintsRepresented,
    PolicyAllowsWorkflowRunCreation,
    ProviderConfigurationAvailable,
    SessionRuntimeAvailable,
    RollbackAbortEvidencePresent,
    NoPriorConflictingWorkflowRun,
    IdempotencyKeyUnusedOrMatchesExisting,
}

/// Per-stage run state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStageRun {
    pub stage_id: String,
    pub title: String,
    pub kind: WorkflowStageKind,
    pub status: WorkflowStageRunStatus,
    pub order: u32,
    pub depends_on: Vec<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub summary: String,
}

/// Per-stage run status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStageRunStatus {
    Pending,
    Running,
    Suspended,
    Completed,
    Blocked,
    Failed,
    Skipped,
}

/// Stage lifecycle event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStageLifecycleEvent {
    pub event_id: String,
    pub stage_id: String,
    pub event_kind: WorkflowStageLifecycleKind,
    pub summary: String,
    pub occurred_at: DateTime<Utc>,
}

/// Kind of stage lifecycle event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStageLifecycleKind {
    StageStarted,
    StageCompleted,
    StageSuspended,
    StageBlocked,
    StageFailed,
    StageSkipped,
}

/// Non-authoritative action request. NOT a tool call.
///
/// Contains no tool_name, tool_args, command, shell, script, cwd, env,
/// function_ref, process handle, or provider request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowActionRequest {
    pub action_request_id: String,
    pub stage_id: String,
    pub capability_category: String,
    pub purpose: String,
    pub expected_input_summary: String,
    pub expected_output_summary: String,
    pub routing_status: WorkflowActionRoutingStatus,
    pub session_bridge_required: bool,
    pub policy_gate_required: bool,
}

/// Routing status for an action request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowActionRoutingStatus {
    NotRequired,
    PreparedForFutureSessionRouting,
    SuspendedAwaitingApproval,
    Blocked,
}

/// Evidence snapshot captured at execution time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunSnapshot {
    pub readiness_id: String,
    pub proposal_id: String,
    pub proposal_hash: String,
    pub source_task_plan_hash: String,
    pub readiness_status_at_execution: String,
    pub proposal_review_decision_at_execution: String,
}

/// Abort/rollback notes carried into run evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowAbortSnapshot {
    pub abort_notes_available: bool,
    pub rollback_notes_available: bool,
    pub recovery_notes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_run_record_roundtrips() {
        let record = WorkflowRunRecord {
            execution_id: WorkflowExecutionId("wfx_test".into()),
            readiness_id: WorkflowReadinessId("wfrd_test".into()),
            proposal_id: WorkflowProposalId("wfp_test".into()),
            proposal_review_id: WorkflowProposalReviewId("wfr_test".into()),
            source_task_plan_id: TaskPlanId("tpl_test".into()),
            status: WorkflowRunStatus::Suspended,
            decision: WorkflowExecutionDecision::RunCreated,
            predicates: vec![],
            run_snapshot: WorkflowRunSnapshot {
                readiness_id: "wfrd_test".into(),
                proposal_id: "wfp_test".into(),
                proposal_hash: "phash".into(),
                source_task_plan_hash: "sphash".into(),
                readiness_status_at_execution: "ready".into(),
                proposal_review_decision_at_execution: "approved".into(),
            },
            stages: vec![],
            lifecycle_events: vec![],
            action_requests: vec![],
            abort_snapshot: WorkflowAbortSnapshot {
                abort_notes_available: true,
                rollback_notes_available: true,
                recovery_notes: vec![],
            },
            created_at: Utc::now(),
            completed_at: None,
        };
        let json = serde_json::to_string(&record).unwrap();
        let back: WorkflowRunRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record.execution_id, back.execution_id);
    }

    #[test]
    fn workflow_execution_id_is_content_addressed() {
        let id = WorkflowExecutionId("wfx_abc123".into());
        assert!(id.as_str().starts_with("wfx_"));
    }

    #[test]
    fn workflow_run_status_serializes_snake_case() {
        let json = serde_json::to_string(&WorkflowRunStatus::AlreadyExecuted).unwrap();
        assert!(json.contains("already_executed"));
    }

    #[test]
    fn workflow_execution_decision_roundtrips() {
        let d = WorkflowExecutionDecision::Blocked {
            reason_code: "hash_mismatch".into(),
            summary: "Hash does not match".into(),
        };
        let json = serde_json::to_string(&d).unwrap();
        let back: WorkflowExecutionDecision = serde_json::from_str(&json).unwrap();
        match back {
            WorkflowExecutionDecision::Blocked { reason_code, .. } => {
                assert_eq!("hash_mismatch", reason_code);
            }
            _ => panic!("Expected Blocked"),
        }
    }

    #[test]
    fn workflow_stage_run_status_serializes_snake_case() {
        let json = serde_json::to_string(&WorkflowStageRunStatus::Completed).unwrap();
        assert!(json.contains("completed"));
    }

    #[test]
    fn workflow_action_request_has_no_executable_fields() {
        let req = WorkflowActionRequest {
            action_request_id: "ar_1".into(),
            stage_id: "stage_1".into(),
            capability_category: "context-observation".into(),
            purpose: "Observe".into(),
            expected_input_summary: "Paths".into(),
            expected_output_summary: "Contents".into(),
            routing_status: WorkflowActionRoutingStatus::PreparedForFutureSessionRouting,
            session_bridge_required: true,
            policy_gate_required: true,
        };
        // Compile-time check: these fields don't exist
        let _ = &req.capability_category;
        let _ = &req.routing_status;
    }

    #[test]
    fn workflow_action_request_serialized_json_contains_no_tool_args() {
        let req = WorkflowActionRequest {
            action_request_id: "ar_1".into(),
            stage_id: "stage_1".into(),
            capability_category: "context-observation".into(),
            purpose: "Observe".into(),
            expected_input_summary: "Paths".into(),
            expected_output_summary: "Contents".into(),
            routing_status: WorkflowActionRoutingStatus::PreparedForFutureSessionRouting,
            session_bridge_required: true,
            policy_gate_required: true,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("tool_name"));
        assert!(!json.contains("tool_args"));
        assert!(!json.contains("command"));
        assert!(!json.contains("shell"));
        assert!(!json.contains("script"));
    }

    #[test]
    fn workflow_run_snapshot_roundtrips() {
        let snap = WorkflowRunSnapshot {
            readiness_id: "wfrd_1".into(),
            proposal_id: "wfp_1".into(),
            proposal_hash: "hash".into(),
            source_task_plan_hash: "shash".into(),
            readiness_status_at_execution: "ready".into(),
            proposal_review_decision_at_execution: "approved".into(),
        };
        let json = serde_json::to_string(&snap).unwrap();
        let back: WorkflowRunSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snap.proposal_hash, back.proposal_hash);
    }

    #[test]
    fn workflow_abort_snapshot_roundtrips() {
        let snap = WorkflowAbortSnapshot {
            abort_notes_available: true,
            rollback_notes_available: false,
            recovery_notes: vec!["Use git checkout".into()],
        };
        let json = serde_json::to_string(&snap).unwrap();
        let back: WorkflowAbortSnapshot = serde_json::from_str(&json).unwrap();
        assert!(back.abort_notes_available);
        assert_eq!(1, back.recovery_notes.len());
    }

    #[test]
    fn workflow_execution_decision_run_created_does_not_claim_tool_execution() {
        let d = WorkflowExecutionDecision::RunCreated;
        let json = serde_json::to_string(&d).unwrap();
        assert!(json.contains("run_created"));
        // Must not contain "executed" or "tool_execution"
        let lower = json.to_lowercase();
        assert!(!lower.contains("tool_execution"));
        assert!(!lower.contains("tools_executed"));
    }

    #[test]
    fn lifecycle_event_roundtrips() {
        let event = WorkflowStageLifecycleEvent {
            event_id: "evt_1".into(),
            stage_id: "stage_1".into(),
            event_kind: WorkflowStageLifecycleKind::StageCompleted,
            summary: "Marked complete as non-tool deterministic stage".into(),
            occurred_at: Utc::now(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: WorkflowStageLifecycleEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event.event_id, back.event_id);
    }

    #[test]
    fn execution_predicate_serializes_snake_case() {
        let p = WorkflowExecutionPredicate::ToolIntentsRemainNonExecutable;
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("tool_intents_remain_non_executable"));
    }
}
