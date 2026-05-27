# OpenWand Core Crate Design

**Date:** 2026-05-26  
**Status:** Design — locked  
**Crate:** `openwand-core`  
**Depends on:** `serde`, `serde_json`, `chrono`, `ulid`  
**Blocks:** Every other crate

---

## Boundary Rules

### Contains

- 14 domain IDs
- `OpenWandTraceEvent` top-level enum
- 9 event family enums with thin DTO payloads
- Snapshot DTOs
- Shared vocabulary enums
- `event_family()`, `event_kind()`, `schema_version()` methods

### Does NOT Contain

- Errors (each crate owns its own)
- Trace traits (`TraceStore<E>`, `TraceEventEnvelope`)
- Store traits (`MemoryStore`, `TraceStore`)
- Memory traits (`Extractor`, `EntityResolver`, `TemporalPolicy`)
- Loro (`loro`)
- `AgentEvent` (transient, lives in `openwand-session`)
- `RunConfig`, `RunLifecycle` (runtime, lives in `openwand-session`)
- Rich domain structs (`Claim`, `Entity`, `Fact`, `Decision`, `MemoryChunk`)
- Policy evaluation logic
- Tool execution logic
- LLM client logic
- Async runtime (`tokio`)
- Hash computation (`blake3`)

### Dependency Contract

```
openwand-core compiles without importing:
  openwand-trace, openwand-memory, openwand-session,
  openwand-policy, openwand-tools, openwand-store,
  loro, rig, rmcp, tokio, blake3, uuid, thiserror
```

Only: `serde`, `serde_json`, `chrono`, `ulid`.

---

## Serialization Contract

### Serde Tagging

```rust
#[serde(tag = "family", content = "payload", rename_all = "snake_case")]
pub enum OpenWandTraceEvent { ... }
```

Stable serialized names, independent of Rust refactoring.

### Event Kind Strings

Two granularities:

| Method | Returns | Example | Use case |
|---|---|---|---|
| `event_family()` | Family name | `"tool"` | Broad filtering |
| `event_kind()` | Dotted name | `"tool.called"` | Precise indexed queries |

Each family enum implements its own `event_kind()`:

```rust
impl ToolEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Called { .. } => "tool.called",
            Self::Completed { .. } => "tool.completed",
            Self::Failed { .. } => "tool.failed",
            Self::Suspended { .. } => "tool.suspended",
            Self::Resumed { .. } => "tool.resumed",
            Self::Denied { .. } => "tool.denied",
        }
    }
}
```

The top-level enum delegates:

```rust
impl OpenWandTraceEvent {
    pub fn event_kind(&self) -> &'static str {
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
}
```

### Versioning Rules

1. Event kind strings are **permanent**. Once `"tool.called"` is persisted, it can never change.
2. Fields are **additive only**. New fields must have defaults. Never remove or rename.
3. `schema_version()` starts at `1`. Increment per event family when semantics change.
4. Breaking changes require a **new event kind** (e.g., `"tool.called_v2"`).
5. Rust refactors must **not** change serialized format.

### TraceEventEnvelope Bridge

Core defines methods on the enum. The `TraceEventEnvelope` trait (from `openwand-trace`) is implemented in `openwand-store` or `openwand-app`:

```rust
// openwand-store or openwand-app
impl TraceEventEnvelope for OpenWandTraceEvent {
    fn event_kind(&self) -> &'static str { self.event_kind() }
    fn schema_version(&self) -> u16 { self.schema_version() }
}
```

This avoids a dependency between `core` and `trace`.

---

## Module Structure

```
openwand-core/
  Cargo.toml
  src/
    lib.rs                 — re-exports everything flat
    ids.rs                 — 14 domain IDs
    mode.rs                — InteractionMode, ConfirmationLevel
    risk.rs                — RiskLevelSnapshot
    memory_vocab.rs        — EntityKind, Predicate, ClaimKind, ClaimStatusSnapshot,
                            MemoryScope, ProvenanceSnapshot, ConfidenceLevel
    tool_vocab.rs           — ToolInvoker, ToolEffect, ToolResultStatus
    session_vocab.rs        — SessionEndReason, ThinkingBudgetSnapshot
    snapshots.rs           — TokenUsageSnapshot, GateResultSnapshot,
                            AccuracyRecordSnapshot, AccuracyCheckSnapshot,
                            PromptAssemblySnapshot, ErrorSnapshot
    events/
      mod.rs               — OpenWandTraceEvent + re-exports
      session.rs           — SessionEvent
      inference.rs         — InferenceEvent
      gate.rs              — GateEvent
      tool.rs              — ToolEvent
      file.rs              — FileEvent
      memory.rs            — MemoryEvent
      mode.rs              — ModeEvent
      workflow.rs          — WorkflowEvent
      artifact.rs          — ArtifactEvent
```

