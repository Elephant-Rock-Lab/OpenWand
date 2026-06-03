//! Workflow readiness persistence.
//!
//! Readiness records are durable evidence stored as JSON files
//! under eval_reports/workflow_readiness/. No workflow run, execution queue,
//! scheduler state, approval request, or worker state is created.

use std::path::Path;

use openwand_workflow::plan::TaskPlanId;
use openwand_workflow::workflow_proposal::WorkflowProposalId;
use openwand_workflow::workflow_proposal_review::WorkflowProposalReviewId;
use openwand_workflow::workflow_readiness::{WorkflowReadinessId, WorkflowReadinessRecord, WorkflowReadinessStatus};

/// Root directory for workflow readiness evidence.
fn workflow_readiness_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_readiness")
}

fn records_dir(store_root: &Path) -> std::path::PathBuf {
    workflow_readiness_root(store_root).join("records")
}

fn by_proposal_dir(store_root: &Path) -> std::path::PathBuf {
    workflow_readiness_root(store_root).join("by_proposal")
}

fn by_review_dir(store_root: &Path) -> std::path::PathBuf {
    workflow_readiness_root(store_root).join("by_review")
}

fn by_task_plan_dir(store_root: &Path) -> std::path::PathBuf {
    workflow_readiness_root(store_root).join("by_task_plan")
}

/// Save a workflow readiness record to disk.
///
/// Idempotency: same (proposal_id, review_id, idempotency_key) returns existing record.
/// Ready records cannot duplicate for same proposal/review, even with different key.
/// Blocked/Inconclusive may retry with new key.
pub fn save_workflow_readiness(
    store_root: &Path,
    record: &WorkflowReadinessRecord,
) -> Result<std::path::PathBuf, String> {
    let dir = records_dir(store_root);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create workflow_readiness/records dir: {}", e))?;

    // Idempotency check: same proposal + review + key → return existing
    let existing = find_existing_readiness(store_root, &record.proposal_id, &record.review_id);
    if let Some(existing) = &existing {
        if let Ok(existing_records) = list_workflow_readiness(store_root) {
            for er in &existing_records {
                if er.proposal_id == record.proposal_id
                    && er.review_id == record.review_id
                {
                    // Same key → return existing path
                    if let Ok(req) = find_idempotency_key_for_record(store_root, &er.readiness_id) {
                        if req == find_idempotency_key_for_record(store_root, &record.readiness_id).unwrap_or_default() {
                            let path = dir.join(format!("{}.json", er.readiness_id.0));
                            return Ok(path);
                        }
                    }
                    // Ready cannot duplicate even with different key
                    if er.status == WorkflowReadinessStatus::Ready
                        && record.status == WorkflowReadinessStatus::Ready
                    {
                        let path = dir.join(format!("{}.json", er.readiness_id.0));
                        return Ok(path);
                    }
                }
            }
        }
    }

    // Save the record
    let path = dir.join(format!("{}.json", record.readiness_id.0));
    let json = serde_json::to_string_pretty(record)
        .map_err(|e| format!("Failed to serialize readiness record: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write readiness record: {}", e))?;

    // Update latest pointer
    let latest_path = dir.join("latest.json");
    std::fs::write(&latest_path, record.readiness_id.0.as_bytes())
        .map_err(|e| format!("Failed to write latest pointer: {}", e))?;

    // Update by_proposal index
    let bp_dir = by_proposal_dir(store_root);
    std::fs::create_dir_all(&bp_dir)
        .map_err(|e| format!("Failed to create by_proposal dir: {}", e))?;
    let bp_path = bp_dir.join(format!("{}.json", record.proposal_id.0));
    std::fs::write(&bp_path, record.readiness_id.0.as_bytes())
        .map_err(|e| format!("Failed to write by_proposal pointer: {}", e))?;

    // Update by_review index
    let br_dir = by_review_dir(store_root);
    std::fs::create_dir_all(&br_dir)
        .map_err(|e| format!("Failed to create by_review dir: {}", e))?;
    let br_path = br_dir.join(format!("{}.json", record.review_id.0));
    std::fs::write(&br_path, record.readiness_id.0.as_bytes())
        .map_err(|e| format!("Failed to write by_review pointer: {}", e))?;

    // Update by_task_plan index
    let tp_dir = by_task_plan_dir(store_root);
    std::fs::create_dir_all(&tp_dir)
        .map_err(|e| format!("Failed to create by_task_plan dir: {}", e))?;
    let tp_path = tp_dir.join(format!("{}.json", record.source_task_plan_id.0));
    std::fs::write(&tp_path, record.readiness_id.0.as_bytes())
        .map_err(|e| format!("Failed to write by_task_plan pointer: {}", e))?;

    Ok(path)
}

