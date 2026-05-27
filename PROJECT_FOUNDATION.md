# Project Foundation

> **This document is the single source of truth for everything that exists before Wave 1.**
> It is generated during Phases 0–4 and never modified during implementation.

---

## 1. Project Identity

| Field | Value |
|-------|-------|
| Project name | OpenWand |
| Repository path | `C:\Next-Era\OpenWand\` |
| Primary language | Rust (edition 2024) |
| Runtime version | rustc 1.85+ (2024 edition) |
| Test runner | `cargo test` + `cargo nextest` (recommended) |
| Package manager | Cargo (workspace) |
| Demo command | `cargo run -p openwand-app` |
| Test command | `cargo test --workspace` |

---

## 2. Core Constraints (Permanent Risk Boundary)

These constraints apply to **every wave, forever**. They are inherited by every `WAVE_{N}_PLAN.md` and every `WAVE_{N}_HANDOFF.md`.

| Capability | Allowed | Notes |
|------------|---------|-------|
| Network calls | YES | LLM provider APIs only. MCP server communication via stdio child processes. No arbitrary outbound. |
| Shell execution | YES | Tool execution via `openwand-tools` only, gated by policy. Never unsandboxed in core. |
| Subprocess calls | YES | MCP stdio servers launched as child processes via rmcp `TokioChildProcess`. Managed lifecycle. |
| Filesystem mutation (audited repos) | YES | Through `openwand-tools` only. Working directory sandboxed. Canonical path validation. Path escapes = hard error. |
| Database persistence | YES | SQLite via rusqlite (Batch 1). Future: optional CozoDB/SurrealDB via backend swap. |
| LLM calls | YES | Through `openwand-llm` only. Circuit breaker per provider. Never direct `reqwest` in session/tools/policy. |
| Async/background execution | YES | tokio runtime. All I/O is async. Blocking SQLite writes on dedicated worker thread. |
| External API calls | YES | LLM providers (OpenAI, Anthropic, Ollama). MCP servers. No other external APIs without explicit config. |
| Automatic publishing | NO | No auto-publish to crates.io, GitHub releases, or any registry. |
| PR creation | NO | OpenWand is standalone. No automated PR generation. |
| Package/module rename | NO | Crate names locked after Wave 0. Renames require a dedicated refactor wave. |

**Rule:** If a wave requires relaxing a constraint, it must be a **dedicated refactor wave** with explicit justification. Constraints are not weakened incrementally.

### Hard Boundaries (Permanent)

| ID | Boundary | Verification |
|----|----------|--------------|
| HB-G1 | Binary < 20MB | `cargo bloat --release` CI check |
| HB-G2 | Zero telemetry, zero cloud storage dependencies | `cargo deny` + grep audit |
| HB-G3 | All data in `~/.openwand/` | Path validation in store |
| HB-G4 | Zero `unsafe` in OpenWand code | `cargo geiger` + clippy deny |
| HB-G5 | `cargo clippy --workspace` = zero warnings | CI gate |

---

## 3. Functional Requirements

| ID | Requirement | Priority | First Wave |
|----|-------------|----------|------------|
| FR-001 | 10-phase agent loop: RunStart → StepStart → BeforeInference → Inference → AfterInference → ToolGate → BeforeToolExecute → AfterToolExecute → StepEnd → RunEnd | Must | Wave 1 |
| FR-002 | Multi-provider LLM streaming: OpenAI, Anthropic, Ollama (Batch 1) | Must | Wave 1 |
| FR-003 | Tool execution dispatch: local tools + MCP server tools via unified registry | Must | Wave 1 |
| FR-004 | Policy gate: every tool call evaluated before execution. Three-way decision: Allow / RequireConfirmation / Block | Must | Wave 1 |
| FR-005 | Unified trace log: append-only, typed events, source of truth for all durable state | Must | Wave 1 |
| FR-006 | Loro CRDT session projection: live reactive state, undo/redo, branching | Must | Wave 1 |
| FR-007 | Local tool set: read_file, search_files, list_directory | Must | Wave 1 |
| FR-008 | MCP server pool: stdio transport, tool discovery, tool execution via rmcp | Must | Wave 1 |
| FR-009 | SQLite persistence: trace log, memory projections, content-addressed blobs | Must | Wave 1 |
| FR-010 | Canonical tool naming: `local__{tool}` / `mcp__{server}__{tool}` | Must | Wave 1 |
| FR-011 | Tool effect resolution: config override → server default → annotation hints → Unknown (blocked) | Must | Wave 1 |
| FR-012 | Infallible tool execution: `execute()` returns `ToolResult`, never `Err` | Must | Wave 1 |
| FR-013 | Dioxus desktop window with chat interface | Should | Wave 2 |
| FR-014 | Memory ingestion through trace: session appends, memory consumes trace IDs | Should | Wave 2 |
| FR-015 | Tool result normalization with 50K char truncation | Should | Wave 2 |
| FR-016 | MCP HTTP transport (Streamable HTTP client) | Could | Wave 3 |
| FR-017 | Skills system: YAML + Markdown skill store with auto-discovery | Could | Wave 4 |
| FR-018 | Goals system: fitness functions + autonomous improvement loops | Could | Wave 5 |
| FR-019 | Rich content rendering: syntect syntax highlighting, mermaid diagrams, markdown | Could | Wave 4 |
| FR-020 | Workflow engine: FSM-based multi-step orchestration | Could | Wave 5 |

---

## 4. Non-Functional Requirements

| ID | Requirement | Verification |
|----|-------------|--------------|
| NFR-001 | Deterministic trace: events are append-only, never mutated. Projections are rebuildable from trace log. | Trace replay conformance tests |
| NFR-002 | Local-first operation: all data on disk. No cloud dependencies for core operation. LLM providers are the only network requirement. | Safety scan + offline integration test |
| NFR-003 | Path sandboxing: all file operations resolve to canonical paths within working directory. Escapes = hard error. | Canonical path validation in every local tool |
| NFR-004 | Serde stability: event kind strings are permanent. Fields are additive only. Breaking changes require new event kinds. | Schema version tests per event family |
| NFR-005 | Dependency isolation: rmcp types never escape `openwand-mcp-pool`. Rig types never escape `openwand-llm`. | `cargo tree` + import audit |
| NFR-006 | Fail-closed policy: policy failure = block tool execution. Never fail-open. | Policy conformance tests |
| NFR-007 | No circular dependencies: workspace DAG is a strict DAG. Crate boundaries are enforced. | `cargo tree --duplicates` + CI check |
| NFR-008 | Core crate has minimal dependencies: only `serde`, `serde_json`, `chrono`, `ulid`. No `tokio`, `blake3`, `loro`, `thiserror`. | Dependency count test in core |
| NFR-009 | Thread safety: all public traits are `Send + Sync`. State uses `RwLock` or `tokio::sync`. No interior mutability without synchronization. | Compile-time enforcement via trait bounds |
| NFR-010 | Circuit breaker per LLM provider: prevents cascade failures. Timeout + error rate thresholds. | LLM conformance tests |

---

## 5. Module Structure

```text
C:\Next-Era\OpenWand\
├── Cargo.toml                    # Workspace definition
├── README.md
├── STATE.md
├── CLAUDE.md
├── PROJECT_FOUNDATION.md
├── crates/
│   ├── core/                     # Domain IDs, event vocabulary, shared DTOs
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ids.rs            # 14 domain ID types
│   │       ├── events.rs         # OpenWandTraceEvent + 9 event families
│   │       ├── snapshots.rs      # 6 snapshot DTOs
│   │       ├── vocab.rs          # Shared enums (RunStatus, ToolEffect, ConfirmationLevel, etc.)
│   │       └── tool_vocab.rs     # ToolEffect enum (used by tools, policy, mcp-pool)
│   ├── trace/                    # TraceStore<E> trait, TraceEventEnvelope, relations (NOT YET IN WORKSPACE)
│   │   └── ...
│   ├── memory/                   # Memory domain model, retrieval, temporal logic
│   │   └── ...
│   ├── store/                    # SQLite implementation of trace + memory traits (NOT YET IN WORKSPACE)
│   │   └── ...
│   ├── session/                  # 10-phase agent loop, Loro projection, coordination
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── runner.rs         # SessionRunner
│   │       ├── phases.rs         # 10-phase loop
│   │       ├── loro_projection.rs
│   │       ├── prompt_assembly.rs
│   │       └── integration.rs    # Integration traits for deps
│   ├── tools/                    # ToolExecutor trait, local tools, MCP dispatch
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── executor.rs       # ToolExecutor trait
│   │       ├── descriptor.rs     # ToolDef, ToolSource
│   │       ├── composite.rs      # CompositeToolExecutor
│   │       ├── local.rs          # LocalToolHandler trait
│   │       ├── naming.rs         # Canonical name generation
│   │       ├── effect.rs         # Tool effect resolution
│   │       └── local_tools/
│   │           ├── read_file.rs
│   │           ├── search_files.rs
│   │           └── list_directory.rs
│   ├── mcp-pool/                 # MCP server lifecycle, rmcp integration
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── gateway.rs        # McpToolGateway trait
│   │       ├── pool.rs           # McpServerPool
│   │       ├── server.rs         # McpServerConnection + state machine
│   │       ├── runner.rs         # McpServerRunner (rmcp Peer wrapper)
│   │       ├── config.rs         # McpServerConfig
│   │       ├── discovered.rs     # McpDiscoveredTool (pool DTOs)
│   │       └── notifications.rs  # OpenWandClientHandler
│   ├── policy/                   # PolicyEngine trait, three-way gate, rule classes
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── engine.rs         # PolicyEngine trait
│   │       ├── gate.rs           # GateDecision: Allow / RequireConfirmation / Block
│   │       ├── rules.rs          # MandatoryDeny / BuiltinDefault / UserOverride
│   │       └── descriptors.rs    # PolicyToolCall, PolicyToolDescriptor
│   ├── llm/                      # LLM provider normalization via Rig
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs         # LlmClient trait
│   │       ├── provider.rs       # Enum dispatch over Rig CompletionModel
│   │       ├── stream.rs         # LlmDelta, ToolCallComplete buffering
│   │       ├── circuit_breaker.rs
│   │       └── conformance.rs    # Provider conformance tests
│   ├── skills/                   # YAML + Markdown skill store (post-Batch 1)
│   │   └── ...
│   ├── goals/                    # Fitness functions + improvement loops (post-Batch 1)
│   │   └── ...
│   ├── content/                  # Rich content: syntect, mermaid, comrak (post-Batch 1)
│   │   └── ...
│   └── app/                      # Dioxus desktop binary — composition root
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── wiring.rs         # Dependency injection: pool → tools → session
│           └── ui/               # Dioxus components
├── Waves/
│   ├── Wave_00/
│   ├── Wave_01/
│   ├── Archive/
│   └── Rejected/
├── quality/
│   └── checkpoints/
│       └── WAVE_00_ACCEPTED.json
└── config/
    └── mcp_servers.toml.example  # Example MCP server configuration
