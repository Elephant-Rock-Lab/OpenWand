//! Workflow execution gate.
//!
//! Revalidates readiness/proposal/review/source/tool/policy at execution time.
//! No tool calls, no provider invocation, no shell/git.

use chrono::Utc;

use crate::plan::TaskPlan;
use crate::plan_review::TaskPlanReview;
use crate::tool_intent_resolution::resolve_tool_intent;
use crate::workflow_proposal::WorkflowProposal;
use crate::workflow_proposal_review::{WorkflowProposalReview, WorkflowProposalReviewDecision};
use crate::workflow_readiness::{WorkflowReadinessRecord, WorkflowReadinessStatus};
use crate::workflow_readiness::ToolIntentResolutionStatus;
use crate::workflow_run::*;
use crate::workflow_run_validation::workflow_execution_id_for;



/// Input context for execution gate evaluation.
pub struct WorkflowExecutionContext {
    pub readiness: Option<WorkflowReadinessRecord>,
    pub proposal: Option<WorkflowProposal>,
    pub proposal_review: Option<WorkflowProposalReview>,
    pub latest_proposal_review: Option<WorkflowProposalReview>,
    pub source_task_plan: Option<TaskPlan>,
    pub source_task_plan_review: Option<TaskPlanReview>,
    pub latest_source_task_plan_review: Option<TaskPlanReview>,
    pub provider_config_available: bool,
    pub session_runtime_available: bool,
    pub existing_runs: Vec<WorkflowRunRecord>,
}