fn find_existing_readiness(
    store_root: &Path,
    proposal_id: &WorkflowProposalId,
    review_id: &WorkflowProposalReviewId,
) -> Option<()> {
    // Check if any record exists for this proposal+review
    if let Ok(records) = list_workflow_readiness(store_root) {
        for r in &records {
            if r.proposal_id == *proposal_id && r.review_id == *review_id {
                return Some(());
            }
        }
    }
    None
}

fn find_idempotency_key_for_record(store_root: &Path, readiness_id: &WorkflowReadinessId) -> Result<String, String> {
    // Idempotency key is embedded in the readiness ID computation
    // For simplicity, we store it alongside. But since readiness records
    // don't store the key directly, we use the readiness_id as unique.
    Ok(readiness_id.0.clone())
}

/// Load a workflow readiness record by ID.
pub fn load_workflow_readiness(
    store_root: &Path,
    readiness_id: &WorkflowReadinessId,
) -> Result<WorkflowReadinessRecord, String> {
    let path = records_dir(store_root).join(format!("{}.json", readiness_id.0));
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read readiness {}: {}", readiness_id.0, e))?;
    serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse readiness {}: {}", readiness_id.0, e))
}

/// List all workflow readiness records sorted by created_at descending.
pub fn list_workflow_readiness(store_root: &Path) -> Result<Vec<WorkflowReadinessRecord>, String> {
    let dir = records_dir(store_root);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut records = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("Failed to read readiness dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "json") {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            if name == "latest" {
                continue;
            }
            let json = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
            if let Ok(record) = serde_json::from_str::<WorkflowReadinessRecord>(&json) {
                records.push(record);
            }
        }
    }
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(records)
}

/// Get the latest workflow readiness record.
pub fn latest_workflow_readiness(store_root: &Path) -> Result<Option<WorkflowReadinessRecord>, String> {
    let latest_path = records_dir(store_root).join("latest.json");
    if !latest_path.exists() {
        return Ok(None);
    }
    let id_str = std::fs::read_to_string(&latest_path)
        .map_err(|e| format!("Failed to read latest pointer: {}", e))?;
    let readiness_id = WorkflowReadinessId(id_str.trim().to_string());
    load_workflow_readiness(store_root, &readiness_id).map(Some)
}

/// Get readiness record for a specific proposal.
pub fn workflow_readiness_by_proposal(
    store_root: &Path,
    proposal_id: &WorkflowProposalId,
) -> Result<Option<WorkflowReadinessRecord>, String> {
    let pointer_path = by_proposal_dir(store_root).join(format!("{}.json", proposal_id.0));
    if !pointer_path.exists() {
        return Ok(None);
    }
    let id_str = std::fs::read_to_string(&pointer_path)
        .map_err(|e| format!("Failed to read by_proposal pointer: {}", e))?;
    let readiness_id = WorkflowReadinessId(id_str.trim().to_string());
    load_workflow_readiness(store_root, &readiness_id).map(Some)
}

/// Get readiness record for a specific review.
pub fn workflow_readiness_by_review(
    store_root: &Path,
    review_id: &WorkflowProposalReviewId,
) -> Result<Option<WorkflowReadinessRecord>, String> {
    let pointer_path = by_review_dir(store_root).join(format!("{}.json", review_id.0));
    if !pointer_path.exists() {
        return Ok(None);
    }
    let id_str = std::fs::read_to_string(&pointer_path)
        .map_err(|e| format!("Failed to read by_review pointer: {}", e))?;
    let readiness_id = WorkflowReadinessId(id_str.trim().to_string());
    load_workflow_readiness(store_root, &readiness_id).map(Some)
}

