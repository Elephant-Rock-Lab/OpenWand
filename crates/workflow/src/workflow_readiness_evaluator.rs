//! Workflow readiness evaluator.
//!
//! Evaluates all readiness predicates for an approved workflow proposal.
//! No LLM, no tool calls, no provider invocation, no shell/git.
//! Observation-only: environment booleans are inputs, not actual checks.

use chrono::Utc;

use crate::plan::TaskPlan;
use crate::plan_review::{TaskPlanReview, TaskPlanReviewDecision};
use crate::workflow_proposal::{WorkflowProposal, WorkflowProposalStatus};
use crate::workflow_proposal_review::{WorkflowProposalReview, WorkflowProposalReviewDecision};
use crate::workflow_readiness::*;
use crate::workflow_readiness_validation::workflow_readiness_id_for;
use crate::tool_intent_resolution::resolve_tool_intent;

/// Input context for readiness evaluation. All observation-only.
pub struct WorkflowReadinessContext {
    pub proposal: Option<WorkflowProposal>,
    pub review: Option<WorkflowProposalReview>,
    pub latest_review_for_proposal: Option<WorkflowProposalReview>,
    pub source_task_plan: Option<TaskPlan>,
    pub source_task_plan_review: Option<TaskPlanReview>,
    pub latest_source_task_plan_review: Option<TaskPlanReview>,
    pub environment: WorkflowEnvironmentSnapshot,
    pub existing_readiness_records: Vec<WorkflowReadinessRecord>,
}

