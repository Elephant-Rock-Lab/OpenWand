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

## Current Gaps (Wave 69G)

| # | Gap | Category | Status |
|---|-----|----------|--------|
| C1 | 9 placeholder UI surfaces (3-line stubs) | Feature | Accepted — not yet prioritized |
| C2 | `openwand-app` 57 test-module clippy warnings | Cosmetic | Accepted (DEFERRED-001) |
| C3 | `openwand-content` crate removed | Feature | Deferred — re-add when needed |
| C4 | Trace store append-only is structural, not enforced by runtime verifier | Architecture | Accepted (claim corrected in README) |
| C5 | 15 cargo audit warnings (all transitive, via Dioxus/Loro) | Dependency | Accepted (DEFERRED-002) |
| C6 | 23 commits ahead of origin/master | Publication | Pending user decision (DEFERRED-007) |
| C7 | No concurrent mutation tests for MutationHelper | Testing | Closed 69G — single-writer is architecturally enforced; 3 direct tests added |

---

## Resolution Summary

- Halt-era blockers: **All resolved (69A–69E).**
- Pre-50A gaps: **All resolved.**
- Current gaps: **1 closed (C7), 6 accepted or pending.**
