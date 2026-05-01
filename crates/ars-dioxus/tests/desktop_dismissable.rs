//! Non-web (Desktop, mobile, SSR) test pass for the Dismissable Dioxus adapter.
//!
//! Satisfies `spec/dioxus-components/utility/dismissable.md` §29-§31:
//! when the adapter compiles without the `web` feature (or with `web` on
//! a non-wasm target), the hook must return a structurally-valid
//! [`dismissable::Handle`], no document listeners install, no
//! outside-interaction callbacks fire, and the dismiss-button activation
//! path remains available. The browser harness covers the listener
//! semantics; this file is the spec-mandated "repeat the outside-click
//! check on the target runtime" pass.
//!
//! The whole file is gated on non-wasm targets — wasm builds run the
//! browser test pass via `wasm-pack test` instead.

#![cfg(not(target_arch = "wasm32"))]

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use ars_dioxus::utility::dismissable::{self, DismissReason, use_dismissable};
use ars_test_harness_dioxus::desktop::DesktopHarness;
use dioxus::prelude::*;

#[derive(Clone)]
struct FixtureProps {
    handle_slot: Rc<RefCell<Option<dismissable::Handle>>>,
    dismiss_log: Arc<Mutex<Vec<DismissReason>>>,
    interact_outside_count: Arc<AtomicUsize>,
    escape_count: Arc<AtomicUsize>,
}

impl PartialEq for FixtureProps {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.handle_slot, &other.handle_slot)
            && Arc::ptr_eq(&self.dismiss_log, &other.dismiss_log)
            && Arc::ptr_eq(&self.interact_outside_count, &other.interact_outside_count)
            && Arc::ptr_eq(&self.escape_count, &other.escape_count)
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus root props are moved into the render function."
)]
#[expect(
    unused_qualifications,
    reason = "rsx! macro expansion confuses the unused-qualifications lint on onmounted: bindings."
)]
fn fixture(state: FixtureProps) -> Element {
    let dismiss_log = Arc::clone(&state.dismiss_log);
    let interact_outside_count = Arc::clone(&state.interact_outside_count);
    let escape_count = Arc::clone(&state.escape_count);

    let props = dismissable::Props::new()
        .on_dismiss(move |reason| {
            dismiss_log
                .lock()
                .expect("dismiss_log mutex must not be poisoned")
                .push(reason);
        })
        .on_interact_outside(move |_attempt| {
            interact_outside_count.fetch_add(1, Ordering::SeqCst);
        })
        .on_escape_key_down(move |_attempt| {
            escape_count.fetch_add(1, Ordering::SeqCst);
        });

    let boundaries = use_signal(Vec::new);

    let mut root_ref = use_signal(|| None);

    let handle = use_dismissable(
        ReadSignal::from(root_ref),
        props,
        ReadSignal::from(boundaries),
    );

    let slot = Rc::clone(&state.handle_slot);

    use_hook(move || {
        *slot.borrow_mut() = Some(handle);
    });

    rsx! {
        div {
            onmounted: move |evt| {
                root_ref.set(Some(evt.data()));
            },
        }
    }
}

fn build_state() -> FixtureProps {
    FixtureProps {
        handle_slot: Rc::new(RefCell::new(None)),
        dismiss_log: Arc::new(Mutex::new(Vec::new())),
        interact_outside_count: Arc::new(AtomicUsize::new(0)),
        escape_count: Arc::new(AtomicUsize::new(0)),
    }
}

#[test]
fn region_mounts_on_desktop_without_panic() {
    let state = build_state();

    let _harness = DesktopHarness::launch_with_props(fixture, state.clone());

    let handle_slot = state.handle_slot.borrow();

    let handle = handle_slot
        .as_ref()
        .expect("fixture must populate the handle slot during the initial rebuild");

    let id = handle.overlay_id.peek();

    assert!(
        !id.is_empty(),
        "overlay_id must be a non-empty stable id even on the non-web cfg branch",
    );
}

#[test]
fn handle_dismiss_fires_on_dismiss_with_dismiss_button_reason() {
    let state = build_state();

    let _harness = DesktopHarness::launch_with_props(fixture, state.clone());

    let handle = state
        .handle_slot
        .borrow()
        .expect("fixture must populate the handle slot during the initial rebuild");

    handle.dismiss.call(());

    let log = state
        .dismiss_log
        .lock()
        .expect("dismiss_log mutex must not be poisoned");

    assert_eq!(
        log.as_slice(),
        &[DismissReason::DismissButton],
        "Handle::dismiss must invoke on_dismiss with DismissButton on the non-web cfg branch",
    );
}

