//! Tool intent resolution.
//!
//! Resolves WorkflowToolIntent.capability to descriptive capability categories.
//! Resolution never produces a tool call.

use crate::workflow_proposal::is_valid_capability_category;
use crate::workflow_readiness::{ToolIntentResolutionSnapshot, ToolIntentResolutionStatus};
#[cfg(test)]
use crate::workflow_proposal::FORBIDDEN_TOOL_NAMES;

/// Known descriptive capability categories.
pub const CAPABILITY_CATEGORIES: &[&str] = &[
    "context_observation",
    "context_gathering",
    "file_observation",
    "text_analysis",
    "code_analysis",
    "test_interpretation",
    "change_preparation",
    "change_application",
    "human_review",
    "review_presentation",
    "outcome_verification",
    "result_reporting",
    "text_search",
];

/// Resolve a single tool intent's capability to a category.
pub fn resolve_tool_intent(
    intent_id: &str,
    capability: &str,
) -> ToolIntentResolutionSnapshot {
    // Check for executable patterns first
    if let Err(e) = is_valid_capability_category(capability) {
        return ToolIntentResolutionSnapshot {
            intent_id: intent_id.into(),
            capability: capability.into(),
            resolution_status: ToolIntentResolutionStatus::RejectedExecutable,
            matched_capability_category: None,
            reason: e,
        };
    }

    // Check for JSON tool args pattern
    if capability.contains('{') || capability.contains('}') || capability.contains("tool_name") {
        return ToolIntentResolutionSnapshot {
            intent_id: intent_id.into(),
            capability: capability.into(),
            resolution_status: ToolIntentResolutionStatus::RejectedExecutable,
            matched_capability_category: None,
            reason: "capability contains JSON or tool_name pattern".into(),
        };
    }

    // Try to match against known categories
    let lower = capability.to_lowercase().replace('-', "_");
    let matches: Vec<&&str> = CAPABILITY_CATEGORIES
        .iter()
        .filter(|cat| lower.contains(**cat) || (**cat).contains(&lower))
        .collect();

    match matches.len() {
        0 => ToolIntentResolutionSnapshot {
            intent_id: intent_id.into(),
            capability: capability.into(),
            resolution_status: ToolIntentResolutionStatus::Unresolved,
            matched_capability_category: None,
            reason: format!("capability '{}' does not match any known category", capability),
        },
        1 => ToolIntentResolutionSnapshot {
            intent_id: intent_id.into(),
            capability: capability.into(),
            resolution_status: ToolIntentResolutionStatus::ResolvedCategory,
            matched_capability_category: Some((*matches[0]).to_string()),
            reason: format!("resolved to category '{}'", matches[0]),
        },
        _ => ToolIntentResolutionSnapshot {
            intent_id: intent_id.into(),
            capability: capability.into(),
            resolution_status: ToolIntentResolutionStatus::Ambiguous,
            matched_capability_category: None,
            reason: format!(
                "capability '{}' matches multiple categories: {}",
                capability,
                matches.iter().map(|m| **m).collect::<Vec<_>>().join(", ")
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_intent_resolves_known_capability_category() {
        let snap = resolve_tool_intent("i1", "context-observation");
        assert_eq!(ToolIntentResolutionStatus::ResolvedCategory, snap.resolution_status);
        assert!(snap.matched_capability_category.is_some());
    }

    #[test]
    fn tool_intent_unresolved_for_unknown_category() {
        let snap = resolve_tool_intent("i1", "quantum-computation");
        assert_eq!(ToolIntentResolutionStatus::Unresolved, snap.resolution_status);
    }

    #[test]
    fn tool_intent_rejects_registered_tool_name() {
        let snap = resolve_tool_intent("i1", "shell");
        assert_eq!(ToolIntentResolutionStatus::RejectedExecutable, snap.resolution_status);
    }

    #[test]
    fn tool_intent_rejects_shell_or_git_pattern() {
        let snap = resolve_tool_intent("i1", "git push");
        assert_eq!(ToolIntentResolutionStatus::RejectedExecutable, snap.resolution_status);
    }

    #[test]
    fn tool_intent_rejects_json_tool_args() {
        let snap = resolve_tool_intent("i1", "{\"tool_name\":\"shell\"}");
        assert_eq!(ToolIntentResolutionStatus::RejectedExecutable, snap.resolution_status);
    }

    #[test]
    fn tool_intent_resolution_never_outputs_tool_call() {
        let snap = resolve_tool_intent("i1", "context-observation");
        // Resolution output never contains tool_call, tool_name, tool_args, command
        assert!(!snap.reason.contains("tool_call"));
        assert!(!snap.reason.contains("tool_name"));
        assert!(!snap.matched_capability_category.as_ref().map_or(false, |c| {
            FORBIDDEN_TOOL_NAMES.contains(&c.as_str())
        }));
    }

    #[test]
    fn tool_intent_ambiguous_for_broad_category() {
        // "observation" is a substring of "context_observation" and "file_observation"
        let snap = resolve_tool_intent("i1", "observation");
        assert_eq!(ToolIntentResolutionStatus::Ambiguous, snap.resolution_status);
    }
}
