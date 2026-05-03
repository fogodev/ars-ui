//! Property-based tests for the navigation/tabs state machine.
//!
//! Each `proptest!` block is `#[ignore]`d so the default `cargo test`
//! run skips them; the nightly `extended-proptest` job clears the
//! ignore filter and runs them with a higher case count via
//! `PROPTEST_CASES`.

use std::collections::BTreeSet;

use ars_collections::Key;
use ars_components::navigation::tabs::{
    ActivationMode, Effect, Event, Machine, Messages, Props, State, TabRegistration,
};
use ars_core::{AriaAttr, Direction, Env, HtmlAttr, Machine as MachineTrait, Orientation, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

// ────────────────────────────────────────────────────────────────────
// Strategies
// ────────────────────────────────────────────────────────────────────

fn arb_key() -> impl Strategy<Value = Key> {
    // Small key universe so collision/registration paths exercise.
    prop_oneof![
        Just(Key::str("a")),
        Just(Key::str("b")),
        Just(Key::str("c")),
        Just(Key::str("d")),
        Just(Key::Int(0)),
        Just(Key::Int(1)),
    ]
}

fn arb_orientation() -> impl Strategy<Value = Orientation> {
    prop_oneof![Just(Orientation::Horizontal), Just(Orientation::Vertical)]
}

fn arb_activation_mode() -> impl Strategy<Value = ActivationMode> {
    prop_oneof![
        Just(ActivationMode::Automatic),
        Just(ActivationMode::Manual),
    ]
}

fn arb_direction() -> impl Strategy<Value = Direction> {
    prop_oneof![
        Just(Direction::Ltr),
        Just(Direction::Rtl),
        Just(Direction::Auto),
    ]
}

fn arb_disabled_keys() -> impl Strategy<Value = BTreeSet<Key>> {
    prop::collection::vec(arb_key(), 0..3).prop_map(|keys| keys.into_iter().collect())
}

fn arb_props() -> impl Strategy<Value = Props> {
    (
        prop::option::of(prop::option::of(arb_key())),
        prop::option::of(arb_key()),
        arb_orientation(),
        arb_activation_mode(),
        arb_direction(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        arb_disabled_keys(),
        any::<bool>(),
    )
        .prop_map(
            |(
                value,
                default_value,
                orientation,
                activation_mode,
                dir,
                loop_focus,
                disallow_empty_selection,
                lazy_mount,
                unmount_on_exit,
                disabled_keys,
                reorderable,
            )| Props {
                id: "tabs".to_string(),
                value,
                default_value,
                orientation,
                activation_mode,
                dir,
                loop_focus,
                disallow_empty_selection,
                lazy_mount,
                unmount_on_exit,
                disabled_keys,
                reorderable,
            },
        )
}

fn arb_tab_registration() -> impl Strategy<Value = TabRegistration> {
    (arb_key(), any::<bool>()).prop_map(|(key, closable)| TabRegistration { key, closable })
}

fn arb_event() -> impl Strategy<Value = Event> {
    prop_oneof![
        arb_key().prop_map(Event::SelectTab),
        arb_key().prop_map(Event::Focus),
        Just(Event::Blur),
        Just(Event::FocusNext),
        Just(Event::FocusPrev),
        Just(Event::FocusFirst),
        Just(Event::FocusLast),
        arb_direction().prop_map(Event::SetDirection),
        prop::collection::vec(arb_tab_registration(), 0..5).prop_map(Event::SetTabs),
        arb_key().prop_map(Event::CloseTab),
        (arb_key(), 0usize..8).prop_map(|(tab, new_index)| Event::ReorderTab { tab, new_index }),
        Just(Event::SyncProps),
    ]
}

/// Combined step: either dispatch an event OR re-set props at runtime.
/// Lets the multi-prop interaction proptest exercise consumer-driven
/// prop changes mid-sequence.
#[derive(Clone, Debug)]
enum Step {
    Send(Event),
    SetProps(Props),
}

fn arb_step() -> impl Strategy<Value = Step> {
    prop_oneof![
        // Event dispatch is more common (8) than prop swap (2) so the
        // generated sequences exercise event-driven transitions more
        // densely than runtime prop churn.
        8 => arb_event().prop_map(Step::Send),
        2 => arb_props().prop_map(Step::SetProps),
    ]
}

// ────────────────────────────────────────────────────────────────────
// Invariants
// ────────────────────────────────────────────────────────────────────

/// Asserts every cross-cutting invariant the tabs machine must hold
/// after any reachable state. Returns a `TestCaseResult` so callers
/// can `?` propagate the `prop_assert` chain.
fn assert_invariants(service: &Service<Machine>) -> TestCaseResult {
    let ctx = service.context();

    // 1. State::Focused implies focused_tab == Some(tab).
    if let State::Focused { tab } = service.state() {
        prop_assert_eq!(
            ctx.focused_tab.as_ref(),
            Some(tab),
            "State::Focused must keep ctx.focused_tab in sync"
        );
    }

    // 2. value points at None or a key currently in `tabs`. The
    //    invariant only applies AFTER at least one `Event::SetTabs`
    //    populated the registered list — pre-registration `value` is
    //    still whatever `default_value` / `value` was at init time,
    //    and the snap doesn't run from init.
    //
    //    Controlled-Some(k) where k isn't registered is also ALLOWED
    //    even after registration — the consumer drives controlled
    //    values and the machine cannot override them via
    //    `Bindable::set`.
    if !ctx.tabs.is_empty()
        && !ctx.value.is_controlled()
        && let Some(selected) = ctx.value.get().as_ref()
    {
        prop_assert!(
            ctx.tabs.iter().any(|k| k == selected),
            "uncontrolled value {selected:?} not in ctx.tabs (registered: {:?})",
            ctx.tabs
        );
    }

    // 3. focused_tab points at a registered, non-disabled key (or None).
    if let Some(focused) = ctx.focused_tab.as_ref() {
        prop_assert!(
            ctx.tabs.iter().any(|k| k == focused),
            "focused_tab {focused:?} is not in ctx.tabs"
        );
        prop_assert!(
            !ctx.disabled_tabs.contains(focused),
            "focused_tab {focused:?} is disabled"
        );
    }

    // 4. closable_tabs ⊆ tabs.
    for closable_key in &ctx.closable_tabs {
        prop_assert!(
            ctx.tabs.iter().any(|k| k == closable_key),
            "closable_tabs contains unregistered key {closable_key:?}"
        );
    }

    // 5. tabs has no duplicate keys.
    let mut seen = BTreeSet::new();

    for key in &ctx.tabs {
        prop_assert!(
            seen.insert(key.clone()),
            "ctx.tabs contains duplicate key {key:?}"
        );
    }

    Ok(())
}

/// Asserts every tab trigger renders the canonical attribute set
/// regardless of mutation state.
fn assert_tab_attr_shape(service: &Service<Machine>) -> TestCaseResult {
    let api = service.connect(&|_| {});

    let tabs_snapshot = service.context().tabs.clone();

    for key in &tabs_snapshot {
        let attrs = api.tab_attrs(key, false);

        prop_assert_eq!(attrs.get(&HtmlAttr::Role), Some("tab"), "tab role missing");
        prop_assert!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Selected)).is_some(),
            "aria-selected missing"
        );
        prop_assert!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Controls)).is_some(),
            "aria-controls missing"
        );
        prop_assert!(attrs.get(&HtmlAttr::TabIndex).is_some(), "tabindex missing");
    }

    Ok(())
}

