# Wave 47 — Read-Only Verification Readiness — LOCK

**Committed:** 6 commits
**Baseline:** 3203 tests (Wave 46 locked)
**Final:** 3266 tests (+63), zero failures

---

## What Shipped

### Workflow Crate — New Modules

| Module | Purpose |
|--------|---------|
| `workflow_verification_readiness.rs` | DTOs: VerificationReadinessRecord, Request, Status, Decision, 15 predicates, 14 no-authority flags |
| `workflow_verification_readiness_evaluator.rs` | Target-specific evaluators: evaluate_manual_result_readiness, evaluate_attestation_readiness, evaluate_audit_packet_readiness |

### App Crate

| File | Purpose |
|------|---------|
| `workflow_verification_readiness.rs` | Persistence: save, load, list, latest, by_workflow_run, by_target, by_target_id |
| `ui/workflow_verification_readiness_state.rs` | Summary display + safety warning |
| `ui/workflow_verification_readiness_components.rs` | Desktop-gated placeholder |
| `tests/workflow_verification_readiness_cli.rs` | 4 CLI tests |
| `tests/workflow_verification_readiness_guards.rs` | 15 guard tests |

### CLI Commands

```bash
openwand workflow-verification-readiness evaluate \
  --target-kind <kind> --target-id <id> \
  --workflow-execution-id <id> --expected-target-hash <hash> [--json]

openwand workflow-verification-readiness show --readiness-id <id> [--json]
openwand workflow-verification-readiness latest --workflow-execution-id <id> [--json]
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. Ready/Blocked/Inconclusive status (not NotReady) | ✅ 2 tests |
| 2. expected_target_hash + workflow_execution_id match predicates | ✅ 3 tests |
| 3. Target-specific status eligibility | ✅ 7 tests (manual_result, review, gate statuses) |
| 4. Manual result requires latest accepted review | ✅ 4 tests |
| 5. Attestation readiness preserves unverified semantics | ✅ 4 tests |
| 6. Audit packet uses in-memory data only, no file reads | ✅ 3 tests |
| 7. 14 structural no-authority flags | ✅ 5 tests + serialized guard |
| 8. Persistence with target indexes + idempotency rules | ✅ 8 tests |
| 9. CLI requires expected-target-hash, forbids verification verbs | ✅ 4 CLI tests |

---

## Test Breakdown

| Area | Count |
|------|------:|
| Workflow DTOs (predicates, status, hash binding, status eligibility) | +20 |
| Workflow evaluator (Patch 4/5/6) | +13 |
| App persistence (Patch 8) | +8 |
| CLI tests | +4 |
| UI state | +3 |
| Guard tests | +15 |
| **Total** | **+63** |

---

## Central Invariant

```
Verification readiness is not verification.
Eligibility is not trust promotion.
Readiness does not fetch, read, execute, verify signatures, inspect artifacts,
call shell/git, mutate workflow state, schedule verification, or certify truth.
```

---

## Honest Caveats

- The `evaluate` CLI command uses `evaluate_readiness_metadata_only` which passes "reported_succeeded" by default — a full evaluator would load the actual target record from persistence
- Target-specific evaluators (manual result, attestation, audit packet) are available in the workflow crate but not yet wired into the CLI
- `VerificationReadinessTargetKind::Other` from the plan was not needed — 5 concrete types cover all evidence types
- Patch 8 idempotency for Ready records deduplicates on target_id + expected_target_hash, not on a more granular target record hash
- The attestation readiness evaluator requires the full `WorkflowExternalAttestation` object — it does not load it from persistence
- Audit packet readiness takes in-memory metadata (chain_hash, certifies flag, workflow_id) — does not re-open exported packet files
