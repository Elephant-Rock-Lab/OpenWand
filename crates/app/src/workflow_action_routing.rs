//! Workflow action routing persistence.
//!
//! Route records are durable linkage evidence under eval_reports/workflow_action_routes/.
//! No tool execution, trace append, memory write, shell/git, approval persistence,
//! or session state mutation.

use std::path::Path;

use openwand_workflow::workflow_action_route::{
    WorkflowActionRouteId, WorkflowActionRouteRecord, WorkflowActionRouteStatus,
};

fn action_routes_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_action_routes")
}

fn records_dir(store_root: &Path) -> std::path::PathBuf {
    action_routes_root(store_root).join("records")
}

fn by_workflow_run_dir(store_root: &Path) -> std::path::PathBuf {
    action_routes_root(store_root).join("by_workflow_run")
}

fn by_stage_dir(store_root: &Path) -> std::path::PathBuf {
    action_routes_root(store_root).join("by_stage")
}

fn by_action_request_dir(store_root: &Path) -> std::path::PathBuf {
    action_routes_root(store_root).join("by_action_request")
}

fn by_session_dir(store_root: &Path) -> std::path::PathBuf {
    action_routes_root(store_root).join("by_session")
}

/// Save a workflow action route record.
pub fn save_workflow_action_route(
    store_root: &Path,
    record: &WorkflowActionRouteRecord,
) -> Result<std::path::PathBuf, String> {
    let dir = records_dir(store_root);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create workflow_action_routes dir: {}", e))?;

    // Idempotency: same key returns existing
    if let Ok(existing) = list_workflow_action_routes(store_root) {
        for er in &existing {
            if er.workflow_execution_id == record.workflow_execution_id
                && er.stage_id == record.stage_id
                && er.action_request_id == record.action_request_id
            {
                // Same route_id → return existing
                if er.route_id == record.route_id {
                    let path = dir.join(format!("{}.json", er.route_id.0));
                    return Ok(path);
                }
                // Completed/SuspendedForApproval cannot duplicate even with different key
                if matches!(er.status, WorkflowActionRouteStatus::Completed | WorkflowActionRouteStatus::SuspendedForApproval)
                    && matches!(record.status, WorkflowActionRouteStatus::Completed | WorkflowActionRouteStatus::SuspendedForApproval)
                {
                    let path = dir.join(format!("{}.json", er.route_id.0));
                    return Ok(path);
                }
            }
        }
    }

    let path = dir.join(format!("{}.json", record.route_id.0));
    let json = serde_json::to_string_pretty(record)
        .map_err(|e| format!("Failed to serialize route: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write route: {}", e))?;

    // Latest pointer
    let latest_path = dir.join("latest.json");
    std::fs::write(&latest_path, record.route_id.0.as_bytes())
        .map_err(|e| format!("Failed to write latest: {}", e))?;

    // Indexes
    for (idx_dir, key) in [
        (by_workflow_run_dir(store_root), record.workflow_execution_id.0.as_str()),
        (by_stage_dir(store_root), record.stage_id.as_str()),
        (by_action_request_dir(store_root), record.action_request_id.as_str()),
    ] {
        std::fs::create_dir_all(&idx_dir)
            .map_err(|e| format!("Failed to create index dir: {}", e))?;
        std::fs::write(idx_dir.join(format!("{}.json", key)), record.route_id.0.as_bytes())
            .map_err(|e| format!("Failed to write index: {}", e))?;
    }

    // Session index (only if session route exists)
    if let Some(ref session_route) = record.session_route {
        let session_idx = by_session_dir(store_root);
        std::fs::create_dir_all(&session_idx)
            .map_err(|e| format!("Failed to create session index dir: {}", e))?;
        std::fs::write(
            session_idx.join(format!("{}.json", session_route.session_id)),
            record.route_id.0.as_bytes(),
        ).map_err(|e| format!("Failed to write session index: {}", e))?;
    }

    Ok(path)
}

