use ars_collections::Collection as _;
use ars_components::selection::menu_bar;
use ars_core::{Env, HtmlAttr, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

use super::common::{arb_key, menu_bar_collection};

fn arb_menu_bar_event() -> impl Strategy<Value = menu_bar::Event> {
    prop_oneof![
        arb_key().prop_map(menu_bar::Event::FocusItem),
        arb_key().prop_map(menu_bar::Event::ActivateMenu),
        Just(menu_bar::Event::DeactivateMenu),
        Just(menu_bar::Event::MoveToNextMenu),
        Just(menu_bar::Event::MoveToPrevMenu),
        Just(menu_bar::Event::Close),
        Just(menu_bar::Event::Focus { is_keyboard: true }),
        Just(menu_bar::Event::Focus { is_keyboard: false }),
        Just(menu_bar::Event::Blur),
        prop::bool::ANY.prop_map(|use_empty| {
            if use_empty {
                menu_bar::Event::UpdateMenus(ars_collections::StaticCollection::default())
            } else {
                menu_bar::Event::UpdateMenus(menu_bar_collection())
            }
        }),
        Just(menu_bar::Event::SyncProps),
    ]
}

fn assert_menu_bar_invariants(service: &Service<menu_bar::Machine>) -> TestCaseResult {
    let ctx = service.context();

    prop_assert_eq!(
        ctx.active_menu.is_some(),
        matches!(service.state(), menu_bar::State::Active { .. })
    );

    if service.props().disabled {
        prop_assert!(ctx.active_menu.is_none());
        prop_assert!(ctx.focused_item.is_none());
    }

    if let Some(active) = &ctx.active_menu {
        prop_assert!(ctx.menus.contains_key(active));
    }

    if let Some(focused) = &ctx.focused_item {
        prop_assert!(ctx.menus.contains_key(focused));
    }

    let api = service.connect(&|_| {});

    let zero_tabindex = ctx
        .menus
        .item_keys()
        .filter(|key| {
            api.menu_trigger_attrs(key)
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
    fn menu_bar_preserves_active_and_focus_invariants(
        disabled in prop::bool::ANY,
        events in prop::collection::vec(arb_menu_bar_event(), 0..40),
    ) {
        let props = menu_bar::Props::new()
            .id("menu-bar")
            .disabled(disabled);

        let mut service = Service::<menu_bar::Machine>::new(
            props,
            &Env::default(),
            &menu_bar::Messages,
        );

        drop(service.send(menu_bar::Event::UpdateMenus(menu_bar_collection())));

        for event in events {
            drop(service.send(event));

            assert_menu_bar_invariants(&service)?;
        }
    }
}
