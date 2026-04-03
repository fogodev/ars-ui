//! Dioxus framework adapter for ars-ui components.
//!
//! Bridges [`ars_core::Machine`] implementations to Dioxus's signal and virtual DOM
//! system, with multi-platform support for web, desktop, and server-side rendering.

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
