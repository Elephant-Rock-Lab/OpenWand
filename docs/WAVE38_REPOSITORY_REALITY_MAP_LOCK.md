# Wave 38 — Repository Reality Map and Roadmap Calibration — LOCK

**Type:** Non-code calibration wave
**Source behavior:** Unchanged
**Test count:** 2824, zero failures (unchanged)
**Git state:** Clean working tree

---

## What This Wave Establishes

A disk-verified repository reality map that enumerates the current workspace, workflow evidence ladder, CLI surface, test topology, guard pattern, ID formats, persistence conventions, protected mutation boundaries, and known roadmap gaps — without changing source behavior, persistence state, or workflow capabilities.

---

## Repository Structure Confirmed on Disk

**14 workspace crates:** core, trace, store, session, memory, tools, mcp-pool, policy, llm, skills, goals, content, workflow, app

**Workflow crate:** 44 source files, 14,674 LOC, exactly 6 dependencies (serde, serde_json, blake3, chrono, thiserror, tracing)

**Evidence ladder:** 19 ID formats from `tpl_` (Wave 23) through `wmr_` (Wave 37)

**CLI surface:** 20 workflow subcommands enumerated from `main.rs`

**Tests:** 2824 passing, zero failures, 72 test files, 17 guard test files

**Lock documents:** 68 files from WAVE00 through WAVE37

---

## Known Gaps (Recorded, Not Implemented)

1. No manual result review
2. No reconciliation bridge (accepted results → workflow state)
3. No operator console (unified UI)
4. No evidence export / audit packet
5. No external attestation model
6. No verification readiness

---

## Protected Mutation Inventory

- `crates/workflow/Cargo.toml` — 6 deps locked
- `crates/session/` — unchanged since Wave 22
- `crates/policy/` — unchanged since Wave 5
- `crates/tools/` — unchanged since Wave 5
- `crates/trace/` — unchanged since Wave 9
- `crates/memory/` — unchanged since Wave 2R
- All lock docs WAVE00–WAVE37

---

## Roadmap Baseline

| Wave | Purpose | Status |
|------|---------|--------|
| 23–37 | Evidence ladder (15 waves) | ✅ Locked, 2824 tests |
| 38 | Repository reality map + calibration | ✅ Locked (this wave) |
| 39 | Disk-verified wave template + guardrails | Next doctrine wave |
| 40 | Capability-to-code traceability matrix | Candidate |
| 41 | Manual result review and acceptance gate | Next feature wave |
| 42 | Manual result reconciliation readiness | Future |
| 43 | Manual result-to-workflow reconciliation gate | Future |
| 44 | Full workflow operator console | Future |
| 45 | Evidence chain inspector and audit packet | Future |
| 46 | External attestation model | Future |
| 47 | Read-only verification readiness | Future |

---

## Doctrine Compliance

- [x] Files inspected before planning
- [x] Observed facts reported separately from proposals
- [x] No feature implementation included
- [x] No mismatched assumptions found; known missing capabilities recorded as roadmap gaps
- [x] Roadmap calibrated from disk truth
- [x] Git working tree clean at seal time
