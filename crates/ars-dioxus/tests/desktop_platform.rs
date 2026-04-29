//! End-to-end Desktop test pass for the Dioxus platform abstraction.
//!
//! Complements the per-method native unit tests in
//! `crates/ars-dioxus/src/platform.rs` (mod tests, gated on
//! `feature = "desktop"`) by exercising `use_platform()` and the
//! `ArsContext.dioxus_platform` threading through a real
//! [`VirtualDom`] mount via [`DesktopHarness`]. The unit tests prove
//! `DesktopPlatform`'s methods do the right thing in isolation; this
//! file proves the trait object survives provider context, hook
//! resolution, render cycles, and harness teardown without panicking
//! or losing identity.
//!
//! Gated on `not(target_arch = "wasm32")` because `DesktopHarness`
//! runs on the native test profile only — wasm builds exercise the
//! same surface via `mod wasm_tests` inside `platform.rs`.
//!
//! Spec refs:
//! - `spec/foundation/09-adapter-dioxus.md` §6.1 (`DioxusPlatform`)
//! - `spec/foundation/09-adapter-dioxus.md` §6.2 (`use_platform`)
//! - `spec/testing/15-test-harness.md` §5.4 (non-web Dioxus tier)

#![cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use ars_core::{
    ColorMode, DefaultModalityContext, I18nRegistries, ModalityContext, NullPlatformEffects,
    PlatformEffects, StyleStrategy,
};
use ars_dioxus::{
    ArsContext, DioxusPlatform, DragData, NullPlatform, PlatformDragEvent, use_platform,
};
use ars_i18n::{Direction, IntlBackend, StubIntlBackend, locales};
use ars_test_harness_dioxus::desktop::DesktopHarness;
use dioxus::prelude::*;

/// Captures every platform method invocation a fixture makes so
/// assertions can run after the harness has dropped. Uses sync
/// primitives (`Arc<Mutex>`) rather than Dioxus signals because the
/// fixture writes from inside `use_hook` and assertions read from the
/// outer test thread after harness teardown.
#[derive(Clone, Debug, Default)]
struct ProbeRecorder {
    new_ids: Arc<Mutex<Vec<String>>>,
    last_now: Arc<Mutex<Option<Duration>>>,
    later_now: Arc<Mutex<Option<Duration>>>,
    drag_data_was_none: Arc<Mutex<Option<bool>>>,
}

#[derive(Clone)]
struct FixtureProps {
    recorder: ProbeRecorder,

    /// Optional explicit platform to install on the [`ArsContext`]
    /// before the fixture component mounts. When `None` the fixture
    /// goes through the default-provider fallback path.
    explicit_platform: Option<Arc<dyn DioxusPlatform>>,

    /// Captured handle to the platform `use_platform()` resolved.
    /// Populated by the fixture during render so the test thread can
    /// verify Arc identity matches what was installed.
    resolved_platform_slot: Rc<RefCell<Option<Arc<dyn DioxusPlatform>>>>,
}

impl PartialEq for FixtureProps {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.recorder.new_ids, &other.recorder.new_ids)
            && Rc::ptr_eq(&self.resolved_platform_slot, &other.resolved_platform_slot)
            && match (&self.explicit_platform, &other.explicit_platform) {
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
    }
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Dioxus root props are moved into the render function."
)]
fn fixture(props: FixtureProps) -> Element {
    // Provider must be installed before any consumer hook in this scope.
    if let Some(explicit) = props.explicit_platform.clone() {
        use_context_provider(move || build_context(explicit));
    }

    // Hooks must be called at the top level of the component — calling
    // them from inside `use_hook(...)` triggers Dioxus's hook-list
    // borrow check at runtime.
    let platform = use_platform();

    let recorder = props.recorder.clone();

    let slot = Rc::clone(&props.resolved_platform_slot);

    let captured = Arc::clone(&platform);

    use_hook(move || {
        // Stash the handle so the test can compare Arc identities.
        *slot.borrow_mut() = Some(Arc::clone(&captured));

        let drag = captured.create_drag_data(PlatformDragEvent::empty());

        let id_a = captured.new_id();
        let id_b = captured.new_id();

        recorder
            .new_ids
            .lock()
            .expect("new_ids mutex must not be poisoned")
            .extend([id_a, id_b]);

        *recorder
            .last_now
            .lock()
            .expect("last_now mutex must not be poisoned") = Some(captured.monotonic_now());

        *recorder
            .later_now
            .lock()
            .expect("later_now mutex must not be poisoned") = Some(captured.monotonic_now());

        *recorder
            .drag_data_was_none
            .lock()
            .expect("drag mutex must not be poisoned") = Some(drag.is_none());
    });

    rsx! {
        div { id: "desktop-platform-probe" }
    }
}

/// Builds a minimal [`ArsContext`] carrying the supplied platform.
/// Other context fields use the same null-effect defaults the
/// `provider.rs` tests rely on.
fn build_context(platform: Arc<dyn DioxusPlatform>) -> ArsContext {
    let core_platform: Arc<dyn PlatformEffects> = Arc::new(NullPlatformEffects);
    let modality: Arc<dyn ModalityContext> = Arc::new(DefaultModalityContext::new());
    let intl: Arc<dyn IntlBackend> = Arc::new(StubIntlBackend);

    ArsContext::new(
        locales::en_us(),
        Direction::Ltr,
        ColorMode::System,
        false,
        false,
        None,
        None,
        None,
        core_platform,
        modality,
        intl,
        Arc::new(I18nRegistries::new()),
        platform,
        StyleStrategy::Inline,
    )
}

