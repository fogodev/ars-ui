use std::collections::BTreeSet;

use ars_collections::Key;
use ars_components::input::{checkbox, checkbox_group};
use ars_core::{Direction, Env, Orientation, Service};
use proptest::prelude::*;

/// Size of the bounded key pool the strategies draw from. Keeping the universe
/// small makes membership invariants meaningful and lets sequences revisit the
/// same keys often.
const KEY_POOL: u64 = 6;

fn arb_key() -> impl Strategy<Value = Key> {
    (0_u64..KEY_POOL).prop_map(Key::Int)
}

fn arb_key_set() -> impl Strategy<Value = BTreeSet<Key>> {
    prop::collection::btree_set(arb_key(), 0..=KEY_POOL as usize)
}

fn arb_checkbox_group_props() -> impl Strategy<Value = checkbox_group::Props> {
    (
        arb_key_set(),
        arb_key_set(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        prop::option::of(0_usize..=KEY_POOL as usize),
    )
        .prop_map(
            |(default_value, all_values, disabled, readonly, required, invalid, max_checked)| {
                checkbox_group::Props {
                    id: "checkbox-group".to_string(),
                    value: None,
                    default_value,
                    name: Some("group".to_string()),
                    form: Some("form".to_string()),
                    disabled,
                    required,
                    readonly,
                    invalid,
                    dir: Direction::Ltr,
                    orientation: Orientation::Vertical,
                    all_values,
                    max_checked,
                    on_change: None,
                }
            },
        )
}

fn arb_checkbox_group_event() -> impl Strategy<Value = checkbox_group::Event> {
    prop_oneof![
        arb_key().prop_map(checkbox_group::Event::Toggle),
        arb_key().prop_map(checkbox_group::Event::Check),
        arb_key().prop_map(checkbox_group::Event::Uncheck),
        arb_key_set().prop_map(checkbox_group::Event::SetValue),
        Just(checkbox_group::Event::CheckAll),
        Just(checkbox_group::Event::UncheckAll),
        Just(checkbox_group::Event::Reset),
        any::<bool>().prop_map(|is_keyboard| checkbox_group::Event::Focus { is_keyboard }),
        Just(checkbox_group::Event::Blur),
        Just(checkbox_group::Event::SetProps),
        any::<bool>().prop_map(checkbox_group::Event::SetHasDescription),
        any::<bool>().prop_map(checkbox_group::Event::SetHasErrorMessage),
    ]
}

/// Mirrors `clamp_to_max` in the machine: keep the first `max` keys in `Ord`
/// order, dropping the rest.
fn clamp_to_max(values: BTreeSet<Key>, max: Option<usize>) -> BTreeSet<Key> {
    match max {
        Some(max) => values.into_iter().take(max).collect(),
        None => values,
    }
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_checkbox_group_event_sequences_preserve_invariants(
        props in arb_checkbox_group_props(),
        events in prop::collection::vec(arb_checkbox_group_event(), 0..128),
    ) {
        let max = props.max_checked;

        let mut service = Service::<checkbox_group::Machine>::new(
            props.clone(),
            &Env::default(),
            &checkbox_group::Messages,
        );

        // The uncontrolled set starts as the clamped default.
        let mut expected = clamp_to_max(props.default_value.clone(), max);

        prop_assert_eq!(service.context().value.get(), &expected);

        for event in events {
            // `disabled`/`readonly` reject Toggle/Check/Uncheck/CheckAll/UncheckAll
            // but NOT SetValue or Reset (the machine guard omits them).
            let blocked = props.disabled || props.readonly;

            match &event {
                checkbox_group::Event::Toggle(key) => {
                    if !blocked {
                        if expected.contains(key) {
                            expected.remove(key);
                        } else if max.is_none_or(|max| expected.len() < max) {
                            expected.insert(key.clone());
                        }
                    }
                }

                checkbox_group::Event::Check(key) => {
                    if !blocked
                        && !expected.contains(key)
                        && max.is_none_or(|max| expected.len() < max)
                    {
                        expected.insert(key.clone());
                    }
                }

                checkbox_group::Event::Uncheck(key) => {
                    if !blocked {
                        expected.remove(key);
                    }
                }

                checkbox_group::Event::CheckAll => {
                    if !blocked {
                        expected = clamp_to_max(props.all_values.clone(), max);
                    }
                }

                checkbox_group::Event::UncheckAll => {
                    if !blocked {
                        expected.clear();
                    }
                }

                checkbox_group::Event::SetValue(value) => {
                    expected = clamp_to_max(value.clone(), max);
                }

                checkbox_group::Event::Reset => {
                    expected = clamp_to_max(props.default_value.clone(), max);
                }

                checkbox_group::Event::Focus { .. }
                | checkbox_group::Event::Blur
                | checkbox_group::Event::SetProps
                | checkbox_group::Event::SetHasDescription(_)
                | checkbox_group::Event::SetHasErrorMessage(_) => {}
            }

            drop(service.send(event));

            let ctx = service.context();

            // The machine's checked set tracks the model exactly.
            prop_assert_eq!(ctx.value.get(), &expected);

            // The checked set never exceeds `max_checked`.
            if let Some(max) = max {
                prop_assert!(ctx.value.get().len() <= max);
            }

            // Parent (select-all) state derives from the children deterministically.
            let parent = ctx.parent_checked_state(&props.all_values);

            let checked = props
                .all_values
                .iter()
                .filter(|key| ctx.value.get().contains(*key))
                .count();

            let derived = if props.all_values.is_empty() || checked == 0 {
                checkbox::State::Unchecked
            } else if checked == props.all_values.len() {
                checkbox::State::Checked
            } else {
                checkbox::State::Indeterminate
            };

            prop_assert_eq!(parent, derived);
        }
    }
}
