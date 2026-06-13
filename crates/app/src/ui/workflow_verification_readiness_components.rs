//! Workflow verification readiness desktop UI components.
//!
//! Read-only display of recorded verification readiness state using Wave 52A
//! design-system tokens. Displays readiness summary with pass/fail counts.
//!
//! Components accept already-prepared state. They do not verify, certify,
//! execute tools, append trace, write memory, or create workflow records.

use crate::ui::design_tokens::*;
use crate::ui::workflow_verification_readiness_state::*;

// ── Pure helpers (always compiled, testable) ──────────────────────────────

/// Semantic tone for readiness status.
pub fn verification_status_tone(status: &str) -> UiTone {
    match status.to_lowercase().as_str() {
        "ready" => UiTone::Success,
        "notready" | "blocked" => UiTone::Error,
        "inconclusive" => UiTone::Warning,
        _ => UiTone::Neutral,
    }
}

/// Human-readable label for verification readiness status.
pub fn verification_status_label(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "ready" => "Ready for verification (recorded)".into(),
        "notready" => "Not ready (recorded)".into(),
        "blocked" => "Blocked (recorded)".into(),
        "inconclusive" => "Inconclusive (recorded)".into(),
        _ => format!("Recorded: {}", status),
    }
}

/// Safety text.
pub fn verification_safety_text() -> String {
    verification_readiness_safety_warning()
}

// ── Desktop-gated Dioxus render functions ────────────────────────────────

#[cfg(feature = "desktop")]
mod desktop_render {
    use super::*;
    use crate::ui::components::*;

    use dioxus::prelude::*;

    /// Empty state.
    pub fn render_verification_readiness_empty_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {}; border-bottom: 1px solid {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_FAINT, colors::BORDER_LIGHT,
        );
        rsx! {
            div { style: "{style}",
                "No verification readiness records"
            }
        }
    }

    /// Loading state.
    pub fn render_verification_readiness_loading_state() -> Element {
        let style = format!(
            "padding: {} {}; text-align: center; font-size: {}; color: {};",
            spacing::SPACE_LG, spacing::SPACE_XL, typo::TEXT_SM,
            colors::TEXT_MUTED,
        );
        rsx! {
            div { style: "{style}",
                "Loading verification readiness\u{2026}"
            }
        }
    }

    /// Full verification readiness panel from summary row.
    pub fn render_verification_readiness_panel(summary: &VerificationReadinessSummaryRow) -> Element {
        let section_title_style = format!(
            "font-size: {}; font-weight: 600; color: {}; margin-bottom: {};",
            typo::TEXT_BASE, colors::TEXT_STRONG, spacing::SPACE_SM,
        );
        let card_style = format!(
            "background: {}; border: 1px solid {}; border-radius: {}; padding: {}; margin-bottom: {};",
            colors::BG_CARD, colors::BORDER_LIGHT, radius::RADIUS_MD,
            spacing::SPACE_MD, spacing::SPACE_MD,
        );
        let tone = verification_status_tone(&summary.status);
        let badge_color = tone_to_badge_color(tone);
        let label = verification_status_label(&summary.status);
        let total = summary.passed_count + summary.failed_count;

        rsx! {
            div {
                // Section header
                div { style: "{section_title_style}",
                    "Verification Readiness"
                }

                // Summary card
                div { style: "{card_style}",
                    // Status badge + target info
                    div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: {spacing::SPACE_SM};",
                        span { style: "background: {badge_color}; color: white; padding: 2px 8px; border-radius: {radius::RADIUS_SM}; font-size: {typo::TEXT_XS};",
                            "{label}"
                        }
                        span { style: "font-size: {typo::TEXT_SM}; color: {colors::TEXT_MUTED};",
                            "{summary.target_kind}: {summary.target_id}"
                        }
                    }

                    // Pass/fail counts
                    div { style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: {spacing::SPACE_MD}; text-align: center;",
                        div {
                            div { style: "font-size: {typo::TEXT_2XL}; font-weight: 700; color: {colors::TEXT_STRONG};",
                                "{total}"
                            }
                            div { style: "font-size: {typo::TEXT_XS}; color: {colors::TEXT_MUTED};",
                                "Total"
                            }
                        }
                        div {
                            div { style: "font-size: {typo::TEXT_2XL}; font-weight: 700; color: {colors::STATUS_SUCCESS};",
                                "{summary.passed_count}"
                            }
                            div { style: "font-size: {typo::TEXT_XS}; color: {colors::TEXT_MUTED};",
                                "Passed"
                            }
                        }
                        div {
                            div { style: "font-size: {typo::TEXT_2XL}; font-weight: 700; color: {colors::STATUS_ERROR};",
                                "{summary.failed_count}"
                            }
                            div { style: "font-size: {typo::TEXT_XS}; color: {colors::TEXT_MUTED};",
                                "Failed"
                            }
                        }
                    }

                    // Readiness ID
                    div { style: "margin-top: {spacing::SPACE_SM}; font-size: {typo::TEXT_XS}; color: {colors::TEXT_FAINT}; font-family: monospace;",
                        "{summary.readiness_id}"
                    }
                }

                // Safety notice
                {render_safety_footer()}
            }
        }
    }

    fn render_safety_footer() -> Element {
        let style = format!(
            "font-size: {}; color: {}; padding: {} 0; border-top: 1px solid {}; margin-top: {};",
            typo::TEXT_XS, colors::TEXT_FAINT, spacing::SPACE_SM,
            colors::BORDER_LIGHT, spacing::SPACE_MD,
        );
        let text = verification_safety_text();
        rsx! {
            div { style: "{style}", "{text}" }
        }
    }
}

#[cfg(feature = "desktop")]
pub use desktop_render::*;
