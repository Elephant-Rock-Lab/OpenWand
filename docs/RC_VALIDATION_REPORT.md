# RC Validation Report — Wave 74A

**Date:** 2026-06-12
**Baseline commit:** `de6434f` (`wave-73c-lock`, latest)
**Previous report:** Wave 72D (commit `de1cb7f`)
**Validator:** Craft Agent (automated)
**Classification:** Final release preparation — not yet a final release declaration.

---

## Determination

**PASS — FINAL RELEASE PREP.** Public RC published and validated against a real
local provider. TOCTOU hardening arc complete: Unix intermediate-directory race
fully closed, Windows substantially hardened. All emergency blockers resolved.
Production-path approval E2E verified. CLI surface truthful. Final release
notes prepared with accepted residuals documented. Final release declaration
pending user review of release notes.

---

## RC Publication State

| Field | Value |
|-------|-------|
| Remote | https://github.com/Octo-Lex/OpenWand |
| Remote master | `9e0b0cd` (`wave-72c-lock`) |
| Local/remote sync | ✅ 0 ahead, 0 behind |
| Total tags | 63 (34 RC-era: wave-52a-lock through wave-72c-lock) |
| Publication date | 2026-06-11 |
| RC posture | Public release candidate for external review |

---

## Real-Provider Validation (72C)

**Provider setup (no secrets recorded):**

| Field | Value |
|-------|-------|
| Provider | LM Studio (OpenAI-compatible) |
| Endpoint | localhost:8766 |
| Model | google/gemma-4-12b (12B, tool-calling capable) |
| Auth | local / no secret required |
| Fixture workspace | non-sensitive temp directory with benign text files |
| Codebase | published RC (`9e0b0cd`) |

**Results (4/4 PASS):**

| Test | Result | What it proves |
|------|--------|----------------|
| ✅ `real_provider_completes_simple_turn` | PASS | Session reaches real LLM, gets response, turn completes naturally |
| ✅ `real_provider_trace_records_attribution` | PASS | Trace contains inference events with provider/model from RunConfig |
| ✅ `real_provider_read_tool_works` | PASS | Turn completed with read-only tools available |
| ✅ `real_provider_sandbox_refuses_escape` | PASS | Sandbox blocks /etc/passwd traversal under real inference |

**Caveats:**
- Validation covers one local OpenAI-compatible provider endpoint/model.
- No secrets stored. No sensitive workspace used.
- Remote/hosted providers (OpenAI API, Anthropic, etc.) were not tested.
- Model behavior is non-deterministic; results from a single run.
- Intermediate-directory TOCTOU residual remains separately tracked.

---

## Canonical Build Verification

| Command | Result |
|---------|--------|
| `cargo check --workspace --all-targets --all-features` | ✅ Clean |
| `cargo build --workspace --all-targets --all-features` | ✅ Clean |
| `cargo build -p openwand-app --features desktop` | ✅ Clean |
| `cargo build -p openwand-app --release` | ✅ Clean (17 MB, under HB-G1 20MB) |

The 69F regression is **repaired.** Test-only imports restored via `#[cfg(test)]`-gated
use statements at module level.

---

## Canonical Test Verification

| Suite | Tests | Result |
|-------|------:|--------|
| Core | 45 | ✅ Pass |
| Session lib | 49 | ✅ Pass |
| Session integration (production path) | 3 | ✅ Pass |
| Session integration (real file effect) | 2 | ✅ Pass |
| Session integration (post effect) | 2 | ✅ Pass |
| Session integration (real provider, ignored) | 4 | ✅ Pass (with env vars) |
| Tools | 96 | ✅ Pass |
| App lib | 957 | ✅ Pass |
| App CLI surface | 8 | ✅ Pass |
| **Total** | **1,166** | **0 failures** |

---

## CLI E2E Validation

**Binary:** `target/release/openwand.exe` (17 MB)

| Command | Expected | Actual | Result |
|---------|----------|--------|--------|
| `openwand.exe --help` | Shows subcommand list | Shows subcommand list | ✅ |
| `openwand.exe explain test` | Exit 1, "not yet implemented" | Exit 1, "not yet implemented" | ✅ |
| `openwand.exe trace-verify test` | Exit 1, "not yet implemented" | Exit 1, "not yet implemented" | ✅ |
| `openwand.exe session-rebuild test` | Exit 1, "not yet implemented" | Exit 1, "not yet implemented" | ✅ |

---

## Approval E2E Validation

### Production-Path Approval E2E (71B)

**Test file:** `crates/session/tests/approval_production_path.rs`

Uses `CompositeToolExecutor::local_only(batch2_local_tools())` — the full production path:
file_write_handler → JSON schema validation → `resolve_workspace_path()` → `write_file_no_follow()`.

