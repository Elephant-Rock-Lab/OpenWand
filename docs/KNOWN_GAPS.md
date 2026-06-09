# OpenWand Known Gaps

Documented from disk-verified reconnaissance at Wave 40. Updated at Wave 50A.

---

## Coverage Gaps

| # | Gap | Status |
|---|-----|--------|
| 1 | Missing guard test for Next-Action Review | ✅ Closed Wave 50A — `crates/app/tests/workflow_next_action_review_guards.rs` created |
| 2 | Missing CLI test for Routing Readiness | ✅ Closed Wave 50A — `crates/app/tests/workflow_routing_readiness_cli.rs` created |
| 3 | Missing UI state for Next-Action Review | ✅ Closed Wave 50A — `crates/app/src/ui/workflow_next_action_review_state.rs` created |
| 4 | Missing UI components for Next-Action Review | ✅ Closed Wave 50A — `crates/app/src/ui/workflow_next_action_review_components.rs` created (placeholder per BYPASS-01) |

## Design Observations

| # | Observation | Status |
|---|-------------|--------|
| 5 | `wnar_` ID prefix built in app crate | ✅ Closed Wave 50A — `next_action_review_id_for()` added to workflow crate; `main.rs` now uses it |
| 6 | Shared lock doc for Wave 32 | Informational — two capabilities in one wave explains the original gaps 1–4 |

## Documentation Drift

| # | Gap | Status |
|---|-----|--------|
| 7 | Wave 39 lock doc count error | ✅ Fixed in Wave 40 (commit 6) |
| 8 | WAVES.md missing Wave 39 | ✅ Fixed in Wave 40 (commit 1) |
| 9 | ROADMAP.md missing Wave 39 | ✅ Fixed in Wave 40 (commit 2) |

---

## Resolution Summary

- Gaps 1–4: **Closed Wave 50A.** All files created and wired.
- Gap 5: **Closed Wave 50A.** ID construction moved to workflow crate.
- Gap 6: Informational. No action needed.
- Gaps 7–9: Fixed in Wave 40.

**All known gaps are resolved as of Wave 50A.**
