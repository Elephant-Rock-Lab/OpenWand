//! Workflow proposal persistence.
//!
//! Proposals and reviews are durable evidence records stored as JSON files
//! under eval_reports/workflow_proposals/. No workflow run, execution queue,
//! or scheduler record is created.

use std::path::Path;

use openwand_workflow::plan::TaskPlanId;
use openwand_workflow::workflow_proposal::{
    WorkflowProposal, WorkflowProposalId, WorkflowProposalStatus,
};
use openwand_workflow::workflow_proposal_review::{
    WorkflowProposalReview, WorkflowProposalReviewId,
};

/// Root directory for workflow proposal evidence.
fn workflow_proposals_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_proposals")
}

fn proposals_dir(store_root: &Path) -> std::path::PathBuf {
    workflow_proposals_root(store_root).join("proposals")
}

fn reviews_dir(store_root: &Path) -> std::path::PathBuf {
    workflow_proposals_root(store_root).join("reviews")
}

fn feedback_dir(store_root: &Path) -> std::path::PathBuf {
    workflow_proposals_root(store_root).join("feedback")
}

fn by_task_plan_dir(store_root: &Path) -> std::path::PathBuf {
    proposals_dir(store_root).join("by_task_plan")
}

fn by_proposal_reviews_dir(store_root: &Path, proposal_id: &WorkflowProposalId) -> std::path::PathBuf {
    reviews_dir(store_root).join("by_proposal").join(proposal_id.0.as_str())
}

/// Save a workflow proposal to disk.
///
/// Supersession: marks existing proposals as Blocked if proposal_hash differs.
/// Mutation in save, not load.
pub fn save_workflow_proposal(
    store_root: &Path,
    proposal: &WorkflowProposal,
) -> Result<std::path::PathBuf, String> {
    let dir = proposals_dir(store_root);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create workflow_proposals/proposals dir: {}", e))?;

    // Supersession: mark existing Draft/Reviewable proposals as Blocked
    if let Ok(existing) = list_workflow_proposals(store_root) {
        for mut existing_proposal in existing {
            if existing_proposal.proposal_id != proposal.proposal_id
                && existing_proposal.proposal_hash != proposal.proposal_hash
                && existing_proposal.status != WorkflowProposalStatus::Blocked
            {
                existing_proposal.status = WorkflowProposalStatus::Blocked;
                let existing_path =
                    dir.join(format!("{}.json", existing_proposal.proposal_id.0));
                let json = serde_json::to_string_pretty(&existing_proposal)
                    .map_err(|e| format!("Failed to serialize superseded proposal: {}", e))?;
                std::fs::write(&existing_path, json)
                    .map_err(|e| format!("Failed to write superseded proposal: {}", e))?;
            }
        }
    }

    // Save the new proposal
    let path = dir.join(format!("{}.json", proposal.proposal_id.0));
    let json = serde_json::to_string_pretty(proposal)
        .map_err(|e| format!("Failed to serialize proposal: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write proposal: {}", e))?;

    // Update latest pointer
    let latest_path = dir.join("latest.json");
    std::fs::write(&latest_path, proposal.proposal_id.0.as_bytes())
        .map_err(|e| format!("Failed to write latest pointer: {}", e))?;

    // Update by_task_plan index
    let tp_dir = by_task_plan_dir(store_root);
    std::fs::create_dir_all(&tp_dir)
        .map_err(|e| format!("Failed to create by_task_plan dir: {}", e))?;
    let tp_path = tp_dir.join(format!("{}.json", proposal.source_task_plan_id.0));
    std::fs::write(&tp_path, proposal.proposal_id.0.as_bytes())
        .map_err(|e| format!("Failed to write by_task_plan pointer: {}", e))?;

    Ok(path)
}

/// Load a workflow proposal by ID.
pub fn load_workflow_proposal(
    store_root: &Path,
    proposal_id: &WorkflowProposalId,
) -> Result<WorkflowProposal, String> {
    let path = proposals_dir(store_root).join(format!("{}.json", proposal_id.0));
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read proposal {}: {}", proposal_id.0, e))?;
    serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse proposal {}: {}", proposal_id.0, e))
}

