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
//! - `spec/foundation/11-dom-utilities.md` §10 — media query utilities
//!
//! # Re-export note
//!
//! `03-accessibility.md` §6.1 now points to `11-dom-utilities.md` §10 as the
//! canonical location for these APIs. Consumers import directly from
//! `ars_dom::media` because `ars-dom` already depends on `ars-a11y`, and an
//! `ars-a11y` re-export would create a circular dependency.

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
    fn media_queries_return_stable_defaults_without_browser() {
        let forced_colors = is_forced_colors_active();

        assert!(!forced_colors);
        assert_eq!(forced_colors, is_forced_colors_active());

        let high_contrast = prefers_high_contrast();

        assert!(!high_contrast);
        assert_eq!(high_contrast, prefers_high_contrast());

        let reduced_motion = prefers_reduced_motion();

        assert!(!reduced_motion);
        assert_eq!(reduced_motion, prefers_reduced_motion());

        let reduced_transparency = prefers_reduced_transparency();

        assert!(!reduced_transparency);
        assert_eq!(reduced_transparency, prefers_reduced_transparency());

        let color_scheme = prefers_color_scheme();

        assert_eq!(color_scheme, ColorScheme::Light);
        assert_eq!(color_scheme, prefers_color_scheme());
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
    use std::{cell::Cell, rc::Rc};

    use js_sys::{Object, Reflect};
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    struct MatchMediaGuard {
        window: web_sys::Window,
        original: JsValue,
        stub: Closure<dyn FnMut(JsValue) -> JsValue>,
    }

    impl Drop for MatchMediaGuard {
        fn drop(&mut self) {
            let result = Reflect::set(
                self.window.as_ref(),
                &JsValue::from_str("matchMedia"),
                &self.original,
            );

            assert!(result.is_ok(), "restoring window.matchMedia must succeed");

            let _ = &self.stub;
        }
    }

    struct StubbedMediaQuery {
        _guard: MatchMediaGuard,
        matches: Rc<Cell<bool>>,
        call_count: Rc<Cell<u32>>,
    }

    impl StubbedMediaQuery {
        fn install(query: &'static str, initial: bool) -> Self {
            let window = web_sys::window().expect("browser window must exist");

            let original = Reflect::get(window.as_ref(), &JsValue::from_str("matchMedia"))
                .expect("window.matchMedia must be readable");

            let matches = Rc::new(Cell::new(initial));

            let getter_state = Rc::clone(&matches);

            let descriptor = Object::new();

            let getter = Closure::<dyn FnMut() -> JsValue>::wrap(Box::new(move || {
                JsValue::from_bool(getter_state.get())
            }));

            let getter_result = Reflect::set(
                &descriptor,
                &JsValue::from_str("get"),
                getter.as_ref().unchecked_ref(),
            );

            assert!(
                getter_result.is_ok(),
                "defining the matches getter must succeed"
            );

            let configurable_result = Reflect::set(
                &descriptor,
                &JsValue::from_str("configurable"),
                &JsValue::TRUE,
            );

            assert!(
                configurable_result.is_ok(),
                "marking the matches getter configurable must succeed"
            );

            let media_query_list = Object::new();

            Object::define_property(
                &media_query_list,
                &JsValue::from_str("matches"),
                &descriptor,
            );

            getter.forget();

            let target_query = query.to_owned();

            let query_object = media_query_list.clone();

            let call_count = Rc::new(Cell::new(0));

            let call_counter = Rc::clone(&call_count);

            let stub = Closure::wrap(Box::new(move |requested_query: JsValue| -> JsValue {
                let Some(requested_query) = requested_query.as_string() else {
                    return JsValue::NULL;
                };

                if requested_query == target_query {
                    call_counter.set(call_counter.get() + 1);
                    query_object.clone().into()
                } else {
                    JsValue::NULL
                }
            }) as Box<dyn FnMut(JsValue) -> JsValue>);

            let install_result = Reflect::set(
                window.as_ref(),
                &JsValue::from_str("matchMedia"),
                stub.as_ref().unchecked_ref(),
            );

            assert!(
                install_result.is_ok(),
                "installing window.matchMedia must succeed"
            );

            Self {
                _guard: MatchMediaGuard {
                    window,
                    original,
                    stub,
                },
                matches,
                call_count,
            }
        }

        fn set_matches(&self, value: bool) {
            self.matches.set(value);
        }

        fn call_count(&self) -> u32 {
            self.call_count.get()
        }
    }

    #[wasm_bindgen_test]
    fn is_forced_colors_active_uses_live_cached_media_query_list() {
        let stub = StubbedMediaQuery::install("(forced-colors: active)", true);

        assert!(is_forced_colors_active());
        assert_eq!(stub.call_count(), 1);

        stub.set_matches(false);

        assert!(!is_forced_colors_active());
        assert_eq!(stub.call_count(), 1);
    }

    // ── prefers_high_contrast ───────────────────────────────────────────

    #[wasm_bindgen_test]
    fn prefers_high_contrast_uses_live_cached_media_query_list() {
        let stub = StubbedMediaQuery::install("(prefers-contrast: more)", true);

        assert!(prefers_high_contrast());
        assert_eq!(stub.call_count(), 1);

        stub.set_matches(false);

        assert!(!prefers_high_contrast());
        assert_eq!(stub.call_count(), 1);
    }

    // ── prefers_reduced_motion ──────────────────────────────────────────

    #[wasm_bindgen_test]
    fn prefers_reduced_motion_uses_live_cached_media_query_list() {
        let stub = StubbedMediaQuery::install("(prefers-reduced-motion: reduce)", true);

        assert!(prefers_reduced_motion());
        assert_eq!(stub.call_count(), 1);

        stub.set_matches(false);

        assert!(!prefers_reduced_motion());
        assert_eq!(stub.call_count(), 1);
    }

    // ── prefers_reduced_transparency ────────────────────────────────────

    #[wasm_bindgen_test]
    fn prefers_reduced_transparency_uses_live_cached_media_query_list() {
        let stub = StubbedMediaQuery::install("(prefers-reduced-transparency: reduce)", true);

        assert!(prefers_reduced_transparency());
        assert_eq!(stub.call_count(), 1);

        stub.set_matches(false);

        assert!(!prefers_reduced_transparency());
        assert_eq!(stub.call_count(), 1);
    }

    // ── prefers_color_scheme ────────────────────────────────────────────

    #[wasm_bindgen_test]
    fn prefers_color_scheme_uses_live_cached_media_query_list() {
        let stub = StubbedMediaQuery::install("(prefers-color-scheme: dark)", true);

        assert_eq!(prefers_color_scheme(), ColorScheme::Dark);
        assert_eq!(stub.call_count(), 1);

        stub.set_matches(false);

        assert_eq!(prefers_color_scheme(), ColorScheme::Light);
        assert_eq!(stub.call_count(), 1);
    }

    #[wasm_bindgen_test]
    fn stubbed_match_media_returns_null_for_non_string_and_unknown_queries() {
        let _stub = StubbedMediaQuery::install("(prefers-reduced-motion: reduce)", true);

        let window = web_sys::window().expect("browser window must exist");

        let match_media = Reflect::get(window.as_ref(), &JsValue::from_str("matchMedia"))
            .expect("window.matchMedia must be readable")
            .dyn_into::<js_sys::Function>()
            .expect("window.matchMedia must be callable");

        let non_string = match_media
            .call1(window.as_ref(), &JsValue::NULL)
            .expect("calling the stubbed matchMedia with null must succeed");

        assert!(non_string.is_null());

        let unknown_query = match_media
            .call1(window.as_ref(), &JsValue::from_str("(unknown-query: true)"))
            .expect("calling the stubbed matchMedia with an unknown query must succeed");

        assert!(unknown_query.is_null());
    }
}
