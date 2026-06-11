# RC Validation Report — Wave 70A

**Date:** 2026-06-11
**Baseline commit:** `7092c09` (wave-69g-lock)
**Validator:** Craft Agent (automated)

---

## Determination

**PASS WITH DEFERRED ITEMS:** Emergency blockers remain resolved, app-canonical build/test
path passes, real-provider validation deferred, full workspace all-target status explicitly
recorded with regression explanation.

---

## 1. Canonical Build Verification

### App-canonical build (CLEAN)

| Command | Result |
|---------|--------|
| `cargo check -p openwand-app --all-targets --all-features` | ✅ Clean |
| `cargo build -p openwand-app --all-targets --all-features` | ✅ Clean |
| `cargo build -p openwand-app --features desktop` | ✅ Clean |
| `cargo build -p openwand-app --release` | ✅ Clean (17 MB, under HB-G1 20MB) |

### Full workspace all-targets (REGRESSION — see Patch 1 finding)

| Command | Result |
|---------|--------|
| `cargo check --workspace --all-targets --all-features` | ❌ 75 + 4 errors |

**Patch 1 finding: C — A regression was introduced in Wave 69F.** The Wave 69C
`cargo check --workspace --all-targets --all-features` baseline was verified clean at
the 69C tag. Wave 69F's `cargo clippy --fix` removed test-only imports from
`openwand-workflow` and `openwand-memory` crate files. These imports are unused in
production code but required by `#[cfg(test)]` modules when the crate is compiled in
isolation (`-p openwand-workflow --all-targets`). The app-canonical path compiles
cleanly because `openwand-app`'s dependency graph provides the needed types.

**RC build baseline:** App-canonical clean; full workspace all-targets is not clean and
requires a dedicated import-migration wave to restore.

---

## 2. Canonical Test Verification

| Suite | Command | Tests | Result |
|-------|---------|------:|--------|
| Core | `cargo test -p openwand-core --lib` | 45 | ✅ Pass |
| Session | `cargo test -p openwand-session --lib --features testing` | 51 | ✅ Pass |
| Tools | `cargo test -p openwand-tools --lib` | 93 | ✅ Pass |
| App lib | `cargo test -p openwand-app --lib` | 957 | ✅ Pass |
| App integration | `cargo test -p openwand-app --tests` | 2,226 | ✅ Pass |
| **Total** | | **1,146 lib / 2,226 integration** | **0 failures** |

---

## 3. CLI E2E Validation

**Binary:** `target/release/openwand.exe` (17 MB)

| Command | Expected | Actual | Result |
|---------|----------|--------|--------|
| `openwand.exe --help` | Shows subcommand list | Shows subcommand list | ✅ |
| `openwand.exe explain test` | Exit 1, "not yet implemented" | Exit 1, "not yet implemented" | ✅ |
| `openwand.exe trace-verify test` | Exit 1, "not yet implemented" | Exit 1, "not yet implemented" | ✅ |
| `openwand.exe session-rebuild test` | Exit 1, "not yet implemented" | Exit 1, "not yet implemented" | ✅ |

---

## 4. Approval E2E Validation (Patch 3)

**New test file:** `crates/session/tests/approval_post_effect.rs` (+2 tests)

| Test | What it proves | Result |
|------|---------------|--------|
| `approval_post_effect_tool_executes_with_correct_trace_order` | Approve write → tool executes → trace: gate.evaluated → tool.suspended → tool.resumed → tool.called → tool.completed (not tool.failed) | ✅ |
| `rejection_does_not_execute_tool` | Reject → no tool.called, no tool.completed, tool.denied present | ✅ |

**Trace ordering verified:**
- `tool.resumed` appears BEFORE `tool.called`
- `tool.completed` present, `tool.failed` absent on approval
- Tool was actually called (1 execution, not placeholder)

