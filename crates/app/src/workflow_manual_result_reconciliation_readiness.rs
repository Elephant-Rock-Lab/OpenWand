//! Manual result reconciliation readiness persistence.
//!
//! Writes only readiness evidence. Does not mutate manual result records,
//! manual result reviews, command reviews, command-composer records,
//! loop-controller records, workflow runs, revisions, routes, outcomes,
//! reconciliations, continuation, reviews, readiness, routing, approvals,
//! sessions, trace, memory, tools, git/provider, or governance records.

use std::path::Path;

use openwand_workflow::workflow_manual_result_reconciliation_readiness::*;

fn readiness_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_manual_result_reconciliation_readiness")
}
fn records_dir(store_root: &Path) -> std::path::PathBuf {
    readiness_root(store_root).join("records")
}
fn index_file(root: &Path, index_name: &str, key: &str) -> std::path::PathBuf {
    root.join(index_name).join(format!("{}.json", key))
}

pub fn save_reconciliation_readiness(
    store_root: &Path,
    record: &WorkflowManualResultReconciliationReadinessRecord,
) -> Result<std::path::PathBuf, String> {
    let dir = records_dir(store_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Dir: {}", e))?;

    // Idempotency
    if let Ok(existing) = list_reconciliation_readiness(store_root) {
        for er in &existing {
            if er.readiness_id == record.readiness_id {
                return Ok(dir.join(format!("{}.json", er.readiness_id.0)));
            }
            // Patch 5: Ready cannot duplicate for same accepted review with different key
            if er.manual_result_review_id == record.manual_result_review_id
                && matches!(er.status, WorkflowManualResultReconciliationReadinessStatus::Ready)
                && matches!(record.status, WorkflowManualResultReconciliationReadinessStatus::Ready)
                && er.readiness_id != record.readiness_id {
                return Ok(dir.join(format!("{}.json", er.readiness_id.0)));
            }
        }
    }

    let path = dir.join(format!("{}.json", record.readiness_id.0));
    let json = serde_json::to_string_pretty(record).map_err(|e| format!("Serialize: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Write: {}", e))?;

    std::fs::write(dir.join("latest.json"), record.readiness_id.0.as_bytes())
        .map_err(|e| format!("Latest: {}", e))?;

    // Patch 5: source-chain indexes
    let root = readiness_root(store_root);
    for (idx_name, key) in [
        ("by_manual_result", record.manual_result_id.0.as_str()),
        ("by_manual_result_review", record.manual_result_review_id.0.as_str()),
        ("by_workflow_run", record.workflow_execution_id.0.as_str()),
        ("by_command_review", record.command_review_id.0.as_str()),
        ("by_command_composer", record.command_composer_id.0.as_str()),
        ("by_loop_controller", record.loop_controller_id.0.as_str()),
    ] {
        let idx_file = index_file(&root, idx_name, key);
        std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Index dir: {}", e))?;
        std::fs::write(&idx_file, record.readiness_id.0.as_bytes()).map_err(|e| format!("Index: {}", e))?;
    }

    Ok(path)
}

pub fn load_reconciliation_readiness(store_root: &Path, id: &WorkflowManualResultReconciliationReadinessId) -> Result<WorkflowManualResultReconciliationReadinessRecord, String> {
    let path = records_dir(store_root).join(format!("{}.json", id.0));
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Parse: {}", e))
}

pub fn list_reconciliation_readiness(store_root: &Path) -> Result<Vec<WorkflowManualResultReconciliationReadinessRecord>, String> {
    let dir = records_dir(store_root);
    if !dir.exists() { return Ok(vec![]); }
    let mut records = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("Dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Entry: {}", e))?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "json") {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            if name == "latest" { continue; }
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(record) = serde_json::from_str::<WorkflowManualResultReconciliationReadinessRecord>(&json) {
                    records.push(record);
                }
            }
        }
    }
    records.sort_by(|a, b| b.evaluated_at.cmp(&a.evaluated_at));
    Ok(records)
}

pub fn latest_reconciliation_readiness(store_root: &Path) -> Result<Option<WorkflowManualResultReconciliationReadinessRecord>, String> {
    let p = records_dir(store_root).join("latest.json");
    if !p.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&p).map_err(|e| format!("{}", e))?;
    load_reconciliation_readiness(store_root, &WorkflowManualResultReconciliationReadinessId(id.trim().into())).map(Some)
}

