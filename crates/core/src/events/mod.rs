mod session;
pub mod inference;
mod gate;
mod tool;
mod file;
mod memory;
mod mode;
mod workflow;
mod artifact;

pub use session::*;
pub use inference::*;
pub use gate::*;
pub use tool::*;
pub use file::*;
pub use memory::*;
pub use mode::*;
pub use workflow::*;
pub use artifact::*;

use serde::{Deserialize, Serialize};

/// Top-level trace event enum.
///
/// Serialization uses internally tagged representation:
/// `{ "family": "tool", "payload": { "Called": { ... } } }`
///
/// Family names and event kind strings are permanent — once persisted,
/// they can never change.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "family", content = "payload", rename_all = "snake_case")]
pub enum OpenWandTraceEvent {
    Session(SessionEvent),
    Inference(InferenceEvent),
    Gate(GateEvent),
    Tool(ToolEvent),
    File(FileEvent),
    Memory(MemoryEvent),
    Mode(ModeEvent),
    Workflow(WorkflowEvent),
    Artifact(ArtifactEvent),
}

impl OpenWandTraceEvent {
    /// Broad family name for general filtering.
    pub fn event_family(&self) -> &'static str {
        match self {
            Self::Session(_) => "session",
            Self::Inference(_) => "inference",
            Self::Gate(_) => "gate",
            Self::Tool(_) => "tool",
            Self::File(_) => "file",
            Self::Memory(_) => "memory",
            Self::Mode(_) => "mode",
            Self::Workflow(_) => "workflow",
            Self::Artifact(_) => "artifact",
        }
    }

    /// Stable dotted name for indexed trace queries.
    /// Example: "tool.called", "memory.fact_accepted"
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Session(e) => e.event_kind(),
            Self::Inference(e) => e.event_kind(),
            Self::Gate(e) => e.event_kind(),
            Self::Tool(e) => e.event_kind(),
            Self::File(e) => e.event_kind(),
            Self::Memory(e) => e.event_kind(),
            Self::Mode(e) => e.event_kind(),
            Self::Workflow(e) => e.event_kind(),
            Self::Artifact(e) => e.event_kind(),
        }
    }

    /// Schema version for this event's payload.
    /// Increment per event family when semantics change.
    pub fn schema_version(&self) -> u16 {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: construct one event from each family for round-trip testing.
    fn one_per_family() -> Vec<(&'static str, OpenWandTraceEvent)> {
        use crate::ids::*;
        use crate::mode::*;
        use crate::risk::*;
        use crate::session_vocab::*;
        use crate::snapshots::*;
        use crate::tool_vocab::*;

        vec![
            ("session", OpenWandTraceEvent::Session(
                SessionEvent::Started {
                    session_id: SessionId::new(),
                    mode: InteractionMode::Conversational,
                },
            )),
            ("inference", OpenWandTraceEvent::Inference(
                InferenceEvent::Called {
                    model: "gpt-4o".into(),
                    provider: "openai".into(),
                    prompt_hash: "abc123".into(),
                    thinking_budget: Some(ThinkingBudgetSnapshot::Medium),
                    prompt_assembly: PromptAssemblySnapshot {
                        system_prompt_hash: "h1".into(),
                        message_window_hash: "h2".into(),
                        memory_hit_ids: vec![],
                        memory_context_hash: None,
                        tool_manifest_hash: "h3".into(),
                        policy_filter_hash: "h4".into(),
                        mode: InteractionMode::Conversational,
                        working_directory: "/tmp".into(),
                    },
                },
            )),
            ("gate", OpenWandTraceEvent::Gate(
                GateEvent::Evaluated {
                    gate_id: "g1".into(),
                    gate_kind: "risk".into(),
                    passed: true,
                    risk_level: Some(RiskLevelSnapshot::Low),
                    reason_code: None,
                    summary: "ok".into(),
                },
            )),
            ("tool", OpenWandTraceEvent::Tool(
                ToolEvent::Called {
                    tool_call_id: ToolCallId::new(),
                    tool_name: "read_file".into(),
                    args_hash: "h1".into(),
                    invoker: ToolInvoker::Llm,
                },
            )),
            ("file", OpenWandTraceEvent::File(
                FileEvent::Read {
                    path: "/tmp/test.rs".into(),
                    bytes: Some(42),
                },
            )),
            ("memory", OpenWandTraceEvent::Memory(
                MemoryEvent::FactExtracted {
                    claim_id: ClaimId::new(),
                    statement: "X uses Y".into(),
                    confidence: 0.89,
                    predicate: "uses".into(),
                },
            )),
            ("mode", OpenWandTraceEvent::Mode(
                ModeEvent::Changed {
                    from: InteractionMode::Direct,
                    to: InteractionMode::Conversational,
                    trigger: "user request".into(),
                    accuracy_check: None,
                },
            )),
            ("workflow", OpenWandTraceEvent::Workflow(
                WorkflowEvent::ModStarted {
                    mod_id: "m1".into(),
                    mod_name: "standard".into(),
                },
            )),
            ("artifact", OpenWandTraceEvent::Artifact(
                ArtifactEvent::Generated {
                    paths: vec!["README.md".into()],
                    artifact_kind: "document".into(),
                    accuracy: AccuracyRecordSnapshot {
                        commit_hash: None,
                        file_coverage: 1.0,
                        sensitivity: "low".into(),
                    },
                },
            )),
        ]
    }

    #[test]
    fn core_event_roundtrip_all_families() {
        for (name, event) in one_per_family() {
            let json = serde_json::to_string(&event).unwrap();
            let restored: OpenWandTraceEvent = serde_json::from_str(&json).unwrap();

            // Kind strings must survive round-trip
            assert_eq!(
                event.event_kind(),
                restored.event_kind(),
                "{name}: event_kind mismatch after round-trip"
            );
            assert_eq!(
                event.event_family(),
                restored.event_family(),
                "{name}: event_family mismatch after round-trip"
            );
            assert_eq!(
                event.schema_version(),
                restored.schema_version(),
                "{name}: schema_version mismatch after round-trip"
            );
        }
    }

    #[test]
    fn event_family_returns_expected_family() {
        for (expected_family, event) in one_per_family() {
            assert_eq!(
                expected_family,
                event.event_family(),
                "event_family mismatch"
            );
        }
    }

    #[test]
    fn event_kind_returns_dotted_stable_name() {
        for (_, event) in one_per_family() {
            let kind = event.event_kind();
            // Must contain exactly one dot
            let dots = kind.chars().filter(|c| *c == '.').count();
            assert_eq!(
                1, dots,
                "event_kind '{kind}' does not have exactly one dot"
            );
            // Must start with the family name
            let family = event.event_family();
            assert!(
                kind.starts_with(family),
                "event_kind '{kind}' does not start with family '{family}'"
            );
        }
    }

    #[test]
    fn schema_version_is_one() {
        for (_, event) in one_per_family() {
            assert_eq!(1, event.schema_version());
        }
    }

    #[test]
    fn serde_uses_family_payload_shape() {
        // Verify the serialization shape: { "family": "...", "payload": { ... } }
        let event = OpenWandTraceEvent::Tool(ToolEvent::Denied {
            tool_call_id: crate::ids::ToolCallId::new(),
            tool_name: "rm_rf".into(),
            approval_request_id: None,
            reason: None,
        });
        let json = serde_json::to_value(&event).unwrap();

        // Must be an object with "family" and "payload" keys
        assert!(json.is_object(), "event did not serialize as JSON object");
        let obj = json.as_object().unwrap();
        assert!(obj.contains_key("family"), "missing 'family' key");
        assert!(obj.contains_key("payload"), "missing 'payload' key");
        assert_eq!("tool", obj["family"]);

        // Payload must contain the variant name
        let payload = &obj["payload"];
        assert!(payload.is_object(), "payload is not an object");
        let payload_obj = payload.as_object().unwrap();
        assert!(payload_obj.contains_key("Denied"), "payload missing 'Denied' variant");
    }
}
