//! Global overlay stack registry for nested overlay dismissal.
//!
//! Overlay components (Dialog, Popover, Menu, Tooltip, etc.) register with a
//! thread-local stack when they mount and deregister when they unmount. The
//! stack determines:
//!
//! - **Topmost overlay** — only the topmost overlay responds to outside
//!   interactions and Escape-key dismissal (spec §12.8 rule 1).
//! - **Child overlay membership** — a click inside a child overlay does NOT
//!   trigger `InteractOutside` on the parent (spec §12.8 rule 2).
//! - **LIFO close ordering** — Escape / outside-click dismisses the topmost
//!   overlay first; the parent remains open (spec §12.8 rule 3).
//!
//! # Thread-local design
//!
//! The stack is stored in a `thread_local!` `RefCell<Vec<OverlayEntry>>`.
//! This is consistent with the z-index allocator (`z_index.rs`) and the
//! library's `Rc`-based, single-threaded WASM-first design. Each thread
//! maintains its own overlay stack.
//!
//! # Spec references
//!
//! - `spec/foundation/11-dom-utilities.md` §9 — Overlay Stack Registry
//! - `spec/foundation/05-interactions.md` §12.8 — Nested Overlay Handling

use std::cell::RefCell;

// ---------------------------------------------------------------------------
// Thread-local state
// ---------------------------------------------------------------------------

thread_local! {
    /// Per-thread overlay stack. Entries are ordered by mount time: the last
    /// entry is the topmost (most recently opened) overlay.
    static OVERLAY_STACK: RefCell<Vec<OverlayEntry>> = const { RefCell::new(Vec::new()) };
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Metadata for a registered overlay in the global stack.
///
/// Each overlay component creates an `OverlayEntry` when it mounts and passes
/// it to [`push_overlay`]. The entry records whether the overlay is modal
/// (triggering scroll lock and background inert) and the allocated z-index
/// (or `None` when using native CSS top-layer).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayEntry {
    /// Unique overlay identifier (matches the component's DOM id).
    pub id: String,

    /// Whether this overlay is modal (triggers scroll lock + background inert).
    pub modal: bool,

    /// Allocated z-index from [`super::z_index::next_z_index`], or `None` when
    /// the overlay uses native CSS top-layer (see
    /// [`super::z_index::supports_top_layer`]).
    pub z_index: Option<u32>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Register an overlay on the global stack.
///
/// Called when an overlay component mounts. If an entry with the same `id`
/// already exists, the call is a no-op to prevent double-registration from
/// framework re-renders.
///
/// # Spec reference
///
/// `spec/foundation/11-dom-utilities.md` §9.3 — Overlay Stack Public API.
/// `spec/foundation/05-interactions.md` §12.8 rule 4 — "Each overlay registers
/// itself with a global overlay stack on mount."
pub fn push_overlay(entry: OverlayEntry) {
    OVERLAY_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();

        if !stack.iter().any(|e| e.id == entry.id) {
            stack.push(entry);
        }
    });
}

/// Deregister an overlay from the global stack.
///
/// Removes the entry with the given `id` regardless of its position in the
/// stack (not limited to the topmost entry). Called when an overlay component
/// unmounts. If no entry with the given `id` exists, the call is a no-op
/// (safe to call multiple times).
///
/// # Spec reference
///
/// `spec/foundation/11-dom-utilities.md` §9.3 — Overlay Stack Public API.
/// `spec/foundation/05-interactions.md` §12.8 rule 4 — "…and deregisters on
/// unmount."
pub fn remove_overlay(id: &str) {
    OVERLAY_STACK.with(|stack| {
        stack.borrow_mut().retain(|e| e.id != id);
    });
}

