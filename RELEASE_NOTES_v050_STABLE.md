# OpenWand v0.5.0 - Stable Release Notes

**Release date:** 2026-06-14
**Tag:** `v0.5.0`
**Theme:** Runtime integrity hardening

---

## What's New

v0.5.0 introduces a new **READ authority** to OpenWand: the runtime verifier. After v0.4.0 made the desktop an operational surface (requesting workflow operations), v0.5.0 answers the next question: can those operations be proven correct after the fact?

This release adds two independent verification layers and prepares the project for future security review.

### Verification Layers

| Layer | What it verifies | CLI command |
|-------|-----------------|-------------|
| Trace integrity | Append-only ordering, hash chain continuity (prev_hash to entry_hash per stream), cross-stream consistency, duplicate detection, well-formedness | `openwand trace-verify <session-id>` |
| Operation correspondence | Desktop-requested operations (workflow initiation, approval resolution, evidence export) match governed trace events by explicit IDs | `openwand operation-replay --session <id> --operations <ops.json>` |

### What "verify" means (authority boundary)

The verifier gained **READ authority**, not **WRITE authority**:

```
Runtime verifier may READ and VALIDATE trace/store/workflow records.
The verifier does not mutate, execute, approve, or dispatch.
```

The verifier:
- **May:** Read trace entries, validate ordering/chain/duplicates/correspondence, report findings
- **May not:** Mutate entries, repair chains, recompute hashes, append, execute tools, approve operations, export evidence, instantiate runners

### What v0.5.0 does NOT verify

- **Not full cryptographic hash correctness.** The trace verifier validates chain continuity (prev_hash links to entry_hash). It does not recompute BLAKE3 hashes. This is backend-specific and deferred.
- **Not physical-layer immutability.** The SQLite file is technically mutable by direct database access. The verifier detects tampering after the fact. It does not prevent it at the physical layer.
- **Not workflow execution replay.** "Operation replay" means correspondence verification between operation descriptors and trace events. The verifier does not instantiate runners, tools, exporters, gates, or policies.
- **Not formal security review.** Wave 94A prepared a threat model, authority-boundary checklist, and caveat ledger. No penetration testing, fuzzing, or adversarial probing was performed.

---

## New CLI Commands

### `openwand trace-verify <session-id>`

Validates trace chain integrity for a session. Loads entries from SQLite store (paginated), runs `TraceVerifier::verify()`, prints structured report.

**Exit codes:**
- 0 = Pass
- 1 = Operational error
- 2 = Fail (integrity violation)
- 3 = Inconclusive
- 4 = Unsupported

**What Pass means:** Chain continuity is structurally valid. Each entry's prev_hash links to the prior entry's entry_hash within its stream. Global ordering, per-stream ordering, and cross-ordering are consistent. No duplicates or malformed entries detected.

**What Pass does NOT mean:** Hash values are cryptographically correct. BLAKE3 hashes are not recomputed.

### `openwand operation-replay --session <id> --operations <ops.json>`

Validates correspondence between desktop-requested operations and governed trace evidence.

**Operation descriptors (JSON format):**
```json
{
  "operations": [
    {"type": "workflow_initiation", "workflow_execution_id": "wfx_..."},
    {"type": "approval_resolution", "approval_request_id": "arid_...", "tool_call_id": "tcid_..."},
    {"type": "evidence_export", "workflow_execution_id": "wfx_..."}
  ]
}
```

**Expected results by operation type:**
- Workflow initiation: **Inconclusive** (workflow modules declare appends_trace: false)
- Approval resolution: **Pass** when ARID matches trace event, **Fail** when contradicted
- Evidence export: **Unsupported** (export does not emit trace events)

**Exit codes:** Same scheme as trace-verify.

---

## Metrics

| Metric | Value |
|--------|-------|
| Tests | 4,071 total (0 failures) |
| Test delta from v0.4.0 | +72 tests |
| Binary size | 18,018,816 bytes (~17.2 MB) |
| SHA-256 | `F0BE80A04D3322C8319711AF51C48BC91CED93D01AD20CFD1AC2DB4B85CA2A3D` |
| Production crate clippy | 0 warnings (11 crates, HB-G5) |
| App crate warnings | ~50 (accepted cosmetic, all in test code) |
| Crates | 14 (openwand-content remains stub) |
| Desktop feature build | PASS (0 errors, 0 warnings) |

---

## New Modules (v0.5.0)

| Module | Purpose |
|--------|---------|
| `crates/trace/src/verifier.rs` | `TraceVerifier::verify()` - read-only chain continuity, ordering, duplicate detection |
| `crates/app/src/operation_audit.rs` | `OperationReplayVerifier` - desktop operation to trace correspondence (Note: TD-93B-1) |
| `crates/app/tests/trace_verify_cli.rs` | CLI integration tests for trace-verify (10 tests) |
| `crates/app/tests/operation_replay_cli.rs` | CLI integration tests for operation-replay (8 tests) |
| `crates/app/tests/security_review_prep.rs` | Documentation-presence guards (3 tests) |
| `docs/SECURITY_REVIEW_PREP.md` | Threat model, authority-boundary checklist, caveat ledger |

