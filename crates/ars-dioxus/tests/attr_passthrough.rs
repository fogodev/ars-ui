//! Regression tests for the `#[props(extends = GlobalAttributes)]` pass-through
//! on every Dioxus adapter component that owns its rendered root.
//!
//! These tests confirm that consumer-supplied `class`, `style`, `data-*`,
//! `aria-*` etc. flow onto the component root and that tokenized values
//! (`class`, `style`) concatenate with the component's own tokens rather than
//! clobbering them.

#![cfg(not(target_arch = "wasm32"))]

use std::{cell::RefCell, rc::Rc};

use ars_dioxus::{
    navigation::tabs::{Tab, Tabs},
    utility::{
        button::Button,
        heading::{Heading, Level},
        highlight::Highlight,
        landmark::{Landmark, Role},
        separator::Separator,
        visually_hidden::VisuallyHidden,
    },
};
use dioxus::{
    dioxus_core::{NoOpMutations, ScopeId},
    prelude::*,
};

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);
    vdom.rebuild_in_place();
    dioxus_ssr::render(&vdom)
}

#[test]
fn heading_forwards_class_and_data_attrs_to_root() {
    fn app() -> Element {
        rsx! {
            Heading {
                id: "h",
                level: Level::Two,
                class: "text-4xl font-bold",
                "data-testid": "title",
                "Title"
            }
        }
    }

    let html = render_app(app);

    assert!(html.trim_start().starts_with("<h2"), "expected h2: {html}");
    assert!(
        html.contains(r#"class="text-4xl font-bold""#),
        "expected class on h2: {html}"
    );
    assert!(
        html.contains(r#"data-testid="title""#),
        "expected data-testid on h2: {html}"
    );
}

#[test]
fn landmark_forwards_class_to_native_role_root() {
    fn app() -> Element {
        rsx! {
            Landmark { id: "n", role: Role::Navigation, class: "sidebar-nav", "links" }
        }
    }

    let html = render_app(app);

    assert!(html.trim_start().starts_with("<nav"));
    assert!(
        html.contains(r#"class="sidebar-nav""#),
        "expected class on nav root: {html}"
    );
}

#[test]
fn landmark_forwards_class_to_search_fallback_div() {
    fn app() -> Element {
        rsx! {
            Landmark { id: "s", role: Role::Search, class: "site-search", "input" }
        }
    }

    let html = render_app(app);

    assert!(html.trim_start().starts_with("<div"));
    assert!(
        html.contains(r#"class="site-search""#),
        "expected class on search fallback div: {html}"
    );
}

#[test]
fn highlight_forwards_class_to_root_span() {
    fn app() -> Element {
        rsx! {
            Highlight {
                query: vec!["x".to_string()],
                text: "hello",
                class: "hl-root",
            }
        }
    }

    let html = render_app(app);

    assert!(html.trim_start().starts_with("<span"));
    assert!(
        html.contains(r#"class="hl-root""#),
        "expected class on highlight root span: {html}"
    );
}

#[test]
fn separator_forwards_class_to_hr_root() {
    fn app() -> Element {
        rsx! {
            Separator { id: "sep", class: "menu-divider" }
        }
    }

    let html = render_app(app);

    assert!(html.trim_start().starts_with("<hr"));
    assert!(
        html.contains(r#"class="menu-divider""#),
        "expected class on hr root: {html}"
    );
}

#[test]
fn visually_hidden_merges_consumer_class_with_component_class() {
    // VisuallyHidden's `<span>` already carries the `ars-visually-hidden`
    // class. Consumer class tokens must concatenate, not clobber.
    fn app() -> Element {
        rsx! {
            VisuallyHidden { id: "v", class: "skip-link", "Hidden" }
        }
    }

    let html = render_app(app);

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
fn heading_data_ars_scope_wins_over_consumer_override() {
    // Ordinary (non-token) attributes prefer the component's value on
    // conflict — the component-managed `data-ars-scope` must not be
    // overridable by a consumer-supplied attribute.
    fn app() -> Element {
        rsx! {
            Heading { level: Level::Two, "data-ars-scope": "consumer-tried", "Title" }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"data-ars-scope="heading""#),
        "component-managed data-ars-scope must survive: {html}"
    );
    assert!(
        !html.contains(r#"data-ars-scope="consumer-tried""#),
        "consumer override must NOT win on component-managed attrs: {html}"
    );
}

#[test]
fn button_updates_root_attrs_when_props_change() {
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn app(class_slot: Rc<RefCell<Option<Signal<&'static str>>>>) -> Element {
        let class_name = use_signal(|| "initial-button");

        *class_slot.borrow_mut() = Some(class_name);

        rsx! {
            Button {
                id: "reactive-button",
                class: "{class_name}",
                "data-state": "{class_name}",
                "Save"
            }
        }
    }

    let class_slot = Rc::new(RefCell::new(None));
    let mut dom = VirtualDom::new_with_props(app, Rc::clone(&class_slot));

    dom.rebuild_in_place();

    let html = dioxus_ssr::render(&dom);

    assert!(
        html.contains(r#"class="initial-button""#),
        "initial class should render: {html}"
    );
    assert!(
        html.contains(r#"data-state="initial-button""#),
        "initial pass-through attr should render: {html}"
    );

    class_slot
        .borrow()
        .expect("class signal initialized")
        .set("updated-button");

    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);

    let html = dioxus_ssr::render(&dom);

    assert!(
        html.contains(r#"class="updated-button""#),
        "updated class should render after prop-only changes: {html}"
    );
    assert!(
        html.contains(r#"data-state="updated-button""#),
        "updated pass-through attr should render after prop-only changes: {html}"
    );
}

#[test]
fn tabs_updates_root_attrs_when_props_change() {
    #[expect(
        clippy::needless_pass_by_value,
        reason = "Dioxus root props are moved into the render function."
    )]
    fn app(class_slot: Rc<RefCell<Option<Signal<&'static str>>>>) -> Element {
        let class_name = use_signal(|| "initial-tabs");

        *class_slot.borrow_mut() = Some(class_name);

        rsx! {
            Tabs {
                default_value: "first",
                class: "{class_name}",
                "data-state": "{class_name}",
                tabs: [
                    Tab::new_static("first", "First", rsx! { p { "First panel" } }),
                    Tab::new_static("second", "Second", rsx! { p { "Second panel" } }),
                ],
            }
        }
    }

    let class_slot = Rc::new(RefCell::new(None));
    let mut dom = VirtualDom::new_with_props(app, Rc::clone(&class_slot));

    dom.rebuild_in_place();

    let html = dioxus_ssr::render(&dom);

    assert!(
        html.contains(r#"class="initial-tabs""#),
        "initial class should render: {html}"
    );
    assert!(
        html.contains(r#"data-state="initial-tabs""#),
        "initial pass-through attr should render: {html}"
    );

    class_slot
        .borrow()
        .expect("class signal initialized")
        .set("updated-tabs");

    dom.mark_dirty(ScopeId::APP);
    dom.render_immediate(&mut NoOpMutations);

    let html = dioxus_ssr::render(&dom);

    assert!(
        html.contains(r#"class="updated-tabs""#),
        "updated class should render after prop-only changes: {html}"
    );
    assert!(
        html.contains(r#"data-state="updated-tabs""#),
        "updated pass-through attr should render after prop-only changes: {html}"
    );
}
