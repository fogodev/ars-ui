//! SSR tests for the Dioxus `as_child` adapter slot.

#![cfg(not(target_arch = "wasm32"))]

use std::cell::RefCell;

use ars_components::utility::as_child::AsChildMerge;
use ars_core::{AriaAttr, AttrMap, CssProperty, HtmlAttr, StyleStrategy};
use ars_dioxus::{
    as_child::{AsChildRenderProps, AsChildSlot, AsChildSlotProps, merge_dioxus_attrs},
    attr_map_to_dioxus, attr_map_to_dioxus_inline_attrs,
};
use dioxus::{dioxus_core::AttributeValue, prelude::*};

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();

    dioxus_ssr::render(&vdom)
}

fn component_attrs() -> AttrMap {
    let mut attrs = AttrMap::new();

    attrs
        .set(HtmlAttr::Id, "trigger")
        .set(HtmlAttr::Role, "button")
        .set(HtmlAttr::Data("ars-scope"), "as-child-test")
        .set(HtmlAttr::Data("ars-part"), "trigger")
        .set(HtmlAttr::Aria(AriaAttr::Expanded), "false");

    attrs
}

fn merged_attrs() -> AttrMap {
    let mut child_attrs = AttrMap::new();

    child_attrs
        .set(HtmlAttr::Class, "child")
        .set(HtmlAttr::Aria(AriaAttr::DescribedBy), "child-hint")
        .set(HtmlAttr::Aria(AriaAttr::LabelledBy), "child-label")
        .set_style(CssProperty::Color, "red");

    let mut component_attrs = component_attrs();

    component_attrs
        .set(HtmlAttr::Class, "component")
        .set(HtmlAttr::Aria(AriaAttr::DescribedBy), "component-hint")
        .set(HtmlAttr::Aria(AriaAttr::LabelledBy), "component-label")
        .set_style(CssProperty::Display, "inline-flex");

    component_attrs.merge_onto(child_attrs)
}

fn native_dioxus_attr(name: &'static str, value: &'static str) -> Attribute {
    Attribute::new(name, AttributeValue::Text(String::from(value)), None, false)
}

