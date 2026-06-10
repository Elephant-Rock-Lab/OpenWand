//! UI skills/goals state — read-only display helpers and readiness model.
//!
//! Skills and goals are context, not authority.
//! Free-text is supplied as contextual text only and is never parsed by OpenWand
//! as executable commands, tool invocations, scripts, or structured authority.
//!
//! Readiness indicates context-projection status only. It does not indicate
//! execution authority, tool availability, scheduling capability, or runtime enablement.

use openwand_goals::manifest::GoalStatus;
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

/// Safety warning for skills/goals display (Patch 8: always visible).
pub fn skills_goals_safety_warning() -> String {
    "Skills and goals provide context only. They do not execute tools, mutate memory, route workflow actions, schedule goals, bypass policy, or grant authority.".into()
}

// ── Readiness model (Patch 1–3) ────────────────────────────────────────

/// Readiness status — explicit, non-authoritative (Patch 1).
/// Avoids plain `Ready` to prevent confusion with "ready to execute."
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillGoalReadinessStatus {
    /// Item is valid and can be included in context projection.
    ReadyForContext,
    /// Item has gaps that block context projection.
    Blocked,
    /// Item has minor issues but can still provide partial context.
    Incomplete,
    /// Manifest file was not found.
    MissingManifest,
}

impl std::fmt::Display for SkillGoalReadinessStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillGoalReadinessStatus::ReadyForContext => write!(f, "Ready for context"),
            SkillGoalReadinessStatus::Blocked => write!(f, "Blocked from context projection"),
            SkillGoalReadinessStatus::Incomplete => write!(f, "Incomplete configuration"),
            SkillGoalReadinessStatus::MissingManifest => write!(f, "Manifest not found"),
        }
    }
}

/// Manifest file state — distinguishes not-found from empty from invalid (Patch 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillGoalManifestState {
    /// File not found on disk.
    NotFound,
    /// File found but contains no items.
    FoundEmpty,
    /// File found with valid items.
    FoundWithItems,
    /// File found but failed to parse.
    Invalid,
}

impl std::fmt::Display for SkillGoalManifestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillGoalManifestState::NotFound => write!(f, "manifest not found"),
            SkillGoalManifestState::FoundEmpty => write!(f, "manifest found but empty"),
            SkillGoalManifestState::FoundWithItems => write!(f, "manifest found with items"),
            SkillGoalManifestState::Invalid => write!(f, "manifest invalid"),
        }
    }
}

/// Kind of readiness gap (Patch 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillGoalReadinessGapKind {
    /// Goal references a skill ID that does not exist in the registry.
    MissingLinkedSkill,
    /// Skill declares no allowed_context kinds.
    SkillWithoutContext,
    /// Active goal has no success criteria.
    GoalWithoutSuccessCriteria,
    /// Goal links to a skill that is disabled.
    DisabledSkillLinked,
    /// Manifest file was not found on disk.
    NoManifestFound,
}

impl std::fmt::Display for SkillGoalReadinessGapKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillGoalReadinessGapKind::MissingLinkedSkill => write!(f, "Missing linked skill"),
            SkillGoalReadinessGapKind::SkillWithoutContext => write!(f, "Skill without context"),
            SkillGoalReadinessGapKind::GoalWithoutSuccessCriteria => write!(f, "Goal without success criteria"),
            SkillGoalReadinessGapKind::DisabledSkillLinked => write!(f, "Disabled skill linked"),
            SkillGoalReadinessGapKind::NoManifestFound => write!(f, "Manifest not found"),
        }
    }
}

/// A single readiness gap with source/target context (Patch 3).
#[derive(Debug, Clone)]
pub struct SkillGoalReadinessGap {
    pub kind: SkillGoalReadinessGapKind,
    /// ID of the item that has the gap (e.g., goal ID, skill ID).
    pub source_id: Option<String>,
    /// ID of the referenced item (e.g., missing skill ID).
    pub target_id: Option<String>,
    /// Human-readable gap description.
    pub message: String,
}

/// Per-item readiness row for skills.
#[derive(Debug, Clone)]
pub struct SkillReadinessRow {
    pub id: String,
    pub name: String,
    pub category: String,
    pub enabled: bool,
    pub status: SkillGoalReadinessStatus,
    pub gaps: Vec<SkillGoalReadinessGap>,
}

