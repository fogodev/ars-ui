//! Leptos framework adapter for ars-ui components.
//!
//! Bridges [`ars_core::Machine`] implementations to Leptos's fine-grained reactivity
//! system, providing SSR and hydration support for web-based applications.
//!
//! # Core primitives
//!
//! - [`use_machine`] — creates a reactive machine service from props
//! - [`UseMachineReturn`] — handle for state, send, derive, and API access
//! - [`EphemeralRef`] — borrow wrapper preventing signal storage of borrowed APIs
//! - [`use_id`] / [`related_id`] — hydration-safe deterministic ID generation

mod ephemeral;
mod id;
pub mod prelude;
mod use_machine;

pub use ephemeral::EphemeralRef;
#[cfg(feature = "ssr")]
pub use id::reset_id_counter;
pub use id::{related_id, use_id};
pub use use_machine::{UseMachineReturn, use_machine, use_machine_with_reactive_props};

/// The name of this framework adapter, used in diagnostic messages and feature gating.
pub const ADAPTER_NAME: &str = "leptos";

/// Describes which rendering capabilities this adapter supports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AdapterCapabilities {
    /// `true` if server-side rendering is enabled.
    pub ssr: bool,
    /// `true` if client-side hydration of server-rendered markup is enabled.
    pub hydrate: bool,
}
