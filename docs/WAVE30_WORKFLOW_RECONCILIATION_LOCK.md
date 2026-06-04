# Wave 30 — Workflow Outcome Reconciliation and Stage Progression — LOCK

**Committed:** 6 commits
**Baseline:** 2194 tests (Wave 29 locked)
**Final:** ~2284 tests, zero failures

---

## What Shipped

### New Modules in `openwand-workflow`

| Module | Purpose |
|--------|---------|
| `workflow_reconciliation.rs` | DTOs: request, record, status, decision, stage progression, run revision, 18 predicates, terminal status set |
| `workflow_reconciliation_validation.rs` | Content-addressed IDs, hash validation |
| `workflow_reconciliation_gate.rs` | 18 deterministic reconciliation predicates |
| `workflow_stage_progression.rs` | Stage transition engine, aggregate status, run revision builder |

### New Modules in `openwand-app`

| File | Purpose |
|------|---------|
| `workflow_reconciliation.rs` | Persistence under `workflow_reconciliations/` + `workflow_run_revisions/` |
| `ui/workflow_reconciliation_state.rs` | UI view helpers + safety warning |
| `ui/workflow_reconciliation_components.rs` | Desktop-gated placeholder |
| `main.rs` additions | CLI: `openwand workflow-reconciliation reconcile/show/latest` |

---

## Test Breakdown

| Area | Count |
|------|------:|
| DTO / validation | 12 |
| Predicate gate | 22 |
| Stage progression + dep guard | 13 |
| Persistence / idempotency | 18 |
| CLI | 7 |
| UI state | 5 |
| Guard / no-mutation | 13 |
| **Total** | **90** |

---

## Central Invariant

```
Outcome records are evidence.
Reconciliation updates workflow run revision evidence.
Reconciliation is not execution.
Reconciliation is not routing.
Reconciliation is not approval resolution.
Reconciliation does not continue the workflow automatically.
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. Aggregate Completed only on revision, not original run | ✅ `compute_aggregate_status` sets on revision only; `reconciliation_does_not_mutate_original_run_status` + `all_terminal_stages_mark_revision_completed_not_original_run` tests |
| 2. Terminal stage set defined | ✅ `is_terminal_stage_status()` + `terminal_stage_set_includes_completed_blocked_failed_skipped` + `pending_running_suspended_are_not_terminal` tests |
| 3. OutcomeEvidenceFromSession strengthened | ✅ Requires at least one signal (trace_ids, tool_call_id, tool_status, approval_request_id) + `blocks_outcome_with_empty_session_evidence_snapshot` + `accepts_outcome_with_trace_id_session_evidence` + `accepts_outcome_with_tool_status_session_evidence` tests |
| 4. Workflow crate dep guard | ✅ `workflow_crate_dependency_guard_still_allows_only_6_deps` test confirms exactly 6 deps |

---

## Stage Progression Rules

| Outcome Status | Stage Transition | Lifecycle Event |
|----------------|------------------|-----------------|
| ToolCompleted | Suspended → Completed | StageCompleted |
| ToolDenied | Suspended → Blocked | StageBlocked |
| Failed | Suspended → Failed | StageFailed |
| ApprovalResolved only | No transition | — |

---

## Reconciliation Flow

```
WorkflowReconciliationRequest (run + route + outcome + hashes)
  → 18 predicates validate full linkage chain
    → Stage Progression Engine computes linked-stage transition
      → apply_progression_to_stages updates only linked stage
      → compute_aggregate_status sets Completed on revision if all terminal (Patch 1)
    → WorkflowRunRevision persisted (immutable, original untouched)
  → WorkflowReconciliationRecord persisted
```

---

## Key Boundary

Wave 30 reconciles one terminal outcome into stage progression evidence. It does NOT:
- Start the next stage
- Route the next action
- Resolve approvals
- Execute tools
- Append trace
- Mutate memory, session state, or original records
- Create route/outcome/readiness/proposal/task-plan records

---

## Honest Caveats

- Wave 30 reconciles one terminal outcome per stage. No multi-outcome batch.
- Run revisions are immutable. Original run records are never mutated.
- Does not advance dependent stages — only the directly linked stage.
- Does not route next actions or continue workflow automatically.
- CLI reconcile blocks without pre-loaded run/route/outcome context (expected).
- No background worker, scheduler, or queue.
- Default CI remains provider-free and network-free.
