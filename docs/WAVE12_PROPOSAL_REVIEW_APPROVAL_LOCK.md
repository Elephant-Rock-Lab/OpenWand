# WAVE 12 — Proposal Review, Approval Record, and Rejection Feedback — LOCK

**Date:** 2026-06-01
**Commits:** 1 batch (82b00e4)
**Tests:** 1110 → 1152 (+42)
**Failures:** 0

## Lock Condition

```
OpenWand can review a governed auto-commit proposal, persist an
approval/rejection/change-request record tied to the exact proposal and
workspace snapshot, expose that decision through CLI, generate structured
rejection feedback for the eval/patch loop, and prove that review does not
execute git commit, staging, push, tag, or workspace mutation.
```

## Non-Negotiable Invariant

```
Wave 11 answers: "What exactly would be proposed as a commit?"
Wave 12 answers: "Was that proposal approved, rejected, or returned for changes — and why?"
Wave 12 must never answer: "The commit was made."

An approval record is not an execution grant.
An approval record is not a policy override.
An approval record is not a git operation.
```

## What Shipped

- `AutoCommitProposalReview` DTO with content-addressed ID
- `build_proposal_review()` builder with validation rules
- Structured rejection feedback (`ProposalRejectionFeedback`)
- Persistence with supersession semantics
- Feedback export for rejected/change-requested reviews
- CLI review subcommands (approve/reject/request-changes/show/latest)
- 42 new tests (33 review + 9 guard)

## What Explicitly Did Not Ship

- git commit, staging, push, tag, branch creation
- Execution grant creation
- Policy mutation
- Automatic approval
- LLM-generated review decisions
- Tool/shell executor integration

## Corrections Applied

| # | Correction | Resolution |
|---|-----------|------------|
| 1 | Hashes copied, not validated | `workspace_hash` and `proposal_hash` copied from proposal; revalidation deferred to Wave 13 |
| 2 | Rejected requires feedback | Builder rejects `Rejected` without `feedback`; CLI `--feedback` required for reject |

## Execution Guard Proof

Source guards: `eval_proposal_review_guards.rs` scans module for:
- No `std::process`, `git_commit`, `git_add`, `git_push`, `git_tag`
- No `ToolExecutor`, `Shell`, `Command` imports
- `creates_execution_grant` and `execution_allowed_now` never set to `true`

Runtime guard: `proposal_review_leaves_git_head_index_and_worktree_unchanged`
- Temp git repo with committed file
- Separate output directory outside repo
- HEAD, index, `git status --porcelain` unchanged after review

## Module Boundary

```
Wave 11: eval_proposal.rs          → proposal generation
Wave 12: eval_proposal_review.rs   → proposal review and feedback
```

## Test Count

| File | Tests |
|------|-------|
| eval_proposal_review.rs | 33 |
| eval_proposal_review_guards.rs | 9 |
| **Total** | **1152** |

## Honest Caveats

None. Both hardcoded invariants (`execution_allowed_now = false`, `creates_execution_grant = false`) are verified by guard tests. The runtime git-state guard proves byte-identical HEAD/index/worktree.

## Future Seam (Wave 13)

Wave 13 may consume:
- Eligible proposal + latest Approved review
- Review workspace_hash == current workspace_hash
- Review proposal_hash == current proposal_hash
- Execution policy gate satisfied
- Rollback plan exists

Wave 12 does not satisfy or create that execution gate.
