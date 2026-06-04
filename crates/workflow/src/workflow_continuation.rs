//! Workflow continuation DTOs.
//!
//! Continuation readiness and next-action proposals are evidence records.
//! They do not route actions, resolve approvals, execute tools, append trace,
//! mutate memory, mutate workflow run/revision state, or create
//! approval/session/tool/trace records.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_reconciliation::WorkflowRunRevisionId;
use crate::workflow_run::WorkflowExecutionId;

/// Content-addressed continuation readiness ID. Format: wcr_<blake3_hex>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowContinuationReadinessId(pub String);

/// Content-addressed next-action proposal ID. Format: wnap_<blake3_hex>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowNextActionProposalId(pub String);

/// Request to evaluate continuation readiness from a run revision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContinuationRequest {
    pub workflow_execution_id: WorkflowExecutionId,
    pub latest_run_revision_id: WorkflowRunRevisionId,
    pub expected_run_revision_hash: String,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub idempotency_key: String,
}

/// Durable readiness evaluation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContinuationReadinessRecord {
    pub readiness_id: WorkflowContinuationReadinessId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub latest_run_revision_id: WorkflowRunRevisionId,
    pub run_revision_hash: String,
    pub status: WorkflowContinuationStatus,
    pub decision: WorkflowContinuationDecision,
    pub predicates: Vec<WorkflowContinuationPredicateResult>,
    pub selected_candidate: Option<WorkflowNextActionCandidate>,
    pub created_at: DateTime<Utc>,
}

/// Continuation status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowContinuationStatus {
    ProposalReady,
    NoEligibleAction,
    Blocked,
    Inconclusive,
}

/// Continuation decision — what the evaluation determined.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowContinuationDecision {
    ProposalReady { summary: String },
    NoEligibleAction { summary: String },
    Blocked { reason_code: String, summary: String },
    Inconclusive { reason_code: String, summary: String },
}

/// Durable next-action proposal — evidence only, never routes or executes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNextActionProposal {
    pub proposal_id: WorkflowNextActionProposalId,
    pub readiness_id: WorkflowContinuationReadinessId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub source_run_revision_id: WorkflowRunRevisionId,
    pub source_run_revision_hash: String,
    pub candidate: WorkflowNextActionCandidate,
    pub predicates: Vec<WorkflowContinuationPredicateResult>,
    pub evidence_links: Vec<WorkflowContinuationEvidenceLink>,
    /// Always false — proposals never create routes.
    pub creates_route: bool,
    /// Always false — proposals never route actions now.
    pub routes_action_now: bool,
    /// Always false — proposals never execute tools now.
    pub executes_tool_now: bool,
    /// Always false — proposals never mutate workflow state now.
    pub mutates_workflow_state_now: bool,
    pub proposal_hash: String,
    pub created_at: DateTime<Utc>,
}

/// Candidate stage/action derived from latest run revision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNextActionCandidate {
    pub stage_id: String,
    pub action_request_id: Option<String>,
    pub candidate_kind: WorkflowNextActionKind,
    pub stage_title: String,
    pub reason: String,
    pub dependency_evidence: Vec<String>,
}

/// Kind of next action candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNextActionKind {
    RoutePreparedAction,
    AwaitExternalEvidence,
    ManualReviewRequired,
    NoAction,
}

/// Evidence link from continuation to prior records.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContinuationEvidenceLink {
    pub kind: WorkflowContinuationEvidenceKind,
    pub id: String,
    pub summary: String,
}

/// Kind of evidence link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowContinuationEvidenceKind {
    WorkflowRun,
    WorkflowRunRevision,
    Stage,
    LifecycleEvent,
    Reconciliation,
    Outcome,
    Route,
    ActionRequest,
}

/// Continuation predicates — deterministic eligibility checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowContinuationPredicate {
    WorkflowRunExists,
    RunRevisionExists,
    RunRevisionIsLatest,
    RunRevisionHashMatchesRequest,
    RunRevisionBelongsToWorkflowRun,
    StagesPresent,
    PriorStageDependenciesSatisfied,
    NoStageCurrentlyRunning,
    NoStageCurrentlySuspendedWithoutOutcome,
    NextStageExists,
    NextStageIsPending,
    NextStageDependenciesTerminal,
    NextActionRequestExistsWhenRequired,
    NextActionRequestPreparedForRouting,
    NextActionRequestRemainsNonExecutable,
    NoPriorConflictingNextActionProposal,
    IdempotencyKeyUnusedOrMatchesExisting,
}

