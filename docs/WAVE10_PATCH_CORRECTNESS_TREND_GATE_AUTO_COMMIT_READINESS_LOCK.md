# WAVE 10 — Patch Correctness Trend Gate & Auto-Commit Readiness — LOCK

**Date:** 2026-05-31
**Commits:** 3 (ef8dffe → f30e935)
**Tests:** 1039 → 1072 (+33)
**Failures:** 0

## Lock Condition

```
OpenWand can compute auto-commit readiness from longitudinal, trace-backed
patch-evaluation reports, using configurable thresholds across patch, policy,
rollback, explain, and rebuild dimensions, and can block auto-commit eligibility
unless the required quality trend is satisfied.
```

## Lock Conditions Proven

| # | Condition | Proof |
|---|-----------|-------|
| 1 | Readiness computable from stored eval reports | `compute_auto_commit_readiness()` + 29 tests |
| 2 | Required scenarios have configurable evidence thresholds | `AutoCommitReadinessThresholds` + `auto_commit_threshold_defaults_are_conservative` |
| 3 | Missing reports → InsufficientEvidence | `readiness_insufficient_when_required_scenario_missing` |
| 4 | Dimension threshold failures → Blocked | 6 blocker tests |
| 5 | Missing rollback/unexpected files → Blocked | 5 patch blocker tests |
| 6 | Regression detection feeds readiness blocking | 4 regression tests |
| 7 | Readiness reports persisted as JSON | `readiness_store_saves_and_loads` |
| 8 | CLI exposes readiness without mutations | Guard tests + feature gate |
| 9 | Default CI remains provider-free | `--features real-model-eval` gate unchanged |

## Changes

### Commit 1 — Readiness DTOs, Decision Engine, Persistence (ef8dffe)
- New module: `crates/app/src/eval_readiness.rs`
- Types: `ReadinessTarget`, `AutoCommitReadinessStatus`, `AutoCommitReadinessReport`, `AutoCommitReadinessThresholds`, `ReadinessScore`, `ReadinessBlocker`, `ReadinessWarning`, `PatchTrendSummary`, `ScenarioSpec`, `EvidenceWindow`, `ScenarioReadinessResult`
- Decision engine: `compute_auto_commit_readiness()` — 6-step hierarchy
- Persistence: `save_readiness_report()` + `load_latest_readiness_report()`
- 25 tests in `crates/app/tests/eval_readiness.rs`

### Commit 2 — Regression Integration (b4f5244)
- Integrated `eval_compare::compare_reports()` into readiness computation
- Chronological adjacent-pair comparison per scenario
- Regressions counted across required dimensions
- 4 new regression tests

### Commit 3 — CLI + Guard Tests (f30e935)
- CLI: `openwand eval readiness --target auto-commit`
- Feature-gated behind `real-model-eval`
- 4 guard tests including workspace snapshot guard (Clarification #4)

## Clarifications Resolved

| # | Clarification | Resolution |
|---|--------------|------------|
| 1 | PatchEvalResult retained in stored reports | Option A: full struct is `Serialize, Deserialize` in `EvalRunReport` |
| 2 | v1/v2 report compatibility | `AllReportsIncompatible` → InsufficientEvidence; `SkippedIncompatibleReport` warning |
| 3 | Scenario-aware plan/apply | `ScenarioPatchExpectation` enum; PlanAndApply scenarios block on planned&&!applied |
| 4 | Runtime workspace snapshot guard | `readiness_guard_persistence_writes_only_readiness_dir` with walkdir |

## Test Count

| File | Tests |
|------|-------|
| eval_readiness.rs | 29 |
| eval_readiness_guards.rs | 4 |
| **Total** | **1072** |

## Honest Caveats

None. Readiness is purely observational. No auto-commit execution. No mutation privilege expansion.

## Architecture Notes

- `compute_auto_commit_readiness()` takes `&[EvalRunReport]` (shared borrow). It cannot mutate the input.
- Persistence writes only to `{root}/readiness/` directory. Workspace snapshot guard proves this.
- Regression detection reuses `eval_compare::compare_reports()` with strict thresholds (max_score_drop=0, max_pass_rate_drop=0).
- Default thresholds are intentionally strict: policy and rebuild require 100% pass rate, patch requires 95%, explain requires 90%.
- The readiness module has zero git/process/tool-execution dependencies.
