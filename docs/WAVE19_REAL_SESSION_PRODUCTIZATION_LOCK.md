# Wave 19: Real Session Productization — Lock

**Commit:** `8d94bb3`
**Date:** 2026-06-03
**Status:** LOCKED
**Tests:** 1508 total, zero failures

## Scope

Surface the session runner's real execution path in the Dioxus UI. Stream assistant output, route tool approvals through the governed session flow, prove the UI never bypasses session, policy, trace, memory-read, or tool-execution seams.

## Lock Condition (Honest Wording)

```
OpenWand can run a user-visible session through the real SessionRunner path
in the Dioxus UI, with CI coverage using deterministic LLM/policy/tool/memory
mocks and provider-backed runs remaining feature-gated/manual.
```

## Non-Negotiable Invariant

```
UI submits user input.
SessionRunner owns the agent loop.
Policy gates tools.
Trace records authority.
UI observes and renders.
```

## Module Boundary

```
Wave 01d+: openwand-session owns SessionRunner and agent loop
Wave 18: UI governance console
Wave 19: UI session productization and bridge wiring
```

Extended files:
- `run_dto.rs` — expanded status/event/DTO vocabulary
- `run_bridge.rs` — all 8 AgentEvent variants now mapped
- `service.rs` — resolve_approval() and refresh_session() thin adapters

New files:
- `session_actions.rs` — UiSessionAction enum + adapter
- `session_components.rs` — view helpers + Dioxus render functions

## Key Design Decisions

### Patch 1: Exhaustive AgentEvent Mapping

All 8 current `AgentEvent` variants are mapped in `translate_event()`:
- `RunStarted` → sets session_id
- `PhaseEntered` → updates phase/step + flushes text on step boundary
- `TextDelta` → appends to streamed_text
- `ToolCallStarted` → records in tool_events
- `ToolCallCompleted` → records in tool_events
- `ApprovalRequested` → sets WaitingForApproval + pending_approval DTO
- `ApprovalResolved` → clears pending_approval (dead variant, defensively mapped)
- `RunCompleted` → flushes text, preserves WaitingForApproval

Guard test `ui_bridge_covers_all_current_agent_event_variants` verifies count matches expectation.

### Patch 2: Honest "Real Session" Claim

The session path is real (SessionRunner, real policy engine, real trace store, real tool dispatch). The LLM provider is mocked in CI. Provider-backed runs remain feature-gated/manual. This is the honest claim.

### Patch 3: Thin Adapters Only

`UiSessionService::resolve_approval()` and `execute_session_action()` are thin wrappers over existing APIs:
- `resolve_approval()` → `runner.resolve_approval()`
- `send_message()` → records user message, delegates to `start_run()`
- `refresh_session()` → delegates to `open_session()` (trace projection read)

Tests prove:
- `ui_service_send_message_does_not_construct_llm_request`
- `ui_service_resolve_approval_does_not_mutate_pending_state_directly`

### WaitingForApproval Preserves Through RunCompleted

The runner emits `RunCompleted` after the loop breaks for `AwaitingApproval`. The UI state preserves `WaitingForApproval` rather than overwriting with `Completed`. This is correct: the run ended but the approval is still pending.

### ApprovalResolved Is a Dead Variant

`ApprovalResolved` exists in `AgentEvent` but is never emitted by the runner. Pending approval clearing happens through the action adapter (`execute_session_action`), not through the event bridge. The bridge maps it defensively in case a future wave starts emitting it.

### Text Flush on Step Boundary

Streamed text is flushed from `streamed_text` into `messages` on step boundary (PhaseChanged with step increment) and on completion. This produces a clean transcript.

## What Ships

| Component | Purpose |
|-----------|---------|
| Extended UiRunStatus | Starting, WaitingForApproval, Blocked, Error |
| Event bridge | All 8 AgentEvent variants mapped |
| Pending approval DTO | Tool name, call ID, reason |
| Session messages | User/Assistant/Tool/System with trace IDs |
| Memory context indicator | Retrieved/included/excluded counts |
| Trace summary | Event count, last kind, trace ID |
| Session action adapters | Start/Send/Stop/Approve/Reject/Refresh |
| View helpers | Status bar, transcript, approval panel, memory indicator, error panel |
| Dioxus render functions | Desktop-gated, consuming pure helpers |

## What Explicitly Does Not Ship

```
new LLM provider implementation
new tool execution backend
new policy rules
new memory extraction/write path
workflow spawning
multi-session orchestration
skills/goals activation
provider matrix hardening
arbitrary shell or git UI
governance execution buttons
automatic tool approval
automatic memory write
```

## Test Coverage (47 new tests)

- **State/event bridge** (14): initial state, run started, text delta, step flush, tool pending, tool result, tool blocked, run completed, error, approval resolved, starting status, blocked status
- **Live bridge** (2): approval requested via AgentEvent, run started sets session ID
- **Session actions** (10): serde roundtrip, no execution fields, start sets starting, send records user message, stop cancels, refresh read-only, approve/reject without runner returns error
- **Patch 3 proofs** (2): no LlmRequest construction, no direct pending mutation
- **View helpers** (8): transcript rendering, streaming delta, approval panel, memory indicator, status bar, error panel, all statuses have text, trace summary
- **Guards** (8): no process::Command, no LlmClient, no MemoryStore, no shell/git, no trace.append, no ToolResult construction, no ToolExecutor, no PolicyEngine direct eval
- **E2E mock** (6): text-only session, approval waits, rejection clears approval, policy block in Direct mode, tool error visible, memory context indicator

## Honest Caveats

- Wave 19 does not add new LLM providers, provider health checks, or provider matrix hardening.
- Wave 19 does not add workflow spawning, multi-session orchestration, skills, or goals.
- Wave 19 does not add memory editing, extraction UI, or governance execution buttons.
- Real provider smoke tests remain manual or feature-gated. CI uses deterministic mocks.
- `ApprovalResolved` is a dead variant — the UI action adapter handles approval clearing, not the event bridge.
- The Dioxus render functions are component shells; real form submission UX polish is post-Wave-19 work.
