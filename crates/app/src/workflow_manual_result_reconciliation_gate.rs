//! Manual result reconciliation gate persistence.
//!
//! Writes gate records and run revisions only.
//! Patch 7: AlreadyReconciled returns existing. Reconciled no-duplicate.
//! Patch 8: 10 source-chain indexes.
//!
//! Does not mutate original workflow runs, manual results, reviews, readiness records,
//! reconciliations, routes, outcomes, sessions, trace, memory, tools, git/provider,
//! or governance records.

use std::path::Path;

use openwand_workflow::workflow_manual_result_reconciliation_gate::*;
use openwand_workflow::workflow_reconciliation::{WorkflowRunRevision, WorkflowRunRevisionId};

fn gate_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_manual_result_reconciliation_gates")
}
fn gate_records_dir(store_root: &Path) -> std::path::PathBuf {
    gate_root(store_root).join("records")
}
fn index_file(root: &Path, index_name: &str, key: &str) -> std::path::PathBuf {
    root.join(index_name).join(format!("{}.json", key))
}

pub fn save_manual_reconciliation_gate(
    store_root: &Path,
    record: &WorkflowManualResultReconciliationGateRecord,
) -> Result<std::path::PathBuf, String> {
    let dir = gate_records_dir(store_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Dir: {}", e))?;

    // Patch 7: idempotency
    if let Ok(existing) = list_manual_reconciliation_gates(store_root) {
        for er in &existing {
            // Same gate ID → return existing
            if er.gate_id == record.gate_id {
                return Ok(dir.join(format!("{}.json", er.gate_id.0)));
            }
            // Patch 7: Reconciled cannot duplicate for same readiness/manual_result/stage with different key
            if er.manual_result_id == record.manual_result_id
                && er.reconciliation_readiness_id == record.reconciliation_readiness_id
                && er.stage_id == record.stage_id
                && matches!(er.status, WorkflowManualResultReconciliationGateStatus::Reconciled)
                && matches!(record.status, WorkflowManualResultReconciliationGateStatus::Reconciled)
                && er.gate_id != record.gate_id {
                return Ok(dir.join(format!("{}.json", er.gate_id.0)));
            }
        }
    }

    let path = dir.join(format!("{}.json", record.gate_id.0));
    let json = serde_json::to_string_pretty(record).map_err(|e| format!("Serialize: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Write: {}", e))?;

    std::fs::write(dir.join("latest.json"), record.gate_id.0.as_bytes())
        .map_err(|e| format!("Latest: {}", e))?;

    // Patch 8: source-chain indexes
    let root = gate_root(store_root);
    for (idx_name, key) in [
        ("by_workflow_run", record.workflow_execution_id.0.as_str()),
        ("by_manual_result", record.manual_result_id.0.as_str()),
        ("by_manual_result_review", record.manual_result_review_id.0.as_str()),
        ("by_readiness", record.reconciliation_readiness_id.0.as_str()),
        ("by_stage", record.stage_id.as_str()),
        ("by_command_review", record.command_review_id.0.as_str()),
        ("by_command_composer", record.command_composer_id.0.as_str()),
        ("by_loop_controller", record.loop_controller_id.0.as_str()),
    ] {
        let idx_file = index_file(&root, idx_name, key);
        std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Index dir: {}", e))?;
        std::fs::write(&idx_file, record.gate_id.0.as_bytes()).map_err(|e| format!("Index: {}", e))?;
    }

    // by_run_revision index
    if let Some(ref rev_id) = record.new_run_revision_id {
        let idx_file = index_file(&root, "by_run_revision", &rev_id.0);
        std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Rev index dir: {}", e))?;
        std::fs::write(&idx_file, record.gate_id.0.as_bytes()).map_err(|e| format!("Rev index: {}", e))?;
    }

    Ok(path)
}

pub fn save_manual_reconciliation_revision(
    store_root: &Path,
    revision: &WorkflowRunRevision,
) -> Result<std::path::PathBuf, String> {
    // Reuse existing workflow_run_revisions/ storage
    crate::workflow_reconciliation::save_workflow_run_revision(store_root, revision)
}

pub fn load_manual_reconciliation_gate(store_root: &Path, id: &WorkflowManualResultReconciliationGateId) -> Result<WorkflowManualResultReconciliationGateRecord, String> {
    let path = gate_records_dir(store_root).join(format!("{}.json", id.0));
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Parse: {}", e))
}

pub fn list_manual_reconciliation_gates(store_root: &Path) -> Result<Vec<WorkflowManualResultReconciliationGateRecord>, String> {
    let dir = gate_records_dir(store_root);
    if !dir.exists() { return Ok(vec![]); }
    let mut records = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("Dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Entry: {}", e))?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "json") {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            if name == "latest" { continue; }
            if let Ok(json) = std::fs::read_to_string(&path) {
                if let Ok(record) = serde_json::from_str::<WorkflowManualResultReconciliationGateRecord>(&json) {
                    records.push(record);
                }
            }
        }
    }
    records.sort_by(|a, b| b.reconciled_at.cmp(&a.reconciled_at));
    Ok(records)
}

