//! Serialized SQLite writer worker.
//!
//! Single `spawn_blocking` task with one `rusqlite::Connection`.
//! All writes go through `mpsc::Sender<WriterCommand>`.
//! Reads use a separate reader connection (SQLite WAL allows concurrent reads).

use openwand_core::OpenWandTraceEvent;
use crate::StoredEvent;
use openwand_trace::stream::EntryHash;
use openwand_trace::{
    Actor, AppendTraceEntry, TraceEntry, TraceEntryWithRelations, TraceId,
    TracePage, TraceQuery, TraceRelation, TraceStreamId,
};
use rusqlite::Connection;
use std::path::PathBuf;
use tokio::sync::oneshot;

use crate::backends::sqlite::hash;
use crate::backends::sqlite::migrations;
use crate::error::StoreError;

/// Commands sent to the blocking writer thread.
enum WriterCommand {
    Append {
        entry: AppendTraceEntry<StoredEvent>,
        reply: oneshot::Sender<Result<TraceId, StoreError>>,
    },
    Get {
        id: TraceId,
        reply: oneshot::Sender<Result<Option<TraceEntry<StoredEvent>>, StoreError>>,
    },
    GetWithRelations {
        id: TraceId,
        reply: oneshot::Sender<Result<Option<TraceEntryWithRelations<StoredEvent>>, StoreError>>,
    },
    Scan {
        query: TraceQuery,
        reply: oneshot::Sender<Result<TracePage<StoredEvent>, StoreError>>,
    },
    ScanRelations {
        from: Option<TraceId>,
        to: Option<TraceId>,
        kind: Option<String>,
        reply: oneshot::Sender<Result<Vec<TraceRelation>, StoreError>>,
    },
    CurrentGlobalSequence {
        reply: oneshot::Sender<Result<u64, StoreError>>,
    },
    CurrentStreamSequence {
        stream_id: TraceStreamId,
        reply: oneshot::Sender<Result<u64, StoreError>>,
    },
    Initialize {
        reply: oneshot::Sender<Result<(), StoreError>>,
    },
    Shutdown,
}

/// Handle to the writer thread.
pub(crate) struct SqliteWriter {
    tx: std::sync::mpsc::Sender<WriterCommand>,
    path: PathBuf,
}

impl SqliteWriter {
    /// Open database, run migrations, spawn writer thread.
    pub fn open(path: PathBuf, run_migrations: bool) -> Result<Self, StoreError> {
        let conn = Connection::open(&path)
            .map_err(|e| StoreError::Database(format!("open {}: {e}", path.display())))?;

        // Pragmas
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;
             PRAGMA synchronous = NORMAL;",
        )
        .map_err(|e| StoreError::Database(format!("pragmas: {e}")))?;

        if run_migrations {
            migrations::run_migrations(&conn)
                .map_err(StoreError::Migration)?;
        }

        let (tx, rx) = std::sync::mpsc::channel::<WriterCommand>();

        // Spawn blocking writer
        std::thread::spawn(move || {
            writer_loop(conn, rx);
        });

