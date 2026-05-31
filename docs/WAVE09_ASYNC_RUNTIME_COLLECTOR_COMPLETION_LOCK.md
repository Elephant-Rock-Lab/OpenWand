# WAVE 09 — Async Runtime Collector Completion — LOCK

**Date:** 2026-05-31
**Commits:** 3 (6f16212 → aef288a)
**Tests:** 1032 → 1039 (+7)
**Failures:** 0

## Lock Condition

```text
OpenWand eval runner populates prompt, memory, tool, policy, patch, explain,
and rebuild dimensions from real runtime execution paths, including async
memory coordination, explanation rendering, and trace replay, with no
placeholder dimension results remaining.
```

## Lock Conditions Proven

| # | Condition | Proof |
|---|-----------|-------|
| 1 | Memory uses coordinator/governed report | `eval_runner_memory_uses_coordinator_report_not_memory_read_search` |
| 2 | Memory fails without governed report | `eval_runner_memory_dimension_fails_without_governed_report` |
| 3 | Explain uses existing explain rendering | `eval_runner_explain_uses_existing_explain_rendering_path` |
| 4 | Rebuild uses rebuild_session API | `eval_runner_rebuild_uses_rebuild_session_result` |
| 5 | All dimensions have evidence refs | DimensionScore construction in main.rs |
| 6 | No placeholder dimensions remain | `eval_guard_no_placeholder_dimensions_in_production_report` |
| 7 | Same-session evidence guard | `eval_runner_rejects_mismatched_session_evidence` |
| 8 | Empty trace produces non-passing results | 9 guard tests |
| 9 | Default CI remains provider-free | `--features real-model-eval` gate unchanged |

## Changes

### Commit 1 — Memory + Explain + Rebuild Runtime Wiring (6f16212)
- `crates/app/src/main.rs`: Replaced placeholder `MemoryEvalResult`, `ExplainEvalResult`, `RebuildEvalResult`
  - Memory: `MemoryCoordinator::project_after_run()` + `produce_prompt_inputs()` → `GovernanceFilteredReport::from_report()` → `collect_memory_eval()`
  - Explain: `Explanation { memory: MemoryExplanation::from_governed_report() }` → `collect_explain_eval()`
  - Rebuild: `rebuild_session(rt.trace, session_id, Some(loro_state()), converter)` → `collect_rebuild_eval()`
- New helper: `make_empty_governed_report(working_dir)` for fallback cases
- New test file: `crates/app/tests/eval_runtime_wiring.rs` (5 tests)

### Commit 2 — Evidence-Linked Dimension Scoring (d3179ed)
- `crates/app/src/main.rs`: All 7 dimensions built with `DimensionScore` + `evidence_refs`
  - prompt: `EvalEvidenceSource::Trace`, `inference.called` event
  - tool: up to 3 trace event refs
  - policy: up to 3 gate event refs
  - patch: up to 2 file event refs
  - memory: `EvalEvidenceSource::GovernedReport`, claim counts
  - explain: `EvalEvidenceSource::Explanation`, section match flags
  - rebuild: `EvalEvidenceSource::Rebuild`, events replayed + state_matches
- Score built BEFORE report construction, used directly

### Commit 3 — Placeholder Elimination Guards (aef288a)
- `crates/app/tests/eval_collector_guards.rs`: 2 new guards (7 → 9 total)
  - `eval_guard_no_placeholder_dimensions_in_production_report`
  - `eval_guard_rebuild_from_empty_works`

## Test Count

| File | Tests |
|------|-------|
| eval_runtime_wiring.rs | 5 (new) |
| eval_collector_guards.rs | 9 (7 existing + 2 new) |
| **Total** | **1039** |

## Honest Caveats

None. All 7 dimensions now use real runtime paths. No placeholders remain.

## Architecture Notes

- Memory collector wiring: `coordinator.produce_prompt_inputs()` produces `PromptInputResult` containing `RepoConsistencyReport`. We derive `GovernanceFilteredReport` via `from_report(&report, &profile, &[])` — the same path as `GovernanceFilteredReport::from_report()` used throughout the codebase.
- Explain collector wiring: `Explanation` struct composed with same fields as `openwand explain` command. The `collect_explain_eval()` function is shared between CLI and eval runner.
- Rebuild collector wiring: `rebuild_session()` called with `rt.runner.loro_state()` — same `&LoroSessionState` type used by production rebuild tests. Generic converter `|e| e.0.clone()` transforms `StoredEvent` to `serde_json::Value`.
