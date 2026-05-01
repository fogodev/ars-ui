//! Shared provider contract types for the `ArsProvider` root context.
//!
//! [`ArsContext`] captures the framework-agnostic values adapters propagate from
//! the root provider into descendant components, including platform effects and
//! the shared instance-scoped modality context.

use alloc::{string::String, sync::Arc};
use core::fmt::{self, Debug};

use ars_i18n::{Direction, IntlBackend, Locale, StubIntlBackend, locales};

use crate::{
    DefaultModalityContext, I18nRegistries, ModalityContext, NullPlatformEffects, PlatformEffects,
    StyleStrategy,
};

/// Active color mode for theme-aware rendering.
///
/// Components access this via `ArsProvider` context. The `System` variant defers
/// to the user's OS-level preference (for example `prefers-color-scheme` on the web).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorMode {
    /// Light color scheme.
    Light,

    /// Dark color scheme.
    Dark,

    /// Inherit from the operating system preference.
    #[default]
    System,
}

/// Framework-agnostic provider context shared with descendant components.
///
/// This is the canonical provider contract shared across adapters. Framework
/// crates wrap these values in reactive signals, but the field set itself stays
/// consistent so runtime-facing hooks and environment resolution follow the same
/// semantics everywhere.
#[derive(Clone)]
pub struct ArsContext {
    locale: Locale,
    direction: Direction,
    color_mode: ColorMode,
    disabled: bool,
    read_only: bool,
    id_prefix: Option<String>,
    portal_container_id: Option<String>,
    root_node_id: Option<String>,
    platform: Arc<dyn PlatformEffects>,
    modality: Arc<dyn ModalityContext>,
    intl_backend: Arc<dyn IntlBackend>,
    i18n_registries: Arc<I18nRegistries>,
    style_strategy: StyleStrategy,
}

impl ArsContext {
    /// Creates a new provider context from explicitly supplied values.
    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "ArsContext intentionally mirrors root-provider context fields."
    )]
    pub fn new(
        locale: Locale,
        direction: Direction,
        color_mode: ColorMode,
        disabled: bool,
        read_only: bool,
        id_prefix: Option<String>,
        portal_container_id: Option<String>,
        root_node_id: Option<String>,
        platform: Arc<dyn PlatformEffects>,
        modality: Arc<dyn ModalityContext>,
        intl_backend: Arc<dyn IntlBackend>,
        i18n_registries: Arc<I18nRegistries>,
        style_strategy: StyleStrategy,
    ) -> Self {
        Self {
            locale,
            direction,
            color_mode,
            disabled,
            read_only,
            id_prefix,
            portal_container_id,
            root_node_id,
            platform,
            modality,
            intl_backend,
            i18n_registries,
            style_strategy,
        }
    }

    /// Returns the active locale.
    #[must_use]
    pub const fn locale(&self) -> &Locale {
        &self.locale
    }

    /// Returns the reading direction.
    #[must_use]
    pub const fn direction(&self) -> Direction {
        self.direction
    }

    /// Returns the active color mode.
    #[must_use]
    pub const fn color_mode(&self) -> ColorMode {
        self.color_mode
    }

    /// Returns whether descendants should render as disabled.
    #[must_use]
    pub const fn disabled(&self) -> bool {
        self.disabled
    }

    /// Returns whether descendants should render as read-only.
    #[must_use]
    pub const fn read_only(&self) -> bool {
        self.read_only
    }

    /// Returns the optional generated-ID prefix.
    #[must_use]
    pub fn id_prefix(&self) -> Option<&str> {
        self.id_prefix.as_deref()
    }

    /// Returns the optional portal container element ID.
    #[must_use]
    pub fn portal_container_id(&self) -> Option<&str> {
        self.portal_container_id.as_deref()
    }

    /// Returns the optional scoped root-node ID.
    #[must_use]
    pub fn root_node_id(&self) -> Option<&str> {
        self.root_node_id.as_deref()
    }

    /// Returns the shared platform-effects handle.
    #[must_use]
    pub fn platform(&self) -> Arc<dyn PlatformEffects> {
        Arc::clone(&self.platform)
    }

    /// Returns the shared modality context for this provider root.
    #[must_use]
    pub fn modality(&self) -> Arc<dyn ModalityContext> {
        Arc::clone(&self.modality)
    }

    /// Returns the shared ICU provider for this provider root.
    #[must_use]
    pub fn intl_backend(&self) -> Arc<dyn IntlBackend> {
        Arc::clone(&self.intl_backend)
    }

    /// Returns the shared per-component message registries.
    #[must_use]
    pub fn i18n_registries(&self) -> Arc<I18nRegistries> {
        Arc::clone(&self.i18n_registries)
    }

    /// Returns the active style strategy.
    #[must_use]
    pub const fn style_strategy(&self) -> &StyleStrategy {
        &self.style_strategy
    }
}

