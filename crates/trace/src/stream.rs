//! Stream identifiers and scopes.

use serde::{Deserialize, Serialize};

/// Identifies a stream within the trace log.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TraceStreamId {
    pub scope: TraceStreamScope,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TraceStreamScope {
    Global,
    Session,
    Claim,
    Entity,
    Workflow,
    Artifact,
    ToolCall,
    MemoryPipelineRun,
}

/// Prevents duplicate appends on retry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct IdempotencyKey(pub String);

/// BLAKE3 hash of entry content for integrity verification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct EntryHash(pub String);
