use std::time::Duration;

use ars_collections::Collection as _;
use ars_components::selection::autocomplete;
use ars_core::{AriaAttr, Env, HtmlAttr, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

use super::common::{arb_key, autocomplete_collection};

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

proptest! {
    #![proptest_config(crate::common::proptest_config())]

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
}
