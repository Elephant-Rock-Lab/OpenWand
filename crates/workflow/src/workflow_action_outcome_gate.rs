//! Outcome predicate gate — deterministic evaluation of approval resolution readiness.

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::workflow_action_outcome::*;
use crate::workflow_action_outcome_validation::{action_outcome_id_for, validate_resolution_rationale};
use crate::workflow_action_route::{WorkflowActionRouteRecord, WorkflowActionRouteStatus};

/// Context for outcome predicate evaluation.
#[derive(Debug, Clone)]
pub struct WorkflowActionOutcomeContext<'a> {
    pub workflow_run: Option<&'a crate::workflow_run::WorkflowRunRecord>,
    pub route_record: Option<&'a WorkflowActionRouteRecord>,
    pub prior_outcomes: Vec<&'a WorkflowActionOutcomeRecord>,
    pub approval_bridge_available: bool,
    pub workflow_run_hash: String,
    pub route_hash: String,
}

/// Evaluate all 16 outcome predicates and produce an outcome record.
pub fn evaluate_action_outcome(
    request: &WorkflowActionOutcomeRequest,
    context: &WorkflowActionOutcomeContext,
) -> WorkflowActionOutcomeRecord {
    let outcome_id = action_outcome_id_for(
        &request.workflow_execution_id.0,
        &request.route_id.0,
        &request.pending_approval_id,
        &request.idempotency_key,
    );

    let mut predicates = Vec::new();

    // 1. WorkflowRunExists
    let run = context.workflow_run;
    predicates.push(WorkflowActionOutcomePredicateResult {
        predicate: WorkflowActionOutcomePredicate::WorkflowRunExists,
        passed: run.is_some(),
        reason: if run.is_some() { "Workflow run found".into() } else { "No workflow run".into() },
    });

    // 2. WorkflowRunHashMatchesRequest
    let run_hash_ok = run.map_or(false, |_| !request.expected_workflow_run_hash.is_empty());
    predicates.push(WorkflowActionOutcomePredicateResult {
        predicate: WorkflowActionOutcomePredicate::WorkflowRunHashMatchesRequest,
        passed: run_hash_ok,
        reason: if run_hash_ok { "Hash provided".into() } else { "Missing hash".into() },
    });

    // 3. RouteRecordExists
    let route = context.route_record;
    predicates.push(WorkflowActionOutcomePredicateResult {
        predicate: WorkflowActionOutcomePredicate::RouteRecordExists,
        passed: route.is_some(),
        reason: if route.is_some() { "Route found".into() } else { "No route".into() },
    });

    // 4. RouteHashMatchesRequest
    let route_hash_ok = route.map_or(false, |_| !request.expected_route_hash.is_empty());
    predicates.push(WorkflowActionOutcomePredicateResult {
        predicate: WorkflowActionOutcomePredicate::RouteHashMatchesRequest,
        passed: route_hash_ok,
        reason: if route_hash_ok { "Hash provided".into() } else { "Missing hash".into() },
    });

    // 5. RouteIsSuspendedForApproval
    let route_suspended = route.map_or(false, |r| r.status == WorkflowActionRouteStatus::SuspendedForApproval);
    predicates.push(WorkflowActionOutcomePredicateResult {
        predicate: WorkflowActionOutcomePredicate::RouteIsSuspendedForApproval,
        passed: route_suspended,
        reason: if route_suspended { "Route suspended for approval".into() } else { "Route not suspended".into() },
    });

    // 6. RouteLinksSameWorkflowRun
    let run_match = route.map_or(false, |r| r.workflow_execution_id == request.workflow_execution_id);
    predicates.push(WorkflowActionOutcomePredicateResult {
        predicate: WorkflowActionOutcomePredicate::RouteLinksSameWorkflowRun,
        passed: run_match,
        reason: if run_match { "Same workflow run".into() } else { "Workflow run mismatch".into() },
    });

    // 7. RouteLinksSameStage
    let stage_match = route.map_or(false, |r| r.stage_id == request.stage_id);
    predicates.push(WorkflowActionOutcomePredicateResult {
        predicate: WorkflowActionOutcomePredicate::RouteLinksSameStage,
        passed: stage_match,
        reason: if stage_match { "Same stage".into() } else { "Stage mismatch".into() },
    });

    // 8. RouteLinksSameActionRequest
    let ar_match = route.map_or(false, |r| r.action_request_id == request.action_request_id);
    predicates.push(WorkflowActionOutcomePredicateResult {
        predicate: WorkflowActionOutcomePredicate::RouteLinksSameActionRequest,
        passed: ar_match,
        reason: if ar_match { "Same action request".into() } else { "Action request mismatch".into() },
    });

    // 9. RouteLinksSameSession
    let sess_match = route.map_or(false, |r| {
        r.session_route.as_ref().map_or(false, |sr| sr.session_id == request.session_id)
    });
    predicates.push(WorkflowActionOutcomePredicateResult {
        predicate: WorkflowActionOutcomePredicate::RouteLinksSameSession,
        passed: sess_match,
        reason: if sess_match { "Same session".into() } else { "Session mismatch".into() },
    });

    // 10. RouteHasExactlyOnePendingApproval (Patch 4)
    let has_one_approval = route.map_or(false, |r| {
        r.session_route.as_ref().map_or(false, |sr| sr.pending_approval_id.is_some())
    });
    predicates.push(WorkflowActionOutcomePredicateResult {
        predicate: WorkflowActionOutcomePredicate::RouteHasExactlyOnePendingApproval,
        passed: has_one_approval,
        reason: if has_one_approval { "Route has exactly one pending approval".into() } else { "Route has no or ambiguous pending approval".into() },
    });

    // 11. PendingApprovalIdMatchesRoute
    let approval_match = route.map_or(false, |r| {
        r.session_route.as_ref().map_or(false, |sr| {
            sr.pending_approval_id.as_ref().map_or(false, |id| *id == request.pending_approval_id)
        })
    });
    predicates.push(WorkflowActionOutcomePredicateResult {
        predicate: WorkflowActionOutcomePredicate::PendingApprovalIdMatchesRoute,
        passed: approval_match,
        reason: if approval_match { "Pending approval matches".into() } else { "Pending approval mismatch".into() },
    });

    // 12. ToolCallIdMatchesWhenPresent
    let tc_match = if let Some(ref req_tc) = request.tool_call_id {
        route.map_or(false, |r| {
            r.session_route.as_ref().map_or(false, |sr| sr.tool_call_id.as_ref() == Some(req_tc))
        })
    } else {
        true // No tool call ID required
    };
    predicates.push(WorkflowActionOutcomePredicateResult {
        predicate: WorkflowActionOutcomePredicate::ToolCallIdMatchesWhenPresent,
        passed: tc_match,
        reason: if tc_match { "Tool call ID matches or not required".into() } else { "Tool call ID mismatch".into() },
    });

    // 13. ResolutionRationalePresent
    let rationale_ok = validate_resolution_rationale(&request.resolution).is_ok();
    predicates.push(WorkflowActionOutcomePredicateResult {
        predicate: WorkflowActionOutcomePredicate::ResolutionRationalePresent,
        passed: rationale_ok,
        reason: if rationale_ok { "Rationale present".into() } else { "Missing rationale".into() },
    });

    // 14. ApprovalBridgeAvailable
    predicates.push(WorkflowActionOutcomePredicateResult {
        predicate: WorkflowActionOutcomePredicate::ApprovalBridgeAvailable,
        passed: context.approval_bridge_available,
        reason: if context.approval_bridge_available { "Bridge available".into() } else { "No bridge".into() },
    });

    // 15. NoPriorConflictingOutcome
    let no_conflict = !context.prior_outcomes.iter().any(|o| {
        matches!(o.status, WorkflowActionOutcomeStatus::ToolCompleted | WorkflowActionOutcomeStatus::ToolDenied)
            && o.route_id == request.route_id
            && o.pending_approval_id == request.pending_approval_id
    });
    predicates.push(WorkflowActionOutcomePredicateResult {
        predicate: WorkflowActionOutcomePredicate::NoPriorConflictingOutcome,
        passed: no_conflict,
        reason: if no_conflict { "No conflict".into() } else { "Prior completed/denied outcome exists".into() },
    });

    // 16. IdempotencyKeyUnusedOrMatchesExisting
    let existing = context.prior_outcomes.iter().find(|o| o.outcome_id == outcome_id);
    let idempotent = existing.is_some() || context.prior_outcomes.iter().all(|o| {
        !(o.route_id == request.route_id && o.pending_approval_id == request.pending_approval_id)
    });
    predicates.push(WorkflowActionOutcomePredicateResult {
        predicate: WorkflowActionOutcomePredicate::IdempotencyKeyUnusedOrMatchesExisting,
        passed: idempotent,
        reason: if idempotent { "Idempotency valid".into() } else { "Conflicting key".into() },
    });

    let all_pass = predicates.iter().all(|p| p.passed);
    let now = Utc::now();

    let (status, decision) = if existing.is_some() {
        (WorkflowActionOutcomeStatus::AlreadyResolved, WorkflowActionOutcomeDecision::ApprovalResolved { summary: "Existing".into() })
    } else if !all_pass {
        let failed: Vec<_> = predicates.iter().filter(|p| !p.passed).collect();
        let reason_code = format!("predicate_failed_{}", failed.len());
        let summary = failed.iter().map(|p| format!("{:?}", p.predicate)).collect::<Vec<_>>().join(", ");
        (WorkflowActionOutcomeStatus::Blocked, WorkflowActionOutcomeDecision::Blocked { reason_code, summary })
    } else {
        (WorkflowActionOutcomeStatus::ApprovalResolved, WorkflowActionOutcomeDecision::ApprovalResolved { summary: "Ready to resolve".into() })
    };

    let is_blocked = matches!(status, WorkflowActionOutcomeStatus::Blocked);

    WorkflowActionOutcomeRecord {
        // ...
        outcome_id,
        workflow_execution_id: request.workflow_execution_id.clone(),
        route_id: request.route_id.clone(),
        stage_id: request.stage_id.clone(),
        action_request_id: request.action_request_id.clone(),
        session_id: request.session_id.clone(),
        pending_approval_id: request.pending_approval_id.clone(),
        tool_call_id: request.tool_call_id.clone(),
        route_hash: context.route_hash.clone(),
        workflow_run_hash: context.workflow_run_hash.clone(),
        status,
        decision,
        predicates,
        approval_resolution: request.resolution.clone(),
        session_outcome: None,
        created_at: now,
        completed_at: if is_blocked { None } else { Some(now) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_run::*;
    use crate::workflow_action_route::*;
    use crate::workflow_readiness::WorkflowReadinessId;
    use crate::workflow_proposal::WorkflowProposalId;
    use crate::workflow_proposal_review::WorkflowProposalReviewId;
    use crate::plan::TaskPlanId;

    fn suspended_route() -> WorkflowActionRouteRecord {
        WorkflowActionRouteRecord {
            route_id: WorkflowActionRouteId("war_test".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_test".into()),
            readiness_id: WorkflowReadinessId("wfrd_t".into()),
            proposal_id: WorkflowProposalId("wfp_t".into()),
            stage_id: "stage_tool".into(),
            action_request_id: "ar_1".into(),
            action_request_hash: "h".into(),
            status: WorkflowActionRouteStatus::SuspendedForApproval,
            decision: WorkflowActionRouteDecision::SuspendedForApproval {
                approval_request_id: "arid_1".into(), summary: "awaiting".into(),
            },
            predicates: vec![],
            session_route: Some(WorkflowSessionRouteSnapshot {
                session_id: "sess_1".into(), session_run_id: Some("run_1".into()),
                trace_ids: vec!["trace_1".into()],
                pending_approval_id: Some("arid_1".into()),
                tool_call_id: Some("tc_1".into()),
                tool_name_observed_from_session: Some("local__file_write".into()),
                session_status: "suspended_for_approval".into(),
            }),
            route_prompt: WorkflowActionRoutePrompt {
                capability_category: "c".into(), purpose: "p".into(),
                expected_input_summary: "i".into(), expected_output_summary: "o".into(),
                safety_constraints: vec![],
            },
            created_at: Utc::now(), completed_at: None,
        }
    }

    fn test_request() -> WorkflowActionOutcomeRequest {
        WorkflowActionOutcomeRequest {
            workflow_execution_id: WorkflowExecutionId("wfx_test".into()),
            route_id: WorkflowActionRouteId("war_test".into()),
            stage_id: "stage_tool".into(),
            action_request_id: "ar_1".into(),
            session_id: "sess_1".into(),
            pending_approval_id: "arid_1".into(),
            tool_call_id: Some("tc_1".into()),
            expected_route_hash: "rh".into(),
            expected_workflow_run_hash: "wrh".into(),
            resolution: WorkflowApprovalResolution::Approve { rationale: "safe".into() },
            requested_by: "test".into(), requested_at: Utc::now(), idempotency_key: "key1".into(),
        }
    }

    fn full_context<'a>(route: &'a WorkflowActionRouteRecord) -> WorkflowActionOutcomeContext<'a> {
        WorkflowActionOutcomeContext {
            workflow_run: None, route_record: Some(route), prior_outcomes: vec![],
            approval_bridge_available: true, workflow_run_hash: "wrh".into(), route_hash: "rh".into(),
        }
    }

    fn full_context_with_run<'a>(route: &'a WorkflowActionRouteRecord, run: &'a WorkflowRunRecord) -> WorkflowActionOutcomeContext<'a> {
        WorkflowActionOutcomeContext {
            workflow_run: Some(run), route_record: Some(route), prior_outcomes: vec![],
            approval_bridge_available: true, workflow_run_hash: "wrh".into(), route_hash: "rh".into(),
        }
    }

    fn test_run() -> WorkflowRunRecord {
        WorkflowRunRecord {
            execution_id: WorkflowExecutionId("wfx_test".into()),
            readiness_id: WorkflowReadinessId("wfrd_t".into()),
            proposal_id: WorkflowProposalId("wfp_t".into()),
            proposal_review_id: WorkflowProposalReviewId("wfr_t".into()),
            source_task_plan_id: TaskPlanId("tpl_t".into()),
            status: WorkflowRunStatus::Suspended,
            decision: WorkflowExecutionDecision::RunCreated,
            predicates: vec![], run_snapshot: WorkflowRunSnapshot {
                readiness_id: "r".into(), proposal_id: "p".into(), proposal_hash: "h".into(),
                source_task_plan_hash: "s".into(), readiness_status_at_execution: "ready".into(),
                proposal_review_decision_at_execution: "approved".into(),
            },
            stages: vec![], lifecycle_events: vec![], action_requests: vec![],
            abort_snapshot: WorkflowAbortSnapshot { abort_notes_available: false, rollback_notes_available: false, recovery_notes: vec![] },
            created_at: Utc::now(), completed_at: None,
        }
    }

    #[test] fn blocks_missing_workflow_run() {
        let route = suspended_route(); let req = test_request();
        let ctx = full_context(&route);
        let rec = evaluate_action_outcome(&req, &ctx);
        assert_eq!(WorkflowActionOutcomeStatus::Blocked, rec.status);
    }
    #[test] fn blocks_workflow_run_hash_mismatch() {
        let route = suspended_route(); let mut req = test_request();
        req.expected_workflow_run_hash = String::new();
        let ctx = full_context(&route);
        let rec = evaluate_action_outcome(&req, &ctx);
        assert_eq!(WorkflowActionOutcomeStatus::Blocked, rec.status);
    }
    #[test] fn blocks_missing_route_record() {
        let route = suspended_route(); let req = test_request();
        let mut ctx = full_context(&route); ctx.route_record = None;
        let rec = evaluate_action_outcome(&req, &ctx);
        assert_eq!(WorkflowActionOutcomeStatus::Blocked, rec.status);
    }
    #[test] fn blocks_route_hash_mismatch() {
        let route = suspended_route(); let mut req = test_request();
        req.expected_route_hash = String::new();
        let ctx = full_context(&route);
        let rec = evaluate_action_outcome(&req, &ctx);
        assert_eq!(WorkflowActionOutcomeStatus::Blocked, rec.status);
    }
    #[test] fn blocks_route_not_suspended_for_approval() {
        let mut route = suspended_route();
        route.status = WorkflowActionRouteStatus::Completed;
        let req = test_request(); let ctx = full_context(&route);
        let rec = evaluate_action_outcome(&req, &ctx);
        assert_eq!(WorkflowActionOutcomeStatus::Blocked, rec.status);
    }
    #[test] fn blocks_route_workflow_run_mismatch() {
        let route = suspended_route(); let mut req = test_request();
        req.workflow_execution_id = WorkflowExecutionId("wfx_other".into());
        let ctx = full_context(&route);
        let rec = evaluate_action_outcome(&req, &ctx);
        assert_eq!(WorkflowActionOutcomeStatus::Blocked, rec.status);
    }
    #[test] fn blocks_route_stage_mismatch() {
        let route = suspended_route(); let mut req = test_request();
        req.stage_id = "other_stage".into();
        let ctx = full_context(&route);
        let rec = evaluate_action_outcome(&req, &ctx);
        assert_eq!(WorkflowActionOutcomeStatus::Blocked, rec.status);
    }
    #[test] fn blocks_route_action_request_mismatch() {
        let route = suspended_route(); let mut req = test_request();
        req.action_request_id = "ar_other".into();
        let ctx = full_context(&route);
        let rec = evaluate_action_outcome(&req, &ctx);
        assert_eq!(WorkflowActionOutcomeStatus::Blocked, rec.status);
    }
    #[test] fn blocks_route_session_mismatch() {
        let route = suspended_route(); let mut req = test_request();
        req.session_id = "sess_other".into();
        let ctx = full_context(&route);
        let rec = evaluate_action_outcome(&req, &ctx);
        assert_eq!(WorkflowActionOutcomeStatus::Blocked, rec.status);
    }
    #[test] fn blocks_route_without_pending_approval() {
        let mut route = suspended_route();
        route.session_route.as_mut().unwrap().pending_approval_id = None;
        let req = test_request(); let ctx = full_context(&route);
        let rec = evaluate_action_outcome(&req, &ctx);
        assert_eq!(WorkflowActionOutcomeStatus::Blocked, rec.status);
    }
    #[test] fn blocks_route_with_ambiguous_pending_approval() {
        // Same as without — if no pending_approval_id, it's ambiguous
        let mut route = suspended_route();
        route.session_route = None;
        let req = test_request(); let ctx = full_context(&route);
        let rec = evaluate_action_outcome(&req, &ctx);
        assert_eq!(WorkflowActionOutcomeStatus::Blocked, rec.status);
    }
    #[test] fn blocks_pending_approval_id_mismatch() {
        let route = suspended_route(); let mut req = test_request();
        req.pending_approval_id = "arid_other".into();
        let ctx = full_context(&route);
        let rec = evaluate_action_outcome(&req, &ctx);
        assert_eq!(WorkflowActionOutcomeStatus::Blocked, rec.status);
    }
    #[test] fn blocks_tool_call_id_mismatch() {
        let route = suspended_route(); let mut req = test_request();
        req.tool_call_id = Some("tc_other".into());
        let ctx = full_context(&route);
        let rec = evaluate_action_outcome(&req, &ctx);
        assert_eq!(WorkflowActionOutcomeStatus::Blocked, rec.status);
    }
    #[test] fn blocks_missing_rationale() {
        let route = suspended_route(); let mut req = test_request();
        req.resolution = WorkflowApprovalResolution::Approve { rationale: "  ".into() };
        let ctx = full_context(&route);
        let rec = evaluate_action_outcome(&req, &ctx);
        assert_eq!(WorkflowActionOutcomeStatus::Blocked, rec.status);
    }
    #[test] fn blocks_missing_approval_bridge() {
        let route = suspended_route(); let req = test_request();
        let mut ctx = full_context(&route); ctx.approval_bridge_available = false;
        let rec = evaluate_action_outcome(&req, &ctx);
        assert_eq!(WorkflowActionOutcomeStatus::Blocked, rec.status);
    }
    #[test] fn blocks_prior_conflicting_outcome() {
        let route = suspended_route(); let req = test_request();
        let prior = WorkflowActionOutcomeRecord {
            outcome_id: WorkflowActionOutcomeId("wao_prior".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_test".into()),
            route_id: WorkflowActionRouteId("war_test".into()),
            stage_id: "stage_tool".into(), action_request_id: "ar_1".into(),
            session_id: "sess_1".into(), pending_approval_id: "arid_1".into(),
            tool_call_id: Some("tc_1".into()), route_hash: "rh".into(), workflow_run_hash: "wrh".into(),
            status: WorkflowActionOutcomeStatus::ToolCompleted,
            decision: WorkflowActionOutcomeDecision::ToolCompleted { summary: "done".into() },
            predicates: vec![], approval_resolution: WorkflowApprovalResolution::Approve { rationale: "ok".into() },
            session_outcome: None, created_at: Utc::now(), completed_at: Some(Utc::now()),
        };
        let mut ctx = full_context(&route); ctx.prior_outcomes = vec![&prior];
        let rec = evaluate_action_outcome(&req, &ctx);
        assert_eq!(WorkflowActionOutcomeStatus::Blocked, rec.status);
    }
    #[test] fn ready_to_resolve_when_all_predicates_pass() {
        let route = suspended_route(); let req = test_request();
        let run = test_run();
        let ctx = full_context_with_run(&route, &run);
        let rec = evaluate_action_outcome(&req, &ctx);
        assert_eq!(WorkflowActionOutcomeStatus::ApprovalResolved, rec.status);
        assert!(rec.predicates.iter().all(|p| p.passed));
    }
}
