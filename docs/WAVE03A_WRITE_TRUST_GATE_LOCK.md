# WAVE 03A — CONTROLLED WRITE THROUGH TRUST GATE — LOCK

**Status:** ✅ COMPLETE (Direct mode proven)
**Date:** 2026-05-27
**Scope:** Write tool implementation, trust gate behavior, Direct mode blocking

## What was proven

### Tool layer (9 tests)
- `local__file_write` creates new files within working directory
- Refuses absolute paths (Windows + Unix)
- Refuses parent directory traversal (`..`)
- Refuses symlink escape (canonicalize parent, check prefix)
- Refuses overwrite by default (must set `overwrite: true`)
- Allows overwrite when explicitly requested
- Refuses to write to directories
- Enforces 1 MB size limit
- Creates parent directories automatically
- Declared effect: `ToolEffect::Write`

### Policy gate (5 tests)
- Write-effect tools require `ConfirmationLevel::Approve` in Conversational mode
- Direct mode: write tools hit `RequireConfirmation`, which runner treats as blocked
- Read/search tools still pass through in all modes
- Unknown-effect tools are blocked
- Delete-effect tools are blocked (no rule allows them)

### Binary E2E (manual)
```
$ openwand "Create a file called hello.txt with the content 'Hello from OpenWand'"
  Stop reason: ToolBlocked
  Tools called: 0
  (hello.txt does not exist)
```

The LLM requested `local__file_write`, the policy gate returned `RequireConfirmation`,
the runner blocked it in Direct mode, and **no disk mutation occurred**.

## What was NOT proven (honest gaps)

- **Conversational mode approval flow**: The runner currently treats `RequireConfirmation` as blocked. There is no pause-resume mechanism for user approval. This requires runner architecture changes (suspending the agent loop, waiting for user input, resuming).
- **Gate events in trace**: `record_blocked_tools` is a no-op. Gate decisions are not recorded in the trace. The plan called for `approval_event_precedes_tool_called_event` — this is not implemented.
- **LLM rejection feedback**: When a tool is blocked, the LLM does not receive feedback explaining why. The run just stops with `ToolBlocked`.
- **Conversational mode binary E2E**: No automated test proves the approval flow because it doesn't exist yet.

## Architecture

```
Tool layer:
  local__file_write
    → validate_write_path() (no absolute, no .., no symlink escape)
    → size limit (1 MB)
    → overwrite guard
    → create parent dirs
    → write file

Policy layer:
  ToolEffect::Write → PolicyRule("write-requires-approve")
    → GateDecision::RequireConfirmation { level: Approve }

Runner layer:
  RequireConfirmation → blocked (Direct mode)
  (Conversational mode pause/resume: NOT YET IMPLEMENTED)
```

## Key decision: Direct ≠ policy bypass

The plan explicitly clarified:
> Direct mode must not mean policy bypass. Direct skips planning, not ToolGate.

This is correct in the implementation: Direct mode hits the same policy rules,
and `RequireConfirmation` is treated as blocked (no way to pause for user input).

## New Files
- `crates/policy/tests/write_gate.rs` — 5 policy gate tests

## Modified Files
- `crates/tools/src/local.rs` — `file_write` tool + `batch2_local_tools()` + 9 tests
- `crates/app/src/main.rs` — `batch2_local_tools` + `build_write_policy()` with write rule

## Tests: 279 total (+14), 0 failures
- 9 file_write tool tests
- 5 write gate policy tests
