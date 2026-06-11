//! Workflow execution persistence.
//!
//! Run records are durable evidence under eval_reports/workflow_runs/.
//! No tool execution, trace append, memory write, shell/git.

use std::path::Path;

use openwand_workflow::plan::TaskPlanId;
use openwand_workflow::workflow_proposal::WorkflowProposalId;
use openwand_workflow::workflow_proposal_review::WorkflowProposalReviewId;
use openwand_workflow::workflow_readiness::WorkflowReadinessId;
use openwand_workflow::workflow_run::{WorkflowExecutionId, WorkflowRunRecord, WorkflowRunStatus};

fn workflow_runs_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_runs")
}

fn records_dir(store_root: &Path) -> std::path::PathBuf {
    workflow_runs_root(store_root).join("records")
}

fn by_readiness_dir(store_root: &Path) -> std::path::PathBuf {
    workflow_runs_root(store_root).join("by_readiness")
}

fn by_proposal_dir(store_root: &Path) -> std::path::PathBuf {
    workflow_runs_root(store_root).join("by_proposal")
}

fn by_review_dir(store_root: &Path) -> std::path::PathBuf {
    workflow_runs_root(store_root).join("by_review")
}

fn by_task_plan_dir(store_root: &Path) -> std::path::PathBuf {
    workflow_runs_root(store_root).join("by_task_plan")
}

/// Save a workflow run record.
pub fn save_workflow_run(
    store_root: &Path,
    record: &WorkflowRunRecord,
) -> Result<std::path::PathBuf, String> {
    let dir = records_dir(store_root);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create workflow_runs/records dir: {}", e))?;

    // Idempotency: Completed can't duplicate for same readiness/proposal/review
    if let Ok(existing) = list_workflow_runs(store_root) {
        for er in &existing {
            if er.readiness_id == record.readiness_id
                && er.proposal_id == record.proposal_id
                && er.proposal_review_id == record.proposal_review_id
            {
                // Same execution_id → return existing
                if er.execution_id == record.execution_id {
                    let path = dir.join(format!("{}.json", er.execution_id.0));
                    return Ok(path);
                }
                // Completed can't duplicate even with different key
                if er.status == WorkflowRunStatus::Completed
                    && record.status == WorkflowRunStatus::Completed
                {
                    let path = dir.join(format!("{}.json", er.execution_id.0));
                    return Ok(path);
                }
            }
        }
    }

    let path = dir.join(format!("{}.json", record.execution_id.0));
    let json = serde_json::to_string_pretty(record)
        .map_err(|e| format!("Failed to serialize run: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write run: {}", e))?;

    // Latest pointer
    let latest_path = dir.join("latest.json");
    std::fs::write(&latest_path, record.execution_id.0.as_bytes())
        .map_err(|e| format!("Failed to write latest: {}", e))?;

    // Indexes
    for (idx_dir, key) in [
        (by_readiness_dir(store_root), record.readiness_id.0.as_str()),
        (by_proposal_dir(store_root), record.proposal_id.0.as_str()),
        (by_review_dir(store_root), record.proposal_review_id.0.as_str()),
        (by_task_plan_dir(store_root), record.source_task_plan_id.0.as_str()),
    ] {
        std::fs::create_dir_all(&idx_dir)
            .map_err(|e| format!("Failed to create index dir: {}", e))?;
        std::fs::write(idx_dir.join(format!("{}.json", key)), record.execution_id.0.as_bytes())
            .map_err(|e| format!("Failed to write index: {}", e))?;
    }

    Ok(path)
}

/// Load a workflow run by ID.
pub fn load_workflow_run(store_root: &Path, execution_id: &WorkflowExecutionId) -> Result<WorkflowRunRecord, String> {
    let path = records_dir(store_root).join(format!("{}.json", execution_id.0));
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read run {}: {}", execution_id.0, e))?;
    serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse run {}: {}", execution_id.0, e))
}

/// List all workflow runs.
pub fn list_workflow_runs(store_root: &Path) -> Result<Vec<WorkflowRunRecord>, String> {
    let dir = records_dir(store_root);
    if !dir.exists() { return Ok(vec![]); }
    let mut records = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("Failed to read dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            if name == "latest" { continue; }
            let json = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
            if let Ok(record) = serde_json::from_str::<WorkflowRunRecord>(&json) {
                records.push(record);
            }
        }
    }
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(records)
}