**Note (TD-93B-1):** `crates/app/src/operation_audit.rs` contains the operation_replay module code. The filename is a maintainability seam from filesystem virtualization during Wave 93B. Functional behavior is correct. Rename is tracked as technical debt.

---

## Wave History (v0.5.0 arc)

| Wave | Commit | Description |
|------|--------|-------------|
| 91A | `6bec666` | Post-v0.4 roadmap reset, v0.5.0 roadmap defined |
| 92A | `e50dc7b` | Trace verifier core: append-only + hash chain validation |
| 92B | `5837d41` | Trace verifier tamper detection + CLI command |
| 93A | `1829cc3` | Operation replay: desktop operations to trace events |
| 93B | `bded8d8` | Operation replay CLI command |
| 94A | `fa08725` | Security review preparation |
| 96A | (this release) | v0.5.0 release preparation |

---

## VE Blocker Resolution

| Blocker | Status | Resolution |
|---------|--------|------------|
| VE-1: Trace verifier | RESOLVED | 92A-92B: TraceVerifier + CLI with tamper detection |
| VE-2: Operation replay | RESOLVED | 93A-93B: OperationReplayVerifier + CLI with correspondence checking |
| VE-3: Linux GUI runtime | DEFERRED | Environment-gated; compile-validated (85A), no display server for runtime |
| VE-4: Security review prep | RESOLVED | 94A: Threat model, authority-boundary checklist, caveat ledger |

**3 of 4 blockers resolved.** VE-3 is explicitly environment-gated and does not block the v0.5.0 verification claim.

---

## Security Review Preparation (Wave 94A)

v0.5.0 includes a consolidated security review preparation document (`docs/SECURITY_REVIEW_PREP.md`) covering:

- Adversary model and attack surface mitigation status
- Authority-boundary checklist for every surface (desktop UI, DTO, service, policy, tools, trace store, verifier, CLI)
- Structured review checklist for external reviewer
- Honest caveat ledger (11 caveats)
- Review-ready assets (release lineage, test counts, CLI commands)

**This is preparation for review, not review itself.** No penetration testing, fuzzing, or adversarial probing was performed.

---

## Caveats

This release carries the following caveats. They do not block the v0.5.0 milestone scope.

| # | Caveat | Status |
|---|--------|--------|
| 1 | Not a formal security review | Security review preparation done (94A); formal review not performed |
| 2 | ~50 app clippy warnings (pedantic/test-only) | Accepted cosmetic |
| 3 | Linux GUI runtime not validated | Compile-only (85A); environment-gated |
| 4 | macOS not validated | No macOS environment |
| 5 | Provider validation limited to 5 models, 2 families | Post-v0.5 |
| 6 | 15 transitive dependency warnings | Upstream-blocked (last audited 82A) |
| 7 | Windows final-component TOCTOU residual | Safe-failure mode (72B) |
| 8 | No full hash recomputation | Verifier validates chain continuity, not BLAKE3 correctness |
| 9 | No full immutability proof | Physical SQLite file is technically mutable; verifier detects after fact |
| 10 | Workflow trace gap | Workflow modules declare appends_trace: false; operation replay reports Inconclusive |
| 11 | Evidence export trace gap | Export does not emit trace events; operation replay reports Unsupported |
| 12 | TD-93B-1: module name debt | operation_audit.rs contains operation_replay code; rename tracked |
| 13 | openwand-content remains a stub | Will be implemented when rich rendering needed |
| 14 | No stable API guarantee | APIs may change in future versions |
| 15 | v0.4.0 caveats inherited | See RELEASE_NOTES_v040_STABLE.md |

---

## What v0.5.0 is NOT

- **Not production-ready.** This is a milestone release for development purposes.
- **Not a formal security review.** Security review preparation was done (94A). Formal review was not performed.
- **Not a stable API guarantee.** APIs may change in future versions.
- **Not full cryptographic verification.** Chain continuity is validated. Hash recomputation is deferred.
- **Not full immutability enforcement.** The verifier detects tampering. It does not prevent physical-layer mutation.
- **Not cross-platform runtime validation.** Linux GUI runtime not validated. macOS not validated.
- **Not a provider expansion release.** Provider matrix remains at v0.4.0 levels.

---

## Authority Posture Summary

```
v0.3.0: Desktop displays stored workflow data (observation)
v0.4.0: Desktop requests workflow operations (operation)
v0.5.0: Runtime verifier validates trace and operation evidence (verification)
```

```
v0.4.0 established: Desktop UI may REQUEST operations through existing authority gates.
v0.5.0 extends:     Runtime verifier may READ and VALIDATE trace/store/workflow records.
                    The verifier does not mutate, execute, approve, or dispatch.
                    The verifier is a new READ authority, not a new WRITE authority.
```

---

## Release Lineage

```
v0.1.0-alpha -> v0.1.0-beta -> v0.2.0-beta -> v0.2.0-rc.1 -> v0.2.0 -> v0.3.0 -> v0.4.0 -> v0.5.0
```

---

*v0.5.0 is stable for milestone scope. It is not production-ready. It is not a formal security review. It does not claim full cryptographic immutability. It adds no new write authority, no policy bypass, no prompt change.*
