# Wave 13: Governed Auto-Commit Execution Gate — Lock

**Commit:** `b4fbd21`  
**Date:** 2026-06-02  
**Status:** LOCKED  
**Tests:** 1192 total, zero failures

## Scope

Execute exactly one previously approved proposal as a single local git commit, after revalidating all predicates at execution time. No push, tag, branch, checkout, reset, merge, rebase, or remote expansion.

## Module Boundary

| Wave | Module | Responsibility |
|------|--------|---------------|
| 11 | `eval_proposal.rs` | Proposal generation |
| 12 | `eval_proposal_review.rs` | Review and feedback |
| **13** | **`eval_proposal_execution.rs`** | **Execution gate and local commit record** |

## Architecture

### Execution Flow

```
1. Load proposal + review from persistence
2. Observe current git state (HEAD, branch, index, worktree)
3. Evaluate 17 predicates atomically
4. If all pass → execute commit via backend
5. If any fail → persist blocked record, no git mutation
6. Persist execution record
```

### GovernedGitCommitBackend Trait

```rust
pub trait GovernedGitCommitBackend {
    fn observe_state(&self, repo: &Path) -> Result<GitStateSnapshot, GitExecutionError>;
    fn create_commit_exact(&self, repo: &Path, request: ExactCommitRequest) -> Result<GitCommitSnapshot, GitExecutionError>;
}
```

Only two methods. No push, tag, branch, checkout, reset, merge, rebase, run_git, run_shell, or stage_all.

### Correction #1: Command Restriction

`std::process::Command` is used **ONLY** inside `LocalGitBackend`. Guard tests verify:
- Command appears only in LocalGitBackend code
- Fixed binary `"git"` — never dynamic
- No `.shell()`, `/bin/sh`, `cmd.exe` invocation
- No `git push`, `git tag`, `git branch` creation

### Correction #2: Exact Approved-Path Staging

Commit sequence:
1. `git add -- <exact approved files>` (not broad staging)
2. Re-check index hash matches expected
3. `git commit -F <msgfile>` with deterministic message
4. Observe resulting commit hash

Guard tests verify:
- Only proposal-listed paths are staged
- No unreviewed files are staged

## Execution Predicates (17)

| # | Predicate | Description |
|---|-----------|-------------|
| 1 | ProposalExists | Proposal file found |
| 2 | ProposalEligible | Status == Eligible |
| 3 | ReviewExists | Review file found |
| 4 | ReviewIsLatestForProposal | No newer review supersedes |
| 5 | ReviewApproved | Decision == Approved |
| 6 | ReviewProposalHashMatchesProposal | Proposal hash from review matches computed |
| 7 | ReviewWorkspaceHashMatchesProposal | Workspace hash from review matches proposal |
| 8 | CurrentWorkspaceHashMatchesReview | Current workspace matches review's snapshot |
| 9 | CurrentProposalHashMatchesReview | Current proposal hash matches review's copy |
| 10 | PolicyAllowsGitCommit | External policy evaluation passes |
| 11 | RollbackPlanExists | Pre-commit state captured |
| 12 | GitHeadMatchesExpected | HEAD hasn't drifted |
| 13 | GitBranchMatchesExpected | Branch hasn't changed |
| 14 | GitIndexMatchesExpected | Index matches expected state |
| 15 | GitWorktreeMatchesExpected | Worktree matches expected state |
| 16 | CommitMessageMatchesProposal | Message is from proposal |
| 17 | IdempotencyKeyUnused | No conflicting prior execution |

**All predicates must pass in the same execution attempt.** A single failure blocks execution.

## Key DTOs

