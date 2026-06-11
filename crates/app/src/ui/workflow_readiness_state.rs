//! UI workflow readiness state — read-only display helpers.
//!
//! Workflow readiness is evidence only. It does not start workflow runs,
//! execute tools, create approval requests, mutate memory, append trace
//! authority, or grant execution.

use openwand_workflow::workflow_readiness::{
    WorkflowReadinessRecord,
    ToolIntentResolutionStatus,
};

/// UI row for a readiness summary.
#[derive(Debug, Clone)]
pub struct WorkflowReadinessSummaryRow {
    pub readiness_id: String,
    pub status: String,
    pub proposal_id: String,
    pub predicates_passed: usize,
    pub predicates_total: usize,
}

/// UI row for a predicate result.
#[derive(Debug, Clone)]
pub struct WorkflowReadinessPredicateRow {
    pub predicate: String,
    pub passed: bool,
    pub reason: String,
}

/// UI row for a tool intent resolution.
#[derive(Debug, Clone)]
pub struct ToolIntentResolutionRow {
    pub intent_id: String,
    pub capability: String,
    pub status: String,
    pub reason: String,
}

/// UI row for an approval marker.
#[derive(Debug, Clone)]
pub struct WorkflowApprovalMarkerRow {
    pub marker_id: String,
    pub stage_id: String,
    pub requirement_understood: bool,
    pub note: String,
}

/// UI row for environment snapshot.
#[derive(Debug, Clone)]
pub struct WorkflowEnvironmentRow {
    pub workspace_observed: bool,
    pub provider_config_available: bool,
    pub session_runtime_available: bool,
    pub notes: Vec<String>,
}

/// UI row for rollback/abort snapshot.
#[derive(Debug, Clone)]
pub struct WorkflowRollbackAbortRow {
    pub abort_notes_present: bool,
    pub rollback_notes_present: bool,
    pub gaps: Vec<String>,
}

/// Combined UI state.
#[derive(Debug, Clone)]
pub struct WorkflowReadinessUiState {
    pub latest_readiness: Option<WorkflowReadinessSummaryRow>,
    pub predicates: Vec<WorkflowReadinessPredicateRow>,
    pub tool_intents: Vec<ToolIntentResolutionRow>,
    pub approval_markers: Vec<WorkflowApprovalMarkerRow>,
    pub environment: Option<WorkflowEnvironmentRow>,
    pub rollback_abort: Option<WorkflowRollbackAbortRow>,
    pub warnings: Vec<String>,
}

/// Safety warning.
pub fn workflow_readiness_safety_warning() -> String {
    "Workflow readiness is evidence only. It does not start workflow runs, \
     execute tools, create approval requests, mutate memory, append trace \
     authority, or grant execution."
        .into()
}

/// Build summary row.
pub fn workflow_readiness_summary_lines(record: &WorkflowReadinessRecord) -> WorkflowReadinessSummaryRow {
    let passed = record.predicates.iter().filter(|p| p.passed).count();
    WorkflowReadinessSummaryRow {
        readiness_id: record.readiness_id.0.clone(),
        status: format!("{:?}", record.status).to_lowercase(),
        proposal_id: record.proposal_id.0.clone(),
        predicates_passed: passed,
        predicates_total: record.predicates.len(),
    }
}

/// Build predicate rows.
pub fn workflow_readiness_predicate_rows(record: &WorkflowReadinessRecord) -> Vec<WorkflowReadinessPredicateRow> {
    record.predicates.iter().map(|p| {
        let pred_str = format!("{:?}", p.predicate);
        WorkflowReadinessPredicateRow {
            predicate: pred_str,
            passed: p.passed,
            reason: p.reason.clone(),
        }
    }).collect()
}

/// Build tool intent rows.
pub fn tool_intent_resolution_rows(record: &WorkflowReadinessRecord) -> Vec<ToolIntentResolutionRow> {
    record.tool_intents.iter().map(|t| {
        let status_str = match t.resolution_status {
            ToolIntentResolutionStatus::ResolvedCategory => "resolved",
            ToolIntentResolutionStatus::Unresolved => "unresolved",
            ToolIntentResolutionStatus::Ambiguous => "ambiguous",
            ToolIntentResolutionStatus::RejectedExecutable => "rejected",
        };
        ToolIntentResolutionRow {
            intent_id: t.intent_id.clone(),
            capability: t.capability.clone(),
            status: status_str.into(),
            reason: t.reason.clone(),
        }
    }).collect()
}

/// Build approval marker rows.
pub fn workflow_approval_marker_rows(record: &WorkflowReadinessRecord) -> Vec<WorkflowApprovalMarkerRow> {
    record.approval_markers.iter().map(|m| {
        WorkflowApprovalMarkerRow {
            marker_id: m.marker_id.clone(),
            stage_id: m.stage_id.clone(),
            requirement_understood: m.requirement_understood,
            note: m.note.clone(),
        }
    }).collect()
}

