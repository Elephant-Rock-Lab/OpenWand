# WAVE 01 Interface Matrix

**Status:** Deep Audit Complete  
**Source:** Cross-document audit of all spine design documents  
**Last Updated:** 2026-05-26  

---

## Phase 1 Audit: Type-Level Conflicts (5 found, 5 resolved)

| # | Conflict | Resolution | Status |
|---|----------|------------|--------|
| 1 | `ToolEffect` defined in both core and policy | Moved to core. Policy imports. | ✅ Patched |
| 2 | `ToolSource` name collision across 3 crates | Core → `ToolInvoker`. Tools keeps `ToolSource`. Policy keeps `PolicyToolSource`. | ✅ Patched |
| 3 | `GateResultSnapshot` 3 vs 5 fields | Core canonical (5 fields). Trace patched. | ✅ Patched |
| 4 | `TraceId(Ulid)` vs core IDs `SessionId(String)` | Different layers, different conventions. No conflict. | ✅ No patch needed |
| 5 | `DecisionId` legacy alias in core | Removed. Use `ClaimId` everywhere. | ✅ Patched |

---

## Phase 2 Audit: Field-Level Event Mismatches (6 found)

Core is canonical for all event definitions. Trace doc duplicated event vocab as reference but drifted.

| # | Event | Core (canonical) | Trace doc (drifted) | Fix |
|---|-------|-----------------|---------------------|----|
| 6 | `SessionEvent` | Has `UserMessageInjected { text }` variant | Missing this variant entirely | ✅ Patched: trace doc §6.2 must match core |
| 7 | `InferenceEvent::Called` | `thinking_budget: Option<ThinkingBudgetSnapshot>` + `prompt_assembly: PromptAssemblySnapshot` | `thinking_budget: Option<u32>` (wrong type) + no `prompt_assembly` field | ✅ Patched: trace doc must match core |
| 8 | `ToolEvent::Resumed` | Has `tool_name: String` field | Missing `tool_name` field | ✅ Patched: trace doc must match core |
| 9 | `ToolEvent::Denied` | Has full variant: `Denied { tool_call_id, tool_name }` | Missing this variant entirely | ✅ Patched: trace doc must match core |
| 10 | `TraceEventEnvelope` impl in trace doc | — | Returns family-level name (`"session"`, `"tool"`) instead of kind-level name (`"session.started"`, `"tool.called"`) | ✅ Patched: trace doc impl is wrong — must delegate to core's `event_kind()` |
| 11 | `InteractionMode` | Defined in core `mode.rs` | Also defined inline in trace doc §6.8 | ✅ Patched: trace doc must import from core, not redefine |

---

## Phase 2 Audit: Trait Name Conflicts (2 found)

| # | Issue | Where | Resolution | Fix |
|---|-------|-------|------------|----|
| 12 | Session references `Arc<dyn MemoryStore>` but memory crate defines a single flat `MemoryStore` trait while store doc splits into `MemoryReadStore` + `MemoryProjectionStore` | session §6.1, memory §10 | Session patched to `MemoryReadStore`. Memory trait split deferred to Wave 02. | ✅ Patched |
| 13 | Session's `StepContext` field `memory: Arc<dyn MemoryStore>` uses wrong trait name | session §3.3 | Patched to `MemoryReadStore` | ✅ Patched |

---

## Phase 2 Audit: Dependency and Adapter Gaps (3 found)

| # | Issue | Where | Resolution |
|---|-------|-------|------------|
| 14 | Session `Session` struct (§6.1) also references `Arc<dyn MemoryStore>` — same as #12 | session §6.1 | Must be `MemoryReadStore` |
| 15 | Memory crate defines `MemoryStore` as single flat trait with `put_episode()`, `search_hybrid()`, etc. But store design says session only sees reads (`MemoryReadStore`). The write methods must be in a separate trait. | memory §10 | Memory crate needs trait split before Wave 02 |
| 16 | Session `ToolCall` has fields `{ id, name, arguments }` — same shape as tools' `ToolCall`. But they're in different crates. This is correct per the "crate-local DTOs" decision. But the session doc also has `ToolResult` with identical fields to tools' `ToolResult`. Need to verify these are intentionally separate. | session §4.1-4.2 vs tools design | Session's `ToolResult` is its own internal type for Loro projection. Tools' `ToolResult` is the executor return. Different lifecycle — session converts from tools' ToolResult. ✅ Correct, no conflict. |

