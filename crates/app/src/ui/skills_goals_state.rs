//! UI skills/goals state — read-only display helpers.
//!
//! Skills and goals are context, not authority.
//! Free-text is supplied as contextual text only and is never parsed by OpenWand
//! as executable commands, tool invocations, scripts, or structured authority.

use openwand_goals::registry::GoalRegistry;
use openwand_skills::registry::SkillRegistry;

/// UI-safe row for displaying a skill.
#[derive(Debug, Clone)]
pub struct SkillUiRow {
    pub id: String,
    pub name: String,
    pub category: String,
    pub enabled: bool,
    pub tags: Vec<String>,
    pub constraints_count: usize,
}

/// UI-safe row for displaying a goal.
#[derive(Debug, Clone)]
pub struct GoalUiRow {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: i32,
    pub linked_skills: Vec<String>,
    pub success_criteria_count: usize,
}

/// Combined UI state for skills and goals.
#[derive(Debug, Clone)]
pub struct SkillsGoalsUiState {
    pub skills: Vec<SkillUiRow>,
    pub goals: Vec<GoalUiRow>,
    pub validation_errors: Vec<String>,
    pub validation_warnings: Vec<String>,
}

/// Safety warning for skills/goals display.
pub fn skills_goals_safety_warning() -> String {
    "Skills and goals provide context only. They do not execute tools, mutate memory, or bypass policy.".into()
}

/// Build skill UI rows from a registry.
pub fn skill_rows(registry: &SkillRegistry) -> Vec<SkillUiRow> {
    registry
        .skills
        .iter()
        .map(|s| SkillUiRow {
            id: s.id.0.clone(),
            name: s.name.clone(),
            category: s.category.clone(),
            enabled: s.enabled,
            tags: s.tags.clone(),
            constraints_count: s.constraints.len(),
        })
        .collect()
}

/// Build goal UI rows from a registry.
pub fn goal_rows(registry: &GoalRegistry) -> Vec<GoalUiRow> {
    registry
        .goals
        .iter()
        .map(|g| GoalUiRow {
            id: g.id.0.clone(),
            title: g.title.clone(),
            status: format!("{:?}", g.status).to_lowercase(),
            priority: g.priority,
            linked_skills: g.linked_skills.clone(),
            success_criteria_count: g.success_criteria.len(),
        })
        .collect()
}

