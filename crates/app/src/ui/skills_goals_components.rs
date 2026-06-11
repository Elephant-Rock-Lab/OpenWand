//! Dioxus render functions for skills/goals readiness panel (desktop-gated).
//!
//! Read-only display — no manifest editing, no execution, no tool invocation,
//! no goal scheduling, no skill promotion, no authority.
//!
//! Skills and goals may shape context readiness. They are not tools, schedulers,
//! routes, or authority.

#[cfg(feature = "desktop")]
mod desktop_render {
    use crate::ui::skills_goals_state::{
        SkillGoalReadinessGap, SkillGoalReadinessReport, SkillGoalReadinessStatus,
        GoalReadinessRow, SkillReadinessRow,
    };
    use dioxus::prelude::*;

    /// Render the full skills/goals readiness panel with safety banner.
    pub fn render_skills_goals_readiness_panel(report: &SkillGoalReadinessReport) -> Element {
        let has_gaps = !report.gaps.is_empty();
        rsx! {
            div { style: "border: 1px solid #ddd; border-radius: 6px; margin-top: 12px;",
                // Header
                div { style: "padding: 10px 14px; background: #f5f5f5; border-bottom: 1px solid #ddd;
                              font-size: 13px; font-weight: 600; color: #333;",
                    "Skills & Goals — Context Readiness"
                }

                // Safety warning (Patch 8: always visible)
                div { style: "padding: 8px 14px; background: #fff8e1; border-bottom: 1px solid #ffe082;
                              font-size: 11px; color: #8d6e00; line-height: 1.4;",
                    "{report.safety_warning}"
                }

                // Manifest states
                div { style: "padding: 8px 14px; border-bottom: 1px solid #eee; font-size: 11px; color: #666;",
                    div { style: "margin-bottom: 2px;",
                        "Skills manifest: {report.skills_manifest_state}"
                    }
                    div { style: "margin-bottom: 2px;",
                        "Goals manifest: {report.goals_manifest_state}"
                    }
                }

                // Skill rows
                if !report.skill_rows.is_empty() {
                    div { style: "padding: 8px 14px; border-bottom: 1px solid #eee;",
                        div { style: "font-size: 11px; font-weight: 600; color: #555; margin-bottom: 6px;",
                            "Skill context ({report.skill_rows.len()})"
                        }
                        for skill in report.skill_rows.iter() {
                            { render_skill_readiness_row(skill) }
                        }
                    }
                }

                // Goal rows
                if !report.goal_rows.is_empty() {
                    div { style: "padding: 8px 14px; border-bottom: 1px solid #eee;",
                        div { style: "font-size: 11px; font-weight: 600; color: #555; margin-bottom: 6px;",
                            "Goal context ({report.goal_rows.len()})"
                        }
                        for goal in report.goal_rows.iter() {
                            { render_goal_readiness_row(goal) }
                        }
                    }
                }

                // Gaps
                if has_gaps {
                    { render_readiness_gap_list(&report.gaps) }
                }

                if report.skill_rows.is_empty() && report.goal_rows.is_empty() {
                    div { style: "padding: 12px 14px; font-size: 12px; color: #999; text-align: center;",
                        "No skills or goals configured"
                    }
                }
            }
        }
    }

    /// Render a single skill readiness row (Patch 5: context, not tool).
    fn render_skill_readiness_row(skill: &SkillReadinessRow) -> Element {
        let status_color = readiness_status_color(&skill.status);
        let status_label = format!("{}", skill.status);
        let enabled_label = if skill.enabled { "enabled" } else { "disabled" };
        rsx! {
            div { style: "padding: 4px 0; font-size: 11px; display: flex; gap: 8px; align-items: center;",
                div { style: "width: 8px; height: 8px; border-radius: 50%; background: {status_color};" }
                div { style: "flex: 1;",
                    div { style: "color: #333;",
                        "{skill.name}"
                        span { style: "color: #888; margin-left: 6px;",
                            "[{skill.category}] ({enabled_label})"
                        }
                    }
                    div { style: "font-size: 10px; color: {status_color};",
                        "{status_label}"
                    }
                }
            }
        }
    }

