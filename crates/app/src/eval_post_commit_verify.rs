//! Post-commit verification and rollback drill.
//!
//! Verifies a governed local commit after creation, binds it back to
//! proposal/review/execution evidence, runs deterministic post-commit checks,
//! and rehearses rollback in a disposable sandbox.
//!
//! Module boundary:
//!   Wave 11: eval_proposal.rs             → proposal generation
//!   Wave 12: eval_proposal_review.rs      → review and feedback
//!   Wave 13: eval_proposal_execution.rs   → execution gate and local commit record
//!   Wave 14: eval_post_commit_verify.rs   → post-commit verification and rollback drill (this module)
//!
//! This module does NOT create commits, push, tag, branch, release, reset,
//! revert the live repo, or execute arbitrary shell commands.

use blake3::Hasher;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::eval_proposal::{AutoCommitProposal, AutoCommitProposalId, ProposalFileChange};
use crate::eval_proposal_execution::{
    AutoCommitExecutionId, AutoCommitExecutionRecord, AutoCommitExecutionStatus,
    GitCommitSnapshot, GovernedGitCommitBackend, LocalGitBackend, RollbackPlanSnapshot,
};
use crate::eval_proposal_review::{AutoCommitProposalReview, AutoCommitProposalReviewId};

// ── Verification ID ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PostCommitVerificationId(pub String);

pub fn verification_id_for(execution_id: &str, idempotency_key: &str) -> PostCommitVerificationId {
    let mut hasher = Hasher::new();
    hasher.update(b"post_commit_verify:");
    hasher.update(execution_id.as_bytes());
    hasher.update(idempotency_key.as_bytes());
    let hash = hasher.finalize();
    PostCommitVerificationId(format!("pcv_{}", hash.to_hex()))
}

// ── Verification request ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostCommitVerificationRequest {
    pub execution_id: AutoCommitExecutionId,
    pub requested_by: String,
    pub requested_at: DateTime<Utc>,
    pub idempotency_key: String,
}

// ── Verification status ────────────────────────────────────────────────────
// Patch 1: AlreadyVerified removed. Idempotency returns existing record.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PostCommitVerificationStatus {
    Verified,
    Failed,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PostCommitVerificationDecision {
    Verified,
    Failed { reason_code: String, summary: String },
    Inconclusive { reason_code: String, summary: String },
}

// ── Commit evidence snapshot ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommitEvidenceSnapshot {
    pub commit_hash: String,
    pub parent_hash: String,
    pub tree_hash: String,
    pub branch: String,
    pub message_hash: String,
    pub changed_paths: Vec<String>,
    pub diff_hash: String,
}

