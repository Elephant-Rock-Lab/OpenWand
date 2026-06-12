# OpenWand v0.1.0-alpha — Release Notes

**Date:** 2026-06-12
**Release commit:** `b9a2138` (`wave-74b-lock`)
**Release tag:** `v0.1.0-alpha`
**Classification:** First public alpha release for evaluation and external review.

---

## Alpha Notice

This is an **alpha** release. It is not production-ready, not fully secure, and
not validated across all providers or platforms. It is published for evaluation
and external review. See Accepted Residuals below.

---

## Artifact Identity

| Field | Value |
|-------|-------|
| Binary | `target/release/openwand.exe` |
| Size | 17,840,640 bytes (17.0 MB) |
| SHA-256 | `9B5611A9440B5A6028984DE50EB015CE521A8BC4A05FBC496B45F90A4D915A93` |
| Toolchain | `rustc 1.95.0 (59807616e 2026-04-14)` |
| Target | `x86_64-pc-windows-msvc` |
| Profile | `release` (optimized) |
| Feature | `--features desktop` |

---

## Overview

OpenWand is a 10-phase agent loop with trace-first mutation, tool execution with
sandboxed filesystem access, capability-context governance, and a desktop workflow
visibility UI.

This document covers the full development arc from initial desktop UI surfaces
(Wave 52A) through the current alpha release (Wave 74B, 39 waves locked).

---

## Security Hardening (72B → 73C)

### Filesystem TOCTOU Hardening Arc

The most significant changes in this release cycle are the layered filesystem
write hardening that addresses the TOCTOU (time-of-check/time-of-use) race in
the `file_write` tool.

**Layer 1 — Centralized sandbox validation (69A):**
All local tools resolve paths through `resolve_workspace_path()`, which rejects
absolute paths, parent traversal (`..`), Windows drive/UNC prefixes, and static
symlink escapes. Containment is independent of policy auto-allow decisions.

**Layer 2 — Final-component no-follow (72B):**
`write_file_no_follow()` uses `FILE_FLAG_NO_REPARSE_POINT` (Windows) and
`O_NOFOLLOW` (Unix) to prevent following symlinks at the final path component
during write.

**Layer 3 — Unix handle-relative traversal (73B):**
`WorkspaceWriteHandle` opens the workspace root as a directory file descriptor,
then walks each path component using `openat()` + `O_NOFOLLOW`. Intermediate
directory symlinks are detected (`ELOOP`) and rejected. Directory creation uses
`mkdirat()` + immediate re-open with `O_NOFOLLOW`. On Linux, macOS, and FreeBSD,
the intermediate-directory TOCTOU race is **fully closed**.

**Layer 4 — Windows per-component reparse point detection (73C):**
On Windows, `WorkspaceWriteHandle` walks each intermediate component checking
`symlink_metadata()` for reparse point status. Directories are created one at a
time with `create_dir()` and immediately re-verified. The final component uses
`write_file_no_follow()` from 72B.

**Windows residual:** A per-component micro-race window remains between
`symlink_metadata()` and the subsequent I/O call. This is orders of magnitude
smaller than the original full-path race but is not fully eliminated. Windows
lacks `openat()` in the stable Win32 API.

---

## Validation

### Real-Provider Validation (72C)

The published RC was validated against a real local LLM:

| Field | Value |
|-------|-------|
| Provider | LM Studio (OpenAI-compatible) |
| Model | google/gemma-4-12b (12B, tool-calling) |
| Endpoint | localhost:8766 |
| Auth | local / no secret recorded |
| Fixture | non-sensitive temp directory |

**Results:** 4/4 tests passed:
- `real_provider_completes_simple_turn` — PASS
- `real_provider_trace_records_attribution` — PASS
- `real_provider_read_tool_works` — PASS
- `real_provider_sandbox_refuses_escape` — PASS

### Production-Path Approval E2E (71B)

3 tests exercise the full production write path:
`MockLlmClient → SessionRunner → MockPolicyEngine → approval → CompositeToolExecutor → BuiltinToolProvider → file_write_handler → resolve_workspace_path() → write_file_no_follow()`

### CLI Surface Truth (71A)

8 binary-level tests verify `--help` output and truthful exit codes for
unimplemented commands.

---

## Accepted Residuals

The following items are accepted for this release:

| Item | Category | Status |
|------|----------|--------|
| Windows per-component TOCTOU micro-race | Security | Reduced residual (73C), requires NT API to fully close |
| App test-module clippy warnings (57) | Code quality | All in `#[cfg(test)]`, cosmetic |
| Transitive dependency warnings (15) | Dependencies | 13 unmaintained + 2 unsound, all via Dioxus/Loro |
| Remote/hosted provider validation | Testing | Only LM Studio + gemma-4-12b validated |
| Desktop UI functional correctness | Testing | Only process lifecycle smoke-tested |
| Non-Windows platform testing | Testing | Binary tested on Windows only |
| 9 placeholder UI surfaces | Feature | 3-line stubs, not prioritized |

---

## Not Claimed

This release does **not** claim:

- Validation across all providers or hosted APIs
- Desktop UI functional correctness
- Full TOCTOU elimination on Windows
- Production deployment readiness
- Freedom from transitive dependency vulnerabilities
- Deterministic model behavior

