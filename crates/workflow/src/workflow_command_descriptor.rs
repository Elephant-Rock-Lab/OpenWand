//! Command descriptor DTO — display-only, non-executable.
//!
//! The descriptor is structured display data. It is NOT an argv, command_args,
//! executable_path, shell, cwd, env, stdin, process, spawn, or exit_status model.

use serde::{Deserialize, Serialize};

use crate::workflow_manual_operation::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowManualCommandDescriptor {
    pub command_kind: WorkflowManualCommandKind,
    /// Display text for the operator. Not parsed, not executed.
    pub display_command: String,
    pub arguments: Vec<WorkflowManualCommandArgument>,
    pub missing_inputs: Vec<WorkflowCommandMissingInput>,
    pub safety_warnings: Vec<String>,
    pub evidence_links: Vec<WorkflowCommandEvidenceLink>,
    /// Convenience text for operator copy. Not parsed, not executed.
    pub copyable_text: String,
    /// Always true — this is display data, not a command.
    pub display_only: bool,
    /// Always false — this cannot be executed.
    pub executable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_command_descriptor_is_display_only() {
        let desc = WorkflowManualCommandDescriptor {
            command_kind: WorkflowManualCommandKind::WorkflowContinuationPropose,
            display_command: "openwand workflow-continuation propose".into(),
            arguments: vec![], missing_inputs: vec![], safety_warnings: vec![],
            evidence_links: vec![], copyable_text: "openwand workflow-continuation propose --workflow-execution-id wfx_1".into(),
            display_only: true, executable: false,
        };
        assert!(desc.display_only);
    }

    #[test]
    fn workflow_command_descriptor_is_not_executable() {
        let desc = WorkflowManualCommandDescriptor {
            command_kind: WorkflowManualCommandKind::NoCommand,
            display_command: String::new(), arguments: vec![], missing_inputs: vec![],
            safety_warnings: vec![], evidence_links: vec![], copyable_text: String::new(),
            display_only: true, executable: false,
        };
        assert!(!desc.executable);
    }

    // Patch 3: no process execution fields
    #[test]
    fn workflow_command_descriptor_has_no_process_execution_fields() {
        let desc = WorkflowManualCommandDescriptor {
            command_kind: WorkflowManualCommandKind::WorkflowLoopRecommend,
            display_command: "test".into(), arguments: vec![], missing_inputs: vec![],
            safety_warnings: vec![], evidence_links: vec![], copyable_text: "test".into(),
            display_only: true, executable: false,
        };
        let json = serde_json::to_string_pretty(&desc).unwrap().to_lowercase();
        let forbidden = ["argv", "command_args", "executable_path", "shell",
                         "cwd", "env", "stdin", "process", "spawn", "exit_status"];
        for f in &forbidden {
            assert!(!json.contains(f), "Contains forbidden field: {}", f);
        }
    }

    #[test]
    fn workflow_command_serialized_json_contains_no_argv_cwd_env_or_stdin() {
        let desc = WorkflowManualCommandDescriptor {
            command_kind: WorkflowManualCommandKind::WorkflowReconciliationReconcile,
            display_command: "reconcile".into(),
            arguments: vec![WorkflowManualCommandArgument {
                name: "id".into(), value_preview: Some("wrc_1".into()),
                source: WorkflowCommandArgumentSource::Reconciliation,
                required: true, missing: false,
            }],
            missing_inputs: vec![], safety_warnings: vec!["display only".into()],
            evidence_links: vec![], copyable_text: "openwand workflow-reconciliation reconcile".into(),
            display_only: true, executable: false,
        };
        let json = serde_json::to_string(&desc).unwrap().to_lowercase();
        assert!(!json.contains("argv"));
        assert!(!json.contains("cwd"));
        assert!(!json.contains("\"env\""));
        assert!(!json.contains("stdin"));
    }
}
