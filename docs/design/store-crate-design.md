# OpenWand Store Crate Design

**Date:** 2026-05-26  
**Status:** Design — locked  
**Crate:** `openwand-store`  
**Depends on:** `openwand-core`, `openwand-trace`, `openwand-memory`  
**Blocks:** Batch 1 trace persistence, session reload, memory retrieval  

---

## North Star

> Store persists. Trace authorizes. Session projects Loro. Memory projects knowledge. Policy gates tools. App wires everything.

`openwand-store` is the physical persistence layer. It implements storage traits defined elsewhere. It owns no domain truth.

---

## Crate Boundary

### Implements

- `TraceStore<OpenWandTraceEvent>` (from `openwand-trace`)
- `MemoryReadStore` (from `openwand-memory`, public read API)
- `MemoryProjectionStore` (internal, trace-backed writes only)
- `ProjectionCheckpointStore` (checkpoint persistence)
- `SnapshotBlobStore` (opaque session snapshots)
- `TraceReplayStore` (ordered stream replay for rebuilds)
- `StoreMigrationRunner` (schema creation and migration)
- `TraceEventEnvelope` for `OpenWandTraceEvent` (bridge from core to trace trait)
- `BlobStore` (content-addressed immutable blobs)

### Does NOT implement

- Memory extraction logic
- Temporal policy / entity resolution
- Session loop / Loro state mutation
- Policy gate evaluation
- Tool execution
- Loro rebuild (session owns this)

### Depends on

```
openwand-core   — event vocab, IDs, DTOs
openwand-trace  — TraceStore<E>, TraceEventEnvelope, trace types
openwand-memory — MemoryReadStore, MemoryProjectionStore, memory types
```

### Does NOT depend on

```
openwand-session   (would create cycle)
openwand-policy    (no policy in store)
openwand-tools     (no tool execution)
openwand-llm       (no inference)
loro               (store doesn't know Loro)
rig, rmcp          (no external integrations)
```

---

## Crate Layout

```
openwand-store/
  Cargo.toml
  src/
    lib.rs
    config.rs
    error.rs
    envelope.rs                — TraceEventEnvelope impl for OpenWandTraceEvent

    trace_store.rs             — shared TraceStore helpers
    memory_read.rs             — MemoryReadStore impl boundary
    memory_projection.rs       — internal projection writer API
    blob.rs                    — immutable content-addressed blobs
    snapshot.rs                — opaque session snapshot save/load
    replay.rs                  — TraceReplayStore: ordered stream replay
    checkpoint.rs              — ProjectionCheckpointStore
    migrations.rs              — migration runner
    integrity.rs               — hash-chain verification
    serde_helpers.rs           — stable payload serialization

    backends/
      mod.rs
      sqlite/
        mod.rs
        store.rs               — SqliteStore struct, open/init
        trace.rs               — TraceStore impl
        memory_read.rs         — MemoryReadStore impl
        memory_projection.rs   — MemoryProjectionStore impl
        blob.rs                — BlobStore impl
        snapshot.rs            — SnapshotBlobStore impl
        replay.rs              — TraceReplayStore impl
        checkpoint.rs          — ProjectionCheckpointStore impl
        schema.rs              — CREATE TABLE statements
        migrations.rs          — migration SQL
        queries.rs             — shared query helpers

  migrations/
    sqlite/
      0001_trace.sql
      0002_blobs.sql
      0003_memory_projection.sql
      0004_projection_checkpoint.sql
      0005_snapshot_blob.sql

  benches/
    trace_append.rs
    memory_search.rs
    projection_rebuild.rs
    backend_matrix.rs

  tests/
    trace_store_conformance.rs
    memory_store_conformance.rs
    migration_conformance.rs
    rebuild_conformance.rs
```

---

## Dependencies