        Ok(Self { tx, path })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Send a command and wait for the reply.
    async fn send<T>(
        &self,
        make_cmd: impl FnOnce(oneshot::Sender<Result<T, StoreError>>) -> WriterCommand,
    ) -> Result<T, StoreError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        let cmd = make_cmd(reply_tx);
        self.tx.send(cmd).map_err(|_| StoreError::WriterClosed)?;
        reply_rx.await.map_err(|_| StoreError::WriterClosed)?
    }

    pub async fn append(
        &self,
        entry: AppendTraceEntry<StoredEvent>,
    ) -> Result<TraceId, StoreError> {
        self.send(|reply| WriterCommand::Append { entry, reply }).await
    }

    pub async fn get(
        &self,
        id: TraceId,
    ) -> Result<Option<TraceEntry<StoredEvent>>, StoreError> {
        self.send(|reply| WriterCommand::Get { id, reply }).await
    }

    pub async fn get_with_relations(
        &self,
        id: TraceId,
    ) -> Result<Option<TraceEntryWithRelations<StoredEvent>>, StoreError> {
        self.send(|reply| WriterCommand::GetWithRelations { id, reply }).await
    }

    pub async fn scan(
        &self,
        query: TraceQuery,
    ) -> Result<TracePage<StoredEvent>, StoreError> {
        self.send(|reply| WriterCommand::Scan { query, reply }).await
    }

    pub async fn scan_relations(
        &self,
        from: Option<TraceId>,
        to: Option<TraceId>,
        kind: Option<String>,
    ) -> Result<Vec<TraceRelation>, StoreError> {
        self.send(|reply| WriterCommand::ScanRelations { from, to, kind, reply }).await
    }

    pub async fn current_global_sequence(&self) -> Result<u64, StoreError> {
        self.send(|reply| WriterCommand::CurrentGlobalSequence { reply }).await
    }

    pub async fn current_stream_sequence(
        &self,
        stream_id: TraceStreamId,
    ) -> Result<u64, StoreError> {
        self.send(|reply| WriterCommand::CurrentStreamSequence { stream_id, reply }).await
    }

    pub async fn initialize(&self) -> Result<(), StoreError> {
        self.send(|reply| WriterCommand::Initialize { reply }).await
    }

    /// Shut down the writer thread.
    pub fn shutdown(&self) -> Result<(), StoreError> {
        self.tx.send(WriterCommand::Shutdown).map_err(|_| StoreError::WriterClosed)
    }
}

fn writer_loop(mut conn: Connection, rx: std::sync::mpsc::Receiver<WriterCommand>) {
    while let Ok(cmd) = rx.recv() {
        match cmd {
            WriterCommand::Shutdown => break,

            WriterCommand::Append { entry, reply } => {
                let _ = reply.send(do_append(&mut conn, entry));
            }

            WriterCommand::Get { id, reply } => {
                let _ = reply.send(do_get(&conn, &id));
            }

            WriterCommand::GetWithRelations { id, reply } => {
                let _ = reply.send(do_get_with_relations(&conn, &id));
            }

            WriterCommand::Scan { query, reply } => {
                let _ = reply.send(do_scan(&conn, &query));
            }

            WriterCommand::ScanRelations { from, to, kind, reply } => {
                let _ = reply.send(do_scan_relations(&conn, &from, &to, &kind));
            }

            WriterCommand::CurrentGlobalSequence { reply } => {
                let _ = reply.send(do_current_global_sequence(&conn));
            }

            WriterCommand::CurrentStreamSequence { stream_id, reply } => {
                let _ = reply.send(do_current_stream_sequence(&conn, &stream_id));
            }

            WriterCommand::Initialize { reply } => {
                let _ = reply.send(Ok(())); // Migrations ran at open
            }
        }
    }
}

// ---- Implementation functions ----

