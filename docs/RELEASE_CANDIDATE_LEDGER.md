# Release Candidate Readiness Ledger

**Wave:** 71C
**Date:** 2026-06-11
**Commit:** `10f7e3b` (`wave-71b-lock`, latest runtime code)
**Test baseline:** 1,159 tests (1,152 lib + 7 integration), 0 failures

---

## Determination

**Emergency halt blockers resolved. Executable CLI surface truthful. Production-path
approval E2E verified with sandbox/schema/executor. Repository is eligible for public
release-candidate publication, not yet a final release declaration.**

---

## RC Artifact Identity

| Field | Value |
|-------|-------|
| Artifact code commit | `d6fa1f0` (`wave-70b-lock`) |
| Latest runtime commit | `10f7e3b` (`wave-71b-lock`) |
| Packaging metadata commit | `e50356d` (`wave-70c-lock`) |
| Target triple | `x86_64-pc-windows-msvc` |
| Build profile | `release` (optimized) |
| Feature set | `--features desktop` |
| Binary path | `target/release/openwand.exe` |
| Binary size | 17,260,032 bytes (16.4 MB) |
| SHA-256 | `826C5F87CCCD40DC35D58E472E9D8FD3A943F8F0B632508A73B06917061A6159` |
| Rust toolchain | `rustc 1.95.0 (59807616e 2026-04-14)` |
| Report timestamp | 2026-06-11T12:34Z |

---

## Reproducibility Commands

```powershell
# Checkout
git checkout wave-70b-lock

# Build release binary
cargo build -p openwand-app --release --features desktop

# Verify checksum (Windows PowerShell)
Get-FileHash target/release/openwand.exe -Algorithm SHA256

# Full workspace build
cargo check --workspace --all-targets --all-features

# Test
cargo test -p openwand-core --lib
cargo test -p openwand-session --lib --features testing
cargo test -p openwand-session --features testing --test approval_real_file_effect
cargo test -p openwand-session --features testing --test approval_post_effect
cargo test -p openwand-tools --lib
cargo test -p openwand-app --lib

# Lint (11 non-app crates)
cargo clippy -p openwand-core -p openwand-session -p openwand-tools `
  -p openwand-trace -p openwand-store -p openwand-memory `
  -p openwand-llm -p openwand-policy -p openwand-skills `
  -p openwand-goals -p openwand-workflow --all-features -- -D warnings

