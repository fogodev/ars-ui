use core::{num::NonZero, time::Duration};

use ars_components::layout::carousel;
use ars_core::{Env, Orientation, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

fn arb_carousel_options() -> impl Strategy<Value = carousel::AutoPlayOptions> {
    (any::<bool>(), any::<bool>(), any::<bool>()).prop_map(
        |(stop_on_interaction, pause_on_focus, pause_on_hover)| carousel::AutoPlayOptions {
            interval: Duration::from_millis(1000),
            stop_on_interaction,
            pause_on_focus,
            pause_on_hover,
        },
    )
}

fn arb_carousel_props() -> impl Strategy<Value = carousel::Props> {
    (
        1usize..8,
        any::<bool>(),
        prop::option::of(arb_carousel_options()),
        prop_oneof![Just(Orientation::Horizontal), Just(Orientation::Vertical)],
        any::<bool>(),
        1usize..4,
    )
        .prop_flat_map(
            |(count, loop_nav, auto_play, orientation, is_rtl, slides_per_move)| {
                (0usize..count).prop_map(move |default_index| carousel::Props {
                    id: String::from("carousel"),
                    slide_count: NonZero::new(count).expect("non-zero slide count"),
                    loop_nav,
                    auto_play: auto_play.clone(),
                    orientation: Some(orientation),
                    is_rtl,
                    slides_per_move: Some(slides_per_move),
                    default_index: Some(default_index),
                    ..carousel::Props::default()
                })
            },
        )
}

/// `GoToSlide`/`FocusSlide` indices are generated past the valid slide range
/// on purpose: the machine saturating-clamps them to the last slide (spec
/// §1.5), so the `current_index < slide_count` invariant must hold even for
/// out-of-range targets.
fn arb_carousel_event(slide_count: usize) -> impl Strategy<Value = carousel::Event> {
    prop_oneof![
        (0..slide_count + 4).prop_map(|index| carousel::Event::GoToSlide { index }),
        Just(carousel::Event::GoToNext),
        Just(carousel::Event::GoToPrev),
        Just(carousel::Event::AutoPlayStart),
        Just(carousel::Event::AutoPlayStop),
        Just(carousel::Event::AutoPlayTick),
        Just(carousel::Event::AutoPlayPause),
        Just(carousel::Event::AutoPlayResume),
        Just(carousel::Event::TransitionEnd),
        (-500.0f64..500.0, 0.0f64..10_000.0)
            .prop_map(|(pos, timestamp)| carousel::Event::PointerDown { pos, timestamp }),
        (-500.0f64..500.0, 0.0f64..10_000.0)
            .prop_map(|(pos, timestamp)| carousel::Event::PointerMove { pos, timestamp }),
        Just(carousel::Event::PointerUp),
        Just(carousel::Event::PointerCancel),
        (0..slide_count + 4).prop_map(|index| carousel::Event::FocusSlide { index }),
        Just(carousel::Event::Blur),
    ]
}

fn assert_carousel_invariants(service: &Service<carousel::Machine>) -> TestCaseResult {
    let ctx = service.context();

    prop_assert!(ctx.current_index() < ctx.slide_count.get());
    prop_assert!(ctx.drag_delta.is_finite());
    prop_assert!(ctx.swipe_velocity.is_finite());

    if let Some(pos) = ctx.drag_start_pos {
        prop_assert!(pos.is_finite());
    }

    if let Some(timestamp) = ctx.drag_last_timestamp {
        prop_assert!(timestamp.is_finite());
    }

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_carousel_event_sequences_preserve_invariants(
        (props, events) in arb_carousel_props().prop_flat_map(|props| {
            let slide_count = props.slide_count.get();
            (
                Just(props),
                prop::collection::vec(arb_carousel_event(slide_count), 0..128),
            )
        }),
    ) {
        let mut service = Service::<carousel::Machine>::new(
            props,
            &Env::default(),
            &carousel::Messages::default(),
        );

        assert_carousel_invariants(&service)?;

        for event in events {
            drop(service.send(event));

            assert_carousel_invariants(&service)?;
        }
    }
}
