# WAVE 05 — Productized Governed Agent Loop — LOCK

**Date:** 2026-05-31
**Commits:** 9 (db390fb → 5b55aa3)
**Tests:** 897 → 935 (+38)
**Failures:** 0

## Lock Conditions

| # | Condition | Proof |
|---|-----------|-------|
| 1 | File patch tool with plan/apply modes | `file_patch_plan_validates_preimage`, `file_patch_apply_creates_rollback`, `e2e_file_patch_plan_then_apply` |
| 2 | BLAKE3 content hashing for all file operations | `blake3_hash_is_deterministic`, preimage/postimage in write and patch output |
| 3 | Preimage hash + rollback on file_write overwrite | `write_tool_records_preimage_when_overwriting`, `write_tool_creates_rollback_on_overwrite` |
| 4 | Task context captures git pre/post state | `before_task_on_non_git_dir_produces_none`, `compute_changed_files_*` |
| 5 | Explain renders from governed report, not raw store | `panel_view_renders_from_coordinator_output_not_raw_store`, `explain_renders_*` |
| 6 | Memory regression CI feature gate | `cargo test -p openwand-app --features memory-regression` |
| 7 | Prompt-panel equivalence invariant | `panel_view_has_no_store_dependency`, `panel_view_renders_from_coordinator_output_not_raw_store` |
| 8 | Session rebuildable from trace | `rebuild_empty_session`, `rebuild_replays_single_event`, `rebuild_detects_divergence`, `idempotent_replay_same_result`, `rebuild_multiple_events_in_order` |
| 9 | E2E flagship: governance + explain + patch + rebuild | `e2e_governance_includes_verified_excludes_low`, `e2e_explain_renders_complete_explanation`, `e2e_file_patch_plan_then_apply`, `e2e_session_rebuildable_from_trace` |

## New Modules

| Crate | Module | Purpose |
|-------|--------|---------|
| `openwand-tools` | `file_patch` | Plan/apply file patching with BLAKE3 hashes + rollback |
| `openwand-session` | `task_context` | Git pre/post observation around task execution |
| `openwand-session` | `rebuild` | Session state reconstruction from trace stream |
| `openwand-app` | `explain` | Trust explanation rendering (memory, policy, execution, completion) |

## New Types

- `TaskContext` — captures git status before task, computes `TaskSummary` after
- `TaskSummary` — changed_files, diff_stat, completed, test_output
- `Explanation` — memory, policy, execution, completion sections
- `MemoryExplanation` / `PolicyExplanation` / `ExecutionExplanation` / `CompletionExplanation`
- `RebuildResult` — events_replayed, state_matches, divergences
- `ClaimEntry` / `ExcludedClaimEntry` / `GateEntry` / `ApprovalEntry` / `ToolCallEntry`

## Corrections Applied

1. **Correction 1**: Deterministic LLM provider wording (deferred to real multi-turn quality eval)
2. **Correction 2**: Trace-first ownership stays in session crate (already enforced)
3. **Correction 3**: BLAKE3 for content hashing — `blake3_content_hash()` in file_patch, `blake3::hash` in file_write
4. **Correction 4**: Plan/apply split in ONE tool with mode field, not two tools
5. **Correction 5**: Patch risk = Medium/Approve (same as file_write), not Critical
6. **Correction 6**: Architecture guard: `panel_view_has_no_store_dependency` proves no raw store access

## Invariant Guards

| Guard | Test |
|-------|------|
| No raw memory query in explain | `panel_view_renders_from_coordinator_output_not_raw_store` |
| Explain uses same data model consumed | `MemoryExplanation::from_governed_report` |
| File mutation requires satisfied policy | Policy crate rule: `ToolEffect::Write` → Medium + Approve |
| No direct file mutation outside governed executor | Trace-first invariant enforced in session runner |
| Memory regression is CI-friendly | `memory-regression` feature gate |
| Governance profile is a dial | `MemoryGovernanceProfileId::Default` vs `Batch02rDefault` |
| Session rebuildable from trace | `rebuild_*` tests + `e2e_session_rebuildable_from_trace` |

## Dependency Guards (Commit 9)

- `batch2_registers_five_tools_including_write_and_patch` — verifies tool count
- `write_tool_records_preimage_when_overwriting` — preimage in output
- `write_tool_creates_rollback_on_overwrite` — rollback material exists
- `file_patch_rejects_out_of_workspace` — path boundary enforcement
- `file_patch_rejects_preimage_mismatch` — preimage validation

## What Does NOT Change

| Item | Why |
|------|-----|
| Governance profile values | Locked in 02r |
| Memory pipeline mechanics | Locked in 02j-02k |
| Policy rules | Locked in 03a-03f, 04a-04b |
| Prompt format | Same section structure |
| Agent loop phases | Same 10-phase lifecycle |
| Trace schema | Append-only, immutable |
| Memory evaluation fixtures | Zero modifications |

## Test Count by Commit

| Commit | Description | Tests |
|--------|-------------|-------|
| 1 | CLI subcommands + E2E fixture | 897 |
| 2 | file_patch tool (plan/apply) | 902 |
| 3 | file_write preimage + rollback | 905 |
| 4 | TaskContext git pre/post | 914 |
| 5 | Explain model + rendering | 923 |
| 6 | Memory regression CI + equivalence | 930 |
| 7 | Session rebuild from trace | 930 (+5 in session integration) |
| 8 | E2E flagship scenario | 935 |
| 9 | This lock doc + guards | 935 |

## Next Wave Considerations

- Real multi-turn LLM quality evaluation with local Qwen3
- Rich text editing (taino-edit-dioxus) for message composition
- Multi-provider support (beyond OpenAI-compatible)
- Auto-commit (currently observation only)
- Full unified diff parsing (not just simple hunks)
- Evaluation gates as pipeline citizens
