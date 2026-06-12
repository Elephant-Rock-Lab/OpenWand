# Release Candidate Readiness Ledger

**Wave:** 72D
**Date:** 2026-06-12
**Commit:** `9e0b0cd` (`wave-72c-lock`, latest)
**Test baseline:** 1,166 tests (1,152 lib + 14 integration), 0 failures

---

## Determination

**PASS — REAL-PROVIDER VALIDATED.** Public RC published and validated against a real
local OpenAI-compatible provider. All emergency blockers resolved. Final-component TOCTOU
hardened. Production-path approval E2E verified with sandbox/schema/executor. CLI surface
truthful. This is a release candidate for external review — not a final release declaration.

---

## RC Artifact Identity

| Field | Value |
|-------|-------|
| Artifact code commit | `d6fa1f0` (`wave-70b-lock`) |
| Latest runtime commit | `9e0b0cd` (`wave-72c-lock`) |
| Packaging metadata commit | `e50356d` (`wave-70c-lock`) |
| Target triple | `x86_64-pc-windows-msvc` |
| Build profile | `release` (optimized) |
| Feature set | `--features desktop` |
| Binary path | `target/release/openwand.exe` |
| Binary size | 17,260,032 bytes (16.4 MB) |
| SHA-256 | `826C5F87CCCD40DC35D58E472E9D8FD3A943F8F0B632508A73B06917061A6159` |
| Rust toolchain | `rustc 1.95.0 (59807616e 2026-04-14)` |

---

## Publication State

**Published. Remote repository contains RC commits and lock tags.**

| Field | Value |
|-------|-------|
| Remote | https://github.com/Octo-Lex/OpenWand |
| Remote master | `9e0b0cd` (`wave-72c-lock`) — verified |
| Local/remote sync | ✅ 0 ahead, 0 behind |
| Total tags | 63 (34 RC-era: wave-52a-lock through wave-72c-lock) |
| Publication date | 2026-06-11 |
| Status | Release candidate for external review — not a final release |

---

## Real-Provider Validation (72C)

| Field | Value |
|-------|-------|
| Provider | LM Studio (OpenAI-compatible) |
| Endpoint | localhost:8766 |
| Model | google/gemma-4-12b (12B, tool-calling capable) |
| Auth | local / no secret recorded |
| Fixture workspace | non-sensitive temp directory |
| Tests | 4/4 PASS |

---

## Release Blocker Status

**6/6 release blockers resolved.**

| Blocker | Wave | Status |
|---------|------|--------|
| Filesystem sandbox escape | 69A | ✅ Resolved |
| Approval workspace swap | 69B | ✅ Resolved |
| Desktop compile failure | 69C | ✅ Resolved |
| Canonical fixture drift | 69C | ✅ Resolved |
| Placeholder verification commands | 69D | ✅ Resolved |
| Mock/unknown production trace attribution | 69E | ✅ Resolved |

---

## Post-Publication Hardening

| Item | Wave | Status |
|------|------|--------|
| Final-component TOCTOU hardening | 72B | ✅ `write_file_no_follow()` with no-follow flags |
| Real-provider validation (LM Studio + gemma-4-12b) | 72C | ✅ 4/4 tests passed |

---

## Validation Closures (70A–72C)

| Item | Wave | Status |
|------|------|--------|
| Full workspace `--all-targets --all-features` | 70B | ✅ Restored |
| 11 non-app crates clippy strict | 70B | ✅ Clean |
| CLI truthful commands | 70A | ✅ Verified |
| Approval post-effect trace ordering | 70A | ✅ Verified |
| Real filesystem approval-effect E2E | 70B | ✅ Verified |
| Production-path approval E2E | 71B | ✅ Verified (3 tests) |
| CLI command surface matches capability matrix | 71A | ✅ Verified (8 binary tests) |
| Approval outcome reporting honest | 71A | ✅ Verified |
| Direct-function tests annotated | 71B | ✅ 5 files annotated |
| Desktop smoke lifecycle | 70A | ✅ Verified |
| Release binary under 20 MB | 70A | ✅ 16.4 MB |
| Cargo audit | 70A | ✅ 0 vulnerabilities |
| Real-provider validation | 72C | ✅ Passed (4/4 tests) |
| Final-component TOCTOU | 72B | ✅ Hardened |

---

## Deferred Risk Summary

