# Wave 14: Post-Commit Verification and Rollback Drill — Lock

**Commit:** `9c2335f`  
**Date:** 2026-06-02  
**Status:** LOCKED  
**Tests:** 1246 total, zero failures

## Scope

Verify a governed local commit after creation, bind it back to proposal/review/execution evidence, run deterministic post-commit checks, persist a verification record, and rehearse rollback in a disposable sandbox.

No push, tag, branch creation, release, remote operation, live reset, live revert, live rollback execution, arbitrary shell, or general git command execution.

## Module Boundary

```
Wave 11: eval_proposal.rs             → proposal generation
Wave 12: eval_proposal_review.rs      → review and feedback
Wave 13: eval_proposal_execution.rs   → execution gate and local commit record
Wave 14: eval_post_commit_verify.rs   → post-commit verification and rollback drill
```

Wave 14 consumes Wave 13 execution records. It does not create commits.

## Invariant Chain

```
Wave 11: "What exactly would be proposed as a commit?"
Wave 12: "Was that proposal approved, rejected, or returned for changes — and why?"
Wave 13: "Was this exact approved proposal still valid and safely committed?"
Wave 14: "Did the resulting commit match the approved evidence, and can rollback be rehearsed safely?"
```

## Patches Applied

### Patch 1: AlreadyVerified removed from status enum

```rust
pub enum PostCommitVerificationStatus {
    Verified,
    Failed,
    Inconclusive,
}
```

Idempotency: same `(execution_id, idempotency_key)` returns the existing record. No second `AlreadyVerified` record persisted.

### Patch 2: Explicit check status enum

```rust
pub enum PostCommitCheckStatus { Passed, Failed, Skipped }
```

- Verified requires all required checks to be Passed.
- Failed check → Failed verification.
- Skipped required check → Failed or Inconclusive, never Verified.

### Patch 3: Two distinct path predicates

```
CommitDiffMatchesApprovedPaths:
  observed_changed_paths == proposal.approved_paths (exact set equality)

CommitDiffContainsNoUnreviewedPaths:
  observed_changed_paths ⊆ proposal.approved_paths (subset check)
```

Both retained to distinguish "missing expected approved path" from "extra unreviewed path."

## Backend Shape

```rust
pub trait PostCommitVerifierBackend {
    fn observe_commit(&self, repo: &Path, commit_hash: &str) -> Result<CommitEvidenceSnapshot, PostCommitVerifyError>;
    fn run_post_commit_checks(&self, repo: &Path, checks: &[PostCommitCheckSpec]) -> Result<Vec<PostCommitCheckResult>, PostCommitVerifyError>;
    fn run_rollback_drill_in_sandbox(&self, repo: &Path, plan: RollbackDrillPlan) -> Result<RollbackDrillResult, PostCommitVerifyError>;
}
```

**Forbidden methods:** push, tag, branch, release, reset_live_repo, revert_live_repo, checkout_live_repo, run_git, run_shell.

**Allowed behavior:** observe existing commit, compute hashes, run fixed local checks, create disposable sandbox, rehearse rollback inside sandbox only, prove live HEAD/index/worktree remain unchanged.

## 17 Verification Predicates

| # | Predicate | Description |
|---|-----------|-------------|
| 1 | ExecutionRecordExists | Execution record file found |
| 2 | ExecutionWasSuccessful | Execution status == Executed |
| 3 | ResultingCommitExists | Execution has resulting_commit |
| 4 | CommitHashMatchesExecutionRecord | Observed hash == recorded hash |
| 5 | CommitParentMatchesRollbackHead | Parent == rollback plan HEAD |
| 6 | CommitBranchMatchesExecutionRecord | Branch matches execution record |
| 7 | CommitMessageHashMatchesProposal | Message hash == proposal message hash |
| 8 | CommitDiffMatchesApprovedPaths | Exact set equality (Patch 3) |
| 9 | CommitDiffContainsNoUnreviewedPaths | Subset check (Patch 3) |
| 10 | CommitTreeMatchesExpectedPostState | Tree hash present and non-empty |
| 11 | EvidenceChainMatches | Full chain: execution → proposal → review |
| 12 | WorkspaceCleanAfterCommit | `git status --porcelain` is empty |
| 13 | PostCommitChecksPass | All required checks Passed (Patch 2) |
| 14 | RollbackDrillCompleted | Drill result exists |
| 15 | RollbackDrillCleanlyApplies | Drill clean == true |
| 16 | LiveRepoUnchangedDuringDrill | Live HEAD/index/worktree match before/after |
| 17 | IdempotencyKeyUnusedOrMatchesExisting | No conflicting verification (Patch 1) |

