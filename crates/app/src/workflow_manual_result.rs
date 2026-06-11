//! Manual result persistence.
//!
//! Writes only manual-result evidence. Does not mutate command reviews,
//! command-composer records, loop-controller records, workflow runs,
//! revisions, routes, outcomes, reconciliations, continuation, reviews,
//! readiness, routing, approvals, sessions, trace, memory, tools,
//! git/provider, or governance records.

use std::path::Path;

use openwand_workflow::workflow_manual_result::*;

fn result_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_manual_results")
}
fn records_dir(store_root: &Path) -> std::path::PathBuf {
    result_root(store_root).join("records")
}
fn artifacts_dir(store_root: &Path) -> std::path::PathBuf {
    result_root(store_root).join("artifacts")
}
fn index_file(root: &Path, index_name: &str, key: &str) -> std::path::PathBuf {
    root.join(index_name).join(format!("{}.json", key))
}

pub fn save_manual_result(
    store_root: &Path,
    record: &WorkflowManualResult,
) -> Result<std::path::PathBuf, String> {
    let dir = records_dir(store_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Dir: {}", e))?;

    // Idempotency
    if let Ok(existing) = list_manual_results(store_root) {
        for er in &existing {
            if er.result_id == record.result_id { return Ok(dir.join(format!("{}.json", er.result_id.0))); }
            // Patch 3: ReportedSucceeded cannot duplicate for same review
            if er.command_review_id == record.command_review_id
                && matches!(er.status, WorkflowManualResultStatus::ReportedSucceeded)
                && matches!(record.status, WorkflowManualResultStatus::ReportedSucceeded)
                && er.result_id != record.result_id {
                return Ok(dir.join(format!("{}.json", er.result_id.0)));
            }
        }
    }

    let path = dir.join(format!("{}.json", record.result_id.0));
    let json = serde_json::to_string_pretty(record).map_err(|e| format!("Serialize: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Write: {}", e))?;

    std::fs::write(dir.join("latest.json"), record.result_id.0.as_bytes())
        .map_err(|e| format!("Latest: {}", e))?;

    // Save artifacts separately if present
    if !record.artifact_references.is_empty() {
        let a_dir = artifacts_dir(store_root);
        std::fs::create_dir_all(&a_dir).map_err(|e| format!("Artifacts dir: {}", e))?;
        let a_json = serde_json::to_string_pretty(&record.artifact_references)
            .map_err(|e| format!("Artifacts: {}", e))?;
        std::fs::write(a_dir.join(format!("{}.json", record.result_id.0)), a_json)
            .map_err(|e| format!("Artifacts write: {}", e))?;
    }

    let root = result_root(store_root);
    for (idx_name, key) in [
        ("by_workflow_run", record.workflow_execution_id.0.as_str()),
        ("by_command_review", record.command_review_id.0.as_str()),
        ("by_command_composer", record.command_composer_id.0.as_str()),
        ("by_loop_controller", record.loop_controller_id.0.as_str()),
    ] {
        let idx_file = index_file(&root, idx_name, key);
        std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Index dir: {}", e))?;
        std::fs::write(&idx_file, record.result_id.0.as_bytes()).map_err(|e| format!("Index: {}", e))?;
    }

    Ok(path)
}

pub fn load_manual_result(store_root: &Path, id: &WorkflowManualResultId) -> Result<WorkflowManualResult, String> {
    let path = records_dir(store_root).join(format!("{}.json", id.0));
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Parse: {}", e))
}

pub fn load_artifacts(store_root: &Path, id: &WorkflowManualResultId) -> Result<Vec<WorkflowManualArtifactReference>, String> {
    let path = artifacts_dir(store_root).join(format!("{}.json", id.0));
    if !path.exists() { return Ok(vec![]); }
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Parse: {}", e))
}

pub fn list_manual_results(store_root: &Path) -> Result<Vec<WorkflowManualResult>, String> {
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
                && let Ok(record) = serde_json::from_str::<WorkflowManualResult>(&json) {
                    records.push(record);
                }
        }
    }
    records.sort_by(|a, b| b.captured_at.cmp(&a.captured_at));
    Ok(records)
}

pub fn latest_manual_result(store_root: &Path) -> Result<Option<WorkflowManualResult>, String> {
    let p = records_dir(store_root).join("latest.json");
    if !p.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&p).map_err(|e| format!("{}", e))?;
    load_manual_result(store_root, &WorkflowManualResultId(id.trim().into())).map(Some)
}

