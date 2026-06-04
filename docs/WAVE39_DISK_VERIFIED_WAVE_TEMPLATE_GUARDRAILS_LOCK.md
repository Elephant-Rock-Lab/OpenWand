# Wave 39 — Disk-Verified Wave Template and Guardrail Integration — LOCK

**Type:** Doctrine/guardrail documentation wave
**Source behavior:** Unchanged
**Tests:** 2824, zero failures (unchanged)
**Source changes:** none
**Test changes:** none
**Templates added:** yes
**Protected files changed:** no

---

## What Shipped

### Updated Files

| File | Change |
|------|--------|
| `CLAUDE.md` | Updated from 11 to 14 crates, added doctrine, evidence ladder, correct test command |
| `AGENTS.md` | New — AI agent operating rules, five-phase wave protocol, pre-wave checklist |

### New Files

| File | Purpose |
|------|---------|
| `docs/templates/WAVE_TEMPLATE.md` | Standard wave plan template with disk-recon, plan, implement, verify, seal phases |
| `docs/templates/LOCK_DOC_TEMPLATE.md` | Lock document template with required/optional sections, honest caveats requirement |
| `docs/templates/GUARD_TEST_TEMPLATE.md` | Guard test template with 15–18 test pattern, Rust code snippets, gotcha notes |
| `docs/PROTECTED_FILES.md` | Protected files and boundaries manifest |

---

## Central Invariant

```
A wave cannot be planned from memory alone.
A lock cannot be accepted without evidence.
Templates guide future waves.
They do not change runtime behavior.
```

---

## Key Boundary

Wave 39 adds documentation templates and guardrails. It does NOT:
- Change source code in any crate
- Change any test file
- Modify any sealed lock document
- Change any git tag
- Add or remove any dependency
- Change runtime behavior in any way

---

## Honest Caveats

- Templates are advisory — they guide future waves but do not mechanically enforce compliance
- The guard test template documents the current pattern; future waves may need wave-specific additions
- The protected files manifest records current boundaries; new crates may be added by future waves
- CI enforcement is not yet in place — compliance depends on agent/developer discipline
- Plan files remain outside the repo (in session directories) — future waves may address this
- No `.github/` CI, issue templates, or PR templates added yet — that remains a future wave

---

## Verification Evidence

```bash
# No source/test changes
git diff --name-only HEAD~6..HEAD -- 'crates/**/src/**' 'crates/**/tests/**'
# Expected: empty output

# Templates exist
ls docs/templates/
# Expected: GUARD_TEST_TEMPLATE.md  LOCK_DOC_TEMPLATE.md  WAVE_TEMPLATE.md

# Root docs exist
test -f AGENTS.md && echo "AGENTS.md exists"
test -f docs/PROTECTED_FILES.md && echo "PROTECTED_FILES.md exists"
# Expected: both print

# Tests unchanged
cargo test --workspace --features "openwand-session/testing,openwand-session/sqlite-testing,openwand-memory/testing,openwand-memory/sqlite-testing"
# Expected: 2824 tests, zero failures
```
