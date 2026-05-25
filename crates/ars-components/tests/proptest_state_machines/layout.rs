use ars_components::layout::{portal, splitter};
use ars_core::{Direction, Env, KeyboardKey, Orientation, RenderMode, SendResult, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

#[derive(Clone, Debug)]
enum PortalStep {
    Send(portal::Event),
    SetProps(portal::Props),
}

fn arb_target_id() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_-]{0,12}".prop_map(String::from)
}

fn arb_portal_target() -> impl Strategy<Value = portal::PortalTarget> {
    prop_oneof![
        Just(portal::PortalTarget::PortalRoot),
        Just(portal::PortalTarget::Body),
        arb_target_id().prop_map(portal::PortalTarget::Id),
        arb_target_id().prop_map(portal::PortalTarget::ResolvedId),
    ]
}

fn arb_portal_props() -> impl Strategy<Value = portal::Props> {
    (arb_portal_target(), any::<bool>()).prop_map(|(container, ssr_inline)| {
        portal::Props::new()
            .id("portal")
            .container(container)
            .ssr_inline(ssr_inline)
    })
}

fn arb_portal_event() -> impl Strategy<Value = portal::Event> {
    prop_oneof![
        Just(portal::Event::Mount),
        Just(portal::Event::Unmount),
        arb_target_id().prop_map(portal::Event::ContainerReady),
        arb_portal_target().prop_map(portal::Event::SetContainer),
    ]
}

fn arb_portal_step() -> impl Strategy<Value = PortalStep> {
    prop_oneof![
        arb_portal_event().prop_map(PortalStep::Send),
        arb_portal_props().prop_map(PortalStep::SetProps),
    ]
}

fn assert_portal_state_context_invariants(service: &Service<portal::Machine>) -> TestCaseResult {
    prop_assert_eq!(
        service.context().mounted,
        matches!(service.state(), portal::State::Mounted)
    );
    prop_assert_eq!(service.context().render_mode, RenderMode::Client);
    prop_assert_eq!(service.context().ids.id(), "portal");

    Ok(())
}

fn assert_portal_send_result_invariants(
    service: &Service<portal::Machine>,
    event: &portal::Event,
    result: &SendResult<portal::Machine>,
    before_state: &portal::State,
    before_context: &portal::Context,
) -> TestCaseResult {
    prop_assert!(result.pending_effects.is_empty());
    prop_assert!(result.cancel_effects.is_empty());

    match event {
        portal::Event::Mount if before_state == &portal::State::Unmounted => {
            prop_assert_eq!(service.state(), &portal::State::Mounted);
            prop_assert!(service.context().mounted);
        }

        portal::Event::Unmount if before_state == &portal::State::Mounted => {
            prop_assert_eq!(service.state(), &portal::State::Unmounted);
            prop_assert!(!service.context().mounted);
        }

        portal::Event::ContainerReady(id)
            if before_state == &portal::State::Unmounted
                && before_context.container == portal::PortalTarget::Id(id.clone()) =>
        {
            prop_assert_eq!(service.state(), &portal::State::Mounted);
            prop_assert_eq!(
                service.context().container.clone(),
                portal::PortalTarget::ResolvedId(id.clone())
            );
            prop_assert!(service.context().mounted);
        }

        portal::Event::ContainerReady(id)
            if before_state == &portal::State::Mounted
                && before_context.container == portal::PortalTarget::Id(id.clone()) =>
        {
            prop_assert_eq!(service.state(), &portal::State::Mounted);
            prop_assert_eq!(
                service.context().container.clone(),
                portal::PortalTarget::ResolvedId(id.clone())
            );
            prop_assert!(service.context().mounted);
        }

        portal::Event::SetContainer(target) => {
            prop_assert_eq!(service.state(), before_state);
            prop_assert_eq!(service.context().container.clone(), target.clone());
            prop_assert_eq!(service.context().mounted, before_context.mounted);
        }

        _ => {
            prop_assert_eq!(service.state(), before_state);
            prop_assert_eq!(service.context(), before_context);
        }
    }

    Ok(())
}

proptest! {
    #![proptest_config(super::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_portal_event_sequences_preserve_invariants(
        props in arb_portal_props(),
        steps in prop::collection::vec(arb_portal_step(), 0..128),
    ) {
        let mut service = Service::<portal::Machine>::new(
            props,
            &Env::default(),
            &portal::Messages,
        );

        assert_portal_state_context_invariants(&service)?;

        for step in steps {
            match step {
                PortalStep::Send(event) => {
                    let before_state = service.state().clone();

                    let before_context = service.context().clone();

                    let result = service.send(event.clone());

                    assert_portal_send_result_invariants(
                        &service,
                        &event,
                        &result,
                        &before_state,
                        &before_context,
                    )?;
                }

                PortalStep::SetProps(props) => {
                    let before_state = service.state().clone();
                    let before_mounted = service.context().mounted;
                    let before_context_container = service.context().container.clone();
                    let before_props_container = service.props().container.clone();

                    let expected_container = props.container.clone();

                    let result = service.set_props(props);

                    prop_assert!(!result.state_changed);
                    prop_assert_eq!(service.state(), &before_state);
                    prop_assert_eq!(service.context().mounted, before_mounted);
                    prop_assert_eq!(
                        service.context().container.clone(),
                        if before_props_container == expected_container {
                            before_context_container
                        } else {
                            expected_container
                        }
                    );
                }
            }

            assert_portal_state_context_invariants(&service)?;
        }
    }
}

