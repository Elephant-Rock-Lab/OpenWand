//! UI governance action adapters.
//!
//! Routes review intent through existing governed review builders.
//! Never calls execution modules, constructs review/execution records directly,
//! or bypasses the governed persistence functions.

use serde::{Deserialize, Serialize};

use crate::eval_proposal::AutoCommitProposalId;
use crate::eval_proposal_review::{
    AutoCommitProposalReview, AutoCommitProposalReviewDecision, AutoCommitProposalReviewer,
    build_proposal_review, save_proposal_review,
};
use crate::eval_remote_push_proposal::{
    RemotePushProposalFeedback, RemotePushProposalId, RemotePushProposalReview,
    RemotePushProposalReviewDecision, RemotePushProposalReviewId,
    build_push_proposal_review, load_push_proposal, save_push_proposal_review,
};

// ── Action enum ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GovernanceUiAction {
    ApproveLocalProposal {
        proposal_id: String,
        reviewer: String,
        rationale: String,
    },
    RejectLocalProposal {
        proposal_id: String,
        reviewer: String,
        rationale: String,
        feedback: String,
    },
    RequestChangesLocalProposal {
        proposal_id: String,
        reviewer: String,
        rationale: String,
        feedback: String,
    },
    ApprovePushProposal {
        proposal_id: String,
        reviewer: String,
        rationale: String,
    },
    RejectPushProposal {
        proposal_id: String,
        reviewer: String,
        rationale: String,
        feedback: String,
    },
    RequestChangesPushProposal {
        proposal_id: String,
        reviewer: String,
        rationale: String,
        feedback: String,
    },
    Refresh,
}

// ── Action result (Patch 2) ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GovernanceActionResult {
    Refreshed,
    LocalReviewCreated {
        review_id: String,
        proposal_id: String,
        decision: String,
        creates_execution_grant: bool,
        execution_allowed_now: bool,
    },
    PushReviewCreated {
        review_id: String,
        proposal_id: String,
        decision: String,
        creates_execution_grant: bool,
        execution_allowed_now: bool,
    },
}

// ── Adapter ─────────────────────────────────────────────────────────────────

pub fn execute_governance_action(
    action: GovernanceUiAction,
    store_root: &std::path::Path,
) -> Result<GovernanceActionResult, String> {
    match action {
        GovernanceUiAction::Refresh => Ok(GovernanceActionResult::Refreshed),

        GovernanceUiAction::ApproveLocalProposal { proposal_id, reviewer, rationale } => {
            let review = build_local_review(&proposal_id, &reviewer, &rationale, None, store_root)?;
            let review_id = review.review_id.0.clone();
            save_proposal_review(store_root, &review)?;
            Ok(GovernanceActionResult::LocalReviewCreated {
                review_id,
                proposal_id,
                decision: "Approved".into(),
                creates_execution_grant: false,
                execution_allowed_now: false,
            })
        }

        GovernanceUiAction::RejectLocalProposal { proposal_id, reviewer, rationale, feedback } => {
            let review = build_local_review(&proposal_id, &reviewer, &rationale, Some(&feedback), store_root)?;
            let review_id = review.review_id.0.clone();
            save_proposal_review(store_root, &review)?;
            Ok(GovernanceActionResult::LocalReviewCreated {
                review_id,
                proposal_id,
                decision: "Rejected".into(),
                creates_execution_grant: false,
                execution_allowed_now: false,
            })
        }

        GovernanceUiAction::RequestChangesLocalProposal { proposal_id, reviewer, rationale, feedback } => {
            let review = build_local_review(&proposal_id, &reviewer, &rationale, Some(&feedback), store_root)?;
            let review_id = review.review_id.0.clone();
            save_proposal_review(store_root, &review)?;
            Ok(GovernanceActionResult::LocalReviewCreated {
                review_id,
                proposal_id,
                decision: "ChangesRequested".into(),
                creates_execution_grant: false,
                execution_allowed_now: false,
            })
        }

        GovernanceUiAction::ApprovePushProposal { proposal_id, reviewer, rationale } => {
            let review = build_push_review(&proposal_id, &reviewer, &rationale, None, store_root)?;
            let review_id = review.review_id.0.clone();
            save_push_proposal_review(store_root, &review)?;
            Ok(GovernanceActionResult::PushReviewCreated {
                review_id,
                proposal_id,
                decision: "Approved".into(),
                creates_execution_grant: false,
                execution_allowed_now: false,
            })
        }

        GovernanceUiAction::RejectPushProposal { proposal_id, reviewer, rationale, feedback } => {
            let review = build_push_review(&proposal_id, &reviewer, &rationale, Some(&feedback), store_root)?;
            let review_id = review.review_id.0.clone();
            save_push_proposal_review(store_root, &review)?;
            Ok(GovernanceActionResult::PushReviewCreated {
                review_id,
                proposal_id,
                decision: "Rejected".into(),
                creates_execution_grant: false,
                execution_allowed_now: false,
            })
        }

        GovernanceUiAction::RequestChangesPushProposal { proposal_id, reviewer, rationale, feedback } => {
            let review = build_push_review(&proposal_id, &reviewer, &rationale, Some(&feedback), store_root)?;
            let review_id = review.review_id.0.clone();
            save_push_proposal_review(store_root, &review)?;
            Ok(GovernanceActionResult::PushReviewCreated {
                review_id,
                proposal_id,
                decision: "ChangesRequested".into(),
                creates_execution_grant: false,
                execution_allowed_now: false,
            })
        }
    }
}

