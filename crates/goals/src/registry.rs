//! Goal registry — validated read-only collection of active goals.
//!
//! Loads from .openwand/goals.toml. Missing files produce empty registries
//! with warnings, not errors. Goals are context, not authority.
//!
//! Cross-registry validation (checking linked_skills exist) happens in
//! openwand-app, not here. This crate preserves linked_skill IDs as
//! unresolved strings.

use std::path::Path;

use crate::manifest::{GoalDefinition, GoalId, GoalManifest, GoalStatus};

/// Validated collection of goals.
#[derive(Debug, Clone)]
pub struct GoalRegistry {
    pub goals: Vec<GoalDefinition>,
    pub validation: GoalValidationReport,
}

/// Validation report for goal manifests.
#[derive(Debug, Clone, Default)]
pub struct GoalValidationReport {
    pub errors: Vec<GoalValidationIssue>,
    pub warnings: Vec<GoalValidationIssue>,
}

/// A single validation issue.
#[derive(Debug, Clone)]
pub struct GoalValidationIssue {
    pub goal_id: String,
    pub message: String,
    pub severity: GoalValidationSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoalValidationSeverity {
    Error,
    Warning,
}

/// Load goals from .openwand/goals.toml.
/// Returns empty registry with warning if file is missing.
pub fn load_goal_registry(path: &Path) -> GoalRegistry {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            return GoalRegistry {
                goals: Vec::new(),
                validation: GoalValidationReport {
                    errors: Vec::new(),
                    warnings: vec![GoalValidationIssue {
                        goal_id: "_global".into(),
                        message: format!(
                            "Goals manifest not found at '{}' — using empty registry",
                            path.display()
                        ),
                        severity: GoalValidationSeverity::Warning,
                    }],
                },
            };
        }
    };

    let manifest: GoalManifest = match toml::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            return GoalRegistry {
                goals: Vec::new(),
                validation: GoalValidationReport {
                    errors: vec![GoalValidationIssue {
                        goal_id: "_global".into(),
                        message: format!("Failed to parse goals manifest: {e}"),
                        severity: GoalValidationSeverity::Error,
                    }],
                    warnings: Vec::new(),
                },
            };
        }
    };

    validate_goal_manifest(manifest)
}

