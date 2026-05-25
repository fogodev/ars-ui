// ────────────────────────────────────────────────────────────────────
// Avatar
// ────────────────────────────────────────────────────────────────────

mod avatar_proptests {
    use core::time::Duration;

    use ars_components::data_display::avatar;
    use ars_core::{Env, SafeUrl, Service};
    use proptest::prelude::*;

    fn arb_avatar_props() -> impl Strategy<Value = avatar::Props> {
        (
            prop::option::of(prop_oneof![
                Just(avatar::ImageSrc::from_safe_url(&SafeUrl::from_static(
                    "/avatar.png"
                ))),
                Just(avatar::ImageSrc::from_safe_url(&SafeUrl::from_static(
                    "https://example.com/avatar.png"
                ))),
            ]),
            prop::option::of(".*"),
            (0u64..=1_000).prop_map(Duration::from_millis),
        )
            .prop_map(|(src, name, fallback_delay)| {
                let mut props = avatar::Props::new()
                    .id("avatar")
                    .fallback_delay(fallback_delay);

                props.src = src;
                props.name = name;

                props
            })
    }

    fn arb_avatar_event() -> impl Strategy<Value = avatar::Event> {
        prop_oneof![
            Just(avatar::Event::ImageLoad),
            Just(avatar::Event::ImageError),
            Just(avatar::Event::FallbackDelayElapsed),
            Just(avatar::Event::SetSrc(None)),
            Just(avatar::Event::SetSrc(Some(
                avatar::ImageSrc::from_safe_url(&SafeUrl::from_static("/next.png"))
            ))),
        ]
    }

    proptest! {
        #![proptest_config(super::super::common::proptest_config())]

        #[test]
        #[ignore = "proptest — nightly extended-proptest job"]
        fn proptest_avatar_event_sequences_preserve_invariants(
            props in arb_avatar_props(),
            events in prop::collection::vec(arb_avatar_event(), 0..64),
        ) {
            let mut service = Service::<avatar::Machine>::new(
                props,
                &Env::default(),
                &avatar::Messages::default(),
            );

            for event in events {
                drop(service.send(event));

                let api = service.connect(&|_| {});

                match service.state() {
                    avatar::State::Loading => {
                        prop_assert_eq!(service.context().loading_status, avatar::LoadingStatus::Loading);
                        prop_assert_eq!(api.is_image_visible(), false);
                        prop_assert_eq!(api.is_fallback_visible(), service.context().fallback_visible);
                    }

                    avatar::State::Loaded => {
                        prop_assert_eq!(service.context().loading_status, avatar::LoadingStatus::Loaded);
                        prop_assert!(api.is_image_visible());
                        prop_assert!(!api.is_fallback_visible());
                    }

                    avatar::State::Error => {
                        prop_assert_eq!(service.context().loading_status, avatar::LoadingStatus::Error);
                        prop_assert_eq!(api.is_image_visible(), false);
                        prop_assert!(api.is_fallback_visible());
                    }

                    avatar::State::Fallback => {
                        prop_assert_eq!(service.context().src.as_ref().map(avatar::ImageSrc::as_str), None);
                        prop_assert_eq!(service.context().loading_status, avatar::LoadingStatus::Error);
                        prop_assert_eq!(api.is_image_visible(), false);
                        prop_assert!(api.is_fallback_visible());
                    }
                }
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Table
// ────────────────────────────────────────────────────────────────────

mod table_proptests {
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
        #![proptest_config(super::super::common::proptest_config())]

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
}

// ────────────────────────────────────────────────────────────────────
// Progress
// ────────────────────────────────────────────────────────────────────

mod progress_proptests {
    use ars_components::data_display::progress;
    use ars_core::{AriaAttr, Env, HtmlAttr, Service};
    use proptest::prelude::*;

    fn arb_props() -> impl Strategy<Value = progress::Props> {
        (
            prop::option::of(prop::option::of(-100.0f64..=200.0)),
            prop::option::of(-100.0f64..=200.0),
            -100.0f64..=50.0,
            51.0f64..=250.0,
            prop_oneof![
                Just(progress::Orientation::Horizontal),
                Just(progress::Orientation::Vertical),
            ],
        )
            .prop_map(
                |(value, default_value, min, max, orientation)| progress::Props {
                    id: "progress".to_string(),
                    value,
                    default_value,
                    min,
                    max,
                    orientation,
                    format_options: None,
                },
            )
    }

    fn arb_event() -> impl Strategy<Value = progress::Event> {
        prop_oneof![
            prop::option::of(-100.0f64..=250.0).prop_map(progress::Event::SetValue),
            (51.0f64..=300.0f64).prop_map(progress::Event::SetMax),
            Just(progress::Event::Complete),
            Just(progress::Event::Reset),
        ]
    }

    proptest! {
        #![proptest_config(super::super::common::proptest_config())]

        #[test]
        #[ignore = "proptest — nightly extended-proptest job"]
        fn proptest_progress_event_sequences_preserve_invariants(
            props in arb_props(),
            events in prop::collection::vec(arb_event(), 0..64),
        ) {
            let mut service = Service::<progress::Machine>::new(
                props,
                &Env::default(),
                &progress::Messages::default(),
            );

            for event in events {
                drop(service.send(event));

                let ctx = service.context();

                prop_assert!((0.0..=100.0).contains(&ctx.percent));
                prop_assert_eq!(ctx.indeterminate, ctx.value.get().is_none());

                match ctx.value.get() {
                    None => {
                        prop_assert!(
                            matches!(service.state(), progress::State::Loading | progress::State::Idle),
                            "indeterminate value must be either active loading or reset idle"
                        );

                        let attrs = service.connect(&|_| {}).root_attrs();

                        prop_assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::ValueNow)));
                    }

                    Some(value) if *value >= ctx.max => {
                        prop_assert_eq!(service.state(), &progress::State::Complete);
                        prop_assert_eq!(ctx.percent, 100.0);
                    }

                    Some(_) => {
                        prop_assert_eq!(service.state(), &progress::State::Idle);
                    }
                }
            }
        }
    }
}
