//! Non-web desktop smoke test for `error_boundary::Boundary`.
//!
//! The SSR string-render tests in `tests/error_boundary.rs` cover the spec'd
//! HTML contract end-to-end, but they exercise `dioxus-ssr` rather than the
//! `VirtualDom` path used by Desktop, mobile, and SSR builds. This file mounts
//! the boundary through [`DesktopHarness`] so we get coverage of the
//! reactive primitive's runtime behaviour:
//!
//! - children mount without panicking on the happy path
//! - a throwing child causes `on_error` to fire on the desktop renderer
//!   (proves the closure runs in the real Dioxus runtime, not only in
//!   `dioxus-ssr`'s synchronous render pass)
//!
//! Mirrors `spec/dioxus-components/utility/dismissable.md` §29-§31's
//! "repeat the contract on the target runtime" pattern.

#![cfg(not(target_arch = "wasm32"))]

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use ars_core::{
    ColorMode, DefaultModalityContext, I18nRegistries, MessageFn, MessagesRegistry,
    NullPlatformEffects, StyleStrategy,
};
use ars_dioxus::{
    ArsContext, NullPlatform,
    utility::error_boundary::{self, Boundary},
};
use ars_i18n::{Direction, Locale, StubIntlBackend};
use ars_test_harness_dioxus::desktop::DesktopHarness;
use dioxus::{CapturedError, prelude::*};

#[derive(Clone)]
struct Fixture {
    captured: Arc<Mutex<Vec<String>>>,
    cause_error: bool,
}

impl PartialEq for Fixture {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.captured, &other.captured) && self.cause_error == other.cause_error
    }
}

#[component]
fn ThrowingChild() -> Element {
    Err(CapturedError::from_display("desktop-boom").into())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus root props are moved into the render function."
)]
fn fixture(state: Fixture) -> Element {
    let captured = Arc::clone(&state.captured);

    let on_error = use_callback(move |err: CapturedError| {
        captured.lock().expect("lock").push(err.to_string());
    });

    rsx! {
        Boundary { on_error: EventHandler::new(move |err| on_error.call(err)),
            if state.cause_error {
                ThrowingChild {}
            } else {
                p { "ok" }
            }
        }
    }
}

#[test]
fn boundary_mounts_without_panic_on_desktop() {
    let state = Fixture {
        captured: Arc::new(Mutex::new(Vec::new())),
        cause_error: false,
    };

    let mut harness = DesktopHarness::launch_with_props(fixture, state.clone());

    harness.flush();

    let captured = state.captured.lock().expect("lock");

    assert!(
        captured.is_empty(),
        "on_error should not fire on the happy path; got {captured:?}"
    );
}

#[test]
fn boundary_fires_on_error_under_desktop_runtime() {
    let state = Fixture {
        captured: Arc::new(Mutex::new(Vec::new())),
        cause_error: true,
    };

    let mut harness = DesktopHarness::launch_with_props(fixture, state.clone());

    harness.flush();

    let captured = state.captured.lock().expect("lock");

    assert!(
        captured.iter().any(|m| m.contains("desktop-boom")),
        "on_error did not fire under the desktop runtime; got {captured:?}"
    );
}

