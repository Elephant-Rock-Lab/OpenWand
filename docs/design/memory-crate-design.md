# OpenWand Memory Crate Design

**Date:** 2026-05-26  
**Status:** Design — engine-independent  
**Crate:** `openwand-memory`  
**Depends on:** `openwand-core`  
**Blocks:** Phase 1 prototype benchmark

---

## Core Principle

> **OpenWand commits to the memory model now. The storage engine is decided by benchmark, not by debate.**

The model is:

```
episode → chunk → entity → fact → decision → retrieval context
```

The doctrine is (from Graphiti, adapted):

1. Episodes are immutable
2. Facts have provenance
3. Facts have temporal validity (`valid_from`, `valid_to`)
4. Facts are superseded, never deleted
5. Retrieval combines semantic + keyword + graph traversal
6. The LLM proposes extraction; deterministic code applies temporal rules
7. Every accepted fact has an audit trail

---

## 1. Domain Model

### 1.1 The Six Concepts

| Concept | What it is | Lifecycle | Mutable? |
|---|---|---|---|
| **Episode** | Raw observed event — user message, tool call, file diff, command output, decision note | Append-only, never modified | No |
| **Chunk** | Embedded text fragment for retrieval — derived from episodes, entities, facts, or decisions | Created/updated when source changes | Replaceable |
| **Entity** | Typed node — project, file, module, function, decision, preference, constraint, tool | Created on first mention, summary updated over time | Summary mutable |
| **Fact** | Temporal edge between entities — "X uses Y", "X depends_on Y", "X rejects Y" | Created, then superseded by newer facts | Status mutable |
| **Decision** | Structured architecture/project decision — chosen option, rejected alternatives, rationale | Created, then superseded by newer decisions | Status mutable |
| **Retrieval Context** | Assembled result of a hybrid query — layered for agent consumption | Ephemeral, per-query | Ephemeral |

### 1.2 Relationships Between Concepts

```
Episode ─mentions──→ Entity
Episode ─supports──→ Fact
Fact ────supersedes→ Fact        (invalidation chain)
Fact ────in/out────→ Entity     (subject/object)
Decision ─about────→ Entity
Decision ─supersedes→ Decision   (invalidation chain)
Chunk ───source────→ Episode | Entity | Fact | Decision
```

---

## 2. Core Types

