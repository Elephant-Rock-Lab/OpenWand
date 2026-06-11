//! Workflow reconciliation persistence.
//!
//! Two evidence areas:
//!   workflow_reconciliations/ — reconciliation records + indexes
//!   workflow_run_revisions/ — immutable run revision snapshots
//!
//! Does not mutate original workflow run records, route/outcome/readiness/proposal/
//! task-plan/governance/trace/memory/session/approval/tool/git/provider records.

use std::path::Path;

use openwand_workflow::workflow_reconciliation::{
    WorkflowReconciliationId, WorkflowReconciliationRecord, WorkflowReconciliationStatus,
    WorkflowRunRevision, WorkflowRunRevisionId,
};

fn reconciliations_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_reconciliations")
}
fn records_dir(store_root: &Path) -> std::path::PathBuf {
    reconciliations_root(store_root).join("records")
}
fn revisions_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_run_revisions")
}
fn revision_records_dir(store_root: &Path) -> std::path::PathBuf {
    revisions_root(store_root).join("records")
}

fn index_file(root: &Path, index_name: &str, key: &str) -> std::path::PathBuf {
    let dir = root.join(index_name);
    dir.join(format!("{}.json", key))
}

pub fn save_workflow_reconciliation(
    store_root: &Path,
    record: &WorkflowReconciliationRecord,
) -> Result<std::path::PathBuf, String> {
    let dir = records_dir(store_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Dir: {}", e))?;

    // Idempotency check
    if let Ok(existing) = list_workflow_reconciliations(store_root) {
        for er in &existing {
            if er.workflow_execution_id == record.workflow_execution_id
                && er.route_id == record.route_id
                && er.outcome_id == record.outcome_id
            {
                // Same content hash → return existing
                if er.reconciliation_id == record.reconciliation_id {
                    return Ok(dir.join(format!("{}.json", er.reconciliation_id.0)));
                }
                // Already reconciled → cannot duplicate
                if matches!(er.status, WorkflowReconciliationStatus::Reconciled)
                    && !matches!(record.status, WorkflowReconciliationStatus::Blocked)
                {
                    return Ok(dir.join(format!("{}.json", er.reconciliation_id.0)));
                }
            }
        }
    }

    let path = dir.join(format!("{}.json", record.reconciliation_id.0));
    let json = serde_json::to_string_pretty(record).map_err(|e| format!("Serialize: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Write: {}", e))?;

    // Latest pointer
    std::fs::write(dir.join("latest.json"), record.reconciliation_id.0.as_bytes())
        .map_err(|e| format!("Latest: {}", e))?;

    // Indexes
    let root = reconciliations_root(store_root);
    for (idx_name, key) in [
        ("by_workflow_run", record.workflow_execution_id.0.as_str()),
        ("by_route", record.route_id.0.as_str()),
        ("by_outcome", record.outcome_id.0.as_str()),
        ("by_stage", record.stage_id.as_str()),
        ("by_action_request", record.action_request_id.as_str()),
    ] {
        let idx_file = index_file(&root, idx_name, key);
        std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Index dir: {}", e))?;
        std::fs::write(&idx_file, record.reconciliation_id.0.as_bytes())
            .map_err(|e| format!("Index write: {}", e))?;
    }

    Ok(path)
}

pub fn load_workflow_reconciliation(store_root: &Path, id: &WorkflowReconciliationId) -> Result<WorkflowReconciliationRecord, String> {
    let path = records_dir(store_root).join(format!("{}.json", id.0));
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Parse: {}", e))
}

pub fn list_workflow_reconciliations(store_root: &Path) -> Result<Vec<WorkflowReconciliationRecord>, String> {
    let dir = records_dir(store_root);
    if !dir.exists() { return Ok(vec![]); }
    let mut records = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("Dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Entry: {}", e))?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            if name == "latest" { continue; }
            if let Ok(json) = std::fs::read_to_string(&path)
                && let Ok(record) = serde_json::from_str::<WorkflowReconciliationRecord>(&json) {
                    records.push(record);
                }
        }
    }
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(records)
}

pub fn latest_workflow_reconciliation(store_root: &Path) -> Result<Option<WorkflowReconciliationRecord>, String> {
    let p = records_dir(store_root).join("latest.json");
    if !p.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&p).map_err(|e| format!("{}", e))?;
    load_workflow_reconciliation(store_root, &WorkflowReconciliationId(id.trim().into())).map(Some)
}

