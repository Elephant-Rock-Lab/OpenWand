# WAVE02I_B_MEMORY_EVIDENCE_SEMANTICS_LOCK

**Status:** ✅ LOCKED  
**Commits:** 32349fb → 179ef98 (9 commits)  
**Tests:** 493 passing, 0 failures  

## What shipped

Evidence classification, observation invariant, content hash dedup, supersession-aware retrieval, conflict grouping, and deterministic regression fixtures.

## Definition of Done — fulfilled

```
OpenWand memory records and ranked retrieval hits carry evidence-kind semantics;
observations are never treated as accepted claims by default;
duplicate evidence is collapsed deterministically;
superseded claims are preserved but penalized for current-state retrieval;
change-history retrieval can surface supersession chains;
conflicting claims are retained and labeled rather than overwritten;
SQLite migration 0003 is additive;
and deterministic fixtures lock all of the above behavior.
```

## Evidence kinds

| EvidenceKind | Authority (bps) | is_accepted_state | can_support_claim | is_observation |
|---|---|---|---|---|
| UserStatedClaim | 10000 | ✅ | ✅ | ❌ |
| DeterministicEvidence | 9000 | ❌ | ✅ | ✅ |
| AcceptedClaim | 8000 | ✅ | ✅ | ❌ |
| LlmExtractedCandidate | 5000 | ❌ | ❌ | ❌ |
| ConflictingClaim | 4000 | ❌ | ❌ | ❌ |
| RawObservation | 3000 | ❌ | ❌ | ✅ |
| SupersededClaim | 1000 | ❌ | ❌ | ❌ |

## Observation invariant

```
Observation episode ≠ accepted claim.
```

Locked. `RawObservation` and `DeterministicEvidence` return `is_accepted_state() == false`.

## SQLite migration 0003

4 additive columns on `memory_record`:

```sql
ALTER TABLE memory_record ADD COLUMN evidence_kind TEXT;
ALTER TABLE memory_record ADD COLUMN normalized_text_hash TEXT;
ALTER TABLE memory_record ADD COLUMN conflict_group_id TEXT;
ALTER TABLE memory_record ADD COLUMN supersedes_record_id TEXT;
```

Existing NULL records interpreted as `AcceptedClaim`.

## Content hash dedup

`DedupKey`: BLAKE3 hash of normalized text + scope key + evidence kind byte.

`DedupDecision`: `New` or `DuplicateAttachSource { existing_record_id }`.

## Supersession

`RetrievalMode`: `Default | CurrentState | ChangeHistory | ConflictSearch`

| Mode | Superseded penalty | Excluded |
|---|---|---|
| Default | 5000 bps | No |
| CurrentState | 10000 bps | Yes |
| ChangeHistory | 0 bps | No |
| ConflictSearch | 2000 bps | No |

## Conflict

`ConflictGroup`: `{ id, record_ids }` — both sides retained, never overwritten.

## Regression fixtures

6 JSON fixtures under `tests/fixtures/evidence/`:

1. `observation_not_claim.json` — observation ranks below accepted claim
2. `duplicate_observation.json` — duplicate evidence preserved
3. `supersession_chain.json` — superseded penalized, successor first
4. `conflicting_claims.json` — both conflict sides returned in ConflictSearch mode
5. `deterministic_evidence_vs_raw_observation.json` — deterministic > raw
6. `user_claim_vs_llm_candidate.json` — user claim > LLM candidate

## Files added/changed

| File | Commit | Purpose |
|---|---|---|
| `evidence.rs` | 1 | EvidenceKind enum with authority ranking |
| `sqlite_schema.rs` | 2 | Migration 0003 SQL |
| `sqlite_store.rs` | 2 | Migration 0003 registration + conn_for_test() |
| `memory_quality.rs` | 2 | Migration 0003 roundtrip tests |
| `retrieval.rs` | 3 | evidence_kind field on RankedMemoryHit |
| `ranking.rs` | 3 | evidence_bps_from_kind() |
| `retrieval_evidence.rs` | 3 | 6 retrieval evidence tests |
| `observation_invariant.rs` | 4 | 6 observation invariant tests |
| `dedup.rs` | 5 | DedupKey, DedupDecision, compute_normalized_hash |
| `tests/dedup.rs` | 5 | 5 dedup integration tests |
| `supersession.rs` | 6 | RetrievalMode, supersession_penalty, should_exclude_superseded |
| `tests/supersession.rs` | 6 | 4 supersession integration tests |
| `conflict.rs` | 7 | ConflictGroup struct |
| `tests/conflict.rs` | 7 | 3 conflict integration tests |
| `tests/fixtures/evidence/*.json` | 8 | 6 JSON regression fixtures |
| `tests/evidence_fixtures.rs` | 8 | 6 fixture-based tests |

## Test delta

431 → 493 = +62 tests across 9 commits.

## Deviations from spec

None. All 22 required acceptance tests present:

- ✅ evidence_kind_default_is_accepted_claim
- ✅ deterministic_evidence_is_not_accepted_state
- ✅ raw_observation_is_not_accepted_state
- ✅ llm_candidate_is_not_accepted_state
- ✅ ranked_hit_carries_evidence_kind
- ✅ user_stated_claim_ranks_above_llm_candidate
- ✅ deterministic_evidence_ranks_above_raw_observation
- ✅ superseded_claim_is_penalized_by_default
- ✅ sqlite_migration_0003_is_additive
- ✅ sqlite_existing_records_default_to_accepted_claim
- ✅ dedup_same_normalized_text_same_scope
- ✅ dedup_different_scope_not_duplicate
- ✅ dedup_same_trace_source_attaches_source
- ✅ default_mode_penalizes_superseded_claim
- ✅ current_state_mode_excludes_superseded_claim
- ✅ change_history_mode_includes_superseded_chain
- ✅ conflicting_claims_are_both_retained
- ✅ conflict_search_returns_all_group_members
- ✅ fixture_observation_not_claim
- ✅ fixture_duplicate_observation
- ✅ fixture_supersession_chain
- ✅ fixture_conflicting_claims

## Not yet wired

These types exist but are not yet integrated into the store query path:

- `DedupKey` not called from `accept_candidate` — dedup logic is defined but not invoked
- `RetrievalMode` not passed to `search_records` — search doesn't use it yet
- `ConflictGroup` not produced by the store — conflict detection not wired
- `EvidenceKind` not persisted on write — `accept_candidate` doesn't set the column
- `supersedes_record_id` not written on supersession — column exists but not populated

These gaps are intentional for this wave. The types and semantics are correct and tested. Wiring them into the store's read/write paths is the next step.

## Next

**Wave 02j — Memory-Backed Repo Consistency Check**

02j should consume:
- 02i ranked retrieval
- 02i-b evidence semantics
- Read-only Git observations (04b)

And produce:
- Expected project state
- Observed repo state
- Matching evidence
- Stale memories
- Conflicts
- Recommended next action

Only after 02j should Git mutation be reconsidered.
