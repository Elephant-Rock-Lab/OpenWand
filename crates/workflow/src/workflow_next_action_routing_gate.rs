//! Next-action routing gate DTOs and validation.
//!
//! Routing readiness is not routing. A reviewed routing-readiness record is not
//! execution. The routing gate may create one route record through the existing
//! workflow-action routing path. The route still enters SessionRunner through
//! the existing routing path. Workflow still does not execute tools directly.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_action_route::WorkflowActionRouteId;
use crate::workflow_continuation::WorkflowNextActionProposalId;
use crate::workflow_next_action_review::WorkflowNextActionReviewId;
use crate::workflow_reconciliation::WorkflowRunRevisionId;
use crate::workflow_routing_readiness::WorkflowRoutingReadinessId;
use crate::workflow_run::WorkflowExecutionId;

/// Content-addressed next-action routing ID. Format: wnaroute_<blake3_hex>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowNextActionRoutingId(pub String);

/// Request to route one reviewed-ready next action through the existing path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNextActionRoutingRequest {
    pub routing_readiness_id: WorkflowRoutingReadinessId,
    pub next_action_proposal_id: WorkflowNextActionProposalId,
    pub next_action_review_id: WorkflowNextActionReviewId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub source_run_revision_id: WorkflowRunRevisionId,
    pub expected_routing_readiness_hash: String,
    pub expected_proposal_hash: String,
    pub expected_review_hash: String,
    pub expected_run_revision_hash: String,
    /// Patch 2: hash-bind the action request at routing boundary.
    pub expected_action_request_hash: String,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub idempotency_key: String,
}

/// Durable evidence that readiness was consumed into one route attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNextActionRoutingRecord {
    pub routing_id: WorkflowNextActionRoutingId,
    pub routing_readiness_id: WorkflowRoutingReadinessId,
    pub next_action_proposal_id: WorkflowNextActionProposalId,
    pub next_action_review_id: WorkflowNextActionReviewId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub source_run_revision_id: WorkflowRunRevisionId,
    pub status: WorkflowNextActionRoutingStatus,
    pub decision: WorkflowNextActionRoutingDecision,
    pub predicates: Vec<WorkflowNextActionRoutingPredicateResult>,
    /// Hash of the route preview at routing time for audit.
    pub route_request_preview_hash: String,
    /// The route record created through the existing path (only when Routed/AlreadyRouted).
    pub created_route_id: Option<WorkflowActionRouteId>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Routing status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNextActionRoutingStatus {
    Blocked,
    Routed,
    Failed,
    AlreadyRouted,
}

/// Routing decision — records what happened, not what was executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowNextActionRoutingDecision {
    Routed { route_id: WorkflowActionRouteId, summary: String },
    Blocked { reason_code: String, summary: String },
    Failed { reason_code: String, summary: String },
    AlreadyRouted { route_id: WorkflowActionRouteId, summary: String },
}

/// Routing-time revalidation predicates — full chain from readiness through preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNextActionRoutingPredicate {
    RoutingReadinessExists,
    RoutingReadinessIsReady,
    RoutingReadinessHashMatchesRequest,
    NextActionProposalExists,
    NextActionReviewExists,
    NextActionReviewIsLatest,
    NextActionReviewApproved,
    ProposalHashMatchesReadiness,
    ProposalHashMatchesRequest,
    ReviewHashMatchesReadiness,
    ReviewHashMatchesRequest,
    RunRevisionExists,
    RunRevisionIsLatest,
    RunRevisionHashMatchesReadiness,
    RunRevisionHashMatchesRequest,
    CandidateStageExists,
    CandidateStageStillPending,
    CandidateStageDependenciesStillTerminal,
    SelectorNoSkipStillHolds,
    ActionRequestExists,
    ActionRequestPreparedForRouting,
    ActionRequestRemainsNonExecutable,
    ActionRequestHashMatchesReadiness,
    ActionRequestHashMatchesRequest,
    RoutePreviewExists,
    RoutePreviewStillDescriptiveOnly,
    RoutePreviewCreatesNoRouteNow,
    NoPriorConflictingNextActionRouting,
    NoPriorRouteForSameReadiness,
    IdempotencyKeyUnusedOrMatchesExisting,
    ProposalReviewReadinessCrossReferencesMatch,
}

