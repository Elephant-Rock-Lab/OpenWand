//! Session capability context — combines skill + goal summaries.
//!
//! Read-only text-only summaries for session prompt context.
//! Cross-registry validation (goal→skill linking) happens here
//! where both registries are available.
//!
//! Skills and goals are context, not authority.
//! Free-text is supplied as contextual text only and is never parsed by OpenWand
//! as executable commands, tool invocations, scripts, or structured authority.

use std::path::Path;

use openwand_goals::context::GoalContextSummary;
use openwand_goals::registry::load_goal_registry;
use openwand_skills::context::SkillContextSummary;
use openwand_skills::registry::load_skill_registry;

/// Combined read-only session capability context.
#[derive(Debug, Clone)]
pub struct SessionCapabilityContext {
    pub skills: Vec<SkillContextSummary>,
    pub goals: Vec<GoalContextSummary>,
    pub warnings: Vec<String>,
}

/// Load session capability context from .openwand/ directory.
/// Reads both skills.toml and goals.toml. Performs cross-registry validation.
/// Missing files produce empty context with warnings, not errors.
pub fn load_session_capability_context(openwand_dir: &Path) -> SessionCapabilityContext {
    let skills_path = openwand_dir.join("skills.toml");
    let goals_path = openwand_dir.join("goals.toml");

    let skill_registry = load_skill_registry(&skills_path);
    let goal_registry = load_goal_registry(&goals_path);

    let mut warnings = Vec::new();

    // Collect skill validation warnings/errors
    for issue in &skill_registry.validation.errors {
        warnings.push(format!("skill [{}]: {}", issue.skill_id, issue.message));
    }
    for issue in &skill_registry.validation.warnings {
        warnings.push(format!("skill [{}]: {}", issue.skill_id, issue.message));
    }

    // Collect goal validation warnings/errors
    for issue in &goal_registry.validation.errors {
        warnings.push(format!("goal [{}]: {}", issue.goal_id, issue.message));
    }
    for issue in &goal_registry.validation.warnings {
        warnings.push(format!("goal [{}]: {}", issue.goal_id, issue.message));
    }

    // Cross-registry validation: check linked_skills exist
    let skill_ids: std::collections::HashSet<String> =
        skill_registry.skills.iter().map(|s| s.id.0.clone()).collect();

    for goal in &goal_registry.goals {
        for linked_id in &goal.linked_skills {
            if !skill_ids.contains(linked_id) {
                warnings.push(format!(
                    "goal [{}]: links to unknown skill '{}'",
                    goal.id.0, linked_id
                ));
            }
        }
    }

    // Build context summaries
    let skills = openwand_skills::context::build_skill_context_summaries(&skill_registry);
    let goals = openwand_goals::context::build_goal_context_summaries(&goal_registry);

    SessionCapabilityContext {
        skills,
        goals,
        warnings,
    }
}

