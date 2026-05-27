//! Actor — who or what caused a trace event.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Actor {
    User,
    Llm { model: String, provider: String },
    System { component: String },
    MemoryPipeline,
    WorkflowEngine,
    PolicyEngine,
}
