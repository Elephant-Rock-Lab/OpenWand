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

/// Maximum size of approval context arguments in canonical JSON bytes.
/// Arguments exceeding this cap are blocked fail-closed before suspension.
pub const MAX_APPROVAL_CONTEXT_ARG_BYTES: usize = 1_048_576; // 1 MiB

/// Full context snapshot for a suspended tool approval.
///
/// Embedded in `ToolEvent::Suspended` to enable crash recovery.
/// After restart, the approval UI and tool execution can be reconstructed
/// from this snapshot alone, without requiring any in-memory state.
///
/// Arguments are inline only in Wave 03d, capped at `MAX_APPROVAL_CONTEXT_ARG_BYTES`.
/// Blob-backed argument storage is a future enhancement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalContextSnapshot {
    pub approval_request_id: crate::ApprovalRequestId,
    pub gate_id: crate::GateId,
    pub step: u64,
    pub tool_call_id: crate::ToolCallId,
    pub tool_name: String,
    /// Full re-executable arguments. Required for restart recovery.
    pub arguments: serde_json::Value,
    /// Hash of canonicalized arguments for UI verification and audit.
    pub args_hash: String,
    pub declared_effect: crate::ToolEffect,
    pub risk_level: crate::RiskLevelSnapshot,
    pub confirmation_level: crate::ConfirmationLevel,
    pub reason_code: String,
    pub policy_summary: String,
    pub requested_action_summary: String,
    pub rollback_plan: Option<String>,
    /// Optional future-proof metadata, not used for authority.
    #[serde(default)]
    pub metadata: serde_json::Value,
    /// Canonical workspace root captured at policy/suspension boundary (Wave 69B).
    /// Set when the approval request is created. Used to reject resume with different workspace.
    /// None for pre-69B snapshots — resume rejects these (fail-closed).
    #[serde(default)]
    pub canonical_workspace: Option<String>,
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
    fn approval_context_snapshot_roundtrip() {
        let snap = ApprovalContextSnapshot {
            approval_request_id: crate::ApprovalRequestId::new(),
            gate_id: crate::GateId::new(),
            step: 1,
            tool_call_id: crate::ToolCallId::new(),
            tool_name: "local__file_write".into(),
            arguments: serde_json::json!({"path": "test.txt", "content": "hello"}),
            args_hash: "sha256:abc123".into(),
            declared_effect: crate::ToolEffect::Write,
            risk_level: crate::RiskLevelSnapshot::Medium,
            confirmation_level: crate::ConfirmationLevel::Approve,
            reason_code: "write-requires-approve".into(),
            policy_summary: "Write tool requires approval".into(),
            requested_action_summary: "Write to test.txt".into(),
            rollback_plan: Some("Delete test.txt".into()),
            metadata: serde_json::Value::Null,
            canonical_workspace: None,
        };
        roundtrip(&snap);
    }

    #[test]
    fn approval_context_snapshot_backward_compat() {
        // Old serialized ApprovalContextSnapshot without metadata field
        // should deserialize with metadata = Value::Null via #[serde(default)]
        let json = r#"{"approval_request_id":"ar_1","gate_id":"g_1","step":1,"tool_call_id":"tc_1","tool_name":"t","arguments":{},"args_hash":"h","declared_effect":"Write","risk_level":"Medium","confirmation_level":"Approve","reason_code":"r","policy_summary":"s","requested_action_summary":"a","rollback_plan":null}"#;
        let snap: ApprovalContextSnapshot = serde_json::from_str(json).unwrap();
        assert!(snap.metadata.is_null());
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