### lib.rs

```rust
pub mod ids;
pub mod mode;
pub mod risk;
pub mod memory_vocab;
pub mod tool_vocab;
pub mod session_vocab;
pub mod snapshots;
pub mod events;

// Flat re-exports — users write:
//   use openwand_core::{SessionId, OpenWandTraceEvent, ToolEvent};
pub use ids::*;
pub use mode::*;
pub use risk::*;
pub use memory_vocab::*;
pub use tool_vocab::*;
pub use session_vocab::*;
pub use snapshots::*;
pub use events::*;
```

### Cargo.toml

```toml
[package]
name = "openwand-core"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
chrono = { workspace = true, features = ["serde"] }
ulid = { version = "1", features = ["serde"] }
```

No `tokio`, `blake3`, `uuid`, `thiserror`, `loro`, or internal crate deps.

---

## Domain IDs

```rust
// ids.rs

/// Macro for generating typed domain IDs.
/// All IDs are ULID-backed strings with serde support.
macro_rules! domain_id {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Display,
        )]
        pub struct $name(pub String);

        impl $name {
            pub fn new() -> Self {
                Self(ulid::Ulid::new().to_string())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

domain_id!(SessionId);
domain_id!(EpisodeId);
domain_id!(EntityId);
domain_id!(ClaimId);       // unified ID for facts, decisions, preferences
// DecisionId removed. Use ClaimId everywhere. If memory needs an internal
// alias, it defines one locally.
domain_id!(ArtifactId);
domain_id!(ToolCallId);
domain_id!(MessageId);
domain_id!(ApprovalRequestId);
domain_id!(ChunkId);
domain_id!(RunId);
domain_id!(GateId);
domain_id!(WorkflowId);
domain_id!(ModId);
```

### FactId Convention

`FactId` does not exist in core. Use `ClaimId` in all trace events. `openwand-memory` may internally alias `type Fact = Claim<ClaimKind::Fact>` or use `FactId` as a local newtype — but trace events always use `ClaimId`.

---

## Shared Vocabulary Enums

### InteractionMode

```rust
// mode.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InteractionMode {
    Direct,
    Conversational,
    AutoRouting,
    Custom { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfirmationLevel {
    /// Auto-accept after deterministic gates pass
    Auto,
    /// Show diff + explanation, accept on ack
    Inform,
    /// Require explicit approval
    Approve,
    /// Approval + rollback plan + optional second review
    Escalate,
}
```

### RiskLevelSnapshot

```rust
// risk.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiskLevelSnapshot {
    Low,
    Medium,
    High,
    Critical,
}
```

### Memory Vocabulary

```rust
// memory_vocab.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityKind {
    Project,
    Repository,
    File,
    Module,
    Function,
    Class,
    Dependency,
    Tool,
    Command,
    ArchitectureComponent,
    Decision,
    Constraint,
    Preference,
    Bug,
    Test,
    Task,
    Concept,
    Technology,
    Custom(String),
}

impl EntityKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Project => "project",
            Self::Repository => "repository",
            Self::File => "file",
            Self::Module => "module",
            Self::Function => "function",
            Self::Class => "class",
            Self::Dependency => "dependency",
            Self::Tool => "tool",
            Self::Command => "command",
            Self::ArchitectureComponent => "architecture_component",
            Self::Decision => "decision",
            Self::Constraint => "constraint",
            Self::Preference => "preference",
            Self::Bug => "bug",
            Self::Test => "test",
            Self::Task => "task",
            Self::Concept => "concept",
            Self::Technology => "technology",
            Self::Custom(s) => s,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Predicate {
    Uses,
    DependsOn,
    Implements,
    Replaces,
    Rejects,
    Prefers,
    Requires,
    Forbids,
    CausedBy,
    FixedBy,
    TestedBy,
    LocatedIn,
    Supersedes,
    DecidedBecause,
    Contradicts,
    Refines,
    Custom(String),
}

impl Predicate {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Uses => "uses",
            Self::DependsOn => "depends_on",
            Self::Implements => "implements",
            Self::Replaces => "replaces",
            Self::Rejects => "rejects",
            Self::Prefers => "prefers",
            Self::Requires => "requires",
            Self::Forbids => "forbids",
            Self::CausedBy => "caused_by",
            Self::FixedBy => "fixed_by",
            Self::TestedBy => "tested_by",
            Self::LocatedIn => "located_in",
            Self::Supersedes => "supersedes",
            Self::DecidedBecause => "decided_because",
            Self::Contradicts => "contradicts",
            Self::Refines => "refines",
            Self::Custom(s) => s,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClaimKind {
    Fact,
    Decision,
    Preference,
    Constraint,
    ArchitectureNote,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClaimStatusSnapshot {
    Active,
    Superseded,
    Invalidated,
    Reverted,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryScope {
    Global,
    Project { repo: String },
    Session { session_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProvenanceSnapshot {
    UserStated,
    LlmExtracted { model: String, confidence_bps: u16 },  // u16 basis points (0-10000)
    SystemDerived { rule: String },
}

impl ProvenanceSnapshot {
    /// Returns confidence as a float in [0.0, 1.0].
    pub fn confidence(&self) -> Option<f64> {
        match self {
            Self::LlmExtracted { confidence_bps, .. } => Some(*confidence_bps as f64 / 10000.0),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    Explicit,
    Inferred,
    Speculative,
}
```

