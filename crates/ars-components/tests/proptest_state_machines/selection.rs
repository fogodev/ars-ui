//! Property-based tests for selection component state machines.

use std::{collections::BTreeSet, time::Duration};

use ars_collections::{Collection, CollectionBuilder, Key, selection};
use ars_components::selection::{
    autocomplete, combobox, context_menu, listbox, menu, menu_bar, segment_group, select,
};
use ars_core::{AriaAttr, ConnectApi as _, Env, HtmlAttr, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

fn key(value: &'static str) -> Key {
    Key::str(value)
}

fn arb_key() -> impl Strategy<Value = Key> {
    prop_oneof![
        Just(key("alpha")),
        Just(key("bravo")),
        Just(key("charlie")),
        Just(key("delta")),
    ]
}

fn arb_disabled_keys() -> impl Strategy<Value = BTreeSet<Key>> {
    prop::collection::vec(arb_key(), 0..3).prop_map(|keys| keys.into_iter().collect())
}

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
    ]
}

fn segment_group_items(disabled: &BTreeSet<Key>) -> Vec<segment_group::Segment> {
    [key("alpha"), key("bravo"), key("charlie"), key("delta")]
        .into_iter()
        .map(|value| {
            let is_disabled = disabled.contains(&value);

            segment_group::Segment::new(value).disabled(is_disabled)
        })
        .collect()
}

fn listbox_collection() -> ars_collections::StaticCollection<listbox::Item> {
    CollectionBuilder::new()
        .item(
            key("alpha"),
            "Alpha",
            listbox::Item {
                label: "Alpha".into(),
            },
        )
        .item(
            key("bravo"),
            "Bravo",
            listbox::Item {
                label: "Bravo".into(),
            },
        )
        .item(
            key("charlie"),
            "Charlie",
            listbox::Item {
                label: "Charlie".into(),
            },
        )
        .item(
            key("delta"),
            "Delta",
            listbox::Item {
                label: "Delta".into(),
            },
        )
        .build()
}

fn select_collection() -> ars_collections::StaticCollection<select::Item> {
    CollectionBuilder::new()
        .item(
            key("alpha"),
            "Alpha",
            select::Item {
                label: "Alpha".into(),
            },
        )
        .item(
            key("bravo"),
            "Bravo",
            select::Item {
                label: "Bravo".into(),
            },
        )
        .item(
            key("charlie"),
            "Charlie",
            select::Item {
                label: "Charlie".into(),
            },
        )
        .item(
            key("delta"),
            "Delta",
            select::Item {
                label: "Delta".into(),
            },
        )
        .build()
}

fn combobox_collection() -> ars_collections::StaticCollection<combobox::Item> {
    CollectionBuilder::new()
        .item(
            key("alpha"),
            "Alpha",
            combobox::Item {
                label: "Alpha".into(),
            },
        )
        .item(
            key("bravo"),
            "Bravo",
            combobox::Item {
                label: "Bravo".into(),
            },
        )
        .item(
            key("charlie"),
            "Charlie",
            combobox::Item {
                label: "Charlie".into(),
            },
        )
        .item(
            key("delta"),
            "Delta",
            combobox::Item {
                label: "Delta".into(),
            },
        )
        .build()
}

fn autocomplete_collection() -> ars_collections::StaticCollection<autocomplete::Item> {
    CollectionBuilder::new()
        .item(
            key("alpha"),
            "Alpha",
            autocomplete::Item {
                label: "Alpha".into(),
            },
        )
        .item(
            key("bravo"),
            "Bravo",
            autocomplete::Item {
                label: "Bravo".into(),
            },
        )
        .item(
            key("charlie"),
            "Charlie",
            autocomplete::Item {
                label: "Charlie".into(),
            },
        )
        .item(
            key("delta"),
            "Delta",
            autocomplete::Item {
                label: "Delta".into(),
            },
        )
        .build()
}

