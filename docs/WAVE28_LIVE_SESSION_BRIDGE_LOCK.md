# Wave 28 — Live Session Bridge Runtime Proof — LOCK

**Committed:** 4 commits
**Baseline:** 2064 tests (Wave 27 locked)
**Final:** ~2101 tests, zero failures

---

## What Shipped

### Modified Files

| File | Change |
|------|--------|
| `crates/app/src/workflow_session_bridge.rs` | `LiveSessionBridge` impl with event collection, status mapping, trace scoping |
| `crates/app/src/ui/workflow_action_routing_state.rs` | Bridge status helper, updated safety warning |

### New Files

| File | Purpose |
|------|---------|
| `crates/app/tests/workflow_live_session_bridge.rs` | 17 tests: runtime scenarios + event mapping + regression |
| `crates/app/tests/workflow_live_route_integration.rs` | 9 tests: full route-to-persist through live bridge |
| `crates/app/tests/work_live_session_bridge_guards.rs` | 11 guard tests |

---

## Test Breakdown

| Area | Count |
|------|------:|
| Runtime scenarios | 12 |
| Route integration | 9 |
| Guard / no-mutation | 11 |
| UI bridge status | 2 |
| Regression | 5 |
| **Total** | **39** |

---

## Central Invariant

```
Workflow submits a descriptive route prompt.
SessionRunner owns the turn.
Policy gates tools.
ToolExecutor runs tools.
Trace records authority.
Workflow observes session-produced outcomes only.
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. Production constructor separate from SessionHarness | ✅ `new()` takes generic runner+trace, `from_harness()` documented test-only |
| 2. Exhaustive AgentEvent variant coverage | ✅ Match covers all 8 variants, `live_bridge_covers_all_current_agent_event_variants` test |
| 3. Safe runtime context handling | ✅ Dedicated `Runtime::new()` per call, `live_bridge_does_not_panic_in_configured_runtime_context` test |
| 4. Trace IDs scoped to routed session | ✅ `TraceQuery { stream_id: Some(session_stream_id) }`, `live_bridge_ignores_unrelated_trace_ids` test |

---

## LiveSessionBridge Architecture

```
LiveSessionBridge::route_action_to_session(prompt)
  → creates tokio::Runtime (Patch 3)
  → runner.run_turn(prompt.to_session_instruction(), config)
  → collects AgentEvent stream
  → maps events to WorkflowSessionRouteSnapshot
  → scans trace store scoped to session_id (Patch 4)
  ← returns snapshot
```

Event mapping (Patch 2: all current AgentEvent variants):
- `RunStarted` → session_id
- `ToolCallStarted` → tool_call_id, tool_name_observed_from_session
- `ApprovalRequested` → pending_approval_id
- `RunCompleted` → session_status (from stop_reason)
- `PhaseEntered`, `TextDelta`, `ToolCallCompleted`, `ApprovalResolved` → observed but no snapshot fields

---

## Runtime Scenarios Proven

| Scenario | Bridge Input | Session Outcome | Route Status |
|----------|-------------|-----------------|-------------|
| Text-only completion | text prompt | MockLlmClient text response | Completed |
| Tool approval suspension | text prompt | MockLlmClient emits tool call, policy requires confirmation | SuspendedForApproval |
| Tool completion | text prompt | MockLlmClient emits tool call, policy allows, MockToolExecutor returns result | Completed |
| Session error | max-steps edge case | SessionRunner returns error summary | Failed |

---

## Honest Caveats

- Live bridge proven with deterministic `SessionHarness` fixtures only. Real LLM provider routing not tested.
- Sync trait requires dedicated `Runtime::new()` per call. A future wave may make the trait async (Patch 3).
- `from_harness()` is test-only but not `#[cfg(test)]` gated (integration tests need it). Documented as test-only (Patch 1).
- Workflow does not retry, resume, or autonomously continue after observing session outcomes.
- Default CI remains provider-free and network-free.
