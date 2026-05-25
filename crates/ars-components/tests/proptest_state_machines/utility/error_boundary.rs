use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    // ── ErrorBoundary ─────────────────────────────────────────────
    //
    // ErrorBoundary's framework-agnostic core is an attribute-only
    // surface driven by a single input: the captured error count. The
    // proptests below pin the invariants the adapter wrappers depend on
    // — the count round-trips through `error_count()`, the count-as-
    // string survives the `data-ars-error-count` attribute, every
    // anatomy part emits the canonical scope/part pair, and the alert
    // markup is invariant under count changes (count is the only field
    // that affects Root's payload).

    /// `Api::error_count()` round-trips any non-negative count we feed it.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_error_boundary_count_round_trips(count in 0usize..=10_000) {
        let api = utility_core::error_boundary::Api::new(count);

        prop_assert_eq!(api.error_count(), count);
    }

    /// `data-ars-error-count` is the `Display` of the count, for any count.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_error_boundary_count_attr_matches_count(count in 0usize..=10_000) {
        let attrs = utility_core::error_boundary::Api::new(count).root_attrs();

        let expected = count.to_string();

        prop_assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-error-count")),
            Some(expected.as_str())
        );
    }

    /// Every anatomy part emits the canonical scope and matching part
    /// data attribute, regardless of error count. The connect-API
    /// dispatch must produce the same `AttrMap` as the inherent helper
    /// for each part.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_error_boundary_part_dispatch_is_canonical(count in 0usize..=10_000) {
        let api = utility_core::error_boundary::Api::new(count);

        let cases = [
            (utility_core::error_boundary::Part::Root,    "root",    api.root_attrs()),
            (utility_core::error_boundary::Part::Message, "message", api.message_attrs()),
            (utility_core::error_boundary::Part::List,    "list",    api.list_attrs()),
            (utility_core::error_boundary::Part::Item,    "item",    api.item_attrs()),
        ];

        for (part, name, helper_attrs) in cases {
            let dispatched = api.part_attrs(part);

            prop_assert_eq!(
                dispatched.get(&HtmlAttr::Data("ars-scope")),
                Some("error-boundary"),
                "scope missing for part {}", name
            );
            prop_assert_eq!(
                dispatched.get(&HtmlAttr::Data("ars-part")),
                Some(name),
            );
            prop_assert_eq!(
                dispatched, helper_attrs,
                "ConnectApi dispatch must equal inherent helper for part {}", name
            );
        }
    }

    /// The accessibility primitives (`role="alert"`, `aria-live`,
    /// `aria-atomic`) are constant across all error counts. The count
    /// is the only input that influences `data-ars-error-count`; every
    /// other Root attr stays put.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_error_boundary_aria_primitives_are_count_invariant(
        a in 0usize..=10_000,
        b in 0usize..=10_000,
    ) {
        let attrs_a = utility_core::error_boundary::Api::new(a).root_attrs();
        let attrs_b = utility_core::error_boundary::Api::new(b).root_attrs();

        for attr in [
            HtmlAttr::Role,
            HtmlAttr::Aria(ars_core::AriaAttr::Live),
            HtmlAttr::Aria(ars_core::AriaAttr::Atomic),
            HtmlAttr::Data("ars-scope"),
            HtmlAttr::Data("ars-part"),
            HtmlAttr::Data("ars-error"),
        ] {
            prop_assert_eq!(
                attrs_a.get(&attr),
                attrs_b.get(&attr),
                "attr {:?} should be count-invariant", attr
            );
        }
    }
}
