//! Memory domain vocabulary — entity kinds, predicates, claim types.
//!
//! These are shared vocabulary enums used by both the memory crate and trace events.
//! Rich domain structs (Entity, Fact, Decision, etc.) live in openwand-memory.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityKind {
    Project,
    Repository,
    File,
    Module,
    Function,
    Class,
    Dependency,
    Tool,
    Command,
    ArchitectureComponent,
    Decision,
    Constraint,
    Preference,
    Bug,
    Test,
    Task,
    Concept,
    Technology,
    Custom(String),
}

impl EntityKind {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Project => "project",
            Self::Repository => "repository",
            Self::File => "file",
            Self::Module => "module",
            Self::Function => "function",
            Self::Class => "class",
            Self::Dependency => "dependency",
            Self::Tool => "tool",
            Self::Command => "command",
            Self::ArchitectureComponent => "architecture_component",
            Self::Decision => "decision",
            Self::Constraint => "constraint",
            Self::Preference => "preference",
            Self::Bug => "bug",
            Self::Test => "test",
            Self::Task => "task",
            Self::Concept => "concept",
            Self::Technology => "technology",
            Self::Custom(s) => s,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Predicate {
    Uses,
    DependsOn,
    Implements,
    Replaces,
    Rejects,
    Prefers,
    Requires,
    Forbids,
    CausedBy,
    FixedBy,
    TestedBy,
    LocatedIn,
    Supersedes,
    DecidedBecause,
    Contradicts,
    Refines,
    Custom(String),
}

impl Predicate {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Uses => "uses",
            Self::DependsOn => "depends_on",
            Self::Implements => "implements",
            Self::Replaces => "replaces",
            Self::Rejects => "rejects",
            Self::Prefers => "prefers",
            Self::Requires => "requires",
            Self::Forbids => "forbids",
            Self::CausedBy => "caused_by",
            Self::FixedBy => "fixed_by",
            Self::TestedBy => "tested_by",
            Self::LocatedIn => "located_in",
            Self::Supersedes => "supersedes",
            Self::DecidedBecause => "decided_because",
            Self::Contradicts => "contradicts",
            Self::Refines => "refines",
            Self::Custom(s) => s,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClaimKind {
    Fact,
    Decision,
    Preference,
    Constraint,
    ArchitectureNote,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClaimStatusSnapshot {
    Active,
    Superseded,
    Invalidated,
    Reverted,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryScope {
    Global,
    Project { repo: String },
    Session { session_id: String },
}

/// Provenance of a claim — who or what produced it.
/// Note: `LlmExtracted` uses a u16 for confidence (0-10000, representing 0.0000-1.0000)
/// because f64 cannot derive Eq/Hash. Use `confidence_bps() / 10000.0` for the float value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProvenanceSnapshot {
    UserStated,
    LlmExtracted { model: String, confidence_bps: u16 },
    SystemDerived { rule: String },
}

impl ProvenanceSnapshot {
    /// Returns confidence as a float in [0.0, 1.0].
    /// Returns None for non-LLM provenance.
    pub fn confidence(&self) -> Option<f64> {
        match self {
            Self::LlmExtracted { confidence_bps, .. } => Some(*confidence_bps as f64 / 10000.0),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfidenceLevel {
    Explicit,
    Inferred,
    Speculative,
}
