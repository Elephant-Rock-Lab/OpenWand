//! Canonical tool naming.
//!
//! Format:
//! - Local tools: `local__{tool}`
//! - MCP tools:   `mcp__{server}__{remote_tool}`

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedToolName {
    Local { name: String },
    Mcp {
        server: String,
        remote_name: String,
    },
}

pub fn canonical_local_tool_name(name: &str) -> String {
    format!("local__{name}")
}

pub fn canonical_mcp_tool_name(server: &str, remote_name: &str) -> String {
    format!("mcp__{server}__{remote_name}")
}

pub fn parse_canonical_tool_name(name: &str) -> Option<ParsedToolName> {
    if let Some(rest) = name.strip_prefix("local__") {
        if rest.is_empty() {
            return None;
        }
        return Some(ParsedToolName::Local {
            name: rest.to_string(),
        });
    }

    if let Some(rest) = name.strip_prefix("mcp__") {
        let (server, remote_name) = rest.split_once("__")?;
        if server.is_empty() || remote_name.is_empty() {
            return None;
        }
        return Some(ParsedToolName::Mcp {
            server: server.to_string(),
            remote_name: remote_name.to_string(),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_tool_name_local() {
        let name = canonical_local_tool_name("read_file");
        assert_eq!("local__read_file", name);
        let parsed = parse_canonical_tool_name(&name).unwrap();
        assert_eq!(
            ParsedToolName::Local {
                name: "read_file".into()
            },
            parsed
        );
    }

    #[test]
    fn canonical_tool_name_mcp() {
        let name = canonical_mcp_tool_name("filesystem", "read_file");
        assert_eq!("mcp__filesystem__read_file", name);
        let parsed = parse_canonical_tool_name(&name).unwrap();
        assert_eq!(
            ParsedToolName::Mcp {
                server: "filesystem".into(),
                remote_name: "read_file".into()
            },
            parsed
        );
    }

    #[test]
    fn parse_rejects_empty_names() {
        assert!(parse_canonical_tool_name("local__").is_none());
        assert!(parse_canonical_tool_name("mcp____tool").is_none());
        assert!(parse_canonical_tool_name("mcp__server__").is_none());
        assert!(parse_canonical_tool_name("mcp___").is_none());
        assert!(parse_canonical_tool_name("unknown_prefix__tool").is_none());
    }
}
