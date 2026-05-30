# WAVE02T_INTEGRATION_DELTA_PROOF_LOCK

**Date:** 2026-05-30
**Commits:** b7b3536 → 936e90e (4 commits)
**Tests:** 874 → 897, zero failures

## Lock condition (met)

```
Wave 02t is locked when an integration test using the real prompt-input
coordinator proves that a 2500 bps claim accepted by a lowered-threshold
InMemoryMemoryStore is retrieved by search_ranked, included in the prompt under
Default, excluded from the prompt under Batch02rDefault, remains audit-visible,
produces different concrete prompt hashes, while a high-confidence scenario
remains hash-stable and all governance, prompt-fork, audit, panel, memory,
trace, and evaluation guards remain green.
```

## Root cause of the 02s gap

```
InMemoryMemoryStore::accept_candidate had hardcoded CONFIDENCE_THRESHOLD = 0.7
  → rejected claims below 0.7 confidence
Batch02rDefault governance excludes below 3000 bps = 0.30 confidence
  → no confidence value exists in [0.30, 0.70) that passes both gates
  → the 02s delta harness seeded at 0.2, store rejected it, coordinator saw zero records
  → both profiles produced empty prompts, both hashes were empty
```

Fix: `InMemoryMemoryStore::with_confidence_threshold(0.1)` for integration tests only.

## What shipped

### Commit 1: Configurable acceptance threshold

- `InMemoryMemoryStore::new()` preserves 0.7 threshold (backward compatible)
- `InMemoryMemoryStore::with_confidence_threshold(threshold)` for tests
- 5 new tests proving threshold behavior

### Commits 2-3: Integration delta proof (THE CENTRAL PROOF)

**Test:** `low_confidence_delta_visible_through_real_coordinator`

```
Setup:
  Store threshold: 0.1
  Claim: "crate core exists"
  Confidence: 0.25 (2500 bps)
  Workspace: fixture with crate "core"

Path (both profiles):
  seed claim → list_active_records → search_ranked → classify → govern → assemble → prompt

Under Default (prompt_include_min_bps = 0):
  Supported finding → governance includes (2500 >= 0) → prompt contains claim → hash A

Under Batch02rDefault (prompt_include_min_bps = 3000):
  Supported finding → governance excludes (2500 < 3000) → prompt omits claim → hash B

Result: A ≠ B ✓
```

**Content-level assertions:**
- Default prompt contains "crate core" ✓
- Batch02rDefault prompt does NOT contain "crate core" ✓
- Batch02rDefault report findings still contain the excluded claim ✓

**High-confidence stability:**
- Claim at 0.9 confidence → identical prompts under both profiles ✓
- Claim at 0.9 confidence → included under both profiles ✓

### Commit 4: Regression classifier + guards

- `PromptInputDeltaClassification` enum (Approved/Unchanged/UnapprovedRegression)
- 6 classifier tests
- 5 no-behavior-change guards

## Integration fixtures

| Fixture | Confidence | Default | Batch02rDefault | Hash delta |
|---------|-----------|---------|-----------------|------------|
| "crate core exists" | 0.25 (2500 bps) | Included | Excluded (audit-only) | **Different** |
| "crate core exists" | 0.9 (9000 bps) | Included | Included | Identical |

## What did NOT change

| Item | Why |
|------|-----|
| Governance profile values | Same as 02r lock |
| Ranking weights | Same as 02r lock |
| Classifiers | No new stale/conflict detection |
| Prompt format | Same section structure |
| Panel DTOs | Same bucket enum |
| SQLite store threshold | Still 0.7 |
| Existing fixtures | Zero modifications |
| Production InMemoryMemoryStore | new() still uses 0.7 |

## Test delta

| Suite | Before | After | Delta |
|-------|--------|-------|-------|
| memory (in_memory) | 0 | 5 | +5 |
| app (integration_delta_proof) | 0 | 7 | +7 |
| app (integration_delta_guards) | 0 | 11 | +11 |
| **Total** | **874** | **897** | **+23** |

## Full test commands

```bash
cd crates/mcp-pool/tests/fixtures/echo-server && cargo build --release
cargo test --workspace --features "openwand-session/testing,openwand-session/sqlite-testing,openwand-memory/testing,openwand-memory/sqlite-testing"
```

Result: 897 tests, zero failures.

## Next recommended wave

The governance pipeline is now proven end-to-end. Options:

1. **Real LLM quality evaluation** with Qwen3 — measure whether governance tuning improves model behavior
2. **Rich text editing** (taino-edit-dioxus) for message composition
3. **Agent version as immutable artefact** — inspired by foundry-cicd analysis
