//! Reference-counted scroll locking for modal overlays.
//!
//! When a modal overlay (Dialog, `AlertDialog`, Drawer) opens, page scrolling is
//! prevented. A depth counter handles nested modals: styles are applied when the
//! first modal opens and restored only when the last modal closes.
//!
//! Two API levels are provided:
//!
//! - **Low-level**: [`acquire()`] / [`release()`] with a global depth counter.
//! - **High-level**: [`ScrollLockManager`] with per-overlay-ID deduplication.
//! - **Aliases**: [`prevent_scroll()`] / [`restore_scroll()`] for ergonomic use.

// ---------------------------------------------------------------------------
// Platform-conditional synchronization primitives
// ---------------------------------------------------------------------------

// WASM targets are single-threaded (no SharedArrayBuffer needed).
#[cfg(target_arch = "wasm32")]
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
// Native targets (Dioxus Desktop) may run components on different threads.
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{
    Mutex, PoisonError,
    atomic::{AtomicU32, Ordering},
};

// ---------------------------------------------------------------------------
// Global state
// ---------------------------------------------------------------------------

// Process-global scroll lock depth counter (WASM — single-threaded).
#[cfg(target_arch = "wasm32")]
thread_local! {
    static SCROLL_LOCK_DEPTH: Cell<u32> = const { Cell::new(0) };
    static SCROLL_LOCK_SAVED: RefCell<Option<ScrollLockSavedState>> = const { RefCell::new(None) };
}

/// Process-global scroll lock depth counter (native — multi-threaded).
///
/// **Threading model exception:** Uses `AtomicU32` and `Mutex` instead of the
/// crate's usual `Rc<RefCell<>>` pattern because Dioxus Desktop runs the event
/// loop and rendering on separate threads.
#[cfg(not(target_arch = "wasm32"))]
static SCROLL_LOCK_DEPTH: AtomicU32 = AtomicU32::new(0);

/// Saved scroll/style state from before the first lock was acquired.
/// Protected by `Mutex` since multiple threads may acquire/release on native.
#[cfg(not(target_arch = "wasm32"))]
static SCROLL_LOCK_SAVED: Mutex<Option<ScrollLockSavedState>> = Mutex::new(None);

// ---------------------------------------------------------------------------
// Saved state
// ---------------------------------------------------------------------------

/// Captured style and scroll state saved before the first lock is applied.
/// Restored when the last lock is released.
///
/// All fields are read on `wasm32` targets inside [`restore_scroll_state`].
/// On native/test targets the struct is created but never destructured
/// (restore is a no-op), so the fields appear unused to the compiler.
#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(
        dead_code,
        reason = "fields read only on wasm32; struct kept for platform parity"
    )
)]
struct ScrollLockSavedState {
    /// Original `overflow` style on the body element.
    overflow: String,

    /// Original `padding-right` on the body element.
    padding_right: String,

    /// Saved horizontal scroll position.
    scroll_x: f64,

    /// Saved vertical scroll position (iOS restore).
    scroll_y: f64,

    /// Original `top` on the body element (iOS workaround).
    body_top: String,

    /// Original `overflow` on the `<html>` element (tiered strategy).
    html_overflow: String,

    /// Original `overscroll-behavior` on the body (tiered strategy).
    overscroll_behavior: String,
}

