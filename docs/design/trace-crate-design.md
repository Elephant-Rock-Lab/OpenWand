# OpenWand Trace Crate Design

**Date:** 2026-05-26  
**Status:** Design — locked decisions  
**Crate:** `openwand-trace`  
**Depends on:** nothing (generic over `<E>`)  
**Consumed by:** all domain crates

---

## Core Principle

> **OpenWand has one authoritative history: the append-only trace log. Domain state, memory, workflow, decisions, artifact accuracy, and user-facing explanations are projections over that history.**

Everything else is a projection, cache, index, or materialized view. The trace log is the only source of truth.

---

## 1. Locked Decisions

| # | Decision | Final |
|---|---|---|
| 1 | Source of truth | One append-only trace log |
| 2 | Event typing | Typed Rust enum, no untyped payloads |
| 3 | Trace crate | Generic over `E`, no dependency on `core` |
| 4 | Event vocabulary | `core::OpenWandTraceEvent` and family enums |
| 5 | Event payloads | Thin DTOs only — no rich domain types |
| 6 | Relation ownership | `trace` owns `TraceRelationKind` |
| 7 | Relation storage | Normalized table, not embedded JSON |
| 8 | Memory | Projection over trace + producer of derived trace events |
| 9 | Workflow | Projection / state machine over trace, no separate event store |
| 10 | Store | Implements trace/memory/projection storage, owns no domain truth |
| 11 | App | Composition root |
| 12 | Write invariant | All important mutations append trace first |
| 13 | Projections | Rebuildable, checkpointed materialized views |
| 14 | Domain events vs trace events | Merged into one event log |
| 15 | Crate split | `core` (vocabulary) → `trace` (substrate) → `memory`/`workflow` (projections) → `store` (implementations) → `app` (wiring) |

---

## 2. Architecture Overview

```
┌──────────────────────────────────────────────────────────────┐
│                      openwand-app                            │
│  wires TraceStore<OpenWandTraceEvent> + all projectors       │
└──────────────────────────┬───────────────────────────────────┘
                           │
       ┌───────────────────┼───────────────────┐
       │                   │                   │
       ▼                   ▼                   ▼
┌─────────────┐   ┌───────────────┐   ┌───────────────┐
│  session    │   │    memory     │   │   workflow    │
│  policy     │   │               │   │               │
│  tools      │   │  producers +  │   │  producers +  │
│  llm        │   │  projectors   │   │  projectors   │
│  skills     │   │               │   │               │
│  goals      │   └───────┬───────┘   └───────┬───────┘
│  content    │           │                   │
│  mcp-pool   │           │                   │
└──────┬──────┘           │                   │
       │                  │                   │
       │    ┌─────────────┘                   │
       │    │                                 │
       ▼    ▼                                 ▼
┌──────────────────────────────────────────────────────────────┐
│                      openwand-trace                          │
│  generic append-only substrate, TraceStore<E>, relations     │
└──────────────────────────┬───────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│                      openwand-store                          │
│  TraceStore<OpenWandTraceEvent> + MemoryStore backends       │
│  SQLite / CozoDB / SurrealDB implementations                │
└──────────────────────────────────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────┐
│                      openwand-core                           │
│  OpenWandTraceEvent enum, domain IDs, DTOs                   │
│  no dependency on trace, memory, session, policy, store      │
└──────────────────────────────────────────────────────────────┘
```

### Dependency Rules

| Crate | Depends on | Does NOT depend on |
|---|---|---|
| `core` | `serde`, `chrono`, `uuid` | trace, memory, session, policy, store |
| `trace` | `serde`, `chrono` | core (generic `<E>`) |
| `memory` | core, trace | store, session, policy |
| `store` | core, trace, memory | session, policy, tools |
| `session` | core, trace, llm, tools, policy, memory | store |
| `policy` | core, trace | memory, session |
| `tools` | core, trace | memory, session, policy |
| `workflow` | core, trace | store, session |
| `app` | everything | — (composition root) |

**No cycles.** `core` never imports `trace`. `trace` never imports `core`. They meet at `TraceStore<OpenWandTraceEvent>`.

---

## 3. Core Types

### 3.1 TraceId

```rust
/// Unique identifier for a trace entry.
/// Assigned by the trace store on append — never by callers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TraceId(pub Ulid);
```

### 3.2 TraceEntry

