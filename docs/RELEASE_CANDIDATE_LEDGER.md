# Release Candidate Readiness Ledger

**Release:** v0.1.0-alpha
**Date:** 2026-06-12
**Commit:** `b9a2138` (`wave-74b-lock`, release baseline)
**Release tag:** `v0.1.0-alpha`
**Test baseline:** 2,266 lib tests + 22 integration, 0 failures

---

## Determination

**v0.1.0-ALPHA RELEASED.** First public alpha for evaluation and external review.
All release blockers resolved. TOCTOU fully closed on Unix, substantially
hardened on Windows. Real-provider validated. Production-path approval E2E
verified. CLI surface truthful. Final audit passed: 0 overclaims, 0 blockers.
Accepted residuals documented in RELEASE_NOTES.md.
This is final release preparation — not yet a final release declaration.

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
| Remote master | `b9a2138` (`wave-74b-lock`, v0.1.0-alpha) — verified |
| Local/remote sync | ✅ 0 ahead, 0 behind |
| Total tags | 69 (40 RC-era + v0.1.0-alpha) |
| Publication date | 2026-06-12 |
| Status | v0.1.0-alpha — first public alpha release |

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
| Unix intermediate-directory TOCTOU | 73B | ✅ Fully closed (openat + O_NOFOLLOW) |
| Windows intermediate-directory TOCTOU | 73C | ✅ Substantially hardened (per-component reparse point check) |

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
| 3 | Windows per-component TOCTOU micro-race (DEFERRED-008) | Reduced residual (73C) | Undocumented `NtCreateFile` with `RootDirectory` handle |
| 4 | Remote/hosted provider validation | Not tested | Configure and test against OpenAI/Anthropic API |
| 5 | Desktop UI functional correctness | Process lifecycle only | Manual or automated UI testing |

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
| openwand-tools | 111 | — |
| openwand-app | 957 | 8 |
| openwand-workflow | 728 | — |
| openwand-trace | 41 | — |
| openwand-store | 3 | — |
| openwand-memory | 57 | — |
| openwand-llm | 13 | — |
| openwand-policy | 12 | — |
| openwand-skills | 4 | — |
| openwand-goals | 19 | — |
| **Total** | **2,266** | **22** |

Session integration tests: 3 production-path + 2 real-file-effect + 2 post-effect + 4 real-provider (ignored without env vars) + 3 other.
App integration tests: 8 binary CLI surface tests.

---

## Tag Sequence

**39 tags** from `wave-52a-lock` through `wave-74a-lock`:

| Range | Count | Waves |
|-------|------:|-------|
| 52A–58A | 7 | Desktop workflow visibility |
| 59A–61A | 3 | Shell decomposition |
| 62A–68A | 7 | Capability-context integration |
| 69A–69G | 7 | Release-blocker remediation + hardening |
| 70A–70D | 4 | RC validation + packaging |
| 71A–71C | 3 | CLI surface truth + E2E honesty + reconciliation |
| 72A–72D | 4 | Publication + TOCTOU + real-provider + ledger |
| 73A–73C | 3 | TOCTOU design + Unix hardening + Windows hardening |
| 74A | 1 | Final release preparation |
| **Total** | **39** | |

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

v0.1.0-alpha is released. Remaining work is post-alpha iteration:

| Option | Description |
|--------|-------------|
| A. External feedback | Collect and triage external review feedback |
| B. v0.1.1-alpha | Windows NT API TOCTOU closure, clippy cleanup, dep refresh |
| C. v0.2.0-alpha | New feature work (placeholder UI surfaces, hosted providers) |
| D. v0.1.0 stable | Promote alpha to stable after feedback period |

---

*v0.1.0-alpha released 2026-06-12. First public alpha for evaluation and external
review. Accepted residuals documented. Not production-ready. Not fully secure.
See RELEASE_NOTES.md for full details.*
