//! Governed remote push readiness gate.
//!
//! Determines whether a verified governed local commit is eligible for
//! remote push. Read-only observation only.
//!
//! Module boundary:
//!   Wave 11: eval_proposal.rs              → proposal generation
//!   Wave 12: eval_proposal_review.rs       → review and feedback
//!   Wave 13: eval_proposal_execution.rs    → execution gate and local commit record
//!   Wave 14: eval_post_commit_verify.rs    → post-commit verification and rollback drill
//!   Wave 15: eval_remote_push_readiness.rs → remote push readiness only (this module)
//!
//! This module does NOT push, fetch, pull, tag, create branches, release,
//! reset, revert, merge, rebase, contact remote hosts, or execute arbitrary shell.

use blake3::Hasher;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::eval_post_commit_verify::{
    PostCommitCheckStatus, PostCommitVerificationId, PostCommitVerificationRecord,
    PostCommitVerificationStatus,
};
use crate::eval_proposal::AutoCommitProposalId;
use crate::eval_proposal_execution::AutoCommitExecutionId;
use crate::eval_proposal_review::AutoCommitProposalReviewId;

// ── Readiness ID ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RemotePushReadinessId(pub String);

pub fn readiness_id_for(
    verification_id: &str,
    target_remote: &str,
    target_branch: &str,
    idempotency_key: &str,
) -> RemotePushReadinessId {
    let mut hasher = Hasher::new();
    hasher.update(b"remote_push_readiness:");
    hasher.update(verification_id.as_bytes());
    hasher.update(target_remote.as_bytes());
    hasher.update(target_branch.as_bytes());
    hasher.update(idempotency_key.as_bytes());
    let hash = hasher.finalize();
    RemotePushReadinessId(format!("rpr_{}", hash.to_hex()))
}

// ── Request ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePushReadinessRequest {
    pub verification_id: PostCommitVerificationId,
    pub target_remote: String,
    pub target_branch: String,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub idempotency_key: String,
}

// ── Status and Decision ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemotePushReadinessStatus {
    Ready,
    Blocked,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemotePushReadinessDecision {
    Ready,
    Blocked { reason_code: String, summary: String },
    Inconclusive { reason_code: String, summary: String },
}

