//! Trace ID types.

use serde::{Deserialize, Serialize};

/// Unique identifier for a trace entry.
/// Assigned by the trace store on append — never by callers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TraceId(pub String);

impl TraceId {
    pub fn new() -> Self {
        Self(ulid::Ulid::new().to_string())
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TraceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_id_roundtrip() {
        let id = TraceId::new();
        let json = serde_json::to_string(&id).unwrap();
        let restored: TraceId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, restored);
    }

    #[test]
    fn trace_id_is_26_char_ulid() {
        let id = TraceId::new();
        assert_eq!(26, id.0.len());
    }
}
