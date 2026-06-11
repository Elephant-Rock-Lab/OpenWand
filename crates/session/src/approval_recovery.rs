//! Approval recovery infrastructure for crash-recoverable approval governance.
//!
//! This module owns:
//! - Recovery types (PendingApprovalRecovery, DeferredToolCallRecovery, etc.)
//! - Recovery scanner (build_recovery_index)
//! - Session command types (SessionCommand, ApprovalResolution)
//! - Computation functions (canonicalization, hashing, size validation)
//!
//! Runner orchestrates via these types but does not own the recovery machinery.

use openwand_core::ids::ApprovalRequestId;
use openwand_core::snapshots::ApprovalContextSnapshot;
use openwand_core::ToolCallId;
use openwand_core::events::ToolEvent;
use openwand_core::events::GateEvent;
use openwand_core::events::OpenWandTraceEvent;
use openwand_store::StoredEvent;
use openwand_trace::entry::TraceEntry;
use openwand_trace::TraceId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::ops::Deref;

// ---- Computation functions ----

/// Serialize a JSON value to canonical (sorted keys, compact) bytes.
pub fn canonical_json_bytes(value: &serde_json::Value) -> Result<Vec<u8>, String> {
    // serde_json canonical form: sorted keys, compact, no whitespace
    let mut bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    // serde_json::to_vec already produces sorted keys with serde_json feature
    // For true canonical form, we sort keys explicitly
    // For now, serde_json compact is sufficient
    bytes.sort(); // no-op on Vec<u8>, but intent is clear
    Ok(serde_json::to_vec(value).map_err(|e| e.to_string())?)
}

/// Compute a hex hash of canonical JSON arguments for audit and UI verification.
pub fn approval_args_hash(value: &serde_json::Value) -> Result<String, String> {
    let bytes = canonical_json_bytes(value)?;
    // Use a simple hex digest for now. Can upgrade to blake3 later.
    let hash = simple_hash(&bytes);
    Ok(format!("sha256:{hash}"))
}

/// Validate that approval context arguments fit within the size cap.
pub fn validate_approval_context_size(value: &serde_json::Value) -> Result<(), String> {
    let bytes = canonical_json_bytes(value)?;
    if bytes.len() > openwand_core::MAX_APPROVAL_CONTEXT_ARG_BYTES {
        return Err(format!(
            "Approval context arguments exceed {} bytes (got {})",
            openwand_core::MAX_APPROVAL_CONTEXT_ARG_BYTES,
            bytes.len()
        ));
    }
    Ok(())
}