/// Get latest run.
pub fn latest_workflow_run(store_root: &Path) -> Result<Option<WorkflowRunRecord>, String> {
    let latest_path = records_dir(store_root).join("latest.json");
    if !latest_path.exists() { return Ok(None); }
    let id_str = std::fs::read_to_string(&latest_path)
        .map_err(|e| format!("Failed to read latest: {}", e))?;
    load_workflow_run(store_root, &WorkflowExecutionId(id_str.trim().into())).map(Some)
}

/// Get run by readiness.
pub fn workflow_run_by_readiness(store_root: &Path, readiness_id: &WorkflowReadinessId) -> Result<Option<WorkflowRunRecord>, String> {
    let pointer = by_readiness_dir(store_root).join(format!("{}.json", readiness_id.0));
    if !pointer.exists() { return Ok(None); }
    let id_str = std::fs::read_to_string(&pointer).map_err(|e| format!("Failed: {}", e))?;
    load_workflow_run(store_root, &WorkflowExecutionId(id_str.trim().into())).map(Some)
}

/// Get run by proposal.
pub fn workflow_run_by_proposal(store_root: &Path, proposal_id: &WorkflowProposalId) -> Result<Option<WorkflowRunRecord>, String> {
    let pointer = by_proposal_dir(store_root).join(format!("{}.json", proposal_id.0));
    if !pointer.exists() { return Ok(None); }
    let id_str = std::fs::read_to_string(&pointer).map_err(|e| format!("Failed: {}", e))?;
    load_workflow_run(store_root, &WorkflowExecutionId(id_str.trim().into())).map(Some)
}

/// Get run by review.
pub fn workflow_run_by_review(store_root: &Path, review_id: &WorkflowProposalReviewId) -> Result<Option<WorkflowRunRecord>, String> {
    let pointer = by_review_dir(store_root).join(format!("{}.json", review_id.0));
    if !pointer.exists() { return Ok(None); }
    let id_str = std::fs::read_to_string(&pointer).map_err(|e| format!("Failed: {}", e))?;
    load_workflow_run(store_root, &WorkflowExecutionId(id_str.trim().into())).map(Some)
}

