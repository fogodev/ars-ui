//! Property-based tests for selection component state machines.

use std::collections::BTreeSet;

use ars_collections::{Collection, CollectionBuilder, Key, selection};
use ars_components::selection::{listbox, select};
use ars_core::{AriaAttr, Env, HtmlAttr, Service};
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
}