/// List all workflow proposals sorted by created_at descending.
pub fn list_workflow_proposals(store_root: &Path) -> Result<Vec<WorkflowProposal>, String> {
    let dir = proposals_dir(store_root);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut proposals = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("Failed to read proposals dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            if name == "latest" {
                continue;
            }
            let json = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
            if let Ok(proposal) = serde_json::from_str::<WorkflowProposal>(&json) {
                proposals.push(proposal);
            }
        }
    }
    proposals.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(proposals)
}

/// Get the latest workflow proposal.
pub fn latest_workflow_proposal(store_root: &Path) -> Result<Option<WorkflowProposal>, String> {
    let latest_path = proposals_dir(store_root).join("latest.json");
    if !latest_path.exists() {
        return Ok(None);
    }
    let proposal_id_str = std::fs::read_to_string(&latest_path)
        .map_err(|e| format!("Failed to read latest pointer: {}", e))?;
    let proposal_id = WorkflowProposalId(proposal_id_str.trim().to_string());
    load_workflow_proposal(store_root, &proposal_id).map(Some)
}

/// Get workflow proposal for a specific task plan.
pub fn workflow_proposal_by_task_plan(
    store_root: &Path,
    task_plan_id: &TaskPlanId,
) -> Result<Option<WorkflowProposal>, String> {
    let pointer_path = by_task_plan_dir(store_root).join(format!("{}.json", task_plan_id.0));
    if !pointer_path.exists() {
        return Ok(None);
    }
    let proposal_id_str = std::fs::read_to_string(&pointer_path)
        .map_err(|e| format!("Failed to read by_task_plan pointer: {}", e))?;
    let proposal_id = WorkflowProposalId(proposal_id_str.trim().to_string());
    load_workflow_proposal(store_root, &proposal_id).map(Some)
}