```toml
[package]
name = "openwand-store"
version.workspace = true
edition.workspace = true

[dependencies]
openwand-core = { path = "../core" }
openwand-trace = { path = "../trace" }
openwand-memory = { path = "../memory" }

anyhow = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
chrono = { workspace = true, features = ["serde"] }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
tokio = { workspace = true, features = ["sync", "macros"] }
tracing = { workspace = true }
ulid = { version = "1", features = ["serde"] }
blake3 = { workspace = true }

rusqlite = { version = "0.32", features = ["bundled"], optional = true }
sqlite-vec = { version = "0.1", optional = true }

cozo = { version = "0.7", features = ["storage-sqlite"], optional = true }
surrealdb = { version = "3", features = ["kv-rocksdb"], optional = true }

[features]
default = ["sqlite"]
sqlite = ["dep:rusqlite", "dep:sqlite-vec"]
cozo = ["dep:cozo"]
surrealdb = ["dep:surrealdb"]
all-backends = ["sqlite", "cozo", "surrealdb"]
```

---

## Public API

### Top-Level Type

```rust
pub struct OpenWandStore {
    backend: StoreBackend,
}

pub enum StoreBackend {
    #[cfg(feature = "sqlite")]
    Sqlite(SqliteStore),

    #[cfg(feature = "cozo")]
    Cozo(CozoStore),

    #[cfg(feature = "surrealdb")]
    Surreal(SurrealStore),
}

impl OpenWandStore {
    pub async fn open(config: StoreConfig) -> Result<Self>;
}
```

### Configuration

```rust
pub struct StoreConfig {
    pub backend: BackendConfig,
    pub data_dir: PathBuf,
    pub run_migrations: bool,
}

pub enum BackendConfig {
    Sqlite { path: PathBuf },
    Cozo { path: PathBuf },
    Surreal { path: PathBuf },
}
```

### Implemented Traits

```rust
// All backends implement these via their concrete types
impl TraceStore<OpenWandTraceEvent> for SqliteStore { ... }
impl MemoryReadStore for SqliteStore { ... }
impl MemoryProjectionStore for SqliteStore { ... }
impl ProjectionCheckpointStore for SqliteStore { ... }
impl SnapshotBlobStore for SqliteStore { ... }
impl TraceReplayStore for SqliteStore { ... }
impl BlobStore for SqliteStore { ... }
```

Swappability comes from shared traits + conformance tests, not from a generic query abstraction.

---

## Envelope Bridge

The `TraceEventEnvelope` trait lives in `openwand-trace`. Core defines `event_kind()` and `schema_version()` methods on the enum. Store bridges them:

```rust
// envelope.rs

use openwand_core::OpenWandTraceEvent;
use openwand_trace::TraceEventEnvelope;

impl TraceEventEnvelope for OpenWandTraceEvent {
    fn event_kind(&self) -> &'static str {
        // Delegates to core's method
        self.event_kind()
    }

    fn schema_version(&self) -> u16 {
        self.schema_version()
    }
}
```

This is the only place `openwand-core` meets `openwand-trace`. Clean.

---

## SQLite Connection Model

Single serialized writer via dedicated blocking worker. No direct blocking on the async runtime.

```rust
pub struct SqliteStore {
    writer: SqliteWriter,
    base_path: PathBuf,
}

struct SqliteWriter {
    tx: mpsc::Sender<WriterCommand>,
}

enum WriterCommand {
    AppendTrace {
        entry: AppendTraceEntry<OpenWandTraceEvent>,
        relations: Vec<TraceRelationDraft>,
        response: oneshot::Sender<Result<TraceId>>,
    },
    QueryTrace {
        query: TraceQuery<OpenWandTraceEvent>,
        response: oneshot::Sender<Result<Vec<TraceEntry<OpenWandTraceEvent>>>>,
    },
    // ... other commands
}

impl SqliteWriter {
    fn spawn(conn: Connection, rx: mpsc::Receiver<WriterCommand>) -> JoinHandle<()> {
        tokio::task::spawn_blocking(move || {
            while let Some(cmd) = rx.blocking_recv() {
                match cmd {
                    WriterCommand::AppendTrace { entry, relations, response } => {
                        let result = Self::do_append(&conn, &entry, &relations);
                        let _ = response.send(result);
                    }
                    // ... handle other commands
                }
            }
        })
    }
}
```

SQLite settings for Batch 1:

```text
WAL mode                — concurrent reads + serialized writes
foreign_keys = ON       — referential integrity
busy_timeout = 5000     — wait instead of immediate SQLITE_BUSY
synchronous = NORMAL    — safe enough with WAL, faster than FULL
journal_size_limit      — prevent unbounded WAL growth
```

