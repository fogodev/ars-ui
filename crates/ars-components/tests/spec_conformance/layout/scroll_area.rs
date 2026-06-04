use ars_components::layout::scroll_area;

use super::*;

#[test]
fn scroll_area_anatomy_matches_spec() {
    assert_anatomy(
        "scroll-area",
        &[
            (scroll_area::Part::Root, "root"),
            (scroll_area::Part::Viewport, "viewport"),
            (scroll_area::Part::Content, "content"),
            (scroll_area::Part::ScrollbarY, "scrollbar-y"),
            (scroll_area::Part::ThumbY, "thumb-y"),
            (scroll_area::Part::ScrollbarX, "scrollbar-x"),
            (scroll_area::Part::ThumbX, "thumb-x"),
            (scroll_area::Part::CornerSquare, "corner-square"),
        ],
    );
}
