//! Governance console state projection.
//!
//! Read-only projection of the governed execution chain for UI display.
//! This module never mutates any state, calls git/shell/execution backends,
//! or constructs review/execution records.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::eval_post_commit_verify::{
    PostCommitVerificationRecord,
    load_latest_verification,
};
use crate::eval_proposal::{
    AutoCommitProposal,
    load_latest_proposal,
};
use crate::eval_proposal_execution::{
    AutoCommitExecutionRecord,
    load_latest_execution,
};
use crate::eval_proposal_review::{
    AutoCommitProposalReview,
    load_latest_proposal_review,
};
use crate::eval_remote_push_execution::{
    RemotePushExecutionRecord,
    load_latest_push_execution,
};
use crate::eval_remote_push_proposal::{
    RemotePushProposal,
    RemotePushProposalReview,
    load_latest_push_proposal, load_latest_push_review,
};
use crate::eval_remote_push_readiness::{
    RemotePushReadinessRecord,
    load_latest_readiness,
};

// ── DTOs ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GovernanceRecordKind {
    LocalProposal,
    LocalReview,
    LocalExecution,
    PostCommitVerification,
    PushReadiness,
    PushProposal,
    PushReview,
    PushExecution,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GovernanceRecordSummary {
    pub kind: GovernanceRecordKind,
    pub id: String,
    pub status: String,
    pub decision: Option<String>,
    pub hash: Option<String>,
    pub linked_ids: Vec<(String, String)>,
    pub created_at: Option<DateTime<Utc>>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GovernancePredicateSummary {
    pub source_record_id: String,
    pub source_kind: GovernanceRecordKind,
    pub predicate: String,
    pub passed: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GovernanceFeedbackSummary {
    pub review_id: String,
    pub kind: GovernanceRecordKind,
    pub summary: String,
    pub blocking_reasons: Vec<String>,
    pub requested_changes: Vec<String>,
    pub evidence_gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GovernanceConsoleState {
    pub local_proposal: Option<GovernanceRecordSummary>,
    pub local_review: Option<GovernanceRecordSummary>,
    pub local_execution: Option<GovernanceRecordSummary>,
    pub post_commit_verification: Option<GovernanceRecordSummary>,
    pub push_readiness: Option<GovernanceRecordSummary>,
    pub push_proposal: Option<GovernanceRecordSummary>,
    pub push_review: Option<GovernanceRecordSummary>,
    pub push_execution: Option<GovernanceRecordSummary>,
    pub predicates: Vec<GovernancePredicateSummary>,
    pub feedback: Vec<GovernanceFeedbackSummary>,
    pub chain_warnings: Vec<String>,
}

impl GovernanceConsoleState {
    pub fn empty() -> Self {
        Self {
            local_proposal: None,
            local_review: None,
            local_execution: None,
            post_commit_verification: None,
            push_readiness: None,
            push_proposal: None,
            push_review: None,
            push_execution: None,
            predicates: vec![],
            feedback: vec![],
            chain_warnings: vec![],
        }
    }
}

// ── Summary builders ────────────────────────────────────────────────────────

fn proposal_summary(p: &AutoCommitProposal) -> GovernanceRecordSummary {
    GovernanceRecordSummary {
        kind: GovernanceRecordKind::LocalProposal,
        id: p.proposal_id.0.clone(),
        status: format!("{:?}", p.status),
        decision: None,
        hash: Some(p.workspace_snapshot_id.clone()),
        linked_ids: vec![],
        created_at: Some(p.generated_at),
        summary: p.commit_title.clone(),
    }
}

fn review_summary(r: &AutoCommitProposalReview) -> GovernanceRecordSummary {
    GovernanceRecordSummary {
        kind: GovernanceRecordKind::LocalReview,
        id: r.review_id.0.clone(),
        status: format!("{:?}", r.decision),
        decision: Some(format!("{:?}", r.decision)),
        hash: Some(r.proposal_hash.clone()),
        linked_ids: vec![("proposal_id".into(), r.proposal_id.0.clone())],
        created_at: Some(r.reviewed_at),
        summary: r.rationale.clone(),
    }
}

fn execution_summary(e: &AutoCommitExecutionRecord) -> GovernanceRecordSummary {
    let summary = match &e.resulting_commit {
        Some(c) => format!("commit {}", &c.commit_hash[..8.min(c.commit_hash.len())]),
        None => "no commit".into(),
    };
    GovernanceRecordSummary {
        kind: GovernanceRecordKind::LocalExecution,
        id: e.execution_id.0.clone(),
        status: format!("{:?}", e.status),
        decision: Some(format!("{:?}", e.decision.decision)),
        hash: None,
        linked_ids: vec![
            ("proposal_id".into(), e.proposal_id.0.clone()),
            ("review_id".into(), e.review_id.0.clone()),
        ],
        created_at: Some(e.created_at),
        summary,
    }
}

fn verification_summary(v: &PostCommitVerificationRecord) -> GovernanceRecordSummary {
    GovernanceRecordSummary {
        kind: GovernanceRecordKind::PostCommitVerification,
        id: v.verification_id.0.clone(),
        status: format!("{:?}", v.status),
        decision: Some(format!("{:?}", v.decision)),
        hash: v.commit_evidence.as_ref().map(|e| e.diff_hash.clone()),
        linked_ids: vec![
            ("execution_id".into(), v.execution_id.0.clone()),
            ("proposal_id".into(), v.proposal_id.0.clone()),
        ],
        created_at: Some(v.created_at),
        summary: format!("{:?}", v.status),
    }
}

fn readiness_summary(r: &RemotePushReadinessRecord) -> GovernanceRecordSummary {
    GovernanceRecordSummary {
        kind: GovernanceRecordKind::PushReadiness,
        id: r.readiness_id.0.clone(),
        status: format!("{:?}", r.status),
        decision: Some(format!("{:?}", r.decision)),
        hash: None,
        linked_ids: vec![
            ("verification_id".into(), r.verification_id.0.clone()),
            ("execution_id".into(), r.execution_id.0.clone()),
        ],
        created_at: Some(r.created_at),
        summary: format!("{}/{} {:?}", r.target_remote, r.target_branch, r.status),
    }
}

fn push_proposal_summary(p: &RemotePushProposal) -> GovernanceRecordSummary {
    GovernanceRecordSummary {
        kind: GovernanceRecordKind::PushProposal,
        id: p.proposal_id.0.clone(),
        status: format!("{:?}", p.status),
        decision: None,
        hash: Some(p.proposal_hash.clone()),
        linked_ids: vec![
            ("readiness_id".into(), p.readiness_id.0.clone()),
            ("verification_id".into(), p.verification_id.0.clone()),
        ],
        created_at: Some(p.created_at),
        summary: format!("push {}/{} {:?}", p.target_remote, p.target_branch, p.status),
    }
}

fn push_review_summary(r: &RemotePushProposalReview) -> GovernanceRecordSummary {
    GovernanceRecordSummary {
        kind: GovernanceRecordKind::PushReview,
        id: r.review_id.0.clone(),
        status: format!("{:?}", r.decision),
        decision: Some(format!("{:?}", r.decision)),
        hash: Some(r.proposal_hash.clone()),
        linked_ids: vec![("proposal_id".into(), r.proposal_id.0.clone())],
        created_at: Some(r.reviewed_at),
        summary: r.rationale.clone(),
    }
}

fn push_execution_summary(e: &RemotePushExecutionRecord) -> GovernanceRecordSummary {
    GovernanceRecordSummary {
        kind: GovernanceRecordKind::PushExecution,
        id: e.execution_id.0.clone(),
        status: format!("{:?}", e.status),
        decision: Some(format!("{:?}", e.decision)),
        hash: None,
        linked_ids: vec![
            ("proposal_id".into(), e.proposal_id.0.clone()),
            ("review_id".into(), e.review_id.0.clone()),
        ],
        created_at: Some(e.created_at),
        summary: format!("{}/{} {:?}", e.target_remote, e.target_branch, e.status),
    }
}

// ── Feedback extraction ─────────────────────────────────────────────────────

fn extract_local_feedback(r: &AutoCommitProposalReview) -> Option<GovernanceFeedbackSummary> {
    let blocking_reasons: Vec<String> = r.checklist.iter()
        .filter(|c| !c.checked)
        .map(|c| c.description.clone())
        .collect();
    let (summary, requested_changes) = match &r.feedback {
        Some(fb) => (fb.summary.clone(), fb.required_changes.iter().map(|c| c.description.clone()).collect()),
        None => (r.rationale.clone(), vec![]),
    };
    Some(GovernanceFeedbackSummary {
        review_id: r.review_id.0.clone(),
        kind: GovernanceRecordKind::LocalReview,
        summary,
        blocking_reasons,
        requested_changes,
        evidence_gaps: vec![],
    })
}

fn extract_push_feedback(r: &RemotePushProposalReview) -> Option<GovernanceFeedbackSummary> {
    r.feedback.as_ref().map(|fb| GovernanceFeedbackSummary {
        review_id: r.review_id.0.clone(),
        kind: GovernanceRecordKind::PushReview,
        summary: fb.summary.clone(),
        blocking_reasons: fb.blocking_reasons.clone(),
        requested_changes: fb.requested_changes.clone(),
        evidence_gaps: fb.evidence_gaps.clone(),
    })
}

// ── Chain consistency checks (Patch 4) ──────────────────────────────────────

fn check_chain_warnings(state: &mut GovernanceConsoleState) {
    // Local review → local proposal
    if let (Some(review), Some(proposal)) = (&state.local_review, &state.local_proposal) {
        let linked = review.linked_ids.iter().find(|(k, _)| k == "proposal_id");
        if let Some((_, rid)) = linked
            && rid != &proposal.id {
                state.chain_warnings.push(format!(
                    "Local review {} references proposal {} but latest proposal is {}",
                    review.id, rid, proposal.id
                ));
            }
    }

    // Local execution → proposal + review
    if let Some(exec) = &state.local_execution {
        let exec_proposal = exec.linked_ids.iter().find(|(k, _)| k == "proposal_id").map(|(_, v)| v.clone());
        let exec_review = exec.linked_ids.iter().find(|(k, _)| k == "review_id").map(|(_, v)| v.clone());

        if let (Some(ep), Some(proposal)) = (&exec_proposal, &state.local_proposal)
            && ep != &proposal.id {
                state.chain_warnings.push(format!(
                    "Local execution {} references proposal {} but latest proposal is {}",
                    exec.id, ep, proposal.id
                ));
            }
        if let (Some(er), Some(review)) = (&exec_review, &state.local_review)
            && er != &review.id {
                state.chain_warnings.push(format!(
                    "Local execution {} references review {} but latest review is {}",
                    exec.id, er, review.id
                ));
            }
    }

    // Verification → execution
    if let Some(verify) = &state.post_commit_verification {
        let verify_exec = verify.linked_ids.iter().find(|(k, _)| k == "execution_id").map(|(_, v)| v.clone());
        if let (Some(ve), Some(exec)) = (&verify_exec, &state.local_execution)
            && ve != &exec.id {
                state.chain_warnings.push(format!(
                    "Verification {} references execution {} but latest execution is {}",
                    verify.id, ve, exec.id
                ));
            }
    }

    // Push readiness → verification
    if let Some(readiness) = &state.push_readiness {
        let readiness_verify = readiness.linked_ids.iter().find(|(k, _)| k == "verification_id").map(|(_, v)| v.clone());
        if let (Some(rv), Some(verify)) = (&readiness_verify, &state.post_commit_verification)
            && rv != &verify.id {
                state.chain_warnings.push(format!(
                    "Push readiness {} references verification {} but latest verification is {}",
                    readiness.id, rv, verify.id
                ));
            }
    }

    // Push proposal → readiness
    if let Some(proposal) = &state.push_proposal {
        let proposal_readiness = proposal.linked_ids.iter().find(|(k, _)| k == "readiness_id").map(|(_, v)| v.clone());
        if let (Some(pr), Some(readiness)) = (&proposal_readiness, &state.push_readiness)
            && pr != &readiness.id {
                state.chain_warnings.push(format!(
                    "Push proposal {} references readiness {} but latest readiness is {}",
                    proposal.id, pr, readiness.id
                ));
            }
    }

    // Push review → push proposal
    if let (Some(review), Some(proposal)) = (&state.push_review, &state.push_proposal) {
        let linked = review.linked_ids.iter().find(|(k, _)| k == "proposal_id");
        if let Some((_, rid)) = linked
            && rid != &proposal.id {
                state.chain_warnings.push(format!(
                    "Push review {} references proposal {} but latest push proposal is {}",
                    review.id, rid, proposal.id
                ));
            }
    }

    // Push execution → push proposal + push review
    if let Some(exec) = &state.push_execution {
        let exec_proposal = exec.linked_ids.iter().find(|(k, _)| k == "proposal_id").map(|(_, v)| v.clone());
        let exec_review = exec.linked_ids.iter().find(|(k, _)| k == "review_id").map(|(_, v)| v.clone());

        if let (Some(ep), Some(proposal)) = (&exec_proposal, &state.push_proposal)
            && ep != &proposal.id {
                state.chain_warnings.push(format!(
                    "Push execution {} references proposal {} but latest push proposal is {}",
                    exec.id, ep, proposal.id
                ));
            }
        if let (Some(er), Some(review)) = (&exec_review, &state.push_review)
            && er != &review.id {
                state.chain_warnings.push(format!(
                    "Push execution {} references review {} but latest push review is {}",
                    exec.id, er, review.id
                ));
            }
    }
}

// ── Loader ──────────────────────────────────────────────────────────────────

pub fn load_governance_console(store_root: &std::path::Path) -> GovernanceConsoleState {
    let mut state = GovernanceConsoleState::empty();

    // Load local proposal
    if let Ok(Some(p)) = load_latest_proposal(store_root) {
        state.local_proposal = Some(proposal_summary(&p));
    }

    // Load local review
    if let Ok(Some(r)) = load_latest_proposal_review(store_root) {
        if let Some(fb) = extract_local_feedback(&r) {
            state.feedback.push(fb);
        }
        state.local_review = Some(review_summary(&r));
    }

    // Load local execution
    if let Ok(Some(e)) = load_latest_execution(store_root) {
        state.local_execution = Some(execution_summary(&e));
    }

    // Load verification
    if let Ok(Some(v)) = load_latest_verification(store_root) {
        // Extract predicates from verification
        for pr in &v.predicates {
            state.predicates.push(GovernancePredicateSummary {
                source_record_id: v.verification_id.0.clone(),
                source_kind: GovernanceRecordKind::PostCommitVerification,
                predicate: format!("{:?}", pr.predicate),
                passed: pr.passed,
                reason: pr.reason.clone(),
            });
        }
        state.post_commit_verification = Some(verification_summary(&v));
    }

    // Load push readiness
    if let Ok(Some(r)) = load_latest_readiness(store_root) {
        for pr in &r.predicates {
            state.predicates.push(GovernancePredicateSummary {
                source_record_id: r.readiness_id.0.clone(),
                source_kind: GovernanceRecordKind::PushReadiness,
                predicate: format!("{:?}", pr.predicate),
                passed: pr.passed,
                reason: pr.reason.clone(),
            });
        }
        state.push_readiness = Some(readiness_summary(&r));
    }

    // Load push proposal
    if let Ok(Some(p)) = load_latest_push_proposal(store_root) {
        state.push_proposal = Some(push_proposal_summary(&p));
    }

    // Load push review
    if let Ok(Some(r)) = load_latest_push_review(store_root) {
        if let Some(fb) = extract_push_feedback(&r) {
            state.feedback.push(fb);
        }
        state.push_review = Some(push_review_summary(&r));
    }

    // Load push execution
    if let Ok(Some(e)) = load_latest_push_execution(store_root) {
        for pr in &e.predicates {
            state.predicates.push(GovernancePredicateSummary {
                source_record_id: e.execution_id.0.clone(),
                source_kind: GovernanceRecordKind::PushExecution,
                predicate: format!("{:?}", pr.predicate),
                passed: pr.passed,
                reason: pr.reason.clone(),
            });
        }
        state.push_execution = Some(push_execution_summary(&e));
    }

    // Patch 4: chain consistency warnings
    check_chain_warnings(&mut state);

    state
}
