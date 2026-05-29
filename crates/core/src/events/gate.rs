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
    /// Post-inference output record screening.
    /// Records whether the durable assistant message was screened.
    /// This does NOT guarantee the user never saw the raw text
    /// (streaming remains live).
    OutputScreened {
        gate_id: String,
        passed: bool,
        forbidden_hits: Vec<String>,
        fallback_used: bool,
    },
}

impl GateEvent {
    pub fn event_kind(&self) -> &'static str {
        match self {
            Self::Evaluated { .. } => "gate.evaluated",
            Self::BatchCompleted { .. } => "gate.batch_completed",
            Self::OutputScreened { .. } => "gate.output_screened",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_risk() -> RiskLevelSnapshot {
        RiskLevelSnapshot::Low
    }

    #[test]
    fn output_screened_serializes() {
        let event = GateEvent::OutputScreened {
            gate_id: "g1".to_string(),
            passed: false,
            forbidden_hits: vec!["git pull".to_string()],
            fallback_used: true,
        };
        let json = serde_json::to_string(&event).unwrap();
        let de: GateEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(de, GateEvent::OutputScreened { .. }));
    }

    #[test]
    fn output_screened_event_kind() {
        let event = GateEvent::OutputScreened {
            gate_id: "g1".to_string(),
            passed: true,
            forbidden_hits: vec![],
            fallback_used: false,
        };
        assert_eq!("gate.output_screened", event.event_kind());
    }

    #[test]
    fn output_screened_passed_serializes() {
        let event = GateEvent::OutputScreened {
            gate_id: "g2".to_string(),
            passed: true,
            forbidden_hits: vec![],
            fallback_used: false,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"passed\":true"));
        assert!(json.contains("\"fallback_used\":false"));
    }
}
