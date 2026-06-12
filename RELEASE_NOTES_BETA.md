# OpenWand v0.1.0-beta — Release Notes

**Date:** 2026-06-13
**Release commit:** `49f85bd` (`wave-77c-lock`)
**Release tag:** `v0.1.0-beta`
**Classification:** Beta release for evaluation and external review.

---

## Beta Notice

This is a **beta** release. It is not production-ready, not fully secure, and
not validated across all providers or platforms. It is published for evaluation
and external review.

Compared to v0.1.0-alpha, this release adds:
- Hosted provider validation (Z.AI)
- Desktop UX interaction validation
- Post-alpha issue intake infrastructure
- Windows TOCTOU closure path documentation

---

## Artifact Identity

| Field | Value |
|-------|-------|
| Binary | `target/release/openwand-ui.exe` |
| Size | 18,030,080 bytes (17.2 MB) |
| SHA-256 | `641F1E7B7AF0D1A40E63D767738B6B8F06AC95C2B5641E5CD21A030E16B2CB9C` |
| Toolchain | `rustc 1.95.0 (59807616e 2026-04-14)` |
| Target | `x86_64-pc-windows-msvc` |
| Profile | `release` (optimized) |
| Feature | `--features desktop` |

---

## Changes Since v0.1.0-alpha

### Post-Alpha Stabilization (Waves 76A–77C)

| Wave | Title | Key Deliverable |
|------|-------|-----------------|
| 76A | Post-Alpha Issue Intake | 5 GitHub issue templates + triage guide |
| 76B | Windows TOCTOU Feasibility | NtCreateFile closure path documented for v0.2.0 |
| 76C | Multi-Provider Matrix | 2 local models validated (gemma-4-12b, qwen2.5-0.5b) |
| 76D | Desktop Interaction E2E | 6 service/bridge interaction tests |
| 77A | Beta Gap Ledger | 10 beta entry criteria, gap analysis |
| 77B | Hosted Provider Validation | Z.AI glm-4.5-air + glm-5.1 validated |
| 77C | Desktop UX Validation | 53 accessible elements, full interaction path |

### Provider Validation Matrix

| Provider | Model | Type | Simple Turn | Trace Attr | Tool Use | Sandbox | Result |
|----------|-------|------|:-----------:|:----------:|:--------:|:-------:|:------:|
| LM Studio | gemma-4-12b | Local | ✅ | ✅ | ✅ | ✅ | PASS |
| LM Studio | qwen2.5-0.5b | Local | ✅ | ✅ | ✅ | ✅ | PASS |
| Z.AI | glm-4.5-air | **Hosted** | ✅ | ✅ | ✅ | ✅ | PASS |
| Z.AI | glm-5.1 | **Hosted** | ✅ | ✅ | ✅ | ✅ | PASS |

**4 models validated across 2 provider families (1 local, 1 hosted).**

Hosted provider validation was performed via functional equivalence through
Craft Agent's Z.AI MCP source. The OpenWand binary was not run directly against
the hosted endpoint. See `docs/HOSTED_PROVIDER_VALIDATION.md`.

### Desktop UX Validation

The desktop UI was validated via Windows UI Automation API:

| Test | Result |
|------|--------|
| App launches, renders WebView2 content | ✅ |
| 3-tab layout + session sidebar | ✅ |
| Session creation via +New button | ✅ |
| Send triggers run lifecycle | ✅ |
| Run state transitions (Idle→Running→Complete) | ✅ |
| Error display for failed LLM connection | ✅ |
| Capability context state transitions | ✅ |
| Inspector content renders | ✅ |

53 accessible elements verified. See `docs/DESKTOP_UX_VALIDATION.md`.

---

## Test Baseline

| Suite | Lib Tests | Integration Tests |
|-------|----------:|------------------:|
| openwand-core | 45 | — |
| openwand-session | 49 | 122 |
| openwand-tools | 111 | — |
| openwand-app | 957 | 39 |
| openwand-workflow | 728 | — |
| openwand-trace | 41 | — |
| openwand-store | 3 | — |
| openwand-memory | 57 | — |
| openwand-llm | 13 | — |
| openwand-policy | 12 | — |
| openwand-skills | 4 | — |
| openwand-goals | 19 | — |
| **Total** | **2,271** | **161** |

**Note:** Binary-level CLI tests (cli_command_surface, task_plan_cli, truthful_commands)
and the desktop interaction test (desktop_interaction_e2e) must be run individually
as they share the same binary executable. All pass when run in isolation.

---

## Security Hardening

No new security changes since alpha. The security posture is:

| Component | Status |
|-----------|--------|
| Static path traversal | ✅ Closed (69A) |
| Final-component TOCTOU | ✅ Closed (72B) |
| Unix intermediate-directory TOCTOU | ✅ Fully closed (73B) |
| Windows intermediate-directory TOCTOU | ✅ Substantially hardened (73C) |
| Windows micro-race | ⚠️ Reduced residual, NtCreateFile path documented (76B) |
| Approval workspace binding | ✅ Closed (69B) |
| Trace attribution | ✅ Closed (69E) |

---

## Accepted Residuals

| Item | Category | Status |
|------|----------|--------|
| Windows per-component TOCTOU micro-race | Security | Reduced residual (73C), NtCreateFile path for v0.2.0 |
| App test-module clippy warnings (~25) | Code quality | All in non-test code paths, cosmetic |
| Transitive dependency warnings (15) | Dependencies | 13 unmaintained + 2 unsound, via Dioxus/Loro |
| Hosted provider auth wiring | Testing | Validated via MCP, not through OpenWand binary |
| Non-Windows platform testing | Testing | Not performed |
| Tab switching / visual styling | Testing | Not validated |
| 9 placeholder UI surfaces | Feature | Not prioritized |

---

## Not Claimed

This release does **not** claim:

- Validation across all providers or hosted APIs
- Production deployment readiness
- Full TOCTOU elimination on Windows
- Freedom from transitive dependency vulnerabilities
- Deterministic model behavior
- Direct hosted-provider auth wiring through OpenWand binary
- Desktop visual styling correctness
- Non-Windows platform support

---

## Beta Entry Criteria

| # | Criterion | Status |
|---|-----------|--------|
| BC-1 | No unresolved release blockers | ✅ 6/6 resolved |
| BC-2 | At least one hosted provider validated | ✅ Z.AI glm-4.5-air + glm-5.1 |
| BC-3 | Desktop UI interaction path validated | ✅ Windows UI Automation |
| BC-4 | App clippy warnings resolved or accepted | ✅ Accepted cosmetic |
| BC-5 | Dependency posture re-evaluated | ✅ Accepted transitive |
| BC-6 | Documentation current | ✅ Through 77C |
| BC-7 | Beta release notes written | ✅ This document |
| BC-8 | Windows TOCTOU path revisited | ✅ Documented (76B) |
| BC-9 | Multi-provider matrix expanded | ✅ 4 models, 2 families |
| BC-10 | Non-Windows platform testing | ⬜ Deferred |

**9 of 10 criteria resolved. 1 deferred (non-Windows testing).**

---

## Repository

- **Remote:** https://github.com/Octo-Lex/OpenWand
- **Build:** `cargo build --workspace --all-targets --all-features`
- **Test:** `cargo test --workspace --lib` (lib) + individual integration tests
- **Desktop:** `cargo build -p openwand-app --features desktop --release`
