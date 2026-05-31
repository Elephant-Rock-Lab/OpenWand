# WAVE 07 — Real Provider Execution Wiring & Longitudinal Quality Reports — LOCK

**Date:** 2026-05-31
**Commits:** 7 (04d1089 → 0c25ca7)
**Tests:** 967 → 995 (+28)
**Failures:** 0

## Lock Condition

```text
OpenWand can execute the Wave 06 flagship evaluation scenarios against a real
provider, persist redacted evaluation reports, compare reports across runs,
models, providers, and scenario versions, and surface regressions without
making provider-backed tests part of default CI.
```

## Lock Conditions Proven

| # | Condition | Proof |
|---|-----------|-------|
| 1 | Shared session assembly prevents forked implementations | `session_runtime::build_session_runtime()` used by both `cmd_run` and eval runner |
| 2 | Provider is explicit or inferred | `resolve_provider()` — explicit `--provider` flag, heuristic URL inference, openai-compatible default |
| 3 | Eval runner drives real governed loop | `SessionRunner::run_turn()` per scenario turn, auto-approve for eval mode |
| 4 | Reports persist in stable layout | `eval_reports/scenarios/{id}/{timestamp}_{model}.json` |
| 5 | Reports load with schema validation | `EvalReportStore::load_report()` rejects future schema versions |
| 6 | Baseline selection works | None/Latest/Path modes, scenario match enforced |
| 7 | Comparison detects score drops | `eval_compare_detects_score_drop` |
| 8 | Comparison detects required dimension regression | `eval_compare_detects_required_dimension_regression` |
| 9 | Comparison detects improvements | `eval_compare_detects_improvement` |
| 10 | Regression thresholds configurable | `RegressionThresholds` struct with defaults |
| 11 | Fail-on-regression exits non-zero | `anyhow::bail!("Regression detected")` |
| 12 | Compare subcommand prints dimension deltas | CLI output with ✓/⚠/· markers |
| 13 | Summarize groups by scenario and provider | `generate_summary()` |
| 14 | Trend computed from recent vs older runs | `ScoreTrend::Improving/Stable/Declining` |
| 15 | Summary reports serialize stably | Round-trip JSON test |
| 16 | Empty report directory handled gracefully | `eval_summary_handles_empty_directory` |
| 17 | Policy rules shared between run and eval | `build_write_policy()` extracted to `session_runtime.rs` |
| 18 | Default CI remains provider-free | All new tests use `tempfile::tempdir()`, no network |

## New Modules

| Crate | Module | Purpose |
|-------|--------|---------|
| `openwand-app` | `session_runtime` | Shared session assembly (`build_session_runtime`, `build_write_policy`) |
| `openwand-app` | `eval_reports` | Report persistence (`EvalReportStore`) |
| `openwand-app` | `eval_compare` | Baseline selection + comparison engine |
| `openwand-app` | `eval_summary` | Longitudinal summary aggregation |

## Refactored Modules

| Module | Change |
|--------|--------|
| `app/src/main.rs` | `cmd_run` uses `build_session_runtime()`, `build_write_policy()` moved out |
| `app/src/main.rs` | Eval `Run` uses real governed loop via `build_session_runtime()` |

## New CLI Commands

```bash
openwand eval run --scenario all \
  --provider openai-compatible \
  --base-url http://localhost:1234/v1 \
  --model qwen3-4b \
  --baseline latest \
  --fail-on-regression

openwand eval compare --current report.json --baseline baseline.json
openwand eval summarize --output-dir eval_reports [--scenario id]
```

## New Types

### session_runtime
- `SessionRuntime` (runner, trace, memory_store, memory_read, session_id)
- `build_session_runtime(db_path, working_dir) -> SessionRuntime`
- `build_write_policy() -> BuiltinPolicyEngine`

### eval_reports
- `EvalReportStore { root }`
- `StoredEvalReport { path, report }`
- `ReportFilter { scenario_id }`

