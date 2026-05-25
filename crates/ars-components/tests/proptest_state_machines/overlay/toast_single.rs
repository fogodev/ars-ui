use super::*;

const TOAST_SINGLE_EFFECTS: &[core_toast_single::Effect] = &[
    core_toast_single::Effect::DurationTimer,
    core_toast_single::Effect::ExitAnimation,
    core_toast_single::Effect::AnnouncePolite,
    core_toast_single::Effect::AnnounceAssertive,
    core_toast_single::Effect::OpenChange,
];

fn arb_toast_props() -> impl Strategy<Value = core_toast_single::Props> {
    (
        arb_toast_kind(),
        prop::option::of(arb_duration(30_000)),
        any::<bool>(),
        10.0f64..=200.0,
    )
        .prop_map(|(kind, duration, show_progress, swipe_threshold)| {
            core_toast_single::Props {
                id: "toast".to_string(),
                title: Some("title".to_string()),
                description: Some("body".to_string()),
                kind,
                duration,
                show_progress,
                swipe_threshold,
            }
        })
}

fn arb_toast_event() -> impl Strategy<Value = core_toast_single::Event> {
    prop_oneof![
        Just(core_toast_single::Event::Dismiss),
        arb_duration(30_000).prop_map(|remaining| core_toast_single::Event::Pause { remaining }),
        Just(core_toast_single::Event::Resume),
        Just(core_toast_single::Event::DurationExpired),
        Just(core_toast_single::Event::AnimationComplete),
        (-200.0f64..=200.0).prop_map(core_toast_single::Event::SwipeStart),
        (-200.0f64..=200.0).prop_map(core_toast_single::Event::SwipeMove),
        (-2.0f64..=2.0, -200.0f64..=200.0).prop_map(|(velocity, offset)| {
            core_toast_single::Event::SwipeEnd { velocity, offset }
        }),
        Just(core_toast_single::Event::SyncProps),
    ]
}

#[derive(Clone, Debug)]
enum ToastSingleStep {
    Send(core_toast_single::Event),
    SetProps(core_toast_single::Props),
}

fn arb_toast_single_step() -> impl Strategy<Value = ToastSingleStep> {
    prop_oneof![
        arb_toast_event().prop_map(ToastSingleStep::Send),
        arb_toast_props().prop_map(ToastSingleStep::SetProps),
    ]
}

fn assert_toast_send_result_invariants(
    service: &Service<core_toast_single::Machine>,
    event: core_toast_single::Event,
    before_state: core_toast_single::State,
    result: &SendResult<core_toast_single::Machine>,
) -> TestCaseResult {
    // Effect-name allow-list — every emitted name is one of the documented
    // `toast::single::Effect` variants.
    for effect in &result.pending_effects {
        prop_assert!(
            TOAST_SINGLE_EFFECTS.contains(&effect.name),
            "unexpected effect name: {:?}",
            effect.name
        );

        // Per-toast machine never emits typed metadata (all intents are
        // name-only).
        prop_assert!(effect.metadata.is_none());
    }

    // Dismissed is terminal — once we have entered it, no further event
    // flips the state.
    if matches!(before_state, core_toast_single::State::Dismissed) {
        prop_assert!(!result.state_changed);
        prop_assert_eq!(service.state(), &core_toast_single::State::Dismissed);
    }

    // Open mirrors state: only Visible and Paused are "open"; Dismissing
    // and Dismissed have ctx.open == false.
    let expected_open = matches!(
        service.state(),
        core_toast_single::State::Visible | core_toast_single::State::Paused
    );

    prop_assert_eq!(service.context().open, expected_open);

    // `Context::paused` mirrors `State::Paused` exactly. This was the
    // round-6 regression: dismissing from Paused used to leave
    // `ctx.paused == true` while `state == Dismissing`. Asserting it
    // every step here catches any future arm that desyncs the flag.
    let expected_paused = matches!(service.state(), core_toast_single::State::Paused);

    prop_assert_eq!(service.context().paused, expected_paused);

    // Swipe state must not survive into Dismissing/Dismissed: those
    // are exit-animation states where adapters style/position the
    // toast based on the dismiss source, not on a half-finished drag.
    if matches!(
        service.state(),
        core_toast_single::State::Dismissing | core_toast_single::State::Dismissed
    ) {
        prop_assert!(!service.context().swiping);
        prop_assert_eq!(service.context().swipe_offset, 0.0);
    }

    // Pause atomically records the remaining-time snapshot.
    if let core_toast_single::Event::Pause { remaining } = event
        && result.state_changed
    {
        prop_assert_eq!(service.context().remaining, Some(remaining));
        prop_assert!(
            result
                .cancel_effects
                .contains(&core_toast_single::Effect::DurationTimer)
        );
    }

    // Resume re-emits `DurationTimer` on a successful Paused → Visible
    // transition **only when the toast has a finite duration**. Persistent
    // toasts (`duration: None`, typical for `Kind::Loading`) deliberately
    // skip the timer effect — emitting one would either schedule a
    // `set_timeout(None)` or auto-dismiss a toast that's supposed to stay
    // until explicit update / removal.
    if matches!(event, core_toast_single::Event::Resume) && result.state_changed {
        let names = result
            .pending_effects
            .iter()
            .map(|e| e.name)
            .collect::<Vec<_>>();

        let has_duration = service.context().duration.is_some();

        if has_duration {
            prop_assert!(names.contains(&core_toast_single::Effect::DurationTimer));
        } else {
            prop_assert!(!names.contains(&core_toast_single::Effect::DurationTimer));
        }
    }

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_toast_single_state_context_invariants_hold(
        props in arb_toast_props(),
        steps in prop::collection::vec(arb_toast_single_step(), 0..128),
    ) {
        let mut service = Service::<core_toast_single::Machine>::new(
            props,
            &Env::default(),
            &core_toast_single::Messages::default(),
        );

        // initial_effects emits Announce* always, plus DurationTimer when
        // duration is Some. Drained exactly once.
        let initial = service
            .take_initial_effects()
            .into_iter()
            .map(|effect| effect.name)
            .collect::<Vec<_>>();

        let kind = service.context().kind;

        let assertive = matches!(kind, core_toast_single::Kind::Warning | core_toast_single::Kind::Error);

        if assertive {
            prop_assert!(initial.contains(&core_toast_single::Effect::AnnounceAssertive));
        } else {
            prop_assert!(initial.contains(&core_toast_single::Effect::AnnouncePolite));
        }

        if service.context().duration.is_some() {
            prop_assert!(initial.contains(&core_toast_single::Effect::DurationTimer));
        }

        prop_assert!(service.take_initial_effects().is_empty());

        for step in steps {
            match step {
                ToastSingleStep::Send(event) => {
                    let before_state = *service.state();

                    let result = service.send(event);

                    assert_toast_send_result_invariants(&service, event, before_state, &result)?;
                }

                ToastSingleStep::SetProps(props) => {
                    drop(service.set_props(props));

                    // After set_props, context-backed prop fields must
                    // mirror props (SyncProps reapplied them). The four
                    // fields covered: title, description, kind, duration.
                    let p = service.props();
                    let c = service.context();

                    prop_assert_eq!(&c.title, &p.title);
                    prop_assert_eq!(&c.description, &p.description);
                    prop_assert_eq!(c.kind, p.kind);
                    prop_assert_eq!(c.duration, p.duration);
                }
            }
        }
    }
}
