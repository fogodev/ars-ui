use core::time::Duration;

use ars_a11y::FocusTarget;
use ars_components::overlay::{
    dialog, popover,
    positioning::{ArrowOffset, Offset, Placement, PositioningOptions, PositioningSnapshot},
    presence,
    toast::{manager as toast_manager, single as toast_single},
    tooltip,
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
    #![proptest_config(super::common::proptest_config())]

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

// ────────────────────────────────────────────────────────────────────
// Dialog proptest
// ────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum DialogStep {
    Send(dialog::Event),
    SetProps(dialog::Props),
}

const DIALOG_EFFECTS: &[dialog::Effect] = &[
    dialog::Effect::OpenChange,
    dialog::Effect::FocusInitial,
    dialog::Effect::FocusFirstTabbable,
    dialog::Effect::ScrollLockAcquire,
    dialog::Effect::ScrollLockRelease,
    dialog::Effect::SetBackgroundInert,
    dialog::Effect::RemoveBackgroundInert,
    dialog::Effect::RestoreFocus,
];

fn arb_focus_target() -> impl Strategy<Value = Option<FocusTarget>> {
    prop_oneof![
        Just(None),
        Just(Some(FocusTarget::First)),
        Just(Some(FocusTarget::Last)),
        Just(Some(FocusTarget::AutofocusMarked)),
        Just(Some(FocusTarget::PreviouslyActive)),
    ]
}

fn arb_dialog_role() -> impl Strategy<Value = dialog::Role> {
    prop_oneof![Just(dialog::Role::Dialog), Just(dialog::Role::AlertDialog)]
}

fn arb_dialog_props() -> impl Strategy<Value = dialog::Props> {
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
                (final_focus, role, title_level, lazy_mount, unmount_on_exit),
            )| {
                dialog::Props::new()
                    .id("dialog")
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
            },
        )
}

fn arb_dialog_event() -> impl Strategy<Value = dialog::Event> {
    prop_oneof![
        Just(dialog::Event::Open),
        Just(dialog::Event::Close),
        Just(dialog::Event::Toggle),
        Just(dialog::Event::CloseOnBackdropClick),
        Just(dialog::Event::CloseOnEscape),
        Just(dialog::Event::RegisterTitle),
        Just(dialog::Event::RegisterDescription),
        Just(dialog::Event::SyncProps),
    ]
}

fn arb_dialog_step() -> impl Strategy<Value = DialogStep> {
    prop_oneof![
        arb_dialog_event().prop_map(DialogStep::Send),
        arb_dialog_props().prop_map(DialogStep::SetProps),
    ]
}

fn assert_dialog_state_context_invariants(service: &Service<dialog::Machine>) -> TestCaseResult {
    // 1. State ⇔ context.open invariant.
    prop_assert_eq!(
        service.context().open,
        matches!(service.state(), dialog::State::Open)
    );

    // 2. ID stability — derived from props.id at init and never mutated.
    prop_assert_eq!(service.context().ids.part("trigger"), "dialog-trigger");
    prop_assert_eq!(service.context().ids.part("content"), "dialog-content");
    prop_assert_eq!(service.context().ids.part("title"), "dialog-title");
    prop_assert_eq!(
        service.context().ids.part("description"),
        "dialog-description"
    );

    Ok(())
}