#[derive(Clone, Debug)]
enum SplitterStep {
    Send(splitter::Event),
    SetProps(splitter::Props),
}

fn splitter_panel(id: &'static str, default_size: f64) -> splitter::Panel {
    splitter::Panel {
        id: id.into(),
        min_size: 10.0,
        max_size: Some(90.0),
        default_size,
        collapsible: true,
        collapsed_size: 0.0,
        collapse_threshold: 0.5,
    }
}

fn arb_splitter_keyboard_event() -> impl Strategy<Value = splitter::KeyboardEvent> {
    prop_oneof![
        Just(KeyboardKey::ArrowLeft),
        Just(KeyboardKey::ArrowRight),
        Just(KeyboardKey::ArrowUp),
        Just(KeyboardKey::ArrowDown),
        Just(KeyboardKey::Home),
        Just(KeyboardKey::End),
        Just(KeyboardKey::Enter),
        Just(KeyboardKey::Space),
        Just(KeyboardKey::Escape),
    ]
    .prop_flat_map(|key| {
        any::<bool>().prop_map(move |shift| splitter::KeyboardEvent {
            key,
            shift,
            alt: false,
            ctrl: false,
            meta: false,
        })
    })
}

fn arb_splitter_props() -> impl Strategy<Value = splitter::Props> {
    (
        prop_oneof![Just(Orientation::Horizontal), Just(Orientation::Vertical)],
        prop_oneof![Just(Direction::Ltr), Just(Direction::Rtl)],
        prop_oneof![
            Just(splitter::SizeUnit::Percent),
            Just(splitter::SizeUnit::Pixels)
        ],
    )
        .prop_map(|(orientation, dir, size_unit)| {
            splitter::Props::new()
                .id("split")
                .panels(vec![
                    splitter_panel("left", 40.0),
                    splitter_panel("middle", 30.0),
                    splitter_panel("right", 30.0),
                ])
                .orientation(orientation)
                .dir(dir)
                .size_unit(size_unit)
                .default_sizes(vec![40.0, 30.0, 30.0])
        })
}

fn arb_splitter_event() -> impl Strategy<Value = splitter::Event> {
    prop_oneof![
        (0usize..4, -200.0f64..200.0)
            .prop_map(|(handle_index, pos)| { splitter::Event::DragStart { handle_index, pos } }),
        (-200.0f64..200.0).prop_map(|pos| splitter::Event::DragMove { pos }),
        Just(splitter::Event::DragEnd),
        (0usize..4, arb_splitter_keyboard_event()).prop_map(|(handle_index, event)| {
            splitter::Event::KeyDown {
                handle_index,
                event,
            }
        }),
        (0usize..4).prop_map(|handle_index| splitter::Event::HandleFocus { handle_index }),
        Just(splitter::Event::HandleBlur),
        (0usize..4).prop_map(|panel_index| splitter::Event::CollapsePanel { panel_index }),
        (0usize..4).prop_map(|panel_index| splitter::Event::ExpandPanel { panel_index }),
        prop::collection::vec(0.0f64..100.0, 0..5)
            .prop_map(|sizes| splitter::Event::SetSizes { sizes }),
    ]
}

fn arb_splitter_step() -> impl Strategy<Value = SplitterStep> {
    prop_oneof![
        arb_splitter_event().prop_map(SplitterStep::Send),
        arb_splitter_props().prop_map(SplitterStep::SetProps),
    ]
}

fn assert_splitter_invariants(service: &Service<splitter::Machine>) -> TestCaseResult {
    let sizes = service.context().sizes.get();

    prop_assert_eq!(sizes.len(), service.context().panels.len());
    prop_assert!(sizes.iter().all(|size| size.is_finite()));
    prop_assert!(service.context().keyboard_step.is_finite());
    prop_assert!(service.context().drag_scale_factor.is_finite());

    if let Some(handle_index) = service.context().focused_handle {
        prop_assert!(handle_index + 1 < sizes.len());
    }

    if let splitter::State::Dragging { handle_index } = service.state() {
        prop_assert!(*handle_index + 1 < sizes.len());
        prop_assert_eq!(service.context().drag_start_sizes.len(), sizes.len());
    }

    Ok(())
}

proptest! {
    #![proptest_config(super::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_splitter_event_sequences_preserve_invariants(
        props in arb_splitter_props(),
        steps in prop::collection::vec(arb_splitter_step(), 0..128),
    ) {
        let mut service = Service::<splitter::Machine>::new(
            props,
            &Env::default(),
            &splitter::Messages::default(),
        );

        assert_splitter_invariants(&service)?;

        for step in steps {
            match step {
                SplitterStep::Send(event) => {
                    drop(service.send(event));
                }

                SplitterStep::SetProps(props) => {
                    drop(service.set_props(props));
                }
            }

            assert_splitter_invariants(&service)?;
        }
    }
}