/// Get run by task plan.
pub fn workflow_run_by_task_plan(store_root: &Path, task_plan_id: &TaskPlanId) -> Result<Option<WorkflowRunRecord>, String> {
    let pointer = by_task_plan_dir(store_root).join(format!("{}.json", task_plan_id.0));
    if !pointer.exists() { return Ok(None); }
    let id_str = std::fs::read_to_string(&pointer).map_err(|e| format!("Failed: {}", e))?;
    load_workflow_run(store_root, &WorkflowExecutionId(id_str.trim().into())).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::builder::build_task_plan;
    use openwand_workflow::context::TaskPlanInput;
    use openwand_workflow::plan_review::{TaskPlanReview, TaskPlanReviewDecision, task_review_id_for};
    use openwand_workflow::workflow_proposal_builder::{WorkflowProposalInput, build_workflow_proposal};
    use openwand_workflow::workflow_proposal_review::{WorkflowProposalReview, WorkflowProposalReviewDecision, workflow_review_id_for};
    use openwand_workflow::workflow_readiness::{WorkflowReadinessRequest, WorkflowEnvironmentSnapshot};
    use openwand_workflow::workflow_readiness_evaluator::{WorkflowReadinessContext, evaluate_workflow_readiness};
    use openwand_workflow::workflow_run::{WorkflowExecutionRequest, WorkflowRunSnapshot, WorkflowAbortSnapshot, WorkflowExecutionDecision};
    use openwand_workflow::workflow_execution_gate::{WorkflowExecutionContext, evaluate_workflow_execution};
    use chrono::Utc;

    fn test_dir() -> std::path::PathBuf { tempfile::tempdir().unwrap().into_path() }

    fn full_run_record() -> WorkflowRunRecord {
        let plan = build_task_plan(&TaskPlanInput {
            user_intent: "Run persistence test".into(), skill_context: vec![], goal_context: vec![],
            memory_summaries: vec!["mem".into()], trace_summaries: vec!["trace".into()],
            governance_summaries: vec![], policy_constraints: vec!["No shell".into()],
        }).unwrap();
        let pr_id = task_review_id_for(&plan.plan_id, &TaskPlanReviewDecision::Approved, "OK");
        let plan_review = TaskPlanReview { review_id: pr_id, plan_id: plan.plan_id.clone(), plan_hash: plan.plan_hash.clone(),
            decision: TaskPlanReviewDecision::Approved, reviewer: "t".into(), rationale: "OK".into(), feedback: None,
            creates_execution_grant: false, execution_allowed_now: false, reviewed_at: Utc::now() };
        let proposal = build_workflow_proposal(WorkflowProposalInput { task_plan: plan.clone(),
            latest_task_plan_review: Some(plan_review.clone()), task_plan_hash: plan.plan_hash.clone() }).unwrap();
        let wr_id = workflow_review_id_for(&proposal.proposal_id, &WorkflowProposalReviewDecision::Approved, "OK");
        let proposal_review = WorkflowProposalReview { review_id: wr_id, proposal_id: proposal.proposal_id.clone(),
            source_task_plan_id: proposal.source_task_plan_id.clone(), proposal_hash: proposal.proposal_hash.clone(),
            decision: WorkflowProposalReviewDecision::Approved, reviewer: "t".into(), rationale: "OK".into(),
            feedback: None, creates_execution_grant: false, execution_allowed_now: false, reviewed_at: Utc::now() };
        let readiness_req = WorkflowReadinessRequest { proposal_id: proposal.proposal_id.clone(),
            review_id: proposal_review.review_id.clone(), expected_proposal_hash: proposal.proposal_hash.clone(),
            expected_source_task_plan_hash: proposal.source_task_plan_hash.clone(),
            requested_by: "t".into(), requested_at: Utc::now(), idempotency_key: "k".into() };
        let readiness_ctx = WorkflowReadinessContext { proposal: Some(proposal.clone()), review: Some(proposal_review.clone()),
            latest_review_for_proposal: Some(proposal_review.clone()), source_task_plan: Some(plan.clone()),
            source_task_plan_review: Some(plan_review.clone()), latest_source_task_plan_review: Some(plan_review),
            environment: WorkflowEnvironmentSnapshot { workspace_observed: true, provider_config_available: true,
                session_runtime_available: true, tool_manifest_available: true, policy_context_available: true, notes: vec![] },
            existing_readiness_records: vec![] };
        let readiness = evaluate_workflow_readiness(&readiness_req, &readiness_ctx);
        let exec_req = WorkflowExecutionRequest { readiness_id: readiness.readiness_id.clone(),
            proposal_id: proposal.proposal_id.clone(), proposal_review_id: proposal_review.review_id.clone(),
            expected_readiness_hash: readiness.proposal_hash.clone(), expected_proposal_hash: proposal.proposal_hash.clone(),
            requested_by: "t".into(), requested_at: Utc::now(), idempotency_key: "k".into() };
        let exec_ctx = WorkflowExecutionContext { readiness: Some(readiness), proposal: Some(proposal),
            proposal_review: Some(proposal_review.clone()), latest_proposal_review: Some(proposal_review),
            source_task_plan: Some(plan.clone()), source_task_plan_review: None, latest_source_task_plan_review: None,
            provider_config_available: true, session_runtime_available: true, existing_runs: vec![] };
        evaluate_workflow_execution(&exec_req, &exec_ctx)
    }

    #[test]
    fn workflow_run_persists_and_loads_roundtrip() {
        let dir = test_dir(); let record = full_run_record();
        save_workflow_run(&dir, &record).unwrap();
        let loaded = load_workflow_run(&dir, &record.execution_id).unwrap();
        assert_eq!(record.execution_id, loaded.execution_id);
    }

    #[test]
    fn latest_workflow_run_returns_expected() {
        let dir = test_dir(); let record = full_run_record();
        save_workflow_run(&dir, &record).unwrap();
        let latest = latest_workflow_run(&dir).unwrap().unwrap();
        assert_eq!(record.execution_id, latest.execution_id);
    }

    #[test]
    fn workflow_run_by_readiness_returns_expected() {
        let dir = test_dir(); let record = full_run_record();
        save_workflow_run(&dir, &record).unwrap();
        let found = workflow_run_by_readiness(&dir, &record.readiness_id).unwrap().unwrap();
        assert_eq!(record.execution_id, found.execution_id);
    }

    #[test]
    fn workflow_run_by_proposal_returns_expected() {
        let dir = test_dir(); let record = full_run_record();
        save_workflow_run(&dir, &record).unwrap();
        let found = workflow_run_by_proposal(&dir, &record.proposal_id).unwrap().unwrap();
        assert_eq!(record.execution_id, found.execution_id);
    }

    #[test]
    fn workflow_run_by_review_returns_expected() {
        let dir = test_dir(); let record = full_run_record();
        save_workflow_run(&dir, &record).unwrap();
        let found = workflow_run_by_review(&dir, &record.proposal_review_id).unwrap().unwrap();
        assert_eq!(record.execution_id, found.execution_id);
    }

    #[test]
    fn workflow_run_by_task_plan_returns_expected() {
        let dir = test_dir(); let record = full_run_record();
        save_workflow_run(&dir, &record).unwrap();
        let found = workflow_run_by_task_plan(&dir, &record.source_task_plan_id).unwrap().unwrap();
        assert_eq!(record.execution_id, found.execution_id);
    }

    #[test]
    fn same_idempotency_key_returns_existing_workflow_run() {
        let dir = test_dir(); let record = full_run_record();
        let p1 = save_workflow_run(&dir, &record).unwrap();
        let p2 = save_workflow_run(&dir, &record).unwrap();
        assert_eq!(p1, p2);
    }

    #[test]
    fn blocked_workflow_run_can_retry_with_new_key() {
        let dir = test_dir();
        let mut record = full_run_record();
        record.status = WorkflowRunStatus::Blocked;
        record.decision = WorkflowExecutionDecision::Blocked { reason_code: "test".into(), summary: "blocked".into() };
        save_workflow_run(&dir, &record).unwrap();
        record.execution_id = WorkflowExecutionId("wfx_new".into());
        let p2 = save_workflow_run(&dir, &record).unwrap();
        assert!(p2.exists());
    }

    #[test]
    fn suspended_workflow_run_can_retry_with_new_key() {
        let dir = test_dir();
        let mut record = full_run_record();
        record.execution_id = WorkflowExecutionId("wfx_susp1".into());
        save_workflow_run(&dir, &record).unwrap();
        record.execution_id = WorkflowExecutionId("wfx_susp2".into());
        let p2 = save_workflow_run(&dir, &record).unwrap();
        assert!(p2.exists());
    }

    #[test]
    fn completed_workflow_run_cannot_duplicate_with_different_key() {
        let dir = test_dir();
        let mut record = full_run_record();
        record.status = WorkflowRunStatus::Completed;
        record.decision = WorkflowExecutionDecision::RunCreated;
        let p1 = save_workflow_run(&dir, &record).unwrap();
        record.execution_id = WorkflowExecutionId("wfx_diff".into());
        let p2 = save_workflow_run(&dir, &record).unwrap();
        assert_eq!(p1, p2);
    }

    #[test]
    fn workflow_execution_writes_only_workflow_run_evidence() {
        let dir = test_dir(); let record = full_run_record();
        save_workflow_run(&dir, &record).unwrap();
        assert!(workflow_runs_root(&dir).exists());
        assert!(!dir.join("workflow_proposals").exists());
        assert!(!dir.join("workflow_readiness").exists());
        assert!(!dir.join("task_plans").exists());
        assert!(!dir.join("trace.db").exists());
    }

    #[test]
    fn workflow_execution_does_not_write_readiness_records() {
        let dir = test_dir(); save_workflow_run(&dir, &full_run_record()).unwrap();
        assert!(!dir.join("workflow_readiness").exists());
    }

    #[test]
    fn workflow_execution_does_not_write_proposal_records() {
        let dir = test_dir(); save_workflow_run(&dir, &full_run_record()).unwrap();
        assert!(!dir.join("workflow_proposals").exists());
    }

    #[test]
    fn workflow_execution_does_not_write_task_plan_records() {
        let dir = test_dir(); save_workflow_run(&dir, &full_run_record()).unwrap();
        assert!(!dir.join("task_plans").exists());
    }
}
