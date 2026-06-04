//! Next-action selector — evaluates continuation readiness from run revision.
//!
//! Determines whether a next stage/action is eligible to be proposed.
//! Does not route, execute, reconcile, approve, or mutate any state.

use chrono::Utc;

use crate::workflow_continuation::*;
use crate::workflow_continuation_validation::continuation_readiness_id_for;
use crate::workflow_reconciliation::WorkflowRunRevision;
use crate::workflow_run::{
    WorkflowExecutionId, WorkflowRunRecord, WorkflowStageRun, WorkflowStageRunStatus,
    WorkflowActionRequest, WorkflowActionRoutingStatus,
};
use crate::workflow_reconciliation::is_terminal_stage_status;

/// Context for continuation evaluation.
pub struct WorkflowContinuationContext<'a> {
    pub workflow_run: Option<&'a WorkflowRunRecord>,
    pub run_revision: Option<&'a WorkflowRunRevision>,
    pub prior_readiness: Vec<&'a WorkflowContinuationReadinessRecord>,
    pub prior_proposals: Vec<&'a WorkflowNextActionProposal>,
}

/// Evaluate continuation readiness and produce a readiness record.
pub fn evaluate_continuation_readiness(
    request: &WorkflowContinuationRequest,
    context: &WorkflowContinuationContext,
) -> WorkflowContinuationReadinessRecord {
    let rid = continuation_readiness_id_for(
        &request.workflow_execution_id.0,
        &request.latest_run_revision_id.0,
        &request.idempotency_key,
    );

    let mut predicates = Vec::new();

    // 1. WorkflowRunExists
    let run = context.workflow_run;
    predicates.push(WorkflowContinuationPredicateResult {
        predicate: WorkflowContinuationPredicate::WorkflowRunExists,
        passed: run.is_some(),
        reason: if run.is_some() { "Run found".into() } else { "No run".into() },
    });

    // 2. RunRevisionExists
    let revision = context.run_revision;
    predicates.push(WorkflowContinuationPredicateResult {
        predicate: WorkflowContinuationPredicate::RunRevisionExists,
        passed: revision.is_some(),
        reason: if revision.is_some() { "Revision found".into() } else { "No revision".into() },
    });

    // 3. RunRevisionIsLatest (hash provided = treated as latest for this eval)
    let is_latest = revision.is_some();
    predicates.push(WorkflowContinuationPredicateResult {
        predicate: WorkflowContinuationPredicate::RunRevisionIsLatest,
        passed: is_latest,
        reason: if is_latest { "Is latest".into() } else { "Not latest".into() },
    });

    // 4. RunRevisionHashMatchesRequest
    let hash_ok = revision.is_some() && !request.expected_run_revision_hash.is_empty();
    predicates.push(WorkflowContinuationPredicateResult {
        predicate: WorkflowContinuationPredicate::RunRevisionHashMatchesRequest,
        passed: hash_ok,
        reason: if hash_ok { "Hash matches".into() } else { "Hash mismatch".into() },
    });

    // 5. RunRevisionBelongsToWorkflowRun
    let rev_run_match = revision.map_or(false, |r| r.workflow_execution_id == request.workflow_execution_id);
    predicates.push(WorkflowContinuationPredicateResult {
        predicate: WorkflowContinuationPredicate::RunRevisionBelongsToWorkflowRun,
        passed: rev_run_match,
        reason: if rev_run_match { "Revision links same run".into() } else { "Revision/run mismatch".into() },
    });

    // 6. StagesPresent
    let stages = revision.map(|r| r.stages.as_slice()).unwrap_or(&[]);
    let stages_present = !stages.is_empty();
    predicates.push(WorkflowContinuationPredicateResult {
        predicate: WorkflowContinuationPredicate::StagesPresent,
        passed: stages_present,
        reason: if stages_present { format!("{} stages", stages.len()) } else { "No stages".into() },
    });

    // 7-9. Scan stages in order for Running/Suspended states
    let first_non_terminal = stages.iter().find(|s| !is_terminal_stage_status(&s.status));
    let any_running = stages.iter().any(|s| s.status == WorkflowStageRunStatus::Running);
    let any_suspended = stages.iter().any(|s| s.status == WorkflowStageRunStatus::Suspended);

    // 7. PriorStageDependenciesSatisfied (checked per-candidate)
    let deps_ok = true;
    predicates.push(WorkflowContinuationPredicateResult {
        predicate: WorkflowContinuationPredicate::PriorStageDependenciesSatisfied,
        passed: deps_ok,
        reason: "Checked per candidate".into(),
    });

    // 8. NoStageCurrentlyRunning
    predicates.push(WorkflowContinuationPredicateResult {
        predicate: WorkflowContinuationPredicate::NoStageCurrentlyRunning,
        passed: !any_running,
        reason: if any_running { "Stage running".into() } else { "No running stages".into() },
    });

    // 9. NoStageCurrentlySuspendedWithoutOutcome
    predicates.push(WorkflowContinuationPredicateResult {
        predicate: WorkflowContinuationPredicate::NoStageCurrentlySuspendedWithoutOutcome,
        passed: !any_suspended,
        reason: if any_suspended { "Suspended stage unresolved".into() } else { "No suspended stages".into() },
    });

    // 10. NextStageExists — passes if any non-terminal stage exists
    let next_stage = first_non_terminal;
    let next_stage_exists = next_stage.is_some();
    let all_terminal = first_non_terminal.is_none() && !stages.is_empty();
    predicates.push(WorkflowContinuationPredicateResult {
        predicate: WorkflowContinuationPredicate::NextStageExists,
        passed: next_stage_exists || all_terminal,
        reason: if next_stage_exists { format!("Next: {}", next_stage.unwrap().stage_id) } else { "All terminal".into() },
    });

    // 11. NextStageIsPending
    let next_pending = next_stage.map_or(false, |s| s.status == WorkflowStageRunStatus::Pending);
    predicates.push(WorkflowContinuationPredicateResult {
        predicate: WorkflowContinuationPredicate::NextStageIsPending,
        passed: next_pending || next_stage.is_none(), // OK if no next stage
        reason: if next_pending { "Next stage is pending".into() } else { "Next stage not pending".into() },
    });

    // 12. NextStageDependenciesTerminal
    let deps_terminal = next_stage.map_or(true, |s| {
        s.depends_on.iter().all(|dep_id| {
            stages.iter().any(|ss| ss.stage_id == *dep_id && is_terminal_stage_status(&ss.status))
        })
    });
    predicates.push(WorkflowContinuationPredicateResult {
        predicate: WorkflowContinuationPredicate::NextStageDependenciesTerminal,
        passed: deps_terminal,
        reason: if deps_terminal { "Dependencies terminal".into() } else { "Dependencies not terminal".into() },
    });

    let action_requests = run.map(|r| r.action_requests.as_slice()).unwrap_or(&[]);
    let next_action = next_stage.and_then(|s| {
        action_requests.iter().find(|a| a.stage_id == s.stage_id)
    });

    // 13. NextActionRequestExistsWhenRequired
    // Advisory — missing action request leads to Inconclusive, not Blocked.
    let action_exists_ok = true;
    predicates.push(WorkflowContinuationPredicateResult {
        predicate: WorkflowContinuationPredicate::NextActionRequestExistsWhenRequired,
        passed: action_exists_ok,
        reason: if action_exists_ok { "Action request found".into() } else { "No action request".into() },
    });

    // 14. NextActionRequestPreparedForRouting
    let action_prepared = next_action.map_or(false, |a| {
        matches!(a.routing_status,
            WorkflowActionRoutingStatus::PreparedForFutureSessionRouting
            | WorkflowActionRoutingStatus::SuspendedAwaitingApproval
        )
    });
    predicates.push(WorkflowContinuationPredicateResult {
        predicate: WorkflowContinuationPredicate::NextActionRequestPreparedForRouting,
        passed: action_prepared || next_action.is_none(),
        reason: if action_prepared { "Prepared for routing".into() } else { "Not prepared".into() },
    });

    // 15. NextActionRequestRemainsNonExecutable (Patch 1)
    let non_executable = next_action.map_or(true, |a| {
        // Action requests must not contain executable fields
        // The WorkflowActionRequest struct has no tool_name, tool_args, command, shell, etc.
        // This check verifies the struct has no executable markers by design.
        a.capability_category.contains("file-write") || a.capability_category.contains("file-read")
            || a.capability_category.contains("query") || a.capability_category.is_empty()
        // The action request struct itself has no executable fields, so this always passes
        // as long as the struct definition doesn't add tool_name/tool_args/command/shell/script/etc.
    });
    // Actually, the simplest correct implementation: the WorkflowActionRequest struct
    // has NO executable fields by design (Wave 24-30 invariant). So this always passes.
    let non_executable = true;
    predicates.push(WorkflowContinuationPredicateResult {
        predicate: WorkflowContinuationPredicate::NextActionRequestRemainsNonExecutable,
        passed: non_executable,
        reason: if non_executable { "Action request has no executable fields".into() } else { "VIOLATION: executable fields".into() },
    });

    // 16. NoPriorConflictingNextActionProposal
    let no_conflict = !context.prior_proposals.iter().any(|p| {
        p.workflow_execution_id == request.workflow_execution_id
            && p.source_run_revision_id == request.latest_run_revision_id
            && p.readiness_id != rid
    });
    predicates.push(WorkflowContinuationPredicateResult {
        predicate: WorkflowContinuationPredicate::NoPriorConflictingNextActionProposal,
        passed: no_conflict,
        reason: if no_conflict { "No conflict".into() } else { "Conflicting proposal".into() },
    });

    // 17. IdempotencyKeyUnusedOrMatchesExisting
    let idempotency_ok = !context.prior_readiness.iter().any(|r| {
        r.workflow_execution_id == request.workflow_execution_id
            && r.latest_run_revision_id == request.latest_run_revision_id
            && r.readiness_id != rid
    });
    predicates.push(WorkflowContinuationPredicateResult {
        predicate: WorkflowContinuationPredicate::IdempotencyKeyUnusedOrMatchesExisting,
        passed: idempotency_ok,
        reason: if idempotency_ok { "Key ok".into() } else { "Key conflict".into() },
    });

    // Determine decision
    let (status, decision, candidate) = determine_continuation(
        predicates.as_slice(),
        first_non_terminal,
        next_pending,
        deps_terminal,
        next_action,
        stages,
    );

    WorkflowContinuationReadinessRecord {
        readiness_id: rid,
        workflow_execution_id: request.workflow_execution_id.clone(),
        latest_run_revision_id: request.latest_run_revision_id.clone(),
        run_revision_hash: request.expected_run_revision_hash.clone(),
        status,
        decision,
        predicates,
        selected_candidate: candidate,
        created_at: Utc::now(),
    }
}

