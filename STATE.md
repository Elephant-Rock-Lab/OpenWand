# OpenWand — Project State

## Version
0.6.0 (stable) — planning v0.7.0

## Status
**v0.6.0 stable released. v0.7.0 in progress. Wave 103A (post-v0.6 roadmap reset) locked.**

Release: v0.6.0 — tag `v0.6.0`

Binary: 18,027,008 bytes (~17.2 MB), SHA-256 `A9C00D5BBA402BDB42FA6E2E595C90612126E0FD604ED4066D5A27174AE860AC`

Stable for v0.6.0 milestone scope. Not production-ready. Not formal security review.
No full physical immutability. No external trust anchor.

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

**v0.6.0 stable baseline (Wave 102A):** 4,099 tests on Windows, 0 failures.
- 4,071 carried from v0.5.0
- +28 from v0.6.0 arc (98A: +14, 98B: +4, 99A: +5, 99B: +4, 101A: rename only)

**Clippy posture:** 0 actionable production warnings on 11 non-app crates (HB-G5).
50 app crate pedantic/test-only warnings accepted as cosmetic.

**Desktop feature build:** PASS (0 errors, 0 warnings).

## v0.7.0 External Assurance Arc

| Wave | Title | Tag | Deliverable |
|------|-------|-----|-------------|
| 103A | Post-v0.6 Roadmap Reset | `wave-103a-lock` | v0.7.0 roadmap (VG-1 through VG-5) |
| 104A | External Anchor Design | (this wave) | Anchor DTOs, root-hash computation, verification semantics |

## v0.7.0 Blocker Plan

| Blocker | Description | Priority |
|---------|-------------|----------|
| VG-1: External anchor / checkpoint | Periodic checkpoint hash persisted outside store root; verifier checks anchor | P1 (core) |
| VG-2: Security review execution | Automated scanning + structured authority review | P1 (core) |
| VG-3: Linux GUI runtime | Environment-gated; may defer | P2 |
| VG-4: Provider validation expansion | Direct OpenAI/Anthropic/Ollama if strategic | P2 |
| VG-5: Evidence UX hardening | Exportable verification reports | P2 |

## v0.6.0 Evidence-Backed Assurance Arc

| Wave | Title | Tag | Deliverable |
|------|-------|-----|-------------|
| 97A | Post-v0.5 Roadmap Reset | `wave-97a-lock` | v0.6.0 roadmap (VF-1 through VF-5) |
| 98A | Hash Verification Policy | `wave-98a-lock` | HashVerificationPolicy trait + Blake3HashPolicy |
| 98B | Hash Recomputation CLI | `wave-98b-lock` | trace-verify with hash correctness checking |
| 99A | Trace-backed Workflow Initiation | `wave-99a-lock` | ModStarted/ModCompleted trace emission |
| 99B | Trace-backed Evidence Export | `wave-99b-lock` | ArtifactGenerated trace emission |
| 101A | TD-93B-1 Module Naming | `wave-101a-lock` | Renamed operation_audit.rs to operation_replay.rs |
| 102A | v0.6.0 Release Preparation | `wave-102a-lock` | Release artifact, notes, blocker reconciliation |
| 102B | v0.6.0 Declaration | `v0.6.0` | Tag v0.6.0, publish release |

## v0.6.0 Blocker Resolution

| Blocker | Status | Resolution |
|---------|--------|------------|
| VF-1: Backend hash-correctness | RESOLVED | 98A-98B: HashVerificationPolicy + CLI integration |
| VF-2: Trace-backed operation coverage | RESOLVED | 99A-99B: Workflow initiation + evidence export trace |
| VF-3: Security review execution | DEFERRED | Post-v0.6 |
| VF-4: Linux GUI runtime | DEFERRED | Environment-gated; compile-validated only |
| VF-5: TD-93B-1 naming | RESOLVED | 101A: Renamed to operation_replay.rs |

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
| 97A | Post-v0.5 Roadmap Reset | (this wave) | v0.6.0 roadmap, VF-1 through VF-5 proposed |

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
v0.1.0-alpha -> v0.1.0-beta -> v0.2.0-beta -> v0.2.0-rc.1 -> v0.2.0 -> v0.3.0 -> v0.4.0 -> v0.5.0 -> v0.6.0
```

## Hard Boundaries (Global)
- HB-G1: Binary < 20MB
- HB-G2: Zero telemetry, zero cloud storage dependencies
- HB-G3: All data in `~/.openwand/`
- HB-G4: Zero `unsafe` in OpenWand production code (test-only env var manipulation
  excepted; dependencies may use it; Unix libc openat封装在 WorkspaceWriteHandle)
- HB-G5: `cargo clippy` zero warnings on 11 non-app production crates.
  `openwand-app` test-module style warnings accepted as cosmetic.
