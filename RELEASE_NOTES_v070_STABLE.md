# OpenWand v0.7.0 - Stable Release Notes

**Release date:** 2026-06-14
**Tag:** `v0.7.0`
**Theme:** External assurance and operational completeness

---

## What's New

v0.7.0 moves beyond self-consistent internal verification by adding externally
persisted checkpoint anchors, reviewer-facing evidence export, automated scan
evidence, and structured authority-review evidence.

### Three Assurance Strengthenings

| Area | v0.6.0 | v0.7.0 |
|------|--------|--------|
| External anchoring | Not available | **Checkpoint anchors** persisted outside trace store root |
| Security review evidence | Preparation only (94A) | **Automated scanning (105A) + structured authority review (105B)** |
| Evidence UX | Text-only CLI output | **Reviewer-facing JSON evidence report (106A)** |

---

## 1. External Checkpoint Anchors (VG-1)

### Problem solved

v0.6.0 could detect self-inconsistent trace tampering (modified entry with stale hash), but could NOT detect an attacker who modifies the trace AND recomputes all hashes consistently.

### What v0.7.0 adds

- **`openwand anchor-write`** — Creates a checkpoint anchor file containing a BLAKE3 root hash over all trace entry hashes. Written to a user-supplied directory OUTSIDE the trace store root, with canonical path containment enforced.
- **`openwand anchor-verify`** — Verifies the checkpointed prefix of the trace against the externally persisted anchor. Detects post-checkpoint trace modifications.

### Key design decisions

- **Prefix verification:** Anchor covers entries up to `last_global_sequence`. Appended entries make the anchor stale (Pass + Stale), not failed.
- **Separate result + freshness:** `AnchorVerificationResult` (Pass/Fail/Missing/Unsupported) is independent from `AnchorFreshness` (Current/Stale).
- **Path containment:** `anchor_root` must be canonicalized and separate from `store_root` in both directions.

### Known limitation

An attacker who can rewrite BOTH the trace store AND the external anchor file can still produce a self-consistent state. Full immutability requires remote attestation or cryptographic signatures, which are out of scope.

---

## 2. Automated Security Scanning + Authority Review (VG-2)

### Security scan (105A)

- **cargo audit:** 721 dependencies scanned, 0 vulnerabilities (CVEs), 15 upstream-blocked warnings (GTK3 bindings, atomic-polyfill, rand 0.7 — all transitive, desktop-only)
- **Production clippy:** 0 warnings on 12 non-app production crates
- **Authority boundary guards:** All pass (verifier read-only, no backend imports, no execution)
- **Unsafe review:** 1 production usage (`libc::dup` in Unix sandbox — intentional)
- **Results:** `docs/SECURITY_SCAN_RESULTS.md`

### Structured authority review (105B)

- **12 authority surfaces** inventoried (S1–S12)
- **4 write-capable surfaces** identified: UiSessionService, Tool Executor, Session Runner, Anchor Writer
- **3 read-only verifiers** confirmed: TraceVerifier, OperationReplayVerifier, AnchorVerifier
- **7 residual risks** documented with severity and mitigation
- **Results:** `docs/AUTHORITY_REVIEW.md`

### What this is NOT

Not a formal security review. Not penetration testing. Not production-readiness certification.

---

## 3. Evidence Report Export (VG-5)

### Problem solved

Verification results were scattered across multiple CLI commands and documents. External reviewers had no single artifact to review.

### What v0.7.0 adds

- **`openwand evidence-report`** — Generates a structured JSON report aggregating:
  - Live trace verification (chain continuity + hash correctness)
  - Live operation replay (with explicit operation descriptors)
  - Optional anchor verification (if checkpoint file supplied)
  - Sourced security scan summary (from recorded artifact)
  - Sourced authority review summary (from recorded artifact)
  - Standard caveats and honest limitations

### Key design decisions

- **Sourced summaries:** Scan and authority-review data is loaded from recorded artifacts, not re-evaluated. Missing documents produce `status: "unavailable"`, not fake zeros.
- **Explicit operations required:** Operation replay requires `--operations` argument. No inference from trace alone.
- **No overwrite:** Output file collision is rejected.
- **Report-only writes:** The exporter writes ONLY the report file.

---

## 4. Module Naming Cleanup (VF-5, completed 101A)

`operation_audit.rs` renamed to `operation_replay.rs`. All references updated.

---

## Metrics

| Metric | Value |
|--------|-------|
| Tests | 4,176 total (0 failures) |
| Test delta from v0.6.0 | +77 tests |
| Binary size | 18,344,960 bytes (~17.5 MB) |
| SHA-256 | `3CBBB103BC386D579801F2F50EB4E3A27DCB031D015E147C0324EA9B4A02BD3C` |
| Production crate clippy | 0 warnings (12 crates) |
| Crates | 14 (openwand-content remains stub) |
| Desktop feature build | PASS (0 errors) |
| cargo audit | 0 CVEs, 15 upstream-blocked warnings |

