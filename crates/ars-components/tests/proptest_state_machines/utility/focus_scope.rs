use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    /// `Activate → Deactivate { restore_focus: false }` always lands at
    /// `State::Inactive` with `saved_focus = None`, regardless of any
    /// intermediate events. Intermediate events may toggle the scope in
    /// and out of `Active`, but the final forced `Deactivate(false)`
    /// from `Active` clears the saved focus via its apply step.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_focus_scope_activate_deactivate_round_trip(
        props in arb_focus_scope_props(),
        intermediate in prop::collection::vec(arb_focus_scope_event(), 0..64),
        saved_focus_id in arb_optional_focus_target(),
    ) {
        let mut service = Service::<utility_core::focus_scope::Machine>::new(
            props,
            &Env::default(),
            &utility_core::focus_scope::Messages,
        );

        // Force-activate so the round-trip always exercises both halves.
        drop(service.send(utility_core::focus_scope::Event::Activate {
            trapped: true,
            saved_focus_id,
        }));

        for event in intermediate {
            drop(service.send(event));
        }

        // Re-activate if the intermediate sequence landed us back at
        // `Inactive`. Without this the final `Deactivate(false)` would be
        // a no-op (the wildcard arm ignores it) and any leftover
        // `saved_focus` from an earlier `Deactivate(true)` would survive.
        if matches!(service.state(), utility_core::focus_scope::State::Inactive) {
            drop(service.send(utility_core::focus_scope::Event::Activate {
                trapped: false,
                saved_focus_id: Some("force-active".to_string()),
            }));
        }

        // Force back to Inactive without restoration. Any active scope
        // MUST return to Inactive after a Deactivate(false); the apply
        // step clears `saved_focus` to drop the stale token.
        drop(service.send(utility_core::focus_scope::Event::Deactivate { restore_focus: false }));

        prop_assert_eq!(service.state(), &utility_core::focus_scope::State::Inactive);
        prop_assert!(service.context().saved_focus.is_none());
    }

    /// `TrapFocus` and `ReleaseTrap` only have a state-affecting effect
    /// while the scope is `Active`; they are ignored from `Inactive`.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_focus_scope_trap_release_only_changes_state_in_active(
        props in arb_focus_scope_props(),
        events in prop::collection::vec(arb_focus_scope_event(), 0..32),
    ) {
        let mut service = Service::<utility_core::focus_scope::Machine>::new(
            props,
            &Env::default(),
            &utility_core::focus_scope::Messages,
        );

        for event in events {
            let was_active = matches!(service.state(), utility_core::focus_scope::State::Active { .. });

            let result = service.send(event.clone());

            match event {
                utility_core::focus_scope::Event::TrapFocus | utility_core::focus_scope::Event::ReleaseTrap
                    if !was_active =>
                {
                    prop_assert!(
                        !result.state_changed,
                        "{:?} from Inactive must be ignored",
                        event,
                    );
                }
                _ => {}
            }

            // The trapped flag and the State::Active variant always agree.
            match service.state() {
                utility_core::focus_scope::State::Inactive => {
                    // No further invariant — saved_focus may be set or cleared.
                }

                utility_core::focus_scope::State::Active { trapped } => {
                    // Empty arm: we just assert the variant carries the
                    // current `trapped` flag, which is structural.
                    let _ = trapped;
                }
            }
        }
    }

    /// Focus-navigation events (`FocusFirst`, `FocusLast`, `RestoreFocus`)
    /// never change the high-level state — they either emit an effect
    /// intent (when their state precondition holds) or are no-ops.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_focus_scope_navigation_events_never_leave_state(
        props in arb_focus_scope_props(),
        events in prop::collection::vec(arb_focus_scope_event(), 0..32),
    ) {
        let mut service = Service::<utility_core::focus_scope::Machine>::new(
            props,
            &Env::default(),
            &utility_core::focus_scope::Messages,
        );

        for event in events {
            let before = *service.state();

            let result = service.send(event.clone());

            if matches!(
                event,
                utility_core::focus_scope::Event::FocusFirst
                    | utility_core::focus_scope::Event::FocusLast
                    | utility_core::focus_scope::Event::RestoreFocus,
            ) {
                prop_assert!(
                    !result.state_changed,
                    "{:?} must not change the high-level state (it only emits an effect intent)",
                    event,
                );
                prop_assert_eq!(service.state(), &before);
            }
        }
    }
}