fn menu_collection() -> ars_collections::StaticCollection<menu::Item> {
    CollectionBuilder::new()
        .item(
            key("alpha"),
            "Alpha",
            menu::Item {
                label: "Alpha".into(),
                item_type: menu::ItemType::Normal,
                shortcut: None,
                aria_keyshortcuts: None,
                close_on_action: None,
            },
        )
        .item(
            key("bravo"),
            "Bravo",
            menu::Item {
                label: "Bravo".into(),
                item_type: menu::ItemType::Checkbox,
                shortcut: None,
                aria_keyshortcuts: None,
                close_on_action: Some(false),
            },
        )
        .item(
            key("charlie"),
            "Charlie",
            menu::Item {
                label: "Charlie".into(),
                item_type: menu::ItemType::Radio {
                    group: "density".into(),
                },
                shortcut: None,
                aria_keyshortcuts: None,
                close_on_action: Some(false),
            },
        )
        .item(
            key("delta"),
            "Delta",
            menu::Item {
                label: "Delta".into(),
                item_type: menu::ItemType::Submenu,
                shortcut: None,
                aria_keyshortcuts: None,
                close_on_action: None,
            },
        )
        .build()
}

fn menu_bar_collection() -> ars_collections::StaticCollection<menu_bar::Menu> {
    CollectionBuilder::new()
        .item(
            key("alpha"),
            "Alpha",
            menu_bar::Menu {
                label: "Alpha".into(),
            },
        )
        .item(
            key("bravo"),
            "Bravo",
            menu_bar::Menu {
                label: "Bravo".into(),
            },
        )
        .item(
            key("charlie"),
            "Charlie",
            menu_bar::Menu {
                label: "Charlie".into(),
            },
        )
        .item(
            key("delta"),
            "Delta",
            menu_bar::Menu {
                label: "Delta".into(),
            },
        )
        .build()
}

fn alternate_combobox_collection() -> ars_collections::StaticCollection<combobox::Item> {
    CollectionBuilder::new()
        .item(
            key("alpha"),
            "Alpha",
            combobox::Item {
                label: "Alpha".into(),
            },
        )
        .item(
            key("echo"),
            "Echo",
            combobox::Item {
                label: "Echo".into(),
            },
        )
        .build()
}

fn arb_listbox_event() -> impl Strategy<Value = listbox::Event> {
    prop_oneof![
        Just(listbox::Event::Focus { is_keyboard: true }),
        Just(listbox::Event::Blur),
        arb_key().prop_map(listbox::Event::SelectItem),
        arb_key().prop_map(listbox::Event::ToggleItem),
        arb_key().prop_map(|key| listbox::Event::HighlightItem(Some(key))),
        Just(listbox::Event::HighlightNext),
        Just(listbox::Event::HighlightPrev),
        Just(listbox::Event::HighlightFirst),
        Just(listbox::Event::HighlightLast),
        Just(listbox::Event::SelectAll),
        Just(listbox::Event::DeselectAll),
    ]
}

fn arb_select_event() -> impl Strategy<Value = select::Event> {
    prop_oneof![
        Just(select::Event::Open),
        Just(select::Event::Close),
        Just(select::Event::Toggle),
        arb_key().prop_map(select::Event::SelectItem),
        arb_key().prop_map(|key| select::Event::HighlightItem(Some(key))),
        Just(select::Event::HighlightNext),
        Just(select::Event::HighlightPrev),
        Just(select::Event::HighlightFirst),
        Just(select::Event::HighlightLast),
        Just(select::Event::Clear),
    ]
}