/// Load a route by ID.
pub fn load_workflow_action_route(store_root: &Path, route_id: &WorkflowActionRouteId) -> Result<WorkflowActionRouteRecord, String> {
    let path = records_dir(store_root).join(format!("{}.json", route_id.0));
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read route {}: {}", route_id.0, e))?;
    serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse route {}: {}", route_id.0, e))
}

/// List all routes.
pub fn list_workflow_action_routes(store_root: &Path) -> Result<Vec<WorkflowActionRouteRecord>, String> {
    let dir = records_dir(store_root);
    if !dir.exists() { return Ok(vec![]); }
    let mut records = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("Failed to read dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "json") {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            if name == "latest" { continue; }
            let json = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
            if let Ok(record) = serde_json::from_str::<WorkflowActionRouteRecord>(&json) {
                records.push(record);
            }
        }
    }
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(records)
}

/// Get latest route.
pub fn latest_workflow_action_route(store_root: &Path) -> Result<Option<WorkflowActionRouteRecord>, String> {
    let latest_path = records_dir(store_root).join("latest.json");
    if !latest_path.exists() { return Ok(None); }
    let id_str = std::fs::read_to_string(&latest_path)
        .map_err(|e| format!("Failed to read latest: {}", e))?;
    load_workflow_action_route(store_root, &WorkflowActionRouteId(id_str.trim().into())).map(Some)
}

/// Get route by workflow run.
pub fn route_by_workflow_run(store_root: &Path, execution_id: &str) -> Result<Option<WorkflowActionRouteRecord>, String> {
    let pointer = by_workflow_run_dir(store_root).join(format!("{}.json", execution_id));
    if !pointer.exists() { return Ok(None); }
    let id_str = std::fs::read_to_string(&pointer).map_err(|e| format!("Failed: {}", e))?;
    load_workflow_action_route(store_root, &WorkflowActionRouteId(id_str.trim().into())).map(Some)
}

/// Get route by stage.
pub fn route_by_stage(store_root: &Path, stage_id: &str) -> Result<Option<WorkflowActionRouteRecord>, String> {
    let pointer = by_stage_dir(store_root).join(format!("{}.json", stage_id));
    if !pointer.exists() { return Ok(None); }
    let id_str = std::fs::read_to_string(&pointer).map_err(|e| format!("Failed: {}", e))?;
    load_workflow_action_route(store_root, &WorkflowActionRouteId(id_str.trim().into())).map(Some)
}

/// Get route by action request.
pub fn route_by_action_request(store_root: &Path, action_request_id: &str) -> Result<Option<WorkflowActionRouteRecord>, String> {
    let pointer = by_action_request_dir(store_root).join(format!("{}.json", action_request_id));
    if !pointer.exists() { return Ok(None); }
    let id_str = std::fs::read_to_string(&pointer).map_err(|e| format!("Failed: {}", e))?;
    load_workflow_action_route(store_root, &WorkflowActionRouteId(id_str.trim().into())).map(Some)
}

