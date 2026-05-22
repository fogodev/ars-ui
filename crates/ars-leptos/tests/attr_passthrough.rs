//! Regression tests for the explicit `class` prop on Leptos adapter
//! components, and the `attr:` prefix pass-through for other attributes.
//!
//! Leptos's `attr:class` does NOT merge with a component-supplied internal
//! class — it emits a second `class=` attribute, which browsers silently
//! drop. To keep Tailwind/CSS-utility workflows working, every Leptos
//! adapter component that owns a root element exposes an explicit `class`
//! prop that pre-merges with the component's own class tokens in the
//! `AttrMap` layer.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr", feature = "icu4x"))]

use ars_leptos::utility::{
    heading::{Heading, Level},
    highlight::Highlight,
    landmark::{Landmark, Role},
    separator::Separator,
    visually_hidden::VisuallyHidden,
};
use leptos::{prelude::*, reactive::owner::Owner};

fn render(view_fn: impl FnOnce() -> String + 'static) -> String {
    let owner = Owner::new();
    let result = owner.with(view_fn);

    drop(owner);

    result
}

#[test]
fn heading_forwards_class_prop_to_root() {
    let html = render(|| {
        view! {
            <Heading id="x" level=Level::Two class="text-4xl font-bold">
                "Title"
            </Heading>
        }
        .to_html()
    });

    assert!(
        html.trim_start().starts_with("<h2"),
        "expected h2 root: {html}"
    );
    assert!(
        html.contains(r#"class="text-4xl font-bold""#),
        "expected class on root: {html}"
    );
    assert_eq!(
        html.matches("class=").count(),
        1,
        "expected exactly one class= attribute: {html}"
    );
}

#[test]
fn landmark_forwards_class_to_native_role_root() {
    let html = render(|| {
        view! {
            <Landmark id="n" role=Role::Navigation class="sidebar-nav">
                "links"
            </Landmark>
        }
        .to_html()
    });

    assert!(html.trim_start().starts_with("<nav"));
    assert!(
        html.contains(r#"class="sidebar-nav""#),
        "expected class on nav root: {html}"
    );
    assert_eq!(
        html.matches("class=").count(),
        1,
        "expected exactly one class= attribute: {html}"
    );
}

#[test]
fn landmark_forwards_class_to_search_fallback_div() {
    let html = render(|| {
        view! {
            <Landmark id="s" role=Role::Search class="site-search">
                "input"
            </Landmark>
        }
        .to_html()
    });

    assert!(html.trim_start().starts_with("<div"));
    assert!(
        html.contains(r#"class="site-search""#),
        "expected class on fallback div: {html}"
    );
}

#[test]
fn highlight_forwards_class_to_root_span() {
    let html = render(|| {
        view! { <Highlight query=vec!["x".to_string()] text="hello".to_string() class="hl-root" /> }
            .to_html()
    });

    assert!(html.trim_start().starts_with("<span"));
    assert!(
        html.contains(r#"class="hl-root""#),
        "expected class on highlight root: {html}"
    );
}

#[test]
fn separator_forwards_class_to_hr_root() {
    let html = render(|| view! { <Separator id="sep" class="menu-divider" /> }.to_html());

    assert!(html.trim_start().starts_with("<hr"));
    assert!(
        html.contains(r#"class="menu-divider""#),
        "expected class on hr root: {html}"
    );
}

#[test]
fn visually_hidden_merges_consumer_class_with_internal_class() {
    // VisuallyHidden's non-focusable `<span>` already carries the
    // `ars-visually-hidden` class. Consumer class tokens must concatenate
    // into a single `class=` attribute, not produce duplicate attributes.
    let html = render(|| {
        view! {
            <VisuallyHidden id="v" class="skip-link">
                "Hidden"
            </VisuallyHidden>
        }
        .to_html()
    });

    assert_eq!(
        html.matches("class=").count(),
        1,
        "expected merge to produce a single class=: {html}"
    );
    assert!(
        html.contains("skip-link"),
        "expected consumer class present: {html}"
    );
    assert!(
        html.contains("ars-visually-hidden"),
        "expected component class still present: {html}"
    );
}

#[test]
fn heading_forwards_attr_data_attrs_to_root() {
    // `attr:` still works for data/aria/etc. — only `class` had the
    // duplicate-attribute problem since none of the new components emit
    // internal data-testid / lang / etc.
    let html = render(|| {
        view! {
            <Heading id="t" level=Level::Two attr:data-testid="x">
                "y"
            </Heading>
        }
        .to_html()
    });

    assert!(html.contains(r#"data-testid="x""#));
}
