# Wave 29 — Workflow Approval Resume and Action Outcome Linkage — LOCK

**Committed:** 6 commits
**Baseline:** 2103 tests (Wave 28 locked)
**Final:** ~2188 tests, zero failures

---

## What Shipped

### New Modules in `openwand-workflow`

| Module | Purpose |
|--------|---------|
| `workflow_action_outcome.rs` | DTOs: outcome request, record, approval resolution, outcome snapshot, 16 predicates |
| `workflow_action_outcome_validation.rs` | Content-addressed IDs, rationale validation |
| `workflow_action_outcome_gate.rs` | 16 deterministic outcome predicates |

### New Modules in `openwand-app`

| File | Purpose |
|------|---------|
| `workflow_approval_bridge.rs` | Bridge trait + DeterministicApprovalBridge + LiveApprovalBridge |
| `workflow_action_outcome.rs` | Persistence under `workflow_action_outcomes/` |
| `ui/workflow_action_outcome_state.rs` | UI view helpers + safety warning |
| `ui/workflow_action_outcome_components.rs` | Desktop-gated placeholder |
| `main.rs` additions | CLI: `openwand workflow-action-outcome resolve/show/latest` |

---

## Test Breakdown

| Area | Count |
|------|------:|
| DTO / validation | 14 |
| Predicate gate | 17 |
| Approval bridge | 16 |
| Persistence / idempotency | 18 |
| CLI | 8 |
| UI state | 6 |
| Guard / no-mutation | 12 |
| **Total** | **91** |

---

## Central Invariant

```
Workflow may correlate approval outcome.
SessionRunner owns approval resolution.
Approval governance owns approval state.
Policy gates tools.
ToolExecutor runs tools.
Trace records authority.
Workflow records linkage evidence only.
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. Bridge trait takes only request, not runner | ✅ `resolve_workflow_routed_approval(&request)` — runner owned at construction |
| 2. Production bridge separate from SessionHarness | ✅ `new(runner, trace)` vs `from_harness()` documented test-only |
| 3. Resolution maps to API input only | ✅ `to_approval_decision()` returns `ApprovalDecision`, never constructs records |
| 4. RouteHasExactlyOnePendingApproval | ✅ Predicate 10 + `blocks_route_without_pending_approval` + `blocks_route_with_ambiguous_pending_approval` |
| 5. Session-state forbidden write | ✅ `outcome_does_not_write_session_state_directly` |

---

## Approval Resolution Flow

```
WorkflowActionOutcomeRequest (Approve/Reject + rationale)
  → 16 predicates validate linkage
    → WorkflowApprovalBridge::resolve_workflow_routed_approval(&request)
      → LiveApprovalBridge maps to ApprovalDecision (Patch 3)
      → SessionRunner::resolve_approval(decision, config)
      → Observes ApprovalResult + AgentEvent stream
      → Scans trace store scoped to session
    ← WorkflowSessionActionOutcomeSnapshot
  → WorkflowActionOutcomeRecord persisted
```

---

## Key Boundary

Wave 29 persists outcome linkage. It does NOT:
- Advance workflow stages based on outcomes
- Create approval records
- Mutate pending approval state
- Execute tools
- Append trace
- Write memory

A future wave reconciles outcome records into workflow-run stage progression.

---

## Honest Caveats

- Wave 29 links one approval outcome per route. No multi-action orchestration.
- Outcome records do not advance workflow stages.
- DeterministicApprovalBridge proves the seam. LiveApprovalBridge uses SessionHarness fixtures.
- No retry, resume, scheduling, or autonomous continuation.
- Default CI remains provider-free and network-free.
