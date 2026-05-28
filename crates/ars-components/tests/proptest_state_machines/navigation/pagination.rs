//! Property-based tests for the `navigation/pagination` state machine.

use ars_components::navigation::pagination;
use ars_core::{Env, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

fn arb_pagination_event() -> impl Strategy<Value = pagination::Event> {
    prop_oneof![
        (0u32..64).prop_map(pagination::Event::GoToPage),
        Just(pagination::Event::NextPage),
        Just(pagination::Event::PrevPage),
        Just(pagination::Event::GoToFirstPage),
        Just(pagination::Event::GoToLastPage),
        (1u32..32).prop_map(|size| pagination::Event::SetPageSize(
            core::num::NonZeroU32::new(size).expect("range starts at one")
        )),
    ]
}

fn arb_pagination_props() -> impl Strategy<Value = pagination::Props> {
    (
        prop::option::of(0u32..64),
        0u32..64,
        1u32..32,
        0u32..256,
        0u32..4,
        1u32..3,
    )
        .prop_map(
            |(page, default_page, page_size, total_items, sibling_count, boundary_count)| {
                let mut props = pagination::Props::new()
                    .id("pagination")
                    .default_page(default_page)
                    .page_size(core::num::NonZeroU32::new(page_size).expect("range starts at one"))
                    .total_items(total_items)
                    .sibling_count(sibling_count)
                    .boundary_count(boundary_count);

                if let Some(page) = page {
                    props = props.page(page);
                }

                props
            },
        )
}

fn assert_pagination_invariants(service: &Service<pagination::Machine>) -> TestCaseResult {
    let ctx = service.context();
    let page = *ctx.page.get();

    prop_assert!(page >= 1);
    prop_assert!(page <= ctx.page_count);

    let mut previous = None;

    for entry in ctx.page_range().into_iter().flatten() {
        if let Some(previous) = previous {
            prop_assert!(entry > previous, "page range must be strictly increasing");
        }

        previous = Some(entry);
    }

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    /// Pagination keeps page state within the derived one-based bounds.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_pagination_page_bounds_and_ranges_hold(
        props in arb_pagination_props(),
        events in prop::collection::vec(arb_pagination_event(), 0..64),
    ) {
        let mut service = Service::<pagination::Machine>::new(
            props,
            &Env::default(),
            &pagination::Messages::default(),
        );

        assert_pagination_invariants(&service)?;

        for event in events {
            drop(service.send(event));

            assert_pagination_invariants(&service)?;
        }
    }
}
