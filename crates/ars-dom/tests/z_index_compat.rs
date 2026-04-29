//! Compatibility tests for DOM-facing z-index re-exports.

use std::sync::{Mutex, MutexGuard};

static TEST_SERIAL: Mutex<()> = Mutex::new(());

fn serial_reset() -> MutexGuard<'static, ()> {
    let guard = TEST_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    ars_dom::reset_z_index(ars_dom::Z_INDEX_BASE);

    guard
}

#[test]
fn z_index_ars_dom_and_ars_core_free_functions_interleave_on_same_sequence() {
    let _guard = serial_reset();

    assert_eq!(ars_dom::next_z_index(), ars_dom::Z_INDEX_BASE);
    assert_eq!(ars_core::next_z_index(), ars_dom::Z_INDEX_BASE + 1);
    assert_eq!(ars_dom::next_z_index(), ars_dom::Z_INDEX_BASE + 2);
}

#[test]
fn z_index_ars_dom_allocator_is_provider_scoped() {
    let _guard = serial_reset();

    let allocator = ars_dom::ZIndexAllocator::new();

    assert_eq!(ars_dom::next_z_index(), ars_dom::Z_INDEX_BASE);
    assert_eq!(allocator.allocate(), ars_dom::Z_INDEX_BASE);
    assert_eq!(ars_core::next_z_index(), ars_dom::Z_INDEX_BASE + 1);
    assert_eq!(allocator.allocate(), ars_dom::Z_INDEX_BASE + 1);
}