impl Default for ScrollLockSavedState {
    fn default() -> Self {
        Self {
            overflow: String::new(),
            padding_right: String::new(),
            scroll_x: 0.0,
            scroll_y: 0.0,
            body_top: String::new(),
            html_overflow: String::new(),
            overscroll_behavior: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Low-level API: acquire / release / is_locked / depth
// ---------------------------------------------------------------------------

/// Increment the scroll lock depth counter.
///
/// On the first call (depth 0 → 1), captures the current scroll position and
/// body styles, then applies scroll lock styles. Subsequent calls only increment
/// the counter.
#[cfg(not(target_arch = "wasm32"))]
pub fn acquire() {
    let prev = SCROLL_LOCK_DEPTH.fetch_add(1, Ordering::SeqCst);

    if prev == 0 {
        let saved = save_current_scroll_state();

        *SCROLL_LOCK_SAVED
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(saved);

        apply_scroll_lock();
    }
}

/// Increment the scroll lock depth counter (WASM variant).
///
/// On the first call (depth 0 → 1), captures the current scroll position and
/// body styles, then applies scroll lock styles. Subsequent calls only increment
/// the counter.
#[cfg(target_arch = "wasm32")]
pub fn acquire() {
    SCROLL_LOCK_DEPTH.with(|depth| {
        let prev = depth.get();

        depth.set(prev + 1);

        if prev == 0 {
            let saved = save_current_scroll_state();

            SCROLL_LOCK_SAVED.with(|s| s.borrow_mut().replace(saved));

            apply_scroll_lock();
        }
    });
}

/// Decrement the scroll lock depth counter.
///
/// On the last release (depth 1 → 0), restores the original body styles and
/// scroll position. Uses saturating subtraction to prevent underflow.
#[cfg(not(target_arch = "wasm32"))]
pub fn release() {
    let prev = SCROLL_LOCK_DEPTH
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| {
            Some(n.saturating_sub(1))
        })
        .unwrap_or(0);

    if prev == 1
        && let Some(saved) = SCROLL_LOCK_SAVED
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    {
        restore_scroll_state(saved);
    }
}

/// Decrement the scroll lock depth counter (WASM variant).
///
/// On the last release (depth 1 → 0), restores the original body styles and
/// scroll position. Uses saturating subtraction to prevent underflow.
#[cfg(target_arch = "wasm32")]
pub fn release() {
    SCROLL_LOCK_DEPTH.with(|depth| {
        let prev = depth.get();

        depth.set(prev.saturating_sub(1));

        if prev == 1 {
            SCROLL_LOCK_SAVED.with(|s| {
                if let Some(saved) = s.borrow_mut().take() {
                    restore_scroll_state(&saved);
                }
            });
        }
    });
}

/// Returns `true` if scroll locking is currently active (depth > 0).
#[cfg(not(target_arch = "wasm32"))]
pub fn is_locked() -> bool {
    SCROLL_LOCK_DEPTH.load(Ordering::SeqCst) > 0
}

/// Returns `true` if scroll locking is currently active (depth > 0).
#[cfg(target_arch = "wasm32")]
pub fn is_locked() -> bool {
    SCROLL_LOCK_DEPTH.with(|d| d.get() > 0)
}

/// Returns the current nesting depth of active scroll locks.
#[cfg(not(target_arch = "wasm32"))]
pub fn depth() -> u32 {
    SCROLL_LOCK_DEPTH.load(Ordering::SeqCst)
}

/// Returns the current nesting depth of active scroll locks.
#[cfg(target_arch = "wasm32")]
pub fn depth() -> u32 {
    SCROLL_LOCK_DEPTH.with(Cell::get)
}

// ---------------------------------------------------------------------------
// Public aliases
// ---------------------------------------------------------------------------

/// Prevent page scrolling. Call when a modal overlay opens.
///
/// Alias for [`acquire()`] — increments the depth counter and applies lock
/// styles on the first call.
pub fn prevent_scroll() {
    acquire();
}

/// Restore page scrolling. Call when a modal overlay closes.
///
/// Alias for [`release()`] — decrements the depth counter and restores styles
/// when the last overlay closes.
pub fn restore_scroll() {
    release();
}

// ---------------------------------------------------------------------------
// ScrollLockManager (high-level, reference-counted with owner tracking)
// ---------------------------------------------------------------------------

/// Manages document scroll locking with reference counting.
///
/// Ensures that nested overlays correctly coordinate scroll lock/unlock.
/// Each overlay acquires a lock by its unique ID; duplicate lock requests
/// from the same overlay are ignored. Scroll styles are applied when the
/// first overlay locks and restored when the last overlay unlocks.
///
/// Delegates to the global [`acquire()`] / [`release()`] depth counter
/// internally, so both the manager-based API and the bare function API
/// share a single global depth counter.
#[derive(Debug)]
pub struct ScrollLockManager {
    /// Tracks which overlay instances currently own a lock, keyed by overlay ID.
    /// Prevents double-lock from a single overlay.
    lock_owners: HashSet<String>,
}

impl ScrollLockManager {
    /// Creates a new, unlocked scroll lock manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            lock_owners: HashSet::new(),
        }
    }

    /// Acquire a scroll lock for the given overlay.
    ///
    /// The first lock delegates to the global scroll-lock state, which captures
    /// the current scroll position and applies the document-level lock.
    /// Subsequent locks only add another owner. Duplicate lock requests from
    /// the same overlay ID are ignored.
    pub fn lock(&mut self, overlay_id: &str) {
        // Prevent double-lock from the same overlay
        if self.lock_owners.contains(overlay_id) {
            return;
        }

        self.lock_owners.insert(overlay_id.to_string());

        acquire();
    }

    /// Release the scroll lock for the given overlay.
    ///
    /// When the last owner is removed, the global scroll-lock state restores
    /// the original document styles and scroll position. Unlock requests for
    /// unknown overlay IDs are ignored.
    pub fn unlock(&mut self, overlay_id: &str) {
        if !self.lock_owners.remove(overlay_id) {
            return; // Not locked by this overlay — no-op
        }

        release();
    }

    /// Returns `true` if scroll locking is currently active through this manager.
    #[must_use]
    pub fn is_locked(&self) -> bool {
        !self.lock_owners.is_empty()
    }
}

