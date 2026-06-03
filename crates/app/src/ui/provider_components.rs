//! Dioxus render functions for provider configuration panel.
//!
//! Desktop-gated. Read-only display — no form submission, no provider calls.

// Note: Actual Dioxus render functions require the desktop feature.
// The test coverage lives in provider_config.rs which tests the pure helpers.
// This file is the integration point for Dioxus desktop rendering.

#[cfg(feature = "desktop")]
mod desktop_render {
    // Desktop Dioxus render functions will be added here.
    // For Wave 21, the pure helpers in provider_config.rs provide the data layer.
    // The Dioxus render functions consume that data.
}
