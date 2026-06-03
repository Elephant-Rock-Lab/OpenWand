# Wave 25 — Workflow Execution Readiness Gate — LOCK

**Committed:** 6 commits
**Baseline:** 1822 tests (Wave 24 locked)
**Final:** 1900 tests (+78), zero Wave 25 failures

---

## What Shipped

### New Modules in `openwand-workflow`

| Module | Purpose |
|--------|---------|
| `workflow_readiness.rs` | DTOs: request, record, decision, 21 predicates, snapshots |
| `workflow_readiness_validation.rs` | Validation: Ready requires all predicates pass |
| `workflow_readiness_evaluator.rs` | Deterministic predicate evaluation engine |
| `tool_intent_resolution.rs` | Capability category registry + resolution |

### New Modules in `openwand-app`

| File | Purpose |
|------|---------|
| `workflow_readiness.rs` | Persistence under `workflow_readiness/` |
| `ui/workflow_readiness_state.rs` | UI view helpers + safety warning |
| `ui/workflow_readiness_components.rs` | Desktop-gated placeholder |
| `main.rs` additions | CLI: `openwand workflow-readiness evaluate/show/latest` |

---

## Dependency Posture

Workflow crate stays at 6 dependencies: `serde, serde_json, blake3, chrono, thiserror, tracing`.

---

## Test Breakdown

| Area | Count |
|------|------:|
| DTO / validation | 16 |
| Predicate engine | 16 |
| Tool intent resolution | 7 |
| Persistence / idempotency | 14 |
| CLI | 8 |
| UI state | 6 |
| Guard / no-mutation | 11 |
| **Total** | **78** |

---

## Central Invariant

```
A Ready record is not an execution grant.
A resolvable tool intent is not a tool call.
A future approval requirement is not an approval request.
Provider/session availability is not permission to run.
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. `requirement_understood` (not `satisfied_now`) | ✅ Dedicated test confirms no "approved/satisfied/granted" |
| 2. `PolicyConstraintsRepresented` (not execution policy decision) | ✅ Predicate checks representation only, dedicated test |
| 3. CLI namespace matches `workflow-proposal` | ✅ Both top-level commands, guard test confirms |
| 4. Idempotency covers all statuses + different keys | ✅ 5 idempotency tests including Inconclusive + Ready with different key |

---

## 21 Readiness Predicates

ProposalExists, ProposalReviewExists, ProposalReviewIsLatest, ProposalReviewApproved,
ProposalHashMatchesReview, ProposalHashMatchesRequest, SourceTaskPlanExists,
SourceTaskPlanHashMatchesProposal, SourceTaskPlanHashMatchesRequest,
SourceTaskPlanLatestReviewApproved, WorkflowProposalIsReviewable,
RequiredApprovalMarkersPresent, ToolIntentsResolvable, ToolIntentsRemainNonExecutable,
PolicyConstraintsRepresented, ProviderConfigurationAvailable, SessionRuntimeAvailable,
WorkspacePreconditionsObserved, RollbackAbortEvidencePresent,
NoPriorConflictingReadiness, IdempotencyKeyUnusedOrMatchesExisting.

Provider/session missing → **Inconclusive** (not Blocked).
All other failures → **Blocked**.

---

## CLI Surface

```bash
openwand workflow-readiness evaluate --proposal-id <id> --review-id <id> \
  --expected-proposal-hash <hash> --expected-source-task-plan-hash <hash> \
  [--idempotency-key <key>] [--output-dir] [--json]
openwand workflow-readiness show <readiness-id> [--output-dir] [--json]
openwand workflow-readiness latest [--proposal-id|--review-id|--task-plan-id <id>] [--output-dir] [--json]
```

---

## Honest Caveats

- Wave 25 determines readiness only. It does not execute workflows.
- A Ready record is evidence for a future execution gate, not permission to execute.
- Environment availability (provider, session) is a boolean input, not an actual check.
- Tool intent resolution matches against a static capability category registry.
- No workflow versioning — readiness is computed per (proposal, review, key) triple.