```

---

## 6. Core Data Models

| Model | Purpose | Crate | First Defined |
|-------|---------|-------|---------------|
| `SessionId` | Unique session identifier (ULID-based) | core | Wave 1 |
| `RunId` | Unique agent run identifier | core | Wave 1 |
| `ToolCallId` | Unique tool call identifier | core | Wave 1 |
| `ClaimId` | Unique claim identifier (facts, decisions, preferences) | core | Wave 1 |
| `EpisodeId` | Unique conversation episode identifier | core | Wave 1 |
| `WorkflowId` | Unique workflow identifier | core | Wave 1 |
| `SkillId` | Unique skill identifier | core | Wave 2 |
| `ToolEffect` | Tool side-effect classification (Read/Search/Write/Delete/Execute/Network/Unknown) | core | Wave 1 |
| `ConfirmationLevel` | Risk-tiered confirmation requirement (Auto/Inform/Approve/Escalate) | core | Wave 1 |
| `RunStatus` | Agent run lifecycle status | core | Wave 1 |
| `OpenWandTraceEvent` | Top-level tagged union of 9 event families | core | Wave 1 |
| `SessionEvent` | Session lifecycle events (started, paused, resumed, ended) | core | Wave 1 |
| `InferenceEvent` | LLM inference events (started, delta, completed, failed) | core | Wave 1 |
| `GateEvent` | Policy gate events (evaluated, approved, denied, suspended) | core | Wave 1 |
| `ToolEvent` | Tool execution events (called, completed, failed, denied) | core | Wave 1 |
| `FileEvent` | Filesystem events (read, written, created, deleted) | core | Wave 1 |
| `MemoryEvent` | Memory system events (episode recorded, fact accepted, entity resolved) | core | Wave 2 |
| `ModeEvent` | Interaction mode transitions (conversational, agentic, workflow, review) | core | Wave 1 |
| `WorkflowEvent` | Workflow execution events (started, step completed, finished, failed) | core | Wave 4 |
| `ToolDef` | Full tool descriptor with name, effect, schema, source | tools | Wave 1 |
| `ToolResult` | Tool execution result (infallible, always has output) | tools | Wave 1 |
| `ToolSource` | Tool origin: Local or Mcp { server, remote_name } | tools | Wave 1 |
| `GateDecision` | Policy gate outcome: Allow / RequireConfirmation / Block | policy | Wave 1 |
| `PolicyToolDescriptor` | Policy-facing tool metadata (effect, risk hints, tags) | policy | Wave 1 |
| `LlmRequest` | Provider-normalized LLM request | llm | Wave 1 |
| `LlmDelta` | Streaming response chunk (Text / ToolCallDelta / ToolCallComplete / Usage) | llm | Wave 1 |
| `McpServerConfig` | MCP server connection configuration | mcp-pool | Wave 1 |
| `McpDiscoveredTool` | MCP-discovered tool (pool's own DTO, never escapes pool) | mcp-pool | Wave 1 |
| `TraceEntry<E>` | Timestamped trace entry with causation links | trace | Wave 1 |
| `TraceRelation` | Causal relation between trace entries | trace | Wave 1 |

---

## 7. Public API Surface

### openwand-core

| Function/Type | Signature | First Defined |
|---------------|-----------|---------------|
| `OpenWandTraceEvent` | `enum (Session, Inference, Gate, Tool, File, Memory, Mode, Workflow, Artifact)` | Wave 1 |
| `event_family()` | `fn(&self) -> &'static str` | Wave 1 |
| `event_kind()` | `fn(&self) -> &'static str` | Wave 1 |
| `schema_version()` | `fn(&self) -> u32` | Wave 1 |
| `ToolEffect` | `enum (Read, Search, Write, Delete, Execute, Network, Git, ..., Unknown)` | Wave 1 |
| `ConfirmationLevel` | `enum (Auto, Inform, Approve, Escalate)` | Wave 1 |

