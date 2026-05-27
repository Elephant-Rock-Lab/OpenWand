//! In-memory trace store for testing.
//!
//! Only compiled with `#[cfg(feature = "testing")]`.
//! Not for production use — no persistence, no crash recovery.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;

use crate::append::AppendTraceEntry;
use crate::entry::{TraceEntry, TraceEntryWithRelations};
use crate::envelope::TraceEventEnvelope;
use crate::error::TraceError;
use crate::ids::TraceId;
use crate::query::{ActorFilter, RelationQuery, TracePage, TraceQuery};
use crate::relation::TraceRelation;
use crate::stream::{EntryHash, IdempotencyKey, TraceStreamId};
use crate::store::TraceStore;

pub struct InMemoryTraceStore<E> {
    inner: Arc<RwLock<InMemoryTraceState<E>>>,
}

struct InMemoryTraceState<E> {
    entries: Vec<TraceEntry<E>>,
    by_id: HashMap<TraceId, usize>,
    relations: Vec<TraceRelation>,
    idempotency: HashMap<IdempotencyKey, TraceId>,
    stream_sequences: HashMap<String, u64>,
    global_sequence: u64,
    initialized: bool,
}

impl<E> Default for InMemoryTraceState<E> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            by_id: HashMap::new(),
            relations: Vec::new(),
            idempotency: HashMap::new(),
            stream_sequences: HashMap::new(),
            global_sequence: 0,
            initialized: false,
        }
    }
}

impl<E> InMemoryTraceStore<E> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(InMemoryTraceState::default())),
        }
    }

    /// Deterministic entry hash for testing.
    /// Uses a simple format: `{global_seq}:{event_kind}:{stream_id}`.
    /// Not cryptographically secure — that's the SQLite store's job.
    fn compute_entry_hash(global_seq: u64, event_kind: &str, stream_key: &str) -> EntryHash {
        EntryHash(format!("mem:{global_seq}:{event_kind}:{stream_key}"))
    }

    fn stream_key(stream_id: &TraceStreamId) -> String {
        format!("{:?}:{}", stream_id.scope, stream_id.id)
    }

    /// Return event_kind strings for all stored entries.
    /// Lightweight inspection for test assertions.
    pub async fn event_kinds(&self) -> Vec<String> {
        let state = self.inner.read().await;
        state.entries.iter().map(|e| e.event_kind.clone()).collect()
    }
}

