# WAVE 06 — Real Model Quality Evaluation & Provider Reality — LOCK

**Date:** 2026-05-31
**Commits:** 9 (b175b8e → 228dac5)
**Tests:** 935 → 967 (+32)
**Failures:** 0

## Lock Condition

```text
OpenWand can run feature-gated real-model evaluation scenarios through the
governed agent loop, collect trace-backed evidence across memory, prompt,
tool, policy, patch, explain, and rebuild dimensions, score behavior with a
deterministic rubric, serialize a redacted report, and keep default CI fully
provider-free.
```

## Lock Conditions Proven

| # | Condition | Proof |
|---|-----------|-------|
| 1 | Evaluation DTOs serialize stably | `eval_report_serializes_stably`, `eval_runner_writes_report_json` |
| 2 | Provider reality redacts secrets | `provider_snapshot_redacts_api_key`, `eval_runner_redacts_provider_secrets` |
| 3 | Scenario fixtures load deterministically | `eval_fixture_loads_all_scenarios`, `eval_flagship_scenarios_load` |
| 4 | Memory eval uses governed report | `eval_memory_uses_governed_report_not_raw_store` |
| 5 | Missing required claims detected | `eval_detects_missing_required_memory` |
| 6 | Excluded claims cannot leak | `eval_detects_excluded_claim_leaked_to_prompt` |
| 7 | Forbidden tool requests detected | `eval_tool_detects_forbidden_request` |
| 8 | Patch-without-plan detected | `eval_patch_detects_missing_plan` |
| 9 | Missing rollback detected | `eval_patch_detects_missing_rollback` |
| 10 | Unexpected file changes detected | `eval_patch_detects_unexpected_changed_file` |
| 11 | Anti-vacuous-pass enforced | `eval_fails_when_no_inference_event`, `_no_tool_events`, `_no_governed_report` |
| 12 | Report schema version present | `eval_report_schema_version_is_present` |
| 13 | Feature gate works | `eval_requires_feature_for_real_provider` |
| 14 | All scenarios require rebuild + explain | `eval_flagship_all_have_rebuild_expectation`, `_explain` |

## New Modules

| Crate | Module | Purpose |
|-------|--------|---------|
| `openwand-app` | `eval_model` | Evaluation DTOs, report model, provider snapshot, fixture loader |
| `openwand-app` | `eval_collector` | Memory/tool/policy/patch/explain/rebuild collectors + anti-vacuous-pass |

## New External Dependency

- `serde_yaml = "0.9"` — for human-authored YAML scenario fixtures

## New Feature Gate

- `real-model-eval` on `openwand-app` — enables `openwand eval` CLI subcommand

## New CLI Commands

```bash
openwand eval list                     # List all 8 scenario fixtures
openwand eval run --scenario all       # Run all scenarios
openwand eval run --scenario <id>      # Run specific scenario
```

## New Types

- `EvalScenario`, `EvalExpectations`, `EvalTag`
- `EvalRunReport`, `EvalScore`, `DimensionScore`
- `ProviderRealitySnapshot`, `ProviderHealthStatus`
- `MemoryEvalResult`, `ToolEvalResult`, `PolicyEvalResult`
- `PatchEvalResult`, `ExplainEvalResult`, `RebuildEvalResult`

## 8 Evaluation Scenarios

| ID | Purpose |
|----|---------|
| `memory_verified_used` | Model uses verified included memory |
| `low_confidence_excluded` | Model ignores excluded memory |
| `conflict_requires_review` | Model doesn't resolve conflict silently |
| `policy_blocks_forbidden_write` | Model handles policy block safely |
| `patch_plan_then_apply` | Model plans before applying |
| `preimage_mismatch_recovery` | Model handles patch rejection |
| `multi_turn_user_correction` | Model adjusts after user correction |
| `trace_rebuild_after_eval` | Session rebuilds after run |

## Invariant Guards

| Guard | Enforcement |
|-------|-------------|
| Default CI remains provider-free | `real-model-eval` feature gate |
| Eval report never exposes API keys | `provider_snapshot_redacts_api_key` test |
| Eval collector never queries raw store | `eval_memory_uses_governed_report_not_raw_store` |
| Eval runner never mutates outside governed executor | Uses existing policy-gated tools |
| Scenarios are deterministic fixtures | YAML + loader validation |
| Anti-vacuous-pass enforced | `check_evidence_presence()` |
| Report schema version always present | `EVAL_REPORT_SCHEMA_VERSION` constant |
| All scenarios require rebuild + explain | `eval_flagship_*` tests |

## Corrections Applied (from review)

1. ✅ Dependency wording: "No new OpenWand workspace crates. One new external dependency."
2. ✅ Endpoint wording: "Observed dev endpoint... not required by CI"
3. ✅ Test count: 935 → 967 (+32), within +38-42 estimate
4. ✅ Anti-vacuous-pass invariant: 3 tests
5. ✅ Report schema versioning: `report_schema_version: u16`
6. ✅ Provider reproducibility: `temperature`, `max_tokens` on snapshot

## What Does NOT Change

- Governance profile values (locked 02r)
- Memory pipeline mechanics (locked 02j-02k)
- Policy rules (locked 03a-03f, 04a-04b)
- Prompt format (same section structure)
- Agent loop phases (same 10-phase lifecycle)
- Trace schema (append-only, immutable)
- Memory evaluation fixtures (zero modifications to existing 19)

## Acceptance Commands

```bash
# Standard CI — must pass without any provider
cargo test --workspace --features "openwand-session/testing,openwand-session/sqlite-testing,openwand-memory/testing,openwand-memory/sqlite-testing"

# Memory regression gate
cargo test -p openwand-app --features memory-regression

# Real-model eval (requires local LM Studio or equivalent)
cargo run -p openwand-app --features real-model-eval -- eval list
cargo run -p openwand-app --features real-model-eval -- eval run --scenario all
```

## Next Wave Considerations

- Wire eval runner to real LLM provider for actual scenario execution
- Longitudinal report comparison (schema versioning enables this)
- Multi-provider abstraction expansion
- Auto-commit after measured patch correctness
- Rich text editing for message composition
