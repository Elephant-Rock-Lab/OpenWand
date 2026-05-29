# WAVE02P_MEMORY_EVALUATION_HARNESS_LOCK

**Date:** 2026-05-29
**Commits:** 964eb81 → d180065 (5 commits)
**Tests:** 738 → 785, zero failures

## Lock Condition (all met)

```
Wave 02p is locked when deterministic memory evaluation scenarios can seed
memory and trace fixtures, run the existing prompt-input path, capture hydrated
claims with provenance and trace lineage, execute deterministic mock model
behaviors, judge expected versus actual memory use, render stable reports, and
prove that evaluation does not change prompt text, memory context hashes,
ranking, bucket assignment, memory records, trace entries, or normal runtime
behavior.
```

## What Shipped

### Commit 1 — Evaluation DTOs
- New `evaluation` module in `openwand-memory`
- `MemoryEvaluationScenario`: seed memory + trace + relations + expectations
- `ExpectedScenarioOutcome`: Pass / Fail — fixtures declare their expected outcome
- `MemoryEvaluationExpectations`: must_retrieve/include/exclude/use, bucket assertions, provenance, trace lineage
- `MemoryEvaluationReport`: passed/failed + snapshot + failures + warnings
- `PromptInputEvaluationSnapshot`: prompt_block + **memory_context_hash** + hydrated claims + report summary
- `RepoConsistencySummarySnapshot`: distinct type (not a copy of canonical `RepoConsistencyReport`)
- `MemoryEvaluationFailure`: 9 variants including **UnsupportedClaimUsedByModel**
- `EvaluationModelConfig`: Mock (5 behaviors) + Real (stub-only, not in CI)

### Commit 2 — Deterministic judgment engine
- `MemoryEvaluationJudge::judge()` — pure function, no I/O
- Deterministic string matching: exact → normalized → phrase presence
- Bucket, provenance, and trace lineage assertions
- `ExpectedScenarioOutcome::Pass` → `passed = failures.is_empty()`
- `ExpectedScenarioOutcome::Fail` → `passed = !failures.is_empty()` (correctly detected failure)

### Commits 3+4 — Harness + mock model
- `MemoryEvaluationHarness`: isolated `InMemoryMemoryStore` + `InMemoryTraceStore`
- Seeds fixtures → creates `MemoryCoordinator` → calls `produce_prompt_inputs()`
- `run_mock_model()`: EchoIncludedMemory, IgnoreIncludedMemory, UseExcludedMemory, HallucinateUnsupportedClaim, CorrectAnswer

### Commits 5-8 — Fixtures, report rendering, guards, E2E
- 3 JSON fixtures: pass scenario + 2 fail scenarios
- `MemoryEvaluationReport::to_markdown()`: stable markdown with claims, provenance, lineage
- 8 architecture guards proving evaluation doesn't change runtime behavior
- 6 E2E tests: fixture loading, JSON roundtrips, structural stability

## Architecture

```text
MemoryEvaluationHarness::run_scenario()
  1. Seed InMemoryMemoryStore + InMemoryTraceStore
  2. Create MemoryCoordinator with isolated stores
  3. Call produce_prompt_inputs() (same path as runtime)
  4. Capture PromptInputEvaluationSnapshot
  5. Run mock model (or stub real LLM)
  6. MemoryEvaluationJudge::judge() [pure]
  7. Return MemoryEvaluationReport

Isolation guarantee:
  - Each harness creates its own stores
  - No shared state with runtime
  - Same seed → same prompt hash across runs
```

## Prompt Stability Guarantee

02p does NOT change what the model sees. The harness calls the same
`produce_prompt_inputs()` path as runtime. Eight architecture guard tests
prove:

- Prompt text unchanged
- No provenance tags rendered into prompt
- No trace IDs rendered into prompt
- Bucket assignment unchanged
- No memory records written after seed
- No trace entries appended after seed
- Isolated stores
- Prompt hash matches across identical runs

## Five Review Patches Applied

| Patch | Status |
|-------|--------|
| No redefined `RepoConsistencySummary` → `RepoConsistencySummarySnapshot` | ✅ |
| `memory_context_hash` on `PromptInputEvaluationSnapshot` | ✅ |
| `UnsupportedClaimUsedByModel` failure variant | ✅ |
| `ExpectedScenarioOutcome` separates pass/fail fixtures | ✅ |
| `memory_evaluation_does_not_append_trace_entries_after_seed` renamed | ✅ |

## What Does NOT Change

| Item | Why |
|------|-----|
| `MemoryStore` trait | No new methods |
| `TraceStore` trait | No new methods |
| `MemoryCoordinator` | No changes — harness calls existing API |
| `produce_prompt_inputs()` | Same path as runtime |
| `prompt_assembly` module | Prompt output unchanged |
| Ranking formulas | No score changes |
| Trust buckets | No new buckets |
| Runner's prompt path | Prompt text unchanged |
| Panel store access | Still forbidden |

## Known Gaps (honest)

| Gap | Why acceptable |
|-----|---------------|
| No semantic LLM-as-judge | Correct for first harness; deterministic only |
| No prompt provenance tags | Preserves fork decision |
| No ranking tuning | Evaluation observes ranking; does not alter it |
| No real LLM in CI | `EvaluationModelConfig::Real` is stub-only |
| No MemoryReasoningPromptBuilder | Separate explicit interaction mode |
| No automatic regression dashboard | Future tooling |
| No statistical eval suite | Future after deterministic scenarios |
| No automatic claim extraction from model output | Start with explicit expected phrases |
| Record IDs non-deterministic in markdown reports | ULID-based; structural stability tested instead |
| Fixtures depend on temp workspace matching | Low-confidence claims may classify differently |

## Test Delta

| Suite | Before | After | Delta |
|-------|--------|-------|-------|
| memory (evaluation) | 0 | 5 | +5 |
| memory (evaluation_judge) | 0 | 11 | +11 |
| memory (evaluation_report) | 0 | 6 | +6 |
| app (memory_eval harness+model) | 0 | 11 | +11 |
| app (memory_eval_e2e) | 0 | 6 | +6 |
| app (memory_evaluation_guards) | 0 | 8 | +8 |
| **Total** | **738** | **785** | **+47** |
