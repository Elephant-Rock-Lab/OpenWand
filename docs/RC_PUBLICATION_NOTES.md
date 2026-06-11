# Release Candidate Publication Notes

**Release:** OpenWand RC-1
**Date:** 2026-06-11
**Prepared by:** Craft Agent (automated)
**Status:** Local RC artifact prepared; remote publication pending by user decision.

---

## Artifact

| Field | Value |
|-------|-------|
| Binary | `target/release/openwand.exe` |
| Size | 17,260,032 bytes (16.4 MB) |
| SHA-256 | `826C5F87CCCD40DC35D58E472E9D8FD3A943F8F0B632508A73B06917061A6159` |
| Artifact code commit | `d6fa1f0` (`wave-70b-lock`) |
| Packaging metadata commit | `e50356d` (`wave-70c-lock`) |
| Target | `x86_64-pc-windows-msvc` |
| Profile | `release` (optimized) |
| Features | `desktop` |
| Rust | `rustc 1.95.0 (59807616e 2026-04-14)` |

---

## What Changed (Wave 52A → 70B)

26 waves of development across 5 phases:

| Phase | Waves | Focus |
|-------|-------|-------|
| Desktop workflow visibility | 52A–58A | Operator console, evidence inspector, audit packets, manual results, workflow routing, execution timeline |
| Shell decomposition | 59A–61A | 3-tab shell refactor, session shell, desktop bootstrap boundary |
| Capability-context integration | 62A–68A | Skills/goals context, prompt preview, audit trace, deterministic eval harness, real-model boundary eval, reporting |
| Release-blocker remediation | 69A–69G | Filesystem sandbox, approval binding, build baseline, truthful commands, trace attribution, release hardening, truth ledger |
| RC validation | 70A–70B | RC validation pass, real filesystem approval E2E, workspace build repair |

---

## Validation Evidence

### Build and Lint
- `cargo check --workspace --all-targets --all-features`: ✅ Clean
- `cargo clippy` (11 non-app crates, strict): ✅ Clean
- Release binary: ✅ 16.4 MB (under 20 MB boundary)

### Tests
- **1,148 tests**, 0 failures
- Core: 45 · Session: 49 + 4 integration · Tools: 93 · App: 957

### Security
- `cargo audit`: 0 vulnerabilities
- 16 transitive warnings (all via Dioxus desktop / Loro CRDT — none affect OpenWand paths)

### Real Filesystem Approval E2E (Wave 70B)
- Approved write creates file with expected contents ✅
- Rejected write creates no file ✅
- Trace ordering verified (resumed → called → completed) ✅
- No `tool.failed` on successful write ✅

### Desktop Smoke (Wave 70A)
- Process starts, alive 3s, no stderr, clean exit, 38 MB debug

### CLI (Wave 70A)
- `openwand.exe --help`: works
- `explain` / `trace-verify` / `session-rebuild`: exit 1 with "not yet implemented"

---

## Deferred Items

The following are **not** verified by this RC and are explicitly carried forward:

1. **Real-provider validation** — no LLM provider was contacted during testing
2. **App test-module clippy cleanup** — cosmetic warnings in test code, accepted
3. **Transitive dependency warnings** — pending upstream Dioxus/Loro upgrades
4. **Remote publication** — pending user decision

---

## Reproduce

```powershell
git checkout wave-70b-lock
cargo build -p openwand-app --release --features desktop
Get-FileHash target/release/openwand.exe -Algorithm SHA256
cargo test -p openwand-core --lib
cargo test -p openwand-session --lib --features testing
cargo test -p openwand-session --features testing --test approval_real_file_effect
cargo test -p openwand-session --features testing --test approval_post_effect
cargo test -p openwand-tools --lib
cargo test -p openwand-app --lib
cargo clippy -p openwand-core -p openwand-session -p openwand-tools `
  -p openwand-trace -p openwand-store -p openwand-memory `
  -p openwand-llm -p openwand-policy -p openwand-skills `
  -p openwand-goals -p openwand-workflow --all-features -- -D warnings
cargo audit
```

---

## Scope Boundaries

This RC:
- \u{2705} Proves approval control flow with real I/O through a test executor (not production path)
- \u{2705} Proves CLI command surface matches capability matrix (binary-level tests)
- \u{2705} Proves approval outcomes are reported honestly
- \u{2705} Proves build, lint, and test baseline
- \u{2705} Records artifact identity and reproduction steps
- \u{274c} Does **not** prove behavior through production tool executor with sandbox
- \u{274c} Does **not** prove behavior under real LLM provider calls
- \u{274c} Does **not** prove behavior with real network access
- \u{274c} Does **not** include external security audit
- \u{274c} Is **not** a final release declaration