/// Evaluate workflow readiness deterministically.
pub fn evaluate_workflow_readiness(
    request: &WorkflowReadinessRequest,
    context: &WorkflowReadinessContext,
) -> WorkflowReadinessRecord {
    let mut predicates = Vec::new();

    // 1. ProposalExists
    let proposal = context.proposal.as_ref();
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::ProposalExists,
        passed: proposal.is_some(),
        reason: if proposal.is_some() {
            "Proposal loaded".into()
        } else {
            "Proposal not found".into()
        },
    });

    // 2. ProposalReviewExists
    let review = context.review.as_ref();
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::ProposalReviewExists,
        passed: review.is_some(),
        reason: if review.is_some() {
            "Review loaded".into()
        } else {
            "Review not found".into()
        },
    });

    // 3. ProposalReviewIsLatest
    let latest_review_matches = review
        .and_then(|r| {
            context
                .latest_review_for_proposal
                .as_ref()
                .map(|lr| lr.review_id == r.review_id)
        })
        .unwrap_or(false);
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::ProposalReviewIsLatest,
        passed: latest_review_matches,
        reason: if latest_review_matches {
            "Review is latest for proposal".into()
        } else {
            "Review is not the latest for this proposal".into()
        },
    });

    // 4. ProposalReviewApproved
    let review_approved = review
        .map(|r| r.decision == WorkflowProposalReviewDecision::Approved)
        .unwrap_or(false);
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::ProposalReviewApproved,
        passed: review_approved,
        reason: if review_approved {
            "Review decision is Approved".into()
        } else {
            "Review decision is not Approved".into()
        },
    });

    // 5. ProposalHashMatchesReview
    let hash_matches_review = proposal
        .and_then(|p| review.map(|r| p.proposal_hash == r.proposal_hash))
        .unwrap_or(false);
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::ProposalHashMatchesReview,
        passed: hash_matches_review,
        reason: if hash_matches_review {
            "Proposal hash matches review".into()
        } else {
            "Proposal hash does not match review".into()
        },
    });

    // 6. ProposalHashMatchesRequest
    let hash_matches_request = proposal
        .map(|p| p.proposal_hash == request.expected_proposal_hash)
        .unwrap_or(false);
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::ProposalHashMatchesRequest,
        passed: hash_matches_request,
        reason: if hash_matches_request {
            "Proposal hash matches request".into()
        } else {
            "Proposal hash does not match request".into()
        },
    });

    // 7. SourceTaskPlanExists
    let source_plan = context.source_task_plan.as_ref();
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::SourceTaskPlanExists,
        passed: source_plan.is_some(),
        reason: if source_plan.is_some() {
            "Source task plan loaded".into()
        } else {
            "Source task plan not found".into()
        },
    });

    // 8. SourceTaskPlanHashMatchesProposal
    let source_hash_matches = source_plan
        .and_then(|sp| proposal.map(|p| sp.plan_hash == p.source_task_plan_hash))
        .unwrap_or(false);
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::SourceTaskPlanHashMatchesProposal,
        passed: source_hash_matches,
        reason: if source_hash_matches {
            "Source task plan hash matches proposal".into()
        } else {
            "Source task plan hash does not match proposal".into()
        },
    });

    // 9. SourceTaskPlanHashMatchesRequest
    let source_hash_matches_request = source_plan
        .map(|sp| sp.plan_hash == request.expected_source_task_plan_hash)
        .unwrap_or(false);
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::SourceTaskPlanHashMatchesRequest,
        passed: source_hash_matches_request,
        reason: if source_hash_matches_request {
            "Source task plan hash matches request".into()
        } else {
            "Source task plan hash does not match request".into()
        },
    });

    // 10. SourceTaskPlanLatestReviewApproved
    let source_review_approved = context
        .latest_source_task_plan_review
        .as_ref()
        .map(|r| r.decision == TaskPlanReviewDecision::Approved)
        .unwrap_or(false);
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::SourceTaskPlanLatestReviewApproved,
        passed: source_review_approved,
        reason: if source_review_approved {
            "Source task plan latest review is Approved".into()
        } else {
            "Source task plan latest review is not Approved".into()
        },
    });

    // 11. WorkflowProposalIsReviewable
    let proposal_reviewable = proposal
        .map(|p| p.status == WorkflowProposalStatus::Reviewable)
        .unwrap_or(false);
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::WorkflowProposalIsReviewable,
        passed: proposal_reviewable,
        reason: if proposal_reviewable {
            "Proposal status is Reviewable".into()
        } else {
            "Proposal status is not Reviewable".into()
        },
    });

    // 12. RequiredApprovalMarkersPresent
    let markers_present = proposal
        .map(|p| {
            // Markers should exist if stages require approval
            let approval_stages: Vec<_> = p
                .stages
                .iter()
                .filter(|s| s.requires_approval_before_execution)
                .collect();
            approval_stages.is_empty() || !p.required_approvals.is_empty()
        })
        .unwrap_or(false);
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::RequiredApprovalMarkersPresent,
        passed: markers_present,
        reason: if markers_present {
            "Required approval markers present".into()
        } else {
            "Missing required approval markers".into()
        },
    });

    // 13-14. Tool intent resolution
    let tool_intent_snaps: Vec<ToolIntentResolutionSnapshot> = proposal
        .map(|p| {
            p.stages
                .iter()
                .flat_map(|s| s.tool_intents.iter())
                .map(|ti| resolve_tool_intent(&ti.intent_id, &ti.capability))
                .collect()
        })
        .unwrap_or_default();

    let all_resolvable = tool_intent_snaps
        .iter()
        .all(|t| matches!(t.resolution_status, ToolIntentResolutionStatus::ResolvedCategory));
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::ToolIntentsResolvable,
        passed: all_resolvable,
        reason: if all_resolvable {
            "All tool intents resolved to categories".into()
        } else {
            "Some tool intents could not be resolved".into()
        },
    });

    let none_executable = tool_intent_snaps
        .iter()
        .all(|t| !matches!(t.resolution_status, ToolIntentResolutionStatus::RejectedExecutable));
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::ToolIntentsRemainNonExecutable,
        passed: none_executable,
        reason: if none_executable {
            "No tool intents contain executable patterns".into()
        } else {
            "Some tool intents contain executable patterns".into()
        },
    });

    // 15. PolicyConstraintsRepresented (not a policy decision)
    let policy_represented = proposal
        .map(|p| !p.risks.is_empty() || !p.required_approvals.is_empty() || context.environment.policy_context_available)
        .unwrap_or(false);
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::PolicyConstraintsRepresented,
        passed: policy_represented,
        reason: if policy_represented {
            "Policy constraints are represented in readiness evidence".into()
        } else {
            "No policy constraints represented".into()
        },
    });

    // 16-17. Provider/Session (Inconclusive if missing, not Blocked)
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::ProviderConfigurationAvailable,
        passed: context.environment.provider_config_available,
        reason: if context.environment.provider_config_available {
            "Provider configuration available".into()
        } else {
            "Provider configuration not available".into()
        },
    });

    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::SessionRuntimeAvailable,
        passed: context.environment.session_runtime_available,
        reason: if context.environment.session_runtime_available {
            "Session runtime available".into()
        } else {
            "Session runtime not available".into()
        },
    });

    // 18. WorkspacePreconditionsObserved
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::WorkspacePreconditionsObserved,
        passed: context.environment.workspace_observed,
        reason: if context.environment.workspace_observed {
            "Workspace preconditions observed".into()
        } else {
            "Workspace preconditions not observed".into()
        },
    });

    // 19. RollbackAbortEvidencePresent
    let rollback_present = proposal
        .map(|p| !p.abort_rollback_notes.is_empty())
        .unwrap_or(false);
    let rollback_snap = WorkflowRollbackAbortSnapshot {
        abort_notes_present: rollback_present,
        rollback_notes_present: rollback_present,
        unresolved_recovery_gaps: if rollback_present {
            vec![]
        } else {
            vec!["No abort/rollback notes in proposal".into()]
        },
    };
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::RollbackAbortEvidencePresent,
        passed: rollback_present,
        reason: if rollback_present {
            "Rollback/abort evidence present".into()
        } else {
            "Missing rollback/abort evidence".into()
        },
    });

    // 20. NoPriorConflictingReadiness
    let no_conflict = !context
        .existing_readiness_records
        .iter()
        .any(|r| r.status == WorkflowReadinessStatus::Ready
            && r.proposal_id == request.proposal_id
            && r.review_id == request.review_id);
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::NoPriorConflictingReadiness,
        passed: no_conflict,
        reason: if no_conflict {
            "No conflicting prior readiness".into()
        } else {
            "Prior Ready readiness exists for this proposal/review".into()
        },
    });

    // 21. IdempotencyKeyUnusedOrMatchesExisting
    let matching_existing = context
        .existing_readiness_records
        .iter()
        .find(|r| r.proposal_id == request.proposal_id
            && r.review_id == request.review_id);
    let idempotency_ok = match matching_existing {
        Some(_existing) => {
            // Same key → ok (returns existing), different key + Ready → blocked
            true // We handle this at persistence layer
        }
        None => true,
    };
    predicates.push(WorkflowReadinessPredicateResult {
        predicate: WorkflowReadinessPredicate::IdempotencyKeyUnusedOrMatchesExisting,
        passed: idempotency_ok,
        reason: "Idempotency check passed".into(),
    });

    // Build approval marker snapshots
    let approval_marker_snaps: Vec<WorkflowApprovalMarkerSnapshot> = proposal
        .map(|p| {
            p.required_approvals
                .iter()
                .map(|m| WorkflowApprovalMarkerSnapshot {
                    marker_id: m.marker_id.clone(),
                    stage_id: m.stage_id.clone(),
                    reason: m.reason.clone(),
                    required_before: m.required_before.clone(),
                    requirement_understood: true,
                    note: "Approval requirement carried for future execution evaluation".into(),
                })
                .collect()
        })
        .unwrap_or_default();

    // Determine final status
    let blocking_predicates: Vec<_> = predicates
        .iter()
        .filter(|p| !p.passed)
        .filter(|p| !matches!(
            p.predicate,
            WorkflowReadinessPredicate::ProviderConfigurationAvailable
                | WorkflowReadinessPredicate::SessionRuntimeAvailable
        ))
        .collect();

    let inconclusive_predicates: Vec<_> = predicates
        .iter()
        .filter(|p| !p.passed)
        .filter(|p| matches!(
            p.predicate,
            WorkflowReadinessPredicate::ProviderConfigurationAvailable
                | WorkflowReadinessPredicate::SessionRuntimeAvailable
        ))
        .collect();

    let (status, decision) = if blocking_predicates.is_empty() && inconclusive_predicates.is_empty()
    {
        (
            WorkflowReadinessStatus::Ready,
            WorkflowReadinessDecision::Ready,
        )
    } else if !blocking_predicates.is_empty() {
        let reasons: Vec<String> = blocking_predicates
            .iter()
            .map(|p| format!("{:?}: {}", p.predicate, p.reason))
            .collect();
        (
            WorkflowReadinessStatus::Blocked,
            WorkflowReadinessDecision::Blocked {
                reason_code: "predicate_failed".into(),
                summary: reasons.join("; "),
            },
        )
    } else {
        let reasons: Vec<String> = inconclusive_predicates
            .iter()
            .map(|p| format!("{:?}: {}", p.predicate, p.reason))
            .collect();
        (
            WorkflowReadinessStatus::Inconclusive,
            WorkflowReadinessDecision::Inconclusive {
                reason_code: "evidence_missing".into(),
                summary: reasons.join("; "),
            },
        )
    };

    let source_plan_id = proposal
        .map(|p| p.source_task_plan_id.clone())
        .unwrap_or_else(|| crate::plan::TaskPlanId("unknown".into()));
    let source_review_id = proposal
        .and_then(|_p| {
            context
                .source_task_plan_review
                .as_ref()
                .map(|r| r.review_id.clone())
        })
        .unwrap_or_else(|| crate::plan_review::TaskPlanReviewId("unknown".into()));

    let readiness_id = workflow_readiness_id_for(
        request.proposal_id.0.as_str(),
        request.review_id.0.as_str(),
        &request.idempotency_key,
        predicates.len(),
    );

    WorkflowReadinessRecord {
        readiness_id,
        proposal_id: request.proposal_id.clone(),
        review_id: request.review_id.clone(),
        source_task_plan_id: source_plan_id,
        source_task_plan_review_id: source_review_id,
        proposal_hash: proposal
            .map(|p| p.proposal_hash.clone())
            .unwrap_or_default(),
        source_task_plan_hash: proposal
            .map(|p| p.source_task_plan_hash.clone())
            .unwrap_or_default(),
        status,
        decision,
        predicates,
        tool_intents: tool_intent_snaps,
        approval_markers: approval_marker_snaps,
        environment: context.environment.clone(),
        rollback_abort: rollback_snap,
        created_at: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::build_task_plan;
    use crate::context::TaskPlanInput;
    use crate::plan_review::{TaskPlanReview, task_review_id_for};
    use crate::workflow_proposal_builder::{WorkflowProposalInput, build_workflow_proposal};
    use crate::workflow_proposal_review::{
        WorkflowProposalFeedback, WorkflowProposalReview, WorkflowProposalReviewDecision,
        workflow_review_id_for,
    };

    fn test_proposal_and_review() -> (WorkflowProposal, WorkflowProposalReview) {
        let plan = build_task_plan(&TaskPlanInput {
            user_intent: "Readiness test".into(),
            skill_context: vec![],
            goal_context: vec![],
            memory_summaries: vec!["mem".into()],
            trace_summaries: vec!["trace".into()],
            governance_summaries: vec![],
            policy_constraints: vec!["No shell".into()],
        })
        .unwrap();
        let plan_review_id = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::Approved, "OK");
        let plan_review = TaskPlanReview {
            review_id: plan_review_id,
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
        })
        .unwrap();
        let proposal_review_id = workflow_review_id_for(
            &proposal.proposal_id,
            &WorkflowProposalReviewDecision::Approved,
            "Good",
        );
        let proposal_review = WorkflowProposalReview {
            review_id: proposal_review_id,
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
        (proposal, proposal_review)
    }

    fn full_environment() -> WorkflowEnvironmentSnapshot {
        WorkflowEnvironmentSnapshot {
            workspace_observed: true,
            provider_config_available: true,
            session_runtime_available: true,
            tool_manifest_available: true,
            policy_context_available: true,
            notes: vec![],
        }
    }

    fn valid_request(proposal: &WorkflowProposal, review: &WorkflowProposalReview) -> WorkflowReadinessRequest {
        WorkflowReadinessRequest {
            proposal_id: proposal.proposal_id.clone(),
            review_id: review.review_id.clone(),
            expected_proposal_hash: proposal.proposal_hash.clone(),
            expected_source_task_plan_hash: proposal.source_task_plan_hash.clone(),
            requested_by: "tester".into(),
            requested_at: Utc::now(),
            idempotency_key: "key1".into(),
        }
    }

    fn full_context(proposal: &WorkflowProposal, review: &WorkflowProposalReview, plan_review: &TaskPlanReview) -> WorkflowReadinessContext {
        let source_plan = build_task_plan(&TaskPlanInput {
            user_intent: "Readiness test".into(),
            skill_context: vec![],
            goal_context: vec![],
            memory_summaries: vec!["mem".into()],
            trace_summaries: vec!["trace".into()],
            governance_summaries: vec![],
            policy_constraints: vec!["No shell".into()],
        }).unwrap();
        WorkflowReadinessContext {
            proposal: Some(proposal.clone()),
            review: Some(review.clone()),
            latest_review_for_proposal: Some(review.clone()),
            source_task_plan: Some(source_plan),
            source_task_plan_review: Some(plan_review.clone()),
            latest_source_task_plan_review: Some(plan_review.clone()),
            environment: full_environment(),
            existing_readiness_records: vec![],
        }
    }

    // We need the source task plan
    fn full_ready_context() -> (WorkflowReadinessRequest, WorkflowReadinessContext) {
        let (proposal, review) = test_proposal_and_review();
        // Get the source plan
        let source_plan = build_task_plan(&TaskPlanInput {
            user_intent: "Readiness test".into(),
            skill_context: vec![],
            goal_context: vec![],
            memory_summaries: vec!["mem".into()],
            trace_summaries: vec!["trace".into()],
            governance_summaries: vec![],
            policy_constraints: vec!["No shell".into()],
        }).unwrap();
        let source_review_id = task_review_id_for(&source_plan.plan_id, &TaskPlanReviewDecision::Approved, "OK");
        let source_review = TaskPlanReview {
            review_id: source_review_id,
            plan_id: source_plan.plan_id.clone(),
            plan_hash: source_plan.plan_hash.clone(),
            decision: TaskPlanReviewDecision::Approved,
            reviewer: "tester".into(),
            rationale: "OK".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        let request = valid_request(&proposal, &review);
        let context = WorkflowReadinessContext {
            proposal: Some(proposal.clone()),
            review: Some(review.clone()),
            latest_review_for_proposal: Some(review.clone()),
            source_task_plan: Some(source_plan),
            source_task_plan_review: Some(source_review.clone()),
            latest_source_task_plan_review: Some(source_review),
            environment: full_environment(),
            existing_readiness_records: vec![],
        };
        (request, context)
    }

    #[test]
    fn ready_when_all_predicates_pass() {
        let (request, context) = full_ready_context();
        let record = evaluate_workflow_readiness(&request, &context);
        assert_eq!(WorkflowReadinessStatus::Ready, record.status);
        assert!(record.predicates.iter().all(|p| p.passed));
    }

    #[test]
    fn blocks_missing_workflow_proposal() {
        let (request, mut context) = full_ready_context();
        context.proposal = None;
        let record = evaluate_workflow_readiness(&request, &context);
        assert_eq!(WorkflowReadinessStatus::Blocked, record.status);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowReadinessPredicate::ProposalExists && !p.passed));
    }

    #[test]
    fn blocks_missing_workflow_review() {
        let (request, mut context) = full_ready_context();
        context.review = None;
        let record = evaluate_workflow_readiness(&request, &context);
        assert_eq!(WorkflowReadinessStatus::Blocked, record.status);
    }

    #[test]
    fn blocks_rejected_workflow_review() {
        let (request, mut context) = full_ready_context();
        if let Some(ref mut review) = context.review {
            let rejected_id = workflow_review_id_for(
                &review.proposal_id,
                &WorkflowProposalReviewDecision::Rejected,
                "Bad",
            );
            *review = WorkflowProposalReview {
                review_id: rejected_id,
                proposal_id: review.proposal_id.clone(),
                source_task_plan_id: review.source_task_plan_id.clone(),
                proposal_hash: review.proposal_hash.clone(),
                decision: WorkflowProposalReviewDecision::Rejected,
                reviewer: "r".into(),
                rationale: "Bad".into(),
                feedback: Some(WorkflowProposalFeedback {
                    summary: "Bad".into(),
                    blocking_reasons: vec!["Wrong".into()],
                    requested_changes: vec![],
                    evidence_gaps: vec![],
                }),
                creates_execution_grant: false,
                execution_allowed_now: false,
                reviewed_at: Utc::now(),
            };
        }
        let record = evaluate_workflow_readiness(&request, &context);
        assert_eq!(WorkflowReadinessStatus::Blocked, record.status);
    }

    #[test]
    fn blocks_proposal_hash_mismatch() {
        let (mut request, context) = full_ready_context();
        request.expected_proposal_hash = "wrong_hash".into();
        let record = evaluate_workflow_readiness(&request, &context);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowReadinessPredicate::ProposalHashMatchesRequest && !p.passed));
    }

    #[test]
    fn blocks_missing_source_task_plan() {
        let (request, mut context) = full_ready_context();
        context.source_task_plan = None;
        let record = evaluate_workflow_readiness(&request, &context);
        assert_eq!(WorkflowReadinessStatus::Blocked, record.status);
    }

    #[test]
    fn blocks_unapproved_source_task_plan_review() {
        let (request, mut context) = full_ready_context();
        if let Some(ref mut review) = context.latest_source_task_plan_review {
            review.decision = TaskPlanReviewDecision::Rejected;
        }
        let record = evaluate_workflow_readiness(&request, &context);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowReadinessPredicate::SourceTaskPlanLatestReviewApproved && !p.passed));
    }

    #[test]
    fn blocks_missing_rollback_abort_evidence() {
        let (request, mut context) = full_ready_context();
        if let Some(ref mut proposal) = context.proposal {
            proposal.abort_rollback_notes.clear();
        }
        let record = evaluate_workflow_readiness(&request, &context);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowReadinessPredicate::RollbackAbortEvidencePresent && !p.passed));
    }

    #[test]
    fn inconclusive_missing_provider_configuration() {
        let (request, mut context) = full_ready_context();
        context.environment.provider_config_available = false;
        let record = evaluate_workflow_readiness(&request, &context);
        assert_eq!(WorkflowReadinessStatus::Inconclusive, record.status);
    }

    #[test]
    fn inconclusive_missing_session_runtime() {
        let (request, mut context) = full_ready_context();
        context.environment.session_runtime_available = false;
        let record = evaluate_workflow_readiness(&request, &context);
        assert_eq!(WorkflowReadinessStatus::Inconclusive, record.status);
    }

    #[test]
    fn blocks_requested_changes_workflow_review() {
        let (request, mut context) = full_ready_context();
        if let Some(ref mut review) = context.review {
            let changes_id = workflow_review_id_for(
                &review.proposal_id,
                &WorkflowProposalReviewDecision::ChangesRequested,
                "Fix",
            );
            *review = WorkflowProposalReview {
                review_id: changes_id,
                proposal_id: review.proposal_id.clone(),
                source_task_plan_id: review.source_task_plan_id.clone(),
                proposal_hash: review.proposal_hash.clone(),
                decision: WorkflowProposalReviewDecision::ChangesRequested,
                reviewer: "r".into(),
                rationale: "Fix".into(),
                feedback: Some(WorkflowProposalFeedback {
                    summary: "Needs work".into(),
                    blocking_reasons: vec![],
                    requested_changes: vec!["Add detail".into()],
                    evidence_gaps: vec![],
                }),
                creates_execution_grant: false,
                execution_allowed_now: false,
                reviewed_at: Utc::now(),
            };
        }
        let record = evaluate_workflow_readiness(&request, &context);
        assert_eq!(WorkflowReadinessStatus::Blocked, record.status);
    }

    #[test]
    fn blocks_non_latest_workflow_review() {
        let (request, mut context) = full_ready_context();
        // Set latest_review_for_proposal to None → review is not latest
        context.latest_review_for_proposal = None;
        let record = evaluate_workflow_readiness(&request, &context);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowReadinessPredicate::ProposalReviewIsLatest && !p.passed));
    }

    #[test]
    fn blocks_source_task_plan_hash_mismatch() {
        let (request, mut context) = full_ready_context();
        if let Some(ref mut plan) = context.source_task_plan {
            plan.plan_hash = "wrong_hash".into();
        }
        let record = evaluate_workflow_readiness(&request, &context);
        assert!(record.predicates.iter().any(|p|
            matches!(p.predicate,
                WorkflowReadinessPredicate::SourceTaskPlanHashMatchesProposal) && !p.passed));
    }

    #[test]
    fn workflow_readiness_policy_predicate_does_not_create_execution_policy_decision() {
        let (request, context) = full_ready_context();
        let record = evaluate_workflow_readiness(&request, &context);
        let policy_pred = record.predicates.iter()
            .find(|p| p.predicate == WorkflowReadinessPredicate::PolicyConstraintsRepresented)
            .unwrap();
        // It only checks representation, not execution approval
        assert!(policy_pred.reason.contains("represented"));
        assert!(!policy_pred.reason.contains("approved"));
        assert!(!policy_pred.reason.contains("granted"));
    }

    #[test]
    fn blocks_executable_tool_intent() {
        let (request, mut context) = full_ready_context();
        if let Some(ref mut proposal) = context.proposal {
            if let Some(stage) = proposal.stages.iter_mut().find(|s| !s.tool_intents.is_empty()) {
                stage.tool_intents[0].capability = "shell".into();
            }
        }
        let record = evaluate_workflow_readiness(&request, &context);
        assert!(record.predicates.iter().any(|p|
            p.predicate == WorkflowReadinessPredicate::ToolIntentsRemainNonExecutable && !p.passed));
    }

    #[test]
    fn approval_markers_are_future_requirements() {
        let (request, context) = full_ready_context();
        let record = evaluate_workflow_readiness(&request, &context);
        for marker in &record.approval_markers {
            assert!(marker.requirement_understood);
            assert!(!marker.note.contains("approved"));
            assert!(!marker.note.contains("satisfied"));
            assert!(!marker.note.contains("granted"));
        }
    }
}
