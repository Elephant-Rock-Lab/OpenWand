# WAVE 11 — Governed Auto-Commit Proposal, Not Execution — LOCK

**Date:** 2026-05-31
**Commits:** 1 batch (d78e9cf)
**Tests:** 1072 → 1110 (+38)
**Failures:** 0

## Lock Condition

```
OpenWand can generate, persist, display, and audit a governed auto-commit
proposal derived from readiness/eval/patch evidence, while proving that no git
commit, staging, push, tag, or workspace mutation occurs.
```

## What Shipped

- `AutoCommitProposal` DTO with content-addressed ID (BLAKE3)
- `build_auto_commit_proposal()` builder consuming shared-borrow evidence
- Deterministic commit message template (no LLM generation)
- Persistence: save/load/list with supersession
- CLI: `openwand eval auto-commit propose` and `show` subcommands
- Governance summary with hardcoded `execution_allowed_now = false`
- 38 new tests (30 proposal + 8 guard)
- Forbidden completion phrases guard

## What Explicitly Did Not Ship

- git commit execution
- git add/staging
- Branch creation, tagging, push
- Automatic approval
- Tool executor or shell executor integration
- Any "convenience" helper that executes the proposal

## Corrections Applied

| # | Correction | Resolution |
|---|-----------|------------|
| 1 | BLAKE3 content-addressed ID, not ULID | `proposal_id_for()` uses BLAKE3 hash of `readiness_id:workspace_hash` |
| 2 | Supersession in save, not load | `save_proposal()` marks old proposals; all `load_*` are read-only |
| 3 | Rename "Trace Wiring" to "Governance Summary Wiring" | Commit 6 renamed; no trace events appended |
| 4 | Output dir outside temp git repo | Guard test uses separate `tempfile::tempdir()` for output |
| 5 | CLI uses subcommand enum | `AutoCommitCommands` with `Propose` and `Show` variants |

## Execution Guard Proof

Source guards: `eval_proposal_guards.rs` scans module source for:
- No `std::process` imports
- No `git_commit`, `git_add`, `git_push`, `git_tag` code references
- No `ToolExecutor` or `Shell` imports

Runtime guard: `proposal_generation_leaves_git_head_index_and_worktree_unchanged`
- Creates temp git repo with committed file
- Creates **separate** output directory outside repo (Correction #4)
- Generates proposal, saves to separate dir
- Asserts `.git/HEAD`, `.git/index`, and `git status --porcelain` unchanged

## Non-Negotiable Invariant

```
Wave 10 answers: "Is this patch eligible for auto-commit consideration?"
Wave 11 answers: "What exactly would be proposed as a commit?"
Wave 11 must never answer: "The commit was made."
```

## Test Count

| File | Tests |
|------|-------|
| eval_proposal.rs | 30 |
| eval_proposal_guards.rs | 8 |
| **Total** | **1110** |

## Honest Caveats

None. The proposal module has zero git/process/tool-execution imports. The `execution_allowed_now` field is hardcoded `false`. The runtime guard proves git state is byte-identical before and after proposal generation.
