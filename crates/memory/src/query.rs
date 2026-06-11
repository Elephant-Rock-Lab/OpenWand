use serde::{Deserialize, Serialize};

/// Tokenize text for search matching.
/// Lowercases, strips punctuation, removes stopwords, normalizes plurals.
pub fn tokenize(text: &str) -> Vec<String> {
    let stop_words: &[&str] = &[
        "a", "an", "the", "is", "are", "was", "were", "be", "been", "being",
        "have", "has", "had", "do", "does", "did", "will", "would", "could",
        "should", "may", "might", "can", "shall",
        "i", "me", "my", "myself", "we", "our", "ours",
        "you", "your", "yours", "it", "its", "they", "them", "their",
        "he", "him", "his", "she", "her", "hers",
        "what", "which", "who", "whom", "that", "this", "these", "those",
        "and", "or", "but", "not", "no", "nor",
        "if", "then", "so", "than", "too", "very",
        "of", "in", "on", "at", "to", "for", "with", "from",
        "by", "about", "as", "into", "through",
        "how", "when", "where", "why", "up", "out", "just",
    ];

    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .filter(|s| s.len() > 1)
        .filter(|s| !stop_words.contains(s))
        .map(|s| {
            #[allow(clippy::manual_strip)]
            if s.ends_with("ies") {
                format!("{}y", &s[..s.len() - 3])
            } else if s.ends_with('s') && !s.ends_with("ss") {
                s[..s.len() - 1].to_string()
            } else {
                s.to_string()
            }
        })
        .collect()
}

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
