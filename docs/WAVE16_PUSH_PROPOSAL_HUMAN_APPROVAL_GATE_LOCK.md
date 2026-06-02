# Wave 16: Push Proposal and Human Approval Gate — Lock

**Commit:** `ae5add5`
**Date:** 2026-06-02
**Status:** LOCKED
**Tests:** 1360 total, zero failures

## Scope

Turn a Ready remote-push-readiness record into an exact governed push proposal, persist the proposal, require human approval/rejection/change-request, persist the review decision, and export structured feedback. No push, fetch, pull, tag, branch creation, release, remote mutation, live rollback, arbitrary shell, or general git execution.

## Module Boundary

```
Wave 15: eval_remote_push_readiness.rs    → remote push readiness only
Wave 16: eval_remote_push_proposal.rs      → push proposal and human review
```

Wave 16 consumes Wave 15 readiness records. It does not push.

## Invariant Chain

```
Wave 15: "Is this verified commit eligible to be proposed for remote push?"
Wave 16: "Was the exact push proposal approved, rejected, or returned for changes?"
Wave 17: "Was this exact approved push proposal still valid and safely pushed?"
```

## Patches Applied

### Patch 1: 2 commits

1. Code + tests
2. Lock doc

### Patch 2: Explicit idempotency

`RemotePushProposalRequest` and `RemotePushProposalReviewRequest` both carry `idempotency_key`. Same key returns existing record. Different key creates new review preserving audit history.

### Patch 3: `readiness_hash` from persisted record

Copied from serialized Wave 15 readiness record at proposal creation time. Never recomputed from current git state. Preserves the Wave 15→16→17 evidence chain.

## Core DTOs

- `RemotePushProposalId` — BLAKE3 content-addressed (`rpp_<hex>`)
- `RemotePushProposalReviewId` — BLAKE3 content-addressed (`rprv_<hex>`)
- `RemotePushProposal` — ref_update, status, hashes, all copied from readiness
- `RemotePushProposalReview` — decision, reviewer, rationale, feedback, `creates_execution_grant: false`, `execution_allowed_now: false`
- `RemoteRefUpdateSnapshot` — remote_name, branch, ref_name, expected_old_commit, proposed_new_commit, fast_forward_only, ahead/behind/diverged
- `RemotePushProposalFeedback` — summary, blocking_reasons, requested_changes, evidence_gaps, suggested_next_action

## Builder Rules

### Proposal

Can create only when readiness exists + status == Ready + decision == Ready. Copies all fields from readiness (not recomputed). Ref update is fast-forward-only. `readiness_hash` from persisted record.

### Review

- **Approved**: non-empty reviewer + rationale. No execution grant.
- **Rejected**: non-empty reviewer + rationale + feedback with blocking_reasons.
- **ChangesRequested**: non-empty reviewer + rationale + feedback with requested_changes.

All hardcode `creates_execution_grant: false` and `execution_allowed_now: false`.

## Persistence

```
eval_reports/remote_push_proposals/
  proposals/<proposal_id>.json, latest.json, by_readiness/<id>.json
  reviews/<review_id>.json, latest.json, by_proposal/<id>.json
  feedback/<review_id>.json
```

## CLI

```
openwand eval auto-commit push-proposal create/show/latest
openwand eval auto-commit push-proposal review approve/reject/request-changes/show-review/latest-review
```

Feature-gated behind `real-model-eval`. No `push` command exposed.

## Test Coverage (61 tests)

- **DTO** (8): roundtrips, content-addressed, deterministic, differs
- **Proposal** (10): create, copy, fast-forward, blocks missing/blocked/inconclusive, hash changes, readiness hash
- **Review** (11): approval/rejection/changes validation, no execution grant, latest supersedes, audit preserved
- **Feedback** (6): export, blocking_reasons, requested_changes, roundtrips
- **Idempotency** (3): same key returns existing, different key preserves history
- **Persistence** (8): proposal/review/feedback roundtrips, latest, by_readiness/by_proposal, sorted
- **CLI** (8): create, show, latest, approve, reject, changes, review latest, no push
- **Guards** (8): no push/fetch/pull/ls-remote/tags/branches/shell/process/push_backend

## Honest Caveats

- Wave 16 creates and reviews a push proposal. It does not push.
- Human approval is review evidence only — no execution grant, no bypass.
- Stale remote-tracking refs remain a known limitation from Wave 15.
- Future waves may add governed push execution, tagging, release, post-push verification, or live rollback.