---

## Test Baseline

| Suite | Tests |
|-------|------:|
| openwand-core | 45 |
| openwand-session | 49 + 14 integration |
| openwand-tools | 111 |
| openwand-app | 957 + 8 CLI surface |
| openwand-workflow | 728 |
| openwand-trace | 41 |
| openwand-store | 3 |
| openwand-memory | 57 |
| openwand-llm | 13 |
| openwand-policy | 12 |
| openwand-skills | 4 |
| openwand-goals | 19 |
| **Total** | **2,266 lib + 22 integration** |

---

## Wave History (52A → 73C)

| Arc | Waves | Description |
|-----|-------|-------------|
| Desktop workflow visibility | 52A–58A | Design system + 6 workflow surfaces + shell |
| Shell decomposition | 59A–61A | Desktop shell refactor + bootstrap guards |
| Capability-context integration | 62A–68A | Skills/goals + prompt preview + audit trace + eval |
| Release-blocker remediation | 69A–69E | Sandbox, approval binding, build, truthful commands, trace |
| Release hardening | 69F–69G | Full workspace regression + truth ledger |
| RC validation | 70A–70D | Canonical build, gap closure, packaging, publication prep |
| CLI surface truth | 71A | Missing commands wired, honest reporting |
| E2E honesty | 71B | Production-path approval E2E, test annotations |
| RC reconciliation | 71C | Determination, TOCTOU risk ledger |
| Publication | 72A | Public push to GitHub |
| TOCTOU hardening | 72B–73C | Final-component, Unix handle-relative, Windows per-component |
| Real-provider validation | 72C | LM Studio + gemma-4-12b, 4/4 PASS |
| Ledger refresh | 72D | RC docs reconciled, decision point |
| Final release prep | 74A | Release notes, audit, document reconciliation |
| External audit | 74B | Audit pass: 9 staleness findings fixed, 0 overclaims, 0 blockers |
| **Total** | **40 waves locked** | | | |

---

## Repository

- **Remote:** https://github.com/Octo-Lex/OpenWand
- **License:** See repository
- **Build:** `cargo build --workspace --all-targets --all-features`
- **Test:** `cargo test --workspace`
- **Desktop:** `cargo build -p openwand-app --features desktop`

---

## Post-Alpha Stabilization (76A–76D)

Four stabilization waves completed after alpha publication:

### 76A — Post-Alpha Issue Intake

5 GitHub issue templates (bug, security, provider validation, UX feedback,
validation checklist) + config.yml + `docs/POST_ALPHA_TRIAGE.md`.
Issues matching documented residuals are closed with reference unless new
 evidence expands scope.

### 76B — Windows TOCTOU Residual Hardening Feasibility

Investigated whether the remaining Windows intermediate-directory micro-race
can be fully closed. **Result: Yes, via `NtCreateFile` with `RootDirectory` +
`FILE_OPEN_REPARSE_POINT`.** Scheduled for v0.2.0. The 73C per-component
hardening remains in place.

See `docs/WINDOWS_TOCTOU_FEASIBILITY.md`.

### 76C — Multi-Provider Validation Matrix

Expanded real-provider validation from 1 to 2 local LM Studio models:

| Model | Size | Simple Turn | Trace Attr | Tool Use | Sandbox | Result |
|-------|------|:-----------:|:----------:|:--------:|:-------:|:------:|
| google/gemma-4-12b | 12B | ✅ | ✅ | ✅ | ✅ | PASS |
| bartowski/qwen2.5-0.5b-instruct | 0.5B | ✅ | ✅ | ✅ | ✅ | PASS |

No hosted providers tested. Anthropic uses non-OpenAI-compatible API format,
requires separate adapter.

See `docs/PROVIDER_VALIDATION_MATRIX.md`.

### 76D — Desktop Interaction E2E

6 new tests exercise the desktop interaction path at the service/runner level:

| Test | What it validates |
|------|------------------|
| UI DTO defaults | `UiRunState` starts in `Idle` with empty fields |
| New running state | `new_running()` sets `Running` + `RunStart` phase |
| Runner session ID | `SessionRunner` exposes `session_id` for rendering |
| Turn updates state | Full path: mock LLM → runner → bridge → `UiRunState` |
| Message structure | Messages have non-empty content for rendering |
| Binary lifecycle | Desktop binary stays alive for 3 seconds |

Dioxus rendering, click events, and visual behavior remain untested (no
headless framework for Dioxus 0.7).

See `crates/app/tests/desktop_interaction_e2e.rs`.

### Post-Alpha Test Baseline

| Suite | Tests |
|-------|------:|
| openwand-core | 45 |
| openwand-session | 49 + 14 integration |
| openwand-tools | 111 |
| openwand-app | 957 + 14 integration |
| openwand-workflow | 728 |
| openwand-trace | 41 |
| openwand-store | 3 |
| openwand-memory | 57 |
| openwand-llm | 13 |
| openwand-policy | 12 |
| openwand-skills | 4 |
| openwand-goals | 19 |
| **Total** | **2,272 lib + 28 integration** |

### Beta Readiness

See `docs/BETA_GAP_LEDGER.md` for the full beta gap analysis and roadmap.
Beta-blocking items: hosted provider validation, desktop UX validation.
