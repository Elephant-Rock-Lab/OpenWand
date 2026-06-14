# v0.6.0 Roadmap - Post-v0.5 Reset

**Created:** 2026-06-14 (Wave 97A)
**Status:** Complete (Wave 102A)
**Predecessor:** v0.5.0 stable (`b70968e`)

---

## v0.5.0 Stable State (Baseline)

| Metric | Value |
|--------|-------|
| Tests | 4,071 Windows, 0 failures |
| Binary size | 18,018,816 bytes (~17.2 MB) |
| SHA-256 | `F0BE80A04D3322C8319711AF51C48BC91CED93D01AD20CFD1AC2DB4B85CA2A3D` |
| Desktop feature build | PASS (0 errors, 0 warnings) |
| Production crate clippy | 0 warnings (11 crates) |
| App crate warnings | ~50 (accepted cosmetic, all in test code) |
| Trace verifier | Chain continuity + ordering + duplicates (92A-92B) |
| Operation replay | Correspondence with honest Inconclusive/Unsupported gaps (93A-93B) |
| Security review prep | Threat model + authority-boundary checklist + caveat ledger (94A) |
| Platforms validated | Windows (full), Linux (compile + tests) |

---

## v0.5.0 Verification Gaps (What v0.6.0 Targets)

v0.5.0 established read-only verification with two honest gaps:

### Gap 1: No hash recomputation

The verifier validates chain continuity (`prev_hash` links to prior `entry_hash` per stream). It does **not** recompute BLAKE3 to confirm the stored `entry_hash` itself is correct. A tampered entry that updates both `entry_hash` and the next entry's `prev_hash` consistently would pass chain-continuity verification.

**Important caveat:** Even with recomputation, an attacker who can rewrite the trace store and recompute all hashes can still produce a self-consistent trace unless there is an external anchor, signature, checkpoint, or append-only storage guarantee. VF-1 moves from *chain continuity* to *stored hash correctness under the canonical hash policy*, not to *full immutability*.

### Gap 2: Incomplete operation coverage

Workflow initiation reports **Inconclusive** (workflow modules declare `appends_trace: false`). Evidence export reports **Unsupported** (export does not emit trace events). The operation replay verifier cannot move beyond these because the underlying trace evidence does not exist.

---

## v0.6.0 Theme

```
Evidence-backed assurance hardening.
```

v0.5.0: verify chain continuity and operation correspondence.
v0.6.0: strengthen the evidence behind those verification claims.

---

## Proposed VF Blockers

| ID | Name | Description | Priority |
|----|------|-------------|----------|
| VF-1 | Backend hash-correctness verification | Verifier recomputes BLAKE3 entry_hash and compares to stored value. Moves from chain-continuity to hash-correctness verification. Does NOT claim full physical immutability. | P1 |
| VF-2 | Trace-backed operation coverage expansion | Add trace evidence for workflow initiation and/or evidence export so operation-replay can report Pass/Fail instead of Inconclusive/Unsupported for more operation classes. | P1 |
| VF-3 | Security review execution package | Turn SECURITY_REVIEW_PREP into an actual review workflow or external reviewer packet. | P2 |
| VF-4 | Linux GUI runtime validation | Close the long-standing display-runtime caveat if environment is available. | P2 (environment-gated) |
| VF-5 | TD-93B-1 module naming maintenance | Rename `operation_audit.rs` to `operation_replay.rs` and align module names. | P3 (opportunistic) |

---

## VF-1 Design Considerations

The verifier currently lives in `crates/trace/` which does not depend on `openwand-store`. VF-1 must not create a direct dependency from `openwand-trace` to `openwand-store`. Three architectural options:

**Option A:** `TraceVerifier` accepts a `HashVerificationPolicy` trait. The store-backed layer supplies the SQLite/BLAKE3 implementation.

**Option B:** Core verifier stays in `openwand-trace` (chain continuity + ordering). Store/app layer performs hash-correctness checks as a separate verification step.

**Option C:** Move only backend-specific hash verification into a separate adapter module that depends on both `openwand-trace` and `openwand-store`.

The architectural decision will be made in the VF-1 design wave (98A). Whatever option is chosen:

- The verifier does not gain write authority.
- The verifier does not claim full physical immutability.
- The verifier gains deeper READ capability (hash recomputation against the canonical policy).

### What VF-1 still does NOT prove

Even with hash recomputation, an attacker who can rewrite the trace store and recompute all hashes can still produce a self-consistent trace unless there is an external anchor, signature, checkpoint, or append-only storage guarantee. VF-1 strengthens the verification from "chain links are structurally consistent" to "stored hashes match recomputed hashes under the canonical policy." Full immutability requires an external trust anchor, which is out of scope for v0.6.0.

---

## VF-2 Design Considerations

Workflow modules currently declare `appends_trace: false`. To add trace evidence:

- Add `WorkflowEvent::ModStarted` / `ModCompleted` emission to the workflow run lifecycle (for workflow initiation coverage)
- Add `ArtifactEvent::Generated` emission to the evidence export path (for evidence export coverage)
- Update operation replay verifier to match against these new events

This is a behavioral change (new trace emissions from the execution path), not just documentation. It must be planned carefully to avoid breaking the append-only invariant.

**VF-2 is independent of VF-1.** Adding trace emissions does not depend on hash recomputation. Both strengthen the verification claim from different angles.

