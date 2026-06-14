# OpenWand v0.6.0 - Stable Release Notes

**Release date:** 2026-06-14
**Tag:** `v0.6.0`
**Theme:** Evidence-backed assurance hardening

---

## What's New

v0.6.0 strengthens the evidence behind v0.5.0 verification claims. Where v0.5 could verify chain continuity and operation correspondence, v0.6 verifies hash correctness and expands operation trace coverage.

### Two Verification Strengthenings

| Area | v0.5.0 | v0.6.0 |
|------|--------|--------|
| Hash verification | Chain continuity only (prev_hash links) | **Hash correctness** (BLAKE3 recomputation under Blake3HashPolicy) |
| Workflow initiation | Always Inconclusive | **Pass/Fail** when trace evidence exists |
| Evidence export | Always Unsupported | **Pass/Fail** when trace evidence exists |

### Hash Correctness Verification (VF-1)

The trace verifier now recomputes BLAKE3 entry hashes and compares them to stored values. This catches content tampering where an attacker modifies an entry's event payload but leaves the stored `entry_hash` unchanged - an attack that chain-continuity-only verification would miss.

**Architecture:** `HashVerificationPolicy<E>` trait injection. The generic `openwand-trace` verifier accepts a policy object that knows how to serialize events and compute canonical hashes. The `openwand-store` crate supplies the `Blake3HashPolicy` implementation for `StoredEvent`.

**Known limitation:** Even with hash recomputation, an attacker who can rewrite the trace store AND recompute all hashes can still produce a self-consistent trace. Full immutability requires an external trust anchor (signature, checkpoint, or append-only storage guarantee), which is out of scope.

### Trace-backed Operation Coverage (VF-2)

The governed execution path now emits trace evidence for two operation classes that previously had none:

**Workflow initiation (99A):** After a successful workflow run creation, the service emits `WorkflowEvent::ModStarted` and `WorkflowEvent::ModCompleted` trace events. Operation replay can now verify correspondence for new traces.

**Evidence export (99B):** After a successful audit packet export, the service emits `ArtifactEvent::Generated` trace events. Operation replay can now verify correspondence for new traces.

**Legacy compatibility:** Traces from v0.4/v0.5 without these events remain Inconclusive (workflow initiation) or Unsupported (evidence export). They do not retroactively become Pass or Fail.

### Module Naming Cleanup (VF-5 / TD-93B-1)

`operation_audit.rs` renamed to `operation_replay.rs`. All references updated. Behavior-neutral.

---

## Metrics

| Metric | Value |
|--------|-------|
| Tests | 4,099 total (0 failures) |
| Test delta from v0.5.0 | +28 tests |
| Binary size | 18,027,008 bytes (~17.2 MB) |
| SHA-256 | `A9C00D5BBA402BDB42FA6E2E595C90612126E0FD604ED4066D5A27174AE860AC` |
| Production crate clippy | 0 warnings (11 crates, HB-G5) |
| App crate warnings | ~50 (accepted cosmetic, all in test code) |
| Crates | 14 (openwand-content remains stub) |
| Desktop feature build | PASS (0 errors, 0 warnings) |

---

## New / Changed Modules (v0.6.0)

| Module | Change |
|--------|--------|
| `crates/trace/src/verifier.rs` | Added `HashVerificationPolicy<E>` trait, `Blake3HashPolicy` struct, `verify_with_hash_policy()` method |
| `crates/store/src/envelope.rs` | Implements `HashVerificationPolicy<StoredEvent> for Blake3HashPolicy` |
| `crates/store/src/backends/sqlite/hash.rs` | Delegates to canonical `Blake3HashPolicy::compute_hash()` in trace crate |
| `crates/app/src/ui/service.rs` | Instance `request_workflow_run()` emits workflow trace events; new `request_evidence_export()` emits artifact trace events |
| `crates/app/src/operation_replay.rs` | Updated `verify_exp()` for stream+hash matching; renamed from `operation_audit.rs` |
| `crates/app/src/main.rs` | `trace-verify` CLI uses `verify_with_hash_policy()` with honest output about scope |

