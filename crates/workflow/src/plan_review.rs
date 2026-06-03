//! Plan review DTOs.
//!
//! A review is evidence, not an execution grant.
//! creates_execution_grant is always false.
//! execution_allowed_now is always false.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::plan::TaskPlanId;

/// Content-addressed review ID. Format: `tpr_<blake3_hex>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskPlanReviewId(pub String);

impl TaskPlanReviewId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A human review of a task plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlanReview {
    pub review_id: TaskPlanReviewId,
    pub plan_id: TaskPlanId,
    pub plan_hash: String,
    pub decision: TaskPlanReviewDecision,
    pub reviewer: String,
    pub rationale: String,
    pub feedback: Option<TaskPlanFeedback>,
    /// Always false. A review is evidence, not an execution grant.
    pub creates_execution_grant: bool,
    /// Always false. Approval does not authorize execution now.
    pub execution_allowed_now: bool,
    pub reviewed_at: DateTime<Utc>,
}

/// Review decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPlanReviewDecision {
    Approved,
    Rejected,
    ChangesRequested,
}

/// Structured feedback on a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlanFeedback {
    pub summary: String,
    pub blocking_reasons: Vec<String>,
    pub requested_changes: Vec<String>,
    pub evidence_gaps: Vec<String>,
}

/// Compute content-addressed review ID.
pub fn task_review_id_for(plan_id: &TaskPlanId, decision: &TaskPlanReviewDecision, rationale: &str) -> TaskPlanReviewId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(plan_id.0.as_bytes());
    let decision_str = serde_json::to_string(decision).unwrap_or_default();
    hasher.update(decision_str.as_bytes());
    hasher.update(rationale.as_bytes());
    let hash = hasher.finalize();
    TaskPlanReviewId(format!("tpr_{}", hash.to_hex()))
}
