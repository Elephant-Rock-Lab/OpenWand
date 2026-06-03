//! Adapters for real LLM providers.

#[cfg(feature = "openai-compatible")]
pub mod openai_compatible;

#[cfg(feature = "anthropic-compatible")]
pub mod anthropic_compatible;