---

## Legend

| Status | Meaning |
|--------|---------|
| ✅ Confirmed | Unambiguous ownership, no conflicts |
| ⚠️ Needs Patch | Conflict found, resolution identified, doc update needed |
| ❌ Conflict | Conflict found, resolution needed |
| 🔲 Missing | Referenced but not defined in any doc |

---

## 1. Domain IDs (openwand-core)

| Interface | Owning Crate | Imported By | Wave | Status | Notes |
|-----------|-------------|-------------|------|--------|-------|
| `SessionId` | core | session, trace, store, memory, policy, llm | 01a | ✅ | ULID-based String |
| `EpisodeId` | core | session, memory, store | 01a | ✅ | |
| `EntityId` | core | memory, store | 01a | ✅ | |
| `ClaimId` | core | memory, session, store, trace | 01a | ✅ | Unified ID for facts, decisions, preferences |
| `DecisionId` | core | (removed — use ClaimId everywhere) | 01a | ✅ | Removed from core. Use ClaimId everywhere. If memory needs an internal alias, it defines one locally. |
| `ArtifactId` | core | store, content | 01a | ✅ | |
| `ToolCallId` | core | session, tools, policy, llm, trace | 01a | ✅ | |
| `MessageId` | core | session | 01a | ✅ | |
| `ApprovalRequestId` | core | session, policy | 01a | ✅ | |
| `ChunkId` | core | memory, store | 01a | ✅ | |
| `RunId` | core | session | 01a | ✅ | |
| `GateId` | core | session, policy | 01a | ✅ | |
| `WorkflowId` | core | workflow | 01a | ✅ | |
| `ModId` | core | workflow | 01a | ✅ | |

---

## 2. Event Vocabulary (openwand-core)

| Interface | Owning Crate | Imported By | Wave | Status | Notes |
|-----------|-------------|-------------|------|--------|-------|
| `OpenWandTraceEvent` | core | trace, store, session | 01a | ✅ | Tagged union, serde tagged |
| `SessionEvent` | core | session | 01a | ✅ | |
| `InferenceEvent` | core | session, llm | 01a | ✅ | |
| `GateEvent` | core | session, policy | 01a | ✅ | |
| `ToolEvent` | core | session, tools | 01a | ✅ | |
| `FileEvent` | core | tools, session | 01a | ✅ | |
| `MemoryEvent` | core | memory, session | 01a | ✅ | |
| `ModeEvent` | core | session | 01a | ✅ | |
| `WorkflowEvent` | core | workflow, session | 01a | ✅ | Post-Batch 1 |
| `ArtifactEvent` | core | content, session | 01a | ✅ | Post-Batch 1 |
| `event_family()` | core | trace, store | 01a | ✅ | Returns family name string |
| `event_kind()` | core | trace, store | 01a | ✅ | Returns dotted name string |
| `schema_version()` | core | trace, store | 01a | ✅ | Returns u16 |

---

## 3. Shared Vocabulary Enums (openwand-core)

| Interface | Owning Crate | Imported By | Wave | Status | Notes |
|-----------|-------------|-------------|------|--------|-------|
| `InteractionMode` | core | session, policy | 01a | ✅ | Direct/Conversational/AutoRouting/Custom |
| `ConfirmationLevel` | core | session, policy | 01a | ✅ | Auto/Inform/Approve/Escalate |
| `RiskLevelSnapshot` | core | policy, trace, session | 01a | ✅ | Low/Medium/High/Critical |
| `ToolEffect` | core | policy, tools, mcp-pool | 01a | ✅ | **PATCHED:** Moved to core tool_vocab.rs. Policy imports from core. |
| `ToolSource` | core | tools, session, policy | 01a | ✅ | **PATCHED:** Core's ToolSource renamed to ToolInvoker. Tools keeps its own ToolSource. Policy uses PolicyToolSource. |
| `ToolResultStatus` | core | tools, session | 01a | ✅ | Success/Error/Partial/Pending |
| `SessionEndReason` | core | session | 01a | ✅ | |
| `ThinkingBudgetSnapshot` | core | session, llm | 01a | ✅ | Off/Low/Medium/High/Max/Tokens(u32) |
| `EntityKind` | core | memory | 01a | ✅ | Post-Batch 1 |
| `Predicate` | core | memory | 01a | ✅ | Post-Batch 1 |
| `ClaimKind` | core | memory | 01a | ✅ | Post-Batch 1 |
| `ClaimStatusSnapshot` | core | memory | 01a | ✅ | Post-Batch 1 |
| `MemoryScope` | core | memory | 01a | ✅ | Post-Batch 1 |
| `ProvenanceSnapshot` | core | memory | 01a | ✅ | Post-Batch 1 |
| `ConfidenceLevel` | core | memory | 01a | ✅ | Post-Batch 1 |

