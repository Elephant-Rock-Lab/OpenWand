# OpenWand — Project State

## Version
0.1.0-alpha

## Status
**Wave 01c complete. Ready for Wave 01d (Session + Mocks).**

## Workspace Structure
```
crates/
├── core/       Domain IDs, vocabulary, events, snapshots   (lib) — 01a ✅
├── trace/      Generic trace substrate (TraceStore<E>)      (lib) — 01a ✅
├── store/      Trace+Memory persistence, StoredEvent bridge (lib) — 01a ✅
├── policy/     Deterministic trust gate, BuiltinPolicyEngine(lib) — 01b ✅
├── llm/        Provider-normalized LLM boundary, MockClient (lib) — 01b ✅
├── tools/      ToolExecutor + local tools + composite seam  (lib) — 01c ✅
├── mcp-pool/   MCP server pool via rmcp + MockGateway       (lib) — 01c ✅
├── session/    Loro CRDT session store                       (lib) — scaffold
├── memory/     3-tier memory + ACE Skillbook                 (lib) — scaffold
├── skills/     YAML + Markdown skill store                   (lib) — scaffold
├── goals/      Fitness functions + improvement               (lib) — scaffold
├── content/    Rich content rendering                        (lib) — scaffold
└── app/        Dioxus desktop binary                         (bin: openwand)
```

## Wave History

| Wave | Goal | Status |
|------|------|--------|
| 00 | Cross-document audit, lock all seams | ✅ Complete |
| 01a | Core IDs + vocabulary + events + trace substrate | ✅ Complete (47 tests) |
| 01b | Policy + LLM contracts | ✅ Complete (114 tests) |
| 01c | Tools + MCP Pool | ✅ Complete (149 tests) |
| 01d | Session + mocks | ⬚ Next |
| 01e | SQLite store | ⬚ |

## Test Count
**149 tests, zero failures, zero warnings.**

- openwand-core: 17 (15 unit + 2 guards)
- openwand-trace: 21 (19 unit + 2 guards)
- openwand-store: 9 (9 conformance)
- openwand-policy: 40 (25 unit + 14 integration + 1 guard)
- openwand-llm: 25 (23 unit + 2 guards, +11 with testing feature)
- openwand-mcp-pool: 5 (3 unit + 2 guards)
- openwand-tools: 26 (22 unit + 4 integration)
- Scaffold crates: 6 placeholder

## Key Architecture Decisions
- `StoredEvent` newtype bridges core ↔ trace (orphan-rule compliant)
- `ProvenanceSnapshot::confidence_bps: u16` (not f64) for Eq+Hash
- Rule-declared confirmation is canonical (not re-derived from risk)
- Mode floor can only raise, never lower confirmation
- No System variant in LlmMessage — system_prompt is separate field
- No Error variant in LlmDelta — errors via Result<LlmDelta, LlmError>
- ToolCallBuffer prevents partial tool calls from reaching policy
- API keys skipped in serde serialization
- No Rig dependency yet — DTO contract proven first
- Canonical tool names: `local__{tool}` / `mcp__{server}__{tool}`
- MCP effect resolution: config override → server default → annotation hints → Unknown
- ToolExecutor::execute is infallible — always returns ToolResult, never Err
- rmcp types never escape openwand-mcp-pool — pool returns own DTOs
- MockMcpGateway for CI — no real MCP servers needed for tests
- CompositeToolExecutor unifies local + MCP behind one ToolExecutor seam
- Batch 1 local tools: file_read, file_list, file_search

## Next Action
Begin Wave 01d: SessionRunner + Loro projection + mock wiring.

## Hard Boundaries (Global)
- HB-G1: Binary < 20MB
- HB-G2: Zero telemetry, zero cloud storage dependencies
- HB-G3: All data in `~/.openwand/`
- HB-G4: Zero `unsafe` in OpenWand code (dependencies may use it)
- HB-G5: `cargo clippy --workspace` = zero warnings
