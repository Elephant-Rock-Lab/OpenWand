# OpenWand — Project State

## Version
0.1.0-alpha

## Status
**Wave 02a complete. Real provider tool-call loop proven. Ready for Wave 02b.**

## Workspace Structure
```
crates/
├── core/       Domain IDs, vocabulary, events, snapshots   (lib) — 01a ✅
├── trace/      Generic trace substrate (TraceStore<E>)      (lib) — 01a ✅
├── store/      Trace+Memory persistence, StoredEvent bridge (lib) — 01e ✅
├── policy/     Deterministic trust gate, BuiltinPolicyEngine(lib) — 01b ✅
├── llm/        Provider-normalized LLM boundary, SSE adapter(lib) — 01b ✅ + 02a
├── tools/      ToolExecutor + local tools + composite seam  (lib) — 01c ✅
├── mcp-pool/   MCP server pool via rmcp + MockGateway       (lib) — 01c ✅
├── session/    Loro CRDT session + SessionRunner            (lib) — 01d ✅ + 02a
├── memory/     3-tier memory + ACE Skillbook                 (lib) — scaffold
├── skills/     YAML + Markdown skill store                   (lib) — scaffold
├── goals/      Fitness functions + improvement               (lib) — scaffold
├── content/    Rich content rendering                        (lib) — scaffold
└── app/        CLI smoke binary (openwand.exe)              (bin) — 02a ✅
```

## Wave History

| Wave | Goal | Status |
|------|------|--------|
| 00 | Cross-document audit, lock all seams | ✅ Complete |
| 01a | Core IDs + vocabulary + events + trace substrate | ✅ Complete (47 tests) |
| 01b | Policy + LLM contracts | ✅ Complete (114 tests) |
| 01c | Tools + MCP Pool | ✅ Complete (149 tests) |
| 01d | Session + mocks | ✅ Complete (167 tests) |
| 01e | SQLite TraceStore | ✅ Complete (187 tests) |
| 02a | Reality smoke: real model + real tool calls | ✅ Complete (197 tests) |
| 02b | TBD | ⬚ Next |
| 02b-0 to 02b-5 | Dioxus + runtime spike through crash recovery | ✅ Complete (228 tests) |
| 02c | Real MCP stdio | ✅ Complete (235 tests) |
| 02d | Memory extraction v0 | ✅ Complete (247 tests) |
| 02e | Memory persistence + UI visibility | ✅ Complete (257 tests) |
| 02f | Memory integration hardening | ✅ Complete (265 tests) |
| 02g | Real wiring | ✅ Complete (267 tests) |
| 02h | Honesty correction + binary E2E | ✅ Complete (267 tests) |
| 03a-03f | Approval governance arc | ✅ Complete (376 tests) |
| 04a-04b | Governed shell + git execution | ✅ Complete (412 tests) |
| 02i-02i-c | Explainable memory + evidence semantics | ✅ Complete (543 tests) |
| 02j | Memory-backed repo consistency check | ✅ Complete (592 tests) |
| 02k | Memory-guided prompt assembly | ✅ Complete (629 tests) |
| Output Guard | Post-inference output screening | ✅ Complete (652 tests) |
| 02l | Coordinator 02j→02k wiring | ✅ Complete (667 tests) |
| 02m | Memory panel repo-filtered view | ✅ Complete (681 tests) |

## Test Count
**197 tests, zero failures, zero warnings.**

- openwand-core: 17 (15 unit + 2 guards)
- openwand-trace: 21 (19 unit + 2 guards)
- openwand-store: 35 (conformance + migrations + query/replay)
- openwand-policy: 40 (25 unit + 14 integration + 1 guard)
- openwand-llm: 29 (25 unit + 4 SSE buffer flush)
- openwand-mcp-pool: 5 (3 unit + 2 guards)
- openwand-tools: 26 (22 unit + 4 integration)
- openwand-session: 23 (11 acceptance + 2 guards + 10 unit)
- openwand-app: 6 (1 wiring + 5 policy profile)

## Real Provider Verification (Wave 02a)

**Proven end-to-end with Qwen3 4B via LM Studio:**
```
user message
→ real LLM inference (streamed SSE)
→ tool call parsed from SSE deltas
→ buffer flushed on finish_reason: "tool_calls"
→ policy gate (Read/Search allowed)
→ local tool execution (file_list)
→ tool result returned to model
→ final assistant answer
→ SQLite trace persisted
```

## Key Architecture Decisions
- `StoredEvent` newtype bridges core ↔ trace (orphan-rule compliant)
- `ProvenanceSnapshot::confidence_bps: u16` (not f64) for Eq+Hash
- Rule-declared confirmation is canonical (not re-derived from risk)
- Mode floor can only raise, never lower confirmation
- No System variant in LlmMessage — system_prompt is separate field
- No Error variant in LlmDelta — errors via Result<LlmDelta, LlmError>
- ToolCallBuffer prevents partial tool calls from reaching policy
- API keys skipped in serde serialization
- No Rig dependency in adapter — direct reqwest for OpenAI-compatible
- Canonical tool names: `local__{tool}` / `mcp__{server}__{tool}`
- MCP effect resolution: config override → server default → annotation hints → Unknown
- ToolExecutor::execute is infallible — always returns ToolResult, never Err
- rmcp types never escape openwand-mcp-pool — pool returns own DTOs
- MockMcpGateway for CI — no real MCP servers needed for tests
- CompositeToolExecutor unifies local + MCP behind one ToolExecutor seam
- Batch 1 local tools: file_read, file_list, file_search
- SSE buffer uses std::sync::Mutex (sync scan closure)
- Smoke policy: Read + Search only. Write/Delete/Unknown blocked.
- tool_choice invariant: `tools.is_empty() → None`; `tools.non_empty() → Auto`
- RunConfig carries mode; runner uses config.mode for policy evaluation
- HTTP client: 120s timeout, to be made configurable

## Next Action
Define Wave 02b scope.

## Hard Boundaries (Global)
- HB-G1: Binary < 20MB
- HB-G2: Zero telemetry, zero cloud storage dependencies
- HB-G3: All data in `~/.openwand/`
- HB-G4: Zero `unsafe` in OpenWand code (dependencies may use it)
- HB-G5: `cargo clippy --workspace` = zero warnings