### Tool Vocabulary

```rust
// tool_vocab.rs

/// Who or what invoked a tool call.
/// Used in trace events to record provenance.
/// NOT the same as openwand-tools::ToolSource (which is about dispatch routing).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolInvoker {
    Llm,
    User,
    System,
    Mcp { server: String },
}

/// What kind of side effect does this tool produce?
/// Declared at tool registration time. Used by policy for risk assessment.
/// Tools declare this; policy evaluates against it; session records it in trace.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolEffect {
    Read,
    Search,
    Write,
    Delete,
    Execute,
    Network,
    Git,
    DependencyChange,
    PolicyChange,
    PersistenceChange,
    AuthChange,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolResultStatus {
    Success,
    Error,
    Partial,
    Pending,
}
```

### Session Vocabulary

```rust
// session_vocab.rs

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionEndReason {
    Natural,
    UserStopped,
    TokenBudgetExhausted,
    MaxStepsReached,
    Error,
    Cancelled,
}

/// Snapshot version of ThinkingBudget for trace events.
/// Runtime version lives in openwand-session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThinkingBudgetSnapshot {
    Off,
    Low,
    Medium,
    High,
    Max,
    Tokens(u32),
}
```

---

## Snapshot DTOs

```rust
// snapshots.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageSnapshot {
    pub input: u64,
    pub output: u64,
    pub reasoning: Option<u64>,
    pub cache_read: Option<u64>,
    pub cache_write: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResultSnapshot {
    pub gate_kind: String,
    pub passed: bool,
    pub risk_level: Option<RiskLevelSnapshot>,
    pub reason_code: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccuracyRecordSnapshot {
    pub commit_hash: Option<String>,
    pub file_coverage: f64,
    pub sensitivity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccuracyCheckSnapshot {
    pub artifact: String,
    pub commit_hash: String,
    pub file_coverage: f64,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptAssemblySnapshot {
    pub system_prompt_hash: String,
    pub message_window_hash: String,
    pub memory_hit_ids: Vec<String>,
    pub memory_context_hash: Option<String>,
    pub tool_manifest_hash: String,
    pub policy_filter_hash: String,
    pub mode: InteractionMode,
    pub working_directory: String,
}

/// Serializable error summary for trace events.
/// Only used when an error needs to be persisted in a trace entry.
/// Not a replacement for crate-specific error types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorSnapshot {
    pub kind: String,
    pub message: String,
    pub recoverable: bool,
}
```

---

## Event Family Enums

### SessionEvent

```rust
// events/session.rs

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

impl SessionEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Started { .. } => "session.started",
            Self::Ended { .. } => "session.ended",
            Self::StepStarted { .. } => "session.step_started",
            Self::StepCompleted { .. } => "session.step_completed",
            Self::UserMessageInjected { .. } => "session.user_message_injected",
        }
    }
}
```

### InferenceEvent

```rust
// events/inference.rs

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

impl InferenceEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Called { .. } => "inference.called",
            Self::Completed { .. } => "inference.completed",
            Self::Failed { .. } => "inference.failed",
        }
    }
}
```

### GateEvent

```rust
// events/gate.rs

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

impl GateEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Evaluated { .. } => "gate.evaluated",
            Self::BatchCompleted { .. } => "gate.batch_completed",
        }
    }
}
```

### ToolEvent

