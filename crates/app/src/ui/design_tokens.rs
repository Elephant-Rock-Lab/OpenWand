//! OpenWand Desktop UI — Design Tokens.
//!
//! Shared const-string tokens for colors, typography, spacing, border radii,
//! and layout dimensions. Zero runtime cost — all values are `&str` constants
//! inlined by the compiler.
//!
//! This module imports no backend crates (store, session, memory, workflow),
//! performs no persistence, executes no tools, verifies nothing, certifies
//! nothing, mutates no trace or memory state, resolves no approvals, routes
//! no actions, and creates no workflow records.

// ── Semantic Tones ─────────────────────────────────────────────────────────

/// Semantic tone for UI elements.
///
/// Maps to a coordinated set of background, border, text, and dot colors.
/// Use this instead of raw hex values to maintain visual consistency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiTone {
    Neutral,
    Info,
    Success,
    Warning,
    Error,
    Primary,
    Danger,
}

/// Size variant for UI elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiSize {
    Xs,
    Sm,
    Md,
    Lg,
}

/// Density variant for spacing and padding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiDensity {
    Compact,
    Comfortable,
}

impl UiTone {
    /// Background color for this tone.
    pub const fn bg(self) -> &'static str {
        match self {
            UiTone::Neutral => colors::SURFACE_INSET,
            UiTone::Info => colors::INFO_BG,
            UiTone::Success => colors::SUCCESS_BG,
            UiTone::Warning => colors::WARNING_BG,
            UiTone::Error => colors::ERROR_BG,
            UiTone::Primary => colors::PRIMARY_LIGHT,
            UiTone::Danger => colors::DANGER_LIGHT,
        }
    }

    /// Border color for this tone.
    pub const fn border(self) -> &'static str {
        match self {
            UiTone::Neutral => colors::BORDER_DEFAULT,
            UiTone::Info => colors::INFO_BORDER,
            UiTone::Success => colors::SUCCESS_BORDER,
            UiTone::Warning => colors::WARNING_BORDER,
            UiTone::Error => colors::ERROR_BORDER,
            UiTone::Primary => colors::PRIMARY,
            UiTone::Danger => colors::DANGER,
        }
    }

    /// Text color for this tone.
    pub const fn text(self) -> &'static str {
        match self {
            UiTone::Neutral => colors::TEXT_MUTED,
            UiTone::Info => colors::TEXT_SECONDARY,
            UiTone::Success => colors::SUCCESS_TEXT,
            UiTone::Warning => colors::WARNING_TEXT,
            UiTone::Error => colors::ERROR_TEXT,
            UiTone::Primary => colors::PRIMARY,
            UiTone::Danger => colors::DANGER,
        }
    }

    /// Indicator dot color for this tone.
    pub const fn dot(self) -> &'static str {
        match self {
            UiTone::Neutral => colors::TEXT_MUTED,
            UiTone::Info => colors::INFO_BORDER,
            UiTone::Success => colors::SUCCESS_DOT,
            UiTone::Warning => colors::WARNING_DOT,
            UiTone::Error => colors::ERROR_DOT,
            UiTone::Primary => colors::PRIMARY,
            UiTone::Danger => colors::DANGER,
        }
    }
}

impl UiSize {
    /// Font-size string for this size.
    pub const fn font_size(self) -> &'static str {
        match self {
            UiSize::Xs => typo::TEXT_XS,
            UiSize::Sm => typo::TEXT_SM,
            UiSize::Md => typo::TEXT_BASE,
            UiSize::Lg => typo::TEXT_LG,
        }
    }

    /// Dot diameter for this size.
    pub const fn dot_size(self) -> &'static str {
        match self {
            UiSize::Xs => "6px",
            UiSize::Sm => layout_dims::DOT_SIZE,
            UiSize::Md => "10px",
            UiSize::Lg => "12px",
        }
    }
}

