# OpenWand Known Gaps

Documented from disk-verified reconnaissance at Wave 40. Gaps are documented, not fixed.

---

## Coverage Gaps

| # | Gap | Detail | Impact |
|---|-----|--------|--------|
| 1 | Missing guard test for Next-Action Review | `workflow_next_action_review_guards.rs` does not exist | No import/behavior guard coverage for Wave 32's next-action review module |
| 2 | Missing CLI test for Routing Readiness | `workflow_routing_readiness_cli.rs` does not exist | CLI integration for `workflow-routing-readiness` is untested |
| 3 | Missing UI state for Next-Action Review | `workflow_next_action_review_state.rs` does not exist | No UI helper for next-action review display |
| 4 | Missing UI components for Next-Action Review | `workflow_next_action_review_components.rs` does not exist | No desktop UI rendering for next-action review |

## Design Observations

| # | Observation | Detail | Assessment |
|---|-------------|--------|------------|
| 5 | `wnar_` ID prefix built in app crate | `WorkflowNextActionReviewId` uses `format!("wnar_{}", &hex[..16])` in `crates/app/src/main.rs`, not in a workflow crate validation module | Only ID prefix not content-addressed in workflow crate. May be intentional (Wave 32 collapsed review+routing readiness). |
| 6 | Shared lock doc for Wave 32 | Waves 32 (Next-Action Review + Routing Readiness) share a single lock doc `WAVE32_NEXT_ACTION_REVIEW_ROUTING_READINESS_LOCK.md` | Two capabilities in one wave — coverage gaps in gap 1–4 may be due to this collapse |

## Documentation Drift

| # | Gap | Detail | Resolution |
|---|-----|--------|------------|
| 7 | Wave 39 lock doc count error | `WAVE39_DISK_VERIFIED_WAVE_TEMPLATE_GUARDRAILS_LOCK.md` stated 68 lock docs; actual count was 73 before Wave 39 lock, 74 after | Corrected in Wave 40 commit 6 |
| 8 | WAVES.md missing Wave 39 | Wave 39 was not added to the doctrine calibration table after lock | Fixed in Wave 40 commit 1 |
| 9 | ROADMAP.md missing Wave 39 | Wave 39 was not moved to the completed table | Fixed in Wave 40 commit 2 |

---

## Resolution Status

- Gaps 1–4: Documented, not fixed. Future wave may address.
- Gap 5: Design observation, not necessarily a defect. Requires explicit decision.
- Gap 6: Explains gaps 1–4 — two capabilities collapsed into one wave.
- Gaps 7–9: Fixed in Wave 40 (commits 1, 2, 6).

---

## How to Close Gaps 1–4

If a future wave addresses these gaps, it would:

1. Create `crates/app/tests/workflow_next_action_review_guards.rs` with standard 15–18 guard tests
2. Create `crates/app/tests/workflow_routing_readiness_cli.rs` with CLI integration tests
3. Create `crates/app/src/ui/workflow_next_action_review_state.rs` with UI helpers
4. Create `crates/app/src/ui/workflow_next_action_review_components.rs` with UI rendering
5. Wire both UI files into `crates/app/src/ui/mod.rs`
6. Optionally move `wnar_` ID construction into the workflow crate

That wave would be a **coverage gap closure** wave, not a feature wave.
