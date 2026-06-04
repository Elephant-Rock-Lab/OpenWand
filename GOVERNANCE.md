# OpenWand Governance

## Operating Doctrine

From Wave 38 onward, OpenWand follows **Disk-Verified Large-Wave Execution**.

```
Scale by wave.
Ground by disk.
Accept by tests.
Seal by evidence.
```

### Core Rules

1. **Code on disk is authoritative.**
2. **Documentation is advisory.**
3. **Conversation memory is non-authoritative.**
4. **No plan may be produced before inspecting relevant files.**
5. **No wave may be accepted without tests or machine-checkable evidence.**
6. **Approved production state must not be mutated unless the wave explicitly authorizes it.**
7. **Generated artifacts must be distinguishable from source-of-truth code.**
8. **Any missing file, missing test, or mismatched assumption must be reported as a blocker or constraint.**

---

## Wave Protocol

Every wave follows five phases:

### Phase 1 — Disk Reconnaissance

Before planning, inspect relevant source files, tests, schemas, fixtures, generated artifacts, dependency boundaries, and release/audit files. Report observed facts separately from proposed changes.

### Phase 2 — Wave Plan

Only after disk reconnaissance, produce the wave plan. The plan must be file-specific with exact paths, no invented files, and no speculative modules.

### Phase 3 — Implementation

File-specific: modify, add, or create only what the plan specifies. Do not modify protected files unless explicitly authorized.

### Phase 4 — Verification

Every wave ends with: tests passing, no forbidden files changed, expected outputs confirmed.

### Phase 5 — Seal

A wave is complete only when: the intended change exists, the forbidden change did not happen, the tests passed, and the artifacts match the new behavior.

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

---

## Workflow Crate Constraints

The workflow crate (`openwand-workflow`) is a **leaf dependency**:

- **Exactly 6 dependencies:** `serde`, `serde_json`, `blake3`, `chrono`, `thiserror`, `tracing`
- **No imports from:** `openwand-core`, `openwand-session`, `openwand-tools`, `openwand-policy`, `openwand-memory`, `openwand-trace`
- **No execution surface:** no ToolExecutor, PolicyEngine, SessionRunner, LlmClient, std::process

---

## Central Invariant

```
No execution surface in evidence layers.
All tool execution flows through:
  SessionRunner → PolicyEngine → ToolExecutor → Trace
```

Evidence layers (plan, review, proposal, readiness, execution record, route, outcome, reconciliation, run revision, continuation, review, readiness, routing, gate, loop controller, command composer, command review, manual result) are **non-executing**.

---

## Lock Document Convention

Every locked wave produces a lock document in `docs/`:

```
docs/WAVE##_<DESCRIPTIVE_NAME>_LOCK.md
```

Contents:
- Test count (baseline and final)
- Commit list
- Scope and non-scope
- Invariants upheld
- Guard proofs
- Known caveats
- Next seam

---

## Publishing Posture

```
Commit frequently.
Push internally regularly.
Publish publicly deliberately.
Release only when sealed by evidence.
```

- Every locked wave may be pushed to GitHub
- No unlocked wave should be presented as stable
- Tag every locked wave: `wave-##-lock`
- Keep `main`/`master` clean with only locked waves
