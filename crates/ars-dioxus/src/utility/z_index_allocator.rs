//! Dioxus `ZIndexAllocator` adapter.
//!
//! Publishes the framework-agnostic z-index allocation context to descendants
//! without rendering an adapter-owned DOM node.

pub use ars_components::utility::z_index_allocator::{
    Context, Props, Z_INDEX_BASE, Z_INDEX_CEILING, ZIndexAllocator, ZIndexClaim, next_z_index,
    reset_z_index,
};
use dioxus::prelude::*;

/// Props for the Dioxus [`ZIndexAllocatorProvider`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct ZIndexAllocatorProviderProps {
    /// Children that share the provider-scoped allocation context.
    pub children: Element,
}

/// Dioxus provider for the framework-agnostic z-index allocator context.
#[component]
pub fn ZIndexAllocatorProvider(props: ZIndexAllocatorProviderProps) -> Element {
    use_context_provider(Context::new);

    rsx! {
        {props.children}
    }
}
