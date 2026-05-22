//! Hydration-safe deterministic ID generation.
//!
//! Provides monotonic counters that produce the same ID sequence on both SSR and
//! client when the component tree renders in the same order.

// Deterministic counters for generated adapter IDs.
//
// Leptos SSR can resume async render segments for one request on different
// worker threads, so native SSR uses a request-scoped context counter when
// `reset_id_counter()` runs inside the request owner. Calls outside a reactive
// owner fall back to a process counter rather than per-thread storage, avoiding
// duplicate fallback IDs if work crosses worker threads before a request scope
// is installed.
mod counter {
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };

    #[cfg(feature = "ssr")]
    use leptos::prelude::provide_context;
    use leptos::prelude::use_context;

    #[derive(Clone, Debug)]
    struct RequestIdCounter {
        value: Arc<AtomicU64>,
    }

    impl RequestIdCounter {
        #[cfg(feature = "ssr")]
        fn new() -> Self {
            Self {
                value: Arc::new(AtomicU64::new(0)),
            }
        }

        fn next_id(&self) -> u64 {
            self.value.fetch_add(1, Ordering::Relaxed)
        }
    }

    static FALLBACK_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn next_fallback_id() -> u64 {
        FALLBACK_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    pub(super) fn next_id() -> u64 {
        use_context::<RequestIdCounter>().map_or_else(next_fallback_id, |counter| counter.next_id())
    }

    /// Provides a fresh request-scoped ID counter on the current reactive owner.
    ///
    /// Must be called at the start of each SSR request so that server-rendered
    /// IDs generated under that owner match the client-side hydration sequence.
    /// Calls outside the owner fall back to the monotonic process-wide counter.
    #[cfg(feature = "ssr")]
    pub(super) fn reset() {
        let counter = RequestIdCounter::new();

        provide_context(counter);
    }
}

/// Generates a deterministic component ID with the given prefix.
///
/// Returns a string of the form `"{prefix}-{counter}"`. IDs generated under an
/// SSR request owner use that owner's request-scoped sequence. Ownerless calls
/// use a process-wide monotonic fallback counter.
///
/// # Hydration safety
///
/// The request counter produces identical sequences on SSR and client when the
/// component tree renders in the same order. Call `reset_id_counter()`
/// (available with the `ssr` feature) under the request owner at the start of
/// each SSR request to install a fresh request sequence.
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

/// Installs a fresh request-scoped ID counter for a new SSR request.
///
/// Must be called under the request's reactive owner at the start of each
/// server-side render pass so that IDs generated inside that owner match the
/// client-side hydration sequence exactly. Ownerless fallback IDs remain
/// monotonic for the lifetime of the process.
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

    #[test]
    fn use_id_native_fallback_does_not_restart_on_worker_threads() {
        let id1 = std::thread::spawn(|| use_id("component"))
            .join()
            .expect("worker thread should not panic");
        let id2 = std::thread::spawn(|| use_id("component"))
            .join()
            .expect("worker thread should not panic");

        assert_ne!(
            id1, id2,
            "fallback ID generation must not emit duplicate IDs when SSR work resumes on a different thread"
        );
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn reset_id_counter_does_not_restart_global_fallback_counter() {
        let before = use_id("component");

        let owner = leptos::reactive::owner::Owner::new();
        owner.with(reset_id_counter);
        drop(owner);

        let after = use_id("component");

        let before_counter = before
            .strip_prefix("component-")
            .expect("generated ID should include the requested prefix")
            .parse::<u64>()
            .expect("generated ID suffix should be numeric");
        let after_counter = after
            .strip_prefix("component-")
            .expect("generated ID should include the requested prefix")
            .parse::<u64>()
            .expect("generated ID suffix should be numeric");

        assert!(
            after_counter > before_counter,
            "reset_id_counter must not rewind the process-wide fallback counter"
        );
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
