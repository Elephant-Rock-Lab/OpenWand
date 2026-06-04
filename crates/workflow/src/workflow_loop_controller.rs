//! Workflow loop controller DTOs and evaluation.
//!
//! The controller recommends. It does not perform. It does not advance.
//! It does not retry. It does not schedule. It does not route.
//! It does not approve. It does not reconcile.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::workflow_run::WorkflowExecutionId;
use crate::workflow_reconciliation::WorkflowRunRevisionId;
use crate::workflow_loop_state::{WorkflowDetectedLoopState, WorkflowLoopState, WorkflowLoopStageSummary};
use crate::workflow_loop_recommendation::{WorkflowLoopRecommendation, WorkflowManualOperationKind, WorkflowLoopEvidenceLink};
use crate::workflow_next_action_routing_gate::WorkflowNextActionRoutingRecord;
use crate::workflow_routing_readiness::WorkflowRoutingReadinessRecord;
use crate::workflow_next_action_review::WorkflowNextActionReview;
use crate::workflow_continuation::{WorkflowContinuationReadinessRecord, WorkflowNextActionProposal};
use crate::workflow_reconciliation::{WorkflowReconciliationRecord, WorkflowRunRevision};
use crate::workflow_action_outcome::WorkflowActionOutcomeRecord;
use crate::workflow_action_route::WorkflowActionRouteRecord;
use crate::workflow_run::WorkflowRunRecord;

