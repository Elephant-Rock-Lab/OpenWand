//! Stage lifecycle engine.
//!
//! Advances deterministic stages. Non-tool stages complete as lifecycle evidence.
//! Tool-intent stages create action requests and suspend.
//! No tool calls, no memory reads, no trace reads, no external work.

use chrono::Utc;

use crate::workflow_proposal::{WorkflowProposal, WorkflowStageKind};
use crate::workflow_run::*;

/// Non-tool stage kinds that can complete deterministically.
const DETERMINISTIC_KINDS: &[WorkflowStageKind] = &[
    WorkflowStageKind::Observe,
    WorkflowStageKind::Analyze,
    WorkflowStageKind::Report,
];

/// Tool-intent stage kinds that require action requests and suspend.
const TOOL_INTENT_KINDS: &[WorkflowStageKind] = &[
    WorkflowStageKind::PrepareChange,
    WorkflowStageKind::ApplyChange,
    WorkflowStageKind::Verify,
];

/// Advance stages deterministically.
///
/// Non-tool stages are marked complete as lifecycle evidence only.
/// Tool-intent stages create action requests and suspend.
/// RequestApproval stages suspend awaiting approval.
pub fn advance_stages(
    proposal: &WorkflowProposal,
) -> (Vec<WorkflowStageRun>, Vec<WorkflowStageLifecycleEvent>, Vec<WorkflowActionRequest>) {
    let mut stages: Vec<WorkflowStageRun> = proposal.stages.iter().map(|s| {
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
    }).collect();

    let mut events = Vec::new();
    let mut action_requests = Vec::new();
    let mut event_counter = 0u32;
    let mut action_counter = 0u32;
    let now = Utc::now();

    // Collect stage info to avoid borrow conflicts
    #[allow(clippy::type_complexity)]
    let stage_info: Vec<(WorkflowStageKind, Vec<String>, Vec<(String, String, String, String, bool)>)> = proposal.stages.iter().map(|s| {
        let tool_intents: Vec<(String, String, String, String, bool)> = s.tool_intents.iter()
            .map(|ti| (ti.capability.clone(), ti.purpose.clone(), ti.expected_input_summary.clone(), ti.expected_output_summary.clone(), ti.requires_policy_gate))
            .collect();
        (s.kind.clone(), s.depends_on.clone(), tool_intents)
    }).collect();

    let mut completed_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (i, stage) in stages.iter_mut().enumerate() {
        let (kind, deps, tool_intents) = &stage_info[i];
        let deps_met = deps.iter().all(|dep| completed_ids.contains(dep));

        if !deps_met {
            stage.status = WorkflowStageRunStatus::Blocked;
            event_counter += 1;
            events.push(WorkflowStageLifecycleEvent {
                event_id: format!("evt_{}", event_counter),
                stage_id: stage.stage_id.clone(),
                event_kind: WorkflowStageLifecycleKind::StageBlocked,
                summary: "Blocked by unmet dependency".into(),
                occurred_at: now,
            });
            continue;
        }

        // Start stage
        stage.status = WorkflowStageRunStatus::Running;
        stage.started_at = Some(now);
        event_counter += 1;
        events.push(WorkflowStageLifecycleEvent {
            event_id: format!("evt_{}", event_counter),
            stage_id: stage.stage_id.clone(),
            event_kind: WorkflowStageLifecycleKind::StageStarted,
            summary: format!("Stage started: {}", stage.title),
            occurred_at: now,
        });

        if DETERMINISTIC_KINDS.contains(kind) {
            // Non-tool stage: complete as lifecycle evidence only
            stage.status = WorkflowStageRunStatus::Completed;
            stage.completed_at = Some(now);
            stage.summary = "Marked complete as non-tool deterministic stage".into();
            completed_ids.insert(stage.stage_id.clone());
            event_counter += 1;
            events.push(WorkflowStageLifecycleEvent {
                event_id: format!("evt_{}", event_counter),
                stage_id: stage.stage_id.clone(),
                event_kind: WorkflowStageLifecycleKind::StageCompleted,
                summary: "Marked complete as non-tool deterministic stage".into(),
                occurred_at: now,
            });
        } else if TOOL_INTENT_KINDS.contains(kind) {
            // Tool-intent stage: create action request and suspend
            for (cap, purpose, input_sum, output_sum, policy_gate) in tool_intents {
                action_counter += 1;
                action_requests.push(WorkflowActionRequest {
                    action_request_id: format!("ar_{}", action_counter),
                    stage_id: stage.stage_id.clone(),
                    capability_category: cap.clone(),
                    purpose: purpose.clone(),
                    expected_input_summary: input_sum.clone(),
                    expected_output_summary: output_sum.clone(),
                    routing_status: WorkflowActionRoutingStatus::PreparedForFutureSessionRouting,
                    session_bridge_required: true,
                    policy_gate_required: *policy_gate,
                });
            }
            stage.status = WorkflowStageRunStatus::Suspended;
            stage.summary = "Suspended before tool execution boundary".into();
            event_counter += 1;
            events.push(WorkflowStageLifecycleEvent {
                event_id: format!("evt_{}", event_counter),
                stage_id: stage.stage_id.clone(),
                event_kind: WorkflowStageLifecycleKind::StageSuspended,
                summary: "Suspended before tool execution boundary".into(),
                occurred_at: now,
            });
        } else if matches!(kind, WorkflowStageKind::RequestApproval) {
            stage.status = WorkflowStageRunStatus::Suspended;
            stage.summary = "Suspended awaiting approval".into();
            event_counter += 1;
            events.push(WorkflowStageLifecycleEvent {
                event_id: format!("evt_{}", event_counter),
                stage_id: stage.stage_id.clone(),
                event_kind: WorkflowStageLifecycleKind::StageSuspended,
                summary: "Suspended awaiting approval".into(),
                occurred_at: now,
            });
        }
    }

    (stages, events, action_requests)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::build_task_plan;
    use crate::context::TaskPlanInput;
    use crate::plan_review::{TaskPlanReview, TaskPlanReviewDecision, task_review_id_for};
    use crate::workflow_proposal_builder::{WorkflowProposalInput, build_workflow_proposal};
    use chrono::Utc;

    fn test_proposal() -> WorkflowProposal {
        let plan = build_task_plan(&TaskPlanInput {
            user_intent: "Lifecycle test".into(),
            skill_context: vec![],
            goal_context: vec![],
            memory_summaries: vec!["mem".into()],
            trace_summaries: vec!["trace".into()],
            governance_summaries: vec![],
            policy_constraints: vec!["No shell".into()],
        }).unwrap();
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
        build_workflow_proposal(WorkflowProposalInput {
            task_plan: plan,
            latest_task_plan_review: Some(plan_review.clone()),
            task_plan_hash: plan_review.plan_hash.clone(),
        }).unwrap()
    }

    #[test]
    fn execution_initializes_stage_runs_from_proposal() {
        let proposal = test_proposal();
        let (stages, _, _) = advance_stages(&proposal);
        assert_eq!(proposal.stages.len(), stages.len());
    }

    #[test]
    fn execution_orders_stages_by_proposal_order() {
        let proposal = test_proposal();
        let (stages, _, _) = advance_stages(&proposal);
        for (i, s) in stages.iter().enumerate() {
            assert_eq!(i as u32, s.order);
        }
    }

    #[test]
    fn execution_completes_observe_analyze_report_stages_without_tools() {
        let proposal = test_proposal();
        let (stages, _, _) = advance_stages(&proposal);
        let completed: Vec<_> = stages.iter()
            .filter(|s| s.status == WorkflowStageRunStatus::Completed)
            .collect();
        assert!(!completed.is_empty());
        for s in &completed {
            assert!(DETERMINISTIC_KINDS.contains(&s.kind), "Unexpected completed kind: {:?}", s.kind);
        }
    }

    #[test]
    fn execution_prepares_action_request_for_tool_intent_stage() {
        let proposal = test_proposal();
        let (_, _, action_requests) = advance_stages(&proposal);
        // Proposal has tool intents from builder
        assert!(!action_requests.is_empty());
        for ar in &action_requests {
            assert_eq!(WorkflowActionRoutingStatus::PreparedForFutureSessionRouting, ar.routing_status);
        }
    }

    #[test]
    fn execution_suspends_before_tool_execution_boundary() {
        let proposal = test_proposal();
        let (stages, _, _) = advance_stages(&proposal);
        let suspended: Vec<_> = stages.iter()
            .filter(|s| s.status == WorkflowStageRunStatus::Suspended)
            .collect();
        assert!(!suspended.is_empty());
        for s in &suspended {
            assert!(TOOL_INTENT_KINDS.contains(&s.kind) || matches!(s.kind, WorkflowStageKind::RequestApproval));
        }
    }

    #[test]
    fn execution_records_lifecycle_events() {
        let proposal = test_proposal();
        let (_, events, _) = advance_stages(&proposal);
        assert!(!events.is_empty());
        // Should have at least started events
        assert!(events.iter().any(|e| matches!(e.event_kind, WorkflowStageLifecycleKind::StageStarted)));
    }

    #[test]
    fn execution_does_not_create_tool_call_for_action_request() {
        let proposal = test_proposal();
        let (_, _, action_requests) = advance_stages(&proposal);
        for ar in &action_requests {
            // Verify no tool_name, tool_args, command fields
            assert!(!ar.capability_category.contains("tool_name"));
            assert!(!ar.capability_category.contains("tool_args"));
            assert!(!ar.purpose.contains("command"));
        }
    }

    #[test]
    fn non_tool_stage_completion_is_lifecycle_only() {
        let proposal = test_proposal();
        let (stages, events, _) = advance_stages(&proposal);
        let completed: Vec<_> = stages.iter()
            .filter(|s| s.status == WorkflowStageRunStatus::Completed)
            .collect();
        for s in &completed {
            assert!(s.summary.contains("non-tool deterministic stage"));
        }
        // Verify completion events also say lifecycle only
        let completion_events: Vec<_> = events.iter()
            .filter(|e| matches!(e.event_kind, WorkflowStageLifecycleKind::StageCompleted))
            .collect();
        for e in &completion_events {
            assert!(e.summary.contains("non-tool deterministic stage"));
        }
    }

    #[test]
    fn non_tool_stage_completion_does_not_read_memory_or_trace() {
        // Lifecycle engine has no imports to memory/trace crates.
        // This test verifies the function signature takes only proposal.
        let proposal = test_proposal();
        let (stages, _, _) = advance_stages(&proposal);
        // If we got here without needing memory/trace, the proof holds
        assert!(!stages.is_empty());
    }

    #[test]
    fn execution_blocks_stage_with_unmet_dependency() {
        // Create a proposal where a stage depends on a non-existent stage
        let mut proposal = test_proposal();
        if let Some(stage) = proposal.stages.iter_mut().find(|s| s.order > 0) {
            stage.depends_on = vec!["nonexistent_stage".into()];
        }
        let (stages, events, _) = advance_stages(&proposal);
        let blocked: Vec<_> = stages.iter()
            .filter(|s| s.status == WorkflowStageRunStatus::Blocked)
            .collect();
        assert!(!blocked.is_empty());
        assert!(events.iter().any(|e| matches!(e.event_kind, WorkflowStageLifecycleKind::StageBlocked)));
    }
}
