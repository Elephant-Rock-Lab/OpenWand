//! Push proposal and human approval gate.
//!
//! Turns a Ready remote-push-readiness record into an exact governed push
//! proposal, requires human approval/rejection/change-request, persists the
//! review decision, and exports structured feedback.
//!
//! Module boundary:
//!   Wave 15: eval_remote_push_readiness.rs    → remote push readiness only
//!   Wave 16: eval_remote_push_proposal.rs      → push proposal and human review (this module)
//!
//! This module does NOT push, fetch, pull, tag, create branches, release,
//! reset, revert, merge, rebase, contact remote hosts, execute arbitrary
//! shell, or import std::process::Command.

use blake3::Hasher;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::eval_post_commit_verify::PostCommitVerificationId;
use crate::eval_proposal::AutoCommitProposalId;
use crate::eval_proposal_execution::AutoCommitExecutionId;
use crate::eval_proposal_review::AutoCommitProposalReviewId;
use crate::eval_remote_push_readiness::{
    RemotePushReadinessId, RemotePushReadinessRecord, RemotePushReadinessStatus,
    RemotePushReadinessDecision,
};

// ── Proposal ID ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RemotePushProposalId(pub String);

pub fn push_proposal_id_for(readiness_id: &str, idempotency_key: &str) -> RemotePushProposalId {
    let mut hasher = Hasher::new();
    hasher.update(b"remote_push_proposal:");
    hasher.update(readiness_id.as_bytes());
    hasher.update(idempotency_key.as_bytes());
    let hash = hasher.finalize();
    RemotePushProposalId(format!("rpp_{}", hash.to_hex()))
}

// ── Review ID ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RemotePushProposalReviewId(pub String);

pub fn push_review_id_for(proposal_id: &str, reviewer: &str, idempotency_key: &str) -> RemotePushProposalReviewId {
    let mut hasher = Hasher::new();
    hasher.update(b"remote_push_review:");
    hasher.update(proposal_id.as_bytes());
    hasher.update(reviewer.as_bytes());
    hasher.update(idempotency_key.as_bytes());
    let hash = hasher.finalize();
    RemotePushProposalReviewId(format!("rprv_{}", hash.to_hex()))
}

// ── Request DTOs ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePushProposalRequest {
    pub readiness_id: RemotePushReadinessId,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePushProposalReviewRequest {
    pub proposal_id: RemotePushProposalId,
    pub decision: RemotePushProposalReviewDecision,
    pub reviewer: String,
    pub rationale: String,
    pub feedback: Option<RemotePushProposalFeedback>,
    pub idempotency_key: String,
}

// ── Ref update snapshot ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteRefUpdateSnapshot {
    pub remote_name: String,
    pub branch: String,
    pub ref_name: String,
    pub expected_old_commit: String,
    pub proposed_new_commit: String,
    pub fast_forward_only: bool,
    pub ahead_count: u32,
    pub behind_count: u32,
    pub diverged: bool,
}

// ── Proposal ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemotePushProposalStatus {
    Eligible,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePushProposal {
    pub proposal_id: RemotePushProposalId,
    pub readiness_id: RemotePushReadinessId,
    pub verification_id: PostCommitVerificationId,
    pub execution_id: AutoCommitExecutionId,
    pub proposal_source_commit_id: AutoCommitProposalId,
    pub review_source_id: AutoCommitProposalReviewId,
    pub commit_hash: String,
    pub target_remote: String,
    pub target_branch: String,
    pub remote_ref: String,
    pub expected_remote_tracking_commit: String,
    pub proposed_new_commit: String,
    pub ref_update: RemoteRefUpdateSnapshot,
    pub status: RemotePushProposalStatus,
    pub proposal_hash: String,
    pub readiness_hash: String,
    pub created_at: DateTime<Utc>,
}

