# Protected Files and Boundaries

This document lists files and boundaries that are protected during ordinary workflow and doctrine waves. They may be modified only when a wave explicitly authorizes the change, explains why, and adds matching tests/guards.

---

## Workflow Crate Dependency Boundary

**File:** `crates/workflow/Cargo.toml`

The workflow crate must have exactly 6 dependencies:
- `serde`
- `serde_json`
- `blake3`
- `chrono`
- `thiserror`
- `tracing`

No dependency on: `openwand-core`, `openwand-session`, `openwand-tools`, `openwand-policy`, `openwand-memory`, `openwand-trace`, `openwand-store`, `openwand-skills`, `openwand-goals`, `tokio`, `uuid`, or any other crate.

This boundary is enforced by guard tests in every workflow wave.

---

## Protected Crates (Default)

The following crates are protected by default during ordinary workflow/doctrine waves. They may be modified only when a wave explicitly authorizes the crate, explains why, and adds matching tests/guards:

| Crate | Last Modified | Role |
|-------|--------------|------|
| `crates/session/` | Wave 22 | Session runner, config, testing harness |
| `crates/policy/` | Wave 05 | Governance, budget, redaction, access control |
| `crates/tools/` | Wave 05 | Built-in tools, filesystem, shell, web, browser |
| `crates/trace/` | Wave 09 | Authority, append, query, stream, store |
| `crates/memory/` | Wave 02R | 3-tier memory, evaluation, evidence |

---

## Sealed Lock Documents

All lock documents in `docs/WAVE*.md` are sealed after their respective wave completes. They must not be modified after seal except by an explicit correction wave that records the reason in its own lock doc.

**Current count:** 73 lock documents before Wave 39, 74 after Wave 39 lock (WAVE00 through WAVE39). The initial reconnaissance count of 68 was incorrect.

---

## Git Tags

All `wave-*-lock` tags are sealed. They must not be moved, deleted, or re-created pointing to different commits.

**Current tags:** 23 (`wave-23-lock` through `wave-45-lock`)

---

## Root Documentation

The following root docs may be updated by doctrine/calibration waves with explicit justification, but should not be changed during feature waves:

| File | Purpose |
|------|---------|
| `README.md` | Project overview, architecture, quick start |
| `ROADMAP.md` | Forward wave plan calibrated from disk truth |
| `GOVERNANCE.md` | Operating doctrine, authority separation |
| `WAVES.md` | Wave index, commit convention, ID prefix registry |
| `CLAUDE.md` | Agent/developer context, architecture rules |
| `AGENTS.md` | AI agent operating rules, wave protocol |

---

## Workflow Evidence Chain

The evidence chain is append-only. Each wave adds new evidence layers but must not modify existing sealed layers:

```
skills/goals → task plan → review → proposal → review → readiness
→ execution → routing → bridge → outcome → reconciliation
→ run revision → continuation → next-action review → routing readiness
→ routing gate → loop controller → command composer → command review
→ manual result capture
```

---

## Authority Separation

```
Workflow owns evidence and progression.
Session owns execution.
Policy owns gates.
Tools own effects.
Trace owns authority.
Memory remains governed/derived.
Operators own manual action.
OpenWand records what it can prove.
```

Cross-boundary violations must be caught by guard tests.
