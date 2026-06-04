# Wave Template

Use this template when planning a new wave. Start with disk reconnaissance, then fill in each section.

---

## Wave ## — [Title]

**Purpose:** One-sentence purpose.
**Baseline:** [N] tests, zero failures (Wave ## locked)
**Scope:** What this wave adds.
**Non-scope:** What this wave explicitly does not add.

---

## Phase 1 — Disk Reconnaissance

### Files Inspected

| What | Path(s) | Method |
|------|---------|--------|
| | | |

### Observed on Disk

Report facts separately from proposals:

```
Observed on disk:
- File A contains X.
- File B already enforces Y.
- Test C covers Z.
- No current implementation found for Q.
```

### Files That Must Not Be Touched (Without Explicit Authorization)

- All sealed lock docs
- Protected crates (session, policy, tools, trace, memory) unless wave explicitly authorizes
- `crates/workflow/Cargo.toml` (6 deps)
- Git tags

---

## Phase 2 — Wave Plan

Because the current code on disk does X, this wave should add/change Y, without mutating Z.

### Commit Breakdown

| Commit | What | Expected Tests |
|--------|------|---------------:|
| 1 | DTOs and validation | +N |
| 2 | Engine / builder / hash binding | +N |
| 3 | Review / bridge / artifacts | +N |
| 4 | Persistence and idempotency | +N |
| 5 | CLI | +N |
| 6 | UI + guards + lock doc | +N |

### File Inventory

| File | Crate | Commit |
|------|-------|--------|
| | | |

---

## Phase 3 — Implementation

### Modify:
- path/to/file_1
- path/to/file_2

### Add:
- path/to/new_module.rs
- path/to/new_test.rs

### Do not modify:
- protected files (unless explicitly authorized)

---

## Phase 4 — Verification

```bash
# Full workspace tests
cargo test --workspace --features "openwand-session/testing,openwand-session/sqlite-testing,openwand-memory/testing,openwand-memory/sqlite-testing"

# Confirm no source/test diff for doctrine waves
git diff --name-only HEAD~1..HEAD -- 'crates/**/src/**' 'crates/**/tests/**'

# Confirm guard pattern
cargo test -p openwand-app --test [guard_file]
```

Expected: [N] tests, zero failures.

---

## Phase 5 — Seal

### Acceptance Criteria

- [ ] Disk reconnaissance completed
- [ ] All planned files modified/added
- [ ] No forbidden files changed
- [ ] Tests pass: [N], zero failures
- [ ] Lock document written
- [ ] Working tree clean

### Lock Document

Create `docs/WAVE##_[DESCRIPTIVE_NAME]_LOCK.md` using `LOCK_DOC_TEMPLATE.md`.

---

## Central Invariant

```
State the one-sentence invariant this wave upholds.
```

## Honest Caveats

- List what this wave does NOT do
- List known limitations
- List assumptions that may not hold in future waves
