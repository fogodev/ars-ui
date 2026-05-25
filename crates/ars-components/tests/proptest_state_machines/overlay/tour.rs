use super::*;

#[derive(Clone, Debug)]
enum TourStep {
    Send(core_tour::Event),
    SetProps(core_tour::Props),
}

const TOUR_EFFECTS: &[core_tour::Effect] = &[
    core_tour::Effect::OpenChange,
    core_tour::Effect::StepChange,
    core_tour::Effect::AllocateZIndex,
    core_tour::Effect::ReleaseZIndex,
    core_tour::Effect::AttachOverlayClick,
    core_tour::Effect::DetachOverlayClick,
    core_tour::Effect::FocusStepContent,
    core_tour::Effect::ScrollTargetIntoView,
    core_tour::Effect::PositionStepContent,
    core_tour::Effect::MeasureSpotlight,
];

fn arb_tour_step_type() -> impl Strategy<Value = core_tour::StepType> {
    prop_oneof![
        Just(core_tour::StepType::Tooltip),
        Just(core_tour::StepType::Dialog),
        Just(core_tour::StepType::Floating),
        Just(core_tour::StepType::Wait),
    ]
}

fn arb_tour_step_def() -> impl Strategy<Value = core_tour::Step> {
    (
        prop::option::of("[a-z]{1,8}".prop_map(|target| format!("#{target}"))),
        "[a-z]{1,12}",
        "[a-z ]{0,24}",
        arb_tour_step_type(),
        arb_placement(),
        0.0f64..=32.0,
        0.0f64..=32.0,
    )
        .prop_map(
            |(target, title, content, step_type, placement, spotlight_radius, spotlight_offset)| {
                core_tour::Step {
                    target,
                    title,
                    content,
                    step_type,
                    placement,
                    spotlight_radius,
                    spotlight_offset,
                }
            },
        )
}

fn arb_spotlight_snapshot() -> impl Strategy<Value = core_tour::SpotlightSnapshot> {
    (
        -200.0f64..=200.0,
        -200.0f64..=200.0,
        0.0f64..=600.0,
        0.0f64..=600.0,
        0.0f64..=32.0,
        0.0f64..=32.0,
    )
        .prop_map(
            |(x, y, width, height, offset, radius)| core_tour::SpotlightSnapshot {
                rect: core_tour::SpotlightRect {
                    x,
                    y,
                    width,
                    height,
                },
                offset,
                radius,
            },
        )
}

fn arb_tour_props() -> impl Strategy<Value = core_tour::Props> {
    (
        prop::collection::vec(arb_tour_step_def(), 0..8),
        prop::option::of(any::<bool>()),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(
                steps,
                open,
                default_open,
                auto_start,
                close_on_overlay_click,
                close_on_escape,
                keyboard_navigation,
                lazy_mount,
                unmount_on_exit,
            )| {
                core_tour::Props::new()
                    .id("tour")
                    .steps(steps)
                    .open(open)
                    .default_open(default_open)
                    .auto_start(auto_start)
                    .close_on_overlay_click(close_on_overlay_click)
                    .close_on_escape(close_on_escape)
                    .keyboard_navigation(keyboard_navigation)
                    .lazy_mount(lazy_mount)
                    .unmount_on_exit(unmount_on_exit)
            },
        )
}

fn arb_tour_event() -> impl Strategy<Value = core_tour::Event> {
    prop_oneof![
        Just(core_tour::Event::Start),
        Just(core_tour::Event::NextStep),
        Just(core_tour::Event::PrevStep),
        (0usize..12).prop_map(core_tour::Event::GoToStep),
        Just(core_tour::Event::Skip),
        Just(core_tour::Event::Complete),
        Just(core_tour::Event::Dismiss),
        (0usize..12, arb_tour_step_def())
            .prop_map(|(index, step)| core_tour::Event::AddStep { index, step }),
        (0usize..12).prop_map(core_tour::Event::RemoveStep),
        (0usize..12, arb_tour_step_def())
            .prop_map(|(index, step)| core_tour::Event::UpdateStep { index, step }),
        (0usize..12).prop_map(core_tour::Event::StepChange),
        any::<bool>().prop_map(|is_keyboard| core_tour::Event::Focus { is_keyboard }),
        Just(core_tour::Event::Blur),
        (0..=4_000u32).prop_map(core_tour::Event::SetZIndex),
        arb_positioning_snapshot().prop_map(core_tour::Event::PositioningUpdate),
        arb_spotlight_snapshot().prop_map(core_tour::Event::SpotlightUpdate),
        Just(core_tour::Event::SyncProps),
    ]
}