fn do_append(
    conn: &mut Connection,
    command: AppendTraceEntry<StoredEvent>,
) -> Result<TraceId, StoreError> {
    let tx = conn
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(|e| StoreError::Database(format!("begin transaction: {e}")))?;

    // 1. Idempotency check
    if let Some(ref key) = command.idempotency_key {
        let existing: Option<String> = tx
            .query_row(
                "SELECT id FROM trace_entry WHERE idempotency_key = ?1",
                rusqlite::params![key.0],
                |row| row.get(0),
            )
            .ok();
        if let Some(id_str) = existing {
            return Ok(TraceId(id_str));
        }
    }

    // 2. Assign TraceId
    let id = TraceId::new();

    // 3. Current global sequence
    let global_seq: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(global_sequence), 0) FROM trace_entry",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        + 1;

    // 4. Current stream sequence
    let stream_scope = serde_json::to_string(&command.stream_id.scope)
        .map_err(|e| StoreError::Serialization(format!("stream_scope: {e}")))?;
    let stream_id_str = &command.stream_id.id;

    let stream_seq: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(stream_sequence), 0) FROM trace_entry WHERE stream_scope = ?1 AND stream_id = ?2",
            rusqlite::params![stream_scope, stream_id_str],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0)
        + 1;

    // 5. Previous hash for this stream
    let prev_hash: Option<String> = tx
        .query_row(
            "SELECT entry_hash FROM trace_entry WHERE stream_scope = ?1 AND stream_id = ?2 ORDER BY stream_sequence DESC LIMIT 1",
            rusqlite::params![stream_scope, stream_id_str],
            |row| row.get(0),
        )
        .ok();

    let prev_entry_hash = prev_hash.clone().map(EntryHash);

    // 6. Serialize event
    let event_kind = command.event.event_kind().to_owned();
    let event_schema_version = command.event.schema_version() as i64;
    let event_payload = serde_json::to_string(&command.event.0)
        .map_err(|e| StoreError::Serialization(format!("event payload: {e}")))?;

    // 7. Compute hash
    let entry_hash = hash::compute_entry_hash(
        global_seq as u64,
        &stream_scope,
        stream_id_str,
        stream_seq as u64,
        &event_kind,
        &event_payload,
        prev_entry_hash.as_ref(),
    );

    // 8. Serialize actor
    let actor_kind = serde_json::to_string(&command.actor)
        .map_err(|e| StoreError::Serialization(format!("actor: {e}")))?;

    let now_ts = chrono::Utc::now().timestamp();

    // 9. Insert trace_entry
    tx.execute(
        "INSERT INTO trace_entry (id, stream_scope, stream_id, stream_sequence, global_sequence,
         occurred_at, actor_kind, actor_payload, event_kind, event_payload,
         event_schema_version, trace_schema_version, prev_hash, entry_hash,
         idempotency_key, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8, ?9, ?10, 1, ?11, ?12, ?13, ?14)",
        rusqlite::params![
            id.0,
            stream_scope,
            stream_id_str,
            stream_seq,
            global_seq,
            now_ts,
            actor_kind,
            event_kind,
            event_payload,
            event_schema_version,
            prev_hash,
            entry_hash.0,
            command.idempotency_key.as_ref().map(|k| k.0.as_str()),
            now_ts,
        ],
    )
    .map_err(|e| StoreError::Database(format!("insert trace_entry: {e}")))?;

    // 10. Insert relations
    for draft in &command.relations {
        let kind_str = serde_json::to_string(&draft.kind)
            .map_err(|e| StoreError::Serialization(format!("relation kind: {e}")))?;
        tx.execute(
            "INSERT INTO trace_relation (from_trace_id, to_trace_id, kind, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![id.0, draft.to.0, kind_str, now_ts],
        )
        .map_err(|e| StoreError::Database(format!("insert trace_relation: {e}")))?;
    }

    // 11. Commit
    tx.commit()
        .map_err(|e| StoreError::Database(format!("commit: {e}")))?;

    Ok(id)
}

fn row_to_entry(row: &rusqlite::Row) -> Result<TraceEntry<StoredEvent>, StoreError> {
    let id: String = row.get(0)?;
    let stream_scope_str: String = row.get(1)?;
    let stream_id_str: String = row.get(2)?;
    let stream_sequence: i64 = row.get(3)?;
    let global_sequence: i64 = row.get(4)?;
    let occurred_at_ts: i64 = row.get(5)?;
    let actor_kind_str: String = row.get(6)?;
    let event_kind_str: String = row.get(8)?;
    let event_payload_str: String = row.get(9)?;
    let event_schema_version: i64 = row.get(10)?;
    let trace_schema_version: i64 = row.get(11)?;
    let prev_hash_str: Option<String> = row.get(12)?;
    let entry_hash_str: String = row.get(13)?;

    let actor: Actor = serde_json::from_str(&actor_kind_str)
        .map_err(|e| StoreError::Serialization(format!("deserialize actor: {e}")))?;
    let event: OpenWandTraceEvent = serde_json::from_str(&event_payload_str)
        .map_err(|e| StoreError::Serialization(format!("deserialize event: {e}")))?;
    let stream_scope: openwand_trace::TraceStreamScope = serde_json::from_str(&stream_scope_str)
        .map_err(|e| StoreError::Serialization(format!("deserialize stream scope: {e}")))?;

    let occurred_at = chrono::DateTime::from_timestamp(occurred_at_ts, 0)
        .unwrap_or_default();

    Ok(TraceEntry {
        id: TraceId(id),
        stream_id: TraceStreamId {
            scope: stream_scope,
            id: stream_id_str,
        },
        stream_sequence: stream_sequence as u64,
        global_sequence: global_sequence as u64,
        occurred_at,
        actor,
        event: StoredEvent(event),
        event_kind: event_kind_str,
        event_schema_version: event_schema_version as u16,
        trace_schema_version: trace_schema_version as u16,
        prev_hash: prev_hash_str.map(EntryHash),
        entry_hash: EntryHash(entry_hash_str),
    })
}

