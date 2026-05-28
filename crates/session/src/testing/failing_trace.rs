//! A trace store wrapper that fails on a specific event kind.
//!
//! Used for proving that trace append failures in the tool lifecycle
//! correctly block execution or surface as errors.

use openwand_store::StoredEvent;
use openwand_trace::entry::{TraceEntry, TraceEntryWithRelations};
use openwand_trace::ids::TraceId;
use openwand_trace::relation::{TraceRelation, TraceRelationDraft};
use openwand_trace::stream::TraceStreamId;
use openwand_trace::{
    AppendTraceEntry, IdempotencyKey, RelationQuery, TraceError, TracePage, TraceQuery, TraceStore,
};
use std::sync::Arc;
use async_trait::async_trait;

/// A trace store that fails `append` when the event kind matches.
pub struct FailOnEventKind {
    inner: Arc<dyn TraceStore<StoredEvent>>,
    fail_on: String,
}

impl FailOnEventKind {
    pub fn new(inner: Arc<dyn TraceStore<StoredEvent>>, fail_on: impl Into<String>) -> Self {
        Self {
            inner,
            fail_on: fail_on.into(),
        }
    }
}

#[async_trait]
impl TraceStore<StoredEvent> for FailOnEventKind {
    async fn append(&self, command: AppendTraceEntry<StoredEvent>) -> Result<TraceId, TraceError> {
        if command.event.event_kind() == self.fail_on {
            return Err(TraceError::Storage(format!(
                "injected failure on event kind '{}'",
                self.fail_on
            )));
        }
        self.inner.append(command).await
    }

    async fn append_and_project(
        &self,
        command: AppendTraceEntry<StoredEvent>,
        projectors: &[&str],
    ) -> Result<TraceId, TraceError> {
        if command.event.event_kind() == self.fail_on {
            return Err(TraceError::Storage(format!(
                "injected failure on event kind '{}'",
                self.fail_on
            )));
        }
        self.inner.append_and_project(command, projectors).await
    }

    async fn get(&self, id: TraceId) -> Result<Option<TraceEntry<StoredEvent>>, TraceError> {
        self.inner.get(id).await
    }

    async fn get_with_relations(
        &self,
        id: TraceId,
    ) -> Result<Option<TraceEntryWithRelations<StoredEvent>>, TraceError> {
        self.inner.get_with_relations(id).await
    }

    async fn scan(&self, query: TraceQuery) -> Result<TracePage<StoredEvent>, TraceError> {
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
        from_checkpoint: Option<TraceId>,
    ) -> Result<(), TraceError> {
        self.inner.rebuild_projection(projector_name, from_checkpoint).await
    }
}
