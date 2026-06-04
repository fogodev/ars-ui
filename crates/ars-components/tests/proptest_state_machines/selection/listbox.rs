use std::time::Duration;

use ars_collections::{Collection as _, selection};
use ars_components::selection::listbox;
use ars_core::{AriaAttr, Env, HtmlAttr, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

use super::common::{arb_disabled_keys, arb_key, listbox_collection};

fn arb_listbox_event() -> impl Strategy<Value = listbox::Event> {
    prop_oneof![
        Just(listbox::Event::Focus { is_keyboard: true }),
        Just(listbox::Event::Blur),
        arb_key().prop_map(listbox::Event::SelectItem),
        arb_key().prop_map(listbox::Event::ToggleItem),
        arb_key().prop_map(|key| listbox::Event::HighlightItem(Some(key))),
        arb_key().prop_map(listbox::Event::ExtendSelection),
        Just(listbox::Event::HighlightNext),
        Just(listbox::Event::HighlightPrev),
        Just(listbox::Event::HighlightFirst),
        Just(listbox::Event::HighlightLast),
        Just(listbox::Event::HighlightPageUp),
        Just(listbox::Event::HighlightPageDown),
        Just(listbox::Event::SelectAll),
        Just(listbox::Event::DeselectAll),
        Just(listbox::Event::TypeaheadSearch(
            'b',
            Duration::from_millis(100)
        )),
        Just(listbox::Event::TypeaheadSearch(
            'd',
            Duration::from_millis(700)
        )),
        Just(listbox::Event::ClearTypeahead),
        Just(listbox::Event::CompositionStart),
        Just(listbox::Event::CompositionEnd),
        prop::bool::ANY.prop_map(listbox::Event::SetDescriptionPresent),
        Just(listbox::Event::SyncProps),
        prop::bool::ANY.prop_map(|use_empty| {
            if use_empty {
                listbox::Event::UpdateItems(ars_collections::StaticCollection::default())
            } else {
                listbox::Event::UpdateItems(listbox_collection())
            }
        }),
        arb_key().prop_map(listbox::Event::ItemActivated),
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

proptest! {
    #![proptest_config(crate::common::proptest_config())]

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
}