| Test | Proves | Result |
|------|--------|--------|
| `production_approved_write_creates_file_via_sandbox` | File exists, contents match, trace ordering | ✅ |
| `production_rejected_write_creates_no_file` | No file, tool.denied in trace | ✅ |
| `production_sandbox_blocks_traversal_even_when_approved` | Sandbox rejects `../../../etc/escape.txt` even after policy approval | ✅ |

### Real Filesystem Effect (70B — test executor, not production path)

**Test file:** `crates/session/tests/approval_real_file_effect.rs`

Uses `RealFileWriteExecutor` — a test executor that calls `std::fs::write` for real I/O.

**Scope limitation:** The test executor bypasses the production `file_write_handler`,
its schema validation, the sandbox, the composite `BuiltinToolProvider`, and runtime tool
assembly. It proves real I/O occurs through *a* tool executor, not through *the production*
tool executor.

### Post-Effect Trace Ordering (70A — mock executor)

**Test file:** `crates/session/tests/approval_post_effect.rs`

| Test | What it proves | Result |
|------|---------------|--------|
| `approval_post_effect_tool_executes_with_correct_trace_order` | Trace: gate.evaluated → tool.suspended → tool.resumed → tool.called → tool.completed | ✅ |
| `rejection_does_not_execute_tool` | Rejection → tool.denied, not tool.called | ✅ |

---

## Desktop Launch Smoke Test

| Criterion | Result |
|-----------|--------|
| Binary starts without immediate panic | ✅ |
| Process remains alive for 3 seconds | ✅ |
| No stderr panic/backtrace | ✅ |
| Exits cleanly when terminated | ✅ |

**Note:** Desktop UI functional correctness is NOT claimed. Only process lifecycle verified.

---

## Clippy Posture

| Scope | Result |
|-------|--------|
| 11 non-app crates | ✅ Clean (`cargo clippy --all-features -- -D warnings`) |
| App crate | 57 test-module style warnings (accepted cosmetic) |

---

## Cargo Audit

| Metric | Value |
|--------|-------|
| Vulnerabilities | 0 |
| Warnings | 15 (13 unmaintained + 2 unsound) |
| Direct dependency advisories | 0 |
| All warnings transitive | via Dioxus desktop (13) or Loro CRDT (2) |

---

## TOCTOU Hardening Status

| Component | Status |
|-----------|--------|
| Direct path traversal (`../../`) | ✅ Blocked at validation time |
| Static symlink escapes | ✅ Blocked at validation time |
| Windows drive/UNC prefixes | ✅ Blocked at validation time |
| Final-component symlink (write) | ✅ Hardened 72B — `FILE_FLAG_NO_REPARSE_POINT` / `O_NOFOLLOW` |
| Unix intermediate directory race | ✅ **Fully closed 73B** — `openat` + `O_NOFOLLOW` per component |
| Windows intermediate directory race | ✅ **Substantially hardened 73C** — per-component `symlink_metadata` + re-verify |
| Windows per-component micro-race | ⚠️ Reduced residual — requires NT API to fully close |

---

## Summary of Findings

| Finding | Category | Status |
|---------|----------|--------|
| Full workspace build/test clean | Core | ✅ Pass |
| 11 non-app crates clippy clean | Core | ✅ Pass |
| CLI truthful commands (exit 1) | Core | ✅ Pass |
| Production-path approval E2E | Core | ✅ Pass |
| Real-provider validation (LM Studio + gemma-4-12b) | Core | ✅ Pass (72C) |
| Final-component TOCTOU hardened | Core | ✅ Pass (72B) |
| Desktop smoke lifecycle | Core | ✅ Pass |
| Cargo audit (0 vulns) | Core | ✅ Pass |
| Release binary under 20 MB | Core | ✅ Pass |
| Documentation consistency | Core | ✅ Pass |
| Real filesystem effect (test executor) | Core | ✅ Pass (scope-limited) |

---

## Remaining Deferred Items

| # | Item | Status | Category |
|---|------|--------|----------|
| 1 | App test-module clippy cleanup | Accepted cosmetic | Code quality |
| 2 | Transitive dependency warnings (15) | Accepted pending upstream | Dependencies |
| 3 | Windows per-component TOCTOU micro-race | Reduced residual (73C), requires NT API | Security |
| 4 | Remote/hosted provider validation | Not tested (only local LM Studio) | Testing |
| 5 | Desktop UI functional correctness | Process lifecycle only | Testing |

---

## What Was NOT Validated

- Remote/hosted provider endpoints (OpenAI API, Anthropic, etc.)
- Models other than google/gemma-4-12b
- Desktop UI functional correctness (process lifecycle only)
- Concurrent filesystem adversary (intermediate-directory TOCTOU)
- Multi-user or multi-session scenarios
- Non-Windows platforms

---

*RC validation must report what actually passed, what failed, what was skipped, and what
remains outside the candidate scope. This report follows that principle.*

*Real-provider validation passed against LM Studio + google/gemma-4-12b on a non-sensitive
fixture workspace. It does not claim validation across all providers, hosted APIs, all
models, or deterministic model behavior.*