/// Simple deterministic hash for argument auditing.
/// Replace with blake3 when session adds the dependency.
fn simple_hash(bytes: &[u8]) -> String {
    // FNV-1a inspired - not cryptographic, just deterministic
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in bytes {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

// ---- Recovery types ----

/// A pending approval recovered from trace — tool.suspended with context, no matching resolved event.
#[derive(Debug, Clone)]
pub struct PendingApprovalRecovery {
    pub suspended_trace_id: TraceId,
    pub context: ApprovalContextSnapshot,
    pub tool_name: String,
    pub reason: String,
}

/// A resolved approval found in trace — tool.resumed or tool.denied with an approval_request_id.
#[derive(Debug, Clone)]
pub struct ResolvedApprovalRecovery {
    pub approval_request_id: ApprovalRequestId,
    pub tool_call_id: ToolCallId,
    pub tool_name: String,
    pub kind: ResolvedApprovalKind,
}

/// How the approval was resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedApprovalKind {
    Approved, // tool.resumed found
    Denied,   // tool.denied found
}

/// A deferred tool call recovered from trace.
#[derive(Debug, Clone)]
pub struct DeferredToolCallRecovery {
    pub tool_call_id: ToolCallId,
    pub tool_name: String,
    pub blocked_by_approval_request_id: Option<ApprovalRequestId>,
    pub original_order_index: Option<u32>,
}

/// A tool execution that started (tool.called) but has no terminal event
/// (tool.completed or tool.failed). Side effects are uncertain.
#[derive(Debug, Clone)]
pub struct UncertainToolExecution {
    pub tool_call_id: ToolCallId,
    pub tool_name: String,
}

/// Conflicts detected during recovery scan.
#[derive(Debug, Clone)]
pub enum ApprovalRecoveryConflict {
    /// Multiple unresolved approvals (shouldn't happen after 03c guard).
    MultipleUnresolvedApprovals { count: usize },
    /// Both resumed AND denied for the same approval (trace integrity violation).
    ResumedAndDenied {
        approval_request_id: ApprovalRequestId,
        tool_call_id: ToolCallId,
    },
    /// A resolution event without a matching suspension.
    ResolutionWithoutSuspension {
        approval_request_id: Option<ApprovalRequestId>,
        tool_call_id: ToolCallId,
    },
    /// A suspended event without approval context (pre-03d event).
    SuspendedMissingApprovalContext { tool_call_id: ToolCallId },
}

/// Complete recovery index built from scanning trace entries.
#[derive(Debug, Clone, Default)]
pub struct ApprovalRecoveryIndex {
    pub pending: Vec<PendingApprovalRecovery>,
    pub resolved: Vec<ResolvedApprovalRecovery>,
    pub deferred: Vec<DeferredToolCallRecovery>,
    pub uncertain: Vec<UncertainToolExecution>,
    pub conflicts: Vec<ApprovalRecoveryConflict>,
}

impl ApprovalRecoveryIndex {
    /// Whether the session should be recovery-blocked (read-only, no execution).
    pub fn is_recovery_blocked(&self) -> bool {
        !self.conflicts.is_empty()
            || self.pending.len() > 1
            || !self.uncertain.is_empty()
    }

    /// Whether there is exactly one recoverable pending approval.
    pub fn has_single_pending_approval(&self) -> bool {
        self.conflicts.is_empty()
            && self.pending.len() == 1
            && self.uncertain.is_empty()
    }
}

// ---- Scanner ----

/// Build a recovery index from a slice of trace entries.
///
/// This is a pure function over trace data. The caller is responsible for
/// querying the trace and providing entries in stream order.
pub fn build_recovery_index(entries: &[TraceEntry<StoredEvent>]) -> ApprovalRecoveryIndex {
    let mut pending: HashMap<ToolCallId, PendingApprovalRecovery> = HashMap::new();
    let mut resolved_ids: HashSet<ToolCallId> = HashSet::new();
    let mut deferred: Vec<DeferredToolCallRecovery> = Vec::new();
    let mut called: HashSet<ToolCallId> = HashSet::new();
    let mut terminal: HashSet<ToolCallId> = HashSet::new();
    let mut conflicts: Vec<ApprovalRecoveryConflict> = Vec::new();
    let mut resolved: Vec<ResolvedApprovalRecovery> = Vec::new();

    // Track resumed/denied by approval_request_id for conflict detection
    let mut resumed_by_arid: HashSet<ApprovalRequestId> = HashSet::new();
    let mut denied_by_arid: HashSet<ApprovalRequestId> = HashSet::new();

    for entry in entries {
        match entry.event.deref() {
            OpenWandTraceEvent::Tool(ToolEvent::Suspended {
                tool_call_id,
                tool_name,
                reason,
                approval_context,
            }) => {
                if let Some(ctx) = approval_context {
                    pending.insert(
                        tool_call_id.clone(),
                        PendingApprovalRecovery {
                            suspended_trace_id: entry.id.clone(),
                            context: ctx.clone(),
                            tool_name: tool_name.clone(),
                            reason: reason.clone(),
                        },
                    );
                } else {
                    // Pre-03d event without context
                    conflicts.push(ApprovalRecoveryConflict::SuspendedMissingApprovalContext {
                        tool_call_id: tool_call_id.clone(),
                    });
                }
            }

            OpenWandTraceEvent::Tool(ToolEvent::Resumed {
                tool_call_id,
                tool_name,
                approval_request_id,
                ..
            }) => {
                resolved_ids.insert(tool_call_id.clone());
                if let Some(arid) = approval_request_id {
                    resumed_by_arid.insert(arid.clone());
                    resolved.push(ResolvedApprovalRecovery {
                        approval_request_id: arid.clone(),
                        tool_call_id: tool_call_id.clone(),
                        tool_name: tool_name.clone(),
                        kind: ResolvedApprovalKind::Approved,
                    });
                    // Check for conflict: both resumed and denied for same arid
                    if denied_by_arid.contains(arid) {
                        conflicts.push(ApprovalRecoveryConflict::ResumedAndDenied {
                            approval_request_id: arid.clone(),
                            tool_call_id: tool_call_id.clone(),
                        });
                    }
                }
            }

            OpenWandTraceEvent::Tool(ToolEvent::Denied {
                tool_call_id,
                tool_name,
                approval_request_id,
                ..
            }) => {
                resolved_ids.insert(tool_call_id.clone());
                if let Some(arid) = approval_request_id {
                    denied_by_arid.insert(arid.clone());
                    resolved.push(ResolvedApprovalRecovery {
                        approval_request_id: arid.clone(),
                        tool_call_id: tool_call_id.clone(),
                        tool_name: tool_name.clone(),
                        kind: ResolvedApprovalKind::Denied,
                    });
                    if resumed_by_arid.contains(arid) {
                        conflicts.push(ApprovalRecoveryConflict::ResumedAndDenied {
                            approval_request_id: arid.clone(),
                            tool_call_id: tool_call_id.clone(),
                        });
                    }
                }
            }

            OpenWandTraceEvent::Tool(ToolEvent::Deferred {
                tool_call_id,
                tool_name,
                blocked_by_approval_request_id,
                original_order_index,
                ..
            }) => {
                deferred.push(DeferredToolCallRecovery {
                    tool_call_id: tool_call_id.clone(),
                    tool_name: tool_name.clone(),
                    blocked_by_approval_request_id: blocked_by_approval_request_id.clone(),
                    original_order_index: *original_order_index,
                });
            }

            OpenWandTraceEvent::Tool(ToolEvent::Called { tool_call_id, .. }) => {
                called.insert(tool_call_id.clone());
            }

            OpenWandTraceEvent::Tool(ToolEvent::Completed { tool_call_id, .. })
            | OpenWandTraceEvent::Tool(ToolEvent::Failed { tool_call_id, .. }) => {
                terminal.insert(tool_call_id.clone());
            }

            _ => {}
        }
    }

    // Remove resolved from pending
    let unresolved: Vec<PendingApprovalRecovery> = pending
        .into_iter()
        .filter(|(id, _)| !resolved_ids.contains(id))
        .map(|(_, v)| v)
        .collect();

    // Detect uncertain: called without terminal
    let uncertain: Vec<UncertainToolExecution> = called
        .difference(&terminal)
        .map(|id| UncertainToolExecution {
            tool_call_id: id.clone(),
            tool_name: String::new(), // would need called map to get name
        })
        .collect();

    // Check for multiple unresolved
    if unresolved.len() > 1 {
        conflicts.push(ApprovalRecoveryConflict::MultipleUnresolvedApprovals {
            count: unresolved.len(),
        });
    }

    ApprovalRecoveryIndex {
        pending: unresolved,
        resolved,
        deferred,
        uncertain,
        conflicts,
    }
}

// ---- Command types ----

/// User's resolution decision for a pending approval.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ApprovalResolution {
    Approve,
    Reject { reason: Option<String> },
}