fn arb_tour_step() -> impl Strategy<Value = TourStep> {
    prop_oneof![
        arb_tour_event().prop_map(TourStep::Send),
        arb_tour_props().prop_map(TourStep::SetProps),
    ]
}

fn assert_tour_invariants(service: &Service<core_tour::Machine>) -> TestCaseResult {
    prop_assert_eq!(
        service.context().open,
        matches!(service.state(), core_tour::State::Active { .. })
    );

    if let core_tour::State::Active { step_index } = service.state() {
        prop_assert!(*step_index < service.context().total_steps);
        prop_assert_eq!(*step_index, service.context().current_step);
    }

    if matches!(
        service.state(),
        core_tour::State::Inactive | core_tour::State::Completed
    ) {
        prop_assert!(!service.context().open);
    }

    Ok(())
}

fn assert_tour_result_invariants(
    before_state: core_tour::State,
    before_context: &core_tour::Context,
    event: &core_tour::Event,
    result: &SendResult<core_tour::Machine>,
    service: &Service<core_tour::Machine>,
) -> TestCaseResult {
    for effect in &result.pending_effects {
        prop_assert!(
            TOUR_EFFECTS.contains(&effect.name),
            "unexpected tour effect: {:?}",
            effect.name
        );
        prop_assert!(effect.metadata.is_none());
    }

    if matches!(event, core_tour::Event::GoToStep(index) if *index >= before_context.total_steps) {
        prop_assert!(!result.state_changed);
    }

    if result.state_changed
        && matches!(before_state, core_tour::State::Active { .. })
        && matches!(
            event,
            core_tour::Event::Skip
                | core_tour::Event::Dismiss
                | core_tour::Event::Complete
                | core_tour::Event::NextStep
        )
        && !matches!(service.state(), core_tour::State::Active { .. })
    {
        prop_assert!(service.context().z_index.is_none());
        prop_assert!(service.context().spotlight.is_none());
    }

    if matches!(
        event,
        core_tour::Event::PositioningUpdate(_) | core_tour::Event::SpotlightUpdate(_)
    ) && !matches!(before_state, core_tour::State::Active { .. })
    {
        prop_assert!(!result.context_changed);
        prop_assert!(!result.state_changed);
    }

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_tour_state_context_invariants_hold(
        props in arb_tour_props(),
        steps in prop::collection::vec(arb_tour_step(), 0..128),
    ) {
        let mut service = Service::<core_tour::Machine>::new(
            props,
            &Env::default(),
            &core_tour::Messages::default(),
        );

        let initial_effects = service
            .take_initial_effects()
            .into_iter()
            .map(|effect| effect.name)
            .collect::<Vec<_>>();

        if matches!(service.state(), core_tour::State::Active { .. }) {
            prop_assert!(initial_effects.contains(&core_tour::Effect::OpenChange));
            prop_assert!(initial_effects.contains(&core_tour::Effect::StepChange));
            prop_assert!(initial_effects.contains(&core_tour::Effect::AllocateZIndex));
        } else {
            prop_assert!(initial_effects.is_empty());
        }

        assert_tour_invariants(&service)?;

        for step in steps {
            match step {
                TourStep::Send(event) => {
                    let before_state = *service.state();
                    let before_context = service.context().clone();

                    let result = service.send(event.clone());

                    assert_tour_result_invariants(
                        before_state,
                        &before_context,
                        &event,
                        &result,
                        &service,
                    )?;
                }

                TourStep::SetProps(props) => {
                    drop(service.set_props(props));
                }
            }

            assert_tour_invariants(&service)?;
        }
    }
}
