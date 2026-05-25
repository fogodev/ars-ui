use super::*;

#[derive(Clone, Debug)]
enum DrawerStep {
    Send(core_drawer::Event),
    SetProps(core_drawer::Props),
}

const DRAWER_EFFECTS: &[core_drawer::Effect] = &[
    core_drawer::Effect::OpenChange,
    core_drawer::Effect::FocusInitial,
    core_drawer::Effect::FocusFirstTabbable,
    core_drawer::Effect::ScrollLockAcquire,
    core_drawer::Effect::ScrollLockRelease,
    core_drawer::Effect::SetBackgroundInert,
    core_drawer::Effect::RemoveBackgroundInert,
    core_drawer::Effect::RestoreFocus,
    core_drawer::Effect::AllocateZIndex,
    core_drawer::Effect::ReleaseZIndex,
    core_drawer::Effect::SnapChange,
];

fn arb_drawer_placement() -> impl Strategy<Value = core_drawer::Placement> {
    prop_oneof![
        Just(core_drawer::Placement::Top),
        Just(core_drawer::Placement::Bottom),
        Just(core_drawer::Placement::Left),
        Just(core_drawer::Placement::Right),
        Just(core_drawer::Placement::Start),
        Just(core_drawer::Placement::End),
    ]
}

fn arb_drawer_snap_points() -> impl Strategy<Value = Option<Vec<f64>>> {
    prop_oneof![
        Just(None),
        prop::collection::vec(0.0f64..=1.0, 1..=5).prop_map(Some),
    ]
}

