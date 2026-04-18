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
//! - [`use_id`] — hydration-safe deterministic ID generation

mod attrs;
mod controlled;
mod ephemeral;
mod id;
pub mod prelude;
mod provider;
mod use_machine;

#[cfg(not(feature = "ssr"))]
pub use attrs::apply_styles_cssom;
pub use attrs::{LeptosAttrResult, attr_map_to_leptos, use_style_strategy};
pub use controlled::use_controlled_prop;
pub use ephemeral::EphemeralRef;
#[cfg(feature = "ssr")]
pub use id::reset_id_counter;
pub use id::use_id;
pub use provider::{
    ArsContext, provide_ars_context, resolve_locale, t, use_icu_provider, use_locale, use_messages,
    use_number_formatter, warn_missing_provider,
};
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