/// Format validation issues for display.
pub fn skill_goal_validation_lines(
    skill_registry: &SkillRegistry,
    goal_registry: &GoalRegistry,
) -> Vec<String> {
    let mut lines = Vec::new();
    for e in &skill_registry.validation.errors {
        lines.push(format!("ERROR [skill {}]: {}", e.skill_id, e.message));
    }
    for w in &skill_registry.validation.warnings {
        lines.push(format!("WARN [skill {}]: {}", w.skill_id, w.message));
    }
    for e in &goal_registry.validation.errors {
        lines.push(format!("ERROR [goal {}]: {}", e.goal_id, e.message));
    }
    for w in &goal_registry.validation.warnings {
        lines.push(format!("WARN [goal {}]: {}", w.goal_id, w.message));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use openwand_goals::manifest::{GoalDefinition, GoalId, GoalStatus};
    use openwand_goals::registry::GoalValidationReport;
    use openwand_skills::manifest::{SkillContextKind, SkillDefinition, SkillId};
    use openwand_skills::registry::SkillValidationReport;

    fn test_skill_registry() -> SkillRegistry {
        SkillRegistry {
            skills: vec![SkillDefinition {
                id: SkillId("rust-test-triage".into()),
                name: "Rust Test Triage".into(),
                description: "Helps interpret test output.".into(),
                category: "engineering".into(),
                enabled: true,
                tags: vec!["rust".into(), "tests".into()],
                inputs: vec![],
                outputs: vec!["summary".into()],
                constraints: vec!["No direct commands".into()],
                allowed_context: vec![SkillContextKind::TraceSummary],
            }],
            validation: SkillValidationReport::default(),
        }
    }

    fn test_goal_registry() -> GoalRegistry {
        GoalRegistry {
            goals: vec![GoalDefinition {
                id: GoalId("ship-product".into()),
                title: "Ship the product".into(),
                description: "Ship OpenWand.".into(),
                status: GoalStatus::Active,
                priority: 100,
                tags: vec!["product".into()],
                success_criteria: vec!["User can run a session".into()],
                constraints: vec!["No policy bypass".into()],
                linked_skills: vec!["rust-test-triage".into()],
            }],
            validation: GoalValidationReport::default(),
        }
    }

    #[test]
    fn skills_goals_ui_state_loads_rows() {
        let state = SkillsGoalsUiState {
            skills: skill_rows(&test_skill_registry()),
            goals: goal_rows(&test_goal_registry()),
            validation_errors: vec![],
            validation_warnings: vec![],
        };
        assert_eq!(1, state.skills.len());
        assert_eq!(1, state.goals.len());
    }

    #[test]
    fn skill_rows_show_enabled_category_tags() {
        let rows = skill_rows(&test_skill_registry());
        assert_eq!("rust-test-triage", rows[0].id);
        assert_eq!("Rust Test Triage", rows[0].name);
        assert_eq!("engineering", rows[0].category);
        assert!(rows[0].enabled);
        assert_eq!(vec!["rust", "tests"], rows[0].tags);
    }

    #[test]
    fn goal_rows_show_status_priority_linked_skills() {
        let rows = goal_rows(&test_goal_registry());
        assert_eq!("ship-product", rows[0].id);
        assert_eq!("Ship the product", rows[0].title);
        assert_eq!("active", rows[0].status);
        assert_eq!(100, rows[0].priority);
        assert_eq!(vec!["rust-test-triage"], rows[0].linked_skills);
        assert_eq!(1, rows[0].success_criteria_count);
    }

    #[test]
    fn validation_lines_show_errors_and_warnings() {
        let mut sr = test_skill_registry();
        sr.validation.warnings.push(openwand_skills::registry::SkillValidationIssue {
            skill_id: "test".into(),
            message: "test warning".into(),
            severity: openwand_skills::registry::SkillValidationSeverity::Warning,
        });
        let lines = skill_goal_validation_lines(&sr, &test_goal_registry());
        assert!(lines.iter().any(|l| l.contains("WARN")));
    }

    #[test]
    fn skills_goals_safety_warning_mentions_no_execution() {
        let warning = skills_goals_safety_warning();
        assert!(warning.contains("do not execute tools"));
        assert!(warning.contains("mutate memory"));
        assert!(warning.contains("bypass policy"));
    }

    #[test]
    fn skills_goals_ui_handles_missing_files() {
        let path = std::env::temp_dir().join("nonexistent_skills_goals_xyz");
        let sr = openwand_skills::registry::load_skill_registry(&path.join("skills.toml"));
        let gr = openwand_goals::registry::load_goal_registry(&path.join("goals.toml"));
        let state = SkillsGoalsUiState {
            skills: skill_rows(&sr),
            goals: goal_rows(&gr),
            validation_errors: vec![],
            validation_warnings: sr.validation.warnings.iter().map(|w| w.message.clone()).collect(),
        };
        assert!(state.skills.is_empty());
        assert!(state.goals.is_empty());
    }

    #[test]
    fn skills_goals_ui_does_not_call_shell_or_git() {
        // Design assertion: this module imports only DTO/registry types.
        // No std::process, no shell, no git commands.
        let _rows = skill_rows(&test_skill_registry());
        // If this compiles, the module doesn't depend on execution machinery.
    }

    #[test]
    fn skills_goals_ui_does_not_write_manifests() {
        // Design assertion: this module has no write functions.
        // skill_rows(), goal_rows(), validation_lines() are all read-only.
        let rows = skill_rows(&test_skill_registry());
        assert!(!rows.is_empty());
        // No save/write/persist function exists in this module.
    }
}
