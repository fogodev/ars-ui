//! Media query detection utilities for accessibility and user preferences.
//!
//! These functions detect user-configured accessibility and preference settings
//! via CSS media queries. They live in `ars-dom` (not `ars-a11y`) because they
//! depend on `web_sys::window()` which requires std and web-sys.
//!
//! # Caching
//!
//! Each function caches the [`web_sys::MediaQueryList`] object in a
//! `thread_local!` [`OnceCell`] on first access. Subsequent calls read
//! [`MediaQueryList::matches()`], which is a live property that always reflects
//! the current state — no explicit `change` event listener is needed. This
//! avoids the cost of re-parsing the query string on every call while still
//! tracking runtime changes (e.g., user toggles High Contrast Mode).
//!
//! # Platform behavior
//!
//! - **wasm32 with `web` feature**: Queries `window.matchMedia()` on first
//!   access, caches the `MediaQueryList`, and reads `.matches()` on each call.
//!   Returns `false` when `window()` is unavailable (SSR, Web Worker).
//! - **Non-wasm or without `web` feature**: Always returns `false` (or
//!   [`ColorScheme::Light`]). These are browser-only concepts with no meaningful
//!   server-side equivalent.
//!
//! # Spec references
//!
//! - `spec/foundation/05-interactions.md` §10 — forced-colors interaction styling
//! - `spec/foundation/03-accessibility.md` §6.1 — high contrast and forced colors
//! - `spec/foundation/11-dom-utilities.md` §9 — media query utilities
//!
//! # Re-export note
//!
//! The spec (`03-accessibility.md` §6.1) says these functions should be
//! re-exported by `ars-a11y` behind `#[cfg(feature = "dom")]`. However,
//! `ars-dom` already depends on `ars-a11y`, so adding a back-dependency would
//! create a circular dependency. Consumers import directly from
//! `ars_dom::media` instead. The spec should be updated to reflect this.

// ── Cached matchMedia infrastructure (wasm32 + web only) ────────────────

#[cfg(all(feature = "web", target_arch = "wasm32"))]
use std::{cell::OnceCell, thread};

#[cfg(all(feature = "web", target_arch = "wasm32"))]
thread_local! {
    static FORCED_COLORS_MQL: OnceCell<Option<web_sys::MediaQueryList>> = const { OnceCell::new() };

    static HIGH_CONTRAST_MQL: OnceCell<Option<web_sys::MediaQueryList>> = const { OnceCell::new() };

    static REDUCED_MOTION_MQL: OnceCell<Option<web_sys::MediaQueryList>> = const { OnceCell::new() };

    static REDUCED_TRANSPARENCY_MQL: OnceCell<Option<web_sys::MediaQueryList>> = const { OnceCell::new() };

    static COLOR_SCHEME_MQL: OnceCell<Option<web_sys::MediaQueryList>> = const { OnceCell::new() };
}

/// Cache the [`web_sys::MediaQueryList`] object on first access and read
/// [`.matches()`](web_sys::MediaQueryList::matches) on each call. Caching the
/// object avoids re-parsing the query string (the expensive part);
/// `.matches()` always returns the live current state, so no explicit `change`
/// event listener is needed.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn cached_matches(
    cache: &'static thread::LocalKey<OnceCell<Option<web_sys::MediaQueryList>>>,
    query: &str,
) -> bool {
    cache.with(|cell| {
        cell.get_or_init(|| web_sys::window().and_then(|w| w.match_media(query).ok().flatten()))
            .as_ref()
            .is_some_and(web_sys::MediaQueryList::matches)
    })
}

// ── Public API ──────────────────────────────────────────────────────────

/// Detects if the user has enabled forced colors (Windows High Contrast Mode).
///
/// Returns `true` when `@media (forced-colors: active)` matches. This value
/// CAN change at runtime (user can toggle via Win+U or Settings). Components
/// that apply inline styles for visual feedback (e.g., drag preview opacity,
/// hover background) SHOULD check this function and skip custom color overrides
/// when forced colors are active, allowing the system colors to take effect.
///
/// Returns `false` on non-wasm targets and when `window()` is unavailable
/// (SSR, Web Worker).
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn is_forced_colors_active() -> bool {
    cached_matches(&FORCED_COLORS_MQL, "(forced-colors: active)")
}

