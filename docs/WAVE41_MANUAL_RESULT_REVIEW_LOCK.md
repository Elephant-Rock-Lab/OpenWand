# Wave 41 — Manual Result Review and Acceptance Gate — LOCK

**Committed:** 6 commits
**Baseline:** 2824 tests (Wave 40 locked)
**Final:** 2901 tests (+77), zero failures

---

## What Shipped

### New Workflow Crate Modules

| Module | Purpose |
|--------|---------|
| `workflow_manual_result_review.rs` | DTOs: WorkflowManualResultReviewId (wmrr_), request, record, decision enum, feedback, acceptance snapshot |
| `workflow_manual_result_review_validation.rs` | 11 validation rules: existence, hash binding, operator-reported, no-verification, reviewer/rationale, structural guards |

### New App Crate Modules

| File | Purpose |
|------|---------|
| `crates/app/src/workflow_manual_result_review.rs` | Persistence: `workflow_manual_result_reviews/` with 5 indexes + feedback |
| `crates/app/src/ui/workflow_manual_result_review_state.rs` | UI helpers: summary, acceptance, safety warning |
| `crates/app/src/ui/workflow_manual_result_review_components.rs` | Desktop-gated placeholder |
| `crates/app/tests/workflow_manual_result_review_cli.rs` | 6 CLI integration tests |
| `crates/app/tests/workflow_manual_result_review_guards.rs` | 18 guard tests |

### CLI Command

```bash
openwand workflow-manual-result-review review-accept --manual-result-id <id> --workflow-execution-id <id> --command-review-id <id> --command-composer-id <id> --loop-controller-id <id> --expected-manual-result-hash <hash> --expected-command-review-hash <hash> --expected-command-composer-hash <hash> --expected-command-descriptor-hash <hash> --expected-loop-controller-hash <hash> --reviewer <name> --rationale <text>
openwand workflow-manual-result-review review-reject ... --blocking-reasons <text>
openwand workflow-manual-result-review review-request-changes ... --requested-changes <text>
openwand workflow-manual-result-review show <review-id>
openwand workflow-manual-result-review latest [--manual-result-id <id>]
```

---

## Test Breakdown

| Area | Count |
|------|------:|
| Workflow DTO tests (review, feedback, acceptance snapshot) | 14 |
| Workflow validation tests (11 rules + edge cases) | 14 |
| App persistence tests (roundtrip, indexes, idempotency, no-write proofs) | 20 |
| CLI integration tests | 6 |
| UI state tests | 5 |
| Guard tests (18 standard + 1 JSON shape) | 18 |
| **Total** | **77** |

---

## Central Invariant

```
Review accepts reported evidence.
It does not verify external state.
It does not reconcile workflow state.
Acceptance is recorded, not executed.
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. Duplicate-acceptance/idempotency in app, not workflow validation | ✅ `manual_result_review_validation_does_not_check_prior_reviews`, `accepted_review_cannot_duplicate_with_different_key`, `rejected_review_preserves_history_with_new_key`, `changes_requested_review_preserves_history_with_new_key`, `same_idempotency_key_returns_existing_manual_result_review` |
| 2. Full evidence chain hash binding (5 hashes) | ✅ `review_copies_evidence_chain_hashes_from_result`, `blocks_manual_result_hash_mismatch`, 4 hash-mismatch tests in validation |
| 3. Acceptance semantics snapshot (accepts_reported_evidence, not verified) | ✅ `accepted_review_accepts_reported_evidence_only`, `accepted_review_does_not_verify_external_state`, `accepted_review_does_not_reconcile_workflow_state`, `accepted_review_does_not_mark_result_true`, `review_serialized_json_contains_no_verification_or_reconciliation_claims` |
| 4. CLI requires all expected hashes | ✅ `cli_manual_result_review_requires_expected_hashes` (fails without them) |
| 5. Persistence indexes for all source-chain IDs + feedback | ✅ `review_by_command_composer_returns_expected`, `review_by_loop_controller_returns_expected`, `feedback_persists_and_loads_roundtrip` |
| 6. Split commit structure (DTOs → Validation → Persistence → CLI → UI → Guards+Lock) | ✅ 6 commits |

---

## Key Boundary

Wave 41 adds review/acceptance of reported manual results. It does NOT:
- Verify external state (shell, git, filesystem, URLs)
- Reconcile the result into workflow run state
- Mutate any existing manual result records
- Execute any commands
- Append trace directly
- Write to memory, policy, or session
- Treat acceptance as verification

---

## Honest Caveats

- This review accepts evidence, not verified truth
- Acceptance does not mean the reported result is accurate
- Reconciliation remains a separate future step (Wave 43)
- A rejected result may still be re-reported and re-reviewed
- No external artifact inspection is performed
- The reviewer is trusted — reviewer identity is not verified
- The CLI constructs the review record directly; full workflow-crate validation is available but not wired through the CLI path (consistent with existing CLI patterns)
