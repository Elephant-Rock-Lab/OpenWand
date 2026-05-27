//! Policy request and context types.

use openwand_core::mode::InteractionMode;
use openwand_core::snapshots::GateResultSnapshot;
use openwand_core::SessionId;
use serde::{Deserialize, Serialize};

use crate::tool::{PolicyToolCall, PolicyToolDescriptor};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyContext {
    pub working_directory: String,
    pub model: String,
    pub session_id: SessionId,
    pub recent_gate_history: Vec<GateResultSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRequest {
    pub tool_call: PolicyToolCall,
    pub mode: InteractionMode,
    pub context: PolicyContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFilterRequest {
    pub tools: Vec<PolicyToolDescriptor>,
    pub mode: InteractionMode,
    pub context: PolicyContext,
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_core::tool_vocab::ToolEffect;

    #[test]
    fn policy_request_roundtrip() {
        let req = PolicyRequest {
            tool_call: crate::tool::PolicyToolCall {
                id: openwand_core::ToolCallId::new(),
                name: "read_file".into(),
                arguments: serde_json::json!({"path": "/tmp/a.rs"}),
                declared_effect: ToolEffect::Read,
            },
            mode: InteractionMode::Conversational,
            context: PolicyContext {
                working_directory: "/home/user/project".into(),
                model: "gpt-4o".into(),
                session_id: SessionId::new(),
                recent_gate_history: vec![],
            },
        };
        let json = serde_json::to_string(&req).unwrap();
        let restored: PolicyRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req.tool_call.name, restored.tool_call.name);
        assert_eq!(req.mode, restored.mode);
        assert_eq!(
            req.context.working_directory,
            restored.context.working_directory
        );
    }
}