### openwand-tools

| Function/Type | Signature | First Defined |
|---------------|-----------|---------------|
| `ToolExecutor` | `trait: available_tools(), get_descriptor(), execute(), refresh_mcp_tools()` | Wave 1 |
| `CompositeToolExecutor` | `struct: implements ToolExecutor, dispatches to local + MCP` | Wave 1 |
| `ToolDef` | `struct: name, display_name, description, parameters_schema, declared_effect, source, ...` | Wave 1 |
| `ToolResult` | `struct: tool_call_id, output, is_error, duration_ms, truncated` | Wave 1 |
| `LocalToolHandler` | `trait: definition(), execute()` | Wave 1 |

### openwand-mcp-pool

| Function/Type | Signature | First Defined |
|---------------|-----------|---------------|
| `McpToolGateway` | `trait: execute_tool(), discover_all_tools(), ensure_started()` | Wave 1 |
| `McpServerPool` | `struct: implements McpToolGateway, manages all MCP connections` | Wave 1 |
| `McpServerConfig` | `struct: name, transport, auto_start, default_effect, tool_effects` | Wave 1 |
| `McpDiscoveredTool` | `struct: remote_name, description, input_schema, annotations` | Wave 1 |

### openwand-policy

| Function/Type | Signature | First Defined |
|---------------|-----------|---------------|
| `PolicyEngine` | `trait: evaluate_tool_call() -> GateDecision` | Wave 1 |
| `GateDecision` | `enum: Allow / RequireConfirmation{level} / Block{reason}` | Wave 1 |
| `PolicyToolCall` | `struct: name, arguments, declared_effect` | Wave 1 |
| `PolicyToolDescriptor` | `struct: name, declared_effect, risk_hints, tags` | Wave 1 |

