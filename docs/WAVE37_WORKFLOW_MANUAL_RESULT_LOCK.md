# Wave 37 — Manual Result Capture and Evidence Attachment — LOCK

**Committed:** 6 commits
**Baseline:** 2746 tests (Wave 36 locked)
**Final:** ~2824 tests, zero failures

---

## What Shipped

### New Modules in `openwand-workflow`

| Module | Purpose |
|--------|---------|
| `workflow_manual_result.rs` | Result DTOs, artifact references, validation snapshot |
| `workflow_manual_result_validation.rs` | 15 validation rules, 4-hash binding, acknowledgment gate |

### New Modules in `openwand-app`

| File | Purpose |
|------|---------|
| `workflow_manual_result.rs` | Persistence under `workflow_manual_results/` with artifacts + 4 indexes |
| `ui/workflow_manual_result_state.rs` | UI view helpers + safety warning |
| `ui/workflow_manual_result_components.rs` | Desktop-gated placeholder |
| `main.rs` additions | CLI: `workflow-manual-result capture/show/latest` |

---

## Test Breakdown

| Area | Count |
|------|------:|
| DTO / Validation (incl. Patch 1) | 10 |
| Hash binding / Review validation (incl. Patch 2) | 12 |
| Artifact references (incl. Patch 5) | 6 |
| Persistence / Idempotency (incl. Patch 3+4) | 20 |
| CLI | 7 |
| UI State | 5 |
| Guard / No-Mutation | 18 |
| **Total** | **78** |

---

## Central Invariant

```
Manual result capture is reported evidence.
Reported evidence is not verified execution.
Artifact attachment is not validation.
Result capture is not reconciliation.
Result capture does not advance workflow state.
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. Evidence classification vs authority flags | ✅ `workflow_manual_result_has_reported_true_and_verified_false_flags` |
| 2. Hash match not just presence | ✅ Rules 4–7 check `actual == expected` for all 4 hashes |
| 3. Full idempotency coverage | ✅ `reported_partial_preserves_history` + `inconclusive_preserves_history` + `same_idempotency_key_returns_existing_inconclusive_result` |
| 4. Approval/session no-write | ✅ `manual_result_does_not_write_approval_or_session_records` |
| 5. Artifact hash verbatim storage | ✅ `operator_supplied_artifact_hash_is_stored_without_recomputation` |

---

## Result Flow

```
WorkflowManualResultRequest (4 hashes + status + operator + summary)
  → 15 validation rules (4 hash matches + acknowledged review + not-performed)
    → ReportedSucceeded/Failed/Partial/NotPerformed/Inconclusive
    → Artifact references (metadata-only, hash stored verbatim)
    → Validation snapshot captures hash-match + acknowledgment state
```

---

## Key Boundary

Wave 37 records operator-reported results. It does NOT:
- Execute commands, invoke shell/git/process
- Verify external state (shell, git, filesystem, URLs)
- Read artifact file contents or fetch URLs
- Route actions, resolve approvals, reconcile outcomes
- Execute tools, call PolicyEngine, SessionRunner, or LlmClient
- Append trace, mutate memory, or mutate workflow state
- Recompute operator-supplied artifact hashes
- Create execution grants or allow execution now
