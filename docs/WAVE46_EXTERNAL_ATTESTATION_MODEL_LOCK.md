# Wave 46 — External Attestation Model — LOCK

**Committed:** 6 commits
**Baseline:** 3139 tests (Wave 45 locked)
**Final:** 3203 tests (+64), zero failures

---

## What Shipped

### Workflow Crate — New Modules

| Module | Purpose |
|--------|---------|
| `workflow_external_attestation.rs` | DTOs: WorkflowExternalAttestation, ExternalAttestationTarget, ExternalAttestationReference, ExternalReportedSignature, ExternalAttestationSource, ExternalAttestationKind, ExternalAttestationTargetKind, ExternalAttestationReferenceKind |
| `workflow_external_attestation_validation.rs` | Request validation (target_id, claim, source_name, idempotency_key non-empty) |

### Workflow Crate — Modified

| Module | Change |
|--------|--------|
| `workflow_evidence_chain_inspector.rs` | Added `AuditPacketAttestationLink`, `reported_attestations` field on `AuditPacket` |

### App Crate

| File | Purpose |
|------|---------|
| `workflow_external_attestation.rs` | Persistence: save, load, list, latest, by_workflow_run, by_target, by_target_id, by_kind, by_source |
| `ui/workflow_external_attestation_state.rs` | Summary display + safety warning |
| `ui/workflow_external_attestation_components.rs` | Desktop-gated placeholder |
| `tests/workflow_external_attestation_cli.rs` | 4 CLI tests |
| `tests/workflow_external_attestation_guards.rs` | 16 guard tests |

### CLI Commands

```bash
openwand workflow-external-attestation attach \
  --workflow-execution-id <id> --target-kind <kind> --target-id <id> \
  --kind <kind> --source-name <name> --claim <claim> [--json]

openwand workflow-external-attestation show --attestation-id <id> [--json]
openwand workflow-external-attestation list --workflow-execution-id <id> [--json]
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. Vec<ExternalAttestationReference> (metadata-only) | ✅ 5 tests proving no file/URL fetch, no file bytes |
| 2. ExternalAttestationTargetKind enum with typed IDs | ✅ snake_case serialization, workflow_execution_id on target |
| 3. ExternalReportedSignature (NotVerifiedByOpenWand only) | ✅ 3 tests proving no trust promotion |
| 4. No trust scoring, no confidence, no promotion fields | ✅ 5 tests + serialized guard |
| 5. Persistence indexes (workflow_run, target, target_id, kind, source) | ✅ 8 persistence tests with idempotency |
| 6. Inspector integration (reported_attestations, not chain validity) | ✅ 5 tests proving attestations don't affect chain |
| 7. CLI without verify/trust/certify semantics | ✅ 4 CLI tests |
| 8. Full doc updates with JSON verification | ✅ Matrix, JSON, WAVES.md, ROADMAP.md, PROTECTED_FILES.md |

---

## Test Breakdown

| Area | Count |
|------|------:|
| Workflow DTOs (attestation, validation, references, signature) | +28 |
| Workflow inspector (Patch 6 attestation links) | +5 |
| App persistence (idempotency, indexes) | +8 |
| CLI tests | +4 |
| UI state | +3 |
| Guard tests | +16 |
| **Total** | **+64** |

---

## Central Invariant

```
External attestation is reported evidence.
It is not verification.
It is not trust promotion.
It is not reconciliation.
It does not certify external truth.
```

---

## Honest Caveats

- `ExternalAttestationTargetKind::Other` variant exists for future extensibility but has no special handling
- `by_kind` and `by_source` indexes use a single file per key (not optimized for high cardinality)
- Audit packet attestation links are populated by the app assembler only if attestations exist for the workflow run
- Inspector integration is minimal — attestations are metadata-only, not part of chain hash
- Target hash validation (`expected_target_hash`) is recorded but not checked at attachment time in the workflow crate (would require loading the target record)
- `ExternalSignatureVerificationStatus` has only one variant — future waves may add verified statuses after explicit verification gates
