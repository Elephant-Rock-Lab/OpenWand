//! Routing readiness persistence.
//!
//! Readiness under workflow_routing_readiness/. Does not mutate review,
//! continuation, workflow run/revision, route, outcome, reconciliation,
//! session, approval, trace, memory, tool, git, or provider records.

use std::path::Path;

use openwand_workflow::workflow_routing_readiness::*;

fn readiness_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_routing_readiness")
}
fn records_dir(store_root: &Path) -> std::path::PathBuf {
    readiness_root(store_root).join("records")
}

fn index_file(root: &Path, index_name: &str, key: &str) -> std::path::PathBuf {
    root.join(index_name).join(format!("{}.json", key))
}

pub fn save_workflow_routing_readiness(
    store_root: &Path,
    record: &WorkflowRoutingReadinessRecord,
) -> Result<std::path::PathBuf, String> {
    let dir = records_dir(store_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Dir: {}", e))?;

    // Idempotency
    if let Ok(existing) = list_workflow_routing_readiness(store_root) {
        for er in &existing {
            if er.proposal_id == record.proposal_id
                && er.review_id == record.review_id
                && er.source_run_revision_id == record.source_run_revision_id
            {
                if er.readiness_id == record.readiness_id {
                    return Ok(dir.join(format!("{}.json", er.readiness_id.0)));
                }
                // Ready cannot duplicate
                if matches!(er.status, WorkflowRoutingReadinessStatus::Ready)
                    && matches!(record.status, WorkflowRoutingReadinessStatus::Ready)
                {
                    return Ok(dir.join(format!("{}.json", er.readiness_id.0)));
                }
            }
        }
    }

    let path = dir.join(format!("{}.json", record.readiness_id.0));
    let json = serde_json::to_string_pretty(record).map_err(|e| format!("Serialize: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Write: {}", e))?;

    std::fs::write(dir.join("latest.json"), record.readiness_id.0.as_bytes())
        .map_err(|e| format!("Latest: {}", e))?;

    let root = readiness_root(store_root);
    for (idx_name, key) in [
        ("by_proposal", record.proposal_id.0.as_str()),
        ("by_review", record.review_id.0.as_str()),
        ("by_workflow_run", record.workflow_execution_id.0.as_str()),
        ("by_run_revision", record.source_run_revision_id.0.as_str()),
    ] {
        let idx_file = index_file(&root, idx_name, key);
        std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Index dir: {}", e))?;
        std::fs::write(&idx_file, record.readiness_id.0.as_bytes()).map_err(|e| format!("Index: {}", e))?;
    }
    if let Some(ref c) = record.candidate {
        let idx_file = index_file(&root, "by_stage", &c.stage_id);
        std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Index dir: {}", e))?;
        std::fs::write(&idx_file, record.readiness_id.0.as_bytes()).map_err(|e| format!("Index: {}", e))?;
        if let Some(ref ar) = c.action_request_id {
            let idx_file = index_file(&root, "by_action_request", ar);
            std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Index dir: {}", e))?;
            std::fs::write(&idx_file, record.readiness_id.0.as_bytes()).map_err(|e| format!("Index: {}", e))?;
        }
    }

    Ok(path)
}

pub fn load_workflow_routing_readiness(store_root: &Path, id: &WorkflowRoutingReadinessId) -> Result<WorkflowRoutingReadinessRecord, String> {
    let path = records_dir(store_root).join(format!("{}.json", id.0));
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Parse: {}", e))
}

pub fn list_workflow_routing_readiness(store_root: &Path) -> Result<Vec<WorkflowRoutingReadinessRecord>, String> {
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
                && let Ok(record) = serde_json::from_str::<WorkflowRoutingReadinessRecord>(&json) {
                    records.push(record);
                }
        }
    }
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(records)
}

pub fn latest_workflow_routing_readiness(store_root: &Path) -> Result<Option<WorkflowRoutingReadinessRecord>, String> {
    let p = records_dir(store_root).join("latest.json");
    if !p.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&p).map_err(|e| format!("{}", e))?;
    load_workflow_routing_readiness(store_root, &WorkflowRoutingReadinessId(id.trim().into())).map(Some)
}

