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
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

pub use ars_components::utility::error_boundary::{Api, Messages, Part};
use ars_i18n::Locale;
pub use leptos::error::Error as CapturedError;
use leptos::{
    children::{TypedChildren, ViewFnOnce},
    error::ErrorId,
    prelude::*,
};

use crate::{
    attrs::attr_map_to_leptos_inline_attrs,
    provider::{resolve_locale, use_messages},
};

// ────────────────────────────────────────────────────────────────────
// FallbackHandler
// ────────────────────────────────────────────────────────────────────

/// Adapter-side fallback handler for [`Boundary`].
///
/// Consumers can pass a closure directly to [`Boundary`]'s `fallback` prop
/// without spelling out this type. The closure receives Leptos's reactive
/// multi-error signal and returns any typed Leptos view. The adapter erases
/// that view only after the closure runs, matching Leptos's framework
/// `ErrorBoundary` fallback contract.
#[derive(Clone, Copy, Debug)]
pub struct FallbackHandler(Callback<ArcRwSignal<Errors>, ViewFnOnce>);

impl FallbackHandler {
    /// Create a fallback handler from a typed view-producing closure.
    #[must_use]
    pub fn new<F, V>(fallback: F) -> Self
    where
        F: Fn(ArcRwSignal<Errors>) -> V + Send + Sync + 'static,
        V: RenderHtml + Send + 'static,
    {
        Self(Callback::new(move |errors| {
            let view = fallback(errors);

            ViewFnOnce::from(move || view)
        }))
    }

    /// Run the fallback handler with the current error signal.
    #[must_use]
    pub(crate) fn run(self, errors: ArcRwSignal<Errors>) -> ViewFnOnce {
        self.0.run(errors)
    }
}

impl<F, V> From<F> for FallbackHandler
where
    F: Fn(ArcRwSignal<Errors>) -> V + Send + Sync + 'static,
    V: RenderHtml + Send + 'static,
{
    fn from(fallback: F) -> Self {
        Self::new(fallback)
    }
}

// ────────────────────────────────────────────────────────────────────
// Boundary
// ────────────────────────────────────────────────────────────────────

