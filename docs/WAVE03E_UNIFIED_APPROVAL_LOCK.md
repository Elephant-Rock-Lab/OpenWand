# WAVE 03E: UNIFIED APPROVAL RESOLUTION — LOCK

**Commit:** (pending)
**Tests:** 360 total, 0 failures
**Date:** 2026-05-28

## What changed

Replaced two public approval resolution methods with one unified API, keyed by `approval_request_id` instead of `tool_call_id`.

### Deleted

| Symbol | Location | Reason |
|--------|----------|--------|
| `enum ApprovalDecision { Approved, Rejected }` | `runner.rs` | Replaced by struct with `resolution` + optional `approval_request_id` |
| `resume_with_approval()` | `runner.rs` | Merged into `resolve_approval()` |
| `resolve_recovered_approval()` | `runner.rs` | Merged into `resolve_approval()` |
| `resolve_approval_internal()` | `runner.rs` | Replaced by `resolve_from_index()` |
| `record_tool_resumed()` | `runner.rs` | Unused after refactor; resolver builds events directly |
| `record_tool_denied_event()` | `runner.rs` | Unused after refactor; resolver builds events directly |

### New types

| Symbol | Location | Purpose |
|--------|----------|---------|
| `struct ApprovalDecision` | `runner.rs` | Governance decision with optional scope + resolution |
| `enum ApprovalResolution` | `runner.rs` | `Approve` / `Reject { reason }` |
| `enum ApprovalSource` | `runner.rs` | `Live` / `Recovered` / `StaleCache` — how the resolver found the approval |
| `struct ApprovalResult` (updated) | `runner.rs` | Now carries `resolution`, `approval_request_id`, `source` |
| `struct CachedApproval` | `runner.rs` | Links live pending tool to its `approval_request_id` |
| `struct ResolvedApprovalRecovery` | `approval_recovery.rs` | Tracks already-resolved approvals in recovery index |
| `enum ResolvedApprovalKind` | `approval_recovery.rs` | `Approved` / `Denied` |
| `ApprovalRecoveryIndex.resolved` field | `approval_recovery.rs` | Populated from `tool.resumed` and `tool.denied` events |

### New functions

| Symbol | Location | Purpose |
|--------|----------|---------|
| `resolve_approval()` | `runner.rs` | Single public API for live and recovered approvals |
| `resolve_from_index()` | `runner.rs` | Effectful resolver, takes pre-built index (no scan) |
| `select_approval_target()` | `runner.rs` | Pure selector: index + cache hint + decision → target + source |

### Convenience constructors on `ApprovalDecision`

```rust
ApprovalDecision::approve()                  // unscoped approve
ApprovalDecision::reject()                   // unscoped reject
ApprovalDecision::reject_with_reason("...")  // unscoped reject with reason
ApprovalDecision::for_approval(arid, res)    // scoped to specific approval_request_id
```

## Algorithm

```
resolve_approval(decision, config)
│
├─ Phase 1: Build recovery index (single scan)
│
├─ Phase 2: select_approval_target(index, cache_hint, decision)
│  │
│  ├─ Explicit arid: find by approval_request_id in index.pending
│  │  ├─ Found → (target, Live|Recovered|StaleCache)
│  │  └─ Not found → Err(NoPendingApproval)
│  │
│  └─ No explicit arid: try cache hint, then single pending fallback
│     ├─ Cache hit → (target, Live)
│     ├─ Cache miss, 1 pending → (target, Recovered)
│     ├─ Cache miss, 0 pending → Err(NoPendingApproval)
│     └─ Cache miss, N pending → Err(MultiplePending)
│
├─ Phase 2.5: Idempotency check
│  └─ If explicit arid found in index.resolved → return existing result
│
├─ Phase 3: resolve_from_index(index, target, decision, config)
│  ├─ Check conflicts
│  ├─ Append tool.resumed or tool.denied (with reason from caller)
│  ├─ If approved: tool.called → execute → tool.completed/tool.failed
│  ├─ Update Loro state, clear cache
│  └─ Return ApprovalResult with source
```

## Key invariants

1. **Single scan.** The recovery index is built once in `resolve_approval`. Both selection and resolution use the same index. No double scan.

2. **Cache is a hint, not authority.** `pending_approval: Mutex<Option<CachedApproval>>` provides an `approval_request_id` to narrow the search. It is never the source of truth. If the cache is stale, the selector falls through to the index.

3. **Explicit arid never falls back.** When `decision.approval_request_id = Some(id)`, the selector resolves that exact approval or fails. It does not fall back to "the single pending one" even if one exists. This prevents stale UI state from approving the wrong tool.

4. **Idempotency via resolved tracking.** The recovery index now tracks `resolved: Vec<ResolvedApprovalRecovery>` from `tool.resumed` and `tool.denied` events. Duplicate resolution returns the existing result without appending new trace events or re-executing the tool.

5. **Rejection reason flows through.** `ApprovalResolution::Reject { reason: Option<String> }` is passed by the caller and written into `tool.denied.reason`. Previously hardcoded to `"user_rejected"`.

6. **Source is observable.** Every `ApprovalResult` carries `source: ApprovalSource`, telling the caller whether the approval came from cache (Live), trace scan (Recovered), or mismatched cache (StaleCache).

## Tests added

| Test file | Tests | What |
|-----------|-------|------|
| `select_approval_target.rs` | 13 | Pure selector: all combinations of explicit/unscoped, cache hit/miss/stale, zero/single/multiple pending |
| Index resolved tracking | 2 | `recovery_index_populates_resolved_from_resumed`, `recovery_index_populates_resolved_from_denied` |

All existing approval tests (hardening, lifecycle, recovery E2E, unified resolution, deferred emission) updated to use new API. Same assertions on trace events, tool execution, Loro state.

## What is NOT done

1. **Idempotency test for the full `resolve_approval` method.** The `select_approval_target` tests prove the pure selector handles the "already resolved" case, but no integration test calls `resolve_approval` twice with the same arid and asserts the idempotent return. The logic exists in `resolve_approval` (Phase 2.5) but is not exercised by an automated test.

2. **StaleCache integration test.** The selector produces `StaleCache` when the caller passes an explicit arid that differs from the cache's arid. This is proven in the pure selector tests but not in an integration test that exercises the full runner with real trace.

3. **`pending_approval()` public getter still returns `Option<PendingTool>`.** It should also expose the `approval_request_id` so callers can display it without scanning trace. Currently the CLI doesn't use the arid from the getter — it just shows the tool name. This is a minor API gap, not a correctness issue.

4. **`ApprovalResult.tool_result` is `None` for idempotent returns.** When an already-resolved approval returns idempotently, `tool_result` is `None` because the execution result is not stored in `ResolvedApprovalRecovery`. The caller cannot distinguish "denied (no execution)" from "approved but re-queried (execution already happened)". This is acceptable for now because idempotency is a safety guard, not a normal flow.

## Remaining gaps from earlier waves

1. **Crash after approved + called but before terminal** — detected as "uncertain" and blocked, but no recovery mechanism. Now actually reachable through normal runner execution since lifecycle events are emitted.

2. **Rich UI reconstruction** — only approval state. Full session view from trace is a separate milestone.

3. **Incremental recovery index** — rebuilt from scratch on every call. Acceptable at current scale. Future optimization.
