use crate::{MemoryError, MemoryQuery, RetrievalContext};
use async_trait::async_trait;

/// Public read-only interface to memory.
/// Session uses only this. Never `MemoryProjectionStore`.
#[async_trait]
pub trait MemoryReadStore: Send + Sync {
    async fn search(&self, query: MemoryQuery) -> Result<RetrievalContext, MemoryError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prove MemoryReadStore is object-safe.
    #[test]
    fn memory_read_store_trait_object_compiles() {
        fn _uses_arc_dyn(_store: std::sync::Arc<dyn MemoryReadStore>) {}
    }
}
