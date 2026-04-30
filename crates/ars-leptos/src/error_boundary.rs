//! Error boundary wrapper.
//!
//! [`Boundary`] wraps a subtree in Leptos's [`ErrorBoundary`] and renders the
//! canonical accessible fallback (`<div role="alert">` with a localized
//! heading and `<ul>`/`<li>` error list) defined in
//! `spec/components/utility/error-boundary.md`. It composes around the
//! framework primitive: locale resolution, message bundle resolution, and
//! adapter-symmetric DOM output that matches the Dioxus adapter
//! byte-for-byte.
//!
//! See `spec/components/utility/error-boundary.md` and
//! `spec/leptos-components/utility/error-boundary.md` for the full
//! specification.

use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

pub use ars_components::utility::error_boundary::{Api, Messages, Part};
use ars_i18n::Locale;
pub use leptos::error::Error as CapturedError;
use leptos::{error::ErrorId, prelude::*, tachys::view::any_view::AnyView};

use crate::{
    attrs::attr_map_to_leptos_inline_attrs,
    provider::{resolve_locale, use_messages},
};

// ────────────────────────────────────────────────────────────────────
// FallbackHandler
// ────────────────────────────────────────────────────────────────────

/// Adapter-side fallback handler.
///
/// A thin alias over [`Callback<ArcRwSignal<Errors>, AnyView>`] so consumers
/// can pass a closure directly to [`Boundary`]'s `fallback` prop without
/// spelling out the generic parameters. The closure receives Leptos's
/// reactive multi-error signal and returns any view as a type-erased
/// [`AnyView`].
pub type FallbackHandler = Callback<ArcRwSignal<Errors>, AnyView>;

// ────────────────────────────────────────────────────────────────────
// Boundary
// ────────────────────────────────────────────────────────────────────

/// Wrapper around Leptos's [`ErrorBoundary`] that renders an accessible
/// fallback when a descendant component returns an error.
///
/// The fallback is a `<div role="alert" data-ars-error="true">` containing
/// a localized heading paragraph and a `<ul>` of `<li>` error entries —
/// matching the Dioxus adapter byte-for-byte. Optional props expose a
/// custom fallback override, an `on_error` telemetry hook, and a
/// `messages` bundle override.
///
/// # Heading capture
///
/// The heading string is resolved **once per `Boundary` render** (via
/// `use_messages` + `resolve_locale`) and captured by-value into the
/// fallback closure. If the surrounding `ArsProvider`'s locale signal
/// changes between an error being caught and the fallback re-rendering
/// without an outer re-render, the captured heading is stale until the
/// `Boundary` itself re-renders. In practice this is invisible: locale
/// changes typically swap the entire route subtree, which triggers a
/// new `Boundary` render. Apps that need synchronous heading reactivity
/// (e.g. a locale picker that mutates the signal in-place while the
/// fallback is on screen) should pass a `messages` prop wrapped in a
/// reactive view or render their own custom `fallback`.
///
/// See `spec/leptos-components/utility/error-boundary.md` for the full
/// adapter contract.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Leptos `#[component]` props must be owned; `messages` is passed by reference \
              into `use_messages` once, but the macro signature requires it by value."
)]
pub fn Boundary(
    /// Optional override for the entire fallback closure. When `None`,
    /// the wrapper renders the canonical accessible default markup.
    #[prop(optional, into)]
    fallback: Option<FallbackHandler>,

    /// Optional telemetry hook fired once for each newly captured error
    /// episode. Multi-error episodes fire the callback once per distinct
    /// `(ErrorId, Error)` pair, without replaying already-seen entries when
    /// the fallback re-renders.
    #[prop(optional, into)]
    on_error: Option<Callback<CapturedError>>,

    /// Override the default [`Messages`] bundle. When `None`, the wrapper
    /// resolves the bundle from `ArsProvider`'s `i18n_registries` for
    /// the active locale, falling back to [`Messages::default`].
    #[prop(optional)]
    messages: Option<Messages>,

    /// Subtree wrapped by the boundary.
    children: Children,
) -> impl IntoView {
    let resolved_locale = resolve_locale(None);

    let resolved_messages = use_messages(messages.as_ref(), Some(&resolved_locale));

    let heading = (resolved_messages.message)(&resolved_locale);

    let seen_error_ids = Arc::new(Mutex::new(HashSet::new()));

    // The fallback closure must be `FnMut + Send + 'static`; capture the
    // resolved heading + optional callbacks by value and dispatch through
    // `run_fallback` so the on-error/custom-fallback/default-fallback
    // branching logic stays unit-testable in isolation from the
    // framework primitive.
    let fallback_closure = move |errors: ArcRwSignal<Errors>| -> AnyView {
        run_fallback(errors, on_error, fallback, heading.clone(), &seen_error_ids)
    };

    view! {
        <ErrorBoundary fallback=fallback_closure>
            {children()}
        </ErrorBoundary>
    }
}

