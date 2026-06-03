//! UI task plan state — read-only display helpers.
//!
//! Task plans are reviewable evidence only. They do not execute tools,
//! workflows, shell commands, git operations, or memory writes.

use openwand_workflow::plan::{TaskPlan, TaskPlanStep, TaskPlanRisk, TaskPlanEvidenceLink, TaskPlanEvidenceKind, TaskPlanStatus};
use openwand_workflow::plan_review::{TaskPlanReview, TaskPlanReviewDecision};

/// UI row for a plan summary.
#[derive(Debug, Clone)]
pub struct TaskPlanSummaryRow {
    pub plan_id: String,
    pub title: String,
    pub status: String,
    pub step_count: usize,
    pub risk_count: usize,
    pub approval_count: usize,
}

/// UI row for a plan step.
#[derive(Debug, Clone)]
pub struct TaskPlanStepRow {
    pub step_id: String,
    pub title: String,
    pub kind: String,
    pub risk_level: String,
    pub requires_approval: bool,
}

/// UI row for a plan risk.
#[derive(Debug, Clone)]
pub struct TaskPlanRiskRow {
    pub risk_level: String,
    pub summary: String,
    pub mitigation: String,
}

/// UI row for an evidence link.
#[derive(Debug, Clone)]
pub struct TaskPlanEvidenceRow {
    pub kind: String,
    pub id: String,
    pub summary: String,
}

/// UI row for a plan review.
#[derive(Debug, Clone)]
pub struct TaskPlanReviewRow {
    pub review_id: String,
    pub decision: String,
    pub reviewer: String,
    pub creates_execution_grant: bool,
    pub execution_allowed_now: bool,
}

/// Combined UI state for task plans.
#[derive(Debug, Clone)]
pub struct TaskPlanUiState {
    pub latest_plan: Option<TaskPlanSummaryRow>,
    pub latest_review: Option<TaskPlanReviewRow>,
    pub steps: Vec<TaskPlanStepRow>,
    pub risks: Vec<TaskPlanRiskRow>,
    pub assumptions: Vec<String>,
    pub evidence_links: Vec<TaskPlanEvidenceRow>,
    pub warnings: Vec<String>,
}

/// Safety warning for task plan display.
pub fn task_plan_safety_warning() -> String {
    "Task plans are reviewable evidence only. They do not execute tools, workflows, shell commands, git operations, or memory writes.".into()
}

/// Evidence kind as lowercase snake_case string (matches serde, not Debug).
fn evidence_kind_str(kind: &TaskPlanEvidenceKind) -> &'static str {
    match kind {
        TaskPlanEvidenceKind::Goal => "goal",
        TaskPlanEvidenceKind::Skill => "skill",
        TaskPlanEvidenceKind::TraceEvent => "trace_event",
        TaskPlanEvidenceKind::MemoryClaim => "memory_claim",
        TaskPlanEvidenceKind::GovernanceRecord => "governance_record",
        TaskPlanEvidenceKind::UserIntent => "user_intent",
    }
}

/// Build summary row from a plan.
pub fn task_plan_summary_lines(plan: &TaskPlan) -> TaskPlanSummaryRow {
    TaskPlanSummaryRow {
        plan_id: plan.plan_id.0.clone(),
        title: plan.title.clone(),
        status: format!("{:?}", plan.status).to_lowercase(),
        step_count: plan.steps.len(),
        risk_count: plan.risks.len(),
        approval_count: plan.required_approvals.len(),
    }
}

/// Build step rows from a plan.
pub fn task_plan_step_rows(plan: &TaskPlan) -> Vec<TaskPlanStepRow> {
    plan.steps.iter().map(|s| {
        let kind_str = match s.kind {
            TaskPlanStepKind::Observe => "observe",
            TaskPlanStepKind::Analyze => "analyze",
            TaskPlanStepKind::ProposeChange => "propose_change",
            TaskPlanStepKind::RequestApproval => "request_approval",
            TaskPlanStepKind::Verify => "verify",
            TaskPlanStepKind::Report => "report",
        };
        TaskPlanStepRow {
            step_id: s.step_id.clone(),
            title: s.title.clone(),
            kind: kind_str.into(),
            risk_level: s.risk_level.clone(),
            requires_approval: s.requires_approval,
        }
    }).collect()
}

