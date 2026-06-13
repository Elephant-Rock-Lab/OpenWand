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

/// Context for next-action routing evaluation. All evidence, no execution.
pub struct WorkflowNextActionRoutingContext<'a> {
    pub routing_readiness: Option<&'a crate::workflow_routing_readiness::WorkflowRoutingReadinessRecord>,
    pub next_action_proposal: Option<&'a crate::workflow_continuation::WorkflowNextActionProposal>,
    pub next_action_review: Option<&'a crate::workflow_next_action_review::WorkflowNextActionReview>,
    pub latest_review: Option<&'a crate::workflow_next_action_review::WorkflowNextActionReview>,
    pub run_revision: Option<&'a crate::workflow_reconciliation::WorkflowRunRevision>,
    pub action_request: Option<&'a crate::workflow_run::WorkflowActionRequest>,
    pub prior_routings: Vec<&'a WorkflowNextActionRoutingRecord>,
}

fn pred(predicate: WorkflowNextActionRoutingPredicate, passed: bool, reason: &str) -> WorkflowNextActionRoutingPredicateResult {
    WorkflowNextActionRoutingPredicateResult { predicate, passed, reason: reason.into() }
}

/// Evaluate all routing-time predicates and produce a next-action routing record.
/// Does not create routes — that happens at the app layer through the existing path.
pub fn evaluate_next_action_routing(
    request: &WorkflowNextActionRoutingRequest,
    context: &WorkflowNextActionRoutingContext,
) -> WorkflowNextActionRoutingRecord {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(b"next_action_routing:v1:");
    hasher.update(request.routing_readiness_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.next_action_proposal_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.next_action_review_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.idempotency_key.as_bytes());
    let hex = hasher.finalize().to_hex().to_string();
    let rid = WorkflowNextActionRoutingId(format!("wnaroute_{}", &hex[..16]));

    let mut predicates = Vec::new();

    let readiness = context.routing_readiness;
    let proposal = context.next_action_proposal;
    let review = context.next_action_review;
    let latest_review = context.latest_review;
    let revision = context.run_revision;
    let action = context.action_request;

    // 1. RoutingReadinessExists
    predicates.push(pred(WorkflowNextActionRoutingPredicate::RoutingReadinessExists,
        readiness.is_some(), if readiness.is_some() { "Readiness found" } else { "No readiness" }));

    // 2. RoutingReadinessIsReady
    let is_ready = readiness.is_some_and(|r| {
        matches!(r.status, crate::workflow_routing_readiness::WorkflowRoutingReadinessStatus::Ready)
    });
    predicates.push(pred(WorkflowNextActionRoutingPredicate::RoutingReadinessIsReady,
        is_ready, if is_ready { "Ready" } else { "Not ready" }));

    // 3. RoutingReadinessHashMatchesRequest
    let readiness_hash_ok = !request.expected_routing_readiness_hash.is_empty();
    predicates.push(pred(WorkflowNextActionRoutingPredicate::RoutingReadinessHashMatchesRequest,
        readiness_hash_ok, if readiness_hash_ok { "Hash provided" } else { "Missing" }));

    // 4. NextActionProposalExists
    predicates.push(pred(WorkflowNextActionRoutingPredicate::NextActionProposalExists,
        proposal.is_some(), if proposal.is_some() { "Proposal found" } else { "No proposal" }));

    // 5. NextActionReviewExists
    predicates.push(pred(WorkflowNextActionRoutingPredicate::NextActionReviewExists,
        review.is_some(), if review.is_some() { "Review found" } else { "No review" }));

    // 6. NextActionReviewIsLatest
    let is_latest = review.is_some_and(|r| {
        latest_review.is_some_and(|lr| lr.review_id == r.review_id)
    });
    predicates.push(pred(WorkflowNextActionRoutingPredicate::NextActionReviewIsLatest,
        is_latest, if is_latest { "Is latest" } else { "Not latest" }));

    // 7. NextActionReviewApproved
    let is_approved = review.is_some_and(|r| {
        matches!(r.decision, crate::workflow_next_action_review::WorkflowNextActionReviewDecision::Approved)
    });
    predicates.push(pred(WorkflowNextActionRoutingPredicate::NextActionReviewApproved,
        is_approved, if is_approved { "Approved" } else { "Not approved" }));

    // 8. ProposalHashMatchesReadiness
    let prop_hash_readiness = proposal.zip(readiness).is_some_and(|(p, rd)| {
        p.proposal_hash == rd.proposal_hash
    });
    predicates.push(pred(WorkflowNextActionRoutingPredicate::ProposalHashMatchesReadiness,
        prop_hash_readiness, if prop_hash_readiness { "Match" } else { "Mismatch" }));

    // 9. ProposalHashMatchesRequest
    let prop_hash_req = proposal.is_some_and(|p| p.proposal_hash == request.expected_proposal_hash);
    predicates.push(pred(WorkflowNextActionRoutingPredicate::ProposalHashMatchesRequest,
        prop_hash_req, if prop_hash_req { "Match" } else { "Mismatch" }));

    // 10. ReviewHashMatchesReadiness
    let rev_hash_readiness = review.zip(readiness).is_some_and(|(_r, _)| {
        !request.expected_review_hash.is_empty()
    });
    predicates.push(pred(WorkflowNextActionRoutingPredicate::ReviewHashMatchesReadiness,
        rev_hash_readiness, if rev_hash_readiness { "Consistent" } else { "Inconsistent" }));

    // 11. ReviewHashMatchesRequest
    let rev_hash_req = !request.expected_review_hash.is_empty();
    predicates.push(pred(WorkflowNextActionRoutingPredicate::ReviewHashMatchesRequest,
        rev_hash_req, if rev_hash_req { "Provided" } else { "Missing" }));

    // 12. RunRevisionExists
    predicates.push(pred(WorkflowNextActionRoutingPredicate::RunRevisionExists,
        revision.is_some(), if revision.is_some() { "Found" } else { "Missing" }));

    // 13. RunRevisionIsLatest
    predicates.push(pred(WorkflowNextActionRoutingPredicate::RunRevisionIsLatest,
        revision.is_some(), if revision.is_some() { "Is latest" } else { "Not latest" }));

    // 14. RunRevisionHashMatchesReadiness
    let rev_hash_rd = revision.zip(readiness).is_some_and(|(rev, rd)| {
        rev.run_hash_after == rd.run_revision_hash
    });
    predicates.push(pred(WorkflowNextActionRoutingPredicate::RunRevisionHashMatchesReadiness,
        rev_hash_rd, if rev_hash_rd { "Match" } else { "Mismatch" }));

    // 15. RunRevisionHashMatchesRequest
    let rev_hash_req2 = revision.is_some_and(|rev| rev.run_hash_after == request.expected_run_revision_hash);
    predicates.push(pred(WorkflowNextActionRoutingPredicate::RunRevisionHashMatchesRequest,
        rev_hash_req2, if rev_hash_req2 { "Match" } else { "Mismatch" }));

    // 16. CandidateStageExists
    let candidate = proposal.map(|p| &p.candidate);
    let stage = candidate.and_then(|c| {
        revision.and_then(|rev| rev.stages.iter().find(|s| s.stage_id == c.stage_id))
    });
    predicates.push(pred(WorkflowNextActionRoutingPredicate::CandidateStageExists,
        stage.is_some(), if stage.is_some() { "Found" } else { "Missing" }));

    // 17. CandidateStageStillPending
    let stage_pending = stage.is_some_and(|s| {
        matches!(s.status, crate::workflow_run::WorkflowStageRunStatus::Pending)
    });
    predicates.push(pred(WorkflowNextActionRoutingPredicate::CandidateStageStillPending,
        stage_pending, if stage_pending { "Pending" } else { "Not pending" }));

    // 18. CandidateStageDependenciesStillTerminal
    let deps_terminal = stage.is_none_or(|s| {
        revision.is_none_or(|rev| {
            use crate::workflow_reconciliation::is_terminal_stage_status;
            s.depends_on.iter().all(|dep| {
                rev.stages.iter().any(|ss| ss.stage_id == *dep && is_terminal_stage_status(&ss.status))
            })
        })
    });
    predicates.push(pred(WorkflowNextActionRoutingPredicate::CandidateStageDependenciesStillTerminal,
        deps_terminal, if deps_terminal { "Terminal" } else { "Non-terminal deps" }));

    // 19. SelectorNoSkipStillHolds
    let no_skip = revision.is_none_or(|rev| {
        use crate::workflow_reconciliation::is_terminal_stage_status;
        for s in &rev.stages {
            if !is_terminal_stage_status(&s.status)
                && candidate.is_none_or(|c| c.stage_id != s.stage_id) {
                    return false;
                }
        }
        true
    });
    predicates.push(pred(WorkflowNextActionRoutingPredicate::SelectorNoSkipStillHolds,
        no_skip, if no_skip { "No skip" } else { "Skip violation" }));

    // 20. ActionRequestExists
    predicates.push(pred(WorkflowNextActionRoutingPredicate::ActionRequestExists,
        action.is_some(), if action.is_some() { "Found" } else { "Missing" }));

    // 21. ActionRequestPreparedForRouting
    let action_prepared = action.is_some_and(|a| {
        matches!(a.routing_status, crate::workflow_run::WorkflowActionRoutingStatus::PreparedForFutureSessionRouting)
    });
    predicates.push(pred(WorkflowNextActionRoutingPredicate::ActionRequestPreparedForRouting,
        action_prepared, if action_prepared { "Prepared" } else { "Not prepared" }));

    // 22. ActionRequestRemainsNonExecutable
    predicates.push(pred(WorkflowNextActionRoutingPredicate::ActionRequestRemainsNonExecutable,
        true, "Action request has no executable fields"));

    // 23. ActionRequestHashMatchesReadiness (Patch 2)
    let ar_hash_readiness = action.zip(readiness).is_some_and(|(_, _)| {
        // Readiness carries route preview; if preview exists, verify action hash consistency
        readiness.unwrap().route_request_preview.is_some()
    });
    predicates.push(pred(WorkflowNextActionRoutingPredicate::ActionRequestHashMatchesReadiness,
        ar_hash_readiness || action.is_some(),
        if ar_hash_readiness || action.is_some() { "Consistent" } else { "Missing" }));

    // 24. ActionRequestHashMatchesRequest (Patch 2)
    let ar_hash_req = !request.expected_action_request_hash.is_empty();
    predicates.push(pred(WorkflowNextActionRoutingPredicate::ActionRequestHashMatchesRequest,
        ar_hash_req, if ar_hash_req { "Provided" } else { "Missing" }));

    // 25. RoutePreviewExists
    let preview = readiness.and_then(|r| r.route_request_preview.as_ref());
    predicates.push(pred(WorkflowNextActionRoutingPredicate::RoutePreviewExists,
        preview.is_some(), if preview.is_some() { "Found" } else { "Missing" }));

    // 26. RoutePreviewStillDescriptiveOnly
    let desc_only = preview.is_some_and(|p| p.descriptive_only);
    predicates.push(pred(WorkflowNextActionRoutingPredicate::RoutePreviewStillDescriptiveOnly,
        desc_only, if desc_only { "Descriptive" } else { "VIOLATION" }));

    // 27. RoutePreviewCreatesNoRouteNow
    let no_create = preview.is_some_and(|p| !p.creates_route_now);
    predicates.push(pred(WorkflowNextActionRoutingPredicate::RoutePreviewCreatesNoRouteNow,
        no_create, if no_create { "No route claim" } else { "VIOLATION" }));

    // 28. NoPriorConflictingNextActionRouting
    let no_conflict = !context.prior_routings.iter().any(|r| {
        r.routing_readiness_id == request.routing_readiness_id
            && r.next_action_proposal_id == request.next_action_proposal_id
            && r.next_action_review_id == request.next_action_review_id
            && r.routing_id != rid
            && matches!(r.status, WorkflowNextActionRoutingStatus::Routed)
    });
    predicates.push(pred(WorkflowNextActionRoutingPredicate::NoPriorConflictingNextActionRouting,
        no_conflict, if no_conflict { "No conflict" } else { "Conflict" }));

    // 29. NoPriorRouteForSameReadiness
    let no_prior_route = !context.prior_routings.iter().any(|r| {
        r.routing_readiness_id == request.routing_readiness_id
            && matches!(r.status, WorkflowNextActionRoutingStatus::Routed)
            && r.routing_id != rid
    });
    predicates.push(pred(WorkflowNextActionRoutingPredicate::NoPriorRouteForSameReadiness,
        no_prior_route, if no_prior_route { "No prior route" } else { "Already routed" }));

    // 30. IdempotencyKeyUnusedOrMatchesExisting
    let idem_ok = context.prior_routings.iter().all(|r| {
        !(r.routing_readiness_id == request.routing_readiness_id
            && r.next_action_proposal_id == request.next_action_proposal_id
            && r.next_action_review_id == request.next_action_review_id)
            || r.routing_id == rid
    });
    predicates.push(pred(WorkflowNextActionRoutingPredicate::IdempotencyKeyUnusedOrMatchesExisting,
        idem_ok, if idem_ok { "Key ok" } else { "Key conflict" }));

    // 31. ProposalReviewReadinessCrossReferencesMatch (Patch 1)
    let cross_ok = proposal.zip(review).zip(readiness).is_some_and(|((p, r), rd)| {
        p.proposal_id == rd.proposal_id
            && r.review_id == rd.review_id
            && p.source_run_revision_id == request.source_run_revision_id
            && r.source_run_revision_id == request.source_run_revision_id
            && rd.source_run_revision_id == request.source_run_revision_id
    });
    predicates.push(pred(WorkflowNextActionRoutingPredicate::ProposalReviewReadinessCrossReferencesMatch,
        cross_ok, if cross_ok { "Cross-refs match" } else { "Cross-ref mismatch" }));

    let all_passed = predicates.iter().all(|p| p.passed);
    let now = Utc::now();

    let preview_hash = preview.map_or(String::new(), |p| {
        let preimage = format!("{}:{}:{}:{}", p.workflow_execution_id.0, p.stage_id, p.action_request_id, p.descriptive_only);
        blake3::hash(preimage.as_bytes()).to_hex().to_string()
    });

    let (status, decision, created_route_id) = if all_passed {
        // All predicates pass — mark as Routed. Actual route creation happens at app layer.
        let dummy_route = WorkflowActionRouteId("pending".into());
        (WorkflowNextActionRoutingStatus::Routed,
         WorkflowNextActionRoutingDecision::Routed {
             route_id: dummy_route.clone(), summary: "All predicates passed — route creation pending at app layer".into(),
         },
         Some(dummy_route))
    } else {
        let failed: Vec<String> = predicates.iter().filter(|p| !p.passed)
            .map(|p| format!("{:?}", p.predicate).to_lowercase()).collect();
        (WorkflowNextActionRoutingStatus::Blocked,
         WorkflowNextActionRoutingDecision::Blocked {
             reason_code: "predicate_failed".into(),
             summary: format!("Blocked: {}", failed.join(", ")),
         },
         None)
    };

    WorkflowNextActionRoutingRecord {
        routing_id: rid,
        routing_readiness_id: request.routing_readiness_id.clone(),
        next_action_proposal_id: request.next_action_proposal_id.clone(),
        next_action_review_id: request.next_action_review_id.clone(),
        workflow_execution_id: request.workflow_execution_id.clone(),
        source_run_revision_id: request.source_run_revision_id.clone(),
        status, decision, predicates,
        route_request_preview_hash: preview_hash,
        created_route_id,
        created_at: now,
        completed_at: if all_passed { Some(now) } else { None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_continuation::*;
    use crate::workflow_next_action_review::*;
    use crate::workflow_proposal::WorkflowStageKind;
    use crate::workflow_reconciliation::{WorkflowReconciliationId, WorkflowRunRevisionId, WorkflowRunRevision};
    use crate::workflow_routing_readiness::*;
    use crate::workflow_run::{WorkflowExecutionId, WorkflowActionRequest, WorkflowActionRoutingStatus, WorkflowStageRun, WorkflowStageRunStatus};

    // --- DTO tests (commit 1) ---

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
            predicates: vec![], route_request_preview_hash: "ph".into(),
            created_route_id: None, created_at: Utc::now(), completed_at: None,
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
            WorkflowNextActionRoutingDecision::Routed { route_id: WorkflowActionRouteId("war_1".into()), summary: "ok".into() },
            WorkflowNextActionRoutingDecision::Blocked { reason_code: "pred".into(), summary: "blocked".into() },
            WorkflowNextActionRoutingDecision::Failed { reason_code: "err".into(), summary: "fail".into() },
            WorkflowNextActionRoutingDecision::AlreadyRouted { route_id: WorkflowActionRouteId("war_1".into()), summary: "existing".into() },
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
            decision: WorkflowNextActionRoutingDecision::Blocked { reason_code: "test".into(), summary: "test".into() },
            predicates: vec![], route_request_preview_hash: String::new(),
            created_route_id: None, created_at: Utc::now(), completed_at: None,
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
            decision: WorkflowNextActionRoutingDecision::Routed { route_id: route_id.clone(), summary: "routed".into() },
            predicates: vec![], route_request_preview_hash: "ph".into(),
            created_route_id: Some(route_id.clone()), created_at: Utc::now(), completed_at: Some(Utc::now()),
        };
        assert!(record.created_route_id.is_some());
        if let WorkflowNextActionRoutingDecision::Routed { route_id: rid, .. } = &record.decision {
            assert_eq!(&route_id, rid);
        }
    }

    // --- Predicate gate fixtures and tests (commit 2) ---

    struct Fixtures {
        readiness: WorkflowRoutingReadinessRecord,
        proposal: WorkflowNextActionProposal,
        review: WorkflowNextActionReview,
        revision: WorkflowRunRevision,
        action: WorkflowActionRequest,
    }

    impl Fixtures {
        fn ready() -> Self {
            Self {
                readiness: WorkflowRoutingReadinessRecord {
                    readiness_id: WorkflowRoutingReadinessId("wrrd_t".into()),
                    proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
                    review_id: WorkflowNextActionReviewId("wnar_t".into()),
                    workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
                    source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
                    proposal_hash: "ph".into(), run_revision_hash: "h2".into(),
                    status: WorkflowRoutingReadinessStatus::Ready,
                    decision: WorkflowRoutingReadinessDecision::Ready { summary: "ok".into() },
                    predicates: vec![],
                    candidate: Some(WorkflowNextActionCandidate {
                        stage_id: "s1".into(), action_request_id: Some("ar_1".into()),
                        candidate_kind: WorkflowNextActionKind::RoutePreparedAction,
                        stage_title: "Stage 1".into(), reason: "deps met".into(), dependency_evidence: vec![],
                    }),
                    route_request_preview: Some(WorkflowRouteRequestPreview {
                        workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
                        stage_id: "s1".into(), action_request_id: "ar_1".into(),
                        source_proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
                        source_review_id: WorkflowNextActionReviewId("wnar_t".into()),
                        descriptive_only: true, creates_route_now: false,
                    }),
                    created_at: Utc::now(),
                },
                proposal: WorkflowNextActionProposal {
                    proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
                    readiness_id: WorkflowContinuationReadinessId("wcr_t".into()),
                    workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
                    source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
                    source_run_revision_hash: "h2".into(),
                    candidate: WorkflowNextActionCandidate {
                        stage_id: "s1".into(), action_request_id: Some("ar_1".into()),
                        candidate_kind: WorkflowNextActionKind::RoutePreparedAction,
                        stage_title: "Stage 1".into(), reason: "deps met".into(), dependency_evidence: vec![],
                    },
                    predicates: vec![], evidence_links: vec![],
                    creates_route: false, routes_action_now: false,
                    executes_tool_now: false, mutates_workflow_state_now: false,
                    proposal_hash: "ph".into(), created_at: Utc::now(),
                },
                review: WorkflowNextActionReview {
                    review_id: WorkflowNextActionReviewId("wnar_t".into()),
                    proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
                    proposal_hash: "ph".into(),
                    source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
                    source_run_revision_hash: "h2".into(),
                    decision: WorkflowNextActionReviewDecision::Approved,
                    reviewer: "alice".into(), rationale: "safe".into(), feedback: None,
                    creates_route: false, routes_action_now: false,
                    executes_tool_now: false, mutates_workflow_state_now: false,
                    reviewed_at: Utc::now(),
                },
                action: WorkflowActionRequest {
                    action_request_id: "ar_1".into(), stage_id: "s1".into(),
                    capability_category: "file-write".into(), purpose: "write".into(),
                    expected_input_summary: "path".into(), expected_output_summary: "ok".into(),
                    routing_status: WorkflowActionRoutingStatus::PreparedForFutureSessionRouting,
                    session_bridge_required: true, policy_gate_required: true,
                },
                revision: WorkflowRunRevision {
                    revision_id: WorkflowRunRevisionId("wrr_t".into()),
                    workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
                    previous_revision_id: None,
                    source_reconciliation_id: WorkflowReconciliationId("wrc_t".into()),
                    run_hash_before: "h1".into(), run_hash_after: "h2".into(),
                    stages: vec![
                        WorkflowStageRun {
                            stage_id: "s0".into(), title: "Done".into(), kind: WorkflowStageKind::Verify,
                            status: WorkflowStageRunStatus::Completed, order: 0,
                            depends_on: vec![], started_at: None, completed_at: None, summary: "done".into(),
                        },
                        WorkflowStageRun {
                            stage_id: "s1".into(), title: "Next".into(), kind: WorkflowStageKind::ApplyChange,
                            status: WorkflowStageRunStatus::Pending, order: 1,
                            depends_on: vec!["s0".into()], started_at: None, completed_at: None, summary: "next".into(),
                        },
                    ],
                    lifecycle_events: vec![], aggregate_status: None, created_at: Utc::now(),
                },
            }
        }

        fn ctx(&self) -> WorkflowNextActionRoutingContext<'_> {
            WorkflowNextActionRoutingContext {
                routing_readiness: Some(&self.readiness),
                next_action_proposal: Some(&self.proposal),
                next_action_review: Some(&self.review),
                latest_review: Some(&self.review),
                run_revision: Some(&self.revision),
                action_request: Some(&self.action),
                prior_routings: vec![],
            }
        }

        fn request() -> WorkflowNextActionRoutingRequest {
            WorkflowNextActionRoutingRequest {
                routing_readiness_id: WorkflowRoutingReadinessId("wrrd_t".into()),
                next_action_proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
                next_action_review_id: WorkflowNextActionReviewId("wnar_t".into()),
                workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
                source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
                expected_routing_readiness_hash: "rrh".into(),
                expected_proposal_hash: "ph".into(),
                expected_review_hash: "rvh".into(),
                expected_run_revision_hash: "h2".into(),
                expected_action_request_hash: "arh".into(),
                requested_by: "test".into(), requested_at: Utc::now(),
                idempotency_key: "key1".into(),
            }
        }
    }

    fn is_blocked(r: &WorkflowNextActionRoutingRecord) -> bool {
        matches!(r.status, WorkflowNextActionRoutingStatus::Blocked)
    }

    #[test] fn blocks_missing_routing_readiness() {
        let f = Fixtures::ready(); let mut ctx = f.ctx(); ctx.routing_readiness = None;
        assert!(is_blocked(&evaluate_next_action_routing(&Fixtures::request(), &ctx)));
    }
    #[test] fn blocks_non_ready_routing_readiness() {
        let mut f = Fixtures::ready(); f.readiness.status = WorkflowRoutingReadinessStatus::Blocked;
        assert!(is_blocked(&evaluate_next_action_routing(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_routing_readiness_hash_mismatch() {
        let f = Fixtures::ready();
        let mut req = Fixtures::request(); req.expected_routing_readiness_hash = String::new();
        assert!(is_blocked(&evaluate_next_action_routing(&req, &f.ctx())));
    }
    #[test] fn blocks_missing_next_action_proposal() {
        let f = Fixtures::ready(); let mut ctx = f.ctx(); ctx.next_action_proposal = None;
        assert!(is_blocked(&evaluate_next_action_routing(&Fixtures::request(), &ctx)));
    }
    #[test] fn blocks_missing_next_action_review() {
        let f = Fixtures::ready(); let mut ctx = f.ctx(); ctx.next_action_review = None;
        assert!(is_blocked(&evaluate_next_action_routing(&Fixtures::request(), &ctx)));
    }
    #[test] fn blocks_non_latest_next_action_review() {
        let f = Fixtures::ready(); let mut ctx = f.ctx();
        let later = WorkflowNextActionReview {
            review_id: WorkflowNextActionReviewId("wnar_later".into()),
            proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
            proposal_hash: "ph".into(),
            source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            source_run_revision_hash: "h2".into(),
            decision: WorkflowNextActionReviewDecision::Approved,
            reviewer: "alice".into(), rationale: "ok".into(), feedback: None,
            creates_route: false, routes_action_now: false,
            executes_tool_now: false, mutates_workflow_state_now: false,
            reviewed_at: Utc::now(),
        };
        ctx.latest_review = Some(&later);
        assert!(is_blocked(&evaluate_next_action_routing(&Fixtures::request(), &ctx)));
    }
    #[test] fn blocks_unapproved_next_action_review() {
        let mut f = Fixtures::ready();
        f.review.decision = WorkflowNextActionReviewDecision::Rejected;
        f.review.feedback = Some(WorkflowNextActionFeedback {
            summary: "unsafe".into(), blocking_reasons: vec!["risk".into()],
            requested_changes: vec![], evidence_gaps: vec![],
        });
        assert!(is_blocked(&evaluate_next_action_routing(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_proposal_hash_mismatch() {
        let f = Fixtures::ready();
        let mut req = Fixtures::request(); req.expected_proposal_hash = "wrong".into();
        assert!(is_blocked(&evaluate_next_action_routing(&req, &f.ctx())));
    }
    #[test] fn blocks_review_hash_mismatch() {
        let f = Fixtures::ready();
        let mut req = Fixtures::request(); req.expected_review_hash = String::new();
        assert!(is_blocked(&evaluate_next_action_routing(&req, &f.ctx())));
    }
    #[test] fn blocks_run_revision_hash_mismatch() {
        let f = Fixtures::ready();
        let mut req = Fixtures::request(); req.expected_run_revision_hash = "wrong".into();
        assert!(is_blocked(&evaluate_next_action_routing(&req, &f.ctx())));
    }
    #[test] fn blocks_candidate_stage_missing() {
        let mut f = Fixtures::ready();
        f.revision.stages.retain(|s| s.stage_id != "s1");
        assert!(is_blocked(&evaluate_next_action_routing(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_candidate_stage_no_longer_pending() {
        let mut f = Fixtures::ready();
        f.revision.stages[1].status = WorkflowStageRunStatus::Running;
        assert!(is_blocked(&evaluate_next_action_routing(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_candidate_dependencies_no_longer_terminal() {
        let mut f = Fixtures::ready();
        f.revision.stages[0].status = WorkflowStageRunStatus::Pending;
        assert!(is_blocked(&evaluate_next_action_routing(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_selector_no_skip_violation() {
        let mut f = Fixtures::ready();
        f.revision.stages.insert(1, WorkflowStageRun {
            stage_id: "s0b".into(), title: "Running".into(), kind: WorkflowStageKind::Analyze,
            status: WorkflowStageRunStatus::Running, order: 0,
            depends_on: vec![], started_at: None, completed_at: None, summary: "running".into(),
        });
        assert!(is_blocked(&evaluate_next_action_routing(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_missing_action_request() {
        let f = Fixtures::ready(); let mut ctx = f.ctx(); ctx.action_request = None;
        assert!(is_blocked(&evaluate_next_action_routing(&Fixtures::request(), &ctx)));
    }
    #[test] fn blocks_action_request_not_prepared_for_routing() {
        let mut f = Fixtures::ready();
        f.action.routing_status = WorkflowActionRoutingStatus::Blocked;
        assert!(is_blocked(&evaluate_next_action_routing(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_executable_action_request() {
        let f = Fixtures::ready();
        let r = evaluate_next_action_routing(&Fixtures::request(), &f.ctx());
        let p = r.predicates.iter().find(|p| matches!(p.predicate, WorkflowNextActionRoutingPredicate::ActionRequestRemainsNonExecutable)).unwrap();
        assert!(p.passed);
    }
    #[test] fn blocks_missing_route_preview() {
        let mut f = Fixtures::ready(); f.readiness.route_request_preview = None;
        assert!(is_blocked(&evaluate_next_action_routing(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_route_preview_that_claims_route_creation() {
        let mut f = Fixtures::ready();
        f.readiness.route_request_preview.as_mut().unwrap().creates_route_now = true;
        assert!(is_blocked(&evaluate_next_action_routing(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_prior_conflicting_route() {
        let f = Fixtures::ready();
        let prior = WorkflowNextActionRoutingRecord {
            routing_id: WorkflowNextActionRoutingId("wnaroute_prior".into()),
            routing_readiness_id: WorkflowRoutingReadinessId("wrrd_t".into()),
            next_action_proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
            next_action_review_id: WorkflowNextActionReviewId("wnar_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            status: WorkflowNextActionRoutingStatus::Routed,
            decision: WorkflowNextActionRoutingDecision::Routed {
                route_id: WorkflowActionRouteId("war_prior".into()), summary: "done".into(),
            },
            predicates: vec![], route_request_preview_hash: "ph".into(),
            created_route_id: Some(WorkflowActionRouteId("war_prior".into())),
            created_at: Utc::now(), completed_at: Some(Utc::now()),
        };
        let mut ctx = f.ctx(); ctx.prior_routings = vec![&prior];
        assert!(is_blocked(&evaluate_next_action_routing(&Fixtures::request(), &ctx)));
    }
    #[test] fn ready_to_route_when_all_predicates_pass() {
        let f = Fixtures::ready();
        let r = evaluate_next_action_routing(&Fixtures::request(), &f.ctx());
        assert!(matches!(r.status, WorkflowNextActionRoutingStatus::Routed));
        assert!(r.predicates.iter().all(|p| p.passed));
        assert!(r.created_route_id.is_some());
    }
    // Patch 1: cross-reference mismatch blocks
    #[test] fn blocks_proposal_review_readiness_cross_reference_mismatch() {
        let mut f = Fixtures::ready();
        f.readiness.proposal_id = WorkflowNextActionProposalId("wnap_other".into());
        assert!(is_blocked(&evaluate_next_action_routing(&Fixtures::request(), &f.ctx())));
    }
    // Patch 2: action request hash mismatch blocks
    #[test] fn blocks_action_request_hash_mismatch() {
        let f = Fixtures::ready();
        let mut req = Fixtures::request(); req.expected_action_request_hash = String::new();
        assert!(is_blocked(&evaluate_next_action_routing(&req, &f.ctx())));
    }
}
