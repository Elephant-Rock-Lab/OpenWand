# Lock Document Template

Every locked wave produces a lock document at `docs/WAVE##_<DESCRIPTIVE_NAME>_LOCK.md`. Use this template to maintain consistency across waves.

---

## Required Sections

### Title and Metadata

```markdown
# Wave ## — [Descriptive Name] — LOCK

**Committed:** [N] commits
**Baseline:** [X] tests (Wave ##-1 locked)
**Final:** [Y] tests (+Z), zero failures
```

For non-code waves, use:

```markdown
**Type:** [Non-code calibration / Doctrine / Guardrail]
**Source behavior:** Unchanged
**Test count:** [N], zero failures (unchanged)
```

---

### `## What Shipped`

Tables of new modules added to each crate:

```markdown
### New Modules in `openwand-workflow`

| Module | Purpose |
|--------|---------|
| `module_name.rs` | One-line description |

### New Modules in `openwand-app`

| File | Purpose |
|------|---------|
| `module_name.rs` | One-line description |
```

---

### `## Test Breakdown`

Table of test areas and counts:

```markdown
| Area | Count |
|------|------:|
| DTO / Validation | N |
| Hash binding / Validation | N |
| Persistence / Idempotency | N |
| CLI | N |
| UI + Guards + Lock Doc | N |
| **Total** | **N** |
```

---

### `## Central Invariant`

A code block with the core invariant in plain text:

```markdown
## Central Invariant

```
[One-sentence invariant this wave upholds.]
```
```

Every workflow wave must state what its evidence layer is NOT (not execution, not verification, not routing, etc.).

---

### `## Patch Compliance` (if patches were required)

Table of patches and their proof:

```markdown
| Patch | Status |
|-------|--------|
| 1. [Description] | ✅ `[test_name]` |
```

If no patches were required, omit this section.

---

### `## Key Boundary`

A bullet list of what the wave explicitly does NOT do:

```markdown
## Key Boundary

Wave ## [does X]. It does NOT:
- Execute commands
- Route actions
- Resolve approvals
- Mutate workflow state
- ...
```

---

### `## Honest Caveats`

**Always present.** List limitations and assumptions:

```markdown
## Honest Caveats

- This wave does not [X]. Future waves may add [X].
- [Known limitation].
- [Assumption that may not hold].
```

Do NOT omit this section. Even if the only caveat is "No known caveats beyond those listed in the spec," say so explicitly.

---

## Optional Sections (Wave-Dependent)

| Section | When to Use |
|---------|------------|
| `## Dependency Posture` | When adding/modifying crate dependencies |
| `## CLI Surface` | When adding CLI subcommands |
| `## [Flow] Flow` | When there's a clear data/control flow to document |
| `## Detected [States/Loop States]` | When defining state machines |
| `## Stage Progression Rules` | When modifying stage transitions |
| `## Routing Algorithm` | When defining routing logic |

---

## Lock Doc Must NOT

- Claim execution, verification, or routing that didn't happen
- Omit the Honest Caveats section
- Modify or contradict previous lock docs
- Assert test counts without running the full suite
