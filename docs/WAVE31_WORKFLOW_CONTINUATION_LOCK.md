# Wave 31 — Workflow Continuation Readiness and Next-Action Proposal — LOCK

**Committed:** 6 commits
**Baseline:** 2284 tests (Wave 30 locked)
**Final:** ~2367 tests, zero failures

---

## What Shipped

### New Modules in `openwand-workflow`

| Module | Purpose |
|--------|---------|
| `workflow_continuation.rs` | DTOs: readiness, proposal, candidate, 17 predicates, evidence links, hardcoded false invariants |
| `workflow_continuation_validation.rs` | Content-addressed IDs, proposal hash, validation |
| `workflow_next_action_selector.rs` | 17 predicates + candidate selection + proposal builder |

### New Modules in `openwand-app`

| File | Purpose |
|------|---------|
| `workflow_continuation.rs` | Persistence under `workflow_continuation/readiness/` + `proposals/` |
| `ui/workflow_continuation_state.rs` | UI view helpers + safety warning |
| `ui/workflow_continuation_components.rs` | Desktop-gated placeholder |
| `main.rs` additions | CLI: `openwand workflow-continuation propose/show-readiness/show-proposal/latest` |

---

## Test Breakdown

| Area | Count |
|------|------:|
| DTO / validation | 13 |
| Selector / predicates | 17 |
| Proposal builder | 8 |
| Persistence / idempotency | 17 |
| CLI | 9 |
| UI state | 5 |
| Guard / no-mutation | 14 |
| **Total** | **83** |

---

## Central Invariant

```
Run revision is evidence.
Next-action proposal is evidence.
A proposed next action is not routing.
A proposed next action is not execution.
A continuation proposal is not workflow continuation.
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. NextActionRequestRemainsNonExecutable predicate | ✅ Predicate + `blocks_non_executable_action_request_violation` test |
| 2. CLI exposes readiness + proposal separately | ✅ `show-readiness` + `show-proposal` + `cli_show_readiness_roundtrips_record` + `cli_propose_no_eligible_action_outputs_readiness_id_not_proposal_id` |
| 3. NoEligibleAction idempotency | ✅ `no_eligible_action_cannot_duplicate_for_same_revision_with_different_key` + `no_eligible_action_can_repeat_after_revision_changes` |
| 4. Selector does not skip Running/Suspended | ✅ `selector_does_not_skip_running_stage_to_later_pending_stage` + `selector_does_not_skip_suspended_stage_to_later_pending_stage` |
| 5. Test count corrected | ✅ 83 total (13+17+8+17+9+5+14) |

---

## Four-Way Decision

| Decision | When |
|----------|------|
| ProposalReady | First pending stage with terminal deps and prepared action request |
| NoEligibleAction | All stages terminal |
| Blocked | Running/suspended stage, predicate failure, dependency failure |
| Inconclusive | Pending stage but no action request available |

---

## Hardcoded Invariants

```rust
creates_route = false
routes_action_now = false
executes_tool_now = false
mutates_workflow_state_now = false
```

---

## Honest Caveats

- Wave 31 proposes the next eligible action only — does not route, review, execute, or reconcile it.
- One candidate at a time. No multi-action selection.
- No retry/resume/scheduling/worker/queue.
- Selector does not skip Running/Suspended stages (Patch 4).
- Default CI remains provider-free and network-free.