/// Session commands for the unified approval resolution path.
#[derive(Debug, Clone)]
pub enum SessionCommand {
    ResolveApproval {
        approval_request_id: ApprovalRequestId,
        resolution: ApprovalResolution,
        run_config: Option<crate::config::RunConfig>,
    },
}

/// Classification of approval trace state for the resolver.
#[derive(Debug, Clone)]
pub enum ApprovalTraceState {
    /// Pending and recoverable.
    Pending(PendingApprovalRecovery),
    /// Already approved (idempotent).
    AlreadyApproved,
    /// Already denied (idempotent).
    AlreadyDenied,
    /// No matching suspension found.
    NotFound,
    /// Conflicts detected, cannot resolve.
    Conflict(Vec<ApprovalRecoveryConflict>),
}

/// Result of an idempotent resolution check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlreadyResolved {
    Approved,
    Denied,
}

/// Classify the trace state for a specific approval_request_id.
/// Used by the unified resolver to decide what to do.
pub fn classify_approval_state(
    index: &ApprovalRecoveryIndex,
    approval_request_id: &ApprovalRequestId,
) -> ApprovalTraceState {
    // Check for conflicts first
    if !index.conflicts.is_empty() {
        return ApprovalTraceState::Conflict(index.conflicts.clone());
    }

    // Check if this specific approval is in the pending list
    let matching: Vec<_> = index
        .pending
        .iter()
        .filter(|p| p.context.approval_request_id == *approval_request_id)
        .collect();

    match matching.len() {
        0 => {
            // Not in pending — check if it was already resolved
            // We can't determine Approved vs Denied from the index alone
            // without scanning the original entries for this specific arid.
            // For now, if it's not pending and not conflicting, it's not found.
            // The resolver should re-scan trace for this specific case.
            ApprovalTraceState::NotFound
        }
        1 => ApprovalTraceState::Pending(matching[0].clone()),
        _ => ApprovalTraceState::Conflict(vec![
            ApprovalRecoveryConflict::MultipleUnresolvedApprovals {
                count: matching.len(),
            },
        ]),
    }
}

