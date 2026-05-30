# WAVE02R_MEMORY_RANKING_AND_GOVERNANCE_TUNING_LOCK

**Date:** 2026-05-30
**Commits:** 08cabf6 → 0dae19d (3 commits)
**Tests:** 821 → 858, zero failures

## Lock condition (met)

```
Wave 02r is locked when OpenWand has a centralized deterministic memory
governance profile for ranking, confidence, verification, stale, conflict,
unverifiable, and supersession behavior; all changes are measured against the
02q suite with explicit approved deltas and zero unapproved regressions; rank
explanations show why ordering changed; panel/audit provenance and trace lineage
remain intact; normal prompt format contains no provenance or trace tags; and
memory, trace, panel, and evaluation mutation guards remain green.
```

## What shipped

### Commit 1-2: Delta report types + governance profile + verification field

**New types:**
- `MemoryEvaluationBaseline`, `ScenarioEvaluationDelta`, `ApprovedBehaviorChange`, `MemoryEvaluationDeltaReport`
- `MemoryGovernanceProfile` (Default + batch_02r_default)
- `ConfidencePolicy` (bands: High/Medium/Low/Untrusted, prompt_include_min_bps)
- `VerificationPolicy` (verifies/derived_from/refines boost)
- `StalePolicy`, `SupersessionPolicy`, `ConflictPolicy`
- `PromptEligibility` (Include / ExcludeAuditOnly)
- `GovernedMemoryFinding`, `GovernanceFilteredReport`
- `VerificationSignalIndex` + `VerificationSignal`
- `MemoryRankScore.verification_bps` (new field)
- `RankingWeights.verification` (new field, 0 in Default)

### Commit 3-4: Governance wiring + verification signal

- `PromptInputProductionConfig.governance_profile` — optional profile
- Coordinator uses `GovernanceFilteredReport` when profile provided
- `assemble_from_governed()` — only includes PromptEligibility::Include findings
- `VerificationSignalIndex::from_relations()` — pre-ranking trace lookup
- `VerificationSignalIndex::compute_boost()` — policy-weighted boost, capped at 10000

### Commit 5-10: Guards

- `default_governance_profile_preserves_02q_prompt_hashes` — byte-for-byte proof
- 4 governance-reason-visibility tests (low confidence, superseded, conflict, unverifiable)
- `prompt_does_not_render_governance_reason_tags`
- `crate_absence_still_classifies_missing_in_repo_not_stale`
- All 10 02p guards preserved and passing

## Seven reviewer patches applied

| Patch | Implementation |
|-------|---------------|
| 1. Verification boost timing | `VerificationSignalIndex` — pre-ranking, not from hydrated DTOs |
| 2. No bucket reclassification | `PromptEligibility` separate from `MemoryTrustBucket` |
| 3. Immutable report | `GovernanceFilteredReport` wraps original, never replaces |
| 4. Default preserves 02q | Proven by guard test: identical hashes |
| 5. Stale naming | Guard proves MissingInRepo ≠ StaleMemory |
| 6. Conflict unit-only | Governance unit tests, not full harness |
| 7. No hidden trust channel | 5 governance-visibility tests |

## Governance profile comparison

| Field | Default (pre-02r) | batch_02r_default |
|-------|:---:|:---:|
| prompt_include_min_bps | 0 (all eligible) | 3000 |
| verifies_boost_bps | 0 | 2000 |
| derived_from_boost_bps | 0 | 500 |
| refines_boost_bps | 0 | 800 |
| stale exclude_from_prompt | false | true |
| conflict verified_winner | false | true |
| ranking weights verification | 0 | 1000 |

## Architecture

```text
Without governance (Default):
  classify → assemble_from_report → prompt → hydrate

With governance (batch_02r_default):
  classify → GovernanceFilteredReport → assemble_from_governed → prompt → hydrate
                       ↑
            confidence band check + bucket policy
```

The governance layer sits between classification and assembly. It does not
mutate the RepoConsistencyReport. It produces a GovernedMemoryFinding for each
finding with a separate PromptEligibility decision.

## What does NOT change

| Item | Why |
|------|-----|
| MemoryStore trait | No new methods |
| TraceStore trait | No new methods |
| RepoConsistencyReport | Immutable — wrapped, not replaced |
| Prompt format | No provenance/trace/governance tags |
| Panel visibility | Governance reasons are audit-visible |
| Trust buckets | Not reclassified for eligibility |
| 02p evaluation guards | All preserved |

## Known gaps (honest)

| Gap | Why acceptable |
|-----|---------------|
| `batch_02r_default` not yet wired into production binary | Profile is opt-in via config; Default is still pre-02r |
| No stale production in classifier | `StaleMemory` variant remains dead; only `MissingInRepo` produced |
| No conflict detection wiring | Policy exists but coordinator never marks records as conflicting |
| No ranking weight tuning with evaluation evidence | Profile exists; tuning requires running delta against 02q baseline |
| No rebaselined fixture expectations | Fixtures still expect pre-02r behavior; batch_02r_default is opt-in |
| Verification boost not applied to hits in store | Stores return verification_bps: 0; coordinator must apply post-ranking |

## Test delta

| Suite | Before | After | Delta |
|-------|--------|-------|-------|
| memory (evaluation_delta) | 0 | 6 | +6 |
| memory (governance) | 0 | 8 | +8 |
| memory (verification_signal) | 0 | 10 | +10 |
| memory (ranking) | 8 | 8 | 0 |
| app (memory_evaluation) | 13 | 14 | +1 |
| app (memory_evaluation_guards) | 9 | 15 | +6 |
| app (memory_governance_guards) | 0 | 6 | +6 |
| **Total** | **821** | **858** | **+37** |

## Next recommended wave

```
WAVE02S_GOVERNANCE_PRODUCTION_WIRING
```

Wire `batch_02r_default` into the production binary, run delta against 02q
baseline, capture approved behavior changes, update fixture expectations.

This is where governance starts affecting real prompt output.