```rust
/// A single append-only record in the trace log.
/// Generic over event type E — completely independent of domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry<E> {
    /// Unique ID, assigned by store
    pub id: TraceId,

    /// Which stream this entry belongs to
    pub stream_id: TraceStreamId,

    /// Monotonic sequence within the stream
    pub stream_sequence: u64,

    /// Global monotonic sequence across all streams
    pub global_sequence: u64,

    /// When this event occurred (wall clock)
    pub occurred_at: DateTime<Utc>,

    /// Who or what caused this event
    pub actor: Actor,

    /// The typed event payload
    pub event: E,

    /// Stable event kind name (independent of Rust enum layout)
    pub event_kind: String,

    /// Schema version of the event payload
    pub event_schema_version: u16,

    /// Schema version of the trace envelope
    pub trace_schema_version: u16,

    /// Hash of the previous entry in this stream (integrity chain)
    pub prev_hash: Option<EntryHash>,

    /// Hash of this entry (integrity check)
    pub entry_hash: EntryHash,
}
```

### 3.3 EntryHash

```rust
/// BLAKE3 hash of entry content for integrity verification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EntryHash(pub String);
```

### 3.4 Actor

```rust
/// Who or what caused a trace event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Actor {
    User,
    Llm {
        model: String,
        provider: String,
    },
    System {
        component: String,
    },
    MemoryPipeline,
    WorkflowEngine,
    PolicyEngine,
}
```

### 3.5 TraceStreamId

```rust
/// Identifies a stream within the trace log.
/// Hybrid model: global ordering + per-stream local ordering.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TraceStreamId {
    pub scope: TraceStreamScope,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TraceStreamScope {
    /// Global audit trail
    Global,
    /// Per-session agent loop
    Session,
    /// Per-claim lifecycle
    Claim,
    /// Per-entity history
    Entity,
    /// Per-workflow instance
    Workflow,
    /// Per-artifact history
    Artifact,
    /// Per-tool-call chain
    ToolCall,
    /// Memory pipeline extraction run
    MemoryPipelineRun,
}
```

### 3.6 TraceRelation

```rust
/// A typed causal/provenance edge between trace entries.
/// Stored in a separate normalized table — never embedded in the entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceRelation {
    pub from: TraceId,
    pub to: TraceId,
    pub kind: TraceRelationKind,
    pub created_at: DateTime<Utc>,
}
```

### 3.7 TraceRelationKind

```rust
/// Typed causal relationships between trace entries.
/// Lives in `openwand-trace` — it is graph substrate, not domain vocabulary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TraceRelationKind {
    /// Event would not have happened without the prior event
    CausedBy,

    /// New event was inferred from prior evidence (LLM extraction, etc.)
    DerivedFrom,

    /// Prior event/evidence supports a claim
    Verifies,

    /// New claim shows old claim was wrong
    Invalidates,

    /// New claim replaces old claim because context changed
    Supersedes,

    /// New claim narrows or clarifies old claim
    Refines,

    /// Both claims cannot be simultaneously true (unresolved contradiction)
    ConflictsWith,

    /// Artifact realizes a decision or claim
    Implements,

    /// Event explicitly undoes a previous event
    Reverts,

    /// Weak link — avoid unless no stronger relation applies
    References,
}
```

### 3.8 Relation Semantics Guide

Every producer must follow these rules when creating relations:

| Relation | Use only when | Example |
|---|---|---|
| `CausedBy` | Event would not have happened without the prior event | Tool call caused by LLM inference |
| `DerivedFrom` | New event was inferred from prior evidence | Fact extracted from episode |
| `Verifies` | Prior evidence supports a claim | Test result verifies architecture decision |
| `Invalidates` | New claim shows old claim was wrong | User correction invalidates LLM hallucination |
| `Supersedes` | Old claim was acceptable but is now outdated | "Use SurrealDB" supersedes "Use CozoDB" |
| `Refines` | Old claim was incomplete or imprecise | "Module uses RMCP v1.6" refines "Module uses RMCP" |
| `ConflictsWith` | Both claims cannot be simultaneously true | Two sessions produced contradictory preferences |
| `Implements` | Artifact realizes a decision or claim | File written implements architecture decision |
| `Reverts` | Event explicitly undoes a previous event | User reverts a commit |
| `References` | Weak link — avoid unless no stronger relation applies | Cross-session mention |

**Critical distinction:**
- `Invalidates` = old claim was **wrong**
- `Supersedes` = old claim was once acceptable but is now **outdated**
- `Refines` = old claim was **incomplete** or imprecise
- `ConflictsWith` = **unresolved** contradiction

---

## 4. Trace Substrate API

### 4.1 TraceStore

