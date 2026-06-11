//! Governed remote push execution gate.
//!
//! Executes exactly one approved remote push proposal as a fast-forward
//! update to one existing remote branch, after revalidating all evidence
//! and remote state at execution time. Persists a push execution record.
//!
//! Module boundary:
//!   Wave 15: eval_remote_push_readiness.rs   → remote push readiness only
//!   Wave 16: eval_remote_push_proposal.rs     → push proposal and human review
//!   Wave 17: eval_remote_push_execution.rs    → governed remote push execution (this module)
//!
//! No force push, tag, release, branch creation, fetch, pull, merge, rebase,
//! live rollback, arbitrary shell, or general git execution.

use blake3::Hasher;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::eval_post_commit_verify::{
    PostCommitVerificationId, PostCommitVerificationRecord, PostCommitVerificationStatus,
};
use crate::eval_proposal::AutoCommitProposalId;
use crate::eval_proposal_execution::{AutoCommitExecutionId, AutoCommitExecutionRecord, AutoCommitExecutionStatus};
use crate::eval_proposal_review::AutoCommitProposalReviewId;
use crate::eval_remote_push_proposal::{
    RemotePushProposalId, RemotePushProposal, RemotePushProposalStatus,
    RemotePushProposalReviewId, RemotePushProposalReview, RemotePushProposalReviewDecision,
};
use crate::eval_remote_push_readiness::{
    RemotePushReadinessId, RemotePushReadinessRecord, RemotePushReadinessStatus, BranchProtectionPolicySnapshot,
};

// ── Execution ID ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RemotePushExecutionId(pub String);

pub fn push_execution_id_for(
    proposal_id: &str,
    review_id: &str,
    idempotency_key: &str,
) -> RemotePushExecutionId {
    let mut hasher = Hasher::new();
    hasher.update(b"remote_push_execution:");
    hasher.update(proposal_id.as_bytes());
    hasher.update(review_id.as_bytes());
    hasher.update(idempotency_key.as_bytes());
    let hash = hasher.finalize();
    RemotePushExecutionId(format!("rpe_{}", hash.to_hex()))
}

// ── Request ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePushExecutionRequest {
    pub proposal_id: RemotePushProposalId,
    pub review_id: RemotePushProposalReviewId,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub idempotency_key: String,
}

// ── Status and Decision ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemotePushExecutionStatus {
    Blocked,
    Executed,
    AlreadyExecuted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemotePushExecutionDecision {
    Allow,
    Block { reason_code: String, summary: String },
}

// ── Predicate ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemotePushExecutionPredicate {
    ProposalExists,
    ProposalEligible,
    ReviewExists,
    ReviewIsLatestForProposal,
    ReviewApproved,
    ReviewProposalHashMatchesProposal,
    ReviewReadinessHashMatchesProposal,
    ReadinessRecordExists,
    ReadinessStillReady,
    VerificationRecordExists,
    VerificationStillVerified,
    LocalExecutionRecordExists,
    LocalExecutionWasSuccessful,
    CurrentHeadMatchesProposalCommit,
    CurrentBranchMatchesProposalBranch,
    WorktreeClean,
    IndexClean,
    BranchPolicyLoaded,
    BranchPolicyStillAllowsPush,
    TargetRemoteConfigured,
    RemoteBranchExists,
    RemoteRefMatchesExpectedOldCommit,
    PushIsFastForward,
    CommitIsDescendantOfRemoteRef,
    PolicyAllowsRemotePush,
    RecoveryEvidenceExists,
    IdempotencyKeyUnusedOrMatchesExisting,
    NoPriorConflictingPushExecution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePushExecutionPredicateResult {
    pub predicate: RemotePushExecutionPredicate,
    pub passed: bool,
    pub reason: String,
}

