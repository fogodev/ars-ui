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
    assert!(
        css::STYLES.contains(r#"[data-ars-part="tab-shell"][data-ars-focus-visible]:not("#),
        "CSS Tabs focus ring should consume mirrored shell focus state directly"
    );
    assert!(
        !css::STYLES.contains(":has("),
        "CSS Tabs focus ring should not depend on :has()"
    );

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

#[test]
fn tailwind_tabs_indicator_consumes_adapter_measurement_variables() {
    let html =
        render(|| view! { <tailwind::Tabs default_value="first" tabs=two_tabs() /> }.to_html());

    for fragment in [
        r#"**:data-[ars-part=tab-indicator]:w-(--ars-indicator-width)"#,
        r#"**:data-[ars-part=tab-indicator]:h-(--ars-indicator-height)"#,
        r#"**:data-[ars-part=tab-indicator]:translate-x-(--ars-indicator-left)"#,
        r#"**:data-[ars-part=tab-indicator]:translate-y-(--ars-indicator-top)"#,
    ] {
        assert!(html.contains(fragment), "missing {fragment}: {html}");
    }
}

#[test]
fn tailwind_tabs_gates_closable_spacing_to_enabled_closable_rows() {
    let html =
        render(|| view! { <tailwind::Tabs default_value="first" tabs=two_tabs() /> }.to_html());

    assert!(
        html.contains("[data-ars-closable]:not([data-ars-disabled])]:pr-2"),
        "shell spacing should require closable and enabled state: {html}"
    );
    assert!(
        html.contains(r#".group[data-ars-closable]:not([data-ars-disabled])_"#)
            && html.contains("]:pr-2"),
        "trigger spacing should require closable and enabled shell state: {html}"
    );
    assert!(
        !html.contains("data-ars-closable:pr-2"),
        "Tailwind template should not reserve close spacing for disabled closable tabs: {html}"
    );
}

#[test]
fn tailwind_tabs_root_does_not_define_unnamed_group_scope() {
    let html =
        render(|| view! { <tailwind::Tabs default_value="first" tabs=two_tabs() /> }.to_html());

    assert!(
        !html.contains(r#"group mt-6 grid gap-3 text-gray-900"#),
        "root-level group should not leak hover state to all close triggers: {html}"
    );
}
