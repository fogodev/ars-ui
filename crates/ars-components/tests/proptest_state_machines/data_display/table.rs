use std::collections::BTreeSet;

use ars_collections::{Key, selection};
use ars_components::data_display::table;
use ars_core::{Direction, Env, Service};
use proptest::prelude::*;

fn arb_key() -> impl Strategy<Value = Key> {
    (0u64..8).prop_map(Key::int)
}

fn arb_props() -> impl Strategy<Value = table::Props> {
    let modes = prop_oneof![
        Just(selection::Mode::None),
        Just(selection::Mode::Single),
        Just(selection::Mode::Multiple),
    ];

    let behaviors = prop_oneof![
        Just(selection::Behavior::Toggle),
        Just(selection::Behavior::Replace),
    ];

    let dir = prop_oneof![Just(Direction::Ltr), Just(Direction::Rtl)];

    (
        modes,
        behaviors,
        prop::collection::btree_set(arb_key(), 0..4),
        any::<bool>(),
        any::<bool>(),
        dir,
    )
        .prop_map(
            |(mode, behavior, disabled, disallow_empty, interactive, dir)| table::Props {
                id: "table".to_string(),
                selection_mode: mode,
                selection_behavior: behavior,
                disabled_keys: disabled,
                disallow_empty_selection: disallow_empty,
                interactive,
                dir,
                min_column_width: 50.0,
                column_resize_step: 10.0,
                ..table::Props::default()
            },
        )
}

fn arb_event() -> impl Strategy<Value = table::Event> {
    prop_oneof![
        arb_key().prop_map(table::Event::SelectRow),
        arb_key().prop_map(table::Event::DeselectRow),
        arb_key().prop_map(table::Event::ToggleRow),
        Just(table::Event::SelectAll),
        Just(table::Event::DeselectAll),
        arb_key().prop_map(table::Event::ExpandRow),
        arb_key().prop_map(table::Event::CollapseRow),
        arb_key().prop_map(table::Event::RowAction),
        arb_key().prop_map(table::Event::FocusRow),
        (arb_key(), 0usize..6).prop_map(|(row, col)| table::Event::FocusCell {
            row,
            col,
            row_index: 0,
        }),
        (0usize..6, 0usize..6).prop_map(|(c, r)| table::Event::Focus { cell: (c, r) }),
        Just(table::Event::Blur),
        Just(table::Event::EscapeKey),
        ("[a-c]", -50.0f64..400.0)
            .prop_map(|(column, width)| table::Event::ColumnResize { column, width }),
        "[a-c]".prop_map(|column| table::Event::ColumnResizeEnd { column }),
        any::<bool>().prop_map(table::Event::SetLoading),
    ]
}

fn no_disabled_keys_in_selection(set: &selection::Set, disabled: &BTreeSet<Key>) -> bool {
    match set {
        selection::Set::Single(key) => !disabled.contains(key),
        selection::Set::Multiple(keys) => keys.is_disjoint(disabled),
        // `Empty` and `All` (plus any future non-exhaustive variants)
        // cannot independently carry a disabled key.
        _ => true,
    }
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    /// Core invariants preserved across any event sequence:
    ///   * `selected_rows` never contains a disabled key.
    ///   * `expanded_rows` never contains a disabled key.
    ///   * `column_widths` are all clamped to `min_column_width`.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_table_event_sequences_preserve_invariants(
        props in arb_props(),
        events in prop::collection::vec(arb_event(), 0..64),
    ) {
        let mut service = Service::<table::Machine>::new(
            props,
            &Env::default(),
            &table::Messages::default(),
        );

        // Register a known row set so SetRows pruning isn't a side
        // channel for selection invariants.
        drop(service.send(table::Event::SetRows(vec![
            Key::int(0),
            Key::int(1),
            Key::int(2),
            Key::int(3),
        ])));

        for event in events {
            drop(service.send(event));

            let ctx = service.context();

            prop_assert!(
                no_disabled_keys_in_selection(ctx.selected_rows.get(), &ctx.disabled_keys),
                "selection contained disabled key: {:?} (disabled={:?})",
                ctx.selected_rows.get(),
                ctx.disabled_keys,
            );

            prop_assert!(
                ctx.expanded_rows.get().is_disjoint(&ctx.disabled_keys),
                "expansion contained disabled key: {:?} (disabled={:?})",
                ctx.expanded_rows.get(),
                ctx.disabled_keys,
            );

            for (column, width) in &ctx.column_widths {
                prop_assert!(
                    *width >= ctx.min_column_width,
                    "column {column:?} width {width} below min {}",
                    ctx.min_column_width,
                );
            }
        }
    }
}
