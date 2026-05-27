use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum McpServerState {
    NotStarted,
    Starting,
    Ready {
        started_at: DateTime<Utc>,
        last_discovered_at: Option<DateTime<Utc>>,
    },
    Failed {
        reason: String,
        failed_at: DateTime<Utc>,
    },
    Stopped,
}
