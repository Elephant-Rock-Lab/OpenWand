# v0.5.0 Roadmap — Post-v0.4 Reset

**Created:** 2026-06-13 (Wave 91A)
**Status:** Planning
**Predecessor:** v0.4.0 stable (`cb6b56b`)

---

## v0.4.0 Stable State (Baseline)

| Metric | Value |
|--------|-------|
| Tests | 3,999 Windows, 0 failures |
| Binary size | 17,853,952 bytes (~17.0 MB) |
| SHA-256 | `6C928123E05FD16B5AA2B223C19E3A990F222C679C90818FC56696CDB028C934` |
| Desktop feature build | PASS (0 errors, 0 warnings) |
| Production crate clippy | 0 warnings (11 crates) |
| App crate warnings | 50 (accepted cosmetic) |
| Operation surfaces | 3 (initiate, approve/reject, export) + refresh |
| Workflow UI surfaces | 10/10 complete, 5/10 live-wired |
| Platforms validated | Windows (full), Linux (compile + tests) |
| Providers validated | 5 models, 2 families |

---

## v0.4.0 Caveats Carried Forward

| # | Caveat | v0.5.0 Target? |
|---|--------|----------------|
| 1 | Not a formal security review | **v0.5.0 candidate** (VE-4) |
| 2 | 50 app clippy warnings | Optional patch |
| 3 | Linux GUI runtime not validated | **v0.5.0 candidate** (VE-3) |
| 4 | macOS validation deferred | Deferred (no macOS env) |
| 5 | Hosted provider validation indirect | Post-v0.5 |
| 6 | Post-v0.3 provider expansion pending | Post-v0.5 |
| 7 | 15 transitive dependency warnings | Upstream-blocked |
| 8 | Windows final-component on 72B path | Accepted |
| 9 | ARID/tool-call-ID mismatch in 88B | Precision refinement |
| 10 | `openwand-content` remains a stub | When rich rendering needed |
| 11 | Synchronous workflow run initiation | Background task (later) |
| 12 | No stable API guarantee | Ongoing |

---

## v0.5.0 Theme

```text
Runtime integrity hardening.
```

v0.4.0 made the desktop an operational surface. v0.5.0 makes the runtime more independently verifiable. The desktop now requests operations — the next question is whether those operations can be proven correct after the fact.

This is a natural progression:
- v0.3.0: observation (display stored workflow data)
- v0.4.0: operation (request workflow operations from desktop)
- v0.5.0: verification (prove operations happened correctly)

---

## Proposed v0.5.0 Blockers

| ID | Name | Description | Environment-Gated? |
|----|------|-------------|-------------------|
| VE-1 | Trace verifier | Append-only trace verification: event ordering, hash/chain validation, tamper detection. Proves the evidence chain is structurally sound at runtime, not just by design. | No |
| VE-2 | Operation replay/audit check | Verify that desktop-requested operations (88A-88C) correspond to governed trace events. Bridges the v0.4 operation surface to the v0.5 verification surface. | No |
| VE-3 | Linux GUI runtime validation | Desktop launches and renders on a native Linux display. Smoke interaction test. | Yes (Linux display) |
| VE-4 | Security review preparation | Threat model refresh incorporating v0.4 desktop operation surface. Authority-boundary checklist for the new DTO→service→gate paths. Dependency caveat refresh. | No |

---

## Backlog Triage

### Category A: Runtime Integrity (v0.5.0 primary)

| Item | Description | Complexity | Priority |
|------|-------------|------------|----------|
| Trace verifier core | `TraceVerifier` that reads trace store and validates append-only ordering, hash chain, event consistency | High | P1 |
| Operation replay | Replay a workflow run from trace events and verify desktop-requested operations match | Medium | P1 |
| Tamper detection test | Prove that modified trace entries are detected by verifier | Medium | P1 |
| Trace verifier CLI | `openwand trace verify` command that exits non-zero on verification failure | Low | P2 |

### Category B: Desktop Operation Hardening (v0.5.0 secondary)

| Item | Description | Complexity | Priority |
|------|-------------|------------|----------|
| Background task for workflow initiation | Move synchronous request_workflow_run off main thread | Medium | P2 |
| ARID precision in UiRunState | Expose canonical ApprovalRequestId instead of tool_call_id | Low | P3 |
| Refresh after evidence export | Best-effort UI refresh after read-only export | Low | P3 |
| Export progress indicator | Show export progress for large packets | Low | P3 |

### Category C: Platform Hardening (environment-gated)

| Item | Description | Complexity | Priority |
|------|-------------|------------|----------|
| Linux GUI runtime | Launch desktop on X11/Wayland, smoke test | Medium | P2 |
| macOS compilation check | Compile workspace + desktop on macOS | Low | P3 |
| Linux release binary | Build optimized binary on Linux | Medium | P2 |

### Category D: Security & Audit (v0.5.0 or later)

| Item | Description | Complexity | Priority |
|------|-------------|------------|----------|
| Threat model refresh | Update for v0.4 desktop operation paths | Medium | P1 |
| Authority-boundary checklist | Systematic review of all DTO→service→gate paths | Medium | P1 |
| Fuzz testing | cargo-fuzz on sandbox, policy, memory, trace verifier | Medium | P2 |
| Dependency caveat refresh | Re-run cargo audit, update DEFERRED_RISKS.md | Low | P2 |

### Category E: Code Quality (ongoing)

| Item | Description | Complexity | Priority |
|------|-------------|------------|----------|
| 50 app clippy warnings | Mechanical fixes in test code | Low | P3 |
| openwand-content stub | Implement or remove | Low | P3 |
| Test-only unsafe cleanup | 2 `unsafe` blocks in `#[cfg(test)]` | Low | P3 |

### Category F: Upstream Dependencies (ongoing monitoring)

| Item | Advisory | Status |
|------|----------|--------|
| Dioxus 0.8+ | 12 GTK3 warnings + 1 unsound | Monitor — wait for upstream |
| loro CRDT update | atomic-polyfill (1) | Monitor — wait for upstream |
| kuchikiki/selectors | fxhash, rand 0.7 (2) | Monitor — wait for upstream |

**Assessment:** 15 warnings, all upstream-blocked. No action until framework authors release updates.

---

## Authority Boundary for v0.5.0

v0.4.0 established:

```text
Desktop UI may REQUEST operations through existing authority gates.
```

v0.5.0 extends:

```text
Runtime verifier may READ and VALIDATE trace/store/workflow records.
The verifier does not mutate, execute, approve, or dispatch.
```

The verifier is a new READ authority, not a new WRITE authority.

---

## Candidate Wave Sequence

| Wave | Description | Depends On |
|------|-------------|------------|
| 91A | Post-v0.4 roadmap reset (this wave) | — |
| 92A | Trace verifier core: append-only + hash chain validation | 91A |
| 92B | Trace verifier tamper detection + CLI command | 92A |
| 93A | Operation replay: desktop operations ↔ trace events | 92B |
| 93B | Operation audit check: automated verification | 93A |
| 94A | Security review preparation: threat model + checklist | 91A |
| 95A | Linux GUI runtime validation (if environment available) | 91A |
| 96A | v0.5.0 release preparation | 93B + 94A |
| 96B | v0.5.0 declaration | 96A |

**Note:** 95A requires a Linux display environment. VE-3 may be deferred.

---

*This roadmap defines v0.5.0 priorities. It does not commit to specific wave content. Actual implementation depends on environment access, emerging priorities, and external feedback on v0.4.0. It adds no feature behavior, no new authority, no policy change, no prompt change, and no unsupported production-readiness claim.*
