//! Workflow command review desktop UI components.
//!
//! Read-only display of recorded command review state using Wave 52A
//! design-system tokens. Displays review decision, acknowledgment snapshot,
//! feedback, and safety copy.
//!
//! Components accept already-prepared state. They do not execute, approve,
//! reconcile, schedule, or mutate workflow state.

use crate::ui::design_tokens::*;
use crate::ui::workflow_command_review_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

pub fn command_review_decision_tone(decision: &str) -> UiTone {
    match decision {
        "acknowledged" | "proceed" => UiTone::Success,
        "rejected" | "blocked" => UiTone::Error,
        "pending" => UiTone::Warning,
        _ => UiTone::Neutral,
    }
}

pub fn command_review_safety_text() -> String {
    workflow_command_review_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;
    use dioxus::prelude::*;

    pub fn render_command_review_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_FAINT, colors::BORDER_LIGHT,
        );
        rsx! { div { style: "{style}", "No command review records" } }
    }

    pub fn render_command_review_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM, colors::TEXT_MUTED,
        );
        rsx! { div { style: "{style}", "Loading command review\u{2026}" } }
    }

    pub fn render_command_review_panel(state: &WorkflowCommandReviewUiState) -> Element {
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
                div { style: "{title_style}", "Command Review" }

                // Review summary
                if let Some(review) = &state.latest_review {
                    div { style: "{card}",
                        div { style: "display: flex; align-items: center; gap: {spacing::SPACE_SM}; margin-bottom: {spacing::SPACE_SM};",
                            {
                                let tone = command_review_decision_tone(&review.decision);
                                let badge = tone_to_badge_color(tone);
                                rsx! {
                                    span { style: "background: {badge}; color: white; padding: 2px 8px; border-radius: {radius::RADIUS_SM}; font-size: {typo::TEXT_XS};",
                                        "{review.decision}"
                                    }
                                }
                            }
                            span { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_MUTED};",
                                "by {review.reviewer}"
                            }
                        }
                        div { style: "font-size: {typo::TEXT_XS}; color: {colors::TEXT_FAINT}; font-family: monospace;",
                            "{review.review_id}"
                        }
                    }
                }

                // Acknowledgment snapshot
                if let Some(snap) = &state.acknowledgment_snapshot {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Acknowledgment Snapshot"
                        }
                        div { style: "font-family: monospace; font-size: {typo::TEXT_SM}; color: {colors::TEXT_BODY}; background: {colors::BG_SUBTLE}; padding: {spacing::SPACE_SM}; border-radius: {radius::RADIUS_SM}; margin-bottom: {spacing::SPACE_SM};",
                            "{snap.display_command}"
                        }

                        // Status flags
                        div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: {spacing::SPACE_SM}; font-size: {typo::TEXT_SM};",
                            div { style: "color: {if snap.display_only {colors::TEXT_MUTED} else {colors::STATUS_ERROR}};",
                                if snap.display_only { "Display only" } else { "Executable (not from this UI)" }
                            }
                            div { style: "color: {if snap.review_only {colors::TEXT_MUTED} else {colors::TEXT_BODY}};",
                                if snap.review_only { "Review only" } else { "Acknowledged" }
                            }
                        }

                        // Missing inputs
                        if !snap.missing_inputs.is_empty() {
                            div { style: "margin-top: {spacing::SPACE_SM};",
                                div { style: "font-size: {typo::TEXT_XS}; color: {colors::STATUS_ERROR}; font-weight: 600;",
                                    "Missing inputs: {snap.missing_inputs.len()}"
                                }
                                for mi in &snap.missing_inputs {
                                    div { style: "font-size: {typo::TEXT_XS}; color: {colors::TEXT_MUTED}; padding-left: {spacing::SPACE_SM};",
                                        "{mi}"
                                    }
                                }
                            }
                        }

                        div { style: "margin-top: {spacing::SPACE_SM}; font-size: {typo::TEXT_XS}; color: {colors::TEXT_FAINT};",
                            "Hash: {snap.copyable_text_hash}"
                        }
                    }
                }

                // Feedback
                if !state.feedback.is_empty() {
                    div { style: "{card}",
                        div { style: "font-weight: 600; font-size: {typo::TEXT_SM}; margin-bottom: {spacing::SPACE_XS};",
                            "Feedback"
                        }
                        for line in &state.feedback {
                            div { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_BODY}; padding: {spacing::SPACE_XS} 0; border-bottom: 1px solid {colors::BORDER_LIGHT};",
                                "{line}"
                            }
                        }
                    }
                }

                for w in &state.warnings {
                    {render_warning(w)}
                }

                {render_safety_footer()}
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
        let text = command_review_safety_text();
        rsx! { div { style: "{style}", "{text}" } }
    }
}

#[cfg(feature = "desktop")]
pub use desktop_render::*;
