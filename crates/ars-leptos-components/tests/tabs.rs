//! SSR tests for styled Leptos Tabs components.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use ars_leptos::prelude::tabs;
use ars_leptos_components::navigation::tabs::{css, tailwind};
use leptos::{prelude::*, reactive::owner::Owner};

type TestTab = tabs::Tab<&'static str>;

fn render(view_fn: impl FnOnce() -> String + 'static) -> String {
    let owner = Owner::new();
    let result = owner.with(view_fn);

    drop(owner);

    result
}

fn two_tabs() -> [TestTab; 2] {
    [
        tabs::Tab::new_static("first", "First", view! { <p>"First panel"</p> }),
        tabs::Tab::new_static("second", "Second", view! { <p>"Second panel"</p> }),
    ]
}

#[test]
fn styled_tabs_forward_controlled_value_to_primitive_root() {
    let html = render(|| {
        view! {
            <css::Tabs
                default_value="first"
                value=Signal::derive(|| Some("second"))
                tabs=two_tabs()
            />
        }
        .to_html()
    });

    assert!(
        html.contains(r#"data-ars-part="tab""#) && html.contains(r#"Second"#),
        "controlled styled tabs should render tab anatomy: {html}"
    );
    assert!(
        html.contains(r#"aria-selected="true""#) && html.contains("Second panel"),
        "controlled value should select the externally provided tab: {html}"
    );
}

#[test]
fn styled_tabs_accept_absent_reorder_callback_for_external_sources() {
    let html = render(|| {
        view! {
            <tailwind::Tabs
                default_value="first"
                tabs=two_tabs()
                reorderable=true
            />
        }
        .to_html()
    });

    assert!(
        html.contains(r#"aria-roledescription="draggable tab""#),
        "reorderable styled tabs should still render draggable semantics: {html}"
    );
}