pub fn readiness_by_proposal(store_root: &Path, id: &str) -> Result<Option<WorkflowRoutingReadinessRecord>, String> {
    load_index(store_root, "by_proposal", id)
}
pub fn readiness_by_review(store_root: &Path, id: &str) -> Result<Option<WorkflowRoutingReadinessRecord>, String> {
    load_index(store_root, "by_review", id)
}
pub fn readiness_by_workflow_run(store_root: &Path, id: &str) -> Result<Option<WorkflowRoutingReadinessRecord>, String> {
    load_index(store_root, "by_workflow_run", id)
}
pub fn readiness_by_run_revision(store_root: &Path, id: &str) -> Result<Option<WorkflowRoutingReadinessRecord>, String> {
    load_index(store_root, "by_run_revision", id)
}
pub fn readiness_by_stage(store_root: &Path, id: &str) -> Result<Option<WorkflowRoutingReadinessRecord>, String> {
    load_index(store_root, "by_stage", id)
}
pub fn readiness_by_action_request(store_root: &Path, id: &str) -> Result<Option<WorkflowRoutingReadinessRecord>, String> {
    load_index(store_root, "by_action_request", id)
}

fn load_index(store_root: &Path, index_name: &str, key: &str) -> Result<Option<WorkflowRoutingReadinessRecord>, String> {
    let pointer = index_file(&readiness_root(store_root), index_name, key);
    if !pointer.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&pointer).map_err(|e| format!("{}", e))?;
    load_workflow_routing_readiness(store_root, &WorkflowRoutingReadinessId(id.trim().into())).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_continuation::*;
    use openwand_workflow::workflow_next_action_review::WorkflowNextActionReviewId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;
    use chrono::Utc;

    fn test_dir() -> std::path::PathBuf { tempfile::tempdir().unwrap().into_path() }

    fn test_readiness(status: WorkflowRoutingReadinessStatus, suffix: &str) -> WorkflowRoutingReadinessRecord {
        WorkflowRoutingReadinessRecord {
            readiness_id: WorkflowRoutingReadinessId(format!("wrrd_{}", suffix)),
            proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
            review_id: WorkflowNextActionReviewId("wnar_t".into()),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            proposal_hash: "ph".into(), run_revision_hash: "rh".into(),
            status: status.clone(),
            decision: match &status {
                WorkflowRoutingReadinessStatus::Ready => WorkflowRoutingReadinessDecision::Ready { summary: "ok".into() },
                WorkflowRoutingReadinessStatus::Blocked => WorkflowRoutingReadinessDecision::Blocked { reason_code: "test".into(), summary: "blocked".into() },
                WorkflowRoutingReadinessStatus::Inconclusive => WorkflowRoutingReadinessDecision::Inconclusive { reason_code: "test".into(), summary: "inconclusive".into() },
            },
            predicates: vec![], candidate: None, route_request_preview: None,
            created_at: Utc::now(),
        }
    }

    #[test] fn routing_readiness_persists_and_loads_roundtrip() {
        let d = test_dir(); let r = test_readiness(WorkflowRoutingReadinessStatus::Ready, "rt");
        save_workflow_routing_readiness(&d, &r).unwrap();
        let l = load_workflow_routing_readiness(&d, &r.readiness_id).unwrap();
        assert_eq!(r.readiness_id, l.readiness_id);
    }
    #[test] fn latest_routing_readiness_returns_expected() {
        let d = test_dir(); let r = test_readiness(WorkflowRoutingReadinessStatus::Ready, "lt");
        save_workflow_routing_readiness(&d, &r).unwrap();
        assert_eq!(r.readiness_id, latest_workflow_routing_readiness(&d).unwrap().unwrap().readiness_id);
    }
    #[test] fn routing_readiness_by_proposal_returns_expected() {
        let d = test_dir(); let r = test_readiness(WorkflowRoutingReadinessStatus::Ready, "bp");
        save_workflow_routing_readiness(&d, &r).unwrap();
        assert_eq!(r.readiness_id, readiness_by_proposal(&d, "wnap_t").unwrap().unwrap().readiness_id);
    }
    #[test] fn routing_readiness_by_review_returns_expected() {
        let d = test_dir(); let r = test_readiness(WorkflowRoutingReadinessStatus::Ready, "br");
        save_workflow_routing_readiness(&d, &r).unwrap();
        assert_eq!(r.readiness_id, readiness_by_review(&d, "wnar_t").unwrap().unwrap().readiness_id);
    }
    #[test] fn routing_readiness_by_workflow_run_returns_expected() {
        let d = test_dir(); let r = test_readiness(WorkflowRoutingReadinessStatus::Ready, "wr");
        save_workflow_routing_readiness(&d, &r).unwrap();
        assert_eq!(r.readiness_id, readiness_by_workflow_run(&d, "wfx_t").unwrap().unwrap().readiness_id);
    }
    #[test] fn routing_readiness_by_run_revision_returns_expected() {
        let d = test_dir(); let r = test_readiness(WorkflowRoutingReadinessStatus::Ready, "rr");
        save_workflow_routing_readiness(&d, &r).unwrap();
        assert_eq!(r.readiness_id, readiness_by_run_revision(&d, "wrr_t").unwrap().unwrap().readiness_id);
    }
    #[test] fn same_idempotency_key_returns_existing_routing_readiness() {
        let d = test_dir(); let r = test_readiness(WorkflowRoutingReadinessStatus::Ready, "id");
        let p1 = save_workflow_routing_readiness(&d, &r).unwrap();
        let p2 = save_workflow_routing_readiness(&d, &r).unwrap();
        assert_eq!(p1, p2);
    }
    #[test] fn ready_routing_readiness_cannot_duplicate_with_different_key() {
        let d = test_dir();
        let r1 = test_readiness(WorkflowRoutingReadinessStatus::Ready, "rd1");
        save_workflow_routing_readiness(&d, &r1).unwrap();
        let mut r2 = test_readiness(WorkflowRoutingReadinessStatus::Ready, "rd2");
        r2.proposal_id = r1.proposal_id.clone(); r2.review_id = r1.review_id.clone();
        r2.source_run_revision_id = r1.source_run_revision_id.clone();
        let p2 = save_workflow_routing_readiness(&d, &r2).unwrap();
        assert!(p2.to_string_lossy().contains("rd1"));
    }
    #[test] fn blocked_routing_readiness_can_retry_with_new_key() {
        let d = test_dir();
        let r1 = test_readiness(WorkflowRoutingReadinessStatus::Blocked, "bk1");
        save_workflow_routing_readiness(&d, &r1).unwrap();
        let mut r2 = test_readiness(WorkflowRoutingReadinessStatus::Ready, "bk2");
        r2.proposal_id = r1.proposal_id.clone(); r2.review_id = r1.review_id.clone();
        r2.source_run_revision_id = r1.source_run_revision_id.clone();
        let p2 = save_workflow_routing_readiness(&d, &r2).unwrap();
        assert!(p2.exists());
    }
    #[test] fn inconclusive_routing_readiness_can_retry_with_new_key() {
        let d = test_dir();
        let r1 = test_readiness(WorkflowRoutingReadinessStatus::Inconclusive, "ic1");
        save_workflow_routing_readiness(&d, &r1).unwrap();
        let mut r2 = test_readiness(WorkflowRoutingReadinessStatus::Ready, "ic2");
        r2.proposal_id = r1.proposal_id.clone(); r2.review_id = r1.review_id.clone();
        r2.source_run_revision_id = r1.source_run_revision_id.clone();
        let p2 = save_workflow_routing_readiness(&d, &r2).unwrap();
        assert!(p2.exists());
    }
    #[test] fn review_readiness_writes_only_review_and_readiness_evidence() {
        let d = test_dir();
        save_workflow_routing_readiness(&d, &test_readiness(WorkflowRoutingReadinessStatus::Ready, "wo")).unwrap();
        assert!(readiness_root(&d).exists());
        assert!(!d.join("workflow_next_action_reviews").exists());
        assert!(!d.join("workflow_runs").exists());
        assert!(!d.join("workflow_continuation").exists());
    }
    #[test] fn does_not_write_route_outcome_reconciliation_or_revision_records() {
        let d = test_dir();
        save_workflow_routing_readiness(&d, &test_readiness(WorkflowRoutingReadinessStatus::Ready, "nr")).unwrap();
        assert!(!d.join("workflow_action_routes").exists());
        assert!(!d.join("workflow_action_outcomes").exists());
        assert!(!d.join("workflow_reconciliations").exists());
        assert!(!d.join("workflow_run_revisions").exists());
    }
    #[test] fn does_not_write_approval_or_session_records() {
        let d = test_dir();
        save_workflow_routing_readiness(&d, &test_readiness(WorkflowRoutingReadinessStatus::Ready, "ns")).unwrap();
        assert!(!d.join("approvals").exists());
        assert!(!d.join("sessions").exists());
    }
    // Patch 2: explicit no-route-record proof
    #[test] fn ready_routing_readiness_does_not_create_workflow_action_route_record() {
        let d = test_dir();
        save_workflow_routing_readiness(&d, &test_readiness(WorkflowRoutingReadinessStatus::Ready, "nr2")).unwrap();
        assert!(!d.join("workflow_action_routes").exists());
        let records = list_workflow_routing_readiness(&d).unwrap();
        let rec = records.first().unwrap();
        if let Some(ref preview) = rec.route_request_preview {
            assert!(preview.descriptive_only);
            assert!(!preview.creates_route_now);
        }
    }
}