impl<E> Default for InMemoryTraceStore<E> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<E> TraceStore<E> for InMemoryTraceStore<E>
where
    E: TraceEventEnvelope + Clone + Send + Sync + 'static,
{
    async fn append(&self, command: AppendTraceEntry<E>) -> Result<TraceId, TraceError> {
        let mut state = self.inner.write().await;

        // 1. Idempotency check
        if let Some(ref key) = command.idempotency_key {
            if let Some(existing_id) = state.idempotency.get(key) {
                return Ok(existing_id.clone());
            }
        }

        // 2. Assign new TraceId
        let id = TraceId::new();

        // 3. Increment global sequence
        state.global_sequence += 1;
        let global_seq = state.global_sequence;

        // 4. Increment stream sequence
        let skey = Self::stream_key(&command.stream_id);
        let stream_seq = state.stream_sequences.entry(skey.clone()).or_insert(0);
        *stream_seq += 1;
        let stream_seq_val = *stream_seq;

        // 5. Find previous entry hash for this stream
        let prev_hash = state
            .entries
            .iter()
            .rev()
            .find(|e| Self::stream_key(&e.stream_id) == skey)
            .map(|e| e.entry_hash.clone());

        // 6-8. Build TraceEntry
        let event_kind = command.event.event_kind().to_owned();
        let schema_version = command.event.schema_version();
        let entry_hash = Self::compute_entry_hash(global_seq, &event_kind, &skey);

        let entry = TraceEntry {
            id: id.clone(),
            stream_id: command.stream_id,
            stream_sequence: stream_seq_val,
            global_sequence: global_seq,
            occurred_at: Utc::now(),
            actor: command.actor,
            event: command.event,
            event_kind,
            event_schema_version: schema_version,
            trace_schema_version: 1,
            prev_hash,
            entry_hash,
        };

        // 9. Store entry
        let idx = state.entries.len();
        state.by_id.insert(id.clone(), idx);
        state.entries.push(entry);

        // 10. Store relations
        let now = Utc::now();
        for draft in command.relations {
            state.relations.push(TraceRelation {
                from: id.clone(),
                to: draft.to,
                kind: draft.kind,
                created_at: now,
            });
        }

        // 11. Register idempotency key
        if let Some(key) = command.idempotency_key {
            state.idempotency.insert(key, id.clone());
        }

        Ok(id)
    }

    async fn append_and_project(
        &self,
        command: AppendTraceEntry<E>,
        _projectors: &[&str],
    ) -> Result<TraceId, TraceError> {
        // In-memory store doesn't support named projections — just append.
        self.append(command).await
    }

    async fn get(&self, id: TraceId) -> Result<Option<TraceEntry<E>>, TraceError> {
        let state = self.inner.read().await;
        Ok(state
            .by_id
            .get(&id)
            .map(|&idx| state.entries[idx].clone()))
    }

    async fn get_with_relations(
        &self,
        id: TraceId,
    ) -> Result<Option<TraceEntryWithRelations<E>>, TraceError> {
        let state = self.inner.read().await;
        let entry = state.by_id.get(&id).map(|&idx| state.entries[idx].clone());
        Ok(entry.map(|e| {
            let rels: Vec<TraceRelation> = state
                .relations
                .iter()
                .filter(|r| r.from == id || r.to == id)
                .cloned()
                .collect();
            TraceEntryWithRelations { entry: e, relations: rels }
        }))
    }

    async fn scan(&self, query: TraceQuery) -> Result<TracePage<E>, TraceError> {
        let state = self.inner.read().await;

        let mut results: Vec<&TraceEntry<E>> = Vec::new();

        // Start from cursor if provided
        let start_seq = if let Some(ref cursor) = query.cursor {
            state
                .by_id
                .get(cursor)
                .map(|&idx| state.entries[idx].global_sequence)
                .unwrap_or(0)
        } else {
            0
        };

        for entry in &state.entries {
            // Skip entries at or before cursor
            if entry.global_sequence <= start_seq {
                continue;
            }

            // Apply filters
            if let Some(ref stream_id) = query.stream_id {
                if Self::stream_key(&entry.stream_id) != Self::stream_key(stream_id) {
                    continue;
                }
            }
            if let Some(ref kind) = query.event_kind {
                if &entry.event_kind != kind {
                    continue;
                }
            }
            if let Some(ref actor_filter) = query.actor {
                let matches = match actor_filter {
                    ActorFilter::UserOnly => matches!(entry.actor, crate::actor::Actor::User),
                    ActorFilter::LlmOnly => {
                        matches!(entry.actor, crate::actor::Actor::Llm { .. })
                    }
                    ActorFilter::SystemOnly => {
                        matches!(entry.actor, crate::actor::Actor::System { .. })
                    }
                    ActorFilter::Component(name) => {
                        matches!(&entry.actor, crate::actor::Actor::System { component } if component == name)
                    }
                };
                if !matches {
                    continue;
                }
            }
            if let Some(from) = query.from_sequence {
                if entry.global_sequence < from {
                    continue;
                }
            }
            if let Some(to) = query.to_sequence {
                if entry.global_sequence > to {
                    continue;
                }
            }
            if let Some(from) = query.from_timestamp {
                if entry.occurred_at < from {
                    continue;
                }
            }
            if let Some(to) = query.to_timestamp {
                if entry.occurred_at > to {
                    continue;
                }
            }

            results.push(entry);
        }

        // Results are already in ascending global_sequence order (storage order)
        let limit = query.limit.unwrap_or(results.len());
        let total = results.len();
        let limited: Vec<TraceEntry<E>> = results.into_iter().take(limit).cloned().collect();

        let next_cursor = if limited.len() < total {
            limited.last().map(|e| e.id.clone())
        } else {
            None
        };

        Ok(TracePage {
            entries: limited,
            next_cursor,
            total,
        })
    }

    async fn scan_relations(
        &self,
        query: RelationQuery,
    ) -> Result<Vec<TraceRelation>, TraceError> {
        let state = self.inner.read().await;

        let results: Vec<TraceRelation> = state
            .relations
            .iter()
            .filter(|r| {
                if let Some(ref from) = query.from {
                    if &r.from != from {
                        return false;
                    }
                }
                if let Some(ref to) = query.to {
                    if &r.to != to {
                        return false;
                    }
                }
                if let Some(ref kind) = query.kind {
                    if &r.kind != kind {
                        return false;
                    }
                }
                true
            })
            .take(query.limit.unwrap_or(usize::MAX))
            .cloned()
            .collect();

        Ok(results)
    }

    async fn current_global_sequence(&self) -> Result<u64, TraceError> {
        let state = self.inner.read().await;
        Ok(state.global_sequence)
    }

    async fn current_stream_sequence(
        &self,
        stream_id: &TraceStreamId,
    ) -> Result<u64, TraceError> {
        let state = self.inner.read().await;
        let skey = Self::stream_key(stream_id);
        Ok(*state.stream_sequences.get(&skey).unwrap_or(&0))
    }

    async fn initialize(&self) -> Result<(), TraceError> {
        let mut state = self.inner.write().await;
        if state.initialized {
            return Err(TraceError::AlreadyInitialized);
        }
        state.initialized = true;
        Ok(())
    }

    async fn rebuild_projection(
        &self,
        _projector_name: &str,
        _from: Option<TraceId>,
    ) -> Result<(), TraceError> {
        // In-memory store has no projections to rebuild.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor::Actor;
    use crate::relation::TraceRelationKind;
    use crate::TraceRelationDraft;
    use crate::stream::{TraceStreamId, TraceStreamScope};

    /// Minimal test event that implements TraceEventEnvelope.
    #[derive(Debug, Clone, PartialEq)]
    enum TestEvent {
        Alpha { label: String },
        Beta { value: i32 },
    }

    impl TraceEventEnvelope for TestEvent {
        fn event_kind(&self) -> &'static str {
            match self {
                Self::Alpha { .. } => "test.alpha",
                Self::Beta { .. } => "test.beta",
            }
        }
        fn schema_version(&self) -> u16 {
            1
        }
    }

    fn test_stream(name: &str) -> TraceStreamId {
        TraceStreamId {
            scope: TraceStreamScope::Session,
            id: name.into(),
        }
    }

    #[tokio::test]
    async fn in_memory_append_assigns_ids() {
        let store = InMemoryTraceStore::<TestEvent>::new();
        let id = store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Alpha { label: "first".into() },
                relations: vec![],
                stream_id: test_stream("s1"),
                idempotency_key: None,
            })
            .await
            .unwrap();

        // ID should be a valid ULID
        assert_eq!(26, id.0.len());
    }

    #[tokio::test]
    async fn in_memory_append_assigns_global_sequence() {
        let store = InMemoryTraceStore::<TestEvent>::new();

        store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Alpha { label: "a".into() },
                relations: vec![],
                stream_id: test_stream("s1"),
                idempotency_key: None,
            })
            .await
            .unwrap();
        store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Alpha { label: "b".into() },
                relations: vec![],
                stream_id: test_stream("s1"),
                idempotency_key: None,
            })
            .await
            .unwrap();

        assert_eq!(2, store.current_global_sequence().await.unwrap());
    }

    #[tokio::test]
    async fn in_memory_append_assigns_stream_sequence() {
        let store = InMemoryTraceStore::<TestEvent>::new();
        let s1 = test_stream("s1");
        let s2 = test_stream("s2");

        store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Alpha { label: "a".into() },
                relations: vec![],
                stream_id: s1.clone(),
                idempotency_key: None,
            })
            .await
            .unwrap();
        store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Beta { value: 1 },
                relations: vec![],
                stream_id: s2.clone(),
                idempotency_key: None,
            })
            .await
            .unwrap();
        store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Alpha { label: "c".into() },
                relations: vec![],
                stream_id: s1.clone(),
                idempotency_key: None,
            })
            .await
            .unwrap();

        assert_eq!(2, store.current_stream_sequence(&s1).await.unwrap());
        assert_eq!(1, store.current_stream_sequence(&s2).await.unwrap());
    }

    #[tokio::test]
    async fn in_memory_get_returns_entry() {
        let store = InMemoryTraceStore::<TestEvent>::new();
        let id = store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Alpha { label: "hello".into() },
                relations: vec![],
                stream_id: test_stream("s1"),
                idempotency_key: None,
            })
            .await
            .unwrap();

        let entry = store.get(id).await.unwrap().unwrap();
        assert_eq!("test.alpha", entry.event_kind);
        assert_eq!(1, entry.global_sequence);
        assert_eq!(1, entry.stream_sequence);
        assert!(entry.prev_hash.is_none());
    }

    #[tokio::test]
    async fn in_memory_scan_by_stream() {
        let store = InMemoryTraceStore::<TestEvent>::new();
        let s1 = test_stream("s1");
        let s2 = test_stream("s2");

        store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Alpha { label: "a".into() },
                relations: vec![],
                stream_id: s1.clone(),
                idempotency_key: None,
            })
            .await
            .unwrap();
        store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Beta { value: 1 },
                relations: vec![],
                stream_id: s2.clone(),
                idempotency_key: None,
            })
            .await
            .unwrap();
        store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Alpha { label: "c".into() },
                relations: vec![],
                stream_id: s1.clone(),
                idempotency_key: None,
            })
            .await
            .unwrap();

        let page = store
            .scan(TraceQuery {
                stream_id: Some(s1),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(2, page.entries.len());
        assert_eq!("test.alpha", page.entries[0].event_kind);
        assert_eq!("test.alpha", page.entries[1].event_kind);
    }

    #[tokio::test]
    async fn in_memory_scan_by_event_kind() {
        let store = InMemoryTraceStore::<TestEvent>::new();

        store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Alpha { label: "a".into() },
                relations: vec![],
                stream_id: test_stream("s1"),
                idempotency_key: None,
            })
            .await
            .unwrap();
        store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Beta { value: 1 },
                relations: vec![],
                stream_id: test_stream("s1"),
                idempotency_key: None,
            })
            .await
            .unwrap();

        let page = store
            .scan(TraceQuery {
                event_kind: Some("test.beta".into()),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(1, page.entries.len());
        assert_eq!(2, page.entries[0].global_sequence);
    }

    #[tokio::test]
    async fn in_memory_relations_roundtrip() {
        let store = InMemoryTraceStore::<TestEvent>::new();

        let first_id = store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Alpha { label: "first".into() },
                relations: vec![],
                stream_id: test_stream("s1"),
                idempotency_key: None,
            })
            .await
            .unwrap();

        let second_id = store
            .append(AppendTraceEntry {
                actor: Actor::Llm {
                    model: "gpt-4o".into(),
                    provider: "openai".into(),
                },
                event: TestEvent::Beta { value: 42 },
                relations: vec![TraceRelationDraft {
                    to: first_id.clone(),
                    kind: TraceRelationKind::CausedBy,
                }],
                stream_id: test_stream("s1"),
                idempotency_key: None,
            })
            .await
            .unwrap();

        // Query by from
        let rels = store
            .scan_relations(RelationQuery {
                from: Some(second_id.clone()),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(1, rels.len());
        assert_eq!(second_id, rels[0].from);
        assert_eq!(first_id, rels[0].to);
        assert_eq!(TraceRelationKind::CausedBy, rels[0].kind);

        // get_with_relations
        let with_rels = store
            .get_with_relations(second_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(1, with_rels.relations.len());
    }

    #[tokio::test]
    async fn in_memory_idempotency_key_deduplicates() {
        let store = InMemoryTraceStore::<TestEvent>::new();
        let key = IdempotencyKey("unique-op-1".into());

        let id1 = store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Alpha { label: "first".into() },
                relations: vec![],
                stream_id: test_stream("s1"),
                idempotency_key: Some(key.clone()),
            })
            .await
            .unwrap();

        let id2 = store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Alpha { label: "duplicate".into() },
                relations: vec![],
                stream_id: test_stream("s1"),
                idempotency_key: Some(key.clone()),
            })
            .await
            .unwrap();

        // Same ID returned, no second entry
        assert_eq!(id1, id2);
        assert_eq!(1, store.current_global_sequence().await.unwrap());

        // Original entry preserved
        let entry = store.get(id1).await.unwrap().unwrap();
        match &entry.event {
            TestEvent::Alpha { label } => assert_eq!("first", label),
            _ => panic!("wrong event variant"),
        }
    }

    #[tokio::test]
    async fn in_memory_current_sequences() {
        let store = InMemoryTraceStore::<TestEvent>::new();
        let s1 = test_stream("s1");

        assert_eq!(0, store.current_global_sequence().await.unwrap());
        assert_eq!(
            0,
            store.current_stream_sequence(&s1).await.unwrap()
        );

        store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Alpha { label: "a".into() },
                relations: vec![],
                stream_id: s1.clone(),
                idempotency_key: None,
            })
            .await
            .unwrap();

        assert_eq!(1, store.current_global_sequence().await.unwrap());
        assert_eq!(1, store.current_stream_sequence(&s1).await.unwrap());
    }

    #[tokio::test]
    async fn in_memory_hash_chain_links_previous_entry() {
        let store = InMemoryTraceStore::<TestEvent>::new();
        let s1 = test_stream("s1");

        let id1 = store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Alpha { label: "first".into() },
                relations: vec![],
                stream_id: s1.clone(),
                idempotency_key: None,
            })
            .await
            .unwrap();

        let id2 = store
            .append(AppendTraceEntry {
                actor: Actor::User,
                event: TestEvent::Alpha { label: "second".into() },
                relations: vec![],
                stream_id: s1.clone(),
                idempotency_key: None,
            })
            .await
            .unwrap();

        let entry1 = store.get(id1).await.unwrap().unwrap();
        let entry2 = store.get(id2).await.unwrap().unwrap();

        // First entry has no previous hash
        assert!(entry1.prev_hash.is_none());

        // Second entry links to first's hash
        assert!(entry2.prev_hash.is_some());
        assert_eq!(entry1.entry_hash, entry2.prev_hash.unwrap());
    }
}

