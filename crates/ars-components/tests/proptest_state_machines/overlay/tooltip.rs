use super::*;

#[derive(Clone, Debug)]
enum TooltipStep {
    Send(core_tooltip::Event),
    SetProps(core_tooltip::Props),
}

fn arb_tooltip_props() -> impl Strategy<Value = core_tooltip::Props> {
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
            )| core_tooltip::Props {
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

fn arb_tooltip_event() -> impl Strategy<Value = core_tooltip::Event> {
    prop_oneof![
        Just(core_tooltip::Event::PointerEnter),
        Just(core_tooltip::Event::PointerLeave),
        Just(core_tooltip::Event::Focus),
        Just(core_tooltip::Event::Blur),
        Just(core_tooltip::Event::ContentPointerEnter),
        Just(core_tooltip::Event::ContentPointerLeave),
        Just(core_tooltip::Event::OpenTimerFired),
        Just(core_tooltip::Event::CloseTimerFired),
        Just(core_tooltip::Event::CloseOnEscape),
        Just(core_tooltip::Event::CloseOnClick),
        Just(core_tooltip::Event::CloseOnScroll),
        Just(core_tooltip::Event::Open),
        Just(core_tooltip::Event::Close),
        any::<bool>().prop_map(core_tooltip::Event::SetControlledOpen),
        Just(core_tooltip::Event::SyncProps),
        (0..=4_000u32).prop_map(core_tooltip::Event::SetZIndex),
    ]
}

fn arb_tooltip_step() -> impl Strategy<Value = TooltipStep> {
    prop_oneof![
        arb_tooltip_event().prop_map(TooltipStep::Send),
        arb_tooltip_props().prop_map(TooltipStep::SetProps),
    ]
}

const fn tooltip_event_bypasses_disabled(event: core_tooltip::Event) -> bool {
    matches!(
        event,
        core_tooltip::Event::SetControlledOpen(_)
            | core_tooltip::Event::SyncProps
            | core_tooltip::Event::CloseTimerFired
            | core_tooltip::Event::Close
            | core_tooltip::Event::SetZIndex(_)
    )
}

const fn tooltip_guard_rejects(event: core_tooltip::Event, props: &core_tooltip::Props) -> bool {
    matches!(event, core_tooltip::Event::CloseOnEscape) && !props.close_on_escape
        || matches!(event, core_tooltip::Event::CloseOnClick) && !props.close_on_click
        || matches!(event, core_tooltip::Event::CloseOnScroll) && !props.close_on_scroll
}

fn assert_tooltip_state_context_invariants(
    service: &Service<core_tooltip::Machine>,
) -> TestCaseResult {
    prop_assert_eq!(
        service.context().open,
        matches!(
            service.state(),
            core_tooltip::State::Open | core_tooltip::State::ClosePending
        )
    );
    prop_assert!(service.context().touch_auto_hide >= MIN_TOUCH_AUTO_HIDE);
    prop_assert_eq!(service.context().ids.id(), "tooltip");

    let trigger_id = service.context().ids.part("trigger");
    let content_id = service.context().ids.part("content");

    prop_assert_eq!(&service.context().trigger_id, &trigger_id);
    prop_assert_eq!(&service.context().content_id, &content_id);
    prop_assert_eq!(&service.context().trigger_id, "tooltip-trigger");
    prop_assert_eq!(&service.context().content_id, "tooltip-content");
    prop_assert_eq!(
        &service.context().hidden_description_id,
        "tooltip-content-description"
    );

    Ok(())
}

fn assert_tooltip_send_result_invariants(
    service: &Service<core_tooltip::Machine>,
    event: core_tooltip::Event,
    result: &SendResult<core_tooltip::Machine>,
    before_state: core_tooltip::State,
    before_context: &core_tooltip::Context,
    before_props: &core_tooltip::Props,
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

    if before_props.open.is_some() && !matches!(event, core_tooltip::Event::SetControlledOpen(_)) {
        prop_assert_eq!(service.context().open, before_context.open);
    }

    if let core_tooltip::Event::SetControlledOpen(open) = event {
        prop_assert_eq!(service.context().open, open);
        prop_assert_eq!(
            service.state(),
            if open {
                &core_tooltip::State::Open
            } else {
                &core_tooltip::State::Closed
            }
        );
    }

    if let core_tooltip::Event::SetZIndex(z_index) = event {
        prop_assert_eq!(service.context().z_index, Some(z_index));
    } else {
        prop_assert_eq!(service.context().z_index, before_context.z_index);
    }

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_tooltip_state_context_invariants_hold(
        props in arb_tooltip_props(),
        steps in prop::collection::vec(arb_tooltip_step(), 0..128),
    ) {
        let mut service = Service::<core_tooltip::Machine>::new(props, &Env::default(), &core_tooltip::Messages);

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