---

## Logical Schema

All backends implement the same logical schema. Physical layout may differ.

### 4.1 Trace Tables

```sql
CREATE TABLE IF NOT EXISTS trace_entry (
    id                    TEXT PRIMARY KEY,
    stream_scope          TEXT NOT NULL,
    stream_id             TEXT NOT NULL,
    stream_sequence       INTEGER NOT NULL,
    global_sequence       INTEGER NOT NULL,
    occurred_at           INTEGER NOT NULL,
    actor_kind            TEXT NOT NULL,
    actor_payload         TEXT,
    event_kind            TEXT NOT NULL,
    event_payload         TEXT NOT NULL,
    event_schema_version  INTEGER NOT NULL,
    trace_schema_version  INTEGER NOT NULL DEFAULT 1,
    prev_hash             TEXT,
    entry_hash            TEXT NOT NULL,
    idempotency_key       TEXT,
    created_at            INTEGER NOT NULL,

    UNIQUE(stream_id, stream_sequence),
    UNIQUE(global_sequence)
);

CREATE INDEX idx_trace_stream ON trace_entry(stream_id, stream_sequence);
CREATE INDEX idx_trace_kind ON trace_entry(event_kind);
CREATE INDEX idx_trace_occurred ON trace_entry(occurred_at);
CREATE INDEX idx_trace_idempotency ON trace_entry(idempotency_key)
    WHERE idempotency_key IS NOT NULL;
```

```sql
CREATE TABLE IF NOT EXISTS trace_relation (
    id            TEXT PRIMARY KEY,
    from_trace_id TEXT NOT NULL,
    to_trace_id   TEXT NOT NULL,
    kind          TEXT NOT NULL,
    created_at    INTEGER NOT NULL,

    FOREIGN KEY (from_trace_id) REFERENCES trace_entry(id),
    FOREIGN KEY (to_trace_id) REFERENCES trace_entry(id)
);

CREATE INDEX idx_relation_from ON trace_relation(from_trace_id);
CREATE INDEX idx_relation_to ON trace_relation(to_trace_id);
```

### 4.2 Blob Table

Content-addressed immutable blobs. Critical for rebuild-from-trace — episode text needs actual content, not just a hash.

```sql
CREATE TABLE IF NOT EXISTS trace_blob (
    hash          TEXT PRIMARY KEY,
    media_type    TEXT NOT NULL,
    bytes         BLOB NOT NULL,
    size_bytes    INTEGER NOT NULL,
    compression   TEXT,
    created_at    INTEGER NOT NULL
);
```

### 4.3 Projection Checkpoint Tables

```sql
CREATE TABLE IF NOT EXISTS projection_checkpoint (
    projector_name          TEXT PRIMARY KEY,
    last_global_sequence    INTEGER NOT NULL,
    last_trace_id           TEXT,
    error_count             INTEGER NOT NULL DEFAULT 0,
    last_error              TEXT,
    updated_at              INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS projection_error (
    id                    TEXT PRIMARY KEY,
    projector_name         TEXT NOT NULL,
    trace_id               TEXT,
    global_sequence        INTEGER,
    error_message          TEXT NOT NULL,
    recoverable            INTEGER NOT NULL DEFAULT 1,
    created_at             INTEGER NOT NULL
);
```

### 4.4 Snapshot Blob Table

Opaque session snapshots. Store doesn't know the format — could be Loro bytes, JSON metadata, anything.

```sql
CREATE TABLE IF NOT EXISTS snapshot_blob (
    key           TEXT PRIMARY KEY,
    bytes         BLOB NOT NULL,
    media_type    TEXT NOT NULL,
    size_bytes    INTEGER NOT NULL,
    metadata_json TEXT,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);
```

### 4.5 Memory Projection Tables

