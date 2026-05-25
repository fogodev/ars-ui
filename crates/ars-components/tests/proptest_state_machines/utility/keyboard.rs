use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_keyboard_decorative_controls_aria_hidden(decorative in any::<bool>()) {
        let api = utility_core::keyboard::Api::new(
            utility_core::keyboard::Props::new().decorative(decorative),
        );

        let attrs = api.root_attrs();

        prop_assert_eq!(api.part_attrs(utility_core::keyboard::Part::Root), attrs.clone());
        prop_assert_eq!(
            attrs.get(&HtmlAttr::Aria(ars_core::AriaAttr::Hidden)),
            decorative.then_some("true"),
        );
    }
}
