//! UI workflow proposal state — read-only display helpers.
//!
//! Workflow proposals are reviewable evidence only. They do not execute tools,
//! start workflow runs, schedule work, mutate memory, append trace authority,
//! or create execution grants.

use openwand_workflow::workflow_proposal::{
    WorkflowProposal, WorkflowProposalEvidenceKind, WorkflowStageKind,
};
use openwand_workflow::workflow_proposal_review::{
    WorkflowProposalReview, WorkflowProposalReviewDecision,
};

/// UI row for a proposal summary.
#[derive(Debug, Clone)]
pub struct WorkflowProposalSummaryRow {
    pub proposal_id: String,
    pub title: String,
    pub status: String,
    pub stage_count: usize,
    pub risk_count: usize,
    pub source_task_plan_id: String,
}

/// UI row for a proposal stage.
#[derive(Debug, Clone)]
pub struct WorkflowStageRow {
    pub stage_id: String,
    pub title: String,
    pub kind: String,
    pub order: u32,
    pub risk_level: String,
    pub requires_approval: bool,
    pub tool_intent_count: usize,
}

/// UI row for a tool intent.
#[derive(Debug, Clone)]
pub struct WorkflowToolIntentRow {
    pub intent_id: String,
    pub capability: String,
    pub purpose: String,
    pub requires_policy_gate: bool,
}

/// UI row for an approval marker.
#[derive(Debug, Clone)]
pub struct WorkflowApprovalMarkerRow {
    pub marker_id: String,
    pub stage_id: String,
    pub reason: String,
    pub required_before: String,
}

/// UI row for an abort/rollback note.
#[derive(Debug, Clone)]
pub struct WorkflowAbortRollbackRow {
    pub stage_id: Option<String>,
    pub summary: String,
    pub recovery_hint: String,
}

/// UI row for a risk.
#[derive(Debug, Clone)]
pub struct WorkflowProposalRiskRow {
    pub risk_level: String,
    pub summary: String,
    pub mitigation: String,
}

/// UI row for an evidence link.
#[derive(Debug, Clone)]
pub struct WorkflowProposalEvidenceRow {
    pub kind: String,
    pub id: String,
    pub summary: String,
}

/// UI row for a proposal review.
#[derive(Debug, Clone)]
pub struct WorkflowProposalReviewRow {
    pub review_id: String,
    pub decision: String,
    pub reviewer: String,
    pub creates_execution_grant: bool,
    pub execution_allowed_now: bool,
}

/// Combined UI state for workflow proposals.
#[derive(Debug, Clone)]
pub struct WorkflowProposalUiState {
    pub latest_proposal: Option<WorkflowProposalSummaryRow>,
    pub latest_review: Option<WorkflowProposalReviewRow>,
    pub stages: Vec<WorkflowStageRow>,
    pub tool_intents: Vec<WorkflowToolIntentRow>,
    pub risks: Vec<WorkflowProposalRiskRow>,
    pub approvals: Vec<WorkflowApprovalMarkerRow>,
    pub abort_rollback_notes: Vec<WorkflowAbortRollbackRow>,
    pub evidence_links: Vec<WorkflowProposalEvidenceRow>,
    pub warnings: Vec<String>,
}

/// Safety warning for workflow proposal display.
pub fn workflow_proposal_safety_warning() -> String {
    "Workflow proposals are reviewable evidence only. They do not execute tools, \
     start workflow runs, schedule work, mutate memory, append trace authority, \
     or create execution grants."
        .into()
}

/// Stage kind as lowercase snake_case string.
fn stage_kind_str(kind: &WorkflowStageKind) -> &'static str {
    match kind {
        WorkflowStageKind::Observe => "observe",
        WorkflowStageKind::Analyze => "analyze",
        WorkflowStageKind::PrepareChange => "prepare_change",
        WorkflowStageKind::RequestApproval => "request_approval",
        WorkflowStageKind::ApplyChange => "apply_change",
        WorkflowStageKind::Verify => "verify",
        WorkflowStageKind::Report => "report",
    }
}

/// Evidence kind as lowercase snake_case string.
fn evidence_kind_str(kind: &WorkflowProposalEvidenceKind) -> &'static str {
    match kind {
        WorkflowProposalEvidenceKind::TaskPlan => "task_plan",
        WorkflowProposalEvidenceKind::TaskPlanReview => "task_plan_review",
        WorkflowProposalEvidenceKind::TaskPlanStep => "task_plan_step",
        WorkflowProposalEvidenceKind::Goal => "goal",
        WorkflowProposalEvidenceKind::Skill => "skill",
        WorkflowProposalEvidenceKind::TraceEvent => "trace_event",
        WorkflowProposalEvidenceKind::MemoryClaim => "memory_claim",
        WorkflowProposalEvidenceKind::GovernanceRecord => "governance_record",
        WorkflowProposalEvidenceKind::UserIntent => "user_intent",
    }
}

