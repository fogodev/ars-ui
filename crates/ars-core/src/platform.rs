//! Platform-agnostic interface for side effects triggered by state machine transitions.
//!
//! Effect closures must be platform-agnostic — they MUST NOT call DOM APIs directly.
//! Instead, all platform-specific operations (focus, timers, announcements, positioning,
//! scroll lock) go through the [`PlatformEffects`] trait, resolved from the adapter's
//! framework context.
//!
//! Each adapter provides an implementation:
//! - `ars-dom` provides `WebPlatformEffects` (web targets via `web_sys`)
//! - Native adapters provide their own (e.g., AccessKit for accessibility)
//!
//! Two built-in implementations are provided:
//! - [`NullPlatformEffects`] — no-op for unit tests and SSR (no warnings)
//! - [`MissingProviderEffects`] — no-op fallback with debug-mode warnings when no
//!   `ArsProvider` is found in the component tree

use alloc::{boxed::Box, string::String, vec::Vec};

use ars_i18n::Direction;

/// Platform-agnostic interface for side effects triggered by [`PendingEffect`](crate::PendingEffect) closures.
///
/// Each adapter provides an implementation:
/// - `ars-dom` provides `WebPlatformEffects` (web targets via `web_sys`)
/// - Native adapters provide their own (e.g., AccessKit for accessibility)
///
/// Components resolve this via `use_platform_effects()` inside effect closures.
///
/// Requires `Send + Sync` so implementations can be wrapped in
/// [`Arc`](alloc::sync::Arc) and safely shared across threads on native targets.
/// On wasm (single-threaded), `Send + Sync` is trivially satisfied.
pub trait PlatformEffects: Send + Sync {
    // -- Focus ---------------------------------------------------------------

    /// Focus the element with the given ID. No-op if not found.
    fn focus_element_by_id(&self, id: &str);

    /// Focus the first tabbable element inside a container. No-op if not found.
    fn focus_first_tabbable(&self, container_id: &str);

    /// Focus the last tabbable element inside a container. No-op if not found.
    fn focus_last_tabbable(&self, container_id: &str);

    /// Return IDs of all tabbable elements inside a container, in DOM order.
    fn tabbable_element_ids(&self, container_id: &str) -> Vec<String>;

    /// Focus the document body (last-resort fallback).
    fn focus_body(&self);

    // -- Timers --------------------------------------------------------------

    /// Schedule a callback after `delay_ms` milliseconds. Returns a handle for cancellation.
    fn set_timeout(&self, delay_ms: u32, callback: Box<dyn FnOnce()>) -> TimerHandle;

    /// Cancel a previously scheduled timeout.
    fn clear_timeout(&self, handle: TimerHandle);

    // -- Announcements -------------------------------------------------------

    /// Announce a message to assistive technology with polite priority.
    fn announce(&self, message: &str);

    /// Announce a message with assertive priority (interrupts current speech).
    fn announce_assertive(&self, message: &str);

    // -- Positioning ---------------------------------------------------------

    /// Position an element at absolute (x, y) coordinates.
    fn position_element_at(&self, id: &str, x: f64, y: f64);

    /// Resolve the computed text direction of an element. Returns `Ltr` or `Rtl`.
    fn resolved_direction(&self, id: &str) -> Direction;

    // -- Modal / Inert -------------------------------------------------------

    /// Set `inert` on all siblings of the portal root. Returns a cleanup function
    /// that restores the original state.
    fn set_background_inert(&self, portal_root_id: &str) -> Box<dyn FnOnce()>;

    /// Remove `inert` from all siblings of the given element.
    fn remove_inert_from_siblings(&self, portal_id: &str);

    /// Lock body scroll (prevent background scrolling under modals).
    fn scroll_lock_acquire(&self);

    /// Unlock body scroll.
    fn scroll_lock_release(&self);

    // -- DOM queries ---------------------------------------------------------

    /// Check whether an element with the given ID exists in the document.
    fn document_contains_id(&self, id: &str) -> bool;

    // -- Pointer tracking ----------------------------------------------------

