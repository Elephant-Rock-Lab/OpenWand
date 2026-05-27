//! SQLite implementation of SessionRegistryStore.
//!
//! Uses direct connections (not the writer channel) because the registry is
//! lightweight navigation metadata, not high-throughput trace data.
//! WAL mode allows safe concurrent reads while the writer thread handles trace.

use crate::error::StoreError;
use crate::registry::{NewSessionRecord, SessionListFilter, SessionRecord, SessionRegistryUpdate, SessionSummary};
use crate::registry_store::SessionRegistryStore;

use super::store::SqliteStore;

impl SessionRegistryStore for SqliteStore {
    fn create_session(&self, record: NewSessionRecord) -> Result<SessionRecord, StoreError> {
        let conn = self.open_writer_connection()?;
        let now = chrono::Utc::now().timestamp();
        let session_id = record.session_id.clone();

        conn.execute(
            "INSERT INTO session_registry
                (session_id, title, status, created_at, updated_at,
                 provider, model, base_url, working_directory,
                 interaction_mode, current_step, projection_stale)
             VALUES (?1, ?2, 'active', ?3, ?3, ?4, ?5, ?6, ?7, ?8, 0, 0)",
            rusqlite::params![
                record.session_id,
                record.title,
                now,
                record.provider,
                record.model,
                record.base_url,
                record.working_directory,
                record.interaction_mode,
            ],
        ).map_err(|e| StoreError::Write { message: format!("create_session: {e}") })?;

        drop(conn);
        self.get_session(&session_id)?
            .ok_or_else(|| StoreError::Write {
                message: "create_session: failed to read back".into(),
            })
    }

    fn get_session(&self, session_id: &str) -> Result<Option<SessionRecord>, StoreError> {
        let conn = self.open_reader_connection()?;
        let mut stmt = conn.prepare(
            "SELECT session_id, title, status, created_at, updated_at, last_opened_at,
                    provider, model, base_url, working_directory,
                    interaction_mode, current_phase, current_step,
                    last_message_preview, last_trace_id, last_global_sequence,
                    snapshot_key, projection_stale, metadata_json
             FROM session_registry WHERE session_id = ?1"
        ).map_err(|e| StoreError::Read { message: format!("get_session prepare: {e}") })?;

        let result = stmt.query_row(rusqlite::params![session_id], |row| {
            Ok(SessionRecord {
                session_id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                last_opened_at: row.get(5)?,
                provider: row.get(6)?,
                model: row.get(7)?,
                base_url: row.get(8)?,
                working_directory: row.get(9)?,
                interaction_mode: row.get(10)?,
                current_phase: row.get(11)?,
                current_step: row.get(12)?,
                last_message_preview: row.get(13)?,
                last_trace_id: row.get(14)?,
                last_global_sequence: row.get(15)?,
                snapshot_key: row.get(16)?,
                projection_stale: row.get::<_, i64>(17)? != 0,
                metadata_json: row.get(18)?,
            })
        });

        match result {
            Ok(record) => Ok(Some(record)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StoreError::Read { message: format!("get_session: {e}") }),
        }
    }

    fn list_sessions(&self, filter: SessionListFilter) -> Result<Vec<SessionSummary>, StoreError> {
        let conn = self.open_reader_connection()?;
        let limit = filter.limit.unwrap_or(100);

        let sql = if filter.include_archived {
            "SELECT session_id, title, status, model, created_at, updated_at,
                    last_message_preview, current_phase
             FROM session_registry
             ORDER BY updated_at DESC, rowid DESC
             LIMIT ?1"
        } else {
            "SELECT session_id, title, status, model, created_at, updated_at,
                    last_message_preview, current_phase
             FROM session_registry
             WHERE status != 'archived'
             ORDER BY updated_at DESC, rowid DESC
             LIMIT ?1"
        };

        let mut stmt = conn.prepare(sql)
            .map_err(|e| StoreError::Read { message: format!("list_sessions prepare: {e}") })?;

        let rows = stmt.query_map(rusqlite::params![limit], |row| {
            Ok(SessionSummary {
                session_id: row.get(0)?,
                title: row.get(1)?,
                status: row.get(2)?,
                model: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                last_message_preview: row.get(6)?,
                current_phase: row.get(7)?,
            })
        }).map_err(|e| StoreError::Read { message: format!("list_sessions query: {e}") })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| StoreError::Read { message: format!("list_sessions row: {e}") })?);
        }
        Ok(results)
    }

    fn update_session(&self, update: SessionRegistryUpdate) -> Result<(), StoreError> {
        let conn = self.open_writer_connection()?;
        let now = chrono::Utc::now().timestamp();

        // Build dynamic SET clause for non-None fields
        let mut sets = vec!["updated_at = ?1".to_string()];
        let mut param_idx = 2u32;
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(now)];

        macro_rules! maybe_set {
            ($field:ident, $col:literal) => {
                if let Some(ref val) = update.$field {
                    sets.push(format!("{} = ?{}", $col, param_idx));
                    params.push(Box::new(val.clone()));
                    param_idx += 1;
                }
            };
        }

        maybe_set!(title, "title");
        maybe_set!(status, "status");
        maybe_set!(current_phase, "current_phase");
        maybe_set!(current_step, "current_step");
        maybe_set!(last_message_preview, "last_message_preview");
        maybe_set!(last_trace_id, "last_trace_id");
        maybe_set!(last_global_sequence, "last_global_sequence");
        maybe_set!(snapshot_key, "snapshot_key");
        maybe_set!(metadata_json, "metadata_json");

        if let Some(stale) = update.projection_stale {
            sets.push(format!("projection_stale = ?{}", param_idx));
            params.push(Box::new(stale as i64));
            param_idx += 1;
        }

        let sql = format!(
            "UPDATE session_registry SET {} WHERE session_id = ?{}",
            sets.join(", "),
            param_idx
        );
        params.push(Box::new(update.session_id.clone()));

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let changes = conn.execute(&sql, param_refs.as_slice())
            .map_err(|e| StoreError::Write { message: format!("update_session: {e}") })?;

        if changes == 0 {
            return Err(StoreError::Read {
                message: format!("update_session: session {} not found", update.session_id),
            });
        }

        Ok(())
    }

    fn archive_session(&self, session_id: &str) -> Result<(), StoreError> {
        let conn = self.open_writer_connection()?;
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "UPDATE session_registry SET status = 'archived', updated_at = ?1 WHERE session_id = ?2",
            rusqlite::params![now, session_id],
        ).map_err(|e| StoreError::Write { message: format!("archive_session: {e}") })?;
        Ok(())
    }
}

/// Helper methods on SqliteStore for direct connection access (registry operations).
impl SqliteStore {
    /// Open a read-only connection (WAL mode allows concurrent reads).
    fn open_reader_connection(&self) -> Result<rusqlite::Connection, StoreError> {
        let path = self.writer.path();
        let conn = rusqlite::Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        ).map_err(|e| StoreError::Database(format!("open reader: {e}")))?;
        Ok(conn)
    }

    /// Open a write connection for registry operations.
    /// This bypasses the writer thread — safe because registry writes are infrequent.
    fn open_writer_connection(&self) -> Result<rusqlite::Connection, StoreError> {
        let path = self.writer.path();
        let conn = rusqlite::Connection::open(path)
            .map_err(|e| StoreError::Database(format!("open writer: {e}")))?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;")
            .map_err(|e| StoreError::Database(format!("pragmas: {e}")))?;
        Ok(conn)
    }
}
