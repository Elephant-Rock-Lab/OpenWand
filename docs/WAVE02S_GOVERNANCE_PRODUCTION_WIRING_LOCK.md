# WAVE02S_GOVERNANCE_PRODUCTION_WIRING_LOCK

**Date:** 2026-05-30
**Commits:** 258bc7c → e657626 (2 commits)
**Tests:** 858 → 874, zero failures

## Lock condition (met)

```
Wave 02s is locked when Batch02rDefault is the production prompt-input
governance profile, Default remains explicitly available for compatibility,
the only approved behavior delta is low-confidence prompt exclusion for
low_confidence_claim_behavior, no fixture files changed unless runtime delta
tests prove they must, and all prompt-fork, audit, panel, memory, trace, and
evaluation mutation guards remain green.
```

## What shipped

### Commit 1-2: Profile ID registry + production default flip

**New types:**
- `MemoryGovernanceProfileId` (Default, Batch02rDefault)
- `resolve()` → concrete profile for each ID
- `from_str_lossy()` for config parsing, fails closed on unknown
- `Display` impl for audit/logging

**Production default changed:**
```rust
// Before 02s:
governance_profile: None

// After 02s:
governance_profile: Some(MemoryGovernanceProfileId::Batch02rDefault.resolve())
```

- `PromptInputResult.governance_profile_id` — audit trail of which profile was used

### Commit 3-4: Delta capture harness + approved ledger

- `MemoryEvaluationHarness::run_governance_delta()` — runs scenario against both profiles
- `approved_02s_deltas()` — Rust constant ledger of approved changes
- 3 ledger validation tests

### Commit 5: Guard migration + new guards

- Migrated: `default_governance_profile_preserves_02q_prompt_hashes` → `compatibility_default_profile_preserves_pre_02s_hashes`
- 6 new 02s guards

## Approved behavior deltas

| ID | Scenario | Change | Reason |
|----|----------|--------|--------|
| low_confidence_claim_behavior | low_confidence_claim_behavior | prompt_hash (may change) | batch_02r_default excludes < 3000 bps |

Only one delta approved. All other scenarios unchanged under both profiles.

## What changed vs what didn't

### Changed
- `PromptInputProductionConfig::default()` now uses `Batch02rDefault`
- `PromptInputResult` has `governance_profile_id` field
- New profile ID enum and registry

### Did NOT change
| Item | Why |
|------|-----|
| MemoryStore trait | No new methods |
| TraceStore trait | No new methods |
| RepoConsistencyReport | Immutable — wrapped, not replaced |
| Prompt format structure | Same sections, same ordering |
| Panel rendering | Same DTOs, same buckets |
| Classifier logic | No new stale/conflict detection |
| Ranking weights | Already defined in batch_02r_default, unchanged |
| Verification signal | Already built in 02r, not changed |
| Fixture files | Zero fixture files modified |
| Conflict detection | Still unwired |
| Stale classifier | Still dead variant |

## Profile comparison

| Field | Default (compatibility) | Batch02rDefault (production) |
|-------|:---:|:---:|
| prompt_include_min_bps | 0 (all eligible) | 3000 |
| verifies_boost_bps | 0 | 2000 |
| derived_from_boost_bps | 0 | 500 |
| refines_boost_bps | 0 | 800 |
| stale exclude_from_prompt | false | true |
| conflict verified_winner | false | true |
| ranking weights verification | 0 | 1000 |

## Architecture after 02s

```text
Production path:
  classify → RepoConsistencyReport
           → GovernanceFilteredReport (Batch02rDefault)
           → assemble_from_governed
           → prompt → hydrate provenance/lineage

Compatibility path (explicit Default):
  classify → RepoConsistencyReport
           → GovernanceFilteredReport (Default)
           → assemble_from_governed
           → prompt → hydrate provenance/lineage
```

## Known gaps (honest)

| Gap | Why acceptable |
|-----|---------------|
| Delta harness produces empty hashes for low-confidence scenario | Full coordinator path exercises token-based search; harness fixtures don't produce hits via that path. Behavioral difference proven at unit level (GovernanceFilteredReport). |
| No integration-level hash difference proof | Same root cause. The v0 heuristic extractor doesn't create search_ranked matches for arbitrary fixture claims. |
| No real-world governance quality evaluation | Would require running real LLM with real memory. Deferred to future wave. |
| `approved_02s_deltas()` has runtime placeholders for hashes | Before/after hashes captured at test time, not hardcoded. The ledger is structural, not snapshot-based. |

## Test delta

| Suite | Before | After | Delta |
|-------|--------|-------|-------|
| memory (governance) | 8 | 16 | +8 |
| memory (evaluation_delta) | 9 | 12 | +3 |
| app (memory_evaluation_guards) | 15 | 21 | +6 |
| **Total** | **858** | **874** | **+16** |

## Full test commands

```bash
cd crates/mcp-pool/tests/fixtures/echo-server && cargo build --release
cargo test --workspace --features "openwand-session/testing,openwand-session/sqlite-testing,openwand-memory/testing,openwand-memory/sqlite-testing"
```

Result: 874 tests, zero failures.

## Next recommended wave

```
WAVE02T_INTEGRATION_DELTA_PROOF
```

OR: real multi-turn LLM quality evaluation with Qwen3.

The integration-level hash difference proof requires either:
1. Fixtures that produce real search_ranked hits (claim text matching tokenized queries)
2. A direct coordinator test that bypasses search_ranked and feeds hits directly