```sql
CREATE TABLE IF NOT EXISTS memory_episode (
    episode_id      TEXT PRIMARY KEY,
    episode_kind    TEXT NOT NULL,
    text_hash       TEXT NOT NULL,
    content_ref     TEXT,
    scope_kind      TEXT NOT NULL,
    scope_payload   TEXT,
    recorded_at     INTEGER NOT NULL,
    valid_from      INTEGER,
    valid_to        INTEGER,
    source_trace_id TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,

    FOREIGN KEY (content_ref) REFERENCES trace_blob(hash)
);

CREATE TABLE IF NOT EXISTS memory_chunk (
    chunk_id          TEXT PRIMARY KEY,
    source_kind       TEXT NOT NULL,
    source_id         TEXT NOT NULL,
    text              TEXT NOT NULL,
    text_hash         TEXT NOT NULL,
    embedding         BLOB,
    embedding_model   TEXT,
    created_at        INTEGER NOT NULL,
    metadata_json     TEXT,

    FOREIGN KEY (source_id) REFERENCES memory_episode(episode_id)
);

-- FTS5 for Batch 1 text search
CREATE VIRTUAL TABLE IF NOT EXISTS memory_chunk_fts
USING fts5(chunk_id UNINDEXED, text, content='');

-- Trigger to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS memory_chunk_fts_insert
AFTER INSERT ON memory_chunk BEGIN
    INSERT INTO memory_chunk_fts(rowid, chunk_id, text)
    VALUES (new.rowid, new.chunk_id, new.text);
END;
```

```sql
CREATE TABLE IF NOT EXISTS memory_entity (
    entity_id       TEXT PRIMARY KEY,
    kind            TEXT NOT NULL,
    name            TEXT NOT NULL,
    canonical_key   TEXT NOT NULL UNIQUE,
    summary         TEXT,
    summary_hash    TEXT,
    confidence      REAL NOT NULL DEFAULT 1.0,
    scope_kind      TEXT NOT NULL,
    scope_payload   TEXT,
    provenance      TEXT NOT NULL,
    first_seen_at   INTEGER NOT NULL,
    last_updated_at INTEGER NOT NULL,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS memory_entity_alias (
    alias           TEXT NOT NULL,
    entity_id       TEXT NOT NULL,
    created_at      INTEGER NOT NULL,
    PRIMARY KEY (alias, entity_id),
    FOREIGN KEY (entity_id) REFERENCES memory_entity(entity_id)
);
```

```sql
CREATE TABLE IF NOT EXISTS memory_claim (
    claim_id             TEXT PRIMARY KEY,
    claim_kind           TEXT NOT NULL,
    status               TEXT NOT NULL DEFAULT 'active',

    statement            TEXT NOT NULL,
    statement_hash       TEXT NOT NULL,
    predicate            TEXT,
    confidence           REAL NOT NULL,

    scope_kind           TEXT NOT NULL,
    scope_payload        TEXT,

    subject_entity_id    TEXT,
    object_entity_id     TEXT,

    valid_from           INTEGER,
    valid_to             INTEGER,

    created_by_trace_id  TEXT NOT NULL,
    updated_by_trace_id  TEXT NOT NULL,

    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL,

    FOREIGN KEY (subject_entity_id) REFERENCES memory_entity(entity_id),
    FOREIGN KEY (object_entity_id) REFERENCES memory_entity(entity_id)
);

CREATE INDEX idx_claim_kind ON memory_claim(claim_kind);
CREATE INDEX idx_claim_status ON memory_claim(status);
CREATE INDEX idx_claim_subject ON memory_claim(subject_entity_id);
CREATE INDEX idx_claim_object ON memory_claim(object_entity_id);
CREATE INDEX idx_claim_predicate ON memory_claim(predicate);

-- Link claims to their source episodes
CREATE TABLE IF NOT EXISTS memory_claim_source (
    claim_id    TEXT NOT NULL,
    episode_id  TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    PRIMARY KEY (claim_id, episode_id),
    FOREIGN KEY (claim_id) REFERENCES memory_claim(claim_id),
    FOREIGN KEY (episode_id) REFERENCES memory_episode(episode_id)
);
```

### 4.6 Migration Table

```sql
CREATE TABLE IF NOT EXISTS openwand_migration (
    version     INTEGER PRIMARY KEY,
    name        TEXT NOT NULL,
    checksum    TEXT NOT NULL,
    dirty       INTEGER NOT NULL DEFAULT 0,
    applied_at  INTEGER NOT NULL
);
```

---

## Trace Append Semantics

Trace append runs in a write-serialized critical section on the blocking worker:

```text
1. Check idempotency_key.
2. If key exists, return existing TraceId.
3. Load current global_sequence.
4. Load current stream_sequence for this stream.
5. Load previous stream entry hash (for hash chain).
6. Assign new TraceId.
7. Serialize event payload to stable JSON.
8. Compute entry_hash (blake3 of canonical representation).
9. INSERT trace_entry.
10. INSERT trace_relation rows.
11. Commit transaction.
12. Return TraceId.
```

Projection does **not** run inside the trace transaction:

```text
Trace append transaction
  → succeeds → trace entry is authoritative

Projection transaction (separate)
  → succeeds → checkpoint advances
  → fails    → checkpoint does NOT advance, projection_error recorded
```

This matches the failure taxonomy: trace failure = hard stop. Projection failure = recoverable.

---

## Trace Replay

Store provides ordered trace replay. Session uses this to rebuild Loro. Memory uses this to rebuild projections.

```rust
pub trait TraceReplayStore: Send + Sync {
    /// Replay a trace stream from a given sequence number.
    /// Returns a stream of trace entries in order.
    async fn replay_stream(
        &self,
        stream_id: &TraceStreamId,
        from_sequence: Option<u64>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<TraceEntry<OpenWandTraceEvent>>> + Send>>>;

    /// Replay all entries for a given event kind.
    async fn replay_by_kind(
        &self,
        event_kind: &str,
        from_global_sequence: Option<u64>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<TraceEntry<OpenWandTraceEvent>>> + Send>>>;
}
```

Session rebuild:

```rust
// In openwand-session, NOT in openwand-store
pub async fn rebuild_loro_from_trace(
    replay: &dyn TraceReplayStore,
    blob: &dyn BlobStore,
    session_id: &SessionId,
) -> Result<LoroDoc> {
    let doc = LoroDoc::new();
    let mut state = LoroState::new(&doc);

    let stream = replay.replay_stream(
        &TraceStreamId { scope: Session, id: session_id.to_string() },
        None,
    ).await?;

    pin_mut!(stream);
    while let Some(entry) = stream.next().await {
        let entry = entry?;
        match &entry.event {
            OpenWandTraceEvent::Session(e) => state.apply_session_event(e, &entry)?,
            OpenWandTraceEvent::Inference(e) => state.apply_inference_event(e, &entry, blob)?,
            OpenWandTraceEvent::Tool(e) => state.apply_tool_event(e, &entry)?,
            OpenWandTraceEvent::Memory(e) => state.apply_memory_event(e, &entry)?,
            _ => {} // other event families don't affect Loro session state
        }
    }

    Ok(doc)
}
```

Store doesn't know Loro. Session doesn't know storage internals. Clean separation.

---

## Projection Checkpointing

Store owns the checkpoint table. Domain crates own projector logic.

```rust
pub trait ProjectionCheckpointStore: Send + Sync {
    async fn get_checkpoint(&self, projector: &str) -> Result<Option<ProjectionCheckpoint>>;
    async fn advance_checkpoint(&self, projector: &str, trace_id: &TraceId, seq: u64) -> Result<()>;
    async fn record_projection_error(&self, error: ProjectionErrorRecord) -> Result<()>;
    async fn projection_lag(&self) -> Result<Vec<ProjectionLag>>;
}

#[derive(Debug, Clone)]
pub struct ProjectionCheckpoint {
    pub projector_name: String,
    pub last_global_sequence: u64,
    pub last_trace_id: Option<TraceId>,
    pub error_count: u32,
    pub last_error: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ProjectionErrorRecord {
    pub projector_name: String,
    pub trace_id: Option<TraceId>,
    pub global_sequence: Option<u64>,
    pub error_message: String,
    pub recoverable: bool,
}

#[derive(Debug, Clone)]
pub struct ProjectionLag {
    pub projector_name: String,
    pub current_global_sequence: u64,
    pub checkpoint_sequence: u64,
    pub entries_behind: u64,
}
```

Projector ownership:

```text
session projector  → openwand-session (applies trace events to Loro)
memory projector   → openwand-memory (extracts entities/claims from episodes)
workflow projector → openwand-workflow (derives workflow state from events)
store              → checkpoint persistence + replay cursor + error recording
```

---

## Memory Write Boundary

### MemoryReadStore (Public)

Public consumers (session, app) get read access only:

```rust
// Defined in openwand-memory, implemented in openwand-store
pub trait MemoryReadStore: Send + Sync {
    async fn get_episode(&self, id: &EpisodeId) -> Result<Option<Episode>>;
    async fn search_hybrid(&self, query: HybridQuery) -> Result<SearchResult>;
    async fn get_entity(&self, id: &EntityId) -> Result<Option<Entity>>;
    async fn get_entity_by_key(&self, canonical_key: &str) -> Result<Option<Entity>>;
    async fn active_facts_about(
        &self,
        entity: &EntityId,
        predicate: &Predicate,
        as_of: DateTime<Utc>,
    ) -> Result<Vec<Fact>>;
    async fn decision_history(&self, entity: &EntityId) -> Result<Vec<Decision>>;
}
```

### MemoryProjectionStore (Internal)

Only the memory pipeline/projector gets write access. Writes are always trace-backed:

```rust
// Defined in openwand-memory, implemented in openwand-store
pub trait MemoryProjectionStore: Send + Sync {
    /// Apply a trace event to memory projections.
    /// This is the ONLY way to mutate memory state.
    async fn apply_memory_event(
        &self,
        entry: &TraceEntry<OpenWandTraceEvent>,
        relations: &[TraceRelation],
    ) -> Result<()>;

    /// Clear all memory projections for rebuild.
    async fn clear_memory_projection(&self) -> Result<()>;
}
```

This prevents accidental direct writes to derived memory state. Every mutation comes from a trace event.

---

## Content-Addressed Blobs

Episode text cannot be only a hash. Rebuild-from-trace needs actual content.

```rust
pub trait BlobStore: Send + Sync {
    async fn put(&self, hash: &str, media_type: &str, bytes: &[u8]) -> Result<()>;
    async fn get(&self, hash: &str) -> Result<Option<Vec<u8>>>;
    async fn exists(&self, hash: &str) -> Result<bool>;
    async fn size(&self, hash: &str) -> Result<Option<u64>>;
}
```

Write path:

```text
Session records episode:
  1. Compute blake3 hash of text
  2. Store blob: BlobStore::put(hash, "text/plain", text_bytes)
  3. Emit MemoryEvent::EpisodeRecorded { episode_id, text_hash, content_ref: Some(hash) }
  4. Append to trace
```

Rebuild path:

```text
Rebuild from trace:
  1. Read trace entry → MemoryEvent::EpisodeRecorded { content_ref: Some(hash), ... }
  2. BlobStore::get(hash) → actual text bytes
  3. Memory projector rebuilds episode from content
```

### Core Crate Addendum

`MemoryEvent::EpisodeRecorded` needs an additive field. This goes into `openwand-core`:

```rust
pub enum MemoryEvent {
    EpisodeRecorded {
        episode_id: EpisodeId,
        episode_kind: String,
        text_hash: String,
        content_ref: Option<String>,   // ← NEW, additive, default None
    },
    // ...
}
```

Old events have `content_ref: None`. New events have `content_ref: Some(hash)`. No migration needed.

---

## Snapshot Blob Store

Store persists opaque blobs. Session handles Loro ↔ bytes conversion.

```rust
pub trait SnapshotBlobStore: Send + Sync {
    async fn save_snapshot_blob(
        &self,
        key: &str,
        bytes: &[u8],
        media_type: &str,
        metadata: Option<&serde_json::Value>,
    ) -> Result<()>;

    async fn load_snapshot_blob(&self, key: &str) -> Result<Option<SnapshotData>>;
}

pub struct SnapshotData {
    pub bytes: Vec<u8>,
    pub media_type: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

Session uses:

```rust
// In openwand-session
let snapshot = doc.export(ExportMode::Snapshot);
store.save_snapshot_blob(
    &format!("session:{}", session_id.as_str()),
    &snapshot,
    "application/x-loro-snapshot",
    None,
).await?;