- `AutoCommitExecutionId` — BLAKE3 content-addressed (`aex_<hex>`)
- `AutoCommitExecutionRequest` — proposal/review/key + metadata
- `ExecutionGateDecision` — Allow | Block { reason_code, summary }
- `AutoCommitExecutionRecord` — full record with status, decision, predicates, resulting commit
- `AutoCommitExecutionStatus` — Blocked | Executed | AlreadyExecuted
- `ExactCommitRequest` — narrow input for backend (message, file_paths, expected hashes)
- `RollbackPlanSnapshot` — pre-commit HEAD, branch, index, worktree, recovery command
- `GitCommitSnapshot` — resulting commit hash, parent, branch, message hash

## Persistence

Records stored under `<store_root>/proposal_executions/`:
- `<execution_id>.json` — individual record
- `latest.json` — most recent execution
- `by_proposal/<proposal_id>.json` — latest execution per proposal

## Idempotency

Same `(proposal_id, idempotency_key)` → returns existing `Executed` record without re-executing. Prevents double commits.

## CLI Commands

```
openwand eval auto-commit execute --proposal-id <id> --review-id <id> [--idempotency-key <key>]
openwand eval auto-commit execution show <execution-id>
openwand eval auto-commit execution latest [--proposal-id <id>]
```

All execution commands feature-gated behind `real-model-eval`.

## Test Coverage (40 tests)

### Commit 1: DTO and Builder (10 tests)
- Execution request roundtrip serialization
- Execution record roundtrip serialization
- Decision requires predicates
- Executed record requires commit snapshot
- Blocked record must not have commit snapshot
- Rollback plan requires pre-commit HEAD
- Rollback plan requires recovery command
- Execution ID is content-addressed
- Execution ID deterministic
- Execution ID different for different inputs

### Commit 2: Predicate Evaluation (9 tests)
- Blocks missing proposal
- Blocks missing review
- Blocks non-latest review
- Blocks rejected review
- Blocks requested-changes review
- Blocks workspace hash drift
- Blocks proposal hash drift
- All predicates pass for valid state
- Predicate results include reasons

### Commit 3: Policy and Rollback (5 tests)
- Policy gate called for git effect (typo fixed: calle → called)
- Policy failure blocks execution
- Missing rollback plan blocks execution
- Rollback plan captures pre-commit state
- Rollback plan includes recovery command

### Commit 4: Backend and Runtime (4 tests)
- Test backend observe state
- Test backend commit exact creates commit
- Happy path executes commit
- Blocked execution leaves git HEAD/index/worktree unchanged (runtime guard)

### Correction #1: Command Guards (4 tests)
- Command only used inside LocalGitBackend
- LocalGitBackend uses fixed git binary
- LocalGitBackend never invokes shell
- Module does not push/tag/branch

### Correction #2: Staging Guards (2 tests)
- Stages only approved paths
- Does not stage unreviewed file

### Commit 5: Persistence and Idempotency (6 tests)
- Execution persists and loads roundtrip
- Same idempotency key returns existing record
- Execution record links proposal and review
- Latest execution returns expected
- Latest execution for proposal returns expected
- List execution records returns sorted by date

### CLI Surface (4 tests)
- cli_execute_blocked_outputs_predicates: blocked record carries predicate list for CLI display
- cli_execute_blocked_prints_no_commit_executed: blocked record has no resulting_commit; CLI won't falsely print commit hash
- cli_execution_show_roundtrips_record: load by execution_id roundtrips all fields including resulting_commit
- cli_execution_latest_returns_latest: unfiltered latest returns newest; proposal-filtered returns correct per-proposal record

## Doctrines Enforced

1. **All predicates pass in the same execution attempt** — no partial execution
2. **Blocked execution leaves no trace in git** — runtime guard test proves HEAD/index/worktree unchanged
3. **Approval record is not an execution grant** — from Wave 12, `creates_execution_grant: false`
4. **Execution idempotency** — same key returns same result, no double commits
5. **Rollback plan mandatory** — pre-commit state captured before any git mutation
6. **Proposal is observational-only** — from Wave 11, execution is a separate phase
7. **Command restricted to backend** — Correction #1, verified by guard tests
8. **Exact approved-path staging** — Correction #2, verified by guard tests