---

## Wave History (v0.6.0 arc)

| Wave | Commit | Description |
|------|--------|-------------|
| 97A | `ade07e1` | Post-v0.5 roadmap reset |
| 98A | `519655b` | Hash verification policy / architecture decision |
| 98B | `38397a2` | Backend hash recomputation CLI integration |
| 99A | `387796f` | Trace-backed workflow initiation |
| 99B | `4fedb11` | Trace-backed evidence export |
| 101A | `7aa5840` | TD-93B-1 module naming maintenance |
| 102A | (this release) | v0.6.0 release preparation |

---

## VF Blocker Resolution

| Blocker | Status | Resolution |
|---------|--------|------------|
| VF-1: Backend hash-correctness | RESOLVED | 98A-98B: HashVerificationPolicy + CLI integration |
| VF-2: Trace-backed operation coverage | RESOLVED | 99A-99B: Workflow initiation + evidence export trace |
| VF-3: Security review execution | DEFERRED | Post-v0.6 |
| VF-4: Linux GUI runtime | DEFERRED | Environment-gated |
| VF-5: TD-93B-1 naming | RESOLVED | 101A: Renamed to operation_replay.rs |

**3 of 5 blockers resolved.** VF-3 and VF-4 are deferred without blocking the v0.6.0 assurance claim.

---

## Caveats

| # | Caveat | Status |
|---|--------|--------|
| 1 | Not a formal security review | Security review prep done (94A); formal review not performed |
| 2 | ~50 app clippy warnings | Accepted cosmetic |
| 3 | Linux GUI runtime not validated | Compile-only; environment-gated |
| 4 | macOS not validated | No environment |
| 5 | Provider validation limited (5 models, 2 families) | Post-v0.6 |
| 6 | Transitive dependency warnings | Upstream-blocked; refresh before any assurance claim |
| 7 | Windows final-component TOCTOU residual | Safe-failure mode |
| 8 | No external trust anchor | Self-consistent tamper still possible if attacker controls full store |
| 9 | No physical immutability proof | Verifier detects after fact; does not prevent physical mutation |
| 10 | Legacy traces not retroactively verifiable | Workflow initiation remains Inconclusive; export remains Unsupported for pre-99A/99B traces |
| 11 | openwand-content stub | When rich rendering needed |
| 12 | No stable API guarantee | APIs may change |
| 13 | v0.5.0 caveats inherited | See RELEASE_NOTES_v050_STABLE.md |

---

## What v0.6.0 is NOT

- **Not production-ready.** Milestone release for development purposes.
- **Not formal security review.** Preparation done; review not performed.
- **Not full cryptographic immutability.** Hash correctness verified under policy. Physical-layer immutability requires external trust anchor.
- **Not retroactive verification.** Legacy traces from v0.4/v0.5 without workflow/export events remain Inconclusive/Unsupported.
- **Not cross-platform runtime validation.** Linux GUI not validated. macOS not validated.
- **Not stable API guarantee.** APIs may change.

---

## Authority Posture Summary

```
v0.3.0: Desktop displays stored workflow data (observation)
v0.4.0: Desktop requests workflow operations (operation)
v0.5.0: Runtime verifier validates trace and operation evidence (verification)
v0.6.0: Verifier recomputes hashes and reads new operation trace evidence (hardening)
```

```
v0.5.0: Runtime verifier may READ and VALIDATE.
v0.6.0: Verifier gains DEEPER READ (hash recomputation under canonical policy).
        Service path gains new WRITE (trace emissions for workflow/export).
        Verifier does NOT gain any new WRITE authority.
```

---

## Release Lineage

```
v0.1.0-alpha -> v0.1.0-beta -> v0.2.0 -> v0.3.0 -> v0.4.0 -> v0.5.0 -> v0.6.0
```

---

*v0.6.0 is stable for milestone scope. It is not production-ready. It is not a formal security review. It does not claim full cryptographic immutability or physical-layer tamper prevention. It adds no new write authority to the verifier, no policy bypass, no prompt change.*