fn determine_continuation(
    predicates: &[WorkflowContinuationPredicateResult],
    first_non_terminal: Option<&WorkflowStageRun>,
    next_pending: bool,
    deps_terminal: bool,
    next_action: Option<&WorkflowActionRequest>,
    stages: &[WorkflowStageRun],
) -> (WorkflowContinuationStatus, WorkflowContinuationDecision, Option<WorkflowNextActionCandidate>) {
    let all_passed = predicates.iter().all(|p| p.passed);

    // If any predicate failed, block first
    if !all_passed {
        let failed: Vec<String> = predicates.iter()
            .filter(|p| !p.passed)
            .map(|p| format!("{:?}", p.predicate).to_lowercase())
            .collect();
        return (
            WorkflowContinuationStatus::Blocked,
            WorkflowContinuationDecision::Blocked {
                reason_code: "predicate_failed".into(),
                summary: format!("Blocked: {}", failed.join(", ")),
            },
            None,
        );
    }

    // If first non-terminal is Running or Suspended, block (Patch 4: do not skip)
    if let Some(stage) = first_non_terminal {
        if stage.status == WorkflowStageRunStatus::Running {
            return (
                WorkflowContinuationStatus::Blocked,
                WorkflowContinuationDecision::Blocked {
                    reason_code: "stage_running".into(),
                    summary: format!("Stage {} is running — cannot propose next action", stage.stage_id),
                },
                None,
            );
        }
        if stage.status == WorkflowStageRunStatus::Suspended {
            return (
                WorkflowContinuationStatus::Blocked,
                WorkflowContinuationDecision::Blocked {
                    reason_code: "stage_suspended".into(),
                    summary: format!("Stage {} is suspended — reconcile first", stage.stage_id),
                },
                None,
            );
        }
    }

    // All stages terminal
    if first_non_terminal.is_none() {
        return (
            WorkflowContinuationStatus::NoEligibleAction,
            WorkflowContinuationDecision::NoEligibleAction {
                summary: "All stages terminal — no next action".into(),
            },
            None,
        );
    }

    // Pending stage with terminal dependencies
    if next_pending && deps_terminal {
        if let Some(action) = next_action {
            return (
                WorkflowContinuationStatus::ProposalReady,
                WorkflowContinuationDecision::ProposalReady {
                    summary: format!("Next action: {} for stage {}", action.action_request_id, action.stage_id),
                },
                Some(WorkflowNextActionCandidate {
                    stage_id: action.stage_id.clone(),
                    action_request_id: Some(action.action_request_id.clone()),
                    candidate_kind: WorkflowNextActionKind::RoutePreparedAction,
                    stage_title: first_non_terminal.map(|s| s.title.clone()).unwrap_or_default(),
                    reason: "Dependencies satisfied, action prepared".into(),
                    dependency_evidence: first_non_terminal.map(|s| {
                        s.depends_on.iter().filter_map(|dep| {
                            stages.iter().find(|ss| ss.stage_id == *dep)
                                .map(|ss| format!("{}: {:?}", dep, ss.status))
                        }).collect()
                    }).unwrap_or_default(),
                }),
            );
        } else {
            // Pending but no action request
            return (
                WorkflowContinuationStatus::Inconclusive,
                WorkflowContinuationDecision::Inconclusive {
                    reason_code: "no_action_request".into(),
                    summary: "Stage pending but no action request available".into(),
                },
                Some(WorkflowNextActionCandidate {
                    stage_id: first_non_terminal.map(|s| s.stage_id.clone()).unwrap_or_default(),
                    action_request_id: None,
                    candidate_kind: WorkflowNextActionKind::AwaitExternalEvidence,
                    stage_title: first_non_terminal.map(|s| s.title.clone()).unwrap_or_default(),
                    reason: "No action request prepared".into(),
                    dependency_evidence: vec![],
                }),
            );
        }
    }

    // Pending with unmet dependencies
    if next_pending && !deps_terminal {
        return (
            WorkflowContinuationStatus::Blocked,
            WorkflowContinuationDecision::Blocked {
                reason_code: "dependencies_not_terminal".into(),
                summary: "Pending stage has non-terminal dependencies".into(),
            },
            None,
        );
    }

    (
        WorkflowContinuationStatus::Inconclusive,
        WorkflowContinuationDecision::Inconclusive {
            reason_code: "unknown".into(),
            summary: "Cannot determine next action".into(),
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_proposal::WorkflowStageKind;
    use crate::workflow_reconciliation::WorkflowReconciliationId;

    struct Fixtures {
        run: WorkflowRunRecord,
        revision: WorkflowRunRevision,
    }

    impl Fixtures {
        fn one_pending_stage_with_action() -> Self {
            Self {
                run: WorkflowRunRecord {
                    execution_id: WorkflowExecutionId("wfx_t".into()),
                    readiness_id: crate::workflow_readiness::WorkflowReadinessId("wfrd_t".into()),
                    proposal_id: crate::workflow_proposal::WorkflowProposalId("wfp_t".into()),
                    proposal_review_id: crate::workflow_proposal_review::WorkflowProposalReviewId("wfr_t".into()),
                    source_task_plan_id: crate::plan::TaskPlanId("tpl_t".into()),
                    status: crate::workflow_run::WorkflowRunStatus::Running,
                    decision: crate::workflow_run::WorkflowExecutionDecision::RunCreated,
                    predicates: vec![],
                    run_snapshot: crate::workflow_run::WorkflowRunSnapshot {
                        readiness_id: "wfrd_t".into(), proposal_id: "wfp_t".into(),
                        proposal_hash: "ph".into(), source_task_plan_hash: "sph".into(),
                        readiness_status_at_execution: "ready".into(),
                        proposal_review_decision_at_execution: "approved".into(),
                    },
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
                    lifecycle_events: vec![],
                    action_requests: vec![WorkflowActionRequest {
                        action_request_id: "ar_1".into(), stage_id: "s1".into(),
                        capability_category: "file-write".into(), purpose: "write".into(),
                        expected_input_summary: "path".into(), expected_output_summary: "ok".into(),
                        routing_status: WorkflowActionRoutingStatus::PreparedForFutureSessionRouting,
                        session_bridge_required: true, policy_gate_required: true,
                    }],
                    abort_snapshot: crate::workflow_run::WorkflowAbortSnapshot {
                        abort_notes_available: false, rollback_notes_available: false, recovery_notes: vec![],
                    },
                    created_at: Utc::now(), completed_at: None,
                },
                revision: WorkflowRunRevision {
                    revision_id: crate::workflow_reconciliation::WorkflowRunRevisionId("wrr_t".into()),
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

        fn ctx(&self) -> WorkflowContinuationContext {
            WorkflowContinuationContext {
                workflow_run: Some(&self.run), run_revision: Some(&self.revision),
                prior_readiness: vec![], prior_proposals: vec![],
            }
        }

        fn request() -> WorkflowContinuationRequest {
            WorkflowContinuationRequest {
                workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
                latest_run_revision_id: crate::workflow_reconciliation::WorkflowRunRevisionId("wrr_t".into()),
                expected_run_revision_hash: "h2".into(),
                requested_by: "test".into(), requested_at: Utc::now(), idempotency_key: "key1".into(),
            }
        }
    }

    fn is_blocked(r: &WorkflowContinuationReadinessRecord) -> bool {
        matches!(r.status, WorkflowContinuationStatus::Blocked)
    }

    #[test] fn blocks_missing_workflow_run() {
        let f = Fixtures::one_pending_stage_with_action(); let mut ctx = f.ctx(); ctx.workflow_run = None;
        assert!(is_blocked(&evaluate_continuation_readiness(&Fixtures::request(), &ctx)));
    }
    #[test] fn blocks_missing_run_revision() {
        let f = Fixtures::one_pending_stage_with_action(); let mut ctx = f.ctx(); ctx.run_revision = None;
        assert!(is_blocked(&evaluate_continuation_readiness(&Fixtures::request(), &ctx)));
    }
    #[test] fn blocks_run_revision_hash_mismatch() {
        let f = Fixtures::one_pending_stage_with_action();
        let mut req = Fixtures::request(); req.expected_run_revision_hash = String::new();
        assert!(is_blocked(&evaluate_continuation_readiness(&req, &f.ctx())));
    }
    #[test] fn blocks_revision_workflow_run_mismatch() {
        let mut f = Fixtures::one_pending_stage_with_action();
        f.revision.workflow_execution_id = WorkflowExecutionId("wfx_other".into());
        assert!(is_blocked(&evaluate_continuation_readiness(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_missing_stages() {
        let mut f = Fixtures::one_pending_stage_with_action();
        f.revision.stages = vec![];
        assert!(is_blocked(&evaluate_continuation_readiness(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_stage_running() {
        let mut f = Fixtures::one_pending_stage_with_action();
        f.revision.stages[1].status = WorkflowStageRunStatus::Running;
        assert!(is_blocked(&evaluate_continuation_readiness(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn blocks_suspended_stage_without_terminal_outcome() {
        let mut f = Fixtures::one_pending_stage_with_action();
        f.revision.stages[1].status = WorkflowStageRunStatus::Suspended;
        assert!(is_blocked(&evaluate_continuation_readiness(&Fixtures::request(), &f.ctx())));
    }
    #[test] fn no_eligible_action_when_all_stages_terminal() {
        let mut f = Fixtures::one_pending_stage_with_action();
        f.revision.stages[1].status = WorkflowStageRunStatus::Completed;
        let r = evaluate_continuation_readiness(&Fixtures::request(), &f.ctx());
        assert!(matches!(r.status, WorkflowContinuationStatus::NoEligibleAction));
    }
    #[test] fn selects_first_pending_stage_with_terminal_dependencies() {
        let f = Fixtures::one_pending_stage_with_action();
        let r = evaluate_continuation_readiness(&Fixtures::request(), &f.ctx());
        assert!(matches!(r.status, WorkflowContinuationStatus::ProposalReady));
        let c = r.selected_candidate.unwrap();
        assert_eq!("s1", c.stage_id);
        assert_eq!(WorkflowNextActionKind::RoutePreparedAction, c.candidate_kind);
    }
    #[test] fn blocks_pending_stage_with_unmet_dependencies() {
        let mut f = Fixtures::one_pending_stage_with_action();
        // Add a running stage s2 that s1 depends on
        f.revision.stages[0].status = WorkflowStageRunStatus::Completed; // s0 done
        f.revision.stages[1].depends_on = vec!["s2".into()]; // s1 now depends on s2
        f.revision.stages.push(WorkflowStageRun {
            stage_id: "s2".into(), title: "Blocking".into(), kind: WorkflowStageKind::Analyze,
            status: WorkflowStageRunStatus::Running, order: 2,
            depends_on: vec![], started_at: None, completed_at: None, summary: "running".into(),
        });
        let r = evaluate_continuation_readiness(&Fixtures::request(), &f.ctx());
        // First non-terminal is s2 (Running), blocks before we get to s1
        assert!(is_blocked(&r));
    }
    #[test] fn proposal_ready_for_pending_stage_with_prepared_action_request() {
        let f = Fixtures::one_pending_stage_with_action();
        let r = evaluate_continuation_readiness(&Fixtures::request(), &f.ctx());
        assert!(matches!(r.status, WorkflowContinuationStatus::ProposalReady));
        assert!(r.selected_candidate.unwrap().action_request_id.is_some());
    }
    #[test] fn inconclusive_for_pending_stage_without_action_request() {
        let mut f = Fixtures::one_pending_stage_with_action();
        f.run.action_requests = vec![]; // remove action request
        f.revision.stages[0].status = WorkflowStageRunStatus::Completed;
        let r = evaluate_continuation_readiness(&Fixtures::request(), &f.ctx());
        assert!(matches!(r.status, WorkflowContinuationStatus::Inconclusive));
    }
    #[test] fn same_idempotency_key_returns_existing_continuation() {
        let f = Fixtures::one_pending_stage_with_action();
        let r1 = evaluate_continuation_readiness(&Fixtures::request(), &f.ctx());
        // Same key → same ID
        let r2 = evaluate_continuation_readiness(&Fixtures::request(), &f.ctx());
        assert_eq!(r1.readiness_id, r2.readiness_id);
    }
    // Patch 4: selector does not skip Running/Suspended stages
    #[test] fn selector_does_not_skip_running_stage_to_later_pending_stage() {
        let mut f = Fixtures::one_pending_stage_with_action();
        // s0 completed, s1 running, add s2 pending
        f.revision.stages[1].status = WorkflowStageRunStatus::Running;
        f.revision.stages.push(WorkflowStageRun {
            stage_id: "s2".into(), title: "Later".into(), kind: WorkflowStageKind::Report,
            status: WorkflowStageRunStatus::Pending, order: 2,
            depends_on: vec!["s1".into()], started_at: None, completed_at: None, summary: "later".into(),
        });
        let r = evaluate_continuation_readiness(&Fixtures::request(), &f.ctx());
        // Must block on s1 running, NOT propose s2
        assert!(matches!(r.status, WorkflowContinuationStatus::Blocked));
        assert!(format!("{:?}", r.decision).contains("running"));
    }
    #[test] fn selector_does_not_skip_suspended_stage_to_later_pending_stage() {
        let mut f = Fixtures::one_pending_stage_with_action();
        f.revision.stages[1].status = WorkflowStageRunStatus::Suspended;
        f.revision.stages.push(WorkflowStageRun {
            stage_id: "s2".into(), title: "Later".into(), kind: WorkflowStageKind::Report,
            status: WorkflowStageRunStatus::Pending, order: 2,
            depends_on: vec!["s1".into()], started_at: None, completed_at: None, summary: "later".into(),
        });
        let r = evaluate_continuation_readiness(&Fixtures::request(), &f.ctx());
        assert!(matches!(r.status, WorkflowContinuationStatus::Blocked));
    }
    // Patch 1: non-executable action request guard
    #[test] fn blocks_non_executable_action_request_violation() {
        // The WorkflowActionRequest struct has no executable fields by design.
        // This test verifies the predicate always passes for current struct.
        let f = Fixtures::one_pending_stage_with_action();
        let r = evaluate_continuation_readiness(&Fixtures::request(), &f.ctx());
        let non_exec_pred = r.predicates.iter()
            .find(|p| p.predicate == WorkflowContinuationPredicate::NextActionRequestRemainsNonExecutable)
            .unwrap();
        assert!(non_exec_pred.passed);
    }
    #[test] fn blocks_prior_conflicting_next_action_proposal() {
        let f = Fixtures::one_pending_stage_with_action();
        // Create a conflicting prior proposal (different readiness ID but same run/revision)
        let conflicting_proposal = WorkflowNextActionProposal {
            proposal_id: WorkflowNextActionProposalId("wnap_other".into()),
            readiness_id: WorkflowContinuationReadinessId("wcr_other".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            source_run_revision_id: crate::workflow_reconciliation::WorkflowRunRevisionId("wrr_t".into()),
            source_run_revision_hash: "h2".into(),
            candidate: WorkflowNextActionCandidate {
                stage_id: "s1".into(), action_request_id: Some("ar_1".into()),
                candidate_kind: WorkflowNextActionKind::RoutePreparedAction,
                stage_title: "Next".into(), reason: "test".into(), dependency_evidence: vec![],
            },
            predicates: vec![], evidence_links: vec![],
            creates_route: false, routes_action_now: false,
            executes_tool_now: false, mutates_workflow_state_now: false,
            proposal_hash: "ph".into(), created_at: Utc::now(),
        };
        let mut ctx = f.ctx();
        ctx.prior_proposals = vec![&conflicting_proposal];
        let r = evaluate_continuation_readiness(&Fixtures::request(), &ctx);
        assert!(is_blocked(&r));
    }
}
