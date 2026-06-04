//! Workflow loop controller persistence.
//!
//! Controller writes only loop-controller recommendation evidence.
//! Does not mutate workflow runs, revisions, routes, outcomes, reconciliations,
//! continuation records, reviews, readiness, routing, approvals, sessions, trace,
//! memory, tools, git, or provider records.

use std::path::Path;

use openwand_workflow::workflow_loop_controller::*;

fn controller_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_loop_controller")
}
fn records_dir(store_root: &Path) -> std::path::PathBuf {
    controller_root(store_root).join("records")
}
fn index_file(root: &Path, index_name: &str, key: &str) -> std::path::PathBuf {
    root.join(index_name).join(format!("{}.json", key))
}

pub fn save_loop_controller(
    store_root: &Path,
    record: &WorkflowLoopControllerRecord,
) -> Result<std::path::PathBuf, String> {
    let dir = records_dir(store_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Dir: {}", e))?;

    // Idempotency: same key returns existing
    if let Ok(existing) = list_loop_controllers(store_root) {
        for er in &existing {
            if er.workflow_execution_id == record.workflow_execution_id
                && er.controller_id == record.controller_id
            {
                return Ok(dir.join(format!("{}.json", er.controller_id.0)));
            }
            // RecommendationReady cannot duplicate for same workflow + key
            if er.workflow_execution_id == record.workflow_execution_id
                && matches!(er.status, WorkflowLoopControllerStatus::RecommendationReady)
                && matches!(record.status, WorkflowLoopControllerStatus::RecommendationReady)
                && er.controller_id != record.controller_id
            {
                return Ok(dir.join(format!("{}.json", er.controller_id.0)));
            }
        }
    }

    let path = dir.join(format!("{}.json", record.controller_id.0));
    let json = serde_json::to_string_pretty(record).map_err(|e| format!("Serialize: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Write: {}", e))?;

    std::fs::write(dir.join("latest.json"), record.controller_id.0.as_bytes())
        .map_err(|e| format!("Latest: {}", e))?;

    let root = controller_root(store_root);
    for (idx_name, key) in [
        ("by_workflow_run", record.workflow_execution_id.0.as_str()),
    ] {
        let idx_file = index_file(&root, idx_name, key);
        std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Index dir: {}", e))?;
        std::fs::write(&idx_file, record.controller_id.0.as_bytes()).map_err(|e| format!("Index: {}", e))?;
    }
    if let Some(ref rev_id) = record.latest_run_revision_id {
        let idx_file = index_file(&root, "by_run_revision", &rev_id.0);
        std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Index dir: {}", e))?;
        std::fs::write(&idx_file, record.controller_id.0.as_bytes()).map_err(|e| format!("Index: {}", e))?;
    }

    Ok(path)
}

pub fn load_loop_controller(store_root: &Path, id: &WorkflowLoopControllerId) -> Result<WorkflowLoopControllerRecord, String> {
    let path = records_dir(store_root).join(format!("{}.json", id.0));
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Parse: {}", e))
}

pub fn list_loop_controllers(store_root: &Path) -> Result<Vec<WorkflowLoopControllerRecord>, String> {
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
                if let Ok(record) = serde_json::from_str::<WorkflowLoopControllerRecord>(&json) {
                    records.push(record);
                }
            }
        }
    }
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(records)
}

pub fn latest_loop_controller(store_root: &Path) -> Result<Option<WorkflowLoopControllerRecord>, String> {
    let p = records_dir(store_root).join("latest.json");
    if !p.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&p).map_err(|e| format!("{}", e))?;
    load_loop_controller(store_root, &WorkflowLoopControllerId(id.trim().into())).map(Some)
}

pub fn controller_by_workflow_run(store_root: &Path, id: &str) -> Result<Option<WorkflowLoopControllerRecord>, String> {
    load_index(store_root, "by_workflow_run", id)
}

