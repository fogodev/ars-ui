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

fn arb_value() -> impl Strategy<Value = String> {
    prop_oneof![
        "[a-zA-Z0-9 ]{0,24}",
        r"https://[a-z]{1,12}\.[a-z]{2,4}",
        r"http://[a-z]{1,12}\.[a-z]{2,4}",
        r"(?i:https)://[a-z]{1,12}\.[a-z]{2,4}",
        r"(?i:http)://[a-z]{1,12}\.[a-z]{2,4}",
    ]
}

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

fn effective_module_size(module_size: f64) -> f64 {
    if module_size.is_finite() && module_size > 0.0 {
        module_size
    } else {
        4.0
    }
}

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

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
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

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
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

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
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

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
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

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
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

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
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

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
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
