//! Manual result review persistence.
//!
//! Writes only manual-result-review evidence. Does not mutate manual result records,
//! command reviews, command-composer records, loop-controller records, workflow runs,
//! revisions, routes, outcomes, reconciliations, continuation, reviews,
//! readiness, routing, approvals, sessions, trace, memory, tools,
//! git/provider, or governance records.

use std::path::Path;

use openwand_workflow::workflow_manual_result_review::*;

fn review_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_manual_result_reviews")
}
fn records_dir(store_root: &Path) -> std::path::PathBuf {
    review_root(store_root).join("records")
}
fn feedback_dir(store_root: &Path) -> std::path::PathBuf {
    review_root(store_root).join("feedback")
}
fn index_file(root: &Path, index_name: &str, key: &str) -> std::path::PathBuf {
    root.join(index_name).join(format!("{}.json", key))
}

pub fn save_manual_result_review(
    store_root: &Path,
    record: &WorkflowManualResultReview,
) -> Result<std::path::PathBuf, String> {
    let dir = records_dir(store_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Dir: {}", e))?;

    // Patch 1: idempotency in app layer, not workflow validation
    if let Ok(existing) = list_manual_result_reviews(store_root) {
        for er in &existing {
            // Same idempotency key returns existing review for all decisions
            if er.review_id == record.review_id {
                return Ok(dir.join(format!("{}.json", er.review_id.0)));
            }
            // Accepted cannot duplicate for the same manual result with different key
            if er.manual_result_id == record.manual_result_id
                && matches!(er.decision, WorkflowManualResultReviewDecision::Accepted)
                && matches!(record.decision, WorkflowManualResultReviewDecision::Accepted)
                && er.review_id != record.review_id {
                return Ok(dir.join(format!("{}.json", er.review_id.0)));
            }
        }
    }

    let path = dir.join(format!("{}.json", record.review_id.0));
    let json = serde_json::to_string_pretty(record).map_err(|e| format!("Serialize: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Write: {}", e))?;

    std::fs::write(dir.join("latest.json"), record.review_id.0.as_bytes())
        .map_err(|e| format!("Latest: {}", e))?;

    // Save feedback separately if present
    if let Some(ref feedback) = record.feedback {
        let f_dir = feedback_dir(store_root);
        std::fs::create_dir_all(&f_dir).map_err(|e| format!("Feedback dir: {}", e))?;
        let f_json = serde_json::to_string_pretty(feedback)
            .map_err(|e| format!("Feedback: {}", e))?;
        std::fs::write(f_dir.join(format!("{}.json", record.review_id.0)), f_json)
            .map_err(|e| format!("Feedback write: {}", e))?;
    }

    // Patch 5: indexes for all source-chain IDs
    let root = review_root(store_root);
    for (idx_name, key) in [
        ("by_manual_result", record.manual_result_id.0.as_str()),
        ("by_workflow_run", record.workflow_execution_id.0.as_str()),
        ("by_command_review", record.command_review_id.0.as_str()),
        ("by_command_composer", record.command_composer_id.0.as_str()),
        ("by_loop_controller", record.loop_controller_id.0.as_str()),
    ] {
        let idx_file = index_file(&root, idx_name, key);
        std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Index dir: {}", e))?;
        std::fs::write(&idx_file, record.review_id.0.as_bytes()).map_err(|e| format!("Index: {}", e))?;
    }

    Ok(path)
}

pub fn load_manual_result_review(store_root: &Path, id: &WorkflowManualResultReviewId) -> Result<WorkflowManualResultReview, String> {
    let path = records_dir(store_root).join(format!("{}.json", id.0));
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Parse: {}", e))
}

pub fn load_feedback(store_root: &Path, id: &WorkflowManualResultReviewId) -> Result<Option<WorkflowManualResultReviewFeedback>, String> {
    let path = feedback_dir(store_root).join(format!("{}.json", id.0));
    if !path.exists() { return Ok(None); }
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&json).map(Some).map_err(|e| format!("Parse: {}", e))
}

