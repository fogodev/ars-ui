use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    /// `Api::part_attrs(Part::Root)` always equals `Api::root_attrs()` for
    /// any valid `Props`. Pins the `ConnectApi` dispatch shape.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_visually_hidden_part_root_dispatch_equals_root_attrs(
        props in arb_visually_hidden_props(),
    ) {
        let api = utility_core::visually_hidden::Api::new(props);

        prop_assert_eq!(api.part_attrs(utility_core::visually_hidden::Part::Root), api.root_attrs());
    }

    /// `root_attrs()` always carries the canonical scope and part data
    /// attrs. Pins the agnostic-core anatomy contract from spec §2.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_visually_hidden_root_attrs_always_have_scope_and_part(
        props in arb_visually_hidden_props(),
    ) {
        let attrs = utility_core::visually_hidden::Api::new(props).root_attrs();

        prop_assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-scope")),
            Some("visually-hidden")
        );
        prop_assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
    }

    /// The `is_focusable` flag and the `ars-visually-hidden` class are
    /// mutually exclusive (spec §4 forbids combining them — the class
    /// would clip unconditionally and break focus reveal).
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_visually_hidden_focusable_and_class_are_mutually_exclusive(
        props in arb_visually_hidden_props(),
    ) {
        let is_focusable = props.is_focusable;

        let attrs = utility_core::visually_hidden::Api::new(props).root_attrs();

        let has_class = attrs.get(&HtmlAttr::Class) == Some("ars-visually-hidden");

        let has_focusable_hook = attrs.contains(&HtmlAttr::Data("ars-visually-hidden-focusable"));

        prop_assert!(
            !(has_class && has_focusable_hook),
            "class and focusable hook must never coexist"
        );
        prop_assert_eq!(has_focusable_hook, is_focusable);
        prop_assert_eq!(has_class, !is_focusable);
    }

    /// `as_child` is an adapter render-path flag and must NOT influence
    /// agnostic-core attribute output.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_visually_hidden_as_child_does_not_affect_root_attrs(
        id in arb_short_id(),
        is_focusable in any::<bool>(),
    ) {
        let without = utility_core::visually_hidden::Api::new(utility_core::visually_hidden::Props {
            id: id.clone(),
            as_child: false,
            is_focusable,
        })
        .root_attrs();

        let with = utility_core::visually_hidden::Api::new(utility_core::visually_hidden::Props {
            id,
            as_child: true,
            is_focusable,
        })
        .root_attrs();

        prop_assert_eq!(without, with);
    }

    /// `Api::props()` returns a reference to the originally-supplied Props
    /// (round-trip). Pins the F13 escape-hatch contract.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_visually_hidden_api_props_round_trip(
        props in arb_visually_hidden_props(),
    ) {
        let original = props.clone();

        let api = utility_core::visually_hidden::Api::new(props);

        prop_assert_eq!(api.props(), &original);
        prop_assert_eq!(api.id(), original.id.as_str());
        prop_assert_eq!(api.as_child(), original.as_child);
        prop_assert_eq!(api.is_focusable(), original.is_focusable);
    }
}
