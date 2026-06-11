//! Workflow continuation persistence.
//!
//! Two evidence areas:
//!   workflow_continuation/readiness/ — readiness records + indexes
//!   workflow_continuation/proposals/ — next-action proposal records + indexes
//!
//! Does not mutate workflow runs, run revisions, routes, outcomes, reconciliations,
//! readiness/proposal records, task plans, governance, trace, memory, session,
//! approval, tool, git, or provider records.

use std::path::Path;

use openwand_workflow::workflow_continuation::*;

fn continuation_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_continuation")
}
fn readiness_dir(store_root: &Path) -> std::path::PathBuf {
    continuation_root(store_root).join("readiness").join("records")
}
fn proposals_dir(store_root: &Path) -> std::path::PathBuf {
    continuation_root(store_root).join("proposals").join("records")
}

fn index_file(root: &Path, index_name: &str, key: &str) -> std::path::PathBuf {
    let dir = root.join(index_name);
    dir.join(format!("{}.json", key))
}

// --- Readiness ---

pub fn save_continuation_readiness(
    store_root: &Path,
    record: &WorkflowContinuationReadinessRecord,
) -> Result<std::path::PathBuf, String> {
    let dir = readiness_dir(store_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Dir: {}", e))?;

    // Idempotency: same run + revision + key → return existing
    if let Ok(existing) = list_continuation_readiness(store_root) {
        for er in &existing {
            if er.workflow_execution_id == record.workflow_execution_id
                && er.latest_run_revision_id == record.latest_run_revision_id
            {
                if er.readiness_id == record.readiness_id {
                    return Ok(dir.join(format!("{}.json", er.readiness_id.0)));
                }
                // NoEligibleAction cannot duplicate for same revision (Patch 3)
                if matches!(er.status, WorkflowContinuationStatus::NoEligibleAction)
                    && matches!(record.status, WorkflowContinuationStatus::NoEligibleAction)
                {
                    return Ok(dir.join(format!("{}.json", er.readiness_id.0)));
                }
                // ProposalReady cannot duplicate
                if matches!(er.status, WorkflowContinuationStatus::ProposalReady)
                    && matches!(record.status, WorkflowContinuationStatus::ProposalReady)
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

    let root = continuation_root(store_root).join("readiness");
    for (idx_name, key) in [
        ("by_workflow_run", record.workflow_execution_id.0.as_str()),
        ("by_run_revision", record.latest_run_revision_id.0.as_str()),
    ] {
        let idx_file = index_file(&root, idx_name, key);
        std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Index dir: {}", e))?;
        std::fs::write(&idx_file, record.readiness_id.0.as_bytes()).map_err(|e| format!("Index: {}", e))?;
    }

    Ok(path)
}

pub fn load_continuation_readiness(store_root: &Path, id: &WorkflowContinuationReadinessId) -> Result<WorkflowContinuationReadinessRecord, String> {
    let path = readiness_dir(store_root).join(format!("{}.json", id.0));
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Parse: {}", e))
}

pub fn list_continuation_readiness(store_root: &Path) -> Result<Vec<WorkflowContinuationReadinessRecord>, String> {
    let dir = readiness_dir(store_root);
    if !dir.exists() { return Ok(vec![]); }
    let mut records = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("Dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Entry: {}", e))?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            if name == "latest" { continue; }
            if let Ok(json) = std::fs::read_to_string(&path)
                && let Ok(record) = serde_json::from_str::<WorkflowContinuationReadinessRecord>(&json) {
                    records.push(record);
                }
        }
    }
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(records)
}

pub fn latest_continuation_readiness(store_root: &Path) -> Result<Option<WorkflowContinuationReadinessRecord>, String> {
    let p = readiness_dir(store_root).join("latest.json");
    if !p.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&p).map_err(|e| format!("{}", e))?;
    load_continuation_readiness(store_root, &WorkflowContinuationReadinessId(id.trim().into())).map(Some)
}

pub fn readiness_by_workflow_run(store_root: &Path, id: &str) -> Result<Option<WorkflowContinuationReadinessRecord>, String> {
    load_index(&continuation_root(store_root).join("readiness"), "by_workflow_run", id)
}
pub fn readiness_by_run_revision(store_root: &Path, id: &str) -> Result<Option<WorkflowContinuationReadinessRecord>, String> {
    load_index(&continuation_root(store_root).join("readiness"), "by_run_revision", id)
}

// --- Proposals ---

