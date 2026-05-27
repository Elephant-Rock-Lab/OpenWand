use serde::{Deserialize, Serialize};

/// Query for hybrid (semantic + keyword) memory search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    pub text: String,
    pub max_results: Option<usize>,
}

impl MemoryQuery {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            max_results: None,
        }
    }
}
