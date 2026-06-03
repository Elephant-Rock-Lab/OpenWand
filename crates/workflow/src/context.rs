//! Plan derivation input and evidence link helpers.
//!
//! Receives context as strings, not typed imports from other crates.

use crate::plan::{TaskPlanEvidenceKind, TaskPlanEvidenceLink};

/// Input for plan derivation. All context is string-based.
#[derive(Debug, Clone)]
pub struct TaskPlanInput {
    pub user_intent: String,
    pub skill_context: Vec<String>,
    pub goal_context: Vec<String>,
    pub memory_summaries: Vec<String>,
    pub trace_summaries: Vec<String>,
    pub governance_summaries: Vec<String>,
    pub policy_constraints: Vec<String>,
}

/// Create an evidence link.
pub fn evidence_link(kind: TaskPlanEvidenceKind, id: impl Into<String>, summary: impl Into<String>) -> TaskPlanEvidenceLink {
    TaskPlanEvidenceLink {
        kind,
        id: id.into(),
        summary: summary.into(),
    }
}

/// Create a goal evidence link.
pub fn goal_evidence(goal_id: impl Into<String>, summary: impl Into<String>) -> TaskPlanEvidenceLink {
    evidence_link(TaskPlanEvidenceKind::Goal, goal_id, summary)
}

/// Create a skill evidence link.
pub fn skill_evidence(skill_id: impl Into<String>, summary: impl Into<String>) -> TaskPlanEvidenceLink {
    evidence_link(TaskPlanEvidenceKind::Skill, skill_id, summary)
}

/// Create a user-intent evidence link.
pub fn user_intent_evidence(summary: impl Into<String>) -> TaskPlanEvidenceLink {
    evidence_link(TaskPlanEvidenceKind::UserIntent, "user_intent", summary)
}
