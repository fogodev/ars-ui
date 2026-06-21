//! SSR tests for styled Dioxus Tabs components.

#![cfg(not(target_arch = "wasm32"))]

use ars_dioxus::prelude::tabs;
use ars_dioxus_components::navigation::tabs::tailwind;
use dioxus::prelude::*;

type TestTab = tabs::Tab<&'static str>;

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();

    dioxus_ssr::render(&vdom)
}

fn two_tabs() -> [TestTab; 2] {
    [
        tabs::Tab::new_static("first", "First", rsx! { p { "First panel" } }),
        tabs::Tab::new_static("second", "Second", rsx! { p { "Second panel" } }),
    ]
}

#[test]
fn tailwind_tabs_indicator_consumes_adapter_measurement_variables() {
    fn app() -> Element {
        rsx! { tailwind::Tabs { default_value: "first", tabs: two_tabs() } }
    }

    let html = render_app(app);

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
fn tailwind_tabs_root_does_not_define_unnamed_group_scope() {
    fn app() -> Element {
        rsx! { tailwind::Tabs { default_value: "first", tabs: two_tabs() } }
    }

    let html = render_app(app);

    assert!(
        !html.contains(r#"group mt-6 grid gap-3 text-gray-900"#),
        "root-level group should not leak hover state to all close triggers: {html}"
    );
}