pub fn latest_manual_reconciliation_gate(store_root: &Path) -> Result<Option<WorkflowManualResultReconciliationGateRecord>, String> {
    let p = gate_records_dir(store_root).join("latest.json");
    if !p.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&p).map_err(|e| format!("{}", e))?;
    load_manual_reconciliation_gate(store_root, &WorkflowManualResultReconciliationGateId(id.trim().into())).map(Some)
}

// Patch 8: source-chain index lookups
pub fn gate_by_workflow_run(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReconciliationGateRecord>, String> {
    load_gate_index(store_root, "by_workflow_run", id)
}
pub fn gate_by_manual_result(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReconciliationGateRecord>, String> {
    load_gate_index(store_root, "by_manual_result", id)
}
pub fn gate_by_manual_result_review(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReconciliationGateRecord>, String> {
    load_gate_index(store_root, "by_manual_result_review", id)
}
pub fn gate_by_readiness(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReconciliationGateRecord>, String> {
    load_gate_index(store_root, "by_readiness", id)
}
pub fn gate_by_stage(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReconciliationGateRecord>, String> {
    load_gate_index(store_root, "by_stage", id)
}
pub fn gate_by_command_review(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReconciliationGateRecord>, String> {
    load_gate_index(store_root, "by_command_review", id)
}
pub fn gate_by_command_composer(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReconciliationGateRecord>, String> {
    load_gate_index(store_root, "by_command_composer", id)
}
pub fn gate_by_loop_controller(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReconciliationGateRecord>, String> {
    load_gate_index(store_root, "by_loop_controller", id)
}
pub fn gate_by_run_revision(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReconciliationGateRecord>, String> {
    load_gate_index(store_root, "by_run_revision", id)
}

fn load_gate_index(store_root: &Path, index_name: &str, key: &str) -> Result<Option<WorkflowManualResultReconciliationGateRecord>, String> {
    let pointer = index_file(&gate_root(store_root), index_name, key);
    if !pointer.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&pointer).map_err(|e| format!("{}", e))?;
    load_manual_reconciliation_gate(store_root, &WorkflowManualResultReconciliationGateId(id.trim().into())).map(Some)
}

// Patch 7: AlreadyReconciled check
pub fn find_existing_reconciled_gate(store_root: &Path, manual_result_id: &str, readiness_id: &str, stage_id: &str) -> Result<Option<WorkflowManualResultReconciliationGateRecord>, String> {
    let gates = list_manual_reconciliation_gates(store_root)?;
    Ok(gates.into_iter().find(|g| {
        g.manual_result_id.0 == manual_result_id
            && g.reconciliation_readiness_id.0 == readiness_id
            && g.stage_id == stage_id
            && matches!(g.status, WorkflowManualResultReconciliationGateStatus::Reconciled)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_manual_result::WorkflowManualResultId;
    use openwand_workflow::workflow_manual_result_review::WorkflowManualResultReviewId;
    use openwand_workflow::workflow_manual_result_reconciliation_readiness::WorkflowManualResultReconciliationReadinessId;
    use openwand_workflow::workflow_command_review::WorkflowCommandReviewId;
    use openwand_workflow::workflow_command_composer::WorkflowCommandComposerId;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;
    use chrono::Utc;

    fn test_dir() -> std::path::PathBuf { tempfile::tempdir().unwrap().into_path() }

    fn test_record(status: WorkflowManualResultReconciliationGateStatus, suffix: &str) -> WorkflowManualResultReconciliationGateRecord {
        let decision = match &status {
            WorkflowManualResultReconciliationGateStatus::Reconciled =>
                WorkflowManualResultReconciliationGateDecision::Reconciled { revision_id: Some(format!("wrr_{}", suffix)), summary: "ok".into() },
            WorkflowManualResultReconciliationGateStatus::Blocked =>
                WorkflowManualResultReconciliationGateDecision::Blocked { reason_code: "test".into(), summary: "blocked".into() },
            WorkflowManualResultReconciliationGateStatus::Failed =>
                WorkflowManualResultReconciliationGateDecision::Failed { reason_code: "test".into(), summary: "failed".into() },
            WorkflowManualResultReconciliationGateStatus::AlreadyReconciled =>
                WorkflowManualResultReconciliationGateDecision::AlreadyReconciled { revision_id: Some(format!("wrr_{}", suffix)), summary: "dup".into() },
        };
        let creates = matches!(status, WorkflowManualResultReconciliationGateStatus::Reconciled);
        WorkflowManualResultReconciliationGateRecord {
            gate_id: WorkflowManualResultReconciliationGateId(format!("wmrrg_{}", suffix)),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            manual_result_id: WorkflowManualResultId("wmr_t".into()),
            manual_result_review_id: WorkflowManualResultReviewId("wmrr_t".into()),
            reconciliation_readiness_id: WorkflowManualResultReconciliationReadinessId("wmrrr_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            stage_id: "stage_1".into(),
            workflow_run_hash: "wrh".into(), reconciliation_readiness_hash: "rrh".into(),
            manual_result_review_hash: "mrrh".into(), manual_result_hash: "mrh".into(),
            command_review_hash: "crh".into(), command_composer_hash: "cch".into(),
            command_descriptor_hash: "cdh".into(), loop_controller_hash: "lch".into(),
            status, decision, predicates: vec![],
            progression: None, new_run_revision_id: if creates { Some(WorkflowRunRevisionId(format!("wrr_{}", suffix))) } else { None },
            creates_run_revision: creates, mutates_original_workflow_run: false,
            verifies_external_truth: false, executes_command: false,
            routes_continuation: false, appends_trace: false, writes_memory: false,
            creates_execution_grant: false, execution_allowed_now: false,
            reconciled_by: "test".into(), reconciled_at: Utc::now(),
        }
    }

    #[test] fn gate_persists_and_loads_roundtrip() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationGateStatus::Reconciled, "rt");
        save_manual_reconciliation_gate(&d, &r).unwrap();
        let l = load_manual_reconciliation_gate(&d, &r.gate_id).unwrap();
        assert_eq!(r.gate_id, l.gate_id);
    }
    #[test] fn latest_gate_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationGateStatus::Reconciled, "lt");
        save_manual_reconciliation_gate(&d, &r).unwrap();
        assert_eq!(r.gate_id, latest_manual_reconciliation_gate(&d).unwrap().unwrap().gate_id);
    }
    // Patch 8: source-chain indexes
    #[test] fn gate_by_manual_result_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationGateStatus::Reconciled, "mr");
        save_manual_reconciliation_gate(&d, &r).unwrap();
        assert_eq!(r.gate_id, gate_by_manual_result(&d, "wmr_t").unwrap().unwrap().gate_id);
    }
    #[test] fn gate_by_manual_result_review_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationGateStatus::Reconciled, "rv");
        save_manual_reconciliation_gate(&d, &r).unwrap();
        assert_eq!(r.gate_id, gate_by_manual_result_review(&d, "wmrr_t").unwrap().unwrap().gate_id);
    }
    #[test] fn gate_by_command_review_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationGateStatus::Reconciled, "cr");
        save_manual_reconciliation_gate(&d, &r).unwrap();
        assert_eq!(r.gate_id, gate_by_command_review(&d, "wcrv_t").unwrap().unwrap().gate_id);
    }
    #[test] fn gate_by_command_composer_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationGateStatus::Reconciled, "cc");
        save_manual_reconciliation_gate(&d, &r).unwrap();
        assert_eq!(r.gate_id, gate_by_command_composer(&d, "wcc_t").unwrap().unwrap().gate_id);
    }
    #[test] fn gate_by_loop_controller_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationGateStatus::Reconciled, "lc");
        save_manual_reconciliation_gate(&d, &r).unwrap();
        assert_eq!(r.gate_id, gate_by_loop_controller(&d, "wlc_t").unwrap().unwrap().gate_id);
    }
    #[test] fn gate_by_run_revision_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationGateStatus::Reconciled, "rv2");
        save_manual_reconciliation_gate(&d, &r).unwrap();
        assert_eq!(r.gate_id, gate_by_run_revision(&d, "wrr_rv2").unwrap().unwrap().gate_id);
    }
    // Patch 7: idempotency
    #[test] fn same_idempotency_key_returns_existing() {
        let d = test_dir(); let r = test_record(WorkflowManualResultReconciliationGateStatus::Reconciled, "id");
        let p1 = save_manual_reconciliation_gate(&d, &r).unwrap();
        let p2 = save_manual_reconciliation_gate(&d, &r).unwrap();
        assert_eq!(p1, p2);
    }
    #[test] fn reconciled_cannot_duplicate_with_different_key() {
        let d = test_dir();
        let r1 = test_record(WorkflowManualResultReconciliationGateStatus::Reconciled, "dp1");
        save_manual_reconciliation_gate(&d, &r1).unwrap();
        let r2 = test_record(WorkflowManualResultReconciliationGateStatus::Reconciled, "dp2");
        let p2 = save_manual_reconciliation_gate(&d, &r2).unwrap();
        assert!(p2.to_string_lossy().contains("dp1"), "Should return existing");
    }
    #[test] fn blocked_may_retry_with_new_key() {
        let d = test_dir();
        let r1 = test_record(WorkflowManualResultReconciliationGateStatus::Blocked, "bl1");
        save_manual_reconciliation_gate(&d, &r1).unwrap();
        let r2 = test_record(WorkflowManualResultReconciliationGateStatus::Reconciled, "bl2");
        let p2 = save_manual_reconciliation_gate(&d, &r2).unwrap();
        assert!(p2.exists(), "New Reconciled creates new record");
    }
    #[test] fn failed_may_retry_with_new_key() {
        let d = test_dir();
        let r1 = test_record(WorkflowManualResultReconciliationGateStatus::Failed, "fl1");
        save_manual_reconciliation_gate(&d, &r1).unwrap();
        let r2 = test_record(WorkflowManualResultReconciliationGateStatus::Reconciled, "fl2");
        let p2 = save_manual_reconciliation_gate(&d, &r2).unwrap();
        assert!(p2.exists(), "New Reconciled creates new record");
    }
    #[test] fn already_reconciled_returns_existing_revision() {
        let d = test_dir();
        let r1 = test_record(WorkflowManualResultReconciliationGateStatus::Reconciled, "ar1");
        save_manual_reconciliation_gate(&d, &r1).unwrap();
        let found = find_existing_reconciled_gate(&d, "wmr_t", "wmrrr_t", "stage_1").unwrap();
        assert!(found.is_some());
        assert_eq!("wrr_ar1", found.unwrap().new_run_revision_id.unwrap().0);
    }
    // No-write proofs
    #[test] fn gate_writes_only_gate_and_revision_evidence() {
        let d = test_dir();
        save_manual_reconciliation_gate(&d, &test_record(WorkflowManualResultReconciliationGateStatus::Reconciled, "wo")).unwrap();
        assert!(gate_root(&d).exists());
        assert!(!d.join("workflow_manual_results").exists());
        assert!(!d.join("workflow_manual_result_reviews").exists());
        assert!(!d.join("workflow_runs").exists());
    }
    #[test] fn gate_does_not_write_route_outcome_or_session_records() {
        let d = test_dir();
        save_manual_reconciliation_gate(&d, &test_record(WorkflowManualResultReconciliationGateStatus::Reconciled, "nr")).unwrap();
        assert!(!d.join("workflow_action_routes").exists());
        assert!(!d.join("workflow_action_outcomes").exists());
        assert!(!d.join("sessions").exists());
    }
}