---

## 4. Snapshot DTOs (openwand-core)

| Interface | Owning Crate | Imported By | Wave | Status | Notes |
|-----------|-------------|-------------|------|--------|-------|
| `TokenUsageSnapshot` | core | llm, session, trace | 01a | ✅ | |
| `GateResultSnapshot` | core | policy, memory, trace | 01a | ✅ | **PATCHED:** Core is canonical (5 fields). Trace doc updated. |
| `AccuracyRecordSnapshot` | core | content, workflow | 01a | ✅ | Post-Batch 1 |
| `AccuracyCheckSnapshot` | core | session | 01a | ✅ | Post-Batch 1 |
| `PromptAssemblySnapshot` | core | session | 01a | ✅ | |
| `ErrorSnapshot` | core | session, tools | 01a | ✅ | |

---

## 5. Trace Substrate (openwand-trace)

| Interface | Owning Crate | Imported By | Wave | Status | Notes |
|-----------|-------------|-------------|------|--------|-------|
| `TraceStore<E>` | trace | store, session | 01a | ✅ | Generic trait, no core dependency |
| `TraceEventEnvelope` | trace | store (impl for OpenWandTraceEvent) | 01a | ✅ | |
| `TraceEntry<E>` | trace | store, session, memory | 01a | ✅ | |
| `TraceId` | trace | store, session, memory, tools | 01a | ✅ | Different layers, different conventions. Trace IDs are internal to trace substrate. No conflict. |
| `TraceStreamId` | trace | store, session | 01a | ✅ | |
| `TraceStreamScope` | trace | store, session | 01a | ✅ | |
| `TraceRelation` | trace | store, session, memory | 01a | ✅ | |
| `TraceRelationKind` | trace | store, session, memory | 01a | ✅ | |
| `TraceRelationDraft` | trace | session | 01a | ✅ | |
| `AppendTraceEntry<E>` | trace | session | 01a | ✅ | |
| `IdempotencyKey` | trace | store, session | 01a | ✅ | |
| `TraceQuery` | trace | store, session | 01a | ✅ | |
| `RelationQuery` | trace | store, session | 01a | ✅ | |
| `TracePage<E>` | trace | store, session | 01a | ✅ | |
| `TraceProjector<E>` | trace | store, memory | 01a | ✅ | |
| `ProjectionCheckpoint` | trace | store | 01a | ✅ | |
| `Actor` | trace | session, store | 01a | ✅ | User/Llm/System/MemoryPipeline/etc. |
| `EntryHash` | trace | store | 01a | ✅ | BLAKE3 hash wrapper |
| `TraceError` | trace | store, session | 01a | ✅ | |

---

## 6. Policy (openwand-policy)

| Interface | Owning Crate | Imported By | Wave | Status | Notes |
|-----------|-------------|-------------|------|--------|-------|
| `PolicyEngine` | policy | session | 01b | ✅ | Trait: evaluate_tool_call, filter_tools |
| `GateDecision` | policy | session | 01b | ✅ | Allow/RequireConfirmation/Block |
| `PolicyToolCall` | policy | session | 01b | ✅ | Session adapts from internal ToolCall |
| `PolicyToolDescriptor` | policy | session, tools | 01b | ✅ | Tools adapts from ToolDef |
| `PolicyToolSource` | policy | session | 01b | ✅ | Local/Mcp{server}/System — distinct from core ToolSource |
| `ToolEffect` | core | policy, tools, mcp-pool | 01a | ✅ | **PATCHED:** Moved to core. Policy imports from core. |
| `PolicyRequest` | policy | session | 01b | ✅ | |
| `PolicyContext` | policy | session | 01b | ✅ | |
| `ToolFilterRequest` | policy | session | 01b | ✅ | |
| `PolicyEvaluation` | policy | session | 01b | ✅ | |
| `GateFinding` | policy | session | 01b | ✅ | |
| `GateFindingResult` | policy | session | 01b | ✅ | |
| `PolicyError` | policy | session | 01b | ✅ | |