### openwand-llm

| Function/Type | Signature | First Defined |
|---------------|-----------|---------------|
| `LlmClient` | `trait: stream(request) -> impl Stream<LlmDelta>` | Wave 1 |
| `LlmRequest` | `struct: system_prompt, messages, tool_defs, config` | Wave 1 |
| `LlmDelta` | `enum: Text / ToolCallDelta / ToolCallComplete / Usage` | Wave 1 |
| `LlmMessage` | `enum: User / Assistant / ToolResult` | Wave 1 |

### openwand-trace

| Function/Type | Signature | First Defined |
|---------------|-----------|---------------|
| `TraceStore<E>` | `trait: append(), query(), replay(), get_relations()` | Wave 1 |
| `TraceEventEnvelope` | `trait: event_kind(), causation_id(), correlation_id()` | Wave 1 |
| `TraceRelation` | `struct: source_id, target_id, kind` | Wave 1 |

### openwand-session

| Function/Type | Signature | First Defined |
|---------------|-----------|---------------|
| `SessionRunner` | `struct: runs the 10-phase agent loop` | Wave 1 |
| `AgentEvent` | `enum: transient UI transport (not durable, not in trace)` | Wave 1 |
| `RunHandle` | `struct: cancellation handle for active run` | Wave 1 |

