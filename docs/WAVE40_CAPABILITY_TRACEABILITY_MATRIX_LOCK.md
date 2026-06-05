# Wave 40 — Capability-to-Code Traceability Matrix — LOCK

**Type:** Doctrine/productization documentation wave
**Source behavior:** Unchanged
**Tests:** 2824, zero failures (unchanged)
**Source changes:** none
**Test changes:** none

---

## What Shipped

### Updated Files

| File | Change |
|------|--------|
| `WAVES.md` | Added Wave 39 to doctrine calibration table |
| `ROADMAP.md` | Moved Wave 39 to completed table, updated Wave 40 section |
| `docs/PROTECTED_FILES.md` | Corrected lock doc count from 68 to 73 pre-Wave-39 / 74 post-Wave-39 |

### New Files

| File | Purpose |
|------|---------|
| `docs/CAPABILITY_TRACEABILITY_MATRIX.md` | 15 primary capability rows mapping all 13 matrix dimensions |
| `docs/capability_traceability_matrix.json` | Machine-readable JSON companion (15 capabilities, 19 ID prefixes) |
| `docs/KNOWN_GAPS.md` | 9 gaps documented (4 coverage, 2 design, 3 documentation drift) |

---

## Central Invariant

```
Every capability maps to code.
Every code path maps to tests.
Every test maps to a lock doc.
Every gap is documented, not hidden.
```

---

## Key Boundary

Wave 40 maps existing capabilities. It does NOT:
- Change source code in any crate
- Change any test file
- Modify any sealed lock document (except PROTECTED_FILES.md count correction)
- Change any git tag
- Add or remove any dependency
- Change runtime behavior in any way
- Fix any of the documented gaps

---

## Matrix Dimensions

Each capability row maps 13 dimensions:

1. Workflow source modules
2. App persistence module
3. CLI command + subcommands
4. UI state file
5. UI components file
6. CLI integration test
7. Guard test file
8. Persistence root directory
9. ID prefix
10. ID type name
11. ID constructor location
12. Lock document filename
13. Git tag

---

## Observed Gaps Summary

| Category | Count | Status |
|----------|------:|--------|
| Coverage gaps (missing tests/guards/UI) | 4 | Documented, not fixed |
| Design observations | 2 | Documented for future decision |
| Documentation drift | 3 | Fixed in this wave |
| **Total** | **9** | |

---

## Honest Caveats

- The matrix is a point-in-time snapshot; it will drift as waves add capabilities
- Gaps 1–4 stem from Wave 32 collapsing two capabilities into one wave
- The `wnar_` ID being in the app crate is a design observation, not necessarily a bug
- No machine enforcement of markdown↔JSON consistency (verified manually)
- The JSON companion is for machine use; the markdown is the authoritative source
- Future waves should update both the markdown and JSON when adding capabilities
- The lock doc count error in Wave 39 (68 vs 74) suggests earlier disk reconnaissance was less thorough than Wave 38+ doctrine requires

---

## Verification Evidence

```bash
# No source/test changes across all 7 commits
git diff --name-only HEAD~6..HEAD -- 'crates/**/src/**' 'crates/**/tests/**'
# Expected: empty output

# All changed files are docs-only
git diff --name-only HEAD~6..HEAD
# Expected: WAVES.md, ROADMAP.md, docs/PROTECTED_FILES.md,
#   docs/CAPABILITY_TRACEABILITY_MATRIX.md,
#   docs/capability_traceability_matrix.json, docs/KNOWN_GAPS.md,
#   docs/WAVE40_CAPABILITY_TRACEABILITY_MATRIX_LOCK.md

# JSON consistency
python3 -c "import json; d=json.load(open('docs/capability_traceability_matrix.json')); assert len(d['capabilities'])==15; assert len(d['id_prefixes'])==19; print('OK')"
# Expected: OK

# Matrix files exist
test -f docs/CAPABILITY_TRACEABILITY_MATRIX.md && echo "matrix md exists"
test -f docs/capability_traceability_matrix.json && echo "matrix json exists"
test -f docs/KNOWN_GAPS.md && echo "gaps exist"
# Expected: all three print

# Tests unchanged
cargo test --workspace --features "openwand-session/testing,openwand-session/sqlite-testing,openwand-memory/testing,openwand-memory/sqlite-testing"
# Expected: 2824 tests, zero failures
```
