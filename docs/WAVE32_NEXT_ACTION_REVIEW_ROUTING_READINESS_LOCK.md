# Wave 32 — Next-Action Proposal Review and Routing Readiness — LOCK

**Committed:** 6 commits
**Baseline:** 2367 tests (Wave 31 locked)
**Final:** ~2457 tests, zero failures

---

## What Shipped

### New Modules in `openwand-workflow`

| Module | Purpose |
|--------|---------|
| `workflow_next_action_review.rs` | Review DTOs (Approved/Rejected/ChangesRequested), Feedback, review hash, validation |
| `workflow_routing_readiness.rs` | Readiness DTOs, RouteRequestPreview (descriptive only), 25 predicates |
| `workflow_routing_readiness_gate.rs` | Full revalidation chain from proposal through review through revision |

### New Modules in `openwand-app`

| File | Purpose |
|------|---------|
| `workflow_next_action_review.rs` | Review persistence under `workflow_next_action_reviews/` |
| `workflow_routing_readiness.rs` | Readiness persistence under `workflow_routing_readiness/` |
| `ui/workflow_routing_readiness_state.rs` | UI view helpers + safety warning |
| `ui/workflow_routing_readiness_components.rs` | Desktop-gated placeholder |
| `main.rs` additions | CLI: review + routing readiness commands |

---

## Test Breakdown

| Area | Count |
|------|------:|
| Review DTO / validation | 8 |
| Readiness DTO | 7 |
| Predicate gate | 21 |
| Persistence / idempotency | 18 |
| CLI | 10 |
| UI state | 6 |
| Guard / no-mutation + dep guard | 15 |
| **Total** | **85** |

---

## Central Invariant

```
Next-action proposal review is not routing.
Routing readiness is not routing.
A reviewed next action is not permission to execute.
A Ready routing-readiness record is not a route record.
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. `expected_review_hash` + `ReviewHashMatchesRequest` | ✅ Added to request DTO, predicate, `blocks_review_hash_mismatch` test |
| 2. No-route-record proof | ✅ `ready_routing_readiness_does_not_create_workflow_action_route_record` + `route_request_preview_has_no_route_id` |
| 3. Workflow crate dep guard | ✅ `workflow_crate_dependency_guard_still_allows_only_6_deps` confirms exactly 6 |
| 4. File naming `workflow_*` | ✅ All files use `workflow_*` prefix |

---

## Review Decision Flow

```
WorkflowNextActionProposal (from Wave 31)
  └─ WorkflowNextActionReview (approve/reject/request-changes)
       ├─ Approved → eligible for routing readiness
       ├─ Rejected → blocks readiness, requires feedback.blocking_reasons
       └─ ChangesRequested → blocks readiness, requires feedback.requested_changes
```

---

## Routing Readiness Flow

```
WorkflowRoutingReadinessRequest (proposal + review + hashes)
  → 25 predicates revalidate full chain
    → Ready → WorkflowRoutingReadinessRecord + RouteRequestPreview (descriptive_only=true)
    → Blocked → hash mismatch, rejected review, invalid candidate
    → Inconclusive → missing evidence
```

---

## Key Boundary

Wave 32 reviews proposals and evaluates routing readiness. It does NOT:
- Route actions
- Create route records
- Create session turns
- Resolve approvals
- Execute tools
- Append trace
- Mutate memory, session state, or workflow run state

---

## Honest Caveats

- Wave 32 reviews and evaluates readiness — it does not route.
- A "Ready" record is evidence, not permission to execute.
- RouteRequestPreview is descriptive only (`descriptive_only=true, creates_route_now=false`).
- No retry/resume/scheduling/worker/queue.
- Default CI remains provider-free and network-free.