/// Result of evaluating one routing predicate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNextActionRoutingPredicateResult {
    pub predicate: WorkflowNextActionRoutingPredicate,
    pub passed: bool,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_action_routing_record_roundtrips() {
        let record = WorkflowNextActionRoutingRecord {
            routing_id: WorkflowNextActionRoutingId("wnaroute_abc123".into()),
            routing_readiness_id: WorkflowRoutingReadinessId("wrrd_1".into()),
            next_action_proposal_id: WorkflowNextActionProposalId("wnap_1".into()),
            next_action_review_id: WorkflowNextActionReviewId("wnar_1".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_1".into()),
            source_run_revision_id: WorkflowRunRevisionId("wrr_1".into()),
            status: WorkflowNextActionRoutingStatus::Blocked,
            decision: WorkflowNextActionRoutingDecision::Blocked {
                reason_code: "test".into(), summary: "test".into(),
            },
            predicates: vec![],
            route_request_preview_hash: "ph".into(),
            created_route_id: None,
            created_at: Utc::now(),
            completed_at: None,
        };
        let json = serde_json::to_string(&record).unwrap();
        let back: WorkflowNextActionRoutingRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record.routing_id, back.routing_id);
    }

    #[test]
    fn next_action_routing_id_is_content_addressed() {
        let hash = blake3::hash(b"test-content");
        let id = WorkflowNextActionRoutingId(format!("wnaroute_{}", hash.to_hex()));
        assert!(id.0.starts_with("wnaroute_"));
        // wnaroute_ (9 chars) + 64 hex chars = 73
        assert_eq!(id.0.len(), 9 + 64);
    }

    #[test]
    fn next_action_routing_status_serializes_snake_case() {
        let json = serde_json::to_string(&WorkflowNextActionRoutingStatus::AlreadyRouted).unwrap();
        assert!(json.contains("already_routed"));
    }

    #[test]
    fn next_action_routing_decision_roundtrips() {
        let decisions = vec![
            WorkflowNextActionRoutingDecision::Routed {
                route_id: WorkflowActionRouteId("war_1".into()), summary: "ok".into(),
            },
            WorkflowNextActionRoutingDecision::Blocked {
                reason_code: "pred".into(), summary: "blocked".into(),
            },
            WorkflowNextActionRoutingDecision::Failed {
                reason_code: "err".into(), summary: "fail".into(),
            },
            WorkflowNextActionRoutingDecision::AlreadyRouted {
                route_id: WorkflowActionRouteId("war_1".into()), summary: "existing".into(),
            },
        ];
        for d in &decisions {
            let json = serde_json::to_string(d).unwrap();
            let back: WorkflowNextActionRoutingDecision = serde_json::from_str(&json).unwrap();
            let json2 = serde_json::to_string(&back).unwrap();
            assert_eq!(json, json2);
        }
    }

    #[test]
    fn next_action_routing_requires_predicates() {
        let record = WorkflowNextActionRoutingRecord {
            routing_id: WorkflowNextActionRoutingId("wnaroute_x".into()),
            routing_readiness_id: WorkflowRoutingReadinessId("wrrd_x".into()),
            next_action_proposal_id: WorkflowNextActionProposalId("wnap_x".into()),
            next_action_review_id: WorkflowNextActionReviewId("wnar_x".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_x".into()),
            source_run_revision_id: WorkflowRunRevisionId("wrr_x".into()),
            status: WorkflowNextActionRoutingStatus::Blocked,
            decision: WorkflowNextActionRoutingDecision::Blocked {
                reason_code: "test".into(), summary: "test".into(),
            },
            predicates: vec![],
            route_request_preview_hash: String::new(),
            created_route_id: None,
            created_at: Utc::now(),
            completed_at: None,
        };
        assert!(record.predicates.is_empty());
    }

    #[test]
    fn next_action_routing_routed_links_route_id() {
        let route_id = WorkflowActionRouteId("war_linked".into());
        let record = WorkflowNextActionRoutingRecord {
            routing_id: WorkflowNextActionRoutingId("wnaroute_r".into()),
            routing_readiness_id: WorkflowRoutingReadinessId("wrrd_r".into()),
            next_action_proposal_id: WorkflowNextActionProposalId("wnap_r".into()),
            next_action_review_id: WorkflowNextActionReviewId("wnar_r".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_r".into()),
            source_run_revision_id: WorkflowRunRevisionId("wrr_r".into()),
            status: WorkflowNextActionRoutingStatus::Routed,
            decision: WorkflowNextActionRoutingDecision::Routed {
                route_id: route_id.clone(), summary: "routed".into(),
            },
            predicates: vec![],
            route_request_preview_hash: "ph".into(),
            created_route_id: Some(route_id.clone()),
            created_at: Utc::now(),
            completed_at: Some(Utc::now()),
        };
        assert!(record.created_route_id.is_some());
        if let WorkflowNextActionRoutingDecision::Routed { route_id: rid, .. } = &record.decision {
            assert_eq!(&route_id, rid);
        }
    }
}