    /// Track global pointer events during a drag operation (color sliders,
    /// signature pad, image cropper, etc.). Attaches `pointermove` and `pointerup`
    /// listeners at the document/window level so the drag continues even when the
    /// pointer leaves the originating element.
    ///
    /// Returns a cleanup function that removes both listeners.
    fn track_pointer_drag(
        &self,
        on_move: Box<dyn Fn(f64, f64)>,
        on_up: Box<dyn FnOnce()>,
    ) -> Box<dyn FnOnce()>;

    // -- Focus scope / focus management --------------------------------------

    /// Return the ID of the currently focused element, or `None` if nothing is focused.
    fn active_element_id(&self) -> Option<String>;

    /// Attach a focus trap to a container element so Tab/Shift+Tab cycles within it.
    /// Returns a cleanup function that removes the trap listeners.
    fn attach_focus_trap(&self, container_id: &str, on_escape: Box<dyn Fn()>) -> Box<dyn FnOnce()>;

    /// Check whether focus can be safely restored to the element with the given ID
    /// (element exists, is visible, is focusable, and has layout).
    fn can_restore_focus(&self, id: &str) -> bool;

    /// Find the nearest focusable ancestor of the element with the given ID.
    /// Returns the ancestor's ID, or `None`.
    fn nearest_focusable_ancestor_id(&self, id: &str) -> Option<String>;

    // -- Scroll --------------------------------------------------------------

    /// Set the vertical scroll position of a container element.
    fn set_scroll_top(&self, container_id: &str, scroll_top: f64);

    // -- Element measurement -------------------------------------------------

    /// Resize an element to fit its content (used by textarea auto-resize).
    /// `max_height` is an optional CSS max-height value.
    fn resize_to_content(&self, id: &str, max_height: Option<&str>);

    // -- Platform queries ----------------------------------------------------

    /// Listen for `prefers-reduced-motion` media query changes.
    /// Returns a cleanup function that removes the listener.
    fn on_reduced_motion_change(&self, callback: Box<dyn Fn(bool)>) -> Box<dyn FnOnce()>;

    /// Returns `true` if the platform is macOS (for modifier key mapping).
    fn is_mac_platform(&self) -> bool;

    /// Returns the current monotonic time in milliseconds (e.g., `performance.now()`
    /// on web, `Instant::now()` on native). Used for skip-delay window tracking.
    fn now_ms(&self) -> u64;

    /// Get the bounding rectangle of an element by ID.
    fn get_bounding_rect(&self, id: &str) -> Option<Rect>;

    // -- Animation / Transition ----------------------------------------------

    /// Watch an element for CSS animation and/or transition completion.
    /// The callback fires once when all active animations and transitions
    /// on the element have ended. Handles reduced-motion detection,
    /// dual animation+transition support, and a fallback timeout.
    /// Returns a cleanup function that removes all listeners and timers.
    /// See `spec/components/overlay/presence.md` §11 for the full
    /// web implementation specification.
    fn on_animation_end(&self, id: &str, callback: Box<dyn FnOnce()>) -> Box<dyn FnOnce()>;
}

/// Platform-agnostic bounding rectangle.
/// Replaces [`web_sys::DomRect`](https://docs.rs/web-sys/latest/web_sys/struct.DomRect.html) in core machine types.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    /// The x coordinate of the rectangle's origin.
    pub x: f64,
    /// The y coordinate of the rectangle's origin.
    pub y: f64,
    /// The width of the rectangle.
    pub width: f64,
    /// The height of the rectangle.
    pub height: f64,
}

/// Opaque timer handle returned by [`PlatformEffects::set_timeout`].
/// The only operation is cancellation via [`PlatformEffects::clear_timeout`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerHandle(u64);

impl TimerHandle {
    /// Creates a new timer handle from a platform-specific ID.
    #[must_use]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Returns the platform-specific ID wrapped by this handle.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.0
    }
}