```rust
/// Generic append-only trace store.
/// Implemented by `openwand-store` against chosen backend.
/// E = concrete event type (e.g., OpenWandTraceEvent).
#[async_trait]
pub trait TraceStore<E>: Send + Sync {
    /// Append a new entry. Store assigns id, timestamps, sequences, hashes.
    async fn append(&self, command: AppendTraceEntry<E>) -> Result<TraceId, TraceError>;

    /// Append + synchronously update named projections.
    /// For local-first: transactional append-then-project.
    async fn append_and_project(
        &self,
        command: AppendTraceEntry<E>,
        projectors: &[ProjectorName],
    ) -> Result<TraceId, TraceError>;

    /// Get a single entry by ID.
    async fn get(&self, id: TraceId) -> Result<Option<TraceEntry<E>>, TraceError>;

    /// Get an entry with its relations.
    async fn get_with_relations(
        &self,
        id: TraceId,
    ) -> Result<Option<TraceEntryWithRelations<E>>, TraceError>;

    /// Scan entries matching a query.
    async fn scan(&self, query: TraceQuery) -> Result<TracePage<E>, TraceError>;

    /// Scan relations matching a query.
    async fn scan_relations(
        &self,
        query: RelationQuery,
    ) -> Result<Vec<TraceRelation>, TraceError>;

    /// Get current global sequence number (for projection lag visibility).
    async fn current_global_sequence(&self) -> Result<u64, TraceError>;

    /// Get current stream sequence for a given stream.
    async fn current_stream_sequence(&self, stream_id: &TraceStreamId)
        -> Result<u64, TraceError>;

    /// Initialize storage (create tables, indexes).
    async fn initialize(&self) -> Result<(), TraceError>;

    /// Run a projection rebuild from a checkpoint.
    async fn rebuild_projection(
        &self,
        projector_name: &str,
        from: Option<TraceId>,
    ) -> Result<(), TraceError>;
}
```

### 4.2 AppendTraceEntry

```rust
/// Command to append a trace entry.
/// Callers construct this. The store assigns the rest.
#[derive(Debug, Clone)]
pub struct AppendTraceEntry<E> {
    /// Who caused this event
    pub actor: Actor,

    /// What happened
    pub event: E,

    /// How this event relates to other events
    pub relations: Vec<TraceRelationDraft>,

    /// Which stream this entry belongs to
    pub stream_id: TraceStreamId,

    /// Prevents duplicate appends on retry
    pub idempotency_key: Option<IdempotencyKey>,
}
```

### 4.3 TraceRelationDraft

```rust
/// A relation to be created alongside an entry.
/// `from` is implicitly the new entry's ID (assigned by store).
#[derive(Debug, Clone)]
pub struct TraceRelationDraft {
    pub to: TraceId,
    pub kind: TraceRelationKind,
}
```

### 4.4 IdempotencyKey

```rust
/// Prevents duplicate appends on retry.
/// If an entry with this key already exists, append returns the existing ID.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct IdempotencyKey(pub String);
```

### 4.5 TraceQuery

```rust
/// Query parameters for scanning the trace log.
#[derive(Debug, Clone, Default)]
pub struct TraceQuery {
    pub stream_id: Option<TraceStreamId>,
    pub event_kind: Option<String>,
    pub actor: Option<ActorFilter>,
    pub from_sequence: Option<u64>,
    pub to_sequence: Option<u64>,
    pub from_timestamp: Option<DateTime<Utc>>,
    pub to_timestamp: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub cursor: Option<TraceId>,
}

#[derive(Debug, Clone)]
pub enum ActorFilter {
    UserOnly,
    LlmOnly,
    SystemOnly,
    Component(String),
}

#[derive(Debug, Clone)]
pub struct TracePage<E> {
    pub entries: Vec<TraceEntry<E>>,
    pub next_cursor: Option<TraceId>,
    pub total: usize,
}

#[derive(Debug, Clone)]
pub struct TraceEntryWithRelations<E> {
    pub entry: TraceEntry<E>,
    pub relations: Vec<TraceRelation>,
}
```

### 4.6 RelationQuery

```rust
/// Query parameters for scanning relations.
#[derive(Debug, Clone, Default)]
pub struct RelationQuery {
    /// Filter by from entry
    pub from: Option<TraceId>,

    /// Filter by to entry
    pub to: Option<TraceId>,

    /// Filter by relation kind
    pub kind: Option<TraceRelationKind>,

    /// Traverse depth (default: 1, max: configurable)
    pub depth: Option<usize>,

    pub limit: Option<usize>,
}
```

---

## 5. TraceEventEnvelope

The versioning contract. Every event type must implement this.

```rust
/// Versioning contract for trace events.
/// Ensures persisted events have stable kind names and schema versions
/// independent of Rust enum layout.
pub trait TraceEventEnvelope {
    /// Stable event kind name. Used for storage and queries.
    /// Must never change once an event is persisted.
    fn event_kind(&self) -> &'static str;

    /// Schema version of this event's payload.
    /// Increment when fields are added or semantics change.
    /// Old versions must remain readable.
    fn schema_version(&self) -> u16;
}
```

### Implementation for OpenWandTraceEvent