**Scope limitation (honest disclosure):** These tests use `MockToolExecutor` which
records tool calls but does not perform real filesystem I/O. The following assertions
are NOT verified by these tests:
- File exists on disk after approved write
- File contents match expected payload

Real-file approval E2E requires wiring `BuiltinToolProvider` into the session test
harness, which is a non-trivial architectural change. This is recorded as a deferred
validation item, not a pass.

---

## 5. Desktop Launch Smoke Test (Patch 5)

| Criterion | Result |
|-----------|--------|
| Binary starts without immediate panic | ✅ |
| Process remains alive for 3 seconds | ✅ |
| No stderr panic/backtrace | ✅ (no stderr output) |
| Exits cleanly when terminated | ✅ (taskkill success) |
| Binary size recorded | Debug: 38 MB, Release: N/A (not built) |

**Note:** Desktop UI functional correctness is NOT claimed from this smoke test.
Only process lifecycle verified.

---

## 6. Real-Provider Validation (Patch 2 — DEFERRED)

**Status:** Deferred pending auth setup and non-sensitive fixture workspace.
Wave 70A does not claim real-provider validation was performed.

---

## 7. Clippy Posture

| Scope | Command | Result |
|-------|---------|--------|
| 11 non-app crates | `cargo clippy -p {11 crates} --all-features -- -D warnings` | ✅ Clean |
| App crate | `cargo clippy -p openwand-app --all-features -- -D warnings` | 57 test-module style warnings (accepted cosmetic) |

---

## 8. Cargo Audit

| Metric | Value |
|--------|-------|
| Vulnerabilities | 0 |
| Warnings | 16 (14 unmaintained + 2 unsound) |
| Direct dependency advisories | 0 |
| Changed since 69G | No — identical |

---

## 9. Documentation Consistency

| Check | Result |
|-------|--------|
| No stale "immutable" claims in README.md | ✅ (corrected to "append-only" in 69G) |
| No stale "release blocker open" claims | ✅ (all closed in KNOWN_GAPS.md) |
| RELEASE_CANDIDATE_LEDGER.md matches test counts | ✅ (1,146 lib tests) |
| KNOWN_GAPS.md halt-era closures present | ✅ (H1–H6 with wave references) |
| DEFERRED_RISKS.md statuses match current state | ✅ |
| STATE.md test baseline current | ✅ |
| HB-G4 correctly qualified (test-only unsafe) | ✅ |
| HB-G5 correctly qualified (app cosmetic warnings accepted) | ✅ |

---

## 10. Binary Sizes

| Binary | Debug | Release |
|--------|-------|---------|
| `openwand.exe` (CLI) | 31 MB | 17 MB ✅ (under HB-G1 20 MB) |
| `openwand-ui.exe` (Desktop) | 38 MB | N/A |

---

## Summary of Findings

| Finding | Category | Status |
|---------|----------|--------|
| App-canonical build/test clean | Core | ✅ Pass |
| 11 non-app crates clippy clean | Core | ✅ Pass |
| CLI truthful commands (exit 1) | Core | ✅ Pass |
| Approval post-effect trace ordering | Core | ✅ Pass |
| Desktop smoke lifecycle | Core | ✅ Pass |
| Cargo audit (0 vulns, 16 transitive warnings) | Core | ✅ Pass |
| Release CLI binary under 20 MB | Core | ✅ Pass |
| Documentation consistency | Core | ✅ Pass |
| Real-provider validation | Deferred | Explicitly not performed |
| Real-file approval E2E | Deferred | Mock executor only; real I/O requires harness wiring |
| Workspace --all-targets --all-features | Regression | Broken by 69F clippy --fix; needs import-migration wave |

---

## Test Delta

+2 session tests (approval_post_effect.rs): 1,146 total (core:45, session:51, tools:93, app:957)

---

*RC validation must report what actually passed, what failed, what was skipped, and what
remains outside the candidate scope. This report follows that principle.*
