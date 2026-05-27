# WAVE 01a — CORE + TRACE — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-26

## Completed Commits

| # | Scope | Status |
|---|-------|--------|
| 1 | Core IDs + shared vocabulary | ✅ Accepted |
| 2 | Snapshot DTOs | ✅ Accepted |
| 3 | Event family DTOs + OpenWandTraceEvent | ✅ Accepted |
| 4 | Trace substrate types (generic, no core dep) | ✅ Accepted |
| 5 | InMemoryTraceStore (feature-gated) | ✅ Accepted |
| 6 | Core↔Trace conformance seam (StoredEvent bridge) | ✅ Accepted |
| 7 | Dependency guard tests | ✅ Accepted |
| 8 | Wave 01a lock (this document) | ✅ Accepted |

## Accepted Deviations

| Deviation | Reason | Impact |
|---|---------|--------|
| `ProvenanceSnapshot::LlmExtracted` uses `confidence_bps: u16` instead of `confidence: f64` | `f64` cannot derive `Eq + Hash`. Core types must support use as map/query keys. | Zero semantic loss — `confidence()` helper returns `Option<f64>`. Memory crate keeps `f64` internally and converts at boundary. |
| `StoredEvent` newtype instead of bare `impl TraceEventEnvelope for OpenWandTraceEvent` | Rust orphan rules block foreign-trait-on-foreign-type impls even in store. | `StoredEvent` wraps at store boundary, derefs to `OpenWandTraceEvent`. Clean API with zero cognitive overhead. |
| Hash chain in `InMemoryTraceStore` uses deterministic `mem:` prefix, not BLAKE3 | Testing store only. Real hash enforcement in SQLite store. | None — in-memory store is test-only, feature-gated. |

## Final Dependency DAG

```
openwand-core (serde, serde_json, chrono, ulid only)
  ← no deps on any other openwand-* crate

openwand-trace (async-trait, chrono, serde, serde_json, ulid, blake3, thiserror, tracing, tokio)
  ← no deps on any other openwand-* crate
  ← testing feature adds InMemoryTraceStore (uses tokio::sync::RwLock)

openwand-store
  ← openwand-core (production)
  ← openwand-trace (production)
  ← openwand-trace/testing (dev-dependency for conformance tests)
  ← Stores StoredEvent (newtype bridge) implementing TraceEventEnvelope
```

## Bridge Decision

```rust
// In openwand-store::envelope
pub struct StoredEvent(pub OpenWandTraceEvent);

impl std::ops::Deref for StoredEvent {
    type Target = OpenWandTraceEvent;
    fn deref(&self) -> &Self::Target { &self.0 }
}

impl TraceEventEnvelope for StoredEvent {
    fn event_kind(&self) -> &'static str { self.0.event_kind() }
    fn schema_version(&self) -> u16 { self.0.schema_version() }
}
```

Three constraints satisfied:
1. Core does not depend on trace.
2. Trace does not depend on core.
3. Store owns the bridge between OpenWandTraceEvent and TraceEventEnvelope.

## Final Test Count

| Crate | Unit Tests | Integration Tests | Dependency Guards | Total |
|---|---|---|---|---|
| openwand-core | 15 | 0 | 2 | 17 |
| openwand-trace | 19 | 0 | 2 | 21 |
| openwand-store | 0 | 9 | 0 | 9 |
| Scaffold crates (×8) | 8 | 0 | 0 | 8 |
| **Total** | **42** | **9** | **4** | **47** |

## Acceptance Tests Satisfied

From `docs/WAVE01_ACCEPTANCE_TESTS.md`, 01a section:

- [x] Core IDs serialize as strings
- [x] Core event round-trip all families
- [x] Core no forbidden dependencies
- [x] Trace append assigns IDs and sequences
- [x] Trace append 1000 entries
- [x] Trace query by stream
- [x] Trace query by event kind
- [x] Trace relations round-trip
- [x] Trace idempotency key deduplicates
- [x] Trace hash chain links previous entries
- [x] StoredEvent deref exposes core methods
- [x] Trace event serde round-trip through store
- [x] Dependency guards: core has no forbidden deps
- [x] Dependency guards: trace has no forbidden deps
- [x] Dependency guards: core direct deps are locked
- [x] Dependency guards: trace does not depend on core