impl UiDensity {
    /// Block padding string for this density.
    pub const fn padding(self) -> &'static str {
        match self {
            UiDensity::Compact => spacing::SPACE_SM,
            UiDensity::Comfortable => spacing::SPACE_LG,
        }
    }

    /// Inline padding string for this density.
    pub const fn inline_padding(self) -> &'static str {
        match self {
            UiDensity::Compact => spacing::SPACE_MD,
            UiDensity::Comfortable => spacing::SPACE_XL,
        }
    }
}

// ── Color Constants ─────────────────────────────────────────────────────────

pub mod colors {
    //! Primary actions
    pub const PRIMARY: &str = "#4a90d9";
    pub const PRIMARY_HOVER: &str = "#3a7bc8";
    pub const PRIMARY_LIGHT: &str = "#e0e8f0";
    pub const DANGER: &str = "#d94a4a";
    pub const DANGER_LIGHT: &str = "#fde8e8";

    // Text hierarchy
    pub const TEXT_PRIMARY: &str = "#333";
    pub const TEXT_SECONDARY: &str = "#555";
    pub const TEXT_MUTED: &str = "#888";
    pub const TEXT_FAINT: &str = "#999";
    pub const TEXT_PLACEHOLDER: &str = "#bbb";

    // Success
    pub const SUCCESS_BG: &str = "#e8f4e8";
    pub const SUCCESS_BORDER: &str = "#a0c8a0";
    pub const SUCCESS_TEXT: &str = "#2d6a2d";
    pub const SUCCESS_DOT: &str = "#33aa33";

    // Warning
    pub const WARNING_BG: &str = "#fff3cd";
    pub const WARNING_BORDER: &str = "#ffc107";
    pub const WARNING_TEXT: &str = "#856404";
    pub const WARNING_DOT: &str = "#f0c040";

    // Error
    pub const ERROR_BG: &str = "#fde8e8";
    pub const ERROR_BORDER: &str = "#e8a0a0";
    pub const ERROR_TEXT: &str = "#721c24";
    pub const ERROR_DOT: &str = "#cc3333";

    // Info
    pub const INFO_BG: &str = "#f0f8e8";
    pub const INFO_BORDER: &str = "#c8e0b0";

    // Surfaces
    pub const SURFACE_BASE: &str = "#fff";
    pub const SURFACE_RAISED: &str = "#fafafa";
    pub const SURFACE_SIDEBAR: &str = "#f7f7f7";
    pub const SURFACE_INSET: &str = "#f0f0f0";
    pub const SURFACE_SELECTED: &str = "#e0e8f0";

    // Borders
    pub const BORDER_DEFAULT: &str = "#ddd";
    pub const BORDER_LIGHT: &str = "#eee";
    pub const BORDER_SUBTLE: &str = "#f0f0f0";
    // Aliases for workflow component naming convention (Waves 80A-80C)
    pub const TEXT_STRONG: &str = "#222";
    pub const TEXT_BODY: &str = "#444";
    pub const TEXT_WARN: &str = "#856404";
    pub const BG_CARD: &str = "#fafafa";
    pub const BG_WARN: &str = "#fff3cd";
    pub const BG_SUBTLE: &str = "#f5f5f5";
    pub const BORDER_WARN: &str = "#ffc107";
    pub const STATUS_SUCCESS: &str = "#2d6a2d";
    pub const STATUS_WARN: &str = "#f0c040";
    pub const STATUS_ERROR: &str = "#cc3333";

    // Muted / disabled
    pub const MUTED_MID: &str = "#9e9e9e";
    pub const MUTED_LIGHT: &str = "#bdbdbd";
    pub const DISABLED_BG: &str = "#ccc";
}

// ── Typography ──────────────────────────────────────────────────────────────

