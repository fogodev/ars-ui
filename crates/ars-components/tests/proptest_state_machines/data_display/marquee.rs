use ars_components::data_display::marquee;
use ars_core::{Env, Service};
use proptest::prelude::*;

fn arb_props() -> impl Strategy<Value = marquee::Props> {
    (
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        prop::option::of(1usize..=5),
        prop_oneof![
            Just(marquee::Direction::Left),
            Just(marquee::Direction::Right),
            Just(marquee::Direction::Up),
            Just(marquee::Direction::Down),
        ],
    )
        .prop_map(
            |(auto_play, disabled, pause_on_hover, pause_on_focus, loop_count, direction)| {
                marquee::Props::new()
                    .id("marquee")
                    .auto_play(auto_play)
                    .disabled(disabled)
                    .pause_on_hover(pause_on_hover)
                    .pause_on_focus(pause_on_focus)
                    .loop_count_option(loop_count)
                    .direction(direction)
            },
        )
}

fn arb_event() -> impl Strategy<Value = marquee::Event> {
    prop_oneof![
        Just(marquee::Event::Play),
        Just(marquee::Event::Pause),
        Just(marquee::Event::HoverIn),
        Just(marquee::Event::HoverOut),
        Just(marquee::Event::FocusIn),
        Just(marquee::Event::FocusOut),
        Just(marquee::Event::LoopComplete),
        Just(marquee::Event::SyncProps),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_marquee_event_sequences_preserve_invariants(
        props in arb_props(),
        events in prop::collection::vec(arb_event(), 0..64),
    ) {
        let disabled = props.disabled;
        let loop_count = props.loop_count;

        let mut service = Service::<marquee::Machine>::new(
            props,
            &Env::default(),
            &marquee::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            let ctx = service.context();

            if *service.state() == marquee::State::Playing {
                prop_assert!(!ctx.paused_by_hover);
                prop_assert!(!ctx.paused_by_focus);
            }

            if let Some(max) = loop_count {
                prop_assert!(ctx.current_loop <= max);

                if ctx.current_loop >= max {
                    prop_assert_eq!(service.state(), &marquee::State::Paused);
                }
            }

            if disabled {
                prop_assert_eq!(service.state(), &marquee::State::Paused);
                prop_assert_eq!(ctx.current_loop, 0);
                prop_assert!(!ctx.paused_by_hover);
                prop_assert!(!ctx.paused_by_focus);
            }
        }
    }
}
