//! Z-index allocator for overlay stacking order.
//!
//! Overlay components (Dialog, Popover, Menu, Tooltip, etc.) need predictable
//! stacking. When multiple overlays are open simultaneously — or nested (e.g., a
//! Dialog containing a Menu) — each overlay must render above the previous one.
//!
//! This module provides:
//!
//! - **[`next_z_index()`]**: A thread-local monotonic counter that returns the
//!   next z-index value starting at 1000. Values are never reused.
//! - **[`reset_z_index()`]**: Resets the counter for tests or application teardown.
//! - **[`ZIndexAllocator`]**: A structured wrapper with explicit lifecycle
//!   control (allocate / release / reset) for overlay managers.

use std::cell::{Cell, RefCell};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Starting value for z-index allocation. Application content uses 0–999;
/// the allocator exclusively manages values from 1000 upward.
const Z_INDEX_BASE: u32 = 1000;

/// Maximum z-index value before the counter wraps back to [`Z_INDEX_BASE`].
/// Prevents overflow on very long-running SPAs with many overlay open/close
/// cycles (e.g., tooltips, popovers across thousands of interactions).
const Z_INDEX_CEILING: u32 = u32::MAX - 1000;

// ---------------------------------------------------------------------------
// Thread-local counter
// ---------------------------------------------------------------------------

thread_local! {
    /// Per-thread monotonic z-index counter. Starts at [`Z_INDEX_BASE`] (1000).
    /// Single-threaded per thread: consistent with the library's `Rc`-based,
    /// WASM-first design. On native targets, each thread gets its own counter.
    static NEXT_Z_INDEX: Cell<u32> = const { Cell::new(Z_INDEX_BASE) };
}

// ---------------------------------------------------------------------------
// Free functions
// ---------------------------------------------------------------------------

/// Allocate the next z-index for an overlay.
///
/// Each call returns a monotonically increasing value starting at 1000.
/// The counter is thread-local: each thread maintains its own sequence.
/// Values are never reused — gaps from released overlays are expected
/// and harmless.
///
/// # Overflow protection
///
/// When the counter reaches `Z_INDEX_CEILING` (`u32::MAX - 1000`), it
/// resets to `Z_INDEX_BASE` (1000). Existing overlays at high z-index values
/// still render above normal content; new overlays start a fresh sequence.
///
/// # Examples
///
/// ```
/// use ars_dom::z_index::{next_z_index, reset_z_index};
///
/// reset_z_index(1000);
/// let z1 = next_z_index(); // 1000
/// let z2 = next_z_index(); // 1001
/// assert!(z2 > z1);
/// ```
#[must_use]
pub fn next_z_index() -> u32 {
    NEXT_Z_INDEX.with(|z| {
        let val = z.get();
        if val >= Z_INDEX_CEILING {
            // Returning Z_INDEX_BASE here and storing BASE + 1 keeps the
            // sequence monotonic from the caller's perspective after wrap:
            // this call receives the fresh base and the next call advances.
            // Reset to base — existing overlays at high z-indexes will still
            // render above normal content; new overlays start fresh.
            #[cfg(feature = "debug")]
            log::warn!(
                "[ars-dom] z-index counter reached ceiling ({Z_INDEX_CEILING}), \
                 resetting to base ({Z_INDEX_BASE})"
            );
            z.set(Z_INDEX_BASE + 1);
            Z_INDEX_BASE
        } else {
            z.set(val + 1);
            val
        }
    })
}

/// Reset the z-index counter to a given base value.
///
/// Intended for use in tests to ensure deterministic z-index values,
/// and for application-level teardown (e.g., full-page navigation in an SPA).
///
/// # Examples
///
/// ```
/// use ars_dom::z_index::{next_z_index, reset_z_index};
///
/// reset_z_index(1000);
/// assert_eq!(next_z_index(), 1000);
/// assert_eq!(next_z_index(), 1001);
/// ```
pub fn reset_z_index(base: u32) {
    NEXT_Z_INDEX.with(|z| z.set(base));
}

// ---------------------------------------------------------------------------
// ZIndexAllocator
// ---------------------------------------------------------------------------