// ────────────────────────────────────────────────────────────────────
// proptest! blocks
// ────────────────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(super::common::proptest_config())]

    /// Drive the machine with arbitrary event sequences and assert
    /// none of the cross-cutting invariants break.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_tabs_invariants_hold_after_arbitrary_events(
        props in arb_props(),
        events in prop::collection::vec(arb_event(), 0..32),
    ) {
        let mut service = Service::<Machine>::new(
            props,
            &Env::default(),
            &Messages::default(),
        );

        // Initial state must satisfy invariants.
        assert_invariants(&service)?;

        for event in events {
            drop(service.send(event));

            assert_invariants(&service)?;
        }
    }

    /// `tab_attrs` for every registered tab always renders the
    /// canonical ARIA set (id / role / aria-selected / aria-controls /
    /// tabindex), regardless of selection, focus, disabled, or
    /// reorderable state.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_tab_attrs_always_render_canonical_attrs(
        props in arb_props(),
        events in prop::collection::vec(arb_event(), 0..32),
    ) {
        let mut service = Service::<Machine>::new(
            props,
            &Env::default(),
            &Messages::default(),
        );

        for event in events {
            drop(service.send(event));
        }

        assert_tab_attr_shape(&service)?;
    }

    /// FocusNext / FocusPrev / FocusFirst / FocusLast that produce a
    /// transition always emit `Effect::FocusFocusedTab` and leave
    /// `focused_tab` at a registered, non-disabled key.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_focus_transitions_emit_focus_effect(
        props in arb_props(),
        registrations in prop::collection::vec(arb_tab_registration(), 1..6),
        focus_event in prop_oneof![
            Just(Event::FocusNext),
            Just(Event::FocusPrev),
            Just(Event::FocusFirst),
            Just(Event::FocusLast),
        ],
    ) {
        let mut service = Service::<Machine>::new(
            props,
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::SetTabs(registrations)));

        let result = service.send(focus_event);

        if result.state_changed || result.context_changed {
            // When a focus-movement event causes a transition, the
            // machine must request live focus from the adapter.
            let names = result.pending_effects.iter().map(|e| e.name).collect::<Vec<_>>();

            prop_assert!(
                names.contains(&Effect::FocusFocusedTab),
                "focus transition without FocusFocusedTab effect: {names:?}"
            );

            let ctx = service.context();

            if let Some(focused) = ctx.focused_tab.as_ref() {
                prop_assert!(
                    ctx.tabs.iter().any(|k| k == focused),
                    "focused tab not in registered list"
                );
                prop_assert!(
                    !ctx.disabled_tabs.contains(focused),
                    "focus moved to a disabled tab"
                );
            }
        }
    }

    /// Drive the machine with a mix of events AND runtime prop changes
    /// (`Service::set_props` followed by the `SyncProps` event the
    /// adapter would dispatch via `on_props_changed`). Asserts the same
    /// invariants hold across the more chaotic event-prop interleaving
    /// — covers consumer flows that swap `disabled_keys`,
    /// `orientation`, `dir`, etc. while the user is interacting.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_invariants_hold_across_event_and_prop_interleavings(
        initial_props in arb_props(),
        steps in prop::collection::vec(arb_step(), 0..32),
    ) {
        let mut service = Service::<Machine>::new(
            initial_props,
            &Env::default(),
            &Messages::default(),
        );

        assert_invariants(&service)?;

        for step in steps {
            match step {
                Step::Send(event) => {
                    drop(service.send(event));
                }

                Step::SetProps(new_props) => {
                    let old_props = service.props().clone();

                    let triggered = <Machine as MachineTrait>::on_props_changed(&old_props, &new_props);

                    drop(service.set_props(new_props));

                    // Consumer adapters are expected to forward the
                    // events `on_props_changed` returns. Replay them
                    // here so the machine sees the same sequence the
                    // adapter would dispatch.
                    for event in triggered {
                        drop(service.send(event));
                    }
                }
            }

            assert_invariants(&service)?;
        }
    }

    /// `Api::successor_for_close(k)` and `Api::can_close_tab(k)` are
    /// mutually consistent: when `can_close_tab` returns `false`,
    /// `successor_for_close` returns `None` and the machine still
    /// holds invariants if the consumer ignores the close.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_successor_and_can_close_are_consistent(
        registrations in prop::collection::vec(arb_tab_registration(), 1..6),
        disallow_empty in any::<bool>(),
        target in arb_key(),
    ) {
        let mut service = Service::<Machine>::new(
            Props {
                id: "tabs".to_string(),
                disallow_empty_selection: disallow_empty,
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::SetTabs(registrations.clone())));

        let api = service.connect(&|_| {});

        let registered_unique =
            registrations.iter().map(|r| r.key.clone()).collect::<BTreeSet<_>>();

        let in_list = registered_unique.contains(&target);

        let can_close = api.can_close_tab(&target);

        let successor = api.successor_for_close(&target);

        if !in_list {
            prop_assert!(!can_close, "can_close_tab({target:?}) must be false for unregistered key");
            prop_assert!(successor.is_none(), "successor_for_close({target:?}) must be None for unregistered key");
        } else if disallow_empty && registered_unique.len() == 1 {
            prop_assert!(!can_close, "disallow_empty + only-tab must refuse close");
        }

        // Successor (when present) is always a valid registered key
        // distinct from the closing key.
        if let Some(next) = successor.as_ref() {
            prop_assert!(registered_unique.contains(next));
            prop_assert_ne!(next, &target);
        }
    }
}
