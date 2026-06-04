//! Routing readiness DTOs.
//!
//! Routing readiness is evidence, not routing. A Ready record contains a route
//! preview (descriptive only), not a route record. It creates no route records,
//! session turns, approval requests, tool calls, trace events, or workflow mutations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_continuation::{WorkflowNextActionCandidate, WorkflowNextActionProposalId};
use crate::workflow_next_action_review::WorkflowNextActionReviewId;
use crate::workflow_reconciliation::WorkflowRunRevisionId;
use crate::workflow_run::WorkflowExecutionId;

/// Content-addressed routing readiness ID. Format: wrrd_<blake3_hex>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowRoutingReadinessId(pub String);

/// Request to evaluate routing readiness for an approved next-action proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRoutingReadinessRequest {
    pub proposal_id: WorkflowNextActionProposalId,
    pub review_id: WorkflowNextActionReviewId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub source_run_revision_id: WorkflowRunRevisionId,
    pub expected_proposal_hash: String,
    pub expected_run_revision_hash: String,
    /// Patch 1: expected review hash for linkage verification.
    pub expected_review_hash: String,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub idempotency_key: String,
}

/// Durable routing readiness evidence record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRoutingReadinessRecord {
    pub readiness_id: WorkflowRoutingReadinessId,
    pub proposal_id: WorkflowNextActionProposalId,
    pub review_id: WorkflowNextActionReviewId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub source_run_revision_id: WorkflowRunRevisionId,
    pub proposal_hash: String,
    pub run_revision_hash: String,
    pub status: WorkflowRoutingReadinessStatus,
    pub decision: WorkflowRoutingReadinessDecision,
    pub predicates: Vec<WorkflowRoutingReadinessPredicateResult>,
    pub candidate: Option<WorkflowNextActionCandidate>,
    pub route_request_preview: Option<WorkflowRouteRequestPreview>,
    pub created_at: DateTime<Utc>,
}

/// Routing readiness status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRoutingReadinessStatus {
    Ready,
    Blocked,
    Inconclusive,
}

/// Routing readiness decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowRoutingReadinessDecision {
    Ready { summary: String },
    Blocked { reason_code: String, summary: String },
    Inconclusive { reason_code: String, summary: String },
}

/// Route request preview — descriptive only, never a route record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRouteRequestPreview {
    pub workflow_execution_id: WorkflowExecutionId,
    pub stage_id: String,
    pub action_request_id: String,
    pub source_proposal_id: WorkflowNextActionProposalId,
    pub source_review_id: WorkflowNextActionReviewId,
    /// Always true — this is a description, not a route.
    pub descriptive_only: bool,
    /// Always false — this does not create a route now.
    pub creates_route_now: bool,
}

/// Routing readiness predicates — full revalidation chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRoutingReadinessPredicate {
    NextActionProposalExists,
    NextActionProposalReviewExists,
    NextActionProposalReviewIsLatest,
    NextActionProposalReviewApproved,
    ProposalHashMatchesReview,
    ProposalHashMatchesRequest,
    ReviewHashMatchesRequest,
    RunRevisionExists,
    RunRevisionIsLatest,
    RunRevisionHashMatchesProposal,
    RunRevisionHashMatchesRequest,
    ProposalBelongsToRunRevision,
    CandidateStageExists,
    CandidateStageStillPending,
    CandidateStageDependenciesStillTerminal,
    SelectorNoSkipStillHolds,
    ActionRequestExists,
    ActionRequestPreparedForRouting,
    ActionRequestRemainsNonExecutable,
    ProposalStillDoesNotCreateRoute,
    ReviewStillDoesNotCreateRoute,
    GovernanceConstraintsRepresented,
    NoPriorConflictingRoutingReadiness,
    IdempotencyKeyUnusedOrMatchesExisting,
}

/// Result of evaluating one routing readiness predicate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRoutingReadinessPredicateResult {
    pub predicate: WorkflowRoutingReadinessPredicate,
    pub passed: bool,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_readiness_record_roundtrips() {
        let rec = WorkflowRoutingReadinessRecord {
            readiness_id: WorkflowRoutingReadinessId("wrrd_abc".into()),
            proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
            review_id: WorkflowNextActionReviewId("wnar_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            proposal_hash: "ph".into(), run_revision_hash: "rh".into(),
            status: WorkflowRoutingReadinessStatus::Ready,
            decision: WorkflowRoutingReadinessDecision::Ready { summary: "ok".into() },
            predicates: vec![], candidate: None, route_request_preview: None,
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&rec).unwrap();
        let back: WorkflowRoutingReadinessRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(rec.readiness_id, back.readiness_id);
    }

    #[test]
    fn routing_readiness_id_is_content_addressed() {
        let id = WorkflowRoutingReadinessId("wrrd_deadbeef".into());
        assert!(id.0.starts_with("wrrd_"));
    }

    #[test]
    fn routing_readiness_status_serializes_snake_case() {
        let json = serde_json::to_string(&WorkflowRoutingReadinessStatus::Ready).unwrap();
        assert!(json.contains("ready"));
    }

    #[test]
    fn routing_readiness_decision_roundtrips() {
        let d = WorkflowRoutingReadinessDecision::Blocked {
            reason_code: "hash_mismatch".into(), summary: "bad hash".into(),
        };
        let json = serde_json::to_string(&d).unwrap();
        let back: WorkflowRoutingReadinessDecision = serde_json::from_str(&json).unwrap();
        if let WorkflowRoutingReadinessDecision::Blocked { reason_code, .. } = back {
            assert_eq!("hash_mismatch", reason_code);
        } else { panic!("Expected Blocked"); }
    }

    #[test]
    fn routing_readiness_requires_predicates() {
        let rec = WorkflowRoutingReadinessRecord {
            readiness_id: WorkflowRoutingReadinessId("wrrd_t".into()),
            proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
            review_id: WorkflowNextActionReviewId("wnar_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            proposal_hash: "ph".into(), run_revision_hash: "rh".into(),
            status: WorkflowRoutingReadinessStatus::Blocked,
            decision: WorkflowRoutingReadinessDecision::Blocked { reason_code: "test".into(), summary: "test".into() },
            predicates: vec![], candidate: None, route_request_preview: None,
            created_at: Utc::now(),
        };
        assert!(rec.predicates.is_empty());
    }

    #[test]
    fn route_request_preview_does_not_create_route() {
        let preview = WorkflowRouteRequestPreview {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            stage_id: "s1".into(), action_request_id: "ar_1".into(),
            source_proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
            source_review_id: WorkflowNextActionReviewId("wnar_t".into()),
            descriptive_only: true, creates_route_now: false,
        };
        assert!(preview.descriptive_only);
        assert!(!preview.creates_route_now);
    }

    // Patch 2: explicit no-route-id proof
    #[test]
    fn route_request_preview_has_no_route_id() {
        let preview = WorkflowRouteRequestPreview {
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            stage_id: "s1".into(), action_request_id: "ar_1".into(),
            source_proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
            source_review_id: WorkflowNextActionReviewId("wnar_t".into()),
            descriptive_only: true, creates_route_now: false,
        };
        let json = serde_json::to_string(&preview).unwrap();
        assert!(!json.contains("route_id"), "Preview must not contain route_id");
    }
}