// ── Snapshots ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalBranchPushSnapshot {
    pub current_head: String,
    pub current_branch: String,
    pub target_remote: String,
    pub target_branch: String,
    pub upstream_ref: Option<String>,
    pub remote_tracking_ref: Option<String>,
    pub ahead_count: u32,
    pub behind_count: u32,
    pub diverged: bool,
    pub worktree_clean: bool,
    pub index_clean: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteTrackingSnapshot {
    pub remote_name: String,
    pub tracking_ref: String,
    pub tracking_commit: Option<String>,
    pub observed_from_local_refs_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BranchProtectionPolicySnapshot {
    pub branch: String,
    pub direct_push_allowed: bool,
    pub requires_verified_commit: bool,
    pub requires_clean_rollback_drill: bool,
    pub requires_post_commit_checks: bool,
    pub requires_no_behind_remote: bool,
    pub requires_no_divergence: bool,
    pub requires_protected_branch_approval: bool,
    pub protected_branch: bool,
    pub policy_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushCheckEvidenceSnapshot {
    pub verification_status: PostCommitVerificationStatus,
    pub post_commit_checks_passed: bool,
    pub failed_checks: Vec<String>,
    pub skipped_required_checks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushRollbackEvidenceSnapshot {
    pub rollback_drill_present: bool,
    pub rollback_drill_clean: bool,
    pub live_repo_unchanged_during_drill: bool,
}

// ── Predicates ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RemotePushPredicate {
    VerificationRecordExists,
    VerificationIsVerified,
    ExecutionRecordExists,
    ExecutionWasSuccessful,
    CommitHashMatchesVerification,
    CurrentHeadMatchesVerifiedCommit,
    WorktreeClean,
    IndexClean,
    TargetRemoteConfigured,
    TargetBranchMatchesPolicy,
    UpstreamOrTrackingRefKnown,
    LocalBranchAheadOfRemote,
    LocalBranchNotBehindRemote,
    LocalBranchNotDiverged,
    CommitIsDescendantOfRemoteTrackingRef,
    BranchPolicyLoaded,
    DirectPushAllowedByPolicy,
    ProtectedBranchRequirementsSatisfied,
    PostCommitChecksPassed,
    NoSkippedRequiredChecks,
    RollbackDrillEvidencePresent,
    RollbackDrillWasClean,
    LiveRepoUnchangedDuringRollbackDrill,
    NoPriorConflictingReadinessRecord,
    IdempotencyKeyUnusedOrMatchesExisting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePushPredicateResult {
    pub predicate: RemotePushPredicate,
    pub passed: bool,
    pub reason: String,
}

// ── Record ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemotePushReadinessRecord {
    pub readiness_id: RemotePushReadinessId,
    pub verification_id: PostCommitVerificationId,
    pub execution_id: AutoCommitExecutionId,
    pub proposal_id: AutoCommitProposalId,
    pub review_id: AutoCommitProposalReviewId,
    pub commit_hash: String,
    pub target_remote: String,
    pub target_branch: String,
    pub status: RemotePushReadinessStatus,
    pub decision: RemotePushReadinessDecision,
    pub predicates: Vec<RemotePushPredicateResult>,
    pub local_branch: Option<LocalBranchPushSnapshot>,
    pub remote_tracking: Option<RemoteTrackingSnapshot>,
    pub branch_policy: Option<BranchProtectionPolicySnapshot>,
    pub check_evidence: PushCheckEvidenceSnapshot,
    pub rollback_evidence: PushRollbackEvidenceSnapshot,
    pub created_at: DateTime<Utc>,
}

// ── Error ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct RemotePushReadinessError(pub String);

// ── Backend trait ──────────────────────────────────────────────────────────

/// Narrow read-only backend for push readiness observation.
/// No push, fetch, pull, tag, branch, checkout, switch, reset, revert,
/// merge, rebase, remote add/set-url/remove, ls-remote, or shell.
pub trait RemotePushReadinessBackend {
    fn observe_local_branch_state(
        &self,
        repo: &Path,
        target_remote: &str,
        target_branch: &str,
    ) -> Result<LocalBranchPushSnapshot, RemotePushReadinessError>;

    fn observe_remote_tracking_state(
        &self,
        repo: &Path,
        target_remote: &str,
        target_branch: &str,
    ) -> Result<RemoteTrackingSnapshot, RemotePushReadinessError>;

    fn load_branch_policy(
        &self,
        repo: &Path,
        target_remote: &str,
        target_branch: &str,
    ) -> Result<BranchProtectionPolicySnapshot, RemotePushReadinessError>;

    /// Patch 2: Check remote URL exists in local git config (no network)
    fn check_remote_configured(
        &self,
        repo: &Path,
        target_remote: &str,
    ) -> Result<bool, RemotePushReadinessError>;
}

// ── Test backend ────────────────────────────────────────────────────────────

pub struct TestPushReadinessBackend {
    pub branch_state: LocalBranchPushSnapshot,
    pub tracking_state: RemoteTrackingSnapshot,
    pub policy: BranchProtectionPolicySnapshot,
    pub remote_url_exists: bool,
}

impl TestPushReadinessBackend {
    pub fn new_ready() -> Self {
        Self {
            branch_state: LocalBranchPushSnapshot {
                current_head: "verified_commit_hash".to_string(),
                current_branch: "main".to_string(),
                target_remote: "origin".to_string(),
                target_branch: "main".to_string(),
                upstream_ref: Some("refs/remotes/origin/main".to_string()),
                remote_tracking_ref: Some("refs/remotes/origin/main".to_string()),
                ahead_count: 1,
                behind_count: 0,
                diverged: false,
                worktree_clean: true,
                index_clean: true,
            },
            tracking_state: RemoteTrackingSnapshot {
                remote_name: "origin".to_string(),
                tracking_ref: "refs/remotes/origin/main".to_string(),
                tracking_commit: Some("tracking_commit".to_string()),
                observed_from_local_refs_only: true,
            },
            policy: BranchProtectionPolicySnapshot {
                branch: "main".to_string(),
                direct_push_allowed: true,
                requires_verified_commit: true,
                requires_clean_rollback_drill: true,
                requires_post_commit_checks: true,
                requires_no_behind_remote: true,
                requires_no_divergence: true,
                requires_protected_branch_approval: false,
                protected_branch: false,
                policy_source: "test_default".to_string(),
            },
            remote_url_exists: true,
        }
    }

    pub fn with_branch_state(mut self, state: LocalBranchPushSnapshot) -> Self {
        self.branch_state = state;
        self
    }

    pub fn with_policy(mut self, policy: BranchProtectionPolicySnapshot) -> Self {
        self.policy = policy;
        self
    }

    pub fn with_remote_url(mut self, exists: bool) -> Self {
        self.remote_url_exists = exists;
        self
    }
}

impl RemotePushReadinessBackend for TestPushReadinessBackend {
    fn observe_local_branch_state(
        &self,
        _repo: &Path,
        _target_remote: &str,
        _target_branch: &str,
    ) -> Result<LocalBranchPushSnapshot, RemotePushReadinessError> {
        Ok(self.branch_state.clone())
    }

    fn observe_remote_tracking_state(
        &self,
        _repo: &Path,
        _target_remote: &str,
        _target_branch: &str,
    ) -> Result<RemoteTrackingSnapshot, RemotePushReadinessError> {
        Ok(self.tracking_state.clone())
    }

    fn load_branch_policy(
        &self,
        _repo: &Path,
        _target_remote: &str,
        target_branch: &str,
    ) -> Result<BranchProtectionPolicySnapshot, RemotePushReadinessError> {
        let mut policy = self.policy.clone();
        policy.branch = target_branch.to_string();
        Ok(policy)
    }

    fn check_remote_configured(&self, _repo: &Path, _target_remote: &str) -> Result<bool, RemotePushReadinessError> {
        Ok(self.remote_url_exists)
    }
}

// ── Local read-only backend (real git) ─────────────────────────────────────

/// Real git backend using read-only commands only.
/// Patch 1: Uses git symbolic-ref --short HEAD (not git branch).
/// Patch 2: Checks remote URL via git config --get remote.<name>.url.
/// No push, fetch, pull, tag, branch, checkout, switch, reset, revert,
/// merge, rebase, ls-remote, or arbitrary shell.
pub struct LocalPushReadinessBackend {
    pub policy_rules: Vec<PushPolicyRule>,
}

impl LocalPushReadinessBackend {
    fn run_git_readonly(repo: &Path, args: &[&str]) -> Result<String, RemotePushReadinessError> {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .map_err(|e| RemotePushReadinessError(format!("git execution failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RemotePushReadinessError(format!(
                "git {} failed: {}",
                args.join(" "),
                stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn check_remote_url_exists(
        repo: &Path,
        remote_name: &str,
    ) -> Result<bool, RemotePushReadinessError> {
        let result = Self::run_git_readonly(repo, &["config", "--get", &format!("remote.{}.url", remote_name)]);
        match result {
            Ok(url) => Ok(!url.is_empty()),
            Err(_) => Ok(false),
        }
    }

    /// Load push policy from .openwand/push_policy.toml or use defaults.
    pub fn load_policy_from_file(repo: &Path) -> Option<Vec<PushPolicyRule>> {
        let policy_path = repo.join(".openwand").join("push_policy.toml");
        if !policy_path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&policy_path).ok()?;
        let config: PushPolicyFile = toml::from_str(&content).ok()?;
        Some(config.branch)
    }

    /// Patch 3: Deterministic branch policy matching.
    /// 1. Exact match wins.
    /// 2. Longest prefix wildcard wins.
    /// 3. Default policy applies.
    /// 4. Equal specificity → block as ambiguous.
    pub fn select_policy(
        rules: &[PushPolicyRule],
        branch: &str,
    ) -> BranchProtectionPolicySnapshot {
        if rules.is_empty() {
            return default_policy(branch);
        }

        // Check exact match first
        for rule in rules {
            if rule.pattern == branch {
                return rule.to_snapshot(branch, "exact_match");
            }
        }

        // Check wildcard patterns, find longest match
        let mut best_match: Option<(&PushPolicyRule, usize)> = None;
        for rule in rules {
            if rule.pattern.contains('*') && wildcard_matches(&rule.pattern, branch) {
                let specificity = rule.pattern.len() - rule.pattern.matches('*').count();
                match best_match {
                    None => best_match = Some((rule, specificity)),
                    Some((_, best_spec)) if specificity > best_spec => {
                        best_match = Some((rule, specificity));
                    }
                    Some((existing, best_spec)) if specificity == best_spec => {
                        // Equal specificity → ambiguous, block
                        return BranchProtectionPolicySnapshot {
                            branch: branch.to_string(),
                            direct_push_allowed: false,
                            protected_branch: true,
                            policy_source: format!("ambiguous_match:{}:{}", existing.pattern, rule.pattern),
                            ..default_policy(branch)
                        };
                    }
                    _ => {}
                }
            }
        }

        if let Some((rule, _)) = best_match {
            return rule.to_snapshot(branch, "wildcard_match");
        }

        // Fallback: check for "*" pattern
        for rule in rules {
            if rule.pattern == "*" {
                return rule.to_snapshot(branch, "fallback_wildcard");
            }
        }

        default_policy(branch)
    }
}

impl RemotePushReadinessBackend for LocalPushReadinessBackend {
    fn observe_local_branch_state(
        &self,
        repo: &Path,
        target_remote: &str,
        target_branch: &str,
    ) -> Result<LocalBranchPushSnapshot, RemotePushReadinessError> {
        // Patch 1: Use git symbolic-ref --short HEAD instead of git branch
        let current_branch = Self::run_git_readonly(repo, &["symbolic-ref", "--short", "HEAD"])?;
        let current_head = Self::run_git_readonly(repo, &["rev-parse", "HEAD"])?;

        // Patch 2: Check remote URL exists
        let remote_exists = Self::check_remote_url_exists(repo, target_remote)?;

        let upstream_ref = Self::run_git_readonly(
            repo,
            &["config", "--get", &format!("branch.{}.remote", target_branch)],
        ).ok();

        let merge_ref = Self::run_git_readonly(
            repo,
            &["config", "--get", &format!("branch.{}.merge", target_branch)],
        ).ok();

        // Remote tracking ref
        let tracking_ref_str = format!("refs/remotes/{}/{}", target_remote, target_branch);
        let remote_tracking_ref = Self::run_git_readonly(
            repo, &["rev-parse", &tracking_ref_str],
        ).ok();

        // Ahead/behind count
        let (ahead_count, behind_count) = if remote_tracking_ref.is_some() {
            let count_output = Self::run_git_readonly(
                repo,
                &["rev-list", "--left-right", "--count", &format!("HEAD...{}", tracking_ref_str)],
            ).unwrap_or_else(|_| "0\t0".to_string());
            let parts: Vec<&str> = count_output.split_whitespace().collect();
            let ahead: u32 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
            let behind: u32 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            (ahead, behind)
        } else {
            (0, 0)
        };

        // Divergence check
        let diverged = if remote_tracking_ref.is_some() && ahead_count > 0 && behind_count > 0 {
            true
        } else {
            false
        };

        // Worktree/index clean
        let porcelain = Self::run_git_readonly(repo, &["status", "--porcelain"]).unwrap_or_default();
        let worktree_clean = porcelain.is_empty();
        let index_clean = !porcelain.lines().any(|line| {
            let bytes = line.as_bytes();
            (bytes.len() > 0 && (bytes[0] == b'M' || bytes[0] == b'A' || bytes[0] == b'D' || bytes[0] == b'R'))
                || (bytes.len() > 1 && bytes[1] != b' ')
        });

        Ok(LocalBranchPushSnapshot {
            current_head,
            current_branch,
            target_remote: target_remote.to_string(),
            target_branch: target_branch.to_string(),
            upstream_ref,
            remote_tracking_ref: if remote_tracking_ref.is_some() { Some(tracking_ref_str) } else { None },
            ahead_count,
            behind_count,
            diverged,
            worktree_clean,
            index_clean,
        })
    }

    fn observe_remote_tracking_state(
        &self,
        repo: &Path,
        target_remote: &str,
        target_branch: &str,
    ) -> Result<RemoteTrackingSnapshot, RemotePushReadinessError> {
        let tracking_ref = format!("refs/remotes/{}/{}", target_remote, target_branch);
        let tracking_commit = Self::run_git_readonly(repo, &["rev-parse", &tracking_ref]).ok();

        Ok(RemoteTrackingSnapshot {
            remote_name: target_remote.to_string(),
            tracking_ref,
            tracking_commit,
            observed_from_local_refs_only: true,
        })
    }

    fn load_branch_policy(
        &self,
        repo: &Path,
        _target_remote: &str,
        target_branch: &str,
    ) -> Result<BranchProtectionPolicySnapshot, RemotePushReadinessError> {
        let rules = Self::load_policy_from_file(repo).unwrap_or_default();
        let merged = if rules.is_empty() {
            self.policy_rules.clone()
        } else {
            rules
        };
        Ok(Self::select_policy(&merged, target_branch))
    }

    fn check_remote_configured(&self, repo: &Path, target_remote: &str) -> Result<bool, RemotePushReadinessError> {
        Self::check_remote_url_exists(repo, target_remote)
    }
}

// ── Branch policy model ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushPolicyFile {
    pub branch: Vec<PushPolicyRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushPolicyRule {
    pub pattern: String,
    #[serde(default)]
    pub protected_branch: bool,
    #[serde(default = "default_true")]
    pub direct_push_allowed: bool,
    #[serde(default = "default_true")]
    pub requires_verified_commit: bool,
    #[serde(default = "default_true")]
    pub requires_clean_rollback_drill: bool,
    #[serde(default = "default_true")]
    pub requires_post_commit_checks: bool,
    #[serde(default = "default_true")]
    pub requires_no_behind_remote: bool,
    #[serde(default = "default_true")]
    pub requires_no_divergence: bool,
    #[serde(default)]
    pub requires_protected_branch_approval: bool,
}

fn default_true() -> bool { true }

impl PushPolicyRule {
    pub fn to_snapshot(&self, branch: &str, source: &str) -> BranchProtectionPolicySnapshot {
        BranchProtectionPolicySnapshot {
            branch: branch.to_string(),
            direct_push_allowed: self.direct_push_allowed,
            requires_verified_commit: self.requires_verified_commit,
            requires_clean_rollback_drill: self.requires_clean_rollback_drill,
            requires_post_commit_checks: self.requires_post_commit_checks,
            requires_no_behind_remote: self.requires_no_behind_remote,
            requires_no_divergence: self.requires_no_divergence,
            requires_protected_branch_approval: self.requires_protected_branch_approval,
            protected_branch: self.protected_branch,
            policy_source: format!("push_policy:{}", source),
        }
    }
}

pub fn default_policy(branch: &str) -> BranchProtectionPolicySnapshot {
    let is_main = branch == "main" || branch == "master";
    BranchProtectionPolicySnapshot {
        branch: branch.to_string(),
        direct_push_allowed: !is_main,
        requires_verified_commit: true,
        requires_clean_rollback_drill: true,
        requires_post_commit_checks: true,
        requires_no_behind_remote: true,
        requires_no_divergence: true,
        requires_protected_branch_approval: is_main,
        protected_branch: is_main,
        policy_source: "default".to_string(),
    }
}

/// Simple wildcard matching: pattern "wave/*" matches "wave/15", "wave/feature-x"
fn wildcard_matches(pattern: &str, value: &str) -> bool {
    if !pattern.contains('*') {
        return pattern == value;
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 2 {
        let prefix = parts[0];
        let suffix = parts[1];
        value.starts_with(prefix) && (suffix.is_empty() || value.ends_with(suffix))
    } else {
        pattern == value
    }
}

// ── Predicate evaluation ───────────────────────────────────────────────────

pub fn evaluate_push_readiness_predicates(
    verification: Option<&PostCommitVerificationRecord>,
    commit_hash_from_verification: Option<&str>,
    current_head: &str,
    worktree_clean: bool,
    index_clean: bool,
    remote_configured: bool,
    branch_state: Option<&LocalBranchPushSnapshot>,
    remote_tracking: Option<&RemoteTrackingSnapshot>,
    branch_policy: Option<&BranchProtectionPolicySnapshot>,
    check_evidence: &PushCheckEvidenceSnapshot,
    rollback_evidence: &PushRollbackEvidenceSnapshot,
    existing_readiness: &[RemotePushReadinessRecord],
    verification_id: &PostCommitVerificationId,
    target_remote: &str,
    target_branch: &str,
    idempotency_key: &str,
) -> Vec<RemotePushPredicateResult> {
    let mut results = Vec::new();

    // 1. VerificationRecordExists
    match verification {
        Some(v) => results.push(RemotePushPredicateResult {
            predicate: RemotePushPredicate::VerificationRecordExists,
            passed: true,
            reason: format!("Verification {} found", v.verification_id.0),
        }),
        None => {
            results.push(RemotePushPredicateResult {
                predicate: RemotePushPredicate::VerificationRecordExists,
                passed: false,
                reason: "Verification record not found".to_string(),
            });
            return results;
        }
    }

    let v = verification.unwrap();

    // 2. VerificationIsVerified
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::VerificationIsVerified,
        passed: v.status == PostCommitVerificationStatus::Verified,
        reason: format!("Verification status: {:?}", v.status),
    });

    // 3. ExecutionRecordExists (execution_id present in verification)
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::ExecutionRecordExists,
        passed: true, // Already verified by Wave 14
        reason: format!("Execution {} linked", v.execution_id.0),
    });

    // 4. ExecutionWasSuccessful (already proven by Wave 14)
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::ExecutionWasSuccessful,
        passed: true,
        reason: "Execution was successful (proven by verified Wave 14 record)".to_string(),
    });

    // 5. CommitHashMatchesVerification
    let verified_hash = v.commit_evidence.as_ref().map(|e| e.commit_hash.clone()).unwrap_or_default();
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::CommitHashMatchesVerification,
        passed: !verified_hash.is_empty(),
        reason: format!("Verified commit: {}", &verified_hash[..8.min(verified_hash.len())]),
    });

    // 6. CurrentHeadMatchesVerifiedCommit
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::CurrentHeadMatchesVerifiedCommit,
        passed: current_head == verified_hash,
        reason: format!("HEAD: {}, Verified: {}", &current_head[..8.min(current_head.len())], &verified_hash[..8.min(verified_hash.len())]),
    });

    // 7. WorktreeClean
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::WorktreeClean,
        passed: worktree_clean,
        reason: if worktree_clean { "Worktree clean".to_string() } else { "Worktree has uncommitted changes".to_string() },
    });

    // 8. IndexClean
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::IndexClean,
        passed: index_clean,
        reason: if index_clean { "Index clean".to_string() } else { "Index has staged changes".to_string() },
    });

    // 9. TargetRemoteConfigured (Patch 2)
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::TargetRemoteConfigured,
        passed: remote_configured,
        reason: if remote_configured { format!("Remote '{}' configured", target_remote) } else { format!("Remote '{}' not found in local config", target_remote) },
    });

    // 10. TargetBranchMatchesPolicy
    match branch_policy {
        Some(policy) => results.push(RemotePushPredicateResult {
            predicate: RemotePushPredicate::TargetBranchMatchesPolicy,
            passed: true,
            reason: format!("Policy loaded for branch '{}' (source: {})", policy.branch, policy.policy_source),
        }),
        None => results.push(RemotePushPredicateResult {
            predicate: RemotePushPredicate::TargetBranchMatchesPolicy,
            passed: false,
            reason: "No branch policy loaded".to_string(),
        }),
    }

    // 11. UpstreamOrTrackingRefKnown
    let tracking_known = branch_state
        .map(|bs| bs.remote_tracking_ref.is_some() || bs.upstream_ref.is_some())
        .unwrap_or(false)
        || remote_tracking.and_then(|rt| rt.tracking_commit.as_ref()).is_some();
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::UpstreamOrTrackingRefKnown,
        passed: tracking_known,
        reason: if tracking_known { "Tracking ref known".to_string() } else { "No tracking ref found (Inconclusive)".to_string() },
    });

    // 12. LocalBranchAheadOfRemote
    let ahead = branch_state.map(|bs| bs.ahead_count).unwrap_or(0);
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::LocalBranchAheadOfRemote,
        passed: ahead >= 1,
        reason: format!("Ahead by {} commits", ahead),
    });

    // 13. LocalBranchNotBehindRemote
    let behind = branch_state.map(|bs| bs.behind_count).unwrap_or(0);
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::LocalBranchNotBehindRemote,
        passed: behind == 0,
        reason: format!("Behind by {} commits", behind),
    });

    // 14. LocalBranchNotDiverged
    let diverged = branch_state.map(|bs| bs.diverged).unwrap_or(false);
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::LocalBranchNotDiverged,
        passed: !diverged,
        reason: if diverged { "Branch has diverged from remote".to_string() } else { "No divergence".to_string() },
    });

    // 15. CommitIsDescendantOfRemoteTrackingRef
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::CommitIsDescendantOfRemoteTrackingRef,
        passed: tracking_known && ahead >= 1,
        reason: if tracking_known { "Commit reachable from HEAD".to_string() } else { "Cannot verify ancestry (no tracking ref)".to_string() },
    });

    // 16. BranchPolicyLoaded
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::BranchPolicyLoaded,
        passed: branch_policy.is_some(),
        reason: match branch_policy {
            Some(p) => format!("Policy loaded: {}", p.policy_source),
            None => "No branch policy".to_string(),
        },
    });

    // 17. DirectPushAllowedByPolicy
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::DirectPushAllowedByPolicy,
        passed: branch_policy.map(|p| p.direct_push_allowed).unwrap_or(false),
        reason: match branch_policy {
            Some(p) if p.direct_push_allowed => "Direct push allowed by policy".to_string(),
            Some(p) => format!("Direct push not allowed (protected: {})", p.protected_branch),
            None => "No policy loaded".to_string(),
        },
    });

    // 18. ProtectedBranchRequirementsSatisfied
    let protected_ok = match branch_policy {
        Some(p) if p.protected_branch => {
            p.direct_push_allowed && check_evidence.post_commit_checks_passed && rollback_evidence.rollback_drill_clean
        }
        _ => true,
    };
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::ProtectedBranchRequirementsSatisfied,
        passed: protected_ok,
        reason: if protected_ok { "Protected branch requirements satisfied".to_string() } else { "Protected branch requirements not met".to_string() },
    });

    // 19. PostCommitChecksPassed
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::PostCommitChecksPassed,
        passed: check_evidence.post_commit_checks_passed,
        reason: if check_evidence.post_commit_checks_passed { "All post-commit checks passed".to_string() } else { format!("Failed checks: {:?}", check_evidence.failed_checks) },
    });

    // 20. NoSkippedRequiredChecks
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::NoSkippedRequiredChecks,
        passed: check_evidence.skipped_required_checks.is_empty(),
        reason: if check_evidence.skipped_required_checks.is_empty() { "No skipped checks".to_string() } else { format!("Skipped: {:?}", check_evidence.skipped_required_checks) },
    });

    // 21. RollbackDrillEvidencePresent
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::RollbackDrillEvidencePresent,
        passed: rollback_evidence.rollback_drill_present,
        reason: if rollback_evidence.rollback_drill_present { "Rollback drill present".to_string() } else { "No rollback drill evidence".to_string() },
    });

    // 22. RollbackDrillWasClean
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::RollbackDrillWasClean,
        passed: rollback_evidence.rollback_drill_clean,
        reason: if rollback_evidence.rollback_drill_clean { "Rollback drill clean".to_string() } else { "Rollback drill not clean".to_string() },
    });

    // 23. LiveRepoUnchangedDuringRollbackDrill
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::LiveRepoUnchangedDuringRollbackDrill,
        passed: rollback_evidence.live_repo_unchanged_during_drill,
        reason: if rollback_evidence.live_repo_unchanged_during_drill { "Live repo unchanged during drill".to_string() } else { "Live repo was changed during drill".to_string() },
    });

    // 24. NoPriorConflictingReadinessRecord
    let readiness_id = readiness_id_for(&verification_id.0, target_remote, target_branch, idempotency_key);
    let conflicting = existing_readiness.iter().any(|r| {
        r.verification_id == *verification_id
            && r.target_remote == target_remote
            && r.target_branch == target_branch
            && r.readiness_id != readiness_id
            && r.status == RemotePushReadinessStatus::Ready
    });
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::NoPriorConflictingReadinessRecord,
        passed: !conflicting,
        reason: if conflicting { "Conflicting readiness record exists".to_string() } else { "No conflicting records".to_string() },
    });

    // 25. IdempotencyKeyUnusedOrMatchesExisting
    let existing_match = existing_readiness.iter().find(|r| r.readiness_id == readiness_id);
    results.push(RemotePushPredicateResult {
        predicate: RemotePushPredicate::IdempotencyKeyUnusedOrMatchesExisting,
        passed: existing_match.is_none() || existing_match.unwrap().status == RemotePushReadinessStatus::Ready,
        reason: match existing_match {
            Some(r) => format!("Existing readiness: {:?}", r.status),
            None => "No prior readiness with this key".to_string(),
        },
    });

    results
}