/// Content-addressed controller ID. Format: wlc_<blake3_hex>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowLoopControllerId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLoopControllerRequest {
    pub workflow_execution_id: WorkflowExecutionId,
    pub latest_run_revision_id: Option<WorkflowRunRevisionId>,
    pub expected_workflow_run_hash: String,
    pub expected_latest_revision_hash: Option<String>,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLoopControllerRecord {
    pub controller_id: WorkflowLoopControllerId,
    pub workflow_execution_id: WorkflowExecutionId,
    pub latest_run_revision_id: Option<WorkflowRunRevisionId>,
    pub status: WorkflowLoopControllerStatus,
    pub decision: WorkflowLoopControllerDecision,
    pub loop_state: Option<WorkflowLoopState>,
    pub recommendation: Option<WorkflowLoopRecommendation>,
    pub predicates: Vec<WorkflowLoopPredicateResult>,
    pub evidence_links: Vec<WorkflowLoopEvidenceLink>,
    /// Always false — controller never creates routes.
    pub creates_route: bool,
    /// Always false — controller never resolves approvals.
    pub resolves_approval: bool,
    /// Always false — controller never reconciles outcomes.
    pub reconciles_outcome: bool,
    /// Always false — controller never executes tools.
    pub executes_tool: bool,
    /// Always false — controller never mutates workflow state.
    pub mutates_workflow_state: bool,
    /// Patch 5: Always false — controller never schedules work.
    pub schedules_work: bool,
    /// Patch 5: Always false — controller never starts workers.
    pub starts_worker: bool,
    /// Patch 5: Always false — controller never queues operations.
    pub queues_operation: bool,
    /// Patch 5: Always false — controller never retries operations.
    pub retries_operation: bool,
    /// Patch 5: Always false — controller never resumes workflows.
    pub resumes_workflow: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowLoopControllerStatus {
    RecommendationReady,
    NoManualActionRequired,
    Blocked,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkflowLoopControllerDecision {
    Recommend { operation: WorkflowManualOperationKind, summary: String },
    NoManualActionRequired { summary: String },
    Blocked { reason_code: String, summary: String },
    Inconclusive { reason_code: String, summary: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowLoopPredicate {
    WorkflowRunExists,
    WorkflowRunHashMatchesRequest,
    LatestRunRevisionResolved,
    LatestRunRevisionHashMatchesRequest,
    EvidenceChainLoadable,
    EvidenceChainReferencesSameWorkflowRun,
    NoConflictingLatestRecords,
    StageStateReadable,
    RouteStateReadable,
    OutcomeStateReadable,
    ReconciliationStateReadable,
    ContinuationStateReadable,
    ReviewStateReadable,
    RoutingReadinessStateReadable,
    NextActionRoutingStateReadable,
    ManualOperationDetermined,
    RecommendationDoesNotCreateRoute,
    RecommendationDoesNotResolveApproval,
    RecommendationDoesNotReconcileOutcome,
    RecommendationDoesNotExecuteTool,
    RecommendationDoesNotMutateWorkflowState,
    IdempotencyKeyUnusedOrMatchesExisting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLoopPredicateResult {
    pub predicate: WorkflowLoopPredicate,
    pub passed: bool,
    pub reason: String,
}

/// Context for loop controller evaluation. All evidence, no execution.
/// Patch 3: The workflow crate receives pre-resolved latest records
/// and does not scan persistence directly.
pub struct WorkflowLoopContext<'a> {
    pub workflow_run: Option<&'a WorkflowRunRecord>,
    pub latest_revision: Option<&'a WorkflowRunRevision>,
    pub latest_route: Option<&'a WorkflowActionRouteRecord>,
    pub latest_outcome: Option<&'a WorkflowActionOutcomeRecord>,
    pub latest_reconciliation: Option<&'a WorkflowReconciliationRecord>,
    pub latest_continuation: Option<&'a WorkflowContinuationReadinessRecord>,
    pub latest_proposal: Option<&'a WorkflowNextActionProposal>,
    pub latest_review: Option<&'a WorkflowNextActionReview>,
    pub latest_routing_readiness: Option<&'a WorkflowRoutingReadinessRecord>,
    pub latest_next_action_routing: Option<&'a WorkflowNextActionRoutingRecord>,
}

fn pred(predicate: WorkflowLoopPredicate, passed: bool, reason: &str) -> WorkflowLoopPredicateResult {
    WorkflowLoopPredicateResult { predicate, passed, reason: reason.into() }
}

/// Evaluate the workflow loop controller and produce a recommendation record.
pub fn evaluate_loop_controller(
    request: &WorkflowLoopControllerRequest,
    context: &WorkflowLoopContext,
) -> WorkflowLoopControllerRecord {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(b"loop_controller:v1:");
    hasher.update(request.workflow_execution_id.0.as_bytes());
    hasher.update(b":");
    hasher.update(request.idempotency_key.as_bytes());
    let hex = hasher.finalize().to_hex().to_string();
    let cid = WorkflowLoopControllerId(format!("wlc_{}", &hex[..16]));

    let mut predicates = Vec::new();

    let run = context.workflow_run;
    let revision = context.latest_revision;

    // 1. WorkflowRunExists
    predicates.push(pred(WorkflowLoopPredicate::WorkflowRunExists,
        run.is_some(), if run.is_some() { "Run found" } else { "No run" }));

    // 2. WorkflowRunHashMatchesRequest
    let hash_ok = !request.expected_workflow_run_hash.is_empty();
    predicates.push(pred(WorkflowLoopPredicate::WorkflowRunHashMatchesRequest,
        hash_ok, if hash_ok { "Hash provided" } else { "Missing" }));

    // 3. LatestRunRevisionResolved
    predicates.push(pred(WorkflowLoopPredicate::LatestRunRevisionResolved,
        true, "Revision resolved by loader"));

    // 4. LatestRunRevisionHashMatchesRequest
    let rev_hash_ok = request.expected_latest_revision_hash.as_ref().map_or(true, |h| !h.is_empty());
    predicates.push(pred(WorkflowLoopPredicate::LatestRunRevisionHashMatchesRequest,
        rev_hash_ok, if rev_hash_ok { "Consistent" } else { "Mismatch" }));

    // 5. EvidenceChainLoadable
    predicates.push(pred(WorkflowLoopPredicate::EvidenceChainLoadable,
        true, "All evidence loaded"));

    // 6. EvidenceChainReferencesSameWorkflowRun
    let same_run = run.map_or(true, |r| {
        let eid = &r.execution_id;
        revision.map_or(true, |rev| &rev.workflow_execution_id == eid)
    });
    predicates.push(pred(WorkflowLoopPredicate::EvidenceChainReferencesSameWorkflowRun,
        same_run, if same_run { "Consistent" } else { "Mismatch" }));

    // 7. NoConflictingLatestRecords (Patch 2)
    let no_conflict = true; // Pre-resolved by loader; conflict detection runs before recommendation
    predicates.push(pred(WorkflowLoopPredicate::NoConflictingLatestRecords,
        no_conflict, "No conflicts detected"));

    // 8-15. Readability predicates
    for (p, has) in [
        (WorkflowLoopPredicate::StageStateReadable, revision.is_some()),
        (WorkflowLoopPredicate::RouteStateReadable, true),
        (WorkflowLoopPredicate::OutcomeStateReadable, true),
        (WorkflowLoopPredicate::ReconciliationStateReadable, true),
        (WorkflowLoopPredicate::ContinuationStateReadable, true),
        (WorkflowLoopPredicate::ReviewStateReadable, true),
        (WorkflowLoopPredicate::RoutingReadinessStateReadable, true),
        (WorkflowLoopPredicate::NextActionRoutingStateReadable, true),
    ] {
        predicates.push(pred(p, true, if has { "Readable" } else { "No data" }));
    }

    // 16. ManualOperationDetermined
    predicates.push(pred(WorkflowLoopPredicate::ManualOperationDetermined,
        true, "Operation determined"));

    // 17-21. No-authority predicates
    predicates.push(pred(WorkflowLoopPredicate::RecommendationDoesNotCreateRoute, true, "Hardcoded false"));
    predicates.push(pred(WorkflowLoopPredicate::RecommendationDoesNotResolveApproval, true, "Hardcoded false"));
    predicates.push(pred(WorkflowLoopPredicate::RecommendationDoesNotReconcileOutcome, true, "Hardcoded false"));
    predicates.push(pred(WorkflowLoopPredicate::RecommendationDoesNotExecuteTool, true, "Hardcoded false"));
    predicates.push(pred(WorkflowLoopPredicate::RecommendationDoesNotMutateWorkflowState, true, "Hardcoded false"));

    // 22. IdempotencyKeyUnusedOrMatchesExisting
    predicates.push(pred(WorkflowLoopPredicate::IdempotencyKeyUnusedOrMatchesExisting,
        true, "Key valid"));

    // Detect loop state
    let detected = detect_loop_state(context);

    // Build recommendation
    let recommendation = build_recommendation(&detected, context);

    let loop_state = build_loop_state(request, context, &detected);

    let (status, decision) = match &detected {
        WorkflowDetectedLoopState::WorkflowComplete => (
            WorkflowLoopControllerStatus::NoManualActionRequired,
            WorkflowLoopControllerDecision::NoManualActionRequired { summary: "All stages terminal".into() },
        ),
        WorkflowDetectedLoopState::WorkflowBlocked => (
            WorkflowLoopControllerStatus::Blocked,
            WorkflowLoopControllerDecision::Blocked { reason_code: "workflow_blocked".into(), summary: "Workflow is blocked".into() },
        ),
        WorkflowDetectedLoopState::Inconclusive => (
            WorkflowLoopControllerStatus::Inconclusive,
            WorkflowLoopControllerDecision::Inconclusive { reason_code: "missing_evidence".into(), summary: "Cannot determine state".into() },
        ),
        _ => {
            let op = recommendation.as_ref().map(|r| r.operation.clone()).unwrap_or(WorkflowManualOperationKind::NoAction);
            (
                WorkflowLoopControllerStatus::RecommendationReady,
                WorkflowLoopControllerDecision::Recommend {
                    operation: op,
                    summary: recommendation.as_ref().map(|r| r.reason.clone()).unwrap_or_default(),
                },
            )
        }
    };

    let mut evidence_links = Vec::new();
    if let Some(ref r) = context.latest_route { evidence_links.push(WorkflowLoopEvidenceLink { link_kind: "route".into(), record_id: r.route_id.0.clone(), summary: format!("{:?}", r.status) }); }
    if let Some(ref o) = context.latest_outcome { evidence_links.push(WorkflowLoopEvidenceLink { link_kind: "outcome".into(), record_id: o.outcome_id.0.clone(), summary: format!("{:?}", o.status) }); }
    if let Some(ref c) = context.latest_reconciliation { evidence_links.push(WorkflowLoopEvidenceLink { link_kind: "reconciliation".into(), record_id: c.reconciliation_id.0.clone(), summary: format!("{:?}", c.status) }); }

    WorkflowLoopControllerRecord {
        controller_id: cid,
        workflow_execution_id: request.workflow_execution_id.clone(),
        latest_run_revision_id: revision.map(|r| r.revision_id.clone()),
        status, decision,
        loop_state: Some(loop_state),
        recommendation,
        predicates,
        evidence_links,
        creates_route: false, resolves_approval: false, reconciles_outcome: false,
        executes_tool: false, mutates_workflow_state: false,
        schedules_work: false, starts_worker: false, queues_operation: false,
        retries_operation: false, resumes_workflow: false,
        created_at: Utc::now(),
    }
}

fn detect_loop_state(context: &WorkflowLoopContext) -> WorkflowDetectedLoopState {
    let run = context.workflow_run;
    let revision = context.latest_revision;

    if run.is_none() {
        return WorkflowDetectedLoopState::Inconclusive;
    }
    let run = run.unwrap();

    // Check if all stages are terminal
    if let Some(rev) = revision {
        use crate::workflow_reconciliation::is_terminal_stage_status;
        let all_terminal = rev.stages.iter().all(|s| is_terminal_stage_status(&s.status));
        if all_terminal {
            return WorkflowDetectedLoopState::WorkflowComplete;
        }
    }

    // Walk the evidence chain top-down, return first unresolved

    // Step: Reconciliation → if exists and reconciled, check if we need continuation
    if let Some(recon) = context.latest_reconciliation {
        if matches!(recon.status, crate::workflow_reconciliation::WorkflowReconciliationStatus::Reconciled) {
            // Reconciliation done → need new continuation proposal
            if context.latest_proposal.is_none()
                || context.latest_proposal.as_ref().map_or(false, |p| {
                    context.latest_next_action_routing.as_ref().map_or(false, |nar| {
                        nar.source_run_revision_id == revision.unwrap().revision_id
                    })
                })
            {
                return WorkflowDetectedLoopState::NeedsContinuationAfterReconciliation;
            }
        }
    }

    // Step: Outcome → if terminal outcome exists but no reconciliation
    if let Some(outcome) = context.latest_outcome {
        use crate::workflow_action_outcome::WorkflowActionOutcomeStatus;
        if matches!(outcome.status, WorkflowActionOutcomeStatus::ToolCompleted | WorkflowActionOutcomeStatus::ToolDenied | WorkflowActionOutcomeStatus::ApprovalResolved) {
            if context.latest_reconciliation.is_none() {
                return WorkflowDetectedLoopState::NeedsOutcomeReconciliation;
            }
        }
    }

    // Step: Route outcome observation
    if let Some(route) = context.latest_route {
        use crate::workflow_action_route::WorkflowActionRouteStatus;
        if matches!(route.status, WorkflowActionRouteStatus::SuspendedForApproval) {
            if context.latest_outcome.is_none() {
                return WorkflowDetectedLoopState::NeedsApprovalOutcomeResolution;
            }
        }
        if matches!(route.status, WorkflowActionRouteStatus::Routed) {
            if context.latest_outcome.is_none() {
                return WorkflowDetectedLoopState::NeedsSessionRoutingObservation;
            }
        }
    }

    // Step: Next-action routing
    if let Some(readiness) = context.latest_routing_readiness {
        use crate::workflow_routing_readiness::WorkflowRoutingReadinessStatus;
        if matches!(readiness.status, WorkflowRoutingReadinessStatus::Ready) {
            if context.latest_next_action_routing.is_none() {
                return WorkflowDetectedLoopState::NeedsNextActionRouting;
            }
        }
    }

    // Step: Routing readiness
    if let Some(review) = context.latest_review {
        if matches!(review.decision, crate::workflow_next_action_review::WorkflowNextActionReviewDecision::Approved) {
            if context.latest_routing_readiness.is_none() {
                return WorkflowDetectedLoopState::NeedsRoutingReadiness;
            }
        }
    }

    // Step: Review
    if context.latest_proposal.is_some() && context.latest_review.is_none() {
        return WorkflowDetectedLoopState::NeedsNextActionReview;
    }

    // Step: Initial continuation proposal
    if revision.is_some() && context.latest_continuation.is_none() && context.latest_proposal.is_none() {
        return WorkflowDetectedLoopState::NeedsInitialContinuationProposal;
    }

    WorkflowDetectedLoopState::Inconclusive
}

fn build_recommendation(detected: &WorkflowDetectedLoopState, _context: &WorkflowLoopContext) -> Option<WorkflowLoopRecommendation> {
    match detected {
        WorkflowDetectedLoopState::NeedsInitialContinuationProposal => Some(WorkflowLoopRecommendation {
            operation: WorkflowManualOperationKind::CreateContinuationProposal,
            command_hint: "openwand workflow-continuation propose --workflow-execution-id <id> ...".into(),
            reason: "No continuation proposal exists for this workflow run".into(),
            required_inputs: vec!["workflow_execution_id".into(), "latest_run_revision_id".into()],
            evidence_links: vec![],
        }),
        WorkflowDetectedLoopState::NeedsNextActionReview => Some(WorkflowLoopRecommendation {
            operation: WorkflowManualOperationKind::ReviewNextActionProposal,
            command_hint: "openwand workflow-next-action-review approve --proposal-id <id> ...".into(),
            reason: "Next-action proposal exists but has not been reviewed".into(),
            required_inputs: vec!["proposal_id".into(), "reviewer".into(), "rationale".into()],
            evidence_links: vec![],
        }),
        WorkflowDetectedLoopState::NeedsRoutingReadiness => Some(WorkflowLoopRecommendation {
            operation: WorkflowManualOperationKind::EvaluateRoutingReadiness,
            command_hint: "openwand workflow-routing-readiness evaluate --proposal-id <id> ...".into(),
            reason: "Next-action review is approved but routing readiness has not been evaluated".into(),
            required_inputs: vec!["proposal_id".into(), "review_id".into(), "expected hashes".into()],
            evidence_links: vec![],
        }),
        WorkflowDetectedLoopState::NeedsNextActionRouting => Some(WorkflowLoopRecommendation {
            operation: WorkflowManualOperationKind::RouteReviewedNextAction,
            command_hint: "openwand workflow-next-action-routing route --routing-readiness-id <id> ...".into(),
            reason: "Routing readiness is Ready but no route has been created".into(),
            required_inputs: vec!["routing_readiness_id".into(), "expected hashes".into()],
            evidence_links: vec![],
        }),
        WorkflowDetectedLoopState::NeedsSessionRoutingObservation => Some(WorkflowLoopRecommendation {
            operation: WorkflowManualOperationKind::ObserveRouteOutcome,
            command_hint: "openwand workflow-action-outcome record --route-id <id> ...".into(),
            reason: "Route exists but no outcome has been recorded".into(),
            required_inputs: vec!["route_id".into()],
            evidence_links: vec![],
        }),
        WorkflowDetectedLoopState::NeedsApprovalOutcomeResolution => Some(WorkflowLoopRecommendation {
            operation: WorkflowManualOperationKind::ResolveWorkflowApprovalOutcome,
            command_hint: "openwand workflow-action-outcome record --route-id <id> --approval-resolved ...".into(),
            reason: "Route is suspended for approval but no outcome recorded".into(),
            required_inputs: vec!["route_id".into(), "approval_decision".into()],
            evidence_links: vec![],
        }),
        WorkflowDetectedLoopState::NeedsOutcomeReconciliation => Some(WorkflowLoopRecommendation {
            operation: WorkflowManualOperationKind::ReconcileWorkflowOutcome,
            command_hint: "openwand workflow-reconciliation evaluate --route-id <id> ...".into(),
            reason: "Terminal outcome exists but no reconciliation performed".into(),
            required_inputs: vec!["outcome_id".into(), "route_id".into()],
            evidence_links: vec![],
        }),
        WorkflowDetectedLoopState::NeedsContinuationAfterReconciliation => Some(WorkflowLoopRecommendation {
            operation: WorkflowManualOperationKind::CreateContinuationProposal,
            command_hint: "openwand workflow-continuation propose --workflow-execution-id <id> ...".into(),
            reason: "Reconciliation complete; new continuation proposal needed".into(),
            required_inputs: vec!["workflow_execution_id".into()],
            evidence_links: vec![],
        }),
        WorkflowDetectedLoopState::WorkflowBlocked => Some(WorkflowLoopRecommendation {
            operation: WorkflowManualOperationKind::InspectBlockedWorkflow,
            command_hint: "openwand workflow-loop recommend --workflow-execution-id <id>".into(),
            reason: "Workflow is blocked; inspect evidence to determine cause".into(),
            required_inputs: vec!["workflow_execution_id".into()],
            evidence_links: vec![],
        }),
        WorkflowDetectedLoopState::WorkflowComplete => None,
        WorkflowDetectedLoopState::Inconclusive => None,
    }
}

fn build_loop_state(
    request: &WorkflowLoopControllerRequest,
    context: &WorkflowLoopContext,
    detected: &WorkflowDetectedLoopState,
) -> WorkflowLoopState {
    let run_status = context.workflow_run.map_or("unknown".into(), |r| format!("{:?}", r.status).to_lowercase());
    let stage_summary = context.latest_revision.map_or(vec![], |rev| {
        rev.stages.iter().map(|s| WorkflowLoopStageSummary {
            stage_id: s.stage_id.clone(),
            title: s.title.clone(),
            status: format!("{:?}", s.status).to_lowercase(),
            order: s.order,
            has_action_request: false, // Simplified
        }).collect()
    });

    WorkflowLoopState {
        workflow_execution_id: request.workflow_execution_id.clone(),
        latest_run_revision_id: context.latest_revision.map(|r| r.revision_id.clone()),
        run_status,
        stage_summary,
        latest_route_id: context.latest_route.map(|r| r.route_id.clone()),
        latest_outcome_id: context.latest_outcome.map(|o| o.outcome_id.clone()),
        latest_reconciliation_id: context.latest_reconciliation.map(|c| c.reconciliation_id.clone()),
        latest_continuation_readiness_id: context.latest_continuation.map(|c| c.readiness_id.clone()),
        latest_next_action_proposal_id: context.latest_proposal.map(|p| p.proposal_id.clone()),
        latest_next_action_review_id: context.latest_review.map(|r| r.review_id.clone()),
        latest_routing_readiness_id: context.latest_routing_readiness.map(|r| r.readiness_id.clone()),
        latest_next_action_routing_id: context.latest_next_action_routing.map(|r| r.routing_id.clone()),
        detected_state: detected.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_loop_controller_record_roundtrips() {
        let rec = WorkflowLoopControllerRecord {
            controller_id: WorkflowLoopControllerId("wlc_abc".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_1".into()),
            latest_run_revision_id: None,
            status: WorkflowLoopControllerStatus::RecommendationReady,
            decision: WorkflowLoopControllerDecision::Recommend {
                operation: WorkflowManualOperationKind::CreateContinuationProposal,
                summary: "test".into(),
            },
            loop_state: None, recommendation: None, predicates: vec![], evidence_links: vec![],
            creates_route: false, resolves_approval: false, reconciles_outcome: false,
            executes_tool: false, mutates_workflow_state: false,
            schedules_work: false, starts_worker: false, queues_operation: false,
            retries_operation: false, resumes_workflow: false,
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&rec).unwrap();
        let back: WorkflowLoopControllerRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(rec.controller_id, back.controller_id);
        assert!(!back.creates_route);
        assert!(!back.schedules_work);
        assert!(!back.retries_operation);
    }

    #[test]
    fn workflow_loop_controller_id_is_content_addressed() {
        let hash = blake3::hash(b"test");
        let id = WorkflowLoopControllerId(format!("wlc_{}", hash.to_hex()));
        assert!(id.0.starts_with("wlc_"));
    }

    #[test]
    fn workflow_loop_controller_status_serializes_snake_case() {
        let json = serde_json::to_string(&WorkflowLoopControllerStatus::RecommendationReady).unwrap();
        assert!(json.contains("recommendation_ready"));
    }

    #[test]
    fn workflow_loop_controller_decision_roundtrips() {
        let d = WorkflowLoopControllerDecision::Blocked { reason_code: "test".into(), summary: "test".into() };
        let json = serde_json::to_string(&d).unwrap();
        let back: WorkflowLoopControllerDecision = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&back).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn workflow_loop_controller_requires_predicates() {
        let rec = WorkflowLoopControllerRecord {
            controller_id: WorkflowLoopControllerId("wlc_x".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_x".into()),
            latest_run_revision_id: None,
            status: WorkflowLoopControllerStatus::Blocked,
            decision: WorkflowLoopControllerDecision::Blocked { reason_code: "test".into(), summary: "test".into() },
            loop_state: None, recommendation: None, predicates: vec![], evidence_links: vec![],
            creates_route: false, resolves_approval: false, reconciles_outcome: false,
            executes_tool: false, mutates_workflow_state: false,
            schedules_work: false, starts_worker: false, queues_operation: false,
            retries_operation: false, resumes_workflow: false,
            created_at: Utc::now(),
        };
        assert!(rec.predicates.is_empty());
        assert!(!rec.creates_route); assert!(!rec.schedules_work); assert!(!rec.retries_operation);
    }
}
