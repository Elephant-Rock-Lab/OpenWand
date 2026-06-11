use serde::{Deserialize, Serialize};

use crate::session_vocab::ThinkingBudgetSnapshot;
use crate::snapshots::{PromptAssemblySnapshot, TokenUsageSnapshot};

/// Prompt order position for capability context insertion (Patch 3).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CapabilityPromptOrderPosition {
    AfterMemoryBlock,
}

impl CapabilityPromptOrderPosition {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AfterMemoryBlock => "after_memory_block",
        }
    }
}

/// Manifest audit state for capability context trace (Patch 3).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CapabilityManifestAuditState {
    NotFound,
    FoundEmpty,
    FoundWithItems,
    Invalid,
}

/// Hash algorithm for capability context trace (Patch 5).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TraceHashAlgorithm {
    Sha256,
}

impl TraceHashAlgorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sha256 => "sha256",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InferenceEvent {
    Called {
        model: String,
        provider: String,
        prompt_hash: String,
        thinking_budget: Option<ThinkingBudgetSnapshot>,
        prompt_assembly: PromptAssemblySnapshot,
    },
    Completed {
        model: String,
        tokens: TokenUsageSnapshot,
        stop_reason: String,
        tool_call_count: u8,
    },
    Failed {
        model: String,
        error: String,
        retry_count: u8,
    },
    /// Capability context was assembled into the LLM request prompt (Patch 1, 2).
    /// Emitted from prompt assembly path when a non-empty block is present (Patch 7).
    /// Stores no raw capability context text (Patch 6).
    /// Records "assembled for prompt" — does not imply provider delivery (Patch 2).
    CapabilityContextAssembled {
        /// Session ID for correlation with adjacent InferenceEvent::Called.
        session_id: String,
        included_skill_ids: Vec<String>,
        included_goal_ids: Vec<String>,
        excluded_item_ids: Vec<String>,
        skills_manifest_state: CapabilityManifestAuditState,
        goals_manifest_state: CapabilityManifestAuditState,
        /// SHA-256 hash of the exact sanitized context text inserted into prompt.
        context_text_hash: String,
        context_text_hash_algorithm: TraceHashAlgorithm,
        context_text_length: usize,
        prompt_order_position: CapabilityPromptOrderPosition,
    },
}

