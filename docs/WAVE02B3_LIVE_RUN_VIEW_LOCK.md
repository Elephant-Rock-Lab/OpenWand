# WAVE 02B-3 — LIVE RUN VIEW — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-27
**Scope:** Live LLM run from desktop UI, streaming text, tool events, cancel

## Proven

- Desktop app sends user text to a real LLM via SessionRunner
- Assistant text streams into the detail pane via UiRunBridge
- Tool call/result events appear in order
- Phase/step indicator shows run progress
- Cancel button stops the run via CancellationToken
- UI remains responsive during runs (bridge runs in spawned tokio task)
- Send button disabled during active run

## Architecture

```
User types text → handle_send()
  → creates SessionRunner with real LLM + tools + policy
  → service.start_run() → subscribe() + start_bridge()
  → bridge: broadcast::Receiver<AgentEvent> → Arc<Mutex<UiRunState>>
  → poll_run_state(): polls bridge state → RUN_STATE GlobalSignal
  → Dioxus re-renders from RUN_STATE changes

Cleanup:
  - CancellationToken drops the bridge task
  - ACTIVE_RUNNER cleared on completion
  - No leaked tasks
```

## Backpressure Rules

| Event Type | Handling |
|-----------|----------|
| TextDelta | Append to `streamed_text` (coalesce OK) |
| ToolCallStarted/Completed | Push to `tool_events` (never dropped) |
| PhaseChanged | Keep latest (overwrite OK) |
| Error/Completed | Status update (never dropped) |
| Lagged | Warning in error field, bridge continues |

## New Types

- `UiRunState` — snapshot of run (status, phase, streamed_text, tool_events, error)
- `UiRunStatus` — Idle, Running, Completed, Failed, Cancelled
- `UiRunEvent` — TextDelta, ToolCallStarted, ToolCallCompleted, PhaseChanged, Completed, Error
- `RunHandle` — returned by start_run(), holds shared state + cancellation
- `ActiveRun` — UI-side tracking of runner + cancellation + bridge state

## New Files

- `crates/app/src/ui/run_dto.rs` — UiRunState, UiRunStatus, UiRunEvent
- `crates/app/src/ui/run_bridge.rs` — bridge from AgentEvent to UiRunState
- `crates/app/tests/run_bridge.rs` — 6 bridge + 5 state tests
- `crates/app/src/ui_main.rs` — rewritten with input, streaming, tool events

## Accepted Limitations

- Runner created per-run (no persistent runner across sessions)
- No message persistence after run completes (deferred to 02b-4)
- No trace replay for message reload (deferred to 02b-4)
- Tool result output text empty in UI (AgentEvent doesn't carry it)
- No Shift+Enter newline in textarea yet
- No session switching during active run

## Tests: 222 total, 0 failures

- +6 run bridge async tests
- +5 run state apply tests