fn assert_dialog_send_result_invariants(
    event: dialog::Event,
    result: &SendResult<dialog::Machine>,
    before_state: dialog::State,
    before_context: &dialog::Context,
) -> TestCaseResult {
    // 3. Effect-name allow-list — every emitted name is one of the
    // documented `EFFECT_*` constants.
    for effect in &result.pending_effects {
        prop_assert!(
            DIALOG_EFFECTS.contains(&effect.name),
            "unexpected effect name: {:?}",
            effect.name
        );

        // 4. Payload-free invariant — no metadata leaks through any
        // emitted intent (the agnostic-core contract).
        prop_assert!(effect.metadata.is_none());
    }

    // 5. SyncProps is state-preserving: state never changes.
    if matches!(event, dialog::Event::SyncProps) {
        prop_assert!(!result.state_changed);
        prop_assert!(result.pending_effects.is_empty());
    }

    // 6. Guards: CloseOnBackdropClick / CloseOnEscape MUST NOT change
    // state when the corresponding `ctx.close_on_*` flag is false.
    if matches!(event, dialog::Event::CloseOnBackdropClick) && !before_context.close_on_backdrop {
        prop_assert_eq!(result.state_changed, false);
    }
    if matches!(event, dialog::Event::CloseOnEscape) && !before_context.close_on_escape {
        prop_assert_eq!(result.state_changed, false);
    }

    // 7. Register{Title,Description} monotonicity: once `has_title` is
    // true, it stays true; same for `has_description`. (Catch-all
    // route in the state machine prevents flipping back.)
    if matches!(
        event,
        dialog::Event::RegisterTitle | dialog::Event::RegisterDescription
    ) {
        prop_assert!(!result.state_changed);
    }

    // 8. State-flipping events emit `EFFECT_OPEN_CHANGE` exactly when
    // the state actually changed. Conversely, no-op transitions
    // (e.g., Open while already Open) do not emit it.
    let emitted_open_change = result
        .pending_effects
        .iter()
        .any(|e| e.name == dialog::Effect::OpenChange);

    let state_actually_flipped = result.state_changed;

    if matches!(
        event,
        dialog::Event::Open
            | dialog::Event::Close
            | dialog::Event::Toggle
            | dialog::Event::CloseOnBackdropClick
            | dialog::Event::CloseOnEscape
    ) {
        prop_assert_eq!(emitted_open_change, state_actually_flipped);
    } else {
        prop_assert!(!emitted_open_change);
    }

    let _ = before_state;

    Ok(())
}

