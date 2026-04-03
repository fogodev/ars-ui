//! Hydration-safe deterministic ID generation.
//!
//! Provides monotonic counters that produce the same ID sequence on both SSR and
//! client when the component tree renders in the same order.

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
    /// Must be called at the start of each SSR request so that server-rendered IDs
    /// match the client-side hydration sequence.
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
    /// Must be called at the start of each SSR request so that server-rendered IDs
    /// match the client-side hydration sequence.
    #[cfg(feature = "ssr")]
    pub(super) fn reset() {
        ID_COUNTER.store(0, Ordering::Relaxed);
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
/// renders in the same order. Call [`reset_id_counter()`] at the start of each SSR
/// request to reset the sequence.
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

/// Generates a related element ID by appending a suffix to a base ID.
///
/// # Examples
///
/// ```
/// use ars_leptos::related_id;
///
/// let base = "menu-1";
/// assert_eq!(related_id(base, "trigger"), "menu-1-trigger");
/// assert_eq!(related_id(base, "content"), "menu-1-content");
/// ```
pub fn related_id(base: &str, suffix: &str) -> String {
    format!("{base}-{suffix}")
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

    #[test]
    fn related_id_appends_suffix() {
        assert_eq!(related_id("menu-1", "trigger"), "menu-1-trigger");
        assert_eq!(related_id("menu-1", "content"), "menu-1-content");
    }
}