// ── Review ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemotePushProposalReviewDecision {
    Approved,
    Rejected,
    ChangesRequested,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePushProposalFeedback {
    pub summary: String,
    pub blocking_reasons: Vec<String>,
    pub requested_changes: Vec<String>,
    pub evidence_gaps: Vec<String>,
    pub suggested_next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePushProposalReview {
    pub review_id: RemotePushProposalReviewId,
    pub proposal_id: RemotePushProposalId,
    pub readiness_id: RemotePushReadinessId,
    pub proposal_hash: String,
    pub readiness_hash: String,
    pub decision: RemotePushProposalReviewDecision,
    pub reviewer: String,
    pub rationale: String,
    pub feedback: Option<RemotePushProposalFeedback>,
    pub creates_execution_grant: bool,
    pub execution_allowed_now: bool,
    pub reviewed_at: DateTime<Utc>,
}

// ── Proposal builder ────────────────────────────────────────────────────────

/// Build a push proposal from a Ready readiness record.
/// Patch 3: readiness_hash is copied from persisted record, never recomputed.
pub fn build_push_proposal(
    request: &RemotePushProposalRequest,
    readiness: Option<&RemotePushReadinessRecord>,
    existing_proposals: &[RemotePushProposal],
) -> Result<RemotePushProposal, String> {
    // Idempotency check
    let proposal_id = push_proposal_id_for(&request.readiness_id.0, &request.idempotency_key);
    if let Some(existing) = existing_proposals.iter().find(|p| p.proposal_id == proposal_id) {
        return Ok(existing.clone());
    }

    // 1. Readiness record must exist
    let readiness = readiness.ok_or("Readiness record not found")?;

    // 2. Readiness status must be Ready
    if readiness.status != RemotePushReadinessStatus::Ready {
        return Err(format!("Readiness status is {:?}, not Ready", readiness.status));
    }

    // 3. Readiness decision must be Ready
    if readiness.decision != RemotePushReadinessDecision::Ready {
        return Err("Readiness decision is not Ready".to_string());
    }

    // 4. Copy fields from readiness (not recomputed)
    let target_remote = readiness.target_remote.clone();
    let target_branch = readiness.target_branch.clone();
    let commit_hash = readiness.commit_hash.clone();

    // Expected old remote-tracking commit from readiness snapshot
    let expected_old_commit = readiness.remote_tracking
        .as_ref()
        .and_then(|rt| rt.tracking_commit.clone())
        .unwrap_or_default();

    // Ref update
    let ref_name = format!("refs/remotes/{}/{}", target_remote, target_branch);
    let ahead_count = readiness.local_branch.as_ref().map(|bs| bs.ahead_count).unwrap_or(0);
    let behind_count = readiness.local_branch.as_ref().map(|bs| bs.behind_count).unwrap_or(0);
    let diverged = readiness.local_branch.as_ref().map(|bs| bs.diverged).unwrap_or(false);

    let ref_update = RemoteRefUpdateSnapshot {
        remote_name: target_remote.clone(),
        branch: target_branch.clone(),
        ref_name: ref_name.clone(),
        expected_old_commit: expected_old_commit.clone(),
        proposed_new_commit: commit_hash.clone(),
        fast_forward_only: true, // Always fast-forward-only
        ahead_count,
        behind_count,
        diverged,
    };

    // Compute proposal_hash from proposal fields
    let proposal_hash = {
        let mut hasher = Hasher::new();
        hasher.update(commit_hash.as_bytes());
        hasher.update(target_remote.as_bytes());
        hasher.update(target_branch.as_bytes());
        hasher.update(expected_old_commit.as_bytes());
        hasher.update(ref_name.as_bytes());
        format!("{}", hasher.finalize().to_hex())
    };

    // Patch 3: readiness_hash copied from persisted record
    let readiness_hash = {
        let json = serde_json::to_string(readiness).map_err(|e| format!("Serialize readiness: {}", e))?;
        format!("{}", blake3::hash(json.as_bytes()).to_hex())
    };

    Ok(RemotePushProposal {
        proposal_id,
        readiness_id: readiness.readiness_id.clone(),
        verification_id: readiness.verification_id.clone(),
        execution_id: readiness.execution_id.clone(),
        proposal_source_commit_id: readiness.proposal_id.clone(),
        review_source_id: readiness.review_id.clone(),
        commit_hash,
        target_remote,
        target_branch,
        remote_ref: ref_name,
        expected_remote_tracking_commit: expected_old_commit,
        proposed_new_commit: readiness.commit_hash.clone(),
        ref_update,
        status: RemotePushProposalStatus::Eligible,
        proposal_hash,
        readiness_hash,
        created_at: Utc::now(),
    })
}

// ── Review builder ──────────────────────────────────────────────────────────

pub fn build_push_proposal_review(
    proposal: &RemotePushProposal,
    request: &RemotePushProposalReviewRequest,
    existing_reviews: &[RemotePushProposalReview],
) -> Result<RemotePushProposalReview, String> {
    // Idempotency check
    let review_id = push_review_id_for(&proposal.proposal_id.0, &request.reviewer, &request.idempotency_key);
    if let Some(existing) = existing_reviews.iter().find(|r| r.review_id == review_id) {
        return Ok(existing.clone());
    }

    // Reviewer must be non-empty
    if request.reviewer.trim().is_empty() {
        return Err("Reviewer must be non-empty".to_string());
    }

    // Rationale must be non-empty
    if request.rationale.trim().is_empty() {
        return Err("Rationale must be non-empty".to_string());
    }

    // Validate decision-specific requirements
    match request.decision {
        RemotePushProposalReviewDecision::Approved => {
            // No feedback required
        }
        RemotePushProposalReviewDecision::Rejected => {
            let feedback = request.feedback.as_ref().ok_or("Rejection requires feedback")?;
            if feedback.blocking_reasons.is_empty() {
                return Err("Rejection feedback must include blocking_reasons".to_string());
            }
        }
        RemotePushProposalReviewDecision::ChangesRequested => {
            let feedback = request.feedback.as_ref().ok_or("Change request requires feedback")?;
            if feedback.requested_changes.is_empty() {
                return Err("Change request feedback must include requested_changes".to_string());
            }
        }
    }

    Ok(RemotePushProposalReview {
        review_id,
        proposal_id: proposal.proposal_id.clone(),
        readiness_id: proposal.readiness_id.clone(),
        proposal_hash: proposal.proposal_hash.clone(),
        readiness_hash: proposal.readiness_hash.clone(),
        decision: request.decision.clone(),
        reviewer: request.reviewer.clone(),
        rationale: request.rationale.clone(),
        feedback: request.feedback.clone(),
        creates_execution_grant: false,
        execution_allowed_now: false,
        reviewed_at: Utc::now(),
    })
}

// ── Persistence ─────────────────────────────────────────────────────────────

fn ensure_dirs(store_root: &Path) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let base = store_root.join("remote_push_proposals");
    let proposals_dir = base.join("proposals");
    let reviews_dir = base.join("reviews");
    let feedback_dir = base.join("feedback");
    for dir in [&base, &proposals_dir, &reviews_dir, &feedback_dir] {
        std::fs::create_dir_all(dir).map_err(|e| format!("mkdir: {}", e))?;
    }
    let by_readiness = proposals_dir.join("by_readiness");
    std::fs::create_dir_all(&by_readiness).map_err(|e| format!("mkdir: {}", e))?;
    let by_proposal = reviews_dir.join("by_proposal");
    std::fs::create_dir_all(&by_proposal).map_err(|e| format!("mkdir: {}", e))?;
    Ok((proposals_dir, reviews_dir, feedback_dir))
}