pub fn reconciliation_by_workflow_run(store_root: &Path, id: &str) -> Result<Option<WorkflowReconciliationRecord>, String> {
    load_reconciliation_index(store_root, "by_workflow_run", id)
}
pub fn reconciliation_by_route(store_root: &Path, id: &str) -> Result<Option<WorkflowReconciliationRecord>, String> {
    load_reconciliation_index(store_root, "by_route", id)
}
pub fn reconciliation_by_outcome(store_root: &Path, id: &str) -> Result<Option<WorkflowReconciliationRecord>, String> {
    load_reconciliation_index(store_root, "by_outcome", id)
}
pub fn reconciliation_by_stage(store_root: &Path, id: &str) -> Result<Option<WorkflowReconciliationRecord>, String> {
    load_reconciliation_index(store_root, "by_stage", id)
}
pub fn reconciliation_by_action_request(store_root: &Path, id: &str) -> Result<Option<WorkflowReconciliationRecord>, String> {
    load_reconciliation_index(store_root, "by_action_request", id)
}

fn load_reconciliation_index(store_root: &Path, index_name: &str, key: &str) -> Result<Option<WorkflowReconciliationRecord>, String> {
    let pointer = index_file(&reconciliations_root(store_root), index_name, key);
    if !pointer.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&pointer).map_err(|e| format!("{}", e))?;
    load_workflow_reconciliation(store_root, &WorkflowReconciliationId(id.trim().into())).map(Some)
}

// --- Run Revision Persistence ---

pub fn save_workflow_run_revision(
    store_root: &Path,
    revision: &WorkflowRunRevision,
) -> Result<std::path::PathBuf, String> {
    let dir = revision_records_dir(store_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Dir: {}", e))?;

    let path = dir.join(format!("{}.json", revision.revision_id.0));
    let json = serde_json::to_string_pretty(revision).map_err(|e| format!("Serialize: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Write: {}", e))?;

    // Latest pointer
    std::fs::write(dir.join("latest.json"), revision.revision_id.0.as_bytes())
        .map_err(|e| format!("Latest: {}", e))?;

    // Indexes
    let root = revisions_root(store_root);
    for (idx_name, key) in [
        ("by_workflow_run", revision.workflow_execution_id.0.as_str()),
        ("by_reconciliation", revision.source_reconciliation_id.0.as_str()),
    ] {
        let idx_file = index_file(&root, idx_name, key);
        std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Index dir: {}", e))?;
        std::fs::write(&idx_file, revision.revision_id.0.as_bytes())
            .map_err(|e| format!("Index write: {}", e))?;
    }

    Ok(path)
}

pub fn load_workflow_run_revision(store_root: &Path, id: &WorkflowRunRevisionId) -> Result<WorkflowRunRevision, String> {
    let path = revision_records_dir(store_root).join(format!("{}.json", id.0));
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Parse: {}", e))
}

pub fn latest_workflow_run_revision(store_root: &Path) -> Result<Option<WorkflowRunRevision>, String> {
    let p = revision_records_dir(store_root).join("latest.json");
    if !p.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&p).map_err(|e| format!("{}", e))?;
    load_workflow_run_revision(store_root, &WorkflowRunRevisionId(id.trim().into())).map(Some)
}

pub fn run_revision_by_workflow_run(store_root: &Path, id: &str) -> Result<Option<WorkflowRunRevision>, String> {
    load_revision_index(store_root, "by_workflow_run", id)
}
pub fn run_revision_by_reconciliation(store_root: &Path, id: &str) -> Result<Option<WorkflowRunRevision>, String> {
    load_revision_index(store_root, "by_reconciliation", id)
}

