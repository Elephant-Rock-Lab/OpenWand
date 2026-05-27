use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileEvent {
    Read {
        path: String,
        bytes: Option<u64>,
    },
    Written {
        path: String,
        diff_hash: String,
        lines_added: u32,
        lines_removed: u32,
    },
    Deleted {
        path: String,
    },
}

impl FileEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Read { .. } => "file.read",
            Self::Written { .. } => "file.written",
            Self::Deleted { .. } => "file.deleted",
        }
    }
}
