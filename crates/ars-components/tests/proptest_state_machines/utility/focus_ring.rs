use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_focus_ring_attrs_track_context(focus_visible in any::<bool>()) {
        let api = utility_core::focus_ring::Api::new(
            utility_core::focus_ring::Context { focus_visible },
            utility_core::focus_ring::Props::new().id("ring"),
        );
        let attrs = api.root_attrs();

        prop_assert_eq!(api.part_attrs(utility_core::focus_ring::Part::Root), attrs.clone());
        prop_assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-focus-visible")),
            focus_visible.then_some("true"),
        );
    }
}
