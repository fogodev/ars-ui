use ars_components::overlay::{
    popover,
    positioning::{Placement, PositioningSnapshot},
};
use ars_core::{Env, HtmlAttr, Service};

use crate::helper::assert_anatomy;

#[test]
fn popover_anatomy_matches_spec() {
    assert_anatomy(
        "popover",
        &[
            (popover::Part::Root, "root"),
            (popover::Part::Anchor, "anchor"),
            (popover::Part::Trigger, "trigger"),
            (popover::Part::Positioner, "positioner"),
            (popover::Part::Content, "content"),
            (popover::Part::Arrow, "arrow"),
            (popover::Part::Title, "title"),
            (popover::Part::Description, "description"),
            (popover::Part::CloseTrigger, "close-trigger"),
        ],
    );
}

#[test]
fn popover_positioner_and_arrow_attrs_carry_reported_placement() {
    let mut service = Service::<popover::Machine>::new(
        popover::Props::new().id("popover").default_open(true),
        &Env::default(),
        &popover::Messages::default(),
    );

    drop(
        service.send(popover::Event::PositioningUpdate(PositioningSnapshot {
            placement: Placement::TopStart,
            arrow: None,
        })),
    );

    let api = service.connect(&|_| {});

    assert_eq!(
        api.positioner_attrs().get(&HtmlAttr::Data("ars-placement")),
        Some("top-start")
    );
    assert_eq!(
        api.arrow_attrs().get(&HtmlAttr::Data("ars-placement")),
        Some("top-start")
    );
}