pub fn result_by_workflow_run(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResult>, String> {
    load_index(store_root, "by_workflow_run", id)
}
pub fn result_by_command_review(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResult>, String> {
    load_index(store_root, "by_command_review", id)
}
pub fn result_by_command_composer(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResult>, String> {
    load_index(store_root, "by_command_composer", id)
}
pub fn result_by_loop_controller(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResult>, String> {
    load_index(store_root, "by_loop_controller", id)
}

fn load_index(store_root: &Path, index_name: &str, key: &str) -> Result<Option<WorkflowManualResult>, String> {
    let pointer = index_file(&result_root(store_root), index_name, key);
    if !pointer.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&pointer).map_err(|e| format!("{}", e))?;
    load_manual_result(store_root, &WorkflowManualResultId(id.trim().into())).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_manual_result::*;
    use openwand_workflow::workflow_command_review::WorkflowCommandReviewId;
    use openwand_workflow::workflow_command_composer::WorkflowCommandComposerId;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use chrono::Utc;

    fn test_dir() -> std::path::PathBuf { tempfile::tempdir().unwrap().into_path() }

    fn test_result(status: WorkflowManualResultStatus, suffix: &str) -> WorkflowManualResult {
        WorkflowManualResult {
            result_id: WorkflowManualResultId(format!("wmr_{}", suffix)),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            command_review_hash: "rh".into(), command_composer_hash: "ch".into(),
            command_descriptor_hash: "dh".into(), loop_controller_hash: "lh".into(),
            status: status.clone(),
            operator: "tester".into(),
            summary: WorkflowManualResultSummary {
                operator_summary: "done".into(),
                operator_details: if matches!(status, WorkflowManualResultStatus::ReportedFailed | WorkflowManualResultStatus::ReportedPartial) {
                    Some("details".into())
                } else { None },
                reported_status: status.clone(),
                caveat: "Not verified".into(),
            },
            artifact_references: if matches!(status, WorkflowManualResultStatus::ReportedFailed) {
                vec![WorkflowManualArtifactReference {
                    artifact_id: "art_1".into(), label: "log".into(),
                    kind: WorkflowManualArtifactKind::LogExcerpt,
                    reference: "/tmp/log".into(),
                    operator_supplied_hash: Some("abc".into()), description: None,
                }]
            } else { vec![] },
            validation_snapshot: WorkflowManualResultValidationSnapshot {
                command_review_was_acknowledged: true,
                command_review_hash_matched: true, command_composer_hash_matched: true,
                command_descriptor_hash_matched: true, loop_controller_hash_matched: true,
                command_review_marked_not_performed_by_openwand: true,
            },
            reported_by_operator: true,
            verified_by_openwand: false, command_executed_by_openwand: false,
            mutates_workflow_state: false, reconciles_outcome: false,
            routes_action: false, resolves_approval: false,
            appends_trace: false, writes_memory: false,
            invokes_shell: false, invokes_git: false,
            creates_execution_grant: false, execution_allowed_now: false,
            captured_at: Utc::now(),
        }
    }

    #[test] fn manual_result_persists_and_loads_roundtrip() {
        let d = test_dir(); let r = test_result(WorkflowManualResultStatus::ReportedSucceeded, "rt");
        save_manual_result(&d, &r).unwrap();
        let l = load_manual_result(&d, &r.result_id).unwrap();
        assert_eq!(r.result_id, l.result_id);
    }
    #[test] fn latest_manual_result_returns_expected() {
        let d = test_dir(); let r = test_result(WorkflowManualResultStatus::ReportedSucceeded, "lt");
        save_manual_result(&d, &r).unwrap();
        assert_eq!(r.result_id, latest_manual_result(&d).unwrap().unwrap().result_id);
    }
    #[test] fn manual_result_by_workflow_run_returns_expected() {
        let d = test_dir(); let r = test_result(WorkflowManualResultStatus::ReportedSucceeded, "wr");
        save_manual_result(&d, &r).unwrap();
        assert_eq!(r.result_id, result_by_workflow_run(&d, "wfx_t").unwrap().unwrap().result_id);
    }
    #[test] fn manual_result_by_command_review_returns_expected() {
        let d = test_dir(); let r = test_result(WorkflowManualResultStatus::ReportedSucceeded, "cr");
        save_manual_result(&d, &r).unwrap();
        assert_eq!(r.result_id, result_by_command_review(&d, "wcrv_t").unwrap().unwrap().result_id);
    }
    #[test] fn manual_result_by_command_composer_returns_expected() {
        let d = test_dir(); let r = test_result(WorkflowManualResultStatus::ReportedSucceeded, "cc");
        save_manual_result(&d, &r).unwrap();
        assert_eq!(r.result_id, result_by_command_composer(&d, "wcc_t").unwrap().unwrap().result_id);
    }
    #[test] fn manual_result_by_loop_controller_returns_expected() {
        let d = test_dir(); let r = test_result(WorkflowManualResultStatus::ReportedSucceeded, "lc");
        save_manual_result(&d, &r).unwrap();
        assert_eq!(r.result_id, result_by_loop_controller(&d, "wlc_t").unwrap().unwrap().result_id);
    }
    #[test] fn manual_result_artifacts_persist_and_load_roundtrip() {
        let d = test_dir(); let r = test_result(WorkflowManualResultStatus::ReportedFailed, "fb");
        save_manual_result(&d, &r).unwrap();
        let arts = load_artifacts(&d, &r.result_id).unwrap();
        assert_eq!(1, arts.len());
        assert_eq!("art_1", arts[0].artifact_id);
    }
    #[test] fn same_idempotency_key_returns_existing_result() {
        let d = test_dir(); let r = test_result(WorkflowManualResultStatus::ReportedSucceeded, "id");
        let p1 = save_manual_result(&d, &r).unwrap();
        let p2 = save_manual_result(&d, &r).unwrap();
        assert_eq!(p1, p2);
    }
    #[test] fn reported_success_cannot_duplicate_with_different_key() {
        let d = test_dir();
        let r1 = test_result(WorkflowManualResultStatus::ReportedSucceeded, "dp1");
        save_manual_result(&d, &r1).unwrap();
        let r2 = test_result(WorkflowManualResultStatus::ReportedSucceeded, "dp2");
        let p2 = save_manual_result(&d, &r2).unwrap();
        assert!(p2.to_string_lossy().contains("dp1"), "Should return existing");
    }
    #[test] fn reported_failure_preserves_history_with_new_key() {
        let d = test_dir();
        let r1 = test_result(WorkflowManualResultStatus::ReportedFailed, "fl1");
        save_manual_result(&d, &r1).unwrap();
        let r2 = test_result(WorkflowManualResultStatus::ReportedFailed, "fl2");
        let p2 = save_manual_result(&d, &r2).unwrap();
        assert!(p2.exists(), "New failure creates new record");
    }
    // Patch 3: ReportedPartial preserves history
    #[test] fn reported_partial_preserves_history_with_new_key() {
        let d = test_dir();
        let r1 = test_result(WorkflowManualResultStatus::ReportedPartial, "pt1");
        save_manual_result(&d, &r1).unwrap();
        let r2 = test_result(WorkflowManualResultStatus::ReportedPartial, "pt2");
        let p2 = save_manual_result(&d, &r2).unwrap();
        assert!(p2.exists(), "New partial creates new record");
    }
    #[test] fn not_performed_preserves_history_with_new_key() {
        let d = test_dir();
        let r1 = test_result(WorkflowManualResultStatus::NotPerformed, "np1");
        save_manual_result(&d, &r1).unwrap();
        let r2 = test_result(WorkflowManualResultStatus::NotPerformed, "np2");
        let p2 = save_manual_result(&d, &r2).unwrap();
        assert!(p2.exists(), "New not-performed creates new record");
    }
    // Patch 3: Inconclusive preserves history
    #[test] fn inconclusive_preserves_history_with_new_key() {
        let d = test_dir();
        let r1 = test_result(WorkflowManualResultStatus::Inconclusive, "ic1");
        save_manual_result(&d, &r1).unwrap();
        let r2 = test_result(WorkflowManualResultStatus::Inconclusive, "ic2");
        let p2 = save_manual_result(&d, &r2).unwrap();
        assert!(p2.exists(), "New inconclusive creates new record");
    }
    // Patch 3: Inconclusive idempotency
    #[test] fn same_idempotency_key_returns_existing_inconclusive_result() {
        let d = test_dir(); let r = test_result(WorkflowManualResultStatus::Inconclusive, "ii");
        let p1 = save_manual_result(&d, &r).unwrap();
        let p2 = save_manual_result(&d, &r).unwrap();
        assert_eq!(p1, p2);
    }
    #[test] fn manual_result_writes_only_manual_result_evidence() {
        let d = test_dir();
        save_manual_result(&d, &test_result(WorkflowManualResultStatus::ReportedSucceeded, "wo")).unwrap();
        assert!(result_root(&d).exists());
        assert!(!d.join("workflow_runs").exists());
        assert!(!d.join("workflow_loop_controller").exists());
    }
    #[test] fn manual_result_does_not_write_command_review_or_composer_records() {
        let d = test_dir();
        save_manual_result(&d, &test_result(WorkflowManualResultStatus::ReportedSucceeded, "nc")).unwrap();
        assert!(!d.join("workflow_command_reviews").exists());
        assert!(!d.join("workflow_command_composer").exists());
    }
    #[test] fn manual_result_does_not_write_loop_controller_records() {
        let d = test_dir();
        save_manual_result(&d, &test_result(WorkflowManualResultStatus::ReportedSucceeded, "nl")).unwrap();
        assert!(!d.join("workflow_loop_controller").exists());
    }
    #[test] fn manual_result_does_not_write_route_outcome_reconciliation_records() {
        let d = test_dir();
        save_manual_result(&d, &test_result(WorkflowManualResultStatus::ReportedSucceeded, "nr")).unwrap();
        assert!(!d.join("workflow_action_routes").exists());
        assert!(!d.join("workflow_action_outcomes").exists());
        assert!(!d.join("workflow_reconciliations").exists());
    }
    // Patch 4: approval/session no-write
    #[test] fn manual_result_does_not_write_approval_or_session_records() {
        let d = test_dir();
        save_manual_result(&d, &test_result(WorkflowManualResultStatus::ReportedSucceeded, "ns")).unwrap();
        assert!(!d.join("approvals").exists());
        assert!(!d.join("sessions").exists());
    }
    #[test] fn manual_result_does_not_write_governance_or_provider_records() {
        let d = test_dir();
        save_manual_result(&d, &test_result(WorkflowManualResultStatus::ReportedSucceeded, "ng")).unwrap();
        assert!(!d.join("governance").exists());
        assert!(!d.join("providers").exists());
    }
}
