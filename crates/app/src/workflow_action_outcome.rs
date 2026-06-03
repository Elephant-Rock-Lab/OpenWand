//! Workflow action outcome persistence.
//!
//! Outcome records under eval_reports/workflow_action_outcomes/.
//! No direct approval, tool, policy, trace, memory, shell, git, or session writes.

use std::path::Path;

use openwand_workflow::workflow_action_outcome::{
    WorkflowActionOutcomeId, WorkflowActionOutcomeRecord, WorkflowActionOutcomeStatus,
};

fn outcomes_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_action_outcomes")
}

fn records_dir(store_root: &Path) -> std::path::PathBuf {
    outcomes_root(store_root).join("records")
}

macro_rules! index_dir {
    ($store_root:expr, $name:expr) => { outcomes_root($store_root).join($name) };
}

pub fn save_workflow_action_outcome(
    store_root: &Path,
    record: &WorkflowActionOutcomeRecord,
) -> Result<std::path::PathBuf, String> {
    let dir = records_dir(store_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create outcomes dir: {}", e))?;

    if let Ok(existing) = list_workflow_action_outcomes(store_root) {
        for er in &existing {
            if er.route_id == record.route_id && er.pending_approval_id == record.pending_approval_id {
                if er.outcome_id == record.outcome_id {
                    return Ok(dir.join(format!("{}.json", er.outcome_id.0)));
                }
                if matches!(er.status, WorkflowActionOutcomeStatus::ToolCompleted | WorkflowActionOutcomeStatus::ToolDenied)
                    && matches!(record.status, WorkflowActionOutcomeStatus::ToolCompleted | WorkflowActionOutcomeStatus::ToolDenied)
                {
                    return Ok(dir.join(format!("{}.json", er.outcome_id.0)));
                }
            }
        }
    }

    let path = dir.join(format!("{}.json", record.outcome_id.0));
    let json = serde_json::to_string_pretty(record).map_err(|e| format!("Serialize: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Write: {}", e))?;

    std::fs::write(dir.join("latest.json"), record.outcome_id.0.as_bytes())
        .map_err(|e| format!("Latest: {}", e))?;

    for (idx_dir, key) in [
        (index_dir!(store_root, "by_workflow_run"), record.workflow_execution_id.0.as_str()),
        (index_dir!(store_root, "by_route"), record.route_id.0.as_str()),
        (index_dir!(store_root, "by_stage"), record.stage_id.as_str()),
        (index_dir!(store_root, "by_action_request"), record.action_request_id.as_str()),
        (index_dir!(store_root, "by_session"), record.session_id.as_str()),
        (index_dir!(store_root, "by_pending_approval"), record.pending_approval_id.as_str()),
    ] {
        std::fs::create_dir_all(&idx_dir).map_err(|e| format!("Index dir: {}", e))?;
        std::fs::write(idx_dir.join(format!("{}.json", key)), record.outcome_id.0.as_bytes())
            .map_err(|e| format!("Index write: {}", e))?;
    }

    Ok(path)
}

pub fn load_workflow_action_outcome(store_root: &Path, outcome_id: &WorkflowActionOutcomeId) -> Result<WorkflowActionOutcomeRecord, String> {
    let path = records_dir(store_root).join(format!("{}.json", outcome_id.0));
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Parse: {}", e))
}

pub fn list_workflow_action_outcomes(store_root: &Path) -> Result<Vec<WorkflowActionOutcomeRecord>, String> {
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
                if let Ok(record) = serde_json::from_str::<WorkflowActionOutcomeRecord>(&json) {
                    records.push(record);
                }
            }
        }
    }
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(records)
}

pub fn latest_workflow_action_outcome(store_root: &Path) -> Result<Option<WorkflowActionOutcomeRecord>, String> {
    let p = records_dir(store_root).join("latest.json");
    if !p.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&p).map_err(|e| format!("{}", e))?;
    load_workflow_action_outcome(store_root, &WorkflowActionOutcomeId(id.trim().into())).map(Some)
}

pub fn outcome_by_workflow_run(store_root: &Path, id: &str) -> Result<Option<WorkflowActionOutcomeRecord>, String> {
    load_index(store_root, "by_workflow_run", id)
}
pub fn outcome_by_route(store_root: &Path, id: &str) -> Result<Option<WorkflowActionOutcomeRecord>, String> {
    load_index(store_root, "by_route", id)
}
pub fn outcome_by_stage(store_root: &Path, id: &str) -> Result<Option<WorkflowActionOutcomeRecord>, String> {
    load_index(store_root, "by_stage", id)
}
pub fn outcome_by_action_request(store_root: &Path, id: &str) -> Result<Option<WorkflowActionOutcomeRecord>, String> {
    load_index(store_root, "by_action_request", id)
}
pub fn outcome_by_session(store_root: &Path, id: &str) -> Result<Option<WorkflowActionOutcomeRecord>, String> {
    load_index(store_root, "by_session", id)
}
pub fn outcome_by_pending_approval(store_root: &Path, id: &str) -> Result<Option<WorkflowActionOutcomeRecord>, String> {
    load_index(store_root, "by_pending_approval", id)
}