/// Internal dispatch helper for the [`Boundary`] fallback closure.
///
/// Extracted so the on-error iteration, custom-fallback delegation, and
/// default-fallback fall-through paths can be unit-tested without going
/// through the framework primitive (whose SSR pass collapses multi-error
/// payloads to the most-recent entry, hiding the loop's per-error
/// behaviour).
fn run_fallback(
    errors: ArcRwSignal<Errors>,
    on_error: Option<Callback<CapturedError>>,
    fallback: Option<FallbackHandler>,
    heading: String,
    seen_error_ids: &Mutex<HashSet<ErrorId>>,
) -> AnyView {
    if let Some(handler) = on_error {
        let snapshot = errors.get().into_iter().collect::<Vec<_>>();

        let mut seen = seen_error_ids.lock().expect("lock seen error ids");

        for (id, error) in snapshot {
            if seen.insert(id) {
                handler.run(error.clone());
            }
        }
    }

    if let Some(custom) = fallback {
        custom.run(errors)
    } else {
        render_default_fallback(errors, heading).into_any()
    }
}

// ────────────────────────────────────────────────────────────────────
// default_fallback
// ────────────────────────────────────────────────────────────────────

/// Renders the canonical accessible fallback markup using English defaults.
///
/// Use this when consuming Leptos's [`ErrorBoundary`] directly without
/// the [`Boundary`] wrapper:
///
/// ```ignore
/// view! {
///     <ErrorBoundary fallback=ars_leptos::error_boundary::default_fallback>
///         <ChildComponent/>
///     </ErrorBoundary>
/// }
/// ```
///
/// For localized headings, use [`Boundary`] which resolves [`Messages`]
/// from the surrounding `ArsProvider`. This standalone function does not
/// read any reactive context (it cannot — it is a plain function, not a
/// component) and always falls back to English.
#[must_use]
pub fn default_fallback(errors: ArcRwSignal<Errors>) -> AnyView {
    let messages = Messages::default();

    let locale = en_us_locale();

    let heading = (messages.message)(&locale);

    render_default_fallback(errors, heading).into_any()
}

fn en_us_locale() -> Locale {
    Locale::parse("en-US").expect("'en-US' is always a valid BCP-47 locale")
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Leptos's `ErrorBoundary` invokes the fallback closure with an owned \
              `ArcRwSignal<Errors>`; matching that signature lets `default_fallback` be \
              passed directly as the fallback prop without an adapter closure."
)]
fn render_default_fallback(errors: ArcRwSignal<Errors>, heading: String) -> impl IntoView {
    // Snapshot once for both the count attribute and the iteration so the
    // rendered `data-ars-error-count` matches the rendered list length.
    let snapshot = errors.get().into_iter().collect::<Vec<_>>();

    let api = Api::new(snapshot.len());

    let root_attrs = attr_map_to_leptos_inline_attrs(api.root_attrs());
    let message_attrs = attr_map_to_leptos_inline_attrs(api.message_attrs());
    let list_attrs = attr_map_to_leptos_inline_attrs(api.list_attrs());

    view! {
        <div {..root_attrs}>
            <p {..message_attrs}>{heading}</p>
            <ul {..list_attrs}>
                {snapshot.into_iter()
                    .map(move |(_, e)| view! {
                        <li {..attr_map_to_leptos_inline_attrs(api.item_attrs())}>{e.to_string()}</li>
                    })
                    .collect_view()
                }
            </ul>
        </div>
    }
}

