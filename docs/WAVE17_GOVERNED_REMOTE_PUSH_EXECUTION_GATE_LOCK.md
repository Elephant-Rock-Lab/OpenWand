# Wave 17: Governed Remote Push Execution Gate — Lock

**Commit:** `2598e22`
**Date:** 2026-06-02
**Status:** LOCKED
**Tests:** 1418 total, zero failures

## Scope

Execute exactly one approved remote push proposal as a fast-forward update to one existing remote branch, after revalidating all evidence and remote state at execution time. Persist a push execution record. No force push, tag, release, branch creation, fetch, pull, merge, rebase, live rollback, arbitrary shell, or general git execution.

## Module Boundary

```
Wave 15: eval_remote_push_readiness.rs    → remote push readiness only
Wave 16: eval_remote_push_proposal.rs      → push proposal and human review
Wave 17: eval_remote_push_execution.rs     → governed remote push execution (this module)
```

Wave 17 is the first remote mutation crossing.

## Invariant Chain

```
Wave 15: "Is this verified commit eligible to be proposed for remote push?"
Wave 16: "Was the exact push proposal approved, rejected, or returned for changes?"
Wave 17: "Was this exact approved push proposal still valid and safely pushed?"
```

A Ready readiness record is not a push.
A push proposal is not a push.
A human approval is not a push.
Only Wave 17 may cross the remote mutation boundary.

## Core DTOs

- `RemotePushExecutionId` — BLAKE3 content-addressed (`rpe_<hex>`)
- `RemotePushExecutionRequest` — proposal_id, review_id, requested_by, idempotency_key
- `RemotePushExecutionRecord` — full execution record with status/decision/predicates/snapshots
- `RemotePushExecutionStatus` — Blocked / Executed / AlreadyExecuted
- `RemotePushExecutionDecision` — Allow / Block { reason_code, summary }
- `RemotePushExecutionPredicate` — 28 predicates (see below)
- `RemoteRefObservedSnapshot` — remote ref state observed before/after push
- `RemotePushResultSnapshot` — resulting remote ref update evidence
- `RemotePushRecoverySnapshot` — recovery evidence (strategy, old/new commits, notes)
- `LocalPushExecutionSnapshot` — current HEAD, branch, worktree/index clean

## Execution Predicates (28)

All must pass in the same execution attempt:

1. ProposalExists
2. ProposalEligible
3. ReviewExists
4. ReviewIsLatestForProposal
5. ReviewApproved
6. ReviewProposalHashMatchesProposal
7. ReviewReadinessHashMatchesProposal
8. ReadinessRecordExists
9. ReadinessStillReady
10. VerificationRecordExists
11. VerificationStillVerified
12. LocalExecutionRecordExists
13. LocalExecutionWasSuccessful
14. CurrentHeadMatchesProposalCommit
15. CurrentBranchMatchesProposalBranch
16. WorktreeClean
17. IndexClean
18. BranchPolicyLoaded
19. BranchPolicyStillAllowsPush
20. TargetRemoteConfigured
21. RemoteBranchExists
22. RemoteRefMatchesExpectedOldCommit
23. PushIsFastForward
24. CommitIsDescendantOfRemoteRef
25. PolicyAllowsRemotePush
26. RecoveryEvidenceExists
27. IdempotencyKeyUnusedOrMatchesExisting
28. NoPriorConflictingPushExecution

## Backend Trait

```rust
pub trait RemotePushExecutionBackend: Send + Sync {
    fn observe_current_local_state(&self, repo: &Path)
        -> Result<LocalPushExecutionSnapshot, RemotePushExecutionError>;
    fn observe_remote_ref(&self, repo: &Path, remote: &str, branch: &str)
        -> Result<RemoteRefObservedSnapshot, RemotePushExecutionError>;
    fn execute_fast_forward_push_exact(&self, repo: &Path, request: ExactRemotePushRequest)
        -> Result<RemotePushResultSnapshot, RemotePushExecutionError>;
}
```

## Allowed Command Templates (exactly 6)

1. `git rev-parse HEAD`
2. `git symbolic-ref --short HEAD`
3. `git status --porcelain`
4. `git merge-base --is-ancestor <old> <new>`
5. `git ls-remote <remote> refs/heads/<branch>` (exact ref only)
6. `git push --porcelain <remote> <commit>:refs/heads/<branch>` (exact refspec only)

All arguments constructed programmatically. No shell string interpolation.

## Idempotency Rules

Same `(proposal_id, review_id, idempotency_key)` returns the existing execution record, whether Blocked or Executed.

A blocked attempt may be retried only with a new idempotency key after conditions change.
An executed attempt cannot be duplicated with any key for the same proposal/review/commit.

## CLI

```
openwand eval auto-commit push execute --proposal-id --review-id [--idempotency-key] [--output-dir] [--json]
openwand eval auto-commit push execution show <execution-id> [--output-dir]
openwand eval auto-commit push execution latest [--proposal-id] [--review-id] [--commit] [--output-dir]
```

Feature-gated behind `real-model-eval`.

## Test Coverage (58 tests)

- **DTO** (8): roundtrips, content-addressed, deterministic, differs, snapshots
- **Predicates** (22): missing proposal, rejected/changes review, hash mismatch, non-ready readiness, non-verified verification, failed execution, HEAD/branch mismatch, dirty worktree/index, policy denial, missing remote, remote ref mismatch, non-fast-forward, missing review/readiness/verification/remote-branch, conflicting prior execution, all-pass
- **Persistence/Idempotency** (10): roundtrip, latest, by_proposal/review/commit, sorted, same-key blocked, retry new key, executed cannot duplicate, blocked includes recovery
- **CLI** (7): execution_id, roundtrip, by_proposal/review/commit, blocked outputs predicates, no general git
- **Guards** (10): no force push, no tags/all/mirror/delete, no fetch/pull, no tags/branches, no release tools, no shell, command in backend only, fixed commands, failed not executed, pre/post snapshots

## Honest Caveats

- Runtime git integration tests using local bare remotes are not yet included. The test backend covers all predicate paths. Real git bare-remote tests are the next priority.
- Push recovery is evidence-only. It records old/new commits and recommended strategy but does not execute remote rollback.
- `ls-remote` contact with real remotes is network-dependent. The local backend calls it; the test backend injects values.
- Policy integration uses a simple boolean flag. Full policy engine integration deferred.