---

## New / Changed CLI Commands

| Command | Description | Since |
|---------|-------------|-------|
| `openwand anchor-write` | Create checkpoint anchor file outside store root | 104B |
| `openwand anchor-verify` | Verify trace against external anchor | 104B |
| `openwand evidence-report` | Generate aggregated evidence report JSON | 106A |
| `openwand trace-verify` | Verify trace chain + hash correctness | 92B/98B |
| `openwand operation-replay` | Verify operation-to-trace correspondence | 93B |

---

## New / Changed Modules (v0.7.0)

| Module | Change |
|--------|--------|
| `crates/trace/src/anchor.rs` | **New.** CheckpointAnchor DTO, root-hash computation, CheckpointWriter, path containment, verify_anchor |
| `crates/app/src/evidence_report.rs` | **New.** EvidenceReport DTO, source loaders, standard caveats |
| `crates/app/src/main.rs` | Added `anchor-write`, `anchor-verify`, `evidence-report` CLI commands |

---

## Wave History (v0.7.0 arc)

| Wave | Commit | Description |
|------|--------|-------------|
| 103A | `e2a1040` | Post-v0.6 roadmap reset |
| 104A | `950fbbc` | External anchor design (DTOs, root hash, verification) |
| 104B | `ee096c7` | CheckpointWriter + CLI + path containment |
| 105A | `0f0ba89` | Automated security scanning (cargo audit, clippy, guards) |
| 105B | `030245e` | Structured authority review (12 surfaces) |
| 106A | `4cefdd7` | Evidence report export |
| 107A | (this release) | v0.7.0 release preparation |

---

## VG Blocker Resolution

| Blocker | Status | Resolution |
|---------|--------|------------|
| VG-1: External anchor/checkpoint | RESOLVED | 104A-104B: CheckpointWriter + verify_anchor + CLI |
| VG-2: Security review execution | RESOLVED | 105A-105B: Automated scanning + structured authority review |
| VG-5: Evidence UX hardening | RESOLVED | 106A: Evidence report export with sourced summaries |
| VG-3: Linux GUI runtime | DEFERRED | Environment-gated; compile-validated only |
| VG-4: Provider validation expansion | DEFERRED | Strategic; LM Studio + Z.AI validated |

**3 of 5 blockers resolved.** VG-3 and VG-4 are deferred without blocking the v0.7.0 assurance claim.

---

## Caveats

| # | Caveat | Status |
|---|--------|--------|
| 1 | Not a formal security review | Authority review + scan done; formal review not performed |
| 2 | External anchor also mutable | Attacker who controls both store AND anchor can produce consistent state |
| 3 | No remote attestation | Local file checkpoint is stronger than self-contained DB but not remote |
| 4 | Not physical immutability | Verifier + anchor detect after fact; do not prevent |
| 5 | Linux GUI runtime not validated | Compile-only; environment-gated |
| 6 | macOS not validated | No environment |
| 7 | Provider validation limited | 5 models, 2 families (LM Studio, Z.AI) |
| 8 | Upstream dependency warnings | 15 warnings, all upstream-blocked, 0 CVEs |
| 9 | Windows final-component TOCTOU residual | Safe-failure mode |
| 10 | openwand-content stub | When rich rendering needed |
| 11 | No stable API guarantee | APIs may change |
| 12 | Not production-ready | Milestone release for development purposes |
| 13 | v0.6.0 caveats inherited | See RELEASE_NOTES_v060_STABLE.md |

---

## Authority Posture Summary

```
v0.3.0: Desktop displays stored workflow data (observation)
v0.4.0: Desktop requests workflow operations (operation)
v0.5.0: Runtime verifier validates trace and operation evidence (verification)
v0.6.0: Verifier recomputes hashes and reads new operation trace evidence (hardening)
v0.7.0: External anchors, scan evidence, authority review, evidence reports (external assurance)
```

```
v0.5.0: Runtime verifier may READ and VALIDATE.
v0.6.0: Verifier gains DEEPER READ (hash recomputation under canonical policy).
        Service path gains new WRITE (trace emissions for workflow/export).
v0.7.0: New anchor WRITER (files outside store root, path-containment-gated).
        New evidence REPORT WRITER (aggregation only, no new assurance facts).
        All verifiers remain READ-ONLY.
```

---

## Release Lineage

```
v0.1.0-alpha -> v0.1.0-beta -> v0.2.0 -> v0.3.0 -> v0.4.0 -> v0.5.0 -> v0.6.0 -> v0.7.0
```

---

*v0.7.0 is stable for milestone scope. It is not production-ready. It is not a formal security review. It does not claim physical immutability, remote attestation, or provider completeness. It adds externally persisted checkpoint anchors and reviewer-facing evidence export while preserving the boundary that verifiers read and report but do not mutate, repair, execute, approve, or export.*