/// Save a proposal review to disk.
pub fn save_proposal_review(
    store_root: &Path,
    review: &WorkflowProposalReview,
) -> Result<std::path::PathBuf, String> {
    let dir = reviews_dir(store_root);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create workflow_proposals/reviews dir: {}", e))?;

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

    // Update by_proposal index
    let proposal_reviews_dir = by_proposal_reviews_dir(store_root, &review.proposal_id);
    std::fs::create_dir_all(&proposal_reviews_dir)
        .map_err(|e| format!("Failed to create by_proposal dir: {}", e))?;
    let proposal_review_path = proposal_reviews_dir.join(format!("{}.json", review.review_id.0));
    std::fs::write(&proposal_review_path, json)
        .map_err(|e| format!("Failed to write by_proposal review: {}", e))?;

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

/// Load a proposal review by ID.
pub fn load_proposal_review(
    store_root: &Path,
    review_id: &WorkflowProposalReviewId,
) -> Result<WorkflowProposalReview, String> {
    let path = reviews_dir(store_root).join(format!("{}.json", review_id.0));
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read review {}: {}", review_id.0, e))?;
    serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse review {}: {}", review_id.0, e))
}

/// Get the latest review (supersedes prior for lookup).
pub fn latest_proposal_review(store_root: &Path) -> Result<Option<WorkflowProposalReview>, String> {
    let latest_path = reviews_dir(store_root).join("latest.json");
    if !latest_path.exists() {
        return Ok(None);
    }
    let review_id_str = std::fs::read_to_string(&latest_path)
        .map_err(|e| format!("Failed to read latest review pointer: {}", e))?;
    let review_id = WorkflowProposalReviewId(review_id_str.trim().to_string());
    load_proposal_review(store_root, &review_id).map(Some)
}

/// Load proposal and proposal review for a workflow run by its execution ID.
/// Read-only: loads the run record, then reads the proposal and review by their IDs.
pub fn proposal_and_review_by_workflow_run(
    store_root: &std::path::Path,
    execution_id: &str,
) -> Result<Option<(WorkflowProposal, Option<WorkflowProposalReview>)>, String> {
    use openwand_workflow::workflow_run::WorkflowRunRecord;
    let run_path = store_root
        .join("workflow_runs")
        .join("records")
        .join(format!("{}.json", execution_id));
    let run_json = std::fs::read_to_string(&run_path)
        .map_err(|e| format!("Failed to read workflow run {}: {}", execution_id, e))?;
    let run: WorkflowRunRecord = serde_json::from_str(&run_json)
        .map_err(|e| format!("Failed to parse workflow run {}: {}", execution_id, e))?;
    let proposal = load_workflow_proposal(store_root, &run.proposal_id)?;
    let review = load_proposal_review(store_root, &run.proposal_review_id).ok();
    Ok(Some((proposal, review)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::builder::build_task_plan;
    use openwand_workflow::context::TaskPlanInput;
    use openwand_workflow::plan_review::{
        TaskPlanFeedback, TaskPlanReview, TaskPlanReviewDecision, task_review_id_for,
    };
    use openwand_workflow::workflow_proposal_builder::{
        WorkflowProposalInput, build_workflow_proposal,
    };
    use openwand_workflow::workflow_proposal_review::{
        WorkflowProposalFeedback as WfFeedback,
        workflow_review_id_for,
    };
    use chrono::Utc;

    fn test_plan_and_review() -> (openwand_workflow::plan::TaskPlan, TaskPlanReview) {
        let plan = build_task_plan(&TaskPlanInput {
            user_intent: "Test proposal persistence".into(),
            skill_context: vec![],
            goal_context: vec![],
            memory_summaries: vec!["mem".into()],
            trace_summaries: vec!["trace".into()],
            governance_summaries: vec![],
            policy_constraints: vec![],
        })
        .unwrap();
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
        (plan, review)
    }

    fn test_proposal() -> WorkflowProposal {
        let (plan, review) = test_plan_and_review();
        build_workflow_proposal(WorkflowProposalInput {
            task_plan: plan,
            latest_task_plan_review: Some(review.clone()),
            task_plan_hash: review.plan_hash.clone(),
        })
        .unwrap()
    }

    fn test_dir() -> std::path::PathBuf {
        tempfile::tempdir().unwrap().into_path()
    }

    #[test]
    fn workflow_proposal_persists_and_loads_roundtrip() {
        let dir = test_dir();
        let proposal = test_proposal();
        save_workflow_proposal(&dir, &proposal).unwrap();
        let loaded = load_workflow_proposal(&dir, &proposal.proposal_id).unwrap();
        assert_eq!(proposal.proposal_id, loaded.proposal_id);
        assert_eq!(proposal.title, loaded.title);
        assert_eq!(proposal.stages.len(), loaded.stages.len());
    }

    #[test]
    fn latest_workflow_proposal_returns_expected() {
        let dir = test_dir();
        let proposal = test_proposal();
        save_workflow_proposal(&dir, &proposal).unwrap();
        let latest = latest_workflow_proposal(&dir).unwrap().unwrap();
        assert_eq!(proposal.proposal_id, latest.proposal_id);
    }

    #[test]
    fn workflow_proposal_by_task_plan_returns_expected() {
        let dir = test_dir();
        let proposal = test_proposal();
        save_workflow_proposal(&dir, &proposal).unwrap();
        let found = workflow_proposal_by_task_plan(&dir, &proposal.source_task_plan_id)
            .unwrap()
            .unwrap();
        assert_eq!(proposal.proposal_id, found.proposal_id);
    }

    #[test]
    fn workflow_review_persists_and_loads_roundtrip() {
        let dir = test_dir();
        let proposal = test_proposal();
        save_workflow_proposal(&dir, &proposal).unwrap();

        let review_id = workflow_review_id_for(
            &proposal.proposal_id,
            &openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::Approved,
            "Good proposal",
        );
        let review = WorkflowProposalReview {
            review_id,
            proposal_id: proposal.proposal_id.clone(),
            source_task_plan_id: proposal.source_task_plan_id.clone(),
            proposal_hash: proposal.proposal_hash.clone(),
            decision: openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::Approved,
            reviewer: "tester".into(),
            rationale: "Good proposal".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        save_proposal_review(&dir, &review).unwrap();
        let loaded = load_proposal_review(&dir, &review.review_id).unwrap();
        assert_eq!(review.review_id, loaded.review_id);
        assert!(!loaded.creates_execution_grant);
    }

    #[test]
    fn latest_workflow_review_returns_expected() {
        let dir = test_dir();
        let proposal = test_proposal();
        save_workflow_proposal(&dir, &proposal).unwrap();

        let review_id = workflow_review_id_for(
            &proposal.proposal_id,
            &openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::Approved,
            "OK",
        );
        let review = WorkflowProposalReview {
            review_id,
            proposal_id: proposal.proposal_id.clone(),
            source_task_plan_id: proposal.source_task_plan_id.clone(),
            proposal_hash: proposal.proposal_hash.clone(),
            decision: openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::Approved,
            reviewer: "tester".into(),
            rationale: "OK".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        save_proposal_review(&dir, &review).unwrap();
        let latest = latest_proposal_review(&dir).unwrap().unwrap();
        assert_eq!(review.review_id, latest.review_id);
    }

    #[test]
    fn prior_workflow_reviews_remain_persisted() {
        let dir = test_dir();
        let proposal = test_proposal();
        save_workflow_proposal(&dir, &proposal).unwrap();

        // First review
        let review_id_1 = workflow_review_id_for(
            &proposal.proposal_id,
            &openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::ChangesRequested,
            "Fix it",
        );
        let review1 = WorkflowProposalReview {
            review_id: review_id_1.clone(),
            proposal_id: proposal.proposal_id.clone(),
            source_task_plan_id: proposal.source_task_plan_id.clone(),
            proposal_hash: proposal.proposal_hash.clone(),
            decision: openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::ChangesRequested,
            reviewer: "r".into(),
            rationale: "Fix it".into(),
            feedback: Some(WfFeedback {
                summary: "Needs work".into(),
                blocking_reasons: vec![],
                requested_changes: vec!["Add detail".into()],
                evidence_gaps: vec![],
            }),
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        save_proposal_review(&dir, &review1).unwrap();

        // Second review (supersedes first for latest lookup)
        let review_id_2 = workflow_review_id_for(
            &proposal.proposal_id,
            &openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::Approved,
            "Fixed",
        );
        let review2 = WorkflowProposalReview {
            review_id: review_id_2.clone(),
            proposal_id: proposal.proposal_id.clone(),
            source_task_plan_id: proposal.source_task_plan_id.clone(),
            proposal_hash: proposal.proposal_hash.clone(),
            decision: openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::Approved,
            reviewer: "r".into(),
            rationale: "Fixed".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        save_proposal_review(&dir, &review2).unwrap();

        // Both reviews should still be loadable
        let loaded1 = load_proposal_review(&dir, &review_id_1).unwrap();
        assert_eq!(
            openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::ChangesRequested,
            loaded1.decision
        );
        let loaded2 = load_proposal_review(&dir, &review_id_2).unwrap();
        assert_eq!(
            openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::Approved,
            loaded2.decision
        );

        // Latest should be review2
        let latest = latest_proposal_review(&dir).unwrap().unwrap();
        assert_eq!(review_id_2, latest.review_id);
    }

    #[test]
    fn latest_workflow_review_supersedes_prior_for_lookup() {
        let dir = test_dir();
        let proposal = test_proposal();
        save_workflow_proposal(&dir, &proposal).unwrap();

        let review_id_1 = workflow_review_id_for(
            &proposal.proposal_id,
            &openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::Rejected,
            "Nope",
        );
        let review1 = WorkflowProposalReview {
            review_id: review_id_1,
            proposal_id: proposal.proposal_id.clone(),
            source_task_plan_id: proposal.source_task_plan_id.clone(),
            proposal_hash: proposal.proposal_hash.clone(),
            decision: openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::Rejected,
            reviewer: "r".into(),
            rationale: "Nope".into(),
            feedback: Some(WfFeedback {
                summary: "Bad".into(),
                blocking_reasons: vec!["Wrong".into()],
                requested_changes: vec![],
                evidence_gaps: vec![],
            }),
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        save_proposal_review(&dir, &review1).unwrap();

        let review_id_2 = workflow_review_id_for(
            &proposal.proposal_id,
            &openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::Approved,
            "OK now",
        );
        let review2 = WorkflowProposalReview {
            review_id: review_id_2,
            proposal_id: proposal.proposal_id.clone(),
            source_task_plan_id: proposal.source_task_plan_id.clone(),
            proposal_hash: proposal.proposal_hash.clone(),
            decision: openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::Approved,
            reviewer: "r".into(),
            rationale: "OK now".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        save_proposal_review(&dir, &review2).unwrap();

        let latest = latest_proposal_review(&dir).unwrap().unwrap();
        assert_eq!(
            openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::Approved,
            latest.decision
        );
    }

    #[test]
    fn workflow_feedback_persists_and_loads_roundtrip() {
        let dir = test_dir();
        let proposal = test_proposal();
        save_workflow_proposal(&dir, &proposal).unwrap();

        let review_id = workflow_review_id_for(
            &proposal.proposal_id,
            &openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::Rejected,
            "Bad",
        );
        let review = WorkflowProposalReview {
            review_id: review_id.clone(),
            proposal_id: proposal.proposal_id.clone(),
            source_task_plan_id: proposal.source_task_plan_id.clone(),
            proposal_hash: proposal.proposal_hash.clone(),
            decision: openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::Rejected,
            reviewer: "r".into(),
            rationale: "Bad".into(),
            feedback: Some(WfFeedback {
                summary: "Critical issues".into(),
                blocking_reasons: vec!["Missing stages".into()],
                requested_changes: vec![],
                evidence_gaps: vec!["No trace evidence".into()],
            }),
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        save_proposal_review(&dir, &review).unwrap();

        let fb_path = feedback_dir(&dir).join(format!("{}.json", review_id.0));
        assert!(fb_path.exists());
        let loaded = load_proposal_review(&dir, &review_id).unwrap();
        assert!(loaded.feedback.is_some());
        assert_eq!(1, loaded.feedback.unwrap().blocking_reasons.len());
    }

    #[test]
    fn workflow_proposal_creation_writes_only_workflow_proposal_evidence() {
        let dir = test_dir();
        let proposal = test_proposal();
        save_workflow_proposal(&dir, &proposal).unwrap();

        // Allowed: workflow_proposals directory exists
        assert!(workflow_proposals_root(&dir).exists());

        // Forbidden: no governance records written
        assert!(!dir.join("proposals").exists());
        assert!(!dir.join("readiness").exists());
        assert!(!dir.join("verifications").exists());

        // Forbidden: no trace store writes
        assert!(!dir.join("trace.db").exists());
        assert!(!dir.join("sessions.db").exists());

        // Forbidden: no task_plan records written
        assert!(!dir.join("task_plans").exists());
    }

    #[test]
    fn workflow_review_writes_only_workflow_review_evidence() {
        let dir = test_dir();
        let proposal = test_proposal();
        save_workflow_proposal(&dir, &proposal).unwrap();

        let review_id = workflow_review_id_for(
            &proposal.proposal_id,
            &openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::Approved,
            "OK",
        );
        let review = WorkflowProposalReview {
            review_id,
            proposal_id: proposal.proposal_id.clone(),
            source_task_plan_id: proposal.source_task_plan_id.clone(),
            proposal_hash: proposal.proposal_hash.clone(),
            decision: openwand_workflow::workflow_proposal_review::WorkflowProposalReviewDecision::Approved,
            reviewer: "r".into(),
            rationale: "OK".into(),
            feedback: None,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        };
        save_proposal_review(&dir, &review).unwrap();

        // Only workflow_proposals directory should exist
        assert!(reviews_dir(&dir).exists());
        assert!(!dir.join("task_plans").exists());
        assert!(!dir.join("trace.db").exists());
    }

    #[test]
    fn workflow_proposal_creation_does_not_write_task_plan_records() {
        let dir = test_dir();
        let proposal = test_proposal();
        save_workflow_proposal(&dir, &proposal).unwrap();

        // The proposal references a task plan, but must not create task plan records
        assert!(!dir.join("task_plans").exists());
        assert!(!dir.join("task_plans").join("plans").exists());
    }

    #[test]
    fn workflow_proposal_creation_does_not_write_governance_records() {
        let dir = test_dir();
        let proposal = test_proposal();
        save_workflow_proposal(&dir, &proposal).unwrap();

        // No governance-related directories
        assert!(!dir.join("governance").exists());
        assert!(!dir.join("governance_records").exists());
        assert!(!dir.join("push_proposals").exists());
        assert!(!dir.join("push_readiness").exists());
        assert!(!dir.join("push_executions").exists());
    }
}
