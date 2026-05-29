# WAVE02N MEMORY PROVENANCE HYDRATION LOCK

**Date:** 2026-05-29
**Commits:** f97529b → 7efc122 + fork guard (de485f3)
**Tests:** 681 → 709, zero failures

## Lock Condition (all met)

```
Wave 02n is locked when prompt-input and panel rows share one hydrated
provenance DTO, every included/excluded claim exposes available evidence
or an explicit missing/partial status, conflict and supersession fields are
threaded from existing MemoryRecord/RankedMemoryHit data, prompt text remains
unchanged, and the panel remains unable to query raw memory/store/trace state.
```

## What Shipped

### Commit 1+2 — Provenance hydration DTOs and pure hydrator
- New `provenance_hydration` module
- `HydratedMemoryClaim`: display-ready DTO with no store references
- `MemoryEvidenceProvenance`: record_id, source traces, confidence, rank score summary
- `ConflictProvenance`: conflict_group_id from MemoryRecord
- `SupersessionProvenance`: supersedes/superseded_by record IDs
- `ProvenanceHydrationStatus`: Complete/Partial/Missing (honest)
- `MemoryTrustBucket`: one-to-one with RepoConsistencyFindingKind
- `MemoryProvenanceHydrator`: pure `hydrate()` + batch `hydrate_findings()`
- Lookup precedence: record_id → normalized_text_hash → exact text → lowercase text
- Rank score exposed as formatted summary string, not raw struct
- 20 new tests including duplicate claim text handling and determinism

### Commit 3 — Coordinator threading
- `PromptInputResult` gains `hydrated_claims: Vec<HydratedMemoryClaim>`
- Hydration runs after report assembly, using `records` + `all_hits` already in scope
- No new queries, no async, no trace lookups

### Commit 4 — panel_view consumes hydrated claims
- `MemoryPanelClaim` gains 7 new fields: record_id, source_trace_ids, confidence, provenance_label, conflict_group_id, superseded_by, hydration_status
- New `from_hydrated_claims()` constructor — preferred path
- `from_coordinator_output()` preserved for backward compat

### Commit 5 — UI DTOs and service
- `UiMemoryPanelRow` gains 7 new fields matching MemoryPanelClaim
- `memory_service` uses `from_hydrated_claims()` when available
- Dioxus panel shows provenance label under each claim

### Commit 6 — Integration tests + lock doc
- 7 E2E tests proving provenance wiring, prompt stability, and architectural guards

## Architecture

```text
MemoryCoordinator::produce_prompt_inputs()
  → RepoConsistencyReport (findings)
  → MemoryPromptAssemblyInputs (prompt inclusion)
  → MemoryProvenanceHydrator::hydrate_findings(findings, hits, records)
  → Vec<HydratedMemoryClaim> (full provenance)
  → PromptInputResult carries all three

build_filtered_panel(&PromptInputResult)
  → RepoFilteredPanelView::from_hydrated_claims(...)
  → UiFilteredMemoryPanel (UI DTOs)
  → Dioxus rendering with provenance labels
```

## Prompt Stability Guarantee

02n does NOT change what the model sees. Provenance tags are NOT injected into
prompt context. The `hydration_does_not_change_prompt_context_text` test proves
that `to_prompt_block()` output is unchanged after hydration. Provenance is for
the panel and audit surface only.

## What Does NOT Change

| Item | Why |
|------|-----|
| `MemoryStore` trait | No new methods |
| `MemoryReadStore` trait | Untouched |
| `repo_consistency` module | Findings unchanged |
| `prompt_assembly` module | Prompt output unchanged |
| Ranking logic | No score changes |
| Bucket classification | No new buckets |
| Runner's prompt path | Prompt text unchanged |
| `CachedMemoryPromptInputs` | Unchanged |

## Provenance Fork Decision

**Locked:** Memory provenance remains panel-visible, audit-visible, and evaluation-visible,
but not prompt-visible during normal agent/tool-use runs.

Trace-relation data may improve deterministic ranking, exclusion, confidence, conflict handling,
and audit explanations, but must not be rendered into the standard memory prompt context.

Any future provenance-in-prompt behavior requires:
1. A separate `MemoryReasoningPromptBuilder`
2. An explicit interaction mode (epistemic, not operational)
3. Feature gating
4. Measured model-quality justification

**Hard invariant tests:**
- `hydration_does_not_change_prompt_context_text`
- `normal_prompt_assembly_does_not_render_provenance_tags`

**Trace relation placement:**

| Relation | Normal prompt | Deterministic use |
|----------|--------------|-------------------|
| Supersedes | Do not show | Exclusion / stale |
| Invalidates | Do not show | Exclusion / correction |
| ConflictsWith | Do not show raw conflict | Downgrade / exclude / review |
| DerivedFrom | Do not show | Panel/audit lineage |
| Verifies | Do not show | Confidence/ranking boost |
| Refines | Show refined only | Supersession lineage |
| References | Do not show | Weak provenance label |

## Known Gaps (honest list)

| Gap | Status |
|-----|--------|
| No trace-relation hydration | `TraceRelationKind::Supersedes/ConflictsWith` exists but hydrator doesn't query trace store |
| `conflicting_claim_texts` always empty | No store query to look up competing records in same conflict group |
| `extraction_model` always None | Not stored on MemoryRecord, requires trace metadata |
| `conflict_group_id` never written by store | Known from 02i-c, hydrator shows None |
| Prompt provenance tags not added | Deferred — changes model behavior, separate decision |
| No `GateResultSnapshot` | Gate decisions not captured on MemoryRecord |

## Test Delta

| Suite | Before | After | Delta |
|-------|--------|-------|-------|
| memory (provenance_hydration) | 0 | 20 | +20 |
| app (provenance_wiring) | 0 | 7 | +7 |
| **Total** | **681** | **708** | **+27** |