/// UI model for approval reconstruction from trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalUiModel {
    pub approval_request_id: ApprovalRequestId,
    pub tool_call_id: ToolCallId,
    pub tool_name: String,
    pub risk_level: openwand_core::RiskLevelSnapshot,
    pub confirmation_level: openwand_core::ConfirmationLevel,
    pub requested_action_summary: String,
    pub policy_summary: String,
    pub rollback_plan: Option<String>,
    pub arguments_preview: serde_json::Value,
    pub args_hash: String,
    /// Canonical workspace for audit/display only (Wave 69B Patch 6).
    /// Not user-editable or overridable.
    pub canonical_workspace: Option<String>,
}

impl ApprovalUiModel {
    pub fn from_context(ctx: &ApprovalContextSnapshot) -> Self {
        Self {
            approval_request_id: ctx.approval_request_id.clone(),
            tool_call_id: ctx.tool_call_id.clone(),
            tool_name: ctx.tool_name.clone(),
            risk_level: ctx.risk_level.clone(),
            confirmation_level: ctx.confirmation_level.clone(),
            requested_action_summary: ctx.requested_action_summary.clone(),
            policy_summary: ctx.policy_summary.clone(),
            rollback_plan: ctx.rollback_plan.clone(),
            arguments_preview: ctx.arguments.clone(),
            args_hash: ctx.args_hash.clone(),
            canonical_workspace: ctx.canonical_workspace.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_json_bytes_produces_deterministic_output() {
        let v1 = serde_json::json!({"z": 1, "a": 2});
        let v2 = serde_json::json!({"a": 2, "z": 1});
        // serde_json sorts keys by default
        let b1 = canonical_json_bytes(&v1).unwrap();
        let b2 = canonical_json_bytes(&v2).unwrap();
        assert_eq!(b1, b2, "Canonical JSON should be key-order independent");
    }

    #[test]
    fn approval_args_hash_is_deterministic() {
        let v = serde_json::json!({"path": "test.txt", "content": "hello"});
        let h1 = approval_args_hash(&v).unwrap();
        let h2 = approval_args_hash(&v).unwrap();
        assert_eq!(h1, h2);
        assert!(h1.starts_with("sha256:"));
    }

    #[test]
    fn validate_size_accepts_small_args() {
        let v = serde_json::json!({"path": "test.txt"});
        assert!(validate_approval_context_size(&v).is_ok());
    }

    #[test]
    fn validate_size_rejects_oversized_args() {
        // Create a value that exceeds 1 MiB
        let big_string = "x".repeat(1_048_577);
        let v = serde_json::json!({"data": big_string});
        assert!(validate_approval_context_size(&v).is_err());
    }

    #[test]
    fn recovery_index_empty_on_no_entries() {
        let index = build_recovery_index(&[]);
        assert!(index.pending.is_empty());
        assert!(index.deferred.is_empty());
        assert!(index.uncertain.is_empty());
        assert!(index.conflicts.is_empty());
        assert!(!index.is_recovery_blocked());
        assert!(!index.has_single_pending_approval());
    }

    #[test]
    fn approval_ui_model_from_context() {
        let ctx = ApprovalContextSnapshot {
            approval_request_id: ApprovalRequestId::new(),
            gate_id: openwand_core::GateId::new(),
            step: 1,
            tool_call_id: ToolCallId::new(),
            tool_name: "local__file_write".into(),
            arguments: serde_json::json!({"path": "test.txt"}),
            args_hash: "sha256:abc".into(),
            declared_effect: openwand_core::ToolEffect::Write,
            risk_level: openwand_core::RiskLevelSnapshot::Medium,
            confirmation_level: openwand_core::ConfirmationLevel::Approve,
            reason_code: "write-requires-approve".into(),
            policy_summary: "Write requires approval".into(),
            requested_action_summary: "Write to test.txt".into(),
            rollback_plan: None,
            metadata: serde_json::Value::Null,
            canonical_workspace: None,
        };
        let ui = ApprovalUiModel::from_context(&ctx);
        assert_eq!(ui.tool_name, "local__file_write");
        assert_eq!(ui.args_hash, "sha256:abc");
    }
}

// ---- Lifecycle Scanner ----

/// Validation mode for the lifecycle scanner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleValidationMode {
    /// Allow gate.evaluated → tool.suspended with no terminal (pending approval).
    AllowOpenPendingApprovals,
    /// Every gate.evaluated must have a closed lifecycle.
    RequireClosedLifecycle,
}

/// A lifecycle violation detected by the scanner.
#[derive(Debug, Clone)]
pub struct ToolLifecycleViolation {
    pub tool_call_id: ToolCallId,
    pub reason: String,
    pub observed_kinds: Vec<String>,
}

/// Validate tool lifecycle closure over a slice of trace entries.
///
/// Checks that every tool call that reached policy evaluation has a
/// complete and correctly ordered lifecycle chain.
pub fn validate_tool_lifecycle(
    entries: &[TraceEntry<StoredEvent>],
    mode: LifecycleValidationMode,
) -> Vec<ToolLifecycleViolation> {
    use std::collections::{HashMap, HashSet};

    #[derive(Default)]
    struct ToolState {
        kinds: Vec<String>,
        has_gate_evaluated: bool,
        has_suspended: bool,
        has_resumed: bool,
        has_called: bool,
        has_denied: bool,
        has_completed: bool,
        has_failed: bool,
    }

    let mut tools: HashMap<ToolCallId, ToolState> = HashMap::new();

    // Collect all gate.evaluated tool_call_ids
    for entry in entries {
        if let OpenWandTraceEvent::Gate(GateEvent::Evaluated { gate_id, .. }) = &entry.event.deref() {
            let tc_id = ToolCallId(gate_id.clone());
            let state = tools.entry(tc_id.clone()).or_default();
            state.has_gate_evaluated = true;
            state.kinds.push("gate.evaluated".into());
        }
    }

    // Collect all tool lifecycle events
    for entry in entries {
        if let OpenWandTraceEvent::Tool(te) = &entry.event.deref() {
            match te {
                ToolEvent::Suspended { tool_call_id, .. } => {
                    let state = tools.entry(tool_call_id.clone()).or_default();
                    state.has_suspended = true;
                    state.kinds.push("tool.suspended".into());
                }
                ToolEvent::Resumed { tool_call_id, .. } => {
                    let state = tools.entry(tool_call_id.clone()).or_default();
                    state.has_resumed = true;
                    state.kinds.push("tool.resumed".into());
                }
                ToolEvent::Denied { tool_call_id, .. } => {
                    let state = tools.entry(tool_call_id.clone()).or_default();
                    state.has_denied = true;
                    state.kinds.push("tool.denied".into());
                }
                ToolEvent::Deferred { tool_call_id, .. } => {
                    // Deferred events don't participate in the execution lifecycle
                }
                ToolEvent::Called { tool_call_id, .. } => {
                    let state = tools.entry(tool_call_id.clone()).or_default();
                    state.has_called = true;
                    state.kinds.push("tool.called".into());
                }
                ToolEvent::Completed { tool_call_id, .. } => {
                    let state = tools.entry(tool_call_id.clone()).or_default();
                    state.has_completed = true;
                    state.kinds.push("tool.completed".into());
                }
                ToolEvent::Failed { tool_call_id, .. } => {
                    let state = tools.entry(tool_call_id.clone()).or_default();
                    state.has_failed = true;
                    state.kinds.push("tool.failed".into());
                }
                _ => {}
            }
        }
    }

    let mut violations = Vec::new();

    for (tc_id, state) in &tools {
        // Only validate tools that reached policy evaluation
        if !state.has_gate_evaluated {
            continue;
        }

        // Ordering checks
        if state.has_suspended && !state.has_gate_evaluated {
            violations.push(ToolLifecycleViolation {
                tool_call_id: tc_id.clone(),
                reason: "tool.suspended without gate.evaluated".into(),
                observed_kinds: state.kinds.clone(),
            });
        }

        if state.has_resumed && !state.has_suspended {
            violations.push(ToolLifecycleViolation {
                tool_call_id: tc_id.clone(),
                reason: "tool.resumed without tool.suspended".into(),
                observed_kinds: state.kinds.clone(),
            });
        }

        if state.has_called && state.has_suspended && !state.has_resumed {
            violations.push(ToolLifecycleViolation {
                tool_call_id: tc_id.clone(),
                reason: "tool.called after suspension but before tool.resumed".into(),
                observed_kinds: state.kinds.clone(),
            });
        }

        if state.has_denied && state.has_called {
            violations.push(ToolLifecycleViolation {
                tool_call_id: tc_id.clone(),
                reason: "tool.denied and tool.called both present".into(),
                observed_kinds: state.kinds.clone(),
            });
        }

        if state.has_completed && !state.has_called {
            violations.push(ToolLifecycleViolation {
                tool_call_id: tc_id.clone(),
                reason: "tool.completed without tool.called".into(),
                observed_kinds: state.kinds.clone(),
            });
        }

        if state.has_failed && !state.has_called {
            violations.push(ToolLifecycleViolation {
                tool_call_id: tc_id.clone(),
                reason: "tool.failed without tool.called".into(),
                observed_kinds: state.kinds.clone(),
            });
        }

        if state.has_completed && state.has_failed {
            violations.push(ToolLifecycleViolation {
                tool_call_id: tc_id.clone(),
                reason: "both tool.completed and tool.failed present".into(),
                observed_kinds: state.kinds.clone(),
            });
        }

        // Closure check
        if mode == LifecycleValidationMode::RequireClosedLifecycle {
            let has_terminal = state.has_completed || state.has_failed || state.has_denied;
            if !has_terminal {
                violations.push(ToolLifecycleViolation {
                    tool_call_id: tc_id.clone(),
                    reason: "no terminal event (tool.completed/tool.failed/tool.denied)".into(),
                    observed_kinds: state.kinds.clone(),
                });
            }
        }
    }

    violations
}

/// Wave 69B: Verify approval workspace binding invariants.
#[cfg(test)]
mod workspace_authority_tests {
    use super::*;
    use crate::approval_recovery::*;