/// No-op implementation of [`PlatformEffects`] for unit tests and SSR.
///
/// All focus/DOM operations are silent no-ops. Timers and animation-end callbacks
/// fire immediately (no delay). This is the intentional no-op — used when tests
/// or SSR explicitly pass `NullPlatformEffects` to `ArsProvider`. No warnings
/// are emitted.
#[derive(Debug)]
pub struct NullPlatformEffects;

impl PlatformEffects for NullPlatformEffects {
    #[inline]
    fn focus_element_by_id(&self, _id: &str) {}

    #[inline]
    fn focus_first_tabbable(&self, _container_id: &str) {}

    #[inline]
    fn focus_last_tabbable(&self, _container_id: &str) {}

    #[inline]
    fn tabbable_element_ids(&self, _container_id: &str) -> Vec<String> {
        Vec::new()
    }

    #[inline]
    fn focus_body(&self) {}

    #[inline]
    fn set_timeout(&self, _delay_ms: u32, callback: Box<dyn FnOnce()>) -> TimerHandle {
        callback();
        TimerHandle::new(0)
    }

    #[inline]
    fn clear_timeout(&self, _handle: TimerHandle) {}

    #[inline]
    fn announce(&self, _message: &str) {}

    #[inline]
    fn announce_assertive(&self, _message: &str) {}

    #[inline]
    fn position_element_at(&self, _id: &str, _x: f64, _y: f64) {}

    #[inline]
    fn resolved_direction(&self, _id: &str) -> Direction {
        Direction::Ltr
    }

    #[inline]
    fn set_background_inert(&self, _portal_root_id: &str) -> Box<dyn FnOnce()> {
        Box::new(|| {})
    }

    #[inline]
    fn remove_inert_from_siblings(&self, _portal_id: &str) {}

    #[inline]
    fn scroll_lock_acquire(&self) {}

    #[inline]
    fn scroll_lock_release(&self) {}

    #[inline]
    fn document_contains_id(&self, _id: &str) -> bool {
        false
    }

    #[inline]
    fn track_pointer_drag(
        &self,
        _on_move: Box<dyn Fn(f64, f64)>,
        _on_up: Box<dyn FnOnce()>,
    ) -> Box<dyn FnOnce()> {
        Box::new(|| {})
    }

    #[inline]
    fn active_element_id(&self) -> Option<String> {
        None
    }

    #[inline]
    fn attach_focus_trap(
        &self,
        _container_id: &str,
        _on_escape: Box<dyn Fn()>,
    ) -> Box<dyn FnOnce()> {
        Box::new(|| {})
    }

    #[inline]
    fn can_restore_focus(&self, _id: &str) -> bool {
        false
    }

    #[inline]
    fn nearest_focusable_ancestor_id(&self, _id: &str) -> Option<String> {
        None
    }

    #[inline]
    fn set_scroll_top(&self, _container_id: &str, _scroll_top: f64) {}

    #[inline]
    fn resize_to_content(&self, _id: &str, _max_height: Option<&str>) {}

    #[inline]
    fn on_reduced_motion_change(&self, _callback: Box<dyn Fn(bool)>) -> Box<dyn FnOnce()> {
        Box::new(|| {})
    }

    #[inline]
    fn is_mac_platform(&self) -> bool {
        false
    }

    #[inline]
    fn now_ms(&self) -> u64 {
        0
    }

    #[inline]
    fn get_bounding_rect(&self, _id: &str) -> Option<Rect> {
        None
    }

    #[inline]
    fn on_animation_end(&self, _id: &str, callback: Box<dyn FnOnce()>) -> Box<dyn FnOnce()> {
        callback();
        Box::new(|| {})
    }
}

/// Fallback [`PlatformEffects`] used when no `ArsProvider` is in the component tree.
///
/// Behaves identically to [`NullPlatformEffects`] but emits debug warnings per call
/// so the developer sees exactly which platform operations are silently failing.
/// This is NOT used in tests — only in the `use_platform_effects()` fallback path
/// inside adapters.
#[derive(Debug)]
pub struct MissingProviderEffects;

