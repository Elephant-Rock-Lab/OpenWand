//! Task plan and review validation.

use crate::plan::*;
use crate::plan_review::*;

/// Validate a task plan.
pub fn validate_task_plan(plan: &TaskPlan) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if plan.user_intent.trim().is_empty() {
        errors.push("user_intent must not be empty".into());
    }

    if plan.status == TaskPlanStatus::Reviewable && plan.steps.is_empty() {
        errors.push("reviewable plan must have at least one step".into());
    }

    // Validate step dependency references
    let step_ids: Vec<&str> = plan.steps.iter().map(|s| s.step_id.as_str()).collect();
    for step in &plan.steps {
        for dep in &step.depends_on {
            if !step_ids.contains(&dep.as_str()) {
                errors.push(format!(
                    "step '{}' references unknown dependency '{}'",
                    step.step_id, dep
                ));
            }
        }
    }

    // Validate step kinds are known
    let _valid_kinds: &[&str] = &["observe", "analyze", "propose_change", "request_approval", "verify", "report"];

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Validate a task plan review.
pub fn validate_task_plan_review(review: &TaskPlanReview) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if review.reviewer.trim().is_empty() {
        errors.push("reviewer must not be empty".into());
    }

    if review.rationale.trim().is_empty() {
        errors.push("rationale must not be empty".into());
    }

    if review.creates_execution_grant {
        errors.push("creates_execution_grant must be false".into());
    }

    if review.execution_allowed_now {
        errors.push("execution_allowed_now must be false".into());
    }

    match review.decision {
        TaskPlanReviewDecision::Rejected => {
            if let Some(ref feedback) = review.feedback {
                if feedback.blocking_reasons.is_empty() {
                    errors.push("rejection requires at least one blocking reason".into());
                }
            } else {
                errors.push("rejection requires feedback with blocking reasons".into());
            }
        }
        TaskPlanReviewDecision::ChangesRequested => {
            if let Some(ref feedback) = review.feedback {
                if feedback.requested_changes.is_empty() {
                    errors.push("change request requires at least one requested change".into());
                }
            } else {
                errors.push("change request requires feedback with requested changes".into());
            }
        }
        TaskPlanReviewDecision::Approved => {}
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Compute content-addressed plan ID from normalized fields.
pub fn task_plan_id_for(
    user_intent: &str,
    title: &str,
    step_count: usize,
    goal_context_ids: &[String],
    skill_context_ids: &[String],
) -> TaskPlanId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(user_intent.trim().as_bytes());
    hasher.update(title.trim().as_bytes());
    hasher.update(step_count.to_le_bytes().as_slice());
    for id in goal_context_ids {
        hasher.update(id.as_bytes());
    }
    for id in skill_context_ids {
        hasher.update(id.as_bytes());
    }
    let hash = hasher.finalize();
    TaskPlanId(format!("tpl_{}", hash.to_hex()))
}

/// Compute plan hash from steps and constraints.
pub fn compute_plan_hash(steps: &[TaskPlanStep], constraints: &[String]) -> String {
    let mut hasher = blake3::Hasher::new();
    for step in steps {
        hasher.update(step.step_id.as_bytes());
        hasher.update(step.title.as_bytes());
        let kind_str = serde_json::to_string(&step.kind).unwrap_or_default();
        hasher.update(kind_str.as_bytes());
    }
    for c in constraints {
        hasher.update(c.as_bytes());
    }
    let hash = hasher.finalize();
    hash.to_hex().to_string()
}
