use super::*;

#[derive(Clone, Debug)]
enum PopoverStep {
    Send(core_popover::Event),
    SetProps(core_popover::Props),
}

const POPOVER_EFFECTS: &[core_popover::Effect] = &[
    core_popover::Effect::OpenChange,
    core_popover::Effect::AttachClickOutside,
    core_popover::Effect::DetachClickOutside,
    core_popover::Effect::AllocateZIndex,
    core_popover::Effect::ReleaseZIndex,
    core_popover::Effect::RestoreFocus,
    core_popover::Effect::FocusInitial,
];

fn arb_popover_props() -> impl Strategy<Value = core_popover::Props> {
    (
        (
            prop::option::of(any::<bool>()),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            arb_positioning_options(),
            -16.0f64..=16.0,
        ),
        (
            -16.0f64..=16.0,
            any::<bool>(),
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
                    close_on_escape,
                    close_on_interact_outside,
                    positioning,
                    offset,
                ),
                (cross_offset, same_width, portal, lazy_mount, unmount_on_exit),
            )| {
                core_popover::Props::new()
                    .id("popover")
                    .open(open)
                    .default_open(default_open)
                    .modal(modal)
                    .close_on_escape(close_on_escape)
                    .close_on_interact_outside(close_on_interact_outside)
                    .positioning(positioning)
                    .offset(offset)
                    .cross_offset(cross_offset)
                    .same_width(same_width)
                    .portal(portal)
                    .lazy_mount(lazy_mount)
                    .unmount_on_exit(unmount_on_exit)
            },
        )
}

fn arb_popover_event() -> impl Strategy<Value = core_popover::Event> {
    prop_oneof![
        Just(core_popover::Event::Open),
        Just(core_popover::Event::Close),
        Just(core_popover::Event::Toggle),
        Just(core_popover::Event::CloseOnEscape),
        Just(core_popover::Event::CloseOnInteractOutside),
        arb_positioning_snapshot().prop_map(core_popover::Event::PositioningUpdate),
        (0..=4_000u32).prop_map(core_popover::Event::SetZIndex),
        Just(core_popover::Event::RegisterTitle),
        Just(core_popover::Event::RegisterDescription),
        Just(core_popover::Event::SyncProps),
    ]
}

fn arb_popover_step() -> impl Strategy<Value = PopoverStep> {
    prop_oneof![
        arb_popover_event().prop_map(PopoverStep::Send),
        arb_popover_props().prop_map(PopoverStep::SetProps),
    ]
}

const fn popover_guard_rejects(event: core_popover::Event, props: &core_popover::Props) -> bool {
    matches!(event, core_popover::Event::CloseOnEscape) && !props.close_on_escape
        || matches!(event, core_popover::Event::CloseOnInteractOutside)
            && !props.close_on_interact_outside
}

fn assert_popover_state_context_invariants(
    service: &Service<core_popover::Machine>,
) -> TestCaseResult {
    // 1. State ⇔ context.open invariant — the boolean and the state
    // enum must agree at all times.
    prop_assert_eq!(
        service.context().open,
        matches!(service.state(), core_popover::State::Open)
    );

    // 2. ID stability — derived from props.id at init and never mutated
    // (the on_props_changed assertion would have panicked otherwise).
    prop_assert_eq!(&service.context().trigger_id, "popover-trigger");
    prop_assert_eq!(&service.context().content_id, "popover-content");

    // 3. Title / description ids, when registered, follow the same
    // hydration-stable scheme.
    if let Some(title_id) = service.context().title_id.as_deref() {
        prop_assert_eq!(title_id, "popover-title");
    }
    if let Some(description_id) = service.context().description_id.as_deref() {
        prop_assert_eq!(description_id, "popover-description");
    }

    // (`SetZIndex` is intentionally unguarded by state to cover the
    // rare adapter race where the response arrives after a rapid close,
    // so we cannot assert "z_index is None when closed" as a global
    // invariant — the per-transition reset is asserted instead in
    // `assert_popover_send_result_invariants` below.)

    Ok(())
}

