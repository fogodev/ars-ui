//! Hook-slot-stable generated ID allocation.
//!
//! Generated IDs are unique within a render pass and stable for a mounted hook
//! slot. They are not a substitute for explicit IDs in SSR hydration paths where
//! server and client render order may diverge.

use dioxus::prelude::use_hook;

// On WASM (single-threaded), use a thread-local Cell for zero-overhead counting.
#[cfg(target_arch = "wasm32")]
mod counter {
    use std::cell::Cell;

    thread_local! {
        static ID_COUNTER: Cell<u64> = const { Cell::new(0) };
    }

    pub(super) fn next_id() -> u64 {
        ID_COUNTER.with(|c| {
            let v = c.get();
            c.set(v + 1);
            v
        })
    }

    /// Resets the ID counter to zero.
    ///
    /// Must be called at the start of each SSR request to avoid leaking
    /// generated ID allocations across requests.
    #[cfg(feature = "ssr")]
    pub(super) fn reset() {
        ID_COUNTER.with(|c| c.set(0));
    }
}

// On native (multi-threaded SSR), use an atomic counter.
#[cfg(not(target_arch = "wasm32"))]
mod counter {
    use std::sync::atomic::{AtomicU64, Ordering};

    static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

    pub(super) fn next_id() -> u64 {
        ID_COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    /// Resets the ID counter to zero.
    ///
    /// Must be called at the start of each SSR request to avoid leaking
    /// generated ID allocations across requests.
    #[cfg(feature = "ssr")]
    pub(super) fn reset() {
        ID_COUNTER.store(0, Ordering::Relaxed);
    }
}

/// Generates a monotonic component ID with the given prefix.
///
/// Returns a string of the form `"{prefix}-{counter}"`. The counter is global and
/// monotonically increasing, ensuring uniqueness within a single render pass.
///
/// # Hydration safety
///
/// The counter produces identical sequences on SSR and client only when the
/// component tree renders in the same order. Call `reset_id_counter()`
/// (available with the `ssr` feature) at the start of each SSR request to reset
/// the sequence. Components that hydrate over server-rendered DOM should prefer
/// explicit IDs.
///
/// # Examples
///
/// ```
/// use ars_dioxus::use_id;
///
/// let id1 = use_id("dialog");
/// let id2 = use_id("dialog");
/// // id1 and id2 are guaranteed to differ
/// assert_ne!(id1, id2);
/// ```
pub fn use_id(prefix: &str) -> String {
    format!("{prefix}-{}", counter::next_id())
}

/// Generates a component ID with the adapter's stable public prefix format.
///
/// Returns a string of the form `"ars-{prefix}-{id}"`.
///
/// # Hydration safety
///
/// This helper uses Dioxus hook ordering so each component instance stores the
/// counter allocation in a stable hook slot. It still delegates to the adapter's
/// monotonic counter, which is not fully hydration-safe when SSR and client
/// render orders diverge because of Suspense, lazy loading, or code splitting.
/// SSR+hydration users should provide explicit component IDs until a true
/// tree-position-based ID scheme is implemented.
#[must_use]
pub fn use_stable_id(prefix: &str) -> String {
    let id = use_hook(counter::next_id);

    format!("ars-{prefix}-{id}")
}

/// Resets the ID counter to zero for a new SSR request.
///
/// Must be called at the start of each server-side render pass to avoid
/// cross-request counter leakage. This does not make generated IDs safe when
/// server and client render order diverge.
#[cfg(feature = "ssr")]
pub fn reset_id_counter() {
    counter::reset();
}

#[cfg(test)]
mod tests {
    use dioxus::{
        dioxus_core::{NoOpMutations, ScopeId},
        prelude::*,
    };

    use super::*;

    #[test]
    fn use_id_produces_prefixed_ids() {
        let id = use_id("test");
        assert!(id.starts_with("test-"));
    }

    #[test]
    fn use_id_produces_unique_ids() {
        let id1 = use_id("component");
        let id2 = use_id("component");
        assert_ne!(id1, id2);
    }

    #[test]
    fn use_stable_id_produces_ars_prefixed_ids() {
        fn app() -> Element {
            let id = use_stable_id("dialog");

            assert!(id.starts_with("ars-dialog-"));

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn use_stable_id_reuses_hook_slot_across_renders() {
        fn app() -> Element {
            let id = use_stable_id("dialog");

            let mut previous = use_signal(|| None::<String>);

            if let Some(previous) = previous.peek().as_ref() {
                assert_eq!(previous, &id);
            } else {
                previous.set(Some(id));
            }

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
        dom.mark_dirty(ScopeId::APP);
        dom.render_immediate(&mut NoOpMutations);
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn use_id_produces_monotonic_sequence_on_wasm() {
        let id1 = use_id("component");
        let id2 = use_id("component");

        assert!(id1.starts_with("component-"));
        assert!(id2.starts_with("component-"));
        assert_ne!(id1, id2);
    }
}
