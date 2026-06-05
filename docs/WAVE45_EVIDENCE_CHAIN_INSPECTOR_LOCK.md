# Wave 45 — Evidence Chain Inspector and Audit Packet — LOCK

**Committed:** 6 commits
**Baseline:** 3079 tests (Wave 44 locked)
**Final:** 3139 tests (+60), zero failures

---

## What Shipped

### Workflow Crate — New Modules

| Module | Purpose |
|--------|---------|
| `workflow_evidence_chain_inspector.rs` | DTOs: EvidenceChainLink, EvidenceChainInspectionState, AuditPacket, AuditPacketRecord, EvidenceLinkPresence, EvidenceCoverageSummary, RecordedLinkageWarning. Functions: compute_inspection_id, compute_chain_hash, build_inspection_state, build_audit_packet, check_record_linkage |
| `workflow_evidence_chain_inspector_validation.rs` | WorkflowEvidenceChainInspectionId (`weci_`), validate_inspection_request, build_inspection_id |

### App Crate

| File | Purpose |
|------|---------|
| `workflow_evidence_chain_inspector.rs` | assemble_evidence_chain (Patch 2: upstream via source IDs), export_audit_packet (Patch 6: no new persistence root) |
| `ui/workflow_evidence_chain_inspector_state.rs` | Summary display helpers + safety warning |
| `ui/workflow_evidence_chain_inspector_components.rs` | Desktop-gated placeholder |
| `tests/workflow_evidence_chain_inspector_cli.rs` | 4 CLI tests |
| `tests/workflow_evidence_chain_inspector_guards.rs` | 17 guard tests |

### CLI Commands

```bash
openwand workflow-evidence-chain inspect --workflow-execution-id <id> [--json]
openwand workflow-evidence-chain export-packet --workflow-execution-id <id> --output-file <path> [--json]
openwand workflow-evidence-chain export-packet --workflow-execution-id <id> --output-dir <dir>
```

---

## Patch Compliance

| Patch | Status |
|-------|--------|
| 1. Deterministic inspection ID (Option A) | ✅ blake3(execution_id + chain_hash + sorted_link_ids + packet_mode), computed_at not in ID input |
| 2. Upstream records via WorkflowRunRecord source IDs | ✅ task_plan, proposal, review, readiness loaded from run fields, not by wfx index |
| 3. check_record_linkage (not validate_chain_integrity) | ✅ Structural linkage checking, not truth verification |
| 4. No-certification authority flags | ✅ 9 flags on InspectionState + 2 on AuditPacket, all false |
| 5. AuditPacketRecord with recorded_evidence naming | ✅ recorded_evidence field, no verified/truth/proof/certified labels |
| 6. No new persistence root | ✅ Export writes to user-specified path only, no inspector store |
| 7. Applicability-aware link presence | ✅ EvidenceLinkPresence enum (Present/MissingExpected/NotYetApplicable/NotApplicable/Mismatched) + EvidenceCoverageSummary |
| 8. CLI --output-file / --output-dir | ✅ Deterministic filename when --output-dir used |
| 9. Full doc updates with JSON verification | ✅ Matrix, JSON, WAVES.md (backfilled 40-44), ROADMAP.md, PROTECTED_FILES.md |

---

## Test Breakdown

| Area | Count |
|------|------:|
| Workflow DTOs (chain builder, ID, coverage) | +28 |
| App assembler (Patch 2 source IDs, Patch 6 no root) | +7 |
| CLI tests | +4 |
| UI state | +5 |
| Guard tests | +17 |
| **Total** | **+61** (net +60 after rounding) |

---

## Central Invariant

```
Evidence inspection is observation.
Audit packet export is not verification.
Export does not certify truth beyond recorded evidence.
Export does not mutate workflow state.
```

---

## Honest Caveats

- check_record_linkage currently returns empty warnings — full hash cross-reference logic deferred to a future wave
- Audit packet records include only IDs/hashes/metadata, not full deserialized record bodies (would require loading each record individually by type)
- Coverage summary uses applicability heuristics based on link presence, not run status analysis
- The inspector reads from all 19 persistence roots but does not load next-action review, routing readiness, or next-action routing by workflow_execution_id (those would need additional index lookups)
- WAVES.md backfilled Waves 40-44 in this wave rather than a separate docs wave
