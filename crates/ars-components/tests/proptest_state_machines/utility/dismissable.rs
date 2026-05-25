use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_dismissable_part_dispatch_matches_helpers(label in "[a-zA-Z0-9 _-]{0,24}") {
        let api = utility_core::dismissable::Api::new(
            utility_core::dismissable::Props::new(),
            label,
        );

        prop_assert_eq!(
            api.part_attrs(utility_core::dismissable::Part::Root),
            api.root_attrs()
        );
        prop_assert_eq!(
            api.part_attrs(utility_core::dismissable::Part::DismissButton),
            api.dismiss_button_attrs()
        );
    }
}
