use ars_collections::{Collection as _, selection};
use ars_components::selection::combobox;
use ars_core::{AriaAttr, Env, HtmlAttr, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

use super::common::{
    alternate_combobox_collection, arb_disabled_keys, arb_key, combobox_collection,
};

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
        arb_key().prop_map(combobox::Event::ItemPointerDown),
        Just(combobox::Event::HighlightNext),
        Just(combobox::Event::HighlightPrev),
        Just(combobox::Event::HighlightFirst),
        Just(combobox::Event::HighlightLast),
        Just(combobox::Event::Dismiss),
        Just(combobox::Event::ClickOutside),
        Just(combobox::Event::CommitInput),
        Just(combobox::Event::ClearInlineCompletion),
        prop::bool::ANY.prop_map(combobox::Event::SetDescriptionPresent),
        Just(combobox::Event::SyncProps),
        prop::bool::ANY.prop_map(|use_alternate| {
            if use_alternate {
                combobox::Event::UpdateItems(alternate_combobox_collection())
            } else {
                combobox::Event::UpdateItems(combobox_collection())
            }
        }),
    ]
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

proptest! {
    #![proptest_config(crate::common::proptest_config())]

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
}
