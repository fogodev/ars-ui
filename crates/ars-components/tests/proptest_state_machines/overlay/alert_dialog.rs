use super::*;

#[derive(Clone, Debug)]
enum AlertDialogStep {
    Send(core_alert_dialog::Event),
    SetProps(core_alert_dialog::Props),
}

const ALERT_DIALOG_EFFECTS: &[core_alert_dialog::Effect] = &[
    core_alert_dialog::Effect::OpenChange,
    core_alert_dialog::Effect::FocusInitial,
    core_alert_dialog::Effect::FocusFirstTabbable,
    core_alert_dialog::Effect::ScrollLockAcquire,
    core_alert_dialog::Effect::ScrollLockRelease,
    core_alert_dialog::Effect::SetBackgroundInert,
    core_alert_dialog::Effect::RemoveBackgroundInert,
    core_alert_dialog::Effect::RestoreFocus,
];

fn arb_alert_dialog_props() -> impl Strategy<Value = core_alert_dialog::Props> {
    (
        (
            prop::option::of(any::<bool>()),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            arb_focus_target(),
        ),
        (
            arb_focus_target(),
            arb_dialog_role(),
            0_u8..=10,
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
        ),
    )
        .prop_map(
            |(
                (
                    open,
                    default_open,
                    modal,
                    close_on_backdrop,
                    close_on_escape,
                    prevent_scroll,
                    restore_focus,
                    initial_focus,
                ),
                (final_focus, role, title_level, lazy_mount, unmount_on_exit, is_destructive),
            )| {
                core_alert_dialog::Props::new()
                    .id("alert")
                    .open(open)
                    .default_open(default_open)
                    .modal(modal)
                    .close_on_backdrop(close_on_backdrop)
                    .close_on_escape(close_on_escape)
                    .prevent_scroll(prevent_scroll)
                    .restore_focus(restore_focus)
                    .initial_focus(initial_focus)
                    .final_focus(final_focus)
                    .role(role)
                    .title_level(title_level)
                    .lazy_mount(lazy_mount)
                    .unmount_on_exit(unmount_on_exit)
                    .is_destructive(is_destructive)
            },
        )
}

fn arb_alert_dialog_event() -> impl Strategy<Value = core_alert_dialog::Event> {
    prop_oneof![
        Just(core_alert_dialog::Event::Open),
        Just(core_alert_dialog::Event::Close),
        Just(core_alert_dialog::Event::Toggle),
        Just(core_alert_dialog::Event::CloseOnBackdropClick),
        Just(core_alert_dialog::Event::CloseOnEscape),
        Just(core_alert_dialog::Event::RegisterTitle),
        Just(core_alert_dialog::Event::RegisterDescription),
        Just(core_alert_dialog::Event::SyncProps),
    ]
}

fn arb_alert_dialog_step() -> impl Strategy<Value = AlertDialogStep> {
    prop_oneof![
        arb_alert_dialog_event().prop_map(AlertDialogStep::Send),
        arb_alert_dialog_props().prop_map(AlertDialogStep::SetProps),
    ]
}

fn assert_alert_dialog_state_context_invariants(
    service: &Service<core_alert_dialog::Machine>,
) -> TestCaseResult {
    prop_assert_eq!(
        service.context().open,
        matches!(service.state(), core_alert_dialog::State::Open)
    );

    prop_assert_eq!(service.context().ids.part("trigger"), "alert-trigger");
    prop_assert_eq!(service.context().ids.part("content"), "alert-content");
    prop_assert_eq!(service.context().ids.part("title"), "alert-title");
    prop_assert_eq!(
        service.context().ids.part("description"),
        "alert-description"
    );

    Ok(())
}

