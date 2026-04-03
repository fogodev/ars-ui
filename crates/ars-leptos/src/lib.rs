//! Leptos framework adapter for ars-ui components.
//!
//! Bridges [`ars_core::Machine`] implementations to Leptos's fine-grained reactivity
//! system, providing SSR and hydration support for web-based applications.

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
