//! Command review persistence.
//!
//! Writes only command-review evidence. Does not mutate command-composer,
//! loop-controller, workflow runs, revisions, routes, outcomes, reconciliations,
//! continuation, reviews, readiness, routing, approvals, sessions, trace,
//! memory, tools, git/provider, or governance records.

use std::path::Path;

use openwand_workflow::workflow_command_review::*;

fn review_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_command_reviews")
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

pub fn save_command_review(
    store_root: &Path,
    record: &WorkflowCommandReview,
) -> Result<std::path::PathBuf, String> {
    let dir = records_dir(store_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Dir: {}", e))?;

    // Idempotency: same idempotency key returns existing
    if let Ok(existing) = list_command_reviews(store_root) {
        for er in &existing {
            if er.command_composer_id == record.command_composer_id
                && er.review_id == record.review_id {
                return Ok(dir.join(format!("{}.json", er.review_id.0)));
            }
            // Acknowledged review cannot duplicate for same composer
            if er.command_composer_id == record.command_composer_id
                && matches!(er.decision, WorkflowCommandReviewDecision::Acknowledged)
                && matches!(record.decision, WorkflowCommandReviewDecision::Acknowledged)
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
        let fb_dir = feedback_dir(store_root);
        std::fs::create_dir_all(&fb_dir).map_err(|e| format!("Feedback dir: {}", e))?;
        let fb_json = serde_json::to_string_pretty(feedback).map_err(|e| format!("Feedback: {}", e))?;
        std::fs::write(fb_dir.join(format!("{}.json", record.review_id.0)), fb_json)
            .map_err(|e| format!("Feedback write: {}", e))?;
    }

    let root = review_root(store_root);
    for (idx_name, key) in [
        ("by_workflow_run", record.workflow_execution_id.0.as_str()),
        ("by_command_composer", record.command_composer_id.0.as_str()),
        ("by_loop_controller", record.loop_controller_id.0.as_str()),
    ] {
        let idx_file = index_file(&root, idx_name, key);
        std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Index dir: {}", e))?;
        std::fs::write(&idx_file, record.review_id.0.as_bytes()).map_err(|e| format!("Index: {}", e))?;
    }

    Ok(path)
}

pub fn load_command_review(store_root: &Path, id: &WorkflowCommandReviewId) -> Result<WorkflowCommandReview, String> {
    let path = records_dir(store_root).join(format!("{}.json", id.0));
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Parse: {}", e))
}

pub fn load_feedback(store_root: &Path, id: &WorkflowCommandReviewId) -> Result<Option<WorkflowCommandReviewFeedback>, String> {
    let path = feedback_dir(store_root).join(format!("{}.json", id.0));
    if !path.exists() { return Ok(None); }
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    Ok(Some(serde_json::from_str(&json).map_err(|e| format!("Parse: {}", e))?))
}

pub fn list_command_reviews(store_root: &Path) -> Result<Vec<WorkflowCommandReview>, String> {
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
                if let Ok(record) = serde_json::from_str::<WorkflowCommandReview>(&json) {
                    records.push(record);
                }
            }
        }
    }
    records.sort_by(|a, b| b.reviewed_at.cmp(&a.reviewed_at));
    Ok(records)
}

pub fn latest_command_review(store_root: &Path) -> Result<Option<WorkflowCommandReview>, String> {
    let p = records_dir(store_root).join("latest.json");
    if !p.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&p).map_err(|e| format!("{}", e))?;
    load_command_review(store_root, &WorkflowCommandReviewId(id.trim().into())).map(Some)
}

pub fn review_by_workflow_run(store_root: &Path, id: &str) -> Result<Option<WorkflowCommandReview>, String> {
    load_index(store_root, "by_workflow_run", id)
}
pub fn review_by_command_composer(store_root: &Path, id: &str) -> Result<Option<WorkflowCommandReview>, String> {
    load_index(store_root, "by_command_composer", id)
}
pub fn review_by_loop_controller(store_root: &Path, id: &str) -> Result<Option<WorkflowCommandReview>, String> {
    load_index(store_root, "by_loop_controller", id)
}