/// Build summary row from a proposal.
pub fn workflow_proposal_summary_lines(proposal: &WorkflowProposal) -> WorkflowProposalSummaryRow {
    WorkflowProposalSummaryRow {
        proposal_id: proposal.proposal_id.0.clone(),
        title: proposal.title.clone(),
        status: format!("{:?}", proposal.status).to_lowercase(),
        stage_count: proposal.stages.len(),
        risk_count: proposal.risks.len(),
        source_task_plan_id: proposal.source_task_plan_id.0.clone(),
    }
}

/// Build stage rows from a proposal.
pub fn workflow_stage_rows(proposal: &WorkflowProposal) -> Vec<WorkflowStageRow> {
    proposal.stages.iter().map(|s| WorkflowStageRow {
        stage_id: s.stage_id.clone(),
        title: s.title.clone(),
        kind: stage_kind_str(&s.kind).into(),
        order: s.order,
        risk_level: s.risk_level.clone(),
        requires_approval: s.requires_approval_before_execution,
        tool_intent_count: s.tool_intents.len(),
    }).collect()
}

/// Build tool intent rows from a proposal.
pub fn workflow_tool_intent_rows(proposal: &WorkflowProposal) -> Vec<WorkflowToolIntentRow> {
    proposal.stages.iter().flat_map(|s| {
        s.tool_intents.iter().map(|ti| WorkflowToolIntentRow {
            intent_id: ti.intent_id.clone(),
            capability: ti.capability.clone(),
            purpose: ti.purpose.clone(),
            requires_policy_gate: ti.requires_policy_gate,
        })
    }).collect()
}

/// Build approval marker rows from a proposal.
pub fn workflow_approval_marker_rows(proposal: &WorkflowProposal) -> Vec<WorkflowApprovalMarkerRow> {
    proposal.required_approvals.iter().map(|m| WorkflowApprovalMarkerRow {
        marker_id: m.marker_id.clone(),
        stage_id: m.stage_id.clone(),
        reason: m.reason.clone(),
        required_before: m.required_before.clone(),
    }).collect()
}

/// Build abort/rollback rows from a proposal.
pub fn workflow_abort_rollback_rows(proposal: &WorkflowProposal) -> Vec<WorkflowAbortRollbackRow> {
    proposal.abort_rollback_notes.iter().map(|n| WorkflowAbortRollbackRow {
        stage_id: n.stage_id.clone(),
        summary: n.summary.clone(),
        recovery_hint: n.recovery_hint.clone(),
    }).collect()
}

/// Build risk rows from a proposal.
pub fn workflow_risk_rows(proposal: &WorkflowProposal) -> Vec<WorkflowProposalRiskRow> {
    proposal.risks.iter().map(|r| WorkflowProposalRiskRow {
        risk_level: r.risk_level.clone(),
        summary: r.summary.clone(),
        mitigation: r.mitigation.clone(),
    }).collect()
}

/// Build evidence rows from a proposal.
pub fn workflow_proposal_evidence_rows(proposal: &WorkflowProposal) -> Vec<WorkflowProposalEvidenceRow> {
    proposal.evidence_links.iter().map(|e| WorkflowProposalEvidenceRow {
        kind: evidence_kind_str(&e.kind).into(),
        id: e.id.clone(),
        summary: e.summary.clone(),
    }).collect()
}

