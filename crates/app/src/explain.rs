//! Explain — renders trust explanation for a completed session.
//!
//! `openwand explain <session_id>` produces a human-readable breakdown of:
//! - What memory was included/excluded and why (governance)
//! - What policy decisions were made (gates, approvals)
//! - What tools were called and their results (execution)
//! - What changed on disk (task summary)
//!
//! Design invariant: explain renders from the same artifacts the model consumed.
//! No raw store queries — uses PromptInputResult and trace events.

use openwand_core::ToolCallId;
use openwand_memory::governance::GovernanceFilteredReport;
use openwand_session::task_context::TaskSummary;
use serde::{Deserialize, Serialize};

/// Complete explanation of a session's behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Explanation {
    pub memory: MemoryExplanation,
    pub policy: PolicyExplanation,
    pub execution: ExecutionExplanation,
    pub completion: CompletionExplanation,
}

/// Why memory was included or excluded from the prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryExplanation {
    pub included: Vec<ClaimEntry>,
    pub excluded: Vec<ExcludedClaimEntry>,
}

/// A memory claim that was included in the prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimEntry {
    pub claim: String,
    pub confidence_bps: u32,
    pub evidence_kind: String,
    pub source: String,
}

/// A memory claim that was excluded, with reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcludedClaimEntry {
    pub claim: String,
    pub confidence_bps: u32,
    pub reason: String,
}

/// Policy gates and approval decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyExplanation {
    pub gates: Vec<GateEntry>,
    pub approvals: Vec<ApprovalEntry>,
}

/// A policy gate evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateEntry {
    pub tool_name: String,
    pub risk: String,
    pub confirmation: String,
    pub decision: String,
}

/// An approval decision (granted or denied).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalEntry {
    pub tool_name: String,
    pub decision: String,
    pub reason: Option<String>,
}

/// Tool execution chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionExplanation {
    pub tool_calls: Vec<ToolCallEntry>,
}

/// A single tool invocation and its result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallEntry {
    pub tool_name: String,
    pub call_id: String,
    pub success: bool,
    pub output_preview: String,
    pub duration_ms: Option<u64>,
}

/// Task completion summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionExplanation {
    pub completed: bool,
    pub changed_files: Vec<String>,
    pub diff_stat: Option<String>,
    pub test_output: Option<String>,
}

/// Build a MemoryExplanation from a GovernanceFilteredReport.
impl MemoryExplanation {
    pub fn from_governed_report(report: &GovernanceFilteredReport) -> Self {
        let mut included = Vec::new();
        let mut excluded = Vec::new();

        for finding in &report.included_claims {
            if let Some(ref claim_text) = finding.finding.claim_text {
                included.push(ClaimEntry {
                    claim: claim_text.clone(),
                    confidence_bps: 0,
                    evidence_kind: finding.finding.evidence_kind
                        .map(|k| format!("{:?}", k))
                        .unwrap_or_default(),
                    source: format!("{:?}", finding.finding.kind),
                });
            }
        }

        for finding in &report.audit_only_claims {
            if let Some(ref claim_text) = finding.finding.claim_text {
                excluded.push(ExcludedClaimEntry {
                    claim: claim_text.clone(),
                    confidence_bps: 0,
                    reason: finding.governance_reasons.join("; "),
                });
            }
        }

        MemoryExplanation { included, excluded }
    }
}

/// Build a CompletionExplanation from a TaskSummary.
impl CompletionExplanation {
    pub fn from_task_summary(summary: &TaskSummary) -> Self {
        CompletionExplanation {
            completed: summary.completed,
            changed_files: summary.changed_files.clone(),
            diff_stat: summary.diff_stat.clone(),
            test_output: summary.test_output.clone(),
        }
    }
}