fn empty_recorder() -> ProbeRecorder {
    ProbeRecorder::default()
}

fn make_props(
    recorder: ProbeRecorder,
    explicit_platform: Option<Arc<dyn DioxusPlatform>>,
) -> FixtureProps {
    FixtureProps {
        recorder,
        explicit_platform,
        resolved_platform_slot: Rc::new(RefCell::new(None)),
    }
}

#[test]
fn use_platform_resolves_desktop_default_when_no_provider_is_present() {
    // No `ArsProvider` mounted → `use_platform()` falls through
    // [`default_dioxus_platform`] which selects `DesktopPlatform`
    // (since `web` is not on a wasm32 target here).
    let recorder = empty_recorder();

    let props = make_props(recorder.clone(), None);

    let _harness = DesktopHarness::launch_with_props(fixture, props.clone());

    // The fixture must have driven every method without panicking.
    let new_ids = recorder
        .new_ids
        .lock()
        .expect("new_ids mutex must not be poisoned");

    assert_eq!(new_ids.len(), 2);

    // DesktopPlatform mints UUIDv4 strings, never `null-id-…`.
    for id in new_ids.iter() {
        assert_eq!(id.len(), 36, "expected UUIDv4 length, got {id:?}");
        assert!(!id.starts_with("null-id-"), "got null-id from desktop path");
    }

    assert_ne!(new_ids[0], new_ids[1], "UUIDv4 collision is real signal");

    // Instant-backed monotonic_now must not go backwards. The first
    // sample can legitimately be zero because the static start point is
    // initialized on first use.
    let now = recorder
        .last_now
        .lock()
        .expect("last_now mutex must not be poisoned")
        .expect("fixture must record monotonic_now");

    let later = recorder
        .later_now
        .lock()
        .expect("later_now mutex must not be poisoned")
        .expect("fixture must record second monotonic_now");

    assert!(later >= now);

    // Empty drag wrappers return their no-payload default.
    let lock_bool = |slot: &Arc<Mutex<Option<bool>>>, label: &str| -> Option<bool> {
        *slot
            .lock()
            .unwrap_or_else(|_| panic!("{label} mutex must not be poisoned"))
    };

    assert_eq!(lock_bool(&recorder.drag_data_was_none, "drag"), Some(true));
}

#[test]
fn use_platform_returns_explicit_arc_from_ars_context_via_arc_ptr_eq() {
    // Install a `NullPlatform` Arc explicitly; `use_platform()` must
    // hand back the *same* Arc, proving the context lookup path
    // doesn't accidentally allocate a fresh handle.
    let recorder = empty_recorder();

    let explicit: Arc<dyn DioxusPlatform> = Arc::new(NullPlatform);

    let props = make_props(recorder.clone(), Some(Arc::clone(&explicit)));

    let resolved_slot = Rc::clone(&props.resolved_platform_slot);

    let _harness = DesktopHarness::launch_with_props(fixture, props);

    let resolved = resolved_slot
        .borrow()
        .as_ref()
        .map(Arc::clone)
        .expect("fixture must record the resolved platform");

    assert!(
        Arc::ptr_eq(&resolved, &explicit),
        "use_platform() must return the same Arc the provider installed",
    );

    // NullPlatform's monotonic_now is `Duration::ZERO` and new_id
    // returns the `null-id-…` counter — distinct from
    // DesktopPlatform's UUIDv4 output.
    let now = recorder
        .last_now
        .lock()
        .expect("last_now mutex must not be poisoned")
        .expect("fixture must record monotonic_now");

    assert_eq!(now, Duration::ZERO);

    let later = recorder
        .later_now
        .lock()
        .expect("later_now mutex must not be poisoned")
        .expect("fixture must record second monotonic_now");

    assert_eq!(later, Duration::ZERO);

    let new_ids = recorder
        .new_ids
        .lock()
        .expect("new_ids mutex must not be poisoned");

    for id in new_ids.iter() {
        assert!(id.starts_with("null-id-"), "expected null-id, got {id:?}");
    }
}

#[test]
fn use_platform_handle_survives_render_cycle_drop_without_panic() {
    // Drop the harness explicitly, then access the captured Arc.
    // `Arc<dyn DioxusPlatform>` should outlive the VirtualDom because
    // it's reference-counted, not arena-allocated.
    let recorder = empty_recorder();

    let explicit: Arc<dyn DioxusPlatform> = Arc::new(NullPlatform);

    let props = make_props(recorder, Some(Arc::clone(&explicit)));

    let resolved_slot = Rc::clone(&props.resolved_platform_slot);

    {
        let _harness = DesktopHarness::launch_with_props(fixture, props);
        // Harness drops at end of this scope.
    }

    let resolved = resolved_slot
        .borrow()
        .as_ref()
        .map(Arc::clone)
        .expect("fixture must record the resolved platform");

    // After harness teardown the Arc must still be live and usable.
    let id = resolved.new_id();

    assert!(id.starts_with("null-id-"));
    assert_eq!(resolved.monotonic_now(), Duration::ZERO);

    // And it must still be the Arc we installed.
    assert!(Arc::ptr_eq(&resolved, &explicit));
}

#[test]
fn drag_data_default_is_equal_under_partial_eq_derive() {
    // Pin the F1 derive cascade end-to-end on the desktop binary:
    // `DragData` derives `PartialEq + Eq` by virtue of
    // `ars_interactions::DragItem` deriving them. Two default
    // instances must compare equal.
    assert_eq!(DragData::default(), DragData::default());
}
