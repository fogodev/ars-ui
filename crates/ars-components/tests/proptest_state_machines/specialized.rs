//! Property-based tests for the specialized components.
//!
//! The color-machine blocks feed an arbitrary event sequence through a machine
//! and assert the component's core invariants always hold: stored values stay
//! in range, states stay within the declared set, and `connect()` never panics.
//! The stateless `QrCode` block instead pins its prop->attr mapping across the
//! whole input space. Run in the nightly `extended-proptest` job; `#[ignore]`d
//! in the fast tier.

// ────────────────────────────────────────────────────────────────────
// ColorArea
// ────────────────────────────────────────────────────────────────────

mod color_area_proptests {
    use ars_components::specialized::color_area::{Event, Machine, Props};
    use ars_core::{AriaAttr, ColorChannel, Env, HtmlAttr, Service};
    use proptest::prelude::*;

    fn arb_event() -> impl Strategy<Value = Event> {
        prop_oneof![
            (0.0f64..1.0, 0.0f64..1.0).prop_map(|(x, y)| Event::DragStart { x, y }),
            (0.0f64..1.0, 0.0f64..1.0).prop_map(|(x, y)| Event::DragMove { x, y }),
            Just(Event::DragEnd),
            (0.0f64..0.5).prop_map(|step| Event::IncrementX { step }),
            (0.0f64..0.5).prop_map(|step| Event::DecrementX { step }),
            (0.0f64..0.5).prop_map(|step| Event::IncrementY { step }),
            (0.0f64..0.5).prop_map(|step| Event::DecrementY { step }),
            Just(Event::SetXToMin),
            Just(Event::SetXToMax),
            Just(Event::SetYToMin),
            Just(Event::SetYToMax),
            any::<bool>().prop_map(|is_keyboard| Event::Focus { is_keyboard }),
            Just(Event::Blur),
        ]
    }