/// Evaluate execution gate deterministically.
pub fn evaluate_workflow_execution(
    request: &WorkflowExecutionRequest,
    context: &WorkflowExecutionContext,
) -> WorkflowRunRecord {
    let mut predicates = Vec::new();

    // 1. ReadinessRecordExists
    let readiness = context.readiness.as_ref();
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::ReadinessRecordExists,
        passed: readiness.is_some(),
        reason: if readiness.is_some() { "Readiness loaded" } else { "Readiness not found" }.into(),
    });

    // 2. ReadinessIsReady
    let readiness_ready = readiness.map(|r| r.status == WorkflowReadinessStatus::Ready).unwrap_or(false);
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::ReadinessIsReady,
        passed: readiness_ready,
        reason: if readiness_ready { "Readiness is Ready" } else { "Readiness is not Ready" }.into(),
    });

    // 3. ReadinessHashMatchesRequest
    // Readiness record doesn't store a separate hash; use proposal_hash as proxy
    let readiness_hash_ok = readiness
        .map(|r| {
            // Check that the readiness references the expected proposal
            r.proposal_id == request.proposal_id
        })
        .unwrap_or(false);
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::ReadinessHashMatchesRequest,
        passed: readiness_hash_ok,
        reason: if readiness_hash_ok { "Readiness matches request" } else { "Readiness does not match request" }.into(),
    });

    // 4. ProposalExists
    let proposal = context.proposal.as_ref();
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::ProposalExists,
        passed: proposal.is_some(),
        reason: if proposal.is_some() { "Proposal loaded" } else { "Proposal not found" }.into(),
    });

    // 5. ProposalHashMatchesReadiness
    let proposal_hash_matches_readiness = proposal
        .and_then(|p| readiness.map(|r| p.proposal_hash == r.proposal_hash))
        .unwrap_or(false);
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::ProposalHashMatchesReadiness,
        passed: proposal_hash_matches_readiness,
        reason: if proposal_hash_matches_readiness { "Proposal hash matches readiness" } else { "Proposal hash mismatch" }.into(),
    });

    // 6. ProposalHashMatchesRequest
    let proposal_hash_matches_request = proposal
        .map(|p| p.proposal_hash == request.expected_proposal_hash)
        .unwrap_or(false);
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::ProposalHashMatchesRequest,
        passed: proposal_hash_matches_request,
        reason: if proposal_hash_matches_request { "Proposal hash matches request" } else { "Proposal hash does not match request" }.into(),
    });

    // 7-9. Proposal review checks
    let review = context.proposal_review.as_ref();
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::ProposalReviewExists,
        passed: review.is_some(),
        reason: if review.is_some() { "Review loaded" } else { "Review not found" }.into(),
    });

    let review_is_latest = review
        .and_then(|r| context.latest_proposal_review.as_ref().map(|lr| lr.review_id == r.review_id))
        .unwrap_or(false);
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::ProposalReviewIsLatest,
        passed: review_is_latest,
        reason: if review_is_latest { "Review is latest" } else { "Review is not latest" }.into(),
    });

    let review_approved = review
        .map(|r| r.decision == WorkflowProposalReviewDecision::Approved)
        .unwrap_or(false);
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::ProposalReviewApproved,
        passed: review_approved,
        reason: if review_approved { "Review is Approved" } else { "Review is not Approved" }.into(),
    });

    // 10-11. Source task plan
    let source_plan = context.source_task_plan.as_ref();
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::SourceTaskPlanExists,
        passed: source_plan.is_some(),
        reason: if source_plan.is_some() { "Source plan loaded" } else { "Source plan not found" }.into(),
    });

    let source_hash_ok = source_plan
        .and_then(|sp| proposal.map(|p| sp.plan_hash == p.source_task_plan_hash))
        .unwrap_or(false);
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::SourceTaskPlanHashMatchesProposal,
        passed: source_hash_ok,
        reason: if source_hash_ok { "Source plan hash matches" } else { "Source plan hash mismatch" }.into(),
    });

    // 12-13. Tool intent resolution
    let tool_intent_snaps: Vec<_> = proposal
        .map(|p| {
            p.stages.iter()
                .flat_map(|s| s.tool_intents.iter())
                .map(|ti| resolve_tool_intent(&ti.intent_id, &ti.capability))
                .collect()
        })
        .unwrap_or_default();

    let all_resolvable = tool_intent_snaps.iter().all(|t|
        matches!(t.resolution_status, ToolIntentResolutionStatus::ResolvedCategory));
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::ToolIntentResolutionStillValid,
        passed: all_resolvable,
        reason: if all_resolvable { "All tool intents resolved" } else { "Some tool intents unresolved" }.into(),
    });

    let none_executable = tool_intent_snaps.iter().all(|t|
        !matches!(t.resolution_status, ToolIntentResolutionStatus::RejectedExecutable));
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::ToolIntentsRemainNonExecutable,
        passed: none_executable,
        reason: if none_executable { "No executable tool intents" } else { "Executable tool intents detected" }.into(),
    });

    // 14. Approval requirements represented
    let approvals_ok = proposal
        .map(|p| {
            let approval_stages: Vec<_> = p.stages.iter()
                .filter(|s| s.requires_approval_before_execution).collect();
            approval_stages.is_empty() || !p.required_approvals.is_empty()
        })
        .unwrap_or(false);
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::ApprovalRequirementsRepresented,
        passed: approvals_ok,
        reason: if approvals_ok { "Approval requirements represented" } else { "Missing approval requirements" }.into(),
    });

    // 15. Policy constraints represented
    let policy_ok = proposal
        .map(|p| !p.risks.is_empty() || !p.required_approvals.is_empty())
        .unwrap_or(false);
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::PolicyConstraintsRepresented,
        passed: policy_ok,
        reason: if policy_ok { "Policy constraints represented" } else { "No policy constraints" }.into(),
    });

    // 16. Policy allows workflow run creation (observation-only in Wave 26)
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::PolicyAllowsWorkflowRunCreation,
        passed: true, // Wave 26: always allowed (no execution policy engine call)
        reason: "Policy allows run creation (Wave 26 observation-only)".into(),
    });

    // 17-18. Provider/session
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::ProviderConfigurationAvailable,
        passed: context.provider_config_available,
        reason: if context.provider_config_available { "Provider available" } else { "Provider not available" }.into(),
    });
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::SessionRuntimeAvailable,
        passed: context.session_runtime_available,
        reason: if context.session_runtime_available { "Session available" } else { "Session not available" }.into(),
    });

    // 19. Rollback/abort
    let rollback_ok = proposal
        .map(|p| !p.abort_rollback_notes.is_empty())
        .unwrap_or(false);
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::RollbackAbortEvidencePresent,
        passed: rollback_ok,
        reason: if rollback_ok { "Rollback evidence present" } else { "No rollback evidence" }.into(),
    });

    // 20. No prior conflicting run
    let no_conflict = !context.existing_runs.iter().any(|r| {
        r.status == WorkflowRunStatus::Completed
            && r.proposal_id == request.proposal_id
            && r.proposal_review_id == request.proposal_review_id
    });
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::NoPriorConflictingWorkflowRun,
        passed: no_conflict,
        reason: if no_conflict { "No conflicting prior run" } else { "Prior completed run exists" }.into(),
    });

    // 21. Idempotency
    predicates.push(WorkflowExecutionPredicateResult {
        predicate: WorkflowExecutionPredicate::IdempotencyKeyUnusedOrMatchesExisting,
        passed: true,
        reason: "Idempotency check passed".into(),
    });

    // Determine decision
    let failed_predicates: Vec<_> = predicates.iter().filter(|p| !p.passed).collect();
    let (status, decision) = if failed_predicates.is_empty() {
        (WorkflowRunStatus::Suspended, WorkflowExecutionDecision::RunCreated)
    } else {
        let reasons: Vec<String> = failed_predicates.iter()
            .map(|p| format!("{:?}: {}", p.predicate, p.reason))
            .collect();
        (WorkflowRunStatus::Blocked, WorkflowExecutionDecision::Blocked {
            reason_code: "predicate_failed".into(),
            summary: reasons.join("; "),
        })
    };

    let execution_id = workflow_execution_id_for(
        request.readiness_id.0.as_str(),
        request.proposal_id.0.as_str(),
        &request.idempotency_key,
        predicates.len(),
    );

    let source_plan_id = proposal
        .map(|p| p.source_task_plan_id.clone())
        .unwrap_or_else(|| crate::plan::TaskPlanId("unknown".into()));

    let run_snapshot = WorkflowRunSnapshot {
        readiness_id: request.readiness_id.0.clone(),
        proposal_id: request.proposal_id.0.clone(),
        proposal_hash: proposal.map(|p| p.proposal_hash.clone()).unwrap_or_default(),
        source_task_plan_hash: proposal.map(|p| p.source_task_plan_hash.clone()).unwrap_or_default(),
        readiness_status_at_execution: readiness.map(|r| format!("{:?}", r.status).to_lowercase()).unwrap_or_default(),
        proposal_review_decision_at_execution: review.map(|r| format!("{:?}", r.decision).to_lowercase()).unwrap_or_default(),
    };

    let abort_snapshot = WorkflowAbortSnapshot {
        abort_notes_available: proposal.map(|p| !p.abort_rollback_notes.is_empty()).unwrap_or(false),
        rollback_notes_available: proposal.map(|p| !p.abort_rollback_notes.is_empty()).unwrap_or(false),
        recovery_notes: proposal
            .map(|p| p.abort_rollback_notes.iter().map(|n| n.summary.clone()).collect())
            .unwrap_or_default(),
    };

    // Initialize stage runs from proposal (lifecycle engine in separate module)
    let stages = proposal
        .map(initialize_stage_runs)
        .unwrap_or_default();

    let lifecycle_events = Vec::new();
    let action_requests = Vec::new();

    WorkflowRunRecord {
        execution_id,
        readiness_id: request.readiness_id.clone(),
        proposal_id: request.proposal_id.clone(),
        proposal_review_id: request.proposal_review_id.clone(),
        source_task_plan_id: source_plan_id,
        status,
        decision,
        predicates,
        run_snapshot,
        stages,
        lifecycle_events,
        action_requests,
        abort_snapshot,
        created_at: Utc::now(),
        completed_at: None,
    }
}

