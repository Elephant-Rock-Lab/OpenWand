# Wave 33 — Reviewed Next-Action Routing Gate — LOCK

**Committed:** 6 commits
**Baseline:** 2452 tests (Wave 32 locked)
**Final:** ~2536 tests, zero failures

---

## What Shipped

### New Module in `openwand-workflow`

| Module | Purpose |
|--------|---------|
| `workflow_next_action_routing_gate.rs` | Routing DTOs, 31 predicates, evaluation gate |

### New Modules in `openwand-app`

| File | Purpose |
|------|---------|
| `workflow_next_action_routing.rs` | Route adapter + persistence under `workflow_next_action_routing/` |
| `ui/workflow_next_action_routing_state.rs` | UI view helpers + safety warning |
| `ui/workflow_next_action_routing_components.rs` | Desktop-gated placeholder |
| `main.rs` additions | CLI: `workflow-next-action-routing route/show/latest` |

---

## Test Breakdown

| Area | Count |
|------|------:|
| DTO / Validation | 6 |
| Predicate Gate | 23 |
| Route Integration | 11 |
| Persistence / Idempotency | 6 |
| CLI | 6 |
| UI State | 4 |
| Guard / No-Mutation | 14 |
| **Total** | **70** |

---

## Central Invariant

```
Routing readiness is not routing.
A reviewed routing-readiness record is not execution.
The routing gate may create one route record.
The route still enters the existing SessionRunner routing path.
Workflow still does not execute tools directly.
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. `ProposalReviewReadinessCrossReferencesMatch` predicate + test | ✅ 31st real predicate, `blocks_proposal_review_readiness_cross_reference_mismatch` |
| 2. `expected_action_request_hash` + hash predicates + tests | ✅ Added to request DTO, `ActionRequestHashMatchesReadiness`, `ActionRequestHashMatchesRequest` predicates, CLI `--expected-action-request-hash` |
| 3. `route_next_action_via_existing_workflow_action_route()` adapter | ✅ Named adapter, `next_action_routing_app_uses_existing_workflow_action_routing_path` + `next_action_routing_app_does_not_duplicate_action_route_persistence_logic` guard tests |
| 4. Session boundary explicit | ✅ `session_bridge_available=false`, `session_runner_available=false` in adapter, `routing_gate_does_not_call_session_runner_directly` + `routing_gate_records_session_effects_only_through_created_route_record` tests |

---

## Routing Gate Flow

```
WorkflowRoutingReadinessRecord (Ready)
  → WorkflowNextActionRoutingRequest (5 hashes)
    → 31 predicates revalidate full chain
      → Routed → route_next_action_via_existing_workflow_action_route()
                    → builds WorkflowActionRouteRequest from preview
                    → calls existing evaluate_action_route gate
                    → persists via existing workflow_action_routes path
                    → links created_route_id in routing record
      → Blocked → no route created
```

---

## Key Boundary

Wave 33 routes exactly one reviewed-ready next action through the existing path. It does NOT:
- Call SessionRunner directly (session_bridge_available=false)
- Execute tools
- Resolve approvals
- Append trace
- Mutate memory, session state, or workflow run state
- Duplicate route creation logic (delegates to existing path)

---

## Honest Caveats

- The adapter sets `session_bridge_available=false` and `session_runner_available=false` — the existing route gate will produce a `Blocked` route record for the bridge/runner predicates. This is correct: the route record exists as evidence linkage, not as a live session trigger.
- A "Routed" next-action routing record means "predicates passed and route record created," not "session turn was executed."
- No retry/resume/scheduling/worker/queue.
- Default CI remains provider-free and network-free.
