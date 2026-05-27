//! Snapshot DTOs for trace events.
//!
//! These are thin serializable records captured at a point in time.
//! They are NOT rich domain types — they exist to persist domain-relevant
//! metadata inside trace entries.
//!
//! Derives: `Debug, Clone, PartialEq, Serialize, Deserialize`.
//! No `Eq + Hash` on types containing `f64`.

use serde::{Deserialize, Serialize};

use crate::mode::InteractionMode;
use crate::risk::RiskLevelSnapshot;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenUsageSnapshot {
    pub input: u64,
    pub output: u64,
    pub reasoning: Option<u64>,
    pub cache_read: Option<u64>,
    pub cache_write: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GateResultSnapshot {
    pub gate_kind: String,
    pub passed: bool,
    pub risk_level: Option<RiskLevelSnapshot>,
    pub reason_code: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccuracyRecordSnapshot {
    pub commit_hash: Option<String>,
    pub file_coverage: f64,
    pub sensitivity: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccuracyCheckSnapshot {
    pub artifact: String,
    pub commit_hash: String,
    pub file_coverage: f64,
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PromptAssemblySnapshot {
    pub system_prompt_hash: String,
    pub message_window_hash: String,
    pub memory_hit_ids: Vec<String>,
    pub memory_context_hash: Option<String>,
    pub tool_manifest_hash: String,
    pub policy_filter_hash: String,
    pub mode: InteractionMode,
    pub working_directory: String,
}

/// Serializable error summary for trace events.
/// Only used when an error needs to be persisted in a trace entry.
/// Not a replacement for crate-specific error types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorSnapshot {
    pub kind: String,
    pub message: String,
    pub recoverable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip<T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug>(
        value: &T,
    ) {
        let json = serde_json::to_string(value).unwrap();
        let restored: T = serde_json::from_str(&json).unwrap();
        assert_eq!(value, &restored, "round-trip failed for: {json}");
    }

    #[test]
    fn token_usage_snapshot_roundtrip() {
        let snap = TokenUsageSnapshot {
            input: 1000,
            output: 500,
            reasoning: Some(200),
            cache_read: None,
            cache_write: Some(50),
        };
        roundtrip(&snap);
    }

    #[test]
    fn gate_result_snapshot_roundtrip() {
        let snap = GateResultSnapshot {
            gate_kind: "risk_assessment".into(),
            passed: true,
            risk_level: Some(RiskLevelSnapshot::Low),
            reason_code: Some("effect.read".into()),
            summary: "Read-only tool, auto-allowed".into(),
        };
        roundtrip(&snap);
    }

    #[test]
    fn accuracy_record_snapshot_roundtrip() {
        let snap = AccuracyRecordSnapshot {
            commit_hash: Some("abc123".into()),
            file_coverage: 0.85,
            sensitivity: "high".into(),
        };
        roundtrip(&snap);
    }

    #[test]
    fn accuracy_check_snapshot_roundtrip() {
        let snap = AccuracyCheckSnapshot {
            artifact: "crates/core/src/lib.rs".into(),
            commit_hash: "def456".into(),
            file_coverage: 0.92,
            stale: false,
        };
        roundtrip(&snap);
    }

    #[test]
    fn prompt_assembly_snapshot_roundtrip() {
        let snap = PromptAssemblySnapshot {
            system_prompt_hash: "sha256:abc".into(),
            message_window_hash: "sha256:def".into(),
            memory_hit_ids: vec!["hit_1".into(), "hit_2".into()],
            memory_context_hash: Some("sha256:ghi".into()),
            tool_manifest_hash: "sha256:jkl".into(),
            policy_filter_hash: "sha256:mno".into(),
            mode: InteractionMode::Conversational,
            working_directory: "/home/user/project".into(),
        };
        roundtrip(&snap);
    }

    #[test]
    fn error_snapshot_roundtrip() {
        let snap = ErrorSnapshot {
            kind: "network".into(),
            message: "connection refused".into(),
            recoverable: true,
        };
        roundtrip(&snap);
    }

    #[test]
    fn snapshots_reexported_from_core() {
        // Verify these types are accessible from openwand_core::*
        use crate::{
            TokenUsageSnapshot, GateResultSnapshot, AccuracyRecordSnapshot,
            AccuracyCheckSnapshot, PromptAssemblySnapshot, ErrorSnapshot,
        };
        let _ = TokenUsageSnapshot {
            input: 0, output: 0, reasoning: None, cache_read: None, cache_write: None,
        };
        let _ = GateResultSnapshot {
            gate_kind: String::new(), passed: false, risk_level: None,
            reason_code: None, summary: String::new(),
        };
        let _ = AccuracyRecordSnapshot {
            commit_hash: None, file_coverage: 0.0, sensitivity: String::new(),
        };
        let _ = AccuracyCheckSnapshot {
            artifact: String::new(), commit_hash: String::new(),
            file_coverage: 0.0, stale: false,
        };
        let _ = PromptAssemblySnapshot {
            system_prompt_hash: String::new(), message_window_hash: String::new(),
            memory_hit_ids: vec![], memory_context_hash: None,
            tool_manifest_hash: String::new(), policy_filter_hash: String::new(),
            mode: crate::InteractionMode::Direct,
            working_directory: String::new(),
        };
        let _ = ErrorSnapshot {
            kind: String::new(), message: String::new(), recoverable: false,
        };
    }
}