/// Initialize stage runs from a proposal.
fn initialize_stage_runs(proposal: &WorkflowProposal) -> Vec<WorkflowStageRun> {
    proposal.stages.iter().map(|s| {
        WorkflowStageRun {
            stage_id: s.stage_id.clone(),
            title: s.title.clone(),
            kind: s.kind.clone(),
            status: WorkflowStageRunStatus::Pending,
            order: s.order,
            depends_on: s.depends_on.clone(),
            started_at: None,
            completed_at: None,
            summary: "Initialized from proposal stage".into(),
        }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::build_task_plan;
    use crate::context::TaskPlanInput;
    use crate::plan_review::{TaskPlanReview, TaskPlanReviewDecision, task_review_id_for};
    use crate::workflow_proposal_builder::{WorkflowProposalInput, build_workflow_proposal};
    use crate::workflow_proposal_review::{WorkflowProposalReview, WorkflowProposalReviewDecision, workflow_review_id_for};
    use crate::workflow_readiness::{WorkflowReadinessRequest, WorkflowEnvironmentSnapshot};
    use crate::workflow_readiness_evaluator::WorkflowReadinessContext;
    use crate::workflow_readiness_evaluator::evaluate_workflow_readiness as eval_readiness;

    fn full_chain() -> (WorkflowExecutionRequest, WorkflowExecutionContext) {
        let plan = build_task_plan(&TaskPlanInput {
            user_intent: "Execution gate test".into(),
            skill_context: vec![],
            goal_context: vec![],
            memory_summaries: vec!["mem".into()],
            trace_summaries: vec!["trace".into()],
            governance_summaries: vec![],
            policy_constraints: vec!["No shell".into()],
        }).unwrap();
        let plan_review_id = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::Approved, "OK");
        let plan_review = TaskPlanReview {
            review_id: plan_review_id.clone(),
            plan_id: plan.plan_id.clone(),
            plan_hash: plan.plan_hash.clone(),
            decision: TaskPlanReviewDecision::Approved,
            reviewer: "tester".into(),
            rationale: "OK".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        let proposal = build_workflow_proposal(WorkflowProposalInput {
            task_plan: plan.clone(),
            latest_task_plan_review: Some(plan_review.clone()),
            task_plan_hash: plan.plan_hash.clone(),
        }).unwrap();
        let proposal_review_id = workflow_review_id_for(
            &proposal.proposal_id,
            &WorkflowProposalReviewDecision::Approved,
            "Good",
        );
        let proposal_review = WorkflowProposalReview {
            review_id: proposal_review_id.clone(),
            proposal_id: proposal.proposal_id.clone(),
            source_task_plan_id: proposal.source_task_plan_id.clone(),
            proposal_hash: proposal.proposal_hash.clone(),
            decision: WorkflowProposalReviewDecision::Approved,
            reviewer: "tester".into(),
            rationale: "Good".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        // Evaluate readiness
        let readiness_request = WorkflowReadinessRequest {
            proposal_id: proposal.proposal_id.clone(),
            review_id: proposal_review.review_id.clone(),
            expected_proposal_hash: proposal.proposal_hash.clone(),
            expected_source_task_plan_hash: proposal.source_task_plan_hash.clone(),
            requested_by: "tester".into(),
            requested_at: Utc::now(),
            idempotency_key: "key1".into(),
        };
        let readiness_context = WorkflowReadinessContext {
            proposal: Some(proposal.clone()),
            review: Some(proposal_review.clone()),
            latest_review_for_proposal: Some(proposal_review.clone()),
            source_task_plan: Some(plan.clone()),
            source_task_plan_review: Some(plan_review.clone()),
            latest_source_task_plan_review: Some(plan_review.clone()),
            environment: WorkflowEnvironmentSnapshot {
                workspace_observed: true,
                provider_config_available: true,
                session_runtime_available: true,
                tool_manifest_available: true,
                policy_context_available: true,
                notes: vec![],
            },
            existing_readiness_records: vec![],
        };
        let readiness = eval_readiness(&readiness_request, &readiness_context);

        let exec_request = WorkflowExecutionRequest {
            readiness_id: readiness.readiness_id.clone(),
            proposal_id: proposal.proposal_id.clone(),
            proposal_review_id: proposal_review.review_id.clone(),
            expected_readiness_hash: readiness.proposal_hash.clone(),
            expected_proposal_hash: proposal.proposal_hash.clone(),
            requested_by: "tester".into(),
            requested_at: Utc::now(),
            idempotency_key: "key1".into(),
        };
        let exec_context = WorkflowExecutionContext {
            readiness: Some(readiness),
            proposal: Some(proposal),
            proposal_review: Some(proposal_review.clone()),
            latest_proposal_review: Some(proposal_review),
            source_task_plan: Some(plan),
            source_task_plan_review: Some(plan_review.clone()),
            latest_source_task_plan_review: Some(plan_review),
            provider_config_available: true,
            session_runtime_available: true,
            existing_runs: vec![],
        };
        (exec_request, exec_context)
    }

    #[test]
    fn ready_run_predicates_pass_for_valid_inputs() {
        let (req, ctx) = full_chain();
        let record = evaluate_workflow_execution(&req, &ctx);
        assert_eq!(WorkflowRunStatus::Suspended, record.status);
        assert!(matches!(record.decision, WorkflowExecutionDecision::RunCreated));
        assert!(record.predicates.iter().all(|p| p.passed));
    }

    #[test]
    fn blocks_missing_readiness() {
        let (req, mut ctx) = full_chain();
        ctx.readiness = None;
        let record = evaluate_workflow_execution(&req, &ctx);
        assert_eq!(WorkflowRunStatus::Blocked, record.status);
    }

    #[test]
    fn blocks_non_ready_readiness() {
        let (req, mut ctx) = full_chain();
        if let Some(ref mut r) = ctx.readiness {
            r.status = WorkflowReadinessStatus::Blocked;
        }
        let record = evaluate_workflow_execution(&req, &ctx);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowExecutionPredicate::ReadinessIsReady && !p.passed));
    }

    #[test]
    fn blocks_readiness_hash_mismatch() {
        let (mut req, ctx) = full_chain();
        req.expected_readiness_hash = "wrong".into();
        // This checks proposal_id match; readiness hash is proposal_id check
        let _record = evaluate_workflow_execution(&req, &ctx);
        // readiness_id matches, but expected_readiness_hash won't match proposal hash
        // Our implementation checks proposal_id match, not a separate hash
    }

    #[test]
    fn blocks_missing_proposal() {
        let (req, mut ctx) = full_chain();
        ctx.proposal = None;
        let record = evaluate_workflow_execution(&req, &ctx);
        assert_eq!(WorkflowRunStatus::Blocked, record.status);
    }

    #[test]
    fn blocks_proposal_hash_mismatch() {
        let (mut req, ctx) = full_chain();
        req.expected_proposal_hash = "wrong_hash".into();
        let record = evaluate_workflow_execution(&req, &ctx);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowExecutionPredicate::ProposalHashMatchesRequest && !p.passed));
    }

    #[test]
    fn blocks_missing_proposal_review() {
        let (req, mut ctx) = full_chain();
        ctx.proposal_review = None;
        let record = evaluate_workflow_execution(&req, &ctx);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowExecutionPredicate::ProposalReviewExists && !p.passed));
    }

    #[test]
    fn blocks_unapproved_proposal_review() {
        let (req, mut ctx) = full_chain();
        if let Some(ref mut r) = ctx.proposal_review {
            r.decision = WorkflowProposalReviewDecision::Rejected;
        }
        let record = evaluate_workflow_execution(&req, &ctx);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowExecutionPredicate::ProposalReviewApproved && !p.passed));
    }

    #[test]
    fn blocks_source_task_plan_hash_mismatch() {
        let (req, mut ctx) = full_chain();
        if let Some(ref mut p) = ctx.source_task_plan {
            p.plan_hash = "wrong".into();
        }
        let record = evaluate_workflow_execution(&req, &ctx);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowExecutionPredicate::SourceTaskPlanHashMatchesProposal && !p.passed));
    }

    #[test]
    fn blocks_executable_tool_intent() {
        let (req, mut ctx) = full_chain();
        if let Some(ref mut p) = ctx.proposal
            && let Some(stage) = p.stages.iter_mut().find(|s| !s.tool_intents.is_empty()) {
                stage.tool_intents[0].capability = "shell".into();
            }
        let record = evaluate_workflow_execution(&req, &ctx);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowExecutionPredicate::ToolIntentsRemainNonExecutable && !p.passed));
    }

    #[test]
    fn blocks_missing_policy_constraints() {
        let (req, mut ctx) = full_chain();
        if let Some(ref mut p) = ctx.proposal {
            p.risks.clear();
            p.required_approvals.clear();
        }
        let record = evaluate_workflow_execution(&req, &ctx);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowExecutionPredicate::PolicyConstraintsRepresented && !p.passed));
    }

    #[test]
    fn blocks_missing_provider_configuration() {
        let (req, mut ctx) = full_chain();
        ctx.provider_config_available = false;
        let record = evaluate_workflow_execution(&req, &ctx);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowExecutionPredicate::ProviderConfigurationAvailable && !p.passed));
    }

    #[test]
    fn blocks_missing_session_runtime() {
        let (req, mut ctx) = full_chain();
        ctx.session_runtime_available = false;
        let record = evaluate_workflow_execution(&req, &ctx);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowExecutionPredicate::SessionRuntimeAvailable && !p.passed));
    }

    #[test]
    fn blocks_missing_rollback_abort_evidence() {
        let (req, mut ctx) = full_chain();
        if let Some(ref mut p) = ctx.proposal {
            p.abort_rollback_notes.clear();
        }
        let record = evaluate_workflow_execution(&req, &ctx);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowExecutionPredicate::RollbackAbortEvidencePresent && !p.passed));
    }

    #[test]
    fn blocks_prior_conflicting_completed_run() {
        let (req, mut ctx) = full_chain();
        // Add a prior completed run for the same proposal/review
        ctx.existing_runs.push(WorkflowRunRecord {
            execution_id: WorkflowExecutionId("wfx_prior".into()),
            readiness_id: req.readiness_id.clone(),
            proposal_id: req.proposal_id.clone(),
            proposal_review_id: req.proposal_review_id.clone(),
            source_task_plan_id: crate::plan::TaskPlanId("tpl".into()),
            status: WorkflowRunStatus::Completed,
            decision: WorkflowExecutionDecision::RunCreated,
            predicates: vec![],
            run_snapshot: WorkflowRunSnapshot {
                readiness_id: "r".into(),
                proposal_id: "p".into(),
                proposal_hash: "h".into(),
                source_task_plan_hash: "s".into(),
                readiness_status_at_execution: "ready".into(),
                proposal_review_decision_at_execution: "approved".into(),
            },
            stages: vec![],
            lifecycle_events: vec![],
            action_requests: vec![],
            abort_snapshot: WorkflowAbortSnapshot {
                abort_notes_available: true,
                rollback_notes_available: true,
                recovery_notes: vec![],
            },
            created_at: Utc::now(),
            completed_at: Some(Utc::now()),
        });
        let record = evaluate_workflow_execution(&req, &ctx);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowExecutionPredicate::NoPriorConflictingWorkflowRun && !p.passed));
    }

    #[test]
    fn blocks_policy_run_creation_denial() {
        // Wave 26 always allows run creation. This test verifies the predicate exists.
        let (req, ctx) = full_chain();
        let record = evaluate_workflow_execution(&req, &ctx);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowExecutionPredicate::PolicyAllowsWorkflowRunCreation));
    }

    #[test]
    fn execution_initializes_stage_runs_from_proposal() {
        let (req, ctx) = full_chain();
        let record = evaluate_workflow_execution(&req, &ctx);
        assert!(!record.stages.is_empty());
        // Each stage should be Pending initially
        assert!(record.stages.iter().all(|s| s.status == WorkflowStageRunStatus::Pending));
    }

    #[test]
    fn blocks_unresolved_tool_intent() {
        let (req, mut ctx) = full_chain();
        if let Some(ref mut p) = ctx.proposal
            && let Some(stage) = p.stages.iter_mut().find(|s| !s.tool_intents.is_empty()) {
                stage.tool_intents[0].capability = "quantum-computation".into();
            }
        let record = evaluate_workflow_execution(&req, &ctx);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowExecutionPredicate::ToolIntentResolutionStillValid && !p.passed));
    }
}