pub fn save_next_action_proposal(
    store_root: &Path,
    proposal: &WorkflowNextActionProposal,
) -> Result<std::path::PathBuf, String> {
    let dir = proposals_dir(store_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Dir: {}", e))?;

    // Idempotency: same run + revision + stage → return existing
    if let Ok(existing) = list_next_action_proposals(store_root) {
        for ep in &existing {
            if ep.workflow_execution_id == proposal.workflow_execution_id
                && ep.source_run_revision_id == proposal.source_run_revision_id
                && ep.candidate.stage_id == proposal.candidate.stage_id
                && ep.proposal_id != proposal.proposal_id
            {
                return Ok(dir.join(format!("{}.json", ep.proposal_id.0)));
            }
        }
    }

    let path = dir.join(format!("{}.json", proposal.proposal_id.0));
    let json = serde_json::to_string_pretty(proposal).map_err(|e| format!("Serialize: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Write: {}", e))?;

    std::fs::write(dir.join("latest.json"), proposal.proposal_id.0.as_bytes())
        .map_err(|e| format!("Latest: {}", e))?;

    let root = continuation_root(store_root).join("proposals");
    for (idx_name, key) in [
        ("by_workflow_run", proposal.workflow_execution_id.0.as_str()),
        ("by_run_revision", proposal.source_run_revision_id.0.as_str()),
        ("by_stage", proposal.candidate.stage_id.as_str()),
    ] {
        let key_val = if idx_name == "by_action_request" {
            proposal.candidate.action_request_id.as_deref().unwrap_or("none")
        } else { key };
        let idx_file = index_file(&root, idx_name, key_val);
        std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Index dir: {}", e))?;
        std::fs::write(&idx_file, proposal.proposal_id.0.as_bytes()).map_err(|e| format!("Index: {}", e))?;
    }
    // by_action_request index
    if let Some(ar_id) = &proposal.candidate.action_request_id {
        let idx_file = index_file(&root, "by_action_request", ar_id);
        std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Index dir: {}", e))?;
        std::fs::write(&idx_file, proposal.proposal_id.0.as_bytes()).map_err(|e| format!("Index: {}", e))?;
    }

    Ok(path)
}

pub fn load_next_action_proposal(store_root: &Path, id: &WorkflowNextActionProposalId) -> Result<WorkflowNextActionProposal, String> {
    let path = proposals_dir(store_root).join(format!("{}.json", id.0));
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Parse: {}", e))
}

pub fn list_next_action_proposals(store_root: &Path) -> Result<Vec<WorkflowNextActionProposal>, String> {
    let dir = proposals_dir(store_root);
    if !dir.exists() { return Ok(vec![]); }
    let mut proposals = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("Dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Entry: {}", e))?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            if name == "latest" { continue; }
            if let Ok(json) = std::fs::read_to_string(&path)
                && let Ok(proposal) = serde_json::from_str::<WorkflowNextActionProposal>(&json) {
                    proposals.push(proposal);
                }
        }
    }
    proposals.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(proposals)
}

pub fn latest_next_action_proposal(store_root: &Path) -> Result<Option<WorkflowNextActionProposal>, String> {
    let p = proposals_dir(store_root).join("latest.json");
    if !p.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&p).map_err(|e| format!("{}", e))?;
    load_next_action_proposal(store_root, &WorkflowNextActionProposalId(id.trim().into())).map(Some)
}

pub fn proposal_by_workflow_run(store_root: &Path, id: &str) -> Result<Option<WorkflowNextActionProposal>, String> {
    load_proposal_index(store_root, "by_workflow_run", id)
}
pub fn proposal_by_run_revision(store_root: &Path, id: &str) -> Result<Option<WorkflowNextActionProposal>, String> {
    load_proposal_index(store_root, "by_run_revision", id)
}
pub fn proposal_by_stage(store_root: &Path, id: &str) -> Result<Option<WorkflowNextActionProposal>, String> {
    load_proposal_index(store_root, "by_stage", id)
}
pub fn proposal_by_action_request(store_root: &Path, id: &str) -> Result<Option<WorkflowNextActionProposal>, String> {
    load_proposal_index(store_root, "by_action_request", id)
}

fn load_index<T: serde::de::DeserializeOwned>(root: &Path, index_name: &str, key: &str) -> Result<Option<T>, String> {
    let pointer = index_file(root, index_name, key);
    if !pointer.exists() { return Ok(None); }
    let id_str = std::fs::read_to_string(&pointer).map_err(|e| format!("{}", e))?;
    let path = root.join("records").join(format!("{}.json", id_str.trim()));
    if !path.exists() { return Ok(None); }
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&json).map(Some).map_err(|e| format!("Parse: {}", e))
}

