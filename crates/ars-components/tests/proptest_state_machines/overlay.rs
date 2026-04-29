use core::time::Duration;

use ars_a11y::FocusTarget;
use ars_components::overlay::{
    dialog,
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

// ────────────────────────────────────────────────────────────────────
// Dialog proptest
// ────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
enum DialogStep {
    Send(dialog::Event),
    SetProps(dialog::Props),
}

const DIALOG_EFFECT_NAMES: &[&str] = &[
    dialog::EFFECT_OPEN_CHANGE,
    dialog::EFFECT_FOCUS_INITIAL,
    dialog::EFFECT_FOCUS_FIRST_TABBABLE,
    dialog::EFFECT_SCROLL_LOCK_ACQUIRE,
    dialog::EFFECT_SCROLL_LOCK_RELEASE,
    dialog::EFFECT_SET_BACKGROUND_INERT,
    dialog::EFFECT_REMOVE_BACKGROUND_INERT,
    dialog::EFFECT_RESTORE_FOCUS,
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
            DIALOG_EFFECT_NAMES.contains(&effect.name),
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
        .any(|e| e.name == dialog::EFFECT_OPEN_CHANGE);

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
    #![proptest_config(ProptestConfig::with_cases(
        std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000)
    ))]

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
