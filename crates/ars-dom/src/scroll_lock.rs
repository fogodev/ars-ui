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
use std::collections::HashMap;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
// Native targets (Dioxus Desktop) may run components on different threads.
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicU32, Ordering};

// ---------------------------------------------------------------------------
// Global state
// ---------------------------------------------------------------------------

/// Process-global scroll lock depth counter (WASM — single-threaded).
#[cfg(target_arch = "wasm32")]
thread_local! {
    static SCROLL_LOCK_DEPTH: Cell<u32> = Cell::new(0);
    static SCROLL_LOCK_SAVED: RefCell<Option<ScrollLockSavedState>> = RefCell::new(None);
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
#[expect(
    dead_code,
    reason = "fields read only on wasm32; struct kept for platform parity"
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
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(saved);
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
    if prev == 1 {
        if let Some(saved) = SCROLL_LOCK_SAVED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        {
            restore_scroll_state(saved);
        }
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
                    restore_scroll_state(saved);
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
    SCROLL_LOCK_DEPTH.with(|d| d.get())
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
    /// Number of active scroll locks held through this manager.
    lock_count: u32,
    /// Scroll position captured when the first lock was acquired.
    #[expect(
        dead_code,
        reason = "spec-defined field; state managed by global acquire/release"
    )]
    saved_scroll_x: f64,
    /// Scroll position captured when the first lock was acquired.
    #[expect(
        dead_code,
        reason = "spec-defined field; state managed by global acquire/release"
    )]
    saved_scroll_y: f64,
    /// Original `overflow` style on the body element, restored on last unlock.
    #[expect(
        dead_code,
        reason = "spec-defined field; state managed by global acquire/release"
    )]
    saved_overflow: String,
    /// Tracks which overlay instance owns a lock, keyed by overlay ID.
    /// Prevents double-lock from a single overlay.
    lock_owners: HashMap<String, bool>,
}

