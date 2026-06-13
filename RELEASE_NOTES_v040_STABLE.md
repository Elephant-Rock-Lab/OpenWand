# OpenWand v0.4.0 — Stable Release Notes

**Release date:** 2026-06-13
**Tag:** `v0.4.0`
**Theme:** From observation to operation

---

## What's New

v0.4.0 transforms the desktop UI from an observation tool into an operational tool. The operator can now **request** workflow operations through the desktop interface, while the UI maintains its strict authority boundary — all operations delegate to existing governed runtime gates.

### Three Operation Surfaces + Feedback Loop

| Wave | Capability | What the operator can do |
|------|-----------|--------------------------|
| 88A | Workflow run initiation | Request a new workflow run from the desktop inspector |
| 88B | Approval resolution | Approve or reject pending governed tool approvals from the desktop |
| 88C | Evidence export | Request evidence/audit packet export to a validated directory |
| 89A | Real-time inspector refresh | Inspector auto-refreshes after operations; manual refresh button |

### What "request" means (authority boundary)

The desktop UI gained **intent**, not **authority**. Every operation flows through existing governed paths:

```
Desktop UI → Request DTO → UiSessionService → Existing governance gate → Result
```

The desktop still cannot:
- Import backend crates directly
- Execute tools
- Approve without policy gate
- Append trace
- Write memory
- Create workflow records without delegation
- Bypass sandbox/policy
- Mutate session state directly

---

## Metrics

| Metric | Value |
|--------|-------|
| Tests | 3,999 total (0 failures) |
| Test delta from v0.3.0 | +60 tests |
| Binary size | 17,853,952 bytes (~17.0 MB) |
| SHA-256 | `6C928123E05FD16B5AA2B223C19E3A990F222C679C90818FC56696CDB028C934` |
| Production crate clippy | 0 warnings (11 crates, HB-G5) |
| App crate warnings | 50 (accepted cosmetic, pre-existing) |
| Crates | 14 (openwand-content remains stub) |
| Desktop feature build | PASS (0 errors, 0 warnings) |

---

## New Modules (v0.4.0)

| Module | Purpose |
|--------|---------|
| `ui/workflow_run_request.rs` | Workflow run initiation request DTO + lifecycle states |
| `ui/approval_resolution_request.rs` | Approval decision request DTO + lifecycle states |
| `ui/evidence_export_request.rs` | Evidence export request DTO + lifecycle states |
| `ui/inspector_refresh_state.rs` | Inspector refresh state tracking + lifecycle |

Each DTO module is a pure data type with zero backend imports, validated by authority boundary guard tests.

---

## Wave History (v0.4.0 arc)

| Wave | Commit | Description |
|------|--------|-------------|
| 87A | `2fac92a` | Post-v0.3 roadmap reset, v0.4.0 roadmap defined |
| 88A | `ce5855f` | Workflow run initiation from desktop |
| 88B | `01156b9` | Approval resolution from desktop |
| 88C | `1644ce3` | Evidence export from desktop |
| 89A | `cb51b8d` | Real-time inspector refresh |
| 90B | (this release) | v0.4.0 release preparation |

---

## VD-1 Resolution

**VD-1 (Live workflow execution depth):** RESOLVED by waves 88A-89A.

The desktop can now request workflow initiation, resolve approvals, export evidence, and refresh the inspector after operations — all through delegated authority boundaries.

## VD-2 Status

**VD-2 (Linux GUI runtime validation):** DEFERRED.

Linux desktop feature compilation was validated in 85A (Ubuntu WSL2, GTK3/webkit2gtk-4.1). GUI runtime cannot be validated without a native display server. This remains a caveat, not a blocker.

## VD-3 Status

**VD-3 (Trace verifier implementation):** DEFERRED to v0.5.0.

The trace verifier changes the release theme from "desktop operation" to "runtime integrity hardening." It belongs cleanly in v0.5.0.

---

## Caveats

This release carries the following caveats. They do not block the v0.4.0 milestone scope.

| # | Caveat | Status |
|---|--------|--------|
| 1 | Not a formal security review | Carried from v0.3.0 |
| 2 | 50 app clippy warnings (pedantic/test-only) | Accepted cosmetic |
| 3 | Linux GUI runtime not validated | Compile-only (85A) |
| 4 | macOS not validated | No macOS environment |
| 5 | Hosted provider validation indirect | Post-v0.4 |
| 6 | Post-v0.3 provider expansion pending | Post-v0.4 |
| 7 | 15 transitive dependency warnings | Upstream-blocked |
| 8 | Windows final-component on 72B path | Accepted (Wave 72B) |
| 9 | ARID/tool-call-ID mismatch in 88B | Runner recovery index resolves it; precision refinement deferred |
| 10 | `openwand-content` remains a stub crate | Will be implemented when rich rendering is needed |
| 11 | Synchronous workflow run initiation | May block UI briefly for large workflows; background task deferred |
| 12 | v0.3.0 caveats inherited | See RELEASE_NOTES_v030_STABLE.md |

---

## What v0.4.0 is NOT

- **Not production-ready.** This is a milestone release for development purposes.
- **Not a stable API guarantee.** APIs may change in future versions.
- **Not a formal security review.** Security posture is documented but not externally audited.
- **Not a cross-platform runtime validation.** Linux GUI runtime not validated. macOS not validated.
- **Not a provider expansion release.** Provider matrix remains at v0.3.0 levels.

---

## Release Lineage

```
v0.1.0-alpha → v0.1.0-beta → v0.2.0-beta → v0.2.0-rc.1 → v0.2.0 → v0.3.0 → v0.4.0
```

---

*v0.4.0 is stable for milestone scope. It is not production-ready. It is not a formal security review. It adds no new backend authority, no policy bypass, no prompt change.*