/// Structured z-index allocator for managing overlay stacking with explicit
/// lifecycle control.
///
/// Wraps the thread-local [`next_z_index()`] counter and tracks allocated
/// values for bookkeeping. Released values are removed from the tracked set
/// but never reassigned — the counter only moves forward.
///
/// # Usage
///
/// ```
/// use ars_dom::z_index::{ZIndexAllocator, reset_z_index};
///
/// reset_z_index(1000);
/// let allocator = ZIndexAllocator::new();
/// let z1 = allocator.allocate(); // 1000
/// let z2 = allocator.allocate(); // 1001
/// allocator.release(z1);         // removes from tracking, does not reuse
/// let z3 = allocator.allocate(); // 1002 (not 1000)
/// ```
#[derive(Debug)]
pub struct ZIndexAllocator {
    /// Tracked z-index values for lifecycle management. Enables future
    /// compaction strategies when all values in a range are released.
    allocated: RefCell<Vec<u32>>,
}

impl ZIndexAllocator {
    /// Create a new allocator with no tracked allocations.
    ///
    /// Uses the global thread-local counter starting at 1000.
    #[must_use]
    pub fn new() -> Self {
        Self {
            allocated: RefCell::new(Vec::new()),
        }
    }

    /// Allocate the next z-index value.
    ///
    /// Delegates to the thread-local [`next_z_index()`] counter and records
    /// the value for later release. Returns a monotonically increasing `u32`
    /// starting at 1000. Values are never reused — gaps from released
    /// overlays are expected and harmless.
    #[must_use]
    pub fn allocate(&self) -> u32 {
        let z = next_z_index();
        self.allocated.borrow_mut().push(z);
        z
    }

    /// Release a previously allocated z-index.
    ///
    /// Removes the value from the tracked set. Does **not** make the value
    /// available for reuse — the counter only moves forward. Releasing a
    /// value that was never allocated is a no-op.
    pub fn release(&self, z: u32) {
        self.allocated.borrow_mut().retain(|&v| v != z);
    }

    /// Reset the allocator: clear all tracked allocations and reset the
    /// thread-local counter to the base value (1000).
    ///
    /// Intended for tests and application-level teardown (e.g., full-page
    /// navigation in an SPA).
    pub fn reset(&self) {
        self.allocated.borrow_mut().clear();
        reset_z_index(Z_INDEX_BASE);
    }
}

impl Default for ZIndexAllocator {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Top-layer detection
// ---------------------------------------------------------------------------

/// Check whether the browser supports the CSS top-layer (native `<dialog>` or
/// popover API).
///
/// When `true`, overlay components can skip [`next_z_index()`] and rely on the
/// browser's native stacking via `<dialog>.showModal()` or the `popover`
/// attribute. When `false`, components fall back to z-index-based stacking.
///
/// The result is detected once per thread and cached. Subsequent calls return
/// the cached value without DOM access.
///
/// Returns `false` on non-wasm targets and when `window()` is unavailable
/// (SSR, Web Worker).
#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[must_use]
pub fn supports_top_layer() -> bool {
    thread_local! {
        static CACHED: Cell<Option<bool>> = const { Cell::new(None) };
    }

    CACHED.with(|c| {
        if let Some(v) = c.get() {
            return v;
        }

        let supported = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.create_element("dialog").ok())
            .is_some_and(|el| {
                js_sys::Reflect::has(&el, &wasm_bindgen::JsValue::from_str("showModal"))
                    .unwrap_or(false)
            });

        c.set(Some(supported));

        supported
    })
}