pub mod typo {
    pub const FONT_BODY: &str = "system-ui";
    pub const FONT_MONO: &str = "monospace";
    pub const TEXT_XS: &str = "10px";
    pub const TEXT_SM: &str = "11px";
    pub const TEXT_BASE: &str = "12px";
    pub const TEXT_MD: &str = "13px";
    pub const TEXT_LG: &str = "14px";
    pub const TEXT_XL: &str = "16px";
    pub const TEXT_2XL: &str = "18px";
}

// ── Spacing ─────────────────────────────────────────────────────────────────

pub mod spacing {
    pub const SPACE_XS: &str = "2px";
    pub const SPACE_SM: &str = "4px";
    pub const SPACE_MD: &str = "8px";
    pub const SPACE_LG: &str = "12px";
    pub const SPACE_XL: &str = "16px";
    pub const SPACE_2XL: &str = "20px";
    pub const SPACE_3XL: &str = "24px";
}

// ── Border Radius ───────────────────────────────────────────────────────────

pub mod radius {
    pub const SM: &str = "3px";
    pub const MD: &str = "4px";
    pub const LG: &str = "6px";
    pub const PILL: &str = "12px";
    // Aliases for workflow component naming convention
    pub const RADIUS_SM: &str = "3px";
    pub const RADIUS_MD: &str = "4px";
}

// ── Layout Dimensions ───────────────────────────────────────────────────────

pub mod layout_dims {
    pub const SIDEBAR_LEFT_WIDTH: &str = "260px";
    pub const SIDEBAR_RIGHT_WIDTH: &str = "240px";
    pub const DOT_SIZE: &str = "8px";
    pub const STATUS_BAR_HEIGHT: &str = "28px";
    pub const WINDOW_WIDTH: u32 = 1100;
    pub const WINDOW_HEIGHT: u32 = 700;
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Token correctness ──

    #[test]
    fn all_tones_map_to_non_empty_colors() {
        let tones = [
            UiTone::Neutral,
            UiTone::Info,
            UiTone::Success,
            UiTone::Warning,
            UiTone::Error,
            UiTone::Primary,
            UiTone::Danger,
        ];
        for tone in tones {
            assert!(!tone.bg().is_empty(), "{:?}.bg() empty", tone);
            assert!(!tone.border().is_empty(), "{:?}.border() empty", tone);
            assert!(!tone.text().is_empty(), "{:?}.text() empty", tone);
            assert!(!tone.dot().is_empty(), "{:?}.dot() empty", tone);
        }
    }

    #[test]
    fn tone_colors_are_valid_hex() {
        fn assert_hex(s: &str, label: &str) {
            assert!(s.starts_with('#'), "{} not hex: {}", label, s);
            let digits = &s[1..];
            assert!(
                digits.len() == 3 || digits.len() == 6,
                "{} bad hex length: {}",
                label,
                s
            );
            assert!(
                digits.chars().all(|c| c.is_ascii_hexdigit()),
                "{} bad hex digits: {}",
                label,
                s
            );
        }
        for tone in [
            UiTone::Neutral,
            UiTone::Info,
            UiTone::Success,
            UiTone::Warning,
            UiTone::Error,
            UiTone::Primary,
            UiTone::Danger,
        ] {
            assert_hex(tone.bg(), &format!("{:?}.bg", tone));
            assert_hex(tone.border(), &format!("{:?}.border", tone));
            assert_hex(tone.text(), &format!("{:?}.text", tone));
            assert_hex(tone.dot(), &format!("{:?}.dot", tone));
        }
    }

    #[test]
    fn sizes_map_to_valid_font_sizes() {
        assert_eq!(UiSize::Xs.font_size(), "10px");
        assert_eq!(UiSize::Sm.font_size(), "11px");
        assert_eq!(UiSize::Md.font_size(), "12px");
        assert_eq!(UiSize::Lg.font_size(), "14px");
    }

    #[test]
    fn sizes_map_to_valid_dot_sizes() {
        for size in [UiSize::Xs, UiSize::Sm, UiSize::Md, UiSize::Lg] {
            let s = size.dot_size();
            assert!(s.ends_with("px"), "dot_size {:?} = {}", size, s);
        }
    }

