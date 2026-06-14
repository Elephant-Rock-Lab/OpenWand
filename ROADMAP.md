# OpenWand Roadmap

> **Note:** This is the original Wave 38 roadmap, preserved for historical context.
> For current roadmap, see [docs/V070_ROADMAP.md](docs/V070_ROADMAP.md) (v0.7.0 — planning),
> [docs/V060_ROADMAP.md](docs/V060_ROADMAP.md) (v0.6.0 — complete),
> [docs/V050_ROADMAP.md](docs/V050_ROADMAP.md) (v0.5.0 — complete),
> and [docs/V040_ROADMAP.md](docs/V040_ROADMAP.md) (v0.4.0 — complete).

Calibrated from disk-verified repository state at Wave 38.

---

## Completed Waves

| Wave | Capability | Tests | Status |
|------|-----------|-------|--------|
| 00–22 | Core agent, trace, session, memory, policy, tools, governance, eval, UI | — | ✅ Locked |
| 23 | Workflow task planning | — | ✅ Locked |
| 24 | Workflow proposal | — | ✅ Locked |
| 25 | Workflow readiness | — | ✅ Locked |
| 26 | Workflow execution gate | — | ✅ Locked |
| 27 | Workflow action routing | — | ✅ Locked |
| 28 | Live session bridge | — | ✅ Locked |
| 29 | Workflow action outcome | — | ✅ Locked |
| 30 | Workflow reconciliation | — | ✅ Locked |
| 31 | Workflow continuation | — | ✅ Locked |
| 32 | Next-action review + routing readiness | — | ✅ Locked |
| 33 | Next-action routing gate | — | ✅ Locked |
| 34 | Workflow loop controller | — | ✅ Locked |
| 35 | Command composer | — | ✅ Locked |
| 36 | Command review | — | ✅ Locked |
| 37 | Manual result capture | 2824 | ✅ Locked |
| 38 | Repository reality map + calibration | 2824 | ✅ Locked |
| 39 | Disk-verified wave template + guardrails | 2824 | ✅ Locked |

---

## Phase 1 — Doctrine and Infrastructure

### Wave 39 — Disk-Verified Wave Template and Guardrail Integration

**Purpose:** Turn the disk-verified doctrine into a repeatable project mechanism.

**Expected scope:**
- Standard wave template with required disk-recon section
- Protected file/path checklist
- Acceptance checklist
- Guard test template
- Lock document template

**Key invariant:** A wave cannot be planned from memory alone. A lock cannot be accepted without evidence.

---

### Wave 40 — Capability-to-Code Traceability Matrix

**Purpose:** Map every OpenWand capability to its source files, tests, persistence paths, CLI, UI, and lock docs.

**Expected outputs:**
- Capability matrix
- Crate/file ownership map
- CLI command map
- Persistence path map
- Guard test map
- Known coverage gaps

**Non-scope:** No new runtime behavior. No workflow execution changes.

**Key invariant:** Every capability maps to code. Every code path maps to tests. Every test maps to a lock doc. Every gap is documented, not hidden.

---

## Phase 2 — Manual Workflow Evidence Lane Completion

### Wave 41 — Manual Result Review and Acceptance Gate

**Purpose:** Review the operator-reported manual result.

**Question:** Did a human reviewer accept, reject, or request changes to the reported manual result?

**Boundary:** Review is not verification. Acceptance is not reconciliation. Rejection is not rollback.

---

### Wave 42 — Manual Result Reconciliation Readiness

**Purpose:** Determine whether an accepted manual result is ready to be reconciled into workflow state.

**Boundary:** Readiness is not reconciliation. Accepted reported evidence is not verified execution.

---

### Wave 43 — Manual Result-to-Workflow Reconciliation Gate

**Purpose:** Convert reviewed-ready manual result evidence into a workflow run revision update.

**Boundary:** Reconciliation updates revision evidence. It does not execute, verify, or route.

---

## Phase 3 — Operator Productization

### Wave 44 — Full Workflow Operator Console

**Purpose:** Unified UI surface for loop controller, command composer, review, result capture, and reconciliation state.

**Boundary:** Console displays and records intent. It does not perform operations.

---

### ~~Wave 45 — Evidence Chain Inspector and Audit Packet~~ ✅ Locked

**Tests:** 3139 (+60)

**Purpose:** Export a complete evidence packet for one workflow run.

**Boundary:** Export is observation. Export does not certify truth beyond recorded evidence.

---

## Phase 4 — Verification and Attestation

### ~~Wave 46 — External Attestation Model~~ ✅ Locked

**Tests:** 3203 (+64)

**Purpose:** Attach third-party or external attestations to manual results.

**Boundary:** Attestation is reported evidence unless verified by a later explicit gate.

---

### ~~Wave 47 — Read-Only Verification Readiness~~ ✅ Locked

**Tests:** 3266 (+63)

**Purpose:** Determine whether a manual result is eligible for a future verification attempt.

**Boundary:** Verification readiness is not verification.

---

## Phase 5 — Strategic Fork

After Phase 4, OpenWand reaches a strategic decision point:

**Fork A — Stay Human-Reported:** Improve audit, UI, review flows, and operator discipline.

**Fork B — Add Governed Verification:** Verification proposal → review → readiness → execution gate → result capture → reconciliation. Only after the manual lane is proven stable and protected.

---

## Phase 6 — Controlled Automation

Automation should not begin until:
- Manual loop is complete
- Operator console is stable
- Evidence export works
- Verification model is explicit
- Protected boundaries are enforced by tests

**Potential waves:** Batch planner, batch composer, operator-approved batch execution, worker/scheduler readiness, controlled worker execution, rollback/abort monitor.

**Boundary:** Automation is a later capability. It must not be smuggled into review, readiness, recommendation, or descriptor waves.