fn validate_goal_manifest(manifest: GoalManifest) -> GoalRegistry {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();
    let mut valid_goals = Vec::new();

    for goal in manifest.goals {
        let id_str = goal.id.0.as_str();

        // ID must be non-empty
        if id_str.trim().is_empty() {
            errors.push(GoalValidationIssue {
                goal_id: "<empty>".into(),
                message: "Goal ID must not be empty".into(),
                severity: GoalValidationSeverity::Error,
            });
            continue;
        }

        // ID must be unique
        if seen_ids.contains(id_str) {
            errors.push(GoalValidationIssue {
                goal_id: id_str.into(),
                message: format!("Duplicate goal ID: '{id_str}'"),
                severity: GoalValidationSeverity::Error,
            });
            continue;
        }
        seen_ids.insert(id_str.to_string());

        // Title must be non-empty
        if goal.title.trim().is_empty() {
            errors.push(GoalValidationIssue {
                goal_id: id_str.into(),
                message: "Goal title must not be empty".into(),
                severity: GoalValidationSeverity::Error,
            });
            continue;
        }

        // Active goals with no success criteria → warning
        if matches!(goal.status, GoalStatus::Active) && goal.success_criteria.is_empty() {
            warnings.push(GoalValidationIssue {
                goal_id: id_str.into(),
                message: "Active goal has no success criteria defined".into(),
                severity: GoalValidationSeverity::Warning,
            });
        }

        // linked_skills are preserved without resolution
        // Cross-registry validation happens in openwand-app

        valid_goals.push(goal);
    }

    // Sort by priority (descending) then by ID (ascending)
    valid_goals.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then_with(|| a.id.0.cmp(&b.id.0))
    });

    GoalRegistry {
        goals: valid_goals,
        validation: GoalValidationReport { errors, warnings },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_goals_toml(dir: &std::path::Path, content: &str) -> std::path::PathBuf {
        let path = dir.join("goals.toml");
        let mut file = std::fs::File::create(&path).unwrap();
        write!(file, "{content}").unwrap();
        path
    }

    #[test]
    fn goal_registry_rejects_duplicate_ids() {
        let dir = std::env::temp_dir().join("openwand_test_goals_dup");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_goals_toml(
            &dir,
            r#"
[[goal]]
id = "dup"
title = "First"
description = "First goal"

[[goal]]
id = "dup"
title = "Second"
description = "Second goal"
"#,
        );
        let registry = load_goal_registry(&path);
        assert!(registry.validation.errors.iter().any(|e| e.message.contains("Duplicate")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn goal_registry_rejects_empty_title() {
        let dir = std::env::temp_dir().join("openwand_test_goals_empty_title");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_goals_toml(
            &dir,
            r#"
[[goal]]
id = "no-title"
title = ""
description = "Has description"
"#,
        );
        let registry = load_goal_registry(&path);
        assert!(registry.validation.errors.iter().any(|e| e.message.contains("title")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn goal_registry_rejects_empty_id() {
        let dir = std::env::temp_dir().join("openwand_test_goals_empty_id");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_goals_toml(
            &dir,
            r#"
[[goal]]
id = ""
title = "No ID"
description = "Has description"
"#,
        );
        let registry = load_goal_registry(&path);
        assert!(registry.validation.errors.iter().any(|e| e.message.contains("ID")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn goal_registry_warns_active_goal_without_success_criteria() {
        let dir = std::env::temp_dir().join("openwand_test_goals_no_criteria");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_goals_toml(
            &dir,
            r#"
[[goal]]
id = "no-criteria"
title = "No Criteria"
description = "An active goal with no criteria"
status = "active"
"#,
        );
        let registry = load_goal_registry(&path);
        assert!(registry.validation.warnings.iter().any(|w| w.message.contains("no success criteria")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn goal_registry_preserves_linked_skill_ids_without_resolving() {
        let dir = std::env::temp_dir().join("openwand_test_goals_linked");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_goals_toml(
            &dir,
            r#"
[[goal]]
id = "linked-goal"
title = "Linked"
description = "Links to skills that may not exist"
linked_skills = ["nonexistent-skill", "another-missing"]
"#,
        );
        let registry = load_goal_registry(&path);
        // Registry accepts goals with linked skills without resolving them
        assert_eq!(1, registry.goals.len());
        assert_eq!(2, registry.goals[0].linked_skills.len());
        assert_eq!("nonexistent-skill", registry.goals[0].linked_skills[0]);
        // No warnings about missing skills — that's cross-registry validation in app crate
        assert!(registry.validation.errors.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn goal_registry_missing_file_returns_empty_with_warning() {
        let path = std::env::temp_dir().join("nonexistent_goals_dir_xyz/goals.toml");
        let registry = load_goal_registry(&path);
        assert!(registry.goals.is_empty());
        assert!(registry.validation.warnings.iter().any(|w| w.message.contains("not found")));
        assert!(registry.validation.errors.is_empty());
    }

    #[test]
    fn goal_registry_orders_goals_by_priority_then_id() {
        let dir = std::env::temp_dir().join("openwand_test_goals_order");
        std::fs::create_dir_all(&dir).unwrap();
        let path = write_goals_toml(
            &dir,
            r#"
[[goal]]
id = "low-priority"
title = "Low"
description = "Low priority"
priority = 10

[[goal]]
id = "high-priority"
title = "High"
description = "High priority"
priority = 100

[[goal]]
id = "medium-priority"
title = "Medium"
description = "Medium priority"
priority = 50
"#,
        );
        let registry = load_goal_registry(&path);
        assert_eq!(3, registry.goals.len());
        // Descending priority
        assert_eq!("high-priority", registry.goals[0].id.0);
        assert_eq!("medium-priority", registry.goals[1].id.0);
        assert_eq!("low-priority", registry.goals[2].id.0);
        std::fs::remove_dir_all(&dir).ok();
    }
}
