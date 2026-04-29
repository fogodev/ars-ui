//! DOM-facing z-index helpers for overlay stacking order.
//!
//! Pure allocation lives in [`ars_core::z_index`]. This module preserves the
//! historical `ars_dom::z_index` paths for adapters while keeping DOM-specific
//! feature detection, such as CSS top-layer support, in `ars-dom`.

pub use ars_core::z_index::{
    Z_INDEX_BASE, Z_INDEX_CEILING, ZIndexAllocator, ZIndexClaim, next_z_index, reset_z_index,
};

/// Check whether the browser supports the CSS top-layer.
///
/// Detected once per thread and cached. When top-layer is supported, overlay
/// components can skip z-index allocation and rely on the browser's native
/// stacking. Returns `false` in non-browser environments.
///
/// # Spec reference
///
/// `spec/foundation/11-dom-utilities.md` §6.5 — CSS `top-layer` Note.
#[must_use]
pub fn supports_top_layer() -> bool {
    supports_top_layer_impl()
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn supports_top_layer_impl() -> bool {
    use std::cell::Cell;

    thread_local! {
        static CACHED: Cell<Option<bool>> = const { Cell::new(None) };
    }

    CACHED.with(|c| {
        if let Some(value) = c.get() {
            return value;
        }

        let supported = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.create_element("dialog").ok())
            .is_some_and(|element| {
                js_sys::Reflect::has(&element, &"showModal".into()).unwrap_or(false)
            });

        c.set(Some(supported));

        supported
    })
}

#[cfg(not(all(feature = "web", target_arch = "wasm32")))]
const fn supports_top_layer_impl() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard};

    use super::{Z_INDEX_BASE, ZIndexAllocator, next_z_index, reset_z_index, supports_top_layer};

    static TEST_SERIAL: Mutex<()> = Mutex::new(());

    fn serial_reset() -> MutexGuard<'static, ()> {
        let guard = TEST_SERIAL
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        reset_z_index(Z_INDEX_BASE);

        guard
    }

    #[test]
    fn ars_dom_allocator_uses_provider_scoped_counter() {
        let _guard = serial_reset();

        let allocator = ZIndexAllocator::new();

        assert_eq!(allocator.allocate(), 1000);
        assert_eq!(allocator.allocate(), 1001);
        assert_eq!(next_z_index(), 1000);
    }

    #[test]
    fn supports_top_layer_returns_false_in_test_environment() {
        assert!(!supports_top_layer());
    }

    #[test]
    fn supports_top_layer_is_idempotent() {
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

    #[wasm_bindgen_test]
    fn supports_top_layer_returns_true_in_modern_browser() {
        assert!(supports_top_layer());
    }

    #[wasm_bindgen_test]
    fn supports_top_layer_caches_across_calls() {
        let first = supports_top_layer();
        let second = supports_top_layer();

        assert_eq!(first, second);
    }
}
