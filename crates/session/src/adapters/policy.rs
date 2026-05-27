use openwand_policy::{PolicyToolCall, PolicyToolDescriptor, PolicyToolSource};

/// Convert tools ToolDef to policy ToolDescriptor.
pub fn tool_def_to_policy_descriptor(def: &openwand_tools::ToolDef) -> PolicyToolDescriptor {
    PolicyToolDescriptor {
        name: def.name.clone(),
        source: match &def.source {
            openwand_tools::ToolSource::Local => PolicyToolSource::Local,
            openwand_tools::ToolSource::Mcp { server, .. } => PolicyToolSource::Mcp {
                server: server.clone(),
            },
        },
        declared_effect: def.declared_effect.clone(),
        risk_hints: def.risk_hints.clone(),
        tags: def.tags.clone(),
    }
}

/// Convert session ToolCall + descriptor to policy ToolCall.
pub fn session_tool_call_to_policy(
    call: &crate::tool::ToolCall,
    descriptor: &openwand_tools::ToolDef,
) -> PolicyToolCall {
    PolicyToolCall {
        id: call.id.clone(),
        name: call.name.clone(),
        arguments: call.arguments.clone(),
        declared_effect: descriptor.declared_effect.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_core::tool_vocab::ToolEffect;
    use openwand_tools::descriptor::{ToolDef, ToolSource};

    fn test_def() -> ToolDef {
        ToolDef {
            name: "local__file_read".into(),
            display_name: None,
            description: "Read file".into(),
            parameters_schema: serde_json::json!({}),
            output_schema: None,
            source: ToolSource::Local,
            declared_effect: ToolEffect::Read,
            risk_hints: vec![],
            tags: vec![],
            annotations: None,
        }
    }

    #[test]
    fn adapter_local_tool_to_policy() {
        let def = test_def();
        let policy_desc = tool_def_to_policy_descriptor(&def);
        assert_eq!("local__file_read", policy_desc.name);
        assert_eq!(PolicyToolSource::Local, policy_desc.source);
        assert_eq!(ToolEffect::Read, policy_desc.declared_effect);
    }

    #[test]
    fn adapter_mcp_tool_to_policy() {
        let def = ToolDef {
            source: ToolSource::Mcp {
                server: "echo".into(),
                remote_name: "echo".into(),
            },
            ..test_def()
        };
        let policy_desc = tool_def_to_policy_descriptor(&def);
        match policy_desc.source {
            PolicyToolSource::Mcp { server } => assert_eq!("echo", server),
            _ => panic!("Expected MCP source"),
        }
    }
}
