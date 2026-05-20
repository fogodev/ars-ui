//! SSR tests for the Leptos `ZIndexAllocator` adapter.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use ars_components::utility::z_index_allocator::{Context, Z_INDEX_BASE};
use ars_leptos::utility::z_index_allocator::ZIndexAllocatorProvider;
use leptos::prelude::*;

#[component]
fn AllocationProbe(id: &'static str) -> impl IntoView {
    let context = use_context::<Context>().expect("ZIndexAllocatorProvider should publish context");

    let claim = context.allocate_claim();

    view! { <span id=id data-z=claim.value().to_string()></span> }
}

#[component]
fn ResetProbe(id: &'static str) -> impl IntoView {
    let context = use_context::<Context>().expect("ZIndexAllocatorProvider should publish context");

    let _first = context.allocate_claim();

    context.reset();

    let after_reset = context.allocate_claim();

    view! { <span id=id data-z=after_reset.value().to_string()></span> }
}

#[test]
fn z_index_allocator_provider_renders_children_without_wrapper() {
    let html = view! {
        <ZIndexAllocatorProvider>
            <span id="allocator-child">"child"</span>
        </ZIndexAllocatorProvider>
    }
    .to_html();

    assert!(
        html.trim_start().starts_with("<span"),
        "provider should not render a wrapper node: {html}"
    );
    assert!(
        html.contains(r#"id="allocator-child""#),
        "provider should render children: {html}"
    );
}

#[test]
fn z_index_allocator_provider_publishes_shared_context() {
    let html = view! {
        <ZIndexAllocatorProvider>
            <AllocationProbe id="first-claim" />
            <AllocationProbe id="second-claim" />
        </ZIndexAllocatorProvider>
    }
    .to_html();

    assert!(
        html.contains(&format!(r#"id="first-claim" data-z="{Z_INDEX_BASE}""#)),
        "first claim should start at provider base: {html}"
    );
    assert!(
        html.contains(&format!(
            r#"id="second-claim" data-z="{}""#,
            Z_INDEX_BASE + 1
        )),
        "sibling claim should share provider allocation sequence: {html}"
    );
}

#[test]
fn z_index_allocator_provider_scopes_and_resets_allocations() {
    let html = view! {
        <ZIndexAllocatorProvider>
            <AllocationProbe id="scope-a-first" />
        </ZIndexAllocatorProvider>
        <ZIndexAllocatorProvider>
            <AllocationProbe id="scope-b-first" />
            <ResetProbe id="scope-b-reset" />
        </ZIndexAllocatorProvider>
    }
    .to_html();

    assert!(
        html.contains(&format!(r#"id="scope-a-first" data-z="{Z_INDEX_BASE}""#)),
        "first provider should start at base: {html}"
    );
    assert!(
        html.contains(&format!(r#"id="scope-b-first" data-z="{Z_INDEX_BASE}""#)),
        "second provider should have an independent scope: {html}"
    );
    assert!(
        html.contains(&format!(r#"id="scope-b-reset" data-z="{Z_INDEX_BASE}""#)),
        "reset should return the provider scope to base: {html}"
    );
}
