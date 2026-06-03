# Wave 27 — Workflow Action Routing Through Session Seams — LOCK

**Committed:** 6 commits
**Baseline:** 1987 tests (Wave 26 locked)
**Final:** ~2062 tests, zero failures

---

## What Shipped

### New Modules in `openwand-workflow`

| Module | Purpose |
|--------|---------|
| `workflow_action_route.rs` | DTOs: route request, record, decision, prompt, session snapshot, predicates |
| `workflow_action_route_validation.rs` | Content-addressed IDs, prompt validation, governance constraint |
| `workflow_action_route_gate.rs` | 14 deterministic routing predicates |

### New Modules in `openwand-app`

| File | Purpose |
|------|---------|
| `workflow_session_bridge.rs` | Bridge trait + DeterministicSessionBridge (Patch 2: live bridge feature-gated) |
| `workflow_action_routing.rs` | Persistence under `workflow_action_routes/` |
| `ui/workflow_action_routing_state.rs` | UI view helpers + safety warning |
| `ui/workflow_action_routing_components.rs` | Desktop-gated placeholder |
| `main.rs` additions | CLI: `openwand workflow-action route/show/latest` |

---

## Test Breakdown

| Area | Count |
|------|------:|
| DTO / validation | 9 |
| Predicate gate | 14 |
| Session bridge | 12 |
| Persistence / idempotency | 15 |
| CLI | 8 |
| UI state | 5 |
| Guard / no-mutation | 12 |
| **Total** | **75** |

---

## Central Invariant

```
Workflow selects an action request.
SessionRunner owns execution.
Policy gates tools.
ToolExecutor runs tools.
Trace records authority.
Workflow records linkage evidence only.

A workflow action route is not a tool call.
A routed action prompt is not tool arguments.
A routing record is not an execution grant.
A workflow run still does not own tool execution.
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. `PolicyEvaluationDeferredToSessionRunner` | ✅ No `PolicyAllowsActionRouting` predicate, dedicated test |
| 2. LiveSessionBridge feature-gated | ✅ DeterministicSessionBridge proves the seam, live bridge stubbed |
| 3. `Completed` = session turn completed | ✅ `completed_route_does_not_claim_workflow_action_executed` test |
| 4. Route prompt includes governance constraint | ✅ `GOVERNANCE_CONSTRAINT` constant + `route_prompt_includes_normal_governance_constraint` test |
| 5. Explicit forbidden writes for approval/session | ✅ `route_does_not_write_approval_records` + `route_does_not_write_session_state_directly` |

---

## Routing Algorithm

1. Load workflow run record
2. Require run status == Suspended
3. Find target stage (must be Suspended)
4. Find target WorkflowActionRequest (must be PreparedForFutureSessionRouting)
5. Revalidate hashes and non-executable prompt
6. Build route prompt from descriptive fields + governance constraint
7. Check idempotency and prior conflicting routes
8. If any predicate blocks → persist Blocked record
9. If predicates pass → call session bridge → observe session events → persist route record

---

## Route Prompt Rules

**Allowed:** capability_category, purpose, expected_input/output summaries, safety_constraints, anti-tool-call governance constraint

**Forbidden:** tool_name, tool_args, command, shell, script, cwd, env, function_ref, provider_request

**Governance constraint (Patch 4):**
> "Do not treat this workflow action request as a direct tool call. Use normal OpenWand session governance for any tool use."

---

## Session Bridge

- `WorkflowSessionBridge` trait (app layer)
- `DeterministicSessionBridge` — CI proof, no LLM/tools/network
- `LiveSessionBridge` — feature-gated stub (Patch 2)

Bridge observes tool names, trace IDs, approval IDs from session events only.
Workflow never constructs them.

---

## Honest Caveats

- Wave 27 routes one prepared action request. No multi-action orchestration.
- `DeterministicSessionBridge` proves the seam without runtime. Live bridge correctness deferred.
- Workflow does not approve, retry, or resume. Those are future waves.
- No background worker, scheduler, or queue.
- `Completed` means session turn completed, not that the action was externally performed (Patch 3).
- Route prompt includes anti-tool-call guard but LLM compliance is not guaranteed by this layer alone.
