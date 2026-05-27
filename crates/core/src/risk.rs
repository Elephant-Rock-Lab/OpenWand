//! Risk level snapshots for trace events.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiskLevelSnapshot {
    Low,
    Medium,
    High,
    Critical,
}
