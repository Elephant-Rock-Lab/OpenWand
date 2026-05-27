use serde::{Deserialize, Serialize};

/// Context retrieved from memory for a given query.
/// Layered by utility — most useful first.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalContext {
    /// Factual statements relevant to the query.
    pub facts: Vec<String>,
    /// Relevant past decisions.
    pub decisions: Vec<String>,
    /// Source episode summaries.
    pub episodes: Vec<String>,
    /// Query metadata.
    pub query_text: String,
    pub total_hits: usize,
}

impl RetrievalContext {
    pub fn empty() -> Self {
        Self {
            facts: vec![],
            decisions: vec![],
            episodes: vec![],
            query_text: String::new(),
            total_hits: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.facts.is_empty() && self.decisions.is_empty() && self.episodes.is_empty()
    }

    /// Format as a context block for LLM injection.
    pub fn to_context_block(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }

        let mut parts = Vec::new();
        if !self.facts.is_empty() {
            parts.push(format!("## Facts\n{}", self.facts.join("\n")));
        }
        if !self.decisions.is_empty() {
            parts.push(format!("## Past Decisions\n{}", self.decisions.join("\n")));
        }
        if !self.episodes.is_empty() {
            parts.push(format!("## Context\n{}", self.episodes.join("\n")));
        }

        Some(parts.join("\n\n"))
    }
}