/// Return the topmost (most recently opened) overlay, or `None` if the stack
/// is empty.
///
/// The topmost overlay is the one that responds to outside interactions and
/// Escape-key dismissal.
///
/// # Spec reference
///
/// `spec/foundation/05-interactions.md` §12.8 rule 1 — "Only the topmost
/// overlay responds to outside interactions."
#[must_use]
pub fn topmost_overlay() -> Option<OverlayEntry> {
    OVERLAY_STACK.with(|stack| stack.borrow().last().cloned())
}

/// Check whether the overlay with the given `id` is the topmost overlay.
///
/// Convenience wrapper around [`topmost_overlay`].
#[must_use]
pub fn is_topmost(id: &str) -> bool {
    OVERLAY_STACK.with(|stack| stack.borrow().last().is_some_and(|entry| entry.id == id))
}

/// Return the IDs of all overlays stacked above the overlay with the given
/// `id`.
///
/// Used by `InteractOutside` to determine whether a click target is inside a
/// child overlay. If the `id` is not found in the stack, returns an empty
/// `Vec`.
///
/// # Spec reference
///
/// `spec/foundation/05-interactions.md` §12.8 rule 2 — "A click inside a child
/// overlay does NOT trigger `InteractOutside` on the parent."
#[must_use]
pub fn overlays_above(id: &str) -> Vec<String> {
    OVERLAY_STACK.with(|stack| {
        let stack = stack.borrow();

        stack
            .iter()
            .position(|e| e.id == id)
            .map_or_else(Vec::new, |idx| {
                stack[idx + 1..]
                    .iter()
                    .map(|entry| entry.id.clone())
                    .collect()
            })
    })
}

/// Check whether an overlay with the given `id` is currently registered.
#[must_use]
pub fn contains_overlay(id: &str) -> bool {
    OVERLAY_STACK.with(|stack| stack.borrow().iter().any(|e| e.id == id))
}

/// Return the number of overlays currently on the stack.
#[must_use]
pub fn overlay_count() -> usize {
    OVERLAY_STACK.with(|stack| stack.borrow().len())
}