// Load
let data = store.load_snapshot_blob(&format!("session:{}", session_id.as_str())).await?;
let doc = LoroDoc::from_snapshot(&data.unwrap().bytes);
```

Store doesn't know Loro. Just bytes.

---

## Migration Runner

```rust
pub trait StoreMigrationRunner: Send + Sync {
    async fn initialize(&self) -> Result<()>;
    async fn migrate(&self) -> Result<()>;
    async fn current_version(&self) -> Result<u32>;
    async fn is_dirty(&self) -> Result<bool>;
}
```

Rules:

1. `initialize()` creates the migration table if absent.
2. `migrate()` runs pending migrations in order.
3. Before each migration: mark `dirty = 1`.
4. After success: insert row with `dirty = 0`.
5. On startup: refuse if any row has `dirty = 1`.
6. Store checksum. Detect edited historical migrations.
7. Forward-only for production. Down migrations are test-only.
8. Trace entries are never rewritten during normal migrations.
9. Additive-only schema changes. New tables, new nullable columns, new indexes.

---

## Hash Chain and Integrity

```rust
pub trait IntegrityStore: Send + Sync {
    /// Verify hash chain for a stream.
    /// Returns Ok(()) if chain is intact, Err with break point if not.
    async fn verify_stream_chain(&self, stream_id: &TraceStreamId) -> Result<IntegrityReport>;

    /// Verify all streams.
    async fn verify_all_chains(&self) -> Result<Vec<IntegrityReport>>;
}

pub struct IntegrityReport {
    pub stream_id: TraceStreamId,
    pub entries_checked: u64,
    pub chain_intact: bool,
    pub break_point: Option<TraceId>,
    pub break_reason: Option<String>,
}
```

Each trace entry includes `prev_hash` and `entry_hash`. The chain is:

```text
entry_hash = blake3(prev_hash || stream_id || stream_sequence || event_payload)
```

Verification: walk the chain, confirm each entry's `prev_hash` matches the previous entry's `entry_hash`.

---

## Hybrid Search (v2 Contract)

Batch 1 uses FTS5 only. But the backend search primitive interface is designed for future vector + graph expansion:

```rust
pub trait BackendSearchPrimitives: Send + Sync {
    async fn fts_candidates(&self, query: &str, limit: usize) -> Result<Vec<ScoredId>>;
    async fn vector_candidates(&self, embedding: &[f32], limit: usize) -> Result<Vec<ScoredId>>;
    async fn graph_expand(&self, seeds: &[String], hops: u8) -> Result<Vec<ScoredId>>;
}

pub struct ScoredId {
    pub id: String,
    pub score: f64,
}
```

Shared Rust code handles fusion:

```text
RRF (Reciprocal Rank Fusion) of FTS + vector + graph candidates
  → scope boost
  → recency boost
  → confidence boost
  → graph-distance penalty
  → deduplication
  → final top-k
```

This keeps scoring backend-independent. Benchmarks compare storage engines, not ranking algorithms.

---

## Backend Conformance Tests

Every backend must pass the same tests:

### TraceStore Conformance

```text
append assigns TraceId
global_sequence is monotonic
stream_sequence is monotonic per stream
idempotency_key deduplicates
relations are persisted
scan filters by stream / event_kind / time range
hash chain verifies
concurrent append does not duplicate sequence numbers
read by ID returns correct entry
```

### MemoryReadStore Conformance

```text
episode projection can be read after trace event
chunk projection can be searched via FTS
active_facts_about respects valid_from / valid_to
decision_history returns supersession chain
entity lookup by canonical_key works
```

### Projection Conformance

```text
checkpoint advances only after successful apply
failed projection records projection_error
rebuild produces identical projection rows
lag is observable after burst append
```

### Migration Conformance

```text
initialize creates schema
migrate is idempotent
dirty migration fails startup
adding nullable/defaulted field preserves old data
checksum detects edited historical migrations
```

### Rebuild Conformance

```text
replay_stream returns entries in stream_sequence order
replay_stream from_sequence skips earlier entries
replay_by_kind filters correctly
rebuild from empty trace produces empty Loro
rebuild from populated trace produces correct Loro state
```

---

## Trace Compaction

**Batch 1: No compaction.** Trace is the authoritative audit log.

What can be compacted (later):

```text
Loro snapshots (keep latest + periodic checkpoints)
memory chunks (rebuildable from trace)
retrieval indexes (rebuildable from trace)
large tool output blobs (content-addressed, deduplicated)
```

What must NEVER be compacted without explicit user consent:

```text
trace_entry rows
trace_relation rows
```

Trace compaction can only be revisited after:

1. Export/import is working
2. Integrity verification is working
3. Legal/audit semantics are settled
4. User explicitly opts in

---

## Implementation Batches

### Batch 1 — SQLite TraceStore

```text
Deliver:
  SQLite schema migrations
  trace_entry / trace_relation / trace_blob
  TraceStore<OpenWandTraceEvent>
  idempotency
  hash chain
  scan/query APIs
  integrity verification
  conformance tests