impl MissingProviderEffects {
    #[cfg(feature = "debug")]
    #[inline]
    fn warn(method: &str) {
        log::warn!(
            "[ars-ui] {method}() called without ArsProvider. \
             Platform effects are disabled. Wrap your app root in <ArsProvider>."
        );
    }

    #[cfg(not(feature = "debug"))]
    #[inline]
    const fn warn(_method: &str) {}
}

impl PlatformEffects for MissingProviderEffects {
    #[inline]
    fn focus_element_by_id(&self, _id: &str) {
        Self::warn("focus_element_by_id");
    }

    #[inline]
    fn focus_first_tabbable(&self, _container_id: &str) {
        Self::warn("focus_first_tabbable");
    }

    #[inline]
    fn focus_last_tabbable(&self, _container_id: &str) {
        Self::warn("focus_last_tabbable");
    }

    #[inline]
    fn tabbable_element_ids(&self, _container_id: &str) -> Vec<String> {
        Self::warn("tabbable_element_ids");
        Vec::new()
    }

    #[inline]
    fn focus_body(&self) {
        Self::warn("focus_body");
    }

    #[inline]
    fn set_timeout(&self, _delay_ms: u32, callback: Box<dyn FnOnce()>) -> TimerHandle {
        Self::warn("set_timeout");
        callback();
        TimerHandle::new(0)
    }

    #[inline]
    fn clear_timeout(&self, _handle: TimerHandle) {
        Self::warn("clear_timeout");
    }

    #[inline]
    fn announce(&self, _message: &str) {
        Self::warn("announce");
    }

    #[inline]
    fn announce_assertive(&self, _message: &str) {
        Self::warn("announce_assertive");
    }

    #[inline]
    fn position_element_at(&self, _id: &str, _x: f64, _y: f64) {
        Self::warn("position_element_at");
    }

    #[inline]
    fn resolved_direction(&self, _id: &str) -> Direction {
        Self::warn("resolved_direction");
        Direction::Ltr
    }

    #[inline]
    fn set_background_inert(&self, _portal_root_id: &str) -> Box<dyn FnOnce()> {
        Self::warn("set_background_inert");
        Box::new(|| {})
    }

    #[inline]
    fn remove_inert_from_siblings(&self, _portal_id: &str) {
        Self::warn("remove_inert_from_siblings");
    }

    #[inline]
    fn scroll_lock_acquire(&self) {
        Self::warn("scroll_lock_acquire");
    }

    #[inline]
    fn scroll_lock_release(&self) {
        Self::warn("scroll_lock_release");
    }

    #[inline]
    fn document_contains_id(&self, _id: &str) -> bool {
        Self::warn("document_contains_id");
        false
    }

    #[inline]
    fn track_pointer_drag(
        &self,
        _on_move: Box<dyn Fn(f64, f64)>,
        _on_up: Box<dyn FnOnce()>,
    ) -> Box<dyn FnOnce()> {
        Self::warn("track_pointer_drag");
        Box::new(|| {})
    }

    #[inline]
    fn active_element_id(&self) -> Option<String> {
        Self::warn("active_element_id");
        None
    }

    #[inline]
    fn attach_focus_trap(
        &self,
        _container_id: &str,
        _on_escape: Box<dyn Fn()>,
    ) -> Box<dyn FnOnce()> {
        Self::warn("attach_focus_trap");
        Box::new(|| {})
    }

    #[inline]
    fn can_restore_focus(&self, _id: &str) -> bool {
        Self::warn("can_restore_focus");
        false
    }

    #[inline]
    fn nearest_focusable_ancestor_id(&self, _id: &str) -> Option<String> {
        Self::warn("nearest_focusable_ancestor_id");
        None
    }

    #[inline]
    fn set_scroll_top(&self, _container_id: &str, _scroll_top: f64) {
        Self::warn("set_scroll_top");
    }

    #[inline]
    fn resize_to_content(&self, _id: &str, _max_height: Option<&str>) {
        Self::warn("resize_to_content");
    }

    #[inline]
    fn on_reduced_motion_change(&self, _callback: Box<dyn Fn(bool)>) -> Box<dyn FnOnce()> {
        Self::warn("on_reduced_motion_change");
        Box::new(|| {})
    }

