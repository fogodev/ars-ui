use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_client_only_fallback_round_trips(fallback in prop::option::of("[a-zA-Z0-9 _-]{0,24}".prop_map(String::from))) {
        let props = if let Some(value) = fallback.clone() {
            utility_core::client_only::Props::new().fallback(value)
        } else {
            utility_core::client_only::Props::new()
        };

        prop_assert_eq!(props.fallback, fallback);
    }
}