fn arb_combobox_event() -> impl Strategy<Value = combobox::Event> {
    prop_oneof![
        Just(combobox::Event::Focus { is_keyboard: true }),
        Just(combobox::Event::Blur),
        Just(combobox::Event::Open),
        Just(combobox::Event::Close),
        Just(combobox::Event::Clear),
        Just(combobox::Event::CompositionStart),
        Just(combobox::Event::CompositionEnd("br".into())),
        Just(combobox::Event::InputChange("a".into())),
        Just(combobox::Event::InputChange("br".into())),
        Just(combobox::Event::InputChange("zz".into())),
        arb_key().prop_map(combobox::Event::SelectItem),
        arb_key().prop_map(combobox::Event::SelectItemCtrl),
        arb_key().prop_map(combobox::Event::DeselectItem),
        arb_key().prop_map(|key| combobox::Event::HighlightItem(Some(key))),
        Just(combobox::Event::HighlightNext),
        Just(combobox::Event::HighlightPrev),
        Just(combobox::Event::HighlightFirst),
        Just(combobox::Event::HighlightLast),
        prop::bool::ANY.prop_map(combobox::Event::SetDescriptionPresent),
        prop::bool::ANY.prop_map(|use_alternate| {
            if use_alternate {
                combobox::Event::UpdateItems(alternate_combobox_collection())
            } else {
                combobox::Event::UpdateItems(combobox_collection())
            }
        }),
    ]
}

fn arb_autocomplete_event() -> impl Strategy<Value = autocomplete::Event> {
    prop_oneof![
        Just(autocomplete::Event::Focus { is_keyboard: true }),
        Just(autocomplete::Event::Blur),
        Just(autocomplete::Event::InputChange("a".into())),
        Just(autocomplete::Event::InputChange("br".into())),
        Just(autocomplete::Event::InputChange("zz".into())),
        Just(autocomplete::Event::DebounceExpired),
        Just(autocomplete::Event::CancelDebounce),
        Just(autocomplete::Event::RestartDebounce),
        prop::bool::ANY.prop_map(autocomplete::Event::SetLoading),
        arb_key().prop_map(autocomplete::Event::SelectItem),
        arb_key().prop_map(|key| autocomplete::Event::HighlightItem(Some(key))),
        Just(autocomplete::Event::HighlightNext),
        Just(autocomplete::Event::HighlightPrev),
        Just(autocomplete::Event::HighlightFirst),
        Just(autocomplete::Event::HighlightLast),
        Just(autocomplete::Event::SelectHighlighted),
        Just(autocomplete::Event::Clear),
        prop::bool::ANY.prop_map(|use_empty| {
            if use_empty {
                autocomplete::Event::UpdateItems(ars_collections::StaticCollection::default())
            } else {
                autocomplete::Event::UpdateItems(autocomplete_collection())
            }
        }),
    ]
}

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
    ]
}

fn arb_context_menu_event() -> impl Strategy<Value = context_menu::Event> {
    prop_oneof![
        Just(context_menu::Event::ContextOpen { x: 10.0, y: 20.0 }),
        Just(context_menu::Event::ContextOpen { x: 30.0, y: 40.0 }),
        Just(context_menu::Event::Close),
        arb_key().prop_map(context_menu::Event::SelectItem),
        arb_key().prop_map(context_menu::Event::ToggleCheckboxItem),
        arb_key().prop_map(context_menu::Event::OpenSubmenu),
        arb_key().prop_map(|key| context_menu::Event::HighlightItem(Some(key))),
        Just(context_menu::Event::HighlightNext),
        Just(context_menu::Event::HighlightPrev),
        Just(context_menu::Event::HighlightFirst),
        Just(context_menu::Event::HighlightLast),
        Just(context_menu::Event::CloseSubmenu),
        Just(context_menu::Event::ClickOutside),
        Just(context_menu::Event::TypeaheadSearch(
            'b',
            Duration::from_millis(100)
        )),
        Just(context_menu::Event::TypeaheadSearch(
            'd',
            Duration::from_millis(700)
        )),
        arb_key().prop_map(|value| context_menu::Event::SelectRadioItem {
            group: "density".into(),
            value,
        }),
    ]
}

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
    ]
}

