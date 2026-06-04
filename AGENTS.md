# AGENTS.md — Agent Operating Rules for OpenWand

This file defines how AI agents (Claude, Codex, others) must operate when working on OpenWand.

## Core Rule

```
Code on disk is authoritative.
Documentation is advisory.
Conversation memory is non-authoritative.
```

No plan may be produced before inspecting relevant files on disk. No wave may be accepted without tests or machine-checkable evidence.

## Wave Protocol

Every wave follows five phases:

### Phase 1 — Disk Reconnaissance

Before planning, inspect:
- Relevant source files in the target crate(s)
- Existing tests for the target area
- Lock documents for the previous wave
- Schemas, fixtures, generated artifacts
- Dependency boundaries
- Existing naming conventions

Report observed facts separately from proposed changes.

### Phase 2 — Wave Plan

Only after disk reconnaissance, produce the wave plan. The plan must:
- Be file-specific with exact paths
- Reference observed disk state
- List files to modify, add, and not touch
- Include test targets

### Phase 3 — Implementation

File-specific: modify, add, or create only what the plan specifies. Do not modify protected files unless the wave explicitly authorizes it.

### Phase 4 — Verification

Every wave ends with:
- Tests passing (full workspace)
- No forbidden files changed
- Expected outputs confirmed

### Phase 5 — Seal

A wave is complete only when:
- The intended change exists on disk
- The forbidden change did not happen
- The tests passed
- A lock document records the evidence

## Pre-Wave Checklist

Before starting any wave, verify:

- [ ] Previous wave is locked (lock doc exists, tag exists)
- [ ] Working tree is clean (`git status --short` is empty)
- [ ] Current test count matches expectation
- [ ] Target files exist on disk (do not assume)
- [ ] Dependency boundaries confirmed (workflow crate = 6 deps)
- [ ] Guard test pattern understood (15–18 guards per wave)

## Protected Files

The following are protected by default during ordinary workflow/doctrine waves. They may be modified only when a wave explicitly authorizes the crate, explains why, and adds matching tests/guards:

- `crates/session/`
- `crates/policy/`
- `crates/tools/`
- `crates/trace/`
- `crates/memory/`
- `crates/workflow/Cargo.toml` (6 deps)
- All sealed lock documents in `docs/`
- Git tags (`wave-*-lock`)

## Commit Convention

Workflow waves (23+) follow this structure:

```
Commit 1: DTOs and validation
Commit 2: Engine / builder / hash binding
Commit 3: Review / bridge / artifacts
Commit 4: Persistence and idempotency
Commit 5: CLI
Commit 6: UI + guards + lock doc
```

Smaller waves may collapse commits. The final commit always includes the lock doc.

## Reporting Convention

When reporting disk reconnaissance, use this format:

```
Observed on disk:
- File A contains X.
- File B already enforces Y.
- Test C covers Z.
- No current implementation found for Q.
```

Do not mix observations with proposals. Report facts first, then plan.

## Failure Protocol

If during reconnaissance you discover:
- A file that should exist but doesn't — report it as a blocker
- A test that should pass but doesn't — report it as a blocker
- A mismatch between lock doc and actual code — report it as a gap
- A missing guard test — report it as a gap

Do not silently fix, assume, or invent. Report and wait for direction.