| ID | Description | Status |
|----|-------------|--------|
| DEFERRED-001 | App crate test-module clippy warnings (57) | Accepted cosmetic |
| DEFERRED-002 | Cargo audit transitive warnings | Closed by recording (0 vulnerabilities) |
| DEFERRED-003 | unsafe-env-test claim | Closed by claim correction |
| DEFERRED-004 | Trace immutability claim | Closed by documentation downgrade |
| DEFERRED-005 | MutationHelper live-event correctness | Closed with tests + rationale |
| DEFERRED-006 | STATE.md/documentation update | Closed by update |
| DEFERRED-007 | Local branch publication | ✅ Closed — published 2026-06-11 |
| DEFERRED-008 | Sandbox TOCTOU boundary | Partially closed (72B) — final-component hardened; intermediate-directory residual |

**Categories:** 6 closed (002–007), 1 partially closed (008), 1 accepted cosmetic (001).

---

## Remaining Deferred Items

| # | Item | Status | Resolution Path |
|---|------|--------|-----------------|
| 1 | App test-module clippy cleanup | Accepted cosmetic | Crate-level `#![allow(...)]` or separate test-support crate |
| 2 | Transitive dependency warnings (15) | Accepted pending upstream | Re-evaluate when Dioxus/Loro release updates |
| 3 | Intermediate-directory TOCTOU (DEFERRED-008) | Reduced residual (72B) | Handle-relative directory traversal (dirfd/openat) |

---

## Clippy Posture

**Clean (`cargo clippy --all-features -- -D warnings`):**
- openwand-core, openwand-session, openwand-tools, openwand-trace, openwand-store
- openwand-memory, openwand-llm, openwand-policy, openwand-skills, openwand-goals
- openwand-workflow

**Not yet clean:**
- openwand-app: 57 test-module style warnings (accepted cosmetic, all in `#[cfg(test)]`)

---

## Cargo Audit Summary

- **Vulnerabilities:** 0
- **Warnings:** 15 (13 unmaintained, 2 unsound)
- **Direct dependency advisories:** 0
- **All warnings transitive** via Dioxus desktop rendering (13) or Loro CRDT (2)
- **None affect** OpenWand data, crypto, network, or storage paths

---

## Test Baseline

| Crate | Lib Tests | Integration Tests |
|-------|----------:|------------------:|
| openwand-core | 45 | — |
| openwand-session | 49 | 14 |
| openwand-tools | 96 | — |
| openwand-app | 957 | 8 |
| **Total** | **1,152** | **22** |

Session integration tests: 3 production-path + 2 real-file-effect + 2 post-effect + 4 real-provider (ignored without env vars) + 3 other.
App integration tests: 8 binary CLI surface tests.

---

## Tag Sequence

**34 tags** from `wave-52a-lock` through `wave-72c-lock`:

| Range | Count | Waves |
|-------|------:|-------|
| 52A–58A | 7 | Desktop workflow visibility |
| 59A–61A | 3 | Shell decomposition |
| 62A–68A | 7 | Capability-context integration |
| 69A–69G | 7 | Release-blocker remediation + hardening |
| 70A–70D | 4 | RC validation + packaging |
| 71A–71C | 3 | CLI surface truth + E2E honesty + reconciliation |
| 72A–72C | 3 | Publication + TOCTOU hardening + real-provider validation |
| **Total** | **34** | |

---

## Known Gaps

- 9 placeholder UI surfaces (3-line stubs, not yet prioritized)
- `openwand-content` crate removed (scaffold, to be re-added when needed)
- Trace store append-only is structural, not enforced by runtime verifier
- No concurrent mutation tests (architecturally prevented by single-writer run_lock)
- Desktop UI functional correctness not validated (process lifecycle only)
- Remote/hosted provider endpoints not tested
- Non-Windows platforms not tested

---

## Next Decision Point

The RC is **published and validated under one real local provider.** Remaining work is
hardening and release polish, not hidden contradiction blockers.

| Option | Description | Risk |
|--------|-------------|------|
| A. Final release prep | Prepare version bump, changelog, release notes | Intermediate TOCTOU stays as known gap |
| B. Intermediate-directory TOCTOU | Handle-relative directory traversal | Significant platform-specific work |
| C. App clippy cleanup | Suppress or refactor test-module warnings | Cosmetic only |
| D. Dependency-warning upgrade path | Update Dioxus/Loro transitive deps | Upstream-dependent, may not be possible |

---

*After 72D, the public RC is published, real-provider validated under a local model,
TOCTOU-hardened at the final component, and all release blockers are resolved. Final
release declaration remains pending by user decision. This document does not constitute
a final release claim.*