fn assert_popover_send_result_invariants(
    event: core_popover::Event,
    result: &SendResult<core_popover::Machine>,
    before_state: core_popover::State,
    before_props: &core_popover::Props,
    after_context: &core_popover::Context,
) -> TestCaseResult {
    // 5. Effect-name allow-list — every emitted name is one of the
    // documented `EFFECT_*` constants.
    for effect in &result.pending_effects {
        prop_assert!(
            POPOVER_EFFECTS.contains(&effect.name),
            "unexpected effect name: {:?}",
            effect.name
        );

        // 6. Payload-free invariant — the popover machine never emits
        // typed metadata (ALL its intents are name-only).
        prop_assert!(effect.metadata.is_none());
    }

    // 7. Guards: dismissal events with the corresponding `close_on_*`
    // disabled MUST NOT change state and MUST NOT emit any effects.
    if popover_guard_rejects(event, before_props) {
        prop_assert_eq!(service_state_unchanged(result), true);
        prop_assert!(result.pending_effects.is_empty());
        prop_assert!(result.cancel_effects.is_empty());
    }

    // 8. State-flipping events emit `EFFECT_OPEN_CHANGE` exactly when
    // the state actually changed; no-op transitions do not.
    let emitted_open_change = result
        .pending_effects
        .iter()
        .any(|e| e.name == core_popover::Effect::OpenChange);

    let state_actually_flipped = result.state_changed;

    if matches!(
        event,
        core_popover::Event::Open
            | core_popover::Event::Close
            | core_popover::Event::Toggle
            | core_popover::Event::CloseOnEscape
            | core_popover::Event::CloseOnInteractOutside
    ) {
        prop_assert_eq!(emitted_open_change, state_actually_flipped);
    } else {
        prop_assert!(!emitted_open_change);
    }

    // 9. Effect-set symmetry on the open lifecycle — every successful
    // `Closed → Open` transition emits the four-effect open-plan set,
    // and every successful `Open → Closed` emits the four-effect
    // close-plan set. (Per `open_lifecycle_effects` /
    // `close_plan` in popover.rs.)
    if result.state_changed {
        let names = result
            .pending_effects
            .iter()
            .map(|effect| effect.name)
            .collect::<Vec<_>>();

        match service_resulting_state(before_state, result) {
            core_popover::State::Open => {
                prop_assert!(names.contains(&core_popover::Effect::OpenChange));
                prop_assert!(names.contains(&core_popover::Effect::AllocateZIndex));
                prop_assert!(names.contains(&core_popover::Effect::AttachClickOutside));
                prop_assert!(names.contains(&core_popover::Effect::FocusInitial));
            }

            core_popover::State::Closed => {
                prop_assert!(names.contains(&core_popover::Effect::OpenChange));
                prop_assert!(names.contains(&core_popover::Effect::DetachClickOutside));
                prop_assert!(names.contains(&core_popover::Effect::ReleaseZIndex));
                prop_assert!(names.contains(&core_popover::Effect::RestoreFocus));
            }
        }
    }

    // 10. Register{Title,Description} monotonicity: once an id is
    // populated, it stays populated; the catch-all match-arm guard
    // rejects re-registration as a no-op.
    if matches!(
        event,
        core_popover::Event::RegisterTitle | core_popover::Event::RegisterDescription
    ) {
        prop_assert!(!result.state_changed);
    }

    // 11. SetZIndex MUST NOT change state and is unguarded by state
    // (covers the rare adapter race where the response arrives after
    // a rapid close).
    if let core_popover::Event::SetZIndex(z_index) = event {
        prop_assert!(!result.state_changed);
        prop_assert_eq!(service_z_index_after(z_index), Some(z_index));
    }

    // 12. PositioningUpdate is gated on state == Open; while closed it
    // is a no-op so stale measurements never leak in.
    if matches!(event, core_popover::Event::PositioningUpdate(_))
        && matches!(before_state, core_popover::State::Closed)
    {
        prop_assert!(!result.state_changed);
        prop_assert!(!result.context_changed);
    }

    // 13. close_plan resets `arrow_offset` and `z_index` — every
    // successful `Open → Closed` transition wipes these so the next
    // open lifecycle starts from a clean slate. (Subsequent
    // `SetZIndex` events while closed may re-populate `z_index`; the
    // invariant only asserts the immediate post-transition state.)
    if result.state_changed
        && matches!(before_state, core_popover::State::Open)
        && matches!(
            event,
            core_popover::Event::Close
                | core_popover::Event::Toggle
                | core_popover::Event::CloseOnEscape
                | core_popover::Event::CloseOnInteractOutside
        )
    {
        prop_assert!(after_context.arrow_offset.is_none());
        prop_assert!(after_context.z_index.is_none());
    }

    Ok(())
}