```rust
// events/tool.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolEvent {
    Called {
        tool_call_id: ToolCallId,
        tool_name: String,
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

impl ToolEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Called { .. } => "tool.called",
            Self::Completed { .. } => "tool.completed",
            Self::Failed { .. } => "tool.failed",
            Self::Suspended { .. } => "tool.suspended",
            Self::Resumed { .. } => "tool.resumed",
            Self::Denied { .. } => "tool.denied",
        }
    }
}
```

### FileEvent

```rust
// events/file.rs

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

impl FileEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Read { .. } => "file.read",
            Self::Written { .. } => "file.written",
            Self::Deleted { .. } => "file.deleted",
        }
    }
}
```

### MemoryEvent

```rust
// events/memory.rs

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

impl MemoryEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::EpisodeRecorded { .. } => "memory.episode_recorded",
            Self::EntityCreated { .. } => "memory.entity_created",
            Self::EntityMerged { .. } => "memory.entity_merged",
            Self::EntitySummaryUpdated { .. } => "memory.entity_summary_updated",
            Self::FactExtracted { .. } => "memory.fact_extracted",
            Self::FactAccepted { .. } => "memory.fact_accepted",
            Self::FactRejected { .. } => "memory.fact_rejected",
            Self::FactInvalidated { .. } => "memory.fact_invalidated",
            Self::FactRefined { .. } => "memory.fact_refined",
            Self::DecisionExtracted { .. } => "memory.decision_extracted",
            Self::DecisionAccepted { .. } => "memory.decision_accepted",
            Self::DecisionSuperseded { .. } => "memory.decision_superseded",
            Self::ChunkCreated { .. } => "memory.chunk_created",
            Self::ChunkUpdated { .. } => "memory.chunk_updated",
        }
    }
}
```

### ModeEvent

```rust
// events/mode.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModeEvent {
    Changed {
        from: InteractionMode,
        to: InteractionMode,
        trigger: String,
        accuracy_check: Option<AccuracyCheckSnapshot>,
    },
}

impl ModeEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Changed { .. } => "mode.changed",
        }
    }
}
```

### WorkflowEvent

```rust
// events/workflow.rs

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

impl WorkflowEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::StateChanged { .. } => "workflow.state_changed",
            Self::GatePassed { .. } => "workflow.gate_passed",
            Self::GateFailed { .. } => "workflow.gate_failed",
            Self::ActionExecuted { .. } => "workflow.action_executed",
            Self::ModStarted { .. } => "workflow.mod_started",
            Self::ModCompleted { .. } => "workflow.mod_completed",
        }
    }
}
```

### ArtifactEvent

```rust
// events/artifact.rs

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

impl ArtifactEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Generated { .. } => "artifact.generated",
            Self::Updated { .. } => "artifact.updated",
            Self::Validated { .. } => "artifact.validated",
        }
    }
}
```

---

## Top-Level Event Enum

```rust
// events/mod.rs

mod session;
mod inference;
mod gate;
mod tool;
mod file;
mod memory;
mod mode;
mod workflow;
mod artifact;

pub use session::*;
pub use inference::*;
pub use gate::*;
pub use tool::*;
pub use file::*;
pub use memory::*;
pub use mode::*;
pub use workflow::*;
pub use artifact::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "family", content = "payload", rename_all = "snake_case")]
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

impl OpenWandTraceEvent {
    /// Broad family name for general filtering.
    pub fn event_family(&self) -> &'static str {
        match self {
            Self::Session(_) => "session",
            Self::Inference(_) => "inference",
            Self::Gate(_) => "gate",
            Self::Tool(_) => "tool",
            Self::File(_) => "file",
            Self::Memory(_) => "memory",
            Self::Mode(_) => "mode",
            Self::Workflow(_) => "workflow",
            Self::Artifact(_) => "artifact",
        }
    }

    /// Stable dotted name for indexed trace queries.
    /// Example: "tool.called", "memory.fact_accepted"
    pub fn event_kind(&self) -> &'static str {
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

    /// Schema version for this event's payload.
    /// Increment per event family when semantics change.
    pub fn schema_version(&self) -> u16 {
        1
    }
}
```

---

## Summary

| Category | Count | Files |
|---|---|---|
| Domain IDs | 14 | `ids.rs` |
| Event families | 9 | `events/*.rs` |
| Event variants (total) | ~48 | `events/*.rs` |
| Snapshot DTOs | 6 | `snapshots.rs` |
| Shared vocab enums | 13 | `mode.rs`, `risk.rs`, `memory_vocab.rs`, `tool_vocab.rs`, `session_vocab.rs` |
| Total modules | 15 | 8 top-level + 9 event + 1 mod.rs |

**Estimated LOC:** ~900. All types, no logic. No dependency on any other OpenWand crate.
