//! Global overlay stack registry for nested overlay dismissal.
//!
//! Overlay components (Dialog, Popover, Menu, Tooltip, etc.) register themselves
//! on this stack when they open and deregister when they close. The stack tracks
//! stacking order and provides queries used by `InteractOutside` to determine:
//!
//! - Which overlay is **topmost** (for Escape-key and outside-click dismissal).
//! - Whether a click target is **inside a child overlay** (to suppress the
//!   parent's outside-interaction handler).
//! - **LIFO close ordering** — Escape / outside-click dismiss the topmost
//!   overlay first; parent overlays remain open.
//!
//! # Design
//!
//! The stack is a thread-local `Vec<OverlayEntry>` with LIFO semantics.
//! `last()` is the topmost overlay. This matches the single-threaded,
//! `Rc`-based WASM-first design used throughout `ars-dom`.
//!
//! The stack is **platform-agnostic** — no DOM access, no cfg-gating.
//! It tracks pure metadata; the decision of whether to allocate a z-index
//! (vs. using native top-layer) is made by the overlay component at mount
//! time and recorded in [`OverlayEntry::z_index`].
//!
//! # Spec references
//!
//! - `spec/foundation/11-dom-utilities.md` §6.3 "Usage Pattern"
//! - `spec/foundation/11-dom-utilities.md` §6.5 "CSS top-layer Note"
//! - `spec/foundation/05-interactions.md` §12.8 "Nested Overlay Handling"

use std::cell::RefCell;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Whether an overlay is modal (blocks interaction with background) or
/// non-modal (background content remains interactive).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OverlayKind {
    /// Modal overlay: blocks interaction with background content.
    /// Examples: Dialog, `AlertDialog`.
    Modal,

    /// Non-modal overlay: background content remains interactive.
    /// Examples: Popover, Tooltip, Menu.
    NonModal,
}

/// Metadata for a single overlay registered in the global stack.
///
/// Each overlay component creates an `OverlayEntry` when it opens and passes
/// it to [`push_overlay()`]. The entry tracks the overlay's identity, stacking
/// value, and modality for use by dismissal logic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayEntry {
    /// Unique identifier for the overlay instance.
    ///
    /// Typically matches the portal owner ID or the component's unique
    /// instance ID (e.g., `"dialog-1"`, `"menu-2"`).
    pub id: String,

    /// The z-index assigned to this overlay, or `None` when the overlay uses
    /// the browser's native top-layer (via `<dialog>.showModal()` or the
    /// `popover` attribute).
    ///
    /// When `Some`, the value comes from [`next_z_index()`](crate::z_index::next_z_index).
    pub z_index: Option<u32>,

    /// Whether this overlay is modal or non-modal.
    pub kind: OverlayKind,
}

// ---------------------------------------------------------------------------
// Thread-local stack
// ---------------------------------------------------------------------------

thread_local! {
    /// Global overlay stack tracking all open overlays in stacking order.
    /// The last element is the topmost overlay.
    static OVERLAY_STACK: RefCell<Vec<OverlayEntry>> = const { RefCell::new(Vec::new()) };
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Register an overlay on the global stack.
///
/// The overlay is pushed to the top of the stack. If an overlay with the
/// same `id` is already registered, this is a no-op (idempotent).
pub fn push_overlay(entry: OverlayEntry) {
    OVERLAY_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();

        if !stack.iter().any(|e| e.id == entry.id) {
            stack.push(entry);
        }
    });
}

/// Remove an overlay from the global stack by ID.
///
/// Returns the removed entry, or `None` if no overlay with that ID was
/// registered. Supports out-of-order close — removal is by ID, not by
/// position.
pub fn remove_overlay(id: &str) -> Option<OverlayEntry> {
    OVERLAY_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();

        stack
            .iter()
            .position(|e| e.id == id)
            .map(|pos| stack.remove(pos))
    })
}

