use ars_components::selection::segment_group;
use ars_core::{ConnectApi as _, Env, HtmlAttr, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

use super::common::{arb_disabled_keys, arb_key, key, segment_group_items};

fn arb_segment_group_event() -> impl Strategy<Value = segment_group::Event> {
    prop_oneof![
        arb_key().prop_map(segment_group::Event::SelectValue),
        Just(segment_group::Event::FocusNext),
        Just(segment_group::Event::FocusPrev),
        Just(segment_group::Event::FocusFirst),
        Just(segment_group::Event::FocusLast),
        arb_key().prop_map(|item| segment_group::Event::FocusItem {
            item,
            is_keyboard: true,
        }),
        Just(segment_group::Event::Blur),
        arb_key().prop_map(segment_group::Event::RegisterItem),
        arb_key().prop_map(segment_group::Event::UnregisterItem),
        prop::option::of(arb_key()).prop_map(segment_group::Event::SetValue),
        Just(segment_group::Event::SetProps),
        Just(segment_group::Event::Reset),
    ]
}

fn assert_segment_group_invariants(service: &Service<segment_group::Machine>) -> TestCaseResult {
    let ctx = service.context();

    if let Some(selected) = ctx.value.get() {
        prop_assert!(
            ctx.items
                .iter()
                .any(|item| &item.value == selected && !item.disabled)
        );
    }

    if let Some(focused) = &ctx.focused_item {
        prop_assert!(
            ctx.items
                .iter()
                .any(|item| &item.value == focused && !item.disabled)
        );
    }

    let api = service.connect(&|_| {});

    let zero_tabindex_items = ctx
        .items
        .iter()
        .filter(|item| {
            api.item_attrs(&item.value)
                .get(&HtmlAttr::TabIndex)
                .is_some_and(|value| value == "0")
        })
        .collect::<Vec<_>>();

    prop_assert!(zero_tabindex_items.len() <= 1);

    for item in zero_tabindex_items {
        prop_assert!(!item.disabled);
    }

    let sample = key("alpha");

    prop_assert_eq!(api.part_attrs(segment_group::Part::Root), api.root_attrs());
    prop_assert_eq!(
        api.part_attrs(segment_group::Part::Item {
            value: sample.clone()
        }),
        api.item_attrs(&sample)
    );
    prop_assert_eq!(
        api.part_attrs(segment_group::Part::ItemText {
            value: sample.clone()
        }),
        api.item_text_attrs(&sample)
    );
    prop_assert_eq!(
        api.part_attrs(segment_group::Part::Indicator),
        api.indicator_attrs()
    );
    prop_assert_eq!(
        api.part_attrs(segment_group::Part::HiddenInput),
        api.hidden_input_attrs()
    );

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    fn segment_group_preserves_selection_focus_and_part_invariants(
        disabled in arb_disabled_keys(),
        events in prop::collection::vec(arb_segment_group_event(), 0..40),
    ) {
        let props = segment_group::Props::new()
            .id("segment-group")
            .items(segment_group_items(&disabled));

        let mut service = Service::<segment_group::Machine>::new(
            props,
            &Env::default(),
            &segment_group::Messages,
        );

        for event in events {
            drop(service.send(event));

            assert_segment_group_invariants(&service)?;
        }
    }
}
