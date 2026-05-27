# WAVE 02G — REAL WIRING — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-27
**Scope:** Replace all stubs with real wiring, end-to-end memory loop proven

## What changed

### Stubs eliminated
- `main.rs`: `StubMemoryStore` → real `SqliteMemoryStore` + `MemoryCoordinator` runs after each turn
- `ui_main.rs`: `StubMemoryStore` → real `SqliteMemoryStore` + coordinator runs after run completes + memory panel renders

### New wiring
- CLI: opens SqliteMemoryStore, passes to SessionRunner, runs coordinator after turn
- UI: opens SqliteMemoryStore, passes to runner, polls run state, runs coordinator on completion, refreshes memory panel
- Both: `KeywordExtractor` used as production extractor (v0, real)

### Memory panel in UI
- Right sidebar showing memory records: claim, kind badge, confidence %, source count
- Refreshes automatically after each run completes
- Empty state prompt: "Say 'remember X' to create one"

### Bug fixes
- `SqliteMemoryStore::list_active_records()` now populates source_episode_ids and source_trace_ids (was returning empty vecs)
- `InMemoryMemoryStore` and `SqliteMemoryStore` now implement `MemoryReadStore` (delegating to `search_records`)

## Proven by E2E tests

```
e2e_remember_x_appears_in_memory_and_is_retrieved_later:
  1. User says "remember X" → trace entry recorded
  2. Coordinator projects episodes → extracts candidates → accepts records
  3. Memory panel shows X with source trace IDs
  4. Next search retrieves X
  5. Memory context formats for prompt injection
  6. Memory survives reopen

e2e_memory_coordinator_is_idempotent:
  1. Project twice → still only 1 record (duplicate attaches, doesn't create)
```

## Tests: 267 total (+2 E2E), 0 failures
