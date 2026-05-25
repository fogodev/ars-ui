use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_landmark_labelledby_takes_precedence(labelledby in prop::option::of("[a-zA-Z0-9_-]{1,16}".prop_map(String::from))) {
        let mut props = utility_core::landmark::Props::new().role(utility_core::landmark::Role::Region);

        if let Some(id) = labelledby.clone() {
            props = props.labelledby_id(id);
        }

        let api = utility_core::landmark::Api::new(props, &Env::default(), &utility_core::landmark::Messages::default());

        let attrs = api.root_attrs(false);

        prop_assert_eq!(attrs.get(&HtmlAttr::Aria(ars_core::AriaAttr::LabelledBy)), labelledby.as_deref());
    }
}