fn load_revision_index(store_root: &Path, index_name: &str, key: &str) -> Result<Option<WorkflowRunRevision>, String> {
    let pointer = index_file(&revisions_root(store_root), index_name, key);
    if !pointer.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&pointer).map_err(|e| format!("{}", e))?;
    load_workflow_run_revision(store_root, &WorkflowRunRevisionId(id.trim().into())).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_reconciliation::*;
    use openwand_workflow::workflow_run::{WorkflowExecutionId, WorkflowStageRun, WorkflowStageRunStatus};
    use openwand_workflow::workflow_action_route::WorkflowActionRouteId;
    use openwand_workflow::workflow_action_outcome::WorkflowActionOutcomeId;
    use openwand_workflow::workflow_proposal::WorkflowStageKind;
    use chrono::Utc;

    fn test_dir() -> std::path::PathBuf { tempfile::tempdir().unwrap().into_path() }

    fn test_reconciliation(status: WorkflowReconciliationStatus, suffix: &str) -> WorkflowReconciliationRecord {
        WorkflowReconciliationRecord {
            reconciliation_id: WorkflowReconciliationId(format!("wrc_{}", suffix)),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            route_id: WorkflowActionRouteId("war_t".into()),
            outcome_id: WorkflowActionOutcomeId("wao_t".into()),
            stage_id: "stage_1".into(), action_request_id: "ar_1".into(),
            status: status.clone(),
            decision: match &status {
                WorkflowReconciliationStatus::Reconciled => WorkflowReconciliationDecision::Reconciled { summary: "ok".into() },
                WorkflowReconciliationStatus::Blocked => WorkflowReconciliationDecision::Blocked { reason_code: "test".into(), summary: "blocked".into() },
                _ => WorkflowReconciliationDecision::Failed { reason_code: "test".into(), summary: "failed".into() },
            },
            predicates: vec![], progression: None, new_run_revision_id: None,
            created_at: Utc::now(),
        }
    }

    fn test_stage(id: &str, status: WorkflowStageRunStatus) -> WorkflowStageRun {
        WorkflowStageRun {
            stage_id: id.into(), title: format!("Stage {}", id), kind: WorkflowStageKind::ApplyChange,
            status, order: 0, depends_on: vec![], started_at: None, completed_at: None, summary: "test".into(),
        }
    }

    fn test_revision(suffix: &str) -> WorkflowRunRevision {
        WorkflowRunRevision {
            revision_id: WorkflowRunRevisionId(format!("wrr_{}", suffix)),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            previous_revision_id: None,
            source_reconciliation_id: WorkflowReconciliationId(format!("wrc_{}", suffix)),
            run_hash_before: "h1".into(), run_hash_after: "h2".into(),
            stages: vec![test_stage("s1", WorkflowStageRunStatus::Completed)],
            lifecycle_events: vec![], aggregate_status: None,
            created_at: Utc::now(),
        }
    }

    #[test] fn reconciliation_persists_and_loads_roundtrip() {
        let d = test_dir(); let r = test_reconciliation(WorkflowReconciliationStatus::Reconciled, "rt");
        save_workflow_reconciliation(&d, &r).unwrap();
        let l = load_workflow_reconciliation(&d, &r.reconciliation_id).unwrap();
        assert_eq!(r.reconciliation_id, l.reconciliation_id);
    }
    #[test] fn latest_reconciliation_returns_expected() {
        let d = test_dir(); let r = test_reconciliation(WorkflowReconciliationStatus::Reconciled, "lt");
        save_workflow_reconciliation(&d, &r).unwrap();
        assert_eq!(r.reconciliation_id, latest_workflow_reconciliation(&d).unwrap().unwrap().reconciliation_id);
    }
    #[test] fn reconciliation_by_workflow_run_returns_expected() {
        let d = test_dir(); let r = test_reconciliation(WorkflowReconciliationStatus::Reconciled, "wr");
        save_workflow_reconciliation(&d, &r).unwrap();
        assert_eq!(r.reconciliation_id, reconciliation_by_workflow_run(&d, "wfx_t").unwrap().unwrap().reconciliation_id);
    }
    #[test] fn reconciliation_by_route_returns_expected() {
        let d = test_dir(); let r = test_reconciliation(WorkflowReconciliationStatus::Reconciled, "br");
        save_workflow_reconciliation(&d, &r).unwrap();
        assert_eq!(r.reconciliation_id, reconciliation_by_route(&d, "war_t").unwrap().unwrap().reconciliation_id);
    }
    #[test] fn reconciliation_by_outcome_returns_expected() {
        let d = test_dir(); let r = test_reconciliation(WorkflowReconciliationStatus::Reconciled, "bo");
        save_workflow_reconciliation(&d, &r).unwrap();
        assert_eq!(r.reconciliation_id, reconciliation_by_outcome(&d, "wao_t").unwrap().unwrap().reconciliation_id);
    }
    #[test] fn reconciliation_by_stage_returns_expected() {
        let d = test_dir(); let r = test_reconciliation(WorkflowReconciliationStatus::Reconciled, "st");
        save_workflow_reconciliation(&d, &r).unwrap();
        assert_eq!(r.reconciliation_id, reconciliation_by_stage(&d, "stage_1").unwrap().unwrap().reconciliation_id);
    }
    #[test] fn reconciliation_by_action_request_returns_expected() {
        let d = test_dir(); let r = test_reconciliation(WorkflowReconciliationStatus::Reconciled, "ar");
        save_workflow_reconciliation(&d, &r).unwrap();
        assert_eq!(r.reconciliation_id, reconciliation_by_action_request(&d, "ar_1").unwrap().unwrap().reconciliation_id);
    }
    #[test] fn run_revision_persists_and_loads_roundtrip() {
        let d = test_dir(); let r = test_revision("rt");
        save_workflow_run_revision(&d, &r).unwrap();
        let l = load_workflow_run_revision(&d, &r.revision_id).unwrap();
        assert_eq!(r.revision_id, l.revision_id);
    }
    #[test] fn latest_run_revision_returns_expected() {
        let d = test_dir(); let r = test_revision("lt");
        save_workflow_run_revision(&d, &r).unwrap();
        assert_eq!(r.revision_id, latest_workflow_run_revision(&d).unwrap().unwrap().revision_id);
    }
    #[test] fn run_revision_by_workflow_run_returns_expected() {
        let d = test_dir(); let r = test_revision("wr");
        save_workflow_run_revision(&d, &r).unwrap();
        assert_eq!(r.revision_id, run_revision_by_workflow_run(&d, "wfx_t").unwrap().unwrap().revision_id);
    }
    #[test] fn run_revision_by_reconciliation_returns_expected() {
        let d = test_dir(); let r = test_revision("rc");
        save_workflow_run_revision(&d, &r).unwrap();
        assert_eq!(r.revision_id, run_revision_by_reconciliation(&d, "wrc_rc").unwrap().unwrap().revision_id);
    }
    #[test] fn same_idempotency_key_returns_existing_reconciliation() {
        let d = test_dir(); let r = test_reconciliation(WorkflowReconciliationStatus::Reconciled, "id");
        let p1 = save_workflow_reconciliation(&d, &r).unwrap();
        let p2 = save_workflow_reconciliation(&d, &r).unwrap();
        assert_eq!(p1, p2);
    }
    #[test] fn blocked_reconciliation_can_retry_with_new_key() {
        let d = test_dir();
        let r1 = test_reconciliation(WorkflowReconciliationStatus::Blocked, "bk1");
        save_workflow_reconciliation(&d, &r1).unwrap();
        let mut r2 = test_reconciliation(WorkflowReconciliationStatus::Reconciled, "bk2");
        r2.workflow_execution_id = r1.workflow_execution_id.clone();
        r2.route_id = r1.route_id.clone();
        r2.outcome_id = r1.outcome_id.clone();
        let p2 = save_workflow_reconciliation(&d, &r2).unwrap();
        assert!(p2.exists());
    }
    #[test] fn reconciled_cannot_duplicate_with_different_key() {
        let d = test_dir();
        let r1 = test_reconciliation(WorkflowReconciliationStatus::Reconciled, "rd1");
        save_workflow_reconciliation(&d, &r1).unwrap();
        let mut r2 = test_reconciliation(WorkflowReconciliationStatus::Reconciled, "rd2");
        r2.workflow_execution_id = r1.workflow_execution_id.clone();
        r2.route_id = r1.route_id.clone();
        r2.outcome_id = r1.outcome_id.clone();
        let p2 = save_workflow_reconciliation(&d, &r2).unwrap();
        assert!(p2.to_string_lossy().contains("rd1"));
    }
    #[test] fn writes_only_reconciliation_and_revision_evidence() {
        let d = test_dir();
        save_workflow_reconciliation(&d, &test_reconciliation(WorkflowReconciliationStatus::Reconciled, "wo")).unwrap();
        save_workflow_run_revision(&d, &test_revision("wo")).unwrap();
        assert!(reconciliations_root(&d).exists());
        assert!(revisions_root(&d).exists());
        assert!(!d.join("workflow_runs").exists());
        assert!(!d.join("workflow_action_routes").exists());
        assert!(!d.join("workflow_action_outcomes").exists());
        assert!(!d.join("task_plans").exists());
        assert!(!d.join("approvals").exists());
    }
    #[test] fn reconciliation_does_not_mutate_original_workflow_run_record() {
        let d = test_dir();
        save_workflow_reconciliation(&d, &test_reconciliation(WorkflowReconciliationStatus::Reconciled, "nr")).unwrap();
        assert!(!d.join("workflow_runs").exists());
    }
    #[test] fn reconciliation_does_not_write_route_or_outcome_records() {
        let d = test_dir();
        save_workflow_reconciliation(&d, &test_reconciliation(WorkflowReconciliationStatus::Reconciled, "no")).unwrap();
        assert!(!d.join("workflow_action_routes").exists());
        assert!(!d.join("workflow_action_outcomes").exists());
    }
    #[test] fn reconciliation_does_not_write_approval_or_session_records() {
        let d = test_dir();
        save_workflow_reconciliation(&d, &test_reconciliation(WorkflowReconciliationStatus::Reconciled, "na")).unwrap();
        assert!(!d.join("approvals").exists());
        assert!(!d.join("sessions").exists());
        assert!(!d.join("session_state").exists());
    }
}