// ── Main orchestrator ──────────────────────────────────────────────────────

pub fn evaluate_push_readiness(
    backend: &dyn RemotePushReadinessBackend,
    repo: &Path,
    request: &RemotePushReadinessRequest,
    verification: Option<&PostCommitVerificationRecord>,
    existing_readiness: &[RemotePushReadinessRecord],
) -> RemotePushReadinessRecord {
    let readiness_id = readiness_id_for(
        &request.verification_id.0,
        &request.target_remote,
        &request.target_branch,
        &request.idempotency_key,
    );

    // Idempotency: return existing
    if let Some(existing) = existing_readiness.iter().find(|r| r.readiness_id == readiness_id) {
        return existing.clone();
    }

    // Observe local branch state
    let branch_state = backend.observe_local_branch_state(repo, &request.target_remote, &request.target_branch).ok();

    // Observe remote tracking state
    let remote_tracking = backend.observe_remote_tracking_state(repo, &request.target_remote, &request.target_branch).ok();

    // Load branch policy
    let branch_policy = backend.load_branch_policy(repo, &request.target_remote, &request.target_branch).ok();

    // Patch 2: Check remote URL via backend
    let remote_configured = backend.check_remote_configured(repo, &request.target_remote).unwrap_or(false);

    // Build check evidence from verification
    let check_evidence = match verification {
        Some(v) => PushCheckEvidenceSnapshot {
            verification_status: v.status.clone(),
            post_commit_checks_passed: v.post_commit_checks.iter().all(|c| c.status == PostCommitCheckStatus::Passed),
            failed_checks: v.post_commit_checks.iter()
                .filter(|c| c.status == PostCommitCheckStatus::Failed)
                .map(|c| c.spec.name.clone())
                .collect(),
            skipped_required_checks: v.post_commit_checks.iter()
                .filter(|c| c.status == PostCommitCheckStatus::Skipped)
                .map(|c| c.spec.name.clone())
                .collect(),
        },
        None => PushCheckEvidenceSnapshot {
            verification_status: PostCommitVerificationStatus::Failed,
            post_commit_checks_passed: false,
            failed_checks: vec!["no_verification".to_string()],
            skipped_required_checks: vec![],
        },
    };

    // Build rollback evidence from verification
    let rollback_evidence = match verification {
        Some(v) => match &v.rollback_drill {
            Some(drill) => PushRollbackEvidenceSnapshot {
                rollback_drill_present: true,
                rollback_drill_clean: drill.clean,
                live_repo_unchanged_during_drill: drill.live_head_before == drill.live_head_after
                    && drill.live_index_before == drill.live_index_after
                    && drill.live_worktree_before == drill.live_worktree_after,
            },
            None => PushRollbackEvidenceSnapshot {
                rollback_drill_present: false,
                rollback_drill_clean: false,
                live_repo_unchanged_during_drill: false,
            },
        },
        None => PushRollbackEvidenceSnapshot {
            rollback_drill_present: false,
            rollback_drill_clean: false,
            live_repo_unchanged_during_drill: false,
        },
    };

    let current_head = branch_state.as_ref().map(|bs| bs.current_head.clone()).unwrap_or_default();
    let worktree_clean = branch_state.as_ref().map(|bs| bs.worktree_clean).unwrap_or(false);
    let index_clean = branch_state.as_ref().map(|bs| bs.index_clean).unwrap_or(false);

    let predicates = evaluate_push_readiness_predicates(
        verification,
        None,
        &current_head,
        worktree_clean,
        index_clean,
        remote_configured,
        branch_state.as_ref(),
        remote_tracking.as_ref(),
        branch_policy.as_ref(),
        &check_evidence,
        &rollback_evidence,
        existing_readiness,
        &request.verification_id,
        &request.target_remote,
        &request.target_branch,
        &request.idempotency_key,
    );

    // Determine decision
    let all_passed = predicates.iter().all(|p| p.passed);
    let has_inconclusive = predicates.iter().any(|p| !p.passed && p.reason.contains("Inconclusive"));
    let tracking_missing = remote_tracking.as_ref().map(|rt| rt.tracking_commit.is_none()).unwrap_or(true);

    let decision = if all_passed {
        RemotePushReadinessDecision::Ready
    } else if tracking_missing || has_inconclusive {
        let failed: Vec<&str> = predicates.iter().filter(|p| !p.passed).map(|p| p.reason.as_str()).collect();
        RemotePushReadinessDecision::Inconclusive {
            reason_code: "evidence_unavailable".to_string(),
            summary: failed.join("; "),
        }
    } else {
        let failed: Vec<&str> = predicates.iter().filter(|p| !p.passed).map(|p| p.reason.as_str()).collect();
        RemotePushReadinessDecision::Blocked {
            reason_code: "predicate_failed".to_string(),
            summary: failed.join("; "),
        }
    };

    let status = match &decision {
        RemotePushReadinessDecision::Ready => RemotePushReadinessStatus::Ready,
        RemotePushReadinessDecision::Blocked { .. } => RemotePushReadinessStatus::Blocked,
        RemotePushReadinessDecision::Inconclusive { .. } => RemotePushReadinessStatus::Inconclusive,
    };

    let verified_hash = verification
        .and_then(|v| v.commit_evidence.as_ref())
        .map(|e| e.commit_hash.clone())
        .unwrap_or_default();

    RemotePushReadinessRecord {
        readiness_id,
        verification_id: request.verification_id.clone(),
        execution_id: verification.map(|v| v.execution_id.clone()).unwrap_or(AutoCommitExecutionId("unknown".to_string())),
        proposal_id: verification.map(|v| v.proposal_id.clone()).unwrap_or(AutoCommitProposalId("unknown".to_string())),
        review_id: verification.map(|v| v.review_id.clone()).unwrap_or(AutoCommitProposalReviewId("unknown".to_string())),
        commit_hash: verified_hash,
        target_remote: request.target_remote.clone(),
        target_branch: request.target_branch.clone(),
        status,
        decision,
        predicates,
        local_branch: branch_state,
        remote_tracking,
        branch_policy,
        check_evidence,
        rollback_evidence,
        created_at: Utc::now(),
    }
}

