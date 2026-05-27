# WAVE 01E — SQLITE TRACESTORE — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-27
**Commits:** 28–31

## Summary

Wave 01e is complete. `openwand-store` now provides a SQLite-backed `TraceStore<StoredEvent>` that persists authoritative trace entries, relations, blobs (table only), idempotency keys, sequence numbers, and hash chains across process restart.

## Verification

| Metric | Value |
|---|---:|
| Workspace tests | 187 total |
| Previous test count | 167 |
| New store tests | 17 (4 hash + 5 migration/append + 6 query/replay + 2 guards) |
| New session SQLite tests | 3 |
| Warnings | 0 |
| Compile | Clean |

## Built

- SQLite migration runner with dirty-flag crash recovery
- `trace_entry` table with indexed columns
- `trace_relation` table with foreign keys
- `trace_blob` table (schema reservation only)
- `openwand_migration` table with version/checksum/dirty tracking
- Serialized blocking writer (`mpsc::Sender<WriterCommand>` → `spawn_blocking`)
- `SqliteStore` implementing `TraceStore<StoredEvent>`
- Idempotency key persistence and reload
- Stream/global monotonic sequence assignment
- Production BLAKE3 hash chain
- Relation persistence and reload
- Query by stream, event_kind, global sequence range
- Reopen/replay tests
- Session SQLite acceptance: text-only, tool turn, close-reopen-replay

## Locked Boundary

```text
openwand-store → openwand-core
openwand-store → openwand-trace

openwand-store ↛ openwand-session
openwand-store ↛ openwand-policy
openwand-store ↛ openwand-tools
openwand-store ↛ openwand-llm
openwand-store ↛ openwand-memory
openwand-store ↛ loro
openwand-store ↛ rig
openwand-store ↛ rmcp
```

## Accepted Scope Limit

| Deferred Item | Target |
|---|---|
| Memory projection tables | Wave 02 / Store Batch 3 |
| Loro snapshot persistence | Projection/snapshot wave |
| Full projection checkpointing | Store Batch 2 |
| CozoDB backend | Store Batch 4 |
| SurrealDB backend | Store Batch 5 |
| Trace compaction | Post-Batch 1, explicit user consent |
| Blob read/write code paths | Store Batch 2 |
| `refinery` migration runner | Revisit if migrations exceed 3–5 files |

## Migration Runner Tradeoff

Custom runner was chosen for 01e because:
- Only one migration file needed
- Requirements are small: version, checksum, dirty flag, applied_at
- Dirty-startup refusal is easy to test
- Avoids introducing another dependency before store boundary is locked

Revisit `refinery` if migrations exceed 3–5 files or if multi-backend migration orchestration is needed.

## Accepted Execution Deviation

| Planned | Actual | Reason | Impact |
|---|---|---|---|
| 9 commits (28–36) | 4 commits (28–31) | 01d proved batching is more efficient under real API friction | No reduction in coverage; all acceptance tests pass |

## Final Statement

Wave 01e is locked. Wave 01 is no longer in-memory-only: deterministic session turns can record durable trace events into SQLite, survive process restart, and replay enough session-relevant history to support reload/rebuild.

The Wave 01 gate is satisfied:
> A deterministic session turn can run through the 10-phase loop, record all durable events to trace, project to Loro, and reload from SQLite trace persistence.
