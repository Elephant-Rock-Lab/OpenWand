# OpenWand v0.3.0 Release Notes

**Release date:** 2026-06-13
**Classification:** Stable release — feature-complete for v0.3.0 milestone
**Binary:** `openwand-ui.exe` — 18,941,440 bytes (~18.0 MB)
**SHA-256:** `A5B594A33495E8AE61FB96C77F66042247AEBA768A8E59580F4C6995431FAAC5`
**Commit:** `2a418a1`
**Predecessor:** v0.2.0 (`ca325e2`)

---

## Stable Declaration

v0.3.0 is declared stable after all three v0.3.0 blockers (VC-1, VC-2, VC-3) were resolved. The workflow lifecycle is now live-wired across five surfaces, the Linux desktop feature compiles, and the Unix sandbox is validated on Linux.

**What "stable" means here:**
- The v0.3.0 feature set is frozen
- All v0.3.0 blockers are resolved
- Test baseline passes on both Windows and Linux
- Zero known dependency vulnerabilities
- All limitations are documented as caveats

**What "stable" does NOT mean:**
- Not production-ready
- Not a formal security review
- Not validated on macOS
- Not Linux GUI runtime validated (compilation only)
- Not validated across all providers
- Not a stable API guarantee

---

## v0.3.0 Milestone

**Theme:** From static surfaces to live workflows.

### VC-1: Live Workflow Wiring (Waves 84A-84C)

Five workflow inspector surfaces are now connected to live workflow-run data, completing the operator journey:

```text
proposal → readiness → outcome → reconciliation → loop control
```

| Surface | Wave | Loader | Live Data Source |
|---------|------|--------|-----------------|
| Workflow proposal | 84A | `proposal_and_review_by_workflow_run()` | Stored proposal + review records |
| Workflow readiness | 84A | `readiness_by_workflow_run()` | Stored readiness records |
| Action outcome | 84B | `outcome_by_workflow_run()` | Stored outcome records |
| Reconciliation | 84C | `reconciliation_by_workflow_run()` | Stored reconciliation + run revision |
| Loop controller | 84C | `controller_by_workflow_run()` | Stored loop controller records |

Each surface renders live data when records exist and honest empty/unavailable state (`None`) when they do not. No fixtures rendered as live data.

### VC-2: Linux Desktop Validation (Wave 85A)

OpenWand's desktop feature (`--features desktop`) compiles on Linux (Ubuntu WSL2, GTK3 + webkit2gtk-4.1). This is the first non-Windows platform validation.

**What was validated:**
- Full `--features desktop` compilation against Linux GTK3/webkit2gtk stack
- All Dioxus render functions compile against Linux platform abstraction
- 3,934 workspace tests pass on Linux (0 failures)

**What was NOT validated:**
- Linux GUI runtime (no display server in WSL2)
- macOS desktop compilation or runtime

### VC-3: Unix Sandbox E2E (Wave 85A)

The `openat`-based `WorkspaceWriteHandle` (Wave 73B) is tested natively on Linux for the first time:

| Validation | Result |
|-----------|--------|
| Unix sandbox library compilation | PASS |
| Full workspace test suite on Linux | 3,934 tests, 0 failures |
| Symlink detection at intermediate components | PASS |
| Symlink detection at final component | PASS |
| Path containment (escape prevention) | PASS |
| Workspace write handle lifecycle | PASS |

### Cross-Platform Bug Fixes (Wave 85A)

Six bugs found through Linux validation, all invisible on Windows:

1. **Missing `OsStr` import** in Unix-only sandbox code path — would have blocked all Linux compilation
2. **Unused `AsRawFd`/`FromRawFd` import** — stale import in `WorkspaceWriteHandle::create_and_write`
3. **Windows-specific path tests running on Linux** — `#[cfg(windows)]` gated
4. **Symlink test assertion too strict for Linux** — canonicalization produces `PathContainmentError` instead of `SymlinkDetected`; both are correct rejections
5. **Git path test with Windows paths on Linux** — `#[cfg(windows)]` gated
6. **`chain_hash_display` import not cfg-gated** — only used by desktop/test code paths

---

## Test Baseline

| Platform | Total Tests | Failures |
|----------|-------------|----------|
| Windows | 3,939 | 0 |
| Linux (Ubuntu WSL2) | 3,934 | 0 |

The 5-test delta is Windows-specific tests properly `#[cfg(windows)]` gated (Windows drive prefix rejection, UNC path rejection, and Windows-path git filter assertion).

---

## Accepted Caveats

These caveats are accepted for v0.3.0 stable. They do not represent defects in the released product.

1. **Not a formal security review.** cargo audit covers dependency advisories only.
2. **43 pedantic/test-only clippy warnings.** All in `#[cfg(test)]` modules.
3. **Linux GUI runtime not validated.** Desktop feature compiles on Linux; GUI not launched under native display server.
4. **macOS validation deferred.** No macOS environment available for compilation or runtime testing.
5. **Hosted provider validation indirect.** Tested through MCP source, not OpenWand binary.
6. **Post-v0.3 provider expansion.** Anthropic, Ollama, direct OpenAI validation pending.
7. **15 transitive dependency warnings (0 vulnerabilities).** All upstream, not actionable.
8. **Windows final-component on 72B no-follow path.** Accepted due to NTFS ACL mapping.

---

## Non-Claims

This release does **not** claim:
- Cross-platform GUI runtime support
- macOS compatibility
- Production readiness
- Formal security certification
- Provider compatibility beyond the 5 validated models
- Stable API guarantees

---

## Dependency Audit

**Tool:** cargo-audit 0.22.1
**Database:** RustSec advisory-db
**Scope:** 721+ crate dependencies

| Metric | Result |
|--------|--------|
| Vulnerabilities | **0** |
| CVEs | **0** |
| Unmaintained warnings | 13 |
| Unsound warnings | 2 |
| Direct OpenWand dependency findings | **0** |

---

## Release Lineage

```
v0.1.0-alpha → v0.1.0-beta → v0.2.0-beta → v0.2.0-rc.1 → v0.2.0 → v0.3.0
(967dc96)      (b29898b)     (8034bbf)      (83d6a24)      (ca325e2) (2a418a1)
```

## Upgrading

- Binary rebuilt from v0.2.0 (new commit `2a418a1`)
- No configuration migration needed
- Desktop feature build is mandatory for UI surface changes

---

*This is a stable release for the v0.3.0 milestone. It is not production-ready, not a formal security review, and not a stable API guarantee. All limitations are documented above.*
