use ars_components::layout::portal;
use ars_core::{Env, RenderMode, SendResult, Service};
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
        arb_target_id().prop_map(portal::PortalTarget::Ref),
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
                portal::PortalTarget::Ref(id.clone())
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
    #![proptest_config(ProptestConfig::with_cases(
        std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000)
    ))]

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
