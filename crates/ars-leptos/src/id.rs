//! Hydration-safe deterministic ID generation.
//!
//! Provides monotonic counters that produce the same ID sequence on both SSR and
//! client when the component tree renders in the same order.

// Thread-local counter on all targets.
//
// Leptos SSR renders a single request on one thread (`render_to_string`,
// `ssr_in_order`), so per-thread counters give each request its own
// monotonic sequence that matches the WASM-side hydration sequence
// without races between concurrent requests.
//
// A previous global `AtomicU64` implementation was process-wide, which
// broke both (a) parallel SSR requests interleaving each other's IDs
// and (b) parallel tests asserting specific counter values racing on the
// same shared counter.
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

    /// Resets the ID counter to zero on the current thread.
    ///
    /// Must be called at the start of each SSR request so that server-rendered IDs
    /// match the client-side hydration sequence.
    #[cfg(feature = "ssr")]
    pub(super) fn reset() {
        ID_COUNTER.with(|c| c.set(0));
    }
}

/// Generates a deterministic component ID with the given prefix.
///
/// Returns a string of the form `"{prefix}-{counter}"`. The counter is global and
/// monotonically increasing, ensuring uniqueness within a single render pass.
///
/// # Hydration safety
///
/// The counter produces identical sequences on SSR and client when the component tree
/// renders in the same order. Call `reset_id_counter()` (available with the `ssr`
/// feature) at the start of each SSR request to reset the sequence.
///
/// # Examples
///
/// ```
/// use ars_leptos::use_id;
///
/// let id1 = use_id("dialog");
/// let id2 = use_id("dialog");
/// // id1 and id2 are guaranteed to differ
/// assert_ne!(id1, id2);
/// ```
pub fn use_id(prefix: &str) -> String {
    format!("{prefix}-{}", counter::next_id())
}

/// Resets the ID counter to zero for a new SSR request.
///
/// Must be called at the start of each server-side render pass so that the generated
/// IDs match the client-side hydration sequence exactly.
#[cfg(feature = "ssr")]
pub fn reset_id_counter() {
    counter::reset();
}

#[cfg(test)]
mod tests {
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