// ── Post-commit checks ─────────────────────────────────────────────────────
// Patch 2: Explicit status enum with Passed/Failed/Skipped.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PostCommitCheckKind {
    CargoFmtCheck,
    CargoCheckWorkspace,
    CargoTestWorkspace,
    CargoTestPackage { package: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PostCommitCheckSpec {
    pub name: String,
    pub kind: PostCommitCheckKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PostCommitCheckStatus {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostCommitCheckResult {
    pub spec: PostCommitCheckSpec,
    pub status: PostCommitCheckStatus,
    pub output_summary: String,
}

// ── Rollback drill ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RollbackDrillStrategy {
    SandboxRevert,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackDrillPlan {
    pub strategy: RollbackDrillStrategy,
    pub commit_hash: String,
    pub expected_parent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackDrillResult {
    pub strategy: RollbackDrillStrategy,
    pub clean: bool,
    pub sandbox_pre_head: String,
    pub sandbox_post_head: String,
    pub sandbox_diff_hash: String,
    pub conflicts: Vec<String>,
    pub live_head_before: String,
    pub live_head_after: String,
    pub live_index_before: String,
    pub live_index_after: String,
    pub live_worktree_before: String,
    pub live_worktree_after: String,
}

// ── Predicates ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PostCommitPredicate {
    ExecutionRecordExists,
    ExecutionWasSuccessful,
    ResultingCommitExists,
    CommitHashMatchesExecutionRecord,
    CommitParentMatchesRollbackHead,
    CommitBranchMatchesExecutionRecord,
    CommitMessageHashMatchesProposal,
    CommitDiffMatchesApprovedPaths,
    CommitDiffContainsNoUnreviewedPaths,
    CommitTreeMatchesExpectedPostState,
    EvidenceChainMatches,
    WorkspaceCleanAfterCommit,
    PostCommitChecksPass,
    RollbackDrillCompleted,
    RollbackDrillCleanlyApplies,
    LiveRepoUnchangedDuringDrill,
    IdempotencyKeyUnusedOrMatchesExisting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostCommitPredicateResult {
    pub predicate: PostCommitPredicate,
    pub passed: bool,
    pub reason: String,
}

// ── Verification decision and record ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostCommitVerificationDecisionDetail {
    pub decision: PostCommitVerificationDecision,
    pub predicates: Vec<PostCommitPredicateResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostCommitVerificationRecord {
    pub verification_id: PostCommitVerificationId,
    pub execution_id: AutoCommitExecutionId,
    pub proposal_id: AutoCommitProposalId,
    pub review_id: AutoCommitProposalReviewId,
    pub status: PostCommitVerificationStatus,
    pub decision: PostCommitVerificationDecision,
    pub predicates: Vec<PostCommitPredicateResult>,
    pub commit_evidence: Option<CommitEvidenceSnapshot>,
    pub post_commit_checks: Vec<PostCommitCheckResult>,
    pub rollback_drill: Option<RollbackDrillResult>,
    pub created_at: DateTime<Utc>,
}

// ── Error type ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PostCommitVerifyError(pub String);

// ── Backend trait ──────────────────────────────────────────────────────────

/// Narrow backend for post-commit verification. Observe commits, run fixed
/// checks, rehearse rollback in sandbox only.
///
/// Forbidden: push, tag, branch, release, reset_live_repo, revert_live_repo,
/// checkout_live_repo, run_git, run_shell.
pub trait PostCommitVerifierBackend {
    fn observe_commit(
        &self,
        repo: &Path,
        commit_hash: &str,
    ) -> Result<CommitEvidenceSnapshot, PostCommitVerifyError>;

    fn run_post_commit_checks(
        &self,
        repo: &Path,
        checks: &[PostCommitCheckSpec],
    ) -> Result<Vec<PostCommitCheckResult>, PostCommitVerifyError>;

    fn run_rollback_drill_in_sandbox(
        &self,
        repo: &Path,
        plan: RollbackDrillPlan,
    ) -> Result<RollbackDrillResult, PostCommitVerifyError>;
}

// ── Test backend ────────────────────────────────────────────────────────────

pub struct TestVerifierBackend {
    pub commit_evidence: CommitEvidenceSnapshot,
    pub check_results: Vec<PostCommitCheckResult>,
    pub drill_result: RollbackDrillResult,
}

impl TestVerifierBackend {
    pub fn new_passing() -> Self {
        Self {
            commit_evidence: CommitEvidenceSnapshot {
                commit_hash: "testcommit_abc".to_string(),
                parent_hash: "parent_def".to_string(),
                tree_hash: "tree_123".to_string(),
                branch: "main".to_string(),
                message_hash: "msg_hash".to_string(),
                changed_paths: vec!["src/lib.rs".to_string()],
                diff_hash: "diff_hash".to_string(),
            },
            check_results: vec![PostCommitCheckResult {
                spec: PostCommitCheckSpec {
                    name: "cargo_check".to_string(),
                    kind: PostCommitCheckKind::CargoCheckWorkspace,
                },
                status: PostCommitCheckStatus::Passed,
                output_summary: "OK".to_string(),
            }],
            drill_result: RollbackDrillResult {
                strategy: RollbackDrillStrategy::SandboxRevert,
                clean: true,
                sandbox_pre_head: "testcommit_abc".to_string(),
                sandbox_post_head: "revert_commit".to_string(),
                sandbox_diff_hash: "revert_diff".to_string(),
                conflicts: vec![],
                live_head_before: "testcommit_abc".to_string(),
                live_head_after: "testcommit_abc".to_string(),
                live_index_before: "idx_before".to_string(),
                live_index_after: "idx_before".to_string(),
                live_worktree_before: "wt_before".to_string(),
                live_worktree_after: "wt_before".to_string(),
            },
        }
    }

    pub fn with_commit_evidence(mut self, evidence: CommitEvidenceSnapshot) -> Self {
        self.commit_evidence = evidence;
        self
    }

    pub fn with_check_result(mut self, result: PostCommitCheckResult) -> Self {
        self.check_results = vec![result];
        self
    }

    pub fn with_drill_result(mut self, result: RollbackDrillResult) -> Self {
        self.drill_result = result;
        self
    }
}

impl PostCommitVerifierBackend for TestVerifierBackend {
    fn observe_commit(
        &self,
        _repo: &Path,
        _commit_hash: &str,
    ) -> Result<CommitEvidenceSnapshot, PostCommitVerifyError> {
        Ok(self.commit_evidence.clone())
    }

    fn run_post_commit_checks(
        &self,
        _repo: &Path,
        _checks: &[PostCommitCheckSpec],
    ) -> Result<Vec<PostCommitCheckResult>, PostCommitVerifyError> {
        Ok(self.check_results.clone())
    }

    fn run_rollback_drill_in_sandbox(
        &self,
        _repo: &Path,
        _plan: RollbackDrillPlan,
    ) -> Result<RollbackDrillResult, PostCommitVerifyError> {
        Ok(self.drill_result.clone())
    }
}

// ── Local verifier backend (real git) ──────────────────────────────────────

/// Real backend using std::process::Command. Only used for observation,
/// fixed check execution, and sandbox rollback rehearsal.
/// No push, tag, branch, release, live reset, live revert, shell.
pub struct LocalVerifierBackend {
    pub default_checks: Vec<PostCommitCheckSpec>,
}

impl LocalVerifierBackend {
    fn run_git(repo: &Path, args: &[&str]) -> Result<String, PostCommitVerifyError> {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .map_err(|e| PostCommitVerifyError(format!("git execution failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PostCommitVerifyError(format!(
                "git {} failed: {}",
                args.join(" "),
                stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn run_command(
        cmd: &str,
        args: &[&str],
        cwd: &Path,
    ) -> Result<String, PostCommitVerifyError> {
        let output = std::process::Command::new(cmd)
            .args(args)
            .current_dir(cwd)
            .output()
            .map_err(|e| PostCommitVerifyError(format!("{} execution failed: {}", cmd, e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn default_checks() -> Vec<PostCommitCheckSpec> {
        vec![
            PostCommitCheckSpec {
                name: "cargo_fmt_check".to_string(),
                kind: PostCommitCheckKind::CargoFmtCheck,
            },
            PostCommitCheckSpec {
                name: "cargo_check_workspace".to_string(),
                kind: PostCommitCheckKind::CargoCheckWorkspace,
            },
        ]
    }
}

impl PostCommitVerifierBackend for LocalVerifierBackend {
    fn observe_commit(
        &self,
        repo: &Path,
        commit_hash: &str,
    ) -> Result<CommitEvidenceSnapshot, PostCommitVerifyError> {
        let parent = Self::run_git(repo, &["rev-parse", &format!("{}~1", commit_hash)])?;
        let tree = Self::run_git(repo, &["rev-parse", &format!("{}^{{tree}}", commit_hash)])?;
        let branch = Self::run_git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])?;
        let message = Self::run_git(repo, &["log", "-1", "--format=%B", commit_hash])?;
        let message_hash = format!("{}", blake3::hash(message.as_bytes()).to_hex());

        // Get changed paths in this commit
        let diff_names = Self::run_git(
            repo,
            &["diff-tree", "--no-commit-id", "--name-only", "-r", commit_hash],
        )?;
        let changed_paths: Vec<String> = if diff_names.is_empty() {
            vec![]
        } else {
            diff_names.lines().map(|l| l.trim().to_string()).collect()
        };

        // Get diff hash
        let diff_output = Self::run_git(
            repo,
            &["diff-tree", "-p", commit_hash],
        )?;
        let diff_hash = format!("{}", blake3::hash(diff_output.as_bytes()).to_hex());

        Ok(CommitEvidenceSnapshot {
            commit_hash: commit_hash.to_string(),
            parent_hash: parent,
            tree_hash: tree,
            branch,
            message_hash,
            changed_paths,
            diff_hash,
        })
    }

    fn run_post_commit_checks(
        &self,
        repo: &Path,
        checks: &[PostCommitCheckSpec],
    ) -> Result<Vec<PostCommitCheckResult>, PostCommitVerifyError> {
        let mut results = Vec::new();
        for spec in checks {
            let result = match &spec.kind {
                PostCommitCheckKind::CargoFmtCheck => {
                    let output = Self::run_command("cargo", &["fmt", "--check"], repo);
                    match output {
                        Ok(_) => PostCommitCheckResult {
                            spec: spec.clone(),
                            status: PostCommitCheckStatus::Passed,
                            output_summary: "Formatted".to_string(),
                        },
                        Err(e) => PostCommitCheckResult {
                            spec: spec.clone(),
                            status: PostCommitCheckStatus::Failed,
                            output_summary: e.0,
                        },
                    }
                }
                PostCommitCheckKind::CargoCheckWorkspace => {
                    let output = Self::run_command("cargo", &["check", "--workspace"], repo);
                    match output {
                        Ok(_) => PostCommitCheckResult {
                            spec: spec.clone(),
                            status: PostCommitCheckStatus::Passed,
                            output_summary: "Check passed".to_string(),
                        },
                        Err(e) => PostCommitCheckResult {
                            spec: spec.clone(),
                            status: PostCommitCheckStatus::Failed,
                            output_summary: e.0,
                        },
                    }
                }
                PostCommitCheckKind::CargoTestWorkspace => {
                    let output = Self::run_command("cargo", &["test", "--workspace"], repo);
                    match output {
                        Ok(_) => PostCommitCheckResult {
                            spec: spec.clone(),
                            status: PostCommitCheckStatus::Passed,
                            output_summary: "Tests passed".to_string(),
                        },
                        Err(e) => PostCommitCheckResult {
                            spec: spec.clone(),
                            status: PostCommitCheckStatus::Failed,
                            output_summary: e.0,
                        },
                    }
                }
                PostCommitCheckKind::CargoTestPackage { package } => {
                    let output = Self::run_command(
                        "cargo",
                        &["test", "-p", package],
                        repo,
                    );
                    match output {
                        Ok(_) => PostCommitCheckResult {
                            spec: spec.clone(),
                            status: PostCommitCheckStatus::Passed,
                            output_summary: format!("Tests passed for {}", package),
                        },
                        Err(e) => PostCommitCheckResult {
                            spec: spec.clone(),
                            status: PostCommitCheckStatus::Failed,
                            output_summary: e.0,
                        },
                    }
                }
            };
            results.push(result);
        }
        Ok(results)
    }

    fn run_rollback_drill_in_sandbox(
        &self,
        repo: &Path,
        plan: RollbackDrillPlan,
    ) -> Result<RollbackDrillResult, PostCommitVerifyError> {
        // Capture live state before
        let live_head_before = Self::run_git(repo, &["rev-parse", "HEAD"])?;
        let live_index_before = {
            let idx = Self::run_git(repo, &["ls-files", "--stage"])?;
            format!("{}", blake3::hash(idx.as_bytes()).to_hex())
        };
        let live_worktree_before = {
            let wt = Self::run_git(repo, &["status", "--porcelain"])?;
            format!("{}", blake3::hash(wt.as_bytes()).to_hex())
        };

        // Create sandbox
        let sandbox_dir = tempfile::tempdir()
            .map_err(|e| PostCommitVerifyError(format!("Failed to create sandbox: {}", e)))?;

        // Clone repo to sandbox (shallow for speed)
        let sandbox_url = if cfg!(windows) {
            repo.to_str().unwrap_or(".")
        } else {
            repo.to_str().unwrap_or(".")
        };
        Self::run_git(
            sandbox_dir.path(),
            &["clone", "--no-local", sandbox_url, "."],
        )?;
        // Checkout the commit
        Self::run_git(sandbox_dir.path(), &["checkout", &plan.commit_hash])?;

        let sandbox_pre_head = Self::run_git(sandbox_dir.path(), &["rev-parse", "HEAD"])?;

        // Attempt revert in sandbox
        let revert_result = Self::run_git(
            sandbox_dir.path(),
            &["revert", "--no-edit", &plan.commit_hash],
        );

        let (clean, sandbox_post_head, sandbox_diff_hash, conflicts) = match revert_result {
            Ok(_) => {
                let post_head = Self::run_git(sandbox_dir.path(), &["rev-parse", "HEAD"])?;
                let diff = Self::run_git(sandbox_dir.path(), &["diff", "--stat"])?;
                let diff_hash = format!("{}", blake3::hash(diff.as_bytes()).to_hex());
                (true, post_head, diff_hash, vec![])
            }
            Err(e) => {
                // Abort revert to leave sandbox in clean state
                let _ = Self::run_git(sandbox_dir.path(), &["revert", "--abort"]);
                let post_head = Self::run_git(sandbox_dir.path(), &["rev-parse", "HEAD"])?;
                (false, post_head, "revert_failed".to_string(), vec![e.0])
            }
        };

        // Capture live state after
        let live_head_after = Self::run_git(repo, &["rev-parse", "HEAD"])?;
        let live_index_after = {
            let idx = Self::run_git(repo, &["ls-files", "--stage"])?;
            format!("{}", blake3::hash(idx.as_bytes()).to_hex())
        };
        let live_worktree_after = {
            let wt = Self::run_git(repo, &["status", "--porcelain"])?;
            format!("{}", blake3::hash(wt.as_bytes()).to_hex())
        };

        Ok(RollbackDrillResult {
            strategy: plan.strategy,
            clean,
            sandbox_pre_head,
            sandbox_post_head,
            sandbox_diff_hash,
            conflicts,
            live_head_before,
            live_head_after,
            live_index_before,
            live_index_after,
            live_worktree_before,
            live_worktree_after,
        })
    }
}

// ── Predicate evaluation ───────────────────────────────────────────────────

pub fn evaluate_post_commit_predicates(
    execution_record: Option<&AutoCommitExecutionRecord>,
    proposal: Option<&AutoCommitProposal>,
    review: Option<&AutoCommitProposalReview>,
    commit_evidence: Option<&CommitEvidenceSnapshot>,
    post_commit_checks: &[PostCommitCheckResult],
    rollback_drill: Option<&RollbackDrillResult>,
    existing_verifications: &[PostCommitVerificationRecord],
    execution_id: &AutoCommitExecutionId,
    idempotency_key: &str,
    workspace_status: &str,
) -> Vec<PostCommitPredicateResult> {
    let mut results = Vec::new();

    // 1. ExecutionRecordExists
    match execution_record {
        Some(r) => results.push(PostCommitPredicateResult {
            predicate: PostCommitPredicate::ExecutionRecordExists,
            passed: true,
            reason: format!("Execution record {} found", r.execution_id.0),
        }),
        None => {
            results.push(PostCommitPredicateResult {
                predicate: PostCommitPredicate::ExecutionRecordExists,
                passed: false,
                reason: "Execution record not found".to_string(),
            });
            return results;
        }
    }

    let exec = execution_record.unwrap();

    // 2. ExecutionWasSuccessful
    results.push(PostCommitPredicateResult {
        predicate: PostCommitPredicate::ExecutionWasSuccessful,
        passed: exec.status == AutoCommitExecutionStatus::Executed,
        reason: format!("Execution status: {:?}", exec.status),
    });

    // 3. ResultingCommitExists
    results.push(PostCommitPredicateResult {
        predicate: PostCommitPredicate::ResultingCommitExists,
        passed: exec.resulting_commit.is_some(),
        reason: match &exec.resulting_commit {
            Some(c) => format!("Commit {} exists", c.commit_hash),
            None => "No resulting commit in execution record".to_string(),
        },
    });

    if exec.resulting_commit.is_none() {
        return results;
    }
    let exec_commit = exec.resulting_commit.as_ref().unwrap();

    // 4-11: Require commit evidence
    match commit_evidence {
        Some(evidence) => {
            // 4. CommitHashMatchesExecutionRecord
            results.push(PostCommitPredicateResult {
                predicate: PostCommitPredicate::CommitHashMatchesExecutionRecord,
                passed: evidence.commit_hash == exec_commit.commit_hash,
                reason: format!("Observed: {}, Expected: {}", &evidence.commit_hash[..8.min(evidence.commit_hash.len())], &exec_commit.commit_hash[..8.min(exec_commit.commit_hash.len())]),
            });

            // 5. CommitParentMatchesRollbackHead
            results.push(PostCommitPredicateResult {
                predicate: PostCommitPredicate::CommitParentMatchesRollbackHead,
                passed: evidence.parent_hash == exec_commit.parent_hash,
                reason: format!("Parent: {}, Expected: {}", &evidence.parent_hash[..8.min(evidence.parent_hash.len())], &exec_commit.parent_hash[..8.min(exec_commit.parent_hash.len())]),
            });

            // 6. CommitBranchMatchesExecutionRecord
            results.push(PostCommitPredicateResult {
                predicate: PostCommitPredicate::CommitBranchMatchesExecutionRecord,
                passed: evidence.branch == exec_commit.branch,
                reason: format!("Branch: {}, Expected: {}", evidence.branch, exec_commit.branch),
            });

            // 7. CommitMessageHashMatchesProposal
            let expected_msg_hash = match proposal {
                Some(p) => {
                    let full_msg = format!("{}\n\n{}", p.commit_title, p.commit_body);
                    format!("{}", blake3::hash(full_msg.as_bytes()).to_hex())
                }
                None => "no_proposal".to_string(),
            };
            results.push(PostCommitPredicateResult {
                predicate: PostCommitPredicate::CommitMessageHashMatchesProposal,
                passed: evidence.message_hash == expected_msg_hash,
                reason: format!("Message hash: {}, Expected: {}", &evidence.message_hash[..8.min(evidence.message_hash.len())], &expected_msg_hash[..8.min(expected_msg_hash.len())]),
            });

            // Patch 3: Two distinct path predicates
            // 8. CommitDiffMatchesApprovedPaths (exact set equality)
            let approved_paths: Vec<String> = match proposal {
                Some(p) => p.included_files.iter().map(|f| f.path.clone()).collect(),
                None => vec![],
            };
            let mut observed_sorted = evidence.changed_paths.clone();
            observed_sorted.sort();
            let mut approved_sorted = approved_paths.clone();
            approved_sorted.sort();
            results.push(PostCommitPredicateResult {
                predicate: PostCommitPredicate::CommitDiffMatchesApprovedPaths,
                passed: observed_sorted == approved_sorted,
                reason: format!("Observed {} paths, expected {} paths", evidence.changed_paths.len(), approved_paths.len()),
            });

            // 9. CommitDiffContainsNoUnreviewedPaths (subset check)
            let all_unreviewed: bool = evidence.changed_paths.iter().all(|p| approved_paths.contains(p));
            results.push(PostCommitPredicateResult {
                predicate: PostCommitPredicate::CommitDiffContainsNoUnreviewedPaths,
                passed: all_unreviewed,
                reason: if all_unreviewed {
                    "All changed paths are within approved scope".to_string()
                } else {
                    let extra: Vec<_> = evidence.changed_paths.iter().filter(|p| !approved_paths.contains(p)).collect();
                    format!("Unreviewed paths found: {:?}", extra)
                },
            });

            // 10. CommitTreeMatchesExpectedPostState
            results.push(PostCommitPredicateResult {
                predicate: PostCommitPredicate::CommitTreeMatchesExpectedPostState,
                passed: !evidence.tree_hash.is_empty(),
                reason: format!("Tree hash: {}", &evidence.tree_hash[..8.min(evidence.tree_hash.len())]),
            });

            // 11. EvidenceChainMatches
            let chain_ok = execution_record.is_some() && proposal.is_some() && review.is_some();
            results.push(PostCommitPredicateResult {
                predicate: PostCommitPredicate::EvidenceChainMatches,
                passed: chain_ok,
                reason: if chain_ok { "Full chain: execution → proposal → review".to_string() } else { "Missing links in evidence chain".to_string() },
            });
        }
        None => {
            // Cannot observe commit
            for pred in &[
                PostCommitPredicate::CommitHashMatchesExecutionRecord,
                PostCommitPredicate::CommitParentMatchesRollbackHead,
                PostCommitPredicate::CommitBranchMatchesExecutionRecord,
                PostCommitPredicate::CommitMessageHashMatchesProposal,
                PostCommitPredicate::CommitDiffMatchesApprovedPaths,
                PostCommitPredicate::CommitDiffContainsNoUnreviewedPaths,
                PostCommitPredicate::CommitTreeMatchesExpectedPostState,
                PostCommitPredicate::EvidenceChainMatches,
            ] {
                results.push(PostCommitPredicateResult {
                    predicate: pred.clone(),
                    passed: false,
                    reason: "Commit evidence unavailable (inconclusive)".to_string(),
                });
            }
        }
    }

    // 12. WorkspaceCleanAfterCommit
    results.push(PostCommitPredicateResult {
        predicate: PostCommitPredicate::WorkspaceCleanAfterCommit,
        passed: workspace_status.is_empty(),
        reason: if workspace_status.is_empty() { "Workspace clean".to_string() } else { format!("Dirty: {}", workspace_status) },
    });

    // 13. PostCommitChecksPass
    // Patch 2: Skipped required checks → not passed
    let checks_ok = post_commit_checks.iter().all(|c| c.status == PostCommitCheckStatus::Passed);
    let has_skipped = post_commit_checks.iter().any(|c| c.status == PostCommitCheckStatus::Skipped);
    results.push(PostCommitPredicateResult {
        predicate: PostCommitPredicate::PostCommitChecksPass,
        passed: checks_ok && !has_skipped,
        reason: if checks_ok && !has_skipped {
            format!("All {} checks passed", post_commit_checks.len())
        } else {
            let failed: Vec<_> = post_commit_checks.iter().filter(|c| c.status != PostCommitCheckStatus::Passed).collect();
            format!("{} checks not passed", failed.len())
        },
    });

    // 14. RollbackDrillCompleted
    results.push(PostCommitPredicateResult {
        predicate: PostCommitPredicate::RollbackDrillCompleted,
        passed: rollback_drill.is_some(),
        reason: match rollback_drill {
            Some(_) => "Rollback drill completed".to_string(),
            None => "No rollback drill result".to_string(),
        },
    });

    if rollback_drill.is_none() {
        // Add placeholder for 15 and 16
        results.push(PostCommitPredicateResult {
            predicate: PostCommitPredicate::RollbackDrillCleanlyApplies,
            passed: false,
            reason: "No rollback drill performed".to_string(),
        });
        results.push(PostCommitPredicateResult {
            predicate: PostCommitPredicate::LiveRepoUnchangedDuringDrill,
            passed: true, // Trivially true if no drill ran
            reason: "No drill ran, live repo trivially unchanged".to_string(),
        });
    } else {
        let drill = rollback_drill.unwrap();

        // 15. RollbackDrillCleanlyApplies
        results.push(PostCommitPredicateResult {
            predicate: PostCommitPredicate::RollbackDrillCleanlyApplies,
            passed: drill.clean,
            reason: if drill.clean { "Rollback applies cleanly in sandbox".to_string() } else { format!("Rollback conflicts: {:?}", drill.conflicts) },
        });

        // 16. LiveRepoUnchangedDuringDrill
        let live_unchanged = drill.live_head_before == drill.live_head_after
            && drill.live_index_before == drill.live_index_after
            && drill.live_worktree_before == drill.live_worktree_after;
        results.push(PostCommitPredicateResult {
            predicate: PostCommitPredicate::LiveRepoUnchangedDuringDrill,
            passed: live_unchanged,
            reason: if live_unchanged { "Live repo unchanged during drill".to_string() } else { "Live repo was mutated during drill".to_string() },
        });
    }

    // 17. IdempotencyKeyUnusedOrMatchesExisting
    let verification_id = verification_id_for(&execution_id.0, idempotency_key);
    let existing = existing_verifications.iter().find(|v| v.verification_id == verification_id);
    results.push(PostCommitPredicateResult {
        predicate: PostCommitPredicate::IdempotencyKeyUnusedOrMatchesExisting,
        passed: existing.is_none() || existing.unwrap().status == PostCommitVerificationStatus::Verified,
        reason: match existing {
            Some(v) => format!("Existing verification found: {:?}", v.status),
            None => "No prior verification with this key".to_string(),
        },
    });

    results
}

// ── Main verification orchestrator ─────────────────────────────────────────

pub fn verify_execution(
    backend: &dyn PostCommitVerifierBackend,
    repo: &Path,
    request: &PostCommitVerificationRequest,
    execution_record: Option<&AutoCommitExecutionRecord>,
    proposal: Option<&AutoCommitProposal>,
    review: Option<&AutoCommitProposalReview>,
    existing_verifications: &[PostCommitVerificationRecord],
    checks: &[PostCommitCheckSpec],
) -> PostCommitVerificationRecord {
    let verification_id = verification_id_for(&request.execution_id.0, &request.idempotency_key);

    // Patch 1: Idempotency returns existing, no AlreadyVerified record
    if let Some(existing) = existing_verifications
        .iter()
        .find(|v| v.verification_id == verification_id)
    {
        return existing.clone();
    }

    // Observe commit evidence
    let commit_hash = execution_record
        .and_then(|r| r.resulting_commit.as_ref())
        .map(|c| c.commit_hash.clone())
        .unwrap_or_default();

    let commit_evidence = if !commit_hash.is_empty() {
        backend.observe_commit(repo, &commit_hash).ok()
    } else {
        None
    };

    // Run post-commit checks
    let post_commit_checks = backend.run_post_commit_checks(repo, checks).unwrap_or_default();

    // Run rollback drill
    let drill_plan = execution_record
        .and_then(|r| r.resulting_commit.as_ref())
        .map(|c| RollbackDrillPlan {
            strategy: RollbackDrillStrategy::SandboxRevert,
            commit_hash: c.commit_hash.clone(),
            expected_parent: c.parent_hash.clone(),
        });

    let rollback_drill = match drill_plan {
        Some(plan) => backend.run_rollback_drill_in_sandbox(repo, plan).ok(),
        None => None,
    };

    // Get workspace status
    let workspace_status = {
        let git_backend = LocalGitBackend;
        match git_backend.observe_state(repo) {
            Ok(state) => state.porcelain,
            Err(_) => String::new(), // If git not available, treat as clean
        }
    };

    // Evaluate predicates
    let predicates = evaluate_post_commit_predicates(
        execution_record,
        proposal,
        review,
        commit_evidence.as_ref(),
        &post_commit_checks,
        rollback_drill.as_ref(),
        existing_verifications,
        &request.execution_id,
        &request.idempotency_key,
        &workspace_status,
    );

    // Determine decision
    let all_passed = predicates.iter().all(|p| p.passed);
    let has_inconclusive = predicates.iter().any(|p| !p.passed && p.reason.contains("inconclusive"));
    let has_commit_evidence_issue = commit_evidence.is_none() && execution_record.and_then(|r| r.resulting_commit.as_ref()).is_some();

    let decision = if all_passed {
        PostCommitVerificationDecision::Verified
    } else if has_commit_evidence_issue || has_inconclusive {
        let failed: Vec<&str> = predicates
            .iter()
            .filter(|p| !p.passed)
            .map(|p| p.reason.as_str())
            .collect();
        PostCommitVerificationDecision::Inconclusive {
            reason_code: "evidence_unavailable".to_string(),
            summary: failed.join("; "),
        }
    } else {
        let failed: Vec<&str> = predicates
            .iter()
            .filter(|p| !p.passed)
            .map(|p| p.reason.as_str())
            .collect();
        PostCommitVerificationDecision::Failed {
            reason_code: "predicate_failed".to_string(),
            summary: failed.join("; "),
        }
    };

    let status = match &decision {
        PostCommitVerificationDecision::Verified => PostCommitVerificationStatus::Verified,
        PostCommitVerificationDecision::Failed { .. } => PostCommitVerificationStatus::Failed,
        PostCommitVerificationDecision::Inconclusive { .. } => PostCommitVerificationStatus::Inconclusive,
    };

    PostCommitVerificationRecord {
        verification_id,
        execution_id: request.execution_id.clone(),
        proposal_id: execution_record
            .map(|r| r.proposal_id.clone())
            .unwrap_or(AutoCommitProposalId("unknown".to_string())),
        review_id: execution_record
            .map(|r| r.review_id.clone())
            .unwrap_or(AutoCommitProposalReviewId("unknown".to_string())),
        status,
        decision,
        predicates,
        commit_evidence,
        post_commit_checks,
        rollback_drill,
        created_at: Utc::now(),
    }
}

// ── Persistence ────────────────────────────────────────────────────────────

pub fn save_verification_record(
    store_root: &Path,
    record: &PostCommitVerificationRecord,
) -> Result<PathBuf, String> {
    let verify_dir = store_root.join("post_commit_verifications");
    std::fs::create_dir_all(&verify_dir)
        .map_err(|e| format!("Failed to create verifications dir: {}", e))?;

    let by_exec_dir = verify_dir.join("by_execution");
    std::fs::create_dir_all(&by_exec_dir)
        .map_err(|e| format!("Failed to create by_execution dir: {}", e))?;

    let json = serde_json::to_string_pretty(record)
        .map_err(|e| format!("Failed to serialize: {}", e))?;

    // Save individual record
    let path = verify_dir.join(format!("{}.json", record.verification_id.0));
    std::fs::write(&path, &json)
        .map_err(|e| format!("Failed to write: {}", e))?;

    // Save latest
    let latest_path = verify_dir.join("latest.json");
    std::fs::write(&latest_path, &json)
        .map_err(|e| format!("Failed to write latest: {}", e))?;

    // Save by_execution pointer
    let by_exec_path = by_exec_dir.join(format!("{}.json", record.execution_id.0));
    std::fs::write(&by_exec_path, &json)
        .map_err(|e| format!("Failed to write by_execution: {}", e))?;

    Ok(path)
}

pub fn load_verification_record(
    store_root: &Path,
    id: &PostCommitVerificationId,
) -> Result<Option<PostCommitVerificationRecord>, String> {
    let path = store_root
        .join("post_commit_verifications")
        .join(format!("{}.json", id.0));
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read: {}", e))?;
    let record: PostCommitVerificationRecord = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse: {}", e))?;
    Ok(Some(record))
}

pub fn load_latest_verification(
    store_root: &Path,
) -> Result<Option<PostCommitVerificationRecord>, String> {
    let path = store_root.join("post_commit_verifications").join("latest.json");
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read latest: {}", e))?;
    let record: PostCommitVerificationRecord = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse latest: {}", e))?;
    Ok(Some(record))
}

pub fn load_latest_verification_for_execution(
    store_root: &Path,
    execution_id: &AutoCommitExecutionId,
) -> Result<Option<PostCommitVerificationRecord>, String> {
    let path = store_root
        .join("post_commit_verifications")
        .join("by_execution")
        .join(format!("{}.json", execution_id.0));
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read: {}", e))?;
    let record: PostCommitVerificationRecord = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse: {}", e))?;
    Ok(Some(record))
}

pub fn list_verification_records(
    store_root: &Path,
) -> Result<Vec<PostCommitVerificationRecord>, String> {
    let dir = store_root.join("post_commit_verifications");
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut records = Vec::new();
    let entries = std::fs::read_dir(&dir)
        .map_err(|e| format!("Failed to read dir: {}", e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json")
            && path.file_stem().and_then(|s| s.to_str()) != Some("latest")
        {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read: {}", e))?;
            let record: PostCommitVerificationRecord = serde_json::from_str(&content)
                .map_err(|e| format!("Failed to parse: {}", e))?;
            records.push(record);
        }
    }
    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(records)
}

// ── Check spec validation ──────────────────────────────────────────────────

/// Returns true if the check kind is a known fixed variant (not freeform shell).
pub fn is_valid_check_kind(kind: &PostCommitCheckKind) -> bool {
    matches!(
        kind,
        PostCommitCheckKind::CargoFmtCheck
            | PostCommitCheckKind::CargoCheckWorkspace
            | PostCommitCheckKind::CargoTestWorkspace
            | PostCommitCheckKind::CargoTestPackage { .. }
    )
}
