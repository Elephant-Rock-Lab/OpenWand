//! Proposal review and rejection feedback.
//!
//! This module reviews governed auto-commit proposals, producing approval,
//! rejection, or change-request records. It NEVER executes git operations,
//! creates execution grants, or mutates the workspace.
//!
//! Module boundary: Wave 11 owns proposal generation (eval_proposal.rs).
//! Wave 12 owns proposal review (this module).

use blake3::Hasher;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::eval_proposal::{AutoCommitProposal, AutoCommitProposalId, AutoCommitProposalStatus};

// ── Review ID (content-addressed) ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct AutoCommitProposalReviewId(pub String);

pub fn review_id_for(
    proposal_id: &str,
    decision: &AutoCommitProposalReviewDecision,
    rationale: &str,
) -> AutoCommitProposalReviewId {
    let mut hasher = Hasher::new();
    hasher.update(proposal_id.as_bytes());
    hasher.update(format!("{:?}", decision).as_bytes());
    hasher.update(rationale.as_bytes());
    let hash = hasher.finalize();
    AutoCommitProposalReviewId(format!("arv_{}", hash.to_hex()))
}

// ── Review decision ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AutoCommitProposalReviewDecision {
    Approved,
    Rejected,
    ChangesRequested,
    Superseded,
}

// ── Reviewer ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AutoCommitProposalReviewer {
    User,
    ReviewerSession { session_id: String },
    SystemCheck { name: String },
}

// ── Checklist ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalReviewChecklistItem {
    pub category: String,
    pub description: String,
    pub checked: bool,
}