fn load_proposal_index(store_root: &Path, index_name: &str, key: &str) -> Result<Option<WorkflowNextActionProposal>, String> {
    let pointer = index_file(&continuation_root(store_root).join("proposals"), index_name, key);
    if !pointer.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&pointer).map_err(|e| format!("{}", e))?;
    load_next_action_proposal(store_root, &WorkflowNextActionProposalId(id.trim().into())).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_continuation::*;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;
    use chrono::Utc;

    fn test_dir() -> std::path::PathBuf { tempfile::tempdir().unwrap().into_path() }

    fn test_readiness(status: WorkflowContinuationStatus, suffix: &str) -> WorkflowContinuationReadinessRecord {
        WorkflowContinuationReadinessRecord {
            readiness_id: WorkflowContinuationReadinessId(format!("wcr_{}", suffix)),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            latest_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            run_revision_hash: format!("h_{}", suffix),
            status: status.clone(),
            decision: match &status {
                WorkflowContinuationStatus::ProposalReady => WorkflowContinuationDecision::ProposalReady { summary: "ok".into() },
                WorkflowContinuationStatus::NoEligibleAction => WorkflowContinuationDecision::NoEligibleAction { summary: "none".into() },
                WorkflowContinuationStatus::Blocked => WorkflowContinuationDecision::Blocked { reason_code: "test".into(), summary: "blocked".into() },
                WorkflowContinuationStatus::Inconclusive => WorkflowContinuationDecision::Inconclusive { reason_code: "test".into(), summary: "inconclusive".into() },
            },
            predicates: vec![], selected_candidate: None, created_at: Utc::now(),
        }
    }

    fn test_proposal(suffix: &str) -> WorkflowNextActionProposal {
        WorkflowNextActionProposal {
            proposal_id: WorkflowNextActionProposalId(format!("wnap_{}", suffix)),
            readiness_id: WorkflowContinuationReadinessId(format!("wcr_{}", suffix)),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            source_run_revision_hash: format!("h_{}", suffix),
            candidate: WorkflowNextActionCandidate {
                stage_id: format!("s_{}", suffix), action_request_id: Some(format!("ar_{}", suffix)),
                candidate_kind: WorkflowNextActionKind::RoutePreparedAction,
                stage_title: "Test".into(), reason: "test".into(), dependency_evidence: vec![],
            },
            predicates: vec![], evidence_links: vec![],
            creates_route: false, routes_action_now: false,
            executes_tool_now: false, mutates_workflow_state_now: false,
            proposal_hash: format!("ph_{}", suffix), created_at: Utc::now(),
        }
    }

    #[test] fn continuation_readiness_persists_and_loads_roundtrip() {
        let d = test_dir(); let r = test_readiness(WorkflowContinuationStatus::ProposalReady, "rt");
        save_continuation_readiness(&d, &r).unwrap();
        let l = load_continuation_readiness(&d, &r.readiness_id).unwrap();
        assert_eq!(r.readiness_id, l.readiness_id);
    }
    #[test] fn next_action_proposal_persists_and_loads_roundtrip() {
        let d = test_dir(); let p = test_proposal("rt");
        save_next_action_proposal(&d, &p).unwrap();
        let l = load_next_action_proposal(&d, &p.proposal_id).unwrap();
        assert_eq!(p.proposal_id, l.proposal_id);
    }
    #[test] fn latest_continuation_readiness_returns_expected() {
        let d = test_dir(); let r = test_readiness(WorkflowContinuationStatus::ProposalReady, "lt");
        save_continuation_readiness(&d, &r).unwrap();
        assert_eq!(r.readiness_id, latest_continuation_readiness(&d).unwrap().unwrap().readiness_id);
    }
    #[test] fn latest_next_action_proposal_returns_expected() {
        let d = test_dir(); let p = test_proposal("lt");
        save_next_action_proposal(&d, &p).unwrap();
        assert_eq!(p.proposal_id, latest_next_action_proposal(&d).unwrap().unwrap().proposal_id);
    }
    #[test] fn continuation_by_workflow_run_returns_expected() {
        let d = test_dir(); let r = test_readiness(WorkflowContinuationStatus::ProposalReady, "wr");
        save_continuation_readiness(&d, &r).unwrap();
        assert_eq!(r.readiness_id, readiness_by_workflow_run(&d, "wfx_t").unwrap().unwrap().readiness_id);
    }
    #[test] fn continuation_by_run_revision_returns_expected() {
        let d = test_dir(); let r = test_readiness(WorkflowContinuationStatus::ProposalReady, "rr");
        save_continuation_readiness(&d, &r).unwrap();
        assert_eq!(r.readiness_id, readiness_by_run_revision(&d, "wrr_t").unwrap().unwrap().readiness_id);
    }
    #[test] fn proposal_by_stage_returns_expected() {
        let d = test_dir(); let p = test_proposal("st");
        save_next_action_proposal(&d, &p).unwrap();
        assert_eq!(p.proposal_id, proposal_by_stage(&d, "s_st").unwrap().unwrap().proposal_id);
    }
    #[test] fn proposal_by_action_request_returns_expected() {
        let d = test_dir(); let p = test_proposal("ar");
        save_next_action_proposal(&d, &p).unwrap();
        assert_eq!(p.proposal_id, proposal_by_action_request(&d, "ar_ar").unwrap().unwrap().proposal_id);
    }
    #[test] fn proposal_ready_cannot_duplicate_for_same_revision_candidate() {
        let d = test_dir();
        let p1 = test_proposal("rd1");
        save_next_action_proposal(&d, &p1).unwrap();
        let mut p2 = test_proposal("rd2");
        p2.candidate.stage_id = p1.candidate.stage_id.clone();
        let path2 = save_next_action_proposal(&d, &p2).unwrap();
        assert!(path2.to_string_lossy().contains("wnap_rd1"));
    }
    #[test] fn blocked_continuation_can_retry_with_new_key() {
        let d = test_dir();
        let r1 = test_readiness(WorkflowContinuationStatus::Blocked, "bk1");
        save_continuation_readiness(&d, &r1).unwrap();
        let mut r2 = test_readiness(WorkflowContinuationStatus::ProposalReady, "bk2");
        r2.workflow_execution_id = r1.workflow_execution_id.clone();
        r2.latest_run_revision_id = r1.latest_run_revision_id.clone();
        let p2 = save_continuation_readiness(&d, &r2).unwrap();
        assert!(p2.exists());
    }
    #[test] fn inconclusive_continuation_can_retry_with_new_key() {
        let d = test_dir();
        let r1 = test_readiness(WorkflowContinuationStatus::Inconclusive, "ic1");
        save_continuation_readiness(&d, &r1).unwrap();
        let mut r2 = test_readiness(WorkflowContinuationStatus::ProposalReady, "ic2");
        r2.workflow_execution_id = r1.workflow_execution_id.clone();
        r2.latest_run_revision_id = r1.latest_run_revision_id.clone();
        let p2 = save_continuation_readiness(&d, &r2).unwrap();
        assert!(p2.exists());
    }
    #[test] fn no_eligible_action_cannot_duplicate_for_same_revision_with_different_key() {
        // Patch 3
        let d = test_dir();
        let r1 = test_readiness(WorkflowContinuationStatus::NoEligibleAction, "na1");
        save_continuation_readiness(&d, &r1).unwrap();
        let mut r2 = test_readiness(WorkflowContinuationStatus::NoEligibleAction, "na2");
        r2.workflow_execution_id = r1.workflow_execution_id.clone();
        r2.latest_run_revision_id = r1.latest_run_revision_id.clone();
        let p2 = save_continuation_readiness(&d, &r2).unwrap();
        assert!(p2.to_string_lossy().contains("na1"));
    }
    #[test] fn no_eligible_action_can_repeat_after_revision_changes() {
        // Patch 3
        let d = test_dir();
        let r1 = test_readiness(WorkflowContinuationStatus::NoEligibleAction, "na1");
        save_continuation_readiness(&d, &r1).unwrap();
        let mut r2 = test_readiness(WorkflowContinuationStatus::NoEligibleAction, "na3");
        r2.workflow_execution_id = r1.workflow_execution_id.clone();
        r2.latest_run_revision_id = WorkflowRunRevisionId("wrr_other".into());
        let p2 = save_continuation_readiness(&d, &r2).unwrap();
        assert!(p2.exists());
        assert!(p2.to_string_lossy().contains("na3"));
    }
    #[test] fn writes_only_continuation_evidence() {
        let d = test_dir();
        save_continuation_readiness(&d, &test_readiness(WorkflowContinuationStatus::ProposalReady, "wo")).unwrap();
        save_next_action_proposal(&d, &test_proposal("wo")).unwrap();
        assert!(continuation_root(&d).exists());
        assert!(!d.join("workflow_runs").exists());
        assert!(!d.join("workflow_reconciliations").exists());
        assert!(!d.join("workflow_action_routes").exists());
    }
    #[test] fn does_not_write_route_outcome_or_reconciliation_records() {
        let d = test_dir();
        save_continuation_readiness(&d, &test_readiness(WorkflowContinuationStatus::ProposalReady, "nr")).unwrap();
        assert!(!d.join("workflow_action_routes").exists());
        assert!(!d.join("workflow_action_outcomes").exists());
        assert!(!d.join("workflow_reconciliations").exists());
    }
    #[test] fn does_not_write_workflow_run_or_revision_records() {
        let d = test_dir();
        save_continuation_readiness(&d, &test_readiness(WorkflowContinuationStatus::ProposalReady, "nw")).unwrap();
        assert!(!d.join("workflow_runs").exists());
        assert!(!d.join("workflow_run_revisions").exists());
    }
    #[test] fn does_not_write_approval_or_session_records() {
        let d = test_dir();
        save_continuation_readiness(&d, &test_readiness(WorkflowContinuationStatus::ProposalReady, "ns")).unwrap();
        assert!(!d.join("approvals").exists());
        assert!(!d.join("sessions").exists());
        assert!(!d.join("session_state").exists());
    }
}
