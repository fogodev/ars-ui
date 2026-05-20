//! SSR tests for the Dioxus `ZIndexAllocator` adapter.

#![cfg(not(target_arch = "wasm32"))]

use ars_components::utility::z_index_allocator::{Context, Z_INDEX_BASE};
use ars_dioxus::utility::z_index_allocator::ZIndexAllocatorProvider;
use dioxus::prelude::*;

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();

    dioxus_ssr::render(&vdom)
}

#[component]
fn AllocationProbe(id: &'static str) -> Element {
    let context =
        try_use_context::<Context>().expect("ZIndexAllocatorProvider should publish context");

    let claim = context.allocate_claim();

    rsx! {
        span { id, "data-z": "{claim.value()}" }
    }
}

#[component]
fn ResetProbe(id: &'static str) -> Element {
    let context =
        try_use_context::<Context>().expect("ZIndexAllocatorProvider should publish context");

    let _first = context.allocate_claim();

    context.reset();

    let after_reset = context.allocate_claim();

    rsx! {
        span { id, "data-z": "{after_reset.value()}" }
    }
}

#[test]
fn z_index_allocator_provider_renders_children_without_wrapper() {
    fn app() -> Element {
        rsx! {
            ZIndexAllocatorProvider {
                span { id: "allocator-child", "child" }
            }
        }
    }

    let html = render_app(app);

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
    fn app() -> Element {
        rsx! {
            ZIndexAllocatorProvider {
                AllocationProbe { id: "first-claim" }
                AllocationProbe { id: "second-claim" }
            }
        }
    }

    let html = render_app(app);

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
    fn app() -> Element {
        rsx! {
            ZIndexAllocatorProvider {
                AllocationProbe { id: "scope-a-first" }
            }
            ZIndexAllocatorProvider {
                AllocationProbe { id: "scope-b-first" }
                ResetProbe { id: "scope-b-reset" }
            }
        }
    }

    let html = render_app(app);

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