// ── Feedback types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposalFeedbackCategory {
    Tests,
    Regression,
    Evidence,
    Scope,
    CommitMessage,
    FileSelection,
    Governance,
    Security,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProposalFeedbackSeverity {
    Advisory,
    Blocking,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiredProposalChange {
    pub category: ProposalFeedbackCategory,
    pub description: String,
    pub evidence_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalRejectionFeedback {
    pub feedback_id: String,
    pub proposal_id: AutoCommitProposalId,
    pub review_id: AutoCommitProposalReviewId,
    pub workspace_hash: String,
    pub summary: String,
    pub required_changes: Vec<RequiredProposalChange>,
    pub blocked_dimensions: Vec<String>,
    pub suggested_next_eval_focus: Vec<String>,
    pub severity: ProposalFeedbackSeverity,
}

// ── Full review record ─────────────────────────────────────────────────────

/// A review record for a governed auto-commit proposal.
/// This is an evidence artifact, NOT an execution grant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoCommitProposalReview {
    pub review_id: AutoCommitProposalReviewId,
    pub proposal_id: AutoCommitProposalId,
    pub readiness_id: String,
    pub workspace_hash: String,
    pub proposal_hash: String,

    pub decision: AutoCommitProposalReviewDecision,
    pub reviewer: AutoCommitProposalReviewer,

    pub rationale: String,
    pub checklist: Vec<ProposalReviewChecklistItem>,
    pub feedback: Option<ProposalRejectionFeedback>,

    /// Hard invariant: always false in Wave 12.
    pub execution_allowed_now: bool,
    /// Hard invariant: always false in Wave 12.
    pub creates_execution_grant: bool,

    pub reviewed_at: DateTime<Utc>,
}

// ── Review builder ─────────────────────────────────────────────────────────

/// Build a proposal review.
///
/// Validation rules:
/// - Approved requires proposal.status == Eligible
/// - Approved requires proposal has no blockers
/// - Rejected requires non-empty rationale AND structured feedback (Correction #2)
/// - ChangesRequested requires non-empty rationale AND structured feedback
/// - Blocked/Superseded proposals cannot be approved
///
/// Correction #1: Review copies hashes from proposal. Future execution
/// revalidates both against current state (Wave 13).
pub fn build_proposal_review(
    proposal: &AutoCommitProposal,
    decision: AutoCommitProposalReviewDecision,
    reviewer: AutoCommitProposalReviewer,
    rationale: String,
    checklist: Vec<ProposalReviewChecklistItem>,
    feedback: Option<ProposalRejectionFeedback>,
) -> Result<AutoCommitProposalReview, String> {
    // Validate rationale
    if rationale.trim().is_empty() {
        return Err("Rationale is required for all review decisions".to_string());
    }

    // Decision-specific validation
    match &decision {
        AutoCommitProposalReviewDecision::Approved => {
            if proposal.status != AutoCommitProposalStatus::Eligible {
                return Err(format!(
                    "Cannot approve proposal with status {:?}. Only Eligible proposals can be approved.",
                    proposal.status
                ));
            }
            if !proposal.blockers.is_empty() {
                return Err("Cannot approve proposal with active blockers".to_string());
            }
        }
        AutoCommitProposalReviewDecision::Rejected => {
            // Correction #2: Rejected requires feedback too
            if feedback.is_none() {
                return Err("Rejected reviews require structured feedback".to_string());
            }
        }
        AutoCommitProposalReviewDecision::ChangesRequested => {
            if feedback.is_none() {
                return Err("ChangesRequested reviews require structured feedback".to_string());
            }
        }
        AutoCommitProposalReviewDecision::Superseded => {
            // Superseded is informational, no special validation
        }
    }

    let review_id = review_id_for(
        &proposal.proposal_id.0,
        &decision,
        &rationale,
    );

    // Compute proposal hash from serialized proposal
    let proposal_json = serde_json::to_string(proposal)
        .map_err(|e| format!("Failed to serialize proposal: {}", e))?;
    let proposal_hash = format!("{}", blake3::hash(proposal_json.as_bytes()).to_hex());

    // Readiness ID derived from proposal's readiness reference
    let readiness_id = proposal.readiness_report_path
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    Ok(AutoCommitProposalReview {
        review_id,
        proposal_id: proposal.proposal_id.clone(),
        readiness_id,
        workspace_hash: proposal.workspace_snapshot_id.clone(), // Correction #1: copied, not validated
        proposal_hash, // Correction #1: computed from proposal, revalidated in Wave 13

        decision,
        reviewer,
        rationale,
        checklist,
        feedback,

        execution_allowed_now: false,        // HARD INVARIANT
        creates_execution_grant: false,       // HARD INVARIANT

        reviewed_at: Utc::now(),
    })
}

// ── Feedback export ────────────────────────────────────────────────────────

/// Export rejection feedback for a review.
/// Returns None for approved reviews.
pub fn export_rejection_feedback(
    store_root: &std::path::Path,
    review: &AutoCommitProposalReview,
) -> Result<Option<std::path::PathBuf>, String> {
    match &review.feedback {
        None => Ok(None), // Approved reviews have no feedback
        Some(feedback) => {
            let feedback_dir = store_root.join("proposal_feedback");
            std::fs::create_dir_all(&feedback_dir)
                .map_err(|e| format!("Failed to create feedback dir: {}", e))?;

            let path = feedback_dir.join(format!("{}.json", feedback.feedback_id));
            let json = serde_json::to_string_pretty(feedback)
                .map_err(|e| format!("Failed to serialize feedback: {}", e))?;
            std::fs::write(&path, json)
                .map_err(|e| format!("Failed to write feedback: {}", e))?;

            Ok(Some(path))
        }
    }
}

// ── Persistence ────────────────────────────────────────────────────────────

use std::path::{Path, PathBuf};

/// Save a review record. Supersedes prior reviews for the same proposal.
pub fn save_proposal_review(
    store_root: &Path,
    review: &AutoCommitProposalReview,
) -> Result<PathBuf, String> {
    let reviews_dir = store_root.join("proposal_reviews");
    std::fs::create_dir_all(&reviews_dir)
        .map_err(|e| format!("Failed to create reviews dir: {}", e))?;

    let by_proposal_dir = reviews_dir.join("by_proposal");
    std::fs::create_dir_all(&by_proposal_dir)
        .map_err(|e| format!("Failed to create by_proposal dir: {}", e))?;

    // Mark prior reviews for same proposal as Superseded
    if let Ok(existing) = list_proposal_reviews(store_root) {
        for mut existing_review in existing {
            if existing_review.proposal_id == review.proposal_id
                && existing_review.review_id != review.review_id
                && existing_review.decision != AutoCommitProposalReviewDecision::Superseded
            {
                existing_review.decision = AutoCommitProposalReviewDecision::Superseded;
                let old_path = reviews_dir.join(format!("{}.json", existing_review.review_id.0));
                let json = serde_json::to_string_pretty(&existing_review)
                    .map_err(|e| format!("Failed to serialize superseded review: {}", e))?;
                std::fs::write(&old_path, json)
                    .map_err(|e| format!("Failed to write superseded review: {}", e))?;
            }
        }
    }

    // Save the new review
    let path = reviews_dir.join(format!("{}.json", review.review_id.0));
    let json = serde_json::to_string_pretty(review)
        .map_err(|e| format!("Failed to serialize review: {}", e))?;
    std::fs::write(&path, &json)
        .map_err(|e| format!("Failed to write review: {}", e))?;

    // Write latest pointer
    let latest_path = reviews_dir.join("latest.json");
    std::fs::write(&latest_path, &json)
        .map_err(|e| format!("Failed to write latest review pointer: {}", e))?;

    // Write by_proposal pointer
    let by_proposal_path = by_proposal_dir.join(format!("{}.json", review.proposal_id.0));
    std::fs::write(&by_proposal_path, &json)
        .map_err(|e| format!("Failed to write by_proposal pointer: {}", e))?;

    // Also export feedback if present
    export_rejection_feedback(store_root, review)?;

    Ok(path)
}

/// Load a review by ID. Read-only.
pub fn load_proposal_review(
    store_root: &Path,
    id: &AutoCommitProposalReviewId,
) -> Result<Option<AutoCommitProposalReview>, String> {
    let path = store_root.join("proposal_reviews").join(format!("{}.json", id.0));
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read review: {}", e))?;
    let review: AutoCommitProposalReview = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse review: {}", e))?;
    Ok(Some(review))
}

/// Load the latest review. Read-only.
pub fn load_latest_proposal_review(
    store_root: &Path,
) -> Result<Option<AutoCommitProposalReview>, String> {
    let latest_path = store_root.join("proposal_reviews").join("latest.json");
    if !latest_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&latest_path)
        .map_err(|e| format!("Failed to read latest review: {}", e))?;
    let review: AutoCommitProposalReview = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse latest review: {}", e))?;
    Ok(Some(review))
}