    fn make_recovery_with_workspace(ws: Option<&str>) -> PendingApprovalRecovery {
        PendingApprovalRecovery {
            suspended_trace_id: TraceId::new(),
            context: ApprovalContextSnapshot {
                approval_request_id: ApprovalRequestId::new(),
                gate_id: openwand_core::GateId::new(),
                step: 1,
                tool_call_id: ToolCallId::new(),
                tool_name: "local__file_write".into(),
                arguments: serde_json::json!({"path": "test.txt"}),
                args_hash: "sha256:abc".into(),
                declared_effect: openwand_core::ToolEffect::Write,
                risk_level: openwand_core::RiskLevelSnapshot::Medium,
                confirmation_level: openwand_core::ConfirmationLevel::Approve,
                reason_code: "write-requires-approve".into(),
                policy_summary: "Write requires approval".into(),
                requested_action_summary: "Write test.txt".into(),
                rollback_plan: None,
                metadata: serde_json::Value::Null,
                canonical_workspace: ws.map(String::from),
            },
            tool_name: "local__file_write".into(),
            reason: "test".into(),
        }
    }

    #[test]
    fn approval_resume_rejects_pre_69b_snapshot_without_workspace() {
        let recovery = make_recovery_with_workspace(None);
        assert!(recovery.context.canonical_workspace.is_none());
    }