/// Clear the overlay stack.
///
/// Intended for tests and application-level teardown (e.g., full-page
/// navigation in an SPA). Matches [`super::z_index::reset_z_index`] in
/// purpose.
pub fn reset_overlay_stack() {
    OVERLAY_STACK.with(|stack| stack.borrow_mut().clear());
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard};

    use super::*;

    /// Serializes overlay stack tests so they don't run in parallel.
    ///
    /// Although `thread_local!` is per-thread, the Rust test runner may reuse
    /// threads across tests. Each test calls [`serial_reset()`] which acquires
    /// this lock and clears thread-local state before the test body runs.
    static TEST_SERIAL: Mutex<()> = Mutex::new(());

    /// Acquire the serialization lock and reset global state.
    ///
    /// Returns a [`MutexGuard`] that keeps the lock held until dropped
    /// (end of the calling test).
    fn serial_reset() -> MutexGuard<'static, ()> {
        let guard = TEST_SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        reset_overlay_stack();

        guard
    }

    /// Helper: create an `OverlayEntry` with sensible defaults.
    fn entry(id: &str, modal: bool, z_index: Option<u32>) -> OverlayEntry {
        OverlayEntry {
            id: id.to_owned(),
            modal,
            z_index,
        }
    }

    // == A. Push/pop operations =============================================

    #[test]
    fn push_adds_to_stack() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", false, Some(1000)));

        assert_eq!(overlay_count(), 1);
    }

    #[test]
    fn push_duplicate_id_is_noop() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", false, Some(1000)));
        push_overlay(entry("dialog-1", true, Some(1001)));

        assert_eq!(
            overlay_count(),
            1,
            "duplicate ID must not add a second entry"
        );
    }

    #[test]
    fn push_duplicate_preserves_original_entry() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", false, Some(1000)));

        // Attempt to re-register with different modal + z_index:
        push_overlay(entry("dialog-1", true, Some(9999)));

        let top = topmost_overlay().expect("stack not empty");

        assert!(!top.modal, "original modal flag must be preserved");
        assert_eq!(
            top.z_index,
            Some(1000),
            "original z_index must be preserved"
        );
    }

    #[test]
    fn pop_removes_entry() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", false, Some(1000)));

        assert_eq!(overlay_count(), 1);

        remove_overlay("dialog-1");

        assert_eq!(overlay_count(), 0);
    }

    #[test]
    fn pop_unknown_id_is_noop() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", false, Some(1000)));

        remove_overlay("nonexistent");

        assert_eq!(
            overlay_count(),
            1,
            "popping unknown ID must not remove anything"
        );
    }

    #[test]
    fn pop_mid_stack_preserves_order() {
        let _g = serial_reset();

        push_overlay(entry("a", false, Some(1000)));
        push_overlay(entry("b", false, Some(1001)));
        push_overlay(entry("c", false, Some(1002)));

        remove_overlay("b");

        assert_eq!(overlay_count(), 2);

        // Remaining order: a, c
        assert_eq!(
            topmost_overlay().expect("stack not empty").id,
            "c",
            "topmost after mid-stack removal must be the last pushed"
        );

        assert!(contains_overlay("a"));
        assert!(!contains_overlay("b"));
    }

    #[test]
    fn double_pop_is_noop() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", false, Some(1000)));

        remove_overlay("dialog-1");
        remove_overlay("dialog-1"); // second pop — should not panic

        assert_eq!(overlay_count(), 0);
    }

    // == B. Modal vs non-modal ==============================================

    #[test]
    fn push_modal_preserves_flag() {
        let _g = serial_reset();

        push_overlay(entry("modal-dialog", true, Some(1000)));

        let top = topmost_overlay().expect("stack not empty");

        assert!(top.modal, "modal flag must be preserved");
    }

    #[test]
    fn push_non_modal_preserves_flag() {
        let _g = serial_reset();

        push_overlay(entry("popover-1", false, Some(1000)));

        let top = topmost_overlay().expect("stack not empty");

        assert!(!top.modal, "non-modal flag must be preserved");
    }

    // == C. Topmost overlay detection =======================================

    #[test]
    fn topmost_returns_last_pushed() {
        let _g = serial_reset();

        push_overlay(entry("a", false, Some(1000)));
        push_overlay(entry("b", false, Some(1001)));

        assert_eq!(topmost_overlay().expect("stack not empty").id, "b");
    }

    #[test]
    fn topmost_returns_none_when_empty() {
        let _g = serial_reset();

        assert!(topmost_overlay().is_none());
    }

    #[test]
    fn is_topmost_true_for_single_entry() {
        let _g = serial_reset();

        push_overlay(entry("only", false, Some(1000)));

        assert!(is_topmost("only"));
    }

    #[test]
    fn is_topmost_true_for_last() {
        let _g = serial_reset();

        push_overlay(entry("a", false, Some(1000)));
        push_overlay(entry("b", false, Some(1001)));

        assert!(is_topmost("b"));
    }

    #[test]
    fn is_topmost_false_for_non_topmost() {
        let _g = serial_reset();

        push_overlay(entry("a", false, Some(1000)));
        push_overlay(entry("b", false, Some(1001)));

        assert!(!is_topmost("a"));
    }

    #[test]
    fn is_topmost_false_when_empty() {
        let _g = serial_reset();

        assert!(!is_topmost("anything"));
    }

    #[test]
    fn topmost_after_pop_returns_previous() {
        let _g = serial_reset();

        push_overlay(entry("a", false, Some(1000)));
        push_overlay(entry("b", false, Some(1001)));

        remove_overlay("b");

        assert_eq!(topmost_overlay().expect("stack not empty").id, "a");
    }

    // == D. LIFO close ordering =============================================

    #[test]
    fn lifo_close_ordering() {
        let _g = serial_reset();

        push_overlay(entry("a", false, Some(1000)));
        push_overlay(entry("b", false, Some(1001)));
        push_overlay(entry("c", false, Some(1002)));

        // Pop topmost first (LIFO)
        assert_eq!(topmost_overlay().expect("not empty").id, "c");
        remove_overlay("c");

        assert_eq!(topmost_overlay().expect("not empty").id, "b");
        remove_overlay("b");

        assert_eq!(topmost_overlay().expect("not empty").id, "a");
        remove_overlay("a");

        assert!(topmost_overlay().is_none());
    }

    // == E. Child overlay queries ===========================================

    #[test]
    fn overlays_above_returns_children() {
        let _g = serial_reset();

        push_overlay(entry("dialog", true, Some(1000)));
        push_overlay(entry("menu", false, Some(1001)));
        push_overlay(entry("tooltip", false, Some(1002)));

        let above = overlays_above("dialog");

        assert_eq!(above, vec!["menu", "tooltip"]);
    }

    #[test]
    fn overlays_above_empty_for_topmost() {
        let _g = serial_reset();

        push_overlay(entry("dialog", true, Some(1000)));
        push_overlay(entry("menu", false, Some(1001)));

        let above = overlays_above("menu");

        assert!(above.is_empty(), "topmost overlay has no children above it");
    }

    #[test]
    fn overlays_above_empty_for_unknown_id() {
        let _g = serial_reset();

        push_overlay(entry("dialog", true, Some(1000)));

        let above = overlays_above("nonexistent");

        assert!(above.is_empty(), "unknown ID must return empty vec");
    }

    #[test]
    fn overlays_above_does_not_include_parent_or_below() {
        let _g = serial_reset();

        push_overlay(entry("a", false, Some(1000)));
        push_overlay(entry("b", false, Some(1001)));
        push_overlay(entry("c", false, Some(1002)));

        let above_b = overlays_above("b");

        assert_eq!(
            above_b,
            vec!["c"],
            "must not include 'b' itself or 'a' below it"
        );
    }

    // == F. Utility =========================================================

    #[test]
    fn contains_overlay_true_when_registered() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", false, Some(1000)));

        assert!(contains_overlay("dialog-1"));
    }

    #[test]
    fn contains_overlay_false_when_not() {
        let _g = serial_reset();

        assert!(!contains_overlay("dialog-1"));
    }

    #[test]
    fn overlay_count_tracks_depth() {
        let _g = serial_reset();

        assert_eq!(overlay_count(), 0);

        push_overlay(entry("a", false, Some(1000)));

        assert_eq!(overlay_count(), 1);

        push_overlay(entry("b", false, Some(1001)));

        assert_eq!(overlay_count(), 2);

        remove_overlay("a");

        assert_eq!(overlay_count(), 1);
    }

    #[test]
    fn reset_clears_all() {
        let _g = serial_reset();

        push_overlay(entry("a", false, Some(1000)));
        push_overlay(entry("b", true, Some(1001)));

        assert_eq!(overlay_count(), 2);

        reset_overlay_stack();

        assert_eq!(overlay_count(), 0);
        assert!(topmost_overlay().is_none());
    }

    // == G. Z-index integration =============================================

    #[test]
    fn entry_carries_z_index() {
        let _g = serial_reset();

        push_overlay(entry("dialog", true, Some(1000)));

        let top = topmost_overlay().expect("stack not empty");

        assert_eq!(top.z_index, Some(1000));
    }

    #[test]
    fn entry_z_index_none_for_top_layer() {
        let _g = serial_reset();

        push_overlay(entry("dialog", true, None));

        let top = topmost_overlay().expect("stack not empty");

        assert_eq!(top.z_index, None, "top-layer overlays have no z-index");
    }

    #[test]
    fn entry_debug_format() {
        let e = entry("dlg-1", true, Some(1000));

        let debug = format!("{e:?}");

        assert!(
            debug.contains("OverlayEntry"),
            "Debug output should contain the type name"
        );
        assert!(
            debug.contains("dlg-1"),
            "Debug output should contain the id"
        );
    }
}