fn load_index(store_root: &Path, index_name: &str, key: &str) -> Result<Option<WorkflowActionOutcomeRecord>, String> {
    let pointer = index_dir!(store_root, index_name).join(format!("{}.json", key));
    if !pointer.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&pointer).map_err(|e| format!("{}", e))?;
    load_workflow_action_outcome(store_root, &WorkflowActionOutcomeId(id.trim().into())).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_action_outcome::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_action_route::WorkflowActionRouteId;
    use chrono::Utc;

    fn test_dir() -> std::path::PathBuf { tempfile::tempdir().unwrap().into_path() }

    fn test_outcome(status: WorkflowActionOutcomeStatus, suffix: &str) -> WorkflowActionOutcomeRecord {
        WorkflowActionOutcomeRecord {
            outcome_id: WorkflowActionOutcomeId(format!("wao_{}", suffix)),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            route_id: WorkflowActionRouteId("war_t".into()),
            stage_id: "stage_1".into(), action_request_id: "ar_1".into(),
            session_id: format!("sess_{}", suffix), pending_approval_id: format!("arid_{}", suffix),
            tool_call_id: Some(format!("tc_{}", suffix)),
            route_hash: "rh".into(), workflow_run_hash: "wrh".into(),
            status: status.clone(),
            decision: match &status {
                WorkflowActionOutcomeStatus::ToolCompleted => WorkflowActionOutcomeDecision::ToolCompleted { summary: "done".into() },
                WorkflowActionOutcomeStatus::ToolDenied => WorkflowActionOutcomeDecision::ToolDenied { summary: "denied".into() },
                WorkflowActionOutcomeStatus::Blocked => WorkflowActionOutcomeDecision::Blocked { reason_code: "test".into(), summary: "blocked".into() },
                _ => WorkflowActionOutcomeDecision::ApprovalResolved { summary: "resolved".into() },
            },
            predicates: vec![], approval_resolution: WorkflowApprovalResolution::Approve { rationale: "ok".into() },
            session_outcome: None, created_at: Utc::now(), completed_at: None,
        }
    }

    #[test] fn outcome_persists_and_loads_roundtrip() {
        let d = test_dir(); let r = test_outcome(WorkflowActionOutcomeStatus::ApprovalResolved, "rt");
        save_workflow_action_outcome(&d, &r).unwrap();
        let l = load_workflow_action_outcome(&d, &r.outcome_id).unwrap();
        assert_eq!(r.outcome_id, l.outcome_id);
    }
    #[test] fn latest_outcome_returns_expected() {
        let d = test_dir(); let r = test_outcome(WorkflowActionOutcomeStatus::ApprovalResolved, "lt");
        save_workflow_action_outcome(&d, &r).unwrap();
        assert_eq!(r.outcome_id, latest_workflow_action_outcome(&d).unwrap().unwrap().outcome_id);
    }
    #[test] fn outcome_by_workflow_run_returns_expected() {
        let d = test_dir(); let r = test_outcome(WorkflowActionOutcomeStatus::ApprovalResolved, "wr");
        save_workflow_action_outcome(&d, &r).unwrap();
        assert_eq!(r.outcome_id, outcome_by_workflow_run(&d, "wfx_t").unwrap().unwrap().outcome_id);
    }
    #[test] fn outcome_by_route_returns_expected() {
        let d = test_dir(); let r = test_outcome(WorkflowActionOutcomeStatus::ApprovalResolved, "br");
        save_workflow_action_outcome(&d, &r).unwrap();
        assert_eq!(r.outcome_id, outcome_by_route(&d, "war_t").unwrap().unwrap().outcome_id);
    }
    #[test] fn outcome_by_stage_returns_expected() {
        let d = test_dir(); let r = test_outcome(WorkflowActionOutcomeStatus::ApprovalResolved, "st");
        save_workflow_action_outcome(&d, &r).unwrap();
        assert_eq!(r.outcome_id, outcome_by_stage(&d, "stage_1").unwrap().unwrap().outcome_id);
    }
    #[test] fn outcome_by_action_request_returns_expected() {
        let d = test_dir(); let r = test_outcome(WorkflowActionOutcomeStatus::ApprovalResolved, "ar");
        save_workflow_action_outcome(&d, &r).unwrap();
        assert_eq!(r.outcome_id, outcome_by_action_request(&d, "ar_1").unwrap().unwrap().outcome_id);
    }
    #[test] fn outcome_by_session_returns_expected() {
        let d = test_dir(); let r = test_outcome(WorkflowActionOutcomeStatus::ApprovalResolved, "se");
        save_workflow_action_outcome(&d, &r).unwrap();
        assert_eq!(r.outcome_id, outcome_by_session(&d, &format!("sess_se")).unwrap().unwrap().outcome_id);
    }
    #[test] fn outcome_by_pending_approval_returns_expected() {
        let d = test_dir(); let r = test_outcome(WorkflowActionOutcomeStatus::ApprovalResolved, "pa");
        save_workflow_action_outcome(&d, &r).unwrap();
        assert_eq!(r.outcome_id, outcome_by_pending_approval(&d, &format!("arid_pa")).unwrap().unwrap().outcome_id);
    }
    #[test] fn same_idempotency_key_returns_existing() {
        let d = test_dir(); let r = test_outcome(WorkflowActionOutcomeStatus::ApprovalResolved, "id");
        let p1 = save_workflow_action_outcome(&d, &r).unwrap();
        let p2 = save_workflow_action_outcome(&d, &r).unwrap();
        assert_eq!(p1, p2);
    }
    #[test] fn blocked_outcome_can_retry_with_new_key() {
        let d = test_dir(); let r1 = test_outcome(WorkflowActionOutcomeStatus::Blocked, "bk1");
        save_workflow_action_outcome(&d, &r1).unwrap();
        let mut r2 = test_outcome(WorkflowActionOutcomeStatus::ApprovalResolved, "bk2");
        r2.route_id = r1.route_id.clone(); r2.pending_approval_id = r1.pending_approval_id.clone();
        let p2 = save_workflow_action_outcome(&d, &r2).unwrap();
        assert!(p2.exists());
    }
    #[test] fn failed_outcome_can_retry_with_new_key() {
        let d = test_dir(); let r1 = test_outcome(WorkflowActionOutcomeStatus::Failed, "fl1");
        save_workflow_action_outcome(&d, &r1).unwrap();
        let mut r2 = test_outcome(WorkflowActionOutcomeStatus::ApprovalResolved, "fl2");
        r2.route_id = r1.route_id.clone(); r2.pending_approval_id = r1.pending_approval_id.clone();
        let p2 = save_workflow_action_outcome(&d, &r2).unwrap();
        assert!(p2.exists());
    }
    #[test] fn tool_completed_cannot_duplicate_with_different_key() {
        let d = test_dir(); let r1 = test_outcome(WorkflowActionOutcomeStatus::ToolCompleted, "tc1");
        save_workflow_action_outcome(&d, &r1).unwrap();
        let mut r2 = test_outcome(WorkflowActionOutcomeStatus::ToolCompleted, "tc2");
        r2.route_id = r1.route_id.clone(); r2.pending_approval_id = r1.pending_approval_id.clone();
        let p2 = save_workflow_action_outcome(&d, &r2).unwrap();
        assert!(p2.to_string_lossy().contains("tc1"));
    }
    #[test] fn tool_denied_cannot_duplicate_with_different_key() {
        let d = test_dir(); let r1 = test_outcome(WorkflowActionOutcomeStatus::ToolDenied, "td1");
        save_workflow_action_outcome(&d, &r1).unwrap();
        let mut r2 = test_outcome(WorkflowActionOutcomeStatus::ToolDenied, "td2");
        r2.route_id = r1.route_id.clone(); r2.pending_approval_id = r1.pending_approval_id.clone();
        let p2 = save_workflow_action_outcome(&d, &r2).unwrap();
        assert!(p2.to_string_lossy().contains("td1"));
    }
    #[test] fn outcome_writes_only_outcome_evidence() {
        let d = test_dir(); let r = test_outcome(WorkflowActionOutcomeStatus::ApprovalResolved, "wo");
        save_workflow_action_outcome(&d, &r).unwrap();
        assert!(outcomes_root(&d).exists());
        assert!(!d.join("workflow_runs").exists());
        assert!(!d.join("workflow_action_routes").exists());
        assert!(!d.join("task_plans").exists());
        assert!(!d.join("approvals").exists());
    }
    #[test] fn outcome_does_not_write_workflow_run_records() {
        let d = test_dir(); save_workflow_action_outcome(&d, &test_outcome(WorkflowActionOutcomeStatus::ApprovalResolved, "nr")).unwrap();
        assert!(!d.join("workflow_runs").exists());
    }
    #[test] fn outcome_does_not_write_route_records() {
        let d = test_dir(); save_workflow_action_outcome(&d, &test_outcome(WorkflowActionOutcomeStatus::ApprovalResolved, "nrr")).unwrap();
        assert!(!d.join("workflow_action_routes").exists());
    }
    #[test] fn outcome_does_not_write_approval_records_directly() {
        let d = test_dir(); save_workflow_action_outcome(&d, &test_outcome(WorkflowActionOutcomeStatus::ApprovalResolved, "na")).unwrap();
        assert!(!d.join("approvals").exists());
    }
    #[test] fn outcome_does_not_write_session_state_directly() {
        // Patch 5: explicit forbidden write test for session state
        let d = test_dir(); save_workflow_action_outcome(&d, &test_outcome(WorkflowActionOutcomeStatus::ApprovalResolved, "ns")).unwrap();
        assert!(!d.join("sessions").exists());
        assert!(!d.join("session_state").exists());
        // Only the by_session index pointer exists
        assert!(d.join("workflow_action_outcomes/by_session").exists());
    }
}