    #[inline]
    fn is_mac_platform(&self) -> bool {
        Self::warn("is_mac_platform");
        false
    }

    #[inline]
    fn now_ms(&self) -> u64 {
        Self::warn("now_ms");
        0
    }

    #[inline]
    fn get_bounding_rect(&self, _id: &str) -> Option<Rect> {
        Self::warn("get_bounding_rect");
        None
    }

    #[inline]
    fn on_animation_end(&self, _id: &str, callback: Box<dyn FnOnce()>) -> Box<dyn FnOnce()> {
        Self::warn("on_animation_end");
        callback();
        Box::new(|| {})
    }
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, rc::Rc, sync::Arc};
    use core::cell::Cell;

    use super::*;

    // -- TimerHandle ---------------------------------------------------------

    #[test]
    fn timer_handle_round_trip() {
        let handle = TimerHandle::new(42);
        assert_eq!(handle.id(), 42);
    }

    // -- Rect ----------------------------------------------------------------

    #[test]
    fn rect_default_is_zero() {
        let r = Rect::default();
        assert_eq!(r.x, 0.0);
        assert_eq!(r.y, 0.0);
        assert_eq!(r.width, 0.0);
        assert_eq!(r.height, 0.0);
    }

    // -- NullPlatformEffects -------------------------------------------------

    #[test]
    fn null_set_timeout_fires_immediately() {
        let fired = Rc::new(Cell::new(false));
        let fired_clone = Rc::clone(&fired);
        let _handle =
            NullPlatformEffects.set_timeout(1000, Box::new(move || fired_clone.set(true)));
        assert!(fired.get());
    }

    #[test]
    fn null_on_animation_end_fires_immediately() {
        let fired = Rc::new(Cell::new(false));
        let fired_clone = Rc::clone(&fired);
        let _cleanup =
            NullPlatformEffects.on_animation_end("el", Box::new(move || fired_clone.set(true)));
        assert!(fired.get());
    }

    #[test]
    fn null_resolved_direction_returns_ltr() {
        assert_eq!(
            NullPlatformEffects.resolved_direction("any"),
            Direction::Ltr
        );
    }

    #[test]
    fn null_document_contains_id_returns_false() {
        assert!(!NullPlatformEffects.document_contains_id("any"));
    }

    #[test]
    fn null_active_element_id_returns_none() {
        assert!(NullPlatformEffects.active_element_id().is_none());
    }

    #[test]
    fn null_get_bounding_rect_returns_none() {
        assert!(NullPlatformEffects.get_bounding_rect("any").is_none());
    }

    #[test]
    fn null_tabbable_element_ids_returns_empty() {
        assert!(NullPlatformEffects.tabbable_element_ids("any").is_empty());
    }

    #[test]
    fn null_is_mac_platform_returns_false() {
        assert!(!NullPlatformEffects.is_mac_platform());
    }

    #[test]
    fn null_now_ms_returns_zero() {
        assert_eq!(NullPlatformEffects.now_ms(), 0);
    }

    #[test]
    fn null_can_restore_focus_returns_false() {
        assert!(!NullPlatformEffects.can_restore_focus("any"));
    }

    #[test]
    fn null_nearest_focusable_ancestor_returns_none() {
        assert!(
            NullPlatformEffects
                .nearest_focusable_ancestor_id("any")
                .is_none()
        );
    }

    // -- MissingProviderEffects ----------------------------------------------

    #[test]
    fn missing_provider_set_timeout_fires_immediately() {
        let fired = Rc::new(Cell::new(false));
        let fired_clone = Rc::clone(&fired);
        let _handle =
            MissingProviderEffects.set_timeout(500, Box::new(move || fired_clone.set(true)));
        assert!(fired.get());
    }

    #[test]
    fn missing_provider_on_animation_end_fires_immediately() {
        let fired = Rc::new(Cell::new(false));
        let fired_clone = Rc::clone(&fired);
        let _cleanup =
            MissingProviderEffects.on_animation_end("el", Box::new(move || fired_clone.set(true)));
        assert!(fired.get());
    }

    // -- Compile coverage: trait object usage ---------------------------------

    /// Exercises every method on a `&dyn PlatformEffects` to verify the trait
    /// is object-safe and all signatures compile correctly.
    #[test]
    fn platform_effects_trait_object_coverage() {
        let platform: &dyn PlatformEffects = &NullPlatformEffects;

        platform.focus_element_by_id("id");
        platform.focus_first_tabbable("id");
        platform.focus_last_tabbable("id");
        drop(platform.tabbable_element_ids("id"));
        platform.focus_body();

        let _handle = platform.set_timeout(0, Box::new(|| {}));
        platform.clear_timeout(TimerHandle::new(0));

        platform.announce("msg");
        platform.announce_assertive("msg");

        platform.position_element_at("id", 0.0, 0.0);
        let _ = platform.resolved_direction("id");

        let cleanup = platform.set_background_inert("id");
        cleanup();
        platform.remove_inert_from_siblings("id");
        platform.scroll_lock_acquire();
        platform.scroll_lock_release();

        let _ = platform.document_contains_id("id");

        let cleanup = platform.track_pointer_drag(Box::new(|_x, _y| {}), Box::new(|| {}));
        cleanup();

        drop(platform.active_element_id());
        let cleanup = platform.attach_focus_trap("id", Box::new(|| {}));
        cleanup();
        let _ = platform.can_restore_focus("id");
        drop(platform.nearest_focusable_ancestor_id("id"));

        platform.set_scroll_top("id", 0.0);
        platform.resize_to_content("id", None);
        platform.resize_to_content("id", Some("200px"));

        let cleanup = platform.on_reduced_motion_change(Box::new(|_reduced| {}));
        cleanup();
        let _ = platform.is_mac_platform();
        let _ = platform.now_ms();
        let _ = platform.get_bounding_rect("id");

        let cleanup = platform.on_animation_end("id", Box::new(|| {}));
        cleanup();
    }

    /// Verify `MissingProviderEffects` also satisfies the trait as a trait object.
    #[test]
    fn missing_provider_is_object_safe() {
        let platform: &dyn PlatformEffects = &MissingProviderEffects;
        platform.focus_element_by_id("test");
        let _ = platform.resolved_direction("test");
        assert!(!platform.document_contains_id("test"));
    }

    /// Verify both implementations can be stored as `Arc<dyn PlatformEffects>`,
    /// the pattern used by `ArsProvider`.
    #[test]
    fn implementations_work_as_arc_trait_objects() {
        let null = Arc::new(NullPlatformEffects);
        let missing = Arc::new(MissingProviderEffects);

        null.focus_element_by_id("a");
        missing.focus_element_by_id("b");

        assert_eq!(null.now_ms(), 0);
        assert_eq!(missing.now_ms(), 0);
    }

    /// Verify `TimerHandle` is `Copy + Eq + Hash` for use in maps.
    #[test]
    fn timer_handle_is_copy_eq_hash() {
        use alloc::collections::BTreeMap;

        let a = TimerHandle::new(1);
        let b = a; // Copy
        assert_eq!(a, b); // Eq

        // Prove Debug formatting:
        let debug = alloc::format!("{a:?}");
        assert!(debug.contains("TimerHandle"));
        assert!(debug.contains('1'));

        // Hash is proven by the derive; verify Eq works in a collection context:
        let mut map = BTreeMap::<u64, TimerHandle>::new();
        map.insert(a.id(), a);
        assert_eq!(map[&1], a);
    }

    /// Verify `Rect` fields and `Copy` semantics.
    #[test]
    fn rect_fields_and_copy() {
        let r = Rect {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
        };
        let r2 = r; // Copy
        assert_eq!(r, r2);
        assert_eq!(r.x, 10.0);
        assert_eq!(r.y, 20.0);
        assert_eq!(r.width, 100.0);
        assert_eq!(r.height, 50.0);
    }
}
