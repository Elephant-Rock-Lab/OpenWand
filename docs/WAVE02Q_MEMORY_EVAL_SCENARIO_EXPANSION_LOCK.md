# WAVE02Q_MEMORY_EVAL_SCENARIO_EXPANSION_LOCK

**Date:** 2026-05-29
**Commits:** 28415e0 → ad58827 (4 commits)
**Tests:** 785 → 821, zero failures
**Fixtures:** 3 → 19

## Lock condition (met)

```
Wave 02q is locked when deterministic memory evaluation fixtures cover
prompt-included, stale, missing-in-repo, missing-in-memory, conflict,
unverifiable, superseded, verified, low-confidence, and unsupported-output
cases; each fixture records expected retrieval, inclusion/exclusion, bucket
assignment, provenance, trace lineage, and model usage; all reports are
structurally stable; and evaluation still does not change prompt text,
memory context hashes, ranking formulas, bucket assignment, memory records,
trace entries, or normal runtime behavior.
```

## What Shipped

### Commits 1-3: Category taxonomy + coverage validator + harness expansion

Six reviewer patches applied:
1. **Category is required** — no serde default; old fixtures explicitly updated
2. **Stale is judge-only** — full harness only if current code emits `StaleMemory`
3. **ScenarioExecutionMode** — `FullHarness` / `JudgeOnly`; coverage validator sees all categories
4. **TraceSeed uses labels** — not store-assigned TraceIds; harness maintains `trace_labels` BTreeMap
5. **MemoryRecordSeed has source_trace_labels** — harness resolves labels through trace map before accepting candidates
6. **Supersession is key-based** — `superseded_by_label` + `label` fields; harness resolves after all seeds inserted

New types:
- `MemoryEvaluationCategory` (10 variants, `all()` accessor)
- `ScenarioExecutionMode` (FullHarness/JudgeOnly)
- `SeedResolutionMaps` (trace_labels + memory_labels BTreeMaps)
- `MemoryEvaluationCoverageValidator` + `MemoryEvaluationCoverageReport`
- `MemoryEvaluationSuiteReport` with `to_markdown_index()`

Harness phases:
1. Seed trace entries → capture store-assigned TraceIds in label map
2. Seed trace relations → resolve labels to TraceIds
3. Seed memory records → resolve source_trace_labels via trace map
4. Supersession → resolve label→record_id, call `supersede_record()`

### Commits 4-7: 19 fixtures across all 10 categories

| Category | Fixtures | Mode |
|----------|---------|------|
| PromptIncluded | 4 | Full harness |
| Stale | 1 | Judge-only |
| Superseded | 2 | Full harness |
| Conflict | 1 | Judge-only |
| Unverifiable | 2 | Full harness |
| MissingInRepo | 2 | Full harness |
| MissingInMemory | 1 | Full harness |
| VerifiedTraceLineage | 2 | Full harness |
| LowConfidence | 1 | Full harness |
| UnsupportedOutput | 3 | Full harness |

### Commits 8-10: E2E runner + suite report + guards

- 24 E2E tests: individual fixture validation + suite coverage gate
- Coverage gate: `memory_eval_fixture_suite_covers_all_categories` asserts zero missing
- ID uniqueness check: `memory_eval_fixture_ids_are_unique`
- Suite report: `MemoryEvaluationSuiteReport::to_markdown_index()` renders stable markdown
- New guard: `expanded_eval_suite_does_not_change_runtime_prompt_hashes`

## Fixture reality table (honest)

| Category | Full harness | Judge-only | Note |
|----------|:---:|:---:|-------|
| PromptIncluded | ✅ | — | Matching memory + workspace |
| Stale | ⚠️ | ✅ | Full harness only if current code emits Stale; judge-only covers the evaluator |
| Superseded | ✅ | — | Label-based supersede_record seeding |
| Conflict | — | ✅ | Conflict detection not wired (known gap) |
| Unverifiable | ✅ | — | Non-repo / outside grammar claim |
| MissingInRepo | ✅ | — | Claim about absent repo object |
| MissingInMemory | ✅ | — | Repo object exists, no matching memory |
| VerifiedTraceLineage | ✅ | — | Trace label mapping + memory source trace labels |
| LowConfidence | ✅ | — | Seed confidence 0.2; observe current policy |
| UnsupportedOutput | ✅ | — | Mock hallucination behavior |

## What Does NOT Change

| Item | Why |
|------|-----|
| MemoryStore trait | No new methods |
| TraceStore trait | No new methods |
| MemoryCoordinator | No changes — harness calls existing API |
| produce_prompt_inputs() | Same path as runtime |
| prompt_assembly module | Prompt output unchanged |
| Ranking formulas | No score changes |
| Trust buckets | No new buckets |
| Runner's prompt path | Prompt text unchanged |
| Panel store access | Still forbidden |
| Conflict detection | Still not wired (known gap) |

## Known gaps (honest)

| Gap | Why acceptable |
|-----|---------------|
| Conflict fixtures are judge-only | Conflict detection not wired — test evaluator, not unwired feature |
| Stale fixtures are judge-only | Current code may classify stale claims as MissingInRepo |
| Still deterministic string matching only | Correct for first corpus |
| No real LLM CI path | Correct; no API keys/network in CI |
| Record IDs non-deterministic in reports | ULID-based; structural stability tested |
| Low-confidence fixture observes current policy | Policy tuning is 02r's job |
| No automatic scenario generation | Hand-authored fixtures more trustworthy initially |
| No dashboard | Markdown suite report sufficient |
| No ranking tuning | Evaluation corpus must precede tuning |

## Test delta

| Suite | Before | After | Delta |
|-------|--------|-------|-------|
| memory (evaluation) | 5 | 14 | +9 |
| memory (evaluation_judge) | 11 | 11 | 0 |
| memory (evaluation_report) | 6 | 6 | 0 |
| memory (evaluation_coverage) | 0 | 8 | +8 |
| app (memory_eval harness+model) | 11 | 13 | +2 |
| app (memory_eval_e2e) | 6 | 24 | +18 |
| app (memory_evaluation_guards) | 8 | 9 | +1 |
| **Total** | **785** | **821** | **+36** |

## Next recommended wave

```
WAVE02R_MEMORY_RANKING_AND_GOVERNANCE_TUNING
```

02r should tune against this expanded eval suite, not intuition.