    proptest! {
        #![proptest_config(super::super::common::proptest_config())]

        #[test]
        #[ignore = "proptest — nightly extended-proptest job"]
        fn area_event_sequences_keep_value_valid(
            events in prop::collection::vec(arb_event(), 0..64),
        ) {
            let mut svc = Service::<Machine>::new(
                Props { id: "ca".into(), ..Props::default() },
                &Env::default(),
                &Default::default(),
            );

            for ev in events {
                drop(svc.send(ev));
            }

            let api = svc.connect(&|_| {});

            let value = *api.value();

            prop_assert!((0.0..360.0).contains(&value.hue));
            prop_assert!((0.0..=1.0).contains(&value.saturation));
            prop_assert!((0.0..=1.0).contains(&value.lightness));

            // connect output reflects the in-range state: the thumb always
            // exposes a non-empty `aria-valuetext` describing the current color.
            let thumb = api.thumb_attrs();

            prop_assert!(
                thumb
                    .get(&HtmlAttr::Aria(AriaAttr::ValueText))
                    .is_some_and(|text| !text.is_empty())
            );

            // The two channels stay within their ranges.
            for ch in [ColorChannel::Saturation, ColorChannel::Lightness] {
                let (min, max) = ars_core::channel_range(ch);

                let val = ars_core::channel_value(&value, ch);

                prop_assert!(val >= min - 1e-9 && val <= max + 1e-9);
            }
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// ColorSlider
// ────────────────────────────────────────────────────────────────────

mod color_slider_proptests {
    use ars_components::specialized::color_slider::{Event, Machine, Props};
    use ars_core::{AriaAttr, ColorChannel, Env, HtmlAttr, Service};
    use proptest::prelude::*;

    fn arb_channel() -> impl Strategy<Value = ColorChannel> {
        prop_oneof![
            Just(ColorChannel::Hue),
            Just(ColorChannel::Saturation),
            Just(ColorChannel::Alpha),
            Just(ColorChannel::Red),
        ]
    }

    fn arb_event() -> impl Strategy<Value = Event> {
        prop_oneof![
            (0.0f64..1.0).prop_map(|position| Event::DragStart { position }),
            (0.0f64..1.0).prop_map(|position| Event::DragMove { position }),
            Just(Event::DragEnd),
            (0.0f64..50.0).prop_map(|step| Event::Increment { step }),
            (0.0f64..50.0).prop_map(|step| Event::Decrement { step }),
            Just(Event::SetToMin),
            Just(Event::SetToMax),
            any::<bool>().prop_map(|is_keyboard| Event::Focus { is_keyboard }),
            Just(Event::Blur),
        ]
    }

    proptest! {
        #![proptest_config(super::super::common::proptest_config())]

        #[test]
        #[ignore = "proptest — nightly extended-proptest job"]
        fn slider_event_sequences_keep_channel_in_range(
            channel in arb_channel(),
            events in prop::collection::vec(arb_event(), 0..64),
        ) {
            let mut svc = Service::<Machine>::new(
                Props { id: "cs".into(), channel, ..Props::default() },
                &Env::default(),
                &Default::default(),
            );

            for ev in events {
                drop(svc.send(ev));
            }

            let api = svc.connect(&|_| {});

            let (min, max) = ars_core::channel_range(channel);

            let val = ars_core::channel_value(api.value(), channel);

            prop_assert!(val >= min - 1e-9 && val <= max + 1e-9);

            // The thumb's rendered `aria-valuenow` tracks the in-range value.
            let thumb = api.thumb_attrs();

            let rendered = thumb
                .get(&HtmlAttr::Aria(AriaAttr::ValueNow))
                .expect("thumb exposes aria-valuenow")
                .parse::<f64>()
                .expect("aria-valuenow is numeric");

            prop_assert!(rendered >= min - 1e-9 && rendered <= max + 1e-9);
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// ColorWheel
// ────────────────────────────────────────────────────────────────────

mod color_wheel_proptests {
    use ars_components::specialized::color_wheel::{Event, Machine, Props};
    use ars_core::{AriaAttr, Env, HtmlAttr, Service};
    use proptest::prelude::*;

    fn arb_event() -> impl Strategy<Value = Event> {
        prop_oneof![
            (0.0f64..1.0).prop_map(|position| Event::DragStart { position }),
            (0.0f64..1.0).prop_map(|position| Event::DragMove { position }),
            Just(Event::DragEnd),
            (0.0f64..90.0).prop_map(|step| Event::Increment { step }),
            (0.0f64..90.0).prop_map(|step| Event::Decrement { step }),
            Just(Event::SetToMin),
            Just(Event::SetToMax),
            any::<bool>().prop_map(|is_keyboard| Event::Focus { is_keyboard }),
            Just(Event::Blur),
        ]
    }

    proptest! {
        #![proptest_config(super::super::common::proptest_config())]

        #[test]
        #[ignore = "proptest — nightly extended-proptest job"]
        fn wheel_event_sequences_keep_hue_in_range(
            events in prop::collection::vec(arb_event(), 0..64),
        ) {
            let mut svc = Service::<Machine>::new(
                Props { id: "cw".into(), ..Props::default() },
                &Env::default(),
                &Default::default(),
            );

            for ev in events {
                drop(svc.send(ev));
            }

            let api = svc.connect(&|_| {});

            prop_assert!((0.0..=360.0).contains(&api.value().hue));

            // The thumb's rendered `aria-valuenow` is a hue degree in [0, 360].
            let thumb = api.thumb_attrs();

            let rendered = thumb
                .get(&HtmlAttr::Aria(AriaAttr::ValueNow))
                .expect("thumb exposes aria-valuenow")
                .parse::<f64>()
                .expect("aria-valuenow is numeric");

            prop_assert!((0.0..=360.0).contains(&rendered));
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// AngleSlider
// ────────────────────────────────────────────────────────────────────

mod angle_slider_proptests {
    use ars_components::specialized::angle_slider::{Event, Machine, Props};
    use ars_core::{AriaAttr, Env, HtmlAttr, KeyboardKey, Service};
    use proptest::prelude::*;

    fn arb_event() -> impl Strategy<Value = Event> {
        prop_oneof![
            (0.0f64..360.0).prop_map(|angle| Event::DragStart { angle }),
            (0.0f64..360.0).prop_map(|angle| Event::DragMove { angle }),
            Just(Event::DragEnd),
            Just(Event::Increment),
            Just(Event::Decrement),
            (-90.0f64..450.0).prop_map(|angle| Event::SetValue { angle }),
            any::<bool>().prop_map(|is_keyboard| Event::Focus { is_keyboard }),
            Just(Event::Blur),
            prop_oneof![
                Just(KeyboardKey::ArrowUp),
                Just(KeyboardKey::ArrowDown),
                Just(KeyboardKey::Home),
                Just(KeyboardKey::End),
                Just(KeyboardKey::PageUp),
            ]
            .prop_map(|key| Event::KeyDown { key }),
        ]
    }

    proptest! {
        #![proptest_config(super::super::common::proptest_config())]

        #[test]
        #[ignore = "proptest — nightly extended-proptest job"]
        fn angle_event_sequences_keep_value_in_min_max(
            min in 0.0f64..180.0,
            range in 1.0f64..180.0,
            events in prop::collection::vec(arb_event(), 0..64),
        ) {
            let max = min + range;

            let mut svc = Service::<Machine>::new(
                Props { id: "as".into(), min, max, default_value: min, ..Props::default() },
                &Env::default(),
                &Default::default(),
            );

            for ev in events {
                drop(svc.send(ev));
            }

            let api = svc.connect(&|_| {});

            let value = api.value();

            prop_assert!(value >= min - 1e-9 && value <= max + 1e-9, "value {value} out of [{min}, {max}]");

            // The thumb's rendered `aria-valuenow` tracks the in-range value.
            let thumb = api.thumb_attrs();

            let rendered = thumb
                .get(&HtmlAttr::Aria(AriaAttr::ValueNow))
                .expect("thumb exposes aria-valuenow")
                .parse::<f64>()
                .expect("aria-valuenow is numeric");

            prop_assert!(rendered >= min - 1e-9 && rendered <= max + 1e-9);
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// ColorSwatchPicker
// ────────────────────────────────────────────────────────────────────

mod color_swatch_picker_proptests {
    use ars_components::specialized::color_swatch_picker::{Event, Machine, Props};
    use ars_core::{ColorValue, Env, HtmlAttr, Service};
    use proptest::prelude::*;

    fn arb_event() -> impl Strategy<Value = Event> {
        prop_oneof![
            any::<bool>().prop_map(|is_keyboard| Event::Focus { is_keyboard }),
            Just(Event::Blur),
            (0usize..12).prop_map(|index| Event::Select { index }),
            Just(Event::FocusNext),
            Just(Event::FocusPrev),
            Just(Event::FocusUp),
            Just(Event::FocusDown),
            Just(Event::FocusFirst),
            Just(Event::FocusLast),
        ]
    }

    proptest! {
        #![proptest_config(super::super::common::proptest_config())]

        #[test]
        #[ignore = "proptest — nightly extended-proptest job"]
        fn picker_focus_index_stays_in_bounds(
            count in 1usize..8,
            columns in 1usize..5,
            events in prop::collection::vec(arb_event(), 0..64),
        ) {
            let colors: Vec<ColorValue> = (0..count)
                .map(|index| ColorValue::from_hsl((index as f64) * 30.0, 1.0, 0.5))
                .collect();

            let mut svc = Service::<Machine>::new(
                Props { id: "csp".into(), colors, columns, ..Props::default() },
                &Env::default(),
                &Default::default(),
            );

            for ev in events {
                drop(svc.send(ev));
            }

            let api = svc.connect(&|_| {});

            if let Some(idx) = api.focused_index() {
                prop_assert!(idx < count, "focused index {idx} >= {count}");
            }

            // The listbox root's rendered `data-ars-state` is always one of the
            // two declared states.
            let root = api.root_attrs();

            prop_assert_eq!(root.get(&HtmlAttr::Role), Some("listbox"));

            let state = root
                .get(&HtmlAttr::Data("ars-state"))
                .expect("root exposes data-ars-state");

            prop_assert!(state == "idle" || state == "focused");
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// QrCode (stateless — pins the prop->attr mapping over the input space)
// ────────────────────────────────────────────────────────────────────

mod qr_code_proptests {
    use ars_components::specialized::qr_code::{
        Api, Messages, Part, Props, QrErrorCorrection, QrMatrix,
    };
    use ars_core::{AriaAttr, ConnectApi, CssProperty, Env, HtmlAttr};
    use proptest::prelude::*;

    fn arb_error_correction() -> impl Strategy<Value = QrErrorCorrection> {
        prop_oneof![
            Just(QrErrorCorrection::Low),
            Just(QrErrorCorrection::Medium),
            Just(QrErrorCorrection::Quartile),
            Just(QrErrorCorrection::High),
        ]
    }

    /// A mix of plain text and URL values (including mixed-case schemes) so the
    /// link-label branch and its case-insensitive scheme match are exercised.
    fn arb_value() -> impl Strategy<Value = String> {
        prop_oneof![
            "[a-zA-Z0-9 ]{0,24}",
            r"https://[a-z]{1,12}\.[a-z]{2,4}",
            r"http://[a-z]{1,12}\.[a-z]{2,4}",
            r"(?i:https)://[a-z]{1,12}\.[a-z]{2,4}",
            r"(?i:http)://[a-z]{1,12}\.[a-z]{2,4}",
        ]
    }

    /// Module sizes spanning both the valid range and the invalid values
    /// (`0.0`, negative, `NaN`, infinities) that must fall back to the default.
    fn arb_module_size() -> impl Strategy<Value = f64> {
        prop_oneof![
            1.0f64..20.0,
            Just(0.0),
            -20.0f64..0.0,
            Just(f64::NAN),
            Just(f64::INFINITY),
            Just(f64::NEG_INFINITY),
        ]
    }

    /// Mirrors `Api::effective_module_size`: a non-finite or non-positive
    /// `module_size` falls back to the default `4.0`.
    fn effective_module_size(module_size: f64) -> f64 {
        if module_size.is_finite() && module_size > 0.0 {
            module_size
        } else {
            4.0
        }
    }

    /// Mirrors `qr_code::is_url`: an http(s) scheme matched case-insensitively.
    fn is_url(value: &str) -> bool {
        value
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
            || value
                .get(..8)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
    }

    prop_compose! {
        fn arb_props()(
            id in "[a-z]{0,8}",
            value in arb_value(),
            error_correction in arb_error_correction(),
            module_size in arb_module_size(),
            quiet_zone in prop_oneof![0usize..10, Just(usize::MAX), Just(usize::MAX / 2)],
            overlay_src in proptest::option::of("[a-z./]{1,16}"),
            overlay_size in 0.0f64..0.5,
        ) -> Props {
            Props {
                id,
                value,
                error_correction,
                module_size,
                quiet_zone,
                overlay_src,
                overlay_size,
                ..Props::default()
            }
        }
    }

    prop_compose! {
        /// A square matrix of `n` rows of `n` modules, so `size == n`.
        fn arb_matrix()(n in 1usize..10)(
            rows in prop::collection::vec(
                prop::collection::vec(any::<bool>(), n..=n),
                n..=n,
            ),
        ) -> QrMatrix {
            QrMatrix::new(rows)
        }
    }

    fn arb_opt_matrix() -> impl Strategy<Value = Option<QrMatrix>> {
        proptest::option::of(arb_matrix())
    }

    proptest! {
        #![proptest_config(super::super::common::proptest_config())]

        /// `part_attrs(p)` always equals the dedicated `*_attrs()` method.
        #[test]
        #[ignore = "proptest — nightly extended-proptest job"]
        fn qr_part_dispatch_equals_attrs(
            props in arb_props(),
            matrix in arb_opt_matrix(),
        ) {
            let api = Api::new(&props, matrix, &Env::default(), &Messages::default());

            prop_assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
            prop_assert_eq!(api.part_attrs(Part::Frame), api.frame_attrs());
            prop_assert_eq!(api.part_attrs(Part::Pattern), api.pattern_attrs());
            prop_assert_eq!(api.part_attrs(Part::Overlay), api.overlay_attrs());
            prop_assert_eq!(
                api.part_attrs(Part::DownloadTrigger),
                api.download_trigger_attrs()
            );
        }

        /// The root carries the scope/part contract but never the image role or
        /// accessible name (those live on the pattern), so an interactive
        /// DownloadTrigger inside the root stays in the accessibility tree.
        #[test]
        #[ignore = "proptest — nightly extended-proptest job"]
        fn qr_root_has_scope_part_but_not_image_role(
            props in arb_props(),
            matrix in arb_opt_matrix(),
        ) {
            let attrs =
                Api::new(&props, matrix, &Env::default(), &Messages::default()).root_attrs();

            prop_assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("qr-code"));
            prop_assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
            prop_assert!(!attrs.contains(&HtmlAttr::Role));
            prop_assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Label)));
        }

        /// The pattern is the accessible image: scope/part plus `role="img"` and
        /// the URL-aware aria-label (link template iff the value is an http(s)
        /// URL, scheme matched case-insensitively).
        #[test]
        #[ignore = "proptest — nightly extended-proptest job"]
        fn qr_pattern_is_image_with_url_aware_label(
            props in arb_props(),
            matrix in arb_opt_matrix(),
        ) {
            let expected = if is_url(&props.value) {
                format!("QR code linking to {}", props.value)
            } else {
                format!("QR code: {}", props.value)
            };

            let attrs =
                Api::new(&props, matrix, &Env::default(), &Messages::default()).pattern_attrs();

            prop_assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("qr-code"));
            prop_assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("pattern"));
            prop_assert_eq!(attrs.get(&HtmlAttr::Role), Some("img"));
            prop_assert_eq!(
                attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
                Some(expected.as_str())
            );
        }

        /// The root `id` is emitted exactly when `props.id` is non-empty.
        #[test]
        #[ignore = "proptest — nightly extended-proptest job"]
        fn qr_id_emitted_iff_non_empty(
            props in arb_props(),
            matrix in arb_opt_matrix(),
        ) {
            let id = props.id.clone();

            let attrs =
                Api::new(&props, matrix, &Env::default(), &Messages::default()).root_attrs();

            if id.is_empty() {
                prop_assert_eq!(attrs.get(&HtmlAttr::Id), None);
            } else {
                prop_assert_eq!(attrs.get(&HtmlAttr::Id), Some(id.as_str()));
            }
        }

        /// `pixel_size` follows the quiet-zone formula (or `0.0` with no matrix),
        /// and the width/height styles render that size in pixels.
        #[test]
        #[ignore = "proptest — nightly extended-proptest job"]
        fn qr_pixel_size_matches_formula_and_dimensions(
            props in arb_props(),
            matrix in arb_opt_matrix(),
        ) {
            let api = Api::new(&props, matrix, &Env::default(), &Messages::default());

            let expected = match api.matrix() {
                Some(matrix) => {
                    (matrix.size as f64 + props.quiet_zone as f64 * 2.0)
                        * effective_module_size(props.module_size)
                }

                None => 0.0,
            };

            prop_assert!((api.pixel_size() - expected).abs() < f64::EPSILON);

            let attrs = api.root_attrs();
            let pixels = format!("{expected}px");

            prop_assert_eq!(
                attrs
                    .styles()
                    .iter()
                    .find(|(prop, _)| *prop == CssProperty::Width)
                    .map(|(_, value)| value.as_str()),
                Some(pixels.as_str())
            );
            prop_assert_eq!(
                attrs
                    .styles()
                    .iter()
                    .find(|(prop, _)| *prop == CssProperty::Height)
                    .map(|(_, value)| value.as_str()),
                Some(pixels.as_str())
            );
        }

        /// The overlay `src` and its decorative empty `alt` are emitted exactly
        /// when `overlay_src` is set.
        #[test]
        #[ignore = "proptest — nightly extended-proptest job"]
        fn qr_overlay_src_and_alt_emitted_iff_present(props in arb_props()) {
            let overlay = props.overlay_src.clone();

            let attrs =
                Api::new(&props, None, &Env::default(), &Messages::default()).overlay_attrs();

            if let Some(src) = overlay {
                prop_assert_eq!(attrs.get(&HtmlAttr::Src), Some(src.as_str()));
                prop_assert_eq!(attrs.get(&HtmlAttr::Alt), Some(""));
            } else {
                prop_assert_eq!(attrs.get(&HtmlAttr::Src), None);
                prop_assert_eq!(attrs.get(&HtmlAttr::Alt), None);
            }
        }

        /// The download trigger is always a labelled button regardless of props.
        #[test]
        #[ignore = "proptest — nightly extended-proptest job"]
        fn qr_download_trigger_is_labelled_button(props in arb_props()) {
            let attrs = Api::new(&props, None, &Env::default(), &Messages::default())
                .download_trigger_attrs();

            prop_assert_eq!(attrs.get(&HtmlAttr::Type), Some("button"));
            prop_assert_eq!(
                attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
                Some("Download QR code")
            );
        }
    }
}
