//! Headless non-web Dioxus harness for Desktop, mobile, and SSR test passes.
//!
//! Component adapters in `ars-dioxus` follow a `cfg(feature = "web")`
//! graceful-degrade contract: on Desktop, mobile, and SSR builds the
//! browser-only listeners and DOM lookups are skipped while the
//! component's structural surface (rendered tree, returned handles,
//! callback wiring) remains available. This module exercises that
//! contract without launching a real WRY window — the cfg branches behave
//! identically because the runtime is the same [`VirtualDom`] every
//! Dioxus renderer wraps.
//!
//! The harness is intentionally minimal: it mounts a component, drives an
//! initial render, and exposes a [`flush`](DesktopHarness::flush) drain so
//! reactive effects and queued tasks can run between assertions. Tests
//! express expectations through callbacks captured by the fixture
//! component rather than DOM introspection — adding HTML-string
//! assertions would require a separate renderer crate dependency and is
//! intentionally deferred.
//!
//! See `spec/testing/15-test-harness.md` §5.4 for the canonical
//! description of this non-web Dioxus tier and
//! `spec/dioxus-components/utility/dismissable.md` §29-§31 for the
//! adapter contract it first served.

use std::{
    fmt::{self, Debug},
    sync::Arc,
};

use ars_i18n::Locale;
use dioxus::{core::ComponentFunction, prelude::*};

/// Headless [`VirtualDom`] wrapper for non-web Dioxus component tests.
///
/// Construct with [`launch`](Self::launch),
/// [`launch_with_props`](Self::launch_with_props), or
/// [`launch_with_locale`](Self::launch_with_locale); drive reactive work
/// with [`flush`](Self::flush); and drop the harness to release the
/// underlying runtime. Dropping a `DesktopHarness` runs each scope's
/// `use_drop` cleanup once, so cleanup expectations expressed through
/// fixture-side counters fire exactly when the harness goes out of
/// scope.
pub struct DesktopHarness {
    vdom: VirtualDom,
}

impl DesktopHarness {
    /// Mounts a no-prop component function and runs the initial rebuild.
    ///
    /// After this call returns, every hook in the component has executed
    /// once and any synchronously-scheduled effects have been queued.
    /// Drive queued work with [`flush`](Self::flush).
    #[must_use]
    pub fn launch(component: fn() -> Element) -> Self {
        let mut vdom = VirtualDom::new(component);

        vdom.rebuild_in_place();

        Self { vdom }
    }

    /// Mounts a component with custom root props and runs the initial rebuild.
    ///
    /// `P` mirrors Dioxus's own `new_with_props` bound — props must be
    /// `Clone + 'static`. Use this entry point when the fixture component
    /// needs to receive shared test state (recorders, slots, etc.) at
    /// mount time without going through `provide_context`.
    #[must_use]
    pub fn launch_with_props<P, M>(component: impl ComponentFunction<P, M>, props: P) -> Self
    where
        P: Clone + 'static,
        M: 'static,
    {
        let mut vdom = VirtualDom::new_with_props(component, props);

        vdom.rebuild_in_place();

        Self { vdom }
    }

    /// Mounts a closure-rendered subtree wrapped in
    /// [`ars_dioxus::ArsProvider`] with the supplied [`Locale`].
    ///
    /// Mirrors the wasm tier's
    /// [`HarnessBackend::mount_with_locale`](ars_test_harness::HarnessBackend::mount_with_locale)
    /// contract — when a non-web component test needs to exercise
    /// locale-sensitive output (e.g. the dismissable region's
    /// `dismiss_label`), this entrypoint installs the provider context
    /// before rebuilding so [`use_locale`](ars_dioxus::use_locale) and
    /// [`use_messages`](ars_dioxus::use_messages) resolve to the
    /// requested locale.
    ///
    /// `builder` is an `Fn() -> Element` closure that renders the inner
    /// subtree; calling it inside the harness's wrapper component keeps
    /// the fixture flexible without forcing every caller to define a
    /// dedicated component fn for the locale wrapper.
    #[must_use]
    pub fn launch_with_locale<F>(builder: F, locale: Locale) -> Self
    where
        F: Fn() -> Element + 'static,
    {
        let inner: InnerRenderer = Arc::new(builder);

        let mut vdom =
            VirtualDom::new_with_props(LocaleWrapper, LocaleWrapperProps { locale, inner });

        vdom.rebuild_in_place();

        Self { vdom }
    }

    /// Drains pending Dioxus work — queued events, dirty scopes, and
    /// effects — until the runtime is idle.
    ///
    /// Mirrors the wasm-tier
    /// [`HarnessBackend::flush`](ars_test_harness::HarnessBackend::flush)
    /// contract: after this returns, every reactive update scheduled by
    /// the most recent interaction has been applied. Call it after
    /// dispatching a callback so any reactive effects scheduled in
    /// response get a chance to run before the next assertion.
    pub fn flush(&mut self) {
        self.vdom.process_events();
    }
}

impl Debug for DesktopHarness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DesktopHarness")
            .field("vdom", &"<VirtualDom>")
            .finish()
    }
}

/// Type-erased subtree renderer used by [`DesktopHarness::launch_with_locale`].
type InnerRenderer = Arc<dyn Fn() -> Element + 'static>;

#[derive(Clone, Props)]
struct LocaleWrapperProps {
    locale: Locale,
    inner: InnerRenderer,
}

impl PartialEq for LocaleWrapperProps {
    fn eq(&self, other: &Self) -> bool {
        self.locale == other.locale && Arc::ptr_eq(&self.inner, &other.inner)
    }
}

#[derive(Clone, Props)]
struct InnerWrapperProps {
    inner: InnerRenderer,
}

impl PartialEq for InnerWrapperProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

#[expect(
    non_snake_case,
    reason = "Dioxus components are PascalCase by convention."
)]
fn LocaleWrapper(props: LocaleWrapperProps) -> Element {
    let locale_signal = use_signal(|| props.locale.clone());

    rsx! {
        ars_dioxus::ArsProvider { locale: locale_signal,
            InnerWrapper { inner: props.inner }
        }
    }
}

#[expect(
    non_snake_case,
    reason = "Dioxus components are PascalCase by convention."
)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus root props are moved into the render function."
)]
fn InnerWrapper(props: InnerWrapperProps) -> Element {
    // Calling the renderer inside this child component's scope ensures
    // any `use_locale` / `use_messages` calls inside the closure resolve
    // through the surrounding `ArsProvider`'s context, not through
    // `LocaleWrapper`'s parent scope.
    (props.inner)()
}
