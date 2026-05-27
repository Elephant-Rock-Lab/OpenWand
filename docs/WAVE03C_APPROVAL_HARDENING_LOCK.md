# WAVE 03C — APPROVAL HARDENING — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-27
**Scope:** Hostile ordering, rejection feedback, state cleanup, automated E2E

## What was proven

### 1. Hostile ordering test (most important)

```
trace_failure_on_resumed_prevents_execution:
  - FailOnAppend trace store blocks tool.resumed append
  - resume_with_approval(Approved) fails with error
  - ToolExecutor::execute is NEVER called
  - Tool execution count unchanged
  - Trace shows gate.evaluated + tool.suspended but NOT tool.resumed
```

This directly proves the core invariant from 03b:
> ToolExecutor::execute is only called after tool.resumed is durably recorded.

When the durable record fails, execution is prevented.

### 2. LLM rejection feedback

When a tool is rejected:
- A tool result message is injected into Loro state
- Content: "Tool 'X' was denied by user. Do not retry without asking differently."
- Marked as `is_error: true`
- Visible in conversation messages for subsequent runs

### 3. State cleanup after resolution

- Approval clears pending → new run succeeds without stale state
- Rejection clears pending → new run succeeds without stale state
- Direct mode never sets pending approval
- Pending remains suspended until explicitly resolved

### 4. Unresolved suspension detection

- After suspension, trace contains `tool.suspended` without `tool.resumed` or `tool.denied`
- This pattern identifies a crash-interrupted approval (future: crash recovery)

### 5. Automated binary E2E

```bash
tests/e2e_approval.sh reject   → ✅ PASS (no file created)
tests/e2e_approval.sh approve  → ✅ PASS (tool executed)
```

## New infrastructure

| Component | Description |
|-----------|-------------|
| `FailOnAppend<E>` | Trace store wrapper that fails on matching event_kind |
| `TraceError::AppendFailed` | New error variant for trace append failures |
| `MockToolExecutor::execution_count()` | Count of execute() calls for assertions |
| `tests/e2e_approval.sh` | Automated binary E2E for approve/reject |

## New/Modified Files

| File | Change |
|------|--------|
| `crates/trace/src/testing.rs` | `FailOnAppend<E>` wrapper |
| `crates/trace/src/error.rs` | `AppendFailed` variant |
| `crates/session/src/runner.rs` | Rejection feedback injection |
| `crates/session/src/testing/mock_tools.rs` | `execution_count()` |
| `crates/session/tests/approval_hardening.rs` | 7 tests |
| `tests/e2e_approval.sh` | Automated binary E2E |

## Combined Wave 03 status

| Trust property | Evidence |
|---|---|
| LLM cannot directly mutate disk | ✅ Write goes through policy |
| Direct mode is not a policy bypass | ✅ RequireConfirmation blocks |
| Conversational pauses before mutation | ✅ AwaitingApproval + pending |
| Approval durable before execution | ✅ tool.resumed precedes execute |
| Trace failure prevents execution | ✅ Hostile ordering test |
| Rejection is safe | ✅ tool.denied, no execution |
| Rejection feedback to LLM | ✅ Error result injected |
| Trace order inspectable | ✅ gate/suspend/resume/deny assertions |
| Tool boundary contained | ✅ Filesystem validation |
| Automated E2E | ✅ approve/reject scripts |
| Crash recovery | ❌ Out of scope |
| Multi-tool batch approval | ❌ Single tool only |

## Tests: 299 total (+7 hardening), 0 failures

## Classification

```
Wave 03: Consequential Trust Gate v1 proven with hostile failure proof.
```

OpenWand now has a write-capable tool, deterministic policy escalation,
mode-specific approval behavior, trace-recorded approval governance,
safe rejection with LLM feedback, and proof that trace failure prevents execution.