fn arb_drawer_props() -> impl Strategy<Value = core_drawer::Props> {
    (
        (
            prop::option::of(any::<bool>()),
            any::<bool>(),
            arb_drawer_placement(),
            arb_direction(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
        ),
        (
            any::<bool>(),
            any::<bool>(),
            arb_focus_target(),
            arb_focus_target(),
            0_u8..=10,
            arb_drawer_snap_points(),
            0_usize..=8,
        ),
    )
        .prop_map(
            |(
                (open, default_open, placement, dir, modal, close_on_backdrop, close_on_escape),
                (
                    prevent_scroll,
                    restore_focus,
                    initial_focus,
                    final_focus,
                    title_level,
                    snap_points,
                    default_snap_index,
                ),
            )| core_drawer::Props {
                id: "drawer".to_string(),
                open,
                default_open,
                placement,
                modal,
                close_on_backdrop,
                close_on_escape,
                prevent_scroll,
                restore_focus,
                initial_focus,
                final_focus,
                dir,
                title_level,
                snap_points,
                default_snap_index,
                on_open_change: None,
                lazy_mount: false,
                unmount_on_exit: false,
                on_escape_key_down: None,
                on_interact_outside: None,
            },
        )
}

fn arb_drawer_event() -> impl Strategy<Value = core_drawer::Event> {
    prop_oneof![
        Just(core_drawer::Event::Open),
        Just(core_drawer::Event::Close),
        Just(core_drawer::Event::Toggle),
        (0.0f64..=1.0).prop_map(core_drawer::Event::DragStart),
        (0.0f64..=1.0).prop_map(core_drawer::Event::DragMove),
        (0.0f64..=1.0, -2.0f64..=2.0)
            .prop_map(|(offset, velocity)| { core_drawer::Event::DragEnd { offset, velocity } }),
        (0_usize..=8).prop_map(core_drawer::Event::SnapTo),
        (0_u64..=8, 0..=4_000u32).prop_map(|(request_id, z_index)| {
            core_drawer::Event::SetZIndex {
                request_id,
                z_index,
            }
        }),
        Just(core_drawer::Event::CloseOnBackdropClick),
        Just(core_drawer::Event::CloseOnEscape),
        Just(core_drawer::Event::RegisterTitle),
        Just(core_drawer::Event::UnregisterTitle),
        Just(core_drawer::Event::RegisterDescription),
        Just(core_drawer::Event::UnregisterDescription),
        Just(core_drawer::Event::SyncProps),
    ]
}

fn arb_drawer_step() -> impl Strategy<Value = DrawerStep> {
    prop_oneof![
        arb_drawer_event().prop_map(DrawerStep::Send),
        arb_drawer_props().prop_map(DrawerStep::SetProps),
    ]
}

fn assert_drawer_state_context_invariants(
    service: &Service<core_drawer::Machine>,
) -> TestCaseResult {
    prop_assert_eq!(
        service.context().open,
        matches!(
            service.state(),
            core_drawer::State::Open | core_drawer::State::Dragging(_)
        )
    );

    let expected_direction = service
        .context()
        .dir
        .resolve(ars_core::ResolvedDirection::Ltr);

    prop_assert_eq!(
        service.context().resolved_placement,
        service.context().placement.to_physical(expected_direction)
    );

    if service.context().snap_points.is_empty() {
        prop_assert_eq!(service.context().current_snap, 0);
    } else {
        prop_assert!(service.context().current_snap < service.context().snap_points.len());
    }

    prop_assert_eq!(service.context().ids.part("trigger"), "drawer-trigger");
    prop_assert_eq!(service.context().ids.part("content"), "drawer-content");
    prop_assert_eq!(service.context().ids.part("title"), "drawer-title");
    prop_assert_eq!(
        service.context().ids.part("description"),
        "drawer-description"
    );

    Ok(())
}

fn assert_drawer_send_result_invariants(
    event: &core_drawer::Event,
    result: &SendResult<core_drawer::Machine>,
    before_context: &core_drawer::Context,
) -> TestCaseResult {
    for effect in &result.pending_effects {
        prop_assert!(
            DRAWER_EFFECTS.contains(&effect.name),
            "unexpected drawer effect: {:?}",
            effect.name
        );
        prop_assert!(effect.metadata.is_none());
    }

    if matches!(event, core_drawer::Event::SyncProps) {
        prop_assert!(!result.state_changed);
    }

    if matches!(event, core_drawer::Event::CloseOnBackdropClick)
        && !before_context.close_on_backdrop
    {
        prop_assert!(!result.state_changed);
    }

    if matches!(event, core_drawer::Event::CloseOnEscape) && !before_context.close_on_escape {
        prop_assert!(!result.state_changed);
    }

    if matches!(event, core_drawer::Event::SetZIndex { .. }) {
        prop_assert!(!result.state_changed);
    }

    let emitted_open_change = result
        .pending_effects
        .iter()
        .any(|effect| effect.name == core_drawer::Effect::OpenChange);

    if matches!(
        event,
        core_drawer::Event::Open
            | core_drawer::Event::Close
            | core_drawer::Event::Toggle
            | core_drawer::Event::CloseOnBackdropClick
            | core_drawer::Event::CloseOnEscape
    ) {
        prop_assert_eq!(emitted_open_change, result.state_changed);
    } else if !matches!(event, core_drawer::Event::DragEnd { .. }) {
        prop_assert!(!emitted_open_change);
    }

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_drawer_state_context_invariants_hold(
        props in arb_drawer_props(),
        steps in prop::collection::vec(arb_drawer_step(), 0..128),
    ) {
        let mut service = Service::<core_drawer::Machine>::new(
            props,
            &Env::default(),
            &core_drawer::Messages::default(),
        );

        assert_drawer_state_context_invariants(&service)?;

        for step in steps {
            match step {
                DrawerStep::Send(event) => {
                    let before_context = service.context().clone();

                    let result = service.send(event.clone());

                    assert_drawer_send_result_invariants(&event, &result, &before_context)?;
                }

                DrawerStep::SetProps(props) => {
                    drop(service.set_props(props));

                    let p = service.props();
                    let c = service.context();

                    prop_assert_eq!(c.modal, p.modal);
                    prop_assert_eq!(c.placement, p.placement);
                    prop_assert_eq!(c.dir, p.dir);
                    prop_assert_eq!(c.close_on_backdrop, p.close_on_backdrop);
                    prop_assert_eq!(c.close_on_escape, p.close_on_escape);
                    prop_assert_eq!(c.prevent_scroll, p.prevent_scroll);
                    prop_assert_eq!(c.restore_focus, p.restore_focus);
                    prop_assert_eq!(c.initial_focus, p.initial_focus);
                    prop_assert_eq!(c.final_focus, p.final_focus);
                }
            }

            assert_drawer_state_context_invariants(&service)?;
        }
    }
}