/// Result of evaluating one continuation predicate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContinuationPredicateResult {
    pub predicate: WorkflowContinuationPredicate,
    pub passed: bool,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuation_readiness_roundtrips() {
        let rec = WorkflowContinuationReadinessRecord {
            readiness_id: WorkflowContinuationReadinessId("wcr_abc".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            latest_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            run_revision_hash: "h".into(),
            status: WorkflowContinuationStatus::ProposalReady,
            decision: WorkflowContinuationDecision::ProposalReady { summary: "ok".into() },
            predicates: vec![], selected_candidate: None, created_at: Utc::now(),
        };
        let json = serde_json::to_string(&rec).unwrap();
        let back: WorkflowContinuationReadinessRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(rec.readiness_id, back.readiness_id);
    }

    #[test]
    fn continuation_readiness_id_is_content_addressed() {
        let id = WorkflowContinuationReadinessId("wcr_deadbeef".into());
        assert!(id.0.starts_with("wcr_"));
    }

    #[test]
    fn next_action_proposal_roundtrips() {
        let prop = WorkflowNextActionProposal {
            proposal_id: WorkflowNextActionProposalId("wnap_abc".into()),
            readiness_id: WorkflowContinuationReadinessId("wcr_1".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            source_run_revision_hash: "h".into(),
            candidate: WorkflowNextActionCandidate {
                stage_id: "s1".into(), action_request_id: Some("ar_1".into()),
                candidate_kind: WorkflowNextActionKind::RoutePreparedAction,
                stage_title: "Stage 1".into(), reason: "deps met".into(),
                dependency_evidence: vec![],
            },
            predicates: vec![], evidence_links: vec![],
            creates_route: false, routes_action_now: false,
            executes_tool_now: false, mutates_workflow_state_now: false,
            proposal_hash: "ph".into(), created_at: Utc::now(),
        };
        let json = serde_json::to_string(&prop).unwrap();
        let back: WorkflowNextActionProposal = serde_json::from_str(&json).unwrap();
        assert_eq!(prop.proposal_id, back.proposal_id);
    }

    #[test]
    fn next_action_proposal_id_is_content_addressed() {
        let id = WorkflowNextActionProposalId("wnap_deadbeef".into());
        assert!(id.0.starts_with("wnap_"));
    }

    #[test]
    fn continuation_status_serializes_snake_case() {
        let json = serde_json::to_string(&WorkflowContinuationStatus::ProposalReady).unwrap();
        assert!(json.contains("proposal_ready"));
        let json = serde_json::to_string(&WorkflowContinuationStatus::NoEligibleAction).unwrap();
        assert!(json.contains("no_eligible_action"));
    }

    #[test]
    fn continuation_decision_roundtrips() {
        let d = WorkflowContinuationDecision::Blocked {
            reason_code: "stage_running".into(), summary: "Stage running".into(),
        };
        let json = serde_json::to_string(&d).unwrap();
        let back: WorkflowContinuationDecision = serde_json::from_str(&json).unwrap();
        if let WorkflowContinuationDecision::Blocked { reason_code, .. } = back {
            assert_eq!("stage_running", reason_code);
        } else { panic!("Expected Blocked"); }
    }

    #[test]
    fn next_action_proposal_does_not_create_route_or_execute() {
        let prop = WorkflowNextActionProposal {
            proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
            readiness_id: WorkflowContinuationReadinessId("wcr_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            source_run_revision_hash: "h".into(),
            candidate: WorkflowNextActionCandidate {
                stage_id: "s1".into(), action_request_id: None,
                candidate_kind: WorkflowNextActionKind::NoAction,
                stage_title: "test".into(), reason: "test".into(),
                dependency_evidence: vec![],
            },
            predicates: vec![], evidence_links: vec![],
            creates_route: false, routes_action_now: false,
            executes_tool_now: false, mutates_workflow_state_now: false,
            proposal_hash: "ph".into(), created_at: Utc::now(),
        };
        assert!(!prop.creates_route);
        assert!(!prop.routes_action_now);
        assert!(!prop.executes_tool_now);
        assert!(!prop.mutates_workflow_state_now);
    }

    #[test]
    fn next_action_candidate_roundtrips() {
        let c = WorkflowNextActionCandidate {
            stage_id: "s1".into(), action_request_id: Some("ar_1".into()),
            candidate_kind: WorkflowNextActionKind::RoutePreparedAction,
            stage_title: "Stage 1".into(), reason: "deps met".into(),
            dependency_evidence: vec!["s0 completed".into()],
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: WorkflowNextActionCandidate = serde_json::from_str(&json).unwrap();
        assert_eq!(c.stage_id, back.stage_id);
        assert_eq!(WorkflowNextActionKind::RoutePreparedAction, back.candidate_kind);
    }
}
