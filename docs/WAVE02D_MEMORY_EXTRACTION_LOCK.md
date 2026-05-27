# WAVE 02D — MEMORY EXTRACTION V0 — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-27
**Scope:** Trace-backed memory episodes, deterministic acceptance, keyword retrieval

## Proven

- Trace events project into immutable `MemoryEpisode` rows
- Projection is idempotent by `source_trace_id`
- KeywordExtractor extracts "remember"/"always"/"never" user messages
- Deterministic acceptance: confidence ≥ 0.7, non-empty claim, source episodes required
- Malformed/low-confidence candidates write no fact
- Accepted facts carry `source_trace_ids` (provenance)
- Duplicate claims attach new source episode instead of creating duplicate records
- Superseded facts keep old row with `valid_until` and `superseded_by`
- Keyword search returns relevant facts
- RetrievalContext formats as LLM injection block
- NullExtractor for CI (extracts nothing)

## Architecture

```
Trace events
  → project_episode() creates MemoryEpisode (idempotent)
  → Extractor.propose() returns CandidateMemory[]
  → accept_candidate() applies deterministic rules
  → Accepted → MemoryRecord with source_trace_ids
  → search_records() → RetrievalContext → to_context_block()
  → Injected into session prompt assembly
```

## Invariant

```
LLM extraction proposes.
Deterministic memory policy accepts.
Trace provenance authorizes.
```

No extracted memories become trusted state merely because the model emitted JSON.

## New Files

- `crates/memory/src/types.rs` — MemoryEpisode, CandidateMemory, MemoryRecord DTOs
- `crates/memory/src/extractor.rs` — MemoryExtractor trait
- `crates/memory/src/memory_store.rs` — MemoryStore trait (read+write)
- `crates/memory/src/in_memory.rs` — InMemoryMemoryStore implementation
- `crates/memory/src/testing.rs` — KeywordExtractor + NullExtractor
- `crates/memory/tests/memory_extraction.rs` — 10 acceptance tests

## Tests: 247 total (+12), 0 failures