```rust
impl TraceEventEnvelope for OpenWandTraceEvent {
    fn event_kind(&self) -> &'static str {
        // Delegate to core's per-family event_kind() methods
        // which return dotted names like "tool.called", "memory.fact_accepted"
        match self {
            Self::Session(e) => e.event_kind(),
            Self::Inference(e) => e.event_kind(),
            Self::Gate(e) => e.event_kind(),
            Self::Tool(e) => e.event_kind(),
            Self::File(e) => e.event_kind(),
            Self::Memory(e) => e.event_kind(),
            Self::Mode(e) => e.event_kind(),
            Self::Workflow(e) => e.event_kind(),
            Self::Artifact(e) => e.event_kind(),
        }
    }

    fn schema_version(&self) -> u16 {
        1 // will be incremented per event family as they evolve
    }
}
```

### Event Versioning Rules

1. **Fields are never removed or renamed.** Once a field appears in an event, it persists forever.
2. **New fields have defaults.** When adding a field, old events deserialize with the default value.
3. **Breaking changes require a new event kind.** If semantics change fundamentally, create a new variant (e.g., `FactExtractedV2`).
4. **`schema_version` is per-event-kind, not global.** Each event family versions independently.
5. **Rust refactors do not change storage.** The `event_kind` string is the stable identifier, not the Rust enum variant name.

---

## 6. Event Vocabulary

Defined in `openwand-core`. All payloads are thin DTOs — no rich domain types.

### 6.1 OpenWandTraceEvent

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpenWandTraceEvent {
    Session(SessionEvent),
    Inference(InferenceEvent),
    Gate(GateEvent),
    Tool(ToolEvent),
    File(FileEvent),
    Memory(MemoryEvent),
    Mode(ModeEvent),
    Workflow(WorkflowEvent),
    Artifact(ArtifactEvent),
}
```

### 6.2 SessionEvent

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionEvent {
    Started {
        session_id: SessionId,
        mode: InteractionMode,
    },
    Ended {
        session_id: SessionId,
        reason: SessionEndReason,
        total_steps: u64,
        total_tokens: TokenUsageSnapshot,
    },
    StepStarted {
        step: u64,
    },
    StepCompleted {
        step: u64,
        stop_reason: String,
    },
    UserMessageInjected {
        text: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionEndReason {
    Natural,
    UserStopped,
    TokenBudgetExhausted,
    MaxStepsReached,
    Error,
    Cancelled,
}
```

