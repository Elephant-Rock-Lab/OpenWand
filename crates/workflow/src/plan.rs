//! Task plan DTOs.
//!
//! Structurally omits all executable fields: no command, shell, tool_name,
//! tool_args, script, cwd, env, function_ref, workflow_handle, execution_grant.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Content-addressed plan ID. Format: `tpl_<blake3_hex>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskPlanId(pub String);

impl TaskPlanId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Non-executing task plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub plan_id: TaskPlanId,
    pub title: String,
    pub user_intent: String,
    pub status: TaskPlanStatus,
    pub steps: Vec<TaskPlanStep>,
    pub assumptions: Vec<TaskPlanAssumption>,
    pub risks: Vec<TaskPlanRisk>,
    pub required_approvals: Vec<TaskPlanApprovalRequirement>,
    pub evidence_links: Vec<TaskPlanEvidenceLink>,
    pub skill_context_ids: Vec<String>,
    pub goal_context_ids: Vec<String>,
    pub policy_constraints: Vec<String>,
    pub plan_hash: String,
    pub created_at: DateTime<Utc>,
}

/// Plan lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPlanStatus {
    Draft,
    Reviewable,
    Blocked,
    Superseded,
}

/// A single step in a task plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlanStep {
    pub step_id: String,
    pub title: String,
    pub description: String,
    pub kind: TaskPlanStepKind,
    pub depends_on: Vec<String>,
    pub expected_output: String,
    pub risk_level: String,
    pub requires_approval: bool,
    pub evidence_links: Vec<TaskPlanEvidenceLink>,
}

/// What kind of work a step describes. None of these execute tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPlanStepKind {
    Observe,
    Analyze,
    ProposeChange,
    RequestApproval,
    Verify,
    Report,
}

/// An assumption the plan makes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlanAssumption {
    pub text: String,
    pub evidence_links: Vec<TaskPlanEvidenceLink>,
}

/// A risk identified in the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlanRisk {
    pub risk_level: String,
    pub summary: String,
    pub mitigation: String,
}

/// A required human approval before a step can proceed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlanApprovalRequirement {
    pub step_id: String,
    pub reason: String,
    pub required_before: String,
}

/// A link to evidence that informed the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlanEvidenceLink {
    pub kind: TaskPlanEvidenceKind,
    pub id: String,
    pub summary: String,
}

/// What kind of evidence a link references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPlanEvidenceKind {
    Goal,
    Skill,
    TraceEvent,
    MemoryClaim,
    GovernanceRecord,
    UserIntent,
}
