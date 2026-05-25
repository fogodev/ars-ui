use ars_components::utility::z_index_allocator::{Context, Props, Z_INDEX_BASE, reset_z_index};

#[test]
fn z_index_allocator_props_and_context_match_spec() {
    assert_eq!(Props::new(), Props);

    reset_z_index(Z_INDEX_BASE);

    let context = Context::new();

    assert_eq!(context.allocate(), Z_INDEX_BASE);
}