/// Build environment row.
pub fn workflow_environment_lines(record: &WorkflowReadinessRecord) -> WorkflowEnvironmentRow {
    WorkflowEnvironmentRow {
        workspace_observed: record.environment.workspace_observed,
        provider_config_available: record.environment.provider_config_available,
        session_runtime_available: record.environment.session_runtime_available,
        notes: record.environment.notes.clone(),
    }
}

/// Build rollback/abort row.
pub fn workflow_rollback_abort_lines(record: &WorkflowReadinessRecord) -> WorkflowRollbackAbortRow {
    WorkflowRollbackAbortRow {
        abort_notes_present: record.rollback_abort.abort_notes_present,
        rollback_notes_present: record.rollback_abort.rollback_notes_present,
        gaps: record.rollback_abort.unresolved_recovery_gaps.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::builder::build_task_plan;
    use openwand_workflow::context::TaskPlanInput;
    use openwand_workflow::plan_review::{TaskPlanReview, TaskPlanReviewDecision, task_review_id_for};
    use openwand_workflow::workflow_proposal_builder::{WorkflowProposalInput, build_workflow_proposal};
    use openwand_workflow::workflow_proposal_review::{WorkflowProposalReview, WorkflowProposalReviewDecision, workflow_review_id_for};
    use openwand_workflow::workflow_readiness::WorkflowReadinessRequest;
    use openwand_workflow::workflow_readiness_evaluator::{WorkflowReadinessContext, evaluate_workflow_readiness};
    use openwand_workflow::workflow_readiness::WorkflowEnvironmentSnapshot;
    use chrono::Utc;

    fn full_ready_record() -> WorkflowReadinessRecord {
        let plan = build_task_plan(&TaskPlanInput {
            user_intent: "UI readiness test".into(),
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
        let request = WorkflowReadinessRequest {
            proposal_id: proposal.proposal_id.clone(),
            review_id: proposal_review.review_id.clone(),
            expected_proposal_hash: proposal.proposal_hash.clone(),
            expected_source_task_plan_hash: proposal.source_task_plan_hash.clone(),
            requested_by: "tester".into(),
            requested_at: Utc::now(),
            idempotency_key: "key1".into(),
        };
        let source_review_id = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::Approved, "OK");
        let source_review = TaskPlanReview {
            review_id: source_review_id,
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
        let context = WorkflowReadinessContext {
            proposal: Some(proposal),
            review: Some(proposal_review),
            latest_review_for_proposal: None,
            source_task_plan: Some(plan),
            source_task_plan_review: Some(source_review.clone()),
            latest_source_task_plan_review: Some(source_review),
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
        evaluate_workflow_readiness(&request, &context)
    }

    #[test]
    fn workflow_readiness_ui_state_loads_latest_readiness() {
        let record = full_ready_record();
        let state = WorkflowReadinessUiState {
            latest_readiness: Some(workflow_readiness_summary_lines(&record)),
            predicates: workflow_readiness_predicate_rows(&record),
            tool_intents: tool_intent_resolution_rows(&record),
            approval_markers: workflow_approval_marker_rows(&record),
            environment: Some(workflow_environment_lines(&record)),
            rollback_abort: Some(workflow_rollback_abort_lines(&record)),
            warnings: vec![],
        };
        assert!(state.latest_readiness.is_some());
        assert!(!state.predicates.is_empty());
    }

    #[test]
    fn workflow_readiness_predicate_rows_show_pass_fail_reason() {
        let rows = workflow_readiness_predicate_rows(&full_ready_record());
        assert!(!rows.is_empty());
        assert!(rows.iter().all(|r| !r.predicate.is_empty()));
        assert!(rows.iter().all(|r| !r.reason.is_empty()));
    }

    #[test]
    fn tool_intent_resolution_rows_show_status_reason() {
        let rows = tool_intent_resolution_rows(&full_ready_record());
        assert!(!rows.is_empty());
        for row in &rows {
            assert!(!row.status.is_empty());
            assert!(!row.reason.is_empty());
        }
    }

    #[test]
    fn workflow_approval_marker_rows_show_future_requirement() {
        let rows = workflow_approval_marker_rows(&full_ready_record());
        for row in &rows {
            assert!(row.requirement_understood);
            assert!(!row.note.contains("approved"));
            assert!(!row.note.contains("satisfied"));
        }
    }

    #[test]
    fn workflow_environment_lines_show_availability() {
        let env = workflow_environment_lines(&full_ready_record());
        assert!(env.workspace_observed);
        assert!(env.provider_config_available);
        assert!(env.session_runtime_available);
    }

    #[test]
    fn workflow_readiness_safety_warning_mentions_no_execution() {
        let warning = workflow_readiness_safety_warning();
        assert!(warning.contains("does not"));
        assert!(warning.contains("evidence"));
        assert!(warning.contains("execute"));
    }
}
