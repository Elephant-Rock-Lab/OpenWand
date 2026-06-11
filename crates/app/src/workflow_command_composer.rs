//! Command composer persistence.
//!
//! Writes only command-composer evidence. Does not mutate loop-controller,
//! workflow runs, revisions, routes, outcomes, reconciliations, continuation,
//! reviews, readiness, routing, approvals, sessions, trace, memory, tools,
//! git, provider, or governance records.

use std::path::Path;

use openwand_workflow::workflow_command_composer::*;

fn composer_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_command_composer")
}
fn records_dir(store_root: &Path) -> std::path::PathBuf {
    composer_root(store_root).join("records")
}
fn index_file(root: &Path, index_name: &str, key: &str) -> std::path::PathBuf {
    root.join(index_name).join(format!("{}.json", key))
}

pub fn save_command_composer(
    store_root: &Path,
    record: &WorkflowCommandComposerRecord,
) -> Result<std::path::PathBuf, String> {
    let dir = records_dir(store_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Dir: {}", e))?;

    // Idempotency
    if let Ok(existing) = list_command_composers(store_root) {
        for er in &existing {
            if er.loop_controller_id == record.loop_controller_id && er.composer_id == record.composer_id {
                return Ok(dir.join(format!("{}.json", er.composer_id.0)));
            }
            if er.loop_controller_id == record.loop_controller_id
                && matches!(er.status, WorkflowCommandComposerStatus::DescriptorReady)
                && matches!(record.status, WorkflowCommandComposerStatus::DescriptorReady)
                && er.composer_id != record.composer_id
            {
                return Ok(dir.join(format!("{}.json", er.composer_id.0)));
            }
        }
    }

    let path = dir.join(format!("{}.json", record.composer_id.0));
    let json = serde_json::to_string_pretty(record).map_err(|e| format!("Serialize: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Write: {}", e))?;

    std::fs::write(dir.join("latest.json"), record.composer_id.0.as_bytes())
        .map_err(|e| format!("Latest: {}", e))?;

    let root = composer_root(store_root);
    for (idx_name, key) in [
        ("by_workflow_run", record.workflow_execution_id.0.as_str()),
        ("by_loop_controller", record.loop_controller_id.0.as_str()),
    ] {
        let idx_file = index_file(&root, idx_name, key);
        std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Index dir: {}", e))?;
        std::fs::write(&idx_file, record.composer_id.0.as_bytes()).map_err(|e| format!("Index: {}", e))?;
    }

    Ok(path)
}

pub fn load_command_composer(store_root: &Path, id: &WorkflowCommandComposerId) -> Result<WorkflowCommandComposerRecord, String> {
    let path = records_dir(store_root).join(format!("{}.json", id.0));
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Parse: {}", e))
}

pub fn list_command_composers(store_root: &Path) -> Result<Vec<WorkflowCommandComposerRecord>, String> {
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
                && let Ok(record) = serde_json::from_str::<WorkflowCommandComposerRecord>(&json) {
                    records.push(record);
                }
        }
    }
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(records)
}

pub fn latest_command_composer(store_root: &Path) -> Result<Option<WorkflowCommandComposerRecord>, String> {
    let p = records_dir(store_root).join("latest.json");
    if !p.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&p).map_err(|e| format!("{}", e))?;
    load_command_composer(store_root, &WorkflowCommandComposerId(id.trim().into())).map(Some)
}

pub fn composer_by_workflow_run(store_root: &Path, id: &str) -> Result<Option<WorkflowCommandComposerRecord>, String> {
    load_index(store_root, "by_workflow_run", id)
}
pub fn composer_by_loop_controller(store_root: &Path, id: &str) -> Result<Option<WorkflowCommandComposerRecord>, String> {
    load_index(store_root, "by_loop_controller", id)
}

