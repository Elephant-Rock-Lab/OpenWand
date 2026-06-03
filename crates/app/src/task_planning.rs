//! Task plan persistence.
//!
//! Plans and reviews are durable evidence records stored as JSON files
//! under eval_reports/task_plans/. No execution queue or workflow run
//! is created.

use std::path::Path;

use openwand_workflow::plan::{TaskPlan, TaskPlanId, TaskPlanStatus};
use openwand_workflow::plan_review::{TaskPlanReview, TaskPlanReviewId};

/// Root directory for task plan evidence.
fn task_plans_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("task_plans")
}

fn plans_dir(store_root: &Path) -> std::path::PathBuf {
    task_plans_root(store_root).join("plans")
}

fn reviews_dir(store_root: &Path) -> std::path::PathBuf {
    task_plans_root(store_root).join("reviews")
}

fn feedback_dir(store_root: &Path) -> std::path::PathBuf {
    task_plans_root(store_root).join("feedback")
}

fn by_goal_dir(store_root: &Path) -> std::path::PathBuf {
    task_plans_root(store_root).join("by_goal")
}

fn by_skill_dir(store_root: &Path) -> std::path::PathBuf {
    task_plans_root(store_root).join("by_skill")
}

fn by_plan_reviews_dir(store_root: &Path, plan_id: &TaskPlanId) -> std::path::PathBuf {
    reviews_dir(store_root).join("by_plan").join(plan_id.0.as_str())
}

/// Save a task plan to disk.
///
/// Supersession: marks existing plans for same intent hash as Superseded
/// if plan_hash differs. (Mutation in save, not load — same doctrine as auto-commit proposals.)
pub fn save_task_plan(store_root: &Path, plan: &TaskPlan) -> Result<std::path::PathBuf, String> {
    let dir = plans_dir(store_root);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create task_plans/plans dir: {}", e))?;

    // Supersession: mark existing Draft/Reviewable plans as Superseded
    if let Ok(existing) = list_task_plans(store_root) {
        for mut existing_plan in existing {
            if existing_plan.plan_id != plan.plan_id
                && existing_plan.plan_hash != plan.plan_hash
                && existing_plan.status != TaskPlanStatus::Superseded
            {
                existing_plan.status = TaskPlanStatus::Superseded;
                let existing_path = dir.join(format!("{}.json", existing_plan.plan_id.0));
                let json = serde_json::to_string_pretty(&existing_plan)
                    .map_err(|e| format!("Failed to serialize superseded plan: {}", e))?;
                std::fs::write(&existing_path, json)
                    .map_err(|e| format!("Failed to write superseded plan: {}", e))?;
            }
        }
    }

    // Save the new plan
    let path = dir.join(format!("{}.json", plan.plan_id.0));
    let json = serde_json::to_string_pretty(plan)
        .map_err(|e| format!("Failed to serialize plan: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write plan: {}", e))?;

    // Update latest pointer
    let latest_path = dir.join("latest.json");
    std::fs::write(&latest_path, plan.plan_id.0.as_bytes())
        .map_err(|e| format!("Failed to write latest pointer: {}", e))?;

    // Update by_goal index
    for goal_id in &plan.goal_context_ids {
        let goal_dir = by_goal_dir(store_root);
        std::fs::create_dir_all(&goal_dir)
            .map_err(|e| format!("Failed to create by_goal dir: {}", e))?;
        let goal_path = goal_dir.join(format!("{}.json", goal_id));
        std::fs::write(&goal_path, plan.plan_id.0.as_bytes())
            .map_err(|e| format!("Failed to write by_goal pointer: {}", e))?;
    }

    // Update by_skill index
    for skill_id in &plan.skill_context_ids {
        let skill_dir = by_skill_dir(store_root);
        std::fs::create_dir_all(&skill_dir)
            .map_err(|e| format!("Failed to create by_skill dir: {}", e))?;
        let skill_path = skill_dir.join(format!("{}.json", skill_id));
        std::fs::write(&skill_path, plan.plan_id.0.as_bytes())
            .map_err(|e| format!("Failed to write by_skill pointer: {}", e))?;
    }

    Ok(path)
}