#[test]
fn as_child_slot_props_debug_redacts_render_callback() {
    thread_local! {
        static DEBUG_OUTPUT: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    fn app() -> Element {
        let props = AsChildSlotProps {
            attrs: vec![native_dioxus_attr("data-direct", "yes")],
            render: Callback::new(|slot: AsChildRenderProps| {
                rsx! {
                    button { r#type: "button", ..slot.attrs, "Launch" }
                }
            }),
        };

        DEBUG_OUTPUT.with(|debug| {
            debug.borrow_mut().replace(format!("{props:?}"));
        });

        rsx! {
            div {}
        }
    }

    DEBUG_OUTPUT.with(|debug| debug.borrow_mut().take());

    let html = render_app(app);

    let debug = DEBUG_OUTPUT
        .with(|debug| debug.borrow().clone())
        .unwrap_or_else(|| panic!("debug output was not captured: {html}"));

    assert!(
        debug.contains("AsChildSlotProps"),
        "missing props name: {debug}"
    );
    assert!(debug.contains("attrs"), "missing attrs field: {debug}");
    assert!(
        debug.contains("render: \"<callback>\""),
        "render callback should be redacted: {debug}"
    );
}

#[test]
fn as_child_slot_passes_converted_attrs_to_render_callback() {
    thread_local! {
        static RECEIVED_ATTRS: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
    }

    fn app() -> Element {
        let attrs = attr_map_to_dioxus_inline_attrs(component_attrs());

        rsx! {
            AsChildSlot {
                attrs,
                render: move |slot: AsChildRenderProps| {
                    RECEIVED_ATTRS.with(|received| received.borrow_mut().push(slot.attrs.len()));

                    rsx! {
                        button { r#type: "button", ..slot.attrs, "Launch" }
                    }
                },
            }
        }
    }

    RECEIVED_ATTRS.with(|received| received.borrow_mut().clear());

    let html = render_app(app);

    let received = RECEIVED_ATTRS.with(|received| received.borrow().clone());

    assert_eq!(received, vec![5], "callback did not receive attrs: {html}");
    assert!(html.contains(r#"id="trigger""#), "missing id: {html}");
    assert!(html.contains(r#"role="button""#), "missing role: {html}");
    assert!(
        html.contains(r#"data-ars-scope="as-child-test""#),
        "missing scope: {html}"
    );
    assert!(
        html.contains(r#"data-ars-part="trigger""#),
        "missing part: {html}"
    );
    assert!(
        html.contains(r#"aria-expanded="false""#),
        "missing aria-expanded: {html}"
    );
}

#[test]
fn as_child_slot_accepts_native_attrs_without_attr_map_conversion() {
    fn app() -> Element {
        let attrs = vec![
            native_dioxus_attr("data-direct", "yes"),
            native_dioxus_attr("aria-label", "Native label"),
        ];

        rsx! {
            AsChildSlot {
                attrs,
                render: |slot: AsChildRenderProps| rsx! {
                    button { r#type: "button", ..slot.attrs, "Launch" }
                },
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"data-direct="yes""#),
        "missing direct data attr: {html}"
    );
    assert!(
        html.contains(r#"aria-label="Native label""#),
        "missing direct aria attr: {html}"
    );
}

#[test]
fn as_child_slot_collects_extended_global_attrs() {
    fn app() -> Element {
        rsx! {
            AsChildSlot {
                id: "extended-trigger",
                role: "button",
                class: "extended-class",
                aria_label: "Extended label",
                render: |slot: AsChildRenderProps| rsx! {
                    button { r#type: "button", ..slot.attrs, "Launch" }
                },
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"id="extended-trigger""#),
        "missing extended id: {html}"
    );
    assert!(
        html.contains(r#"role="button""#),
        "missing extended role: {html}"
    );
    assert!(
        html.contains(r#"class="extended-class""#),
        "missing extended class: {html}"
    );
    assert!(
        html.contains(r#"aria-label="Extended label""#),
        "missing extended aria-label: {html}"
    );
}

#[test]
fn as_child_slot_render_callback_spreads_attrs_without_wrapper() {
    fn app() -> Element {
        let attrs = attr_map_to_dioxus_inline_attrs(component_attrs());

        rsx! {
            AsChildSlot {
                attrs,
                render: |slot: AsChildRenderProps| rsx! {
                    button { r#type: "button", ..slot.attrs, "Launch" }
                },
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.trim_start().starts_with("<button"),
        "slot should render the child root directly: {html}"
    );
    assert!(!html.contains("<div"), "unexpected wrapper div: {html}");
}

#[test]
fn as_child_slot_preserves_class_style_and_aria_values() {
    fn app() -> Element {
        let attrs = attr_map_to_dioxus_inline_attrs(merged_attrs());

        rsx! {
            AsChildSlot {
                attrs,
                render: |slot: AsChildRenderProps| rsx! {
                    button { r#type: "button", ..slot.attrs, "Launch" }
                },
            }
        }
    }

    let html = render_app(app);

    assert!(html.contains(r#"class=""#), "missing class attr: {html}");
    assert!(html.contains("child"), "missing child class token: {html}");
    assert!(
        html.contains("component"),
        "missing component class token: {html}"
    );
    assert!(html.contains("color: red;"), "missing color style: {html}");
    assert!(
        html.contains("display: inline-flex;"),
        "missing display style: {html}"
    );
    assert!(
        html.contains(r#"aria-describedby="child-hint component-hint""#),
        "missing merged aria-describedby: {html}"
    );
    assert!(
        html.contains(r#"aria-labelledby="child-label component-label""#),
        "missing merged aria-labelledby: {html}"
    );
}

#[test]
fn as_child_render_props_merges_callback_root_attrs_before_spread() {
    fn app() -> Element {
        let mut attrs = AttrMap::new();

        attrs
            .set(HtmlAttr::Class, "component")
            .set(HtmlAttr::Aria(AriaAttr::LabelledBy), "component-label");

        let attrs = attr_map_to_dioxus_inline_attrs(attrs);

        rsx! {
            AsChildSlot {
                attrs,
                render: |slot: AsChildRenderProps| rsx! {
                    button {
                        r#type: "button",
                        ..slot.merged_attrs(
                            vec![
                                native_dioxus_attr("class", "child"),
                                native_dioxus_attr("aria-labelledby", "child-label"),
                            ],
                        ),
                        "Launch"
                    }
                },
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"class="child component""#),
        "child and component classes should merge before spread: {html}"
    );
    assert!(
        html.contains(r#"aria-labelledby="child-label component-label""#),
        "child and component aria-labelledby values should merge before spread: {html}"
    );
}

#[test]
fn merge_dioxus_attrs_replaces_non_mergeable_conflicts_with_component_attrs() {
    let attrs = merge_dioxus_attrs(
        vec![
            native_dioxus_attr("role", "link"),
            native_dioxus_attr("data-ars-state", "closed"),
        ],
        vec![
            native_dioxus_attr("role", "button"),
            native_dioxus_attr("data-ars-state", "open"),
        ],
    );

    assert_eq!(
        attrs,
        vec![
            native_dioxus_attr("role", "button"),
            native_dioxus_attr("data-ars-state", "open"),
        ]
    );
}

#[test]
fn as_child_slot_ssr_markup_is_hydration_stable() {
    fn app() -> Element {
        let attrs = attr_map_to_dioxus_inline_attrs(component_attrs());

        rsx! {
            AsChildSlot {
                attrs,
                render: |slot: AsChildRenderProps| rsx! {
                    a { href: "/docs", ..slot.attrs, "Docs" }
                },
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.trim_start().starts_with("<a"),
        "expected anchor as SSR root: {html}"
    );
    assert!(html.contains(r#"href="/docs""#), "missing href: {html}");
    assert!(
        html.contains(r#"data-ars-part="trigger""#),
        "missing forwarded part: {html}"
    );
    assert!(
        !html.contains("as-child-slot"),
        "slot should not add adapter marker markup: {html}"
    );
}

#[test]
fn attr_map_to_dioxus_preserves_as_child_style_strategy_payloads() {
    let mut attrs = component_attrs();

    attrs.set_style(CssProperty::Color, "red");

    let cssom = attr_map_to_dioxus(attrs.clone(), &StyleStrategy::Cssom, None);

    assert!(
        cssom
            .attrs
            .iter()
            .all(|attr| !format!("{attr:?}").contains("style")),
        "CSSOM strategy should not inline style attrs: {:?}",
        cssom.attrs
    );
    assert_eq!(
        cssom.cssom_styles,
        vec![(CssProperty::Color, String::from("red"))]
    );

    let nonce = attr_map_to_dioxus(
        attrs,
        &StyleStrategy::Nonce(String::from("nonce")),
        Some("trigger"),
    );

    assert_eq!(nonce.nonce_css_key.as_deref(), Some("trigger"));
    assert!(
        nonce.nonce_css.contains("color: red;"),
        "missing nonce CSS declaration: {}",
        nonce.nonce_css
    );
}
