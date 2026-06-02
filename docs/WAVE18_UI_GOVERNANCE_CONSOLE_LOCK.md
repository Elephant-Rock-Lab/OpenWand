# Wave 18: UI Governance Console — Lock

**Commit:** `af3ea89`
**Date:** 2026-06-03
**Status:** LOCKED
**Tests:** 1461 total, zero failures

## Scope

Surface the full governed execution chain in the Dioxus UI. Allow human review actions through existing governed modules. Display predicate/evidence state. Prove the UI cannot bypass backend governance.

## Invariant

```
UI observes evidence.
UI submits review intent.
Existing governed modules decide and persist.
UI never becomes execution authority.
```

## Module Boundary

```
Wave 10–17a: governed backend/eval spine
Wave 18: UI governance console and UI action adapters
```

New files in `crates/app/src/ui/`:
- `governance_state.rs` — state projection DTOs and loader
- `governance_actions.rs` — UI action adapters and GovernanceActionResult
- `governance_components.rs` — pure view-model helpers and Dioxus render functions

## State Model

`GovernanceConsoleState` is a read-only projection from existing `eval_reports/` persistence. It contains:
- 8 optional `GovernanceRecordSummary` slots (local proposal through push execution)
- Predicates extracted from verification, readiness, and push execution records
- Feedback extracted from local and push reviews
- Chain consistency warnings when cross-record IDs don't align

No new persistence format. No cached UI projection file. State is rebuilt on every load.

## Action Model

`GovernanceUiAction` maps to existing governed review functions only:
- Local proposal reviews → Wave 12 `build_proposal_review()` + `save_proposal_review()`
- Push proposal reviews → Wave 16 `build_push_proposal_review()` + `save_push_proposal_review()`
- Refresh → read-only reload

`GovernanceActionResult` carries `creates_execution_grant: false` and `execution_allowed_now: false` on every variant.

## View-Model Helpers

Pure functions tested without Dioxus:
- `record_card_lines()` → `Vec<String>`
- `predicate_panel_rows()` → `Vec<PredicateRow>`
- `feedback_panel_rows()` → `Vec<FeedbackRow>`
- `safety_banner_text()` → stable string
- `overview_status_lines()` → `Vec<String>`

Dioxus render functions consume these helpers. No brittle HTML assertions in tests.

## Chain Consistency Warnings (Patch 4)

The loader checks 7 ID linkage edges and populates `chain_warnings` when:
- Local review → local proposal mismatch
- Local execution → proposal/review mismatch
- Verification → execution mismatch
- Push readiness → verification mismatch
- Push proposal → readiness mismatch
- Push review → push proposal mismatch
- Push execution → push proposal/review mismatch

This prevents the UI from displaying "latest everything" as if the chain is coherent.

## Rendering

Dioxus render functions and view-model helpers (not stateful components). The safety banner always displays:

> This console reviews and displays governed records. Execution still goes through the backend gates. UI approval is not execution.

## What Did Not Ship

No new git operations, direct git execution, shell execution, push execution, local commit execution, direct persistence writes, execution grant creation, policy override, new proposal/eval semantics, real LLM session UX, or workflow engine UI.

## Test Coverage (34 tests)

- **State projection** (13): full chain, missing records, linked IDs, hashes, feedback, predicates, chain gaps, projection-only, serde, empty dir, 3 chain mismatch warnings
- **View-model helpers** (10): card lines status/hash/linked IDs, predicate rows, feedback rows, overview statuses, safety banner, missing state, CLI/UI equivalence, render without panic
- **Action results** (2): no execution grant, no execution allowed now
- **Guards** (8): no process::Command, no git backend, no push backend, no execution backend, no shell, no direct review construction, no direct execution construction, no trace/memory mutation
- **Runtime** (1): refresh is no-op, review persists only review record

## Honest Caveats

Wave 18 productizes governance visibility and review actions. It does not productize real LLM sessions, provider matrix, memory interaction, or workflow spawning.

The UI console is an observer and review-intent surface. It does not execute commits, push remotes, mutate git, override policy, or create execution grants.