pub fn readiness_by_manual_result(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReconciliationReadinessRecord>, String> {
    load_index(store_root, "by_manual_result", id)
}
pub fn readiness_by_manual_result_review(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReconciliationReadinessRecord>, String> {
    load_index(store_root, "by_manual_result_review", id)
}
pub fn readiness_by_workflow_run(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReconciliationReadinessRecord>, String> {
    load_index(store_root, "by_workflow_run", id)
}
pub fn readiness_by_command_review(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReconciliationReadinessRecord>, String> {
    load_index(store_root, "by_command_review", id)
}
pub fn readiness_by_command_composer(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReconciliationReadinessRecord>, String> {
    load_index(store_root, "by_command_composer", id)
}
pub fn readiness_by_loop_controller(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReconciliationReadinessRecord>, String> {
    load_index(store_root, "by_loop_controller", id)
}

fn load_index(store_root: &Path, index_name: &str, key: &str) -> Result<Option<WorkflowManualResultReconciliationReadinessRecord>, String> {
    let pointer = index_file(&readiness_root(store_root), index_name, key);
    if !pointer.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&pointer).map_err(|e| format!("{}", e))?;
    load_reconciliation_readiness(store_root, &WorkflowManualResultReconciliationReadinessId(id.trim().into())).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_manual_result_reconciliation_readiness::*;
    use openwand_workflow::workflow_manual_result::WorkflowManualResultId;
    use openwand_workflow::workflow_manual_result_review::WorkflowManualResultReviewId;
    use openwand_workflow::workflow_command_review::WorkflowCommandReviewId;
    use openwand_workflow::workflow_command_composer::WorkflowCommandComposerId;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use chrono::Utc;

    fn test_dir() -> std::path::PathBuf { tempfile::tempdir().unwrap().into_path() }

    fn test_record(status: WorkflowManualResultReconciliationReadinessStatus, suffix: &str) -> WorkflowManualResultReconciliationReadinessRecord {
        let decision = match &status {
            WorkflowManualResultReconciliationReadinessStatus::Ready =>
                WorkflowManualResultReconciliationReadinessDecision::Ready { summary: "ok".into() },
            WorkflowManualResultReconciliationReadinessStatus::Blocked =>
                WorkflowManualResultReconciliationReadinessDecision::Blocked { reason_code: "test".into(), summary: "blocked".into() },
            WorkflowManualResultReconciliationReadinessStatus::Inconclusive =>
                WorkflowManualResultReconciliationReadinessDecision::Inconclusive { reason_code: "test".into(), summary: "inconclusive".into() },
        };
        WorkflowManualResultReconciliationReadinessRecord {
            readiness_id: WorkflowManualResultReconciliationReadinessId(format!("wmrrr_{}", suffix)),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            manual_result_id: WorkflowManualResultId("wmr_t".into()),
            manual_result_review_id: WorkflowManualResultReviewId("wmrr_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            manual_result_review_hash: "rrh".into(), manual_result_hash: "mrh".into(),
            command_review_hash: "crh".into(), command_composer_hash: "cch".into(),
            command_descriptor_hash: "cdh".into(), loop_controller_hash: "lch".into(),
            status, decision, predicates: vec![],
            reconciliation_preview: None,
            verifies_external_state: false, reconciles_now: false,
            mutates_workflow_state: false, creates_run_revision: false,
            appends_trace: false, writes_memory: false,
            routes_action: false, resolves_approval: false,
            creates_execution_grant: false, execution_allowed_now: false,
            evaluator: "test".into(), evaluated_at: Utc::now(),
        }
    }

    #[test] fn readiness_persists_and_loads_roundtrip() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationReadinessStatus::Ready, "rt");
        save_reconciliation_readiness(&d, &r).unwrap();
        let l = load_reconciliation_readiness(&d, &r.readiness_id).unwrap();
        assert_eq!(r.readiness_id, l.readiness_id);
    }
    #[test] fn latest_readiness_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationReadinessStatus::Ready, "lt");
        save_reconciliation_readiness(&d, &r).unwrap();
        assert_eq!(r.readiness_id, latest_reconciliation_readiness(&d).unwrap().unwrap().readiness_id);
    }
    #[test] fn readiness_by_manual_result_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationReadinessStatus::Ready, "mr");
        save_reconciliation_readiness(&d, &r).unwrap();
        assert_eq!(r.readiness_id, readiness_by_manual_result(&d, "wmr_t").unwrap().unwrap().readiness_id);
    }
    #[test] fn readiness_by_manual_result_review_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationReadinessStatus::Ready, "rv");
        save_reconciliation_readiness(&d, &r).unwrap();
        assert_eq!(r.readiness_id, readiness_by_manual_result_review(&d, "wmrr_t").unwrap().unwrap().readiness_id);
    }
    #[test] fn readiness_by_workflow_run_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationReadinessStatus::Ready, "wr");
        save_reconciliation_readiness(&d, &r).unwrap();
        assert_eq!(r.readiness_id, readiness_by_workflow_run(&d, "wfx_t").unwrap().unwrap().readiness_id);
    }
    #[test] fn readiness_by_command_review_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationReadinessStatus::Ready, "cr");
        save_reconciliation_readiness(&d, &r).unwrap();
        assert_eq!(r.readiness_id, readiness_by_command_review(&d, "wcrv_t").unwrap().unwrap().readiness_id);
    }
    #[test] fn readiness_by_command_composer_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationReadinessStatus::Ready, "cc");
        save_reconciliation_readiness(&d, &r).unwrap();
        assert_eq!(r.readiness_id, readiness_by_command_composer(&d, "wcc_t").unwrap().unwrap().readiness_id);
    }
    #[test] fn readiness_by_loop_controller_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationReadinessStatus::Ready, "lc");
        save_reconciliation_readiness(&d, &r).unwrap();
        assert_eq!(r.readiness_id, readiness_by_loop_controller(&d, "wlc_t").unwrap().unwrap().readiness_id);
    }
    // Patch 5: idempotency
    #[test] fn same_idempotency_key_returns_existing() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationReadinessStatus::Ready, "id");
        let p1 = save_reconciliation_readiness(&d, &r).unwrap();
        let p2 = save_reconciliation_readiness(&d, &r).unwrap();
        assert_eq!(p1, p2);
    }
    #[test] fn ready_cannot_duplicate_with_different_key() {
        let d = test_dir();
        let r1 = test_record(WorkflowManualResultReconciliationReadinessStatus::Ready, "dp1");
        save_reconciliation_readiness(&d, &r1).unwrap();
        let r2 = test_record(WorkflowManualResultReconciliationReadinessStatus::Ready, "dp2");
        let p2 = save_reconciliation_readiness(&d, &r2).unwrap();
        assert!(p2.to_string_lossy().contains("dp1"), "Should return existing");
    }
    #[test] fn blocked_may_retry_with_new_key() {
        let d = test_dir();
        let r1 = test_record(WorkflowManualResultReconciliationReadinessStatus::Blocked, "bl1");
        save_reconciliation_readiness(&d, &r1).unwrap();
        let r2 = test_record(WorkflowManualResultReconciliationReadinessStatus::Ready, "bl2");
        let p2 = save_reconciliation_readiness(&d, &r2).unwrap();
        assert!(p2.exists(), "New ready creates new record");
    }
    #[test] fn readiness_writes_only_readiness_evidence() {
        let d = test_dir();
        save_reconciliation_readiness(&d, &test_record(WorkflowManualResultReconciliationReadinessStatus::Ready, "wo")).unwrap();
        assert!(readiness_root(&d).exists());
        assert!(!d.join("workflow_manual_results").exists());
        assert!(!d.join("workflow_manual_result_reviews").exists());
        assert!(!d.join("workflow_runs").exists());
    }
    #[test] fn readiness_does_not_write_reconciliation_records() {
        let d = test_dir();
        save_reconciliation_readiness(&d, &test_record(WorkflowManualResultReconciliationReadinessStatus::Ready, "nr")).unwrap();
        assert!(!d.join("workflow_reconciliations").exists());
    }
    #[test] fn readiness_does_not_write_approval_or_session_records() {
        let d = test_dir();
        save_reconciliation_readiness(&d, &test_record(WorkflowManualResultReconciliationReadinessStatus::Ready, "ns")).unwrap();
        assert!(!d.join("approvals").exists());
        assert!(!d.join("sessions").exists());
    }
}