impl ScrollLockManager {
    /// Creates a new, unlocked scroll lock manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            lock_count: 0,
            saved_scroll_x: 0.0,
            saved_scroll_y: 0.0,
            saved_overflow: String::new(),
            lock_owners: HashMap::new(),
        }
    }

    /// Acquire a scroll lock for the given overlay.
    ///
    /// The first lock captures the current scroll position and sets
    /// `overflow: hidden` on the document body. Subsequent locks increment
    /// the counter without changing scroll state. Duplicate lock requests
    /// from the same overlay ID are ignored.
    pub fn lock(&mut self, overlay_id: &str) {
        // Prevent double-lock from the same overlay
        if self.lock_owners.contains_key(overlay_id) {
            return;
        }
        self.lock_owners.insert(overlay_id.to_string(), true);
        self.lock_count += 1;
        acquire();
    }

    /// Release the scroll lock for the given overlay.
    ///
    /// When the last lock is released (count reaches zero), the original
    /// overflow style and scroll position are restored. Unlock requests
    /// for unknown overlay IDs are ignored.
    pub fn unlock(&mut self, overlay_id: &str) {
        if self.lock_owners.remove(overlay_id).is_none() {
            return; // Not locked by this overlay — no-op
        }
        self.lock_count = self.lock_count.saturating_sub(1);
        release();
    }

    /// Returns `true` if scroll locking is currently active through this manager.
    #[must_use]
    pub fn is_locked(&self) -> bool {
        self.lock_count > 0
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

    let window = match web_sys::window() {
        Some(w) => w,
        None => return ScrollLockSavedState::default(),
    };
    let document = match window.document() {
        Some(d) => d,
        None => return ScrollLockSavedState::default(),
    };
    let body = match document.body() {
        Some(b) => b,
        None => return ScrollLockSavedState::default(),
    };
    let doc_el = match document.document_element() {
        Some(el) => el,
        None => return ScrollLockSavedState::default(),
    };

    let body_style = body.style();
    let doc_el_html: &web_sys::HtmlElement = match doc_el.dyn_ref() {
        Some(el) => el,
        None => return ScrollLockSavedState::default(),
    };
    let html_style = doc_el_html.style();

    let scroll_x = window.scroll_x().unwrap_or(0.0);
    let scroll_y = window.scroll_y().unwrap_or(0.0);

    ScrollLockSavedState {
        overflow: body_style
            .get_property_value("overflow")
            .unwrap_or_default(),
        padding_right: body_style
            .get_property_value("padding-right")
            .unwrap_or_default(),
        scroll_x,
        scroll_y,
        body_top: body_style.get_property_value("top").unwrap_or_default(),
        html_overflow: html_style
            .get_property_value("overflow")
            .unwrap_or_default(),
        overscroll_behavior: body_style
            .get_property_value("overscroll-behavior")
            .unwrap_or_default(),
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn apply_scroll_lock() {
    use wasm_bindgen::JsCast;

    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };
    let body = match document.body() {
        Some(b) => b,
        None => return,
    };
    let doc_el = match document.document_element() {
        Some(el) => el,
        None => return,
    };

    let body_style = body.style();

    if needs_ios_workaround() {
        // Tier 2 — iOS Safari fallback: position:fixed on <body>
        let scroll_y = window.scroll_y().unwrap_or(0.0);
        let _ = body_style.set_property("position", "fixed");
        let _ = body_style.set_property("top", &format!("-{scroll_y}px"));
        let _ = body_style.set_property("width", "100%");
        let _ = body_style.set_property("overflow", "hidden");
    } else {
        // Tier 1 — Modern browsers: overflow:clip on <html> + overscroll-behavior:contain on <body>
        let doc_el_html: &web_sys::HtmlElement = match doc_el.dyn_ref() {
            Some(el) => el,
            None => return,
        };
        let _ = doc_el_html.style().set_property("overflow", "clip");
        let _ = body_style.set_property("overscroll-behavior", "contain");
    }

    // Scrollbar width compensation
    apply_scrollbar_compensation();
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn restore_scroll_state(saved: ScrollLockSavedState) {
    use wasm_bindgen::JsCast;

    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };
    let body = match document.body() {
        Some(b) => b,
        None => return,
    };
    let doc_el = match document.document_element() {
        Some(el) => el,
        None => return,
    };

    let body_style = body.style();

    if needs_ios_workaround() {
        // Restore iOS fixed-position overrides
        let _ = body_style.remove_property("position");
        let _ = body_style.remove_property("width");
        let _ = body_style.set_property("top", &saved.body_top);
        let _ = body_style.set_property("overflow", &saved.overflow);
        // Restore scroll position after removing fixed positioning
        window.scroll_to_with_x_and_y(saved.scroll_x, saved.scroll_y);
    } else {
        // Restore tiered strategy styles
        let doc_el_html: &web_sys::HtmlElement = match doc_el.dyn_ref() {
            Some(el) => el,
            None => return,
        };
        let _ = doc_el_html
            .style()
            .set_property("overflow", &saved.html_overflow);
        let _ = body_style.set_property("overscroll-behavior", &saved.overscroll_behavior);
    }

    // Remove scrollbar compensation
    let _ = body_style.set_property("padding-right", &saved.padding_right);
}

/// Detect whether the iOS scroll lock workaround is needed.
///
/// Returns `true` on iOS Safari and iOS WebView.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn needs_ios_workaround() -> bool {
    web_sys::window()
        .and_then(|w| {
            let nav = w.navigator();
            let ua = nav.user_agent().ok()?;
            let platform = nav.platform().ok().unwrap_or_default();
            let max_touch = nav.max_touch_points();

            // Direct iOS user-agent detection
            if ua.contains("iPhone") || ua.contains("iPad") || ua.contains("iPod") {
                return Some(true);
            }
            // iOS 13+ iPad reports as MacIntel but has touch support.
            // Use > 1 to match Platform::detect() in spec — MacBooks report 1.
            if platform == "MacIntel" && max_touch > 1 {
                return Some(true);
            }
            Some(false)
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
    let window = match web_sys::window() {
        Some(w) => w,
        None => return 0.0,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return 0.0,
    };
    let doc_el = match document.document_element() {
        Some(el) => el,
        None => return 0.0,
    };
    // inner_width includes scrollbar; client_width excludes it
    let inner = window
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let client = doc_el.client_width() as f64;
    (inner - client).max(0.0)
}

/// Apply scrollbar width compensation to the body element.
///
/// Adds `padding-right` equal to the scrollbar width to prevent layout shift
/// when the scrollbar disappears. Skips compensation if `scrollbar-gutter: stable`
/// is already in effect.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn apply_scrollbar_compensation() {
    let width = scrollbar_width();
    if width > 0.0 {
        if let Some(body) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.body())
        {
            let _ = body
                .style()
                .set_property("padding-right", &format!("{width}px"));
        }
    }
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
/// Must be called at the start of each test to prevent cross-test interference
/// from the process-global `AtomicU32` depth counter.
#[cfg(test)]
pub(crate) fn reset_for_testing() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        SCROLL_LOCK_DEPTH.store(0, Ordering::SeqCst);
        *SCROLL_LOCK_SAVED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
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
    use super::*;

    // TODO: DOM-touching helpers (save_current_scroll_state, apply_scroll_lock,
    // restore_scroll_state, needs_ios_workaround, scrollbar_width, scrollbar
    // compensation) are only compiled on wasm32+web and cannot be tested here.
    // Add wasm-pack + headless browser tests when the test infrastructure
    // supports it (see spec/testing/15-test-harness.md).

    // == Low-level depth counter =============================================

    #[test]
    fn depth_starts_at_zero() {
        reset_for_testing();
        assert_eq!(depth(), 0);
    }

    #[test]
    fn acquire_increments_depth() {
        reset_for_testing();
        acquire();
        assert_eq!(depth(), 1);
    }

    #[test]
    fn release_decrements_depth() {
        reset_for_testing();
        acquire();
        release();
        assert_eq!(depth(), 0);
    }

    #[test]
    fn release_at_zero_saturates() {
        reset_for_testing();
        release();
        assert_eq!(depth(), 0, "depth must not underflow");
    }

    #[test]
    fn is_locked_reflects_depth() {
        reset_for_testing();
        assert!(!is_locked());
        acquire();
        assert!(is_locked());
        release();
        assert!(!is_locked());
    }

    #[test]
    fn nested_acquire_release_sequence() {
        reset_for_testing();
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
        reset_for_testing();
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
        reset_for_testing();
        let mgr = ScrollLockManager::new();
        assert!(!mgr.is_locked());
    }

    #[test]
    fn manager_lock_makes_locked() {
        reset_for_testing();
        let mut mgr = ScrollLockManager::new();
        mgr.lock("dialog-1");
        assert!(mgr.is_locked());
    }

    #[test]
    fn manager_unlock_makes_unlocked() {
        reset_for_testing();
        let mut mgr = ScrollLockManager::new();
        mgr.lock("dialog-1");
        mgr.unlock("dialog-1");
        assert!(!mgr.is_locked());
    }

    #[test]
    fn manager_duplicate_lock_ignored() {
        reset_for_testing();
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
        reset_for_testing();
        let mut mgr = ScrollLockManager::new();
        mgr.lock("dialog-1");
        mgr.unlock("unknown"); // no-op
        assert!(mgr.is_locked());
        assert_eq!(depth(), 1);
    }

    #[test]
    fn manager_multiple_owners() {
        reset_for_testing();
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
        reset_for_testing();
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
        reset_for_testing();
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
        reset_for_testing();
        prevent_scroll();
        assert_eq!(depth(), 1);
        assert!(is_locked());
    }

    #[test]
    fn restore_scroll_decrements_depth() {
        reset_for_testing();
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
        reset_for_testing();
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
        reset_for_testing();
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
        reset_for_testing();
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
        reset_for_testing();
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
        reset_for_testing();
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
        reset_for_testing();
        let mgr = ScrollLockManager::default();
        assert!(!mgr.is_locked());
    }

    #[test]
    fn manager_debug_format_contains_type_name() {
        reset_for_testing();
        let mgr = ScrollLockManager::new();
        let debug = format!("{mgr:?}");
        assert!(debug.contains("ScrollLockManager"));
    }
}