fn assert_alert_dialog_send_result_invariants(
    event: core_alert_dialog::Event,
    result: &SendResult<core_alert_dialog::Machine>,
    before_context: &core_alert_dialog::Context,
) -> TestCaseResult {
    for effect in &result.pending_effects {
        prop_assert!(
            ALERT_DIALOG_EFFECTS.contains(&effect.name),
            "unexpected effect name: {:?}",
            effect.name
        );
        prop_assert!(effect.metadata.is_none());
    }

    if matches!(event, core_alert_dialog::Event::SyncProps) {
        prop_assert!(!result.state_changed);
        prop_assert!(result.pending_effects.is_empty());
    }

    if matches!(event, core_alert_dialog::Event::CloseOnBackdropClick)
        && !before_context.close_on_backdrop
    {
        prop_assert_eq!(result.state_changed, false);
    }

    if matches!(event, core_alert_dialog::Event::CloseOnEscape) && !before_context.close_on_escape {
        prop_assert_eq!(result.state_changed, false);
    }

    if matches!(
        event,
        core_alert_dialog::Event::RegisterTitle | core_alert_dialog::Event::RegisterDescription
    ) {
        prop_assert!(!result.state_changed);
    }

    let emitted_open_change = result
        .pending_effects
        .iter()
        .any(|effect| effect.name == core_alert_dialog::Effect::OpenChange);

    if matches!(
        event,
        core_alert_dialog::Event::Open
            | core_alert_dialog::Event::Close
            | core_alert_dialog::Event::Toggle
            | core_alert_dialog::Event::CloseOnBackdropClick
            | core_alert_dialog::Event::CloseOnEscape
    ) {
        prop_assert_eq!(emitted_open_change, result.state_changed);
    } else {
        prop_assert!(!emitted_open_change);
    }

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_alert_dialog_state_context_invariants_hold(
        props in arb_alert_dialog_props(),
        steps in prop::collection::vec(arb_alert_dialog_step(), 0..128),
    ) {
        let mut service = Service::<core_alert_dialog::Machine>::new(
            props,
            &Env::default(),
            &core_alert_dialog::Messages::default(),
        );

        assert_alert_dialog_state_context_invariants(&service)?;

        let mut had_title = service.context().has_title;
        let mut had_description = service.context().has_description;

        for step in steps {
            match step {
                AlertDialogStep::Send(event) => {
                    let before_context = service.context().clone();

                    let result = service.send(event);

                    assert_alert_dialog_send_result_invariants(event, &result, &before_context)?;
                }

                AlertDialogStep::SetProps(props) => {
                    let before_open_prop = service.props().open;
                    let next_open_prop = props.open;

                    drop(service.set_props(props));

                    let p = service.props();
                    let c = service.context();

                    prop_assert_eq!(c.modal, p.modal);
                    prop_assert_eq!(c.close_on_backdrop, p.close_on_backdrop);
                    prop_assert_eq!(c.close_on_escape, p.close_on_escape);
                    prop_assert_eq!(c.prevent_scroll, p.prevent_scroll);
                    prop_assert_eq!(c.restore_focus, p.restore_focus);
                    prop_assert_eq!(c.initial_focus, p.initial_focus);
                    prop_assert_eq!(c.final_focus, p.final_focus);
                    prop_assert_eq!(c.role, p.role);

                    if before_open_prop != next_open_prop
                        && let Some(open) = service.props().open
                    {
                        prop_assert_eq!(service.context().open, open);
                    }
                }
            }

            if had_title {
                prop_assert!(service.context().has_title);
            }

            if had_description {
                prop_assert!(service.context().has_description);
            }

            had_title |= service.context().has_title;
            had_description |= service.context().has_description;

            assert_alert_dialog_state_context_invariants(&service)?;
        }
    }

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_alert_dialog_default_guards_hold(
        initial_open in any::<bool>(),
        events in prop::collection::vec(
            prop_oneof![
                Just(core_alert_dialog::Event::CloseOnBackdropClick),
                Just(core_alert_dialog::Event::CloseOnEscape),
            ],
            1..64,
        ),
    ) {
        let mut service = Service::<core_alert_dialog::Machine>::new(
            core_alert_dialog::Props::new()
                .id("alert")
                .default_open(initial_open),
            &Env::default(),
            &core_alert_dialog::Messages::default(),
        );

        for event in events {
            let result = service.send(event);

            prop_assert!(!result.state_changed);
            prop_assert_eq!(service.state(), if initial_open {
                &core_alert_dialog::State::Open
            } else {
                &core_alert_dialog::State::Closed
            });
        }
    }
}