---

## 7. LLM (openwand-llm)

| Interface | Owning Crate | Imported By | Wave | Status | Notes |
|-----------|-------------|-------------|------|--------|-------|
| `LlmClient` | llm | session | 01b | ✅ | Trait: chat_stream, complete, health_check |
| `LlmRequest` | llm | session | 01b | ✅ | |
| `LlmTarget` | llm | session | 01b | ✅ | |
| `LlmProvider` | llm | session | 01b | ✅ | |
| `LlmMessage` | llm | session | 01b | ✅ | No System variant. Adapter in session. |
| `LlmContent` | llm | session | 01b | ✅ | |
| `LlmToolDef` | llm | session | 01b | ✅ | Naming overlap with `ToolDef` — different purpose. Session adapts via explicit adapter. No conflict. |
| `LlmToolChoice` | llm | session | 01b | ✅ | |
| `LlmDelta` | llm | session | 01b | ✅ | Text/Reasoning/ToolCallStart/ToolCallArgsDelta/ToolCallComplete/Done |
| `LlmResponse` | llm | session | 01b | ✅ | |
| `LlmStopReason` | llm | session | 01b | ✅ | Stop/ToolCall/Length/ContentFilter |
| `LlmError` | llm | session | 01b | ✅ | |
| `LlmCapabilities` | llm | session | 01b | ✅ | |
| `LlmStream` | llm | session | 01b | ✅ | Type alias for Pin<Box<Stream>> |

---

## 8. Tools (openwand-tools)

| Interface | Owning Crate | Imported By | Wave | Status | Notes |
|-----------|-------------|-------------|------|--------|-------|
| `ToolExecutor` | tools | session | 01c | ✅ | Trait: available_tools, get_descriptor, execute, refresh_mcp_tools |
| `CompositeToolExecutor` | tools | app | 01c | ✅ | Main implementation |
| `ToolDef` | tools | session, policy (via adapter) | 01c | ✅ | Different from `LlmToolDef`. Session adapts. No conflict. |
| `ToolResult` | tools | session | 01c | ✅ | Infallible. Always has output. |
| `ToolCall` | tools | — (internal) | 01c | ✅ | Crate-local DTO, not shared |
| `ToolCallContext` | tools | session | 01c | ✅ | |
| `ToolInvoker` (was ToolSource) | core | session, trace | 01a | ✅ | **PATCHED:** Renamed from ToolSource. Used in ToolEvent::Called.
| `ToolAnnotations` | tools | — | 01c | ✅ | |
| `LocalToolHandler` | tools | — | 01c | ✅ | |
| `ToolRegistry` | tools | — | 01c | ✅ | Internal |
| `ToolError` | tools | session | 01c | ✅ | |

---

## 9. MCP Pool (openwand-mcp-pool)

| Interface | Owning Crate | Imported By | Wave | Status | Notes |
|-----------|-------------|-------------|------|--------|-------|
| `McpToolGateway` | mcp-pool | tools | 01c | ✅ | Trait: execute_tool, discover_all_tools, ensure_started |
| `McpServerPool` | mcp-pool | app | 01c | ✅ | Implements McpToolGateway |
| `McpServerConfig` | mcp-pool | app, tools | 01c | ✅ | |
| `McpTransportConfig` | mcp-pool | app | 01c | ✅ | |
| `McpDiscoveredTool` | mcp-pool | tools | 01c | ✅ | Pool's own DTO — never ToolDef |
| `McpToolAnnotations` | mcp-pool | — | 01c | ✅ | |
| `McpToolResult` | mcp-pool | tools | 01c | ✅ | |
| `McpServerState` | mcp-pool | — | 01c | ✅ | |
| `McpPoolError` | mcp-pool | tools | 01c | ✅ | |

---

## 10. Session (openwand-session)

| Interface | Owning Crate | Imported By | Wave | Status | Notes |
|-----------|-------------|-------------|------|--------|-------|
| `SessionRunner` | session | app | 01d | ✅ | Owns the 10-phase loop |
| `AgentEvent` | session | app (UI) | 01d | ✅ | Transient only. Not in trace. |
| `RunHandle` | session | app | 01d | ✅ | Cancellation handle |
| `RunConfig` | session | app | 01d | ✅ | Runtime config. NOT in core. |
| `RunLifecycle` | session | app | 01d | ✅ | Runtime lifecycle. NOT in core. |
| Session Loro projection | session | — | 01d | ✅ | Internal to session |

