use std::time::Duration;

use ars_collections::{Collection as _, selection};
use ars_components::selection::select;
use ars_core::{Env, HtmlAttr, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

use super::common::{arb_disabled_keys, arb_key, select_collection};

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
        Just(select::Event::TypeaheadSearch(
            'b',
            Duration::from_millis(100)
        )),
        Just(select::Event::TypeaheadSearch(
            'd',
            Duration::from_millis(700)
        )),
        Just(select::Event::CompositionStart),
        Just(select::Event::CompositionEnd),
        Just(select::Event::Focus { is_keyboard: true }),
        Just(select::Event::Blur),
        Just(select::Event::ClickOutside),
        Just(select::Event::ClearTypeahead),
        prop::bool::ANY.prop_map(select::Event::SetDescriptionPresent),
        Just(select::Event::SyncProps),
        prop::bool::ANY.prop_map(|use_empty| {
            if use_empty {
                select::Event::UpdateItems(ars_collections::StaticCollection::default())
            } else {
                select::Event::UpdateItems(select_collection())
            }
        }),
    ]
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
    #![proptest_config(crate::common::proptest_config())]

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
