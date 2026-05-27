use crate::message::{Message, MessageContent, MessageRole};
use openwand_llm::{LlmContent, LlmMessage, LlmToolDef};

/// Convert session ToolDef to LLM tool definition.
pub fn tool_def_to_llm_tool(def: &openwand_tools::ToolDef) -> LlmToolDef {
    LlmToolDef {
        name: def.name.clone(),
        description: def.description.clone(),
        parameters_schema: def.parameters_schema.clone(),
    }
}

/// Convert session messages to LLM messages.
/// Returns None for unsupported roles (System is handled separately).
pub fn message_to_llm_message(message: &Message) -> Option<LlmMessage> {
    match &message.role {
        MessageRole::User => match &message.content {
            MessageContent::Text { text } => Some(LlmMessage::User {
                content: vec![LlmContent::Text(text.clone())],
            }),
            _ => None,
        },
        MessageRole::Assistant => match &message.content {
            MessageContent::Text { text } => Some(LlmMessage::Assistant {
                content: vec![LlmContent::Text(text.clone())],
            }),
            _ => None,
        },
        MessageRole::Tool => match &message.content {
            MessageContent::ToolResult {
                tool_call_id,
                result,
                is_error,
                ..
            } => Some(LlmMessage::Tool {
                tool_call_id: tool_call_id.as_str().to_string(),
                content: result.clone(),
                is_error: *is_error,
            }),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_core::ToolCallId;

    #[test]
    fn adapter_user_message_to_llm() {
        let msg = Message::user("hello");
        let llm = message_to_llm_message(&msg).unwrap();
        match llm {
            LlmMessage::User { content } => {
                assert_eq!(1, content.len());
            }
            _ => panic!("Expected User message"),
        }
    }

    #[test]
    fn adapter_tool_result_to_llm() {
        let msg = Message::tool_result(
            ToolCallId("tc_1".into()),
            "local__file_read",
            "file contents",
            false,
        );
        let llm = message_to_llm_message(&msg).unwrap();
        match llm {
            LlmMessage::Tool {
                tool_call_id,
                is_error,
                ..
            } => {
                assert_eq!("tc_1", tool_call_id);
                assert!(!is_error);
            }
            _ => panic!("Expected Tool message"),
        }
    }
}
