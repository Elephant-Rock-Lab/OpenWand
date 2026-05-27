use thiserror::Error;

#[derive(Debug, Error)]
pub enum McpPoolError {
    #[error("MCP server '{server}' is not configured")]
    ServerNotConfigured { server: String },

    #[error("MCP server '{server}' is disabled")]
    ServerDisabled { server: String },

    #[error("MCP server '{server}' failed to start: {reason}")]
    StartFailed { server: String, reason: String },

    #[error("MCP tool discovery failed for server '{server}': {reason}")]
    DiscoveryFailed { server: String, reason: String },

    #[error("MCP tool call failed for server '{server}', tool '{tool}': {reason}")]
    CallFailed {
        server: String,
        tool: String,
        reason: String,
    },

    #[error("MCP transport error: {0}")]
    Transport(String),

    #[error("MCP protocol error: {0}")]
    Protocol(String),
}