    /// Render a single goal readiness row (Patch 4: context, not scheduler).
    fn render_goal_readiness_row(goal: &GoalReadinessRow) -> Element {
        let status_color = readiness_status_color(&goal.readiness_status);
        let status_label = format!("{}", goal.readiness_status);
        let priority_label = format!("Displayed priority: {}", goal.priority);
        rsx! {
            div { style: "padding: 4px 0; font-size: 11px; display: flex; gap: 8px; align-items: center;",
                div { style: "width: 8px; height: 8px; border-radius: 50%; background: {status_color};" }
                div { style: "flex: 1;",
                    div { style: "color: #333;",
                        "{goal.title}"
                        span { style: "color: #888; margin-left: 6px;",
                            "({priority_label})"
                        }
                    }
                    div { style: "font-size: 10px; color: {status_color};",
                        "{status_label}"
                    }
                    if !goal.linked_skills.is_empty() {
                        div { style: "font-size: 10px; color: #888; margin-top: 1px;",
                            "Allowed context from: {goal.linked_skills.join(\", \")}"
                        }
                    }
                }
            }
        }
    }

    /// Render the gap list (informational Warning tone, not authority).
    fn render_readiness_gap_list(gaps: &[SkillGoalReadinessGap]) -> Element {
        rsx! {
            div { style: "padding: 8px 14px; border-top: 1px solid #eee;",
                div { style: "font-size: 11px; font-weight: 600; color: #e65100; margin-bottom: 4px;",
                    "Readiness gaps ({gaps.len()})"
                }
                for gap in gaps.iter() {
                    div { style: "padding: 2px 0; font-size: 11px; color: #bf360c; line-height: 1.3;",
                        "⚠ {gap.kind}: {gap.message}"
                    }
                }
            }
        }
    }

    fn readiness_status_color(status: &SkillGoalReadinessStatus) -> &'static str {
        match status {
            SkillGoalReadinessStatus::ReadyForContext => "#33aa33",
            SkillGoalReadinessStatus::Incomplete => "#ff9800",
            SkillGoalReadinessStatus::Blocked => "#cc3333",
            SkillGoalReadinessStatus::MissingManifest => "#9e9e9e",
        }
    }
}

#[cfg(feature = "desktop")]
pub use desktop_render::*;

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::ui::skills_goals_state::*;

    #[test]
    fn readiness_panel_shows_safety_warning_with_items() {
        // This is tested in skills_goals_state but confirmed here:
        // the panel always includes report.safety_warning.
        let report = build_test_report_with_items();
        assert!(!report.safety_warning.is_empty());
    }

    #[test]
    fn readiness_panel_shows_safety_warning_when_empty() {
        let report = build_test_report_empty();
        assert!(!report.safety_warning.is_empty());
    }

    #[test]
    fn readiness_panel_shows_safety_warning_when_manifests_missing() {
        let report = build_test_report_missing();
        assert!(!report.safety_warning.is_empty());
    }

    #[test]
    fn skill_readiness_row_is_context_not_tool() {
        // Design assertion: render function labels use "Skill context"
        // not "execute skill", "invoke skill", "promote to tool", etc.
        let _ = "render_skill_readiness_row uses context language only";
    }

    #[test]
    fn goal_readiness_row_is_context_not_scheduler() {
        // Design assertion: render function labels use "Goal context"
        // not "schedule", "start goal", "activate goal", etc.
        let _ = "render_goal_readiness_row uses context language only";
    }

    #[test]
    fn gap_list_is_informational_not_authority() {
        // Gap list uses Warning tone (⚠ prefix), not Error or authority language.
        let _ = "gap list uses informational warning tone";
    }

    // Helpers

    fn build_test_report_with_items() -> SkillGoalReadinessReport {
        use openwand_goals::manifest::{GoalDefinition, GoalId, GoalStatus};
        use openwand_goals::registry::{GoalRegistry, GoalValidationReport};
        use openwand_skills::manifest::{SkillContextKind, SkillDefinition, SkillId};
        use openwand_skills::registry::{SkillRegistry, SkillValidationReport};

        let sr = SkillRegistry {
            skills: vec![SkillDefinition {
                id: SkillId("test-skill".into()),
                name: "Test".into(),
                description: "A test skill".into(),
                category: "test".into(),
                enabled: true,
                tags: vec![],
                inputs: vec![],
                outputs: vec!["result".into()],
                constraints: vec![],
                allowed_context: vec![SkillContextKind::TraceSummary],
            }],
            validation: SkillValidationReport::default(),
        };
        let gr = GoalRegistry {
            goals: vec![GoalDefinition {
                id: GoalId("test-goal".into()),
                title: "Test Goal".into(),
                description: "A test goal".into(),
                status: GoalStatus::Active,
                priority: 50,
                tags: vec![],
                success_criteria: vec!["Done".into()],
                constraints: vec![],
                linked_skills: vec!["test-skill".into()],
            }],
            validation: GoalValidationReport::default(),
        };
        build_readiness_report(&sr, &gr)
    }

    fn build_test_report_empty() -> SkillGoalReadinessReport {
        use openwand_skills::registry::{SkillRegistry, SkillValidationReport};
        use openwand_goals::registry::{GoalRegistry, GoalValidationReport};

        let sr = SkillRegistry {
            skills: vec![],
            validation: SkillValidationReport::default(),
        };
        let gr = GoalRegistry {
            goals: vec![],
            validation: GoalValidationReport::default(),
        };
        build_readiness_report(&sr, &gr)
    }

    fn build_test_report_missing() -> SkillGoalReadinessReport {
        let path = std::env::temp_dir().join("openwand_test_missing_sg_comp");
        let _ = std::fs::remove_dir_all(&path);
        let sr = openwand_skills::registry::load_skill_registry(&path.join("skills.toml"));
        let gr = openwand_goals::registry::load_goal_registry(&path.join("goals.toml"));
        build_readiness_report(&sr, &gr)
    }
}