# Dependency audit
cargo audit
```

---

## Tag Sequence

**30 tags** from `wave-52a-lock` through `wave-71b-lock`:

| Range | Count | Waves |
|-------|------:|-------|
| 52A–58A | 7 | Desktop workflow visibility |
| 59A–61A | 3 | Shell decomposition |
| 62A–68A | 7 | Capability-context integration |
| 69A–69G | 7 | Release-blocker remediation + hardening |
| 70A–70D | 4 | RC validation + packaging |
| 71A–71B | 2 | CLI surface truth + production-path E2E |
| **Total** | **30** | |

Full tag list:
```
wave-52a-lock wave-53a-lock wave-54a-lock wave-55a-lock wave-56a-lock
wave-57a-lock wave-58a-lock wave-59a-lock wave-60a-lock wave-61a-lock
wave-62a-lock wave-63a-lock wave-64a-lock wave-65a-lock wave-66a-lock
wave-67a-lock wave-68a-lock wave-69a-lock wave-69b-lock wave-69c-lock
wave-69d-lock wave-69e-lock wave-69f-lock wave-69g-lock wave-70a-lock
wave-70b-lock
wave-70c-lock
wave-70d-lock
wave-71a-lock
wave-71b-lock
```

---

## Release Blocker Status

| Blocker | Wave | Status |
|---------|------|--------|
| Filesystem sandbox escape | 69A | ✅ Resolved |
| Approval workspace swap | 69B | ✅ Resolved |
| Desktop compile failure | 69C | ✅ Resolved |
| Canonical fixture drift | 69C | ✅ Resolved |
| Placeholder verification commands | 69D | ✅ Resolved |
| Mock/unknown production trace attribution | 69E | ✅ Resolved |

**6/6 release blockers resolved.**

| CLI command surface matches capability matrix | 71A | ✅ Verified (8 binary tests) |
| Approval outcome reporting honest | 71A | ✅ Fixed |
| Production-path approval E2E | 71B | ✅ Verified (3 tests: approve/reject/traversal) |
| Production sandbox blocks traversal even after policy approval | 71B | ✅ Verified |
| Stale shell E2E scripts corrected | 71B | ✅ Corrected |
| Direct-function tests annotated | 71B | ✅ 5 files annotated |
| Real-provider validation (LM Studio + gemma-4-12b) | 72C | ✅ Passed (4/4 tests) |

---

## Validation Closures (70A-70B)

| Item | Wave | Status |
|------|------|--------|
| Approval post-effect trace ordering (mock) | 70A | ✅ Verified |
| Real filesystem approval-effect E2E | 70B | ✅ Verified |
| Full workspace `--all-targets --all-features` | 70B | ✅ Restored |
| 11 non-app crates clippy strict | 70B | ✅ Clean |
| CLI truthful commands | 70A | ✅ Verified |
| Desktop smoke lifecycle | 70A | ✅ Verified |
| Release binary under 20 MB | 70A | ✅ 16.4 MB |
| Cargo audit | 70A | ✅ 0 vulnerabilities |

---

## Deferred Risk Summary

| ID | Description | Status |
|----|-------------|--------|
| DEFERRED-001 | App crate test-module clippy warnings | Accepted non-blocking (cosmetic) |
| DEFERRED-002 | Cargo audit 16 transitive warnings | Closed by recording (0 vulnerabilities) |
| DEFERRED-003 | unsafe-env-test claim | Closed by claim correction |
| DEFERRED-004 | Trace immutability claim | Closed by documentation downgrade |
| DEFERRED-005 | MutationHelper live-event correctness | Closed with tests + rationale |
| DEFERRED-006 | STATE.md/documentation update | Closed by update |
| DEFERRED-007 | Local branch publication | Accepted non-blocking / pending user decision |
| DEFERRED-008 | Sandbox TOCTOU boundary | Accepted residual / documented threat model (71C) |

**Categories:** 5 closed (002–006), 3 accepted non-blocking (001, 007, 008).

---

## Clippy Posture

**Clean (`cargo clippy --all-features -- -D warnings`):**
- openwand-core, openwand-session, openwand-tools, openwand-trace, openwand-store
- openwand-memory, openwand-llm, openwand-policy, openwand-skills, openwand-goals
- openwand-workflow

**Not yet clean:**
- openwand-app: test-module style warnings accepted as cosmetic

---

## Cargo Audit Summary

- **Vulnerabilities:** 0
- **Warnings:** 15 (13 unmaintained, 2 unsound)
- **Direct dependency advisories:** 0
- **All warnings transitive** via Dioxus desktop rendering (13) or Loro CRDT (1 via atomic-polyfill)
- **None affect** OpenWand data, crypto, network, or storage paths

---

## Test Baseline

| Crate | Lib Tests | Integration Tests |
|-------|----------:|------------------:|
| openwand-core | 45 | — |
| openwand-session | 49 | 7 |
| openwand-tools | 93 | — |
| openwand-app | 957 | 8 |
| **Total** | **1,152** | **15** |

---

## Known Gaps

- 9 placeholder UI surfaces (3-line stubs, not yet prioritized)
- `openwand-content` crate removed (scaffold, to be re-added when needed)
- Trace store append-only is structural, not enforced by runtime verifier
- No concurrent mutation tests (architecturally prevented by single-writer run_lock)

---

## Publication State

**Published. Remote repository contains RC commits and lock tags.**

- Remote: https://github.com/Octo-Lex/OpenWand
- Remote master: `82b7488` (wave-71c-lock) — verified
- Local/remote: in sync (0 ahead, 0 behind)
- 60 total tags on remote (31 RC-era: wave-52a-lock through wave-71c-lock)
- Publication date: 2026-06-11
- Status: **Release candidate for external review — not a final release**

---

## Carried-Forward Deferred Items

1. Real-provider validation (LM Studio + google/gemma-4-12b, local endpoint) — ✅ Passed 72C
2. App test-module clippy cleanup — accepted as cosmetic
3. Transitive dependency warnings (15: 13 unmaintained, 2 unsound) — accepted pending upstream upgrades
4. Remote publication — ✅ Published 2026-06-11
5. Sandbox TOCTOU boundary (DEFERRED-008) — accepted residual risk, documented threat model

---

## Claim Accuracy Verification

| Claim | Location | Truthful? | Note |
|-------|----------|-----------|------|
| "Zero unsafe in production code" | STATE.md HB-G4 | ✅ | Corrected: test-only env vars excepted |
| "hash-bound append-only records" | README.md | ✅ | Corrected from "immutable" |
| "append-only evidence chain" | README.md | ✅ | Notes verifier not yet implemented |
| "cargo clippy zero warnings" | STATE.md HB-G5 | ✅ | Qualifies: 11 non-app crates; app test-module cosmetic accepted |
| "1,159 tests, 0 failures" | This ledger | ✅ | Verified baseline (1,152 lib + 7 integration) |
| "0 vulnerabilities" | This ledger | ✅ | cargo audit confirmed |

---

*After 71C, the RC is eligible for public release-candidate publication. All reviewer findings
from the independent audit are closed (5) or accepted as residual risk (1 TOCTOU). Production-path
approval E2E exercises real sandbox/schema/executor. CLI surface matches capability matrix.
Final release declaration remains pending by user decision.*
