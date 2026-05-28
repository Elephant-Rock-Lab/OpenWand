# WAVE02I_C_MEMORY_EVIDENCE_STORE_WIRING_LOCK

**Status:** ✅ LOCKED  
**Commits:** 1fcc1dd → 8365600 (8 commits)  
**Tests:** 543 passing, 0 failures  

## Lock condition

```
MemoryStore acceptance, supersession, deduplication, SQLite persistence,
in-memory persistence, and ranked retrieval all honor EvidenceKind, DedupKey,
RetrievalMode, normalized_text_hash, supersedes_record_id, and conflict_group_id
with matching behavior across InMemoryStore and SqliteMemoryStore.
```

## What shipped

### Commit 0 — Write authority seam
- Documented `accept_candidate` and `supersede_record` as sole persistence paths
- 4 tests proving no alternative write paths exist

### Commit 1 — Persist evidence_kind on write
- `MemoryRecord.evidence_kind: EvidenceKind` field
- Default `AcceptedClaim`, serde-default for legacy deserialization
- Both stores write `"AcceptedClaim"` on insert
- SQLite reads from column, NULL → AcceptedClaim fallback

### Commit 2 — Persist normalized_text_hash on write
- `MemoryRecord.normalized_text_hash: String` field
- BLAKE3 hash via `dedup::compute_normalized_hash`
- SQLite dedup index: `idx_memory_record_dedup ON (normalized_text_hash, scope_kind, scope_payload, evidence_kind)`
- Index includes scope columns per user correction #2

### Commit 3 — Invoke DedupKey during acceptance
- Replaced claim-text dedup with hash-based dedup in both stores
- In-memory: hash comparison in records map
- SQLite: indexed lookup via dedup index
- Duplicate → attach source episodes + trace IDs, no second record

### Commit 4 — Wire supersession writes
- `MemoryRecord.supersedes_record_id: Option<String>` field
- Successor links to predecessor
- **Key design decision (user correction #4):** evidence_kind NOT mutated to SupersededClaim on write. Original preserved. `SupersededClaim` derived at retrieval via `derived_evidence_kind()`.

### Commit 5 — Add search_ranked with RetrievalMode
- New `MemoryStore::search_ranked(query, mode) → RankedRetrievalContext`
- Both stores implement token-based relevance + evidence authority + supersession penalty
- Four modes: Default, CurrentState, ChangeHistory, ConflictSearch

### Commit 6 — Read conflict_group_id in search
- `MemoryRecord.conflict_group_id: Option<String>` field
- Read from SQLite column
- Conflict labeling not yet wired (needs conflict detection)

### Commit 7 — Backend conformance tests
- 11 tests verifying identical behavior across InMemory and SQLite
- SQLite `list_all_records_for_search()` for non-CurrentState modes

## Accepted deviations from user spec

1. **No `CandidateAcceptanceService` wrapper** — `accept_candidate` stays on `MemoryStore`
2. **No entity model on DedupKey** — too early, no entity model exists
3. **`search_with_options` → `search_ranked`** — simpler API, single new method
4. **`memory_claim` → `memory_record`** — matching actual table name
5. **`EvidenceAudit` mode deferred** — no consumer exists
6. **`evidence_kind` NOT mutated to SupersededClaim on supersession** — derived at retrieval, preserving original evidence classification
7. **Dedup index includes scope columns** — per user correction #2

## Definition of Done — all 12 items met

1. ✅ accepted records persist evidence_kind
2. ✅ accepted records persist normalized_text_hash
3. ✅ duplicate candidates attach source IDs, not new records
4. ✅ supersession writes populate supersedes_record_id
5. ✅ superseded records retrievable in history mode
6. ✅ current-state retrieval excludes superseded records
7. ✅ default retrieval penalizes superseded/conflicting records
8. ✅ conflict_group_id is read and available in records
9. ✅ SQLite migrations preserve legacy records
10. ✅ in-memory and SQLite pass same evidence conformance suite
11. ✅ session remains read-only against memory projections
12. ✅ no Git, repo report, LLM conflict detection, or prompt assembly

## Test delta

431 → 543 = +112 tests across 8 commits.

## Next

**Wave 02j — Memory-Backed Repo Consistency Check**

02j can now safely ask:
- Which current-state memory claims describe the repo?
- Which claims are superseded and should not be used as current truth?
- Which records are conflict-grouped and need explicit report sections?
- Which source/evidence kinds are strong enough to support a consistency finding?
- Which duplicate evidence strengthens an existing claim?