fn do_get(
    conn: &Connection,
    id: &TraceId,
) -> Result<Option<TraceEntry<StoredEvent>>, StoreError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, stream_scope, stream_id, stream_sequence, global_sequence,
                    occurred_at, actor_kind, actor_payload, event_kind, event_payload,
                    event_schema_version, trace_schema_version, prev_hash, entry_hash
             FROM trace_entry WHERE id = ?1",
        )
        .map_err(|e| StoreError::Database(format!("prepare get: {e}")))?;

    let result = stmt
        .query_row(rusqlite::params![id.0], |row| {
            row_to_entry(row).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })
        .ok();

    Ok(result)
}

fn do_get_with_relations(
    conn: &Connection,
    id: &TraceId,
) -> Result<Option<TraceEntryWithRelations<StoredEvent>>, StoreError> {
    let entry = do_get(conn, id)?;

    Ok(entry.map(|e| {
        let rels = load_relations_for(conn, &e.id);
        TraceEntryWithRelations { entry: e, relations: rels }
    }))
}

fn load_relations_for(conn: &Connection, id: &TraceId) -> Vec<TraceRelation> {
    let mut stmt = match conn.prepare(
        "SELECT from_trace_id, to_trace_id, kind, created_at
         FROM trace_relation WHERE from_trace_id = ?1 OR to_trace_id = ?1",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    let rows: Vec<_> = stmt
        .query_map(rusqlite::params![id.0], |row| {
            let from: String = row.get(0)?;
            let to: String = row.get(1)?;
            let kind_str: String = row.get(2)?;
            let created_at_ts: i64 = row.get(3)?;
            Ok((from, to, kind_str, created_at_ts))
        })
        .unwrap_or_else(|_| panic!("query_map failed"))
        .filter_map(|r| r.ok())
        .collect();

    rows.into_iter()
        .filter_map(|(from, to, kind_str, ts)| {
            let kind = serde_json::from_str(&kind_str).ok()?;
            let created_at = chrono::DateTime::from_timestamp(ts, 0).unwrap_or_default();
            Some(TraceRelation {
                from: TraceId(from),
                to: TraceId(to),
                kind,
                created_at,
            })
        })
        .collect()
}

fn do_scan(
    conn: &Connection,
    query: &TraceQuery,
) -> Result<TracePage<StoredEvent>, StoreError> {
    let mut sql = String::from(
        "SELECT id, stream_scope, stream_id, stream_sequence, global_sequence,
                occurred_at, actor_kind, actor_payload, event_kind, event_payload,
                event_schema_version, trace_schema_version, prev_hash, entry_hash
         FROM trace_entry WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref stream_id) = query.stream_id {
        sql.push_str(" AND stream_scope = ?");
        let scope = serde_json::to_string(&stream_id.scope)
            .map_err(|e| StoreError::Serialization(format!("scope: {e}")))?;
        params.push(Box::new(scope));
        sql.push_str(" AND stream_id = ?");
        params.push(Box::new(stream_id.id.clone()));
    }

    if let Some(ref kind) = query.event_kind {
        sql.push_str(" AND event_kind = ?");
        params.push(Box::new(kind.clone()));
    }

    if let Some(from) = query.from_sequence {
        sql.push_str(" AND global_sequence >= ?");
        params.push(Box::new(from as i64));
    }

    if let Some(to) = query.to_sequence {
        sql.push_str(" AND global_sequence <= ?");
        params.push(Box::new(to as i64));
    }

    // Cursor-based pagination: if a cursor TraceId is provided,
    // look up its global_sequence and filter entries after it.
    if let Some(ref cursor) = query.cursor {
        let cursor_seq: Option<i64> = conn
            .query_row(
                "SELECT global_sequence FROM trace_entry WHERE id = ?1",
                rusqlite::params![cursor.0],
                |row| row.get::<_, i64>(0),
            )
            .ok();
        if let Some(seq) = cursor_seq {
            sql.push_str(" AND global_sequence > ?");
            params.push(Box::new(seq));
        }
    }

    // Order by global sequence
    sql.push_str(" ORDER BY global_sequence ASC");

    if let Some(limit) = query.limit {
        sql.push_str(&format!(" LIMIT {limit}"));
    }

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| StoreError::Database(format!("prepare scan: {e}")))?;

    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        params.iter().map(|p| p.as_ref()).collect();

    let entries: Vec<TraceEntry<StoredEvent>> = stmt
        .query_map(param_refs.as_slice(), |row| {
            row_to_entry(row).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })
        .map_err(|e| StoreError::Database(format!("scan: {e}")))?
        .filter_map(|r| r.ok())
        .collect();

    let total = entries.len();
    let next_cursor = entries.last().map(|e| e.id.clone());

    Ok(TracePage {
        entries,
        next_cursor,
        total,
    })
}

