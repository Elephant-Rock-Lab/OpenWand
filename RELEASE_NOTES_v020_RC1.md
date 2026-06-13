# OpenWand v0.2.0-rc.1 Release Notes

**Release date:** 2026-06-13
**Classification:** Release candidate — not stable, not production-ready, pending external review
**Binary:** `openwand.exe` — 17,847,296 bytes (~17.0 MB)
**SHA-256:** `D5DDECF63E9EEE92B36CB12EFB4A80CDA6FE4E7B1A88CC335A06503386C602DC`
**Commit:** `e705f3c`
**Predecessor:** v0.2.0-beta (`8034bbf`)

---

## Release Candidate Declaration

v0.2.0-rc.1 is promoted from v0.2.0-beta after resolving the final beta caveat (dependency audit not run). The cargo audit is now complete with zero vulnerabilities. This release candidate is submitted for external review prior to a potential v0.2.0 stable declaration.

### Why RC, not stable?

1. **Not a formal security review.** cargo audit covers dependency advisories only.
2. **Not fully cross-platform validated.** Windows-only testing.
3. **Hosted provider validation indirect.** Tested through MCP source, not OpenWand binary directly.
4. **Pending external review.** No external validation of the v0.2.0 line yet.

---

## Blocker Closure Summary

| Blocker | Status | Closed In |
|---------|--------|-----------|
| VB-1 — Windows TOCTOU | ✅ Closed | Wave 78C (NtCreateFile handle-relative traversal) |
| VB-2 — Provider expansion | ✅ Demoted post-v0.2 | Wave 79B (5 models / 2 families sufficient) |
| VB-3 — Placeholder UI surfaces | ✅ Closed | Waves 80A–80C (10/10 surfaces implemented) |

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

All 15 warnings are transitive dependencies through dioxus-desktop/wry (12 Linux-only GTK3 bindings) or loro (1 CRDT dependency). None are exploitable. Full report: `docs/DEPENDENCY_AUDIT_REPORT.md`.

---

## Test Baseline

| Category | Count |
|----------|-------|
| Library tests | 2,279 |
| Integration tests | 1,638 |
| **Total** | **3,917** |
| Failures | **0** |

---

## Code Quality

| Metric | Result |
|--------|--------|
| Production clippy warnings | 0 actionable |
| Pedantic/test-only clippy warnings | 43 (accepted) |
| Integration test compilation | Repaired (81A) |

---

## Windows Filesystem Hardening

| Layer | Status |
|-------|--------|
| Intermediate-directory traversal | ✅ Closed — NtCreateFile + `RootDirectory` + `FILE_OPEN_REPARSE_POINT` per component |
| Intermediate-directory reparse detection | ✅ `NtQueryInformationFile(FileBasicInformation)` |
| Final-component no-follow | ✅ `write_file_no_follow()` with `FILE_FLAG_NO_REPARSE_POINT` (72B path) |
| Unix intermediate-directory | ✅ Fully closed — `openat` + `O_NOFOLLOW` per component |

Final-component behavior remains on the 72B no-follow hardening path, not a newly closed NtCreateFile final-write path. This is accepted, not classified as an open residual.

---

## Provider Validation Matrix

| Provider | Model | Path | Result |
|----------|-------|------|--------|
| LM Studio (local) | google/gemma-4-12b | OpenWand binary | 4/4 PASS |
| LM Studio (local) | qwen2.5-0.5b | OpenWand binary | 4/4 PASS |
| Z.AI hosted | glm-4.5-air | MCP (functional equivalence) | PASS |
| Z.AI hosted | glm-5.1 | MCP (functional equivalence) | PASS |
| Z.AI hosted | glm-5-turbo | MCP (functional equivalence) | PASS |

---

## Accepted Caveats

### 1. Not a Formal Security Review

cargo audit covers dependency advisories only. It does not assess build tooling, CI infrastructure, supply-chain integrity, or application-level security design.

### 2. Pedantic Clippy Warnings

43 pedantic/test-only clippy warnings remain (too_many_arguments, sort_by_key, etc.). All in `#[cfg(test)]` modules. None affect production code.

### 3. Non-Windows Platform Validation Deferred

Desktop UX validated on Windows only (53 accessible elements via Windows UI Automation API). Linux/macOS desktop testing deferred.

### 4. Hosted Provider Validation Indirect

Z.AI hosted providers validated through MCP source (functional equivalence), not through the OpenWand binary's environment-variable execution path. Auth wiring not tested through OpenWand directly.

### 5. Post-v0.2 Provider Expansion

Anthropic, Ollama, and direct OpenAI validation are post-v0.2 compatibility hardening items.

### 6. Transitive Dependency Warnings

15 cargo audit warnings accepted (13 unmaintained, 2 unsound). All transitive through UI framework or CRDT library. Remediation requires upstream framework upgrades.

### 7. Windows Final-Component Hardening

Final file write remains on the 72B `write_file_no_follow` no-follow path, not a NtCreateFile final-write path. Accepted due to NTFS ACL generic-to-specific right mapping.

---

## What This Release Is

- A governed local AI workbench with desktop UI
- A release candidate for external review
- Evidence of a complete workflow lifecycle UI surface (10/10)
- Multi-provider architecture validated across 2 families (5 models)
- Windows filesystem sandbox with handle-relative hardening
- Zero known dependency vulnerabilities

## What This Release Is Not

- Not stable
- Not production-ready
- Not a formal security review
- Not validated across all platforms (Windows only)
- Not validated across all providers (5 models, 2 families)
- Not a stable API guarantee

---

## Path to v0.2.0 Stable

| Step | Status |
|------|--------|
| Blockers resolved | ✅ |
| Dependency audit clean | ✅ (0 vulnerabilities) |
| External review | ⬜ Pending |
| Non-Windows validation | ⬜ Deferred |
| Provider expansion | ⬜ Post-v0.2 |

v0.2.0 stable declaration requires external review of this RC. Non-Windows validation and provider expansion may remain deferred for v0.2.0 stable if accepted as documented caveats.

---

## Upgrading from v0.2.0-beta

- Same binary artifact (no code changes since v0.2.0-beta)
- Same SHA-256: `D5DDECF6...C602DC`
- RC classification reflects completed dependency audit
- No configuration migration needed
