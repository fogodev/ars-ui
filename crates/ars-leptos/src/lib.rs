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
mod callbacks;
mod controlled;
pub mod dismissable;
mod ephemeral;
mod event_mapping;
mod hydration;
mod id;
mod nonce;
pub mod prelude;
mod provider;
mod safe_listener;
mod use_machine;

#[cfg(not(feature = "ssr"))]
pub use attrs::{
    CssomStyleHandle, apply_styles_cssom, use_cssom_styles, use_cssom_styles_from_attrs,
};
pub use attrs::{
    LeptosAttrResult, LeptosAttribute, attr_map_to_leptos, attr_map_to_leptos_inline_attrs,
    use_style_strategy,
};
pub use callbacks::{emit, emit_map};
pub use controlled::use_controlled_prop;
pub use ephemeral::EphemeralRef;
pub use event_mapping::leptos_key_to_keyboard_key;
#[cfg(feature = "ssr")]
pub use hydration::{HydrationSnapshot, serialize_snapshot};
#[cfg(all(feature = "hydrate", target_arch = "wasm32"))]
pub use hydration::{
    mark_body_hydrated, setup_focus_scope_hydration_safe, warn_if_mounted_id_mismatch,
};
#[cfg(feature = "ssr")]
pub use id::reset_id_counter;
pub use id::use_id;
pub use nonce::{
    ArsNonceCssCtx, ArsNonceCssProvider, ArsNonceStyle, NonceCssRule, append_nonce_css,
    collect_nonce_css_from_attrs, remove_nonce_css, upsert_nonce_css,
    use_nonce_css_context_provider, use_nonce_css_from_attrs, use_nonce_css_rule,
};
pub use provider::{
    ArsContext, ArsProvider, provide_ars_context, resolve_locale, t, use_intl_backend, use_locale,
    use_messages, use_modality_context, use_number_formatter, warn_missing_provider,
};
pub use safe_listener::{
    SafeEventListener, SafeEventListenerOptions, use_safe_event_listener, use_safe_event_listeners,
};
pub use use_machine::{UseMachineReturn, use_machine, use_machine_with_reactive_props};
#[cfg(feature = "ssr")]
pub use use_machine::{use_machine_hydrated, use_machine_with_reactive_props_hydrated};

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