fn assert_listbox_invariants(service: &Service<listbox::Machine>) -> TestCaseResult {
    let ctx = service.context();

    if let Some(highlighted) = &ctx.highlighted_key {
        prop_assert!(ctx.items.contains_key(highlighted));
        prop_assert!(!ctx.selection_state.disabled_keys.contains(highlighted));
    }

    if ctx.selection_state.mode == selection::Mode::Single {
        prop_assert!(ctx.selection.get().len() <= 1);
    }

    for selected in ctx.selection.get().keys() {
        prop_assert!(!ctx.selection_state.disabled_keys.contains(selected));
    }

    let api = service.connect(&|_| {});

    if let Some(active) = api
        .content_attrs()
        .get(&HtmlAttr::Aria(AriaAttr::ActiveDescendant))
    {
        let highlighted = ctx
            .highlighted_key
            .as_ref()
            .expect("active descendant requires highlight");

        prop_assert_eq!(active, ctx.ids.item("item", highlighted));
    }

    Ok(())
}

fn assert_select_invariants(service: &Service<select::Machine>) -> TestCaseResult {
    let ctx = service.context();

    prop_assert_eq!(ctx.open, matches!(service.state(), select::State::Open));

    if let Some(highlighted) = &ctx.highlighted_key {
        prop_assert!(ctx.items.contains_key(highlighted));
        prop_assert!(!ctx.selection_state.disabled_keys.contains(highlighted));
    }

    if ctx.selection_state.mode == selection::Mode::Single {
        prop_assert!(ctx.selection.get().len() <= 1);
    }

    let hidden = service.connect(&|_| {}).hidden_input_attrs();

    prop_assert_eq!(hidden.get(&HtmlAttr::TabIndex), Some("-1"));

    Ok(())
}

fn assert_combobox_invariants(service: &Service<combobox::Machine>) -> TestCaseResult {
    let ctx = service.context();

    prop_assert_eq!(ctx.open, matches!(service.state(), combobox::State::Open));

    if let Some(highlighted) = &ctx.highlighted_key {
        prop_assert!(ctx.items.contains_key(highlighted));
        prop_assert!(!ctx.selection_state.disabled_keys.contains(highlighted));

        if let Some(visible) = &ctx.visible_keys {
            prop_assert!(visible.contains(highlighted));
        }
    }

    for selected in ctx.selection.get().keys() {
        prop_assert!(ctx.items.contains_key(selected));
        prop_assert!(!ctx.selection_state.disabled_keys.contains(selected));
    }

    if ctx.selection_state.mode == selection::Mode::Single {
        prop_assert!(ctx.selection.get().len() <= 1);
    }

    let api = service.connect(&|_| {});

    let input = api.input_attrs();

    if let Some(active) = input.get(&HtmlAttr::Aria(AriaAttr::ActiveDescendant)) {
        let highlighted = ctx
            .highlighted_key
            .as_ref()
            .expect("active descendant requires highlight");

        prop_assert_eq!(active, ctx.ids.item("item", highlighted));
    }

    Ok(())
}

fn assert_autocomplete_invariants(service: &Service<autocomplete::Machine>) -> TestCaseResult {
    let ctx = service.context();

    prop_assert_eq!(
        ctx.loading,
        matches!(service.state(), autocomplete::State::Loading)
    );

    if let Some(highlighted) = &ctx.highlighted_key {
        prop_assert!(ctx.items.contains_key(highlighted));

        if let Some(visible) = &ctx.visible_keys {
            prop_assert!(visible.contains(highlighted));
        }
    }

    if let Some(selected) = &ctx.selected_key {
        prop_assert!(ctx.items.contains_key(selected));
    }

    let api = service.connect(&|_| {});

    let input = api.input_attrs();

    if let Some(active) = input.get(&HtmlAttr::Aria(AriaAttr::ActiveDescendant)) {
        let highlighted = ctx
            .highlighted_key
            .as_ref()
            .expect("active descendant requires highlight");

        prop_assert_eq!(active, ctx.ids.item("item", highlighted));
    }

    prop_assert_eq!(api.visible_count(), api.visible_items().count());

    Ok(())
}