fn load_index(store_root: &Path, index_name: &str, key: &str) -> Result<Option<WorkflowCommandReview>, String> {
    let pointer = index_file(&review_root(store_root), index_name, key);
    if !pointer.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&pointer).map_err(|e| format!("{}", e))?;
    load_command_review(store_root, &WorkflowCommandReviewId(id.trim().into())).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_command_review::*;
    use openwand_workflow::workflow_command_composer::WorkflowCommandComposerId;
    use openwand_workflow::workflow_loop_controller::WorkflowLoopControllerId;
    use openwand_workflow::workflow_run::WorkflowExecutionId;
    use chrono::Utc;

    fn test_dir() -> std::path::PathBuf { tempfile::tempdir().unwrap().into_path() }

    fn test_review(decision: WorkflowCommandReviewDecision, suffix: &str) -> WorkflowCommandReview {
        WorkflowCommandReview {
            review_id: WorkflowCommandReviewId(format!("wcrv_{}", suffix)),
            workflow_execution_id: WorkflowExecutionId("wfx_t".into()),
            command_composer_id: WorkflowCommandComposerId("wcc_t".into()),
            loop_controller_id: WorkflowLoopControllerId("wlc_t".into()),
            command_composer_hash: "ch".into(),
            command_descriptor_hash: "dh".into(),
            loop_controller_hash: "lh".into(),
            decision: decision.clone(),
            reviewer: "tester".into(),
            rationale: "testing".into(),
            feedback: if matches!(decision, WorkflowCommandReviewDecision::Rejected) {
                Some(WorkflowCommandReviewFeedback {
                    summary: "blocked".into(),
                    blocking_reasons: vec!["reason1".into()],
                    requested_changes: vec![], evidence_gaps: vec![],
                })
            } else if matches!(decision, WorkflowCommandReviewDecision::ChangesRequested) {
                Some(WorkflowCommandReviewFeedback {
                    summary: "changes".into(),
                    blocking_reasons: vec![],
                    requested_changes: vec!["change1".into()], evidence_gaps: vec![],
                })
            } else { None },
            acknowledgment_snapshot: WorkflowCommandAcknowledgmentSnapshot {
                descriptor_display_command: "test".into(),
                descriptor_copyable_text_hash: "cth".into(),
                descriptor_display_only: true, descriptor_executable: false,
                descriptor_missing_inputs: vec![], loop_detected_state: "idle".into(),
                loop_recommended_operation: "none".into(),
                acknowledges_review_only: true, command_performed_now: false,
            },
            executes_command: false, invokes_shell: false, invokes_git: false,
            routes_action: false, resolves_approval: false, reconciles_outcome: false,
            mutates_workflow_state: false, schedules_work: false, starts_worker: false,
            queues_operation: false, creates_execution_grant: false,
            execution_allowed_now: false, reviewed_at: Utc::now(),
        }
    }

    #[test] fn command_review_persists_and_loads_roundtrip() {
        let d = test_dir(); let r = test_review(WorkflowCommandReviewDecision::Acknowledged, "rt");
        save_command_review(&d, &r).unwrap();
        let l = load_command_review(&d, &r.review_id).unwrap();
        assert_eq!(r.review_id, l.review_id);
    }
    #[test] fn latest_command_review_returns_expected() {
        let d = test_dir(); let r = test_review(WorkflowCommandReviewDecision::Acknowledged, "lt");
        save_command_review(&d, &r).unwrap();
        assert_eq!(r.review_id, latest_command_review(&d).unwrap().unwrap().review_id);
    }
    #[test] fn command_review_by_workflow_run_returns_expected() {
        let d = test_dir(); let r = test_review(WorkflowCommandReviewDecision::Acknowledged, "wr");
        save_command_review(&d, &r).unwrap();
        assert_eq!(r.review_id, review_by_workflow_run(&d, "wfx_t").unwrap().unwrap().review_id);
    }
    #[test] fn command_review_by_command_composer_returns_expected() {
        let d = test_dir(); let r = test_review(WorkflowCommandReviewDecision::Acknowledged, "cc");
        save_command_review(&d, &r).unwrap();
        assert_eq!(r.review_id, review_by_command_composer(&d, "wcc_t").unwrap().unwrap().review_id);
    }
    #[test] fn command_review_by_loop_controller_returns_expected() {
        let d = test_dir(); let r = test_review(WorkflowCommandReviewDecision::Acknowledged, "lc");
        save_command_review(&d, &r).unwrap();
        assert_eq!(r.review_id, review_by_loop_controller(&d, "wlc_t").unwrap().unwrap().review_id);
    }
    #[test] fn command_review_feedback_persists_and_loads_roundtrip() {
        let d = test_dir();
        let r = test_review(WorkflowCommandReviewDecision::Rejected, "fb");
        save_command_review(&d, &r).unwrap();
        let fb = load_feedback(&d, &r.review_id).unwrap().unwrap();
        assert_eq!("blocked", fb.summary);
        assert!(!fb.blocking_reasons.is_empty());
    }
    #[test] fn same_idempotency_key_returns_existing_review() {
        let d = test_dir(); let r = test_review(WorkflowCommandReviewDecision::Acknowledged, "id");
        let p1 = save_command_review(&d, &r).unwrap();
        let p2 = save_command_review(&d, &r).unwrap();
        assert_eq!(p1, p2);
    }
    #[test] fn acknowledged_review_cannot_duplicate_with_different_key() {
        let d = test_dir();
        let r1 = test_review(WorkflowCommandReviewDecision::Acknowledged, "dp1");
        save_command_review(&d, &r1).unwrap();
        let r2 = test_review(WorkflowCommandReviewDecision::Acknowledged, "dp2");
        let p2 = save_command_review(&d, &r2).unwrap();
        assert!(p2.to_string_lossy().contains("dp1"), "Should return existing");
    }
    #[test] fn rejected_review_preserves_history_with_new_key() {
        let d = test_dir();
        let r1 = test_review(WorkflowCommandReviewDecision::Rejected, "rj1");
        save_command_review(&d, &r1).unwrap();
        let r2 = test_review(WorkflowCommandReviewDecision::Rejected, "rj2");
        let p2 = save_command_review(&d, &r2).unwrap();
        assert!(p2.exists(), "New rejected review creates new record");
    }
    #[test] fn changes_requested_review_preserves_history_with_new_key() {
        let d = test_dir();
        let r1 = test_review(WorkflowCommandReviewDecision::ChangesRequested, "cr1");
        save_command_review(&d, &r1).unwrap();
        let r2 = test_review(WorkflowCommandReviewDecision::ChangesRequested, "cr2");
        let p2 = save_command_review(&d, &r2).unwrap();
        assert!(p2.exists(), "New changes-requested review creates new record");
    }
    #[test] fn command_review_writes_only_review_evidence() {
        let d = test_dir();
        save_command_review(&d, &test_review(WorkflowCommandReviewDecision::Acknowledged, "wo")).unwrap();
        assert!(review_root(&d).exists());
        assert!(!d.join("workflow_runs").exists());
        assert!(!d.join("workflow_loop_controller").exists());
    }
    #[test] fn command_review_does_not_write_command_composer_records() {
        let d = test_dir();
        save_command_review(&d, &test_review(WorkflowCommandReviewDecision::Acknowledged, "nc")).unwrap();
        assert!(!d.join("workflow_command_composer").exists());
    }
    #[test] fn command_review_does_not_write_loop_controller_records() {
        let d = test_dir();
        save_command_review(&d, &test_review(WorkflowCommandReviewDecision::Acknowledged, "nl")).unwrap();
        assert!(!d.join("workflow_loop_controller").exists());
    }
    #[test] fn command_review_does_not_write_route_outcome_reconciliation_records() {
        let d = test_dir();
        save_command_review(&d, &test_review(WorkflowCommandReviewDecision::Acknowledged, "nr")).unwrap();
        assert!(!d.join("workflow_action_routes").exists());
        assert!(!d.join("workflow_action_outcomes").exists());
        assert!(!d.join("workflow_reconciliations").exists());
    }
    #[test] fn command_review_does_not_write_approval_or_session_records() {
        let d = test_dir();
        save_command_review(&d, &test_review(WorkflowCommandReviewDecision::Acknowledged, "ns")).unwrap();
        assert!(!d.join("approvals").exists());
        assert!(!d.join("sessions").exists());
    }
    #[test] fn command_review_does_not_write_governance_or_provider_records() {
        let d = test_dir();
        save_command_review(&d, &test_review(WorkflowCommandReviewDecision::Acknowledged, "ng")).unwrap();
        assert!(!d.join("governance").exists());
        assert!(!d.join("providers").exists());
    }
}
