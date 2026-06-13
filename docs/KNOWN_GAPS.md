# OpenWand Known Gaps

Documented from disk-verified reconnaissance at Wave 40. Updated at Wave 50A.
Updated again at Wave 69G with halt-era blocker closures and current gaps.

---

## Halt-Era Blocker Closures (69A–69E)

| # | Blocker | Closure Wave | Resolution |
|---|---------|-------------|------------|
| H1 | Filesystem sandbox escape — tools could resolve paths outside workspace | 69A | Centralized `resolve_workspace_path()` in `tools/src/sandbox.rs`; all local tools resolve through it |
| H2 | Approval workspace swap — approval resumption not bound to original workspace | 69B | `canonical_workspace` field on `ApprovalContextSnapshot`; fail-closed on None; canonical comparison before `tool.resumed` |
| H3 | Desktop compile failure — 13 files with design token name drift, missing imports | 69C | Fixed `COLORS::`→`colors::`, `TYPO::`→`typo::`, rsx! format-string patterns, `openwand_app::`→`crate::`, stale fixtures |
| H4 | Canonical fixture drift — eval test fixtures missing capability_context fields | 69C | Added 4 CC scenarios to `make_passing_report_set()`; updated required scenario count |
| H5 | Placeholder verification commands — `cmd_explain`, `cmd_trace_verify`, `cmd_session_rebuild` faked work | 69D | Exit non-zero with "not yet implemented"; +6 CLI integration tests |
| H6 | Mock/unknown production trace attribution — runner used "mock"/"unknown" in production path | 69E | `TraceIdentity` struct derives real provider/model from `RunConfig.llm_target`; "unavailable" when not configured |

**All halt-era blockers resolved.**

---

## Pre-50A Coverage Gaps (Closed)

| # | Gap | Status |
|---|-----|--------|
| 1 | Missing guard test for Next-Action Review | ✅ Closed Wave 50A |
| 2 | Missing CLI test for Routing Readiness | ✅ Closed Wave 50A |
| 3 | Missing UI state for Next-Action Review | ✅ Closed Wave 50A |
| 4 | Missing UI components for Next-Action Review | ✅ Closed Wave 50A (placeholder per BYPASS-01) |
| 5 | `wnar_` ID prefix built in app crate | ✅ Closed Wave 50A |
| 6 | Shared lock doc for Wave 32 | Informational |
| 7 | Wave 39 lock doc count error | ✅ Fixed Wave 40 |
| 8 | WAVES.md missing Wave 39 | ✅ Fixed Wave 40 |
| 9 | ROADMAP.md missing Wave 39 | ✅ Fixed Wave 40 |

---

## Current Gaps (Updated Wave 86B — post-v0.3.0 stable)

| # | Gap | Category | Status |
|---|-----|----------|--------|
| C1 | ~~9 placeholder UI surfaces (3-line stubs)~~ | Feature | ✅ Closed 80A-80C — all 10 surfaces implemented |
| C2 | `openwand-app` 43 test-module clippy warnings | Cosmetic | Accepted (DEFERRED-001) — reduced from 57 in 81A |
| C3 | `openwand-content` crate is a stub (add() only) | Feature | Accepted — README corrected 83A |
| C4 | Trace store append-only is structural, not enforced by runtime verifier | Architecture | Accepted (claim corrected in README) |
| C5 | 15 cargo audit warnings (all transitive, via Dioxus/Loro) | Dependency | Accepted (DEFERRED-002) — 0 vulnerabilities, verified 82A |
| C6 | ~~23 commits ahead of origin/master~~ | Publication | ✅ Resolved — all published to remote |
| C7 | ~~No concurrent mutation tests~~ | Testing | ✅ Closed 69G |
| C8 | ~~69F workspace --all-targets regression~~ | Build | ✅ Closed 70B |
| C9 | ~~Workflow UI surfaces are static, not wired to live data~~ | Feature | ✅ Closed 84A-84C — 5 surfaces live-wired |
| C10 | ~~Non-Windows platform validation deferred~~ | Validation | ✅ Partially closed 85A — Linux compilation validated, GUI runtime deferred |
| C11 | ~~Unix sandbox not tested on Linux~~ | Validation | ✅ Closed 85A — 3,934 tests on Linux, 0 failures |
| C12 | macOS compilation/runtime not validated | Validation | Accepted — no macOS environment available |

---

## Resolution Summary

- Halt-era blockers: **All resolved (69A–69E).**
- Pre-50A gaps: **All resolved.**
- Current gaps: **8 closed, 5 accepted.**
  - Closed: C1 (80A-80C), C6 (published), C7 (69G), C8 (70B), C9 (84A-84C), C10 (85A partial), C11 (85A), C12 (accepted)
  - Accepted: C2 (cosmetic), C3 (stub crate), C4 (architecture), C5 (upstream deps), C12 (macOS deferred)
