use ars_components::overlay::{
    positioning::{Placement, PositioningOptions},
    tooltip,
};
use ars_core::{AriaAttr, Env, HtmlAttr, Service};

use crate::helper::assert_anatomy;

#[test]
fn tooltip_anatomy_matches_spec() {
    assert_anatomy(
        "tooltip",
        &[
            (tooltip::Part::Root, "root"),
            (tooltip::Part::Trigger, "trigger"),
            (tooltip::Part::HiddenDescription, "hidden-description"),
            (tooltip::Part::Positioner, "positioner"),
            (tooltip::Part::Content, "content"),
            (tooltip::Part::Arrow, "arrow"),
        ],
    );
}

#[test]
fn tooltip_attrs_carry_hidden_description_and_placement_contracts() {
    let service = Service::<tooltip::Machine>::new(
        tooltip::Props::new()
            .id("tooltip")
            .default_open(true)
            .positioning(PositioningOptions {
                placement: Placement::TopStart,
                ..PositioningOptions::default()
            }),
        &Env::default(),
        &tooltip::Messages,
    );

    let api = service.connect(&|_| {});

    assert_eq!(
        api.trigger_attrs()
            .get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
        Some("tooltip-content-description")
    );
    assert_eq!(
        api.hidden_description_attrs()
            .get(&HtmlAttr::Data("ars-visually-hidden")),
        Some("true")
    );
    assert_eq!(
        api.positioner_attrs().get(&HtmlAttr::Data("ars-placement")),
        Some("top-start")
    );
    assert_eq!(
        api.arrow_attrs().get(&HtmlAttr::Data("ars-placement")),
        Some("top-start")
    );
}
