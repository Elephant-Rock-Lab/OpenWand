# WAVE02O MEMORY TRACE RELATION AUDIT HYDRATION LOCK

**Date:** 2026-05-29
**Commits:** 974441c → 1879a99 (5 commits)
**Tests:** 709 → 738, zero failures

## Lock Condition (all met)

```
Wave 02o is locked when trace relation lineage is hydrated into the existing
HydratedMemoryClaim audit model, relation summaries are deterministic and honest
about missing evidence, panel/UI surfaces relation counts and summaries, normal
prompt text and memory context hash remain unchanged, and no panel/UI code can
query raw trace or memory stores.
```

## What Shipped

### Commit 1+2 — Trace lineage DTOs and pure relation classifier
- New `trace_relation_hydration` module
- `TraceLineageKind`: 10 variants + `Unknown(String)` for forward compat
- `TraceRelationProvenance`: display-ready relation edge with summary
- `ClaimTraceLineage`: per-claim lineage with named buckets + `other_relations`
- `TraceRelationCounts`: counts for panel display
- `TraceRelationAuditRow` / `TraceEventAuditMetadata`: narrow coordinator DTOs
- `TraceRelationAuditHydrator`: pure `hydrate_claim()` + `hydrate_claims()`
- Bidirectional relation lookup (from OR to source trace IDs)
- Deterministic sort: kind rank → occurred_at → trace IDs
- `Implements`, `CausedBy`, `Reverts`, `References`, `Unknown(_)` → `other_relations`
- Honest hydration status: Complete / Partial / Missing
- `HydratedMemoryClaim` gains `trace_lineage: Option<ClaimTraceLineage>`
- 20 new tests

### Commit 3 — Coordinator-side lineage hydration
- `MemoryCoordinator::hydrate_trace_lineage()`: async orchestration
- Collects + deduplicates all `source_trace_ids` from hydrated claims
- Queries `scan_relations()` bidirectionally for each trace ID
- Queries `get()` for event metadata of all involved trace IDs
- Converts to narrow DTOs, passes to pure hydrator
- Attaches `ClaimTraceLineage` to each `HydratedMemoryClaim`
- Non-fatal: scan/get failures produce `Partial` status, prompt assembly never fails

### Commit 4 — Panel DTO and UI rendering
- `MemoryPanelClaim` gains `trace_lineage_summary`, `trace_relation_counts`, `trace_lineage_status`
- `UiMemoryPanelRow` gains same + `UiTraceRelationCounts`
- `from_hydrated_claims()` populates trace fields from `ClaimTraceLineage`
- `from_coordinator_output()` defaults to `None`

### Commit 5 — Prompt stability and architecture guards
- 9 E2E tests proving trace lineage is audit/panel-only
- `trace_lineage_hydration_does_not_change_prompt_context_text`
- `normal_prompt_assembly_does_not_render_trace_ids`
- `coordinator_deduplicates_source_trace_ids_before_relation_scan`
- `panel_builder_does_not_access_trace_store`
- `trace_lineage_is_panel_audit_only`

## Architecture

```text
MemoryCoordinator::produce_prompt_inputs()
  → 02n: hydrate_findings() → Vec<HydratedMemoryClaim>
  → 02o: hydrate_trace_lineage()
       → collect source_trace_ids
       → deduplicate
       → scan_relations(from) + scan_relations(to) for each
       → get() event metadata for all involved IDs
       → TraceRelationAuditHydrator::hydrate_claims() [pure]
       → attach ClaimTraceLineage to each claim
  → PromptInputResult.hydrated_claims

build_filtered_panel(&PromptInputResult)
  → from_hydrated_claims()
  → trace_lineage_summary, trace_relation_counts, trace_lineage_status
  → Dioxus panel rendering
```

## Prompt Stability Guarantee

02o does NOT change what the model sees. Trace lineage is for the panel and audit
surface only. The `trace_lineage_hydration_does_not_change_prompt_context_text` test
proves that `to_prompt_block()` output is unchanged after trace hydration.

## Provenance Fork Decision Preserved

Memory provenance remains panel-visible, audit-visible, and evaluation-visible,
but not prompt-visible during normal agent/tool-use runs. Trace-relation data
may improve deterministic ranking, exclusion, confidence, conflict handling,
and audit explanations, but must not be rendered into the standard memory
prompt context.

## Four Review Patches Applied

| Patch | Status |
|-------|--------|
| `Unknown(String)` variant on `TraceLineageKind` | ✅ |
| `other: usize` covers Implements/CausedBy/Reverts/References | ✅ |
| Coordinator deduplicates `source_trace_ids` before scan | ✅ |
| Trace scan failures non-fatal, produce `Partial` status | ✅ |

## What Does NOT Change

| Item | Why |
|------|-----|
| `MemoryStore` trait | No new methods |
| `TraceStore` trait | No new methods (uses existing `scan_relations` + `get`) |
| `repo_consistency` module | Findings unchanged |
| `prompt_assembly` module | Prompt output unchanged |
| Ranking formulas | No score changes |
| Trust buckets | No new buckets |
| Runner's prompt path | Prompt text unchanged |
| Panel store access | Still forbidden (compile-time guard) |

## Known Gaps (honest)

| Gap | Status |
|-----|--------|
| No batched `scan_relations` for multiple IDs | Per-ID queries; batching is trace substrate optimization |
| No `MemoryReasoningPromptBuilder` | Separate explicit mode, not 02o |
| Does not resolve conflicts | Model should not arbitrate in normal runs |
| Does not change ranking | Audit hydration only |
| Legacy records may show partial lineage | Honest hydration status covers this |
| No full event payload hydration | Audit summary only |
| Trace relations not authoritative over memory records | Memory projection still owns state |

## Test Delta

| Suite | Before | After | Delta |
|-------|--------|-------|-------|
| memory (trace_relation_hydration) | 0 | 20 | +20 |
| app (trace_lineage_wiring) | 0 | 9 | +9 |
| **Total** | **709** | **738** | **+29** |