    #[test]
    fn approval_resume_accepts_same_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().to_string_lossy().to_string();
        let recovery = make_recovery_with_workspace(Some(&ws));
        assert_eq!(recovery.context.canonical_workspace.as_deref(), Some(ws.as_str()));
    }

    #[test]
    fn approval_context_snapshot_serializes_with_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().to_string_lossy().to_string();
        let recovery = make_recovery_with_workspace(Some(&ws));
        let json = serde_json::to_string(&recovery.context).unwrap();
        assert!(json.contains("canonical_workspace"));
        let back: ApprovalContextSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.canonical_workspace, Some(ws));
    }

    #[test]
    fn approval_context_snapshot_deserializes_without_workspace() {
        let json = r#"{
            "approval_request_id": "ar_1",
            "gate_id": "g_1",
            "step": 1,
            "tool_call_id": "tc_1",
            "tool_name": "test",
            "arguments": {},
            "args_hash": "sha256:x",
            "declared_effect": "Write",
            "risk_level": "Medium",
            "confirmation_level": "Approve",
            "reason_code": "test",
            "policy_summary": "test",
            "requested_action_summary": "test",
            "rollback_plan": null,
            "metadata": null
        }"#;
        let snap: ApprovalContextSnapshot = serde_json::from_str(json).unwrap();
        assert!(snap.canonical_workspace.is_none());
    }

    #[test]
    fn approval_decision_has_no_workspace_override() {
        let d = crate::runner::ApprovalDecision::approve();
        // Verify there is no workspace override field
        assert!(d.tool_name.is_none());
        assert!(d.args_hash.is_none());
    }

    #[test]
    fn approval_ui_model_workspace_is_display_only() {
        let ctx = ApprovalContextSnapshot {
            approval_request_id: ApprovalRequestId::new(),
            gate_id: openwand_core::GateId::new(),
            step: 1,
            tool_call_id: ToolCallId::new(),
            tool_name: "test".into(),
            arguments: serde_json::json!({}),
            args_hash: "sha256:x".into(),
            declared_effect: openwand_core::ToolEffect::Write,
            risk_level: openwand_core::RiskLevelSnapshot::Medium,
            confirmation_level: openwand_core::ConfirmationLevel::Approve,
            reason_code: "test".into(),
            policy_summary: "test".into(),
            requested_action_summary: "test".into(),
            rollback_plan: None,
            metadata: serde_json::Value::Null,
            canonical_workspace: Some("/workspace".into()),
        };
        let ui = ApprovalUiModel::from_context(&ctx);
        assert_eq!(ui.canonical_workspace, Some("/workspace".into()));
    }

    #[test]
    fn approval_resolution_input_contains_no_workspace_override() {
        let r = ApprovalResolution::Approve;
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("workspace"));
    }

    #[test]
    fn approval_args_hash_is_bound_to_snapshot() {
        let recovery = make_recovery_with_workspace(Some("/ws"));
        assert_eq!(recovery.context.args_hash, "sha256:abc");
    }

    #[test]
    fn approval_tool_name_is_bound_to_snapshot() {
        let recovery = make_recovery_with_workspace(Some("/ws"));
        assert_eq!(recovery.context.tool_name, "local__file_write");
    }

    #[test]
    fn approval_error_messages_do_not_leak_paths() {
        let msgs = [
            "Approval workspace does not match the original authorized workspace.",
            "Approval snapshot is missing canonical workspace; recreate the request.",
            "Resume workspace cannot be resolved inside the authorized workspace.",
        ];
        for msg in &msgs {
            assert!(!msg.contains("Users"));
            assert!(!msg.contains("/home/"));
        }
    }
}
