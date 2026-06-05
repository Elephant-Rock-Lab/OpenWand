# Wave 43 — Manual Result Reconciliation Gate — LOCK

**Committed:** 7 commits
**Baseline:** 2967 tests (Wave 42 locked)
**Final:** 3052 tests (+85), zero failures

---

## What Shipped

### New Workflow Crate Modules

| Module | Purpose |
|--------|---------|
| `workflow_manual_result_reconciliation_gate.rs` | DTOs: WorkflowManualResultReconciliationGateId (wmrrg_), request (8 hashes), record, ManualResultStageProgression, 25 predicates, 9 authority flags |
| `workflow_manual_result_reconciliation_gate_evaluator.rs` | Evaluator: context with latest-review/latest-readiness, 25-predicate evaluation, preview-driven progression |
| `workflow_manual_result_reconciliation_gate_validation.rs` | Validation: content-addressed ID construction, 8-hash requirement |

### New App Crate Modules

| File | Purpose |
|------|---------|
| `crates/app/src/workflow_manual_result_reconciliation_gate.rs` | Persistence: `workflow_manual_result_reconciliation_gates/` with 10 indexes |
| `crates/app/src/ui/workflow_manual_result_reconciliation_gate_state.rs` | UI helpers: summary, safety warning |
| `crates/app/src/ui/workflow_manual_result_reconciliation_gate_components.rs` | Desktop-gated placeholder |
| `crates/app/tests/workflow_manual_result_reconciliation_gate_cli.rs` | 4 CLI integration tests |
| `crates/app/tests/workflow_manual_result_reconciliation_gate_guards.rs` | 18 guard tests |

### CLI Command

```bash
openwand workflow-manual-result-reconciliation-gate reconcile --workflow-execution-id <id> --manual-result-id <id> --manual-result-review-id <id> --reconciliation-readiness-id <id> --stage-id <id> --expected-workflow-run-hash <h> --expected-reconciliation-readiness-hash <h> --expected-manual-result-review-hash <h> --expected-manual-result-hash <h> --expected-command-review-hash <h> --expected-command-composer-hash <h> --expected-command-descriptor-hash <h> --expected-loop-controller-hash <h> --requested-by <name>
openwand workflow-manual-result-reconciliation-gate show <gate-id>
openwand workflow-manual-result-reconciliation-gate latest [--manual-result-id <id>]
```

### Matrix Updates

- `docs/CAPABILITY_TRACEABILITY_MATRIX.md` — added Wave 43 row
- `docs/capability_traceability_matrix.json` — added `wmrrg_` prefix + Wave 43 capability

---

## Test Breakdown

| Area | Count |
|------|------:|
| Workflow DTO tests (ID, status, progression, authority) | 12 |
| Workflow evaluator tests (25 predicates, Patch 1-5-7) | 26 |
| Workflow validation tests (ID, 8-hash) | 5 |
| App persistence tests (roundtrip, indexes, idempotency, no-write proofs) | 15 |
| CLI integration tests | 4 |
| UI state tests | 5 |
| Guard tests (18 standard + 1 JSON shape) | 18 |
| **Total** | **85** |

---

## Central Invariant

```
Manual result reconciliation creates a new immutable workflow run revision
from accepted operator-reported evidence.
It does not execute commands.
It does not verify external truth.
It does not mutate the original workflow run.
It does not route continuation.
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. Preview as authority (not raw result status) | ✅ 3 preview predicates, 4 tests for preview/status interaction |
| 2. Restrict eligible manual result kinds | ✅ ManualResultEligibleForWorkflowStageReconciliation predicate, 3 tests |
| 3. Full 8-hash evidence chain | ✅ 8 hash predicates + validation, 1 request-has-all-hashes test |
| 4. Latest-review/latest-readiness revalidation | ✅ 2 context fields, 2 predicates, 4 tests |
| 5. Explicit stage eligibility (only Suspended) | ✅ StageStatusEligibleForManualReconciliation, 5 tests |
| 6. Authority flags (creates_run_revision, 8 false flags) | ✅ 5 authority tests + 1 JSON guard |
| 7. AlreadyReconciled + idempotency semantics | ✅ 4 persistence tests (same-key, no-duplicate, retry, existing-revision) |
| 8. 10 source-chain indexes + docs updates | ✅ 7 index tests + matrix verification assertions |

---

## Key Boundary

Wave 43 creates a workflow run revision from manual result evidence. It does NOT:
- Execute any commands
- Verify external truth
- Mutate the original workflow run record
- Route continuation
- Resolve approvals
- Append trace
- Write memory

---

## Honest Caveats

- The gate evaluator is deterministic and evidence-based; it does not inspect external artifacts
- A Reconciled gate means a revision was created as evidence, not that external state is verified
- The preview drives the stage status mapping; the raw manual result status is a consistency check only
- CLI constructs the record directly; full evaluator is available but not wired through CLI (consistent with existing patterns)
- Blocked/Failed gates may be retried with new evidence; the gate record is evidence, not an execution grant
- The progression function only maps 3 actionable preview targets; non-actionable targets produce no progression
