//! Trace store trait — the core storage interface.

use async_trait::async_trait;

use crate::append::AppendTraceEntry;
use crate::entry::{TraceEntry, TraceEntryWithRelations};
use crate::error::TraceError;
use crate::ids::TraceId;
use crate::query::{RelationQuery, TracePage, TraceQuery};
use crate::relation::TraceRelation;
use crate::stream::TraceStreamId;

/// Generic append-only trace store.
/// Implemented by `openwand-store` against chosen backend.
/// E = concrete event type (e.g., OpenWandTraceEvent).
#[async_trait]
pub trait TraceStore<E>: Send + Sync {
    /// Append a new entry. Store assigns id, timestamps, sequences, hashes.
    async fn append(&self, command: AppendTraceEntry<E>) -> Result<TraceId, TraceError>;

    /// Append + synchronously update named projections.
    async fn append_and_project(
        &self,
        command: AppendTraceEntry<E>,
        projectors: &[&str],
    ) -> Result<TraceId, TraceError>;

    /// Get a single entry by ID.
    async fn get(&self, id: TraceId) -> Result<Option<TraceEntry<E>>, TraceError>;

    /// Get an entry with its relations.
    async fn get_with_relations(
        &self,
        id: TraceId,
    ) -> Result<Option<TraceEntryWithRelations<E>>, TraceError>;

    /// Scan entries matching a query.
    async fn scan(&self, query: TraceQuery) -> Result<TracePage<E>, TraceError>;

    /// Scan relations matching a query.
    async fn scan_relations(
        &self,
        query: RelationQuery,
    ) -> Result<Vec<TraceRelation>, TraceError>;

    /// Get current global sequence number.
    async fn current_global_sequence(&self) -> Result<u64, TraceError>;

    /// Get current stream sequence for a given stream.
    async fn current_stream_sequence(
        &self,
        stream_id: &TraceStreamId,
    ) -> Result<u64, TraceError>;

    /// Initialize storage (create tables, indexes).
    async fn initialize(&self) -> Result<(), TraceError>;

    /// Run a projection rebuild from a checkpoint.
    async fn rebuild_projection(
        &self,
        projector_name: &str,
        from: Option<TraceId>,
    ) -> Result<(), TraceError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prove TraceStore<dyn> compiles behind Arc<dyn TraceStore<E>>.
    #[allow(clippy::extra_unused_lifetimes, dead_code)]
    fn _assert_trace_store_arc_send_sync<E: Send + Sync + 'static>() {
        fn _uses_arc_dyn<E: Send + Sync + 'static>(_store: std::sync::Arc<dyn TraceStore<E>>) {
        }
    }

    #[test]
    fn trace_store_trait_object_compiles() {
        // If this compiles, the trait is object-safe.
        fn _check<E: Send + Sync + 'static>() {
            let _: fn() = _assert_trace_store_arc_send_sync::<E>;
        }
    }
}