/// Returns the topmost (most recently pushed) overlay entry, or `None` if
/// the stack is empty.
#[must_use]
pub fn topmost_overlay() -> Option<OverlayEntry> {
    OVERLAY_STACK.with(|stack| stack.borrow().last().cloned())
}

/// Returns `true` if the given overlay ID is the topmost in the stack.
///
/// Returns `false` if the stack is empty or the ID is not registered.
#[must_use]
pub fn is_topmost(id: &str) -> bool {
    OVERLAY_STACK.with(|stack| stack.borrow().last().is_some_and(|e| e.id == id))
}

/// Returns the current number of overlays in the stack.
#[must_use]
pub fn overlay_count() -> usize {
    OVERLAY_STACK.with(|stack| stack.borrow().len())
}

/// Returns `true` if the overlay with `child_id` is above the overlay with
/// `parent_id` in the stack.
///
/// Used by `InteractOutside` to determine whether a click inside a child
/// overlay should suppress the parent's outside-interaction handler per
/// `spec/foundation/05-interactions.md` §12.8.
///
/// Returns `false` if either ID is not registered, or if both IDs are the
/// same.
#[must_use]
pub fn is_above(child_id: &str, parent_id: &str) -> bool {
    if child_id == parent_id {
        return false;
    }

    OVERLAY_STACK.with(|stack| {
        let stack = stack.borrow();

        let child_pos = stack.iter().position(|e| e.id == child_id);

        let parent_pos = stack.iter().position(|e| e.id == parent_id);

        if let (Some(c), Some(p)) = (child_pos, parent_pos) {
            c > p
        } else {
            false
        }
    })
}

/// Returns all overlay IDs that are above the given overlay ID in the stack.
///
/// Used to determine if a click target is inside any child overlay of the
/// given parent. Returns an empty `Vec` if the ID is the topmost overlay
/// or is not registered.
#[must_use]
pub fn overlays_above(id: &str) -> Vec<String> {
    OVERLAY_STACK.with(|stack| {
        let stack = stack.borrow();

        let Some(pos) = stack.iter().position(|e| e.id == id) else {
            return Vec::new();
        };

        stack[pos + 1..].iter().map(|e| e.id.clone()).collect()
    })
}

