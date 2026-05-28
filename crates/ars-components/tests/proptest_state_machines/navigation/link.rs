//! Property-based tests for the `navigation/link` state machine.

use ars_components::navigation::link;
use ars_core::{AriaAttr, Env, HtmlAttr, SafeUrl, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

fn arb_target() -> impl Strategy<Value = link::Target> {
    prop_oneof![
        Just(link::Target::Href(SafeUrl::from_static("/about"))),
        Just(link::Target::Href(SafeUrl::from_static(
            "https://example.com"
        ))),
        Just(link::Target::Href(SafeUrl::from_static(""))),
        Just(link::Target::Route("/dashboard".to_string())),
    ]
}

fn arb_aria_current() -> impl Strategy<Value = Option<link::AriaCurrent>> {
    prop_oneof![
        Just(None),
        Just(Some(link::AriaCurrent::Page)),
        Just(Some(link::AriaCurrent::Step)),
        Just(Some(link::AriaCurrent::True)),
    ]
}

fn arb_link_props() -> impl Strategy<Value = link::Props> {
    (
        arb_target(),
        prop::option::of(prop_oneof![
            Just("_blank".to_string()),
            Just("_self".to_string())
        ]),
        prop::option::of(Just("noopener".to_string())),
        arb_aria_current(),
        any::<bool>(),
    )
        .prop_map(|(href, target, rel, is_current, disabled)| {
            let mut props = link::Props::new().id("link").href(href).disabled(disabled);

            if let Some(target) = target {
                props = props.target(target);
            }

            if let Some(rel) = rel {
                props = props.rel(rel);
            }

            if let Some(current) = is_current {
                props = props.is_current(current);
            }

            props
        })
}

fn arb_link_event() -> impl Strategy<Value = link::Event> {
    prop_oneof![
        any::<bool>().prop_map(|is_keyboard| link::Event::Focus { is_keyboard }),
        Just(link::Event::Blur),
        Just(link::Event::Press),
        Just(link::Event::PressEnd),
        Just(link::Event::Navigate),
        Just(link::Event::SyncProps),
    ]
}

fn assert_link_invariants(service: &Service<link::Machine>) -> TestCaseResult {
    let state = *service.state();
    let ctx = service.context();

    // `Pressed` is reachable only via `Press` (guarded by `!disabled`, the sole
    // setter of `pressed`), so the state implies the flag and excludes disabled.
    // The converse does NOT hold: `Focus` arriving mid-press moves to `Focused`
    // while the press is still physically held, so `pressed` stays true until
    // `PressEnd`/`Blur` — `pressed` is not exclusive to `State::Pressed`.
    if state == link::State::Pressed {
        prop_assert!(ctx.pressed, "State::Pressed implies ctx.pressed");
        prop_assert!(
            !ctx.disabled,
            "a disabled link can never enter State::Pressed"
        );
    }

    // `Idle` is only entered by `Blur`/disabling-`SyncProps`/init, all of which
    // clear the press flag, so a resting link is never left mid-press.
    if state == link::State::Idle {
        prop_assert!(!ctx.pressed, "State::Idle must clear the press flag");
    }

    // Keyboard-visible focus implies the link is focused (set together by
    // `Focus`, cleared together by `Blur`/disable).
    if ctx.focus_visible {
        prop_assert!(ctx.focused, "focus_visible implies focused");
    }

    // The root host always renders the canonical scope/part tokens and a
    // `data-ars-state` that matches the live state.
    let attrs = service.connect(&|_| {}).root_attrs();

    prop_assert_eq!(
        attrs.get(&HtmlAttr::Data("ars-state")),
        Some(state.as_str()),
        "data-ars-state must mirror the machine state"
    );

    // A disabled link drops its href and advertises `aria-disabled`; an enabled
    // link always renders a navigable href.
    if ctx.disabled {
        prop_assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true"),
            "disabled link must set aria-disabled"
        );
        prop_assert!(
            attrs.get(&HtmlAttr::Href).is_none(),
            "disabled link must not render an href"
        );
    } else {
        prop_assert!(
            attrs.get(&HtmlAttr::Href).is_some(),
            "enabled link must render an href"
        );
    }

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    /// Drive the link with arbitrary focus/press/navigate/sync sequences and
    /// assert the press-state and rendered-attribute invariants hold throughout.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_link_invariants_hold_after_arbitrary_events(
        props in arb_link_props(),
        events in prop::collection::vec(arb_link_event(), 0..32),
    ) {
        let mut service = Service::<link::Machine>::new(
            props,
            &Env::default(),
            &link::Messages::default(),
        );

        assert_link_invariants(&service)?;

        for event in events {
            drop(service.send(event));

            assert_link_invariants(&service)?;
        }
    }

    /// `Navigate` emits `Effect::Navigate` exactly when the link is enabled;
    /// a disabled link swallows the activation with no effect.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_navigate_emits_effect_iff_enabled(
        props in arb_link_props(),
    ) {
        let disabled = props.disabled;

        let mut service = Service::<link::Machine>::new(
            props,
            &Env::default(),
            &link::Messages::default(),
        );

        let result = service.send(link::Event::Navigate);

        let emitted = result
            .pending_effects
            .iter()
            .any(|effect| effect.name == link::Effect::Navigate);

        prop_assert_eq!(emitted, !disabled, "Navigate effect fires iff the link is enabled");
    }
}
