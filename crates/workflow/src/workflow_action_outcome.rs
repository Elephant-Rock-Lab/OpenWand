//! Workflow action outcome DTOs.
//!
//! An outcome record links approval resolution to session/tool/trace evidence.
//! Workflow may correlate approval outcome, but never creates approval records,
//! mutates pending state, executes tools, or appends trace directly.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_action_route::WorkflowActionRouteId;
use crate::workflow_run::WorkflowExecutionId;

/// Content-addressed outcome ID. Format: wao_<blake3_hex>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowActionOutcomeId(pub String);

/// Request to resolve a workflow-routed pending approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowActionOutcomeRequest {
    pub workflow_execution_id: WorkflowExecutionId,
    pub route_id: WorkflowActionRouteId,
    pub stage_id: String,
    pub action_request_id: String,
    pub session_id: String,
    pub pending_approval_id: String,
    pub tool_call_id: Option<String>,
    pub expected_route_hash: String,
    pub expected_workflow_run_hash: String,
    pub resolution: WorkflowApprovalResolution,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub idempotency_key: String,
}

/// Approval resolution intent from workflow UI/CLI.
/// Maps to existing session approval API input only (Patch 3).
/// Never constructs, saves, or mutates approval records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowApprovalResolution {
    Approve { rationale: String },
    Reject { rationale: String },
}

/// Durable outcome linkage evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowActionOutcomeRecord {
    pub outcome_id: WorkflowActionOutcomeId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub route_id: WorkflowActionRouteId,
    pub stage_id: String,
    pub action_request_id: String,
    pub session_id: String,
    pub pending_approval_id: String,
    pub tool_call_id: Option<String>,
    pub route_hash: String,
    pub workflow_run_hash: String,
    pub status: WorkflowActionOutcomeStatus,
    pub decision: WorkflowActionOutcomeDecision,
    pub predicates: Vec<WorkflowActionOutcomePredicateResult>,
    pub approval_resolution: WorkflowApprovalResolution,
    pub session_outcome: Option<WorkflowSessionActionOutcomeSnapshot>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Outcome status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowActionOutcomeStatus {
    Blocked,
    ApprovalResolved,
    ToolCompleted,
    ToolDenied,
    Failed,
    AlreadyResolved,
}

/// Outcome decision — what happened, not what was executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowActionOutcomeDecision {
    ApprovalResolved { summary: String },
    ToolCompleted { summary: String },
    ToolDenied { summary: String },
    Blocked { reason_code: String, summary: String },
    Failed { reason_code: String, summary: String },
}

/// Session-produced result after approval resolution.
/// All fields observed from session output — never constructed by workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSessionActionOutcomeSnapshot {
    pub session_id: String,
    pub session_run_id: Option<String>,
    pub trace_ids: Vec<String>,
    pub approval_request_id_observed: String,
    pub approval_resolution_observed: String,
    pub tool_call_id_observed_from_session: Option<String>,
    pub tool_name_observed_from_session: Option<String>,
    pub tool_status_observed_from_session: Option<String>,
    pub safe_result_summary: Option<String>,
}

/// Outcome predicate — checks readiness to resolve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowActionOutcomePredicate {
    WorkflowRunExists,
    WorkflowRunHashMatchesRequest,
    RouteRecordExists,
    RouteHashMatchesRequest,
    RouteIsSuspendedForApproval,
    RouteLinksSameWorkflowRun,
    RouteLinksSameStage,
    RouteLinksSameActionRequest,
    RouteLinksSameSession,
    RouteHasExactlyOnePendingApproval,
    PendingApprovalIdMatchesRoute,
    ToolCallIdMatchesWhenPresent,
    ResolutionRationalePresent,
    ApprovalBridgeAvailable,
    NoPriorConflictingOutcome,
    IdempotencyKeyUnusedOrMatchesExisting,
}

