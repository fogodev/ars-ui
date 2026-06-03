use std::collections::BTreeSet;

use ars_collections::{Collection, Key, StaticCollection, selection};
use ars_components::data_display::tag_group;
use ars_core::{Env, Service};
use proptest::prelude::*;

const fn key(value: u64) -> Key {
    Key::int(value)
}

fn props(disabled: bool) -> tag_group::Props {
    let items = StaticCollection::new([
        (
            key(0),
            "Zero".to_string(),
            tag_group::Tag {
                key: key(0),
                label: "Zero".to_string(),
                disabled: false,
            },
        ),
        (
            key(1),
            "One".to_string(),
            tag_group::Tag {
                key: key(1),
                label: "One".to_string(),
                disabled: true,
            },
        ),
        (
            key(2),
            "Two".to_string(),
            tag_group::Tag {
                key: key(2),
                label: "Two".to_string(),
                disabled: false,
            },
        ),
    ]);

    tag_group::Props::new()
        .id("tags")
        .items(items)
        .selection_mode(selection::Mode::Multiple)
        .disabled(disabled)
}

fn arb_event() -> impl Strategy<Value = tag_group::Event> {
    prop_oneof![
        (prop::option::of(0u64..3), any::<bool>()).prop_map(|(item, is_keyboard)| {
            tag_group::Event::Focus {
                item: item.map(key),
                is_keyboard,
            }
        }),
        Just(tag_group::Event::Blur),
        (0u64..3).prop_map(|value| tag_group::Event::RemoveTag(key(value))),
        Just(tag_group::Event::FocusNext),
        Just(tag_group::Event::FocusPrevious),
        Just(tag_group::Event::FocusFirst),
        Just(tag_group::Event::FocusLast),
        (0u64..3).prop_map(|value| tag_group::Event::ToggleTag(key(value))),
        (0u64..3).prop_map(|value| tag_group::Event::SelectTag(key(value))),
        (0u64..3).prop_map(|value| tag_group::Event::DeselectTag(key(value))),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_tag_group_event_sequences_preserve_invariants(
        disabled in any::<bool>(),
        events in prop::collection::vec(arb_event(), 0..64),
    ) {
        let mut service = Service::<tag_group::Machine>::new(
            props(disabled),
            &Env::default(),
            &tag_group::Messages::default(),
        );

        let initial_items = service.context().items.keys().cloned().collect::<BTreeSet<_>>();
        let initial_selection = service.context().selected_keys.get().clone();

        for event in events {
            drop(service.send(event));

            let ctx = service.context();

            let item_keys = ctx.items.keys().cloned().collect::<BTreeSet<_>>();

            if let Some(focused) = &ctx.focused_key {
                prop_assert!(item_keys.contains(focused));

                let item = ctx.items.get(focused).expect("focused key is present");

                prop_assert!(!item.value.as_ref().expect("tag value").disabled);
            }

            for key in ctx.selected_keys.get() {
                prop_assert!(item_keys.contains(key));
            }

            if disabled {
                prop_assert_eq!(&item_keys, &initial_items);
                prop_assert_eq!(ctx.selected_keys.get(), &initial_selection);
            }
        }
    }
}