// ── Internal helpers ────────────────────────────────────────────────────────

fn build_local_review(
    proposal_id: &str,
    reviewer: &str,
    rationale: &str,
    feedback: Option<&str>,
    store_root: &std::path::Path,
) -> Result<AutoCommitProposalReview, String> {
    let pid = AutoCommitProposalId(proposal_id.to_string());
    let proposal = crate::eval_proposal::load_proposal(store_root, &pid)
        .map_err(|e| format!("{}", e))?
        .ok_or_else(|| format!("Proposal not found: {}", proposal_id))?;

    let decision = if feedback.is_some() && rationale.contains("reject") {
        AutoCommitProposalReviewDecision::Rejected
    } else if feedback.is_some() {
        AutoCommitProposalReviewDecision::ChangesRequested
    } else {
        AutoCommitProposalReviewDecision::Approved
    };

    let correction_reasons: Vec<String> = feedback.map(|f| vec![f.to_string()]).unwrap_or_default();
    let checklist_items: Vec<crate::eval_proposal_review::ProposalReviewChecklistItem> = correction_reasons.iter().map(|r| crate::eval_proposal_review::ProposalReviewChecklistItem { category: "general".into(), description: r.clone(), checked: false }).collect();

    build_proposal_review(
        &proposal,
        decision,
        AutoCommitProposalReviewer::User,
        rationale.to_string(),
        checklist_items,
        feedback.map(|f| {
            crate::eval_proposal_review::ProposalRejectionFeedback {
                feedback_id: format!("ui_fb_{}", chrono::Utc::now().timestamp_millis()),
                proposal_id: proposal.proposal_id.clone(),
                review_id: crate::eval_proposal_review::AutoCommitProposalReviewId("pending".into()),
                workspace_hash: proposal.workspace_snapshot_id.clone(),
                summary: f.to_string(),
                required_changes: correction_reasons.iter().map(|r| crate::eval_proposal_review::RequiredProposalChange { category: crate::eval_proposal_review::ProposalFeedbackCategory::Other, description: r.clone(), evidence_ref: None }).collect(),
                blocked_dimensions: vec![],
                suggested_next_eval_focus: vec![],
                severity: crate::eval_proposal_review::ProposalFeedbackSeverity::Blocking,
            }
        }),
    )
    .map_err(|e| format!("{}", e))
}

fn build_push_review(
    proposal_id: &str,
    reviewer: &str,
    rationale: &str,
    feedback: Option<&str>,
    store_root: &std::path::Path,
) -> Result<RemotePushProposalReview, String> {
    let pid = RemotePushProposalId(proposal_id.to_string());
    let proposal = load_push_proposal(store_root, &pid)
        .map_err(|e| format!("{}", e))?
        .ok_or_else(|| format!("Push proposal not found: {}", proposal_id))?;

    let decision = if feedback.is_some() && rationale.contains("reject") {
        RemotePushProposalReviewDecision::Rejected
    } else if feedback.is_some() {
        RemotePushProposalReviewDecision::ChangesRequested
    } else {
        RemotePushProposalReviewDecision::Approved
    };

    let fb = feedback.map(|f| RemotePushProposalFeedback {
        summary: f.to_string(),
        blocking_reasons: if matches!(decision, RemotePushProposalReviewDecision::Rejected) {
            vec![f.to_string()]
        } else {
            vec![]
        },
        requested_changes: if matches!(decision, RemotePushProposalReviewDecision::ChangesRequested) {
            vec![f.to_string()]
        } else {
            vec![]
        },
        evidence_gaps: vec![],
        suggested_next_action: String::new(),
    });

    use crate::eval_remote_push_proposal::RemotePushProposalReviewRequest;
    let req = RemotePushProposalReviewRequest {
        proposal_id: pid,
        decision,
        reviewer: reviewer.to_string(),
        rationale: rationale.to_string(),
        feedback: fb,
        idempotency_key: format!("ui_{}", chrono::Utc::now().timestamp_millis()),
    };

    build_push_proposal_review(&proposal, &req, &[]).map_err(|e| format!("{}", e))
}