/// Get route by session.
pub fn route_by_session(store_root: &Path, session_id: &str) -> Result<Option<WorkflowActionRouteRecord>, String> {
    let pointer = by_session_dir(store_root).join(format!("{}.json", session_id));
    if !pointer.exists() { return Ok(None); }
    let id_str = std::fs::read_to_string(&pointer).map_err(|e| format!("Failed: {}", e))?;
    load_workflow_action_route(store_root, &WorkflowActionRouteId(id_str.trim().into())).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_action_route::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_readiness::WorkflowReadinessId;
    use openwand_workflow::workflow_proposal::WorkflowProposalId;
    use openwand_workflow::workflow_proposal_review::WorkflowProposalReviewId;
    use chrono::Utc;

    fn test_dir() -> std::path::PathBuf { tempfile::tempdir().unwrap().into_path() }

    fn test_route(status: WorkflowActionRouteStatus, id_suffix: &str) -> WorkflowActionRouteRecord {
        let mut record = WorkflowActionRouteRecord {
            route_id: WorkflowActionRouteId(format!("war_{}", id_suffix)),
            workflow_execution_id: WorkflowExecutionId("wfx_test".into()),
            readiness_id: WorkflowReadinessId("wfrd_test".into()),
            proposal_id: WorkflowProposalId("wfp_test".into()),
            stage_id: "stage_1".into(),
            action_request_id: "ar_1".into(),
            action_request_hash: "hash".into(),
            status: status.clone(),
            decision: match &status {
                WorkflowActionRouteStatus::Routed => WorkflowActionRouteDecision::Routed,
                WorkflowActionRouteStatus::Completed => WorkflowActionRouteDecision::Completed { summary: "done".into() },
                WorkflowActionRouteStatus::SuspendedForApproval => WorkflowActionRouteDecision::SuspendedForApproval {
                    approval_request_id: "arid_1".into(), summary: "awaiting".into(),
                },
                WorkflowActionRouteStatus::Blocked => WorkflowActionRouteDecision::Blocked {
                    reason_code: "test".into(), summary: "blocked".into(),
                },
                _ => WorkflowActionRouteDecision::Failed { reason_code: "test".into(), summary: "failed".into() },
            },
            predicates: vec![],
            session_route: Some(WorkflowSessionRouteSnapshot {
                session_id: format!("sess_{}", id_suffix),
                session_run_id: Some("run_1".into()),
                trace_ids: vec!["trace_1".into()],
                pending_approval_id: None,
                tool_call_id: None,
                tool_name_observed_from_session: None,
                session_status: "completed".into(),
            }),
            route_prompt: WorkflowActionRoutePrompt {
                capability_category: "c".into(), purpose: "p".into(),
                expected_input_summary: "i".into(), expected_output_summary: "o".into(),
                safety_constraints: vec![],
            },
            created_at: Utc::now(),
            completed_at: None,
        };
        if status == WorkflowActionRouteStatus::Completed {
            record.completed_at = Some(Utc::now());
        }
        record
    }

    #[test]
    fn route_persists_and_loads_roundtrip() {
        let dir = test_dir();
        let record = test_route(WorkflowActionRouteStatus::Routed, "abc");
        save_workflow_action_route(&dir, &record).unwrap();
        let loaded = load_workflow_action_route(&dir, &record.route_id).unwrap();
        assert_eq!(record.route_id, loaded.route_id);
    }

    #[test]
    fn latest_route_returns_expected() {
        let dir = test_dir();
        let record = test_route(WorkflowActionRouteStatus::Routed, "def");
        save_workflow_action_route(&dir, &record).unwrap();
        let latest = latest_workflow_action_route(&dir).unwrap().unwrap();
        assert_eq!(record.route_id, latest.route_id);
    }

    #[test]
    fn route_by_workflow_run_returns_expected() {
        let dir = test_dir();
        let record = test_route(WorkflowActionRouteStatus::Routed, "ghi");
        save_workflow_action_route(&dir, &record).unwrap();
        let found = route_by_workflow_run(&dir, "wfx_test").unwrap().unwrap();
        assert_eq!(record.route_id, found.route_id);
    }

    #[test]
    fn route_by_stage_returns_expected() {
        let dir = test_dir();
        let record = test_route(WorkflowActionRouteStatus::Routed, "jkl");
        save_workflow_action_route(&dir, &record).unwrap();
        let found = route_by_stage(&dir, "stage_1").unwrap().unwrap();
        assert_eq!(record.route_id, found.route_id);
    }

    #[test]
    fn route_by_action_request_returns_expected() {
        let dir = test_dir();
        let record = test_route(WorkflowActionRouteStatus::Routed, "mno");
        save_workflow_action_route(&dir, &record).unwrap();
        let found = route_by_action_request(&dir, "ar_1").unwrap().unwrap();
        assert_eq!(record.route_id, found.route_id);
    }

    #[test]
    fn route_by_session_returns_expected() {
        let dir = test_dir();
        let record = test_route(WorkflowActionRouteStatus::Routed, "pqr");
        save_workflow_action_route(&dir, &record).unwrap();
        let found = route_by_session(&dir, "sess_pqr").unwrap().unwrap();
        assert_eq!(record.route_id, found.route_id);
    }

    #[test]
    fn same_idempotency_key_returns_existing() {
        let dir = test_dir();
        let record = test_route(WorkflowActionRouteStatus::Routed, "stu");
        let p1 = save_workflow_action_route(&dir, &record).unwrap();
        let p2 = save_workflow_action_route(&dir, &record).unwrap();
        assert_eq!(p1, p2);
    }

    #[test]
    fn completed_route_cannot_duplicate_with_different_key() {
        let dir = test_dir();
        let mut r1 = test_route(WorkflowActionRouteStatus::Completed, "vwx1");
        save_workflow_action_route(&dir, &r1).unwrap();
        let mut r2 = test_route(WorkflowActionRouteStatus::Completed, "vwx2");
        // Same execution_id, stage_id, action_request_id but different route_id
        r2.route_id = WorkflowActionRouteId("war_vwx2".into());
        let p2 = save_workflow_action_route(&dir, &r2).unwrap();
        // Should return existing path, not new
        assert!(p2.to_string_lossy().contains("vwx1"), "Should return existing completed route");
    }

    #[test]
    fn suspended_route_cannot_duplicate_with_different_key() {
        let dir = test_dir();
        let r1 = test_route(WorkflowActionRouteStatus::SuspendedForApproval, "yz1");
        save_workflow_action_route(&dir, &r1).unwrap();
        let mut r2 = test_route(WorkflowActionRouteStatus::SuspendedForApproval, "yz2");
        r2.route_id = WorkflowActionRouteId("war_yz2".into());
        let p2 = save_workflow_action_route(&dir, &r2).unwrap();
        assert!(p2.to_string_lossy().contains("yz1"), "Should return existing suspended route");
    }

    #[test]
    fn blocked_route_can_retry_with_new_key() {
        let dir = test_dir();
        let r1 = test_route(WorkflowActionRouteStatus::Blocked, "blk1");
        save_workflow_action_route(&dir, &r1).unwrap();
        let mut r2 = test_route(WorkflowActionRouteStatus::Routed, "blk2");
        r2.route_id = WorkflowActionRouteId("war_blk2".into());
        let p2 = save_workflow_action_route(&dir, &r2).unwrap();
        assert!(p2.exists(), "Blocked → Routed retry should create new record");
    }

    #[test]
    fn route_writes_only_workflow_action_route_evidence() {
        let dir = test_dir();
        let record = test_route(WorkflowActionRouteStatus::Routed, "write_test");
        save_workflow_action_route(&dir, &record).unwrap();
        assert!(action_routes_root(&dir).exists());
        assert!(!dir.join("workflow_runs").exists());
        assert!(!dir.join("workflow_proposals").exists());
        assert!(!dir.join("task_plans").exists());
    }

    #[test]
    fn route_does_not_write_workflow_run_records() {
        let dir = test_dir();
        save_workflow_action_route(&dir, &test_route(WorkflowActionRouteStatus::Routed, "nowr")).unwrap();
        assert!(!dir.join("workflow_runs").exists());
    }

    #[test]
    fn route_does_not_write_readiness_records() {
        let dir = test_dir();
        save_workflow_action_route(&dir, &test_route(WorkflowActionRouteStatus::Routed, "nord")).unwrap();
        assert!(!dir.join("workflow_readiness").exists());
    }

    #[test]
    fn route_does_not_write_approval_records() {
        // Patch 5: explicit forbidden write test for approval records
        let dir = test_dir();
        save_workflow_action_route(&dir, &test_route(WorkflowActionRouteStatus::Routed, "noappr")).unwrap();
        assert!(!dir.join("approvals").exists());
        assert!(!dir.join("approval_records").exists());
    }

    #[test]
    fn route_does_not_write_session_state_directly() {
        // Patch 5: routing does not create/mutate session state files
        let dir = test_dir();
        save_workflow_action_route(&dir, &test_route(WorkflowActionRouteStatus::Routed, "noss")).unwrap();
        assert!(!dir.join("sessions").exists());
        assert!(!dir.join("session_state").exists());
        // Only the by_session index pointer exists
        assert!(dir.join("workflow_action_routes/by_session").exists());
    }
}