/// Detects if the user has enabled forced colors (Windows High Contrast Mode).
///
/// Returns `false` on non-wasm targets — forced colors is a browser-only concept.
#[cfg(not(all(feature = "web", target_arch = "wasm32")))]
pub const fn is_forced_colors_active() -> bool {
    false
}

/// Check whether the user prefers increased contrast.
///
/// Returns `true` when `@media (prefers-contrast: more)` matches. This is
/// distinct from forced-colors mode — a user can prefer high contrast without
/// enabling system forced colors.
///
/// Components SHOULD use CSS custom properties (`--ars-focus-ring-width`,
/// `--ars-border-width`) so that `prefers-contrast: more` can widen focus
/// indicators and borders without JavaScript.
///
/// Returns `false` on non-wasm targets and when `window()` is unavailable
/// (SSR, Web Worker).
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn prefers_high_contrast() -> bool {
    cached_matches(&HIGH_CONTRAST_MQL, "(prefers-contrast: more)")
}

/// Check whether the user prefers increased contrast.
///
/// Returns `false` on non-wasm targets — contrast preference is a browser-only
/// concept.
#[cfg(not(all(feature = "web", target_arch = "wasm32")))]
pub const fn prefers_high_contrast() -> bool {
    false
}

/// Detects if the user prefers reduced motion.
///
/// Returns `true` when `@media (prefers-reduced-motion: reduce)` matches.
/// Components with transitions, animations, or auto-playing content SHOULD
/// check this and suppress motion effects when true.
///
/// Returns `false` on non-wasm targets and when `window()` is unavailable
/// (SSR, Web Worker).
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn prefers_reduced_motion() -> bool {
    cached_matches(&REDUCED_MOTION_MQL, "(prefers-reduced-motion: reduce)")
}

/// Detects if the user prefers reduced motion.
///
/// Returns `false` on non-wasm targets — motion preference is a browser-only
/// concept.
#[cfg(not(all(feature = "web", target_arch = "wasm32")))]
pub const fn prefers_reduced_motion() -> bool {
    false
}

/// Detects if the user prefers a reduced transparency level.
///
/// Returns `true` when `@media (prefers-reduced-transparency: reduce)` matches.
/// Components that use `backdrop-filter` or translucent backgrounds SHOULD
/// disable those effects when true.
///
/// Returns `false` on non-wasm targets and when `window()` is unavailable
/// (SSR, Web Worker).
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn prefers_reduced_transparency() -> bool {
    cached_matches(
        &REDUCED_TRANSPARENCY_MQL,
        "(prefers-reduced-transparency: reduce)",
    )
}

/// Detects if the user prefers a reduced transparency level.
///
/// Returns `false` on non-wasm targets — transparency preference is a
/// browser-only concept.
#[cfg(not(all(feature = "web", target_arch = "wasm32")))]
pub const fn prefers_reduced_transparency() -> bool {
    false
}

/// Detects system color scheme preference.
///
/// Returns [`ColorScheme::Dark`] when `@media (prefers-color-scheme: dark)`
/// matches, otherwise [`ColorScheme::Light`].
///
/// Returns [`ColorScheme::Light`] on non-wasm targets and when `window()` is
/// unavailable (SSR, Web Worker).
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn prefers_color_scheme() -> ColorScheme {
    if cached_matches(&COLOR_SCHEME_MQL, "(prefers-color-scheme: dark)") {
        ColorScheme::Dark
    } else {
        ColorScheme::Light
    }
}

/// Detects system color scheme preference.
///
/// Returns [`ColorScheme::Light`] on non-wasm targets — color scheme preference
/// is a browser-only concept.
#[cfg(not(all(feature = "web", target_arch = "wasm32")))]
pub const fn prefers_color_scheme() -> ColorScheme {
    ColorScheme::Light
}