/// Build review display lines.
pub fn workflow_proposal_review_lines(review: &WorkflowProposalReview) -> WorkflowProposalReviewRow {
    let decision_str = match review.decision {
        WorkflowProposalReviewDecision::Approved => "approved",
        WorkflowProposalReviewDecision::Rejected => "rejected",
        WorkflowProposalReviewDecision::ChangesRequested => "changes_requested",
    };
    WorkflowProposalReviewRow {
        review_id: review.review_id.0.clone(),
        decision: decision_str.into(),
        reviewer: review.reviewer.clone(),
        creates_execution_grant: review.creates_execution_grant,
        execution_allowed_now: review.execution_allowed_now,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::builder::build_task_plan;
    use openwand_workflow::context::TaskPlanInput;
    use openwand_workflow::plan_review::{TaskPlanReview, TaskPlanReviewDecision, task_review_id_for};
    use openwand_workflow::workflow_proposal_builder::{WorkflowProposalInput, build_workflow_proposal};
    use openwand_workflow::workflow_proposal_review::{WorkflowProposalFeedback, workflow_review_id_for};
    use chrono::Utc;

    fn test_proposal() -> WorkflowProposal {
        let plan = build_task_plan(&TaskPlanInput {
            user_intent: "UI test proposal".into(),
            skill_context: vec!["skill-1: Test skill".into()],
            goal_context: vec!["goal-1: Test goal".into()],
            memory_summaries: vec!["memory".into()],
            trace_summaries: vec!["trace".into()],
            governance_summaries: vec![],
            policy_constraints: vec!["No shell".into()],
        }).unwrap();
        let review_id = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::Approved, "OK");
        let review = TaskPlanReview {
            review_id,
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
            latest_task_plan_review: Some(review.clone()),
            task_plan_hash: review.plan_hash.clone(),
        }).unwrap()
    }

    #[test]
    fn workflow_proposal_ui_state_loads_latest_proposal() {
        let proposal = test_proposal();
        let state = WorkflowProposalUiState {
            latest_proposal: Some(workflow_proposal_summary_lines(&proposal)),
            latest_review: None,
            stages: workflow_stage_rows(&proposal),
            tool_intents: workflow_tool_intent_rows(&proposal),
            risks: workflow_risk_rows(&proposal),
            approvals: workflow_approval_marker_rows(&proposal),
            abort_rollback_notes: workflow_abort_rollback_rows(&proposal),
            evidence_links: workflow_proposal_evidence_rows(&proposal),
            warnings: vec![],
        };
        assert!(state.latest_proposal.is_some());
        assert!(!state.stages.is_empty());
    }

    #[test]
    fn workflow_stage_rows_show_kind_risk_approval() {
        let rows = workflow_stage_rows(&test_proposal());
        assert!(rows.iter().any(|r| r.kind == "observe"));
        assert!(rows.iter().any(|r| r.kind == "verify"));
        // With policy constraints, some stage should require approval
        assert!(rows.iter().any(|r| r.requires_approval));
    }

    #[test]
    fn workflow_tool_intent_rows_show_descriptive_intent_only() {
        let rows = workflow_tool_intent_rows(&test_proposal());
        assert!(!rows.is_empty());
        // All capabilities should be valid descriptive categories
        for row in &rows {
            assert!(openwand_workflow::workflow_proposal::is_valid_capability_category(&row.capability).is_ok());
        }
    }

    #[test]
    fn workflow_approval_marker_rows_show_required_before() {
        let proposal = test_proposal();
        let rows = workflow_approval_marker_rows(&proposal);
        if !rows.is_empty() {
            for row in &rows {
                assert!(!row.required_before.is_empty());
                assert!(!row.stage_id.is_empty());
            }
        }
    }

    #[test]
    fn workflow_abort_rollback_rows_show_recovery_hint() {
        let rows = workflow_abort_rollback_rows(&test_proposal());
        assert!(!rows.is_empty());
        for row in &rows {
            assert!(!row.recovery_hint.is_empty());
        }
    }

    #[test]
    fn workflow_proposal_evidence_rows_show_kind_summary() {
        let rows = workflow_proposal_evidence_rows(&test_proposal());
        assert!(rows.iter().any(|r| r.kind == "task_plan"));
        assert!(rows.iter().any(|r| r.kind == "task_plan_review"));
        assert!(rows.iter().any(|r| r.kind == "goal"));
        assert!(rows.iter().any(|r| r.kind == "skill"));
    }

    #[test]
    fn workflow_proposal_review_lines_show_decision() {
        let proposal = test_proposal();
        let review_id = workflow_review_id_for(
            &proposal.proposal_id,
            &WorkflowProposalReviewDecision::Approved,
            "Good",
        );
        let review = WorkflowProposalReview {
            review_id,
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
        let row = workflow_proposal_review_lines(&review);
        assert_eq!("approved", row.decision);
        assert!(!row.creates_execution_grant);
        assert!(!row.execution_allowed_now);
    }

    #[test]
    fn workflow_proposal_safety_warning_mentions_no_execution() {
        let warning = workflow_proposal_safety_warning();
        assert!(warning.contains("do not execute"));
        assert!(warning.contains("reviewable evidence"));
        assert!(warning.contains("workflow runs"));
    }

    #[test]
    fn workflow_proposal_ui_does_not_expose_execute_run_schedule() {
        let _proposal = test_proposal();
        // Compile-time check: UI types don't have execute/run/schedule fields
        let _state = WorkflowProposalUiState {
            latest_proposal: None,
            latest_review: None,
            stages: vec![],
            tool_intents: vec![],
            risks: vec![],
            approvals: vec![],
            abort_rollback_notes: vec![],
            evidence_links: vec![],
            warnings: vec![],
        };
    }
}
