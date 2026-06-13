# OpenWand — Project State

## Version
0.4.0 (stable) — planning v0.5.0

## Status
**v0.4.0 stable released. v0.5.0 in progress. Wave 93A (operation replay) locked.**

Release: v0.4.0 — tag `v0.4.0`

Binary: 17,853,952 bytes (~17.0 MB), SHA-256 `6C928123E05FD16B5AA2B223C19E3A990F222C679C90818FC56696CDB028C934`

Stable for v0.4.0 milestone scope. Not production-ready. Not formal security review.

## Workspace Structure
```
crates/
├── core/       Domain IDs, vocabulary, events, snapshots           (lib)
├── trace/      Generic trace substrate (TraceStore<E>)              (lib)
├── store/      Trace+Memory persistence, StoredEvent bridge         (lib)
├── session/    Loro CRDT session + SessionRunner                    (lib)
├── memory/     3-tier memory + ACE Skillbook                        (lib)
├── tools/      ToolExecutor + local tools + composite seam          (lib)
├── mcp-pool/   MCP server pool via rmcp + MockGateway               (lib)
├── policy/     Deterministic trust gate, BuiltinPolicyEngine        (lib)
├── llm/        Provider-normalized LLM boundary, SSE adapter        (lib)
├── skills/     YAML + Markdown skill store                          (lib)
├── goals/      Fitness functions + improvement                      (lib)
├── workflow/   Evidence ladder: 24 capabilities, leaf crate         (lib)
└── app/        CLI binary + desktop UI + evaluation + coordination  (bin)
```

Note: `openwand-content` is a stub crate (add() only). Will be implemented when rich rendering is needed.

## Test Count

**v0.4.0 stable baseline (Wave 90B):** 3,999 tests on Windows, 0 failures.
- 3,939 carried from v0.3.0
- +60 new tests from v0.4.0 arc (88A: +14, 88B: +20, 88C: +16, 89A: +10)

**v0.5.0 current (Wave 93A):** 4,054 tests, 0 failures.
- 3,999 carried from v0.4.0
- +26 from v0.5.0 (92A: +16, 92B: +10)
- +29 from 93A (trace replay: +19, app operation replay: +10)

**Clippy posture:** 0 actionable production warnings on 11 non-app crates (HB-G5).
50 app crate pedantic/test-only warnings accepted as cosmetic.

**Desktop feature build:** PASS (0 errors, 0 warnings).

## v0.4.0 Operation Arc

| Wave | Title | Tag | Deliverable |
|------|-------|-----|-------------|
| 87A | Post-v0.3 Roadmap Reset | `wave-87a-lock` | v0.4.0 roadmap, VD-1/2/3 proposed |
| 88A | Workflow Run Initiation from Desktop | `wave-88a-lock` | Desktop → UiSessionService → execution gate → run saved |
| 88B | Approval Resolution from Desktop | `wave-88b-lock` | Desktop → explicit ARID + decision → SessionRunner → governed path |
| 88C | Evidence Export from Desktop | `wave-88c-lock` | Desktop → UiSessionService → export_audit_packet → validated output |
| 89A | Real-Time Inspector Refresh | `wave-89a-lock` | Auto-refresh after operations + manual refresh button |
| 90B | v0.4.0 Release Preparation | (this wave) | Release artifact, notes, blocker reconciliation |

## v0.4.0 Blocker Resolution

| Blocker | Status | Resolution |
|---------|--------|------------|
| VD-1: Live workflow execution depth | ✅ RESOLVED | 88A-89A: initiate, approve/reject, export, refresh |
| VD-2: Linux GUI runtime validation | DEFERRED | Compile-validated (85A), no display server for runtime |
| VD-3: Trace verifier implementation | DEFERRED to v0.5 | Belongs in runtime integrity hardening theme |

## Release Lineage

```
v0.1.0-alpha → v0.1.0-beta → v0.2.0-beta → v0.2.0-rc.1 → v0.2.0 → v0.3.0 → v0.4.0
```

## Hard Boundaries (Global)
- HB-G1: Binary < 20MB
- HB-G2: Zero telemetry, zero cloud storage dependencies
- HB-G3: All data in `~/.openwand/`
- HB-G4: Zero `unsafe` in OpenWand production code (test-only env var manipulation
  excepted; dependencies may use it; Unix libc openat封装在 WorkspaceWriteHandle)
- HB-G5: `cargo clippy` zero warnings on 11 non-app production crates.
  `openwand-app` test-module style warnings accepted as cosmetic.