#[test]
fn handle_is_copy_and_shares_overlay_id() {
    let state = build_state();

    let _harness = DesktopHarness::launch_with_props(fixture, state.clone());

    let handle = state
        .handle_slot
        .borrow()
        .expect("fixture must populate the handle slot during the initial rebuild");

    let copied = handle;

    let original_id = handle.overlay_id.peek().clone();
    let copied_id = copied.overlay_id.peek().clone();

    assert_eq!(
        original_id, copied_id,
        "Copy must share the same arena slot, not duplicate it",
    );

    handle.dismiss.call(());
    copied.dismiss.call(());

    let log = state
        .dismiss_log
        .lock()
        .expect("dismiss_log mutex must not be poisoned");

    assert_eq!(
        log.as_slice(),
        &[DismissReason::DismissButton, DismissReason::DismissButton],
        "Copy must point to the same arena-backed dismiss callback",
    );
}

#[test]
fn handle_debug_includes_overlay_id() {
    let state = build_state();

    let _harness = DesktopHarness::launch_with_props(fixture, state.clone());

    let handle = state
        .handle_slot
        .borrow()
        .expect("fixture must populate the handle slot during the initial rebuild");

    let debug = format!("{handle:?}");

    let id = handle.overlay_id.peek();

    assert!(
        debug.contains(id.as_str()),
        "Handle Debug must surface the live overlay_id (got {debug})",
    );
}

#[test]
fn handle_dismiss_is_no_op_when_props_on_dismiss_missing() {
    #[derive(Clone)]
    struct EmptyFixtureProps {
        handle_slot: Rc<RefCell<Option<dismissable::Handle>>>,
    }

    impl PartialEq for EmptyFixtureProps {
        fn eq(&self, other: &Self) -> bool {
            Rc::ptr_eq(&self.handle_slot, &other.handle_slot)
        }
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    #[expect(
        unused_qualifications,
        reason = "rsx! macro expansion confuses the unused-qualifications lint on onmounted: bindings."
    )]
    fn empty_fixture(state: EmptyFixtureProps) -> Element {
        let boundaries = use_signal(Vec::new);
        let mut root_ref = use_signal(|| None);

        let handle = use_dismissable(
            ReadSignal::from(root_ref),
            dismissable::Props::new(),
            ReadSignal::from(boundaries),
        );

        let slot = Rc::clone(&state.handle_slot);

        use_hook(move || {
            *slot.borrow_mut() = Some(handle);
        });

        rsx! {
            div {
                onmounted: move |evt| {
                    root_ref.set(Some(evt.data()));
                },
            }
        }
    }

    let state = EmptyFixtureProps {
        handle_slot: Rc::new(RefCell::new(None)),
    };

    let _harness = DesktopHarness::launch_with_props(empty_fixture, state.clone());

    let handle = state
        .handle_slot
        .borrow()
        .expect("fixture must populate the handle slot during the initial rebuild");

    handle.dismiss.call(());
    handle.dismiss.call(());
}

#[test]
fn outside_interaction_callbacks_do_not_fire_on_desktop() {
    let state = build_state();

    let mut harness = DesktopHarness::launch_with_props(fixture, state.clone());

    harness.flush();

    assert_eq!(
        state.interact_outside_count.load(Ordering::SeqCst),
        0,
        "on_interact_outside must remain silent — no document listeners are installed on Desktop",
    );

    assert_eq!(
        state.escape_count.load(Ordering::SeqCst),
        0,
        "on_escape_key_down must remain silent — no document listeners are installed on Desktop",
    );

    let log = state
        .dismiss_log
        .lock()
        .expect("dismiss_log mutex must not be poisoned");

    assert!(
        log.is_empty(),
        "on_dismiss must not fire on the non-web cfg branch without an explicit Handle::dismiss call",
    );
}

#[test]
fn region_unmount_runs_cleanly() {
    let state = build_state();

    {
        let _harness = DesktopHarness::launch_with_props(fixture, state.clone());

        assert!(
            state.handle_slot.borrow().is_some(),
            "fixture must populate the handle slot before unmount",
        );
    }

    assert_eq!(
        state.interact_outside_count.load(Ordering::SeqCst),
        0,
        "dropping the harness must not synthesize outside-interaction callbacks",
    );

    assert_eq!(
        state.escape_count.load(Ordering::SeqCst),
        0,
        "dropping the harness must not synthesize escape callbacks",
    );

    let log = state
        .dismiss_log
        .lock()
        .expect("dismiss_log mutex must not be poisoned");

    assert!(
        log.is_empty(),
        "dropping the harness must not synthesize on_dismiss callbacks",
    );
}