/// Render an Explanation as plain text for CLI output.
pub fn render_explanation_plain(explanation: &Explanation) -> String {
    let mut lines = Vec::new();

    // Memory section
    lines.push("=== Memory ===".to_string());
    if explanation.memory.included.is_empty() {
        lines.push("  (no memory included)".to_string());
    } else {
        for entry in &explanation.memory.included {
            lines.push(format!(
                "  ✓ {} [confidence: {:.1}%, evidence: {}]",
                entry.claim,
                entry.confidence_bps as f64 / 100.0,
                entry.evidence_kind
            ));
        }
    }
    if !explanation.memory.excluded.is_empty() {
        lines.push(String::new());
        lines.push("  Excluded:".to_string());
        for entry in &explanation.memory.excluded {
            lines.push(format!("  ✗ {} — {}", entry.claim, entry.reason));
        }
    }

    // Policy section
    lines.push(String::new());
    lines.push("=== Policy ===".to_string());
    if explanation.policy.gates.is_empty() {
        lines.push("  (no policy gates triggered)".to_string());
    } else {
        for gate in &explanation.policy.gates {
            lines.push(format!(
                "  Gate: {} | risk={} | confirmation={} | {}",
                gate.tool_name, gate.risk, gate.confirmation, gate.decision
            ));
        }
    }
    if !explanation.policy.approvals.is_empty() {
        lines.push(String::new());
        for approval in &explanation.policy.approvals {
            let reason_str = approval
                .reason
                .as_ref()
                .map(|r| format!(" ({})", r))
                .unwrap_or_default();
            lines.push(format!(
                "  Approval: {} → {}{}",
                approval.tool_name, approval.decision, reason_str
            ));
        }
    }

    // Execution section
    lines.push(String::new());
    lines.push("=== Execution ===".to_string());
    if explanation.execution.tool_calls.is_empty() {
        lines.push("  (no tool calls)".to_string());
    } else {
        for call in &explanation.execution.tool_calls {
            let status = if call.success { "✓" } else { "✗" };
            let duration = call
                .duration_ms
                .map(|d| format!(" [{}ms]", d))
                .unwrap_or_default();
            lines.push(format!(
                "  {} {}{}: {}",
                status, call.tool_name, duration, call.output_preview
            ));
        }
    }

    // Completion section
    lines.push(String::new());
    lines.push("=== Completion ===".to_string());
    lines.push(format!(
        "  Status: {}",
        if explanation.completion.completed {
            "completed"
        } else {
            "incomplete"
        }
    ));
    if !explanation.completion.changed_files.is_empty() {
        lines.push(format!(
            "  Changed files: {}",
            explanation.completion.changed_files.join(", ")
        ));
    }
    if let Some(ref stat) = explanation.completion.diff_stat {
        lines.push(format!("  Diff: {}", stat));
    }
    if let Some(ref test) = explanation.completion.test_output {
        lines.push(format!("  Tests: {}", test));
    }

    lines.join("\n")
}

/// Build an ExecutionExplanation from trace data (simplified for now).
/// Full implementation would scan trace events from the store.
impl ExecutionExplanation {
    pub fn from_tool_results(
        results: &[(String, ToolCallId, bool, String, Option<u64>)],
    ) -> Self {
        ExecutionExplanation {
            tool_calls: results
                .iter()
                .map(|(name, call_id, success, output, duration)| ToolCallEntry {
                    tool_name: name.clone(),
                    call_id: call_id.to_string(),
                    success: *success,
                    output_preview: truncate_preview(output, 200),
                    duration_ms: *duration,
                })
                .collect(),
        }
    }
}

