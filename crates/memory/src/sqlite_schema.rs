//! SQLite schema for memory tables.
//!
//! Runs as a separate migration namespace from the store crate's tables.
//! Shares the same SQLite file via WAL concurrent access.

/// Checksum for memory migration 0001.
pub const MEMORY_MIGRATION_0001_CHECKSUM: &str = "sha256:pending";

/// Migration 0001: memory tables.
pub const MEMORY_MIGRATION_0001_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS memory_episode (
    episode_id       TEXT PRIMARY KEY,
    source_trace_id  TEXT NOT NULL UNIQUE,
    session_id       TEXT NOT NULL,
    event_kind       TEXT NOT NULL,
    role             TEXT NOT NULL,
    content          TEXT NOT NULL,
    created_at       INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memory_episode_session
    ON memory_episode(session_id);

CREATE TABLE IF NOT EXISTS memory_record (
    record_id        TEXT PRIMARY KEY,
    kind             TEXT NOT NULL,
    claim            TEXT NOT NULL,
    confidence_bps   INTEGER NOT NULL,
    status           TEXT NOT NULL DEFAULT 'active',
    valid_from       INTEGER,
    valid_until      INTEGER,
    superseded_by    TEXT,
    created_at       INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memory_record_status
    ON memory_record(status);

CREATE TABLE IF NOT EXISTS memory_record_source (
    record_id        TEXT NOT NULL,
    episode_id       TEXT NOT NULL,
    source_trace_id  TEXT NOT NULL,
    PRIMARY KEY (record_id, episode_id)
);

CREATE INDEX IF NOT EXISTS idx_memory_source_record
    ON memory_record_source(record_id);

CREATE INDEX IF NOT EXISTS idx_memory_source_episode
    ON memory_record_source(episode_id);

CREATE TABLE IF NOT EXISTS memory_projection_checkpoint (
    session_id       TEXT PRIMARY KEY,
    last_global_sequence INTEGER NOT NULL DEFAULT 0,
    updated_at       INTEGER NOT NULL
);
"#;
