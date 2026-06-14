# OpenWand — Project State

## Version
0.5.0 (stable)

## Status
**v0.5.0 stable released. Tag `v0.5.0`.**

Release: v0.5.0 — tag `v0.5.0`

Binary: 18,018,816 bytes (~17.2 MB), SHA-256 `F0BE80A04D3322C8319711AF51C48BC91CED93D01AD20CFD1AC2DB4B85CA2A3D`

Stable for v0.5.0 milestone scope. Not production-ready. Not formal security review.
No full hash recomputation. No full immutability proof.

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

**v0.5.0 stable baseline (Wave 96A):** 4,071 tests on Windows, 0 failures.
- 3,999 carried from v0.4.0
- +72 from v0.5.0 arc (92A: +16, 92B: +10, 93A: +35, 93B: +8, 94A: +3)

**Clippy posture:** 0 actionable production warnings on 11 non-app crates (HB-G5).
50 app crate pedantic/test-only warnings accepted as cosmetic.

**Desktop feature build:** PASS (0 errors, 0 warnings).

## v0.5.0 Runtime Integrity Arc

| Wave | Title | Tag | Deliverable |
|------|-------|-----|-------------|
| 91A | Post-v0.4 Roadmap Reset | `wave-91a-lock` | v0.5.0 roadmap (VE-1 through VE-4) |
| 92A | Trace Verifier Core | `wave-92a-lock` | `TraceVerifier::verify()` - chain continuity, ordering, duplicates |
| 92B | Trace Verifier CLI | `wave-92b-lock` | `openwand trace-verify` with distinct exit codes |
| 93A | Operation Replay | `wave-93a-lock` | `OperationReplayVerifier` - desktop operations to trace correspondence |
| 93B | Operation Replay CLI | `wave-93b-lock` | `openwand operation-replay` with JSON operation descriptors |
| 94A | Security Review Preparation | `wave-94a-lock` | Threat model, authority-boundary checklist, caveat ledger |
| 96A | v0.5.0 Release Preparation | `wave-96a-lock` | Release artifact, notes, blocker reconciliation |
| 96B | v0.5.0 Declaration | `v0.5.0` | Tag v0.5.0, publish release |

## v0.5.0 Blocker Resolution

| Blocker | Status | Resolution |
|---------|--------|------------|
| VE-1: Trace verifier | RESOLVED | 92A-92B: TraceVerifier + CLI with tamper detection |
| VE-2: Operation replay | RESOLVED | 93A-93B: OperationReplayVerifier + CLI with correspondence checking |
| VE-3: Linux GUI runtime | DEFERRED | Environment-gated; compile-validated (85A), no display server |
| VE-4: Security review prep | RESOLVED | 94A: Threat model, authority-boundary checklist, caveat ledger |

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
v0.1.0-alpha -> v0.1.0-beta -> v0.2.0-beta -> v0.2.0-rc.1 -> v0.2.0 -> v0.3.0 -> v0.4.0 -> v0.5.0
```

## Hard Boundaries (Global)
- HB-G1: Binary < 20MB
- HB-G2: Zero telemetry, zero cloud storage dependencies
- HB-G3: All data in `~/.openwand/`
- HB-G4: Zero `unsafe` in OpenWand production code (test-only env var manipulation
  excepted; dependencies may use it; Unix libc openat封装在 WorkspaceWriteHandle)
- HB-G5: `cargo clippy` zero warnings on 11 non-app production crates.
  `openwand-app` test-module style warnings accepted as cosmetic.
