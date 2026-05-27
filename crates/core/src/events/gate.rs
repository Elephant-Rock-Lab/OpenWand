use serde::{Deserialize, Serialize};

use crate::risk::RiskLevelSnapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateEvent {
    Evaluated {
        gate_id: String,
        gate_kind: String,
        passed: bool,
        risk_level: Option<RiskLevelSnapshot>,
        reason_code: Option<String>,
        summary: String,
    },
    BatchCompleted {
        total: u8,
        passed: u8,
        failed: u8,
        overall_risk: RiskLevelSnapshot,
    },
}

impl GateEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Evaluated { .. } => "gate.evaluated",
            Self::BatchCompleted { .. } => "gate.batch_completed",
        }
    }
}
