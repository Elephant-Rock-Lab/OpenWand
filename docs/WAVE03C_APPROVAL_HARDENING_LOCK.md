# WAVE 03C — APPROVAL HARDENING — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-27
**Scope:** Hostile ordering proof, multi-tool batch behavior, model continuation after rejection, unresolved suspension helper

## What was proven

### 1. Hostile ordering test (core invariant proof)

```
trace_failure_on_resumed_prevents_execution:
  - FailOnAppend trace store blocks tool.resumed append
  - resume_with_approval(Approved) fails with error
  - ToolExecutor::execute is NEVER called
  - Tool execution count unchanged
  - Trace shows gate.evaluated + tool.suspended but NOT tool.resumed
  - No tool.called or tool.completed events leaked
```

**Directly proves**: `tool.resumed` append must succeed before tool execution.
When the durable record fails, execution is prevented.

### 2. Multi-tool batch behavior (4 tests)

```
multi_tool_batch_only_first_suspends:
  - LLM emits [write_A, write_B, read_C]
  - Policy: write_A→RequireConfirmation, write_B→RequireConfirmation, read_C→Allow
  - Only write_A becomes pending (tool.suspended)
  - Trace has exactly 1 tool.suspended event
  - MockToolExecutor.execution_count() == 0

multi_tool_batch_allowed_tools_do_not_execute_during_suspension:
  - read_C is allowed by policy but NOT executed
  - Batch is frozen: no tool.called or tool.completed events

multi_tool_batch_after_approval_only_approved_tool_executes:
  - Approve write_A → only write_A executes
  - write_B and read_C are NOT executed
  - Mock executor calls: exactly 1, name == "local__write_A"

multi_tool_batch_deferred_tools_not_in_pending:
  - Only write_A in pending approval
  - After approval, pending is cleared
  - No mechanism to resume write_B (genuinely deferred)
```

**Proves**: Once any tool call suspends the run, no later tool call from the same LLM batch executes. The entire batch is frozen atomically.

### 3. Model continuation after rejection

```
denied_approval_reaches_llm_as_tool_error_and_model_continues:
  - Turn 1: LLM requests write tool → AwaitingApproval
  - Reject → denied result injected into Loro
  - Turn 2: LLM called again, second request messages contain:
    LlmMessage::Tool { is_error: true, content: "...denied..." }
  - Turn 2 completes with Natural stop reason
  - Mock LLM called exactly twice
```

**Proves**: The full rejection→continuation cycle. Denied tool result reaches the LLM as an error tool result. The model can continue and choose a different path.

### 4. Unresolved suspension helper

```
unresolved_suspensions_returns_suspended_without_resolution:
  - Run suspends, no approval/rejection
  - unresolved_suspensions() returns 1 item with correct tool_name

unresolved_suspensions_excludes_resumed_tools:
  - Approve → resumed → unresolved_suspensions() returns empty

unresolved_suspensions_excludes_denied_tools:
  - Reject → denied → unresolved_suspensions() returns empty

unresolved_suspensions_empty_when_no_suspensions:
  - Text-only run → empty
```

**Proves**: The helper correctly identifies crash-interrupted approvals (suspended without resolution) and excludes resolved tools.

## New infrastructure

| Component | Description |
|-----------|-------------|
| `MockLlmClient::multi_tool_then_stop()` | Emits multiple tool calls in one turn |
| `MockLlmClient::tool_then_text_after_denial()` | Two-turn mock: tool call then text |
| Turn-based script (vs. flat drain) | Mock LLM consumes one turn per `chat_stream` call |
| `MockPolicyEngine::RequireConfirmationForMany` | Require confirmation for multiple tool names |
| `MockToolExecutor::with_many()` | Multiple tools, all returning success |
| `SessionHarness::multi_tool_batch()` | Harness with 3-tool LLM + multi-confirmation policy |
| `SessionHarness::write_tool_then_text_after_denial()` | Two-turn rejection harness |
| `SessionRunner::unresolved_suspensions()` | Find suspended tools without matching resumed/denied |
| `UnresolvedSuspension` DTO | tool_call_id, tool_name, suspended_at |

## Modified files

| File | Change |
|------|--------|
| `crates/session/src/testing/mock_llm.rs` | Turn-based script, `multi_tool_then_stop`, `tool_then_text_after_denial` |
| `crates/session/src/testing/mock_policy.rs` | `RequireConfirmationForMany` variant |
| `crates/session/src/testing/mock_tools.rs` | `with_many()` constructor |
| `crates/session/src/testing/harness.rs` | `multi_tool_batch()`, `write_tool_then_text_after_denial()` |
| `crates/session/src/runner.rs` | `unresolved_suspensions()`, `UnresolvedSuspension` DTO |
| `crates/session/tests/approval_hardening.rs` | 14 tests (was 7) |
| `crates/trace/src/testing.rs` | `FailOnAppend<E>` (unchanged from earlier 03c) |
| `crates/trace/src/error.rs` | `AppendFailed` variant (unchanged from earlier 03c) |
| `tests/e2e_approval.sh` | Automated binary E2E (unchanged from earlier 03c) |

## Combined Wave 03 status

| Trust property | Evidence |
|---|---|
| LLM cannot directly mutate disk | ✅ Write goes through policy |
| Direct mode is not a policy bypass | ✅ RequireConfirmation blocks |
| Conversational pauses before mutation | ✅ AwaitingApproval + pending |
| Approval durable before execution | ✅ tool.resumed precedes execute |
| Trace failure prevents execution | ✅ Hostile ordering test |
| Rejection is safe | ✅ tool.denied, no execution |
| Rejection feedback to LLM | ✅ Error result in Loro AND in next LlmRequest |
| Model continues after rejection | ✅ Two-turn test proves Natural stop |
| Multi-tool batch is atomic | ✅ Only first suspends, rest frozen |
| Allowed tools don't execute during suspension | ✅ read_C test |
| Only approved tool executes | ✅ write_A only, write_B/read_C skipped |
| Trace order inspectable | ✅ gate/suspend/resume/deny assertions |
| Tool boundary contained | ✅ Filesystem validation |
| Unresolved suspensions detectable | ✅ Helper with 4 tests |
| Automated E2E | ✅ approve/reject scripts |
| Crash recovery | ❌ Out of scope (helper prepares ground) |
| Multi-tool batch approval | ❌ Single tool only (design choice, not gap) |

## Tests: 306 total, 0 failures

## Classification

```
Wave 03c: Approval hardening complete.
- Hostile failure proof (trace failure prevents execution)
- Multi-tool batch atomicity proven
- Model continuation after rejection proven
- Unresolved suspension helper for crash recovery preparation
```

## Honest gaps remaining

1. **Crash recovery for pending approval** — `unresolved_suspensions()` detects them but doesn't recover. Persistence in SQLite/Loro needed.
2. **Multi-tool batch approval** — When LLM emits multiple confirmation-requiring tools, only the first is suspended. The rest are silently deferred. No `tool.deferred` event type. This is a design choice, not a bug, but should be documented.
3. **Rich UI reconstruction** — Approval requests are in-memory only. A crash loses the context needed for UI reconstruction.