Success criterion:
  Session can append trace entries for read-only tool use.
  Every durable message/tool result has a trace_id.
  Reload can scan trace and reconstruct session-relevant history.
```

### Batch 2 — SQLite Projection Substrate

```text
Deliver:
  projection_checkpoint
  projection_error
  ProjectionCheckpointStore
  TraceReplayStore
  SnapshotBlobStore
  projection lag query
  rebuild conformance tests

Success criterion:
  A failed projection never rolls back trace.
  Restart can resume projection from last checkpoint.
  Session can rebuild Loro from trace replay.
```

### Batch 3 — SQLite Memory Projection

```text
Deliver:
  memory_episode
  memory_chunk + FTS5
  memory_entity
  memory_claim
  MemoryReadStore
  MemoryProjectionStore
  BlobStore
  hybrid search (FTS5 only)
  memory conformance tests

Success criterion:
  Memory consumes trace-backed episodes and supports retrieval.
```

### Batch 4 — CozoDB Backend

```text
Deliver:
  TraceStore conformance
  MemoryReadStore conformance
  projection checkpointing
  graph-heavy query implementation
  benchmark matrix entry
```

### Batch 5 — SurrealDB Backend

```text
Deliver:
  TraceStore conformance
  MemoryReadStore conformance
  projection checkpointing
  SurrealQL implementation
  benchmark matrix entry
```

### Batch 6 — Backend Decision

```text
Apply benchmark elimination:
  1. Eliminate any backend that fails must-pass gates.
  2. Rank survivors by: startup latency, insert latency, hybrid search quality,
     binary size, build time, memory usage.
  3. If tied, choose smaller dependency tree.
```

---

## Benchmark Gates

From the memory design benchmark plan, extended for full store:

| Metric | Must Pass |
|---|---|
| Startup (cold) | < 200ms |
| Trace append p50 | < 1ms |
| Trace append p99 | < 5ms |
| Trace append + relation | < 2ms p50 |
| Episode insert | < 1ms |
| Chunk insert | < 5ms |
| FTS search (Batch 1) | < 50ms |
| Hybrid search (v2) | < 100ms |
| Rebuild 1K entries | < 500ms |
| Rebuild 10K entries | < 5s |
| Binary size | < 15MB (SQLite only) |
| External dependencies | 0 (embedded only) |
| Migration safety | No data loss on upgrade |
| Concurrent append correctness | No duplicate sequences |

---

## Core Crate Addendum

One additive field needed in `openwand-core`:

```rust
// In MemoryEvent::EpisodeRecorded — add content_ref field
pub enum MemoryEvent {
    EpisodeRecorded {
        episode_id: EpisodeId,
        episode_kind: String,
        text_hash: String,
        content_ref: Option<String>,   // ← NEW, additive, default None
    },
    // ... rest unchanged
}
```

Old events have `content_ref: None`. New events have `content_ref: Some(blob_hash)`. No migration needed. No breakage.

---

## Summary

| Aspect | Batch 1 | v2+ |
|---|---|---|
| Backend | SQLite only | SQLite vs CozoDB vs SurrealDB benchmark |
| Trace | Full append + query + hash chain + integrity | Compaction (only after export/import/integrity) |
| Blobs | Content-addressed immutable store | Compression, deduplication |
| Projections | Checkpoint table, error recording | Full rebuild, lag monitoring |
| Memory read | FTS5 text search | FTS5 + vector + graph hybrid |
| Memory write | `MemoryProjectionStore` (trace-backed only) | Same |
| Snapshots | Opaque blobs | Loro-optimized snapshots |
| Replay | Ordered stream replay for session rebuild | Same |
| Connection | Single blocking worker | Same (single-user desktop) |
| Migrations | Custom runner with dirty flag + checksum | Same |
