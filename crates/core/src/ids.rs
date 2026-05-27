//! Domain IDs for OpenWand.
//!
//! All IDs are ULID-backed strings. They serialize as plain strings
//! and are ordered by creation time.

use serde::{Deserialize, Serialize};

/// Macro for generating typed domain IDs.
/// All IDs are ULID-backed strings with serde support.
macro_rules! domain_id {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash,
        )]
        pub struct $name(pub String);

        impl $name {
            pub fn new() -> Self {
                Self(ulid::Ulid::new().to_string())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

domain_id!(SessionId);
domain_id!(EpisodeId);
domain_id!(EntityId);
domain_id!(ClaimId); // Unified ID for facts, decisions, preferences
domain_id!(ArtifactId);
domain_id!(ToolCallId);
domain_id!(MessageId);
domain_id!(ApprovalRequestId);
domain_id!(ChunkId);
domain_id!(RunId);
domain_id!(GateId);
domain_id!(WorkflowId);
domain_id!(ModId);

// Note: DecisionId does NOT exist in core. Use ClaimId everywhere.
// Note: FactId does NOT exist in core. Memory may define a local alias later.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_id_generates_unique() {
        let a = SessionId::new();
        let b = SessionId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn session_id_default_is_new() {
        let a = SessionId::default();
        let b = SessionId::new();
        assert_ne!(a.0, b.0);
        assert!(!a.0.is_empty());
    }

    #[test]
    fn all_ids_serialize_as_strings() {
        let ids: Vec<(String, &str)> = vec![
            (serde_json::to_string(&SessionId::new()).unwrap(), "SessionId"),
            (serde_json::to_string(&EpisodeId::new()).unwrap(), "EpisodeId"),
            (serde_json::to_string(&EntityId::new()).unwrap(), "EntityId"),
            (serde_json::to_string(&ClaimId::new()).unwrap(), "ClaimId"),
            (serde_json::to_string(&ArtifactId::new()).unwrap(), "ArtifactId"),
            (serde_json::to_string(&ToolCallId::new()).unwrap(), "ToolCallId"),
            (serde_json::to_string(&MessageId::new()).unwrap(), "MessageId"),
            (serde_json::to_string(&ApprovalRequestId::new()).unwrap(), "ApprovalRequestId"),
            (serde_json::to_string(&ChunkId::new()).unwrap(), "ChunkId"),
            (serde_json::to_string(&RunId::new()).unwrap(), "RunId"),
            (serde_json::to_string(&GateId::new()).unwrap(), "GateId"),
            (serde_json::to_string(&WorkflowId::new()).unwrap(), "WorkflowId"),
            (serde_json::to_string(&ModId::new()).unwrap(), "ModId"),
        ];

        for (serialized, type_name) in &ids {
            // Must be a JSON string (wrapped in quotes)
            assert!(
                serialized.starts_with('"') && serialized.ends_with('"'),
                "{type_name} did not serialize as a JSON string: {serialized}"
            );
            // Must be 26 chars (ULID length) inside quotes
            let inner = &serialized[1..serialized.len() - 1];
            assert_eq!(inner.len(), 26, "{type_name} inner is not 26 chars: {inner}");
        }

        // Round-trip test
        let original = ClaimId::new();
        let json = serde_json::to_string(&original).unwrap();
        let restored: ClaimId = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }
}
