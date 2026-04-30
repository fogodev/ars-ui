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
//! - [`use_stable_id`] — hook-slot-stable generated ID allocation

mod attrs;
mod callbacks;
pub mod dismissable;
mod ephemeral;
pub mod error_boundary;
mod event_mapping;
mod hydration;
mod id;
mod nonce;
mod platform;
pub mod prelude;
mod provider;
mod safe_listener;
mod use_machine;

#[cfg(feature = "web")]
pub use attrs::{
    CssomStyleHandle, apply_styles_cssom, use_cssom_styles, use_cssom_styles_from_attrs,
};
pub use attrs::{
    DioxusAttrResult, attr_map_to_dioxus, attr_map_to_dioxus_inline_attrs, use_style_strategy,
};
pub use callbacks::{emit, emit_map};
pub use ephemeral::EphemeralRef;
pub use event_mapping::dioxus_key_to_keyboard_key;
#[cfg(any(feature = "ssr", all(feature = "web", target_arch = "wasm32")))]
pub use hydration::HydrationSnapshot;
#[cfg(feature = "ssr")]
pub use hydration::serialize_snapshot;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub use hydration::{
    mark_body_hydrated, setup_focus_scope_hydration_safe, warn_if_mounted_id_mismatch,
};
#[cfg(feature = "ssr")]
pub use id::reset_id_counter;
pub use id::{use_id, use_stable_id};
pub use nonce::{
    ArsNonceCssCtx, ArsNonceCssProvider, ArsNonceStyle, NonceCssRule, append_nonce_css,
    collect_nonce_css_from_attrs, remove_nonce_css, upsert_nonce_css,
    use_nonce_css_context_provider, use_nonce_css_from_attrs, use_nonce_css_rule,
};
#[cfg(feature = "desktop")]
pub use platform::DesktopPlatform;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub use platform::WebPlatform;
pub use platform::{
    DioxusPlatform, DragData, FilePickerOptions, NullPlatform, PlatformDragEvent,
    default_dioxus_platform, use_platform,
};
pub use provider::{
    ArsContext, ArsProvider, ArsProviderProps, resolve_locale, t, use_intl_backend, use_locale,
    use_messages, use_modality_context, use_number_formatter, warn_missing_provider,
};
#[cfg(feature = "web")]
pub use safe_listener::{
    SafeEventListener, SafeEventListenerOptions, use_safe_event_listener, use_safe_event_listeners,
};
pub use use_machine::{UseMachineReturn, use_machine, use_machine_with_reactive_props};
#[cfg(any(feature = "ssr", all(feature = "web", target_arch = "wasm32")))]
pub use use_machine::{use_machine_hydrated, use_machine_with_reactive_props_hydrated};

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