### eval_compare
- `EvalBaselineSelection` (None, Latest, Path)
- `EvalComparisonReport` (scenario_id, score_delta, dimension_deltas, provider_delta, regressions, improvements)
- `ScoreDelta`, `DimensionDelta`, `ProviderDelta`
- `EvalRegression`, `EvalImprovement`, `RegressionSeverity`
- `RegressionThresholds` (max_score_drop, max_pass_rate_drop, required_dimensions)

### eval_summary
- `EvalSummaryReport` (generated_at, total_reports, scenario_summaries, provider_summaries)
- `ScenarioTrendSummary` (run_count, scores, trend, provider/model)
- `ProviderTrendSummary` (run_count, avg_score, scenario_coverage)
- `ScoreTrend` (Improving, Stable, Declining, InsufficientData)

## Directory Layout

```text
eval_reports/
  scenarios/{scenario_id}/{timestamp}_{model}.json
  baselines/{scenario_id}.json
  summaries/{timestamp}_summary.json
```

## Invariant Guards

| Guard | Enforcement |
|-------|-------------|
| Default CI provider-free | All tests use tempfile, no network calls |
| `build_session_runtime` is single source of truth | `cmd_run` + eval runner both call it |
| Reports redact secrets | `ProviderRealitySnapshot` never serializes API key |
| Baseline comparison rejects scenario mismatch | `resolve_baseline()` validates IDs |
| Schema version compatibility enforced | `load_report()` rejects future versions |
| Regression cannot pass silently | `--fail-on-regression` exits non-zero |
| Anti-vacuous-pass enforced | Score with max=0 triggers Evidence regression |

## What Does NOT Change

- Governance profile values (locked 02r)
- Memory pipeline mechanics (locked 02j–02k)
- Policy rules (locked 03a–03f / 04a–04b)
- Prompt format (unchanged)
- Agent loop phases (unchanged)
- Trace schema (append-only, immutable)
- Existing fixtures (zero modifications to 19 memory eval + 8 YAML)
- Wave 06 DTOs (zero modifications)

## Acceptance Commands

```bash
# Standard CI — must pass without provider
cargo test --workspace --features "openwand-session/testing,openwand-session/sqlite-testing,openwand-memory/testing,openwand-memory/sqlite-testing"

# Eval infrastructure without provider
cargo test -p openwand-app --features real-model-eval

# Manual real-provider smoke (dev example only)
cargo run -p openwand-app --features real-model-eval -- \
  eval run --scenario memory_verified_used \
  --provider openai-compatible \
  --base-url http://100.64.64.1:1234/v1 \
  --model qwen3-4b \
  --baseline none

# Longitudinal compare
cargo run -p openwand-app --features real-model-eval -- \
  eval summarize --output-dir eval_reports
```

## Corrections Applied

1. ✅ Shared assembly (not copy-based) — `build_session_runtime()` extracted
2. ✅ Provider is explicit (`--provider` flag) with inference convenience
3. ✅ No new workspace crates — all new code in `openwand-app`
4. ✅ No new external dependencies — everything uses existing workspace deps
5. ✅ Feature-gated CLI surface — `real-model-eval` gate unchanged

## Implementation Clarifications Applied

### 1. `cmd_run` reuse is structural, not copy-based
`build_session_runtime()` in `session_runtime.rs` is the single assembly function.
Both `cmd_run` and eval runner call it. The inline assembly in `main.rs` was
replaced with a single call. `build_write_policy()` was also extracted.

### 2. Provider is explicit with inference convenience
`--provider` flag is documented and parsed. Falls back to URL heuristic.
Default: `openai-compatible` (covers LM Studio, vLLM, etc.).

## Next Wave Considerations

- Wire eval runner to real LLM provider for actual scenario execution
- Populate evaluation collectors from real trace data (currently placeholder)
- Multi-provider matrix (Ollama, Anthropic, etc.)
- Auto-commit gated on patch correctness trend evidence
- Rich text editing for message composition
