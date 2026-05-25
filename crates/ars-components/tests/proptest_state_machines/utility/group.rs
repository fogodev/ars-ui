use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_group_context_reflects_state_flags(disabled in any::<bool>(), invalid in any::<bool>(), read_only in any::<bool>()) {
        let props = utility_core::group::Props::new()
            .disabled(disabled)
            .invalid(invalid)
            .read_only(read_only);

        let api = utility_core::group::Api::new(props);

        let context = api.group_context();

        prop_assert_eq!(context.disabled, disabled);
        prop_assert_eq!(context.invalid, invalid);
        prop_assert_eq!(context.read_only, read_only);
    }
}
