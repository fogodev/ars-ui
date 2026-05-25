use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    /// Same dispatch invariant for Separator.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_separator_part_root_dispatch_equals_root_attrs(
        props in arb_separator_props(),
    ) {
        let api = utility_core::separator::Api::new(props);

        prop_assert_eq!(api.part_attrs(utility_core::separator::Part::Root), api.root_attrs());
    }

    /// Same scope/part contract for Separator.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_separator_root_attrs_always_have_scope_and_part(
        props in arb_separator_props(),
    ) {
        let attrs = utility_core::separator::Api::new(props).root_attrs();

        prop_assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("separator"));
        prop_assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
    }

    /// Decorative separators carry `role="none"` and omit both
    /// `aria-orientation` and the `data-ars-orientation` styling hook.
    /// Semantic separators carry `role="separator"` plus `aria-orientation`
    /// and `data-ars-orientation` matching the layout axis. Pins F3/F9.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_separator_decorative_branch_invariants(
        props in arb_separator_props(),
    ) {
        let decorative = props.decorative;

        let orientation = props.orientation;

        let attrs = utility_core::separator::Api::new(props).root_attrs();

        if decorative {
            prop_assert_eq!(attrs.get(&HtmlAttr::Role), Some("none"));
            prop_assert_eq!(
                attrs.get(&HtmlAttr::Aria(ars_core::AriaAttr::Orientation)),
                None
            );
            prop_assert_eq!(attrs.get(&HtmlAttr::Data("ars-orientation")), None);
            prop_assert_eq!(
                attrs.get(&HtmlAttr::Aria(ars_core::AriaAttr::Hidden)),
                None
            );
        } else {
            let expected = match orientation {
                Orientation::Horizontal => "horizontal",
                Orientation::Vertical => "vertical",
            };

            prop_assert_eq!(attrs.get(&HtmlAttr::Role), Some("separator"));
            prop_assert_eq!(
                attrs.get(&HtmlAttr::Aria(ars_core::AriaAttr::Orientation)),
                Some(expected)
            );
            prop_assert_eq!(
                attrs.get(&HtmlAttr::Data("ars-orientation")),
                Some(expected)
            );
        }
    }

    /// For decorative separators, `orientation` is invisible to the
    /// agnostic-core output. Pins the "decorative collapses orientation"
    /// invariant tested under one example in the unit suite.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_separator_decorative_orientation_does_not_affect_output(
        id in arb_short_id(),
    ) {
        let h = utility_core::separator::Api::new(utility_core::separator::Props {
            id: id.clone(),
            orientation: Orientation::Horizontal,
            decorative: true,
        })
        .root_attrs();

        let v = utility_core::separator::Api::new(utility_core::separator::Props {
            id,
            orientation: Orientation::Vertical,
            decorative: true,
        })
        .root_attrs();

        prop_assert_eq!(h, v);
    }

    /// `Api::props()` round-trip for Separator.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_separator_api_props_round_trip(
        props in arb_separator_props(),
    ) {
        let original = props.clone();

        let api = utility_core::separator::Api::new(props);

        prop_assert_eq!(api.props(), &original);
        prop_assert_eq!(api.id(), original.id.as_str());
        prop_assert_eq!(api.orientation(), original.orientation);
        prop_assert_eq!(api.decorative(), original.decorative);
    }
}
