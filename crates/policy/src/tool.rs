//! Policy's own view of tool calls and descriptors.
//!
//! These DTOs are policy-owned — session adapts from its internal types.
//! Policy never depends on openwand-tools.

use openwand_core::tool_vocab::ToolEffect;
use openwand_core::ToolCallId;
use serde::{Deserialize, Serialize};

/// Neutral tool call representation for policy evaluation.
/// Session constructs this from its internal ToolCall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyToolCall {
    pub id: ToolCallId,
    pub name: String,
    pub arguments: serde_json::Value,
    pub declared_effect: ToolEffect,
}

/// Neutral tool descriptor for manifest filtering.
/// Tools register this with the policy engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyToolDescriptor {
    pub name: String,
    pub source: PolicyToolSource,
    pub declared_effect: ToolEffect,
    pub risk_hints: Vec<String>,
    pub tags: Vec<String>,
}

/// Where a tool lives. Policy's own enum — no dependency on tools crate.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PolicyToolSource {
    Local,
    Mcp { server: String },
    System,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_tool_call_roundtrip() {
        let call = PolicyToolCall {
            id: ToolCallId::new(),
            name: "read_file".into(),
            arguments: serde_json::json!({"path": "/tmp/test.rs"}),
            declared_effect: ToolEffect::Read,
        };
        let json = serde_json::to_string(&call).unwrap();
        let restored: PolicyToolCall = serde_json::from_str(&json).unwrap();
        assert_eq!(call.name, restored.name);
        assert_eq!(call.declared_effect, restored.declared_effect);
    }

    #[test]
    fn policy_tool_descriptor_roundtrip() {
        let desc = PolicyToolDescriptor {
            name: "bash".into(),
            source: PolicyToolSource::Local,
            declared_effect: ToolEffect::Execute,
            risk_hints: vec!["arbitrary_code".into()],
            tags: vec!["shell".into(), "dangerous".into()],
        };
        let json = serde_json::to_string(&desc).unwrap();
        let restored: PolicyToolDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(desc.name, restored.name);
        assert_eq!(desc.source, restored.source);
        assert_eq!(2, restored.tags.len());
    }
}
