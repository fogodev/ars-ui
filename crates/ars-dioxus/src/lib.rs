//! Dioxus framework adapter for ars-ui components.
//!
//! Bridges [`ars_core::Machine`] implementations to Dioxus's signal and virtual DOM
//! system, with multi-platform support for web, desktop, and server-side rendering.
//!
//! # Core primitives
//!
//! - [`use_machine`] — creates a reactive machine service from props
//! - [`UseMachineReturn`] — handle for state, send, derive, and API access
//! - [`EphemeralRef`] — borrow wrapper preventing signal storage of borrowed APIs
//! - [`use_id`] — hydration-safe deterministic ID generation

mod ephemeral;
mod id;
pub mod prelude;
mod use_machine;

pub use ephemeral::EphemeralRef;
#[cfg(feature = "ssr")]
pub use id::reset_id_counter;
pub use id::use_id;
pub use use_machine::{UseMachineReturn, use_machine, use_machine_with_reactive_props};

/// The name of this framework adapter, used in diagnostic messages and feature gating.
pub const ADAPTER_NAME: &str = "dioxus";

/// Describes which rendering platforms this adapter supports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AdapterCapabilities {
    /// `true` if web (WASM) rendering is enabled.
    pub web: bool,
    /// `true` if native desktop rendering is enabled.
    pub desktop: bool,
    /// `true` if server-side rendering is enabled.
    pub ssr: bool,
}
