//! SQLite schema — 01e minimal trace subset.
//!
//! `trace_blob` is schema reservation only (no read/write code paths in 01e).

/// Checksum of the migration SQL for integrity tracking.
/// Computed as SHA-256 of the raw SQL text.
pub const MIGRATION_0001_CHECKSUM: &str = "sha256:1aed65b7d58cd04ae290eb047bf39cea35428b8acd30f6bf5f6be5d1b9286715";

/// Migration 0001: trace tables.
pub const MIGRATION_0001_SQL: &str = r#"
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

    UNIQUE(stream_scope, stream_id, stream_sequence),
    UNIQUE(global_sequence)
);

CREATE INDEX IF NOT EXISTS idx_trace_stream
    ON trace_entry(stream_scope, stream_id, stream_sequence);

CREATE INDEX IF NOT EXISTS idx_trace_kind
    ON trace_entry(event_kind);

CREATE INDEX IF NOT EXISTS idx_trace_occurred
    ON trace_entry(occurred_at);

CREATE UNIQUE INDEX IF NOT EXISTS idx_trace_idempotency
    ON trace_entry(idempotency_key)
    WHERE idempotency_key IS NOT NULL;

CREATE TABLE IF NOT EXISTS trace_relation (
    from_trace_id TEXT NOT NULL,
    to_trace_id   TEXT NOT NULL,
    kind          TEXT NOT NULL,
    created_at    INTEGER NOT NULL,

    PRIMARY KEY (from_trace_id, to_trace_id, kind),
    FOREIGN KEY (from_trace_id) REFERENCES trace_entry(id),
    FOREIGN KEY (to_trace_id) REFERENCES trace_entry(id)
);

CREATE INDEX IF NOT EXISTS idx_relation_from_kind
    ON trace_relation(from_trace_id, kind);

CREATE INDEX IF NOT EXISTS idx_relation_to_kind
    ON trace_relation(to_trace_id, kind);

CREATE INDEX IF NOT EXISTS idx_relation_kind
    ON trace_relation(kind);

CREATE TABLE IF NOT EXISTS trace_blob (
    hash          TEXT PRIMARY KEY,
    media_type    TEXT NOT NULL,
    bytes         BLOB NOT NULL,
    size_bytes    INTEGER NOT NULL,
    compression   TEXT,
    created_at    INTEGER NOT NULL
);
"#;
