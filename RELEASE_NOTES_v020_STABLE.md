# OpenWand v0.2.0 Release Notes

**Release date:** 2026-06-13
**Classification:** Stable release — feature-complete for v0.2.0 milestone
**Binary:** `openwand.exe` — 17,847,296 bytes (~17.0 MB)
**SHA-256:** `D5DDECF63E9EEE92B36CB12EFB4A80CDA6FE4E7B1A88CC335A06503386C602DC`
**Commit:** `77cd139`
**Predecessor:** v0.2.0-rc.1 (`83d6a24`)

---

## Stable Declaration

v0.2.0 is declared stable from v0.2.0-rc.1 after automated external review passed with caveats. The binary artifact is identical to v0.2.0-rc.1 (no code changes since rc.1).

**What "stable" means here:**
- The v0.2.0 feature set is frozen
- All v0.2.0 blockers are resolved
- The full test baseline passes (3,917 tests, 0 failures)
- Zero known dependency vulnerabilities
- All limitations are documented as caveats

**What "stable" does NOT mean:**
- Not production-ready
- Not a formal security review
- Not fully cross-platform validated
- Not validated across all providers
- Not a stable API guarantee

---

## v0.2.0 Milestone

### Windows TOCTOU Closure (VB-1)

OpenWand's filesystem sandbox now uses NtCreateFile handle-relative directory traversal on Windows and `openat` + `O_NOFOLLOW` on Unix. This closes the intermediate-directory TOCTOU race condition from v0.1.0-beta.

- **Unix (73B):** Fully closed. No residual.
- **Windows (78C):** Closed for intermediate directories via NtCreateFile + `RootDirectory` + `FILE_OPEN_REPARSE_POINT`. Final-component behavior remains on the 72B `write_file_no_follow` no-follow path — accepted, not open.

### Provider Scope (VB-2)

Validated 5 models across 2 provider families:

| Provider | Model | Path | Result |
|----------|-------|------|--------|
| LM Studio (local) | google/gemma-4-12b | OpenWand binary | 4/4 PASS |
| LM Studio (local) | qwen2.5-0.5b | OpenWand binary | 4/4 PASS |
| Z.AI hosted | glm-4.5-air | MCP (functional equivalence) | PASS |
| Z.AI hosted | glm-5.1 | MCP (functional equivalence) | PASS |
| Z.AI hosted | glm-5-turbo | MCP (functional equivalence) | PASS |

Anthropic, Ollama, and direct OpenAI validation deferred to post-v0.2 compatibility hardening.

### Workflow UI Surfaces (VB-3)

All 10 placeholder workflow UI surfaces replaced with product-meaningful Dioxus render functions using a shared design system:

| # | Surface | Wave |
|---|---------|------|
| 1 | `workflow_action_outcome` | 80A |
| 2 | `workflow_verification_readiness` | 80A |
| 3 | `workflow_continuation` | 80A |
| 4 | `workflow_proposal` | 80B |
| 5 | `workflow_readiness` | 80B |
| 6 | `workflow_command_composer` | 80B |
| 7 | `workflow_command_review` | 80B |
| 8 | `workflow_reconciliation` | 80C |
| 9 | `workflow_loop_controller` | 80C |
| 10 | `workflow_external_attestation` | 80C |

### Code Quality

- Production clippy: 0 actionable warnings
- 43 pedantic/test-only warnings accepted
- Integration test compilation repaired (81A)

---

## Dependency Audit

**Tool:** cargo-audit 0.22.1
**Database:** RustSec advisory-db (1,131 advisories)
**Scope:** 721 crate dependencies

| Metric | Result |
|--------|--------|
| Vulnerabilities | **0** |
| CVEs | **0** |
| Unmaintained warnings | 13 |
| Unsound warnings | 2 |
| Direct OpenWand dependency findings | **0** |

Full report: `docs/DEPENDENCY_AUDIT_REPORT.md`.

---

## Test Baseline

| Category | Count |
|----------|-------|
| Library tests | 2,279 |
| Integration tests | 1,638 |
| **Total** | **3,917** |
| Failures | **0** |

---

## Accepted Caveats

These caveats are accepted for v0.2.0 stable. They do not represent defects in the released product.

1. **Not a formal security review.** cargo audit covers dependency advisories only.
2. **43 pedantic/test-only clippy warnings.** All in `#[cfg(test)]` modules.
3. **Non-Windows platform validation deferred.** Windows-only testing.
4. **Hosted provider validation indirect.** Tested through MCP source, not OpenWand binary.
5. **Post-v0.2 provider expansion.** Anthropic, Ollama, direct OpenAI validation pending.
6. **15 transitive dependency warnings (0 vulnerabilities).** All upstream, not actionable.
7. **Windows final-component on 72B no-follow path.** Accepted due to NTFS ACL mapping.

---

## Release Lineage

```
v0.1.0-alpha → v0.1.0-beta → v0.2.0-beta → v0.2.0-rc.1 → v0.2.0
(967dc96)      (b29898b)     (8034bbf)      (83d6a24)      (77cd139)
```

## Upgrading

- Same binary as v0.2.0-rc.1 (no code changes)
- Same SHA-256
- No configuration migration needed

---

*This is a stable release for the v0.2.0 milestone. It is not production-ready, not a formal security review, and not a stable API guarantee. All limitations are documented above.*