/// Check whether the browser supports the CSS top-layer.
///
/// Returns `false` on non-wasm targets — top-layer is a browser-only concept.
#[cfg(not(all(feature = "web", target_arch = "wasm32")))]
#[must_use]
pub const fn supports_top_layer() -> bool {
    false
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Reset the thread-local z-index counter to [`Z_INDEX_BASE`].
/// Called by [`tests::serial_reset()`] before each test.
#[cfg(test)]
fn reset_global_state() {
    NEXT_Z_INDEX.with(|z| z.set(Z_INDEX_BASE));
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard};

    use super::*;

    /// Serializes z-index tests so they don't run in parallel.
    ///
    /// Although `thread_local!` is per-thread, the Rust test runner may reuse
    /// threads across tests. Each test calls [`serial_reset()`] which acquires
    /// this lock and zeroes global state before the test body runs. The
    /// returned guard holds the lock for the test's lifetime.
    static TEST_SERIAL: Mutex<()> = Mutex::new(());

    /// Acquire the serialization lock and reset global state.
    ///
    /// Returns a [`MutexGuard`] that keeps the lock held until dropped
    /// (end of the calling test). This ensures no two z-index tests
    /// touch the thread-local counter concurrently.
    fn serial_reset() -> MutexGuard<'static, ()> {
        let guard = TEST_SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        reset_global_state();
        guard
    }

    // -- Test helper for inspecting tracked allocations ----------------------

    impl ZIndexAllocator {
        /// Returns the number of currently tracked allocations (test-only).
        fn tracked_count(&self) -> usize {
            self.allocated.borrow().len()
        }
    }

    // == A. Thread-local monotonic counter ===================================

    #[test]
    fn next_z_index_starts_at_base() {
        let _g = serial_reset();
        assert_eq!(next_z_index(), 1000);
    }

    #[test]
    fn next_z_index_is_monotonically_increasing() {
        let _g = serial_reset();
        let z1 = next_z_index();
        let z2 = next_z_index();
        let z3 = next_z_index();
        assert_eq!(z1, 1000);
        assert_eq!(z2, 1001);
        assert_eq!(z3, 1002);
        assert!(z3 > z2);
        assert!(z2 > z1);
    }

    #[test]
    fn next_z_index_wraps_at_ceiling() {
        let _g = serial_reset();
        reset_z_index(Z_INDEX_CEILING);
        let z = next_z_index();
        assert_eq!(z, Z_INDEX_BASE, "ceiling hit should return Z_INDEX_BASE");
    }

    #[test]
    fn next_z_index_resumes_after_wrap() {
        let _g = serial_reset();
        reset_z_index(Z_INDEX_CEILING);
        let _ = next_z_index(); // triggers wrap, returns Z_INDEX_BASE
        let z = next_z_index();
        assert_eq!(
            z,
            Z_INDEX_BASE + 1,
            "post-wrap should resume from Z_INDEX_BASE + 1"
        );
    }

    #[test]
    fn next_z_index_one_below_ceiling_then_wraps() {
        let _g = serial_reset();
        reset_z_index(Z_INDEX_CEILING - 1);
        let z1 = next_z_index();
        assert_eq!(z1, Z_INDEX_CEILING - 1, "one below ceiling is still normal");
        let z2 = next_z_index();
        assert_eq!(z2, Z_INDEX_BASE, "next call hits ceiling and wraps");
    }

    // == B. Configurable base ================================================

    #[test]
    fn reset_z_index_changes_next_value() {
        let _g = serial_reset();
        reset_z_index(5000);
        assert_eq!(next_z_index(), 5000);
        assert_eq!(next_z_index(), 5001);
    }

    #[test]
    fn reset_z_index_to_base_restores_default() {
        let _g = serial_reset();
        // Move counter forward
        let _ = next_z_index(); // 1000
        let _ = next_z_index(); // 1001
        // Reset back to base
        reset_z_index(Z_INDEX_BASE);
        assert_eq!(next_z_index(), 1000);
    }

    // == C. ZIndexAllocator::allocate() ======================================

    #[test]
    fn allocator_allocate_returns_increasing_values() {
        let _g = serial_reset();
        let alloc = ZIndexAllocator::new();
        let z1 = alloc.allocate();
        let z2 = alloc.allocate();
        let z3 = alloc.allocate();
        assert_eq!(z1, 1000);
        assert_eq!(z2, 1001);
        assert_eq!(z3, 1002);
    }

    #[test]
    fn allocator_allocate_wraps_at_ceiling() {
        let _g = serial_reset();
        reset_z_index(Z_INDEX_CEILING);
        let alloc = ZIndexAllocator::new();
        let z1 = alloc.allocate();
        assert_eq!(z1, Z_INDEX_BASE, "allocator must wrap at ceiling");
        let z2 = alloc.allocate();
        assert_eq!(z2, Z_INDEX_BASE + 1, "allocator must resume after wrap");
        assert_eq!(alloc.tracked_count(), 2);
    }

    #[test]
    fn allocator_allocate_delegates_to_thread_local() {
        let _g = serial_reset();
        let alloc = ZIndexAllocator::new();
        // Interleave bare next_z_index() with allocator
        let z1 = next_z_index(); // 1000 (bare)
        let z2 = alloc.allocate(); // 1001 (allocator)
        let z3 = next_z_index(); // 1002 (bare)
        let z4 = alloc.allocate(); // 1003 (allocator)
        assert_eq!(z1, 1000);
        assert_eq!(z2, 1001);
        assert_eq!(z3, 1002);
        assert_eq!(z4, 1003);
    }

    // == D. ZIndexAllocator::release() =======================================

    #[test]
    fn allocator_release_removes_from_tracked() {
        let _g = serial_reset();
        let alloc = ZIndexAllocator::new();
        let z1 = alloc.allocate();
        let _z2 = alloc.allocate();
        assert_eq!(alloc.tracked_count(), 2);
        alloc.release(z1);
        assert_eq!(alloc.tracked_count(), 1);
    }

    #[test]
    fn allocator_release_unknown_is_noop() {
        let _g = serial_reset();
        let alloc = ZIndexAllocator::new();
        // Release a value that was never allocated — should not panic.
        alloc.release(9999);
        assert_eq!(alloc.tracked_count(), 0);
    }

    #[test]
    fn allocator_release_does_not_affect_counter() {
        let _g = serial_reset();
        let alloc = ZIndexAllocator::new();
        let z1 = alloc.allocate(); // 1000
        alloc.release(z1);
        let z2 = alloc.allocate(); // 1001, NOT 1000 (values never reused)
        assert_eq!(z2, 1001);
    }

    #[test]
    fn allocator_double_release_is_noop() {
        let _g = serial_reset();
        let alloc = ZIndexAllocator::new();
        let z1 = alloc.allocate();
        let _z2 = alloc.allocate();
        assert_eq!(alloc.tracked_count(), 2);
        alloc.release(z1);
        assert_eq!(alloc.tracked_count(), 1);
        // Second release of same value — already removed, no panic.
        alloc.release(z1);
        assert_eq!(
            alloc.tracked_count(),
            1,
            "second release must not remove other entries"
        );
    }

    #[test]
    fn allocator_release_after_reset_is_noop() {
        let _g = serial_reset();
        let alloc = ZIndexAllocator::new();
        let z1 = alloc.allocate();
        alloc.reset();
        assert_eq!(alloc.tracked_count(), 0);
        // Release a value that was cleared by reset — should not panic.
        alloc.release(z1);
        assert_eq!(alloc.tracked_count(), 0);
    }

    // == E. ZIndexAllocator::reset() =========================================

    #[test]
    fn allocator_reset_clears_tracked_and_resets_counter() {
        let _g = serial_reset();
        let alloc = ZIndexAllocator::new();
        let _ = alloc.allocate(); // 1000
        let _ = alloc.allocate(); // 1001
        assert_eq!(alloc.tracked_count(), 2);
        alloc.reset();
        assert_eq!(alloc.tracked_count(), 0);
        // Counter should be back at base
        assert_eq!(alloc.allocate(), 1000);
    }

    // == F. Trait implementations ============================================

    #[test]
    fn allocator_default_is_empty() {
        let _g = serial_reset();
        let alloc = ZIndexAllocator::default();
        assert_eq!(alloc.tracked_count(), 0);
    }

    #[test]
    fn allocator_debug_format() {
        let _g = serial_reset();
        let alloc = ZIndexAllocator::new();
        let debug = format!("{alloc:?}");
        assert!(
            debug.contains("ZIndexAllocator"),
            "Debug output should contain the type name"
        );
    }

    // == G. supports_top_layer() ================================================

    #[test]
    fn supports_top_layer_returns_false_without_browser() {
        // Non-wasm targets always return false — top-layer is a browser-only concept.
        assert!(!supports_top_layer());
    }

    #[test]
    fn supports_top_layer_is_idempotent() {
        // Repeated calls must return the same value (exercises cache path).
        let first = supports_top_layer();

        let second = supports_top_layer();

        assert_eq!(first, second);
    }
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    // ── supports_top_layer ─────────────────────────────────────────────

    #[wasm_bindgen_test]
    fn supports_top_layer_returns_true_in_modern_browser() {
        // Modern test browsers (Chrome, Firefox, Safari 15.4+) support
        // native <dialog> with showModal, so this should return true.
        // The important thing is that the function executes without panic
        // and exercises the window → document → createElement → Reflect path.
        let result = supports_top_layer();

        assert!(result, "modern browsers should support top-layer");
    }

    #[wasm_bindgen_test]
    fn supports_top_layer_caches_across_calls() {
        // Calling twice exercises the Cell<Option<bool>> cache-hit path.
        let first = supports_top_layer();

        let second = supports_top_layer();

        assert_eq!(first, second);
    }
}
