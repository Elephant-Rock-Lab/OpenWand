//! OpenWand LLM — provider-normalized boundary.
//!
//! This crate wraps all LLM providers behind OpenWand's own trait.
//! No Rig types escape this crate. Session calls `LlmClient`, never Rig.
//!
//! Principle: "Rig speaks to providers. OpenWand decides what provider output is allowed to become."

pub mod adapters;
pub mod client;
pub mod error;
pub mod request;
pub mod response;
pub mod tool_buffer;

#[cfg(feature = "testing")]
pub mod testing;

pub use client::*;
pub use error::*;
pub use request::*;
pub use response::*;
pub use tool_buffer::*;

#[cfg(feature = "testing")]
pub use testing::*;