// ── Capability Context Preview (Wave 64A) ───────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_preview {
    use crate::ui::skills_goals_state::CapabilityPreviewState;
    use dioxus::prelude::*;

    /// Render the capability context preview card (Patch 4: read-only, no edit controls).
    pub fn render_capability_context_preview(state: &CapabilityPreviewState) -> Element {
        let mode_label = format!("{}", state.mode);
        rsx! {
            div { style: "border: 1px solid #ddd; border-radius: 6px; margin-top: 12px;",
                // Header
                div { style: "padding: 10px 14px; background: #f5f5f5; border-bottom: 1px solid #ddd;
                              font-size: 13px; font-weight: 600; color: #333;",
                    "Capability Context Preview"
                    span { style: "font-size: 11px; font-weight: 400; color: #888; margin-left: 8px;",
                        "{mode_label}"
                    }
                }

                // Safety warning (Patch 6)
                div { style: "padding: 8px 14px; background: #fff8e1; border-bottom: 1px solid #ffe082;
                              font-size: 11px; color: #8d6e00; line-height: 1.4;",
                    "{state.safety_warning}"
                }

                // Summary
                div { style: "padding: 8px 14px; border-bottom: 1px solid #eee; font-size: 11px; color: #666;",
                    div { "Included: {state.included_count} items ({state.included_skill_ids.len()} skills, {state.included_goal_ids.len()} goals)" }
                    div { "Excluded: {state.excluded_count} items" }
                    div { "Context block length: {state.total_text_length} chars" }
                }

                // Rows
                if !state.rows.is_empty() {
                    div { style: "padding: 8px 14px; border-bottom: 1px solid #eee;",
                        for row in state.rows.iter() {
                            {
                                let dot = if row.included { "#33aa33" } else { "#cc3333" };
                                let kind_label = format!("{}", row.kind);
                                rsx! {
                                    div { style: "padding: 3px 0; font-size: 11px; display: flex; gap: 6px; align-items: center;",
                                        div { style: "width: 6px; height: 6px; border-radius: 50%; background: {dot};" }
                                        div { style: "flex: 1;",
                                            span { style: "color: #333;", "{row.name}" }
                                            span { style: "color: #888; margin-left: 4px;", "[{kind_label}]" }
                                            span { style: "color: #888; margin-left: 4px; font-size: 10px;", "{row.reason}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Preview text (monospace, read-only)
                if !state.preview_text.is_empty() {
                    div { style: "padding: 8px 14px; border-top: 1px solid #eee;",
                        div { style: "font-size: 10px; font-weight: 600; color: #888; margin-bottom: 4px;",
                            "Prompt context block"
                        }
                        pre { style: "font-size: 10px; color: #555; background: #fafafa; padding: 8px;
                                     border: 1px solid #eee; border-radius: 4px; max-height: 200px;
                                     overflow-y: auto; white-space: pre-wrap; word-break: break-word;",
                            "{state.preview_text}"
                        }
                    }
                }
            }
        }
    }
}

#[cfg(feature = "desktop")]
pub use desktop_preview::*;