---

## 11. Store (openwand-store)

| Interface | Owning Crate | Imported By | Wave | Status | Notes |
|-----------|-------------|-------------|------|--------|-------|
| `SqliteTraceStore<E>` | store | app, session | 01e | ✅ | Implements TraceStore<E> |
| `MemoryReadStore` | store | session | 01e | ✅ | Public read API for session |
| `MemoryProjectionStore` | store | memory pipeline | 01e | ✅ | Internal write API. Session must NOT access. |
| `WriterCommand` | store | — | 01e | ✅ | Internal: mpsc channel commands |
| `StoreError` | store | session, app | 01e | ✅ | |

---

## 12. Memory (openwand-memory)

| Interface | Owning Crate | Imported By | Wave | Status | Notes |
|-----------|-------------|-------------|------|--------|-------|
| `MemoryReadStore` (trait) | memory | session, store | 02 | ✅ | Session calls search_hybrid only |
| `MemoryProjectionStore` (trait) | memory | store | 02 | ✅ | Store implements. Session must NOT see. |
| `Extractor` | memory | — | 02 | ✅ | Internal |
| `EntityResolver` | memory | — | 02 | ✅ | Internal |
| `TemporalPolicy` | memory | — | 02 | ✅ | Internal |
| `MemoryError` | memory | session | 02 | ✅ | |

---

## Conflicts Summary

### Conflict 1: ToolEffect Ownership ✅ PATCHED

```text
Policy doc defines:  ToolEffect enum in openwand-policy
Tools/MCP doc says:  ToolEffect must live in openwand-core
Decision:            ToolEffect lives in openwand-core. Policy imports from core.
Patched:             core-crate-design.md — added ToolEffect to tool_vocab.rs
                     policy-crate-design.md — removed ToolEffect definition
```

### Conflict 2: ToolSource Name Collision ✅ PATCHED

```text
Core defines:   ToolSource = Llm | User | System | Mcp{server}
Tools defines:  ToolSource = Local | Mcp{server, remote_name}
Policy defines: PolicyToolSource = Local | Mcp{server} | System

Resolution:
  - Core:      Renamed to ToolInvoker (Llm/User/System/Mcp{server})
  - Tools:     Keeps ToolSource (Local/Mcp{server,remote_name})
  - Policy:    Keeps PolicyToolSource (Local/Mcp{server}/System)

Patched:
  core-crate-design.md — renamed ToolSource to ToolInvoker
  trace-crate-design.md — updated ToolEvent::Called to use ToolInvoker
  tools-mcppool-crate-design.md — no change needed
  policy-crate-design.md — no change needed
```

### Conflict 3: GateResultSnapshot Field Mismatch ✅ PATCHED

```text
Trace doc shows:  GateResultSnapshot { gate_kind, passed, summary }  — 3 fields
Core doc shows:   GateResultSnapshot { gate_kind, passed, risk_level, reason_code, summary } — 5 fields

Decision: Core is canonical. 5 fields.
Patched:  trace-crate-design.md — updated to 5-field version
```

### Conflict 4: TraceId vs Core ID Wrapping ✅ Resolved

```text
Core IDs:   SessionId(String) — wraps ulid string
Trace IDs:  TraceId(Ulid) — wraps ulid::Ulid directly

Problem: Inconsistent. Some use String, some use Ulid.
Decision: TraceId wraps String like all other core IDs. The Ulid crate is used
          for generation only, not as the stored type.
Patch needed: trace-crate-design.md — TraceId(pub String) not TraceId(pub Ulid)
              Or keep Ulid in trace since trace is generic and has no core dependency.
              If trace is generic (no core dep), it defines its own ID type. That's fine.
              The inconsistency is cosmetic, not semantic.

Resolution: Keep TraceId(Ulid) in trace. Core IDs use String. They're different crates
            with different purposes. Trace IDs are internal to the trace substrate.
            If session needs to reference a TraceId, it imports from trace crate.
            No conflict — just different conventions for different layers.
Status:     ✅ Resolved — no patch needed.
```

### Conflict 5: DecisionId Legacy Alias ✅ Resolved