---

## VF-5 Scheduling

TD-93B-1 (module name debt) is low risk. It should be done opportunistically before or during early v0.6.0 waves. Do not let v0.6.0 release carry this debt if the rename is cheap. It may be done as a prelude to 98A or as a standalone maintenance commit.

---

## Authority Boundary for v0.6.0

v0.5.0 established:
```
Runtime verifier may READ and VALIDATE trace/store/workflow records.
The verifier does not mutate, execute, approve, or dispatch.
```

v0.6.0 maintains this boundary. The verifier gains **deeper read** capability (hash recomputation against canonical policy) but does not gain any write authority.

If VF-2 adds new trace emissions, those emissions come from the existing execution path (SessionRunner, workflow lifecycle), not from the verifier. The verifier remains read-only.

```
Strengthen evidence, but keep the assurance boundary honest.
```

---

## Candidate Wave Sequence

| Wave | Description | Depends On |
|------|-------------|------------|
| 97A | Post-v0.5 roadmap reset (this wave) | - |
| 98A | Hash verification policy / architecture decision (VF-1) | 97A |
| 98B | Backend hash recomputation implementation (VF-1) | 98A |
| 99A | Trace-backed workflow initiation evidence (VF-2) | 93A/93B |
| 99B | Trace-backed evidence export evidence (VF-2) | 99A |
| 100A | Security review execution package (VF-3) | 97A |
| 101A | TD-93B-1 module naming maintenance (VF-5) | 97A (opportunistic, before 98A if cheap) |
| 102A | v0.6.0 release preparation | 98B + 99B |
| 102B | v0.6.0 declaration | 102A |

**Note:** VF-4 (Linux GUI runtime) is environment-gated and may be attempted at any point if a display environment becomes available.

Wave sequence may be compressed (combine 98A/98B, 99A/99B) if the design decisions are straightforward.

---

## v0.5.0 Caveats Carried Forward

| # | Caveat | v0.6.0 Target? |
|---|--------|----------------|
| 1 | Not a formal security review | v0.6.0 candidate (VF-3) |
| 2 | ~50 app clippy warnings (test-only) | Optional; refresh during release prep |
| 3 | Linux GUI runtime not validated | v0.6.0 candidate (VF-4, environment-gated) |
| 4 | macOS validation deferred | Deferred (no macOS env) |
| 5 | Provider validation limited (5 models, 2 families) | Post-v0.6 |
| 6 | 15 transitive dependency warnings | Upstream-blocked; refresh audit before any production-readiness or security-assurance claim |
| 7 | Windows final-component TOCTOU residual | Accepted (safe-failure mode) |
| 8 | No full hash recomputation | **v0.6.0 primary target (VF-1)** |
| 9 | No full immutability proof | Accepted (requires external trust anchor; out of scope) |
| 10 | Workflow trace gap | **v0.6.0 primary target (VF-2)** |
| 11 | Evidence export trace gap | **v0.6.0 primary target (VF-2)** |
| 12 | TD-93B-1 module name debt | **v0.6.0 target (VF-5, opportunistic)** |
| 13 | openwand-content stub | When rich rendering needed |
| 14 | No stable API guarantee | Ongoing |
| 15 | Workflow modules do not emit trace | **v0.6.0 primary target (VF-2)** |

---

## Backlog Triage

### Category A: Evidence Hardening (v0.6.0 primary)

| Item | Description | Complexity | Priority |
|------|-------------|------------|----------|
| Hash verification policy | Define how the verifier obtains hash recomputation capability without wrong crate dependencies | Medium | P1 |
| Backend hash recomputation | Implement BLAKE3 hash recomputation and comparison | Medium | P1 |
| Workflow trace emission | Add ModStarted/ModCompleted trace events to workflow run lifecycle | Medium | P1 |
| Export trace emission | Add ArtifactGenerated trace event to evidence export path | Low-Medium | P1 |

### Category B: Security & Audit (v0.6.0 secondary)

| Item | Description | Complexity | Priority |
|------|-------------|------------|----------|
| Security review execution | Turn prep doc into actual review packet or workflow | Medium | P2 |
| Dependency audit refresh | Re-run cargo audit, update DEFERRED_RISKS.md | Low | P2 |

### Category C: Platform Hardening (environment-gated)

| Item | Description | Complexity | Priority |
|------|-------------|------------|----------|
| Linux GUI runtime | Launch desktop on X11/Wayland, smoke test | Medium | P2 |
| macOS compilation check | Compile workspace + desktop on macOS | Low | P3 |

### Category D: Code Quality (ongoing)

| Item | Description | Complexity | Priority |
|------|-------------|------------|----------|
| TD-93B-1 rename | operation_audit.rs to operation_replay.rs | Low | P3 (do early) |
| App clippy warnings | Legacy cosmetic/pedantic clippy warnings accepted where documented; production clippy status should be refreshed during release prep | Low | P3 |
| openwand-content stub | Implement or remove | Low | P3 |

---

*This roadmap defines v0.6.0 priorities. It does not commit to specific wave content. Actual implementation depends on architectural decisions, emerging priorities, and external feedback on v0.5.0. It adds no feature behavior, no new authority, no policy change, no prompt change, and no unsupported production-readiness or assurance claim.*
