//! Shared provider contract types for the `ArsProvider` root context.
//!
//! [`ArsContext`] captures the framework-agnostic values adapters propagate from
//! the root provider into descendant components, including platform effects and
//! the shared instance-scoped modality context.

extern crate alloc;

use alloc::string::String;
use core::fmt;

use ars_i18n::{Direction, Locale};

use crate::{
    ArsRc, DefaultModalityContext, ModalityContext, NullPlatformEffects, PlatformEffects,
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
    platform: ArsRc<dyn PlatformEffects>,
    modality: ArsRc<dyn ModalityContext>,
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
        platform: ArsRc<dyn PlatformEffects>,
        modality: ArsRc<dyn ModalityContext>,
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
    pub fn platform(&self) -> ArsRc<dyn PlatformEffects> {
        ArsRc::clone(&self.platform)
    }

    /// Returns the shared modality context for this provider root.
    #[must_use]
    pub fn modality(&self) -> ArsRc<dyn ModalityContext> {
        ArsRc::clone(&self.modality)
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
            locale: Locale::new("en-US"),
            direction: Direction::Ltr,
            color_mode: ColorMode::System,
            disabled: false,
            read_only: false,
            id_prefix: None,
            portal_container_id: None,
            root_node_id: None,
            platform: ArsRc::from_platform(NullPlatformEffects),
            modality: ArsRc::from_modality(DefaultModalityContext::new()),
            style_strategy: StyleStrategy::Inline,
        }
    }
}

impl fmt::Debug for ArsContext {
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
            .field("style_strategy", &self.style_strategy)
            .finish()
    }
}

#[cfg(test)]
mod tests {
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

        assert_eq!(context.locale().as_str(), "en-US");
        assert_eq!(context.direction(), Direction::Ltr);
        assert_eq!(context.color_mode(), ColorMode::System);
        assert_eq!(context.modality().snapshot(), ModalitySnapshot::default());
    }

    #[test]
    fn ars_context_constructor_preserves_values() {
        let context = ArsContext::new(
            Locale::new("pt-BR"),
            Direction::Ltr,
            ColorMode::Dark,
            true,
            true,
            Some(String::from("prefix")),
            Some(String::from("portal-root")),
            Some(String::from("app-root")),
            ArsRc::from_platform(NullPlatformEffects),
            ArsRc::from_modality(NullModalityContext),
            StyleStrategy::Cssom,
        );

        assert_eq!(context.locale().as_str(), "pt-BR");
        assert!(context.disabled());
        assert!(context.read_only());
        assert_eq!(context.id_prefix(), Some("prefix"));
        assert_eq!(context.portal_container_id(), Some("portal-root"));
        assert_eq!(context.root_node_id(), Some("app-root"));
        assert_eq!(context.style_strategy(), &StyleStrategy::Cssom);
    }
}
