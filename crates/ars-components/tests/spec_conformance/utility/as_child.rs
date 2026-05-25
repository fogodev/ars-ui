use ars_components::utility::as_child::{AsChildMerge, Props};
use ars_core::{AttrMap, HtmlAttr};

#[test]
fn as_child_props_default_matches_spec() {
    assert_eq!(Props::default(), Props { as_child: false });
    assert_eq!(Props::new(), Props { as_child: false });
    assert_eq!(Props::new().as_child(true), Props { as_child: true });
}

#[test]
fn as_child_merge_component_attrs_take_precedence_and_tokens_dedupe() {
    let mut component = AttrMap::new();

    component.set(HtmlAttr::Class, "ars-button primary");
    component.set(HtmlAttr::Role, "button");

    let mut child = AttrMap::new();

    child.set(HtmlAttr::Class, "primary consumer");
    child.set(HtmlAttr::Role, "link");

    let merged = component.merge_onto(child);

    assert_eq!(
        merged.get(&HtmlAttr::Class),
        Some("primary consumer ars-button")
    );
    assert_eq!(merged.get(&HtmlAttr::Role), Some("button"));
}
