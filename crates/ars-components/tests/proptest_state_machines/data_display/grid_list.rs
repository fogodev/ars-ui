use std::{collections::BTreeSet, num::NonZeroUsize, time::Duration};

use ars_collections::{Collection, Key, StaticCollection, selection};
use ars_components::data_display::grid_list;
use ars_core::{Env, Service};
use proptest::prelude::*;

const fn key(value: u64) -> Key {
    Key::int(value)
}

fn item(value: u64, disabled: bool) -> grid_list::ItemDef {
    grid_list::ItemDef {
        key: key(value),
        label: format!("Item {value}"),
        disabled,
        href: None,
    }
}

fn items() -> StaticCollection<grid_list::ItemDef> {
    StaticCollection::new([
        (key(0), "Item 0".to_string(), item(0, false)),
        (key(1), "Item 1".to_string(), item(1, true)),
        (key(2), "Item 2".to_string(), item(2, false)),
        (key(3), "Item 3".to_string(), item(3, false)),
        (key(4), "Item 4".to_string(), item(4, false)),
    ])
}

fn props(disabled: bool, mode: selection::Mode) -> grid_list::Props {
    grid_list::Props::new()
        .id("grid")
        .items(items())
        .columns(NonZeroUsize::new(2).expect("non-zero columns"))
        .selection_mode(mode)
        .disabled(disabled)
}

fn arb_mode() -> impl Strategy<Value = selection::Mode> {
    prop_oneof![
        Just(selection::Mode::None),
        Just(selection::Mode::Single),
        Just(selection::Mode::Multiple),
    ]
}

fn arb_event() -> impl Strategy<Value = grid_list::Event> {
    prop_oneof![
        (prop::option::of(0u64..6), any::<bool>()).prop_map(|(item, is_keyboard)| {
            grid_list::Event::Focus {
                key: item.map(key),
                is_keyboard,
            }
        }),
        Just(grid_list::Event::Blur),
        (0u64..6).prop_map(|value| grid_list::Event::Select(key(value))),
        (0u64..6).prop_map(|value| grid_list::Event::ToggleSelect(key(value))),
        (0u64..6, 0u64..6).prop_map(|(from, to)| grid_list::Event::SelectRange {
            from: key(from),
            to: key(to),
        }),
        Just(grid_list::Event::FocusUp),
        Just(grid_list::Event::FocusDown),
        Just(grid_list::Event::FocusLeft),
        Just(grid_list::Event::FocusRight),
        Just(grid_list::Event::FocusFirst),
        Just(grid_list::Event::FocusLast),
        Just(grid_list::Event::SelectAll),
        Just(grid_list::Event::ClearSelection),
        (0u64..6).prop_map(|value| grid_list::Event::ItemAction(key(value))),
        (b'a'..=b'z', 0u64..10_000).prop_map(|(ch, now)| {
            grid_list::Event::TypeaheadSearch {
                ch: char::from(ch),
                now: Duration::from_millis(now),
            }
        }),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_grid_list_event_sequences_preserve_invariants(
        disabled in any::<bool>(),
        mode in arb_mode(),
        events in prop::collection::vec(arb_event(), 0..64),
    ) {
        let mut service = Service::<grid_list::Machine>::new(
            props(disabled, mode),
            &Env::default(),
            &grid_list::Messages::default(),
        );

        let initial_selection = service.context().selected_keys.get().clone();
        let initial_focus = service.context().focused_key.clone();

        for event in events {
            drop(service.send(event));

            let ctx = service.context();

            let item_keys = ctx.items.keys().cloned().collect::<BTreeSet<_>>();

            if let Some(focused) = &ctx.focused_key {
                prop_assert!(item_keys.contains(focused));
                prop_assert!(!ctx.disabled_keys.contains(focused));
            }

            for selected in ctx.selected_keys.get() {
                prop_assert!(item_keys.contains(selected));
                prop_assert!(!ctx.disabled_keys.contains(selected));
            }

            match ctx.selection_mode {
                selection::Mode::None => prop_assert!(ctx.selected_keys.get().is_empty()),
                selection::Mode::Single => prop_assert!(ctx.selected_keys.get().len() <= 1),
                selection::Mode::Multiple => {}
            }

            if disabled {
                prop_assert_eq!(ctx.selected_keys.get(), &initial_selection);
                prop_assert_eq!(&ctx.focused_key, &initial_focus);
            }
        }
    }
}
