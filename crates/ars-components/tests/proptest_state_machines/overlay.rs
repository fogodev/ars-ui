use core::time::Duration;

use ars_components::overlay::{
    positioning::{Offset, Placement, PositioningOptions},
    presence, tooltip,
};
use ars_core::{Direction, Env, SendResult, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

const MIN_TOUCH_AUTO_HIDE: Duration = Duration::from_secs(5);

fn arb_presence_props() -> impl Strategy<Value = presence::Props> {
    (any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>()).prop_map(
        |(present, lazy_mount, skip_animation, reduce_motion)| presence::Props {
            id: "presence".to_string(),
            present,
            lazy_mount,
            skip_animation,
            reduce_motion,
        },
    )
}

fn arb_presence_event() -> impl Strategy<Value = presence::Event> {
    prop_oneof![
        Just(presence::Event::Mount),
        Just(presence::Event::Unmount),
        Just(presence::Event::ContentReady),
        Just(presence::Event::AnimationEnd),
    ]
}

#[derive(Clone, Debug)]
enum TooltipStep {
    Send(tooltip::Event),
    SetProps(tooltip::Props),
}

fn arb_direction() -> impl Strategy<Value = Direction> {
    prop_oneof![Just(Direction::Ltr), Just(Direction::Rtl)]
}

fn arb_placement() -> impl Strategy<Value = Placement> {
    prop_oneof![
        Just(Placement::Bottom),
        Just(Placement::BottomStart),
        Just(Placement::BottomEnd),
        Just(Placement::Top),
        Just(Placement::TopStart),
        Just(Placement::TopEnd),
        Just(Placement::Left),
        Just(Placement::LeftStart),
        Just(Placement::LeftEnd),
        Just(Placement::Right),
        Just(Placement::RightStart),
        Just(Placement::RightEnd),
        Just(Placement::Auto),
        Just(Placement::AutoStart),
        Just(Placement::AutoEnd),
        Just(Placement::Start),
        Just(Placement::End),
        Just(Placement::StartTop),
        Just(Placement::StartBottom),
        Just(Placement::EndTop),
        Just(Placement::EndBottom),
    ]
}

fn arb_duration(max_millis: u64) -> impl Strategy<Value = Duration> {
    (0..=max_millis).prop_map(Duration::from_millis)
}

fn arb_positioning_options() -> impl Strategy<Value = PositioningOptions> {
    (
        arb_placement(),
        -16.0f64..=16.0,
        -16.0f64..=16.0,
        any::<bool>(),
        any::<bool>(),
        0.0f64..=32.0,
        0.0f64..=32.0,
        any::<bool>(),
        prop::collection::vec(arb_placement(), 0..4),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(
                placement,
                main_axis,
                cross_axis,
                flip,
                shift,
                shift_padding,
                arrow_padding,
                auto_max_size,
                fallback_placements,
                keyboard_aware,
                auto_placement,
            )| PositioningOptions {
                placement,
                offset: Offset {
                    main_axis,
                    cross_axis,
                },
                flip,
                shift,
                shift_padding,
                arrow_padding,
                auto_max_size,
                fallback_placements,
                keyboard_aware,
                auto_placement,
            },
        )
}

fn arb_tooltip_props() -> impl Strategy<Value = tooltip::Props> {
    (
        (
            prop::option::of(any::<bool>()),
            any::<bool>(),
            arb_duration(1_000),
            arb_duration(1_000),
            any::<bool>(),
            arb_positioning_options(),
            any::<bool>(),
        ),
        (
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            arb_direction(),
            arb_duration(30_000),
        ),
    )
        .prop_map(
            |(
                (
                    open,
                    default_open,
                    open_delay,
                    close_delay,
                    disabled,
                    positioning,
                    close_on_escape,
                ),
                (
                    close_on_click,
                    close_on_scroll,
                    lazy_mount,
                    unmount_on_exit,
                    dir,
                    touch_auto_hide,
                ),
            )| tooltip::Props {
                id: "tooltip".to_string(),
                open,
                default_open,
                open_delay,
                close_delay,
                disabled,
                positioning,
                close_on_escape,
                close_on_click,
                close_on_scroll,
                on_open_change: None,
                lazy_mount,
                unmount_on_exit,
                dir,
                touch_auto_hide,
            },
        )
}

fn arb_tooltip_event() -> impl Strategy<Value = tooltip::Event> {
    prop_oneof![
        Just(tooltip::Event::PointerEnter),
        Just(tooltip::Event::PointerLeave),
        Just(tooltip::Event::Focus),
        Just(tooltip::Event::Blur),
        Just(tooltip::Event::ContentPointerEnter),
        Just(tooltip::Event::ContentPointerLeave),
        Just(tooltip::Event::OpenTimerFired),
        Just(tooltip::Event::CloseTimerFired),
        Just(tooltip::Event::CloseOnEscape),
        Just(tooltip::Event::CloseOnClick),
        Just(tooltip::Event::CloseOnScroll),
        Just(tooltip::Event::Open),
        Just(tooltip::Event::Close),
        any::<bool>().prop_map(tooltip::Event::SetControlledOpen),
        Just(tooltip::Event::SyncProps),
        (0..=4_000u32).prop_map(tooltip::Event::SetZIndex),
    ]
}

fn arb_tooltip_step() -> impl Strategy<Value = TooltipStep> {
    prop_oneof![
        arb_tooltip_event().prop_map(TooltipStep::Send),
        arb_tooltip_props().prop_map(TooltipStep::SetProps),
    ]
}

const fn tooltip_event_bypasses_disabled(event: tooltip::Event) -> bool {
    matches!(
        event,
        tooltip::Event::SetControlledOpen(_)
            | tooltip::Event::SyncProps
            | tooltip::Event::CloseTimerFired
            | tooltip::Event::Close
            | tooltip::Event::SetZIndex(_)
    )
}

const fn tooltip_guard_rejects(event: tooltip::Event, props: &tooltip::Props) -> bool {
    matches!(event, tooltip::Event::CloseOnEscape) && !props.close_on_escape
        || matches!(event, tooltip::Event::CloseOnClick) && !props.close_on_click
        || matches!(event, tooltip::Event::CloseOnScroll) && !props.close_on_scroll
}

fn assert_tooltip_state_context_invariants(service: &Service<tooltip::Machine>) -> TestCaseResult {
    prop_assert_eq!(
        service.context().open,
        matches!(
            service.state(),
            tooltip::State::Open | tooltip::State::ClosePending
        )
    );
    prop_assert!(service.context().touch_auto_hide >= MIN_TOUCH_AUTO_HIDE);
    prop_assert_eq!(&service.context().trigger_id, "tooltip-trigger");
    prop_assert_eq!(&service.context().content_id, "tooltip-content");
    prop_assert_eq!(
        &service.context().hidden_description_id,
        "tooltip-content-description"
    );

    Ok(())
}

fn assert_tooltip_send_result_invariants(
    service: &Service<tooltip::Machine>,
    event: tooltip::Event,
    result: &SendResult<tooltip::Machine>,
    before_state: tooltip::State,
    before_context: &tooltip::Context,
    before_props: &tooltip::Props,
) -> TestCaseResult {
    if before_context.disabled && !tooltip_event_bypasses_disabled(event) {
        prop_assert_eq!(service.state(), &before_state);
        prop_assert_eq!(service.context(), before_context);
        prop_assert!(result.pending_effects.is_empty());
        prop_assert!(result.cancel_effects.is_empty());
    }

    if tooltip_guard_rejects(event, before_props) {
        prop_assert_eq!(service.state(), &before_state);
        prop_assert_eq!(service.context(), before_context);
        prop_assert!(result.pending_effects.is_empty());
        prop_assert!(result.cancel_effects.is_empty());
    }

    if before_props.open.is_some() && !matches!(event, tooltip::Event::SetControlledOpen(_)) {
        prop_assert_eq!(service.context().open, before_context.open);
    }

    if let tooltip::Event::SetControlledOpen(open) = event {
        prop_assert_eq!(service.context().open, open);
        prop_assert_eq!(
            service.state(),
            if open {
                &tooltip::State::Open
            } else {
                &tooltip::State::Closed
            }
        );
    }

    if let tooltip::Event::SetZIndex(z_index) = event {
        prop_assert_eq!(service.context().z_index, Some(z_index));
    } else {
        prop_assert_eq!(service.context().z_index, before_context.z_index);
    }

    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(
        std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000)
    ))]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_presence_state_context_invariants_hold(
        props in arb_presence_props(),
        events in prop::collection::vec(arb_presence_event(), 0..128),
    ) {
        let mut service = Service::<presence::Machine>::new(props, &Env::default(), &presence::Messages);

        for event in events {
            drop(service.send(event));

            match service.state() {
                presence::State::Unmounted => {
                    prop_assert!(!service.context().present);
                    prop_assert!(!service.context().mounted);
                    prop_assert!(!service.context().unmounting);
                }

                presence::State::Mounting => {
                    prop_assert!(service.context().present);
                    prop_assert!(service.context().mounted);
                    prop_assert!(!service.context().unmounting);
                }

                presence::State::Mounted => {
                    prop_assert!(service.context().present);
                    prop_assert!(service.context().mounted);
                    prop_assert!(!service.context().unmounting);
                }

                presence::State::UnmountPending => {
                    prop_assert!(!service.context().present);
                    prop_assert!(service.context().mounted);
                    prop_assert!(service.context().unmounting);
                }
            }
        }
    }

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_tooltip_state_context_invariants_hold(
        props in arb_tooltip_props(),
        steps in prop::collection::vec(arb_tooltip_step(), 0..128),
    ) {
        let mut service = Service::<tooltip::Machine>::new(props, &Env::default(), &tooltip::Messages);
        assert_tooltip_state_context_invariants(&service)?;

        for step in steps {
            match step {
                TooltipStep::Send(event) => {
                    let before_state = *service.state();
                    let before_context = service.context().clone();
                    let before_props = service.props().clone();

                    let result = service.send(event);

                    assert_tooltip_send_result_invariants(
                        &service,
                        event,
                        &result,
                        before_state,
                        &before_context,
                        &before_props,
                    )?;
                }

                TooltipStep::SetProps(props) => {
                    let before_z_index = service.context().z_index;
                    let before_open_prop = service.props().open;

                    let next_open_prop = props.open;

                    drop(service.set_props(props));

                    prop_assert_eq!(service.context().z_index, before_z_index);

                    if before_open_prop != next_open_prop
                        && let Some(open) = service.props().open
                    {
                        prop_assert_eq!(service.context().open, open);
                    }
                }
            }

            assert_tooltip_state_context_invariants(&service)?;
        }
    }
}
