use ars_components::layout::scroll_area;
use ars_core::{Direction, Env, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

fn arb_scroll_axis() -> impl Strategy<Value = scroll_area::Axis> {
    prop_oneof![Just(scroll_area::Axis::X), Just(scroll_area::Axis::Y)]
}

fn arb_scroll_area_props() -> impl Strategy<Value = scroll_area::Props> {
    (
        prop_oneof![
            Just(scroll_area::ScrollOrientation::Vertical),
            Just(scroll_area::ScrollOrientation::Horizontal),
            Just(scroll_area::ScrollOrientation::Both),
        ],
        prop_oneof![
            Just(scroll_area::ScrollbarVisibility::Always),
            Just(scroll_area::ScrollbarVisibility::Auto),
            Just(scroll_area::ScrollbarVisibility::Hover),
            Just(scroll_area::ScrollbarVisibility::Scroll),
        ],
        prop::option::of(1.0f64..200.0),
        prop::option::of(prop_oneof![Just(Direction::Ltr), Just(Direction::Rtl)]),
    )
        .prop_map(|(orientation, visibility, min_thumb, dir)| {
            let mut props = scroll_area::Props::new()
                .id("scroll")
                .orientation(orientation)
                .scrollbar_visibility(visibility);

            if let Some(min_thumb) = min_thumb {
                props = props.min_thumb_size(min_thumb);
            }

            if let Some(dir) = dir {
                props = props.dir(dir);
            }

            props
        })
}

fn arb_scroll_area_event() -> impl Strategy<Value = scroll_area::Event> {
    prop_oneof![
        (-2000.0f64..2000.0, -2000.0f64..2000.0)
            .prop_map(|(x, y)| scroll_area::Event::Scroll { x, y }),
        (
            0.0f64..2000.0,
            0.0f64..2000.0,
            0.0f64..4000.0,
            0.0f64..4000.0
        )
            .prop_map(
                |(viewport_width, viewport_height, content_width, content_height)| {
                    scroll_area::Event::Resize {
                        viewport_width,
                        viewport_height,
                        content_width,
                        content_height,
                    }
                }
            ),
        Just(scroll_area::Event::MouseEnter),
        Just(scroll_area::Event::MouseLeave),
        Just(scroll_area::Event::MouseEnterScrollbar),
        Just(scroll_area::Event::MouseLeaveScrollbar),
        (-200.0f64..2000.0, arb_scroll_axis())
            .prop_map(|(pos, axis)| scroll_area::Event::ThumbDragStart { pos, axis }),
        (-200.0f64..2000.0).prop_map(|pos| scroll_area::Event::ThumbDragMove { pos }),
        Just(scroll_area::Event::ThumbDragEnd),
        (-200.0f64..2000.0, arb_scroll_axis())
            .prop_map(|(pos, axis)| scroll_area::Event::TrackClick { pos, axis }),
        Just(scroll_area::Event::HideTimeout),
        Just(scroll_area::Event::SyncProps),
    ]
}

fn assert_scroll_area_invariants(service: &Service<scroll_area::Machine>) -> TestCaseResult {
    let ctx = service.context();

    prop_assert!(ctx.scroll_x.is_finite());
    prop_assert!(ctx.scroll_y.is_finite());
    prop_assert!(ctx.viewport_width.is_finite());
    prop_assert!(ctx.viewport_height.is_finite());
    prop_assert!(ctx.content_width.is_finite());
    prop_assert!(ctx.content_height.is_finite());
    prop_assert!(ctx.min_thumb_size.is_finite());
    prop_assert!(ctx.drag_start_pointer_pos.is_finite());
    prop_assert!(ctx.drag_start_thumb_pos.is_finite());
    prop_assert!(ctx.drag_start_scroll_pos.is_finite());

    // A drag is always tagged with the axis it started on.
    if *service.state() == scroll_area::State::ThumbDragging {
        prop_assert!(ctx.drag_axis.is_some());
    }

    // A scrollbar may only be visible on an axis its orientation enables.
    if ctx.scrollbar_x_visible {
        prop_assert!(ctx.orientation.allows_x());
    }
    if ctx.scrollbar_y_visible {
        prop_assert!(ctx.orientation.allows_y());
    }

    // Stored offsets are always clamped to the scrollable range.
    let max_x = (ctx.content_width - ctx.viewport_width).max(0.0);
    let max_y = (ctx.content_height - ctx.viewport_height).max(0.0);

    prop_assert!(ctx.scroll_x >= 0.0 && ctx.scroll_x <= max_x);
    prop_assert!(ctx.scroll_y >= 0.0 && ctx.scroll_y <= max_y);

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_scroll_area_event_sequences_preserve_invariants(
        props in arb_scroll_area_props(),
        events in prop::collection::vec(arb_scroll_area_event(), 0..128),
    ) {
        let mut service = Service::<scroll_area::Machine>::new(
            props,
            &Env::default(),
            &scroll_area::Messages::default(),
        );

        assert_scroll_area_invariants(&service)?;

        for event in events {
            drop(service.send(event));

            assert_scroll_area_invariants(&service)?;
        }
    }
}
