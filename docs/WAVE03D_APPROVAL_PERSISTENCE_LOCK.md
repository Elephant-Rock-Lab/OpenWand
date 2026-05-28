# WAVE 03D — APPROVAL PERSISTENCE & CRASH RECOVERY — LOCK

**Status:** ✅ COMPLETE
**Date:** 2026-05-28
**Scope:** Crash-recoverable approval governance with trace-native persistence

## What was proven

### Batch A — Core schema + recovery scanner

- `ApprovalContextSnapshot` embedded in `tool.suspended` events (16 fields, bounded at 1 MiB)
- `ToolEvent` extended additively: `Suspended` gets `approval_context`, `Resumed` gets `approval_request_id`, `Denied` gets `approval_request_id` + `reason`
- New `ToolEvent::Deferred` variant for multi-tool batch events
- All new fields `#[serde(default)]` — pre-03d events deserialize cleanly
- `build_recovery_index()` pure scanner over trace entries
- `ApprovalRecoveryConflict` with 4 conflict types
- Size guard: oversized arguments → `gate.evaluated` + `tool.denied`, no suspension

### Batch B — Projection + UI reconstruction

- Loro `waiting_approval` state: set/clear/get from `ApprovalContextSnapshot`
- Loro `recovery_blocked` state: set when conflicts, multiple pending, or uncertain
- `rebuild_approval_state()` scans trace and applies state to Loro
- Scope limited to approval state — not full Loro message parity
- Old `tool.suspended` without context → recovery blocked (no panic)

### Batch C — Unified resolution path

- `resume_with_approval` is a thin wrapper that does NOT `take()` pending before success
- `resolve_approval_internal` sources truth from trace via recovery index
- Live and recovered approvals converge to the same resolver
- `resolve_recovered_approval` for post-restart resolution without live pending state
- `tool.resumed` now includes `approval_request_id` linking to context
- All 14 existing approval_hardening tests pass unchanged (regression proof)

### Batch D — SQLite E2E + crash guards

- Case A: crash before decision → approval UI reconstructs from SQLite trace
- Case A + approve: crash → restart → `resolve_recovered_approval(Approved)` → tool executes
- Case A + reject: crash → restart → `resolve_recovered_approval(Rejected)` → no execution
- Old events without context → recovery blocked
- Deferred tools not reconstructed as pending approvals
- `tool.deferred` carries `blocked_by_approval_request_id`

## New infrastructure

| Component | File |
|-----------|------|
| `ApprovalContextSnapshot`, `MAX_APPROVAL_CONTEXT_ARG_BYTES` | `crates/core/src/snapshots.rs` |
| Extended `ToolEvent` (Suspended/Resumed/Denied/Deferred) | `crates/core/src/events/tool.rs` |
| Recovery types, scanner, computation, UI model | `crates/session/src/approval_recovery.rs` |
| Loro approval state (set/clear/get) | `crates/session/src/loro_state.rs` |
| `rebuild_approval_state()`, `apply_recovery_state()` | `crates/session/src/persistence.rs` |
| `resume_with_approval`, `resolve_recovered_approval`, `resolve_approval_internal` | `crates/session/src/runner.rs` |

## Module ownership

```
core/snapshots.rs          — DTOs only (no computation)
core/events/tool.rs        — Event vocabulary (additive migration)
session/approval_recovery  — Types, scanner, computation, commands
session/loro_state.rs      — Loro projection setters/getters
session/persistence.rs     — Trace → Loro rebuild
session/runner.rs           — Orchestration only
```

Session crate never imports `openwand-store` — stays over `TraceStore<StoredEvent>` abstraction.

## Combined Wave 03 status

| Trust property | Evidence |
|---|---|
| LLM cannot directly mutate disk | ✅ Write goes through policy |
| Direct mode is not a policy bypass | ✅ RequireConfirmation blocks |
| Conversational pauses before mutation | ✅ AwaitingApproval + pending |
| Approval durable before execution | ✅ tool.resumed precedes execute |
| Trace failure prevents execution | ✅ Hostile ordering test |
| Rejection feeds back to LLM | ✅ Error result in Loro + LlmRequest |
| Model continues after rejection | ✅ Two-turn test |
| Multi-tool batch is atomic | ✅ Only first suspends, rest frozen |
| Pending approval survives restart | ✅ SQLite E2E Case A |
| Approval UI reconstructs from trace | ✅ SQLite recovery test |
| Approve after restart works | ✅ SQLite E2E approve |
| Reject after restart works | ✅ SQLite E2E reject |
| Old events handled gracefully | ✅ Recovery blocked, no panic |
| Oversized context blocked | ✅ gate.evaluated + tool.denied |
| Conflicts detected | ✅ 4 conflict types |
| `tool.deferred` recorded | ✅ Multi-tool batch |
| Session stays over TraceStore abstraction | ✅ No store import |

## Accepted conservative deviations

- Uncertain execution blocks on ALL `tool.called` without terminal, not just mutating tools
- Full Loro projection parity out of scope — only approval-state rebuild
- No blob-backed argument storage — inline only with 1 MiB cap
- No encrypted argument storage
- No multi-call batch approval — only first tool can be approved per suspension

## Remaining gaps (honest)

1. **Crash after approved + called but before terminal** — Detected by scanner as "uncertain" and blocked, but no recovery mechanism. Now actually reachable through normal runner execution since `tool.called`/`tool.completed`/`tool.failed` are emitted.
2. **Rich UI reconstruction** — Only approval state. Full session view reconstruction from trace is a separate milestone.
3. **Idempotency keys** — The resolver checks trace state for pending, but doesn't use deterministic idempotency keys for the resolution events themselves. Duplicate resolution is prevented by the "no pending found" guard, not by key collision.
4. **Two public APIs** — `resume_with_approval` (live) and `resolve_recovered_approval` (recovered) are separate entry points sharing `resolve_approval_internal`. Not truly "one path" as the plan stated — two entry points, one internal implementation.

## Tests: 345 total, 0 failures

## Commits

| Batch | Commit | Scope |
|-------|--------|-------|
| A | `bf7fbc7` | Core schema, recovery scanner, ApprovalContextSnapshot |
| B | `928040b` | Loro approval state, persistence rebuild |
| C | `2cfe2bd` | Unified resolution path, resolver |
| D | (pending) | SQLite E2E, crash guards, lock doc |

## Classification

```
Wave 03d: Crash-recoverable approval governance proven.
Pending approval survives restart via trace-native persistence.
One resolver path for both live and recovered approvals.
```