/// A trace store wrapper that fails on append when the event_kind matches
/// a configured predicate. Used for hostile ordering tests.
///
/// All other operations delegate to the inner store.
pub struct FailOnAppend<E> {
    inner: Arc<dyn TraceStore<E>>,
    fail_on: Arc<std::sync::Mutex<Option<Box<dyn Fn(&str) -> bool + Send + Sync>>>>,
}

impl<E: Send + Sync + 'static> FailOnAppend<E> {
    pub fn new(
        inner: Arc<dyn TraceStore<E>>,
        fail_on: Box<dyn Fn(&str) -> bool + Send + Sync>,
    ) -> Self {
        Self {
            inner,
            fail_on: Arc::new(std::sync::Mutex::new(Some(fail_on))),
        }
    }

    /// Create a FailOnAppend that fails when event_kind contains the given substring.
    pub fn fail_on_kind(inner: Arc<dyn TraceStore<E>>, kind_substring: &str) -> Self {
        let substr = kind_substring.to_string();
        Self::new(inner, Box::new(move |kind: &str| kind.contains(&substr)))
    }
}

#[async_trait]
impl<E: TraceEventEnvelope + Clone + Send + Sync + 'static> TraceStore<E> for FailOnAppend<E> {
    async fn append(&self, command: AppendTraceEntry<E>) -> Result<TraceId, TraceError> {
        let event_kind = command.event.event_kind();
        let should_fail = {
            let guard = self.fail_on.lock().unwrap();
            if let Some(ref pred) = *guard {
                pred(event_kind)
            } else {
                false
            }
        };

        if should_fail {
            return Err(TraceError::AppendFailed(format!(
                "Intentional failure for event_kind: {event_kind}"
            )));
        }

        self.inner.append(command).await
    }

    async fn append_and_project(
        &self,
        command: AppendTraceEntry<E>,
        projectors: &[&str],
    ) -> Result<TraceId, TraceError> {
        let event_kind = command.event.event_kind();
        let should_fail = {
            let guard = self.fail_on.lock().unwrap();
            if let Some(ref pred) = *guard {
                pred(event_kind)
            } else {
                false
            }
        };

        if should_fail {
            return Err(TraceError::AppendFailed(format!(
                "Intentional failure for event_kind: {event_kind}"
            )));
        }

        self.inner.append_and_project(command, projectors).await
    }

    async fn get(&self, id: TraceId) -> Result<Option<TraceEntry<E>>, TraceError> {
        self.inner.get(id).await
    }

    async fn get_with_relations(
        &self,
        id: TraceId,
    ) -> Result<Option<TraceEntryWithRelations<E>>, TraceError> {
        self.inner.get_with_relations(id).await
    }

    async fn scan(&self, query: TraceQuery) -> Result<TracePage<E>, TraceError> {
        self.inner.scan(query).await
    }

    async fn scan_relations(
        &self,
        query: RelationQuery,
    ) -> Result<Vec<TraceRelation>, TraceError> {
        self.inner.scan_relations(query).await
    }

    async fn current_global_sequence(&self) -> Result<u64, TraceError> {
        self.inner.current_global_sequence().await
    }

    async fn current_stream_sequence(
        &self,
        stream_id: &TraceStreamId,
    ) -> Result<u64, TraceError> {
        self.inner.current_stream_sequence(stream_id).await
    }

    async fn initialize(&self) -> Result<(), TraceError> {
        self.inner.initialize().await
    }

    async fn rebuild_projection(
        &self,
        projector_name: &str,
        from: Option<TraceId>,
    ) -> Result<(), TraceError> {
        self.inner.rebuild_projection(projector_name, from).await
    }
}