## Files Changed (01a)

### openwand-core (new files)
- `crates/core/src/ids.rs` — 13 domain IDs
- `crates/core/src/mode.rs` — InteractionMode, ConfirmationLevel
- `crates/core/src/risk.rs` — RiskLevelSnapshot
- `crates/core/src/memory_vocab.rs` — EntityKind, Predicate, ClaimKind, etc.
- `crates/core/src/tool_vocab.rs` — ToolInvoker, ToolEffect, ToolResultStatus
- `crates/core/src/session_vocab.rs` — SessionEndReason, ThinkingBudgetSnapshot
- `crates/core/src/snapshots.rs` — 6 snapshot DTOs
- `crates/core/src/events/mod.rs` — OpenWandTraceEvent top-level enum
- `crates/core/src/events/session.rs` — SessionEvent (5 variants)
- `crates/core/src/events/inference.rs` — InferenceEvent (3 variants)
- `crates/core/src/events/gate.rs` — GateEvent (2 variants)
- `crates/core/src/events/tool.rs` — ToolEvent (6 variants)
- `crates/core/src/events/file.rs` — FileEvent (3 variants)
- `crates/core/src/events/memory.rs` — MemoryEvent (14 variants)
- `crates/core/src/events/mode.rs` — ModeEvent (1 variant)
- `crates/core/src/events/workflow.rs` — WorkflowEvent (6 variants)
- `crates/core/src/events/artifact.rs` — ArtifactEvent (3 variants)
- `crates/core/tests/dependency_guards.rs` — 2 guard tests

### openwand-trace (rewritten from stubs)
- `crates/trace/src/ids.rs` — TraceId
- `crates/trace/src/actor.rs` — Actor enum
- `crates/trace/src/stream.rs` — TraceStreamId, TraceStreamScope, IdempotencyKey, EntryHash
- `crates/trace/src/relation.rs` — TraceRelation, TraceRelationKind, TraceRelationDraft
- `crates/trace/src/entry.rs` — TraceEntry<E>, TraceEntryWithRelations<E>
- `crates/trace/src/query.rs` — TraceQuery, TracePage<E>, ActorFilter, RelationQuery
- `crates/trace/src/append.rs` — AppendTraceEntry<E>
- `crates/trace/src/envelope.rs` — TraceEventEnvelope trait
- `crates/trace/src/store.rs` — TraceStore<E> trait (11 async methods)
- `crates/trace/src/projector.rs` — TraceProjector<E> trait, ProjectionCheckpoint
- `crates/trace/src/error.rs` — TraceError (7 variants)
- `crates/trace/src/testing.rs` — InMemoryTraceStore<E> (feature-gated)
- `crates/trace/tests/dependency_guards.rs` — 2 guard tests

### openwand-store (new files)
- `crates/store/src/envelope.rs` — StoredEvent bridge
- `crates/store/tests/trace_conformance.rs` — 9 conformance tests

### Removed
- `crates/trace/src/types.rs` — replaced by ids.rs + stream.rs

## Deferred to Later Waves

| Item | Target | Reason |
|---|---|---|
| SQLite TraceStore implementation | Wave 01e | Needs rusqlite, blocking worker |
| Memory trait split (Read + Projection) | Wave 02 | Design locked, implementation deferred |
| BLAKE3 hash enforcement in production store | Wave 01e | InMemoryTraceStore uses deterministic placeholder |
| Projection rebuild (named projectors) | Wave 02+ | TraceProjector trait defined, no impls yet |
| Relation depth traversal | Wave 02+ | RelationQuery.depth field exists, not traversed in 01a |
| Trace compaction | Post-Batch 1 | Trace is authoritative audit log — no compaction in Batch 1 |
| MCP HTTP transport | Wave 03 | Batch 1 is stdio only |

---

**Wave 01a is closed. Next: Wave 01b — Policy + LLM Contracts.**
