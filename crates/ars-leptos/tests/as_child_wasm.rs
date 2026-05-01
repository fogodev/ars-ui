//! Browser coverage tests for the Leptos `as_child` adapter slot.

#![cfg(target_arch = "wasm32")]

use ars_core::{AriaAttr, AttrMap, HtmlAttr};
use ars_leptos::{
    as_child::{AsChildAttrs, AsChildSlot},
    attr_map_to_leptos_inline_attrs,
};
use leptos::prelude::*;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

fn attr_map_with_class(class: &'static str, labelled_by: &'static str) -> AttrMap {
    let mut attrs = AttrMap::new();

    attrs
        .set(HtmlAttr::Class, class)
        .set(HtmlAttr::Aria(AriaAttr::LabelledBy), labelled_by);

    attrs
}

#[wasm_bindgen_test]
fn as_child_attrs_cover_conversion_paths_in_browser() {
    let attrs = AsChildAttrs::from_attr_map(attr_map_with_class("component", "component-label"));

    assert_eq!(attrs.into_inner().len(), 2);

    let attrs = AsChildAttrs::from_merged_attr_maps(
        attr_map_with_class("component", "component-label"),
        attr_map_with_class("child", "child-label"),
    );

    assert_eq!(attrs.into_inner().len(), 2);

    let attrs = AsChildAttrs::from(attr_map_to_leptos_inline_attrs(attr_map_with_class(
        "native",
        "native-label",
    )));

    assert_eq!(attrs.into_inner().len(), 2);
}

#[wasm_bindgen_test]
fn as_child_slot_builds_typed_child_in_browser() {
    let attrs = AsChildAttrs::from_attr_map(attr_map_with_class("component", "component-label"));

    let _view = view! {
        <AsChildSlot attrs=attrs>
            <button type="button">"Launch"</button>
        </AsChildSlot>
    };
}
