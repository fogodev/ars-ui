use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_heading_level_from_u8_clamps(value in any::<u8>()) {
        let level = utility_core::heading::Level::from_u8(value);

        let numeric = level.as_u8();

        prop_assert!((1..=6).contains(&numeric));
    }
}