/// Load a task plan by ID.
pub fn load_task_plan(store_root: &Path, plan_id: &TaskPlanId) -> Result<TaskPlan, String> {
    let path = plans_dir(store_root).join(format!("{}.json", plan_id.0));
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read plan {}: {}", plan_id.0, e))?;
    serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse plan {}: {}", plan_id.0, e))
}

/// List all task plans sorted by created_at descending.
pub fn list_task_plans(store_root: &Path) -> Result<Vec<TaskPlan>, String> {
    let dir = plans_dir(store_root);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut plans = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("Failed to read plans dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "json") {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            if name == "latest" {
                continue;
            }
            let json = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
            if let Ok(plan) = serde_json::from_str::<TaskPlan>(&json) {
                plans.push(plan);
            }
        }
    }
    plans.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(plans)
}

/// Get the latest task plan.
pub fn latest_task_plan(store_root: &Path) -> Result<Option<TaskPlan>, String> {
    let latest_path = plans_dir(store_root).join("latest.json");
    if !latest_path.exists() {
        return Ok(None);
    }
    let plan_id_str = std::fs::read_to_string(&latest_path)
        .map_err(|e| format!("Failed to read latest pointer: {}", e))?;
    let plan_id = TaskPlanId(plan_id_str.trim().to_string());
    load_task_plan(store_root, &plan_id).map(Some)
}

/// Get latest task plan for a specific goal.
pub fn task_plans_by_goal(store_root: &Path, goal_id: &str) -> Result<Option<TaskPlan>, String> {
    let pointer_path = by_goal_dir(store_root).join(format!("{}.json", goal_id));
    if !pointer_path.exists() {
        return Ok(None);
    }
    let plan_id_str = std::fs::read_to_string(&pointer_path)
        .map_err(|e| format!("Failed to read by_goal pointer: {}", e))?;
    let plan_id = TaskPlanId(plan_id_str.trim().to_string());
    load_task_plan(store_root, &plan_id).map(Some)
}

/// Get latest task plan for a specific skill.
pub fn task_plans_by_skill(store_root: &Path, skill_id: &str) -> Result<Option<TaskPlan>, String> {
    let pointer_path = by_skill_dir(store_root).join(format!("{}.json", skill_id));
    if !pointer_path.exists() {
        return Ok(None);
    }
    let plan_id_str = std::fs::read_to_string(&pointer_path)
        .map_err(|e| format!("Failed to read by_skill pointer: {}", e))?;
    let plan_id = TaskPlanId(plan_id_str.trim().to_string());
    load_task_plan(store_root, &plan_id).map(Some)
}

/// Save a plan review to disk.
pub fn save_plan_review(
    store_root: &Path,
    review: &TaskPlanReview,
) -> Result<std::path::PathBuf, String> {
    let dir = reviews_dir(store_root);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create task_plans/reviews dir: {}", e))?;

    // Save the review
    let path = dir.join(format!("{}.json", review.review_id.0));
    let json = serde_json::to_string_pretty(review)
        .map_err(|e| format!("Failed to serialize review: {}", e))?;
    std::fs::write(&path, &json)
        .map_err(|e| format!("Failed to write review: {}", e))?;

    // Update latest pointer (supersedes prior for lookup)
    let latest_path = dir.join("latest.json");
    std::fs::write(&latest_path, review.review_id.0.as_bytes())
        .map_err(|e| format!("Failed to write latest review pointer: {}", e))?;

    // Update by_plan index
    let plan_reviews_dir = by_plan_reviews_dir(store_root, &review.plan_id);
    std::fs::create_dir_all(&plan_reviews_dir)
        .map_err(|e| format!("Failed to create by_plan dir: {}", e))?;
    let plan_review_path = plan_reviews_dir.join(format!("{}.json", review.review_id.0));
    std::fs::write(&plan_review_path, json)
        .map_err(|e| format!("Failed to write by_plan review: {}", e))?;

    // Save feedback separately if present
    if let Some(ref feedback) = review.feedback {
        let fb_dir = feedback_dir(store_root);
        std::fs::create_dir_all(&fb_dir)
            .map_err(|e| format!("Failed to create feedback dir: {}", e))?;
        let fb_path = fb_dir.join(format!("{}.json", review.review_id.0));
        let fb_json = serde_json::to_string_pretty(feedback)
            .map_err(|e| format!("Failed to serialize feedback: {}", e))?;
        std::fs::write(&fb_path, fb_json)
            .map_err(|e| format!("Failed to write feedback: {}", e))?;
    }

    Ok(path)
}

