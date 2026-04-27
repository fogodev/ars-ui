//! Smoke tests for the non-web [`DesktopHarness`] surface.
//!
//! These tests exist so the harness itself stays honest before component
//! tests start relying on it: confirming the initial rebuild fires the
//! component body once, that `flush` drains queued effects, that
//! dropping the harness runs each scope's `use_drop` cleanup exactly
//! once, and that `launch_with_locale` installs the
//! `ArsProvider`/`use_locale` context the wasm tier expects.
//!
//! The whole file is gated on non-wasm targets — on `wasm32-unknown-unknown`
//! the harness module is not compiled, so this binary would fail to link.

#![cfg(not(target_arch = "wasm32"))]

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
};

use ars_dioxus::use_locale;
use ars_i18n::{Locale, locales};
use ars_test_harness_dioxus::desktop::DesktopHarness;
use dioxus::prelude::*;

type SharedCounter = Rc<RefCell<u32>>;
type SharedLog = Rc<RefCell<Vec<&'static str>>>;

#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus root props are moved into the render function."
)]
fn render_counter_fixture(counter: SharedCounter) -> Element {
    *counter.borrow_mut() += 1;

    rsx! { div {} }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus root props are moved into the render function."
)]
fn effect_fixture(log: SharedLog) -> Element {
    let log_for_effect = Rc::clone(&log);

    use_effect(move || {
        log_for_effect.borrow_mut().push("effect");
    });

    rsx! { div {} }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus root props are moved into the render function."
)]
fn drop_fixture(counter: SharedCounter) -> Element {
    let counter_for_drop = Rc::clone(&counter);

    use_drop(move || {
        *counter_for_drop.borrow_mut() += 1;
    });

    rsx! { div {} }
}

#[test]
fn launch_runs_initial_render() {
    let counter: SharedCounter = Rc::new(RefCell::new(0));

    let _harness = DesktopHarness::launch_with_props(render_counter_fixture, Rc::clone(&counter));

    assert_eq!(
        *counter.borrow(),
        1,
        "render fn body must execute exactly once during the initial rebuild",
    );
}

#[test]
fn flush_runs_effects() {
    let log: SharedLog = Rc::new(RefCell::new(Vec::new()));

    let mut harness = DesktopHarness::launch_with_props(effect_fixture, Rc::clone(&log));

    harness.flush();

    assert_eq!(
        log.borrow().as_slice(),
        &["effect"],
        "flush must drain the queued use_effect body",
    );
}

#[test]
fn harness_drops_cleanly() {
    let counter: SharedCounter = Rc::new(RefCell::new(0));

    {
        let _harness = DesktopHarness::launch_with_props(drop_fixture, Rc::clone(&counter));

        assert_eq!(
            *counter.borrow(),
            0,
            "use_drop cleanup must not run while the harness is alive",
        );
    }

    assert_eq!(
        *counter.borrow(),
        1,
        "use_drop cleanup must run exactly once when the harness is dropped",
    );
}

#[test]
fn launch_with_locale_installs_ars_provider_context() {
    let captured: Arc<Mutex<Option<Locale>>> = Arc::new(Mutex::new(None));

    let captured_for_inner = Arc::clone(&captured);

    let target_locale = locales::de_de();
    let target_for_assert = target_locale.clone();

    let _harness = DesktopHarness::launch_with_locale(
        move || {
            let locale = use_locale();
            captured_for_inner
                .lock()
                .expect("captured locale mutex must not be poisoned")
                .replace(locale.peek().clone());

            rsx! { div {} }
        },
        target_locale,
    );

    let observed = captured
        .lock()
        .expect("captured locale mutex must not be poisoned")
        .clone()
        .expect("inner subtree must run during the initial rebuild");

    assert_eq!(
        observed, target_for_assert,
        "launch_with_locale must wrap the subtree in ArsProvider so use_locale resolves to it",
    );
}