proptest! {
    #![proptest_config(super::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_dialog_state_context_invariants_hold(
        props in arb_dialog_props(),
        steps in prop::collection::vec(arb_dialog_step(), 0..128),
    ) {
        let mut service = Service::<dialog::Machine>::new(
            props,
            &Env::default(),
            &dialog::Messages::default(),
        );

        assert_dialog_state_context_invariants(&service)?;

        // Track monotonic flags — once true, must stay true.
        let mut had_title = service.context().has_title;

        let mut had_description = service.context().has_description;

        for step in steps {
            match step {
                DialogStep::Send(event) => {
                    let before_state = *service.state();
                    let before_context = service.context().clone();

                    let result = service.send(event);

                    assert_dialog_send_result_invariants(
                        event,
                        &result,
                        before_state,
                        &before_context,
                    )?;
                }

                DialogStep::SetProps(props) => {
                    let before_open_prop = service.props().open;
                    let next_open_prop = props.open;

                    drop(service.set_props(props));

                    // After set_props, every context-backed prop in the
                    // service.props() snapshot must equal the same field
                    // in `service.context()` — proves SyncProps replayed
                    // the incoming props.
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

                    // Controlled-open sync: when open prop flipped to a
                    // concrete value, state and ctx.open must match.
                    if before_open_prop != next_open_prop
                        && let Some(open) = service.props().open
                    {
                        prop_assert_eq!(service.context().open, open);
                    }
                }
            }

            // Monotonicity invariants enforced after each step.
            if had_title {
                prop_assert!(service.context().has_title);
            }

            if had_description {
                prop_assert!(service.context().has_description);
            }

            had_title |= service.context().has_title;

            had_description |= service.context().has_description;

            assert_dialog_state_context_invariants(&service)?;
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Popover proptest
// ────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum PopoverStep {
    Send(popover::Event),
    SetProps(popover::Props),
}

const POPOVER_EFFECTS: &[popover::Effect] = &[
    popover::Effect::OpenChange,
    popover::Effect::AttachClickOutside,
    popover::Effect::DetachClickOutside,
    popover::Effect::AllocateZIndex,
    popover::Effect::ReleaseZIndex,
    popover::Effect::RestoreFocus,
    popover::Effect::FocusInitial,
];

fn arb_arrow_offset() -> impl Strategy<Value = Option<ArrowOffset>> {
    prop_oneof![
        Just(None),
        (-32.0f64..=32.0, -32.0f64..=32.0).prop_map(|(main_axis, cross_axis)| Some(ArrowOffset {
            main_axis,
            cross_axis,
        })),
    ]
}

fn arb_positioning_snapshot() -> impl Strategy<Value = PositioningSnapshot> {
    (arb_placement(), arb_arrow_offset())
        .prop_map(|(placement, arrow)| PositioningSnapshot { placement, arrow })
}

fn arb_popover_props() -> impl Strategy<Value = popover::Props> {
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
                popover::Props::new()
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

fn arb_popover_event() -> impl Strategy<Value = popover::Event> {
    prop_oneof![
        Just(popover::Event::Open),
        Just(popover::Event::Close),
        Just(popover::Event::Toggle),
        Just(popover::Event::CloseOnEscape),
        Just(popover::Event::CloseOnInteractOutside),
        arb_positioning_snapshot().prop_map(popover::Event::PositioningUpdate),
        (0..=4_000u32).prop_map(popover::Event::SetZIndex),
        Just(popover::Event::RegisterTitle),
        Just(popover::Event::RegisterDescription),
        Just(popover::Event::SyncProps),
    ]
}

fn arb_popover_step() -> impl Strategy<Value = PopoverStep> {
    prop_oneof![
        arb_popover_event().prop_map(PopoverStep::Send),
        arb_popover_props().prop_map(PopoverStep::SetProps),
    ]
}

const fn popover_guard_rejects(event: popover::Event, props: &popover::Props) -> bool {
    matches!(event, popover::Event::CloseOnEscape) && !props.close_on_escape
        || matches!(event, popover::Event::CloseOnInteractOutside)
            && !props.close_on_interact_outside
}

fn assert_popover_state_context_invariants(service: &Service<popover::Machine>) -> TestCaseResult {
    // 1. State ⇔ context.open invariant — the boolean and the state
    // enum must agree at all times.
    prop_assert_eq!(
        service.context().open,
        matches!(service.state(), popover::State::Open)
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
    event: popover::Event,
    result: &SendResult<popover::Machine>,
    before_state: popover::State,
    before_context: &popover::Context,
    before_props: &popover::Props,
    after_context: &popover::Context,
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
        .any(|e| e.name == popover::Effect::OpenChange);

    let state_actually_flipped = result.state_changed;

    if matches!(
        event,
        popover::Event::Open
            | popover::Event::Close
            | popover::Event::Toggle
            | popover::Event::CloseOnEscape
            | popover::Event::CloseOnInteractOutside
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
        let names: Vec<popover::Effect> = result
            .pending_effects
            .iter()
            .map(|effect| effect.name)
            .collect();

        match service_resulting_state(before_state, result) {
            popover::State::Open => {
                prop_assert!(names.contains(&popover::Effect::OpenChange));
                prop_assert!(names.contains(&popover::Effect::AllocateZIndex));
                prop_assert!(names.contains(&popover::Effect::AttachClickOutside));
                prop_assert!(names.contains(&popover::Effect::FocusInitial));
            }
            popover::State::Closed => {
                prop_assert!(names.contains(&popover::Effect::OpenChange));
                prop_assert!(names.contains(&popover::Effect::DetachClickOutside));
                prop_assert!(names.contains(&popover::Effect::ReleaseZIndex));
                prop_assert!(names.contains(&popover::Effect::RestoreFocus));
            }
        }
    }

    // 10. Register{Title,Description} monotonicity: once an id is
    // populated, it stays populated; the catch-all match-arm guard
    // rejects re-registration as a no-op.
    if matches!(
        event,
        popover::Event::RegisterTitle | popover::Event::RegisterDescription
    ) {
        prop_assert!(!result.state_changed);
    }

    // 11. SetZIndex MUST NOT change state and is unguarded by state
    // (covers the rare adapter race where the response arrives after
    // a rapid close).
    if let popover::Event::SetZIndex(z_index) = event {
        prop_assert!(!result.state_changed);
        prop_assert_eq!(service_z_index_after(z_index), Some(z_index));
    }

    // 12. PositioningUpdate is gated on state == Open; while closed it
    // is a no-op so stale measurements never leak in.
    if matches!(event, popover::Event::PositioningUpdate(_))
        && matches!(before_state, popover::State::Closed)
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
        && matches!(before_state, popover::State::Open)
        && matches!(
            event,
            popover::Event::Close
                | popover::Event::Toggle
                | popover::Event::CloseOnEscape
                | popover::Event::CloseOnInteractOutside
        )
    {
        prop_assert!(after_context.arrow_offset.is_none());
        prop_assert!(after_context.z_index.is_none());
    }

    let _ = before_context;
    Ok(())
}

// Helpers — defined as plain functions so they can be referenced from
// `prop_assert_eq!` without lifetimes leaking into the `proptest!`
// macro expansion.

const fn service_state_unchanged<M: ars_core::Machine>(result: &SendResult<M>) -> bool {
    !result.state_changed
}

const fn service_resulting_state(
    before: popover::State,
    result: &SendResult<popover::Machine>,
) -> popover::State {
    if result.state_changed {
        match before {
            popover::State::Closed => popover::State::Open,
            popover::State::Open => popover::State::Closed,
        }
    } else {
        before
    }
}

const fn service_z_index_after(z_index: u32) -> Option<u32> {
    Some(z_index)
}

proptest! {
    #![proptest_config(super::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_popover_state_context_invariants_hold(
        props in arb_popover_props(),
        steps in prop::collection::vec(arb_popover_step(), 0..128),
    ) {
        let mut service = Service::<popover::Machine>::new(
            props,
            &Env::default(),
            &popover::Messages::default(),
        );

        // Initial-effects invariant: when the popover boots into Open,
        // `take_initial_effects` MUST emit the full open-plan set; when
        // it boots into Closed, the buffer is empty. Assert this BEFORE
        // any step runs (the buffer is captured at construction).
        let initial_state = *service.state();
        let initial_effects: Vec<popover::Effect> = service
            .take_initial_effects()
            .into_iter()
            .map(|effect| effect.name)
            .collect();

        match initial_state {
            popover::State::Open => {
                prop_assert!(initial_effects.contains(&popover::Effect::OpenChange));
                prop_assert!(initial_effects.contains(&popover::Effect::AllocateZIndex));
                prop_assert!(initial_effects.contains(&popover::Effect::AttachClickOutside));
                prop_assert!(initial_effects.contains(&popover::Effect::FocusInitial));
            }
            popover::State::Closed => {
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
                    let before_context = service.context().clone();
                    let before_props = service.props().clone();

                    let result = service.send(event);

                    let after_context = service.context().clone();

                    assert_popover_send_result_invariants(
                        event,
                        &result,
                        before_state,
                        &before_context,
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

// ────────────────────────────────────────────────────────────────────
// Toast (per-toast machine) proptest
// ────────────────────────────────────────────────────────────────────

const TOAST_SINGLE_EFFECTS: &[toast_single::Effect] = &[
    toast_single::Effect::DurationTimer,
    toast_single::Effect::ExitAnimation,
    toast_single::Effect::AnnouncePolite,
    toast_single::Effect::AnnounceAssertive,
    toast_single::Effect::OpenChange,
];

fn arb_toast_kind() -> impl Strategy<Value = toast_single::Kind> {
    prop_oneof![
        Just(toast_single::Kind::Info),
        Just(toast_single::Kind::Success),
        Just(toast_single::Kind::Warning),
        Just(toast_single::Kind::Error),
        Just(toast_single::Kind::Loading),
    ]
}

fn arb_toast_props() -> impl Strategy<Value = toast_single::Props> {
    (
        arb_toast_kind(),
        prop::option::of(arb_duration(30_000)),
        any::<bool>(),
        10.0f64..=200.0,
    )
        .prop_map(
            |(kind, duration, show_progress, swipe_threshold)| toast_single::Props {
                id: "toast".to_string(),
                title: Some("title".to_string()),
                description: Some("body".to_string()),
                kind,
                duration,
                show_progress,
                swipe_threshold,
            },
        )
}

fn arb_toast_event() -> impl Strategy<Value = toast_single::Event> {
    prop_oneof![
        Just(toast_single::Event::Dismiss),
        arb_duration(30_000).prop_map(|remaining| toast_single::Event::Pause { remaining }),
        Just(toast_single::Event::Resume),
        Just(toast_single::Event::DurationExpired),
        Just(toast_single::Event::AnimationComplete),
        (-200.0f64..=200.0).prop_map(toast_single::Event::SwipeStart),
        (-200.0f64..=200.0).prop_map(toast_single::Event::SwipeMove),
        (-2.0f64..=2.0, -200.0f64..=200.0)
            .prop_map(|(velocity, offset)| { toast_single::Event::SwipeEnd { velocity, offset } }),
    ]
}

fn assert_toast_send_result_invariants(
    service: &Service<toast_single::Machine>,
    event: toast_single::Event,
    before_state: toast_single::State,
    result: &SendResult<toast_single::Machine>,
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
    if matches!(before_state, toast_single::State::Dismissed) {
        prop_assert!(!result.state_changed);
        prop_assert_eq!(service.state(), &toast_single::State::Dismissed);
    }

    // Open mirrors state: only Visible and Paused are "open"; Dismissing
    // and Dismissed have ctx.open == false.
    let expected_open = matches!(
        service.state(),
        toast_single::State::Visible | toast_single::State::Paused
    );

    prop_assert_eq!(service.context().open, expected_open);

    // Pause atomically records the remaining-time snapshot.
    if let toast_single::Event::Pause { remaining } = event
        && result.state_changed
    {
        prop_assert_eq!(service.context().remaining, Some(remaining));
        prop_assert!(
            result
                .cancel_effects
                .contains(&toast_single::Effect::DurationTimer)
        );
    }

    // Resume re-emits `DurationTimer` on a successful Paused → Visible
    // transition **only when the toast has a finite duration**. Persistent
    // toasts (`duration: None`, typical for `Kind::Loading`) deliberately
    // skip the timer effect — emitting one would either schedule a
    // `set_timeout(None)` or auto-dismiss a toast that's supposed to stay
    // until explicit update / removal.
    if matches!(event, toast_single::Event::Resume) && result.state_changed {
        let names: Vec<_> = result.pending_effects.iter().map(|e| e.name).collect();

        let has_duration = service.context().duration.is_some();

        if has_duration {
            prop_assert!(names.contains(&toast_single::Effect::DurationTimer));
        } else {
            prop_assert!(!names.contains(&toast_single::Effect::DurationTimer));
        }
    }

    Ok(())
}

proptest! {
    #![proptest_config(super::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_toast_single_state_context_invariants_hold(
        props in arb_toast_props(),
        events in prop::collection::vec(arb_toast_event(), 0..128),
    ) {
        let mut service = Service::<toast_single::Machine>::new(
            props,
            &Env::default(),
            &toast_single::Messages::default(),
        );

        // initial_effects emits Announce* always, plus DurationTimer when
        // duration is Some. Drained exactly once.
        let initial: Vec<toast_single::Effect> = service
            .take_initial_effects()
            .into_iter()
            .map(|effect| effect.name)
            .collect();

        let kind = service.context().kind;

        let assertive = matches!(kind, toast_single::Kind::Warning | toast_single::Kind::Error);

        if assertive {
            prop_assert!(initial.contains(&toast_single::Effect::AnnounceAssertive));
        } else {
            prop_assert!(initial.contains(&toast_single::Effect::AnnouncePolite));
        }

        if service.context().duration.is_some() {
            prop_assert!(initial.contains(&toast_single::Effect::DurationTimer));
        }

        prop_assert!(service.take_initial_effects().is_empty());

        for event in events {
            let before_state = *service.state();

            let result = service.send(event);

            assert_toast_send_result_invariants(&service, event, before_state, &result)?;
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Toast manager proptest
// ────────────────────────────────────────────────────────────────────

const TOAST_MANAGER_EFFECTS: &[toast_manager::Effect] = &[
    toast_manager::Effect::AnnouncePolite,
    toast_manager::Effect::AnnounceAssertive,
    toast_manager::Effect::ScheduleAnnouncement,
    toast_manager::Effect::PauseAllTimers,
    toast_manager::Effect::ResumeAllTimers,
    toast_manager::Effect::DismissAllToasts,
];

fn arb_toast_placement() -> impl Strategy<Value = toast_manager::Placement> {
    prop_oneof![
        Just(toast_manager::Placement::TopStart),
        Just(toast_manager::Placement::TopCenter),
        Just(toast_manager::Placement::TopEnd),
        Just(toast_manager::Placement::BottomStart),
        Just(toast_manager::Placement::BottomCenter),
        Just(toast_manager::Placement::BottomEnd),
        Just(toast_manager::Placement::TopLeft),
        Just(toast_manager::Placement::TopRight),
        Just(toast_manager::Placement::BottomLeft),
        Just(toast_manager::Placement::BottomRight),
    ]
}

fn arb_hotkey() -> impl Strategy<Value = ars_interactions::Hotkey> {
    use ars_interactions::{Hotkey, KeyboardKey};

    let trigger = prop_oneof![
        // Named-key trigger — exercise a couple of representative variants.
        Just(KeyboardKey::F8).prop_map(Hotkey::named),
        Just(KeyboardKey::Escape).prop_map(Hotkey::named),
        Just(KeyboardKey::ArrowUp).prop_map(Hotkey::named),
        // Char trigger — sample lowercase ASCII letters; matching is
        // case-insensitive so we don't need both cases.
        (0_u32..26).prop_map(|n| Hotkey::char(char::from(b'a' + (n as u8)))),
    ];

    (
        trigger,
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(|(hk, alt, ctrl, shift, meta)| {
            let mut hk = hk;
            if alt {
                hk = hk.with_alt();
            }
            if ctrl {
                hk = hk.with_ctrl();
            }
            if shift {
                hk = hk.with_shift();
            }
            if meta {
                hk = hk.with_meta();
            }
            hk
        })
}

fn arb_toast_manager_props() -> impl Strategy<Value = toast_manager::Props> {
    (
        arb_toast_placement(),
        1_usize..=5,
        0.0f64..=32.0,
        arb_duration(1_000),
        any::<bool>(),
        any::<bool>(),
        prop::option::of(arb_hotkey()),
    )
        .prop_map(
            |(placement, max_visible, gap, remove_delay, dedup_all, overlap, hotkey)| {
                let mut props = toast_manager::Props::new()
                    .id("toaster")
                    .placement(placement)
                    .max_visible(max_visible)
                    .gap(gap)
                    .remove_delay(remove_delay)
                    .deduplicate_all(dedup_all)
                    .overlap(overlap);
                if let Some(hk) = hotkey {
                    props = props.hotkey(hk);
                }
                props
            },
        )
}

fn arb_toast_config() -> impl Strategy<Value = toast_manager::Config> {
    // Always leave `id` as `None` so the manager auto-generates monotonic
    // ids. Random explicit ids would collide and let two entries share an
    // id, which is a precondition violation rather than a bug under test.
    (
        arb_toast_kind(),
        prop::option::of(arb_duration(10_000)),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(|(kind, duration, dismissible, deduplicate)| {
            let mut cfg = toast_manager::Config::new(kind, "title").description("body");

            cfg.duration = duration;
            cfg.dismissible = dismissible;
            cfg.deduplicate = deduplicate;

            cfg
        })
}

fn arb_toast_manager_event() -> impl Strategy<Value = toast_manager::Event> {
    prop_oneof![
        arb_toast_config().prop_map(toast_manager::Event::Add),
        Just(toast_manager::Event::PauseAll),
        Just(toast_manager::Event::ResumeAll),
        Just(toast_manager::Event::DismissAll),
        (0_u64..=10_000).prop_map(|now_ms| toast_manager::Event::DrainAnnouncement { now_ms }),
        any::<bool>().prop_map(toast_manager::Event::SetVisibility),
        Just(toast_manager::Event::SyncProps),
    ]
}

#[derive(Clone, Debug)]
enum ToastManagerStep {
    Send(toast_manager::Event),
    SetProps(toast_manager::Props),
}

fn arb_toast_manager_step() -> impl Strategy<Value = ToastManagerStep> {
    prop_oneof![
        arb_toast_manager_event().prop_map(ToastManagerStep::Send),
        arb_toast_manager_props().prop_map(ToastManagerStep::SetProps),
    ]
}

fn assert_toast_manager_send_result_invariants(
    service: &Service<toast_manager::Machine>,
    event: &toast_manager::Event,
    result: &SendResult<toast_manager::Machine>,
    historical_max_visible: usize,
) -> TestCaseResult {
    for effect in &result.pending_effects {
        prop_assert!(
            TOAST_MANAGER_EFFECTS.contains(&effect.name),
            "unexpected manager effect name: {:?}",
            effect.name
        );

        prop_assert!(effect.metadata.is_none());
    }

    // paused_all flag mirrors State::Paused exactly.
    let ctx_paused = service.context().paused_all;

    let state_paused = matches!(service.state(), toast_manager::State::Paused);

    prop_assert_eq!(ctx_paused, state_paused);

    // Visible-toast count never exceeds the **historical** max_visible.
    // SyncProps deliberately preserves existing toasts when `max_visible`
    // shrinks at runtime — a UX choice (don't yank toasts out from under
    // a user just because a config knob moved). So the strict invariant
    // is `visible_count <= max(max_visible at every prior moment)`,
    // *not* `visible_count <= ctx.max_visible`.
    let visible_count = service
        .context()
        .toasts
        .iter()
        .filter(|entry| entry.stage == toast_manager::EntryStage::Visible)
        .count();

    prop_assert!(visible_count <= historical_max_visible);

    // DrainAnnouncement on an empty queue is a state-preserving no-op.
    if matches!(event, toast_manager::Event::DrainAnnouncement { .. })
        && service.context().announcement_queue.is_empty()
    {
        prop_assert!(!result.state_changed);
    }

    Ok(())
}

proptest! {
    #![proptest_config(super::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_toast_manager_state_context_invariants_hold(
        props in arb_toast_manager_props(),
        steps in prop::collection::vec(arb_toast_manager_step(), 0..128),
    ) {
        let mut service = Service::<toast_manager::Machine>::new(
            props,
            &Env::default(),
            &toast_manager::Messages::default(),
        );

        // Manager has no initial effects today.
        prop_assert!(service.take_initial_effects().is_empty());

        // Track the historical maximum cap so SyncProps shrink doesn't
        // retroactively violate the cap invariant on previously-admitted
        // toasts.
        let mut historical_max_visible = service.context().max_visible;

        for step in steps {
            match step {
                ToastManagerStep::Send(event) => {
                    let result = service.send(event.clone());

                    assert_toast_manager_send_result_invariants(
                        &service,
                        &event,
                        &result,
                        historical_max_visible,
                    )?;
                }

                ToastManagerStep::SetProps(props) => {
                    drop(service.set_props(props));
                    // After set_props, context-backed prop fields must
                    // mirror props (SyncProps reapplied), with
                    // `max_visible` clamped to ≥ 1.

                    let p = service.props();

                    let c = service.context();

                    prop_assert_eq!(c.placement, p.placement);
                    prop_assert_eq!(c.max_visible, p.max_visible.max(1));
                    prop_assert_eq!(c.gap, p.gap);
                    prop_assert_eq!(c.remove_delay, p.remove_delay);
                    prop_assert_eq!(&c.default_durations, &p.default_durations);
                    prop_assert_eq!(c.deduplicate_all, p.deduplicate_all);
                    prop_assert_eq!(c.offsets, p.offsets);
                    prop_assert_eq!(c.overlap, p.overlap);

                    historical_max_visible =
                        historical_max_visible.max(c.max_visible);
                }
            }
        }
    }
}
