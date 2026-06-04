//! Next-action review persistence.
//!
//! Reviews under workflow_next_action_reviews/. Does not mutate continuation,
//! workflow run/revision, route, outcome, reconciliation, session, approval,
//! trace, memory, tool, git, or provider records.

use std::path::Path;

use openwand_workflow::workflow_next_action_review::*;

fn reviews_root(store_root: &Path) -> std::path::PathBuf {
    store_root.join("workflow_next_action_reviews")
}
fn records_dir(store_root: &Path) -> std::path::PathBuf {
    reviews_root(store_root).join("records")
}

fn index_file(root: &Path, index_name: &str, key: &str) -> std::path::PathBuf {
    root.join(index_name).join(format!("{}.json", key))
}

pub fn save_next_action_review(
    store_root: &Path,
    review: &WorkflowNextActionReview,
) -> Result<std::path::PathBuf, String> {
    let dir = records_dir(store_root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Dir: {}", e))?;

    let path = dir.join(format!("{}.json", review.review_id.0));
    let json = serde_json::to_string_pretty(review).map_err(|e| format!("Serialize: {}", e))?;
    std::fs::write(&path, json).map_err(|e| format!("Write: {}", e))?;

    std::fs::write(dir.join("latest.json"), review.review_id.0.as_bytes())
        .map_err(|e| format!("Latest: {}", e))?;

    // by_proposal index
    let idx_file = index_file(&reviews_root(store_root), "by_proposal", &review.proposal_id.0);
    std::fs::create_dir_all(idx_file.parent().unwrap()).map_err(|e| format!("Index dir: {}", e))?;
    std::fs::write(&idx_file, review.review_id.0.as_bytes()).map_err(|e| format!("Index: {}", e))?;

    // feedback
    if let Some(ref fb) = review.feedback {
        let fb_dir = reviews_root(store_root).join("feedback");
        std::fs::create_dir_all(&fb_dir).map_err(|e| format!("Fb dir: {}", e))?;
        let fb_json = serde_json::to_string_pretty(fb).map_err(|e| format!("Fb serialize: {}", e))?;
        std::fs::write(fb_dir.join(format!("{}.json", review.review_id.0)), fb_json)
            .map_err(|e| format!("Fb write: {}", e))?;
    }

    Ok(path)
}

pub fn load_next_action_review(store_root: &Path, id: &WorkflowNextActionReviewId) -> Result<WorkflowNextActionReview, String> {
    let path = records_dir(store_root).join(format!("{}.json", id.0));
    let json = std::fs::read_to_string(&path).map_err(|e| format!("Read: {}", e))?;
    serde_json::from_str(&json).map_err(|e| format!("Parse: {}", e))
}

pub fn latest_next_action_review(store_root: &Path) -> Result<Option<WorkflowNextActionReview>, String> {
    let p = records_dir(store_root).join("latest.json");
    if !p.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&p).map_err(|e| format!("{}", e))?;
    load_next_action_review(store_root, &WorkflowNextActionReviewId(id.trim().into())).map(Some)
}

pub fn next_action_review_by_proposal(store_root: &Path, proposal_id: &str) -> Result<Option<WorkflowNextActionReview>, String> {
    let pointer = index_file(&reviews_root(store_root), "by_proposal", proposal_id);
    if !pointer.exists() { return Ok(None); }
    let id = std::fs::read_to_string(&pointer).map_err(|e| format!("{}", e))?;
    load_next_action_review(store_root, &WorkflowNextActionReviewId(id.trim().into())).map(Some)
}

pub fn load_next_action_feedback(store_root: &Path, review_id: &str) -> Result<Option<WorkflowNextActionFeedback>, String> {
    let path = reviews_root(store_root).join("feedback").join(format!("{}.json", review_id));
    if !path.exists() { return Ok(None); }
    let json = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
    serde_json::from_str(&json).map(Some).map_err(|e| format!("Parse: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_workflow::workflow_continuation::WorkflowNextActionProposalId;
    use openwand_workflow::workflow_reconciliation::WorkflowRunRevisionId;
    use chrono::Utc;

    fn test_dir() -> std::path::PathBuf { tempfile::tempdir().unwrap().into_path() }

    fn test_review(decision: WorkflowNextActionReviewDecision, suffix: &str) -> WorkflowNextActionReview {
        let feedback = match &decision {
            WorkflowNextActionReviewDecision::Rejected => Some(WorkflowNextActionFeedback {
                summary: "unsafe".into(), blocking_reasons: vec!["risk".into()],
                requested_changes: vec![], evidence_gaps: vec![],
            }),
            WorkflowNextActionReviewDecision::ChangesRequested => Some(WorkflowNextActionFeedback {
                summary: "needs work".into(), blocking_reasons: vec![],
                requested_changes: vec!["fix".into()], evidence_gaps: vec![],
            }),
            _ => None,
        };
        WorkflowNextActionReview {
            review_id: WorkflowNextActionReviewId(format!("wnar_{}", suffix)),
            proposal_id: WorkflowNextActionProposalId("wnap_t".into()),
            proposal_hash: "ph".into(),
            source_run_revision_id: WorkflowRunRevisionId("wrr_t".into()),
            source_run_revision_hash: "rh".into(),
            decision, reviewer: "alice".into(), rationale: "ok".into(), feedback,
            creates_route: false, routes_action_now: false,
            executes_tool_now: false, mutates_workflow_state_now: false,
            reviewed_at: Utc::now(),
        }
    }

    #[test] fn next_action_review_persists_and_loads_roundtrip() {
        let d = test_dir(); let r = test_review(WorkflowNextActionReviewDecision::Approved, "rt");
        save_next_action_review(&d, &r).unwrap();
        let l = load_next_action_review(&d, &r.review_id).unwrap();
        assert_eq!(r.review_id, l.review_id);
    }
    #[test] fn latest_next_action_review_returns_expected() {
        let d = test_dir(); let r = test_review(WorkflowNextActionReviewDecision::Approved, "lt");
        save_next_action_review(&d, &r).unwrap();
        assert_eq!(r.review_id, latest_next_action_review(&d).unwrap().unwrap().review_id);
    }
    #[test] fn next_action_review_by_proposal_returns_expected() {
        let d = test_dir(); let r = test_review(WorkflowNextActionReviewDecision::Approved, "bp");
        save_next_action_review(&d, &r).unwrap();
        assert_eq!(r.review_id, next_action_review_by_proposal(&d, "wnap_t").unwrap().unwrap().review_id);
    }
    #[test] fn next_action_feedback_persists_and_loads_roundtrip() {
        let d = test_dir(); let r = test_review(WorkflowNextActionReviewDecision::Rejected, "fb");
        save_next_action_review(&d, &r).unwrap();
        let fb = load_next_action_feedback(&d, "wnar_fb").unwrap().unwrap();
        assert_eq!(1, fb.blocking_reasons.len());
    }
}
