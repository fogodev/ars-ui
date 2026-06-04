use std::time::Duration;

use ars_collections::Collection as _;
use ars_components::selection::menu;
use ars_core::{Env, HtmlAttr, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

use super::common::{arb_disabled_keys, arb_key, menu_collection};

fn arb_menu_event() -> impl Strategy<Value = menu::Event> {
    prop_oneof![
        Just(menu::Event::Open),
        Just(menu::Event::Close),
        arb_key().prop_map(menu::Event::SelectItem),
        arb_key().prop_map(menu::Event::ToggleCheckboxItem),
        arb_key().prop_map(menu::Event::OpenSubmenu),
        arb_key().prop_map(|key| menu::Event::HighlightItem(Some(key))),
        Just(menu::Event::HighlightNext),
        Just(menu::Event::HighlightPrev),
        Just(menu::Event::HighlightFirst),
        Just(menu::Event::HighlightLast),
        Just(menu::Event::CloseSubmenu),
        Just(menu::Event::ClickOutside),
        Just(menu::Event::Focus { is_keyboard: true }),
        Just(menu::Event::Blur),
        Just(menu::Event::TypeaheadSearch(
            'b',
            Duration::from_millis(100)
        )),
        Just(menu::Event::TypeaheadSearch(
            'd',
            Duration::from_millis(700)
        )),
        arb_key().prop_map(|value| menu::Event::SelectRadioItem {
            group: "density".into(),
            value,
        }),
        prop::bool::ANY.prop_map(|use_empty| {
            if use_empty {
                menu::Event::UpdateItems(ars_collections::StaticCollection::default())
            } else {
                menu::Event::UpdateItems(menu_collection())
            }
        }),
        Just(menu::Event::SyncProps),
    ]
}

pub(crate) fn assert_menu_invariants(service: &Service<menu::Machine>) -> TestCaseResult {
    let ctx = service.context();

    prop_assert_eq!(ctx.open, matches!(service.state(), menu::State::Open));

    if let Some(highlighted) = &ctx.highlighted_key {
        prop_assert!(ctx.items.contains_key(highlighted));
        prop_assert!(
            ctx.items
                .get(highlighted)
                .is_some_and(ars_collections::Node::is_focusable)
        );
        prop_assert!(
            !service.props().disabled_keys.contains(highlighted)
                || service.props().disabled_behavior
                    == ars_collections::DisabledBehavior::FocusOnly
        );
    }

    if let Some(submenu) = &ctx.submenu_open {
        let item = ctx
            .items
            .get(submenu)
            .and_then(|node| node.value.as_ref())
            .expect("submenu key must reference an item");

        prop_assert!(matches!(item.item_type, menu::ItemType::Submenu));
    }

    for (key, checked) in &ctx.checked_items {
        if *checked {
            let item = ctx
                .items
                .get(key)
                .and_then(|node| node.value.as_ref())
                .expect("checked key must reference an item");

            prop_assert!(matches!(item.item_type, menu::ItemType::Checkbox));
            prop_assert!(!service.props().disabled_keys.contains(key));
        }
    }

    for (group, key) in &ctx.radio_groups {
        let item = ctx
            .items
            .get(key)
            .and_then(|node| node.value.as_ref())
            .expect("radio key must reference an item");

        let group_matches = matches!(&item.item_type, menu::ItemType::Radio { group: item_group } if item_group == group);

        prop_assert!(group_matches);
        prop_assert!(!service.props().disabled_keys.contains(key));
    }

    let api = service.connect(&|_| {});

    let zero_tabindex = ctx
        .items
        .item_keys()
        .filter(|key| {
            api.item_attrs(key)
                .get(&HtmlAttr::TabIndex)
                .is_some_and(|value| value == "0")
        })
        .count();

    prop_assert!(zero_tabindex <= 1);

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore]
    fn menu_preserves_open_highlight_and_selection_invariants(
        disabled in arb_disabled_keys(),
        events in prop::collection::vec(arb_menu_event(), 0..40),
    ) {
        let props = menu::Props::new()
            .id("menu")
            .close_on_action(false)
            .disabled_keys(disabled);

        let mut service = Service::<menu::Machine>::new(
            props,
            &Env::default(),
            &menu::Messages,
        );

        drop(service.send(menu::Event::UpdateItems(menu_collection())));

        for event in events {
            drop(service.send(event));

            assert_menu_invariants(&service)?;
        }
    }
}
