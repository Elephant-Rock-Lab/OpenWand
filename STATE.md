# OpenWand — Project State

## Version
0.1.0-alpha

## Status
**Wave 50A in progress. Gap Remediation I: Provider Settings, Coverage Closure, Workspace Cleanup.**

Previous lock: Wave 49A — `9e9fc98` — `wave-49a-lock`

## Workspace Structure
```
crates/
├── core/       Domain IDs, vocabulary, events, snapshots           (lib) — 13 library crates
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

Note: `openwand-content` removed from workspace at Wave 50A (scaffold since Wave 0, zero implementation).
Will be re-added when syntect/mermaid/comrak rendering is needed.

## Test Count

**Canonical command:**
```bash
cargo test --workspace --features "openwand-session/testing,openwand-session/sqlite-testing,openwand-memory/testing,openwand-memory/sqlite-testing"
```

**Baseline (Wave 49A lock):** 3,392 tests, zero failures.

Wave 50A changes:
- Added: settings module tests (5), next-action review guard tests (12), routing readiness CLI tests (4)
- Added: next-action review UI state tests (3), next-action review UI component tests (1)
- Removed: `openwand-content` crate (1 test: `it_works`)
- Net delta: +24 tests

**Wave 50A target:** approximately 3,416 tests, zero failures.

## Wave History (Selected)

| Wave | Goal | Status |
|------|------|--------|
| 00 | Cross-document audit, lock all seams | ✅ |
| 01a–01e | Foundation crates | ✅ |
| 02a–02t | Runtime, memory, governance wiring | ✅ |
| 03a–04b | Approval governance, governed shell | ✅ |
| 23–49A | Workflow evidence ladder (24 capabilities) | ✅ |
| **50A** | **Gap Remediation I** | **🔄 In progress** |

Full wave history available in `WAVES.md` and individual `docs/WAVE*_LOCK.md` files.

## Hard Boundaries (Global)
- HB-G1: Binary < 20MB
- HB-G2: Zero telemetry, zero cloud storage dependencies
- HB-G3: All data in `~/.openwand/`
- HB-G4: Zero `unsafe` in OpenWand code (dependencies may use it)
- HB-G5: `cargo clippy --workspace` = zero warnings