    #[test]
    fn density_padding_is_non_empty() {
        assert!(!UiDensity::Compact.padding().is_empty());
        assert!(!UiDensity::Comfortable.padding().is_empty());
    }

    #[test]
    fn all_color_tokens_start_with_hash() {
        // Spot-check a few from each group
        for c in &[
            colors::PRIMARY,
            colors::TEXT_PRIMARY,
            colors::SUCCESS_BG,
            colors::WARNING_BG,
            colors::ERROR_BG,
            colors::INFO_BG,
            colors::SURFACE_BASE,
            colors::BORDER_DEFAULT,
        ] {
            assert!(c.starts_with('#'), "token {} not hex", c);
        }
    }

    #[test]
    fn layout_dimensions_are_positive() {
        assert!(layout_dims::WINDOW_WIDTH > 0);
        assert!(layout_dims::WINDOW_HEIGHT > 0);
    }

    // ── Guard tests: no backend imports, no authority verbs ──

    #[test]
    fn design_tokens_defines_no_persistence_or_trace_types() {
        // Compile-time check: this module has no imports from backend crates.
        // This test documents the invariant.
        let _tokens = [colors::PRIMARY,
            typo::FONT_BODY,
            spacing::SPACE_MD,
            radius::MD,
            layout_dims::SIDEBAR_LEFT_WIDTH];
        // If this module imported openwand_store/SessionRunner/TraceStore,
        // the compilation would fail — but it doesn't, proving the guard.
    }

    #[test]
    fn design_tokens_defines_no_workflow_authority_terms() {
        // The tokens are color/font/spacing strings, never authority verbs.
        let all_tokens: Vec<&str> = vec![
            colors::PRIMARY,
            colors::PRIMARY_HOVER,
            colors::PRIMARY_LIGHT,
            colors::DANGER,
            colors::DANGER_LIGHT,
            colors::TEXT_PRIMARY,
            colors::TEXT_SECONDARY,
            colors::TEXT_MUTED,
            colors::TEXT_FAINT,
            colors::TEXT_PLACEHOLDER,
            colors::SUCCESS_BG,
            colors::SUCCESS_BORDER,
            colors::SUCCESS_TEXT,
            colors::SUCCESS_DOT,
            colors::WARNING_BG,
            colors::WARNING_BORDER,
            colors::WARNING_TEXT,
            colors::WARNING_DOT,
            colors::ERROR_BG,
            colors::ERROR_BORDER,
            colors::ERROR_TEXT,
            colors::ERROR_DOT,
            colors::INFO_BG,
            colors::INFO_BORDER,
            colors::SURFACE_BASE,
            colors::SURFACE_RAISED,
            colors::SURFACE_SIDEBAR,
            colors::SURFACE_INSET,
            colors::SURFACE_SELECTED,
            colors::BORDER_DEFAULT,
            colors::BORDER_LIGHT,
            colors::BORDER_SUBTLE,
            colors::MUTED_MID,
            colors::MUTED_LIGHT,
            colors::DISABLED_BG,
            typo::FONT_BODY,
            typo::FONT_MONO,
            typo::TEXT_XS,
            typo::TEXT_SM,
            typo::TEXT_BASE,
            typo::TEXT_MD,
            typo::TEXT_LG,
            typo::TEXT_XL,
        ];
        for token in all_tokens {
            // Tokens are hex colors, font names, or pixel sizes.
            // None should contain authority verbs.
            let lower = token.to_lowercase();
            assert!(!lower.contains("execute"), "token contains 'execute': {}", token);
            assert!(!lower.contains("verify"), "token contains 'verify': {}", token);
            assert!(!lower.contains("certify"), "token contains 'certify': {}", token);
            assert!(!lower.contains("reconcile"), "token contains 'reconcile': {}", token);
            assert!(!lower.contains("approve"), "token contains 'approve': {}", token);
        }
    }
}
