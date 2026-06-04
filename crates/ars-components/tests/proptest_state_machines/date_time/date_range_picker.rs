use ars_components::date_time::date_range_picker;
use ars_core::{ComponentPart, ConnectApi, Env, KeyboardKey, Service};
use ars_i18n::DateRange;
use proptest::prelude::*;

use super::helpers::{arb_date, date};

#[derive(Clone, Debug)]
enum DateRangePickerAction {
    Send(date_range_picker::Event),
    SetControlledValue(Option<DateRange>),
    SetDisabled(bool),
}

fn date_range_picker_presets() -> Vec<date_range_picker::Preset> {
    vec![
        date_range_picker::Preset::new(
            "Last 7 days",
            DateRange::new(date(2025, 5, 26), date(2025, 6, 1)).expect("ordered range"),
        ),
        date_range_picker::Preset::new(
            "Last 30 days",
            DateRange::new(date(2025, 5, 3), date(2025, 6, 1)).expect("ordered range"),
        ),
    ]
}

fn date_range_picker_props() -> date_range_picker::Props {
    date_range_picker::Props {
        id: "range-picker".to_string(),
        presets: date_range_picker_presets(),
        ..date_range_picker::Props::default()
    }
}

fn arb_date_range_picker_event() -> impl Strategy<Value = date_range_picker::Event> {
    prop_oneof![
        Just(date_range_picker::Event::Open),
        Just(date_range_picker::Event::Close),
        Just(date_range_picker::Event::Toggle),
        (arb_date(), arb_date())
            .prop_map(|(first, second)| {
                DateRange::normalized(first, second).expect("comparable generated dates")
            })
            .prop_map(|range| date_range_picker::Event::SelectRangeComplete { range }),
        proptest::option::of(arb_date()).prop_map(date_range_picker::Event::StartValueChange),
        proptest::option::of(arb_date()).prop_map(date_range_picker::Event::EndValueChange),
        (0usize..=3).prop_map(|index| date_range_picker::Event::SelectPreset { index }),
        Just(date_range_picker::Event::Clear),
        Just(date_range_picker::Event::FocusIn),
        Just(date_range_picker::Event::FocusOut),
        Just(date_range_picker::Event::KeyDown {
            key: KeyboardKey::Escape,
        }),
        Just(date_range_picker::Event::KeyDown {
            key: KeyboardKey::ArrowDown,
        }),
    ]
}

fn arb_date_range_picker_action() -> impl Strategy<Value = DateRangePickerAction> {
    prop_oneof![
        arb_date_range_picker_event().prop_map(DateRangePickerAction::Send),
        prop_oneof![
            Just(None),
            (arb_date(), arb_date())
                .prop_map(|(first, second)| DateRange::normalized(first, second))
        ]
        .prop_map(DateRangePickerAction::SetControlledValue),
        any::<bool>().prop_map(DateRangePickerAction::SetDisabled),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    /// Random event and controlled-prop sequences keep the picker's invariants:
    /// the ID stays stable, the state is one of its two variants, the canonical
    /// range stays normalized and consistent with the per-field values, and
    /// connecting every anatomy part never panics.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_date_range_picker_sequences_preserve_invariants(
        actions in prop::collection::vec(arb_date_range_picker_action(), 0..128),
    ) {
        let mut service = Service::<date_range_picker::Machine>::new(
            date_range_picker_props(),
            &Env::default(),
            &date_range_picker::Messages::default(),
        );

        for action in actions {
            match action {
                DateRangePickerAction::Send(event) => {
                    drop(service.send(event));
                }

                DateRangePickerAction::SetControlledValue(value) => {
                    drop(service.set_props(date_range_picker::Props {
                        value: Some(value),
                        ..date_range_picker_props()
                    }));
                }

                DateRangePickerAction::SetDisabled(disabled) => {
                    drop(service.set_props(date_range_picker::Props {
                        disabled,
                        ..date_range_picker_props()
                    }));
                }
            }

            let ctx = service.context();

            prop_assert_eq!(ctx.ids.id(), "range-picker");
            prop_assert!(matches!(
                service.state(),
                date_range_picker::State::Open | date_range_picker::State::Closed,
            ));

            // A selected range is always normalized so `start <= end` (in both
            // modes). In *uncontrolled* mode the canonical range additionally
            // mirrors the per-field values — it is `Some` exactly when both
            // fields are set. In controlled mode `get()` returns the parent's
            // override, which is intentionally decoupled from local field edits
            // (the parent reconciles via the next `SyncProps`), so the mirror
            // check does not apply.
            if let Some(range) = ctx.value.get() {
                prop_assert!(matches!(
                    range.start.compare_within_calendar(&range.end),
                    Some(core::cmp::Ordering::Less | core::cmp::Ordering::Equal)
                ));

                if !ctx.value.is_controlled() {
                    prop_assert!(ctx.start_date.is_some() && ctx.end_date.is_some());
                    prop_assert_eq!(ctx.start_date.as_ref(), Some(&range.start));
                    prop_assert_eq!(ctx.end_date.as_ref(), Some(&range.end));
                }
            } else if !ctx.value.is_controlled() {
                prop_assert!(ctx.start_date.is_none() || ctx.end_date.is_none());
            }

            let send = |_event: date_range_picker::Event| {};

            let api = service.connect(&send);

            for part in date_range_picker::Part::all() {
                drop(api.part_attrs(part));
            }
        }
    }
}
