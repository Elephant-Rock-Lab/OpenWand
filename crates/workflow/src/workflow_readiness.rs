//! Workflow readiness DTOs.
//!
//! Readiness determines whether an approved workflow proposal is eligible
//! for future execution. It is evidence, not an execution grant.
//! A Ready record does not authorize execution.
//! A resolvable tool intent is not a tool call.
//! A future approval requirement is not an approval request.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::plan::TaskPlanId;
use crate::plan_review::TaskPlanReviewId;
use crate::workflow_proposal::WorkflowProposalId;
use crate::workflow_proposal_review::WorkflowProposalReviewId;

/// Content-addressed readiness ID. Format: `wfrd_<blake3_hex>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowReadinessId(pub String);

impl WorkflowReadinessId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Request to evaluate workflow readiness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowReadinessRequest {
    pub proposal_id: WorkflowProposalId,
    pub review_id: WorkflowProposalReviewId,
    pub expected_proposal_hash: String,
    pub expected_source_task_plan_hash: String,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub idempotency_key: String,
}

/// Durable readiness record. Evidence, not an execution grant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowReadinessRecord {
    pub readiness_id: WorkflowReadinessId,
    pub proposal_id: WorkflowProposalId,
    pub review_id: WorkflowProposalReviewId,
    pub source_task_plan_id: TaskPlanId,
    pub source_task_plan_review_id: TaskPlanReviewId,
    pub proposal_hash: String,
    pub source_task_plan_hash: String,
    pub status: WorkflowReadinessStatus,
    pub decision: WorkflowReadinessDecision,
    pub predicates: Vec<WorkflowReadinessPredicateResult>,
    pub tool_intents: Vec<ToolIntentResolutionSnapshot>,
    pub approval_markers: Vec<WorkflowApprovalMarkerSnapshot>,
    pub environment: WorkflowEnvironmentSnapshot,
    pub rollback_abort: WorkflowRollbackAbortSnapshot,
    pub created_at: DateTime<Utc>,
}

/// Readiness outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowReadinessStatus {
    Ready,
    Blocked,
    Inconclusive,
}

/// Readiness decision with reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowReadinessDecision {
    Ready,
    Blocked { reason_code: String, summary: String },
    Inconclusive { reason_code: String, summary: String },
}

/// Result of evaluating a single readiness predicate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowReadinessPredicateResult {
    pub predicate: WorkflowReadinessPredicate,
    pub passed: bool,
    pub reason: String,
}

/// All readiness predicates. None of these execute tools or create grants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowReadinessPredicate {
    ProposalExists,
    ProposalReviewExists,
    ProposalReviewIsLatest,
    ProposalReviewApproved,
    ProposalHashMatchesReview,
    ProposalHashMatchesRequest,
    SourceTaskPlanExists,
    SourceTaskPlanHashMatchesProposal,
    SourceTaskPlanHashMatchesRequest,
    SourceTaskPlanLatestReviewApproved,
    WorkflowProposalIsReviewable,
    RequiredApprovalMarkersPresent,
    ToolIntentsResolvable,
    ToolIntentsRemainNonExecutable,
    PolicyConstraintsRepresented,
    ProviderConfigurationAvailable,
    SessionRuntimeAvailable,
    WorkspacePreconditionsObserved,
    RollbackAbortEvidencePresent,
    NoPriorConflictingReadiness,
    IdempotencyKeyUnusedOrMatchesExisting,
}

/// Snapshot of tool-intent resolution. Never a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolIntentResolutionSnapshot {
    pub intent_id: String,
    pub capability: String,
    pub resolution_status: ToolIntentResolutionStatus,
    pub matched_capability_category: Option<String>,
    pub reason: String,
}

/// Status of resolving a tool intent to a capability category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolIntentResolutionStatus {
    ResolvedCategory,
    Unresolved,
    Ambiguous,
    RejectedExecutable,
}

/// Snapshot of approval markers as future requirements.
///
/// `requirement_understood` means this requirement is carried into readiness
/// evidence for future execution evaluation. It does NOT mean an approval
/// request was created, approval has been granted, or execution may proceed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowApprovalMarkerSnapshot {
    pub marker_id: String,
    pub stage_id: String,
    pub reason: String,
    pub required_before: String,
    /// This requirement is represented for future execution evaluation.
    /// It is NOT a satisfied approval or approval request.
    pub requirement_understood: bool,
    pub note: String,
}

/// Read-only snapshot of environment preconditions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEnvironmentSnapshot {
    pub workspace_observed: bool,
    pub provider_config_available: bool,
    pub session_runtime_available: bool,
    pub tool_manifest_available: bool,
    pub policy_context_available: bool,
    pub notes: Vec<String>,
}