/// Result of evaluating one outcome predicate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowActionOutcomePredicateResult {
    pub predicate: WorkflowActionOutcomePredicate,
    pub passed: bool,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_record_roundtrips() {
        let record = WorkflowActionOutcomeRecord {
            outcome_id: WorkflowActionOutcomeId("wao_abc123".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_1".into()),
            route_id: WorkflowActionRouteId("war_1".into()),
            stage_id: "stage_1".into(),
            action_request_id: "ar_1".into(),
            session_id: "sess_1".into(),
            pending_approval_id: "arid_1".into(),
            tool_call_id: Some("tc_1".into()),
            route_hash: "rh".into(),
            workflow_run_hash: "wrh".into(),
            status: WorkflowActionOutcomeStatus::ToolCompleted,
            decision: WorkflowActionOutcomeDecision::ToolCompleted { summary: "done".into() },
            predicates: vec![],
            approval_resolution: WorkflowApprovalResolution::Approve { rationale: "safe".into() },
            session_outcome: None,
            created_at: Utc::now(),
            completed_at: Some(Utc::now()),
        };
        let json = serde_json::to_string(&record).unwrap();
        let back: WorkflowActionOutcomeRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record.outcome_id, back.outcome_id);
        assert_eq!(record.pending_approval_id, back.pending_approval_id);
    }

    #[test]
    fn outcome_id_is_content_addressed() {
        let hash = blake3::hash(b"test-content");
        let id = WorkflowActionOutcomeId(format!("wao_{}", hash.to_hex()));
        assert!(id.0.starts_with("wao_"));
        assert_eq!(id.0.len(), 4 + 64);
    }

    #[test]
    fn outcome_status_serializes_snake_case() {
        let status = WorkflowActionOutcomeStatus::ToolCompleted;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("tool_completed"));
    }

    #[test]
    fn outcome_decision_roundtrips() {
        let decisions = vec![
            WorkflowActionOutcomeDecision::ApprovalResolved { summary: "resolved".into() },
            WorkflowActionOutcomeDecision::ToolCompleted { summary: "completed".into() },
            WorkflowActionOutcomeDecision::ToolDenied { summary: "denied".into() },
            WorkflowActionOutcomeDecision::Blocked { reason_code: "test".into(), summary: "blocked".into() },
            WorkflowActionOutcomeDecision::Failed { reason_code: "err".into(), summary: "failed".into() },
        ];
        for d in &decisions {
            let json = serde_json::to_string(d).unwrap();
            let back: WorkflowActionOutcomeDecision = serde_json::from_str(&json).unwrap();
            let json2 = serde_json::to_string(&back).unwrap();
            assert_eq!(json, json2);
        }
    }

    #[test]
    fn outcome_requires_predicates() {
        let record = WorkflowActionOutcomeRecord {
            outcome_id: WorkflowActionOutcomeId("wao_x".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_x".into()),
            route_id: WorkflowActionRouteId("war_x".into()),
            stage_id: "s".into(), action_request_id: "ar".into(),
            session_id: "sess".into(), pending_approval_id: "arid".into(),
            tool_call_id: None, route_hash: "rh".into(), workflow_run_hash: "wrh".into(),
            status: WorkflowActionOutcomeStatus::Blocked,
            decision: WorkflowActionOutcomeDecision::Blocked { reason_code: "test".into(), summary: "test".into() },
            predicates: vec![], approval_resolution: WorkflowApprovalResolution::Approve { rationale: "ok".into() },
            session_outcome: None, created_at: Utc::now(), completed_at: None,
        };
        assert!(record.predicates.is_empty());
    }

    #[test]
    fn approval_resolution_requires_rationale() {
        let approve = WorkflowApprovalResolution::Approve { rationale: "safe to proceed".into() };
        let reject = WorkflowApprovalResolution::Reject { rationale: "too risky".into() };
        match (&approve, &reject) {
            (WorkflowApprovalResolution::Approve { rationale: a }, WorkflowApprovalResolution::Reject { rationale: r }) => {
                assert!(!a.is_empty());
                assert!(!r.is_empty());
            }
            _ => panic!("Unexpected resolution combination"),
        }
    }

    #[test]
    fn session_outcome_snapshot_roundtrips() {
        let snap = WorkflowSessionActionOutcomeSnapshot {
            session_id: "sess_1".into(),
            session_run_id: Some("run_1".into()),
            trace_ids: vec!["trace_1".into()],
            approval_request_id_observed: "arid_1".into(),
            approval_resolution_observed: "approved".into(),
            tool_call_id_observed_from_session: Some("tc_1".into()),
            tool_name_observed_from_session: Some("local__file_write".into()),
            tool_status_observed_from_session: Some("completed".into()),
            safe_result_summary: Some("File written successfully".into()),
        };
        let json = serde_json::to_string(&snap).unwrap();
        let back: WorkflowSessionActionOutcomeSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snap.session_id, back.session_id);
        assert_eq!(snap.tool_name_observed_from_session, back.tool_name_observed_from_session);
    }

    #[test]
    fn outcome_serialized_json_contains_no_tool_args() {
        let record = WorkflowActionOutcomeRecord {
            outcome_id: WorkflowActionOutcomeId("wao_test".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            route_id: WorkflowActionRouteId("war_t".into()),
            stage_id: "s".into(), action_request_id: "ar".into(),
            session_id: "sess".into(), pending_approval_id: "arid".into(),
            tool_call_id: None, route_hash: "rh".into(), workflow_run_hash: "wrh".into(),
            status: WorkflowActionOutcomeStatus::ApprovalResolved,
            decision: WorkflowActionOutcomeDecision::ApprovalResolved { summary: "done".into() },
            predicates: vec![], approval_resolution: WorkflowApprovalResolution::Approve { rationale: "ok".into() },
            session_outcome: None, created_at: Utc::now(), completed_at: None,
        };
        let json = serde_json::to_string(&record).unwrap().to_lowercase();
        assert!(!json.contains("tool_args"));
        assert!(!json.contains("command"));
        assert!(!json.contains("shell"));
    }
}
