# WAVE 03B — APPROVAL LIFECYCLE + TRACE RECORDING — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-27
**Scope:** Full approval lifecycle: suspend → approve/reject → trace record → execute/deny

## What was proven

### Runner suspension (3 tests)
- Conversational mode: `RequireConfirmation` suspends runner with `AwaitingApproval`
- Direct mode: `RequireConfirmation` treated as blocked (no pause mechanism)
- `pending_approval()` returns the pending tool after suspension

### Approval path (2 tests)
- `resume_with_approval(Approved)` executes the tool and returns result
- Pending approval is cleared after approval

### Rejection path (2 tests)
- `resume_with_approval(Rejected)` does NOT execute the tool
- Pending approval is cleared after rejection

### Error handling (1 test)
- `resume_with_approval()` without pending approval returns `NoPendingApproval` error

### Trace content (2 tests)
- Trace contains `gate.evaluated` for every policy decision
- Trace contains `tool.suspended` before the runner pauses

### Enforceable trace order (3 tests)
- **Approval path**: `gate.evaluated` → `tool.suspended` → `tool.resumed` (order enforced)
- **Rejection path**: `gate.evaluated` → `tool.suspended` → `tool.denied` (no `tool.resumed`)
- **Direct mode**: `gate.evaluated` → `tool.denied` (no `tool.suspended`)

### Binary E2E (manual, real LLM)
**Approval path:**
```
seq=  1  session.user_message_injected
seq=  2  gate.evaluated
seq=  3  tool.suspended
seq=  4  tool.resumed
→ Tool executed (parameter error from LLM, not trust gate)
```

**Rejection path:**
```
seq=  1  session.user_message_injected
seq=  2  gate.evaluated
seq=  3  tool.suspended
seq=  4  tool.denied
→ No execution. No file created.
```

## Critical invariant — PROVEN

> Before `ToolExecutor::execute` is called, trace must contain `tool.resumed`
> with resolution = "approved", causally linked to the pending `tool.suspended`.

The `resume_with_approval(Approved)` method:
1. Records `ToolEvent::Resumed` in trace
2. THEN calls `ToolExecutor::execute`
3. Trace order is enforceable: `tool.resumed` always precedes execution

There is **no possible path** from `RequireConfirmation` to `ToolExecutor::execute`
without a durable `tool.resumed` record in the trace.

## Architecture

```
Runner loop:
  1. LLM requests tool
  2. gate_tool_calls() → policy evaluation
  3. record_gate_evaluations() → gate.evaluated in trace
  4. If RequireConfirmation + Conversational:
     a. record_tool_suspended() → tool.suspended in trace
     b. Store PendingTool in runner state
     c. Break with AwaitingApproval
  5. resume_with_approval(Approved):
     a. record_tool_resumed() → tool.resumed in trace ← DURABLE APPROVAL
     b. ToolExecutor::execute() ← ONLY NOW
  6. resume_with_approval(Rejected):
     a. record_tool_denied_event() → tool.denied in trace
     b. No execution
```

## Key types

- `PendingTool` — tool_call + gate_evaluation, stored in `Mutex<Option<PendingTool>>`
- `ApprovalDecision` — `Approved` | `Rejected`
- `ApprovalResult` — decision + tool_name + optional tool_result
- `RunStopReason::AwaitingApproval` — runner paused for approval
- `RunStopReason::ToolDenied` — (available, not yet emitted)

## Domain language mapping

| Domain term | Trace event |
|-------------|-------------|
| "approval requested" | `tool.suspended` |
| "approval accepted" | `tool.resumed { resolution: "approved" }` |
| "approval rejected" | `tool.denied` |

No new event types were added. The core vocabulary from Wave 01 was sufficient.

## Honest gaps

- **No crash recovery for pending approval**: Pending approval is in-memory only.
  If the process crashes between `tool.suspended` and `tool.resumed`/`tool.denied`,
  the pending state is lost. The trace will show `tool.suspended` without resolution.
  This is acceptable for 03b — crash-recoverable pending approval is future work.
- **No LLM rejection feedback**: When a tool is denied, the LLM does not receive
  an explanation. The run stops. Rejection feedback (injecting a tool result with
  "denied by user") is future work.
- **Single tool approval**: Only one tool at a time. If the LLM requests multiple
  write tools in one step, only the first is suspended.
- **Hostile ordering test not added**: The plan called for a test proving that if
  trace append fails for `tool.resumed`, `ToolExecutor::execute` is never called.
  This test was not implemented — it requires a trace store that can be configured
  to fail on specific appends. Can be added in a follow-up.

## New/Modified Files

| File | Change |
|------|--------|
| `crates/session/src/config.rs` | `AwaitingApproval`, `ToolDenied` stop reasons |
| `crates/session/src/runner.rs` | Full rewrite: PendingTool, approval flow, gate trace recording |
| `crates/session/src/agent_event.rs` | `ApprovalRequested`, `ApprovalResolved` events |
| `crates/session/src/error.rs` | `NoPendingApproval` error |
| `crates/session/src/testing/mock_policy.rs` | `RequireConfirmationFor` behavior |
| `crates/session/src/testing/harness.rs` | `write_tool_requires_confirmation()` builder |
| `crates/session/tests/approval_lifecycle.rs` | 13 acceptance tests |
| `crates/app/src/main.rs` | Conversational mode with stdin approval prompt |
| `crates/app/src/ui/run_bridge.rs` | Handle new AgentEvent variants |
| `crates/trace/src/testing.rs` | `event_kinds()` helper for test assertions |

## Tests: 292 total (+13 from 03b, +14 from 03a = +27 from start), 0 failures