```text
Core defines DecisionId as "legacy alias — prefer ClaimId in trace events"
Still generates it via domain_id! macro.

Decision: Remove DecisionId from core. All trace events use ClaimId.
          If memory needs an internal alias, it can define one locally.
Patch needed: core-crate-design.md — remove DecisionId from ids.rs
              trace-crate-design.md — verify no references to DecisionId
              Any doc referencing DecisionId — replace with ClaimId
```

---

## Adapter Boundaries

| From → To | Adapter | What converts | Where |
|---|---|---|---|
| Session → Policy | `PolicyToolCall` | Session's internal ToolCall → PolicyToolCall | session |
| Session → Policy | `PolicyToolDescriptor` | ToolDef → PolicyToolDescriptor | session |
| Session → LLM | `LlmMessage` | Session's Loro messages → LlmMessage | session |
| Session → LLM | `LlmToolDef` | ToolDef → LlmToolDef | session |
| Tools → MCP Pool | `McpToolGateway` | ToolExecutor dispatch → pool call | tools (CompositeToolExecutor) |
| MCP Pool → Tools | `McpDiscoveredTool → ToolDef` | Pool DTO → tools descriptor | tools |
| Store → Core | `TraceEventEnvelope for OpenWandTraceEvent` | Core methods → trace trait | store |
| Session → Store | `TraceStore<OpenWandTraceEvent>` | Generic trait → concrete type | app (wiring) |

---

## Type Ownership Summary

| Type | Owner | Note |
|------|-------|------|
| `ToolEffect` | core | ✅ Moved from policy. Locked. |
| `ToolInvoker` (was ToolSource) | core | ✅ Renamed to avoid collision. |
| `ToolSource` | tools | ✅ Kept in tools. Different meaning. |
| `PolicyToolSource` | policy | ✅ Kept in policy. |
| `ToolCall` | tools (crate-local) | ✅ Session adapts to PolicyToolCall and LlmContent. |
| `LlmToolDef` | llm | ✅ Different from ToolDef. Session adapts. |
| `ToolDef` | tools | ✅ Different from LlmToolDef. |
| `LlmMessage` | llm | ✅ Session adapts from its own message model. |
| `AgentEvent` | session | ✅ Transient. Not in any other crate. |
| `TraceEventEnvelope` | trace (trait) | ✅ Impl in store for OpenWandTraceEvent. |
| `TraceStore<E>` | trace (trait) | ✅ Impl in store. |
| `MemoryReadStore` | memory (trait) | ✅ Impl in store. Session reads only. |
| `MemoryProjectionStore` | memory (trait) | ✅ Impl in store. Session must NOT see. |
| `CancellationToken` | tokio_util | ✅ No OpenWand wrapper. |

---

## Patch List

| Doc | Patch Required | Details |
|-----|---------------|---------|
| `core-crate-design.md` | ✅ Phase 1 Patched | Added `ToolEffect` to `tool_vocab.rs`. Renamed `ToolSource` → `ToolInvoker`. Removed `DecisionId`. |
| `policy-crate-design.md` | ✅ Phase 1 Patched | Removed `ToolEffect` enum definition (imports from core). Updated dependency note. |
| `trace-crate-design.md` | ✅ Phase 1+2 Patched | P1: Renamed `ToolSource` → `ToolInvoker`. P2: Added `UserMessageInjected` variant, fixed `InferenceEvent::Called` fields, added `ToolEvent::Denied` variant, added `tool_name` to `Resumed`, fixed `TraceEventEnvelope` impl to delegate, marked `InteractionMode` as import-from-core. |
| `session-crate-design.md` | ✅ Phase 1+2 Patched | P1: Updated `ToolSource` → `ToolInvoker`. P2: Fixed `MemoryStore` → `MemoryReadStore` in both `StepContext` and `Session` struct. |
| `tools-mcppool-crate-design.md` | 🔲 No | Already correct. |
| `llm-crate-design.md` | 🔲 No | No conflicts found. |
| `store-crate-design.md` | 🔲 No | No conflicts found. Already correctly references `MemoryReadStore` + `MemoryProjectionStore`. |
| `memory-crate-design.md` | ⚠️ Wave 02 | Currently defines single flat `MemoryStore` trait. Needs split into `MemoryReadStore` + `MemoryProjectionStore` before Wave 02. Not blocking Wave 01. |
| `trust-architecture.md` | 🔲 No | No types affected. |