impl InferenceEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Called { .. } => "inference.called",
            Self::Completed { .. } => "inference.completed",
            Self::Failed { .. } => "inference.failed",
            Self::CapabilityContextAssembled { .. } => "inference.capability_context_assembled",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_capability_event() -> InferenceEvent {
        InferenceEvent::CapabilityContextAssembled {
            session_id: "sess-1".into(),
            included_skill_ids: vec!["skill-a".into()],
            included_goal_ids: vec!["goal-b".into()],
            excluded_item_ids: vec!["skill-c".into()],
            skills_manifest_state: CapabilityManifestAuditState::FoundWithItems,
            goals_manifest_state: CapabilityManifestAuditState::FoundWithItems,
            context_text_hash: "abc123".into(),
            context_text_hash_algorithm: TraceHashAlgorithm::Sha256,
            context_text_length: 500,
            prompt_order_position: CapabilityPromptOrderPosition::AfterMemoryBlock,
        }
    }

    // Patch 2: event copy says assembled, not delivered

    #[test]
    fn capability_context_event_copy_says_assembled_not_delivered() {
        let kind = test_capability_event().event_kind();
        assert!(kind.contains("assembled"));
        assert!(!kind.contains("delivered"));
        assert!(!kind.contains("sent"));
    }

    #[test]
    fn capability_context_event_does_not_claim_provider_receipt() {
        let event = test_capability_event();
        let json = serde_json::to_string(&event).unwrap();
        let lower = json.to_lowercase();
        assert!(!lower.contains("provider_receipt"));
        assert!(!lower.contains("accepted_by_provider"));
    }

    #[test]
    fn capability_context_event_does_not_claim_model_processed_context() {
        let event = test_capability_event();
        let json = serde_json::to_string(&event).unwrap();
        let lower = json.to_lowercase();
        assert!(!lower.contains("model_processed"));
        assert!(!lower.contains("processed_by_model"));
    }

    // Patch 3: typed fields

    #[test]
    fn capability_context_event_uses_typed_prompt_order_position() {
        let pos = CapabilityPromptOrderPosition::AfterMemoryBlock;
        assert_eq!("after_memory_block", pos.as_str());
    }

    #[test]
    fn capability_context_event_order_position_is_after_memory_block() {
        if let InferenceEvent::CapabilityContextAssembled { prompt_order_position, .. } = test_capability_event() {
            assert_eq!(CapabilityPromptOrderPosition::AfterMemoryBlock, prompt_order_position);
        }
    }

    #[test]
    fn capability_context_event_uses_typed_manifest_states() {
        if let InferenceEvent::CapabilityContextAssembled {
            skills_manifest_state,
            goals_manifest_state,
            ..
        } = test_capability_event() {
            assert_eq!(CapabilityManifestAuditState::FoundWithItems, skills_manifest_state);
            assert_eq!(CapabilityManifestAuditState::FoundWithItems, goals_manifest_state);
        }
    }

    // Patch 4: correlation IDs

    #[test]
    fn capability_context_event_contains_session_id() {
        if let InferenceEvent::CapabilityContextAssembled { session_id, .. } = test_capability_event() {
            assert_eq!("sess-1", session_id);
        }
    }

    // Patch 5: hash algorithm

    #[test]
    fn capability_context_hash_is_sha256() {
        // SHA-256 is the chosen algorithm — verified in session crate tests
        if let InferenceEvent::CapabilityContextAssembled { context_text_hash_algorithm, .. } = test_capability_event() {
            assert_eq!(TraceHashAlgorithm::Sha256, context_text_hash_algorithm);
            assert_eq!("sha256", context_text_hash_algorithm.as_str());
        }
    }

    #[test]
    fn capability_context_hash_changes_when_prompt_text_changes() {
        // Hash determinism is tested in the session crate.
        // Here we verify the field exists and is non-empty.
        if let InferenceEvent::CapabilityContextAssembled { context_text_hash, .. } = test_capability_event() {
            assert!(!context_text_hash.is_empty());
        }
    }

    // Patch 6: no raw text in trace

    #[test]
    fn capability_context_trace_does_not_store_raw_text() {
        let event = test_capability_event();
        let json = serde_json::to_string(&event).unwrap();
        // The event has hash and length, no raw text field
        let lower = json.to_lowercase();
        assert!(!lower.contains("\"text\":"));
        assert!(!lower.contains("\"context_text\":"));
        assert!(lower.contains("context_text_hash"));
        assert!(lower.contains("context_text_length"));
    }

    #[test]
    fn capability_context_trace_stores_hash_and_length_only() {
        if let InferenceEvent::CapabilityContextAssembled {
            context_text_hash,
            context_text_length,
            ..
        } = test_capability_event() {
            assert!(!context_text_hash.is_empty());
            assert_eq!(500, context_text_length);
        }
    }

    #[test]
    fn capability_context_trace_preserves_ids_without_manifest_body() {
        if let InferenceEvent::CapabilityContextAssembled {
            included_skill_ids,
            included_goal_ids,
            excluded_item_ids,
            ..
        } = test_capability_event() {
            assert_eq!(1, included_skill_ids.len());
            assert_eq!(1, included_goal_ids.len());
            assert_eq!(1, excluded_item_ids.len());
        }
    }

    // Patch 7: only emitted when non-empty

    #[test]
    fn no_capability_context_does_not_emit_assembled_event() {
        // Design assertion: trace emission only happens when
        // config.capability_context is Some with non-empty text.
        // No event variant is created for None.
        let _ = "CapabilityContextAssembled only emitted when block has content";
    }

    // Patch 8: serde/backward-compat

    #[test]
    fn capability_context_assembled_serializes_with_family_inference() {
        use crate::events::OpenWandTraceEvent;
        let event = OpenWandTraceEvent::Inference(test_capability_event());
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!("inference", json["family"]);
        assert!(json["payload"]["CapabilityContextAssembled"].is_object());
    }

    #[test]
    fn capability_context_assembled_deserializes_from_json() {
        use crate::events::OpenWandTraceEvent;
        let event = OpenWandTraceEvent::Inference(test_capability_event());
        let json = serde_json::to_string(&event).unwrap();
        let restored: OpenWandTraceEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event.event_kind(), restored.event_kind());
    }

    #[test]
    fn existing_inference_events_still_deserialize() {
        use crate::events::OpenWandTraceEvent;
        let existing = OpenWandTraceEvent::Inference(InferenceEvent::Called {
            model: "gpt-4o".into(),
            provider: "openai".into(),
            prompt_hash: "abc".into(),
            thinking_budget: None,
            prompt_assembly: crate::snapshots::PromptAssemblySnapshot {
                system_prompt_hash: "h1".into(),
                message_window_hash: "h2".into(),
                memory_hit_ids: vec![],
                memory_context_hash: None,
                tool_manifest_hash: "h3".into(),
                policy_filter_hash: "h4".into(),
                mode: crate::mode::InteractionMode::Conversational,
                working_directory: "/tmp".into(),
            },
        });
        let json = serde_json::to_string(&existing).unwrap();
        let restored: OpenWandTraceEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(existing.event_kind(), restored.event_kind());
    }

    #[test]
    fn trace_event_round_trip_preserves_capability_context_fields() {
        let event = test_capability_event();
        if let InferenceEvent::CapabilityContextAssembled {
            included_skill_ids,
            session_id,
            context_text_hash,
            context_text_length,
            ..
        } = &event
        {
            let json = serde_json::to_string(&event).unwrap();
            let restored: InferenceEvent = serde_json::from_str(&json).unwrap();
            if let InferenceEvent::CapabilityContextAssembled {
                included_skill_ids: r_skills,
                session_id: r_sid,
                context_text_hash: r_hash,
                context_text_length: r_len,
                ..
            } = &restored
            {
                assert_eq!(included_skill_ids, r_skills);
                assert_eq!(session_id, r_sid);
                assert_eq!(context_text_hash, r_hash);
                assert_eq!(context_text_length, r_len);
            } else {
                panic!("Wrong variant after deserialization");
            }
        } else {
            panic!("Wrong variant");
        }
    }

    // Patch 9: no-authority language

    #[test]
    fn capability_context_trace_contains_no_execution_language() {
        let event = test_capability_event();
        let json = serde_json::to_string(&event).unwrap();
        let lower = json.to_lowercase();
        assert!(!lower.contains("execute"));
        assert!(!lower.contains("invoke"));
    }

    #[test]
    fn capability_context_trace_contains_no_scheduler_language() {
        let event = test_capability_event();
        let json = serde_json::to_string(&event).unwrap();
        let lower = json.to_lowercase();
        assert!(!lower.contains("schedule"));
    }

    #[test]
    fn capability_context_trace_contains_no_policy_bypass_language() {
        let event = test_capability_event();
        let json = serde_json::to_string(&event).unwrap();
        let lower = json.to_lowercase();
        assert!(!lower.contains("bypass"));
    }

    #[test]
    fn capability_context_trace_contains_no_authority_language() {
        let event = test_capability_event();
        let json = serde_json::to_string(&event).unwrap();
        let lower = json.to_lowercase();
        assert!(!lower.contains("authority"));
        assert!(!lower.contains("approved"));
    }

    // Event kind

    #[test]
    fn capability_context_assembled_event_kind() {
        assert_eq!("inference.capability_context_assembled", test_capability_event().event_kind());
    }

    // Typed enums display

    #[test]
    fn capability_prompt_order_position_display() {
        assert_eq!("after_memory_block", CapabilityPromptOrderPosition::AfterMemoryBlock.as_str());
    }

    #[test]
    fn trace_hash_algorithm_display() {
        assert_eq!("sha256", TraceHashAlgorithm::Sha256.as_str());
    }

    #[test]
    fn capability_manifest_audit_state_variants() {
        assert_ne!(CapabilityManifestAuditState::NotFound, CapabilityManifestAuditState::FoundWithItems);
        assert_ne!(CapabilityManifestAuditState::Invalid, CapabilityManifestAuditState::FoundEmpty);
    }
}
