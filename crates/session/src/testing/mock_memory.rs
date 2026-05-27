use async_trait::async_trait;
use openwand_memory::{MemoryError, MemoryQuery, MemoryReadStore, RetrievalContext};
use tokio::sync::Mutex;

/// Mock memory read store.
pub struct MockMemoryReadStore {
    calls: Mutex<Vec<String>>,
    result: RetrievalContext,
}

impl MockMemoryReadStore {
    pub fn new() -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            result: RetrievalContext::empty(),
        }
    }

    pub fn with_result(result: RetrievalContext) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            result,
        }
    }

    pub async fn calls(&self) -> Vec<String> {
        self.calls.lock().await.clone()
    }
}

#[async_trait]
impl MemoryReadStore for MockMemoryReadStore {
    async fn search(&self, query: MemoryQuery) -> Result<RetrievalContext, MemoryError> {
        self.calls.lock().await.push(query.text.clone());
        Ok(self.result.clone())
    }
}
