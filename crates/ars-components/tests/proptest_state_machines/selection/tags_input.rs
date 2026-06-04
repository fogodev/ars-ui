use std::collections::BTreeSet;

use ars_components::selection::tags_input;
use ars_core::{Env, HtmlAttr, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

fn arb_tag_value() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("alpha".to_string()),
        Just("beta".to_string()),
        Just("gamma".to_string()),
        Just("alpha,beta".to_string()),
        Just("  spaced  ".to_string()),
        Just(String::new()),
    ]
}

fn arb_tags_input_event() -> impl Strategy<Value = tags_input::Event> {
    prop_oneof![
        arb_tag_value().prop_map(tags_input::Event::AddTag),
        arb_tag_value().prop_map(tags_input::Event::RemoveTag),
        (0usize..6).prop_map(tags_input::Event::RemoveTagAtIndex),
        (0usize..6).prop_map(|index| tags_input::Event::EditTag { index }),
        (0usize..6, arb_tag_value())
            .prop_map(|(index, value)| tags_input::Event::CommitEdit { index, value }),
        Just(tags_input::Event::CancelEdit),
        Just(tags_input::Event::Focus { is_keyboard: true }),
        Just(tags_input::Event::Blur),
        arb_tag_value().prop_map(tags_input::Event::InputChange),
        arb_tag_value().prop_map(tags_input::Event::Paste),
        Just(tags_input::Event::ClearAll),
        Just(tags_input::Event::FocusPrevTag),
        Just(tags_input::Event::FocusNextTag),
        Just(tags_input::Event::DeselectTags),
        Just(tags_input::Event::CompositionStart),
        Just(tags_input::Event::CompositionEnd),
        prop::option::of(prop::collection::vec(arb_tag_value(), 0..5))
            .prop_map(tags_input::Event::SetValue),
        Just(tags_input::Event::SetProps),
    ]
}

fn assert_tags_input_invariants(service: &Service<tags_input::Machine>) -> TestCaseResult {
    let ctx = service.context();

    let tags = ctx.value.get();

    // Identity is stable across every transition.
    prop_assert_eq!(ctx.ids.id(), "tags");

    // The max-tags cap is never exceeded.
    if let Some(max) = ctx.max {
        prop_assert!(tags.len() <= max);
    }

    // Duplicates never appear unless explicitly allowed.
    if !ctx.allow_duplicates {
        let mut seen = BTreeSet::new();

        for tag in tags {
            prop_assert!(
                seen.insert(tag.clone()),
                "unexpected duplicate tag: {}",
                tag
            );
        }
    }

    // Focus/edit indices always point at a real tag.
    if let Some(index) = ctx.focused_tag {
        prop_assert!(index < tags.len());
    }

    if let Some(index) = ctx.editing_tag {
        prop_assert!(index < tags.len());
    }

    // The hidden form value is exactly the joined tag list.
    let hidden = service.connect(&|_| {}).hidden_input_attrs();

    let joined = tags.join(ctx.delimiter.as_str());

    prop_assert_eq!(hidden.get(&HtmlAttr::Value), Some(joined.as_str()));

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore]
    fn tags_input_preserves_invariants(
        max in prop::option::of(1usize..5),
        editable in any::<bool>(),
        rtl in any::<bool>(),
        events in prop::collection::vec(arb_tags_input_event(), 0..48),
    ) {
        let dir = if rtl { ars_core::Direction::Rtl } else { ars_core::Direction::Ltr };

        let mut props = tags_input::Props::new().id("tags").editable(editable).dir(dir);

        if let Some(max) = max {
            props = props.max(max);
        }

        let mut service = Service::<tags_input::Machine>::new(
            props,
            &Env::default(),
            &tags_input::Messages::default(),
        );

        assert_tags_input_invariants(&service)?;

        for event in events {
            drop(service.send(event));

            assert_tags_input_invariants(&service)?;
        }
    }
}
