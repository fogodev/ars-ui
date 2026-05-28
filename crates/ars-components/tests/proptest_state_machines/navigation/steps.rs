//! Property-based tests for the `navigation/steps` state machine.

use ars_components::navigation::steps;
use ars_core::{Env, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

use super::arb_orientation;

fn arb_step_status() -> impl Strategy<Value = steps::Status> {
    prop_oneof![
        Just(steps::Status::Incomplete),
        Just(steps::Status::Current),
        Just(steps::Status::Complete),
        Just(steps::Status::Error),
    ]
}

fn arb_steps_event() -> impl Strategy<Value = steps::Event> {
    prop_oneof![
        (0u32..12).prop_map(steps::Event::GoToStep),
        Just(steps::Event::NextStep),
        Just(steps::Event::PrevStep),
        (0u32..12).prop_map(steps::Event::CompleteStep),
        (0u32..12, arb_step_status())
            .prop_map(|(step, status)| steps::Event::SetStatus { step, status }),
    ]
}

fn arb_steps_props() -> impl Strategy<Value = steps::Props> {
    (
        prop::option::of(0u32..12),
        0u32..12,
        1u32..12,
        any::<bool>(),
        arb_orientation(),
        prop::collection::vec(arb_step_status(), 0..12),
    )
        .prop_map(
            |(step, default_step, count, linear, orientation, statuses)| {
                let mut props = steps::Props::new()
                    .id("steps")
                    .default_step(default_step)
                    .count(core::num::NonZeroU32::new(count).expect("range starts at one"))
                    .linear(linear)
                    .orientation(orientation)
                    .statuses(statuses)
                    .is_step_skippable(|_| true)
                    .is_step_valid(|_| true);

                if let Some(step) = step {
                    props = props.step(step);
                }

                props
            },
        )
}

fn assert_steps_invariants(service: &Service<steps::Machine>) -> TestCaseResult {
    let ctx = service.context();
    let step = *ctx.step.get();

    prop_assert!(step < ctx.count.get());
    prop_assert_eq!(ctx.statuses.len(), ctx.count.get() as usize);
    let current_positions = ctx
        .statuses
        .iter()
        .enumerate()
        .filter_map(|(index, status)| (*status == steps::Status::Current).then_some(index as u32))
        .collect::<Vec<_>>();

    prop_assert!(
        current_positions.len() <= 1,
        "steps must not keep multiple current statuses"
    );

    if let Some(current_position) = current_positions.first() {
        prop_assert_eq!(*current_position, step);
    }

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    /// Steps keeps the current index in range and exactly one current status.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_steps_current_status_invariants_hold(
        props in arb_steps_props(),
        events in prop::collection::vec(arb_steps_event(), 0..64),
    ) {
        let mut service = Service::<steps::Machine>::new(
            props,
            &Env::default(),
            &steps::Messages::default(),
        );

        assert_steps_invariants(&service)?;

        for event in events {
            drop(service.send(event));

            assert_steps_invariants(&service)?;
        }
    }
}
