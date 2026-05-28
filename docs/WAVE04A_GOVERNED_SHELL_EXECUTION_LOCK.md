# WAVE04A_GOVERNED_SHELL_EXECUTION_LOCK

**Status:** ✅ LOCKED  
**Commit:** 84a9c39  
**Tests:** 389 passing, 0 failures  

## What shipped

`local__shell_exec` — the first real process-spawning tool, governed through the full approval spine.

## Components

| Component | File | Change |
|-----------|------|--------|
| Tool handler | `crates/tools/src/local.rs` | `shell_exec_handler`, `shell_exec_descriptor`, `validate_program_name`, `resolve_exec_working_dir`, `cap_byte_output`, `local_tools_with_shell_exec` |
| Policy correction | `crates/policy/src/builtin.rs` | `confirm_execute` rule: `High` → `Critical` |
| Session tests | `crates/session/tests/governed_shell_exec.rs` | 6 governance arc tests |

## Governance spine (proven)

```
policy Critical + Escalate
  → gate.evaluated (trace)
  → tool.suspended (trace, durable)
  → user decision
    → approve: tool.resumed → tool.called (durable) → spawn → tool.completed/tool.failed
    → reject:  tool.denied → no spawn
    → direct mode: ToolBlocked → no spawn, no suspension
```

## Three corrections applied

1. **Executor-side validation before spawn.** Program name and working directory validated inside `shell_exec_handler`. Invalid inputs produce `ToolResult::error()` with zero process spawn.
2. **Execute = Critical + Escalate.** `confirm_execute` rule in `builtin.rs` corrected from `High` to `Critical`.
3. **Timeout/cancel = explicit kill + wait/reap.** `cmd.spawn()` → `Option<Child>` → `tokio::select!` with three branches. No `cmd.output()`. Explicit `kill().await` + `wait().await` on timeout and cancellation.

## Safety properties

| Property | Mechanism | Test proof |
|----------|-----------|------------|
| Bare program names only | `validate_program_name` rejects `/`, `\`, `.`/`-` prefix | 5 unit tests |
| Working directory containment | `resolve_exec_working_dir` canonicalizes + `starts_with` check | handler integration |
| Output size bounded | `SHELL_OUTPUT_CAP_BYTES` (200 KiB) + `normalize_output` (50K chars) | `output_capping_truncates_large_output` |
| Timeout enforced | `tokio::select!` with `tokio::time::sleep` + `child.kill()` | `exec_tool_failure_surfaces_as_error_in_run` |
| Cancellation kills child | `ctx.cancellation.cancelled()` branch | handler code (tokio select) |
| No zombie processes | `child.wait()` after `child.kill()` in timeout/cancel branches | handler code |
| Approval denial prevents spawn | `tool.denied` trace, 0 executor calls | `exec_rejection_records_denied_no_execution` |
| Durable `tool.called` before spawn | `record_tool_called` → `ToolExecutor::execute` | `exec_approval_trace_order_*` |
| Terminal event mandatory | `tool.completed`/`tool.failed` after every execution | lifecycle scanner (03f) |

## Test delta

- Tools crate: +7 tests (program validation ×5, output capping, timeout clamping)
- Session crate: +6 tests (suspend, approve lifecycle, reject, allow, block, fail)
- Total: 376 → 389

## Constants

| Constant | Value | Purpose |
|----------|-------|---------|
| `SHELL_OUTPUT_CAP_BYTES` | 200 KiB | Byte-level output cap before UTF-8 conversion |
| `SHELL_DEFAULT_TIMEOUT_MS` | 30,000 | Default command timeout |
| `SHELL_MAX_TIMEOUT_MS` | 300,000 | Maximum allowed timeout (5 minutes) |

## Builder

```rust
pub fn local_tools_with_shell_exec() -> BuiltinToolProvider
```

Extends `batch2_local_tools()` with `shell_exec`. For Batch 3 tool sets.

## Milestone

This is the first tool where the governance spine protects **actual process spawning**. Previous waves validated the spine against mock tools. Wave 04a proves the spine works when the tool can cause real side effects.

## Next

Wave 04b or memory quality hardening — TBD.