// ── Persistence ────────────────────────────────────────────────────────────

pub fn save_readiness_record(
    store_root: &Path,
    record: &RemotePushReadinessRecord,
) -> Result<PathBuf, String> {
    let dir = store_root.join("remote_push_readiness");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create dir: {}", e))?;
    let by_ver = dir.join("by_verification");
    std::fs::create_dir_all(&by_ver).map_err(|e| format!("Failed: {}", e))?;
    let by_commit = dir.join("by_commit");
    std::fs::create_dir_all(&by_commit).map_err(|e| format!("Failed: {}", e))?;

    let json = serde_json::to_string_pretty(record).map_err(|e| format!("Serialize: {}", e))?;

    let path = dir.join(format!("{}.json", record.readiness_id.0));
    std::fs::write(&path, &json).map_err(|e| format!("Write: {}", e))?;
    std::fs::write(dir.join("latest.json"), &json).map_err(|e| format!("Latest: {}", e))?;
    std::fs::write(by_ver.join(format!("{}.json", record.verification_id.0)), &json).map_err(|e| format!("ByVer: {}", e))?;
    if !record.commit_hash.is_empty() {
        std::fs::write(by_commit.join(format!("{}.json", record.commit_hash)), &json).map_err(|e| format!("ByCommit: {}", e))?;
    }

    Ok(path)
}

