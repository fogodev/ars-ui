use super::*;

#[test]
fn drop_zone_anatomy_matches_spec() {
    assert_anatomy(
        "drop-zone",
        &[(utility_core::drop_zone::Part::Root, "root")],
    );
}

#[test]
fn drop_zone_connect_api_uses_current_spec_attrs_not_stale_issue_attrs() {
    let mut service = Service::<utility_core::drop_zone::Machine>::new(
        utility_core::drop_zone::Props::new()
            .id("uploads")
            .accept(["image/*"]),
        &Env::default(),
        &utility_core::drop_zone::Messages::default(),
    );

    drop(service.send(utility_core::drop_zone::Event::DragEnter(
        utility_core::drop_zone::DragData {
            items: Vec::new(),
            types: vec!["image/png".to_string()],
        },
    )));

    let attrs = service.connect(&|_| {}).root_attrs();

    assert_eq!(
        attrs.get(&HtmlAttr::Aria(ars_core::AriaAttr::Description)),
        Some("Release to drop files"),
        "spec §3.1 uses aria-description because aria-dropeffect is deprecated",
    );
    assert_eq!(
        attrs.get(&HtmlAttr::Aria(ars_core::AriaAttr::DropEffect)),
        None,
        "issue #218 mentions aria-dropeffect, but the current spec is authoritative",
    );
    assert_eq!(attrs.get(&HtmlAttr::Data("ars-drag-over")), Some("true"));
    assert_eq!(
        attrs.get(&HtmlAttr::Data("ars-dragging-over")),
        None,
        "issue #218 mentions data-ars-dragging-over, but the current spec uses data-ars-drag-over",
    );
}
