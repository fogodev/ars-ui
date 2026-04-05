//! Shared provider contract types for the `ArsProvider` root context.
//!
//! [`ArsProvider`](crate) is the single root provider that supplies shared configuration,
//! platform capabilities, i18n resources, and style strategy to all descendant components.
//! This module defines the framework-agnostic types used across all adapters.

/// Active color mode for theme-aware rendering.
///
/// Components access this via `ArsProvider` context. The `System` variant defers
/// to the user's OS-level preference (e.g., `prefers-color-scheme` on the web).
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let b = a; // Copy
        assert_eq!(a, b);
    }
}