pub fn list_manual_result_reviews(store_root: &Path) -> Result<Vec<WorkflowManualResultReview>, String> {
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
                if let Ok(record) = serde_json::from_str::<WorkflowManualResultReview>(&json) {
                    records.push(record);
                }
            }
        }
    }
    records.sort_by(|a, b| b.reviewed_at.cmp(&a.reviewed_at));
    Ok(records)
}

pub fn latest_manual_result_review(store_root: &Path) -> Result<Option<WorkflowManualResultReview>, String> {
    let p = records_dir(store_root).join("latest.json");
    if !p.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&p).map_err(|e| format!("{}", e))?;
    load_manual_result_review(store_root, &WorkflowManualResultReviewId(id.trim().into())).map(Some)
}

pub fn review_by_manual_result(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReview>, String> {
    load_index(store_root, "by_manual_result", id)
}
pub fn review_by_workflow_run(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReview>, String> {
    load_index(store_root, "by_workflow_run", id)
}
pub fn review_by_command_review(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReview>, String> {
    load_index(store_root, "by_command_review", id)
}
pub fn review_by_command_composer(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReview>, String> {
    load_index(store_root, "by_command_composer", id)
}
pub fn review_by_loop_controller(store_root: &Path, id: &str) -> Result<Option<WorkflowManualResultReview>, String> {
    load_index(store_root, "by_loop_controller", id)
}

fn load_index(store_root: &Path, index_name: &str, key: &str) -> Result<Option<WorkflowManualResultReview>, String> {
    let pointer = index_file(&review_root(store_root), index_name, key);
    if !pointer.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&pointer).map_err(|e| format!("{}", e))?;
    load_manual_result_review(store_root, &WorkflowManualResultReviewId(id.trim().into())).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_manual_result_review::*;
    use openwand_workflow::workflow_manual_result::WorkflowManualResultId;
    use openwand_workflow::workflow_command_review::WorkflowCommandReviewId;
    use openwand_workflow::workflow_command_composer::WorkflowCommandComposerId;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use chrono::Utc;

    fn test_dir() -> std::path::PathBuf { tempfile::tempdir().unwrap().into_path() }

    fn test_review(decision: WorkflowManualResultReviewDecision, suffix: &str) -> WorkflowManualResultReview {
        WorkflowManualResultReview {
            review_id: WorkflowManualResultReviewId(format!("wmrr_{}", suffix)),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            manual_result_id: WorkflowManualResultId("wmr_t".into()),
            command_review_id: WorkflowCommandReviewId("wcrv_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            manual_result_hash: "mrh".into(),
            command_review_hash: "crh".into(),
            command_composer_hash: "cch".into(),
            command_descriptor_hash: "cdh".into(),
            loop_controller_hash: "lch".into(),
            decision: decision.clone(),
            reviewer: "reviewer".into(),
            rationale: "ok".into(),
            feedback: if matches!(decision, WorkflowManualResultReviewDecision::Rejected) {
                Some(WorkflowManualResultReviewFeedback {
                    summary: "unsafe".into(),
                    blocking_reasons: vec!["risk".into()],
                    requested_changes: vec![],
                    evidence_gaps: vec![],
                })
            } else { None },
            acceptance_snapshot: WorkflowManualResultReviewAcceptanceSnapshot {
                accepts_reported_evidence: matches!(&decision, WorkflowManualResultReviewDecision::Accepted),
                verifies_external_state: false,
                reconciles_workflow_state: false,
                result_verified_by_openwand: false,
            },
            verifies_external_state: false,
            reconciles_workflow_state: false,
            mutates_workflow_state: false,
            executes_command: false,
            invokes_shell: false,
            invokes_git: false,
            routes_action: false,
            resolves_approval: false,
            appends_trace: false,
            writes_memory: false,
            creates_execution_grant: false,
            execution_allowed_now: false,
            reviewed_at: Utc::now(),
        }
    }

    #[test] fn review_persists_and_loads_roundtrip() {
        let d = test_dir(); let r = test_review(WorkflowManualResultReviewDecision::Accepted, "rt");
        save_manual_result_review(&d, &r).unwrap();
        let l = load_manual_result_review(&d, &r.review_id).unwrap();
        assert_eq!(r.review_id, l.review_id);
    }
    #[test] fn latest_review_returns_expected() {
        let d = test_dir(); let r = test_review(WorkflowManualResultReviewDecision::Accepted, "lt");
        save_manual_result_review(&d, &r).unwrap();
        assert_eq!(r.review_id, latest_manual_result_review(&d).unwrap().unwrap().review_id);
    }
    #[test] fn review_by_manual_result_returns_expected() {
        let d = test_dir(); let r = test_review(WorkflowManualResultReviewDecision::Accepted, "mr");
        save_manual_result_review(&d, &r).unwrap();
        assert_eq!(r.review_id, review_by_manual_result(&d, "wmr_t").unwrap().unwrap().review_id);
    }
    #[test] fn review_by_workflow_run_returns_expected() {
        let d = test_dir(); let r = test_review(WorkflowManualResultReviewDecision::Accepted, "wr");
        save_manual_result_review(&d, &r).unwrap();
        assert_eq!(r.review_id, review_by_workflow_run(&d, "wfx_t").unwrap().unwrap().review_id);
    }
    #[test] fn review_by_command_review_returns_expected() {
        let d = test_dir(); let r = test_review(WorkflowManualResultReviewDecision::Accepted, "cr");
        save_manual_result_review(&d, &r).unwrap();
        assert_eq!(r.review_id, review_by_command_review(&d, "wcrv_t").unwrap().unwrap().review_id);
    }
    #[test] fn review_by_command_composer_returns_expected() {
        let d = test_dir(); let r = test_review(WorkflowManualResultReviewDecision::Accepted, "cc");
        save_manual_result_review(&d, &r).unwrap();
        assert_eq!(r.review_id, review_by_command_composer(&d, "wcc_t").unwrap().unwrap().review_id);
    }
    #[test] fn review_by_loop_controller_returns_expected() {
        let d = test_dir(); let r = test_review(WorkflowManualResultReviewDecision::Accepted, "lc");
        save_manual_result_review(&d, &r).unwrap();
        assert_eq!(r.review_id, review_by_loop_controller(&d, "wlc_t").unwrap().unwrap().review_id);
    }
    #[test] fn feedback_persists_and_loads_roundtrip() {
        let d = test_dir(); let r = test_review(WorkflowManualResultReviewDecision::Rejected, "fb");
        save_manual_result_review(&d, &r).unwrap();
        let fb = load_feedback(&d, &r.review_id).unwrap();
        assert!(fb.is_some());
        assert_eq!(1, fb.unwrap().blocking_reasons.len());
    }
    #[test] fn no_feedback_returns_none() {
        let d = test_dir(); let r = test_review(WorkflowManualResultReviewDecision::Accepted, "nf");
        save_manual_result_review(&d, &r).unwrap();
        assert!(load_feedback(&d, &r.review_id).unwrap().is_none());
    }
    // Patch 1: idempotency tests
    #[test] fn same_idempotency_key_returns_existing_manual_result_review() {
        let d = test_dir(); let r = test_review(WorkflowManualResultReviewDecision::Accepted, "id");
        let p1 = save_manual_result_review(&d, &r).unwrap();
        let p2 = save_manual_result_review(&d, &r).unwrap();
        assert_eq!(p1, p2);
    }
    #[test] fn accepted_review_cannot_duplicate_with_different_key() {
        let d = test_dir();
        let r1 = test_review(WorkflowManualResultReviewDecision::Accepted, "dp1");
        save_manual_result_review(&d, &r1).unwrap();
        let r2 = test_review(WorkflowManualResultReviewDecision::Accepted, "dp2");
        let p2 = save_manual_result_review(&d, &r2).unwrap();
        assert!(p2.to_string_lossy().contains("dp1"), "Should return existing");
    }
    #[test] fn rejected_review_preserves_history_with_new_key() {
        let d = test_dir();
        let r1 = test_review(WorkflowManualResultReviewDecision::Rejected, "rj1");
        save_manual_result_review(&d, &r1).unwrap();
        let r2 = test_review(WorkflowManualResultReviewDecision::Rejected, "rj2");
        let p2 = save_manual_result_review(&d, &r2).unwrap();
        assert!(p2.exists(), "New rejection creates new record");
    }
    #[test] fn changes_requested_review_preserves_history_with_new_key() {
        let d = test_dir();
        let mut r1 = test_review(WorkflowManualResultReviewDecision::ChangesRequested, "ch1");
        r1.feedback = Some(WorkflowManualResultReviewFeedback {
            summary: "fix".into(), blocking_reasons: vec![],
            requested_changes: vec!["add evidence".into()], evidence_gaps: vec![],
        });
        save_manual_result_review(&d, &r1).unwrap();
        let mut r2 = test_review(WorkflowManualResultReviewDecision::ChangesRequested, "ch2");
        r2.feedback = Some(WorkflowManualResultReviewFeedback {
            summary: "fix2".into(), blocking_reasons: vec![],
            requested_changes: vec!["more detail".into()], evidence_gaps: vec![],
        });
        let p2 = save_manual_result_review(&d, &r2).unwrap();
        assert!(p2.exists(), "New changes-requested creates new record");
    }
    #[test] fn review_writes_only_review_evidence() {
        let d = test_dir();
        save_manual_result_review(&d, &test_review(WorkflowManualResultReviewDecision::Accepted, "wo")).unwrap();
        assert!(review_root(&d).exists());
        assert!(!d.join("workflow_manual_results").exists());
        assert!(!d.join("workflow_runs").exists());
    }
    #[test] fn review_does_not_write_manual_result_records() {
        let d = test_dir();
        save_manual_result_review(&d, &test_review(WorkflowManualResultReviewDecision::Accepted, "nm")).unwrap();
        assert!(!d.join("workflow_manual_results").exists());
    }
    #[test] fn review_does_not_write_command_review_or_composer_records() {
        let d = test_dir();
        save_manual_result_review(&d, &test_review(WorkflowManualResultReviewDecision::Accepted, "nc")).unwrap();
        assert!(!d.join("workflow_command_reviews").exists());
        assert!(!d.join("workflow_command_composer").exists());
    }
    #[test] fn review_does_not_write_loop_controller_records() {
        let d = test_dir();
        save_manual_result_review(&d, &test_review(WorkflowManualResultReviewDecision::Accepted, "nl")).unwrap();
        assert!(!d.join("workflow_loop_controller").exists());
    }
    #[test] fn review_does_not_write_route_outcome_reconciliation_records() {
        let d = test_dir();
        save_manual_result_review(&d, &test_review(WorkflowManualResultReviewDecision::Accepted, "nr")).unwrap();
        assert!(!d.join("workflow_action_routes").exists());
        assert!(!d.join("workflow_action_outcomes").exists());
        assert!(!d.join("workflow_reconciliations").exists());
    }
    #[test] fn review_does_not_write_approval_or_session_records() {
        let d = test_dir();
        save_manual_result_review(&d, &test_review(WorkflowManualResultReviewDecision::Accepted, "ns")).unwrap();
        assert!(!d.join("approvals").exists());
        assert!(!d.join("sessions").exists());
    }
    #[test] fn review_does_not_write_governance_or_provider_records() {
        let d = test_dir();
        save_manual_result_review(&d, &test_review(WorkflowManualResultReviewDecision::Accepted, "ng")).unwrap();
        assert!(!d.join("governance").exists());
        assert!(!d.join("providers").exists());
    }
}
