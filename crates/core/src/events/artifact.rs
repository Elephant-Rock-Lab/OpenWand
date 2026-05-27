use serde::{Deserialize, Serialize};

use crate::snapshots::AccuracyRecordSnapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactEvent {
    Generated {
        paths: Vec<String>,
        artifact_kind: String,
        accuracy: AccuracyRecordSnapshot,
    },
    Updated {
        paths: Vec<String>,
        commit_hash: Option<String>,
    },
    Validated {
        paths: Vec<String>,
        passed: bool,
        issues: Vec<String>,
    },
}

impl ArtifactEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Generated { .. } => "artifact.generated",
            Self::Updated { .. } => "artifact.updated",
            Self::Validated { .. } => "artifact.validated",
        }
    }
}