fn truncate_preview(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_plain_with_included_memory() {
        let explanation = Explanation {
            memory: MemoryExplanation {
                included: vec![ClaimEntry {
                    claim: "crate core exists".to_string(),
                    confidence_bps: 9500,
                    evidence_kind: "AcceptedClaim".to_string(),
                    source: "UserStated".to_string(),
                }],
                excluded: vec![],
            },
            policy: PolicyExplanation {
                gates: vec![],
                approvals: vec![],
            },
            execution: ExecutionExplanation {
                tool_calls: vec![],
            },
            completion: CompletionExplanation {
                completed: true,
                changed_files: vec![],
                diff_stat: None,
                test_output: None,
            },
        };
        let text = render_explanation_plain(&explanation);
        assert!(text.contains("crate core exists"));
        assert!(text.contains("95.0%"));
        assert!(text.contains("=== Memory ==="));
    }

    #[test]
    fn render_plain_with_excluded_memory() {
        let explanation = Explanation {
            memory: MemoryExplanation {
                included: vec![],
                excluded: vec![ExcludedClaimEntry {
                    claim: "low confidence fact".to_string(),
                    confidence_bps: 1500,
                    reason: "below prompt_include_min_bps threshold".to_string(),
                }],
            },
            policy: PolicyExplanation {
                gates: vec![],
                approvals: vec![],
            },
            execution: ExecutionExplanation {
                tool_calls: vec![],
            },
            completion: CompletionExplanation {
                completed: true,
                changed_files: vec![],
                diff_stat: None,
                test_output: None,
            },
        };
        let text = render_explanation_plain(&explanation);
        assert!(text.contains("✗ low confidence fact"));
        assert!(text.contains("below prompt_include_min_bps"));
    }

    #[test]
    fn render_plain_with_policy_gate() {
        let explanation = Explanation {
            memory: MemoryExplanation {
                included: vec![],
                excluded: vec![],
            },
            policy: PolicyExplanation {
                gates: vec![GateEntry {
                    tool_name: "local__file_patch".to_string(),
                    risk: "Medium".to_string(),
                    confirmation: "Approve".to_string(),
                    decision: "suspended for approval".to_string(),
                }],
                approvals: vec![ApprovalEntry {
                    tool_name: "local__file_patch".to_string(),
                    decision: "granted".to_string(),
                    reason: None,
                }],
            },
            execution: ExecutionExplanation {
                tool_calls: vec![],
            },
            completion: CompletionExplanation {
                completed: true,
                changed_files: vec![],
                diff_stat: None,
                test_output: None,
            },
        };
        let text = render_explanation_plain(&explanation);
        assert!(text.contains("local__file_patch"));
        assert!(text.contains("Medium"));
        assert!(text.contains("suspended for approval"));
        assert!(text.contains("granted"));
    }

    #[test]
    fn render_plain_with_tool_results() {
        let explanation = Explanation {
            memory: MemoryExplanation {
                included: vec![],
                excluded: vec![],
            },
            policy: PolicyExplanation {
                gates: vec![],
                approvals: vec![],
            },
            execution: ExecutionExplanation::from_tool_results(&[(
                "local__file_read".to_string(),
                ToolCallId::new(),
                true,
                "read 42 bytes".to_string(),
                Some(15),
            )]),
            completion: CompletionExplanation {
                completed: true,
                changed_files: vec![],
                diff_stat: None,
                test_output: None,
            },
        };
        let text = render_explanation_plain(&explanation);
        assert!(text.contains("✓ local__file_read"));
        assert!(text.contains("[15ms]"));
    }

    #[test]
    fn render_plain_with_completion_summary() {
        let explanation = Explanation {
            memory: MemoryExplanation {
                included: vec![],
                excluded: vec![],
            },
            policy: PolicyExplanation {
                gates: vec![],
                approvals: vec![],
            },
            execution: ExecutionExplanation {
                tool_calls: vec![],
            },
            completion: CompletionExplanation::from_task_summary(&TaskSummary {
                changed_files: vec!["src/lib.rs".to_string()],
                diff_stat: Some("1 file changed, 5 insertions(+)".to_string()),
                completed: true,
                test_output: Some("3 passed".to_string()),
            }),
        };
        let text = render_explanation_plain(&explanation);
        assert!(text.contains("src/lib.rs"));
        assert!(text.contains("1 file changed"));
        assert!(text.contains("3 passed"));
    }

    #[test]
    fn render_plain_incomplete_shows_no_test_as_uncertain() {
        let explanation = Explanation {
            memory: MemoryExplanation {
                included: vec![],
                excluded: vec![],
            },
            policy: PolicyExplanation {
                gates: vec![],
                approvals: vec![],
            },
            execution: ExecutionExplanation {
                tool_calls: vec![],
            },
            completion: CompletionExplanation {
                completed: false,
                changed_files: vec![],
                diff_stat: None,
                test_output: None,
            },
        };
        let text = render_explanation_plain(&explanation);
        assert!(text.contains("incomplete"));
    }

    #[test]
    fn truncate_preview_short() {
        assert_eq!("hello", truncate_preview("hello", 200));
    }

    #[test]
    fn truncate_preview_long() {
        let long = "x".repeat(300);
        let result = truncate_preview(&long, 200);
        assert_eq!(203, result.len()); // 200 + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn explanation_serializes_round_trip() {
        let explanation = Explanation {
            memory: MemoryExplanation {
                included: vec![ClaimEntry {
                    claim: "test".into(),
                    confidence_bps: 9000,
                    evidence_kind: "AcceptedClaim".into(),
                    source: "UserStated".into(),
                }],
                excluded: vec![],
            },
            policy: PolicyExplanation {
                gates: vec![],
                approvals: vec![],
            },
            execution: ExecutionExplanation {
                tool_calls: vec![],
            },
            completion: CompletionExplanation {
                completed: true,
                changed_files: vec![],
                diff_stat: None,
                test_output: None,
            },
        };
        let json = serde_json::to_string(&explanation).unwrap();
        let back: Explanation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.memory.included.len(), 1);
        assert_eq!(back.memory.included[0].claim, "test");
    }
}