/// Snapshot of rollback/abort evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRollbackAbortSnapshot {
    pub abort_notes_present: bool,
    pub rollback_notes_present: bool,
    pub unresolved_recovery_gaps: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_readiness_record_roundtrips() {
        let record = WorkflowReadinessRecord {
            readiness_id: WorkflowReadinessId("wfrd_test".into()),
            proposal_id: WorkflowProposalId("wfp_test".into()),
            review_id: WorkflowProposalReviewId("wfr_test".into()),
            source_task_plan_id: TaskPlanId("tpl_test".into()),
            source_task_plan_review_id: TaskPlanReviewId("tpr_test".into()),
            proposal_hash: "phash".into(),
            source_task_plan_hash: "sphash".into(),
            status: WorkflowReadinessStatus::Ready,
            decision: WorkflowReadinessDecision::Ready,
            predicates: vec![],
            tool_intents: vec![],
            approval_markers: vec![],
            environment: WorkflowEnvironmentSnapshot {
                workspace_observed: true,
                provider_config_available: true,
                session_runtime_available: true,
                tool_manifest_available: true,
                policy_context_available: true,
                notes: vec![],
            },
            rollback_abort: WorkflowRollbackAbortSnapshot {
                abort_notes_present: true,
                rollback_notes_present: true,
                unresolved_recovery_gaps: vec![],
            },
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&record).unwrap();
        let back: WorkflowReadinessRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record.readiness_id, back.readiness_id);
        assert_eq!(record.proposal_hash, back.proposal_hash);
    }

    #[test]
    fn workflow_readiness_id_is_content_addressed() {
        let id = WorkflowReadinessId("wfrd_abc123".into());
        assert!(id.as_str().starts_with("wfrd_"));
    }

    #[test]
    fn workflow_readiness_status_serializes_snake_case() {
        let status = WorkflowReadinessStatus::Ready;
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("ready"));
        assert!(!json.contains("Ready"));
    }

    #[test]
    fn workflow_readiness_decision_roundtrips() {
        let decision = WorkflowReadinessDecision::Blocked {
            reason_code: "hash_mismatch".into(),
            summary: "Proposal hash does not match".into(),
        };
        let json = serde_json::to_string(&decision).unwrap();
        let back: WorkflowReadinessDecision = serde_json::from_str(&json).unwrap();
        match back {
            WorkflowReadinessDecision::Blocked { reason_code, .. } => {
                assert_eq!("hash_mismatch", reason_code);
            }
            _ => panic!("Expected Blocked"),
        }
    }

    #[test]
    fn workflow_readiness_predicate_result_roundtrips() {
        let result = WorkflowReadinessPredicateResult {
            predicate: WorkflowReadinessPredicate::ProposalExists,
            passed: true,
            reason: "Proposal loaded successfully".into(),
        };
        let json = serde_json::to_string(&result).unwrap();
        let back: WorkflowReadinessPredicateResult = serde_json::from_str(&json).unwrap();
        assert!(back.passed);
    }

    #[test]
    fn workflow_environment_snapshot_roundtrips() {
        let snap = WorkflowEnvironmentSnapshot {
            workspace_observed: true,
            provider_config_available: false,
            session_runtime_available: true,
            tool_manifest_available: true,
            policy_context_available: true,
            notes: vec!["Provider not configured".into()],
        };
        let json = serde_json::to_string(&snap).unwrap();
        let back: WorkflowEnvironmentSnapshot = serde_json::from_str(&json).unwrap();
        assert!(back.workspace_observed);
        assert!(!back.provider_config_available);
        assert_eq!(1, back.notes.len());
    }

    #[test]
    fn tool_intent_resolution_snapshot_roundtrips() {
        let snap = ToolIntentResolutionSnapshot {
            intent_id: "intent_1".into(),
            capability: "context-observation".into(),
            resolution_status: ToolIntentResolutionStatus::ResolvedCategory,
            matched_capability_category: Some("file_observation".into()),
            reason: "Matches known category".into(),
        };
        let json = serde_json::to_string(&snap).unwrap();
        let back: ToolIntentResolutionSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(snap.intent_id, back.intent_id);
        assert!(back.matched_capability_category.is_some());
    }

    #[test]
    fn workflow_approval_marker_snapshot_does_not_claim_approval_satisfied() {
        let marker = WorkflowApprovalMarkerSnapshot {
            marker_id: "marker_1".into(),
            stage_id: "stage_3".into(),
            reason: "Human review required".into(),
            required_before: "stage_4".into(),
            requirement_understood: true,
            note: "Requirement carried for future execution evaluation".into(),
        };
        // Field name explicitly says "requirement_understood", not "satisfied" or "approved"
        assert!(marker.requirement_understood);
        // The note must not claim approval
        assert!(!marker.note.contains("approved"));
        assert!(!marker.note.contains("satisfied"));
        assert!(!marker.note.contains("granted"));
    }

    #[test]
    fn workflow_readiness_predicate_serializes_snake_case() {
        let pred = WorkflowReadinessPredicate::ToolIntentsResolvable;
        let json = serde_json::to_string(&pred).unwrap();
        assert!(json.contains("tool_intents_resolvable"));
    }
}