### 2.1 Episode

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: EpisodeId,
    pub kind: EpisodeKind,
    pub text: String,
    pub payload: Option<serde_json::Value>,
    pub hash: String,               // blake3 of (kind + text + payload)
    pub occurred_at: DateTime<Utc>,
    pub ingested_at: DateTime<Utc>,
    pub actor: Option<String>,      // "user", "assistant", "system"
    pub session_id: Option<String>,
    pub repo: Option<String>,
    pub branch: Option<String>,
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EpisodeKind {
    UserMessage,
    AssistantMessage,
    ToolCall,
    ToolResult,
    FileRead,
    FileWrite,
    FileDiff,
    ShellCommand,
    ShellOutput,
    GitDiff,
    GitCommit,
    TestResult,
    ArchitectureNote,
    DecisionNote,
    DesignNote,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EpisodeId(pub String);   // ULID
```

### 2.2 Entity

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub kind: EntityKind,
    pub name: String,
    pub canonical_key: String,      // unique lookup key (e.g., "crate:openwand-memory")
    pub aliases: Vec<String>,
    pub summary: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub scope: MemoryScope,
    pub provenance: Provenance,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EntityId(pub String);    // ULID
```

### 2.3 Fact

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub id: FactId,
    pub subject_id: EntityId,
    pub object_id: EntityId,
    pub predicate: Predicate,
    pub claim: String,              // human-readable sentence
    pub confidence: f64,            // 0.0–1.0
    pub scope: MemoryScope,
    pub provenance: Provenance,
    pub source_episodes: Vec<EpisodeId>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,        // None = still active
    pub observed_at: DateTime<Utc>,
    pub invalidated_by: Option<FactId>,         // which fact replaced this one
    pub extraction_model: Option<String>,
    pub extraction_version: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct FactId(pub String);      // ULID
```

### 2.4 Decision

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub id: DecisionId,
    pub title: String,
    pub status: DecisionStatus,
    pub chosen_option: String,
    pub rejected_options: Vec<String>,
    pub rationale: String,
    pub tradeoffs: Vec<String>,
    pub constraints: Vec<String>,
    pub source_episodes: Vec<EpisodeId>,
    pub about_entities: Vec<EntityId>,
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_to: Option<DateTime<Utc>>,
    pub superseded_by: Option<DecisionId>,
    pub embedding: Option<Vec<f32>>,
    pub scope: MemoryScope,
    pub provenance: Provenance,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionStatus {
    Active,
    Superseded,
    Deprecated,
    Reverted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct DecisionId(pub String);  // ULID
```

### 2.5 Chunk (Retrieval Workhorse)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryChunk {
    pub id: ChunkId,
    pub source_type: ChunkSourceType,
    pub source_id: String,          // EpisodeId, EntityId, FactId, or DecisionId as string
    pub text: String,
    pub embedding: Vec<f32>,
    pub repo: Option<String>,
    pub file_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkSourceType {
    Episode,
    Entity,
    Fact,
    Decision,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ChunkId(pub String);     // ULID
```

### 2.6 Shared Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryScope {
    Global,                         // applies everywhere
    Project { repo: String },       // applies to one repo
    Session { session_id: String }, // applies to one session
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Provenance {
    UserStated,                      // user explicitly said this
    LlmExtracted { model: String, confidence: f64 },
    SystemDerived { rule: String },  // deterministic inference
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Confidence {
    Explicit,   // user stated or deterministic
    Inferred,   // LLM extracted with high confidence (>0.85)
    Speculative, // LLM extracted with lower confidence
}
```

---

## 3. Temporal Update Logic

This is the Graphiti-inspired core. The LLM proposes extraction; deterministic code enforces temporal rules.

### 3.1 Ingestion Pipeline

```
New Episode
  │
  ▼
Store Episode (immutable)
  │
  ▼
Extract candidates (LLM)
  │
  ▼
Resolve entities (deterministic + fuzzy)
  │
  ▼
For each extracted fact:
  ├─ Find active facts with same subject + predicate
  ├─ Classify: Duplicate | Refinement | Supersession | New
  ├─ Apply temporal rule
  └─ Record decision in audit trail
  │
  ▼
Refresh entity summaries
  │
  ▼
Update retrieval chunks
```

### 3.2 Fact Classification

```rust
#[derive(Debug, Clone)]
pub enum FactClassification {
    /// Same fact already exists — attach new source episode
    Duplicate { existing_id: FactId },

    /// Clarifies or refines existing fact — update claim/confidence, preserve provenance
    Refinement { existing_id: FactId },

    /// Contradicts or replaces existing fact — invalidate old, create new
    Supersession { superseded_id: FactId },

    /// No existing fact matches
    New,
}

#[async_trait]
pub trait TemporalPolicy: Send + Sync {
    async fn classify(
        &self,
        new_fact: &ExtractedFact,
        existing_active: &[Fact],
    ) -> Result<FactClassification>;
}
```

### 3.3 Ingestion Logic (Pseudocode)

```rust
pub async fn ingest_episode(
    store: &dyn MemoryStore,
    extractor: &dyn Extractor,
    resolver: &dyn EntityResolver,
    temporal: &dyn TemporalPolicy,
    input: EpisodeInput,
) -> Result<()> {
    // 1. Store raw episode (immutable)
    let episode = store.put_episode(input).await?;

    // 2. Extract candidates (LLM — non-deterministic)
    let extraction = extractor.extract(&episode).await?;

    // 3. Resolve entities (deterministic fuzzy matching)
    let entities = resolver.resolve_entities(extraction.entities).await?;

    // 4. Apply temporal rules to each extracted fact
    for fact in extraction.facts {
        let subject = entities.get(&fact.subject_key)
            .ok_or_else(|| anyhow!("unresolved subject: {}", fact.subject_key))?;
        let object = entities.get(&fact.object_key)
            .ok_or_else(|| anyhow!("unresolved object: {}", fact.object_key))?;

        let active = store.active_facts_about(
            &subject.id, &fact.predicate, Utc::now()
        ).await?;

        let action = temporal.classify(&fact, &active).await?;

        match action {
            FactClassification::Duplicate { existing_id } => {
                store.attach_source_episode(existing_id, &episode.id).await?;
            }
            FactClassification::Refinement { existing_id } => {
                store.refine_fact(existing_id, &fact.claim, fact.confidence, &episode.id).await?;
            }
            FactClassification::Supersession { superseded_id } => {
                let now = Utc::now();
                store.invalidate_fact(superseded_id, now).await?;
                store.create_fact(subject.id, object.id, fact, &episode.id).await?;
            }
            FactClassification::New => {
                store.create_fact(subject.id, object.id, fact, &episode.id).await?;
            }
        }
    }

    // 5. Handle extracted decisions
    for decision in extraction.decisions {
        let about = decision.about_keys.iter()
            .filter_map(|k| entities.get(k))
            .map(|e| e.id.clone())
            .collect();
        store.create_decision(decision, about, &episode.id).await?;
    }

    // 6. Refresh entity summaries and retrieval chunks
    let touched_entity_ids: Vec<EntityId> = entities.values().map(|e| e.id.clone()).collect();
    store.refresh_summaries(&touched_entity_ids).await?;
    store.update_chunks(&episode, &touched_entity_ids).await?;

    Ok(())
}
```

### 3.4 Important: Two Kinds of Time

| | Recorded time | Semantic validity |
|---|---|---|
| **What** | When OpenWand stored the fact | When the fact is true in reality |
| **Field** | `observed_at`, `ingested_at` | `valid_from`, `valid_to` |
| **Storage** | Automatic via DB engine | Manual via application logic |
| **Example** | "This fact was stored on May 26" | "Loro was used for sessions from May 1 to May 26" |

**Both are required.** Do not delegate semantic temporal logic to database time travel.

---

## 4. Extraction Pipeline

### 4.1 Extractor Trait

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionBatch {
    pub entities: Vec<ExtractedEntity>,
    pub facts: Vec<ExtractedFact>,
    pub decisions: Vec<ExtractedDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub local_id: String,
    pub kind: EntityKind,
    pub name: String,
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedFact {
    pub subject_key: String,
    pub object_key: String,
    pub predicate: Predicate,
    pub claim: String,
    pub valid_from: Option<DateTime<Utc>>,
    pub confidence: f64,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDecision {
    pub title: String,
    pub chosen_option: String,
    pub rejected_options: Vec<String>,
    pub rationale: String,
    pub tradeoffs: Vec<String>,
    pub constraints: Vec<String>,
    pub about_keys: Vec<String>,
}

#[async_trait]
pub trait Extractor: Send + Sync {
    async fn extract(&self, episode: &Episode) -> Result<ExtractionBatch>;
}
```

### 4.2 Extraction Prompt Contract

The LLM receives an episode and must return structured JSON matching the `ExtractionBatch` schema. The extraction is the **only non-deterministic step** in the pipeline. Everything after extraction is deterministic:

| Step | Deterministic? | What it does |
|---|---|---|
| Store episode | ✅ | Immutable append |
| Extract candidates | ❌ | LLM proposes entities, facts, decisions |
| Resolve entities | ✅ | Fuzzy match against existing entities |
| Classify facts | ✅ | Deterministic rules (exact match, confidence threshold, predicate match) |
| Apply temporal rules | ✅ | Supersede/refine/duplicate logic |
| Record audit trail | ✅ | Decision ledger entry |

### 4.3 Extraction Quality Controls

The LLM extraction must pass through these deterministic gates before any fact is written:

```rust
pub struct ExtractionGate {
    /// Maximum entities per episode (prevent LLM explosion)
    pub max_entities: usize,           // default: 10

    /// Maximum facts per episode
    pub max_facts: usize,              // default: 15

    /// Maximum decisions per episode
    pub max_decisions: usize,          // default: 5

    /// Minimum confidence for fact acceptance
    pub min_confidence: f64,           // default: 0.5

    /// Required fields that must be present
    pub required_fact_fields: Vec<String>,  // subject, object, predicate, claim

    /// Allowed predicates (ontology enforcement)
    pub allowed_predicates: Vec<Predicate>,

    /// Allowed entity kinds (ontology enforcement)
    pub allowed_entity_kinds: Vec<EntityKind>,
}
```

---

## 5. Entity Resolution

### 5.1 Resolver Trait

```rust
#[async_trait]
pub trait EntityResolver: Send + Sync {
    /// Resolve an extracted entity to an existing EntityId, or create a new one.
    async fn resolve(&self, extracted: &ExtractedEntity, store: &dyn MemoryStore)
        -> Result<Entity>;
}
```

### 5.2 Resolution Strategy

1. **Exact canonical_key match** — `"crate:openwand-memory"` → existing entity
2. **Alias match** — "memory core" → alias of existing entity
3. **Fuzzy name match** (Levenshtein ≤ 2) — "memmory crate" → "memory crate"
4. **Create new** — if no match found

This is deterministic. No LLM involved in resolution.

---

## 6. Retrieval Engine

### 6.1 Query Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridQuery {
    pub text: String,
    pub embedding: Option<Vec<f32>>,
    pub filters: QueryFilters,
    pub limit: usize,                // default: 20
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryFilters {
    pub scope: Option<MemoryScope>,
    pub entity_kinds: Option<Vec<EntityKind>>,
    pub predicates: Option<Vec<Predicate>>,
    pub as_of: Option<DateTime<Utc>>,    // temporal filter: facts valid at this time
    pub repo: Option<String>,
    pub min_confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub hits: Vec<SearchHit>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: String,                 // ChunkId, EntityId, or FactId
    pub kind: SearchHitKind,
    pub score: f64,
    pub text: String,
    pub source_type: ChunkSourceType,
    pub source_id: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchHitKind {
    Chunk,
    Entity,
    Fact,
    Decision,
}
```

### 6.2 Retrieval Flow

```
Input query (text + optional embedding)
  │
  ▼
┌──────────────────────────────────┐
│ Parallel hybrid search           │
│  ├─ Vector search (chunks)       │
│  ├─ FTS / BM25 search (chunks)   │
│  ├─ Decision search              │
│  └─ Fact search                  │
└──────────────┬───────────────────┘
               │
               ▼
┌──────────────────────────────────┐
│ Result fusion (RRF)              │
│ Merge vector + FTS + graph hits  │
└──────────────┬───────────────────┘
               │
               ▼
┌──────────────────────────────────┐
│ Entity seed extraction           │
│ Identify entities from hits      │
└──────────────┬───────────────────┘
               │
               ▼
┌──────────────────────────────────┐
│ Graph expansion (1–2 hops)       │
│  ├─ Active facts around seeds    │
│  ├─ Recent decisions             │
│  ├─ Constraints / preferences    │
│  └─ Related files / modules      │
└──────────────┬───────────────────┘
               │
               ▼
┌──────────────────────────────────┐
│ Temporal filter                  │
│  ├─ true now                     │
│  ├─ true at requested time       │
│  └─ superseded but relevant      │
└──────────────┬───────────────────┘
               │
               ▼
┌──────────────────────────────────┐
│ Rerank                           │
│  ├─ Semantic relevance           │
│  ├─ BM25 relevance               │
│  ├─ Graph distance               │
│  ├─ Confidence                   │
│  ├─ Recency                      │
│  └─ Scope match                  │
└──────────────┬───────────────────┘
               │
               ▼
Context assembly (layered)
```

### 6.3 Context Assembly

Return context in layers, ordered by utility:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalContext {
    /// Layer 1: Current active facts relevant to the query
    pub active_facts: Vec<Fact>,

    /// Layer 2: Relevant decisions (chosen + rationale)
    pub decisions: Vec<Decision>,

    /// Layer 3: Source episodes that produced these facts
    pub source_episodes: Vec<Episode>,

    /// Layer 4: Superseded facts (only if useful for understanding change)
    pub superseded_facts: Vec<Fact>,

    /// Layer 5: Raw chunks for grounding
    pub chunks: Vec<MemoryChunk>,

    /// Metadata about the retrieval
    pub query: HybridQuery,
    pub total_hits: usize,
    pub retrieval_latency_ms: u64,
}
```

---

## 7. Storage Trait

This is the abstraction that makes the storage engine swappable.

```rust
#[async_trait]
pub trait MemoryStore: Send + Sync {
    // ── Phase 1: Episodes + Chunks + Hybrid Search ──

    async fn put_episode(&self, input: EpisodeInput) -> Result<Episode>;
    async fn get_episode(&self, id: &EpisodeId) -> Result<Option<Episode>>;

    async fn put_chunk(&self, chunk: MemoryChunk) -> Result<ChunkId>;
    async fn search_hybrid(&self, query: HybridQuery) -> Result<SearchResult>;

    // ── Phase 2: Entities + Facts + Temporal Queries ──

    async fn create_entity(&self, entity: Entity) -> Result<EntityId>;
    async fn get_entity(&self, id: &EntityId) -> Result<Option<Entity>>;
    async fn get_entity_by_key(&self, canonical_key: &str) -> Result<Option<Entity>>;
    async fn update_entity_summary(&self, id: &EntityId, summary: &str) -> Result<()>;

    async fn create_fact(
        &self,
        subject: EntityId,
        object: EntityId,
        fact: ExtractedFact,
        source_episode: &EpisodeId,
    ) -> Result<FactId>;
    async fn active_facts_about(
        &self,
        entity: &EntityId,
        predicate: &Predicate,
        as_of: DateTime<Utc>,
    ) -> Result<Vec<Fact>>;
    async fn invalidate_fact(
        &self,
        fact_id: FactId,
        valid_to: DateTime<Utc>,
    ) -> Result<()>;
    async fn attach_source_episode(
        &self,
        fact_id: FactId,
        episode_id: &EpisodeId,
    ) -> Result<()>;
    async fn refine_fact(
        &self,
        fact_id: FactId,
        new_claim: &str,
        new_confidence: f64,
        source_episode: &EpisodeId,
    ) -> Result<()>;

    // ── Phase 3: Decisions ──

    async fn create_decision(
        &self,
        decision: ExtractedDecision,
        about_entities: Vec<EntityId>,
        source_episode: &EpisodeId,
    ) -> Result<DecisionId>;
    async fn decision_history(&self, entity: &EntityId) -> Result<Vec<Decision>>;
    async fn supersede_decision(
        &self,
        old_id: DecisionId,
        new_id: DecisionId,
    ) -> Result<()>;

    // ── Maintenance ──

    async fn refresh_summaries(&self, entity_ids: &[EntityId]) -> Result<()>;
    async fn update_chunks(
        &self,
        episode: &Episode,
        entity_ids: &[EntityId],
    ) -> Result<()>;

    // ── Lifecycle ──

    async fn initialize(&self) -> Result<()>;   // create tables, indexes
    async fn migrate(&self, version: u32) -> Result<()>;  // schema evolution
}
```

---

## 8. Decision Ledger Integration

Every fact acceptance or rejection is recorded in the decision ledger (from the Trust Architecture design):

```rust
pub struct MemoryDecision {
    pub id: DecisionRecordId,
    pub timestamp: DateTime<Utc>,
    pub episode_id: EpisodeId,
    pub action: FactClassification,
    pub extracted_fact: ExtractedFact,
    pub gates_passed: Vec<String>,
    pub outcome: MemoryDecisionOutcome,
}

pub enum MemoryDecisionOutcome {
    Created { new_id: FactId },
    Duplicated { existing_id: FactId },
    Refined { existing_id: FactId },
    Superseded { old_id: FactId, new_id: FactId },
    Rejected { reason: String },
}
```

This means every memory mutation can be audited: "Why did OpenWand believe X on May 26?" → trace through the decision ledger.

---

## 9. Crate Layout

```
openwand-memory/
  Cargo.toml
  src/
    lib.rs                    — public API, re-exports
    types.rs                  — all domain types (Episode, Entity, Fact, Decision, Chunk)
    store.rs                  — MemoryStore trait
    episode.rs                — EpisodeInput, Episode construction, hashing
    extraction.rs             — Extractor trait, ExtractionBatch, ExtractionGate
    entity_resolution.rs      — EntityResolver trait, fuzzy matching
    temporal.rs               — TemporalPolicy trait, classification logic
    retrieval.rs              — HybridQuery, SearchResult, retrieval flow
    context.rs                — RetrievalContext, context assembly
    ingestion.rs              — ingest_episode() pipeline
    ledger.rs                 — MemoryDecision audit trail
    embed.rs                  — EmbeddingProvider trait

    backends/
      mod.rs                  — backend selection, feature flags
      surrealdb.rs            — SurrealDB implementation (behind "surrealdb" feature)
      cozo.rs                 — CozoDB implementation (behind "cozo" feature)
      sqlite.rs               — SQLite + sqlite-vec + FTS5 (behind "sqlite" feature)

    eval.rs                   — evaluation harness (precision, recall, latency benchmarks)
```

### Cargo.toml

```toml
[package]
name = "openwand-memory"
version = "0.1.0"
edition = "2024"

[dependencies]
openwand-core = { path = "../core" }
anyhow = "1"
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
uuid = { version = "1", features = ["v4", "serde"] }
ulid = { version = "1", features = ["serde"] }
blake3 = "1"
tracing = "0.1"

# Embedding (used by all backends)
regex = "1"

# SurrealDB backend (optional)
surrealdb = { version = "3", features = ["kv-rocksdb"], optional = true }

# CozoDB backend (optional)
cozo = { version = "0.7", features = ["storage-sqlite"], optional = true }

# SQLite fallback (optional)
rusqlite = { version = "0.32", features = ["bundled"], optional = true }

[features]
default = ["sqlite"]
surrealdb = ["dep:surrealdb"]
cozo = ["dep:cozo"]
sqlite = ["dep:rusqlite"]
all-backends = ["surrealdb", "cozo", "sqlite"]

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread", "test-util"] }
criterion = "0.5"
insta = "1"
```

---

## 10. Benchmark Plan

### 10.1 Phase 1 Benchmark (Decides the Engine)

Implement `MemoryStore` for all three engines. Test with identical workloads.

#### Dataset

```rust
pub struct BenchmarkDataset {
    /// 1,000 episodes of varying kinds
    pub episodes: Vec<EpisodeInput>,

    /// 500 chunks (pre-computed embeddings)
    pub chunks: Vec<MemoryChunk>,

    /// 50 hybrid search queries with known-relevant results
    pub queries: Vec<(HybridQuery, Vec<String>)>,  // (query, expected chunk IDs)
}
```

#### Gates

| Gate | Pass Criteria | Weight |
|---|---|---|
| **Binary size** | < 5MB added to OpenWand binary | Must-pass |
| **Startup latency** | < 200ms from `initialize()` to ready | Must-pass |
| **Episode insert** | < 1ms per episode (sustained 1000/s) | Must-pass |
| **Chunk insert** | < 5ms per chunk (no embedding) | Must-pass |
| **Hybrid search** | < 100ms for top-20 results (10K chunks) | Must-pass |
| **FTS quality** | Recall ≥ 0.8 on known-relevant queries | Must-pass |
| **Vector quality** | Recall ≥ 0.7 on known-relevant queries | Must-pass |
| **Schema migration** | Can add a field without data loss | Must-pass |
| **Zero external deps** | No separate server process | Must-pass |
| **Query ergonomics** | Developer can write retrieval query in < 15 min | Tiebreaker |
| **Build time** | Incremental build < 30s after a change in memory crate | Tiebreaker |
| **Memory usage** | RSS < 100MB at rest with 10K episodes + 5K chunks | Tiebreaker |

#### Decision Rule

1. Eliminate any engine that fails a must-pass gate
2. Among survivors, rank by tiebreaker scores
3. If tied, prefer the engine with smaller dependency tree

### 10.2 Expected Results (Hypothesis)

| | SurrealDB | CozoDB | SQLite + sqlite-vec |
|---|---|---|---|
| Binary size | ⚠️ Heavy | ✅ Light | ✅ Lightest |
| Startup | ⚠️ Slower | ✅ Fast | ✅ Fastest |
| Insert | ✅ Good | ✅ Good | ✅ Good |
| Hybrid search | ✅ Good (built-in) | ✅ Good (built-in) | ⚠️ Manual (FTS5 + sqlite-vec) |
| Graph queries | ✅ RELATE + traversal | ✅ Datalog recursive | ❌ Not supported |
| Query ergonomics | ✅ SurrealQL (SQL-like) | ⚠️ Datalog (learning curve) | ✅ SQL (familiar) |
| Maturity | ⚠️ v2, API churn risk | ✅ v0.7, stable | ✅ Battle-tested |
| License | ⚠️ BSL 1.1 | ✅ MPL-2.0 | ✅ MIT |

**Predicted winner:** Depends on binary size benchmark. If SurrealDB comes in under 5MB, it wins on ergonomics. If not, CozoDB wins on austerity. SQLite is the MVP fallback if both fail.

---

## 11. Implementation Phases

### Phase 1 — Local Memory Substrate (Weeks 1–3)

**Build:**
- `MemoryStore` trait
- Episode table + append-only writes
- Chunk table + embedding storage
- FTS index on episodes and chunks
- HNSW vector index on chunks
- Basic hybrid search (vector + FTS fusion)
- SQLite backend (default)
- Benchmark harness

**Skip:** Graph, entities, facts, decisions, temporal logic.

**Deliverable:** Can store episodes, search them by text and vector, get results back. Enough for a basic "remember what happened" feature.

### Phase 2 — Temporal Graph (Weeks 4–7)

**Build:**
- Entity table + canonical_key resolution
- Fact relation table with `valid_from`/`valid_to`
- `mentions` and `supports` relations
- Entity resolution (exact → alias → fuzzy → create)
- Temporal policy (classify: duplicate/refinement/supersession/new)
- Source provenance on all facts

**Skip:** Decision intelligence, extraction pipeline (use stub).

**Deliverable:** Can answer "what is true now?" and "where did that fact come from?"

### Phase 3 — Decision Intelligence (Weeks 8–10)

**Build:**
- Decision table
- `decision_about` relation
- Superseded decisions
- Architecture rationale storage
- Rejected alternatives tracking
- Constraint memory

**Deliverable:** Can answer "why did we choose X?" and "what did we reject and why?"

### Phase 4 — Extraction Pipeline (Weeks 11–14)

**Build:**
- LLM extraction prompt + structured output parsing
- ExtractionGate validation
- Confidence scoring
- Extraction error handling (malformed LLM output)
- Decision ledger for all memory mutations

**Skip:** Contradiction detection, Ebbinghaus decay.

**Deliverable:** Episodes are automatically processed into entities, facts, and decisions.

### Phase 5 — Advanced Memory (Weeks 15–20)

**Build:**
- Contradiction detection across sessions
- Fact supersession chains with full history
- Entity summary refresh (LLM-generated summaries)
- Confidence decay (Ebbinghaus-inspired)
- Scope-aware retrieval (project vs global vs session)
- Memory export/import
- MCP tools for external agent access

**Deliverable:** Full temporal knowledge graph with self-maintaining intelligence.

---

## 12. Memory Controls Summary

Extracted memories carry these controls as first-class fields:

| Control | Where | Purpose |
|---|---|---|
| **Provenance** | `Provenance` enum on Fact, Decision | Where did this come from? User, LLM, or system? |
| **Scope** | `MemoryScope` enum on Entity, Fact, Decision | Global, per-project, or per-session? |
| **Confidence** | `confidence: f64` on Fact, Decision | How reliable is this? (0.0–1.0) |
| **Validity** | `valid_from`, `valid_to` on Fact, Decision | When is this true? |
| **Supersession** | `invalidated_by` on Fact, `superseded_by` on Decision | What replaced this? |
| **Provenance chain** | `source_episodes: Vec<EpisodeId>` | Which raw events produced this? |
| **Audit trail** | `MemoryDecision` in decision ledger | Why was this accepted/rejected? |
| **Visibility** | All types are serializable, queryable | User can always inspect and correct |

---

## 13. Ontology (Initial)

Start narrow. Expand when the schema proves insufficient.

### Entity Kinds

```
project, repository, file, module, function, class, dependency,
tool, command, architecture_component, decision, constraint,
preference, bug, test, task, concept, technology
```

### Fact Predicates

```
uses, depends_on, implements, replaces, rejects, prefers,
requires, forbids, caused_by, fixed_by, tested_by, located_in,
supersedes, decided_because, contradicts, refines
```

### Extension Rule

New entity kinds and predicates can be added via `Custom(String)` variants. A custom entry is promoted to a named variant only when it appears in ≥ 5 distinct sessions with consistent semantics. This prevents ontology bloat while allowing organic growth.

---

## 14. What This Is Not

1. **Not a generic agent memory framework.** This is specialized for coding-agent project intelligence.
2. **Not a vector database.** Vectors are one retrieval path. The graph is the source of truth.
3. **Not Graphiti.** Graphiti is the doctrine reference. OpenWand has a narrower, software-specific ontology.
4. **Not committed to a storage engine.** The trait makes the engine swappable. The benchmark decides.
5. **Not building everything at once.** Phase 1 is episodes + chunks + search. The graph comes later.

---

## 15. Open Questions

| Question | Status | When to Decide |
|---|---|---|
| Storage engine: SurrealDB, CozoDB, or SQLite? | Open — benchmark decides | After Phase 1 benchmark |
| Embedding model: local or API? | Open — depends on HB-G1 binary size | During Phase 1 implementation |
| Extraction prompt: per-model or universal? | Open — test with target LLMs first | During Phase 4 |
| Ebbinghaus decay parameters | Open — needs empirical tuning | During Phase 5 |
| MCP exposure: which memory tools? | Open — depends on agent loop design | During Phase 5 |
| Memory export format | Open — consider GraphML, JSON-LD, or custom | When users request portability |