/// Wrapper around Leptos's [`ErrorBoundary`] that renders an accessible
/// fallback when a descendant component returns an error.
///
/// The fallback is a `<div role="alert" data-ars-error="true">` containing
/// a localized heading paragraph and a `<ul>` of `<li>` error entries —
/// matching the Dioxus adapter byte-for-byte. Optional props expose a
/// custom fallback override, an `on_error` telemetry hook, and locale /
/// messages bundle overrides.
///
/// # Heading capture
///
/// See `spec/leptos-components/utility/error-boundary.md` for the full
/// adapter contract.
#[component]
pub fn Boundary<T: 'static>(
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
    /// the resolved locale, falling back to [`Messages::default`].
    #[prop(optional)]
    messages: Option<Messages>,

    /// Override the locale used to resolve and render the fallback heading.
    /// When `None`, the wrapper reads the locale from `ArsProvider`.
    #[prop(optional)]
    locale: Option<Locale>,

    /// Subtree wrapped by the boundary.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    // The fallback closure must be `FnMut + Send + 'static`; capture the
    // resolved heading + optional callbacks by value and dispatch through
    // `run_fallback` so the on-error/custom-fallback/default-fallback
    // branching logic stays unit-testable in isolation from the
    // framework primitive.
    let seen_errors = Arc::new(Mutex::new(HashMap::new()));

    let fallback_closure = move |errors: ArcRwSignal<Errors>| {
        let resolved_locale = resolve_locale(locale.as_ref());
        let resolved_messages = use_messages(messages.as_ref(), Some(&resolved_locale));

        run_fallback(
            errors,
            on_error,
            fallback,
            (resolved_messages.message)(&resolved_locale),
            &seen_errors,
        )
        .run()
    };

    view! { <ErrorBoundary fallback=fallback_closure>{children.into_inner()()}</ErrorBoundary> }
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
    seen_errors: &Mutex<HashMap<ErrorId, String>>,
) -> ViewFnOnce {
    if let Some(handler) = on_error {
        let snapshot = errors.get().into_iter().collect::<Vec<_>>();
        let current_ids = snapshot
            .iter()
            .map(|(id, _)| id.clone())
            .collect::<HashSet<_>>();

        let mut seen = seen_errors.lock().expect("lock seen errors");

        seen.retain(|id, _| current_ids.contains(id));

        for (id, error) in snapshot {
            let error_fingerprint = error.to_string();

            if seen.get(&id) != Some(&error_fingerprint) {
                seen.insert(id, error_fingerprint);
                handler.run(error.clone());
            }
        }
    }

    if let Some(custom) = fallback {
        custom.run(errors)
    } else {
        ViewFnOnce::from(move || render_default_fallback(errors, heading))
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
///     <ErrorBoundary fallback=ars_leptos::utility::error_boundary::default_fallback>
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
pub fn default_fallback(errors: ArcRwSignal<Errors>) -> impl IntoView {
    let messages = Messages::default();

    let locale = en_us_locale();

    let heading = (messages.message)(&locale);

    render_default_fallback(errors, heading)
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
                {snapshot
                    .into_iter()
                    .map(move |(_, e)| {
                        view! {
                            <li {..attr_map_to_leptos_inline_attrs(
                                api.item_attrs(),
                            )}>{e.to_string()}</li>
                        }
                    })
                    .collect_view()}
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
                &Mutex::new(HashMap::new()),
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
            // None branch is exercised; the view thunk itself is opaque.
            let _view = run_fallback(
                ArcRwSignal::new(errors),
                None,
                None,
                "heading".to_string(),
                &Mutex::new(HashMap::new()),
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
                &Mutex::new(HashMap::new()),
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

            let seen_errors = Arc::new(Mutex::new(HashMap::new()));

            let _view = run_fallback(
                signal.clone(),
                Some(on_error),
                None,
                "heading".to_string(),
                &seen_errors,
            );
            let _view = run_fallback(
                signal,
                Some(on_error),
                None,
                "heading".to_string(),
                &seen_errors,
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

    #[test]
    fn run_fallback_prunes_seen_ids_when_errors_clear() {
        let owner = Owner::new();
        owner.with(|| {
            let captured: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
            let captured_for_cb = Arc::clone(&captured);
            let on_error = Callback::new(move |err: CapturedError| {
                captured_for_cb.lock().expect("lock").push(err.to_string());
            });

            let mut first = Errors::default();
            first.insert(ErrorId::from(1usize), BoomError("first episode"));

            let seen_errors = Arc::new(Mutex::new(HashMap::new()));

            let _view = run_fallback(
                ArcRwSignal::new(first),
                Some(on_error),
                None,
                "heading".to_string(),
                &seen_errors,
            );

            let _view = run_fallback(
                ArcRwSignal::new(Errors::default()),
                Some(on_error),
                None,
                "heading".to_string(),
                &seen_errors,
            );

            let mut second = Errors::default();
            second.insert(ErrorId::from(1usize), BoomError("second episode"));

            let _view = run_fallback(
                ArcRwSignal::new(second),
                Some(on_error),
                None,
                "heading".to_string(),
                &seen_errors,
            );

            assert_eq!(
                captured.lock().expect("lock").as_slice(),
                ["first episode", "second episode"],
                "cleared error snapshots must prune seen ids so reused ids can emit again"
            );
        });
    }

    #[test]
    fn run_fallback_emits_when_same_id_receives_new_error_value() {
        let owner = Owner::new();
        owner.with(|| {
            let captured: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
            let captured_for_cb = Arc::clone(&captured);
            let on_error = Callback::new(move |err: CapturedError| {
                captured_for_cb.lock().expect("lock").push(err.to_string());
            });

            let mut first = Errors::default();
            first.insert(ErrorId::from(1usize), BoomError("first value"));

            let seen_errors = Arc::new(Mutex::new(HashMap::new()));

            let _view = run_fallback(
                ArcRwSignal::new(first),
                Some(on_error),
                None,
                "heading".to_string(),
                &seen_errors,
            );

            let mut second = Errors::default();
            second.insert(ErrorId::from(1usize), BoomError("second value"));

            let _view = run_fallback(
                ArcRwSignal::new(second),
                Some(on_error),
                None,
                "heading".to_string(),
                &seen_errors,
            );

            assert_eq!(
                captured.lock().expect("lock").as_slice(),
                ["first value", "second value"],
                "replacing an existing ErrorId with a different error must emit telemetry"
            );
        });
    }
}
