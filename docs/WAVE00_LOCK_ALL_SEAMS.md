# Wave 00 — Lock All Seams

**Status:** In Progress  
**Blocks:** Wave 01 (all sub-waves)  
**Principle:** Freeze every Wave 01 seam before behavior implementation begins.

---

## Deliverables

```text
1. docs/WAVE01_INTERFACE_MATRIX.md
2. docs/WAVE01_ACCEPTANCE_TESTS.md
3. Full patched design document set
4. openwand-trace crate stub
5. openwand-store crate stub
6. Updated workspace Cargo.toml
7. Compile-only crate smoke tests
8. cargo build --workspace passes
9. cargo test --workspace runs
```

---

## Scope

```text
Wave 00 is implementation-prep plus design repair.

It freezes every Wave 01 seam before behavior implementation begins.
It patches every affected design document, not only Wave 01-blocking docs.
```

---

## Allowed Work

```text
- Cross-doc consistency audit
- Interface matrix production
- Crate stubs
- Module stubs
- Trait signatures
- DTO signatures
- Dependency declarations
- Feature declarations
- Compile-only smoke tests
- Design document corrections
- Workspace DAG verification
```

### Not Allowed Yet

```text
- SQLite backend implementation
- Real TraceStore persistence
- Real policy evaluation behavior
- Real LLM provider integration
- Real MCP lifecycle implementation
- Real local tool execution
- Session loop implementation
- Loro projection implementation
- Memory extraction implementation
```

---

## Locked Decisions

### ToolEffect

```text
Owner:       openwand-core
Imported by: openwand-policy, openwand-tools, openwand-mcp-pool
Rationale:   Avoids tools → policy dependency. Cross-cutting vocabulary.
```

### ToolCall

```text
Decision:    No universal rich ToolCall type.
Pattern:     Crate-local DTOs only. Explicit adapters at boundaries.
Rationale:   Each crate has different visibility needs. Adapters keep seams thin.
```

### TraceEventEnvelope

```text
Trait owner:            openwand-trace
Methods on enum owner:  openwand-core (event_kind, event_family, schema_version)
Impl location:          openwand-store (bridge: impl TraceEventEnvelope for OpenWandTraceEvent)
Rationale:              Core defines vocabulary. Trace defines trait. Store connects them.
```

### AgentEvent

```text
Owner:       openwand-session
Scope:       Transient UI transport only.
Restriction: Not emitted by llm/tools/policy directly. Session translates.
Rationale:   Keeps downstream crates UI-agnostic.
```

### InMemoryTraceStore

```text
Owner:       openwand-trace::testing
Visibility:  Feature-gated ("testing" feature)
Scope:       Conformance and testing only. Not for production use.
```

### SQLite TraceStore

```text
Owner:               openwand-store
Implementation wave: 01e
Scope:               trace_entry, trace_relation, trace_blob only.
                     Memory projection tables deferred.
```

### LlmMessage

```text
Owner:       openwand-llm
Adapter:     Session adapts from its internal message model to LlmMessage.
Location:    Adapter lives in openwand-session.
```

### MemoryStore Split

```text
Session may depend on MemoryReadStore.
Session must not depend on MemoryProjectionStore.
Session retrieves context and enqueues trace IDs only.
Memory projection writes are trace-backed and internal to memory/store.

Session  →  MemoryReadStore.search_hybrid()     (retrieval only)
Session  →  enqueues trace IDs for ingestion     (no direct write)
Memory pipeline  →  consumes trace entries       (extraction + temporal)
Memory pipeline  →  writes via MemoryProjectionStore
Store  →  implements both MemoryReadStore + MemoryProjectionStore
```

### CancellationToken

```text
Decision: Use tokio_util::sync::CancellationToken directly.
Rationale: No OpenWand wrapper unless a concrete need is proven.
```

### Errors

```text
Pattern:  Each crate owns its error enum.
Session:  Maps downstream errors into trace events and AgentEvent errors.
Tools:    ToolResult { is_error: true } — infallible at trait boundary.
Policy:   Fail-closed — errors become Block decisions.
LLM:      Errors via Err(LlmError), not delta variants.
```

### MCP Test Fixture

```text
Location: crates/openwand-mcp-pool/tests/fixtures/echo-server/
Type:     Minimal Rust stdio MCP server binary.
Behavior: Responds to tools/list with one tool. Echoes tools/call arguments.
CI:       Self-contained. No Node.js dependency.
```

### SQLite Concurrent Append Test

```text
Tests:    OpenWand single-writer queue invariant (mpsc::Sender<WriterCommand>).
Does NOT: Test SQLite's own locking behavior.
Verifies: No duplicate global or stream sequences under concurrent append.
```

---

## Tasks

### 00.1 Cross-Document Audit

```text
Read all design documents in dependency order:
  Spine:    core → trace → store → session → policy → llm → tools + mcp-pool → memory
  Framework: workflow → lifecycle → interaction modes → trust → three decisions

Extract every cross-crate type, trait, DTO, enum, function, and dependency.
Identify:
  - Duplicate definitions
  - Ownership conflicts
  - Dependency-cycle risks
  - Adapter boundaries
  - Wave 01 acceptance implications
```

### 00.2 Interface Matrix

```text
Create: docs/WAVE01_INTERFACE_MATRIX.md

Columns:
  - Interface
  - Owning crate
  - Imported by
  - Used in wave
  - Canonical shape
  - Adapter boundary
  - Status (confirmed / conflict / needs-patch)
  - Affected docs
```

### 00.3 Patch All Design Docs

```text
Patch every affected document:
  - core-crate-design.md
  - trace-crate-design.md
  - policy-crate-design.md
  - llm-crate-design.md
  - tools-mcppool-crate-design.md
  - session-crate-design.md
  - memory-crate-design.md
  - store-crate-design.md
  - workflow-framework-design.md
  - lifecycle-frameworks-design.md
  - interaction-modes-design.md
  - trust-architecture.md
  - three-decisions-analysis.md
```

### 00.4 Add Crate Stubs

```text
Add:
  crates/trace/   (openwand-trace)
  crates/store/   (openwand-store)

Update:
  Cargo.toml workspace members
  workspace dependencies
  feature declarations
```

### 00.5 Compile Smoke Tests

```text
Each crate gets a minimal compile test:
  openwand-core:     DTOs serialize
  openwand-trace:    TraceStore<E> trait compiles
  openwand-policy:   PolicyEngine trait compiles
  openwand-llm:      LlmClient trait compiles
  openwand-mcp-pool: McpToolGateway trait compiles
  openwand-tools:    ToolExecutor trait compiles
  openwand-session:  SessionRunner type shell compiles
  openwand-store:    OpenWandStore type shell compiles
```

### 00.6 Workspace Verification

```bash
cargo fmt --check
cargo check --workspace
cargo test --workspace
```

---

## Acceptance Criteria

```text
1.  All known cross-document inconsistencies are resolved.
2.  Every affected design document is patched.
3.  docs/WAVE01_INTERFACE_MATRIX.md exists with all entries in "confirmed" status.
4.  docs/WAVE01_ACCEPTANCE_TESTS.md exists.
5.  openwand-trace exists in the workspace.
6.  openwand-store exists in the workspace.
7.  Dependency DAG is real in Cargo, not only documented.
8.  cargo check --workspace passes.
9.  cargo test --workspace runs smoke tests.
10. Wave 01a can begin without unresolved ownership questions.
```