fn assert_menu_invariants(service: &Service<menu::Machine>) -> TestCaseResult {
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

fn assert_context_menu_invariants(service: &Service<context_menu::Machine>) -> TestCaseResult {
    let ctx = service.context();

    prop_assert_eq!(
        ctx.open,
        matches!(service.state(), context_menu::State::Open)
    );
    prop_assert_eq!(ctx.position.is_some(), ctx.open);

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
    #![proptest_config(super::common::proptest_config())]

    #[test]
    #[ignore]
    fn listbox_preserves_selection_invariants(
        disabled in arb_disabled_keys(),
        events in prop::collection::vec(arb_listbox_event(), 0..40),
    ) {
        let props = listbox::Props::new()
            .id("listbox")
            .selection_mode(selection::Mode::Multiple)
            .disabled_keys(disabled);

        let mut service = Service::<listbox::Machine>::new(
            props,
            &Env::default(),
            &listbox::Messages::default(),
        );

        drop(service.send(listbox::Event::UpdateItems(listbox_collection())));

        for event in events {
            drop(service.send(event));

            assert_listbox_invariants(&service)?;
        }
    }

    #[test]
    #[ignore]
    fn select_preserves_open_and_selection_invariants(
        disabled in arb_disabled_keys(),
        events in prop::collection::vec(arb_select_event(), 0..40),
    ) {
        let props = select::Props::new()
            .id("select")
            .multiple(true)
            .selection_mode(selection::Mode::Multiple)
            .disabled_keys(disabled);

        let mut service = Service::<select::Machine>::new(
            props,
            &Env::default(),
            &select::Messages::default(),
        );

        drop(service.send(select::Event::UpdateItems(select_collection())));

        for event in events {
            drop(service.send(event));

            assert_select_invariants(&service)?;
        }
    }

    #[test]
    #[ignore]
    fn combobox_preserves_filter_highlight_and_selection_invariants(
        disabled in arb_disabled_keys(),
        events in prop::collection::vec(arb_combobox_event(), 0..40),
    ) {
        let props = combobox::Props::new()
            .id("combobox")
            .selection_mode(selection::Mode::Multiple)
            .disabled_keys(disabled)
            .open_on_focus(false);

        let mut service = Service::<combobox::Machine>::new(
            props,
            &Env::default(),
            &combobox::Messages::default(),
        );

        drop(service.send(combobox::Event::UpdateItems(combobox_collection())));

        for event in events {
            let before_visible = service.context().visible_keys.clone();
            let before_highlighted = service.context().highlighted_key.clone();
            let before_input = service.context().input_value.get().clone();
            let was_composing = service.context().is_composing;

            let is_input_change = matches!(event, combobox::Event::InputChange(_));

            drop(service.send(event));

            if was_composing && is_input_change {
                prop_assert_eq!(service.context().visible_keys.clone(), before_visible);
                prop_assert_eq!(
                    service.context().highlighted_key.clone(),
                    before_highlighted
                );
                prop_assert_eq!(service.context().input_value.get(), &before_input);
            }

            assert_combobox_invariants(&service)?;
        }
    }

    #[test]
    fn autocomplete_preserves_filter_highlight_and_selection_invariants(
        events in prop::collection::vec(arb_autocomplete_event(), 0..40),
    ) {
        let props = autocomplete::Props::new()
            .id("autocomplete")
            .items(autocomplete_collection())
            .debounce(Duration::from_millis(50));

        let mut service = Service::<autocomplete::Machine>::new(
            props,
            &Env::default(),
            &autocomplete::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            assert_autocomplete_invariants(&service)?;
        }
    }

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

    #[test]
    #[ignore]
    fn context_menu_preserves_open_highlight_and_selection_invariants(
        disabled in arb_disabled_keys(),
        events in prop::collection::vec(arb_context_menu_event(), 0..40),
    ) {
        let props = context_menu::Props::new()
            .id("context-menu")
            .close_on_action(false)
            .disabled_keys(disabled);

        let mut service = Service::<context_menu::Machine>::new(
            props,
            &Env::default(),
            &context_menu::Messages,
        );

        drop(service.send(context_menu::Event::UpdateItems(menu_collection())));

        for event in events {
            drop(service.send(event));

            assert_context_menu_invariants(&service)?;
        }
    }

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