impl Default for ScrollLockManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// DOM-touching helpers — web-only implementations
// ---------------------------------------------------------------------------

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn save_current_scroll_state() -> ScrollLockSavedState {
    use wasm_bindgen::JsCast;

    let Some(window) = web_sys::window() else {
        crate::debug::warn_skipped("save_current_scroll_state()", "window");

        return ScrollLockSavedState::default();
    };

    let Some(document) = window.document() else {
        crate::debug::warn_skipped("save_current_scroll_state()", "window.document");

        return ScrollLockSavedState::default();
    };

    let Some(body) = document.body() else {
        crate::debug::warn_skipped("save_current_scroll_state()", "document.body");

        return ScrollLockSavedState::default();
    };

    let Some(doc_el) = document.document_element() else {
        crate::debug::warn_skipped("save_current_scroll_state()", "document.documentElement");

        return ScrollLockSavedState::default();
    };

    let body_style = body.style();

    let doc_el_html: &web_sys::HtmlElement = if let Some(el) = doc_el.dyn_ref() {
        el
    } else {
        crate::debug::warn_message(format_args!(
            "save_current_scroll_state() skipped because documentElement is not an HtmlElement"
        ));

        return ScrollLockSavedState::default();
    };

    let html_style = doc_el_html.style();

    let scroll_x = crate::debug::dom_value_or_default(
        "reading window.scrollX during scroll lock capture",
        window.scroll_x(),
    );

    let scroll_y = crate::debug::dom_value_or_default(
        "reading window.scrollY during scroll lock capture",
        window.scroll_y(),
    );

    ScrollLockSavedState {
        overflow: crate::debug::dom_value_or_default(
            "reading body overflow during scroll lock capture",
            body_style.get_property_value("overflow"),
        ),
        padding_right: crate::debug::dom_value_or_default(
            "reading body padding-right during scroll lock capture",
            body_style.get_property_value("padding-right"),
        ),
        scroll_x,
        scroll_y,
        body_top: crate::debug::dom_value_or_default(
            "reading body top during scroll lock capture",
            body_style.get_property_value("top"),
        ),
        html_overflow: crate::debug::dom_value_or_default(
            "reading html overflow during scroll lock capture",
            html_style.get_property_value("overflow"),
        ),
        overscroll_behavior: crate::debug::dom_value_or_default(
            "reading body overscroll-behavior during scroll lock capture",
            body_style.get_property_value("overscroll-behavior"),
        ),
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn apply_scroll_lock() {
    use wasm_bindgen::JsCast;

    let Some(window) = web_sys::window() else {
        crate::debug::warn_skipped("apply_scroll_lock()", "window");

        return;
    };

    let Some(document) = window.document() else {
        crate::debug::warn_skipped("apply_scroll_lock()", "window.document");

        return;
    };

    let Some(body) = document.body() else {
        crate::debug::warn_skipped("apply_scroll_lock()", "document.body");

        return;
    };

    let Some(doc_el) = document.document_element() else {
        crate::debug::warn_skipped("apply_scroll_lock()", "document.documentElement");

        return;
    };

    let body_style = body.style();

    if needs_ios_workaround() {
        // Tier 2 — iOS Safari fallback: position:fixed on <body>
        let scroll_y = crate::debug::dom_value_or_default(
            "reading window.scrollY during scroll lock application",
            window.scroll_y(),
        );

        crate::debug::warn_dom_error(
            "setting body position during scroll lock application",
            body_style.set_property("position", "fixed"),
        );

        crate::debug::warn_dom_error(
            "setting body top during scroll lock application",
            body_style.set_property("top", &format!("-{scroll_y}px")),
        );

        crate::debug::warn_dom_error(
            "setting body width during scroll lock application",
            body_style.set_property("width", "100%"),
        );

        crate::debug::warn_dom_error(
            "setting body overflow during scroll lock application",
            body_style.set_property("overflow", "hidden"),
        );
    } else {
        // Tier 1 — Modern browsers: overflow:clip on <html> + overscroll-behavior:contain on <body>
        let doc_el_html: &web_sys::HtmlElement = if let Some(el) = doc_el.dyn_ref() {
            el
        } else {
            crate::debug::warn_message(format_args!(
                "apply_scroll_lock() skipped because documentElement is not an HtmlElement"
            ));

            return;
        };

        crate::debug::warn_dom_error(
            "setting html overflow during scroll lock application",
            doc_el_html.style().set_property("overflow", "clip"),
        );

        crate::debug::warn_dom_error(
            "setting body overscroll-behavior during scroll lock application",
            body_style.set_property("overscroll-behavior", "contain"),
        );
    }

    // Scrollbar width compensation
    apply_scrollbar_compensation();
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn restore_scroll_state(saved: &ScrollLockSavedState) {
    use wasm_bindgen::JsCast;

    let Some(window) = web_sys::window() else {
        crate::debug::warn_skipped("restore_scroll_state()", "window");

        return;
    };

    let Some(document) = window.document() else {
        crate::debug::warn_skipped("restore_scroll_state()", "window.document");

        return;
    };

    let Some(body) = document.body() else {
        crate::debug::warn_skipped("restore_scroll_state()", "document.body");

        return;
    };

    let Some(doc_el) = document.document_element() else {
        crate::debug::warn_skipped("restore_scroll_state()", "document.documentElement");

        return;
    };

    let body_style = body.style();

    if needs_ios_workaround() {
        // Restore iOS fixed-position overrides
        crate::debug::warn_dom_error(
            "removing body position during scroll lock restore",
            body_style.remove_property("position").map(|_| ()),
        );

        crate::debug::warn_dom_error(
            "removing body width during scroll lock restore",
            body_style.remove_property("width").map(|_| ()),
        );

        crate::debug::warn_dom_error(
            "restoring body top during scroll lock restore",
            body_style.set_property("top", &saved.body_top),
        );

        crate::debug::warn_dom_error(
            "restoring body overflow during scroll lock restore",
            body_style.set_property("overflow", &saved.overflow),
        );

        // Restore scroll position after removing fixed positioning
        window.scroll_to_with_x_and_y(saved.scroll_x, saved.scroll_y);
    } else {
        // Restore tiered strategy styles
        let doc_el_html: &web_sys::HtmlElement = if let Some(el) = doc_el.dyn_ref() {
            el
        } else {
            crate::debug::warn_message(format_args!(
                "restore_scroll_state() skipped because documentElement is not an HtmlElement"
            ));

            return;
        };

        crate::debug::warn_dom_error(
            "restoring html overflow during scroll lock restore",
            doc_el_html
                .style()
                .set_property("overflow", &saved.html_overflow),
        );

        crate::debug::warn_dom_error(
            "restoring body overscroll-behavior during scroll lock restore",
            body_style.set_property("overscroll-behavior", &saved.overscroll_behavior),
        );
    }

    // Remove scrollbar compensation
    crate::debug::warn_dom_error(
        "restoring body padding-right during scroll lock restore",
        body_style.set_property("padding-right", &saved.padding_right),
    );
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
#[must_use]
fn is_ios_workaround_platform(ua: &str, platform: &str, max_touch: i32) -> bool {
    ua.contains("iPhone")
        || ua.contains("iPad")
        || ua.contains("iPod")
        || (platform == "MacIntel" && max_touch > 1)
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
#[must_use]
fn measured_scrollbar_width(inner_width: Option<f64>, client_width: i32) -> f64 {
    inner_width.map_or(0.0, |inner| (inner - f64::from(client_width)).max(0.0))
}

#[cfg(any(test, all(feature = "web", target_arch = "wasm32")))]
#[must_use]
fn scrollbar_compensation_padding(width: f64) -> Option<String> {
    (width > 0.0).then(|| format!("{width}px"))
}

/// Detect whether the iOS scroll lock workaround is needed.
///
/// Returns `true` on iOS Safari and iOS `WebView`.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn needs_ios_workaround() -> bool {
    web_sys::window()
        .and_then(|w| {
            let nav = w.navigator();

            let ua = nav.user_agent().ok()?;

            let platform = nav.platform().ok().unwrap_or_default();

            Some(is_ios_workaround_platform(
                &ua,
                &platform,
                nav.max_touch_points(),
            ))
        })
        .unwrap_or(false)
}

/// Measure the width of the vertical scrollbar by comparing the window's
/// inner width to the document element's client width.
///
/// Returns 0 on platforms with overlay scrollbars (macOS, iOS, Android).
/// Returns the scrollbar width in pixels on platforms with classic scrollbars
/// (typically 15–17 px on Windows/Linux).
///
/// This measurement is performed live and accounts for zoom level.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn scrollbar_width() -> f64 {
    let Some(window) = web_sys::window() else {
        crate::debug::warn_skipped("scrollbar_width()", "window");

        return 0.0;
    };

    let Some(document) = window.document() else {
        crate::debug::warn_skipped("scrollbar_width()", "window.document");

        return 0.0;
    };

    let Some(doc_el) = document.document_element() else {
        crate::debug::warn_skipped("scrollbar_width()", "document.documentElement");

        return 0.0;
    };

    // inner_width includes scrollbar; client_width excludes it
    measured_scrollbar_width(
        crate::debug::dom_value_or_default(
            "reading window.innerWidth for scrollbar_width()",
            window.inner_width(),
        )
        .as_f64(),
        doc_el.client_width(),
    )
}

/// Apply scrollbar width compensation to the body element.
///
/// Adds `padding-right` equal to the scrollbar width to prevent layout shift
/// when the scrollbar disappears. Skips compensation if `scrollbar-gutter: stable`
/// is already in effect.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn apply_scrollbar_compensation() {
    let Some(padding_right) = scrollbar_compensation_padding(scrollbar_width()) else {
        return;
    };

    let Some(body) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.body())
    else {
        crate::debug::warn_skipped("apply_scrollbar_compensation()", "document.body");

        return;
    };

    crate::debug::warn_dom_error(
        "setting body padding-right during scrollbar compensation",
        body.style().set_property("padding-right", &padding_right),
    );
}

// ---------------------------------------------------------------------------
// DOM-touching helpers — non-web stubs (native / SSR / test)
// ---------------------------------------------------------------------------

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn save_current_scroll_state() -> ScrollLockSavedState {
    ScrollLockSavedState::default()
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn apply_scroll_lock() {
    // No-op: desktop windowing systems handle modal behavior natively.
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn restore_scroll_state(_saved: ScrollLockSavedState) {
    // No-op: nothing to restore on non-web targets.
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Reset global scroll lock state for test isolation.
///
/// Zeroes the depth counter and clears saved state. On native targets this
/// touches the process-global `AtomicU32` / `Mutex`, so callers MUST hold
/// `TEST_SERIAL` to prevent interference from concurrent tests.
#[cfg(test)]
fn reset_global_state() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        SCROLL_LOCK_DEPTH.store(0, Ordering::SeqCst);

        *SCROLL_LOCK_SAVED
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = None;
    }

    #[cfg(target_arch = "wasm32")]
    {
        SCROLL_LOCK_DEPTH.with(|d| d.set(0));
        SCROLL_LOCK_SAVED.with(|s| *s.borrow_mut() = None);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard, PoisonError};

    use super::*;

    /// Serializes `scroll_lock` tests so they don't run in parallel.
    ///
    /// On native targets the depth counter is a process-global `AtomicU32`.
    /// Without serialization, concurrent test threads would stomp on each
    /// other's counter values. Each test must call [`serial_reset()`] which
    /// acquires this lock and zeroes global state before the test body runs.
    /// The returned guard holds the lock for the test's lifetime.
    static TEST_SERIAL: Mutex<()> = Mutex::new(());

    /// Acquire the serialization lock and reset global state.
    ///
    /// Returns a [`MutexGuard`] that keeps the lock held until dropped
    /// (end of the calling test). This ensures no two `scroll_lock` tests
    /// touch the global depth counter concurrently.
    fn serial_reset() -> MutexGuard<'static, ()> {
        let guard = TEST_SERIAL.lock().unwrap_or_else(PoisonError::into_inner);

        reset_global_state();

        guard
    }

    // == Low-level depth counter =============================================

    #[test]
    fn depth_starts_at_zero() {
        let _g = serial_reset();

        assert_eq!(depth(), 0);
    }

    #[test]
    fn acquire_increments_depth() {
        let _g = serial_reset();

        acquire();

        assert_eq!(depth(), 1);
    }

    #[test]
    fn release_decrements_depth() {
        let _g = serial_reset();

        acquire();

        release();

        assert_eq!(depth(), 0);
    }

    #[test]
    fn release_at_zero_saturates() {
        let _g = serial_reset();

        release();

        assert_eq!(depth(), 0, "depth must not underflow");
    }

    #[test]
    fn is_locked_reflects_depth() {
        let _g = serial_reset();

        assert!(!is_locked());

        acquire();

        assert!(is_locked());

        release();

        assert!(!is_locked());
    }

    #[test]
    fn nested_acquire_release_sequence() {
        let _g = serial_reset();

        acquire();

        assert_eq!(depth(), 1);

        acquire();

        assert_eq!(depth(), 2);

        acquire();

        assert_eq!(depth(), 3);

        release();

        assert_eq!(depth(), 2);

        release();

        assert_eq!(depth(), 1);

        release();

        assert_eq!(depth(), 0);
    }

    #[test]
    fn depth_returns_current_count() {
        let _g = serial_reset();

        for expected in 1..=5 {
            acquire();

            assert_eq!(depth(), expected);
        }

        for expected in (0..5).rev() {
            release();

            assert_eq!(depth(), expected);
        }
    }

    // == ScrollLockManager ===================================================

    #[test]
    fn manager_new_is_unlocked() {
        let _g = serial_reset();

        let mgr = ScrollLockManager::new();

        assert!(!mgr.is_locked());
    }

    #[test]
    fn manager_lock_makes_locked() {
        let _g = serial_reset();

        let mut mgr = ScrollLockManager::new();

        mgr.lock("dialog-1");

        assert!(mgr.is_locked());
    }

    #[test]
    fn manager_unlock_makes_unlocked() {
        let _g = serial_reset();

        let mut mgr = ScrollLockManager::new();

        mgr.lock("dialog-1");
        mgr.unlock("dialog-1");

        assert!(!mgr.is_locked());
    }

    #[test]
    fn manager_duplicate_lock_ignored() {
        let _g = serial_reset();

        let mut mgr = ScrollLockManager::new();

        mgr.lock("dialog-1");
        mgr.lock("dialog-1"); // duplicate — should be ignored

        assert_eq!(depth(), 1, "duplicate lock must not increment global depth");

        mgr.unlock("dialog-1");

        assert!(!mgr.is_locked());
        assert_eq!(depth(), 0);
    }

    #[test]
    fn manager_unlock_unknown_id_ignored() {
        let _g = serial_reset();

        let mut mgr = ScrollLockManager::new();

        mgr.lock("dialog-1");
        mgr.unlock("unknown"); // no-op

        assert!(mgr.is_locked());
        assert_eq!(depth(), 1);
    }

    #[test]
    fn manager_multiple_owners() {
        let _g = serial_reset();

        let mut mgr = ScrollLockManager::new();

        mgr.lock("dialog-a");
        mgr.lock("dialog-b");

        assert_eq!(depth(), 2);

        mgr.unlock("dialog-a");

        assert!(mgr.is_locked(), "still locked by dialog-b");
        assert_eq!(depth(), 1);
    }

    #[test]
    fn manager_last_unlock_clears() {
        let _g = serial_reset();

        let mut mgr = ScrollLockManager::new();

        mgr.lock("dialog-a");
        mgr.lock("dialog-b");
        mgr.lock("dialog-c");

        mgr.unlock("dialog-b");
        mgr.unlock("dialog-a");
        mgr.unlock("dialog-c");

        assert!(!mgr.is_locked());
        assert_eq!(depth(), 0);
    }

    #[test]
    fn manager_lock_unlock_delegates_to_global_depth() {
        let _g = serial_reset();

        let mut mgr = ScrollLockManager::new();

        assert_eq!(depth(), 0);

        mgr.lock("overlay-1");

        assert_eq!(depth(), 1);

        mgr.lock("overlay-2");

        assert_eq!(depth(), 2);

        mgr.unlock("overlay-1");

        assert_eq!(depth(), 1);

        mgr.unlock("overlay-2");

        assert_eq!(depth(), 0);
    }

    // == Public aliases ======================================================

    #[test]
    fn prevent_scroll_increments_depth() {
        let _g = serial_reset();

        prevent_scroll();

        assert_eq!(depth(), 1);
        assert!(is_locked());
    }

    #[test]
    fn restore_scroll_decrements_depth() {
        let _g = serial_reset();

        prevent_scroll();

        restore_scroll();

        assert_eq!(depth(), 0);
        assert!(!is_locked());
    }

    // == Out-of-order unlock (spec §5.5) =====================================

    #[test]
    fn out_of_order_unlock_still_reaches_zero() {
        // Spec §5.5: "If a consumer closes modals out of order, the depth
        // counter still works correctly — styles are only restored when all
        // modals have closed."
        let _g = serial_reset();

        let mut mgr = ScrollLockManager::new();

        mgr.lock("outer");
        mgr.lock("middle");
        mgr.lock("inner");

        assert_eq!(depth(), 3);

        // Close middle first (out of stack order)
        mgr.unlock("middle");

        assert_eq!(depth(), 2);
        assert!(mgr.is_locked());

        // Then inner
        mgr.unlock("inner");

        assert_eq!(depth(), 1);
        assert!(mgr.is_locked());

        // Finally outer — last release
        mgr.unlock("outer");

        assert_eq!(depth(), 0);
        assert!(!mgr.is_locked());
    }

    // == Re-lock after full cycle ============================================

    #[test]
    fn relock_after_full_unlock_cycle() {
        let _g = serial_reset();

        // First cycle
        acquire();
        acquire();

        release();
        release();

        assert_eq!(depth(), 0);

        // Second cycle — must work identically
        acquire();

        assert_eq!(depth(), 1);
        assert!(is_locked());

        release();

        assert_eq!(depth(), 0);
        assert!(!is_locked());
    }

    #[test]
    fn manager_relock_same_id_after_unlock() {
        let _g = serial_reset();

        let mut mgr = ScrollLockManager::new();

        mgr.lock("dialog-1");
        mgr.unlock("dialog-1");

        assert!(!mgr.is_locked());

        // Re-lock with the same ID
        mgr.lock("dialog-1");

        assert!(mgr.is_locked());
        assert_eq!(depth(), 1);
    }

    // == Two independent managers share global state ==========================

    #[test]
    fn two_managers_share_global_depth() {
        // Production pattern: multiple overlay instances each have their own
        // manager, but they share the process-global depth counter.
        let _g = serial_reset();

        let mut mgr_a = ScrollLockManager::new();

        let mut mgr_b = ScrollLockManager::new();

        mgr_a.lock("dialog-a");

        assert_eq!(depth(), 1);

        mgr_b.lock("drawer-b");

        assert_eq!(depth(), 2);

        // Closing dialog-a via mgr_a — mgr_b's lock keeps scroll locked
        mgr_a.unlock("dialog-a");

        assert_eq!(depth(), 1);
        assert!(!mgr_a.is_locked());
        assert!(mgr_b.is_locked());

        // Closing drawer-b via mgr_b — last release
        mgr_b.unlock("drawer-b");

        assert_eq!(depth(), 0);
        assert!(!mgr_b.is_locked());
    }

    // == Mixed bare API + manager API ========================================

    #[test]
    fn bare_acquire_and_manager_lock_share_depth() {
        let _g = serial_reset();

        let mut mgr = ScrollLockManager::new();

        // Bare API locks first
        acquire();

        assert_eq!(depth(), 1);

        // Manager locks second
        mgr.lock("overlay");

        assert_eq!(depth(), 2);

        // Manager unlocks — bare lock still holds
        mgr.unlock("overlay");

        assert_eq!(depth(), 1);
        assert!(is_locked());

        // Bare API releases — fully unlocked
        release();

        assert_eq!(depth(), 0);
        assert!(!is_locked());
    }

    // == Default + Debug =====================================================

    #[test]
    fn manager_default_is_unlocked() {
        let _g = serial_reset();

        let mgr = ScrollLockManager::default();

        assert!(!mgr.is_locked());
    }

    #[test]
    fn manager_debug_format_contains_type_name() {
        let _g = serial_reset();

        let mgr = ScrollLockManager::new();

        let debug = format!("{mgr:?}");

        assert!(debug.contains("ScrollLockManager"));
    }

    #[test]
    fn ios_workaround_helper_matches_platform_rules() {
        assert!(is_ios_workaround_platform(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)",
            "iPhone",
            5,
        ));
        assert!(is_ios_workaround_platform(
            "Mozilla/5.0 (iPad; CPU OS 18_0 like Mac OS X)",
            "iPad",
            5,
        ));
        assert!(is_ios_workaround_platform(
            "Mozilla/5.0 (iPod touch; CPU iPhone OS 18_0 like Mac OS X)",
            "iPod",
            5,
        ));
        assert!(is_ios_workaround_platform(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)",
            "MacIntel",
            5,
        ));
        assert!(!is_ios_workaround_platform(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)",
            "MacIntel",
            1,
        ));
        assert!(!is_ios_workaround_platform(
            "Mozilla/5.0 (X11; Linux x86_64)",
            "Linux x86_64",
            0,
        ));
    }

    #[test]
    fn measured_scrollbar_width_clamps_negative_and_missing_values() {
        assert_eq!(measured_scrollbar_width(Some(1200.0), 1184), 16.0);
        assert_eq!(measured_scrollbar_width(Some(1000.0), 1000), 0.0);
        assert_eq!(measured_scrollbar_width(Some(800.0), 900), 0.0);
        assert_eq!(measured_scrollbar_width(None, 900), 0.0);
    }

    #[test]
    fn scrollbar_compensation_padding_skips_zero_width_and_formats_positive_width() {
        assert_eq!(scrollbar_compensation_padding(0.0), None);
        assert_eq!(scrollbar_compensation_padding(-4.0), None);
        assert_eq!(
            scrollbar_compensation_padding(16.0),
            Some(String::from("16px"))
        );
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
    use web_sys::{Document, HtmlElement, Window};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn window() -> Window {
        web_sys::window().expect("window must exist in browser tests")
    }

    fn document() -> Document {
        window()
            .document()
            .expect("document must exist in browser tests")
    }

    fn body() -> HtmlElement {
        document().body().expect("body must exist in browser tests")
    }

    fn html() -> HtmlElement {
        document()
            .document_element()
            .expect("document element must exist")
            .dyn_into::<HtmlElement>()
            .expect("document element must be HtmlElement")
    }

    fn snapshot_inline_styles() -> (Option<String>, Option<String>) {
        (body().get_attribute("style"), html().get_attribute("style"))
    }

    fn restore_inline_styles(snapshot: (Option<String>, Option<String>)) {
        let (body_style, html_style) = snapshot;

        let body = body();
        let html = html();

        if let Some(style) = body_style {
            body.set_attribute("style", &style)
                .expect("restoring body style attribute must succeed");
        } else {
            body.remove_attribute("style")
                .expect("removing body style attribute must succeed");
        }

        if let Some(style) = html_style {
            html.set_attribute("style", &style)
                .expect("restoring html style attribute must succeed");
        } else {
            html.remove_attribute("style")
                .expect("removing html style attribute must succeed");
        }
    }

    fn set_style_property(element: &HtmlElement, name: &str, value: &str) {
        element
            .style()
            .set_property(name, value)
            .expect("setting style property must succeed");
    }

    fn clear_scroll_lock_state() {
        reset_global_state();

        window().scroll_to_with_x_and_y(0.0, 0.0);
    }

    fn with_depth_and_saved_state_cleared() {
        SCROLL_LOCK_DEPTH.with(|depth| depth.set(1));
        SCROLL_LOCK_SAVED.with(|saved| *saved.borrow_mut() = None);
    }

    #[wasm_bindgen_test]
    fn save_current_scroll_state_captures_live_styles() {
        let snapshot = snapshot_inline_styles();

        clear_scroll_lock_state();

        let body = body();
        let html = html();

        set_style_property(&body, "overflow", "auto");
        set_style_property(&body, "padding-right", "12px");
        set_style_property(&body, "top", "7px");
        set_style_property(&body, "overscroll-behavior", "contain");
        set_style_property(&html, "overflow", "clip");

        let saved = save_current_scroll_state();

        assert_eq!(saved.overflow, "auto");
        assert_eq!(saved.padding_right, "12px");
        assert_eq!(saved.body_top, "7px");
        assert_eq!(saved.overscroll_behavior, "contain");
        assert_eq!(saved.html_overflow, "clip");

        restore_inline_styles(snapshot);

        clear_scroll_lock_state();
    }

    #[wasm_bindgen_test]
    fn apply_and_restore_scroll_lock_round_trips_styles() {
        let snapshot = snapshot_inline_styles();

        clear_scroll_lock_state();

        let body = body();
        let html = html();

        set_style_property(&body, "overflow", "auto");
        set_style_property(&body, "padding-right", "9px");
        set_style_property(&body, "top", "11px");
        set_style_property(&body, "overscroll-behavior", "auto");
        set_style_property(&html, "overflow", "visible");

        let saved = save_current_scroll_state();

        apply_scroll_lock();

        let ios = needs_ios_workaround();
        let expected_position = ["", "fixed"][usize::from(ios)];
        let expected_width = ["", "100%"][usize::from(ios)];
        let expected_overflow = ["auto", "hidden"][usize::from(ios)];
        let expected_html_overflow = ["clip", "visible"][usize::from(ios)];
        let expected_overscroll = ["contain", "auto"][usize::from(ios)];

        assert_eq!(
            body.style()
                .get_property_value("position")
                .expect("body position read must succeed"),
            expected_position
        );
        assert_eq!(
            body.style()
                .get_property_value("width")
                .expect("body width read must succeed"),
            expected_width
        );
        assert_eq!(
            body.style()
                .get_property_value("overflow")
                .expect("body overflow read must succeed"),
            expected_overflow
        );
        assert_eq!(
            html.style()
                .get_property_value("overflow")
                .expect("html overflow read must succeed"),
            expected_html_overflow
        );
        assert_eq!(
            body.style()
                .get_property_value("overscroll-behavior")
                .expect("body overscroll read must succeed"),
            expected_overscroll
        );

        let top = body
            .style()
            .get_property_value("top")
            .expect("body top read must succeed");

        assert_eq!(
            (top.starts_with('-'), top.ends_with("px")),
            [(false, true), (true, true)][usize::from(ios)]
        );

        restore_scroll_state(&saved);

        assert_eq!(
            body.style()
                .get_property_value("overflow")
                .expect("body overflow read must succeed"),
            "auto"
        );
        assert_eq!(
            body.style()
                .get_property_value("padding-right")
                .expect("body padding-right read must succeed"),
            "9px"
        );
        assert_eq!(
            body.style()
                .get_property_value("top")
                .expect("body top read must succeed"),
            "11px"
        );
        assert_eq!(
            body.style()
                .get_property_value("overscroll-behavior")
                .expect("body overscroll read must succeed"),
            "auto"
        );
        assert_eq!(
            html.style()
                .get_property_value("overflow")
                .expect("html overflow read must succeed"),
            "visible"
        );

        restore_inline_styles(snapshot);

        clear_scroll_lock_state();
    }

    #[wasm_bindgen_test]
    fn acquire_and_release_manage_saved_state_and_restore_dom() {
        let snapshot = snapshot_inline_styles();

        clear_scroll_lock_state();

        let body = body();
        let html = html();

        set_style_property(&body, "overflow", "auto");
        set_style_property(&body, "padding-right", "6px");
        set_style_property(&body, "top", "4px");
        set_style_property(&body, "overscroll-behavior", "none");
        set_style_property(&html, "overflow", "visible");

        acquire();

        assert_eq!(depth(), 1);
        assert!(is_locked());

        SCROLL_LOCK_SAVED.with(|saved| assert!(saved.borrow().is_some()));

        release();

        assert_eq!(depth(), 0);
        assert!(!is_locked());

        SCROLL_LOCK_SAVED.with(|saved| assert!(saved.borrow().is_none()));

        assert_eq!(
            body.style()
                .get_property_value("overflow")
                .expect("body overflow read must succeed"),
            "auto"
        );
        assert_eq!(
            body.style()
                .get_property_value("padding-right")
                .expect("body padding-right read must succeed"),
            "6px"
        );
        assert_eq!(
            body.style()
                .get_property_value("top")
                .expect("body top read must succeed"),
            "4px"
        );
        assert_eq!(
            body.style()
                .get_property_value("overscroll-behavior")
                .expect("body overscroll read must succeed"),
            "none"
        );
        assert_eq!(
            html.style()
                .get_property_value("overflow")
                .expect("html overflow read must succeed"),
            "visible"
        );

        restore_inline_styles(snapshot);

        clear_scroll_lock_state();
    }

    #[wasm_bindgen_test]
    fn nested_acquire_and_release_cover_intermediate_depth_paths() {
        let snapshot = snapshot_inline_styles();

        clear_scroll_lock_state();

        let body = body();
        let html = html();

        set_style_property(&body, "overflow", "auto");
        set_style_property(&body, "padding-right", "5px");
        set_style_property(&body, "top", "3px");
        set_style_property(&body, "overscroll-behavior", "auto");
        set_style_property(&html, "overflow", "visible");

        acquire();
        acquire();

        assert_eq!(depth(), 2);

        SCROLL_LOCK_SAVED.with(|saved| assert!(saved.borrow().is_some()));

        release();

        assert_eq!(depth(), 1);
        assert!(is_locked());

        SCROLL_LOCK_SAVED.with(|saved| assert!(saved.borrow().is_some()));

        release();

        assert_eq!(depth(), 0);
        assert!(!is_locked());
        assert_eq!(
            body.style()
                .get_property_value("overflow")
                .expect("body overflow read must succeed"),
            "auto"
        );

        restore_inline_styles(snapshot);

        clear_scroll_lock_state();
    }

    #[wasm_bindgen_test]
    fn release_is_safe_when_depth_is_one_but_saved_state_is_missing() {
        clear_scroll_lock_state();

        with_depth_and_saved_state_cleared();

        release();

        assert_eq!(depth(), 0);
        assert!(!is_locked());

        SCROLL_LOCK_SAVED.with(|saved| assert!(saved.borrow().is_none()));

        clear_scroll_lock_state();
    }

    #[wasm_bindgen_test]
    fn runtime_helpers_match_live_browser_measurements() {
        let snapshot = snapshot_inline_styles();

        clear_scroll_lock_state();

        let body = body();

        let navigator = window().navigator();

        let expected_ios = navigator.user_agent().ok().is_some_and(|ua| {
            is_ios_workaround_platform(
                &ua,
                &navigator.platform().ok().unwrap_or_default(),
                navigator.max_touch_points(),
            )
        });

        assert_eq!(needs_ios_workaround(), expected_ios);

        let expected_scrollbar_width = measured_scrollbar_width(
            window().inner_width().ok().and_then(|value| value.as_f64()),
            html().client_width(),
        );

        assert_eq!(scrollbar_width(), expected_scrollbar_width);

        body.remove_attribute("style")
            .expect("removing body style attribute must succeed");

        apply_scrollbar_compensation();

        let expected_padding =
            scrollbar_compensation_padding(expected_scrollbar_width).unwrap_or_default();

        assert_eq!(
            body.style()
                .get_property_value("padding-right")
                .expect("body padding-right read must succeed"),
            expected_padding
        );

        restore_inline_styles(snapshot);

        clear_scroll_lock_state();
    }

    #[wasm_bindgen_test]
    fn helper_platform_rules_are_covered_in_browser_builds() {
        assert!(is_ios_workaround_platform(
            "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)",
            "iPhone",
            5,
        ));
        assert!(is_ios_workaround_platform(
            "Mozilla/5.0 (iPad; CPU OS 18_0 like Mac OS X)",
            "iPad",
            5,
        ));
        assert!(is_ios_workaround_platform(
            "Mozilla/5.0 (iPod touch; CPU iPhone OS 18_0 like Mac OS X)",
            "iPod",
            5,
        ));
        assert!(is_ios_workaround_platform(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)",
            "MacIntel",
            5,
        ));
        assert!(!is_ios_workaround_platform(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)",
            "MacIntel",
            1,
        ));
        assert!(!is_ios_workaround_platform(
            "Mozilla/5.0 (X11; Linux x86_64)",
            "Linux x86_64",
            0,
        ));
    }

    #[wasm_bindgen_test]
    fn apply_scrollbar_compensation_is_noop_when_scrollbar_width_is_zero() {
        let snapshot = snapshot_inline_styles();

        clear_scroll_lock_state();

        let body = body();
        let html = html();

        body.remove_attribute("style")
            .expect("removing body style attribute must succeed");
        html.remove_attribute("style")
            .expect("removing html style attribute must succeed");

        set_style_property(&body, "padding-right", "");
        set_style_property(&body, "overflow", "hidden");
        set_style_property(&html, "overflow", "hidden");

        assert_eq!(scrollbar_width(), 0.0);

        apply_scrollbar_compensation();

        assert_eq!(
            body.style()
                .get_property_value("padding-right")
                .expect("body padding-right read must succeed"),
            ""
        );

        restore_inline_styles(snapshot);

        clear_scroll_lock_state();
    }
}
