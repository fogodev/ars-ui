use core::time::Duration;

use ars_components::specialized::timer::{Event, Machine, Mode, Props, State};
use ars_core::{Env, Service};
use proptest::prelude::*;

fn arb_mode() -> impl Strategy<Value = Mode> {
    prop_oneof![Just(Mode::Countdown), Just(Mode::Stopwatch)]
}

prop_compose! {
    fn arb_props()(
        mode in arb_mode(),
        target_millis in 1_000u64..=600_000,
        interval_millis in 1u64..=60_000,
        auto_start in any::<bool>(),
    ) -> Props {
        Props::new()
            .id("timer")
            .mode(mode)
            .target(Duration::from_millis(target_millis))
            .interval(Duration::from_millis(interval_millis))
            .auto_start(auto_start)
    }
}

fn arb_event(target: Duration) -> impl Strategy<Value = Event> {
    let target_millis = u64::try_from(target.as_millis()).unwrap_or(u64::MAX);

    prop_oneof![
        Just(Event::Start),
        Just(Event::Pause),
        Just(Event::Resume),
        Just(Event::Reset),
        Just(Event::Restart),
        Just(Event::Tick),
        (0u64..=target_millis).prop_map(|millis| Event::SetTime(Duration::from_millis(millis))),
    ]
}

fn arb_scenario() -> impl Strategy<Value = (Props, Vec<Event>)> {
    arb_props().prop_flat_map(|props| {
        let events = prop::collection::vec(arb_event(props.target), 0..128);

        (Just(props), events)
    })
}

proptest! {
    #![proptest_config(super::super::common::proptest_config())]

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
    fn timer_event_sequences_preserve_invariants((props, events) in arb_scenario()) {
        let mode = props.mode;
        let target = props.target;

        let mut svc = Service::<Machine>::new(props, &Env::default(), &Default::default());

        drop(svc.take_initial_effects());

        for ev in events {
            drop(svc.send(ev));

            let state = *svc.state();
            let current = svc.context().current;

            if mode == Mode::Stopwatch {
                prop_assert_ne!(state, State::Completed);
            }

            if mode == Mode::Countdown {
                prop_assert!(current <= target);
            }

            let api = svc.connect(&|_| {});

            prop_assert!(api.progress().is_finite());
            prop_assert!(!api.formatted_time().is_empty());

            drop(api.root_attrs());
            drop(api.progress_attrs());
            drop(api.start_trigger_attrs());
            drop(api.pause_trigger_attrs());
            drop(api.reset_trigger_attrs());
        }
    }
}
