# WAVE02M MEMORY PANEL REPO-FILTERED VIEW LOCK

**Date:** 2026-05-29
**Commits:** 1d377f0 → a3d6741 (7 commits)
**Tests:** 667 → 681, zero failures

## Lock Condition (all met)

```
The memory panel renders repo-consistency-filtered memory state for the active
session/workdir, separating trusted prompt-included claims from stale,
missing-in-repo, missing-in-memory, conflicted, unverifiable,
and superseded-ignored records, using the same coordinator-produced
report path as session context.
```

## What Shipped

### Commit 1 — Unverifiable claims as list
- `unverifiable_claims_excluded: usize` → `Vec<UnverifiableMemoryClaim>`
- New `UnverifiableMemoryClaim` struct with `claim_text` + `evidence_kind`
- Prompt output unchanged (still shows count, not claim texts)
- Panel can now access individual unverifiable claims

### Commit 2 — Panel DTOs in openwand-memory
- New `panel_view` module
- `RepoFilteredPanelView` — read-only projection of coordinator output
- `MemoryPanelClaim` — single claim with finding kind + provenance
- `MemoryPanelMissingObservation` — repo observation with no memory claim
- `MemoryPanelConflictGroup` — conflict group requiring review
- `MemoryPanelSummary` — counts per trust bucket
- `from_coordinator_output(&Report, &Inputs)` — only constructor, no store access
- Architectural guard test: `filtered_panel_builder_does_not_access_memory_store`

### Commit 3 — Report persisted in PromptInputResult
- `PromptInputResult` gains `report: RepoConsistencyReport`
- All error/fallback paths return empty report
- Panel can render findings without re-running the coordinator

### Commit 4 — UI panel replaced with filtered bucket view
- `UiFilteredMemoryPanel` replaces `UiMemoryPanel`
- Seven trust buckets: prompt_included, stale, missing_in_repo, missing_in_memory, conflicts, unverifiable, superseded_ignored
- `build_filtered_panel(&PromptInputResult)` — pure conversion, no store queries
- Dioxus rendering shows bucket headers with counts and claim rows

### Commit 5 — Provenance affordances
- `MemoryPanelClaim.source_provenance: Option<ProvenanceSnapshot>`
- `UiMemoryPanelRow.has_provenance: bool`
- Currently always None — wiring ready for future population

### Commit 6 — Integration tests
- 7 E2E tests proving panel-prompt parity and trust bucket correctness

## Architecture

```text
MemoryCoordinator::produce_prompt_inputs()
  → RepoConsistencyReport (findings + classification)
  → MemoryPromptAssemblyInputs (prompt inclusion decisions)
  → PromptInputResult carries BOTH

build_filtered_panel(&PromptInputResult)
  → RepoFilteredPanelView::from_coordinator_output(&Report, &Inputs)
  → UiFilteredMemoryPanel (UI DTOs)

Dioxus MEMORY_PANEL signal ← UiFilteredMemoryPanel
```

## One Memory Reality Invariant

The panel builder (`from_coordinator_output`) takes `(&RepoConsistencyReport, &MemoryPromptAssemblyInputs)` only. No `MemoryStore`, no `MemoryReadStore`, no raw record queries. Enforced at compile time by the function signature. Proven by the `filtered_panel_builder_does_not_access_memory_store` guard test.

## Trust Buckets

| Bucket | Source | Trust |
|--------|--------|-------|
| `prompt_included` | `RepoConsistencyFindingKind::Supported` | Trusted, sent to model |
| `stale` | `RepoConsistencyFindingKind::StaleMemory` | Outdated, not sent |
| `missing_in_repo` | `RepoConsistencyFindingKind::MissingInRepo` | Memory claims something repo doesn't have |
| `missing_in_memory` | `RepoConsistencyFindingKind::MissingInMemory` | Repo has structure not in memory |
| `conflicts` | `RepoConsistencyFindingKind::ConflictRequiresReview` | Needs human review |
| `unverifiable` | `RepoConsistencyFindingKind::Unverifiable` | Cannot be checked deterministically |
| `superseded_ignored` | `RepoConsistencyFindingKind::SupersededMemoryIgnored` | Replaced by newer claim |

## What Does NOT Change

- `MemoryStore` trait — no new methods
- `MemoryReadStore` trait — untouched
- `repo_consistency` module — consumed as-is
- `produce_prompt_inputs()` logic — no change, just returns report
- Runner's prompt assembly path — unchanged
- `CachedMemoryPromptInputs` — unchanged
- CLI binary — no panel needed

## Known Gaps

| Gap | Status |
|-----|--------|
| `supported_excluded` bucket | Deferred — requires budget-aware prompt assembly |
| `source_provenance` always None | Deferred — coordinator doesn't populate from RankedMemoryHit yet |
| Conflict grouping by group_id | Deferred — each finding is its own group |
| User editing/correction from panel | Separate write-governance seam |
| Visual provenance graph | UI enhancement |
| Panel refresh during running turn | Refreshes only after turn completion |
| `UnverifiableMemoryClaim` has no `record_id` | Can't link back to source MemoryRecord |

## Test Delta

| Suite | Before | After | Delta |
|-------|--------|-------|-------|
| memory (panel_view) | 0 | 7 | +7 |
| app (panel_wiring) | 0 | 7 | +7 |
| **Total** | **667** | **681** | **+14** |