/// Load a plan review by ID.
pub fn load_plan_review(
    store_root: &Path,
    review_id: &TaskPlanReviewId,
) -> Result<TaskPlanReview, String> {
    let path = reviews_dir(store_root).join(format!("{}.json", review_id.0));
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read review {}: {}", review_id.0, e))?;
    serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse review {}: {}", review_id.0, e))
}

/// Get the latest review (supersedes prior for lookup).
pub fn latest_plan_review(store_root: &Path) -> Result<Option<TaskPlanReview>, String> {
    let latest_path = reviews_dir(store_root).join("latest.json");
    if !latest_path.exists() {
        return Ok(None);
    }
    let review_id_str = std::fs::read_to_string(&latest_path)
        .map_err(|e| format!("Failed to read latest review pointer: {}", e))?;
    let review_id = TaskPlanReviewId(review_id_str.trim().to_string());
    load_plan_review(store_root, &review_id).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::builder::build_task_plan;
    use openwand_workflow::context::TaskPlanInput;
    use openwand_workflow::plan_review::{
        TaskPlanFeedback, TaskPlanReview, TaskPlanReviewDecision,
        task_review_id_for,
    };
    use chrono::Utc;

    fn test_input(intent: &str) -> TaskPlanInput {
        TaskPlanInput {
            user_intent: intent.into(),
            skill_context: vec![],
            goal_context: vec![],
            memory_summaries: vec!["test".into()],
            trace_summaries: vec!["test".into()],
            governance_summaries: vec![],
            policy_constraints: vec![],
        }
    }

    fn test_input_with_goal(intent: &str) -> TaskPlanInput {
        TaskPlanInput {
            goal_context: vec!["ship-product: Ship OpenWand".into()],
            skill_context: vec!["rust-test-triage: Triage tests".into()],
            ..test_input(intent)
        }
    }

    fn test_dir() -> std::path::PathBuf {
        tempfile::tempdir().unwrap().into_path()
    }

    #[test]
    fn task_plan_persists_and_loads_roundtrip() {
        let dir = test_dir();
        let plan = build_task_plan(&test_input("Test persistence")).unwrap();
        save_task_plan(&dir, &plan).unwrap();
        let loaded = load_task_plan(&dir, &plan.plan_id).unwrap();
        assert_eq!(plan.plan_id, loaded.plan_id);
        assert_eq!(plan.title, loaded.title);
        assert_eq!(plan.steps.len(), loaded.steps.len());
    }

    #[test]
    fn latest_task_plan_returns_expected() {
        let dir = test_dir();
        let plan = build_task_plan(&test_input("Latest test")).unwrap();
        save_task_plan(&dir, &plan).unwrap();
        let latest = latest_task_plan(&dir).unwrap().unwrap();
        assert_eq!(plan.plan_id, latest.plan_id);
    }

    #[test]
    fn task_plan_by_goal_returns_expected() {
        let dir = test_dir();
        let plan = build_task_plan(&test_input_with_goal("Goal test")).unwrap();
        save_task_plan(&dir, &plan).unwrap();
        let found = task_plans_by_goal(&dir, "ship-product").unwrap().unwrap();
        assert_eq!(plan.plan_id, found.plan_id);
    }

    #[test]
    fn task_plan_by_skill_returns_expected() {
        let dir = test_dir();
        let plan = build_task_plan(&test_input_with_goal("Skill test")).unwrap();
        save_task_plan(&dir, &plan).unwrap();
        let found = task_plans_by_skill(&dir, "rust-test-triage").unwrap().unwrap();
        assert_eq!(plan.plan_id, found.plan_id);
    }

    #[test]
    fn plan_review_persists_and_loads_roundtrip() {
        let dir = test_dir();
        let plan = build_task_plan(&test_input("Review persistence")).unwrap();
        save_task_plan(&dir, &plan).unwrap();

        let review_id = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::Approved, "Looks good");
        let review = TaskPlanReview {
            review_id,
            plan_id: plan.plan_id.clone(),
            plan_hash: plan.plan_hash.clone(),
            decision: TaskPlanReviewDecision::Approved,
            reviewer: "test-user".into(),
            rationale: "Looks good".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        save_plan_review(&dir, &review).unwrap();
        let loaded = load_plan_review(&dir, &review.review_id).unwrap();
        assert_eq!(review.review_id, loaded.review_id);
        assert!(!loaded.creates_execution_grant);
        assert!(!loaded.execution_allowed_now);
    }

    #[test]
    fn latest_plan_review_returns_expected() {
        let dir = test_dir();
        let plan = build_task_plan(&test_input("Latest review")).unwrap();
        save_task_plan(&dir, &plan).unwrap();

        let review_id = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::Approved, "OK");
        let review = TaskPlanReview {
            review_id,
            plan_id: plan.plan_id.clone(),
            plan_hash: plan.plan_hash.clone(),
            decision: TaskPlanReviewDecision::Approved,
            reviewer: "test-user".into(),
            rationale: "OK".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        save_plan_review(&dir, &review).unwrap();
        let latest = latest_plan_review(&dir).unwrap().unwrap();
        assert_eq!(review.review_id, latest.review_id);
    }

    #[test]
    fn feedback_persists_and_loads_roundtrip() {
        let dir = test_dir();
        let plan = build_task_plan(&test_input("Feedback test")).unwrap();
        save_task_plan(&dir, &plan).unwrap();

        let review_id = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::Rejected, "Bad plan");
        let feedback = TaskPlanFeedback {
            summary: "Missing critical steps".into(),
            blocking_reasons: vec!["No verification step".into()],
            requested_changes: vec![],
            evidence_gaps: vec!["No memory evidence".into()],
        };
        let review = TaskPlanReview {
            review_id,
            plan_id: plan.plan_id.clone(),
            plan_hash: plan.plan_hash.clone(),
            decision: TaskPlanReviewDecision::Rejected,
            reviewer: "test-user".into(),
            rationale: "Bad plan".into(),
            feedback: Some(feedback),
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        save_plan_review(&dir, &review).unwrap();

        // Verify feedback file exists
        let fb_path = feedback_dir(&dir).join(format!("{}.json", review.review_id.0));
        assert!(fb_path.exists());
        let loaded_review = load_plan_review(&dir, &review.review_id).unwrap();
        assert!(loaded_review.feedback.is_some());
        assert_eq!(1, loaded_review.feedback.unwrap().blocking_reasons.len());
    }

    #[test]
    fn list_task_plans_sorted_by_date() {
        let dir = test_dir();
        let plan1 = build_task_plan(&test_input("First plan")).unwrap();
        save_task_plan(&dir, &plan1).unwrap();
        let plan2 = build_task_plan(&test_input("Second plan")).unwrap();
        save_task_plan(&dir, &plan2).unwrap();
        let plans = list_task_plans(&dir).unwrap();
        assert_eq!(2, plans.len());
        // Descending by date — second plan should be first
        assert!(plans[0].created_at >= plans[1].created_at);
    }

    #[test]
    fn missing_plan_dir_returns_empty_not_error() {
        let dir = test_dir();
        let plans = list_task_plans(&dir).unwrap();
        assert!(plans.is_empty());
        assert!(latest_task_plan(&dir).unwrap().is_none());
    }

    #[test]
    fn prior_plan_reviews_remain_persisted() {
        let dir = test_dir();
        let plan = build_task_plan(&test_input("Multi-review")).unwrap();
        save_task_plan(&dir, &plan).unwrap();

        // First review
        let review_id_1 = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::ChangesRequested, "Fix it");
        let review1 = TaskPlanReview {
            review_id: review_id_1.clone(),
            plan_id: plan.plan_id.clone(),
            plan_hash: plan.plan_hash.clone(),
            decision: TaskPlanReviewDecision::ChangesRequested,
            reviewer: "reviewer".into(),
            rationale: "Fix it".into(),
            feedback: Some(TaskPlanFeedback {
                summary: "Needs work".into(),
                blocking_reasons: vec![],
                requested_changes: vec!["Add verify step".into()],
                evidence_gaps: vec![],
            }),
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        save_plan_review(&dir, &review1).unwrap();

        // Second review (supersedes first for latest lookup)
        let review_id_2 = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::Approved, "Fixed");
        let review2 = TaskPlanReview {
            review_id: review_id_2.clone(),
            plan_id: plan.plan_id.clone(),
            plan_hash: plan.plan_hash.clone(),
            decision: TaskPlanReviewDecision::Approved,
            reviewer: "reviewer".into(),
            rationale: "Fixed".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        save_plan_review(&dir, &review2).unwrap();

        // Both reviews should still be loadable
        let loaded1 = load_plan_review(&dir, &review_id_1).unwrap();
        assert_eq!(TaskPlanReviewDecision::ChangesRequested, loaded1.decision);
        let loaded2 = load_plan_review(&dir, &review_id_2).unwrap();
        assert_eq!(TaskPlanReviewDecision::Approved, loaded2.decision);

        // Latest should be review2
        let latest = latest_plan_review(&dir).unwrap().unwrap();
        assert_eq!(review_id_2, latest.review_id);
    }

    #[test]
    fn latest_plan_review_supersedes_prior_for_lookup() {
        let dir = test_dir();
        let plan = build_task_plan(&test_input("Supersede test")).unwrap();
        save_task_plan(&dir, &plan).unwrap();

        let review_id_1 = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::Rejected, "Nope");
        let review1 = TaskPlanReview {
            review_id: review_id_1,
            plan_id: plan.plan_id.clone(),
            plan_hash: plan.plan_hash.clone(),
            decision: TaskPlanReviewDecision::Rejected,
            reviewer: "r".into(),
            rationale: "Nope".into(),
            feedback: Some(TaskPlanFeedback {
                summary: "Bad".into(),
                blocking_reasons: vec!["Wrong".into()],
                requested_changes: vec![],
                evidence_gaps: vec![],
            }),
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        save_plan_review(&dir, &review1).unwrap();

        let review_id_2 = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::Approved, "OK now");
        let review2 = TaskPlanReview {
            review_id: review_id_2,
            plan_id: plan.plan_id.clone(),
            plan_hash: plan.plan_hash.clone(),
            decision: TaskPlanReviewDecision::Approved,
            reviewer: "r".into(),
            rationale: "OK now".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        save_plan_review(&dir, &review2).unwrap();

        let latest = latest_plan_review(&dir).unwrap().unwrap();
        assert_eq!(TaskPlanReviewDecision::Approved, latest.decision);
    }

    #[test]
    fn task_plan_creation_writes_only_task_plan_evidence() {
        let dir = test_dir();
        let plan = build_task_plan(&test_input("Evidence write test")).unwrap();
        save_task_plan(&dir, &plan).unwrap();

        // Allowed: task_plans directory exists
        assert!(task_plans_root(&dir).exists());

        // Forbidden: no governance records written
        assert!(!dir.join("proposals").exists());
        assert!(!dir.join("readiness").exists());
        assert!(!dir.join("verifications").exists());
        assert!(!dir.join("push_readiness").exists());
        assert!(!dir.join("push_proposals").exists());
        assert!(!dir.join("push_executions").exists());

        // Forbidden: no trace store writes
        assert!(!dir.join("trace.db").exists());
        assert!(!dir.join("sessions.db").exists());
    }
}
