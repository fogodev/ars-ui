use ars_components::navigation::pagination;

use crate::helper::assert_anatomy;

#[test]
fn pagination_anatomy_matches_spec() {
    assert_anatomy(
        "pagination",
        &[
            (pagination::Part::Root, "root"),
            (pagination::Part::PrevTrigger, "prev-trigger"),
            (pagination::Part::NextTrigger, "next-trigger"),
            (
                pagination::Part::PageTrigger { page_number: 1 },
                "page-trigger",
            ),
            (pagination::Part::Ellipsis, "ellipsis"),
        ],
    );
}
