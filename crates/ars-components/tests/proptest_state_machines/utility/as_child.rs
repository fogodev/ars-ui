use ars_components::utility::as_child::Props;

use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_as_child_props_round_trip(as_child in any::<bool>()) {
        let props = Props::new().as_child(as_child);

        prop_assert_eq!(props.as_child, as_child);
    }
}