impl Default for ArsContext {
    fn default() -> Self {
        Self {
            locale: locales::en_us(),
            direction: Direction::Ltr,
            color_mode: ColorMode::System,
            disabled: false,
            read_only: false,
            id_prefix: None,
            portal_container_id: None,
            root_node_id: None,
            platform: Arc::new(NullPlatformEffects),
            modality: Arc::new(DefaultModalityContext::new()),
            intl_backend: Arc::new(StubIntlBackend),
            i18n_registries: Arc::new(I18nRegistries::new()),
            style_strategy: StyleStrategy::Inline,
        }
    }
}

impl Debug for ArsContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArsContext")
            .field("locale", &self.locale)
            .field("direction", &self.direction)
            .field("color_mode", &self.color_mode)
            .field("disabled", &self.disabled)
            .field("read_only", &self.read_only)
            .field("id_prefix", &self.id_prefix)
            .field("portal_container_id", &self.portal_container_id)
            .field("root_node_id", &self.root_node_id)
            .field("platform", &"<dyn PlatformEffects>")
            .field("modality", &"<dyn ModalityContext>")
            .field("intl_backend", &"<dyn IntlBackend>")
            .field("i18n_registries", &self.i18n_registries)
            .field("style_strategy", &self.style_strategy)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;
    use crate::{ModalitySnapshot, NullModalityContext};

    #[test]
    fn color_mode_default_is_system() {
        assert_eq!(ColorMode::default(), ColorMode::System);
    }

    #[test]
    fn color_mode_variants_are_distinct() {
        assert_ne!(ColorMode::Light, ColorMode::Dark);
        assert_ne!(ColorMode::Light, ColorMode::System);
        assert_ne!(ColorMode::Dark, ColorMode::System);
    }

    #[test]
    fn color_mode_is_copy() {
        let a = ColorMode::Dark;

        let b = a;

        assert_eq!(a, b);
    }

    #[test]
    fn ars_context_default_uses_default_modality_context() {
        let context = ArsContext::default();

        assert_eq!(context.locale().to_bcp47(), "en-US");
        assert_eq!(context.direction(), Direction::Ltr);
        assert_eq!(context.color_mode(), ColorMode::System);
        assert_eq!(context.modality().snapshot(), ModalitySnapshot::default());
    }

    #[test]
    fn ars_context_constructor_preserves_values() {
        let platform: Arc<dyn PlatformEffects> = Arc::new(NullPlatformEffects);

        let modality: Arc<dyn ModalityContext> = Arc::new(NullModalityContext);

        let intl_backend: Arc<dyn IntlBackend> = Arc::new(StubIntlBackend);

        let i18n_registries = Arc::new(I18nRegistries::new());

        let context = ArsContext::new(
            locales::br(),
            Direction::Ltr,
            ColorMode::Dark,
            true,
            true,
            Some(String::from("prefix")),
            Some(String::from("portal-root")),
            Some(String::from("app-root")),
            Arc::clone(&platform),
            Arc::clone(&modality),
            Arc::clone(&intl_backend),
            Arc::clone(&i18n_registries),
            StyleStrategy::Cssom,
        );

        assert_eq!(context.locale().to_bcp47(), "pt-BR");
        assert!(context.disabled());
        assert!(context.read_only());
        assert_eq!(context.id_prefix(), Some("prefix"));
        assert_eq!(context.portal_container_id(), Some("portal-root"));
        assert_eq!(context.root_node_id(), Some("app-root"));
        assert!(Arc::ptr_eq(&context.platform(), &platform));
        assert!(Arc::ptr_eq(&context.modality(), &modality));
        assert!(Arc::ptr_eq(&context.intl_backend(), &intl_backend));
        assert!(Arc::ptr_eq(&context.i18n_registries(), &i18n_registries));
        assert_eq!(context.style_strategy(), &StyleStrategy::Cssom);
    }

    #[test]
    fn ars_context_default_exposes_default_optional_and_platform_values() {
        let context = ArsContext::default();

        assert_eq!(context.id_prefix(), None);
        assert_eq!(context.portal_container_id(), None);
        assert_eq!(context.root_node_id(), None);
        assert!(!context.disabled());
        assert!(!context.read_only());
        assert_eq!(context.style_strategy(), &StyleStrategy::Inline);
        assert_eq!(context.platform().now_ms(), 0);
        assert_eq!(context.modality().snapshot(), ModalitySnapshot::default());
        let registries = context.i18n_registries();
        assert_eq!(Arc::strong_count(&registries), 2);
    }

    #[test]
    fn ars_context_debug_lists_public_fields() {
        let context = ArsContext::default();

        let debug = format!("{context:?}");

        assert!(debug.contains("ArsContext"));
        assert!(debug.contains("locale"));
        assert!(debug.contains("direction"));
        assert!(debug.contains("color_mode"));
        assert!(debug.contains("platform"));
        assert!(debug.contains("modality"));
        assert!(debug.contains("style_strategy"));
    }
}