/// Get readiness record for a specific task plan.
pub fn workflow_readiness_by_task_plan(
    store_root: &Path,
    task_plan_id: &TaskPlanId,
) -> Result<Option<WorkflowReadinessRecord>, String> {
    let pointer_path = by_task_plan_dir(store_root).join(format!("{}.json", task_plan_id.0));
    if !pointer_path.exists() {
        return Ok(None);
    }
    let id_str = std::fs::read_to_string(&pointer_path)
        .map_err(|e| format!("Failed to read by_task_plan pointer: {}", e))?;
    let readiness_id = WorkflowReadinessId(id_str.trim().to_string());
    load_workflow_readiness(store_root, &readiness_id).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::builder::build_task_plan;
    use openwand_workflow::context::TaskPlanInput;
    use openwand_workflow::plan_review::{TaskPlanReview, TaskPlanReviewDecision, task_review_id_for};
    use openwand_workflow::workflow_proposal_builder::{WorkflowProposalInput, build_workflow_proposal};
    use openwand_workflow::workflow_proposal_review::{WorkflowProposalReview, WorkflowProposalReviewDecision, workflow_review_id_for};
    use openwand_workflow::workflow_readiness::{WorkflowReadinessRequest, WorkflowEnvironmentSnapshot, WorkflowRollbackAbortSnapshot};
    use openwand_workflow::workflow_readiness_evaluator::{WorkflowReadinessContext, evaluate_workflow_readiness};
    use chrono::Utc;

    fn test_dir() -> std::path::PathBuf {
        tempfile::tempdir().unwrap().into_path()
    }

    fn full_ready_record() -> WorkflowReadinessRecord {
        let plan = build_task_plan(&TaskPlanInput {
            user_intent: "Readiness persistence test".into(),
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
            review: Some(proposal_review.clone()),
            latest_review_for_proposal: Some(proposal_review),
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
    fn workflow_readiness_persists_and_loads_roundtrip() {
        let dir = test_dir();
        let record = full_ready_record();
        save_workflow_readiness(&dir, &record).unwrap();
        let loaded = load_workflow_readiness(&dir, &record.readiness_id).unwrap();
        assert_eq!(record.readiness_id, loaded.readiness_id);
        assert_eq!(record.proposal_hash, loaded.proposal_hash);
    }

    #[test]
    fn latest_workflow_readiness_returns_expected() {
        let dir = test_dir();
        let record = full_ready_record();
        save_workflow_readiness(&dir, &record).unwrap();
        let latest = latest_workflow_readiness(&dir).unwrap().unwrap();
        assert_eq!(record.readiness_id, latest.readiness_id);
    }

    #[test]
    fn workflow_readiness_by_proposal_returns_expected() {
        let dir = test_dir();
        let record = full_ready_record();
        save_workflow_readiness(&dir, &record).unwrap();
        let found = workflow_readiness_by_proposal(&dir, &record.proposal_id)
            .unwrap().unwrap();
        assert_eq!(record.readiness_id, found.readiness_id);
    }

    #[test]
    fn workflow_readiness_by_review_returns_expected() {
        let dir = test_dir();
        let record = full_ready_record();
        save_workflow_readiness(&dir, &record).unwrap();
        let found = workflow_readiness_by_review(&dir, &record.review_id)
            .unwrap().unwrap();
        assert_eq!(record.readiness_id, found.readiness_id);
    }

    #[test]
    fn workflow_readiness_by_task_plan_returns_expected() {
        let dir = test_dir();
        let record = full_ready_record();
        save_workflow_readiness(&dir, &record).unwrap();
        let found = workflow_readiness_by_task_plan(&dir, &record.source_task_plan_id)
            .unwrap().unwrap();
        assert_eq!(record.readiness_id, found.readiness_id);
    }

    #[test]
    fn same_idempotency_key_returns_existing_readiness() {
        let dir = test_dir();
        let record = full_ready_record();
        let path1 = save_workflow_readiness(&dir, &record).unwrap();
        let path2 = save_workflow_readiness(&dir, &record).unwrap();
        assert_eq!(path1, path2);
    }

    #[test]
    fn blocked_readiness_can_retry_with_new_key() {
        let dir = test_dir();
        let mut record = full_ready_record();
        // Make it blocked
        record.status = WorkflowReadinessStatus::Blocked;
        record.decision = openwand_workflow::workflow_readiness::WorkflowReadinessDecision::Blocked {
            reason_code: "test".into(),
            summary: "test blocked".into(),
        };
        // First save with key1
        let path1 = save_workflow_readiness(&dir, &record).unwrap();
        assert!(path1.exists());
        // Different readiness_id for different key → new record
        record.readiness_id = WorkflowReadinessId("wfrd_newkey".into());
        let path2 = save_workflow_readiness(&dir, &record).unwrap();
        assert!(path2.exists());
    }

    #[test]
    fn inconclusive_readiness_can_retry_with_new_key() {
        let dir = test_dir();
        let mut record = full_ready_record();
        record.status = WorkflowReadinessStatus::Inconclusive;
        record.decision = openwand_workflow::workflow_readiness::WorkflowReadinessDecision::Inconclusive {
            reason_code: "test".into(),
            summary: "test inconclusive".into(),
        };
        let path1 = save_workflow_readiness(&dir, &record).unwrap();
        record.readiness_id = WorkflowReadinessId("wfrd_inc2".into());
        let path2 = save_workflow_readiness(&dir, &record).unwrap();
        assert!(path2.exists());
    }

    #[test]
    fn ready_readiness_cannot_duplicate_for_same_proposal_review() {
        let dir = test_dir();
        let record = full_ready_record();
        let path1 = save_workflow_readiness(&dir, &record).unwrap();
        // Same proposal+review → same path (idempotent)
        let path2 = save_workflow_readiness(&dir, &record).unwrap();
        assert_eq!(path1, path2);
    }

    #[test]
    fn ready_readiness_cannot_duplicate_with_different_key() {
        let dir = test_dir();
        let record = full_ready_record();
        let path1 = save_workflow_readiness(&dir, &record).unwrap();
        // Even with different readiness_id, Ready + same proposal/review → same
        let mut record2 = record.clone();
        record2.readiness_id = WorkflowReadinessId("wfrd_different_key".into());
        let path2 = save_workflow_readiness(&dir, &record2).unwrap();
        assert_eq!(path1, path2);
    }

    #[test]
    fn workflow_readiness_writes_only_readiness_evidence() {
        let dir = test_dir();
        let record = full_ready_record();
        save_workflow_readiness(&dir, &record).unwrap();
        assert!(workflow_readiness_root(&dir).exists());
        assert!(!dir.join("workflow_proposals").exists());
        assert!(!dir.join("task_plans").exists());
        assert!(!dir.join("trace.db").exists());
    }

    #[test]
    fn workflow_readiness_does_not_write_workflow_proposal_records() {
        let dir = test_dir();
        let record = full_ready_record();
        save_workflow_readiness(&dir, &record).unwrap();
        assert!(!dir.join("workflow_proposals").exists());
    }

    #[test]
    fn workflow_readiness_does_not_write_task_plan_records() {
        let dir = test_dir();
        let record = full_ready_record();
        save_workflow_readiness(&dir, &record).unwrap();
        assert!(!dir.join("task_plans").exists());
    }

    #[test]
    fn same_idempotency_key_returns_existing_inconclusive_readiness() {
        let dir = test_dir();
        let mut record = full_ready_record();
        record.status = WorkflowReadinessStatus::Inconclusive;
        record.decision = openwand_workflow::workflow_readiness::WorkflowReadinessDecision::Inconclusive {
            reason_code: "test".into(),
            summary: "test inconclusive".into(),
        };
        let path1 = save_workflow_readiness(&dir, &record).unwrap();
        // Same readiness_id → same path
        let path2 = save_workflow_readiness(&dir, &record).unwrap();
        assert_eq!(path1, path2);
    }
}
