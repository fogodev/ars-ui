use ars_components::{
    overlay::{popover, positioning::Placement},
    specialized::contextual_help::{Messages, Part, Props, Variant},
};
use ars_core::{AriaAttr, ConnectApi, Env, HtmlAttr, Service};
use ars_i18n::Direction;
use proptest::prelude::*;

fn arb_variant() -> impl Strategy<Value = Variant> {
    prop_oneof![Just(Variant::Help), Just(Variant::Info)]
}

fn arb_dir() -> impl Strategy<Value = Option<Direction>> {
    prop_oneof![
        Just(None),
        Just(Some(Direction::Ltr)),
        Just(Some(Direction::Rtl)),
        Just(Some(Direction::Auto)),
    ]
}

proptest! {
    #![proptest_config(super::super::common::proptest_config())]

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
    fn contextual_help_props_map_to_popover_and_attrs(
        id in "[a-z]{1,8}",
        variant in arb_variant(),
        open in any::<bool>(),
        offset in -32.0f64..=32.0,
        cross_offset in -32.0f64..=32.0,
        should_flip in any::<bool>(),
        dir in arb_dir(),
    ) {
        let props = Props {
            id: id.clone(),
            variant,
            placement: Placement::BottomStart,
            offset,
            cross_offset,
            should_flip,
            dir,
            ..Props::default()
        };

        let popover = props.popover_props();

        prop_assert_eq!(popover.id, id);
        prop_assert_eq!(popover.offset, offset);
        prop_assert_eq!(popover.cross_offset, cross_offset);
        prop_assert_eq!(popover.positioning.flip, should_flip);

        let mut popover_props = props.popover_props();

        popover_props.default_open = open;

        let service = Service::<popover::Machine>::new(
            popover_props,
            &Env::default(),
            &popover::Messages::default(),
        );

        let popover_api = service.connect(&|_| {});

        let api = ars_components::specialized::contextual_help::Api::new(
            popover_api,
            &props,
            &Env::default(),
            &Messages::default(),
        );

        let trigger = api.trigger_attrs();
        let content = api.content_attrs();

        let expected_label = match variant {
            Variant::Help => "Help",
            Variant::Info => "Information",
        };

        prop_assert_eq!(trigger.get(&HtmlAttr::Aria(AriaAttr::Label)), Some(expected_label));
        prop_assert_eq!(trigger.get(&HtmlAttr::Aria(AriaAttr::HasPopup)), Some("dialog"));
        prop_assert_eq!(
            trigger.get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some(if open { "true" } else { "false" })
        );
        prop_assert_eq!(trigger.get(&HtmlAttr::Data("ars-variant")), Some(variant.as_str()));

        prop_assert_eq!(content.get(&HtmlAttr::Role), Some("dialog"));
        prop_assert_eq!(content.get(&HtmlAttr::Data("ars-state")), Some(if open { "open" } else { "closed" }));
        prop_assert_eq!(content.get(&HtmlAttr::TabIndex), Some("-1"));
        prop_assert_eq!(api.part_attrs(Part::Trigger), trigger);
        prop_assert_eq!(api.part_attrs(Part::Content), content);
    }
}