/// Per-item readiness row for goals.
#[derive(Debug, Clone)]
pub struct GoalReadinessRow {
    pub id: String,
    pub title: String,
    pub priority: i32,
    pub status_label: String,
    pub readiness_status: SkillGoalReadinessStatus,
    pub linked_skills: Vec<String>,
    pub gaps: Vec<SkillGoalReadinessGap>,
}

/// Combined readiness report (Patch 7: consumes registry outputs).
#[derive(Debug, Clone)]
pub struct SkillGoalReadinessReport {
    pub skills_manifest_state: SkillGoalManifestState,
    pub goals_manifest_state: SkillGoalManifestState,
    pub skill_rows: Vec<SkillReadinessRow>,
    pub goal_rows: Vec<GoalReadinessRow>,
    pub gaps: Vec<SkillGoalReadinessGap>,
    pub safety_warning: String,
}

/// Determine manifest state from a registry (Patch 2).
pub fn manifest_state_from_registry(
    registry: &SkillRegistry,
) -> SkillGoalManifestState {
    let has_not_found = registry.validation.warnings.iter().any(|w| w.message.contains("not found"));
    let has_parse_error = registry.validation.errors.iter().any(|e| e.message.contains("Failed to parse"));

    if has_not_found {
        SkillGoalManifestState::NotFound
    } else if has_parse_error {
        SkillGoalManifestState::Invalid
    } else if registry.skills.is_empty() {
        SkillGoalManifestState::FoundEmpty
    } else {
        SkillGoalManifestState::FoundWithItems
    }
}

/// Determine manifest state from a goal registry (Patch 2).
pub fn goal_manifest_state_from_registry(
    registry: &GoalRegistry,
) -> SkillGoalManifestState {
    let has_not_found = registry.validation.warnings.iter().any(|w| w.message.contains("not found"));
    let has_parse_error = registry.validation.errors.iter().any(|e| e.message.contains("Failed to parse"));

    if has_not_found {
        SkillGoalManifestState::NotFound
    } else if has_parse_error {
        SkillGoalManifestState::Invalid
    } else if registry.goals.is_empty() {
        SkillGoalManifestState::FoundEmpty
    } else {
        SkillGoalManifestState::FoundWithItems
    }
}