pub fn save_push_proposal(
    store_root: &Path,
    proposal: &RemotePushProposal,
) -> Result<PathBuf, String> {
    let (proposals_dir, _, _) = ensure_dirs(store_root)?;
    let json = serde_json::to_string_pretty(proposal).map_err(|e| format!("{}", e))?;
    let path = proposals_dir.join(format!("{}.json", proposal.proposal_id.0));
    std::fs::write(&path, &json).map_err(|e| format!("{}", e))?;
    std::fs::write(proposals_dir.join("latest.json"), &json).map_err(|e| format!("{}", e))?;
    std::fs::write(proposals_dir.join("by_readiness").join(format!("{}.json", proposal.readiness_id.0)), &json).map_err(|e| format!("{}", e))?;
    Ok(path)
}

pub fn load_push_proposal(
    store_root: &Path,
    id: &RemotePushProposalId,
) -> Result<Option<RemotePushProposal>, String> {
    let path = store_root.join("remote_push_proposals").join("proposals").join(format!("{}.json", id.0));
    if !path.exists() { return Ok(None); }
    let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
    Ok(Some(serde_json::from_str(&c).map_err(|e| format!("{}", e))?))
}

pub fn load_latest_push_proposal(store_root: &Path) -> Result<Option<RemotePushProposal>, String> {
    let path = store_root.join("remote_push_proposals").join("proposals").join("latest.json");
    if !path.exists() { return Ok(None); }
    let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
    Ok(Some(serde_json::from_str(&c).map_err(|e| format!("{}", e))?))
}

