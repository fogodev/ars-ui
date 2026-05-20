//! Leptos `ZIndexAllocator` adapter.
//!
//! Publishes the framework-agnostic z-index allocation context to descendants
//! without rendering an adapter-owned DOM node.

pub use ars_components::utility::z_index_allocator::{
    Context, Props, Z_INDEX_BASE, Z_INDEX_CEILING, ZIndexAllocator, ZIndexClaim, next_z_index,
    reset_z_index,
};
use leptos::{children::TypedChildren, prelude::*, tachys::reactive_graph::OwnedView};

/// Leptos provider for the framework-agnostic z-index allocator context.
#[component]
pub fn ZIndexAllocatorProvider<T>(
    /// Children that share the provider-scoped allocation context.
    children: TypedChildren<T>,
) -> impl IntoView
where
    T: IntoView + 'static,
{
    let owner = Owner::current().map_or_else(Owner::new, |owner| owner.child());

    let children = children.into_inner();

    let children = owner.with(|| {
        provide_context(Context::new());
        children()
    });

    OwnedView::new_with_owner(children, owner)
}
