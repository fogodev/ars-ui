use ars_collections::Key;
use ars_components::input::radio_group;
use ars_core::{Direction, Env, Orientation, Service};
use proptest::prelude::*;

/// Bounded key pool the strategies draw from, so navigation and selection
/// sequences revisit the same items repeatedly.
const KEY_POOL: u64 = 5;

fn arb_key() -> impl Strategy<Value = Key> {
    (0_u64..KEY_POOL).prop_map(Key::Int)
}

fn arb_radio() -> impl Strategy<Value = radio_group::Radio> {
    (arb_key(), any::<bool>()).prop_map(|(value, disabled)| radio_group::Radio { value, disabled })
}

/// A unique-by-key set of radio items rendered into the registry.
fn arb_items() -> impl Strategy<Value = Vec<radio_group::Radio>> {
    prop::collection::btree_set(arb_key(), 0..=KEY_POOL as usize)
        .prop_flat_map(|keys| {
            let count = keys.len();
            (Just(keys), prop::collection::vec(any::<bool>(), count))
        })
        .prop_map(|(keys, disabled_flags)| {
            keys.into_iter()
                .zip(disabled_flags)
                .map(|(value, disabled)| radio_group::Radio { value, disabled })
                .collect()
        })
}

fn arb_radio_group_props() -> impl Strategy<Value = radio_group::Props> {
    (
        prop::option::of(arb_key()),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(default_value, disabled, readonly, required, invalid, loop_focus)| {
                radio_group::Props {
                    id: "radio-group".to_string(),
                    value: None,
                    default_value,
                    disabled,
                    readonly,
                    required,
                    invalid,
                    orientation: Orientation::Vertical,
                    dir: Direction::Ltr,
                    name: Some("group".to_string()),
                    form: Some("form".to_string()),
                    loop_focus,
                    on_value_change: None,
                }
            },
        )
}

fn arb_radio_group_event() -> impl Strategy<Value = radio_group::Event> {
    prop_oneof![
        arb_key().prop_map(radio_group::Event::SelectValue),
        (arb_key(), any::<bool>())
            .prop_map(|(item, is_keyboard)| radio_group::Event::FocusItem { item, is_keyboard }),
        Just(radio_group::Event::FocusNext),
        Just(radio_group::Event::FocusPrev),
        Just(radio_group::Event::FocusFirst),
        Just(radio_group::Event::FocusLast),
        Just(radio_group::Event::Blur),
        arb_radio().prop_map(radio_group::Event::RegisterItem),
        arb_key().prop_map(radio_group::Event::UnregisterItem),
        Just(radio_group::Event::Reset),
        Just(radio_group::Event::SetProps),
        any::<bool>().prop_map(radio_group::Event::SetHasDescription),
        any::<bool>().prop_map(radio_group::Event::SetHasErrorMessage),
    ]
}

/// Whether a key is disabled from the group's point of view: either the whole
/// group is disabled or the registered item carries its own disabled flag.
fn item_disabled(ctx: &radio_group::Context, key: &Key) -> bool {
    ctx.disabled
        || ctx
            .items
            .iter()
            .any(|item| &item.value == key && item.disabled)
}

fn is_registered(ctx: &radio_group::Context, key: &Key) -> bool {
    ctx.items.iter().any(|item| &item.value == key)
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_radio_group_event_sequences_preserve_invariants(
        props in arb_radio_group_props(),
        items in arb_items(),
        events in prop::collection::vec(arb_radio_group_event(), 0..128),
    ) {
        let mut service = Service::<radio_group::Machine>::new(
            props,
            &Env::default(),
            &radio_group::Messages,
        );

        // Seed the registry so roving focus has items to traverse.
        drop(service.send(radio_group::Event::SetItems(items)));

        for event in events {
            let ctx = service.context();

            let prev_value = ctx.value.get().clone();

            // Capture pre-send predicates for the SelectValue assertion: the
            // registry does not change on SelectValue, so these stay valid.
            let select_target = match &event {
                radio_group::Event::SelectValue(key) => Some(key.clone()),
                _ => None,
            };

            let group_blocked = ctx.disabled || ctx.readonly;

            let target_disabled = select_target
                .as_ref()
                .is_some_and(|key| item_disabled(ctx, key));

            drop(service.send(event));

            let ctx = service.context();

            // State and `focused_item` stay consistent.
            match service.state() {
                radio_group::State::Idle => {
                    prop_assert_eq!(ctx.focused_item.as_ref(), None);
                }

                radio_group::State::Focused { item } => {
                    prop_assert_eq!(ctx.focused_item.as_ref(), Some(item));
                }
            }

            // Keyboard focus-visibility cannot outlive a focused item.
            prop_assert!(!ctx.focus_visible || ctx.focused_item.is_some());

            // Focus only ever rests on a registered, enabled item.
            if let Some(focused) = ctx.focused_item.clone() {
                prop_assert!(is_registered(ctx, &focused));
                prop_assert!(!item_disabled(ctx, &focused));
            }

            if let Some(key) = select_target {
                if group_blocked {
                    // Disabled/read-only groups never change the selection.
                    prop_assert_eq!(ctx.value.get(), &prev_value);
                } else if !target_disabled {
                    // Selecting an enabled value replaces the prior selection
                    // (and is idempotent when it was already selected).
                    prop_assert_eq!(ctx.value.get().as_ref(), Some(&key));
                } else {
                    // Selecting a disabled item is rejected.
                    prop_assert_eq!(ctx.value.get(), &prev_value);
                }
            }
        }
    }
}
