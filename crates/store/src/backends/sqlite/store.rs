//! SQLite-backed trace store.
//!
//! The main entry point for persistent trace storage.
//! Uses a serialized blocking writer for all operations.

use async_trait::async_trait;
use openwand_trace::{
    AppendTraceEntry, RelationQuery, TraceEntry, TraceEntryWithRelations, TraceError, TraceId,
    TracePage, TraceQuery, TraceRelation, TraceStore,
};
use crate::StoredEvent;
use std::path::{Path, PathBuf};

use crate::backends::sqlite::writer::SqliteWriter;
use crate::error::StoreError;

/// Configuration for opening a SQLite store.
#[derive(Debug, Clone)]
pub struct SqliteStoreConfig {
    /// Path to the database file.
    pub path: PathBuf,
    /// Whether to run migrations on open.
    pub run_migrations: bool,
}

impl SqliteStoreConfig {
    /// Config for a file-backed store.
    pub fn file(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            run_migrations: true,
        }
    }

    /// Config for an in-memory database (testing).
    pub fn in_memory() -> Self {
        Self {
            path: ":memory:".into(),
            run_migrations: true,
        }
    }
}

/// SQLite-backed trace store.
pub struct SqliteStore {
    pub(crate) writer: SqliteWriter,
}

impl SqliteStore {
    /// Open a SQLite store with the given config.
    pub async fn open(config: SqliteStoreConfig) -> Result<Self, StoreError> {
        // Ensure parent directory exists
        if config.path != PathBuf::from(":memory:") {
            if let Some(parent) = config.path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| StoreError::Database(format!("create dir: {e}")))?;
            }
        }

        let writer = SqliteWriter::open(config.path, config.run_migrations)?;
        Ok(Self { writer })
    }

    /// Open an in-memory database (for tests).
    pub async fn open_in_memory() -> Result<Self, StoreError> {
        Self::open(SqliteStoreConfig::in_memory()).await
    }

    /// Open a temporary database in a temp directory (for tests).
    pub async fn open_in_temp_dir() -> Result<Self, StoreError> {
        let dir = std::env::temp_dir().join(format!("openwand-test-{}", ulid::Ulid::new()));
        let path = dir.join("trace.db");
        Self::open(SqliteStoreConfig::file(&path)).await
    }

    /// Path to the database file.
    pub fn path(&self) -> &Path {
        self.writer.path()
    }

    /// Shut down the writer thread.
    pub fn shutdown(&self) -> Result<(), StoreError> {
        self.writer.shutdown()
    }
}

#[async_trait]
impl TraceStore<StoredEvent> for SqliteStore {
    async fn append(
        &self,
        command: AppendTraceEntry<StoredEvent>,
    ) -> Result<TraceId, TraceError> {
        self.writer
            .append(command)
            .await
            .map_err(|e| TraceError::Storage(e.to_string()))
    }

    async fn append_and_project(
        &self,
        command: AppendTraceEntry<StoredEvent>,
        _projectors: &[&str],
    ) -> Result<TraceId, TraceError> {
        // 01e has no projection support — just append
        self.append(command).await
    }

    async fn get(&self, id: TraceId) -> Result<Option<TraceEntry<StoredEvent>>, TraceError> {
        self.writer
            .get(id)
            .await
            .map_err(|e| TraceError::Storage(e.to_string()))
    }

    async fn get_with_relations(
        &self,
        id: TraceId,
    ) -> Result<Option<TraceEntryWithRelations<StoredEvent>>, TraceError> {
        self.writer
            .get_with_relations(id)
            .await
            .map_err(|e| TraceError::Storage(e.to_string()))
    }

    async fn scan(&self, query: TraceQuery) -> Result<TracePage<StoredEvent>, TraceError> {
        self.writer
            .scan(query)
            .await
            .map_err(|e| TraceError::Storage(e.to_string()))
    }

    async fn scan_relations(
        &self,
        query: RelationQuery,
    ) -> Result<Vec<TraceRelation>, TraceError> {
        let kind_str = query.kind.map(|k| {
            serde_json::to_string(&k).unwrap_or_else(|_| format!("{k:?}"))
        });
        self.writer
            .scan_relations(query.from, query.to, kind_str)
            .await
            .map_err(|e| TraceError::Storage(e.to_string()))
    }

    async fn current_global_sequence(&self) -> Result<u64, TraceError> {
        self.writer
            .current_global_sequence()
            .await
            .map_err(|e| TraceError::Storage(e.to_string()))
    }

    async fn current_stream_sequence(
        &self,
        stream_id: &openwand_trace::TraceStreamId,
    ) -> Result<u64, TraceError> {
        self.writer
            .current_stream_sequence(stream_id.clone())
            .await
            .map_err(|e| TraceError::Storage(e.to_string()))
    }

    async fn initialize(&self) -> Result<(), TraceError> {
        self.writer
            .initialize()
            .await
            .map_err(|e| TraceError::Storage(e.to_string()))
    }

    async fn rebuild_projection(
        &self,
        _projector_name: &str,
        _from: Option<TraceId>,
    ) -> Result<(), TraceError> {
        // 01e has no projection support
        Ok(())
    }
}
