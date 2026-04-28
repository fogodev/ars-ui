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

mod attrs;
pub mod dismissable;
mod ephemeral;
mod id;
pub mod prelude;
mod provider;
mod use_machine;

#[cfg(feature = "web")]
pub use attrs::apply_styles_cssom;
pub use attrs::{
    ArsNonceCssCtx, ArsNonceStyle, DioxusAttrResult, append_nonce_css, attr_map_to_dioxus,
    attr_map_to_dioxus_inline_attrs, intern_attr_name, use_style_strategy,
};
pub use ephemeral::EphemeralRef;
#[cfg(feature = "ssr")]
pub use id::reset_id_counter;
pub use id::use_id;
#[cfg(feature = "desktop")]
pub use provider::DesktopPlatform;
#[cfg(feature = "web")]
pub use provider::WebPlatform;
pub use provider::{
    ArsContext, ArsProvider, ArsProviderProps, DioxusPlatform, DragData, FilePickerOptions,
    NullPlatform, resolve_locale, t, use_intl_backend, use_locale, use_messages,
    use_modality_context, use_number_formatter, use_platform, warn_missing_provider,
};
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