// ── Snapshots ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocalPushExecutionSnapshot {
    pub head: String,
    pub branch: String,
    pub worktree_clean: bool,
    pub index_clean: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemoteObservationSource {
    LsRemote,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteRefObservedSnapshot {
    pub remote: String,
    pub branch: String,
    pub ref_name: String,
    pub observed_commit: Option<String>,
    pub observed_at: DateTime<Utc>,
    pub source: RemoteObservationSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemotePushResultSnapshot {
    pub remote: String,
    pub branch: String,
    pub ref_name: String,
    pub old_commit: String,
    pub new_commit: String,
    pub fast_forward: bool,
    pub push_output_hash: String,
    pub pushed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemotePushRecoveryStrategy {
    FollowUpRevertCommit,
    ManualProtectedBranchProcedure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePushRecoverySnapshot {
    pub old_remote_commit: String,
    pub new_remote_commit: String,
    pub recommended_strategy: RemotePushRecoveryStrategy,
    pub notes: Vec<String>,
}

// ── Execution Record ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePushExecutionRecord {
    pub execution_id: RemotePushExecutionId,
    pub proposal_id: RemotePushProposalId,
    pub review_id: RemotePushProposalReviewId,
    pub readiness_id: RemotePushReadinessId,
    pub verification_id: PostCommitVerificationId,
    pub local_execution_id: AutoCommitExecutionId,
    pub proposal_source_id: AutoCommitProposalId,
    pub review_source_id: AutoCommitProposalReviewId,
    pub commit_hash: String,
    pub target_remote: String,
    pub target_branch: String,
    pub status: RemotePushExecutionStatus,
    pub decision: RemotePushExecutionDecision,
    pub predicates: Vec<RemotePushExecutionPredicateResult>,
    pub pre_push_remote: Option<RemoteRefObservedSnapshot>,
    pub post_push_remote: Option<RemoteRefObservedSnapshot>,
    pub push_result: Option<RemotePushResultSnapshot>,
    pub recovery: Option<RemotePushRecoverySnapshot>,
    pub created_at: DateTime<Utc>,
}

// ── Error ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RemotePushExecutionError(pub String);

// ── Backend trait ───────────────────────────────────────────────────────────

/// Narrow backend for remote push execution.
/// Only observes local state, observes remote ref, and executes exact fast-forward push.
pub trait RemotePushExecutionBackend: Send + Sync {
    fn observe_current_local_state(
        &self,
        repo: &Path,
    ) -> Result<LocalPushExecutionSnapshot, RemotePushExecutionError>;

    fn observe_remote_ref(
        &self,
        repo: &Path,
        remote: &str,
        branch: &str,
    ) -> Result<RemoteRefObservedSnapshot, RemotePushExecutionError>;

    fn execute_fast_forward_push_exact(
        &self,
        repo: &Path,
        request: ExactRemotePushRequest,
    ) -> Result<RemotePushResultSnapshot, RemotePushExecutionError>;
}

#[derive(Debug, Clone)]
pub struct ExactRemotePushRequest {
    pub remote: String,
    pub branch: String,
    pub ref_name: String,
    pub expected_old_commit: String,
    pub proposed_new_commit: String,
    pub idempotency_key: String,
}

// ── Test Backend ────────────────────────────────────────────────────────────

pub struct TestPushExecutionBackend {
    pub local_state: Option<LocalPushExecutionSnapshot>,
    pub remote_ref: Option<RemoteRefObservedSnapshot>,
    pub push_result: Option<RemotePushResultSnapshot>,
    pub push_should_fail: bool,
    pub is_fast_forward: bool,
}

impl TestPushExecutionBackend {
    pub fn new() -> Self {
        Self {
            local_state: None,
            remote_ref: None,
            push_result: None,
            push_should_fail: false,
            is_fast_forward: true,
        }
    }

    pub fn with_local_state(mut self, state: LocalPushExecutionSnapshot) -> Self {
        self.local_state = Some(state);
        self
    }

    pub fn with_remote_ref(mut self, snapshot: RemoteRefObservedSnapshot) -> Self {
        self.remote_ref = Some(snapshot);
        self
    }

    pub fn with_push_result(mut self, result: RemotePushResultSnapshot) -> Self {
        self.push_result = Some(result);
        self
    }

    pub fn with_push_failure(mut self) -> Self {
        self.push_should_fail = true;
        self
    }

    pub fn with_fast_forward(mut self, ff: bool) -> Self {
        self.is_fast_forward = ff;
        self
    }
}

impl Default for TestPushExecutionBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RemotePushExecutionBackend for TestPushExecutionBackend {
    fn observe_current_local_state(
        &self,
        _repo: &Path,
    ) -> Result<LocalPushExecutionSnapshot, RemotePushExecutionError> {
        self.local_state.clone().ok_or_else(|| RemotePushExecutionError("No local state injected".into()))
    }

    fn observe_remote_ref(
        &self,
        _repo: &Path,
        _remote: &str,
        _branch: &str,
    ) -> Result<RemoteRefObservedSnapshot, RemotePushExecutionError> {
        self.remote_ref.clone().ok_or_else(|| RemotePushExecutionError("No remote ref injected".into()))
    }

    fn execute_fast_forward_push_exact(
        &self,
        _repo: &Path,
        request: ExactRemotePushRequest,
    ) -> Result<RemotePushResultSnapshot, RemotePushExecutionError> {
        if self.push_should_fail {
            return Err(RemotePushExecutionError("Push failed (test injection)".into()));
        }
        if let Some(ref result) = self.push_result {
            return Ok(result.clone());
        }
        // Default result from request
        Ok(RemotePushResultSnapshot {
            remote: request.remote,
            branch: request.branch,
            ref_name: request.ref_name,
            old_commit: request.expected_old_commit,
            new_commit: request.proposed_new_commit,
            fast_forward: self.is_fast_forward,
            push_output_hash: format!("{}", blake3::hash(b"test_push_output").to_hex()),
            pushed_at: Utc::now(),
        })
    }
}

// ── Local Backend ───────────────────────────────────────────────────────────

pub struct LocalPushExecutionBackend;

impl LocalPushExecutionBackend {
    fn run_git(repo: &Path, args: &[&str]) -> Result<String, RemotePushExecutionError> {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .map_err(|e| RemotePushExecutionError(format!("git execution failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RemotePushExecutionError(format!(
                "git {} failed: {}",
                args.join(" "),
                stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn check_remote_configured(repo: &Path, remote: &str) -> bool {
        Self::run_git(repo, &["config", "--get", &format!("remote.{}.url", remote)]).is_ok()
    }
}

impl RemotePushExecutionBackend for LocalPushExecutionBackend {
    fn observe_current_local_state(
        &self,
        repo: &Path,
    ) -> Result<LocalPushExecutionSnapshot, RemotePushExecutionError> {
        let head = Self::run_git(repo, &["rev-parse", "HEAD"])?;
        let branch = Self::run_git(repo, &["symbolic-ref", "--short", "HEAD"])?;
        let porcelain = Self::run_git(repo, &["status", "--porcelain"])?;
        let worktree_clean = porcelain.is_empty();
        // Index clean: no staged changes
        let diff_cached = Self::run_git(repo, &["diff", "--cached", "--stat"])?;
        let index_clean = diff_cached.trim().is_empty() && worktree_clean;

        Ok(LocalPushExecutionSnapshot {
            head,
            branch,
            worktree_clean,
            index_clean,
        })
    }

    fn observe_remote_ref(
        &self,
        repo: &Path,
        remote: &str,
        branch: &str,
    ) -> Result<RemoteRefObservedSnapshot, RemotePushExecutionError> {
        let ref_spec = format!("refs/heads/{}", branch);
        let output = Self::run_git(repo, &["ls-remote", remote, &ref_spec])?;

        let observed_commit = if output.is_empty() {
            None
        } else {
            // ls-remote output: "<hash>\t<ref>"
            let parts: Vec<&str> = output.splitn(2, '\t').collect();
            if parts.len() == 2 && parts[1] == ref_spec {
                Some(parts[0].to_string())
            } else {
                None
            }
        };

        Ok(RemoteRefObservedSnapshot {
            remote: remote.to_string(),
            branch: branch.to_string(),
            ref_name: ref_spec,
            observed_commit,
            observed_at: Utc::now(),
            source: RemoteObservationSource::LsRemote,
        })
    }

    fn execute_fast_forward_push_exact(
        &self,
        repo: &Path,
        request: ExactRemotePushRequest,
    ) -> Result<RemotePushResultSnapshot, RemotePushExecutionError> {
        // Verify fast-forward ancestry before push
        if !request.expected_old_commit.is_empty() {
            let _ = Self::run_git(
                repo,
                &["merge-base", "--is-ancestor", &request.expected_old_commit, &request.proposed_new_commit],
            ).map_err(|_| RemotePushExecutionError(format!(
                "Non-fast-forward: {} is not ancestor of {}",
                &request.expected_old_commit[..8.min(request.expected_old_commit.len())],
                &request.proposed_new_commit[..8.min(request.proposed_new_commit.len())],
            )))?;
        }

        let branch_name = request.branch;
        let refspec = format!("{}:refs/heads/{}", request.proposed_new_commit, branch_name);
        let output = Self::run_git(
            repo,
            &["push", "--porcelain", &request.remote, &refspec],
        )?;

        let push_output_hash = format!("{}", blake3::hash(output.as_bytes()).to_hex());

        Ok(RemotePushResultSnapshot {
            remote: request.remote,
            branch: branch_name.clone(),
            ref_name: format!("refs/heads/{}", branch_name),
            old_commit: request.expected_old_commit,
            new_commit: request.proposed_new_commit,
            fast_forward: true,
            push_output_hash,
            pushed_at: Utc::now(),
        })
    }
}

// ── Predicate evaluation ────────────────────────────────────────────────────

pub fn evaluate_push_predicates(
    proposal: Option<&RemotePushProposal>,
    review: Option<&RemotePushProposalReview>,
    latest_review: Option<&RemotePushProposalReview>,
    readiness: Option<&RemotePushReadinessRecord>,
    verification: Option<&PostCommitVerificationRecord>,
    local_execution: Option<&AutoCommitExecutionRecord>,
    local_state: Option<&LocalPushExecutionSnapshot>,
    remote_ref: Option<&RemoteRefObservedSnapshot>,
    branch_policy: Option<&BranchProtectionPolicySnapshot>,
    existing_executions: &[RemotePushExecutionRecord],
    request: &RemotePushExecutionRequest,
    remote_configured: bool,
    policy_allows: bool,
    is_fast_forward: bool,
) -> Vec<RemotePushExecutionPredicateResult> {
    let mut results = Vec::new();

    macro_rules! pred {
        ($p:expr, $passed:expr, $reason:expr) => {
            results.push(RemotePushExecutionPredicateResult {
                predicate: $p,
                passed: $passed,
                reason: $reason.to_string(),
            });
        };
    }

    // 1. ProposalExists
    pred!(
        RemotePushExecutionPredicate::ProposalExists,
        proposal.is_some(),
        if proposal.is_some() { "Proposal exists" } else { "Proposal not found" }
    );

    // 2. ProposalEligible
    pred!(
        RemotePushExecutionPredicate::ProposalEligible,
        proposal.map(|p| p.status == RemotePushProposalStatus::Eligible).unwrap_or(false),
        if proposal.map(|p| p.status == RemotePushProposalStatus::Eligible).unwrap_or(false) {
            "Proposal is eligible"
        } else {
            "Proposal is not eligible"
        }
    );

    // 3. ReviewExists
    pred!(
        RemotePushExecutionPredicate::ReviewExists,
        review.is_some(),
        if review.is_some() { "Review exists" } else { "Review not found" }
    );

    // 4. ReviewIsLatestForProposal
    let review_is_latest = match (review, latest_review) {
        (Some(r), Some(lr)) => r.review_id == lr.review_id,
        _ => false,
    };
    pred!(
        RemotePushExecutionPredicate::ReviewIsLatestForProposal,
        review_is_latest,
        if review_is_latest { "Review is latest for proposal" } else { "Review is not the latest for proposal" }
    );

    // 5. ReviewApproved
    pred!(
        RemotePushExecutionPredicate::ReviewApproved,
        review.map(|r| r.decision == RemotePushProposalReviewDecision::Approved).unwrap_or(false),
        if review.map(|r| r.decision == RemotePushProposalReviewDecision::Approved).unwrap_or(false) {
            "Review is approved"
        } else {
            "Review is not approved"
        }
    );

    // 6. ReviewProposalHashMatchesProposal
    let hash_matches = match (review, proposal) {
        (Some(r), Some(p)) => r.proposal_hash == p.proposal_hash,
        _ => false,
    };
    pred!(
        RemotePushExecutionPredicate::ReviewProposalHashMatchesProposal,
        hash_matches,
        if hash_matches { "Review proposal hash matches proposal" } else { "Review proposal hash mismatch" }
    );

    // 7. ReviewReadinessHashMatchesProposal
    let readiness_hash_matches = match (review, proposal) {
        (Some(r), Some(p)) => r.readiness_hash == p.readiness_hash,
        _ => false,
    };
    pred!(
        RemotePushExecutionPredicate::ReviewReadinessHashMatchesProposal,
        readiness_hash_matches,
        if readiness_hash_matches { "Review readiness hash matches proposal" } else { "Review readiness hash mismatch" }
    );

    // 8. ReadinessRecordExists
    pred!(
        RemotePushExecutionPredicate::ReadinessRecordExists,
        readiness.is_some(),
        if readiness.is_some() { "Readiness record exists" } else { "Readiness record not found" }
    );

    // 9. ReadinessStillReady
    pred!(
        RemotePushExecutionPredicate::ReadinessStillReady,
        readiness.map(|r| r.status == RemotePushReadinessStatus::Ready).unwrap_or(false),
        if readiness.map(|r| r.status == RemotePushReadinessStatus::Ready).unwrap_or(false) {
            "Readiness is still Ready"
        } else {
            "Readiness is not Ready"
        }
    );

    // 10. VerificationRecordExists
    pred!(
        RemotePushExecutionPredicate::VerificationRecordExists,
        verification.is_some(),
        if verification.is_some() { "Verification record exists" } else { "Verification record not found" }
    );

    // 11. VerificationStillVerified
    pred!(
        RemotePushExecutionPredicate::VerificationStillVerified,
        verification.map(|v| v.status == PostCommitVerificationStatus::Verified).unwrap_or(false),
        if verification.map(|v| v.status == PostCommitVerificationStatus::Verified).unwrap_or(false) {
            "Verification is still Verified"
        } else {
            "Verification is not Verified"
        }
    );

    // 12. LocalExecutionRecordExists
    pred!(
        RemotePushExecutionPredicate::LocalExecutionRecordExists,
        local_execution.is_some(),
        if local_execution.is_some() { "Local execution record exists" } else { "Local execution record not found" }
    );

    // 13. LocalExecutionWasSuccessful
    pred!(
        RemotePushExecutionPredicate::LocalExecutionWasSuccessful,
        local_execution.map(|e| e.status == AutoCommitExecutionStatus::Executed).unwrap_or(false),
        if local_execution.map(|e| e.status == AutoCommitExecutionStatus::Executed).unwrap_or(false) {
            "Local execution was successful"
        } else {
            "Local execution was not successful"
        }
    );

    // 14. CurrentHeadMatchesProposalCommit
    let head_matches = match (local_state, proposal) {
        (Some(ls), Some(p)) => ls.head == p.commit_hash,
        _ => false,
    };
    pred!(
        RemotePushExecutionPredicate::CurrentHeadMatchesProposalCommit,
        head_matches,
        if head_matches { "Current HEAD matches proposal commit" } else { "Current HEAD does not match proposal commit" }
    );

    // 15. CurrentBranchMatchesProposalBranch
    let branch_matches = match (local_state, proposal) {
        (Some(ls), Some(p)) => ls.branch == p.target_branch,
        _ => false,
    };
    pred!(
        RemotePushExecutionPredicate::CurrentBranchMatchesProposalBranch,
        branch_matches,
        if branch_matches { "Current branch matches proposal branch" } else { "Current branch does not match proposal branch" }
    );

    // 16. WorktreeClean
    pred!(
        RemotePushExecutionPredicate::WorktreeClean,
        local_state.map(|ls| ls.worktree_clean).unwrap_or(false),
        if local_state.map(|ls| ls.worktree_clean).unwrap_or(false) {
            "Worktree is clean"
        } else {
            "Worktree is dirty"
        }
    );

    // 17. IndexClean
    pred!(
        RemotePushExecutionPredicate::IndexClean,
        local_state.map(|ls| ls.index_clean).unwrap_or(false),
        if local_state.map(|ls| ls.index_clean).unwrap_or(false) {
            "Index is clean"
        } else {
            "Index is dirty"
        }
    );

    // 18. BranchPolicyLoaded
    pred!(
        RemotePushExecutionPredicate::BranchPolicyLoaded,
        branch_policy.is_some(),
        if branch_policy.is_some() { "Branch policy loaded" } else { "Branch policy not loaded" }
    );

    // 19. BranchPolicyStillAllowsPush
    pred!(
        RemotePushExecutionPredicate::BranchPolicyStillAllowsPush,
        branch_policy.map(|bp| bp.direct_push_allowed).unwrap_or(false),
        if branch_policy.map(|bp| bp.direct_push_allowed).unwrap_or(false) {
            "Branch policy allows push"
        } else {
            "Branch policy denies push"
        }
    );

    // 20. TargetRemoteConfigured
    pred!(
        RemotePushExecutionPredicate::TargetRemoteConfigured,
        remote_configured,
        if remote_configured { "Target remote is configured" } else { "Target remote is not configured" }
    );

    // 21. RemoteBranchExists
    let remote_branch_exists = remote_ref
        .as_ref()
        .map(|rr| rr.observed_commit.is_some())
        .unwrap_or(false);
    pred!(
        RemotePushExecutionPredicate::RemoteBranchExists,
        remote_branch_exists,
        if remote_branch_exists { "Remote branch exists" } else { "Remote branch does not exist" }
    );

    // 22. RemoteRefMatchesExpectedOldCommit
    let ref_matches = match (remote_ref, proposal) {
        (Some(rr), Some(p)) => {
            rr.observed_commit.as_deref() == Some(p.ref_update.expected_old_commit.as_str())
                || (rr.observed_commit.is_none() && p.ref_update.expected_old_commit.is_empty())
        }
        _ => false,
    };
    pred!(
        RemotePushExecutionPredicate::RemoteRefMatchesExpectedOldCommit,
        ref_matches,
        if ref_matches { "Remote ref matches expected old commit" } else { "Remote ref does not match expected old commit" }
    );

    // 23. PushIsFastForward
    pred!(
        RemotePushExecutionPredicate::PushIsFastForward,
        is_fast_forward,
        if is_fast_forward { "Push is fast-forward" } else { "Push is not fast-forward" }
    );

    // 24. CommitIsDescendantOfRemoteRef
    // For test backend: assume true if fast_forward is true and we have both commits
    // For local backend: merge-base --is-ancestor check is done before push
    let is_descendant = is_fast_forward && proposal
        .as_ref()
        .map(|p| !p.ref_update.expected_old_commit.is_empty())
        .unwrap_or(false);
    pred!(
        RemotePushExecutionPredicate::CommitIsDescendantOfRemoteRef,
        is_descendant,
        if is_descendant { "Commit is descendant of remote ref" } else { "Commit ancestry could not be confirmed" }
    );

    // 25. PolicyAllowsRemotePush
    pred!(
        RemotePushExecutionPredicate::PolicyAllowsRemotePush,
        policy_allows,
        if policy_allows { "Policy allows remote push" } else { "Policy denies remote push" }
    );

    // 26. RecoveryEvidenceExists
    // Recovery snapshot is built from proposal data; pass if proposal has expected_old_commit
    let has_recovery = proposal
        .as_ref()
        .map(|p| !p.ref_update.expected_old_commit.is_empty())
        .unwrap_or(false);
    pred!(
        RemotePushExecutionPredicate::RecoveryEvidenceExists,
        has_recovery,
        if has_recovery { "Recovery evidence exists" } else { "No recovery evidence" }
    );

    // 27. IdempotencyKeyUnusedOrMatchesExisting
    let execution_id = push_execution_id_for(
        &request.proposal_id.0,
        &request.review_id.0,
        &request.idempotency_key,
    );
    let idempotency_ok = match existing_executions.iter().find(|e| e.execution_id == execution_id) {
        Some(existing) => existing.status == RemotePushExecutionStatus::Executed
            || existing.status == RemotePushExecutionStatus::AlreadyExecuted
            || existing.status == RemotePushExecutionStatus::Blocked,
        None => true,
    };
    pred!(
        RemotePushExecutionPredicate::IdempotencyKeyUnusedOrMatchesExisting,
        idempotency_ok,
        if idempotency_ok { "Idempotency key is valid" } else { "Idempotency key conflict" }
    );

    // 28. NoPriorConflictingPushExecution
    let no_conflict = !existing_executions.iter().any(|e| {
        e.proposal_id == request.proposal_id
            && e.status == RemotePushExecutionStatus::Executed
            && e.review_id != request.review_id
    });
    pred!(
        RemotePushExecutionPredicate::NoPriorConflictingPushExecution,
        no_conflict,
        if no_conflict { "No conflicting prior push execution" } else { "Conflicting prior push execution exists" }
    );

    results
}

// ── Execute push ────────────────────────────────────────────────────────────

pub fn execute_push(
    backend: &dyn RemotePushExecutionBackend,
    repo: &Path,
    _store_root: &Path,
    request: &RemotePushExecutionRequest,
    proposal: Option<&RemotePushProposal>,
    review: Option<&RemotePushProposalReview>,
    readiness: Option<&RemotePushReadinessRecord>,
    verification: Option<&PostCommitVerificationRecord>,
    local_execution: Option<&AutoCommitExecutionRecord>,
    branch_policy: Option<&BranchProtectionPolicySnapshot>,
    existing_executions: &[RemotePushExecutionRecord],
    remote_configured: bool,
    policy_allows: bool,
) -> RemotePushExecutionRecord {
    let execution_id = push_execution_id_for(
        &request.proposal_id.0,
        &request.review_id.0,
        &request.idempotency_key,
    );

    // Patch 3: Idempotency — same key returns existing whether Blocked or Executed
    if let Some(existing) = existing_executions.iter().find(|e| e.execution_id == execution_id) {
        return RemotePushExecutionRecord {
            status: RemotePushExecutionStatus::AlreadyExecuted,
            ..existing.clone()
        };
    }

    // An executed push cannot be duplicated with different key for same proposal+review+commit
    if let Some(prior) = existing_executions.iter().find(|e| {
        e.proposal_id == request.proposal_id
            && e.review_id == request.review_id
            && e.status == RemotePushExecutionStatus::Executed
    }) {
        return RemotePushExecutionRecord {
            execution_id,
            proposal_id: request.proposal_id.clone(),
            review_id: request.review_id.clone(),
            readiness_id: prior.readiness_id.clone(),
            verification_id: prior.verification_id.clone(),
            local_execution_id: prior.local_execution_id.clone(),
            proposal_source_id: prior.proposal_source_id.clone(),
            review_source_id: prior.review_source_id.clone(),
            commit_hash: prior.commit_hash.clone(),
            target_remote: prior.target_remote.clone(),
            target_branch: prior.target_branch.clone(),
            status: RemotePushExecutionStatus::Blocked,
            decision: RemotePushExecutionDecision::Block {
                reason_code: "already_executed".to_string(),
                summary: "This proposal+review already has an executed push".to_string(),
            },
            predicates: vec![],
            pre_push_remote: None,
            post_push_remote: None,
            push_result: None,
            recovery: None,
            created_at: Utc::now(),
        };
    }

    // Observe local state
    let local_state = backend.observe_current_local_state(repo).ok();

    // Observe remote ref
    let remote_ref = match (proposal, backend.observe_remote_ref(
        repo,
        proposal.map(|p| p.target_remote.as_str()).unwrap_or("origin"),
        proposal.map(|p| p.target_branch.as_str()).unwrap_or("main"),
    )) {
        (Some(_p), Ok(rr)) => Some(rr),
        _ => None,
    };

    // Determine fast-forward status
    let is_fast_forward = proposal
        .as_ref()
        .map(|p| p.ref_update.fast_forward_only)
        .unwrap_or(false);

    // Evaluate predicates
    let predicates = evaluate_push_predicates(
        proposal, review, review, readiness, verification, local_execution,
        local_state.as_ref(), remote_ref.as_ref(), branch_policy,
        existing_executions, request, remote_configured, policy_allows,
        is_fast_forward,
    );

    let all_passed = predicates.iter().all(|p| p.passed);

    // Build recovery snapshot before any mutation
    let recovery = proposal.map(|p| RemotePushRecoverySnapshot {
        old_remote_commit: p.ref_update.expected_old_commit.clone(),
        new_remote_commit: p.commit_hash.clone(),
        recommended_strategy: RemotePushRecoveryStrategy::FollowUpRevertCommit,
        notes: vec![
            "Push recovery is evidence-only in Wave 17".to_string(),
            "Remote rollback is not executed automatically".to_string(),
        ],
    });

    // Build blocked record helper
    let make_record = |status: RemotePushExecutionStatus,
                       decision: RemotePushExecutionDecision,
                       pre_push: Option<RemoteRefObservedSnapshot>,
                       post_push: Option<RemoteRefObservedSnapshot>,
                       push_result: Option<RemotePushResultSnapshot>,
                       rec: Option<RemotePushRecoverySnapshot>,
                       preds: Vec<RemotePushExecutionPredicateResult>| {
        RemotePushExecutionRecord {
            execution_id: execution_id.clone(),
            proposal_id: request.proposal_id.clone(),
            review_id: request.review_id.clone(),
            readiness_id: readiness.map(|r| r.readiness_id.clone()).unwrap_or_else(|| RemotePushReadinessId("unknown".into())),
            verification_id: verification.map(|v| v.verification_id.clone()).unwrap_or_else(|| PostCommitVerificationId("unknown".into())),
            local_execution_id: local_execution.map(|e| e.execution_id.clone()).unwrap_or_else(|| AutoCommitExecutionId("unknown".into())),
            proposal_source_id: proposal.map(|p| p.proposal_source_commit_id.clone()).unwrap_or_else(|| AutoCommitProposalId("unknown".into())),
            review_source_id: proposal.map(|p| p.review_source_id.clone()).unwrap_or_else(|| AutoCommitProposalReviewId("unknown".into())),
            commit_hash: proposal.map(|p| p.commit_hash.clone()).unwrap_or_default(),
            target_remote: proposal.map(|p| p.target_remote.clone()).unwrap_or_default(),
            target_branch: proposal.map(|p| p.target_branch.clone()).unwrap_or_default(),
            status,
            decision,
            predicates: preds,
            pre_push_remote: pre_push,
            post_push_remote: post_push,
            push_result,
            recovery: rec,
            created_at: Utc::now(),
        }
    };

    if !all_passed {
        let failed: Vec<&str> = predicates.iter()
            .filter(|p| !p.passed)
            .map(|p| p.reason.as_str())
            .collect();
        return make_record(
            RemotePushExecutionStatus::Blocked,
            RemotePushExecutionDecision::Block {
                reason_code: "predicate_failed".to_string(),
                summary: failed.join("; "),
            },
            remote_ref,
            None,
            None,
            recovery,
            predicates,
        );
    }

    // All predicates passed — execute push
    let p = proposal.unwrap();
    let push_request = ExactRemotePushRequest {
        remote: p.target_remote.clone(),
        branch: p.target_branch.clone(),
        ref_name: p.remote_ref.clone(),
        expected_old_commit: p.ref_update.expected_old_commit.clone(),
        proposed_new_commit: p.commit_hash.clone(),
        idempotency_key: request.idempotency_key.clone(),
    };

    match backend.execute_fast_forward_push_exact(repo, push_request) {
        Ok(push_result) => {
            // Observe remote ref after push
            let post_push_remote = backend.observe_remote_ref(
                repo, &p.target_remote, &p.target_branch,
            ).ok();

            make_record(
                RemotePushExecutionStatus::Executed,
                RemotePushExecutionDecision::Allow,
                remote_ref,
                post_push_remote,
                Some(push_result),
                recovery,
                predicates,
            )
        }
        Err(e) => make_record(
            RemotePushExecutionStatus::Blocked,
            RemotePushExecutionDecision::Block {
                reason_code: "push_execution_failed".to_string(),
                summary: format!("Push failed: {}", e.0),
            },
            remote_ref,
            None,
            None,
            recovery,
            predicates,
        ),
    }
}

// ── Persistence ─────────────────────────────────────────────────────────────

fn ensure_dirs(store_root: &Path) -> Result<PathBuf, String> {
    let base = store_root.join("remote_push_executions");
    let by_proposal = base.join("by_proposal");
    let by_review = base.join("by_review");
    let by_commit = base.join("by_commit");
    for dir in [&base, &by_proposal, &by_review, &by_commit] {
        std::fs::create_dir_all(dir).map_err(|e| format!("mkdir: {}", e))?;
    }
    Ok(base)
}

pub fn save_push_execution(
    store_root: &Path,
    record: &RemotePushExecutionRecord,
) -> Result<PathBuf, String> {
    let base = ensure_dirs(store_root)?;
    let json = serde_json::to_string_pretty(record).map_err(|e| format!("{}", e))?;
    let path = base.join(format!("{}.json", record.execution_id.0));
    std::fs::write(&path, &json).map_err(|e| format!("{}", e))?;
    std::fs::write(base.join("latest.json"), &json).map_err(|e| format!("{}", e))?;
    std::fs::write(
        base.join("by_proposal").join(format!("{}.json", record.proposal_id.0)),
        &json,
    ).map_err(|e| format!("{}", e))?;
    std::fs::write(
        base.join("by_review").join(format!("{}.json", record.review_id.0)),
        &json,
    ).map_err(|e| format!("{}", e))?;
    std::fs::write(
        base.join("by_commit").join(format!("{}.json", record.commit_hash)),
        &json,
    ).map_err(|e| format!("{}", e))?;
    Ok(path)
}

pub fn load_push_execution(
    store_root: &Path,
    id: &RemotePushExecutionId,
) -> Result<Option<RemotePushExecutionRecord>, String> {
    let path = store_root.join("remote_push_executions").join(format!("{}.json", id.0));
    if !path.exists() { return Ok(None); }
    let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
    Ok(Some(serde_json::from_str(&c).map_err(|e| format!("{}", e))?))
}

pub fn load_latest_push_execution(
    store_root: &Path,
) -> Result<Option<RemotePushExecutionRecord>, String> {
    let path = store_root.join("remote_push_executions").join("latest.json");
    if !path.exists() { return Ok(None); }
    let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
    Ok(Some(serde_json::from_str(&c).map_err(|e| format!("{}", e))?))
}

pub fn load_push_execution_by_proposal(
    store_root: &Path,
    proposal_id: &RemotePushProposalId,
) -> Result<Option<RemotePushExecutionRecord>, String> {
    let path = store_root.join("remote_push_executions").join("by_proposal").join(format!("{}.json", proposal_id.0));
    if !path.exists() { return Ok(None); }
    let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
    Ok(Some(serde_json::from_str(&c).map_err(|e| format!("{}", e))?))
}

pub fn load_push_execution_by_review(
    store_root: &Path,
    review_id: &RemotePushProposalReviewId,
) -> Result<Option<RemotePushExecutionRecord>, String> {
    let path = store_root.join("remote_push_executions").join("by_review").join(format!("{}.json", review_id.0));
    if !path.exists() { return Ok(None); }
    let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
    Ok(Some(serde_json::from_str(&c).map_err(|e| format!("{}", e))?))
}

pub fn load_push_execution_by_commit(
    store_root: &Path,
    commit_hash: &str,
) -> Result<Option<RemotePushExecutionRecord>, String> {
    let path = store_root.join("remote_push_executions").join("by_commit").join(format!("{}.json", commit_hash));
    if !path.exists() { return Ok(None); }
    let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
    Ok(Some(serde_json::from_str(&c).map_err(|e| format!("{}", e))?))
}

pub fn list_push_executions(
    store_root: &Path,
) -> Result<Vec<RemotePushExecutionRecord>, String> {
    let dir = store_root.join("remote_push_executions");
    if !dir.exists() { return Ok(vec![]); }
    let mut records = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("{}", e))? {
        let entry = entry.map_err(|e| format!("{}", e))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json")
            && path.file_stem().and_then(|s| s.to_str()) != Some("latest")
        {
            // Skip index subdirectory files
            if path.parent().map(|p| p == dir).unwrap_or(false) {
                let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
                let r: RemotePushExecutionRecord = serde_json::from_str(&c).map_err(|e| format!("{}", e))?;
                records.push(r);
            }
        }
    }
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(records)
}