// ────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use std::{
        fmt::{self, Display},
        sync::{Arc, Mutex},
    };

    use leptos::{error::ErrorId, reactive::owner::Owner};

    use super::*;

    /// `BoomError` is `std::error::Error + Send + Sync`, the minimum
    /// needed to be inserted into Leptos's `Errors` collection.
    #[derive(Debug)]
    struct BoomError(&'static str);

    impl Display for BoomError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }

    impl std::error::Error for BoomError {}

    /// `run_fallback` invokes `on_error` once per `(ErrorId, Error)` entry
    /// in the signal, in iteration order. SSR cannot exercise this loop
    /// through real children (the framework collapses sibling errors to
    /// the most-recent one before the fallback fires), so we drive
    /// `run_fallback` directly with a pre-populated `Errors` signal.
    #[test]
    fn run_fallback_fires_on_error_once_per_entry() {
        let owner = Owner::new();
        owner.with(|| {
            let mut errors = Errors::default();

            errors.insert(ErrorId::from(1usize), BoomError("first"));
            errors.insert(ErrorId::from(2usize), BoomError("second"));
            errors.insert(ErrorId::from(3usize), BoomError("third"));

            let captured: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
            let captured_for_cb = Arc::clone(&captured);

            let on_error = Callback::new(move |err: CapturedError| {
                captured_for_cb.lock().expect("lock").push(err.to_string());
            });

            let _view = run_fallback(
                ArcRwSignal::new(errors),
                Some(on_error),
                None,
                "ignored heading".to_string(),
                &Mutex::new(HashSet::new()),
            );

            let captured = captured.lock().expect("lock");

            assert_eq!(
                captured.len(),
                3,
                "on_error must fire once per error; got {captured:?}"
            );

            for label in ["first", "second", "third"] {
                assert!(
                    captured.iter().any(|m| m.contains(label)),
                    "missing {label} in captured: {captured:?}"
                );
            }
        });
    }

    /// When `on_error` is `None`, `run_fallback` skips the iteration
    /// entirely and still produces a valid view. Pins the optional-prop
    /// branch.
    #[test]
    fn run_fallback_with_no_telemetry_skips_iteration() {
        let owner = Owner::new();
        owner.with(|| {
            let mut errors = Errors::default();

            errors.insert(ErrorId::from(0usize), BoomError("ignored"));

            // The mere fact that this returns without panicking proves the
            // None branch is exercised; the view itself is opaque AnyView.
            let _view = run_fallback(
                ArcRwSignal::new(errors),
                None,
                None,
                "heading".to_string(),
                &Mutex::new(HashSet::new()),
            );
        });
    }

    /// K: the telemetry callback receives a clone of each `CapturedError`,
    /// not a move. After `on_error` fires for every entry, the signal must
    /// still hold the same number of errors so the fallback renderer can
    /// iterate the snapshot a second time without seeing an emptied
    /// collection. Pins the clone-vs-take semantics that Leptos's reactive
    /// recovery (clear-and-retry) depends on.
    #[test]
    fn run_fallback_iterates_without_draining_signal() {
        let owner = Owner::new();
        owner.with(|| {
            let captured_count: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));

            let captured_for_cb = Arc::clone(&captured_count);
            let on_error: Callback<CapturedError> = Callback::new(move |_err: CapturedError| {
                *captured_for_cb.lock().expect("lock") += 1;
            });

            let mut errs = Errors::default();

            errs.insert(ErrorId::from(1usize), BoomError("first"));
            errs.insert(ErrorId::from(2usize), BoomError("second"));

            let signal = ArcRwSignal::new(errs);

            let _view = run_fallback(
                signal.clone(),
                Some(on_error),
                None,
                "heading".to_string(),
                &Mutex::new(HashSet::new()),
            );

            // on_error fired once per entry — proves the loop ran.
            assert_eq!(*captured_count.lock().expect("lock"), 2);

            // The signal's contents are untouched — the helper read by
            // value but cloned each Error, so the original collection is
            // still drainable.
            let remaining = signal.with(|errs| errs.iter().count());

            assert_eq!(
                remaining, 2,
                "signal must still hold 2 errors after on_error iteration; \
                 clone semantics regression"
            );
        });
    }

    /// Re-rendering the fallback with the same `Errors` signal must not
    /// replay telemetry for already-seen `ErrorId`s. This pins the adapter
    /// contract that `on_error` is per error episode, not per fallback render.
    #[test]
    fn run_fallback_deduplicates_telemetry_across_rerenders() {
        let owner = Owner::new();
        owner.with(|| {
            let captured: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
            let captured_for_cb = Arc::clone(&captured);
            let on_error = Callback::new(move |err: CapturedError| {
                captured_for_cb.lock().expect("lock").push(err.to_string());
            });

            let mut errs = Errors::default();

            errs.insert(ErrorId::from(1usize), BoomError("first"));
            errs.insert(ErrorId::from(2usize), BoomError("second"));

            let signal = ArcRwSignal::new(errs);

            let seen_error_ids = Arc::new(Mutex::new(HashSet::new()));

            let _view = run_fallback(
                signal.clone(),
                Some(on_error),
                None,
                "heading".to_string(),
                &seen_error_ids,
            );
            let _view = run_fallback(
                signal,
                Some(on_error),
                None,
                "heading".to_string(),
                &seen_error_ids,
            );

            let mut captured = captured.lock().expect("lock").clone();

            captured.sort();

            assert_eq!(
                captured.as_slice(),
                ["first", "second"],
                "on_error must not replay unchanged errors after fallback rerender"
            );
        });
    }
}
