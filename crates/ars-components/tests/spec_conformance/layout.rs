//! Spec-conformance tests for `crates/ars-components/src/layout/*`.
//!
//! Each test pulls the expected anatomy from the corresponding component
//! spec's §2 anatomy table and asserts the impl's `Part` enum matches.

use ars_components::layout::{
    aspect_ratio, center, collapsible, frame, grid, scroll_area, splitter, stack,
};

use super::helper::assert_anatomy;

#[test]
fn aspect_ratio_anatomy_matches_spec() {
    assert_anatomy("aspect-ratio", &[(aspect_ratio::Part::Root, "root")]);
}

#[test]
fn frame_anatomy_matches_spec() {
    assert_anatomy(
        "frame",
        &[(frame::Part::Root, "root"), (frame::Part::Iframe, "iframe")],
    );
}

#[test]
fn stack_anatomy_matches_spec() {
    assert_anatomy("stack", &[(stack::Part::Root, "root")]);
}

#[test]
fn center_anatomy_matches_spec() {
    assert_anatomy("center", &[(center::Part::Root, "root")]);
}

#[test]
fn collapsible_anatomy_matches_spec() {
    assert_anatomy(
        "collapsible",
        &[
            (collapsible::Part::Root, "root"),
            (collapsible::Part::Trigger, "trigger"),
            (collapsible::Part::Indicator, "indicator"),
            (collapsible::Part::Content, "content"),
        ],
    );
}

#[test]
fn grid_anatomy_matches_spec() {
    assert_anatomy("grid", &[(grid::Part::Root, "root")]);
}

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

#[test]
fn splitter_anatomy_matches_spec() {
    assert_anatomy(
        "splitter",
        &[
            (splitter::Part::Root, "root"),
            (splitter::Part::Panel { index: 0 }, "panel"),
            (splitter::Part::Handle { index: 0 }, "handle"),
        ],
    );
}