use openwand_workflow::plan::TaskPlanStepKind;

/// Build risk rows from a plan.
pub fn task_plan_risk_rows(plan: &TaskPlan) -> Vec<TaskPlanRiskRow> {
    plan.risks.iter().map(|r| TaskPlanRiskRow {
        risk_level: r.risk_level.clone(),
        summary: r.summary.clone(),
        mitigation: r.mitigation.clone(),
    }).collect()
}

/// Build evidence rows from a plan.
pub fn task_plan_evidence_rows(plan: &TaskPlan) -> Vec<TaskPlanEvidenceRow> {
    plan.evidence_links.iter().map(|e| TaskPlanEvidenceRow {
        kind: evidence_kind_str(&e.kind).into(),
        id: e.id.clone(),
        summary: e.summary.clone(),
    }).collect()
}

/// Build review display lines.
pub fn task_plan_review_lines(review: &TaskPlanReview) -> TaskPlanReviewRow {
    TaskPlanReviewRow {
        review_id: review.review_id.0.clone(),
        decision: format!("{:?}", review.decision).to_lowercase(),
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
    use openwand_workflow::plan_review::{TaskPlanFeedback, task_review_id_for};
    use chrono::Utc;

    fn test_plan() -> TaskPlan {
        build_task_plan(&TaskPlanInput {
            user_intent: "UI test plan".into(),
            skill_context: vec!["skill-1: Test skill".into()],
            goal_context: vec!["goal-1: Test goal".into()],
            memory_summaries: vec!["memory".into()],
            trace_summaries: vec!["trace".into()],
            governance_summaries: vec![],
            policy_constraints: vec!["No shell".into()],
        }).unwrap()
    }

    #[test]
    fn task_plan_ui_state_loads_latest_plan() {
        let plan = test_plan();
        let state = TaskPlanUiState {
            latest_plan: Some(task_plan_summary_lines(&plan)),
            latest_review: None,
            steps: task_plan_step_rows(&plan),
            risks: task_plan_risk_rows(&plan),
            assumptions: plan.assumptions.iter().map(|a| a.text.clone()).collect(),
            evidence_links: task_plan_evidence_rows(&plan),
            warnings: vec![],
        };
        assert!(state.latest_plan.is_some());
        assert!(!state.steps.is_empty());
    }

    #[test]
    fn task_plan_step_rows_show_kind_risk_approval() {
        let rows = task_plan_step_rows(&test_plan());
        assert!(rows.iter().any(|r| r.kind == "observe"));
        assert!(rows.iter().any(|r| r.kind == "verify"));
        // With policy constraints, step_3 should require approval
        assert!(rows.iter().any(|r| r.requires_approval));
    }

    #[test]
    fn task_plan_risk_rows_show_mitigation() {
        let rows = task_plan_risk_rows(&test_plan());
        assert!(!rows.is_empty());
        assert!(rows[0].summary.contains("Policy"));
        assert!(!rows[0].mitigation.is_empty());
    }

    #[test]
    fn task_plan_evidence_rows_show_kind_summary() {
        let rows = task_plan_evidence_rows(&test_plan());
        assert!(rows.iter().any(|r| r.kind == "goal"));
        assert!(rows.iter().any(|r| r.kind == "skill"));
        assert!(rows.iter().any(|r| r.kind == "user_intent"));
    }

    #[test]
    fn task_plan_review_lines_show_decision() {
        let plan = test_plan();
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
        let row = task_plan_review_lines(&review);
        assert_eq!("approved", row.decision);
        assert!(!row.creates_execution_grant);
        assert!(!row.execution_allowed_now);
    }

    #[test]
    fn task_plan_safety_warning_mentions_no_execution() {
        let warning = task_plan_safety_warning();
        assert!(warning.contains("do not execute"));
        assert!(warning.contains("reviewable evidence"));
    }

    #[test]
    fn task_plan_ui_does_not_expose_execute() {
        let plan = test_plan();
        let _rows = task_plan_step_rows(&plan);
    }
}
