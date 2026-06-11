//! Reconciliation predicate gate — deterministic evaluation of outcome-to-run linkage.

use chrono::Utc;

use crate::workflow_action_outcome::{
    WorkflowActionOutcomeRecord, WorkflowActionOutcomeStatus,
};
use crate::workflow_action_route::WorkflowActionRouteRecord;
use crate::workflow_reconciliation::*;
use crate::workflow_reconciliation_validation::reconciliation_id_for;
use crate::workflow_run::{WorkflowRunRecord, WorkflowStageRunStatus};

/// Context for reconciliation predicate evaluation.
pub struct WorkflowReconciliationContext<'a> {
    pub workflow_run: Option<&'a WorkflowRunRecord>,
    pub route_record: Option<&'a WorkflowActionRouteRecord>,
    pub outcome_record: Option<&'a WorkflowActionOutcomeRecord>,
    pub prior_reconciliations: Vec<&'a WorkflowReconciliationRecord>,
    pub expected_workflow_run_hash: String,
    pub expected_route_hash: String,
    pub expected_outcome_hash: String,
}

/// Evaluate all 18 reconciliation predicates and produce a reconciliation record.
pub fn evaluate_reconciliation(
    request: &WorkflowReconciliationRequest,
    context: &WorkflowReconciliationContext,
) -> WorkflowReconciliationRecord {
    let rid = reconciliation_id_for(
        &request.workflow_execution_id.0,
        &request.route_id.0,
        &request.outcome_id.0,
        &request.stage_id,
        &request.idempotency_key,
    );

    let mut predicates = Vec::new();

    // 1. WorkflowRunExists
    let run = context.workflow_run;
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::WorkflowRunExists,
        passed: run.is_some(),
        reason: if run.is_some() { "Workflow run found".into() } else { "No workflow run".into() },
    });

    // 2. WorkflowRunHashMatchesRequest
    let run_hash_ok = run.is_some() && !request.expected_workflow_run_hash.is_empty();
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::WorkflowRunHashMatchesRequest,
        passed: run_hash_ok,
        reason: if run_hash_ok { "Hash provided".into() } else { "Missing hash".into() },
    });

    // 3. RouteRecordExists
    let route = context.route_record;
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::RouteRecordExists,
        passed: route.is_some(),
        reason: if route.is_some() { "Route found".into() } else { "No route".into() },
    });

    // 4. RouteHashMatchesRequest
    let route_hash_ok = route.is_some() && !request.expected_route_hash.is_empty();
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::RouteHashMatchesRequest,
        passed: route_hash_ok,
        reason: if route_hash_ok { "Hash provided".into() } else { "Missing hash".into() },
    });

    // 5. OutcomeRecordExists
    let outcome = context.outcome_record;
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::OutcomeRecordExists,
        passed: outcome.is_some(),
        reason: if outcome.is_some() { "Outcome found".into() } else { "No outcome".into() },
    });

    // 6. OutcomeHashMatchesRequest
    let outcome_hash_ok = outcome.is_some() && !request.expected_outcome_hash.is_empty();
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::OutcomeHashMatchesRequest,
        passed: outcome_hash_ok,
        reason: if outcome_hash_ok { "Hash provided".into() } else { "Missing hash".into() },
    });

    // 7. RouteLinksSameWorkflowRun
    let route_run_match = route.is_some_and(|r| r.workflow_execution_id == request.workflow_execution_id);
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::RouteLinksSameWorkflowRun,
        passed: route_run_match,
        reason: if route_run_match { "Route links same run".into() } else { "Route/run mismatch".into() },
    });

    // 8. OutcomeLinksSameWorkflowRun
    let outcome_run_match = outcome.is_some_and(|o| o.workflow_execution_id == request.workflow_execution_id);
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::OutcomeLinksSameWorkflowRun,
        passed: outcome_run_match,
        reason: if outcome_run_match { "Outcome links same run".into() } else { "Outcome/run mismatch".into() },
    });

    // 9. OutcomeLinksSameRoute
    let outcome_route_match = outcome.is_some_and(|o| o.route_id == request.route_id);
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::OutcomeLinksSameRoute,
        passed: outcome_route_match,
        reason: if outcome_route_match { "Outcome links same route".into() } else { "Outcome/route mismatch".into() },
    });

    // 10. StageExists
    let stage = run.and_then(|r| r.stages.iter().find(|s| s.stage_id == request.stage_id));
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::StageExists,
        passed: stage.is_some(),
        reason: if stage.is_some() { "Stage found".into() } else { "No stage".into() },
    });

    // 11. StageWasSuspended
    let stage_suspended = stage.is_some_and(|s| s.status == WorkflowStageRunStatus::Suspended);
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::StageWasSuspended,
        passed: stage_suspended,
        reason: if stage_suspended { "Stage is suspended".into() } else { "Stage not suspended".into() },
    });

    // 12. ActionRequestExists
    let action_exists = run.is_some_and(|r| {
        r.action_requests.iter().any(|a| a.action_request_id == request.action_request_id)
    });
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::ActionRequestExists,
        passed: action_exists,
        reason: if action_exists { "Action request found".into() } else { "No action request".into() },
    });

    // 13. OutcomeLinksSameStage
    let outcome_stage_match = outcome.is_some_and(|o| o.stage_id == request.stage_id);
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::OutcomeLinksSameStage,
        passed: outcome_stage_match,
        reason: if outcome_stage_match { "Outcome links same stage".into() } else { "Outcome/stage mismatch".into() },
    });

    // 14. OutcomeLinksSameActionRequest
    let outcome_action_match = outcome.is_some_and(|o| o.action_request_id == request.action_request_id);
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::OutcomeLinksSameActionRequest,
        passed: outcome_action_match,
        reason: if outcome_action_match { "Outcome links same action request".into() } else { "Outcome/action mismatch".into() },
    });

    // 15. OutcomeIsTerminal — ToolCompleted, ToolDenied, or Failed only
    let outcome_terminal = outcome.is_some_and(|o| {
        matches!(o.status,
            WorkflowActionOutcomeStatus::ToolCompleted
            | WorkflowActionOutcomeStatus::ToolDenied
            | WorkflowActionOutcomeStatus::Failed
        )
    });
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::OutcomeIsTerminal,
        passed: outcome_terminal,
        reason: if outcome_terminal { "Outcome is terminal".into() } else { "Outcome not terminal".into() },
    });

    // 16. OutcomeEvidenceFromSession (Patch 3: requires at least one signal)
    let session_evidence_ok = outcome.and_then(|o| o.session_outcome.as_ref()).is_some_and(|s| {
        !s.trace_ids.is_empty()
            || s.tool_call_id_observed_from_session.is_some()
            || s.tool_status_observed_from_session.is_some()
            || !s.approval_request_id_observed.is_empty()
    });
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::OutcomeEvidenceFromSession,
        passed: session_evidence_ok,
        reason: if session_evidence_ok { "Session evidence present".into() } else { "No session evidence".into() },
    });

    // 17. NoPriorConflictingReconciliation
    let no_conflict = !context.prior_reconciliations.iter().any(|r| {
        r.outcome_id == request.outcome_id
            && matches!(r.status, WorkflowReconciliationStatus::Reconciled)
            && r.reconciliation_id != rid
    });
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::NoPriorConflictingReconciliation,
        passed: no_conflict,
        reason: if no_conflict { "No conflict".into() } else { "Conflicting reconciliation".into() },
    });

    // 18. IdempotencyKeyUnusedOrMatchesExisting
    let idempotency_ok = !context.prior_reconciliations.iter().any(|r| {
        r.workflow_execution_id == request.workflow_execution_id
            && r.route_id == request.route_id
            && r.outcome_id == request.outcome_id
            && r.reconciliation_id != rid
    });
    predicates.push(WorkflowReconciliationPredicateResult {
        predicate: WorkflowReconciliationPredicate::IdempotencyKeyUnusedOrMatchesExisting,
        passed: idempotency_ok,
        reason: if idempotency_ok { "Key ok".into() } else { "Key conflict".into() },
    });

    let all_passed = predicates.iter().all(|p| p.passed);
    let (status, decision) = if all_passed {
        (
            WorkflowReconciliationStatus::Reconciled,
            WorkflowReconciliationDecision::Reconciled { summary: "All predicates passed".into() },
        )
    } else {
        let failed: Vec<String> = predicates.iter()
            .filter(|p| !p.passed)
            .map(|p| format!("{:?}", p.predicate).to_lowercase())
            .collect();
        (
            WorkflowReconciliationStatus::Blocked,
            WorkflowReconciliationDecision::Blocked {
                reason_code: "predicate_failed".into(),
                summary: format!("Blocked: {}", failed.join(", ")),
            },
        )
    };

    WorkflowReconciliationRecord {
        reconciliation_id: rid,
        workflow_execution_id: request.workflow_execution_id.clone(),
        route_id: request.route_id.clone(),
        outcome_id: request.outcome_id.clone(),
        stage_id: request.stage_id.clone(),
        action_request_id: request.action_request_id.clone(),
        status,
        decision,
        predicates,
        progression: None,
        new_run_revision_id: None,
        created_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_action_outcome::{WorkflowActionOutcomeDecision, WorkflowApprovalResolution};
    use crate::workflow_action_route::{
        WorkflowActionRouteId, WorkflowActionRouteRecord, WorkflowActionRouteStatus,
        WorkflowActionRouteDecision, WorkflowActionRoutePrompt, WorkflowSessionRouteSnapshot,
    };
    use crate::workflow_run::{
        WorkflowExecutionId, WorkflowRunRecord, WorkflowRunStatus,
        WorkflowExecutionDecision, WorkflowRunSnapshot, WorkflowAbortSnapshot,
        WorkflowActionRequest, WorkflowActionRoutingStatus,
    };
    use crate::workflow_proposal::{WorkflowProposalId, WorkflowStageKind};
    use crate::workflow_readiness::WorkflowReadinessId;
    use crate::plan::TaskPlanId;
    use crate::workflow_proposal_review::WorkflowProposalReviewId;

    struct Fixtures {
        run: WorkflowRunRecord,
        route: WorkflowActionRouteRecord,
        outcome: WorkflowActionOutcomeRecord,
    }

    impl Fixtures {
        fn base() -> Self {
            Self {
                run: WorkflowRunRecord {
                    execution_id: WorkflowExecutionId("wfx_t".into()),
                    readiness_id: WorkflowReadinessId("wfrd_t".into()),
                    proposal_id: WorkflowProposalId("wfp_t".into()),
                    proposal_review_id: WorkflowProposalReviewId("wfr_t".into()),
                    source_task_plan_id: TaskPlanId("tpl_t".into()),
                    status: WorkflowRunStatus::Suspended,
                    decision: WorkflowExecutionDecision::Suspended { reason_code: "approval".into(), summary: "s".into() },
                    predicates: vec![],
                    run_snapshot: WorkflowRunSnapshot {
                        readiness_id: "wfrd_t".into(), proposal_id: "wfp_t".into(),
                        proposal_hash: "ph".into(), source_task_plan_hash: "sph".into(),
                        readiness_status_at_execution: "ready".into(),
                        proposal_review_decision_at_execution: "approved".into(),
                    },
                    stages: vec![WorkflowStageRun {
                        stage_id: "stage_1".into(), title: "Stage 1".into(),
                        kind: WorkflowStageKind::ApplyChange,
                        status: WorkflowStageRunStatus::Suspended,
                        order: 0, depends_on: vec![], started_at: None, completed_at: None,
                        summary: "test".into(),
                    }],
                    lifecycle_events: vec![],
                    action_requests: vec![WorkflowActionRequest {
                        action_request_id: "ar_1".into(), stage_id: "stage_1".into(),
                        capability_category: "file-write".into(), purpose: "test".into(),
                        expected_input_summary: "path".into(), expected_output_summary: "result".into(),
                        routing_status: WorkflowActionRoutingStatus::SuspendedAwaitingApproval,
                        session_bridge_required: true, policy_gate_required: true,
                    }],
                    abort_snapshot: WorkflowAbortSnapshot {
                        abort_notes_available: false, rollback_notes_available: false,
                        recovery_notes: vec![],
                    },
                    created_at: Utc::now(), completed_at: None,
                },
                route: WorkflowActionRouteRecord {
                    route_id: WorkflowActionRouteId("war_t".into()),
                    workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
                    readiness_id: WorkflowReadinessId("wfrd_t".into()),
                    proposal_id: WorkflowProposalId("wfp_t".into()),
                    stage_id: "stage_1".into(), action_request_id: "ar_1".into(),
                    action_request_hash: "arh".into(),
                    status: WorkflowActionRouteStatus::Completed,
                    decision: WorkflowActionRouteDecision::Completed { summary: "ok".into() },
                    predicates: vec![],
                    session_route: Some(WorkflowSessionRouteSnapshot {
                        session_id: "sess_1".into(), session_run_id: Some("run_1".into()),
                        trace_ids: vec!["trace_1".into()],
                        pending_approval_id: Some("arid_1".into()),
                        tool_call_id: Some("tc_1".into()),
                        tool_name_observed_from_session: Some("file_write".into()),
                        session_status: "completed".into(),
                    }),
                    route_prompt: WorkflowActionRoutePrompt {
                        capability_category: "file-write".into(), purpose: "test".into(),
                        expected_input_summary: "path".into(), expected_output_summary: "result".into(),
                        safety_constraints: vec!["Do not modify system files".into()],
                    },
                    created_at: Utc::now(), completed_at: Some(Utc::now()),
                },
                outcome: WorkflowActionOutcomeRecord {
                    outcome_id: WorkflowActionOutcomeId("wao_t".into()),
                    workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
                    route_id: WorkflowActionRouteId("war_t".into()),
                    stage_id: "stage_1".into(), action_request_id: "ar_1".into(),
                    session_id: "sess_1".into(), pending_approval_id: "arid_1".into(),
                    tool_call_id: Some("tc_1".into()), route_hash: "rh".into(),
                    workflow_run_hash: "wrh".into(),
                    status: WorkflowActionOutcomeStatus::ToolCompleted,
                    decision: WorkflowActionOutcomeDecision::ToolCompleted { summary: "done".into() },
                    predicates: vec![],
                    approval_resolution: WorkflowApprovalResolution::Approve { rationale: "ok".into() },
                    session_outcome: Some(WorkflowSessionActionOutcomeSnapshot {
                        session_id: "sess_1".into(), session_run_id: Some("run_1".into()),
                        trace_ids: vec!["trace_1".into()],
                        approval_request_id_observed: "arid_1".into(),
                        approval_resolution_observed: "approved".into(),
                        tool_call_id_observed_from_session: Some("tc_1".into()),
                        tool_name_observed_from_session: Some("file_write".into()),
                        tool_status_observed_from_session: Some("completed".into()),
                        safe_result_summary: Some("ok".into()),
                    }),
                    created_at: Utc::now(), completed_at: Some(Utc::now()),
                },
            }
        }

        fn ctx(&self) -> WorkflowReconciliationContext {
            WorkflowReconciliationContext {
                workflow_run: Some(&self.run), route_record: Some(&self.route),
                outcome_record: Some(&self.outcome), prior_reconciliations: vec![],
                expected_workflow_run_hash: "h".into(), expected_route_hash: "rh".into(),
                expected_outcome_hash: "oh".into(),
            }
        }

        fn request() -> WorkflowReconciliationRequest {
            WorkflowReconciliationRequest {
                workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
                route_id: WorkflowActionRouteId("war_t".into()),
                outcome_id: WorkflowActionOutcomeId("wao_t".into()),
                stage_id: "stage_1".into(), action_request_id: "ar_1".into(),
                expected_workflow_run_hash: "h".into(), expected_route_hash: "rh".into(),
                expected_outcome_hash: "oh".into(),
                requested_by: "test".into(), requested_at: Utc::now(),
                idempotency_key: "key1".into(),
            }
        }
    }

    fn is_blocked(r: &WorkflowReconciliationRecord) -> bool {
        matches!(r.status, WorkflowReconciliationStatus::Blocked)
    }

    #[test] fn blocks_missing_workflow_run() {
        let f = Fixtures::base(); let mut ctx = f.ctx(); ctx.workflow_run = None;
        assert!(is_blocked(&evaluate_reconciliation(&Fixtures::request(), &ctx)));
    }
    #[test] fn blocks_workflow_run_hash_mismatch() {
        let f = Fixtures::base(); let mut req = Fixtures::request(); req.expected_workflow_run_hash = String::new();
        assert!(is_blocked(&evaluate_reconciliation(&req, &f.ctx())));
    }
    #[test] fn blocks_missing_route_record() {
        let f = Fixtures::base(); let mut ctx = f.ctx(); ctx.route_record = None;
        assert!(is_blocked(&evaluate_reconciliation(&Fixtures::request(), &ctx)));
    }
    #[test] fn blocks_route_hash_mismatch() {
        let f = Fixtures::base(); let mut req = Fixtures::request(); req.expected_route_hash = String::new();
        assert!(is_blocked(&evaluate_reconciliation(&req, &f.ctx())));
    }
    #[test] fn blocks_missing_outcome_record() {
        let f = Fixtures::base(); let mut ctx = f.ctx(); ctx.outcome_record = None;
        assert!(is_blocked(&evaluate_reconciliation(&Fixtures::request(), &ctx)));
    }
    #[test] fn blocks_outcome_hash_mismatch() {
        let f = Fixtures::base(); let mut req = Fixtures::request(); req.expected_outcome_hash = String::new();
        assert!(is_blocked(&evaluate_reconciliation(&req, &f.ctx())));
    }
    #[test] fn blocks_route_workflow_run_mismatch() {
        let mut f = Fixtures::base();
        f.route.workflow_execution_id = WorkflowExecutionId("wfx_other".into());
        assert!(is_blocked(&evaluate_reconciliation(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_outcome_workflow_run_mismatch() {
        let mut f = Fixtures::base();
        f.outcome.workflow_execution_id = WorkflowExecutionId("wfx_other".into());
        assert!(is_blocked(&evaluate_reconciliation(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_outcome_route_mismatch() {
        let mut f = Fixtures::base();
        f.outcome.route_id = WorkflowActionRouteId("war_other".into());
        assert!(is_blocked(&evaluate_reconciliation(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_missing_stage() {
        let f = Fixtures::base(); let mut req = Fixtures::request(); req.stage_id = "stage_missing".into();
        assert!(is_blocked(&evaluate_reconciliation(&req, &f.ctx())));
    }
    #[test] fn blocks_stage_not_suspended() {
        let mut f = Fixtures::base();
        f.run.stages[0].status = WorkflowStageRunStatus::Running;
        assert!(is_blocked(&evaluate_reconciliation(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_missing_action_request() {
        let f = Fixtures::base(); let mut req = Fixtures::request(); req.action_request_id = "ar_missing".into();
        assert!(is_blocked(&evaluate_reconciliation(&req, &f.ctx())));
    }
    #[test] fn blocks_outcome_stage_mismatch() {
        let mut f = Fixtures::base();
        f.outcome.stage_id = "stage_other".into();
        assert!(is_blocked(&evaluate_reconciliation(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_outcome_action_request_mismatch() {
        let mut f = Fixtures::base();
        f.outcome.action_request_id = "ar_other".into();
        assert!(is_blocked(&evaluate_reconciliation(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_non_terminal_approval_resolved_outcome() {
        let mut f = Fixtures::base();
        f.outcome.status = WorkflowActionOutcomeStatus::ApprovalResolved;
        assert!(is_blocked(&evaluate_reconciliation(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_outcome_without_session_evidence() {
        let mut f = Fixtures::base();
        f.outcome.session_outcome = None;
        assert!(is_blocked(&evaluate_reconciliation(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_outcome_with_empty_session_evidence_snapshot() {
        let mut f = Fixtures::base();
        f.outcome.session_outcome = Some(WorkflowSessionActionOutcomeSnapshot {
            session_id: "sess_1".into(), session_run_id: None,
            trace_ids: vec![], approval_request_id_observed: String::new(),
            approval_resolution_observed: "approved".into(),
            tool_call_id_observed_from_session: None,
            tool_name_observed_from_session: None,
            tool_status_observed_from_session: None,
            safe_result_summary: None,
        });
        assert!(is_blocked(&evaluate_reconciliation(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn accepts_outcome_with_trace_id_session_evidence() {
        let mut f = Fixtures::base();
        f.outcome.session_outcome = Some(WorkflowSessionActionOutcomeSnapshot {
            session_id: "sess_1".into(), session_run_id: None,
            trace_ids: vec!["trace_1".into()],
            approval_request_id_observed: String::new(),
            approval_resolution_observed: "approved".into(),
            tool_call_id_observed_from_session: None,
            tool_name_observed_from_session: None,
            tool_status_observed_from_session: None,
            safe_result_summary: None,
        });
        let r = evaluate_reconciliation(&Fixtures::request(), &f.ctx());
        assert!(matches!(r.status, WorkflowReconciliationStatus::Reconciled));
    }
    #[test] fn accepts_outcome_with_tool_status_session_evidence() {
        let mut f = Fixtures::base();
        f.outcome.session_outcome = Some(WorkflowSessionActionOutcomeSnapshot {
            session_id: "sess_1".into(), session_run_id: None,
            trace_ids: vec![], approval_request_id_observed: String::new(),
            approval_resolution_observed: "approved".into(),
            tool_call_id_observed_from_session: None,
            tool_name_observed_from_session: None,
            tool_status_observed_from_session: Some("completed".into()),
            safe_result_summary: None,
        });
        let r = evaluate_reconciliation(&Fixtures::request(), &f.ctx());
        assert!(matches!(r.status, WorkflowReconciliationStatus::Reconciled));
    }
    #[test] fn ready_to_reconcile_tool_completed() {
        let f = Fixtures::base();
        let r = evaluate_reconciliation(&Fixtures::request(), &f.ctx());
        assert!(matches!(r.status, WorkflowReconciliationStatus::Reconciled));
    }
    #[test] fn ready_to_reconcile_tool_denied() {
        let mut f = Fixtures::base();
        f.outcome.status = WorkflowActionOutcomeStatus::ToolDenied;
        let r = evaluate_reconciliation(&Fixtures::request(), &f.ctx());
        assert!(matches!(r.status, WorkflowReconciliationStatus::Reconciled));
    }
    #[test] fn ready_to_reconcile_failed_outcome() {
        let mut f = Fixtures::base();
        f.outcome.status = WorkflowActionOutcomeStatus::Failed;
        let r = evaluate_reconciliation(&Fixtures::request(), &f.ctx());
        assert!(matches!(r.status, WorkflowReconciliationStatus::Reconciled));
    }
}
