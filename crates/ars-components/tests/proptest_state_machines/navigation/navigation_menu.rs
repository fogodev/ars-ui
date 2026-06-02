//! Property-based tests for the `navigation/navigation_menu` state machine.

use core::time::Duration;

use ars_collections::Key;
use ars_components::navigation::navigation_menu;
use ars_core::{AriaAttr, Env, HtmlAttr, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

use super::{arb_direction, arb_orientation};

/// Small key universe so registration / open / focus paths collide and the
/// `Key::str("z")` value exercises the unregistered-key (phantom-panel) guard.
fn arb_nm_key() -> impl Strategy<Value = Key> {
    prop_oneof![
        Just(Key::str("a")),
        Just(Key::str("b")),
        Just(Key::str("c")),
        Just(Key::str("z")), // never registered — exercises the registration gate
    ]
}

fn arb_nm_props() -> impl Strategy<Value = navigation_menu::Props> {
    (
        prop::option::of(prop::option::of(arb_nm_key())),
        prop::option::of(arb_nm_key()),
        0u64..400,
        0u64..400,
        arb_orientation(),
        arb_direction(),
        any::<bool>(),
    )
        .prop_map(
            |(value, default_value, delay, skip_delay, orientation, dir, loop_focus)| {
                let mut props = navigation_menu::Props::new()
                    .id("nav")
                    .delay(Duration::from_millis(delay))
                    .skip_delay(Duration::from_millis(skip_delay))
                    .orientation(orientation)
                    .dir(dir)
                    .loop_focus(loop_focus);

                if let Some(value) = value {
                    props = props.value(value);
                }

                if let Some(default_value) = default_value {
                    props = props.default_value(default_value);
                }

                props
            },
        )
}

fn arb_nm_event() -> impl Strategy<Value = navigation_menu::Event> {
    use navigation_menu::Event;

    prop_oneof![
        arb_nm_key().prop_map(Event::Open),
        (0u64..4000).prop_map(|now| Event::Close(Duration::from_millis(now))),
        (arb_nm_key(), 0u64..4000)
            .prop_map(|(key, now)| Event::PointerEnter(key, Duration::from_millis(now))),
        Just(Event::PointerLeave),
        (arb_nm_key(), any::<bool>())
            .prop_map(|(item, is_keyboard)| Event::FocusTrigger { item, is_keyboard }),
        Just(Event::FocusNext),
        Just(Event::FocusPrev),
        Just(Event::FocusFirst),
        Just(Event::FocusLast),
        (0u64..4000).prop_map(|now| Event::SelectLink(Duration::from_millis(now))),
        (0u64..4000).prop_map(|now| Event::EscapeKey(Duration::from_millis(now))),
        arb_nm_key().prop_map(Event::OpenTimerFired),
        (0u64..4000).prop_map(|now| Event::CloseTimerFired(Duration::from_millis(now))),
        Just(Event::ContentPointerEnter),
        Just(Event::ContentPointerLeave),
        arb_direction().prop_map(Event::SetDirection),
        prop::collection::vec(arb_nm_key(), 0..4).prop_map(Event::SetItems),
        Just(Event::SyncProps),
        prop::option::of(arb_nm_key()).prop_map(Event::SyncControlledValue),
    ]
}

/// Asserts the cross-cutting `NavigationMenu` invariants after any reachable
/// state.
fn assert_nm_invariants(service: &Service<navigation_menu::Machine>) -> TestCaseResult {
    let ctx = service.context();
    let api = service.connect(&|_| {});

    // Once the adapter has registered the trigger list, the open item is never a
    // phantom: `open_item()` only reports a registered trigger, so a stale
    // controlled value pointing at an unregistered key cannot open a panel.
    // Before registration the gate is intentionally permissive so a
    // controlled/default value opens ahead of the first `SetItems` sync (pit of
    // success), so the membership check only applies post-registration.
    if ctx.items_registered
        && let Some(open) = api.open_item()
    {
        prop_assert!(
            ctx.items.contains(open),
            "open_item {open:?} is not among the registered triggers"
        );
    }

    // Every registered trigger renders the canonical menuitem ARIA set, and its
    // `aria-expanded` agrees with `is_item_open`.
    for key in &ctx.items {
        let attrs = api.trigger_attrs(key, "content-id");

        prop_assert_eq!(
            attrs.get(&HtmlAttr::Role),
            Some("menuitem"),
            "trigger role missing"
        );
        prop_assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::HasPopup)),
            Some("true"),
            "aria-haspopup missing"
        );
        prop_assert!(attrs.get(&HtmlAttr::TabIndex).is_some(), "tabindex missing");

        let expanded = attrs.get(&HtmlAttr::Aria(AriaAttr::Expanded));

        let expected = if api.is_item_open(key) {
            "true"
        } else {
            "false"
        };

        prop_assert_eq!(
            expanded,
            Some(expected),
            "aria-expanded must mirror open state"
        );
    }

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    /// Drive the menu with arbitrary event sequences (open/close, hover and
    /// timer intents, focus moves, direction/registry/value sync) and assert
    /// the open-item and trigger-attribute invariants hold throughout.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_navigation_menu_invariants_hold_after_arbitrary_events(
        props in arb_nm_props(),
        events in prop::collection::vec(arb_nm_event(), 0..40),
    ) {
        let mut service = Service::<navigation_menu::Machine>::new(
            props,
            &Env::default(),
            &navigation_menu::Messages::default(),
        );

        assert_nm_invariants(&service)?;

        for event in events {
            drop(service.send(event));

            assert_nm_invariants(&service)?;
        }
    }

    /// Opening a registered trigger shows its panel; opening an unregistered
    /// key never opens a phantom panel (the registration gate).
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_open_respects_registration_gate(
        registered in prop::collection::vec(arb_nm_key(), 1..4),
        target in arb_nm_key(),
    ) {
        let mut service = Service::<navigation_menu::Machine>::new(
            navigation_menu::Props::new().id("nav"),
            &Env::default(),
            &navigation_menu::Messages::default(),
        );

        drop(service.send(navigation_menu::Event::SetItems(registered.clone())));
        drop(service.send(navigation_menu::Event::Open(target.clone())));

        let api = service.connect(&|_| {});

        if registered.contains(&target) {
            prop_assert_eq!(api.open_item(), Some(&target), "registered target must open");
        } else {
            prop_assert!(
                api.open_item().is_none(),
                "unregistered target {target:?} must not open a panel"
            );
        }
    }
}