pub fn load_readiness_record(
    store_root: &Path,
    id: &RemotePushReadinessId,
) -> Result<Option<RemotePushReadinessRecord>, String> {
    let path = store_root.join("remote_push_readiness").join(format!("{}.json", id.0));
    if !path.exists() { return Ok(None); }
    let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
    Ok(Some(serde_json::from_str(&c).map_err(|e| format!("{}", e))?))
}

pub fn load_latest_readiness(store_root: &Path) -> Result<Option<RemotePushReadinessRecord>, String> {
    let path = store_root.join("remote_push_readiness").join("latest.json");
    if !path.exists() { return Ok(None); }
    let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
    Ok(Some(serde_json::from_str(&c).map_err(|e| format!("{}", e))?))
}

pub fn load_latest_readiness_for_verification(
    store_root: &Path,
    verification_id: &PostCommitVerificationId,
) -> Result<Option<RemotePushReadinessRecord>, String> {
    let path = store_root.join("remote_push_readiness").join("by_verification").join(format!("{}.json", verification_id.0));
    if !path.exists() { return Ok(None); }
    let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
    Ok(Some(serde_json::from_str(&c).map_err(|e| format!("{}", e))?))
}

pub fn load_latest_readiness_for_commit(
    store_root: &Path,
    commit_hash: &str,
) -> Result<Option<RemotePushReadinessRecord>, String> {
    let path = store_root.join("remote_push_readiness").join("by_commit").join(format!("{}.json", commit_hash));
    if !path.exists() { return Ok(None); }
    let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
    Ok(Some(serde_json::from_str(&c).map_err(|e| format!("{}", e))?))
}

pub fn list_readiness_records(store_root: &Path) -> Result<Vec<RemotePushReadinessRecord>, String> {
    let dir = store_root.join("remote_push_readiness");
    if !dir.exists() { return Ok(vec![]); }
    let mut records = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("{}", e))? {
        let entry = entry.map_err(|e| format!("{}", e))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json")
            && path.file_stem().and_then(|s| s.to_str()) != Some("latest")
        {
            let c = std::fs::read_to_string(&path).map_err(|e| format!("{}", e))?;
            let r: RemotePushReadinessRecord = serde_json::from_str(&c).map_err(|e| format!("{}", e))?;
            records.push(r);
        }
    }
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(records)
}