/// Reset the overlay stack, removing all entries.
///
/// Intended for tests and application-level teardown (e.g., full-page
/// navigation in an SPA).
pub fn reset_overlay_stack() {
    OVERLAY_STACK.with(|stack| stack.borrow_mut().clear());
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Reset the thread-local overlay stack.
/// Called by [`tests::serial_reset()`] before each test.
#[cfg(test)]
fn reset_global_state() {
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
    /// this lock and clears global state before the test body runs. The
    /// returned guard holds the lock for the test's lifetime.
    static TEST_SERIAL: Mutex<()> = Mutex::new(());

    /// Acquire the serialization lock and reset global state.
    ///
    /// Returns a [`MutexGuard`] that keeps the lock held until dropped
    /// (end of the calling test).
    fn serial_reset() -> MutexGuard<'static, ()> {
        let guard = TEST_SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        reset_global_state();

        guard
    }

    /// Create a non-modal overlay entry with the given ID and z-index.
    fn entry(id: &str, z: u32) -> OverlayEntry {
        OverlayEntry {
            id: id.to_owned(),
            z_index: Some(z),
            kind: OverlayKind::NonModal,
        }
    }

    /// Create a modal overlay entry with the given ID and z-index.
    fn modal_entry(id: &str, z: u32) -> OverlayEntry {
        OverlayEntry {
            id: id.to_owned(),
            z_index: Some(z),
            kind: OverlayKind::Modal,
        }
    }

    // == A. Push/pop operations =================================================

    #[test]
    fn push_increases_count() {
        let _g = serial_reset();

        assert_eq!(overlay_count(), 0);

        push_overlay(entry("dialog-1", 1000));

        assert_eq!(overlay_count(), 1);
    }

    #[test]
    fn push_idempotent_same_id() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", 1000));
        push_overlay(entry("dialog-1", 1001));

        assert_eq!(
            overlay_count(),
            1,
            "duplicate ID should not add second entry"
        );
    }

    #[test]
    fn remove_decreases_count() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", 1000));

        assert_eq!(overlay_count(), 1);

        remove_overlay("dialog-1");

        assert_eq!(overlay_count(), 0);
    }

    #[test]
    fn remove_returns_entry() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", 1000));

        let removed = remove_overlay("dialog-1");

        assert_eq!(removed, Some(entry("dialog-1", 1000)));
    }

    #[test]
    fn remove_unknown_returns_none() {
        let _g = serial_reset();

        assert_eq!(remove_overlay("nonexistent"), None);
    }

    // == B. Modal vs non-modal distinction ======================================

    #[test]
    fn modal_and_non_modal_coexist() {
        let _g = serial_reset();

        push_overlay(modal_entry("dialog-1", 1000));
        push_overlay(entry("menu-1", 1001));

        assert_eq!(overlay_count(), 2);
    }

    #[test]
    fn kind_is_preserved() {
        let _g = serial_reset();

        push_overlay(modal_entry("dialog-1", 1000));

        let top = topmost_overlay().expect("stack should not be empty");

        assert_eq!(top.kind, OverlayKind::Modal);
    }

    // == C. Topmost overlay detection ===========================================

    #[test]
    fn topmost_returns_last_pushed() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", 1000));
        push_overlay(entry("menu-1", 1001));

        let top = topmost_overlay().expect("stack should not be empty");

        assert_eq!(top.id, "menu-1");
    }

    #[test]
    fn topmost_none_when_empty() {
        let _g = serial_reset();

        assert_eq!(topmost_overlay(), None);
    }

    #[test]
    fn is_topmost_true_for_last() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", 1000));
        push_overlay(entry("menu-1", 1001));

        assert!(is_topmost("menu-1"));
    }

    #[test]
    fn is_topmost_false_for_non_topmost() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", 1000));
        push_overlay(entry("menu-1", 1001));

        assert!(!is_topmost("dialog-1"));
    }

    #[test]
    fn is_topmost_false_for_unknown() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", 1000));

        assert!(!is_topmost("nonexistent"));
    }

    // == D. LIFO close ordering =================================================

    #[test]
    fn lifo_close_reveals_previous() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", 1000));
        push_overlay(entry("menu-1", 1001));
        push_overlay(entry("tooltip-1", 1002));

        remove_overlay("tooltip-1");

        let top = topmost_overlay().expect("stack should not be empty");

        assert_eq!(top.id, "menu-1");
    }

    #[test]
    fn lifo_close_full_sequence() {
        let _g = serial_reset();

        push_overlay(entry("a", 1000));
        push_overlay(entry("b", 1001));
        push_overlay(entry("c", 1002));

        remove_overlay("c");
        remove_overlay("b");
        remove_overlay("a");

        assert_eq!(overlay_count(), 0);
        assert_eq!(topmost_overlay(), None);
    }

    // == E. Nested overlay suppression (is_above / overlays_above) ==============

    #[test]
    fn is_above_child_above_parent() {
        let _g = serial_reset();

        push_overlay(entry("parent", 1000));
        push_overlay(entry("child", 1001));

        assert!(is_above("child", "parent"));
    }

    #[test]
    fn is_above_parent_not_above_child() {
        let _g = serial_reset();

        push_overlay(entry("parent", 1000));
        push_overlay(entry("child", 1001));

        assert!(!is_above("parent", "child"));
    }

    #[test]
    fn is_above_same_id_false() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", 1000));

        assert!(!is_above("dialog-1", "dialog-1"));
    }

    #[test]
    fn is_above_unknown_false() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", 1000));

        assert!(!is_above("nonexistent", "dialog-1"));
        assert!(!is_above("dialog-1", "nonexistent"));
        assert!(!is_above("foo", "bar"));
    }

    #[test]
    fn overlays_above_returns_children() {
        let _g = serial_reset();

        push_overlay(entry("a", 1000));
        push_overlay(entry("b", 1001));
        push_overlay(entry("c", 1002));

        let above = overlays_above("a");

        assert_eq!(above, vec!["b", "c"]);
    }

    #[test]
    fn overlays_above_topmost_empty() {
        let _g = serial_reset();

        push_overlay(entry("a", 1000));
        push_overlay(entry("b", 1001));

        let above = overlays_above("b");

        assert!(above.is_empty());
    }

    #[test]
    fn overlays_above_unknown_empty() {
        let _g = serial_reset();

        push_overlay(entry("a", 1000));

        let above = overlays_above("nonexistent");

        assert!(above.is_empty());
    }

    // == F. Out-of-order removal ================================================

    #[test]
    fn out_of_order_preserves_remaining() {
        let _g = serial_reset();

        push_overlay(entry("a", 1000));
        push_overlay(entry("b", 1001));
        push_overlay(entry("c", 1002));

        // Remove middle entry
        remove_overlay("b");

        assert_eq!(overlay_count(), 2);

        let top = topmost_overlay().expect("stack should not be empty");

        assert_eq!(top.id, "c");

        // Remove top
        remove_overlay("c");

        let top = topmost_overlay().expect("stack should not be empty");

        assert_eq!(top.id, "a");
    }

    #[test]
    fn out_of_order_is_above_still_works() {
        let _g = serial_reset();

        push_overlay(entry("a", 1000));
        push_overlay(entry("b", 1001));
        push_overlay(entry("c", 1002));

        // Remove middle entry
        remove_overlay("b");

        // c is still above a
        assert!(is_above("c", "a"));
    }

    // == G. Reset ===============================================================

    #[test]
    fn reset_clears_entire_stack() {
        let _g = serial_reset();

        push_overlay(entry("a", 1000));
        push_overlay(entry("b", 1001));
        push_overlay(entry("c", 1002));

        reset_overlay_stack();

        assert_eq!(overlay_count(), 0);
        assert_eq!(topmost_overlay(), None);
    }

    // == H. Z-index metadata ====================================================

    #[test]
    fn z_index_preserved() {
        let _g = serial_reset();

        push_overlay(entry("dialog-1", 1042));

        let top = topmost_overlay().expect("stack should not be empty");

        assert_eq!(top.z_index, Some(1042));
    }

    #[test]
    fn z_index_none_for_top_layer() {
        let _g = serial_reset();

        push_overlay(OverlayEntry {
            id: "dialog-1".to_owned(),
            z_index: None,
            kind: OverlayKind::Modal,
        });

        let top = topmost_overlay().expect("stack should not be empty");

        assert_eq!(top.z_index, None);
    }

    // == I. Trait implementations ===============================================

    #[test]
    fn overlay_kind_debug() {
        assert_eq!(format!("{:?}", OverlayKind::Modal), "Modal");
        assert_eq!(format!("{:?}", OverlayKind::NonModal), "NonModal");
    }

    #[test]
    fn overlay_entry_debug() {
        let e = entry("dialog-1", 1000);

        let debug = format!("{e:?}");

        assert!(
            debug.contains("OverlayEntry"),
            "Debug output should contain the type name"
        );
        assert!(debug.contains("dialog-1"));
    }

    #[test]
    fn overlay_kind_clone_copy() {
        let kind = OverlayKind::Modal;

        #[expect(clippy::clone_on_copy, reason = "explicitly testing Clone impl")]
        let cloned = kind.clone();
        let copied = kind;

        assert_eq!(kind, cloned);
        assert_eq!(kind, copied);
    }

    #[test]
    fn overlay_entry_clone() {
        let e = entry("dialog-1", 1000);

        let cloned = e.clone();

        assert_eq!(e, cloned);
    }
}