### openwand-store

| Function/Type | Signature | First Defined |
|---------------|-----------|---------------|
| `SqliteTraceStore<E>` | `struct: implements TraceStore<E> via SQLite` | Wave 1 |
| `SqliteMemoryStore` | `struct: implements MemoryReadStore + MemoryProjectionStore` | Wave 1 |
| `WriterCommand` | `enum: append trace, write blob, checkpoint` | Wave 1 |

---

## 8. Testing Taxonomy

```toml
# Each crate's Cargo.toml configures test categories

[[test]]
name = "unit"
path = "tests/unit.rs"

[[test]]
name = "integration"  
path = "tests/integration.rs"

[[test]]
name = "conformance"
path = "tests/conformance.rs"
```

### Test Categories

| Category | Purpose | Example |
|----------|---------|---------|
| `unit` | Isolated module tests, no I/O | Event serialization, ID generation, effect resolution |
| `integration` | Multi-crate tests, real I/O | Session loop with mock LLM, tool execution with temp dirs |
| `conformance` | Backend contract tests | SQLite trace store vs trait, LLM provider behavior |
| `acceptance` | Wave acceptance criteria | End-to-end agent loop with real LLM |
| `safety` | Risk boundary enforcement | Path escape prevention, policy fail-closed, no rmcp leak |
| `regression` | Bug prevention | Specific scenarios from past failures |

### Key Conformance Tests (Wave 1)

| Test | What it verifies |
|------|-----------------|
| `trace_round_trip` | Serialize → deserialize → event_kind matches |
| `trace_replay_invariant` | Replay 1000 entries → projection matches |
| `policy_fail_closed` | Unknown tool effect → always blocked |
| `tool_execute_infallible` | Every error path → ToolResult { is_error: true } |
| `llm_no_rig_leak` | No Rig types in public API surface |
| `mcp_no_rmcp_leak` | No rmcp types in tools/session crates |
| `canonical_name_round_trip` | `local__read_file` → parse → route correctly |
| `effect_resolution_precedence` | Config override beats annotation hint |
| `circuit_breaker_trip` | Provider errors → breaker opens → rejects |

---

## 9. Living Demo

| Field | Value |
|-------|-------|
| Demo script | `cargo run -p openwand-app` |
| Wave 1 scenario | Open app → start chat session → send "read src/lib.rs" → agent reads file and responds |
| Expected output | Agent reads file through policy-approved tool call, streams response to UI |

**Wave-by-wave demo growth:**

| Wave | Demo capability |
|------|----------------|
| Wave 0 | `cargo build --workspace` succeeds, `cargo test --workspace` passes |
| Wave 1 | Chat with agent → reads local files, searches code, responds with file contents |
| Wave 2 | Chat with agent + connected MCP server → uses external tools with policy gates |
| Wave 3 | Rich content in chat: syntax highlighting, mermaid diagrams, markdown rendering |
| Wave 4 | Skills loaded from YAML/MD, agent uses learned behaviors |
| Wave 5 | Multi-step workflows, goal tracking, autonomous improvement |

**Rule:** The demo grows with every wave. Each wave adds one capability to the demo. The demo is never rewritten from scratch.

---

## 10. Wave Roadmap

| Wave | Name | Capability | Depends On | Risk Level |
|------|------|------------|------------|------------|
| 00 | Foundation | Workspace scaffolded, 11 crates compile, tests pass | — | Low |
| 01 | Core Loop | Agent loop + LLM streaming + trace + policy gates + read-only tools | Wave 00 | High |
| 02 | Persistence + Memory | SQLite store + memory ingestion + MCP tool execution | Wave 01 | Medium |
| 03 | Desktop UI | Dioxus chat interface + Loro session projection + rich content | Wave 02 | Medium |
| 04 | Skills + Content | YAML/MD skill store + syntect + mermaid + comrak | Wave 03 | Low |
| 05 | Workflows + Goals | FSM workflow engine + fitness functions + improvement loops | Wave 04 | Medium |