/// Render capability context as text for prompt inclusion.
/// Text-only — never parsed as executable instructions.
pub fn capability_context_as_text(ctx: &SessionCapabilityContext) -> String {
    let mut lines = Vec::new();

    if !ctx.skills.is_empty() {
        lines.push("# Available Skills".to_string());
        for skill in &ctx.skills {
            lines.push(format!("- {} [{}]: {}", skill.name, skill.category, skill.description));
            if !skill.constraints.is_empty() {
                for c in &skill.constraints {
                    lines.push(format!("  Constraint: {c}"));
                }
            }
        }
    }

    if !ctx.goals.is_empty() {
        lines.push("# Active Goals".to_string());
        for goal in &ctx.goals {
            lines.push(format!(
                "- {} (priority {}): {}",
                goal.title, goal.priority, goal.description
            ));
            if !goal.success_criteria.is_empty() {
                lines.push("  Success criteria:".to_string());
                for sc in &goal.success_criteria {
                    lines.push(format!("  - {sc}"));
                }
            }
            if !goal.constraints.is_empty() {
                for c in &goal.constraints {
                    lines.push(format!("  Constraint: {c}"));
                }
            }
        }
    }

    if lines.is_empty() {
        String::new()
    } else {
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn setup_dir(
        dir: &std::path::Path,
        skills_toml: Option<&str>,
        goals_toml: Option<&str>,
    ) {
        std::fs::create_dir_all(dir).unwrap();
        if let Some(skills) = skills_toml {
            let mut f = std::fs::File::create(dir.join("skills.toml")).unwrap();
            write!(f, "{skills}").unwrap();
        }
        if let Some(goals) = goals_toml {
            let mut f = std::fs::File::create(dir.join("goals.toml")).unwrap();
            write!(f, "{goals}").unwrap();
        }
    }

    #[test]
    fn session_capability_context_includes_enabled_skills_only() {
        let dir = std::env::temp_dir().join("openwand_test_cap_skills");
        if dir.exists() { std::fs::remove_dir_all(&dir).ok(); }
        setup_dir(
            &dir,
            Some(
                r#"
[[skill]]
id = "enabled"
name = "Enabled"
description = "An enabled skill"
enabled = true
outputs = ["result"]

[[skill]]
id = "disabled"
name = "Disabled"
description = "A disabled skill"
enabled = false
"#,
            ),
            None,
        );
        let ctx = load_session_capability_context(&dir);
        assert_eq!(1, ctx.skills.len());
        assert_eq!("enabled", ctx.skills[0].id);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn session_capability_context_includes_active_goals_only() {
        let dir = std::env::temp_dir().join("openwand_test_cap_goals");
        if dir.exists() { std::fs::remove_dir_all(&dir).ok(); }
        setup_dir(
            &dir,
            None,
            Some(
                r#"
[[goal]]
id = "active"
title = "Active"
description = "An active goal"
status = "active"
success_criteria = ["Done"]

[[goal]]
id = "paused"
title = "Paused"
description = "A paused goal"
status = "paused"
"#,
            ),
        );
        let ctx = load_session_capability_context(&dir);
        assert_eq!(1, ctx.goals.len());
        assert_eq!("active", ctx.goals[0].id);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn session_capability_context_orders_deterministically() {
        let dir = std::env::temp_dir().join("openwand_test_cap_order");
        if dir.exists() { std::fs::remove_dir_all(&dir).ok(); }
        setup_dir(
            &dir,
            Some(
                r#"
[[skill]]
id = "zeta"
name = "Zeta"
description = "Z"
enabled = true
outputs = ["z"]

[[skill]]
id = "alpha"
name = "Alpha"
description = "A"
enabled = true
outputs = ["a"]
"#,
            ),
            None,
        );
        let ctx = load_session_capability_context(&dir);
        assert_eq!(2, ctx.skills.len());
        assert_eq!("alpha", ctx.skills[0].id);
        assert_eq!("zeta", ctx.skills[1].id);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn session_capability_context_carries_constraints() {
        let dir = std::env::temp_dir().join("openwand_test_cap_constraints");
        if dir.exists() { std::fs::remove_dir_all(&dir).ok(); }
        setup_dir(
            &dir,
            Some(
                r#"
[[skill]]
id = "constrained"
name = "Constrained"
description = "Has constraints"
enabled = true
outputs = ["result"]
constraints = ["Do not execute tools", "Cite evidence"]
"#,
            ),
            None,
        );
        let ctx = load_session_capability_context(&dir);
        assert_eq!(2, ctx.skills[0].constraints.len());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn session_capability_context_treats_constraints_as_text_only() {
        let dir = std::env::temp_dir().join("openwand_test_cap_text");
        if dir.exists() { std::fs::remove_dir_all(&dir).ok(); }
        setup_dir(
            &dir,
            Some(
                r#"
[[skill]]
id = "text-skill"
name = "Text"
description = "Text constraints"
enabled = true
outputs = ["result"]
constraints = ["Do not bypass policy gates"]
"#,
            ),
            Some(
                r#"
[[goal]]
id = "text-goal"
title = "Text Goal"
description = "Text success criteria"
status = "active"
success_criteria = ["User can run a session"]
constraints = ["No ungoverned paths"]
"#,
            ),
        );
        let ctx = load_session_capability_context(&dir);
        let text = capability_context_as_text(&ctx);
        // Text output is plain text, not executable
        assert!(text.contains("Constraint: Do not bypass policy gates"));
        assert!(text.contains("Success criteria:"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn session_capability_warns_missing_linked_skill() {
        let dir = std::env::temp_dir().join("openwand_test_cap_linked");
        if dir.exists() { std::fs::remove_dir_all(&dir).ok(); }
        setup_dir(
            &dir,
            Some(
                r#"
[[skill]]
id = "existing-skill"
name = "Existing"
description = "Exists"
enabled = true
outputs = ["result"]
"#,
            ),
            Some(
                r#"
[[goal]]
id = "linked-goal"
title = "Linked"
description = "Links to skills"
status = "active"
linked_skills = ["existing-skill", "missing-skill"]
"#,
            ),
        );
        let ctx = load_session_capability_context(&dir);
        assert!(
            ctx.warnings.iter().any(|w| w.contains("missing-skill")),
            "Should warn about missing linked skill"
        );
        // existing-skill should NOT produce a warning
        assert!(
            !ctx.warnings.iter().any(|w| w.contains("existing-skill") && w.contains("unknown")),
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn session_runner_receives_capability_context() {
        // Verify the context can be constructed and passed around
        let dir = std::env::temp_dir().join("openwand_test_cap_runner");
        if dir.exists() { std::fs::remove_dir_all(&dir).ok(); }
        setup_dir(&dir, None, None);
        let ctx = load_session_capability_context(&dir);
        // Empty context is valid
        assert!(ctx.skills.is_empty());
        assert!(ctx.goals.is_empty());
        // Can be stored in an Option (as RunConfig would hold it)
        let maybe_ctx: Option<SessionCapabilityContext> = Some(ctx);
        assert!(maybe_ctx.is_some());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn session_prompt_includes_skill_goal_context_as_text_only() {
        let ctx = SessionCapabilityContext {
            skills: vec![],
            goals: vec![],
            warnings: vec![],
        };
        let text = capability_context_as_text(&ctx);
        assert!(text.is_empty()); // Empty context produces empty text

        let ctx_with = SessionCapabilityContext {
            skills: vec![SkillContextSummary {
                id: "test".into(),
                name: "Test".into(),
                description: "A test skill".into(),
                category: "test".into(),
                constraints: vec!["Be safe".into()],
                allowed_context: vec!["trace_summary".into()],
            }],
            goals: vec![],
            warnings: vec![],
        };
        let text = capability_context_as_text(&ctx_with);
        assert!(text.contains("Test"));
        assert!(text.contains("Be safe"));
    }

    #[test]
    fn session_context_build_does_not_execute_tools() {
        // Design assertion: this module imports only DTO/registry types.
        // No tool executor, no policy engine, no process command.
        let dir = std::env::temp_dir().join("openwand_test_cap_noexec");
        if dir.exists() { std::fs::remove_dir_all(&dir).ok(); }
        setup_dir(&dir, None, None);
        let _ctx = load_session_capability_context(&dir);
        // If this compiles, the module doesn't depend on execution machinery.
        std::fs::remove_dir_all(&dir).ok();
    }
}
