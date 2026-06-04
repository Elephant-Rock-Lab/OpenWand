# Wave 35 — Manual Operation Command Composer — LOCK

**Committed:** 6 commits
**Baseline:** 2601 tests (Wave 34 locked)
**Final:** ~2679 tests, zero failures

---

## What Shipped

### New Modules in `openwand-workflow`

| Module | Purpose |
|--------|---------|
| `workflow_manual_operation.rs` | Command kind, argument, missing input, evidence link DTOs |
| `workflow_command_descriptor.rs` | Display-only descriptor (`display_only=true`, `executable=false`) |
| `workflow_command_composer.rs` | Composer record, 15 predicates, composition engine |

### New Modules in `openwand-app`

| File | Purpose |
|------|---------|
| `workflow_command_composer.rs` | Persistence under `workflow_command_composer/` |
| `ui/workflow_command_composer_state.rs` | UI view helpers + safety warning |
| `ui/workflow_command_composer_components.rs` | Desktop-gated placeholder |
| `main.rs` additions | CLI: `workflow-command compose/show/latest` |

---

## Test Breakdown

| Area | Count |
|------|------:|
| DTO / Validation (incl. Patch 3) | 12 |
| Composition Engine (incl. Patch 4) | 14 |
| Predicate Gate | 10 |
| Persistence / Idempotency (incl. Patch 5) | 15 |
| CLI | 6 |
| UI State | 7 |
| Guard / No-Mutation (incl. Patch 2+3) | 16 |
| **Total** | **80** |

---

## Central Invariant

```
The loop controller recommends.
The command composer describes.
The operator performs.
The system does not execute.
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. Test count accounting | ✅ Fixed target to ~2679 |
| 2. Workflow crate dep guard | ✅ `workflow_crate_dependency_guard_still_allows_only_6_deps` confirms exactly 6 |
| 3. No process execution fields | ✅ `workflow_command_descriptor_has_no_process_execution_fields` + `workflow_command_serialized_json_contains_no_argv_cwd_env_or_stdin` + `workflow_command_serialized_json_contains_no_shell_process_or_executable_fields` |
| 4. Operator decision alternatives | ✅ `review_command_requires_operator_review_decision_missing_input` + `approval_outcome_command_requires_operator_resolution_missing_input` + `composer_never_defaults_to_approve_or_reject` |
| 5. Governance + provider no-write | ✅ `command_composer_does_not_write_governance_records` + `command_composer_does_not_write_provider_records` |

---

## Composition Flow

```
WorkflowLoopControllerRecord (recommendation)
  → WorkflowCommandComposerRequest (loop controller ID + hash)
    → 15 predicates validate evidence
      → DescriptorReady → WorkflowManualCommandDescriptor
        → display_command, arguments, missing_inputs, copyable_text
        → display_only=true, executable=false
      → MissingInputs → descriptor with missing_inputs listed
      → NoCommandRequired → no descriptor
```

---

## Key Boundary

Wave 35 composes display-only descriptors. It does NOT:
- Execute commands, invoke shell/git/process
- Route actions, resolve approvals, reconcile outcomes
- Execute tools, call PolicyEngine, SessionRunner, or LlmClient
- Append trace, mutate memory, or mutate workflow state
- Schedule, queue, retry, resume, or start workers
- Default to approve/reject on behalf of the operator
