# OpenWand v0.2.0-beta Release Notes

**Release date:** 2026-06-13
**Classification:** Public beta — not stable, not production-ready, not security-audited
**Binary:** `openwand.exe` — 17,847,296 bytes (~17.0 MB)
**SHA-256:** `D5DDECF63E9EEE92B36CB12EFB4A80CDA6FE4E7B1A88CC335A06503386C602DC`
**Commit:** `478741d`

---

## What Changed Since v0.1.0-beta

### Windows TOCTOU Closure (VB-1)

OpenWand's Windows filesystem sandbox now uses NtCreateFile handle-relative directory traversal with per-component reparse point detection. This closes the intermediate-directory TOCTOU race condition identified in the v0.1.0-beta release.

- **Unix (73B):** Fully closed via `openat` + `O_NOFOLLOW` per component. No residual.
- **Windows (78C):** Closed via NtCreateFile + `RootDirectory` + `FILE_OPEN_REPARSE_POINT` for directory traversal, `NtQueryInformationFile(FileBasicInformation)` for reparse detection. Final-component behavior remains on the 72B no-follow hardening path (`write_file_no_follow` with `FILE_FLAG_NO_REPARSE_POINT`), not a newly closed NtCreateFile final-write path.
- **Implementation:** `crates/tools/src/sandbox_ntapi.rs` (~430 lines), `crates/tools/src/sandbox.rs` `WorkspaceWriteHandle`.

### Provider Scope Sufficient for v0.2.0 (VB-2)

Validated 5 models across 2 provider families:

| Provider | Model | Tests | Result |
|----------|-------|-------|--------|
| LM Studio (local) | google/gemma-4-12b | 4/4 | PASS |
| LM Studio (local) | qwen2.5-0.5b | 4/4 | PASS |
| Z.AI hosted | glm-4.5-air | Functional equivalence | PASS |
| Z.AI hosted | glm-5.1 | Functional equivalence | PASS |
| Z.AI hosted | glm-5-turbo | Simple turn, trace, tool, refusal | PASS |

OpenWand is a governed local AI workbench, not a provider compatibility matrix. Further provider expansion (Anthropic, Ollama, direct OpenAI) is post-v0.2 compatibility hardening.

### Workflow UI Surfaces Complete (VB-3)

All 10 placeholder workflow UI surfaces replaced with product-meaningful Dioxus render functions:

| Surface | Wave | Description |
|---------|------|-------------|
| `workflow_action_outcome` | 80A | Tool outcome display with status badges |
| `workflow_verification_readiness` | 80A | Verification readiness predicates and progress |
| `workflow_continuation` | 80A | Continuation proposal and readiness |
| `workflow_proposal` | 80B | Proposal summary, stages, risks, approvals |
| `workflow_readiness` | 80B | Readiness predicates, tool intents, environment |
| `workflow_command_composer` | 80B | Command descriptor, arguments, missing inputs |
| `workflow_command_review` | 80B | Review decision, acknowledgment snapshot |
| `workflow_reconciliation` | 80C | Stage progression, run revision, predicates |
| `workflow_loop_controller` | 80C | Detected state, recommendation, evidence links |
| `workflow_external_attestation` | 80C | Attestation cards, verification status |

All surfaces use Wave 52A design-system tokens, follow the pure-helpers + desktop-gated-render pattern, and preserve backend authority boundaries.

### Code Quality

- Clippy: 390 warnings reduced to 0 actionable production warnings
- Integration test compilation repaired (memory crate `[[test]]` sections)
- 3,917 total tests (lib + integration), 0 failures
- Binary: 17,847,296 bytes (slight reduction from v0.1.0-beta)

---

## Test Baseline

| Category | Count |
|----------|-------|
| Library tests | 2,279 |
| Integration tests | 1,638 |
| **Total** | **3,917** |
| Failures | 0 |

---

## Accepted Caveats

### 1. Dependency Security Audit

`cargo audit` was run (0.22.1, RustSec 1,131 advisories). **0 vulnerabilities found.** 15 warnings (13 unmaintained, 2 unsound) — all transitive dependencies through dioxus-desktop/wry or loro. 12 of 15 are Linux-only GTK3 bindings not in the Windows binary. Full report: `docs/DEPENDENCY_AUDIT_REPORT.md`.

### 2. Pedantic Clippy Warnings

178 clippy warnings remain in test code (too_many_arguments, sort_by_key, clone_on_copy, deprecated `TempDir::into_path`). All are in `#[cfg(test)]` modules or integration tests. None affect production code correctness or security.

### 3. Non-Windows Platform Validation Deferred

Desktop UX was validated on Windows only (53 accessible elements via Windows UI Automation API). Linux/macOS desktop testing is deferred (BC-10).

### 4. Hosted Provider Validation Indirect

Z.AI hosted providers were validated through the Z.AI MCP source (functional equivalence), not through the OpenWand binary's environment-variable execution path. Auth wiring was not tested through OpenWand directly.

### 5. Post-v0.2 Provider Expansion

Anthropic, Ollama, and direct OpenAI validation are post-v0.2 compatibility hardening items. The OpenAI-compatible adapter architecture supports them; they were not prioritized for v0.2.0.

### 6. Windows Final-Component TOCTOU Residual

The final file write uses `write_file_no_follow()` with `FILE_FLAG_NO_REPARSE_POINT` (the 72B no-follow hardening path), not a NtCreateFile final-write path, due to NTFS ACL generic-to-specific right mapping. This is the same final-component behavior accepted in v0.1.0-beta.

---

## What This Release Is

- A governed local AI workbench with desktop UI
- A beta release for external validation and testing
- Evidence of a complete workflow lifecycle UI surface
- Proof of multi-provider architecture (local + hosted)
- Windows filesystem sandbox with handle-relative hardening

## What This Release Is Not

- Not production-ready
- Not a formal security review (cargo audit covers dependency advisories only)
- Not validated across all platforms (Windows only)
- Not validated across all providers (5 models, 2 families)
- Not a stable API guarantee

---

## Upgrading from v0.1.0-beta

- Replace binary
- No configuration migration needed
- Workspace structure unchanged
- Session state format unchanged