/// Locale signal mutation triggers `Boundary` to re-resolve `Messages`
/// from the registry. Unlike the SSR-side
/// `locale_change_in_provider_re_resolves_registered_messages` test
/// (which renders two separate `VirtualDom`s and asserts each picks a
/// different bundle), this test mounts a single Dioxus runtime and
/// flips the **same** `Signal<Locale>` between renders, proving the
/// reactive subscription is wired correctly.
///
/// We can't read DOM from `DesktopHarness`, so the fixture captures
/// each render's resolved heading into `Arc<Mutex<Vec<String>>>` via a
/// hook that runs once per render. The test then drives:
///
/// 1. Initial mount — fixture records the English heading.
/// 2. `signal.set("es-MX")` — Dioxus marks the scope dirty.
/// 3. `harness.flush()` — re-render runs; fixture records the Spanish
///    heading.
///
/// If the locale subscription is broken or `use_messages` caches
/// across renders, the second recording would still be English and
/// the test fails.
#[test]
fn locale_signal_mutation_re_renders_boundary_with_new_heading() {
    #[derive(Clone)]
    struct LocaleFixture {
        // Each render appends the current heading. After two flushes we
        // expect ["English…", "Spanish…"]. `Arc<Mutex>` because the
        // recorder must outlive the harness (a property the test thread
        // and the rendered fixture both need).
        recorded: Arc<Mutex<Vec<String>>>,

        // The `ArsContext` carries the locale signal. The test mutates
        // it after the initial mount; the same signal is read by the
        // fixture's `use_messages` call, so the change propagates.
        // `Rc<RefCell>` (not `Arc<Mutex>`) because `ArsContext` is
        // single-threaded by design — its Dioxus `Signal<T>` fields are
        // arena-allocated and not `Send`/`Sync`, and Dioxus is
        // single-threaded anyway.
        ctx_slot: Rc<RefCell<Option<ArsContext>>>,
    }

    impl PartialEq for LocaleFixture {
        fn eq(&self, other: &Self) -> bool {
            Arc::ptr_eq(&self.recorded, &other.recorded)
                && Rc::ptr_eq(&self.ctx_slot, &other.ctx_slot)
        }
    }

    fn registries() -> I18nRegistries {
        let mut registries = I18nRegistries::new();

        registries.register(
            MessagesRegistry::new(error_boundary::Messages::default()).register(
                "es",
                error_boundary::Messages {
                    message: MessageFn::static_str("Algo salió mal."),
                },
            ),
        );

        registries
    }

    fn build_ctx(locale: Locale) -> ArsContext {
        ArsContext::new(
            locale,
            Direction::Ltr,
            ColorMode::System,
            false,
            false,
            None,
            None,
            None,
            Arc::new(NullPlatformEffects),
            Arc::new(DefaultModalityContext::new()),
            Arc::new(StubIntlBackend),
            Arc::new(registries()),
            Arc::new(NullPlatform),
            StyleStrategy::Inline,
        )
    }

    /// Identity-equality wrapper for the recorder Arc — Dioxus's
    /// `#[component]` macro derives `PartialEq` on Props, but `Mutex`
    /// is not `PartialEq`. Comparing by Arc pointer identity is correct
    /// for our test fixture: the same recorder is shared across renders.
    #[derive(Clone)]
    struct Recorder(Arc<Mutex<Vec<String>>>);

    impl PartialEq for Recorder {
        fn eq(&self, other: &Self) -> bool {
            Arc::ptr_eq(&self.0, &other.0)
        }
    }

    /// A read-only consumer of the resolved heading. Records on every
    /// render so we can observe the locale-driven dispatch from outside
    /// the runtime.
    #[component]
    fn HeadingProbe(recorded: Recorder) -> Element {
        let messages = ars_dioxus::use_messages::<error_boundary::Messages>(None, None);

        let locale = ars_dioxus::resolve_locale(None);

        let heading = (messages.message)(&locale);

        recorded.0.lock().expect("lock").push(heading);

        rsx! {
            p { "probe" }
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn fixture(state: LocaleFixture) -> Element {
        let ctx = build_ctx(Locale::parse("en-US").expect("locale"));

        // Stash a clone of the context so the test thread can mutate
        // `ctx.locale` after the initial mount; the rendered component
        // tree below shares the same `Signal<Locale>`.
        *state.ctx_slot.borrow_mut() = Some(ctx.clone());

        use_context_provider(|| ctx);

        rsx! {
            Boundary {
                HeadingProbe { recorded: Recorder(Arc::clone(&state.recorded)) }
            }
        }
    }

    let state = LocaleFixture {
        recorded: Arc::new(Mutex::new(Vec::new())),
        ctx_slot: Rc::new(RefCell::new(None)),
    };

    let mut harness = DesktopHarness::launch_with_props(fixture, state.clone());

    harness.flush();

    {
        let recorded = state.recorded.lock().expect("lock");

        assert_eq!(
            recorded.len(),
            1,
            "exactly one render before mutation; got {recorded:?}"
        );
        assert!(
            recorded[0].contains("A component encountered an error."),
            "initial mount must use the en-US default; got {:?}",
            recorded[0]
        );
    }

    // Mutate the locale signal — Dioxus marks the scope dirty; flush
    // drains the resulting re-render.
    {
        let ctx = state.ctx_slot.borrow();

        let mut locale = ctx.as_ref().expect("ctx initialized").locale;

        locale.set(Locale::parse("es-MX").expect("locale"));
    }

    harness.flush();

    let recorded = state.recorded.lock().expect("lock");

    assert!(
        recorded.len() >= 2,
        "expected ≥2 renders after locale mutation; got {recorded:?}"
    );
    assert!(
        recorded
            .last()
            .expect("non-empty")
            .contains("Algo salió mal."),
        "post-mutation render must pick the Spanish bundle; got {recorded:?}"
    );
}
