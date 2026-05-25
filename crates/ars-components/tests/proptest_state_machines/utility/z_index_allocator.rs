use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_z_index_allocator_context_allocates_monotonically(count in 0usize..64) {
        let context = utility_core::z_index_allocator::Context::new();
        let mut previous = None;

        for _ in 0..count {
            let current = context.allocate();

            if let Some(previous) = previous {
                prop_assert!(current > previous);
            }

            previous = Some(current);
        }
    }
}