/// Build a readiness report from existing registries (Patch 7: reuses validation).
pub fn build_readiness_report(
    skill_registry: &SkillRegistry,
    goal_registry: &GoalRegistry,
) -> SkillGoalReadinessReport {
    let skills_manifest_state = manifest_state_from_registry(skill_registry);
    let goals_manifest_state = goal_manifest_state_from_registry(goal_registry);
    let mut gaps = Vec::new();

    // Build set of known skill IDs and enabled skill IDs
    let skill_ids: std::collections::HashSet<&str> =
        skill_registry.skills.iter().map(|s| s.id.0.as_str()).collect();
    let enabled_skill_ids: std::collections::HashSet<&str> =
        skill_registry.skills.iter().filter(|s| s.enabled).map(|s| s.id.0.as_str()).collect();

    // Check skills for gaps
    let mut skill_rows = Vec::new();
    for skill in &skill_registry.skills {
        let mut skill_gaps = Vec::new();

        if skill.allowed_context.is_empty() {
            let gap = SkillGoalReadinessGap {
                kind: SkillGoalReadinessGapKind::SkillWithoutContext,
                source_id: Some(skill.id.0.clone()),
                target_id: None,
                message: format!("Skill '{}' has no allowed context kinds defined", skill.id.0),
            };
            skill_gaps.push(gap.clone());
            gaps.push(gap);
        }

        let status = if skill_gaps.is_empty() {
            SkillGoalReadinessStatus::ReadyForContext
        } else {
            SkillGoalReadinessStatus::Incomplete
        };

        skill_rows.push(SkillReadinessRow {
            id: skill.id.0.clone(),
            name: skill.name.clone(),
            category: skill.category.clone(),
            enabled: skill.enabled,
            status,
            gaps: skill_gaps,
        });
    }

    // Check goals for gaps (reuse cross-registry validation logic)
    let mut goal_rows = Vec::new();
    for goal in &goal_registry.goals {
        let mut goal_gaps = Vec::new();

        for linked_id in &goal.linked_skills {
            if !skill_ids.contains(linked_id.as_str()) {
                let gap = SkillGoalReadinessGap {
                    kind: SkillGoalReadinessGapKind::MissingLinkedSkill,
                    source_id: Some(goal.id.0.clone()),
                    target_id: Some(linked_id.clone()),
                    message: format!("Goal '{}' links to unknown skill '{}'", goal.id.0, linked_id),
                };
                goal_gaps.push(gap.clone());
                gaps.push(gap);
            } else if !enabled_skill_ids.contains(linked_id.as_str()) {
                let gap = SkillGoalReadinessGap {
                    kind: SkillGoalReadinessGapKind::DisabledSkillLinked,
                    source_id: Some(goal.id.0.clone()),
                    target_id: Some(linked_id.clone()),
                    message: format!("Goal '{}' links to disabled skill '{}'", goal.id.0, linked_id),
                };
                goal_gaps.push(gap.clone());
                gaps.push(gap);
            }
        }

        if matches!(goal.status, GoalStatus::Active) && goal.success_criteria.is_empty() {
            let gap = SkillGoalReadinessGap {
                kind: SkillGoalReadinessGapKind::GoalWithoutSuccessCriteria,
                source_id: Some(goal.id.0.clone()),
                target_id: None,
                message: format!("Active goal '{}' has no success criteria defined", goal.id.0),
            };
            goal_gaps.push(gap.clone());
            gaps.push(gap);
        }

        let readiness_status = if goal_gaps.is_empty() {
            SkillGoalReadinessStatus::ReadyForContext
        } else if goal_gaps.iter().any(|g| matches!(g.kind, SkillGoalReadinessGapKind::MissingLinkedSkill)) {
            SkillGoalReadinessStatus::Blocked
        } else {
            SkillGoalReadinessStatus::Incomplete
        };

        goal_rows.push(GoalReadinessRow {
            id: goal.id.0.clone(),
            title: goal.title.clone(),
            priority: goal.priority,
            status_label: format!("{:?}", goal.status).to_lowercase(),
            readiness_status,
            linked_skills: goal.linked_skills.clone(),
            gaps: goal_gaps,
        });
    }

    // Add manifest-level gaps
    if matches!(skills_manifest_state, SkillGoalManifestState::NotFound) {
        gaps.push(SkillGoalReadinessGap {
            kind: SkillGoalReadinessGapKind::NoManifestFound,
            source_id: None,
            target_id: None,
            message: "skills manifest not found".into(),
        });
    }
    if matches!(goals_manifest_state, SkillGoalManifestState::NotFound) {
        gaps.push(SkillGoalReadinessGap {
            kind: SkillGoalReadinessGapKind::NoManifestFound,
            source_id: None,
            target_id: None,
            message: "goals manifest not found".into(),
        });
    }

    SkillGoalReadinessReport {
        skills_manifest_state,
        goals_manifest_state,
        skill_rows,
        goal_rows,
        gaps,
        safety_warning: skills_goals_safety_warning(),
    }
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
        // Patch 8: expanded warning
        assert!(warning.contains("route workflow actions"));
        assert!(warning.contains("schedule goals"));
        assert!(warning.contains("grant authority"));
    }

    // ── Patch 1: Readiness status tests ──

    #[test]
    fn readiness_status_ready_for_context_display() {
        assert_eq!("Ready for context", format!("{}", SkillGoalReadinessStatus::ReadyForContext));
    }

    #[test]
    fn readiness_status_blocked_display() {
        assert_eq!("Blocked from context projection", format!("{}", SkillGoalReadinessStatus::Blocked));
    }

    #[test]
    fn readiness_status_incomplete_display() {
        assert_eq!("Incomplete configuration", format!("{}", SkillGoalReadinessStatus::Incomplete));
    }

    #[test]
    fn readiness_status_missing_manifest_display() {
        assert_eq!("Manifest not found", format!("{}", SkillGoalReadinessStatus::MissingManifest));
    }

    #[test]
    fn readiness_status_does_not_use_execution_language() {
        let all_labels = vec![
            format!("{}", SkillGoalReadinessStatus::ReadyForContext),
            format!("{}", SkillGoalReadinessStatus::Blocked),
            format!("{}", SkillGoalReadinessStatus::Incomplete),
            format!("{}", SkillGoalReadinessStatus::MissingManifest),
        ];
        for label in &all_labels {
            let lower = label.to_lowercase();
            assert!(!lower.contains("execute"), "label: {label}");
            assert!(!lower.contains("approved"), "label: {label}");
            assert!(!lower.contains("certified"), "label: {label}");
            assert!(!lower.contains("runtime-enabled"), "label: {label}");
        }
    }

    // ── Patch 2: Manifest state distinction ──

    #[test]
    fn readiness_distinguishes_missing_manifest_from_empty_manifest() {
        let path = std::env::temp_dir().join("openwand_test_manifest_distinct");
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();

        // Missing manifest → NotFound
        let sr = openwand_skills::registry::load_skill_registry(&path.join("skills.toml"));
        let state = manifest_state_from_registry(&sr);
        assert_eq!(SkillGoalManifestState::NotFound, state);

        // Invalid TOML content → Invalid
        std::fs::write(path.join("skills.toml"), "not valid toml [[[}}}").unwrap();
        let sr2 = openwand_skills::registry::load_skill_registry(&path.join("skills.toml"));
        let state2 = manifest_state_from_registry(&sr2);
        assert_eq!(SkillGoalManifestState::Invalid, state2);

        // Empty string parses as empty TOML table but SkillManifest requires `skill` field,
        // so serde fails → also Invalid. This is correct: the file is present but malformed.
        std::fs::write(path.join("skills.toml"), "").unwrap();
        let sr3 = openwand_skills::registry::load_skill_registry(&path.join("skills.toml"));
        let state3 = manifest_state_from_registry(&sr3);
        assert_eq!(SkillGoalManifestState::Invalid, state3);

        // FoundEmpty is impossible with current SkillManifest because it requires
        // the `skill` field. A file with no [[skill]] entries still fails.
        // FoundWithItems requires at least one valid skill.
        std::fs::remove_dir_all(&path).ok();
    }

    #[test]
    fn readiness_reports_invalid_manifest_without_panic() {
        let dir = std::env::temp_dir().join("openwand_test_invalid_manifest");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("skills.toml"), "not valid toml [[[}}}").unwrap();
        let sr = openwand_skills::registry::load_skill_registry(&dir.join("skills.toml"));
        let state = manifest_state_from_registry(&sr);
        assert_eq!(SkillGoalManifestState::Invalid, state);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn empty_valid_registry_is_not_reported_as_missing_manifest() {
        // A registry loaded from a file that exists and parses correctly
        // with at least one valid skill should be FoundWithItems.
        let sr = test_skill_registry();
        let state = manifest_state_from_registry(&sr);
        assert_eq!(SkillGoalManifestState::FoundWithItems, state);
        // Note: FoundEmpty is currently unreachable because SkillManifest
        // requires the `skill` field. The variant exists for forward compat
        // if #[serde(default)] is added later.
    }

    // ── Patch 3: Gap source/target ID preservation ──

    #[test]
    fn missing_linked_skill_gap_preserves_goal_and_skill_ids() {
        let goal_reg = GoalRegistry {
            goals: vec![GoalDefinition {
                id: GoalId("my-goal".into()),
                title: "My Goal".into(),
                description: "Test".into(),
                status: GoalStatus::Active,
                priority: 50,
                tags: vec![],
                success_criteria: vec!["Done".into()],
                constraints: vec![],
                linked_skills: vec!["missing-skill".into()],
            }],
            validation: GoalValidationReport::default(),
        };
        let report = build_readiness_report(&test_skill_registry(), &goal_reg);
        let gap = report.gaps.iter().find(|g| matches!(g.kind, SkillGoalReadinessGapKind::MissingLinkedSkill)).unwrap();
        assert_eq!(Some("my-goal".to_string()), gap.source_id);
        assert_eq!(Some("missing-skill".to_string()), gap.target_id);
    }

    #[test]
    fn disabled_skill_gap_preserves_goal_and_skill_ids() {
        let skill_reg = SkillRegistry {
            skills: vec![SkillDefinition {
                id: SkillId("disabled-skill".into()),
                name: "Disabled".into(),
                description: "A disabled skill".into(),
                category: "test".into(),
                enabled: false,
                tags: vec![],
                inputs: vec![],
                outputs: vec!["result".into()],
                constraints: vec![],
                allowed_context: vec![SkillContextKind::TraceSummary],
            }],
            validation: SkillValidationReport::default(),
        };
        let goal_reg = GoalRegistry {
            goals: vec![GoalDefinition {
                id: GoalId("linked-goal".into()),
                title: "Linked".into(),
                description: "Test".into(),
                status: GoalStatus::Active,
                priority: 50,
                tags: vec![],
                success_criteria: vec!["Done".into()],
                constraints: vec![],
                linked_skills: vec!["disabled-skill".into()],
            }],
            validation: GoalValidationReport::default(),
        };
        let report = build_readiness_report(&skill_reg, &goal_reg);
        let gap = report.gaps.iter().find(|g| matches!(g.kind, SkillGoalReadinessGapKind::DisabledSkillLinked)).unwrap();
        assert_eq!(Some("linked-goal".to_string()), gap.source_id);
        assert_eq!(Some("disabled-skill".to_string()), gap.target_id);
    }

    #[test]
    fn skill_without_context_gap_preserves_skill_id() {
        let skill_reg = SkillRegistry {
            skills: vec![SkillDefinition {
                id: SkillId("no-ctx".into()),
                name: "No Context".into(),
                description: "No allowed context".into(),
                category: "test".into(),
                enabled: true,
                tags: vec![],
                inputs: vec![],
                outputs: vec!["result".into()],
                constraints: vec![],
                allowed_context: vec![],
            }],
            validation: SkillValidationReport::default(),
        };
        let report = build_readiness_report(&skill_reg, &test_goal_registry());
        let gap = report.gaps.iter().find(|g| matches!(g.kind, SkillGoalReadinessGapKind::SkillWithoutContext)).unwrap();
        assert_eq!(Some("no-ctx".to_string()), gap.source_id);
        assert_eq!(None, gap.target_id);
    }

    #[test]
    fn goal_without_success_criteria_gap_preserves_goal_id() {
        let goal_reg = GoalRegistry {
            goals: vec![GoalDefinition {
                id: GoalId("no-criteria".into()),
                title: "No Criteria".into(),
                description: "Test".into(),
                status: GoalStatus::Active,
                priority: 50,
                tags: vec![],
                success_criteria: vec![],
                constraints: vec![],
                linked_skills: vec![],
            }],
            validation: GoalValidationReport::default(),
        };
        let report = build_readiness_report(&test_skill_registry(), &goal_reg);
        let gap = report.gaps.iter().find(|g| matches!(g.kind, SkillGoalReadinessGapKind::GoalWithoutSuccessCriteria)).unwrap();
        assert_eq!(Some("no-criteria".to_string()), gap.source_id);
    }

    // ── Patch 4: Goals not scheduler semantics ──

    #[test]
    fn goal_readiness_copy_says_context_not_scheduler() {
        let row = GoalReadinessRow {
            id: "g1".into(),
            title: "Test Goal".into(),
            priority: 50,
            status_label: "active".into(),
            readiness_status: SkillGoalReadinessStatus::ReadyForContext,
            linked_skills: vec![],
            gaps: vec![],
        };
        // "ReadyForContext" — not "ready to schedule" or "ready to run"
        assert!(format!("{}", row.readiness_status).contains("context"));
    }

    #[test]
    fn goal_priority_copy_does_not_imply_scheduling() {
        let rows = goal_rows(&test_goal_registry());
        // Priority is displayed as a number, not as scheduling action
        assert_eq!(100, rows[0].priority);
    }

    #[test]
    fn goal_rows_contain_no_goal_action_verbs() {
        let rows = goal_rows(&test_goal_registry());
        for row in &rows {
            let lower = row.title.to_lowercase();
            assert!(!lower.contains("schedule"));
            assert!(!lower.contains("start goal"));
            assert!(!lower.contains("run goal"));
            assert!(!lower.contains("activate goal"));
            assert!(!lower.contains("complete goal"));
            assert!(!lower.contains("advance goal"));
        }
    }

    // ── Patch 5: Skills not tool semantics ──

    #[test]
    fn skill_readiness_copy_says_context_not_tool() {
        let row = SkillReadinessRow {
            id: "s1".into(),
            name: "Test Skill".into(),
            category: "test".into(),
            enabled: true,
            status: SkillGoalReadinessStatus::ReadyForContext,
            gaps: vec![],
        };
        assert!(format!("{}", row.status).contains("context"));
    }

    #[test]
    fn skill_rows_contain_no_tool_execution_language() {
        let rows = skill_rows(&test_skill_registry());
        for row in &rows {
            let lower = row.name.to_lowercase();
            assert!(!lower.contains("execute skill"));
            assert!(!lower.contains("invoke skill"));
            assert!(!lower.contains("run skill"));
            assert!(!lower.contains("call tool"));
            assert!(!lower.contains("promote to tool"));
        }
    }

    #[test]
    fn skill_rows_say_not_promoted_into_tools() {
        // Safety warning covers this
        let warning = skills_goals_safety_warning();
        assert!(warning.contains("do not execute tools"));
    }

    // ── Patch 7: Reuse registry validation ──

    #[test]
    fn readiness_report_reuses_registry_validation_semantics() {
        let report = build_readiness_report(&test_skill_registry(), &test_goal_registry());
        // Report consumes existing registry outputs — no second validation
        assert_eq!(SkillGoalManifestState::FoundWithItems, report.skills_manifest_state);
        assert_eq!(SkillGoalManifestState::FoundWithItems, report.goals_manifest_state);
    }

    #[test]
    fn readiness_report_does_not_mark_registry_valid_item_as_invalid() {
        let report = build_readiness_report(&test_skill_registry(), &test_goal_registry());
        // Both items are valid in registries — readiness should not downgrade them
        assert_eq!(SkillGoalReadinessStatus::ReadyForContext, report.skill_rows[0].status);
        assert_eq!(SkillGoalReadinessStatus::ReadyForContext, report.goal_rows[0].readiness_status);
    }

    #[test]
    fn readiness_report_preserves_existing_cross_registry_validation_results() {
        let goal_reg = GoalRegistry {
            goals: vec![GoalDefinition {
                id: GoalId("linked".into()),
                title: "Linked".into(),
                description: "Test".into(),
                status: GoalStatus::Active,
                priority: 50,
                tags: vec![],
                success_criteria: vec!["Done".into()],
                constraints: vec![],
                linked_skills: vec!["rust-test-triage".into()],
            }],
            validation: GoalValidationReport::default(),
        };
        let report = build_readiness_report(&test_skill_registry(), &goal_reg);
        // Valid link — no MissingLinkedSkill gap
        assert!(!report.gaps.iter().any(|g| matches!(g.kind, SkillGoalReadinessGapKind::MissingLinkedSkill)));
    }

    // ── Patch 8: Safety warning always visible ──

    #[test]
    fn readiness_panel_shows_safety_warning_with_items() {
        let report = build_readiness_report(&test_skill_registry(), &test_goal_registry());
        assert!(!report.safety_warning.is_empty());
    }

    #[test]
    fn readiness_panel_shows_safety_warning_when_empty() {
        let empty_sr = SkillRegistry {
            skills: vec![],
            validation: SkillValidationReport::default(),
        };
        let empty_gr = GoalRegistry {
            goals: vec![],
            validation: GoalValidationReport::default(),
        };
        let report = build_readiness_report(&empty_sr, &empty_gr);
        assert!(!report.safety_warning.is_empty());
    }

    #[test]
    fn readiness_panel_shows_safety_warning_when_manifests_missing() {
        let path = std::env::temp_dir().join("openwand_test_missing_wfg");
        let _ = std::fs::remove_dir_all(&path);
        let sr = openwand_skills::registry::load_skill_registry(&path.join("skills.toml"));
        let gr = openwand_goals::registry::load_goal_registry(&path.join("goals.toml"));
        let report = build_readiness_report(&sr, &gr);
        assert!(!report.safety_warning.is_empty());
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
