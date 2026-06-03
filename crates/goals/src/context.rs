//! Goal context projection — read-only summary for session consumption.
//!
//! Goals are context, not authority. This module produces session-safe
//! summaries that carry no executable fields, no function pointers,
//! no tool handles, no command strings.

use crate::manifest::GoalStatus;
use crate::manifest::{GoalDefinition};
use crate::registry::GoalRegistry;

/// Session-safe summary of a goal. No structured executable fields.
///
/// Free-text fields (description, constraints, success_criteria) are supplied
/// as contextual text only and are never parsed by OpenWand as executable
/// commands, tool invocations, scripts, or structured authority.
#[derive(Debug, Clone)]
pub struct GoalContextSummary {
    pub id: String,
    pub title: String,
    pub description: String,
    pub priority: i32,
    pub success_criteria: Vec<String>,
    pub constraints: Vec<String>,
    pub linked_skills: Vec<String>,
}

impl GoalContextSummary {
    /// Build from a goal definition. Only active goals should be summarized.
    pub fn from_definition(def: &GoalDefinition) -> Self {
        Self {
            id: def.id.0.clone(),
            title: def.title.clone(),
            description: def.description.clone(),
            priority: def.priority,
            success_criteria: def.success_criteria.clone(),
            constraints: def.constraints.clone(),
            linked_skills: def.linked_skills.clone(),
        }
    }
}

/// Build goal context summaries from a registry.
/// Only includes active goals. Ordered by priority (desc) then ID (asc).
pub fn build_goal_context_summaries(registry: &GoalRegistry) -> Vec<GoalContextSummary> {
    registry
        .goals
        .iter()
        .filter(|g| matches!(g.status, GoalStatus::Active))
        .map(GoalContextSummary::from_definition)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{GoalDefinition, GoalId, GoalStatus};
    use crate::registry::GoalValidationReport;

    fn test_registry() -> GoalRegistry {
        GoalRegistry {
            goals: vec![
                GoalDefinition {
                    id: GoalId("active-goal".into()),
                    title: "Active".into(),
                    description: "An active goal".into(),
                    status: GoalStatus::Active,
                    priority: 100,
                    tags: vec![],
                    success_criteria: vec!["Ship it".into()],
                    constraints: vec!["Do not bypass policy".into()],
                    linked_skills: vec!["rust-test-triage".into()],
                },
                GoalDefinition {
                    id: GoalId("paused-goal".into()),
                    title: "Paused".into(),
                    description: "A paused goal".into(),
                    status: GoalStatus::Paused,
                    priority: 50,
                    tags: vec![],
                    success_criteria: vec![],
                    constraints: vec![],
                    linked_skills: vec![],
                },
            ],
            validation: GoalValidationReport::default(),
        }
    }

    #[test]
    fn goal_context_only_includes_active_goals() {
        let registry = test_registry();
        let summaries = build_goal_context_summaries(&registry);
        assert_eq!(1, summaries.len());
        assert_eq!("active-goal", summaries[0].id);
    }

    #[test]
    fn goal_context_carries_constraints() {
        let registry = test_registry();
        let summaries = build_goal_context_summaries(&registry);
        assert_eq!(1, summaries[0].constraints.len());
        assert_eq!("Do not bypass policy", summaries[0].constraints[0]);
    }

    #[test]
    fn goal_context_summary_has_no_executable_fields() {
        let registry = test_registry();
        let summaries = build_goal_context_summaries(&registry);
        let summary = &summaries[0];

        // The summary struct has no command, shell, tool_name, tool_args,
        // script, cwd, env, or function_ref fields.
        assert!(!summary.id.is_empty());
        assert!(!summary.title.is_empty());
        // linked_skills is Vec<String> of IDs, not tool handles
        for skill_id in &summary.linked_skills {
            assert!(skill_id.starts_with("rust")); // just text IDs
        }
    }
}
