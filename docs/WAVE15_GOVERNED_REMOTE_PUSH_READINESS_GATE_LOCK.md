# Wave 15: Governed Remote Push Readiness Gate — Lock

**Commit:** `4f9f7c7`
**Date:** 2026-06-02
**Status:** LOCKED
**Tests:** 1299 total, zero failures

## Scope

Determine whether a verified governed local commit is eligible for remote push. Read-only observation only. No push, fetch, pull, tag, branch creation, release, remote mutation, live rollback, arbitrary shell, or general git execution.

## Module Boundary

```
Wave 11: eval_proposal.rs              → proposal generation
Wave 12: eval_proposal_review.rs       → review and feedback
Wave 13: eval_proposal_execution.rs    → execution gate and local commit record
Wave 14: eval_post_commit_verify.rs    → post-commit verification and rollback drill
Wave 15: eval_remote_push_readiness.rs → remote push readiness only
```

## Invariant Chain

```
Wave 11: "What exactly would be proposed as a commit?"
Wave 12: "Was that proposal approved, rejected, or returned for changes?"
Wave 13: "Was this exact approved proposal still valid and safely committed?"
Wave 14: "Did the resulting commit match the approved evidence, and can rollback be rehearsed?"
Wave 15: "Is this verified commit eligible to be proposed for remote push?"
```

## Patches Applied

### Patch 1: No `git branch` — use `git symbolic-ref`

`LocalPushReadinessBackend` uses `git symbolic-ref --short HEAD` for branch name detection. No `git branch` command appears in code. Guard test verifies.

### Patch 2: Remote URL check without network

`TargetRemoteConfigured` predicate checks `git config --get remote.<name>.url`. Added as `check_remote_configured()` to the backend trait so test backends can override. Does not contact network.

### Patch 3: Deterministic branch policy matching

```
1. Exact match wins.
2. Longest prefix wildcard wins.
3. Default policy applies.
4. Equal specificity → block as ambiguous.
```

### Patch 4: Two commits

1. Code + tests
2. Lock doc (this file)

## Backend Trait

```rust
pub trait RemotePushReadinessBackend {
    fn observe_local_branch_state(...) -> Result<LocalBranchPushSnapshot, ...>;
    fn observe_remote_tracking_state(...) -> Result<RemoteTrackingSnapshot, ...>;
    fn load_branch_policy(...) -> Result<BranchProtectionPolicySnapshot, ...>;
    fn check_remote_configured(&self, repo: &Path, target_remote: &str) -> Result<bool, ...>;
}
```

**Allowed git commands** (read-only): symbolic-ref, rev-parse, config --get, rev-list --left-right --count, merge-base --is-ancestor, status --porcelain.
**Forbidden:** push, fetch, pull, tag, branch, checkout, switch, reset, revert, merge, rebase, remote add/set-url/remove, ls-remote, hub, gh, glab, curl, ssh, arbitrary shell.

## 25 Predicates

| # | Predicate | Description |
|---|-----------|-------------|
| 1 | VerificationRecordExists | Wave 14 record found |
| 2 | VerificationIsVerified | status == Verified |
| 3 | ExecutionRecordExists | execution_id linked |
| 4 | ExecutionWasSuccessful | proven by Wave 14 |
| 5 | CommitHashMatchesVerification | verified hash present |
| 6 | CurrentHeadMatchesVerifiedCommit | HEAD == verified commit |
| 7 | WorktreeClean | no uncommitted changes |
| 8 | IndexClean | no staged changes |
| 9 | TargetRemoteConfigured | remote URL in local config (Patch 2) |
| 10 | TargetBranchMatchesPolicy | policy loaded |
| 11 | UpstreamOrTrackingRefKnown | tracking ref exists |
| 12 | LocalBranchAheadOfRemote | ahead >= 1 |
| 13 | LocalBranchNotBehindRemote | behind == 0 |
| 14 | LocalBranchNotDiverged | not diverged |
| 15 | CommitIsDescendantOfRemoteTrackingRef | ancestry verified |
| 16 | BranchPolicyLoaded | policy snapshot exists |
| 17 | DirectPushAllowedByPolicy | policy allows push |
| 18 | ProtectedBranchRequirementsSatisfied | protected branch checks |
| 19 | PostCommitChecksPassed | all Wave 14 checks passed |
| 20 | NoSkippedRequiredChecks | no skipped checks |
| 21 | RollbackDrillEvidencePresent | drill exists |
| 22 | RollbackDrillWasClean | drill clean == true |
| 23 | LiveRepoUnchangedDuringRollbackDrill | live unchanged |
| 24 | NoPriorConflictingReadinessRecord | no conflicting records |
| 25 | IdempotencyKeyUnusedOrMatchesExisting | idempotency |

## Branch Policy

Source: `.openwand/push_policy.toml` (parsed via `toml = "0.8"`).

Default: main/master → protected (not ready for direct push); non-protected → may be ready if all evidence passes.

## Persistence

```
eval_reports/remote_push_readiness/
  <readiness_id>.json
  latest.json
  by_verification/<verification_id>.json
  by_commit/<commit_hash>.json
```

## Test Coverage (53 tests)

- **DTO** (8): roundtrips, content-addressed, deterministic, differs by remote/branch
- **Predicates** (19): all blocking cases + happy path + inconclusive
- **Branch Policy** (6): default protects main, pattern matching, ambiguous blocks
- **Persistence** (6): roundtrip, latest, by_verification, by_commit, idempotency, sorted
- **CLI** (6): ready/blocked/inconclusive output, show roundtrip, latest, no push
- **Guards** (8): no push/fetch/pull/ls-remote, no tags/branches, no shell, symbolic-ref check

## Honest Caveats

- Wave 15 determines readiness. It does not push.
- Network-free: relies on local remote-tracking refs. Stale/missing → Inconclusive.
- Remote-host branch protection, CI provider status, PR requirements represented only through local policy.
- Future waves may add read-only provider API observation, push proposal review, governed push execution, tagging, release, or post-push audit.
