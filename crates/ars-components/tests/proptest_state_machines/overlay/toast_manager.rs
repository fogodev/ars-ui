use super::*;

const TOAST_MANAGER_EFFECTS: &[core_toast_manager::Effect] = &[
    core_toast_manager::Effect::AnnouncePolite,
    core_toast_manager::Effect::AnnounceAssertive,
    core_toast_manager::Effect::ScheduleAnnouncement,
    core_toast_manager::Effect::PauseAllTimers,
    core_toast_manager::Effect::ResumeAllTimers,
    core_toast_manager::Effect::DismissAllToasts,
];

fn arb_toast_placement() -> impl Strategy<Value = core_toast_manager::Placement> {
    prop_oneof![
        Just(core_toast_manager::Placement::TopStart),
        Just(core_toast_manager::Placement::TopCenter),
        Just(core_toast_manager::Placement::TopEnd),
        Just(core_toast_manager::Placement::BottomStart),
        Just(core_toast_manager::Placement::BottomCenter),
        Just(core_toast_manager::Placement::BottomEnd),
        Just(core_toast_manager::Placement::TopLeft),
        Just(core_toast_manager::Placement::TopRight),
        Just(core_toast_manager::Placement::BottomLeft),
        Just(core_toast_manager::Placement::BottomRight),
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

fn arb_toast_manager_props() -> impl Strategy<Value = core_toast_manager::Props> {
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
                let mut props = core_toast_manager::Props::new()
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

fn arb_toast_config() -> impl Strategy<Value = core_toast_manager::Config> {
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
            let mut cfg = core_toast_manager::Config::new(kind, "title").description("body");

            cfg.duration = duration;
            cfg.dismissible = dismissible;
            cfg.deduplicate = deduplicate;

            cfg
        })
}

fn arb_toast_manager_event() -> impl Strategy<Value = core_toast_manager::Event> {
    prop_oneof![
        arb_toast_config().prop_map(core_toast_manager::Event::Add),
        Just(core_toast_manager::Event::PauseAll),
        Just(core_toast_manager::Event::ResumeAll),
        Just(core_toast_manager::Event::DismissAll),
        (0_u64..=10_000).prop_map(|now_ms| core_toast_manager::Event::DrainAnnouncement { now_ms }),
        any::<bool>().prop_map(core_toast_manager::Event::SetVisibility),
        Just(core_toast_manager::Event::SyncProps),
    ]
}

#[derive(Clone, Debug)]
enum ToastManagerStep {
    Send(core_toast_manager::Event),
    SetProps(core_toast_manager::Props),
}

fn arb_toast_manager_step() -> impl Strategy<Value = ToastManagerStep> {
    prop_oneof![
        arb_toast_manager_event().prop_map(ToastManagerStep::Send),
        arb_toast_manager_props().prop_map(ToastManagerStep::SetProps),
    ]
}

fn assert_toast_manager_send_result_invariants(
    service: &Service<core_toast_manager::Machine>,
    event: &core_toast_manager::Event,
    result: &SendResult<core_toast_manager::Machine>,
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

    let state_paused = matches!(service.state(), core_toast_manager::State::Paused);

    prop_assert_eq!(ctx_paused, state_paused);

    // pause_reasons is the source of truth: the manager is in Paused
    // iff at least one pause reason is active. Verifying both
    // directions catches any future arm that forgets to update one
    // side of the (state, paused_all, pause_reasons) triple.
    let reasons = service.context().pause_reasons;

    prop_assert_eq!(reasons.any(), state_paused);
    prop_assert_eq!(reasons.any(), ctx_paused);

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
        .filter(|entry| entry.stage == core_toast_manager::EntryStage::Visible)
        .count();

    prop_assert!(visible_count <= historical_max_visible);

    // All toast ids — across both `ctx.toasts` and `ctx.queued` — must
    // be globally unique. `Update(id)` and `Remove(id)` use first-match
    // lookup, so a duplicate id silently makes those operations target
    // the wrong entry. The auto-id resolver must skip past any slot
    // already taken by an explicit caller-supplied id (round-8
    // regression).
    let mut all_ids: Vec<&str> = service
        .context()
        .toasts
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();

    all_ids.extend(
        service
            .context()
            .queued
            .iter()
            .filter_map(|cfg| cfg.id.as_deref()),
    );

    let unique = all_ids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();

    prop_assert_eq!(
        all_ids.len(),
        unique.len(),
        "tracked + queued toast ids must be globally unique; got {:?}",
        all_ids
    );

    // The announcement queue must hold at most one entry per toast id.
    // `Update` is documented to **replace** the pending announcement
    // for an id, not stack new ones on top — otherwise repeated
    // updates between heartbeats would each fire their own
    // `Announce*` effect for the same toast (round-9 regression).
    let announce_ids: Vec<&str> = service
        .context()
        .announcement_queue
        .iter()
        .map(|(id, _)| id.as_str())
        .collect();

    let announce_unique = announce_ids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();

    prop_assert_eq!(
        announce_ids.len(),
        announce_unique.len(),
        "announcement queue must hold at most one entry per toast id; got {:?}",
        announce_ids
    );

    // Note: a strict "FIFO admission" invariant
    // (`queue non-empty → occupied >= max_visible`) is *not*
    // expressible here because `SyncProps` can grow `max_visible` at
    // runtime without auto-promoting the existing backlog (promotion
    // is gated on `HideQueueAdvance`). The round-10 P2 regression is
    // covered directly by the `add_does_not_jump_queue_*` unit tests.

    // DrainAnnouncement on an empty queue is a state-preserving no-op.
    if matches!(event, core_toast_manager::Event::DrainAnnouncement { .. })
        && service.context().announcement_queue.is_empty()
    {
        prop_assert!(!result.state_changed);
    }

    // DrainAnnouncement: the heartbeat signal must never be dropped
    // while announcements remain pending. Adapters implement
    // `ScheduleAnnouncement` as a one-shot trigger, so if the queue
    // is non-empty after a drain attempt and no `Announce*` effect
    // fired, then `ScheduleAnnouncement` must be emitted to keep the
    // heartbeat alive (round-7 regression).
    if matches!(event, core_toast_manager::Event::DrainAnnouncement { .. })
        && !service.context().announcement_queue.is_empty()
    {
        let names = result
            .pending_effects
            .iter()
            .map(|e| e.name)
            .collect::<Vec<_>>();

        let announced = names.iter().any(|n| {
            matches!(
                n,
                core_toast_manager::Effect::AnnouncePolite
                    | core_toast_manager::Effect::AnnounceAssertive
            )
        });

        let rescheduled = names.contains(&core_toast_manager::Effect::ScheduleAnnouncement);

        prop_assert!(
            announced || rescheduled,
            "DrainAnnouncement with non-empty queue must announce or reschedule, got effects: {names:?}",
        );
    }

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_toast_manager_state_context_invariants_hold(
        props in arb_toast_manager_props(),
        steps in prop::collection::vec(arb_toast_manager_step(), 0..128),
    ) {
        let mut service = Service::<core_toast_manager::Machine>::new(
            props,
            &Env::default(),
            &core_toast_manager::Messages::default(),
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