/// System color scheme preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorScheme {
    /// Light color scheme (default when preference is unknown).
    Light,
    /// Dark color scheme.
    Dark,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_forced_colors_active_returns_false_without_browser() {
        assert!(!is_forced_colors_active());
    }

    #[test]
    fn prefers_high_contrast_returns_false_without_browser() {
        assert!(!prefers_high_contrast());
    }

    #[test]
    fn prefers_reduced_motion_returns_false_without_browser() {
        assert!(!prefers_reduced_motion());
    }

    #[test]
    fn prefers_reduced_transparency_returns_false_without_browser() {
        assert!(!prefers_reduced_transparency());
    }

    #[test]
    fn prefers_color_scheme_returns_light_without_browser() {
        assert_eq!(prefers_color_scheme(), ColorScheme::Light);
    }

    #[test]
    fn color_scheme_equality() {
        assert_eq!(ColorScheme::Light, ColorScheme::Light);
        assert_eq!(ColorScheme::Dark, ColorScheme::Dark);
        assert_ne!(ColorScheme::Light, ColorScheme::Dark);
    }

    #[test]
    fn color_scheme_clone_and_copy() {
        let scheme = ColorScheme::Dark;
        #[expect(clippy::clone_on_copy, reason = "explicitly testing Clone impl")]
        let cloned = scheme.clone();
        let copied = scheme;
        assert_eq!(scheme, cloned);
        assert_eq!(scheme, copied);
    }

    #[test]
    fn color_scheme_debug() {
        assert_eq!(format!("{:?}", ColorScheme::Light), "Light");
        assert_eq!(format!("{:?}", ColorScheme::Dark), "Dark");
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    // ── is_forced_colors_active ─────────────────────────────────────────

    #[wasm_bindgen_test]
    fn is_forced_colors_active_returns_bool_in_browser() {
        // In a standard browser environment, forced colors is typically inactive.
        // The important thing is that the function executes without panic and
        // exercises the cached_matches → window().match_media() → .matches() path.
        let result = is_forced_colors_active();
        // Standard test browsers don't have forced colors enabled.
        assert!(!result);
    }

    #[wasm_bindgen_test]
    fn is_forced_colors_active_caches_across_calls() {
        // Calling twice exercises the OnceCell cache-hit path.
        let first = is_forced_colors_active();
        let second = is_forced_colors_active();
        assert_eq!(first, second);
    }

    // ── prefers_high_contrast ───────────────────────────────────────────

    #[wasm_bindgen_test]
    fn prefers_high_contrast_returns_bool_in_browser() {
        let result = prefers_high_contrast();
        // Standard test browsers don't have high contrast preference.
        assert!(!result);
    }

    #[wasm_bindgen_test]
    fn prefers_high_contrast_caches_across_calls() {
        let first = prefers_high_contrast();
        let second = prefers_high_contrast();
        assert_eq!(first, second);
    }

    // ── prefers_reduced_motion ──────────────────────────────────────────

    #[wasm_bindgen_test]
    fn prefers_reduced_motion_returns_bool_in_browser() {
        // Result depends on browser/OS settings; we just verify no panic.
        let _result = prefers_reduced_motion();
    }

    #[wasm_bindgen_test]
    fn prefers_reduced_motion_caches_across_calls() {
        let first = prefers_reduced_motion();
        let second = prefers_reduced_motion();
        assert_eq!(first, second);
    }

    // ── prefers_reduced_transparency ────────────────────────────────────

    #[wasm_bindgen_test]
    fn prefers_reduced_transparency_returns_bool_in_browser() {
        let result = prefers_reduced_transparency();
        // Standard test browsers don't have reduced transparency preference.
        assert!(!result);
    }

    #[wasm_bindgen_test]
    fn prefers_reduced_transparency_caches_across_calls() {
        let first = prefers_reduced_transparency();
        let second = prefers_reduced_transparency();
        assert_eq!(first, second);
    }

    // ── prefers_color_scheme ────────────────────────────────────────────

    #[wasm_bindgen_test]
    fn prefers_color_scheme_returns_valid_variant_in_browser() {
        let scheme = prefers_color_scheme();
        // The result is either Light or Dark — both are valid.
        assert!(scheme == ColorScheme::Light || scheme == ColorScheme::Dark);
    }

    #[wasm_bindgen_test]
    fn prefers_color_scheme_caches_across_calls() {
        let first = prefers_color_scheme();
        let second = prefers_color_scheme();
        assert_eq!(first, second);
    }
}