### Wave 01 Detail (Core Loop)

This is the critical first wave. It proves the architectural spine works:

```
LLM proposes
Policy gates
Tools execute
Trace records
Loro projects
Memory derives
UI observes
```

**Wave 01 crates to implement:**

| Crate | Scope |
|-------|-------|
| `openwand-core` | Full: 14 IDs, 9 event families, shared vocab, `ToolEffect` |
| `openwand-trace` | Full: `TraceStore<E>` trait, `TraceEventEnvelope`, `TraceRelation` |
| `openwand-policy` | Full: `PolicyEngine`, `GateDecision`, Batch 1 rules (read/search allowed, everything else blocked) |
| `openwand-llm` | Full: `LlmClient` trait, enum dispatch, `LlmDelta`, tool-call buffering, circuit breaker |
| `openwand-tools` | Full: `ToolExecutor` trait, `CompositeToolExecutor`, 3 local tools, canonical naming, effect resolution |
| `openwand-mcp-pool` | Full: `McpToolGateway`, `McpServerPool`, stdio transport, discovery, lifecycle |
| `openwand-session` | Full: `SessionRunner`, 10-phase loop, Loro projection, prompt assembly |
| `openwand-store` | Stub: in-memory trace store for testing (SQLite deferred to Wave 02) |

**Wave 01 does NOT include:**
- SQLite persistence (Wave 02)
- Memory ingestion (Wave 02)
- Dioxus UI (Wave 03)
- MCP HTTP transport (Wave 03+)

**Rule:** The roadmap is a hypothesis. Waves may be split, merged, or reordered with explicit justification. The roadmap is updated by the Planner, not the Executor.

---

## 11. Wave 0 Baseline

| Field | Value |
|-------|-------|
| Crate count | 11 (workspace members) |
| Binary targets | 1 (`openwand-app`) |
| Library targets | 10 (`core`, `session`, `memory`, `tools`, `mcp-pool`, `policy`, `llm`, `skills`, `goals`, `content`) |
| Build command | `cargo build --workspace` |
| Test command | `cargo test --workspace` |
| Clippy | `cargo clippy --workspace` (zero warnings) |
| Missing from workspace | `openwand-trace`, `openwand-store` (need adding before Wave 1) |
| Demo result | ✅ PASS — workspace compiles, placeholder tests pass |

**Wave 0 is the only wave without a handoff.** It is the scaffolded foundation that Wave 1 builds upon.

### Pre-Wave 1 Checklist

Before Wave 1 begins, these must be resolved:

- [ ] Add `openwand-trace` crate to workspace
- [ ] Add `openwand-store` crate to workspace
- [ ] Update `CLAUDE.md` dependency order to reflect corrected graph
- [ ] Verify `ToolEffect` addition to `openwand-core` design
- [ ] Verify all 13 design documents are consistent with each other
- [ ] Create `Waves/Wave_01/` directory with plan template

---

## 12. Known Limitations (Before Wave 1)

| Limitation | Reason | Expected Resolution |
|------------|--------|---------------------|
| No trace crate in workspace yet | Scaffold only included 11 crates | Wave 01 prerequisite — add before implementation |
| No store crate in workspace yet | Scaffold only included 11 crates | Wave 01 prerequisite — add before implementation |
| All crates have placeholder `lib.rs` | Scaffold phase creates stubs only | Wave 01 replaces with real implementations |
| No Dioxus UI | UI deferred to Wave 03 | Wave 03 |
| No real LLM calls yet | LLM crate not implemented | Wave 01 |
| No MCP servers configured | Pool not implemented | Wave 01 |
| No SQLite persistence | Store not implemented | Wave 02 |
| No memory ingestion | Memory crate not implemented | Wave 02 |
| No HTTP MCP transport | Batch 1 is stdio only | Wave 03 |
| No skills system | Post-Batch 1 feature | Wave 04 |
| No workflow engine | Post-Batch 1 feature | Wave 05 |
| `Workspace edition = "2024"` | Requires rustc 1.85+ | Verify in CI |
| No CI/CD pipeline | Project is pre-release scaffold | Post-Wave 01 |
| Rig pinned to v0.37.0 | Integration boundary stability | Pin until conformance tests prove safe upgrade |
| rmcp pinned to v1.7.0 | Integration boundary stability | Pin until conformance tests prove safe upgrade |

