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
                            matches!(service.state(), progress::State::Loading),
                            "indeterminate value must report loading state"
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

// ────────────────────────────────────────────────────────────────────
// Marquee
// ────────────────────────────────────────────────────────────────────

mod marquee_proptests {
    use ars_components::data_display::marquee;
    use ars_core::{Env, Service};
    use proptest::prelude::*;

    fn arb_props() -> impl Strategy<Value = marquee::Props> {
        (
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            prop::option::of(1usize..=5),
            prop_oneof![
                Just(marquee::Direction::Left),
                Just(marquee::Direction::Right),
                Just(marquee::Direction::Up),
                Just(marquee::Direction::Down),
            ],
        )
            .prop_map(
                |(auto_play, disabled, pause_on_hover, pause_on_focus, loop_count, direction)| {
                    marquee::Props::new()
                        .id("marquee")
                        .auto_play(auto_play)
                        .disabled(disabled)
                        .pause_on_hover(pause_on_hover)
                        .pause_on_focus(pause_on_focus)
                        .loop_count_option(loop_count)
                        .direction(direction)
                },
            )
    }

    fn arb_event() -> impl Strategy<Value = marquee::Event> {
        prop_oneof![
            Just(marquee::Event::Play),
            Just(marquee::Event::Pause),
            Just(marquee::Event::HoverIn),
            Just(marquee::Event::HoverOut),
            Just(marquee::Event::FocusIn),
            Just(marquee::Event::FocusOut),
            Just(marquee::Event::LoopComplete),
            Just(marquee::Event::SyncProps),
        ]
    }

    proptest! {
        #![proptest_config(super::super::common::proptest_config())]

        #[test]
        #[ignore = "proptest — nightly extended-proptest job"]
        fn proptest_marquee_event_sequences_preserve_invariants(
            props in arb_props(),
            events in prop::collection::vec(arb_event(), 0..64),
        ) {
            let disabled = props.disabled;
            let loop_count = props.loop_count;

            let mut service = Service::<marquee::Machine>::new(
                props,
                &Env::default(),
                &marquee::Messages::default(),
            );

            for event in events {
                drop(service.send(event));

                let ctx = service.context();

                if *service.state() == marquee::State::Playing {
                    prop_assert!(!ctx.paused_by_hover);
                    prop_assert!(!ctx.paused_by_focus);
                }

                if let Some(max) = loop_count {
                    prop_assert!(ctx.current_loop <= max);

                    if ctx.current_loop >= max {
                        prop_assert_eq!(service.state(), &marquee::State::Paused);
                    }
                }

                if disabled {
                    prop_assert_eq!(service.state(), &marquee::State::Paused);
                    prop_assert_eq!(ctx.current_loop, 0);
                    prop_assert!(!ctx.paused_by_hover);
                    prop_assert!(!ctx.paused_by_focus);
                }
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// RatingGroup
// ────────────────────────────────────────────────────────────────────

mod rating_group_proptests {
    use std::num::NonZero;

    use ars_components::data_display::rating_group;
    use ars_core::{Env, Service};
    use proptest::prelude::*;

    fn arb_props() -> impl Strategy<Value = rating_group::Props> {
        (
            1u32..=8,
            0.0f64..=8.0,
            any::<bool>(),
            prop_oneof![Just(0.25), Just(0.5), Just(1.0), Just(2.0)],
            any::<bool>(),
            any::<bool>(),
        )
            .prop_map(
                |(count, default_value, allow_half, step, readonly, disabled)| {
                    rating_group::Props::new()
                        .id("rating")
                        .count(NonZero::new(count).expect("non-zero"))
                        .default_value(default_value)
                        .allow_half(allow_half)
                        .step(step)
                        .readonly(readonly)
                        .disabled(disabled)
                },
            )
    }

    fn arb_event() -> impl Strategy<Value = rating_group::Event> {
        prop_oneof![
            (-4.0f64..=12.0f64).prop_map(rating_group::Event::Rate),
            (0usize..8).prop_map(rating_group::Event::HoverItem),
            (-4.0f64..=12.0f64).prop_map(rating_group::Event::HoverValue),
            Just(rating_group::Event::UnHover),
            (0usize..8, any::<bool>()).prop_map(|(index, is_keyboard)| {
                rating_group::Event::Focus { index, is_keyboard }
            }),
            Just(rating_group::Event::Blur),
            Just(rating_group::Event::IncrementRating),
            Just(rating_group::Event::DecrementRating),
            Just(rating_group::Event::ClearRating),
        ]
    }

    proptest! {
        #![proptest_config(super::super::common::proptest_config())]

        #[test]
        #[ignore = "proptest — nightly extended-proptest job"]
        fn proptest_rating_group_event_sequences_preserve_invariants(
            props in arb_props(),
            events in prop::collection::vec(arb_event(), 0..64),
        ) {
            let mut service = Service::<rating_group::Machine>::new(
                props,
                &Env::default(),
                &rating_group::Messages::default(),
            );

            let initial_value = *service.context().value.get();

            for event in events {
                drop(service.send(event));

                let ctx = service.context();
                let value = *ctx.value.get();
                let max = f64::from(ctx.count.get());

                prop_assert!(value.is_finite());
                prop_assert!((0.0..=max).contains(&value));

                if let Some(hovered) = ctx.hovered_value {
                    prop_assert!(hovered.is_finite());
                    prop_assert!((0.0..=max).contains(&hovered));
                }

                if ctx.disabled || ctx.readonly {
                    prop_assert_eq!(value, initial_value);
                }
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// TagGroup
// ────────────────────────────────────────────────────────────────────

mod tag_group_proptests {
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
        #![proptest_config(super::super::common::proptest_config())]

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
}

// ────────────────────────────────────────────────────────────────────
// GridList
// ────────────────────────────────────────────────────────────────────

mod grid_list_proptests {
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
        #![proptest_config(super::super::common::proptest_config())]

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
}