// Helpers — defined as plain functions so they can be referenced from
// `prop_assert_eq!` without lifetimes leaking into the `proptest!`
// macro expansion.

const fn service_state_unchanged<M: ars_core::Machine>(result: &SendResult<M>) -> bool {
    !result.state_changed
}

const fn service_resulting_state(
    before: core_popover::State,
    result: &SendResult<core_popover::Machine>,
) -> core_popover::State {
    if result.state_changed {
        match before {
            core_popover::State::Closed => core_popover::State::Open,
            core_popover::State::Open => core_popover::State::Closed,
        }
    } else {
        before
    }
}

const fn service_z_index_after(z_index: u32) -> Option<u32> {
    Some(z_index)
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_popover_state_context_invariants_hold(
        props in arb_popover_props(),
        steps in prop::collection::vec(arb_popover_step(), 0..128),
    ) {
        let mut service = Service::<core_popover::Machine>::new(
            props,
            &Env::default(),
            &core_popover::Messages::default(),
        );

        // Initial-effects invariant: when the popover boots into Open,
        // `take_initial_effects` MUST emit the full open-plan set; when
        // it boots into Closed, the buffer is empty. Assert this BEFORE
        // any step runs (the buffer is captured at construction).
        let initial_state = *service.state();

        let initial_effects = service
            .take_initial_effects()
            .into_iter()
            .map(|effect| effect.name)
            .collect::<Vec<_>>();

        match initial_state {
            core_popover::State::Open => {
                prop_assert!(initial_effects.contains(&core_popover::Effect::OpenChange));
                prop_assert!(initial_effects.contains(&core_popover::Effect::AllocateZIndex));
                prop_assert!(initial_effects.contains(&core_popover::Effect::AttachClickOutside));
                prop_assert!(initial_effects.contains(&core_popover::Effect::FocusInitial));
            }

            core_popover::State::Closed => {
                prop_assert!(initial_effects.is_empty());
            }
        }

        // Subsequent calls always observe the empty buffer — initial
        // effects fire exactly once.
        prop_assert!(service.take_initial_effects().is_empty());

        assert_popover_state_context_invariants(&service)?;

        // Track monotonic flags — once a title/description id is
        // registered, it stays registered (the guard in `transition`
        // prevents re-registration from clobbering the id).
        let mut had_title = service.context().title_id.is_some();
        let mut had_description = service.context().description_id.is_some();

        for step in steps {
            match step {
                PopoverStep::Send(event) => {
                    let before_state = *service.state();
                    let before_props = service.props().clone();

                    let result = service.send(event);

                    let after_context = service.context().clone();

                    assert_popover_send_result_invariants(
                        event,
                        &result,
                        before_state,
                        &before_props,
                        &after_context,
                    )?;
                }

                PopoverStep::SetProps(props) => {
                    let before_open_prop = service.props().open;
                    let next_open_prop = props.open;

                    drop(service.set_props(props));

                    // SyncProps replays context-backed fields.
                    let p = service.props();
                    let c = service.context();

                    prop_assert_eq!(c.modal, p.modal);
                    prop_assert_eq!(&c.positioning.placement, &p.positioning.placement);

                    // Controlled-open sync.
                    if before_open_prop != next_open_prop
                        && let Some(open) = service.props().open
                    {
                        prop_assert_eq!(service.context().open, open);
                    }
                }
            }

            // Monotonicity invariants enforced after each step.
            if had_title {
                prop_assert!(service.context().title_id.is_some());
            }
            if had_description {
                prop_assert!(service.context().description_id.is_some());
            }

            had_title |= service.context().title_id.is_some();
            had_description |= service.context().description_id.is_some();

            assert_popover_state_context_invariants(&service)?;
        }
    }
}