fn load_index(store_root: &Path, index_name: &str, key: &str) -> Result<Option<WorkflowCommandComposerRecord>, String> {
    let pointer = index_file(&composer_root(store_root), index_name, key);
    if !pointer.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&pointer).map_err(|e| format!("{}", e))?;
    load_command_composer(store_root, &WorkflowCommandComposerId(id.trim().into())).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_command_composer::*;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use chrono::Utc;

    fn test_dir() -> std::path::PathBuf { tempfile::tempdir().unwrap().into_path() }

    fn test_record(status: WorkflowCommandComposerStatus, suffix: &str) -> WorkflowCommandComposerRecord {
        WorkflowCommandComposerRecord {
            composer_id: WorkflowCommandComposerId(format!("wcc_{}", suffix)),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            loop_controller_hash: "h".into(),
            status: status.clone(),
            decision: match &status {
                WorkflowCommandComposerStatus::DescriptorReady => WorkflowCommandComposerDecision::DescriptorReady { summary: "ok".into() },
                WorkflowCommandComposerStatus::MissingInputs => WorkflowCommandComposerDecision::MissingInputs { summary: "missing".into() },
                WorkflowCommandComposerStatus::NoCommandRequired => WorkflowCommandComposerDecision::NoCommandRequired { summary: "none".into() },
                WorkflowCommandComposerStatus::Blocked => WorkflowCommandComposerDecision::Blocked { reason_code: "test".into(), summary: "blocked".into() },
                WorkflowCommandComposerStatus::Inconclusive => WorkflowCommandComposerDecision::Inconclusive { reason_code: "test".into(), summary: "inconclusive".into() },
            },
            predicates: vec![], descriptor: None, missing_inputs: vec![], evidence_links: vec![],
            executes_command: false, invokes_shell: false, invokes_git: false,
            routes_action: false, resolves_approval: false, reconciles_outcome: false,
            mutates_workflow_state: false, schedules_work: false, starts_worker: false,
            queues_operation: false, created_at: Utc::now(),
        }
    }

    #[test] fn command_descriptor_persists_and_loads_roundtrip() {
        let d = test_dir(); let r = test_record(WorkflowCommandComposerStatus::DescriptorReady, "rt");
        save_command_composer(&d, &r).unwrap();
        let l = load_command_composer(&d, &r.composer_id).unwrap();
        assert_eq!(r.composer_id, l.composer_id);
    }
    #[test] fn latest_command_descriptor_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowCommandComposerStatus::DescriptorReady, "lt");
        save_command_composer(&d, &r).unwrap();
        assert_eq!(r.composer_id, latest_command_composer(&d).unwrap().unwrap().composer_id);
    }
    #[test] fn command_descriptor_by_workflow_run_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowCommandComposerStatus::DescriptorReady, "wr");
        save_command_composer(&d, &r).unwrap();
        assert_eq!(r.composer_id, composer_by_workflow_run(&d, "wfx_t").unwrap().unwrap().composer_id);
    }
    #[test] fn command_descriptor_by_loop_controller_returns_expected() {
        let d = test_dir(); let r = test_record(WorkflowCommandComposerStatus::DescriptorReady, "lc");
        save_command_composer(&d, &r).unwrap();
        assert_eq!(r.composer_id, composer_by_loop_controller(&d, "wlc_t").unwrap().unwrap().composer_id);
    }
    #[test] fn same_idempotency_key_returns_existing_descriptor() {
        let d = test_dir(); let r = test_record(WorkflowCommandComposerStatus::DescriptorReady, "id");
        let p1 = save_command_composer(&d, &r).unwrap();
        let p2 = save_command_composer(&d, &r).unwrap();
        assert_eq!(p1, p2);
    }
    #[test] fn descriptor_ready_cannot_duplicate_for_same_loop_controller() {
        let d = test_dir();
        let r1 = test_record(WorkflowCommandComposerStatus::DescriptorReady, "dp1");
        save_command_composer(&d, &r1).unwrap();
        let r2 = test_record(WorkflowCommandComposerStatus::DescriptorReady, "dp2");
        let p2 = save_command_composer(&d, &r2).unwrap();
        assert!(p2.to_string_lossy().contains("dp1"), "Should return existing");
    }
    #[test] fn missing_inputs_can_retry_with_new_key() {
        let d = test_dir();
        let r1 = test_record(WorkflowCommandComposerStatus::MissingInputs, "mi1");
        save_command_composer(&d, &r1).unwrap();
        let r2 = test_record(WorkflowCommandComposerStatus::DescriptorReady, "mi2");
        let p2 = save_command_composer(&d, &r2).unwrap();
        assert!(p2.exists());
    }
    #[test] fn blocked_descriptor_can_retry_with_new_key() {
        let d = test_dir();
        let r1 = test_record(WorkflowCommandComposerStatus::Blocked, "bk1");
        save_command_composer(&d, &r1).unwrap();
        let r2 = test_record(WorkflowCommandComposerStatus::DescriptorReady, "bk2");
        let p2 = save_command_composer(&d, &r2).unwrap();
        assert!(p2.exists());
    }
    #[test] fn command_composer_writes_only_command_descriptor_evidence() {
        let d = test_dir();
        save_command_composer(&d, &test_record(WorkflowCommandComposerStatus::DescriptorReady, "wo")).unwrap();
        assert!(composer_root(&d).exists());
        assert!(!d.join("workflow_runs").exists());
        assert!(!d.join("workflow_loop_controller").exists());
    }
    #[test] fn command_composer_does_not_write_loop_controller_records() {
        let d = test_dir();
        save_command_composer(&d, &test_record(WorkflowCommandComposerStatus::DescriptorReady, "nl")).unwrap();
        assert!(!d.join("workflow_loop_controller").exists());
    }
    #[test] fn command_composer_does_not_write_route_outcome_reconciliation_records() {
        let d = test_dir();
        save_command_composer(&d, &test_record(WorkflowCommandComposerStatus::DescriptorReady, "nr")).unwrap();
        assert!(!d.join("workflow_action_routes").exists());
        assert!(!d.join("workflow_action_outcomes").exists());
        assert!(!d.join("workflow_reconciliations").exists());
    }
    #[test] fn command_composer_does_not_write_workflow_run_or_revision_records() {
        let d = test_dir();
        save_command_composer(&d, &test_record(WorkflowCommandComposerStatus::DescriptorReady, "nw")).unwrap();
        assert!(!d.join("workflow_runs").exists());
        assert!(!d.join("workflow_run_revisions").exists());
    }
    #[test] fn command_composer_does_not_write_approval_or_session_records() {
        let d = test_dir();
        save_command_composer(&d, &test_record(WorkflowCommandComposerStatus::DescriptorReady, "ns")).unwrap();
        assert!(!d.join("approvals").exists());
        assert!(!d.join("sessions").exists());
    }
    // Patch 5: governance + provider no-write
    #[test] fn command_composer_does_not_write_governance_records() {
        let d = test_dir();
        save_command_composer(&d, &test_record(WorkflowCommandComposerStatus::DescriptorReady, "ng")).unwrap();
        assert!(!d.join("governance").exists());
    }
    #[test] fn command_composer_does_not_write_provider_records() {
        let d = test_dir();
        save_command_composer(&d, &test_record(WorkflowCommandComposerStatus::DescriptorReady, "np")).unwrap();
        assert!(!d.join("providers").exists());
    }
}
