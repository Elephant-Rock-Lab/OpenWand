use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("tool registry error: {0}")]
    Registry(String),

    #[error("MCP refresh failed: {0}")]
    McpRefresh(String),

    #[error("invalid tool descriptor: {0}")]
    InvalidDescriptor(String),
}