---

## 13. Design Document Inventory

All design documents live in the agent session's plans folder and are the authoritative reference for implementation.

| Document | Size | Status | Covers |
|----------|------|--------|--------|
| Trust Architecture | 6.6 KB | ✅ Locked | Trust principle, gates, risk-aware confirmation |
| Core Crate Design | 25.7 KB | ✅ Locked | 14 IDs, 9 event families, shared vocab |
| Rig Deep-Dive | 20.7 KB | ✅ Reference | Rig v0.37.0 API, integration boundary |
| LLM Crate Design | 27.2 KB | ✅ Locked | Provider normalization, enum dispatch, circuit breaker |
| rmcp Deep-Dive | 9.8 KB | ✅ Reference | rmcp v1.7.0 API, Peer, transports |
| Tools + MCP Pool Design | 45.1 KB | ✅ Locked | ToolExecutor, local tools, MCP pool, canonical names |
| Memory Crate Design | 32.6 KB | ✅ Locked | Domain model, temporal logic, retrieval |
| Store Crate Design | 31.9 KB | ✅ Locked | SQLite, trace projection, content-addressed blobs |
| Policy Crate Design | 34.4 KB | ✅ Locked | Three-way gate, rule classes, fail-closed |
| Trace Crate Design | 38.6 KB | ✅ Locked | Unified traceability, event vocabulary, relations |
| Session Crate Design | 46.0 KB | ✅ Locked | 10-phase loop, Loro, integration traits |
| Workflow Framework | 55.3 KB | ✅ Draft | FSM engine, 5 execution modes |
| Lifecycle Frameworks | 56.4 KB | ✅ Draft | 6 pre-build lifecycle modules |
| Interaction Modes | — KB | ✅ Draft | 4 modes, transition protocol |
| Three Decisions | 18.1 KB | ✅ Locked | cqrs-es/HelixDB/Awaken decisions |

**Total: ~364 KB of design documentation.**

---

## 14. Dependency Graph (Corrected)

```text
openwand-core          (vocab + IDs + DTOs + ToolEffect)
  ├── openwand-trace   (TraceStore<E> trait)
  ├── openwand-memory  (MemoryStore traits + domain)
  ├── openwand-policy  (PolicyEngine trait, GateDecision)
  ├── openwand-store   (implements TraceStore + MemoryStore, SQLite)
  ├── openwand-session (10-phase loop, Loro, coordination)
  │     depends on: core, trace, memory, tools, policy, llm, loro
  ├── openwand-llm     (LlmClient trait, Rig adapter)
  │     depends on: core, rig-core
  ├── openwand-tools   (ToolExecutor trait, local + MCP dispatch)
  │     depends on: core, mcp-pool
  │     └── openwand-mcp-pool (MCP server lifecycle, rmcp)
  │           depends on: core, rmcp
  ├── openwand-skills  (post-Batch 1)
  ├── openwand-goals   (post-Batch 1)
  ├── openwand-content (post-Batch 1)
  └── openwand-app     (composition root, wires everything)
        depends on: session, tools, mcp-pool, store, policy, llm
```

### Key Boundary Rules

| Rule | Enforcement |
|------|-------------|
| Session depends only on `tools`, NOT `mcp-pool` | Import audit |
| rmcp types never escape `mcp-pool` | Import audit |
| Rig types never escape `llm` | Import audit |
| `tools` types never leak into `mcp-pool` | Import audit |
| `ToolEffect` lives in `core`, not policy | Dependency check |
| `openwand-core` depends on nothing but `serde`, `serde_json`, `chrono`, `ulid` | Cargo.toml audit |

---

## 15. Sign-Off

This foundation is complete and ready for Wave 1 planning.

- [x] Project identity defined
- [x] Core constraints locked
- [x] Requirements documented (20 FRs, 10 NFRs)
- [x] Module structure established (11+2 crates)
- [x] Core data models defined (30+ types)
- [x] Public API surface documented (6 crates, 25+ public APIs)
- [x] Testing taxonomy configured (6 categories, 9 conformance tests)
- [x] Living demo defined (grows per wave)
- [x] Wave roadmap drafted (6 waves)
- [x] Wave 0 baseline verified (workspace compiles)
- [x] Known limitations documented (15 items)
- [x] Design inventory complete (15 documents, ~364 KB)
- [x] Dependency graph locked with boundary rules
