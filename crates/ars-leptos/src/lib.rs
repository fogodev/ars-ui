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

extern crate self as ars_leptos;

/// Hidden re-exports used by proc macros that resolve through this adapter
/// facade.
#[doc(hidden)]
pub mod __private {
    pub use ars_i18n::__private::*;
}

pub mod as_child;
mod attrs;
mod callbacks;
mod controlled;
mod ephemeral;
mod event_mapping;
mod hydration;
mod id;
pub mod navigation;
mod nonce;
pub mod prelude;
mod provider;
mod safe_listener;
mod use_machine;
pub mod utility;

#[cfg(feature = "uuid")]
pub use ars_collections::uuid;
pub use ars_collections::{Key, TabKey};
pub use ars_core::{I18nRegistries, MessageFn, MessagesRegistry};
pub use ars_i18n::{IntlBackend, Locale, Translate};
#[cfg(not(feature = "ssr"))]
pub use attrs::{
    CssomStyleHandle, apply_styles_cssom, use_cssom_styles, use_cssom_styles_from_attrs,
};
pub use attrs::{
    LeptosAttrResult, LeptosAttribute, attr_map_to_leptos, attr_map_to_leptos_inline_attrs,
    consumer_style_prop_to_leptos_attr, merge_consumer_class_prop_into, use_style_strategy,
};
pub use callbacks::{call, emit, emit_map};
pub use controlled::use_controlled_prop;
pub use ephemeral::EphemeralRef;
pub use event_mapping::leptos_key_to_keyboard_key;
#[cfg(any(feature = "ssr", all(feature = "hydrate", target_arch = "wasm32")))]
pub use hydration::HydrationSnapshot;
#[cfg(feature = "ssr")]
pub use hydration::serialize_snapshot;
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
    ArsContext, ArsProvider, Translatable, provide_ars_context, resolve_locale, t, use_direction,
    use_intl_backend, use_locale, use_messages_and_locale, use_modality_context,
    use_number_formatter, use_platform_effects, warn_missing_provider,
};
pub use reactive_stores;
pub use safe_listener::{
    SafeEventListener, SafeEventListenerOptions, use_safe_event_listener, use_safe_event_listeners,
};
pub use use_machine::{UseMachineReturn, use_machine, use_machine_with_reactive_props};
#[cfg(any(feature = "ssr", all(feature = "hydrate", target_arch = "wasm32")))]
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

#[cfg(test)]
mod tests {
    #[test]
    fn reactive_stores_is_reexported_for_tabs_consumers() {
        use crate::reactive_stores as adapter_reactive_stores;

        let _ = core::any::type_name::<adapter_reactive_stores::Store<()>>();
    }
}
