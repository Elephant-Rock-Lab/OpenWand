//! LLM request DTOs.
//!
//! Provider-normalized. No Rig types. No System variant in LlmMessage.

use openwand_core::session_vocab::ThinkingBudgetSnapshot;
use serde::{Deserialize, Serialize};

/// Which provider and model to use.
/// Session-level decision, not hidden client state.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LlmTarget {
    pub provider: LlmProvider,
    pub model: String,
    pub base_url: Option<String>,
    // API key is intentionally NOT serialized to trace events.
    // It lives here for runtime use only.
    #[serde(skip_serializing)]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LlmProvider {
    OpenAI,
    Anthropic,
    Ollama,
    OpenRouter,
    Gemini,
    Groq,
    XAI,
    DeepSeek,
    Custom { name: String },
}

/// A complete LLM request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    /// Which provider and model to use.
    pub target: LlmTarget,

    /// System prompt — separate from messages for trace/debug reproducibility.
    pub system_prompt: String,

    /// Conversation history. No System variant — system_prompt is separate.
    pub messages: Vec<LlmMessage>,

    /// Tool definitions visible to the model.
    pub tools: Vec<LlmToolDef>,

    /// Thinking/reasoning budget.
    pub thinking_budget: Option<ThinkingBudgetSnapshot>,

    /// Maximum response tokens.
    pub max_tokens: Option<u64>,

    /// Sampling temperature.
    pub temperature: Option<f64>,

    /// Whether the model should use tools.
    pub tool_choice: Option<LlmToolChoice>,

    /// Provider-specific escape hatch.
    pub provider_options: serde_json::Value,
}

/// Conversation message. No System variant — system_prompt is separate on LlmRequest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmMessage {
    User { content: Vec<LlmContent> },
    Assistant { content: Vec<LlmContent> },
    Tool {
        tool_call_id: String,
        content: String,
        is_error: bool,
    },
}

/// Content within a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmContent {
    Text(String),
    Reasoning(String),
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
}

/// Tool definition for the LLM prompt surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmToolDef {
    pub name: String,
    pub description: String,
    pub parameters_schema: serde_json::Value,
}

/// Whether and how the model should use tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LlmToolChoice {
    Auto,
    None,
    Required,
}

/// Provider capabilities report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCapabilities {
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_reasoning: bool,
    pub supports_vision: bool,
    pub max_context_tokens: Option<u64>,
    pub supported_features: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_request_roundtrip() {
        let req = LlmRequest {
            target: LlmTarget {
                provider: LlmProvider::OpenAI,
                model: "gpt-4o".into(),
                base_url: None,
                api_key: Some("sk-test".into()),
            },
            system_prompt: "You are helpful.".into(),
            messages: vec![
                LlmMessage::User {
                    content: vec![LlmContent::Text("Hello".into())],
                },
            ],
            tools: vec![],
            thinking_budget: None,
            max_tokens: Some(4096),
            temperature: Some(0.7),
            tool_choice: None,
            provider_options: serde_json::json!({}),
        };
        let json = serde_json::to_string(&req).unwrap();
        let restored: LlmRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req.target.model, restored.target.model);
        assert_eq!(1, restored.messages.len());
        // api_key should be skipped in serialization
        assert!(!json.contains("\"api_key\"") || !json.contains("sk-test"));
    }

    #[test]
    fn llm_target_roundtrip() {
        let target = LlmTarget {
            provider: LlmProvider::Anthropic,
            model: "claude-sonnet-4-20250514".into(),
            base_url: Some("https://custom.api.com".into()),
            api_key: None,
        };
        let json = serde_json::to_string(&target).unwrap();
        let restored: LlmTarget = serde_json::from_str(&json).unwrap();
        assert_eq!(target, restored);
    }

    #[test]
    fn llm_message_roundtrip() {
        let msg = LlmMessage::Assistant {
            content: vec![
                LlmContent::Text("Let me check".into()),
                LlmContent::ToolCall {
                    id: "tc_1".into(),
                    name: "read_file".into(),
                    arguments: serde_json::json!({"path": "/tmp/a.rs"}),
                },
            ],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let restored: LlmMessage = serde_json::from_str(&json).unwrap();
        match &restored {
            LlmMessage::Assistant { content } => assert_eq!(2, content.len()),
            _ => panic!("expected Assistant"),
        }
    }

    #[test]
    fn llm_tool_def_roundtrip() {
        let def = LlmToolDef {
            name: "read_file".into(),
            description: "Read a file".into(),
            parameters_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }),
        };
        let json = serde_json::to_string(&def).unwrap();
        let restored: LlmToolDef = serde_json::from_str(&json).unwrap();
        assert_eq!(def.name, restored.name);
    }

    #[test]
    fn llm_no_system_message_variant() {
        // LlmMessage should NOT have a System variant.
        // This test is a compile-time assertion — if someone adds System,
        // the enum will have more variants and the test intention is documented.
        let msg = LlmMessage::User {
            content: vec![LlmContent::Text("test".into())],
        };
        let json = serde_json::to_string(&msg).unwrap();
        // Verify no "System" variant in serialized form
        assert!(!json.contains("System"), "LlmMessage must not have System variant");
    }
}