pub fn controller_by_run_revision(store_root: &Path, id: &str) -> Result<Option<WorkflowLoopControllerRecord>, String> {
    load_index(store_root, "by_run_revision", id)
}

fn load_index(store_root: &Path, index_name: &str, key: &str) -> Result<Option<WorkflowLoopControllerRecord>, String> {
    let pointer = index_file(&controller_root(store_root), index_name, key);
    if !pointer.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&pointer).map_err(|e| format!("{}", e))?;
    load_loop_controller(store_root, &WorkflowLoopControllerId(id.trim().into())).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_loop_controller::*;
    use openwand_workflow::workflow_loop_recommendation::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;
    use chrono::Utc;

    fn test_dir() -> std::path::PathBuf { tempfile::tempdir().unwrap().into_path() }

    fn test_record(status: WorkflowLoopControllerStatus, suffix: &str) -> WorkflowLoopControllerRecord {
        WorkflowLoopControllerRecord {
            controller_id: WorkflowLoopControllerId(format!("wlc_{}", suffix)),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            latest_run_revision_id: Some(WorkflowRunRevisionId("wrr_t".into())),
            status: status.clone(),
            decision: match &status {
                WorkflowLoopControllerStatus::RecommendationReady => WorkflowLoopControllerDecision::Recommend {
                    operation: WorkflowManualOperationKind::CreateContinuationProposal, summary: "test".into(),
                },
                WorkflowLoopControllerStatus::NoManualActionRequired => WorkflowLoopControllerDecision::NoManualActionRequired { summary: "done".into() },
                WorkflowLoopControllerStatus::Blocked => WorkflowLoopControllerDecision::Blocked { reason_code: "test".into(), summary: "blocked".into() },
                WorkflowLoopControllerStatus::Inconclusive => WorkflowLoopControllerDecision::Inconclusive { reason_code: "test".into(), summary: "inconclusive".into() },
            },
            loop_state: None, recommendation: None, predicates: vec![], evidence_links: vec![],
            creates_route: false, resolves_approval: false, reconciles_outcome: false,
            executes_tool: false, mutates_workflow_state: false,
            schedules_work: false, starts_worker: false, queues_operation: false,
            retries_operation: false, resumes_workflow: false,
            created_at: Utc::now(),
        }
    }

    #[test] fn loop_controller_persists_and_loads_roundtrip() {
        let d = test_dir(); let r = test_record(WorkflowLoopControllerStatus::RecommendationReady, "rt");
        save_loop_controller(&d, &r).unwrap();
        let l = load_loop_controller(&d, &r.controller_id).unwrap();
        assert_eq!(r.controller_id, l.controller_id);
    }
    #[test] fn latest_loop_controller_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowLoopControllerStatus::RecommendationReady, "lt");
        save_loop_controller(&d, &r).unwrap();
        assert_eq!(r.controller_id, latest_loop_controller(&d).unwrap().unwrap().controller_id);
    }
    #[test] fn loop_controller_by_workflow_run_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowLoopControllerStatus::RecommendationReady, "wr");
        save_loop_controller(&d, &r).unwrap();
        assert_eq!(r.controller_id, controller_by_workflow_run(&d, "wfx_t").unwrap().unwrap().controller_id);
    }
    #[test] fn loop_controller_by_run_revision_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowLoopControllerStatus::RecommendationReady, "rr");
        save_loop_controller(&d, &r).unwrap();
        assert_eq!(r.controller_id, controller_by_run_revision(&d, "wrr_t").unwrap().unwrap().controller_id);
    }
    #[test] fn same_idempotency_key_returns_existing_controller_record() {
        let d = test_dir(); let r = test_record(WorkflowLoopControllerStatus::RecommendationReady, "id");
        let p1 = save_loop_controller(&d, &r).unwrap();
        let p2 = save_loop_controller(&d, &r).unwrap();
        assert_eq!(p1, p2);
    }
    #[test] fn recommendation_ready_cannot_duplicate_for_same_evidence_state() {
        let d = test_dir();
        let r1 = test_record(WorkflowLoopControllerStatus::RecommendationReady, "dp1");
        save_loop_controller(&d, &r1).unwrap();
        let r2 = test_record(WorkflowLoopControllerStatus::RecommendationReady, "dp2");
        let p2 = save_loop_controller(&d, &r2).unwrap();
        assert!(p2.to_string_lossy().contains("dp1"), "Should return existing");
    }
    #[test] fn blocked_controller_can_retry_with_new_key() {
        let d = test_dir();
        let r1 = test_record(WorkflowLoopControllerStatus::Blocked, "bk1");
        save_loop_controller(&d, &r1).unwrap();
        let r2 = test_record(WorkflowLoopControllerStatus::RecommendationReady, "bk2");
        let p2 = save_loop_controller(&d, &r2).unwrap();
        assert!(p2.exists());
    }
    #[test] fn inconclusive_controller_can_retry_with_new_key() {
        let d = test_dir();
        let r1 = test_record(WorkflowLoopControllerStatus::Inconclusive, "ic1");
        save_loop_controller(&d, &r1).unwrap();
        let r2 = test_record(WorkflowLoopControllerStatus::RecommendationReady, "ic2");
        let p2 = save_loop_controller(&d, &r2).unwrap();
        assert!(p2.exists());
    }
    #[test] fn loop_controller_writes_only_controller_evidence() {
        let d = test_dir();
        save_loop_controller(&d, &test_record(WorkflowLoopControllerStatus::RecommendationReady, "wo")).unwrap();
        assert!(controller_root(&d).exists());
        assert!(!d.join("workflow_runs").exists());
        assert!(!d.join("workflow_action_routes").exists());
    }
    #[test] fn loop_controller_does_not_write_route_outcome_reconciliation_records() {
        let d = test_dir();
        save_loop_controller(&d, &test_record(WorkflowLoopControllerStatus::RecommendationReady, "nr")).unwrap();
        assert!(!d.join("workflow_action_routes").exists());
        assert!(!d.join("workflow_action_outcomes").exists());
        assert!(!d.join("workflow_reconciliations").exists());
    }
    #[test] fn loop_controller_does_not_write_workflow_run_or_revision_records() {
        let d = test_dir();
        save_loop_controller(&d, &test_record(WorkflowLoopControllerStatus::RecommendationReady, "nw")).unwrap();
        assert!(!d.join("workflow_runs").exists());
        assert!(!d.join("workflow_run_revisions").exists());
    }
    #[test] fn loop_controller_does_not_write_approval_or_session_records() {
        let d = test_dir();
        save_loop_controller(&d, &test_record(WorkflowLoopControllerStatus::RecommendationReady, "ns")).unwrap();
        assert!(!d.join("approvals").exists());
        assert!(!d.join("sessions").exists());
    }

    // Patch 3: loader uses existing indexes
    #[test] fn loop_controller_loader_uses_existing_latest_indexes() {
        let d = test_dir();
        save_loop_controller(&d, &test_record(WorkflowLoopControllerStatus::RecommendationReady, "ix")).unwrap();
        // Loader reads from by_workflow_run index
        let found = controller_by_workflow_run(&d, "wfx_t").unwrap().unwrap();
        assert_eq!("wlc_ix", found.controller_id.0);
    }

    // Patch 3: crate does not scan persistence directly
    #[test] fn loop_controller_crate_does_not_scan_persistence_directly() {
        let src = include_str!("../../workflow/src/workflow_loop_controller.rs");
        // Workflow crate should not import std::fs or Path
        let use_lines: Vec<&str> = src.lines().filter(|l| l.starts_with("use ")).collect();
        assert!(!use_lines.iter().any(|l| l.contains("std::fs")));
        assert!(!use_lines.iter().any(|l| l.contains("std::path")));
    }
}
