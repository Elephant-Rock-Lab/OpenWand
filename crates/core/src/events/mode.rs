use serde::{Deserialize, Serialize};

use crate::mode::InteractionMode;
use crate::snapshots::AccuracyCheckSnapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModeEvent {
    Changed {
        from: InteractionMode,
        to: InteractionMode,
        trigger: String,
        accuracy_check: Option<AccuracyCheckSnapshot>,
    },
}

impl ModeEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Changed { .. } => "mode.changed",
        }
    }
}
