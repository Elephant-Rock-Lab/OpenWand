//! Tool vocabulary — invoker, effect, and result status.
//!
//! `ToolInvoker` records WHO called a tool (for trace events).
//! `ToolEffect` records WHAT a tool does (for policy risk assessment).
//! `ToolResultStatus` records the outcome of a tool call.
//!
//! Note: `openwand-tools` defines its own `ToolSource` (dispatch routing)
//! and `openwand-policy` defines `PolicyToolSource`. These are intentionally
//! different types for different purposes.

use serde::{Deserialize, Serialize};

/// Who or what invoked a tool call.
/// Used in trace events to record provenance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolInvoker {
    Llm,
    User,
    System,
    Mcp { server: String },
}

/// What kind of side effect does this tool produce?
/// Declared at tool registration time. Used by policy for risk assessment.
/// Tools declare this; policy evaluates against it; session records it in trace.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolEffect {
    Read,
    Search,
    Write,
    Delete,
    Execute,
    Network,
    Git,
    DependencyChange,
    PolicyChange,
    PersistenceChange,
    AuthChange,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolResultStatus {
    Success,
    Error,
    Partial,
    Pending,
}
