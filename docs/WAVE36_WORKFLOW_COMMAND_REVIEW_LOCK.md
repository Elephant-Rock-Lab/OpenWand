# Wave 36 — Manual Command Descriptor Review and Operator Acknowledgment — LOCK

**Committed:** 5 commits
**Baseline:** 2679 tests (Wave 35 locked)
**Final:** ~2746 tests, zero failures

---

## What Shipped

### New Modules in `openwand-workflow`

| Module | Purpose |
|--------|---------|
| `workflow_command_review.rs` | Review DTOs, decision, feedback, acknowledgment snapshot |
| `workflow_command_review_validation.rs` | 12 validation rules, hash binding, display-only/executable checks |

### New Modules in `openwand-app`

| File | Purpose |
|------|---------|
| `workflow_command_review.rs` | Persistence under `workflow_command_reviews/` with feedback + 3 indexes |
| `ui/workflow_command_review_state.rs` | UI view helpers + safety warning |
| `ui/workflow_command_review_components.rs` | Desktop-gated placeholder |
| `main.rs` additions | CLI: `workflow-command-review acknowledge/reject/request-changes/show/latest` |

---

## Test Breakdown

| Area | Count |
|------|------:|
| DTO / Validation (incl. Patch 3) | 11 |
| Hash binding / Validation (incl. Patch 1+2) | 11 |
| Persistence / Idempotency | 16 |
| CLI (incl. Patch 4) | 8 |
| UI State | 6 |
| Guard / No-Mutation | 15 |
| **Total** | **67** |

---

## Central Invariant

```
Command descriptor review is not execution.
Operator acknowledgment is not execution permission.
A reviewed descriptor is not a routed action.
An acknowledged descriptor is not a shell command.
Acknowledgment evidence does not perform the command.
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. Remove idempotency from workflow-crate validation | ✅ `command_review_validation_does_not_check_persistence_idempotency` |
| 2. Distinguish composer hash from descriptor hash | ✅ `blocks_command_composer_hash_mismatch` + `blocks_command_descriptor_hash_mismatch` |
| 3. Snapshot marks review-only + not performed | ✅ `acknowledgment_snapshot_marks_review_only` + `acknowledgment_snapshot_marks_command_not_performed` |
| 4. CLI output says "review recorded" not "executed" | ✅ `cli_acknowledge_output_says_review_recorded_not_executed` + `acknowledgment_snapshot_lines_say_not_performed` |

---

## Review Flow

```
WorkflowCommandComposerRecord (display-only descriptor)
  → WorkflowCommandReviewRequest (composer hash + descriptor hash + loop hash)
    → 12 validation rules
      → Acknowledged → snapshot with acknowledges_review_only=true, command_performed_now=false
      → Rejected → feedback with blocking_reasons
      → ChangesRequested → feedback with requested_changes
```

---

## Key Boundary

Wave 36 records operator review. It does NOT:
- Execute commands, invoke shell/git/process
- Route actions, resolve approvals, reconcile outcomes
- Execute tools, call PolicyEngine, SessionRunner, or LlmClient
- Append trace, mutate memory, or mutate workflow state
- Schedule, queue, retry, resume, or start workers
- Create execution grants or allow execution now
- Claim the command was performed or executed
- Check persistence idempotency in workflow-crate validation
