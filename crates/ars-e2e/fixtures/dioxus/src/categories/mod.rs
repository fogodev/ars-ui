//! Per-category fixture modules.
//!
//! Each category submodule owns its own `…Text` enum, panel component,
//! per-component helpers, and message-registry registration. The aggregator
//! [`i18n_registries`] composes the per-category registrations into the
//! single `I18nRegistries` that the fixture's `ArsProvider` consumes.
//!
//! Add a new category by:
//!
//! 1. Creating `categories/<category>.rs` with its own `…Text` enum, panel
//!    component, and `pub(crate) fn register_messages(&mut I18nRegistries)`
//!    function.
//! 2. Declaring `pub mod <category>;` and calling its `register_messages` in
//!    this file.
//! 3. Adding the matching variant to `CategoryTab` and the `Tabs` row in
//!    `main.rs`.
//!
//! Parallel agents implementing different categories edit different files
//! and never collide on the same component module.

use std::sync::Arc;

use ars_dioxus::I18nRegistries;

pub mod navigation;
pub mod utility;

/// Builds the per-fixture [`I18nRegistries`] by delegating to each category
/// module's `register_messages` hook.
#[must_use]
pub fn i18n_registries() -> Arc<I18nRegistries> {
    let mut registries = I18nRegistries::new();

    navigation::register_messages(&mut registries);
    utility::register_messages(&mut registries);

    Arc::new(registries)
}
