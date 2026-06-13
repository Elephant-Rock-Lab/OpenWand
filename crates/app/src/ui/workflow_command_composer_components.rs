//! Workflow command composer desktop UI components.
//!
//! Read-only display of recorded command composer state using Wave 52A
//! design-system tokens. Displays command descriptor, arguments, missing
//! inputs, evidence links, predicates, and safety copy.
//!
//! Components accept already-prepared state. They do not execute, route,
//! approve, reconcile, schedule, or mutate workflow state.

use crate::ui::design_tokens::*;
use crate::ui::workflow_command_composer_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

pub fn composer_status_tone(status: &str) -> UiTone {
    match status {
        "ready" | "complete" => UiTone::Success,
        "missinginputs" | "incomplete" => UiTone::Warning,
        "blocked" | "error" => UiTone::Error,
        _ => UiTone::Neutral,
    }
}

pub fn composer_safety_text() -> String {
    workflow_command_composer_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    use dioxus::prelude::*;

    pub fn render_command_composer_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_FAINT, colors::BORDER_LIGHT,
        );
        rsx! { div { style: "{style}", "No command composer records" } }
    }

    pub fn render_command_composer_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM, colors::TEXT_MUTED,
        );
        rsx! { div { style: "{style}", "Loading command composer\u{2026}" } }
    }

    pub fn render_command_composer_panel(state: &WorkflowCommandComposerUiState) -> Element {
        let title_style = format!(
            "font-size: {}; font-weight: 600; color: {}; margin-bottom: {};",
            typo::TEXT_BASE, colors::TEXT_STRONG, spacing::SPACE_SM,
        );
        let card = format!(
            "background: {}; border: 1px solid {}; border-radius: {}; padding: {}; margin-bottom: {};",
            colors::BG_CARD, colors::BORDER_LIGHT, radius::RADIUS_MD, spacing::SPACE_MD, spacing::SPACE_MD,
        );

        rsx! {
            div {
                div { style: "{title_style}", "Command Composer" }

                // Status
                if let Some(rec) = &state.latest_record {
                    div { style: "{card}",
                        div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_MUTED};",
                            "Composer: {rec.composer_id}"
                        }
                        div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_BODY};",
                            "Status: {rec.status}"
                        }
                    }
                }

                // Command descriptor
                if let Some(desc) = &state.descriptor {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Command"
                        }
                        div { style: "font-family: monospace; font-size: {typo::TEXT_SM}; color: {colors::TEXT_BODY}; background: {colors::BG_SUBTLE}; padding: {spacing::SPACE_SM}; border-radius: {radius::RADIUS_SM}; margin-bottom: {spacing::SPACE_SM};",
                            "{desc.display_command}"
                        }
                        div { style: "font-size: {typo::TEXT_XS}; color: {colors::TEXT_MUTED};",
                            "Kind: {desc.command_kind}"
                        }
                        if desc.has_missing_inputs {
                            div { style: "font-size: {typo::TEXT_XS}; color: {colors::STATUS_ERROR};",
                                "Has missing inputs"
                            }
                        }
                    }
                }

                // Arguments
                if !state.arguments.is_empty() {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Arguments ({state.arguments.len()})"
                        }
                        for arg in &state.arguments {
                            div { style: "font-size: {typo::TEXT_SM}; padding: {spacing::SPACE_XS} 0; border-bottom: 1px solid {colors::BORDER_LIGHT}; display: flex; justify-content: space-between;",
                                span { style: "color: {colors::TEXT_BODY};",
                                    if arg.required { "{arg.name} *" } else { "{arg.name}" }
                                }
                                span { style: "color: {if arg.missing {colors::STATUS_ERROR} else {colors::TEXT_MUTED}};",
                                    if arg.missing { "MISSING" } else {
                                        if let Some(v) = &arg.value_preview { "{v}" } else { "-" }
                                    }
                                }
                            }
                        }
                    }
                }

                // Missing inputs
                if !state.missing_inputs.is_empty() {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; color: {colors::STATUS_ERROR}; margin-bottom: {spacing::SPACE_XS};",
                            "Missing Inputs ({state.missing_inputs.len()})"
                        }
                        for mi in &state.missing_inputs {
                            div { style: "font-size: {typo::TEXT_SM}; padding: {spacing::SPACE_XS} 0; border-bottom: 1px solid {colors::BORDER_LIGHT};",
                                div { style: "color: {colors::TEXT_BODY};", "{mi.name}" }
                                div { style: "color: {colors::TEXT_MUTED}; font-size: {typo::TEXT_XS};",
                                    "{mi.reason} \u{2192} {mi.suggested_source}"
                                }
                            }
                        }
                    }
                }

                // Predicates
                if !state.predicates.is_empty() {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};", "Predicates" }
                        {render_predicates(&state.predicates)}
                    }
                }

                for w in &state.warnings {
                    {render_warning(w)}
                }

                {render_safety_footer()}
            }
        }
    }

    fn render_predicates(preds: &[WorkflowCommandComposerPredicateRow]) -> Element {
        let pass_color = tone_to_badge_color(UiTone::Success);
        let fail_color = tone_to_badge_color(UiTone::Error);
        let row_style = format!(
            "display: flex; justify-content: space-between; padding: {} 0; border-bottom: 1px solid {}; font-size: {};",
            spacing::SPACE_XS, colors::BORDER_LIGHT, typo::TEXT_SM,
        );
        rsx! {
            div {
                for pred in preds {
                    {
                        let bg = if pred.passed { pass_color.clone() } else { fail_color.clone() };
                        let status_text = if pred.passed { "PASS" } else { "FAIL" };
                        rsx! {
                            div { style: "{row_style}",
                                span { style: "color: {colors::TEXT_BODY}; flex: 1;", "{pred.predicate}" }
                                span { style: "background: {bg}; color: white; padding: 1px 6px; border-radius: {radius::RADIUS_SM}; font-size: {typo::TEXT_XS}; margin-right: {spacing::SPACE_SM};", "{status_text}" }
                                span { style: "color: {colors::TEXT_MUTED}; flex: 2;", "{pred.reason}" }
                            }
                        }
                    }
                }
            }
        }
    }

    fn render_warning(text: &str) -> Element {
        let style = format!(
            "background: {}; border: 1px solid {}; border-radius: {}; padding: {}; margin-bottom: {}; font-size: {}; color: {};",
            colors::BG_WARN, colors::BORDER_WARN, radius::RADIUS_SM,
            spacing::SPACE_SM, spacing::SPACE_SM, typo::TEXT_SM, colors::TEXT_WARN,
        );
        rsx! { div { style: "{style}", "{text}" } }
    }

    fn render_safety_footer() -> Element {
        let style = format!(
            "font-size: {}; color: {}; padding: {} 0; border-top: 1px solid {}; margin-top: {};",
            typo::TEXT_XS, colors::TEXT_FAINT, spacing::SPACE_SM,
            colors::BORDER_LIGHT, spacing::SPACE_MD,
        );
        let text = composer_safety_text();
        rsx! { div { style: "{style}", "{text}" } }
    }
}

#[cfg(feature = "desktop")]
pub use desktop_render::*;
