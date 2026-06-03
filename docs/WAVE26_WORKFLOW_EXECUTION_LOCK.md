# Wave 26 — Governed Workflow Execution Gate — LOCK

**Committed:** 6 commits
**Baseline:** 1900 tests (Wave 25 locked)
**Final:** 1987 tests (+87), zero failures

---

## What Shipped

### New Modules in `openwand-workflow`

| Module | Purpose |
|--------|---------|
| `workflow_run.rs` | DTOs: execution request, run record, decision, 21 predicates, stage runs, lifecycle events, action requests, snapshots |
| `workflow_run_validation.rs` | Validation: RunCreated requires all predicates pass and stages present |
| `workflow_execution_gate.rs` | Deterministic execution gate — revalidates everything at execution time |
| `workflow_run_lifecycle.rs` | Stage lifecycle engine — non-tool stages complete, tool-intent stages suspend |

### New Modules in `openwand-app`

| File | Purpose |
|------|---------|
| `workflow_execution.rs` | Persistence under `workflow_runs/` |
| `ui/workflow_execution_state.rs` | UI view helpers + safety warning |
| `ui/workflow_execution_components.rs` | Desktop-gated placeholder |
| `main.rs` additions | CLI: `openwand workflow-execution execute/show/latest` |

---

## Test Breakdown

| Area | Count |
|------|------:|
| DTO / validation | 17 |
| Execution gate | 18 |
| Stage lifecycle | 10 |
| Persistence / idempotency | 14 |
| CLI | 8 |
| UI state | 7 |
| Guard / no-mutation | 13 |
| **Total** | **87** |

---

## Central Invariant

```
A workflow run is not a tool call.
A stage is not authority.
A stage transition is not a policy override.
A workflow run may request governed actions,
but it never executes tools directly.

All tool execution still flows through:
SessionRunner → PolicyEngine → ToolExecutor → Trace.
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. `RunCreated` (not `Executed`) | ✅ Dedicated test confirms no tool_execution claim |
| 2. `trace_count_unchanged` (no except-session-path) | ✅ Guard checks no TraceStore/openwand_trace imports |
| 3. Non-tool stage completion is lifecycle-only | ✅ "Marked complete as non-tool deterministic stage" summary, dedicated tests |
| 4. 6-dep crate guard | ✅ Still only serde/serde_json/blake3/chrono/thiserror/tracing |

---

## Stage Lifecycle Semantics

**Non-tool stages** (Observe, Analyze, Report): marked complete as lifecycle bookkeeping only. No observation performed, no analysis computed, no report generated, no state changed.

**Tool-intent stages** (PrepareChange, ApplyChange, Verify): create `WorkflowActionRequest` evidence marked `PreparedForFutureSessionRouting`, then suspend.

**RequestApproval stage**: suspends awaiting approval.

---

## Action Requests

`WorkflowActionRequest` contains: capability_category, purpose, input/output summaries, routing_status. It does NOT contain: tool_name, tool_args, command, shell, script, cwd, env, function_ref, process handle, or provider request.

Wave 26 does NOT route action requests to live sessions.

---

## CLI Surface

```bash
openwand workflow-execution execute --readiness-id <id> --proposal-id <id> \
  --proposal-review-id <id> --expected-readiness-hash <hash> --expected-proposal-hash <hash>
openwand workflow-execution show <execution-id>
openwand workflow-execution latest [--readiness-id|--proposal-id|--proposal-review-id|--task-plan-id]
```

---

## Honest Caveats

- Wave 26 creates run records and stage lifecycle evidence. It does not perform live tool execution.
- Action requests are `PreparedForFutureSessionRouting` — not routed to sessions.
- `PolicyAllowsWorkflowRunCreation` always passes in Wave 26 (no policy engine call).
- No retry/resume/scheduler/worker/queue/rollback execution.
- Completed runs cannot duplicate for same readiness/proposal/review.