fn do_scan_relations(
    conn: &Connection,
    from: &Option<TraceId>,
    to: &Option<TraceId>,
    kind: &Option<String>,
) -> Result<Vec<TraceRelation>, StoreError> {
    let mut sql = String::from(
        "SELECT from_trace_id, to_trace_id, kind, created_at FROM trace_relation WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(from_id) = from {
        sql.push_str(" AND from_trace_id = ?");
        params.push(Box::new(from_id.0.clone()));
    }

    if let Some(to_id) = to {
        sql.push_str(" AND to_trace_id = ?");
        params.push(Box::new(to_id.0.clone()));
    }

    if let Some(kind_str) = kind {
        let kind_json = serde_json::to_string(&kind_str)
            .unwrap_or_else(|_| kind_str.clone());
        sql.push_str(" AND kind = ?");
        params.push(Box::new(kind_json));
    }

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| StoreError::Database(format!("prepare scan_relations: {e}")))?;

    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        params.iter().map(|p| p.as_ref()).collect();

    let rows: Vec<_> = stmt
        .query_map(param_refs.as_slice(), |row| {
            let from: String = row.get(0)?;
            let to: String = row.get(1)?;
            let kind_str: String = row.get(2)?;
            let created_at_ts: i64 = row.get(3)?;
            Ok((from, to, kind_str, created_at_ts))
        })
        .map_err(|e| StoreError::Database(format!("scan_relations: {e}")))?
        .filter_map(|r| r.ok())
        .collect();

    let relations = rows
        .into_iter()
        .filter_map(|(from, to, kind_str, ts)| {
            let kind = serde_json::from_str(&kind_str).ok()?;
            let created_at = chrono::DateTime::from_timestamp(ts, 0).unwrap_or_default();
            Some(TraceRelation {
                from: TraceId(from),
                to: TraceId(to),
                kind,
                created_at,
            })
        })
        .collect();

    Ok(relations)
}

fn do_current_global_sequence(conn: &Connection) -> Result<u64, StoreError> {
    let seq: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(global_sequence), 0) FROM trace_entry",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|e| StoreError::Database(format!("global sequence: {e}")))?;
    Ok(seq as u64)
}

fn do_current_stream_sequence(
    conn: &Connection,
    stream_id: &TraceStreamId,
) -> Result<u64, StoreError> {
    let scope = serde_json::to_string(&stream_id.scope)
        .map_err(|e| StoreError::Serialization(format!("scope: {e}")))?;
    let seq: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(stream_sequence), 0) FROM trace_entry WHERE stream_scope = ?1 AND stream_id = ?2",
            rusqlite::params![scope, stream_id.id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|e| StoreError::Database(format!("stream sequence: {e}")))?;
    Ok(seq as u64)
}