### 6.3 InferenceEvent

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InferenceEvent {
    Called {
        model: String,
        provider: String,
        prompt_hash: String,
        thinking_budget: Option<ThinkingBudgetSnapshot>,
        prompt_assembly: PromptAssemblySnapshot,
    },
    Completed {
        model: String,
        tokens: TokenUsageSnapshot,
        stop_reason: String,
        tool_call_count: u8,
    },
    Failed {
        model: String,
        error: String,
        retry_count: u8,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageSnapshot {
    pub input: u64,
    pub output: u64,
    pub reasoning: Option<u64>,
    pub cache_read: Option<u64>,
    pub cache_write: Option<u64>,
}
```

### 6.4 GateEvent

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateEvent {
    Evaluated {
        gate_id: String,
        gate_kind: String,
        passed: bool,
        risk_level: Option<RiskLevelSnapshot>,
        reason_code: Option<String>,
        summary: String,
    },
    BatchCompleted {
        total: u8,
        passed: u8,
        failed: u8,
        overall_risk: RiskLevelSnapshot,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevelSnapshot {
    Low,
    Medium,
    High,
    Critical,
}
```

### 6.5 ToolEvent

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolEvent {
    Called {
        tool_name: String,
        tool_call_id: ToolCallId,
        args_hash: String,
        invoker: ToolInvoker,
    },
    Completed {
        tool_call_id: ToolCallId,
        tool_name: String,
        status: ToolResultStatus,
        result_summary: String,
        duration_ms: u64,
    },
    Failed {
        tool_call_id: ToolCallId,
        tool_name: String,
        error: String,
    },
    Suspended {
        tool_call_id: ToolCallId,
        tool_name: String,
        reason: String,
    },
    Resumed {
        tool_call_id: ToolCallId,
        tool_name: String,
        resolution: String,
    },
    Denied {
        tool_call_id: ToolCallId,
        tool_name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolInvoker {
    Llm,
    User,
    System,
    Mcp { server: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolResultStatus {
    Success,
    Error,
    Partial,
    Pending,
}
```

### 6.6 FileEvent

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileEvent {
    Read {
        path: String,
        bytes: Option<u64>,
    },
    Written {
        path: String,
        diff_hash: String,
        lines_added: u32,
        lines_removed: u32,
    },
    Deleted {
        path: String,
    },
}
```

### 6.7 MemoryEvent

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryEvent {
    EpisodeRecorded {
        episode_id: EpisodeId,
        episode_kind: String,
        text_hash: String,
    },
    EntityCreated {
        entity_id: EntityId,
        kind: String,
        name: String,
        canonical_key: String,
    },
    EntityMerged {
        survivor_id: EntityId,
        absorbed_id: EntityId,
        reason: String,
    },
    EntitySummaryUpdated {
        entity_id: EntityId,
        summary_hash: String,
    },
    FactExtracted {
        claim_id: ClaimId,
        statement: String,
        confidence: f64,
        predicate: String,
    },
    FactAccepted {
        claim_id: ClaimId,
        gate_summary: Vec<GateResultSnapshot>,
    },
    FactRejected {
        claim_id: ClaimId,
        reason: String,
    },
    FactInvalidated {
        claim_id: ClaimId,
        replaced_by: Option<ClaimId>,
        reason: Option<String>,
    },
    FactRefined {
        claim_id: ClaimId,
        new_confidence: f64,
        new_statement_hash: String,
    },
    DecisionExtracted {
        claim_id: ClaimId,
        title: String,
        chosen_option: String,
        rejected_count: u8,
    },
    DecisionAccepted {
        claim_id: ClaimId,
    },
    DecisionSuperseded {
        old_claim_id: ClaimId,
        new_claim_id: ClaimId,
    },
    ChunkCreated {
        chunk_id: String,
        source_kind: String,
        source_id: String,
    },
    ChunkUpdated {
        chunk_id: String,
        embedding_model: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResultSnapshot {
    pub gate_kind: String,
    pub passed: bool,
    pub risk_level: Option<RiskLevelSnapshot>,
    pub reason_code: Option<String>,
    pub summary: String,
}
```

### 6.8 ModeEvent

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModeEvent {
    Changed {
        from: InteractionMode,
        to: InteractionMode,
        trigger: String,
        accuracy_check: Option<AccuracyCheckSnapshot>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// InteractionMode is defined in openwand-core::mode.
// Shown here for reference only. The canonical definition is in core.
// InteractionMode { Direct, Conversational, AutoRouting, Custom { name } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccuracyCheckSnapshot {
    pub artifact: String,
    pub commit_hash: String,
    pub file_coverage: f64,
    pub stale: bool,
}
```

### 6.9 WorkflowEvent

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowEvent {
    StateChanged {
        from_state: String,
        to_state: String,
        mod_id: Option<String>,
    },
    GatePassed {
        gate_name: String,
        mod_id: String,
    },
    GateFailed {
        gate_name: String,
        mod_id: String,
        reason: String,
    },
    ActionExecuted {
        action_name: String,
        mod_id: String,
        success: bool,
        duration_ms: u64,
    },
    ModStarted {
        mod_id: String,
        mod_name: String,
    },
    ModCompleted {
        mod_id: String,
        mod_name: String,
        outcome: String,
    },
}
```

### 6.10 ArtifactEvent

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactEvent {
    Generated {
        paths: Vec<String>,
        artifact_kind: String,
        accuracy: AccuracyRecordSnapshot,
    },
    Updated {
        paths: Vec<String>,
        commit_hash: Option<String>,
    },
    Validated {
        paths: Vec<String>,
        passed: bool,
        issues: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccuracyRecordSnapshot {
    pub commit_hash: Option<String>,
    pub file_coverage: f64,
    pub sensitivity: String,
}
```

### 6.11 Domain IDs

All defined in `openwand-core`:

```rust
macro_rules! domain_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
        pub struct $name(pub String);
    };
}

domain_id!(SessionId);
domain_id!(EpisodeId);
domain_id!(EntityId);
domain_id!(ClaimId);
domain_id!(DecisionId);
domain_id!(ArtifactId);
domain_id!(ToolCallId);
```

---

## 7. Projection Model

### 7.1 TraceProjector

```rust
/// A projection consumes trace entries and updates a materialized view.
/// Generic over event type E.
#[async_trait]
pub trait TraceProjector<E>: Send + Sync {
    type Error: std::error::Error + Send + Sync;

    /// Name of this projector (for checkpointing).
    fn name(&self) -> &'static str;

    /// Whether this projector cares about the given event.
    fn applies_to(&self, event: &E) -> bool;

    /// Apply a trace entry to the projection.
    /// Called after the entry is appended to the trace log.
    async fn apply(
        &mut self,
        entry: &TraceEntry<E>,
        relations: &[TraceRelation],
    ) -> Result<(), Self::Error>;
}

/// Alias for convenience.
pub type ProjectorName = &'static str;
```

### 7.2 Projection Checkpoint

```rust
/// Tracks how far a projector has processed.
/// Stored in a separate table — the projector reads this on startup
/// and resumes from the checkpoint.
pub struct ProjectionCheckpoint {
    pub projector_name: String,
    pub last_global_sequence: u64,
    pub last_trace_id: Option<TraceId>,
    pub updated_at: DateTime<Utc>,
    pub error_count: u32,
    pub last_error: Option<String>,
}
```

### 7.3 Registered Projectors

| Projector | Consumes | Produces |
|---|---|---|
| `claim_projector` | `MemoryEvent::*` | `claim_projection` table rows |
| `entity_projector` | `MemoryEvent::Entity*` | `entity_projection` table rows |
| `episode_projector` | `MemoryEvent::EpisodeRecorded` | `episode` table rows |
| `chunk_projector` | `MemoryEvent::Chunk*` | `memory_chunk` table rows |
| `provenance_projector` | All events + relations | Provenance view (derived edges) |
| `workflow_state_projector` | `WorkflowEvent::*` | `workflow_state` table rows |
| `mode_history_projector` | `ModeEvent::Changed` | `mode_history` table rows |
| `gate_history_projector` | `GateEvent::*` | `gate_history` table rows |
| `artifact_projector` | `ArtifactEvent::*` | `artifact` table rows |

### 7.4 Projection Rebuild

Any projection can be rebuilt from the trace log:

```text
1. Drop projection table
2. Reset checkpoint for the projector
3. Replay trace entries from sequence 0
4. Projector applies each entry
5. Checkpoint advances
```

This is the guarantee: **projections are never authoritative.** They can always be rebuilt.

### 7.5 Projection Lag Visibility

```sql
SELECT
    p.projector_name,
    p.last_global_sequence,
    t.max_sequence AS current_sequence,
    t.max_sequence - p.last_global_sequence AS lag,
    p.updated_at
FROM projection_checkpoint p
CROSS JOIN (SELECT MAX(global_sequence) AS max_sequence FROM trace_entry) t
ORDER BY lag DESC;
```

---

## 8. Storage Schema

### 8.1 trace_entry

```sql
CREATE TABLE trace_entry (
    id                  TEXT PRIMARY KEY,       -- ULID
    stream_scope        TEXT NOT NULL,           -- Global, Session, Claim, etc.
    stream_id           TEXT NOT NULL,           -- stream-specific ID
    stream_sequence     BIGINT NOT NULL,         -- monotonic within stream
    global_sequence     BIGINT NOT NULL,         -- monotonic globally
    occurred_at         TIMESTAMP NOT NULL,
    actor_kind          TEXT NOT NULL,            -- User, Llm, System, etc.
    actor_payload       JSONB NOT NULL,           -- model, provider, component, etc.
    event_kind          TEXT NOT NULL,            -- session, inference, gate, etc.
    event_payload       JSONB NOT NULL,           -- typed event data
    event_schema_version SMALLINT NOT NULL,
    trace_schema_version SMALLINT NOT NULL,
    prev_hash           TEXT,                     -- hash of previous entry in stream
    entry_hash          TEXT NOT NULL,            -- hash of this entry
    idempotency_key     TEXT,                     -- optional dedup key
    created_at          TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Total ordering
CREATE UNIQUE INDEX idx_trace_entry_global_seq
    ON trace_entry(global_sequence);

-- Stream-local ordering
CREATE UNIQUE INDEX idx_trace_entry_stream_seq
    ON trace_entry(stream_scope, stream_id, stream_sequence);

-- Event kind filtering
CREATE INDEX idx_trace_entry_event_kind
    ON trace_entry(event_kind);

-- Time-range queries
CREATE INDEX idx_trace_entry_occurred_at
    ON trace_entry(occurred_at);

-- Idempotency lookup
CREATE UNIQUE INDEX idx_trace_entry_idempotency
    ON trace_entry(idempotency_key)
    WHERE idempotency_key IS NOT NULL;
```

### 8.2 trace_relation

```sql
CREATE TABLE trace_relation (
    from_trace_id   TEXT NOT NULL,
    to_trace_id     TEXT NOT NULL,
    kind            TEXT NOT NULL,
    created_at      TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (from_trace_id, to_trace_id, kind)
);

-- Forward traversal: "what did this event cause?"
CREATE INDEX idx_trace_relation_from_kind
    ON trace_relation(from_trace_id, kind);

-- Backward traversal: "what caused this event?"
CREATE INDEX idx_trace_relation_to_kind
    ON trace_relation(to_trace_id, kind);

-- Kind filtering: "show all Invalidates relations"
CREATE INDEX idx_trace_relation_kind
    ON trace_relation(kind);
```

### 8.3 projection_checkpoint

```sql
CREATE TABLE projection_checkpoint (
    projector_name          TEXT PRIMARY KEY,
    last_global_sequence    BIGINT NOT NULL,
    last_trace_id           TEXT,
    error_count             INT NOT NULL DEFAULT 0,
    last_error              TEXT,
    updated_at              TIMESTAMP NOT NULL DEFAULT NOW()
);
```

### 8.4 claim_projection (owned by `openwand-memory`)

```sql
CREATE TABLE claim_projection (
    claim_id        TEXT PRIMARY KEY,
    kind            TEXT NOT NULL,            -- Fact, Decision, Preference, etc.
    predicate       TEXT,                     -- for Fact claims
    statement       TEXT NOT NULL,
    status          TEXT NOT NULL,            -- Active, Superseded, Invalidated, Reverted
    confidence      DOUBLE PRECISION NOT NULL,
    scope           TEXT,                     -- Global, Project, Session
    subject_entity_id  TEXT,                  -- for fact claims
    object_entity_id   TEXT,                  -- for fact claims
    created_by      TEXT NOT NULL,            -- TraceId that created this claim
    updated_by      TEXT NOT NULL,            -- TraceId that last updated this claim
    valid_from      TIMESTAMP,
    valid_to        TIMESTAMP,
    created_at      TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_claim_status_kind ON claim_projection(status, kind);
CREATE INDEX idx_claim_subject ON claim_projection(subject_entity_id);
CREATE INDEX idx_claim_object ON claim_projection(object_entity_id);
CREATE INDEX idx_claim_validity ON claim_projection(valid_from, valid_to);
```

---

## 9. Write Invariant

### 9.1 The Rule

> **No important state mutates without appending a trace entry first.**

### 9.2 Enforcement Pattern

```
Domain crate wants to change state
  │
  ▼
Construct AppendTraceEntry
  │
  ▼
TraceStore::append (or append_and_project)
  │
  ├─ Store assigns: TraceId, timestamps, sequences, hashes
  ├─ Store persists: trace_entry row + trace_relation rows
  └─ Store triggers: applicable projectors
  │
  ▼
Projectors update materialized views
  │
  ▼
Domain crate receives TraceId (confirmation of persistence)
```

Domain crates **never** write to projection tables directly. They go through `TraceStore::append`. Projections update only through the projector mechanism.

### 9.3 The Transactional Guarantee

For local-first embedded use:

```rust
async fn append_and_project(
    &self,
    command: AppendTraceEntry<E>,
    projectors: &[ProjectorName],
) -> Result<TraceId, TraceError> {
    // 1. Append trace entry (source of truth)
    let entry = self.persist_entry(command)?;

    // 2. Persist relations
    self.persist_relations(&entry, &command.relations)?;

    // 3. Update projections
    for name in projectors {
        if let Some(projector) = self.projectors.get(name) {
            match projector.apply(&entry, &relations).await {
                Ok(()) => self.advance_checkpoint(name, entry.global_sequence)?,
                Err(e) => {
                    // Entry persists. Projection checkpoint does not advance.
                    // Recovery will replay from last checkpoint.
                    self.record_projection_error(name, &e)?;
                }
            }
        }
    }

    Ok(entry.id)
}
```

If projection update fails, the trace entry remains in the log. The checkpoint doesn't advance. On next startup, the projector replays from the last checkpoint and catches up.

### 9.4 Enforcement Granularity

**Mandatory trace (every occurrence must be traced):**

| Mutation | Event type |
|---|---|
| LLM inference | `InferenceEvent::Called` + `InferenceEvent::Completed` |
| Tool call | `ToolEvent::Called` + `ToolEvent::Completed` |
| Policy gate evaluation | `GateEvent::Evaluated` |
| File write | `FileEvent::Written` |
| Artifact generation | `ArtifactEvent::Generated` |
| Mode change | `ModeEvent::Changed` |
| Workflow state change | `WorkflowEvent::StateChanged` |
| Memory claim change | `MemoryEvent::Fact*` + `MemoryEvent::Decision*` |
| Entity creation/merge | `MemoryEvent::Entity*` |
| Preference change | `MemoryEvent::FactAccepted` (with preference claim kind) |
| Decision acceptance/rejection | `MemoryEvent::DecisionAccepted` |

**Optional trace (traced when relevant, not mandatory):**

| Mutation | When to trace |
|---|---|
| File read | When used as evidence for a claim |
| Cache hit | Only if debugging retrieval quality |
| UI-only events | Never |
| Token streaming chunks | Never (aggregate in `InferenceEvent::Completed`) |
| Internal retry attempts | Only if retry succeeds after failure |
| Temporary scoring/ranking | Never (redundant with final decision) |

---

## 10. Crate Layout

### 10.1 openwand-trace

```
openwand-trace/
  Cargo.toml
  src/
    lib.rs                    — public API, re-exports
    types.rs                  — TraceId, TraceStreamId, TraceStreamScope, Actor, EntryHash
    entry.rs                  — TraceEntry<E>, TraceEntryWithRelations<E>
    relation.rs               — TraceRelation, TraceRelationKind, TraceRelationDraft
    append.rs                 — AppendTraceEntry<E>, IdempotencyKey
    query.rs                  — TraceQuery, RelationQuery, TracePage, ActorFilter
    store.rs                  — TraceStore<E> trait
    projector.rs              — TraceProjector<E> trait, ProjectionCheckpoint
    envelope.rs               — TraceEventEnvelope trait
    error.rs                  — TraceError
    cursor.rs                 — replay cursor APIs
    integrity.rs              — hash chaining, verification
```

### 10.2 Cargo.toml

```toml
[package]
name = "openwand-trace"
version = "0.1.0"
edition = "2024"

[dependencies]
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
ulid = { version = "1", features = ["serde"] }
blake3 = "1"
thiserror = "2"
tracing = "0.1"

# No dependency on openwand-core — this crate is generic over <E>
```

---

## 11. Implementation Risks & Guards

| Risk | Guard | Enforcement |
|---|---|---|
| `core` becomes a god crate | Litmus: can `core` compile without importing any domain crate? Event payloads are DTOs only | Code review + CI check |
| Trace append APIs are bypassed | Projections update only through projectors. Domain crates never write to projection tables directly | Architectural rule + no direct store access |
| Projections gain source-of-truth state | Projections are checkpointed. Any projection can be rebuilt from trace. `projection_checkpoint` table tracks progress | Rebuild API + periodic integrity check |
| Event versioning is afterthought | `TraceEventEnvelope` trait with `event_kind()` and `schema_version()`. Every persisted event has stable kind + version | Trait requirement on all events |
| Relation semantics become inconsistent | Relation discipline table defines "use only when" rules for each `TraceRelationKind` | Documentation + code review |

---

## 12. How This Collapses the Six Original Systems

| Original system | New role |
|---|---|
| Decision Ledger | `MemoryEvent::FactAccepted/Rejected` entries + `GateEvent::Evaluated` entries, projected by `gate_history_projector` |
| Mode Ledger | `ModeEvent::Changed` entries, projected by `mode_history_projector` |
| Episode provenance | `DerivedFrom` relations from `FactExtracted` entries to `EpisodeRecorded` entries |
| Fact supersession chain | `Invalidates`/`Supersedes`/`Refines` relations between claim-related entries |
| Decision supersession | Same relation model, `MemoryEvent::DecisionSuperseded` entries |
| Artifact accuracy | `ArtifactEvent::Generated` entries + `Implements` relations to decision entries |
| MemoryEvent audit | All `MemoryEvent::*` entries in the trace log — the audit IS the trace |

Six systems → one log + typed edges + projections.

---

## 13. End-to-End Traceability Example

User asks: *"Why does `session.rs` use Loro?"*

The system traces backward:

```
Query: claim_projection WHERE statement mentions "Loro" AND status = "Active"
  → ClaimId("c_loro_sessions")
  → TraceEntry { MemoryEvent::FactAccepted { claim_id: "c_loro_sessions" } }

  ← CausedBy → TraceEntry { MemoryEvent::FactExtracted { claim_id: "c_loro_sessions" } }
  ← DerivedFrom → TraceEntry { MemoryEvent::EpisodeRecorded { episode_id: "ep_design_session" } }
  ← CausedBy → TraceEntry { SessionEvent::StepCompleted { step: 14 } }
  ← CausedBy → TraceEntry { InferenceEvent::Completed { model: "gpt-4o" } }
  ← CausedBy → TraceEntry { InferenceEvent::Called { prompt_hash: "..." } }
  ← CausedBy → TraceEntry { SessionEvent::Started { session_id: "s_design_1" } }

  Also:
  ← Verifies → TraceEntry { GateEvent::Evaluated { gate: "schema", passed: true } }
  ← Implements → TraceEntry { ArtifactEvent::Generated { paths: ["crates/session/src/lib.rs"] } }
  ← DerivedFrom → TraceEntry { FileEvent::Written { path: "crates/session/Cargo.toml" } }
```

The full chain:
1. Session `s_design_1` started in Conversational mode
2. User asked about session storage
3. LLM (gpt-4o) inferred that Loro CRDT fits the requirements
4. Fact was extracted with confidence 0.89
5. Schema gate passed
6. Fact was accepted
7. File was written implementing the decision
8. Artifact was generated

One chain. From current state back to original intent. Every step has an ID, a timestamp, an actor, and typed edges to its causes.
