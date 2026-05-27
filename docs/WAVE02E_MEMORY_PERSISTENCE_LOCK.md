# WAVE 02E — MEMORY PERSISTENCE + UI VISIBILITY — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-27
**Scope:** SQLite memory persistence, UI memory panel, provenance visibility

## Proven

- Memory episodes persist to SQLite and survive app restart
- Memory records persist with full source provenance (episode_id → trace_id)
- Projection is idempotent by source_trace_id (UNIQUE constraint)
- Duplicate claims attach new source episodes instead of creating duplicates
- Superseded records kept with `valid_until` and `superseded_by`
- Keyword search works through SQLite (LIKE, case-insensitive)
- Memory search survives close → reopen (tempfile test)
- UI memory panel shows records with source trace IDs
- Confidence stored as basis points (INTEGER) to avoid float storage

## Architecture

```
SqliteMemoryStore (own migrations, same DB file, WAL mode)
  → memory_episode table (UNIQUE source_trace_id → idempotent)
  → memory_record table (status: active/superseded, confidence_bps)
  → memory_record_source table (record ↔ episode ↔ trace provenance)
  → memory_projection_checkpoint table (reserved)

Memory UI:
  → memory_dto.rs: UiMemoryRecord, UiMemoryPanel
  → memory_service.rs: build_memory_panel() bridge
  → App tests: panel lists records with source trace IDs
```

## Schema

Three tables + one checkpoint table:
- `memory_episode`: immutable trace projections
- `memory_record`: accepted facts/decisions/preferences
- `memory_record_source`: provenance links
- `memory_projection_checkpoint`: for incremental projection

## New Files

- `crates/memory/src/sqlite_schema.rs` — migration SQL
- `crates/memory/src/sqlite_store.rs` — SqliteMemoryStore implementation
- `crates/memory/tests/sqlite_memory.rs` — 8 SQLite acceptance tests
- `crates/app/src/ui/memory_dto.rs` — UiMemoryRecord, UiMemoryPanel
- `crates/app/src/ui/memory_service.rs` — build_memory_panel() bridge
- `crates/app/tests/memory_panel.rs` — 2 UI memory panel tests

## Tests: 257 total (+10), 0 failures