## Post-Commit Checks

Fixed enum only. No freeform shell:

```rust
pub enum PostCommitCheckKind {
    CargoFmtCheck,
    CargoCheckWorkspace,
    CargoTestWorkspace,
    CargoTestPackage { package: String },
}
```

## Rollback Drill Semantics

1. Capture live HEAD/index/worktree before
2. Clone repo to disposable sandbox (tempdir)
3. Checkout commit hash in sandbox
4. Run `git revert --no-edit <commit>` in sandbox
5. Record sandbox pre/post HEAD, diff hash, conflicts
6. Capture live HEAD/index/worktree after
7. Prove live repo unchanged

**Never** mutates live repo. Never pushes, tags, branches.

## Key DTOs

- `PostCommitVerificationId` — BLAKE3 content-addressed (`pcv_<hex>`)
- `PostCommitVerificationRecord` — full record with status, decision, predicates, evidence, checks, drill
- `CommitEvidenceSnapshot` — observed commit hash, parent, tree, branch, message hash, changed paths, diff hash
- `PostCommitCheckResult` — spec + status (Passed/Failed/Skipped) + output summary
- `RollbackDrillResult` — strategy, clean, sandbox state, conflicts, live before/after snapshots

## Persistence

```
eval_reports/post_commit_verifications/
  <verification_id>.json
  latest.json
  by_execution/<execution_id>.json
```

## CLI

```
openwand eval auto-commit verify --execution-id <id> [--idempotency-key] [--json]
openwand eval auto-commit verification show <verification-id>
openwand eval auto-commit verification latest [--execution-id <id>]
```

All feature-gated behind `real-model-eval`.

## Test Coverage (54 tests)

### DTO and Builder (8)
- verification_request_roundtrips, verification_record_roundtrips
- verified_record_requires_commit_evidence, verified_record_requires_rollback_drill_result
- failed_record_requires_reason, inconclusive_record_requires_reason
- verification_id_is_content_addressed, verification_id_is_deterministic

### Predicates (15)
- blocks_missing_execution_record, blocks_non_executed_execution_record
- blocks_missing_resulting_commit, blocks_commit_hash_mismatch
- blocks_parent_hash_mismatch, blocks_branch_mismatch
- blocks_commit_message_hash_mismatch, blocks_unreviewed_changed_path
- blocks_diff_hash_mismatch_via_path_count, blocks_evidence_chain_mismatch
- blocks_failed_post_commit_check, blocks_missing_rollback_drill
- blocks_failed_rollback_drill, inconclusive_when_commit_cannot_be_observed
- all_predicates_pass_for_valid_commit

### Post-Commit Checks (6)
- post_commit_check_spec_rejects_freeform_shell
- cargo_fmt/check/test_workspace maps to fixed commands
- failed_check_blocks_verification, skipped_check_does_not_verify_by_default

### Rollback Drill (8)
- rollback_drill_runs_only_in_sandbox
- rollback_drill_does_not_mutate_live_head/index/worktree
- rollback_drill_clean_revert_succeeds, rollback_drill_conflict_returns_conflicts
- rollback_drill_failure_returns_failed, rollback_drill_result_records_summary

### Persistence and Idempotency (5)
- verification_persists_and_loads_roundtrip
- latest_verification_returns_expected
- latest_verification_for_execution_returns_expected
- same_idempotency_key_returns_existing_verification
- list_verification_records_sorted_by_date

### CLI (5)
- cli_verify_success_outputs_verified
- cli_verify_failed_outputs_predicates
- cli_verify_does_not_execute_live_rollback
- cli_verification_show_roundtrips_record
- cli_verification_latest_returns_latest

### Source and Runtime Guards (7)
- module_does_not_push_tag_branch_or_release
- module_does_not_call_remote_operations
- module_does_not_execute_live_reset_or_live_revert
- command_only_used_inside_verifier_backend
- verifier_backend_uses_fixed_allowed_commands
- verifier_backend_never_invokes_shell
- live_repo_head_index_worktree_unchanged_after_rollback_drill

## Honest Caveats

- Wave 14 verifies and rehearses rollback. It does not execute rollback in the live repository.
- A clean rollback drill means the revert applies in a controlled sandbox from the expected commit state. It does not guarantee future rollback will remain clean after later commits land.
- Future waves may add live rollback execution, remote push governance, tagging, release, or post-release audit — those are explicitly outside Wave 14.
