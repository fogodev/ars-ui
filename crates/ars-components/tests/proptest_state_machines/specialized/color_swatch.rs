use ars_components::specialized::color_swatch::{Api, Messages, Part, Props};
use ars_core::{AriaAttr, ColorValue, ConnectApi, CssProperty, Env, HtmlAttr};
use proptest::prelude::*;

fn arb_color() -> impl Strategy<Value = ColorValue> {
    (0.0f64..360.0, 0.0f64..=1.0, 0.0f64..=1.0, 0.0f64..=1.0).prop_map(
        |(hue, saturation, lightness, alpha)| ColorValue::new(hue, saturation, lightness, alpha),
    )
}

proptest! {
    #![proptest_config(super::super::common::proptest_config())]

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
    fn color_swatch_props_map_to_accessible_attrs(
        id in "[a-z]{0,8}",
        color in arb_color(),
        color_name in proptest::option::of("[a-zA-Z ]{1,24}".prop_map(String::from)),
        respect_alpha in any::<bool>(),
    ) {
        let props = Props {
            id: id.clone(),
            color,
            color_name: color_name.clone(),
            respect_alpha,
        };

        let api = Api::new(&props, &Env::default(), &Messages::default());

        let root = api.root_attrs();
        let inner = api.inner_attrs();

        if id.is_empty() {
            prop_assert_eq!(root.get(&HtmlAttr::Id), None);
        } else {
            prop_assert_eq!(root.get(&HtmlAttr::Id), Some(id.as_str()));
        }

        if let Some(name) = color_name {
            prop_assert_eq!(root.get(&HtmlAttr::Aria(AriaAttr::Label)), Some(name.as_str()));
        } else {
            prop_assert!(root.get(&HtmlAttr::Aria(AriaAttr::Label)).is_some_and(|name| !name.is_empty()));
        }

        prop_assert_eq!(root.get(&HtmlAttr::Role), Some("img"));
        prop_assert!(root.styles().iter().any(|(prop, _)| *prop == CssProperty::Custom("ars-swatch-color")));
        prop_assert!(inner.styles().iter().any(|(prop, _)| *prop == CssProperty::Background));

        if respect_alpha && color.alpha < 1.0 {
            prop_assert!(inner.contains(&HtmlAttr::Data("ars-alpha")));
        } else {
            prop_assert!(!inner.contains(&HtmlAttr::Data("ars-alpha")));
        }

        prop_assert_eq!(api.part_attrs(Part::Root), root);
        prop_assert_eq!(api.part_attrs(Part::Inner), inner);
    }
}