pub fn load_push_proposal_by_readiness(
    store_root: &Path,
    readiness_id: &RemotePushReadinessId,
) -> Result<Option<RemotePushProposal>, String> {
    let path = store_root.join("remote_push_proposals").join("proposals").join("by_readiness").join(format!("{}.json", readiness_id.0));
    if !path.exists() { return Ok(None); }
    let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
    Ok(Some(serde_json::from_str(&c).map_err(|e| format!("{}", e))?))
}

pub fn list_push_proposals(store_root: &Path) -> Result<Vec<RemotePushProposal>, String> {
    let dir = store_root.join("remote_push_proposals").join("proposals");
    if !dir.exists() { return Ok(vec![]); }
    let mut records = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("{}", e))? {
        let entry = entry.map_err(|e| format!("{}", e))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json")
            && path.file_stem().and_then(|s| s.to_str()) != Some("latest")
        {
            let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
            let r: RemotePushProposal = serde_json::from_str(&c).map_err(|e| format!("{}", e))?;
            records.push(r);
        }
    }
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(records)
}

pub fn save_push_proposal_review(
    store_root: &Path,
    review: &RemotePushProposalReview,
) -> Result<PathBuf, String> {
    let (_, reviews_dir, feedback_dir) = ensure_dirs(store_root)?;
    let json = serde_json::to_string_pretty(review).map_err(|e| format!("{}", e))?;
    let path = reviews_dir.join(format!("{}.json", review.review_id.0));
    std::fs::write(&path, &json).map_err(|e| format!("{}", e))?;
    std::fs::write(reviews_dir.join("latest.json"), &json).map_err(|e| format!("{}", e))?;
    std::fs::write(reviews_dir.join("by_proposal").join(format!("{}.json", review.proposal_id.0)), &json).map_err(|e| format!("{}", e))?;
    if let Some(ref feedback) = review.feedback {
        let fb_json = serde_json::to_string_pretty(feedback).map_err(|e| format!("{}", e))?;
        std::fs::write(feedback_dir.join(format!("{}.json", review.review_id.0)), &fb_json).map_err(|e| format!("{}", e))?;
    }
    Ok(path)
}

pub fn load_push_proposal_review(
    store_root: &Path,
    id: &RemotePushProposalReviewId,
) -> Result<Option<RemotePushProposalReview>, String> {
    let path = store_root.join("remote_push_proposals").join("reviews").join(format!("{}.json", id.0));
    if !path.exists() { return Ok(None); }
    let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
    Ok(Some(serde_json::from_str(&c).map_err(|e| format!("{}", e))?))
}

pub fn load_latest_push_review(store_root: &Path) -> Result<Option<RemotePushProposalReview>, String> {
    let path = store_root.join("remote_push_proposals").join("reviews").join("latest.json");
    if !path.exists() { return Ok(None); }
    let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
    Ok(Some(serde_json::from_str(&c).map_err(|e| format!("{}", e))?))
}

pub fn load_latest_push_review_for_proposal(
    store_root: &Path,
    proposal_id: &RemotePushProposalId,
) -> Result<Option<RemotePushProposalReview>, String> {
    let path = store_root.join("remote_push_proposals").join("reviews").join("by_proposal").join(format!("{}.json", proposal_id.0));
    if !path.exists() { return Ok(None); }
    let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
    Ok(Some(serde_json::from_str(&c).map_err(|e| format!("{}", e))?))
}

pub fn load_push_review_feedback(
    store_root: &Path,
    review_id: &RemotePushProposalReviewId,
) -> Result<Option<RemotePushProposalFeedback>, String> {
    let path = store_root.join("remote_push_proposals").join("feedback").join(format!("{}.json", review_id.0));
    if !path.exists() { return Ok(None); }
    let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
    Ok(Some(serde_json::from_str(&c).map_err(|e| format!("{}", e))?))
}

pub fn list_push_reviews(store_root: &Path) -> Result<Vec<RemotePushProposalReview>, String> {
    let dir = store_root.join("remote_push_proposals").join("reviews");
    if !dir.exists() { return Ok(vec![]); }
    let mut records = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("{}", e))? {
        let entry = entry.map_err(|e| format!("{}", e))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json")
            && path.file_stem().and_then(|s| s.to_str()) != Some("latest")
        {
            let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
            let r: RemotePushProposalReview = serde_json::from_str(&c).map_err(|e| format!("{}", e))?;
            records.push(r);
        }
    }
    records.sort_by(|a, b| b.reviewed_at.cmp(&a.reviewed_at));
    Ok(records)
}