/// Load the latest review for a specific proposal. Read-only.
pub fn load_latest_review_for_proposal(
    store_root: &Path,
    proposal_id: &AutoCommitProposalId,
) -> Result<Option<AutoCommitProposalReview>, String> {
    let path = store_root
        .join("proposal_reviews")
        .join("by_proposal")
        .join(format!("{}.json", proposal_id.0));
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read proposal review: {}", e))?;
    let review: AutoCommitProposalReview = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse proposal review: {}", e))?;
    Ok(Some(review))
}

/// List all reviews, ordered by reviewed_at descending. Read-only.
pub fn list_proposal_reviews(
    store_root: &Path,
) -> Result<Vec<AutoCommitProposalReview>, String> {
    let reviews_dir = store_root.join("proposal_reviews");
    if !reviews_dir.exists() {
        return Ok(vec![]);
    }

    let mut reviews = Vec::new();
    let entries = std::fs::read_dir(&reviews_dir)
        .map_err(|e| format!("Failed to read reviews dir: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read dir entry: {}", e))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json")
            && path.file_stem().and_then(|s| s.to_str()) != Some("latest")
        {
            // Skip by_proposal subdirectory entries
            if path.parent().and_then(|p| p.file_name()) == Some(std::ffi::OsStr::new("proposal_reviews")) {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("Failed to read review {}: {}", path.display(), e))?;
                let review: AutoCommitProposalReview = serde_json::from_str(&content)
                    .map_err(|e| format!("Failed to parse review {}: {}", path.display(), e))?;
                reviews.push(review);
            }
        }
    }

    reviews.sort_by(|a, b| b.reviewed_at.cmp(&a.reviewed_at));
    Ok(reviews)
}
