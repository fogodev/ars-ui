//! Inline unit tests for the [`Tabs`](super) state machine.
//!
//! Tests are organized by topic, mirroring the issue's "tests to add
//! first" checklist (#261) and covering the §5 Closable and §6
//! Reorderable variants.

use alloc::{
    string::{String, ToString as _},
    vec,
    vec::Vec,
};
use core::cell::RefCell;

use ars_core::{Machine as MachineTrait, MessageFn, SendResult, Service};
use insta::assert_snapshot;

use super::*;

// ── Test fixtures ────────────────────────────────────────────────────

/// Convenience: build a `Key::String` from a `&str`.
fn key(value: &str) -> Key {
    Key::str(value)
}

/// Builds a baseline keyboard event with the given normalized key and no
/// modifiers held.
fn keydown(k: KeyboardKey) -> KeyboardEventData {
    KeyboardEventData {
        key: k,
        character: None,
        code: String::new(),
        shift_key: false,
        ctrl_key: false,
        alt_key: false,
        meta_key: false,
        repeat: false,
        is_composing: false,
    }
}

/// Builds a Ctrl-modified keyboard event for reorder tests.
fn ctrl_keydown(k: KeyboardKey) -> KeyboardEventData {
    let mut data = keydown(k);

    data.ctrl_key = true;

    data
}

/// Returns the default test props with a stable component id and `a` as
/// the default selected tab.
fn test_props() -> Props {
    Props {
        id: "tabs".to_string(),
        default_value: Some(key("a")),
        ..Props::default()
    }
}

/// Builds a fresh service with the supplied props and registers `tabs`
/// (all non-closable) so arrow-key navigation has data to walk.
fn service_with_tabs(props: Props, tabs: &[Key]) -> Service<Machine> {
    let mut service = Service::<Machine>::new(props, &Env::default(), &Messages::default());

    let registrations = tabs
        .iter()
        .cloned()
        .map(TabRegistration::new)
        .collect::<Vec<_>>();

    drop(service.send(Event::SetTabs(registrations)));

    service
}

/// Builds a service with the supplied props and per-tab closability flags.
fn service_with_registrations(
    props: Props,
    registrations: Vec<TabRegistration>,
) -> Service<Machine> {
    let mut service = Service::<Machine>::new(props, &Env::default(), &Messages::default());

    drop(service.send(Event::SetTabs(registrations)));

    service
}

/// Records every event dispatched through the connected `send` closure.
type EventRecorder = RefCell<Vec<Event>>;

/// Pushes the captured event into the supplied recorder. Inline as
/// `service.connect(&|e| record(&recorder, e))` to keep the closure's
/// borrow tied to the test's local stack frame.
fn record(recorder: &EventRecorder, event: Event) {
    recorder.borrow_mut().push(event);
}

/// Renders an `AttrMap` to a string for snapshot comparison. Ordering
/// follows `AttrMap`'s internal sorted Vec, which is alphabetical by
/// attribute key.
fn snapshot_attrs(attrs: &AttrMap) -> String {
    format!("{attrs:#?}")
}

/// Collects every effect name from a [`SendResult`].
fn effect_names(result: &SendResult<Machine>) -> Vec<Effect> {
    result.pending_effects.iter().map(|e| e.name).collect()
}

// ────────────────────────────────────────────────────────────────────
// 1. Tab list role and orientation
// ────────────────────────────────────────────────────────────────────

#[test]
fn tablist_role_horizontal() {
    let service = service_with_tabs(test_props(), &[]);

    assert_snapshot!(
        "tabs_list_horizontal",
        snapshot_attrs(&service.connect(&|_| {}).list_attrs())
    );
}

#[test]
fn tablist_role_vertical() {
    let service = service_with_tabs(
        Props {
            orientation: Orientation::Vertical,
            ..test_props()
        },
        &[],
    );

    assert_snapshot!(
        "tabs_list_vertical",
        snapshot_attrs(&service.connect(&|_| {}).list_attrs())
    );
}

// ────────────────────────────────────────────────────────────────────
// 2. Tab triggers — aria-selected / aria-controls / focus-visible
// ────────────────────────────────────────────────────────────────────

#[test]
fn tab_attrs_selected_focus_invisible() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    assert_snapshot!(
        "tabs_tab_selected",
        snapshot_attrs(&service.connect(&|_| {}).tab_attrs(&key("a"), false))
    );
}

#[test]
fn tab_attrs_unselected() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    assert_snapshot!(
        "tabs_tab_unselected",
        snapshot_attrs(&service.connect(&|_| {}).tab_attrs(&key("b"), false))
    );
}

#[test]
fn tab_attrs_focus_visible_renders_only_when_focused() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    drop(service.send(Event::Focus(key("a"))));

    let api = service.connect(&|_| {});

    // Focused tab + focus_visible=true → emits data-ars-focus-visible.
    assert_snapshot!(
        "tabs_tab_focus_visible",
        snapshot_attrs(&api.tab_attrs(&key("a"), true))
    );

    // Non-focused tab still gets no focus-visible attr even when caller
    // passes true (focus_visible only applies to the focused tab).
    let other = api.tab_attrs(&key("b"), true);

    assert!(
        other.get(&HtmlAttr::Data("ars-focus-visible")).is_none(),
        "non-focused tab must never carry data-ars-focus-visible"
    );
}

// ────────────────────────────────────────────────────────────────────
// 3. Panels — aria-labelledby / hidden / aria-label fallback
// ────────────────────────────────────────────────────────────────────

#[test]
fn panel_attrs_visible() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    assert_snapshot!(
        "tabs_panel_visible",
        snapshot_attrs(&service.connect(&|_| {}).panel_attrs(&key("a"), None))
    );
}

#[test]
fn panel_attrs_hidden() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    assert_snapshot!(
        "tabs_panel_hidden",
        snapshot_attrs(&service.connect(&|_| {}).panel_attrs(&key("b"), None))
    );
}

#[test]
fn panel_attrs_with_label_fallback() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    assert_snapshot!(
        "tabs_panel_with_label",
        snapshot_attrs(
            &service
                .connect(&|_| {})
                .panel_attrs(&key("a"), Some("Inbox"))
        )
    );
}

// ────────────────────────────────────────────────────────────────────
// 4. Roving tabindex
// ────────────────────────────────────────────────────────────────────

#[test]
fn roving_tabindex_only_selected_is_zero() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b"), key("c")]);

    let api = service.connect(&|_| {});

    assert_eq!(
        api.tab_attrs(&key("a"), false).get(&HtmlAttr::TabIndex),
        Some("0")
    );
    assert_eq!(
        api.tab_attrs(&key("b"), false).get(&HtmlAttr::TabIndex),
        Some("-1")
    );
    assert_eq!(
        api.tab_attrs(&key("c"), false).get(&HtmlAttr::TabIndex),
        Some("-1")
    );
}

// ────────────────────────────────────────────────────────────────────
// 5–7. Activation modes
// ────────────────────────────────────────────────────────────────────

#[test]
fn automatic_activation_focus_selects() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let result = service.send(Event::Focus(key("b")));

    assert!(result.state_changed);
    assert_eq!(service.context().focused_tab.as_ref(), Some(&key("b")));
    assert_eq!(service.context().value.get().as_ref(), Some(&key("b")));
}

#[test]
fn manual_activation_focus_does_not_select() {
    let mut service = service_with_tabs(
        Props {
            activation_mode: ActivationMode::Manual,
            ..test_props()
        },
        &[key("a"), key("b")],
    );

    let result = service.send(Event::Focus(key("b")));

    assert!(result.state_changed);
    assert_eq!(service.context().focused_tab.as_ref(), Some(&key("b")));
    assert_eq!(
        service.context().value.get().as_ref(),
        Some(&key("a")),
        "value should remain unchanged in Manual mode"
    );
}

#[test]
fn manual_activation_enter_selects() {
    let mut service = service_with_tabs(
        Props {
            activation_mode: ActivationMode::Manual,
            ..test_props()
        },
        &[key("a"), key("b")],
    );

    drop(service.send(Event::Focus(key("b"))));

    let recorder: EventRecorder = RefCell::new(Vec::new());
    {
        let send = |event| record(&recorder, event);

        let api = service.connect(&send);

        api.on_tab_keydown(&key("b"), &keydown(KeyboardKey::Enter));
    }

    assert_eq!(
        recorder.borrow().as_slice(),
        &[Event::SelectTab(key("b"))],
        "Manual mode + Enter should dispatch SelectTab"
    );

    drop(service.send(Event::SelectTab(key("b"))));

    assert_eq!(service.context().value.get().as_ref(), Some(&key("b")));
}

#[test]
fn auto_activation_enter_does_not_dispatch_select_tab() {
    // Catches `&&` → `||` between `(Enter || Space)` and `manual` in
    // on_tab_keydown. In Auto mode, Enter must NOT dispatch SelectTab —
    // selection follows focus, not Enter. Mutation would emit
    // SelectTab on every Enter in Auto mode.
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("b"), &keydown(KeyboardKey::Enter));

    assert!(
        recorder.borrow().is_empty(),
        "Auto mode + Enter must NOT dispatch SelectTab"
    );
}

#[test]
fn auto_activation_space_does_not_dispatch_select_tab() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("b"), &keydown(KeyboardKey::Space));

    assert!(
        recorder.borrow().is_empty(),
        "Auto mode + Space must NOT dispatch SelectTab"
    );
}

#[test]
fn manual_activation_space_selects() {
    let service = service_with_tabs(
        Props {
            activation_mode: ActivationMode::Manual,
            ..test_props()
        },
        &[key("a"), key("b")],
    );

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("b"), &keydown(KeyboardKey::Space));

    assert_eq!(recorder.borrow().as_slice(), &[Event::SelectTab(key("b"))]);
}

#[test]
fn focus_event_idempotent_when_already_focused_in_manual_mode() {
    let mut service = service_with_tabs(
        Props {
            activation_mode: ActivationMode::Manual,
            ..test_props()
        },
        &[key("a"), key("b")],
    );

    drop(service.send(Event::Focus(key("b"))));

    let result = service.send(Event::Focus(key("b")));

    assert!(
        !result.state_changed && !result.context_changed,
        "re-focusing the same tab in Manual mode is a no-op"
    );
}

#[test]
fn focus_event_idempotent_in_automatic_mode_when_already_focused_and_selected() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    // First Focus selects b (auto mode advances value).
    drop(service.send(Event::Focus(key("b"))));

    assert_eq!(service.context().value.get().as_ref(), Some(&key("b")));

    // Second Focus on the same tab where value already points at b
    // must be a no-op — both focused_tab and value already match.
    let result = service.send(Event::Focus(key("b")));

    assert!(
        !result.state_changed && !result.context_changed,
        "re-focusing the same tab in Auto mode when value already matches is a no-op"
    );
}

// ────────────────────────────────────────────────────────────────────
// 8–10. Orientation + RTL arrow key matrix
// ────────────────────────────────────────────────────────────────────

#[test]
fn horizontal_arrow_keys_dispatch_focus_events() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b"), key("c")]);

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("a"), &keydown(KeyboardKey::ArrowRight));
    api.on_tab_keydown(&key("a"), &keydown(KeyboardKey::ArrowLeft));

    assert_eq!(
        recorder.borrow().as_slice(),
        &[Event::FocusNext, Event::FocusPrev]
    );
}

#[test]
fn vertical_arrow_keys_dispatch_focus_events() {
    let service = service_with_tabs(
        Props {
            orientation: Orientation::Vertical,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("a"), &keydown(KeyboardKey::ArrowDown));
    api.on_tab_keydown(&key("a"), &keydown(KeyboardKey::ArrowUp));

    assert_eq!(
        recorder.borrow().as_slice(),
        &[Event::FocusNext, Event::FocusPrev]
    );
}

#[test]
fn vertical_orientation_ignores_horizontal_arrows() {
    let service = service_with_tabs(
        Props {
            orientation: Orientation::Vertical,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("a"), &keydown(KeyboardKey::ArrowLeft));
    api.on_tab_keydown(&key("a"), &keydown(KeyboardKey::ArrowRight));

    assert!(
        recorder.borrow().is_empty(),
        "horizontal arrows should be ignored in vertical orientation"
    );
}

#[test]
fn horizontal_rtl_swaps_arrow_keys() {
    let service = service_with_tabs(
        Props {
            dir: Direction::Rtl,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("a"), &keydown(KeyboardKey::ArrowRight));
    api.on_tab_keydown(&key("a"), &keydown(KeyboardKey::ArrowLeft));

    assert_eq!(
        recorder.borrow().as_slice(),
        &[Event::FocusPrev, Event::FocusNext],
        "RTL + horizontal: ArrowRight = prev, ArrowLeft = next"
    );
}

#[test]
fn horizontal_auto_direction_defaults_to_ltr() {
    let service = service_with_tabs(
        Props {
            dir: Direction::Auto,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("a"), &keydown(KeyboardKey::ArrowRight));

    assert_eq!(recorder.borrow().as_slice(), &[Event::FocusNext]);
}

// ────────────────────────────────────────────────────────────────────
// 11–13. Disabled tabs
// ────────────────────────────────────────────────────────────────────

#[test]
fn disabled_tabs_skipped_in_arrow_navigation() {
    let mut disabled = BTreeSet::new();

    disabled.insert(key("b"));

    let mut service = service_with_tabs(
        Props {
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    drop(service.send(Event::Focus(key("a"))));
    drop(service.send(Event::FocusNext));

    assert_eq!(
        service.context().focused_tab.as_ref(),
        Some(&key("c")),
        "disabled tab `b` should be skipped"
    );
}

#[test]
fn disabled_tab_select_event_no_op() {
    let mut disabled = BTreeSet::new();

    disabled.insert(key("b"));

    let mut service = service_with_tabs(
        Props {
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("a"), key("b")],
    );

    let result = service.send(Event::SelectTab(key("b")));

    assert!(!result.state_changed);
    assert!(!result.context_changed);
    assert_eq!(service.context().value.get().as_ref(), Some(&key("a")));
}

#[test]
fn disabled_tab_renders_aria_disabled() {
    let mut disabled = BTreeSet::new();

    disabled.insert(key("b"));

    let service = service_with_tabs(
        Props {
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("a"), key("b")],
    );

    assert_snapshot!(
        "tabs_tab_disabled",
        snapshot_attrs(&service.connect(&|_| {}).tab_attrs(&key("b"), false))
    );
}

#[test]
fn disabled_tab_with_focus_visible_still_emits_focus_visible() {
    // §3.3 invariant: disabled tabs are focusable but not activatable.
    // When a disabled tab somehow ends up focused (e.g., consumer
    // programmatically focused it before disabling), focus-visible
    // styling still applies.
    let mut disabled = BTreeSet::new();

    disabled.insert(key("b"));

    let mut service = service_with_tabs(
        Props {
            disabled_keys: disabled.clone(),
            default_value: Some(key("a")),
            ..test_props()
        },
        &[key("a"), key("b")],
    );

    // Manually plant focused_tab on the disabled tab via an internal
    // path: send Focus first while the tab is enabled, then re-register
    // with the tab disabled.
    let mut props_no_disabled = service.props().clone();

    props_no_disabled.disabled_keys = BTreeSet::new();

    drop(
        Service::<Machine>::new(props_no_disabled, &Env::default(), &Messages::default())
            .send(Event::Focus(key("b"))),
    );
    drop(service.send(Event::Focus(key("a"))));

    let api = service.connect(&|_| {});

    let attrs = api.tab_attrs(&key("a"), true);

    assert_eq!(
        attrs.get(&HtmlAttr::Data("ars-focus-visible")),
        Some("true")
    );
}

// ────────────────────────────────────────────────────────────────────
// 14. Loop focus vs clamp
// ────────────────────────────────────────────────────────────────────

#[test]
fn loop_focus_wraps_at_end() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b"), key("c")]);

    drop(service.send(Event::Focus(key("c"))));
    drop(service.send(Event::FocusNext));

    assert_eq!(service.context().focused_tab.as_ref(), Some(&key("a")));
}

#[test]
fn loop_focus_disabled_clamps_at_end() {
    let mut service = service_with_tabs(
        Props {
            loop_focus: false,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    drop(service.send(Event::Focus(key("c"))));

    let result = service.send(Event::FocusNext);

    assert!(!result.state_changed);
    assert_eq!(service.context().focused_tab.as_ref(), Some(&key("c")));
}

#[test]
fn loop_focus_wraps_at_start() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b"), key("c")]);

    drop(service.send(Event::Focus(key("a"))));
    drop(service.send(Event::FocusPrev));

    assert_eq!(service.context().focused_tab.as_ref(), Some(&key("c")));
}

#[test]
fn loop_focus_disabled_clamps_at_start() {
    let mut service = service_with_tabs(
        Props {
            loop_focus: false,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    drop(service.send(Event::Focus(key("a"))));

    let result = service.send(Event::FocusPrev);

    assert!(!result.state_changed);
    assert_eq!(service.context().focused_tab.as_ref(), Some(&key("a")));
}

// ────────────────────────────────────────────────────────────────────
// 15–16. Home / End and all-disabled edge cases
// ────────────────────────────────────────────────────────────────────

#[test]
fn home_focuses_first_non_disabled_tab() {
    let mut disabled = BTreeSet::new();

    disabled.insert(key("a"));

    let mut service = service_with_tabs(
        Props {
            default_value: Some(key("c")),
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    drop(service.send(Event::Focus(key("c"))));
    drop(service.send(Event::FocusFirst));

    assert_eq!(service.context().focused_tab.as_ref(), Some(&key("b")));
}

#[test]
fn end_focuses_last_non_disabled_tab() {
    let mut disabled = BTreeSet::new();

    disabled.insert(key("c"));

    let mut service = service_with_tabs(
        Props {
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    drop(service.send(Event::FocusLast));

    assert_eq!(service.context().focused_tab.as_ref(), Some(&key("b")));
}

#[test]
fn focus_first_with_all_tabs_disabled_returns_no_op() {
    let mut disabled = BTreeSet::new();

    disabled.insert(key("a"));
    disabled.insert(key("b"));

    let mut service = service_with_tabs(
        Props {
            default_value: Some(key("a")),
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("a"), key("b")],
    );

    let result = service.send(Event::FocusFirst);

    assert!(!result.state_changed);
    assert!(!result.context_changed);
    assert_eq!(service.context().focused_tab, None);
}

#[test]
fn home_key_dispatches_focus_first() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("b"), &keydown(KeyboardKey::Home));

    assert_eq!(recorder.borrow().as_slice(), &[Event::FocusFirst]);
}

#[test]
fn end_key_dispatches_focus_last() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("a"), &keydown(KeyboardKey::End));

    assert_eq!(recorder.borrow().as_slice(), &[Event::FocusLast]);
}

// ────────────────────────────────────────────────────────────────────
// 17. SetDirection updates context
// ────────────────────────────────────────────────────────────────────

#[test]
fn set_direction_event_updates_context_dir() {
    let mut service = service_with_tabs(
        Props {
            dir: Direction::Auto,
            ..test_props()
        },
        &[],
    );

    assert_eq!(service.context().dir, Direction::Auto);

    let result = service.send(Event::SetDirection(Direction::Rtl));

    assert!(result.context_changed);
    assert_eq!(service.context().dir, Direction::Rtl);
}

#[test]
fn set_direction_event_no_op_when_already_resolved() {
    let mut service = service_with_tabs(
        Props {
            dir: Direction::Ltr,
            ..test_props()
        },
        &[],
    );

    let result = service.send(Event::SetDirection(Direction::Ltr));

    assert!(!result.context_changed);
}

// ────────────────────────────────────────────────────────────────────
// 18. FocusNext/Prev/etc. emit Effect::FocusFocusedTab
// ────────────────────────────────────────────────────────────────────

#[test]
fn focus_next_emits_focus_effect_in_focused_state() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    drop(service.send(Event::Focus(key("a"))));

    let result = service.send(Event::FocusNext);

    assert!(effect_names(&result).contains(&Effect::FocusFocusedTab));
    assert_eq!(service.context().focused_tab.as_ref(), Some(&key("b")));
}

#[test]
fn focus_first_emits_focus_effect() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let result = service.send(Event::FocusFirst);

    assert!(effect_names(&result).contains(&Effect::FocusFocusedTab));
}

#[test]
fn focus_last_emits_focus_effect() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let result = service.send(Event::FocusLast);

    assert!(effect_names(&result).contains(&Effect::FocusFocusedTab));
}

#[test]
fn focus_event_does_not_emit_focus_effect() {
    // DOM focus event is "focus already arrived" — we don't ask the
    // platform to focus again. Otherwise an external focus event would
    // create an infinite loop.
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let result = service.send(Event::Focus(key("b")));

    assert!(!effect_names(&result).contains(&Effect::FocusFocusedTab));
}

// ────────────────────────────────────────────────────────────────────
// 19–20. Closable close-trigger label messages
// ────────────────────────────────────────────────────────────────────

#[test]
fn close_trigger_default_label() {
    let service = service_with_tabs(test_props(), &[]);

    assert_snapshot!(
        "tabs_close_trigger_default_label",
        snapshot_attrs(&service.connect(&|_| {}).close_trigger_attrs("Inbox"))
    );
}

#[test]
fn close_trigger_custom_messages_label() {
    let messages = Messages {
        close_tab_label: MessageFn::new(|label: &str, _locale: &Locale| {
            let mut buffer = String::with_capacity("Dismiss ".len() + label.len());
            buffer.push_str("Dismiss ");
            buffer.push_str(label);
            buffer
        }),
        ..Messages::default()
    };

    let mut service = Service::<Machine>::new(test_props(), &Env::default(), &messages);

    drop(service.send(Event::SetTabs(vec![TabRegistration::closable(key("a"))])));

    assert_snapshot!(
        "tabs_close_trigger_custom_label",
        snapshot_attrs(&service.connect(&|_| {}).close_trigger_attrs("Drafts"))
    );
}

#[test]
fn reorder_announce_label_default_template() {
    let messages = Messages::default();

    assert_eq!(
        (messages.reorder_announce_label)("Inbox", 2, 5, &Env::default().locale),
        "Inbox moved to position 2 of 5"
    );
}

#[test]
fn reorder_announce_label_custom_template() {
    let messages = Messages {
        reorder_announce_label: MessageFn::new(
            |label: &str, position: usize, total: usize, _locale: &Locale| {
                let mut buffer = String::new();
                use core::fmt::Write as _;
                let _ = write!(buffer, "{label} → {position}/{total}");
                buffer
            },
        ),
        ..Messages::default()
    };

    assert_eq!(
        (messages.reorder_announce_label)("Drafts", 1, 3, &Env::default().locale),
        "Drafts → 1/3"
    );
}

// ────────────────────────────────────────────────────────────────────
// 21–22. Pass-through props (sanity)
// ────────────────────────────────────────────────────────────────────

#[test]
fn disallow_empty_selection_prop_pass_through() {
    let service = service_with_tabs(
        Props {
            disallow_empty_selection: true,
            ..test_props()
        },
        &[],
    );

    assert!(service.props().disallow_empty_selection);
}

#[test]
fn lazy_mount_prop_read_via_service_props() {
    let service = service_with_tabs(
        Props {
            lazy_mount: true,
            ..test_props()
        },
        &[],
    );

    assert!(service.props().lazy_mount);
}

#[test]
fn unmount_on_exit_prop_read_via_service_props() {
    let service = service_with_tabs(
        Props {
            unmount_on_exit: true,
            ..test_props()
        },
        &[],
    );

    assert!(service.props().unmount_on_exit);
}

// ────────────────────────────────────────────────────────────────────
// 23. ConnectApi::part_attrs round-trip
// ────────────────────────────────────────────────────────────────────

#[test]
fn connect_api_part_attrs_round_trip() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let api = service.connect(&|_| {});

    assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
    assert_eq!(api.part_attrs(Part::List), api.list_attrs());

    // ConnectApi defaults focus_visible to false; tab_attrs is the
    // direct call adapters use when they have ModalityContext data.
    assert_eq!(
        api.part_attrs(Part::Tab { tab_key: key("a") }),
        api.tab_attrs(&key("a"), false)
    );
    assert_eq!(
        api.part_attrs(Part::TabIndicator),
        api.tab_indicator_attrs()
    );
    assert_eq!(
        api.part_attrs(Part::Panel {
            tab_key: key("a"),
            tab_label: None,
        }),
        api.panel_attrs(&key("a"), None)
    );
    assert_eq!(
        api.part_attrs(Part::TabCloseTrigger {
            tab_label: "Inbox".into()
        }),
        api.close_trigger_attrs("Inbox")
    );
}

// ────────────────────────────────────────────────────────────────────
// 24–28. Closable variant — CloseTab is a pure notification
// ────────────────────────────────────────────────────────────────────

#[test]
fn close_tab_event_does_not_mutate_tabs() {
    let mut service = service_with_registrations(
        Props {
            default_value: Some(key("b")),
            ..test_props()
        },
        vec![
            TabRegistration::closable(key("a")),
            TabRegistration::closable(key("b")),
            TabRegistration::closable(key("c")),
        ],
    );

    let before = service.context().tabs.clone();

    drop(service.send(Event::CloseTab(key("a"))));

    assert_eq!(
        service.context().tabs,
        before,
        "CloseTab is a pure notification; consumer applies removal via SetTabs"
    );
    assert_eq!(service.context().value.get().as_ref(), Some(&key("b")));
}

#[test]
fn successor_for_close_picks_next() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b"), key("c")]);

    let api = service.connect(&|_| {});

    assert_eq!(api.successor_for_close(&key("a")), Some(key("b")));
    assert_eq!(api.successor_for_close(&key("b")), Some(key("c")));
}

#[test]
fn successor_for_close_falls_back_to_previous_when_last() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b"), key("c")]);

    let api = service.connect(&|_| {});

    assert_eq!(api.successor_for_close(&key("c")), Some(key("b")));
}

#[test]
fn successor_for_close_returns_none_for_only_tab() {
    let service = service_with_tabs(test_props(), &[key("a")]);

    let api = service.connect(&|_| {});

    assert_eq!(api.successor_for_close(&key("a")), None);
}

#[test]
fn successor_for_close_returns_none_for_unknown_key() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let api = service.connect(&|_| {});

    assert_eq!(api.successor_for_close(&key("ghost")), None);
}

#[test]
fn can_close_tab_when_disallow_empty_selection_off() {
    let service = service_with_tabs(test_props(), &[key("a")]);

    let api = service.connect(&|_| {});

    assert!(api.can_close_tab(&key("a")));
}

#[test]
fn can_close_tab_blocked_when_only_tab_with_disallow_empty_selection() {
    let service = service_with_tabs(
        Props {
            disallow_empty_selection: true,
            ..test_props()
        },
        &[key("a")],
    );

    let api = service.connect(&|_| {});

    assert!(!api.can_close_tab(&key("a")));
}

#[test]
fn can_close_tab_allowed_when_more_than_one_tab_with_disallow_empty_selection() {
    let service = service_with_tabs(
        Props {
            disallow_empty_selection: true,
            ..test_props()
        },
        &[key("a"), key("b")],
    );

    let api = service.connect(&|_| {});

    assert!(api.can_close_tab(&key("a")));
    assert!(api.can_close_tab(&key("b")));
}

#[test]
fn delete_key_emits_close_tab_for_closable_tab() {
    let service = service_with_registrations(
        test_props(),
        vec![
            TabRegistration::closable(key("a")),
            TabRegistration::closable(key("b")),
        ],
    );

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("b"), &keydown(KeyboardKey::Delete));

    assert_eq!(recorder.borrow().as_slice(), &[Event::CloseTab(key("b"))]);
}

#[test]
fn delete_key_no_op_for_non_closable_tab() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("b"), &keydown(KeyboardKey::Delete));

    assert!(
        recorder.borrow().is_empty(),
        "non-closable tabs ignore Delete"
    );
}

#[test]
fn backspace_key_emits_close_tab_for_closable_tab() {
    let service = service_with_registrations(
        test_props(),
        vec![
            TabRegistration::closable(key("a")),
            TabRegistration::closable(key("b")),
        ],
    );

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("a"), &keydown(KeyboardKey::Backspace));

    assert_eq!(recorder.borrow().as_slice(), &[Event::CloseTab(key("a"))]);
}

#[test]
fn on_close_trigger_click_emits_close_tab() {
    let service =
        service_with_registrations(test_props(), vec![TabRegistration::closable(key("a"))]);

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_close_trigger_click(&key("a"));

    assert_eq!(recorder.borrow().as_slice(), &[Event::CloseTab(key("a"))]);
}

// ────────────────────────────────────────────────────────────────────
// Reorderable variant (§6)
// ────────────────────────────────────────────────────────────────────

#[test]
fn reorderable_disabled_ctrl_arrow_falls_through_to_focus_next() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b"), key("c")]);

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("b"), &ctrl_keydown(KeyboardKey::ArrowRight));

    assert_eq!(recorder.borrow().as_slice(), &[Event::FocusNext]);
}

#[test]
fn reorderable_horizontal_ctrl_right_emits_reorder_to_next_index() {
    let service = service_with_tabs(
        Props {
            reorderable: true,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("b"), &ctrl_keydown(KeyboardKey::ArrowRight));

    assert_eq!(
        recorder.borrow().as_slice(),
        &[Event::ReorderTab {
            tab: key("b"),
            new_index: 2,
        }]
    );
}

#[test]
fn reorderable_horizontal_ctrl_left_clamped_at_zero() {
    let service = service_with_tabs(
        Props {
            reorderable: true,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("a"), &ctrl_keydown(KeyboardKey::ArrowLeft));

    assert!(
        recorder.borrow().is_empty(),
        "moving the first tab further left should be a no-op"
    );
}

#[test]
fn reorderable_horizontal_ctrl_arrow_is_direction_naive() {
    // Per spec §6.4: Ctrl+ArrowRight always means "move toward higher
    // index" regardless of `dir`. RTL-aware swapping applies only to
    // FOCUS navigation, not REORDER.
    let service = service_with_tabs(
        Props {
            reorderable: true,
            dir: Direction::Rtl,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("b"), &ctrl_keydown(KeyboardKey::ArrowRight));

    assert_eq!(
        recorder.borrow().as_slice(),
        &[Event::ReorderTab {
            tab: key("b"),
            new_index: 2,
        }],
        "Ctrl+ArrowRight increases index even in RTL"
    );
}

#[test]
fn reorderable_vertical_axis_uses_up_down() {
    let service = service_with_tabs(
        Props {
            orientation: Orientation::Vertical,
            reorderable: true,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("b"), &ctrl_keydown(KeyboardKey::ArrowDown));
    api.on_tab_keydown(&key("b"), &ctrl_keydown(KeyboardKey::ArrowUp));

    assert_eq!(
        recorder.borrow().as_slice(),
        &[
            Event::ReorderTab {
                tab: key("b"),
                new_index: 2,
            },
            Event::ReorderTab {
                tab: key("b"),
                new_index: 0,
            },
        ]
    );
}

#[test]
fn reorder_tab_event_does_not_mutate_context_tabs() {
    let mut service = service_with_tabs(
        Props {
            reorderable: true,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    let before = service.context().tabs.clone();

    drop(service.send(Event::ReorderTab {
        tab: key("a"),
        new_index: 2,
    }));

    assert_eq!(
        service.context().tabs,
        before,
        "agnostic core never mutates ctx.tabs on ReorderTab — consumer applies it"
    );
}

#[test]
fn reorderable_tab_attrs_includes_aria_roledescription() {
    let service = service_with_tabs(
        Props {
            reorderable: true,
            ..test_props()
        },
        &[key("a"), key("b")],
    );

    assert_snapshot!(
        "tabs_tab_reorderable",
        snapshot_attrs(&service.connect(&|_| {}).tab_attrs(&key("a"), false))
    );
}

#[test]
fn tab_registration_struct_round_trip() {
    let plain = TabRegistration::new(key("a"));

    assert_eq!(plain.key, key("a"));
    assert!(!plain.closable);

    let closable = TabRegistration::closable(key("b"));

    assert_eq!(closable.key, key("b"));
    assert!(closable.closable);
}

// ────────────────────────────────────────────────────────────────────
// Props fluent builder
// ────────────────────────────────────────────────────────────────────

// ────────────────────────────────────────────────────────────────────
// Mutation-coverage tests — each test is sized to kill a specific
// mutation that the broader behavioral suite missed. Keeping them
// adjacent makes future re-runs easy to triage.
// ────────────────────────────────────────────────────────────────────

#[test]
fn step_focus_next_advances_one_position_in_non_loop_mode() {
    // Catches `index + 1 < total` → `index + 1 > total` (step_focus
    // would clamp at the FIRST tab instead of advancing).
    // Catches `index + 1` → `index - 1` / `index * 1` (would underflow
    // or stay put instead of advancing).
    let mut service = service_with_tabs(
        Props {
            loop_focus: false,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    drop(service.send(Event::Focus(key("a"))));
    drop(service.send(Event::FocusNext));

    assert_eq!(
        service.context().focused_tab.as_ref(),
        Some(&key("b")),
        "FocusNext from index 0 must advance to index 1 in non-loop mode"
    );
}

#[test]
fn step_focus_prev_decrements_one_position_in_non_loop_mode() {
    // Catches `index - 1` → `index + 1` / `index / 1` mutations.
    let mut service = service_with_tabs(
        Props {
            loop_focus: false,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    drop(service.send(Event::Focus(key("b"))));
    drop(service.send(Event::FocusPrev));

    assert_eq!(
        service.context().focused_tab.as_ref(),
        Some(&key("a")),
        "FocusPrev from index 1 must step to index 0 in non-loop mode"
    );
}

#[test]
fn step_focus_prev_decrements_one_position_in_loop_mode_when_not_at_start() {
    // Catches `index - 1` → `index + 1` / `index / 1` mutations on
    // line 934 (the loop-mode `else` branch — taken when index > 0).
    // The wrap-at-start test only exercises the `if index == 0` branch.
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b"), key("c")]);

    drop(service.send(Event::Focus(key("b"))));
    drop(service.send(Event::FocusPrev));

    assert_eq!(
        service.context().focused_tab.as_ref(),
        Some(&key("a")),
        "FocusPrev from index 1 in loop mode must step to index 0"
    );
}

#[test]
fn step_focus_prev_wraps_to_last_in_loop_mode_with_correct_index() {
    // Catches `total - 1` → `total + 1` / `total / 1` mutations.
    // `total + 1` / `total` would index out of bounds → panic.
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b"), key("c")]);

    drop(service.send(Event::Focus(key("a"))));
    drop(service.send(Event::FocusPrev));

    assert_eq!(
        service.context().focused_tab.as_ref(),
        Some(&key("c")),
        "FocusPrev from index 0 in loop mode must wrap to total-1, not total or total/1"
    );
}

#[test]
fn focus_next_in_auto_mode_advances_value_alongside_focus() {
    // Catches `auto = activation_mode == Automatic` → `!=` mutation in
    // focus_to. With the mutation, Auto-mode FocusNext would NOT
    // advance value, and Manual-mode FocusNext WOULD.
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b"), key("c")]);

    drop(service.send(Event::Focus(key("a"))));
    drop(service.send(Event::FocusNext));

    assert_eq!(
        service.context().value.get().as_ref(),
        Some(&key("b")),
        "Auto mode: FocusNext must advance value"
    );
}

#[test]
fn focus_next_in_manual_mode_does_not_advance_value() {
    // Companion to the test above — Manual mode keeps value pinned
    // even as focused_tab moves.
    let mut service = service_with_tabs(
        Props {
            activation_mode: ActivationMode::Manual,
            ..test_props()
        },
        &[key("a"), key("b")],
    );

    drop(service.send(Event::Focus(key("a"))));
    drop(service.send(Event::FocusNext));

    assert_eq!(service.context().focused_tab.as_ref(), Some(&key("b")));
    assert_eq!(
        service.context().value.get().as_ref(),
        Some(&key("a")),
        "Manual mode: FocusNext must NOT advance value"
    );
}

#[test]
fn snap_value_invalidates_when_value_not_in_new_registration() {
    // Catches `k == *key` → `!=` mutation in snap_value_to_valid_key.
    // The mutation only diverges when `value` is set AND not present
    // in `tabs` AND not disabled. With `==`, the filter rejects (no
    // matching tab) → snap fires. With `!=`, the filter accepts as
    // long as any other tab exists → snap is bypassed.
    let mut service = Service::<Machine>::new(
        Props {
            default_value: Some(key("a")),
            ..test_props()
        },
        &Env::default(),
        &Messages::default(),
    );

    // First registration includes 'a' — snap keeps value at Some(a).
    drop(service.send(Event::SetTabs(vec![
        TabRegistration::new(key("a")),
        TabRegistration::new(key("b")),
        TabRegistration::new(key("c")),
    ])));

    assert_eq!(service.context().value.get().as_ref(), Some(&key("a")));

    // Second registration drops 'a' — snap MUST advance value to the
    // first non-disabled tab in the new list (`b`).
    drop(service.send(Event::SetTabs(vec![
        TabRegistration::new(key("b")),
        TabRegistration::new(key("c")),
    ])));

    assert_eq!(
        service.context().value.get().as_ref(),
        Some(&key("b")),
        "value pointing at unregistered key must snap to first non-disabled"
    );
}

#[test]
fn snap_value_replaces_value_pointing_at_unregistered_key() {
    // Catches the `==` → `!=` mutation in snap_value_to_valid_key.
    // The filter must REJECT a value that isn't in `tabs` so snap
    // promotes to the first non-disabled key. With the mutation
    // (`any(|k| k != *key)`), filter would always pass and `valid`
    // would stay `Some(invalid)` — the snap would never promote.
    //
    // Setup: construct directly (no service_with_tabs helper, which
    // pre-snaps via the empty SetTabs and would mask the mutation).
    let mut service = Service::<Machine>::new(
        Props {
            default_value: Some(key("ghost")),
            ..test_props()
        },
        &Env::default(),
        &Messages::default(),
    );

    assert_eq!(service.context().value.get().as_ref(), Some(&key("ghost")));

    drop(service.send(Event::SetTabs(vec![
        TabRegistration::new(key("a")),
        TabRegistration::new(key("b")),
        TabRegistration::new(key("c")),
    ])));

    assert_eq!(
        service.context().value.get().as_ref(),
        Some(&key("a")),
        "snap must replace unregistered value with first non-disabled key"
    );
}

#[test]
fn snap_value_keeps_valid_key_when_still_in_list() {
    // Catches the inverse — when value IS still registered, snap must
    // NOT move it to the first key in a re-ordered list.
    //
    // Setup must register a non-empty list FIRST (so value is already
    // valid against `tabs`) then re-register in a different order. A
    // helper that registers an empty list as a side effect would
    // already null `value` via the snap and mask the mutation.
    let mut service = Service::<Machine>::new(
        Props {
            default_value: Some(key("a")),
            ..test_props()
        },
        &Env::default(),
        &Messages::default(),
    );

    // First registration includes 'a' — snap keeps value at Some(a).
    drop(service.send(Event::SetTabs(vec![
        TabRegistration::new(key("a")),
        TabRegistration::new(key("b")),
        TabRegistration::new(key("c")),
    ])));

    assert_eq!(service.context().value.get().as_ref(), Some(&key("a")));

    // Re-register in a different order — 'a' is still valid, snap
    // must not move value to the first registered key (c).
    drop(service.send(Event::SetTabs(vec![
        TabRegistration::new(key("c")),
        TabRegistration::new(key("a")),
        TabRegistration::new(key("b")),
    ])));

    assert_eq!(
        service.context().value.get().as_ref(),
        Some(&key("a")),
        "valid registered+enabled value must not be snapped to first key"
    );
}

#[test]
fn snap_focused_tab_keeps_valid_key_when_still_in_list() {
    // Catches `delete !` in `!is_disabled(ctx, key)` inside
    // snap_focused_tab_to_valid_key. With the mutation, focused_tab
    // is treated as still_valid only if disabled — i.e. always cleared
    // for non-disabled tabs.
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    drop(service.send(Event::Focus(key("b"))));

    // Re-register the same set in a different order — focused_tab
    // (b) is still valid.
    drop(service.send(Event::SetTabs(vec![
        TabRegistration::new(key("b")),
        TabRegistration::new(key("a")),
    ])));

    assert_eq!(
        service.context().focused_tab.as_ref(),
        Some(&key("b")),
        "focused_tab must persist across re-registration when still valid"
    );
}

#[test]
fn api_selected_tab_returns_actual_value() {
    // Catches `Api::selected_tab -> Option<&Key>` replaced with
    // `None` / leaked default.
    let service = service_with_tabs(
        Props {
            default_value: Some(key("b")),
            ..test_props()
        },
        &[key("a"), key("b")],
    );

    let api = service.connect(&|_| {});

    assert_eq!(api.selected_tab(), Some(&key("b")));
}

#[test]
fn api_focused_tab_reflects_context_state() {
    // Catches `Api::focused_tab -> Option<&Key>` replaced with `None`.
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    drop(service.send(Event::Focus(key("b"))));

    assert_eq!(service.connect(&|_| {}).focused_tab(), Some(&key("b")));
}

#[test]
fn is_tablist_focus_fallback_anchors_first_non_disabled_tab() {
    // Catches `is_tablist_focus_fallback -> bool with false` (would
    // never anchor any tab → list unreachable) and the `delete !` /
    // `==` → `!=` mutations on the filter chain.
    let mut disabled = BTreeSet::new();

    disabled.insert(key("a"));

    let service = service_with_tabs(
        Props {
            value: Some(None),
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    let api = service.connect(&|_| {});

    // First non-disabled tab is `b`. A is disabled (must NOT anchor),
    // C is non-disabled but not first (must NOT anchor).
    assert_eq!(
        api.tab_attrs(&key("a"), false).get(&HtmlAttr::TabIndex),
        Some("-1"),
    );
    assert_eq!(
        api.tab_attrs(&key("b"), false).get(&HtmlAttr::TabIndex),
        Some("0"),
    );
    assert_eq!(
        api.tab_attrs(&key("c"), false).get(&HtmlAttr::TabIndex),
        Some("-1"),
    );
}

#[test]
fn is_tablist_focus_fallback_disabled_when_any_tab_is_selected() {
    // The fallback is "value is None" — when something is selected,
    // no tab should claim the fallback role.
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let api = service.connect(&|_| {});

    // `a` is selected (default_value), `b` should NOT anchor.
    assert_eq!(
        api.tab_attrs(&key("b"), false).get(&HtmlAttr::TabIndex),
        Some("-1"),
    );
}

#[test]
fn on_tab_click_dispatches_select_tab_event() {
    // Catches `Api::on_tab_click with ()` (replaced with no-op).
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_click(&key("b"));

    assert_eq!(recorder.borrow().as_slice(), &[Event::SelectTab(key("b"))]);
}

#[test]
fn on_tab_focus_dispatches_focus_event() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_focus(&key("a"));

    assert_eq!(recorder.borrow().as_slice(), &[Event::Focus(key("a"))]);
}

#[test]
fn on_tab_blur_dispatches_blur_event() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_blur();

    assert_eq!(recorder.borrow().as_slice(), &[Event::Blur]);
}

#[test]
fn delete_on_closable_tab_dispatches_close_tab() {
    // Catches `&&` → `||` in
    // `(Delete | Backspace) && closable_tabs.contains(tab_key)`.
    // With `||` the agnostic core would also dispatch Delete/Backspace
    // on non-closable tabs.
    let service = service_with_registrations(
        test_props(),
        vec![
            TabRegistration::closable(key("a")),
            TabRegistration::new(key("b")), // non-closable
        ],
    );

    // Closable tab `a`: dispatches.
    let recorder: EventRecorder = RefCell::new(Vec::new());

    {
        let send = |event| record(&recorder, event);

        let api = service.connect(&send);

        api.on_tab_keydown(&key("a"), &keydown(KeyboardKey::Delete));
    }

    assert_eq!(recorder.borrow().as_slice(), &[Event::CloseTab(key("a"))]);

    // Non-closable tab `b`: NO dispatch.
    let recorder: EventRecorder = RefCell::new(Vec::new());

    {
        let send = |event| record(&recorder, event);

        let api = service.connect(&send);

        api.on_tab_keydown(&key("b"), &keydown(KeyboardKey::Delete));
    }

    assert!(
        recorder.borrow().is_empty(),
        "Delete on non-closable tab must not dispatch CloseTab"
    );
}

#[test]
fn step_focus_returns_none_when_tab_list_is_empty() {
    // Reaches the `total == 0` early-return branch in step_focus.
    // After SetTabs([]), ctx.tabs is empty but state stays
    // Focused{prev_tab} (apply only mutates context).
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    drop(service.send(Event::Focus(key("a"))));

    drop(service.send(Event::SetTabs(Vec::new())));

    // State is still Focused{a} but ctx.tabs is now empty.
    let result = service.send(Event::FocusNext);

    assert!(!result.state_changed);
}

#[test]
fn api_debug_fmt_compiles_and_runs() {
    // Exercises the manually-implemented Debug for Api<'_>. Without
    // a test, the impl is a coverage hole and changes there go silently.
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let api = service.connect(&|_| {});

    let formatted = format!("{api:?}");

    assert!(formatted.starts_with("Api {"));
    assert!(formatted.contains("state"));
    assert!(formatted.contains("ctx"));
    assert!(formatted.contains("props"));
}

#[test]
fn reorderable_horizontal_ctrl_vertical_arrow_ignored() {
    // Reaches the `_ => None` arm in the horizontal `reorder_axis_match`
    // (Ctrl+ArrowUp / ArrowDown on a horizontal reorderable list are
    // off-axis — neither dispatches a reorder).
    let service = service_with_tabs(
        Props {
            reorderable: true,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("b"), &ctrl_keydown(KeyboardKey::ArrowUp));
    api.on_tab_keydown(&key("b"), &ctrl_keydown(KeyboardKey::ArrowDown));

    assert!(
        recorder.borrow().is_empty(),
        "off-axis Ctrl+Arrow on horizontal reorderable list must not dispatch"
    );
}

#[test]
fn reorderable_vertical_ctrl_horizontal_arrow_ignored() {
    // Symmetric: reaches the `_ => None` arm in the vertical
    // `reorder_axis_match`.
    let service = service_with_tabs(
        Props {
            orientation: Orientation::Vertical,
            reorderable: true,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_tab_keydown(&key("b"), &ctrl_keydown(KeyboardKey::ArrowLeft));
    api.on_tab_keydown(&key("b"), &ctrl_keydown(KeyboardKey::ArrowRight));

    assert!(
        recorder.borrow().is_empty(),
        "off-axis Ctrl+Arrow on vertical reorderable list must not dispatch"
    );
}

#[test]
fn step_focus_terminates_when_focused_tab_becomes_disabled_at_runtime() {
    // Catches `checked += 1` → `checked *= 1` in step_focus. With the
    // mutation, the all-disabled exit guard never trips and the loop
    // runs forever; cargo-mutants times this out, but only if a test
    // actually drives the all-disabled-from-Focused path.
    //
    // Reachability: focus a tab while enabled, then disable it via
    // SyncProps. State stays `Focused { a }` (apply only mutates
    // context); next FocusNext exercises the loop with all tabs
    // disabled.
    let mut service = service_with_tabs(test_props(), &[key("a")]);

    drop(service.send(Event::Focus(key("a"))));

    let mut disabled = BTreeSet::new();

    disabled.insert(key("a"));

    service.set_props(Props {
        disabled_keys: disabled,
        ..test_props()
    });

    let result = service.send(Event::FocusNext);

    assert!(
        !result.state_changed,
        "FocusNext with all tabs disabled must terminate via the guard"
    );
}

#[test]
fn next_reorder_index_arithmetic() {
    // Catches `position + 1 < total` → `<=` (would emit a reorder
    // when at the last index, which would then be out-of-bounds when
    // the consumer applies it).
    // Catches `position + 1` → `position - 1` / `position * 1`.
    let service = service_with_tabs(
        Props {
            reorderable: true,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    // From last index — must clamp (no event).
    api.on_tab_keydown(&key("c"), &ctrl_keydown(KeyboardKey::ArrowRight));

    assert!(recorder.borrow().is_empty(), "no reorder past last index");

    // From middle — must emit position + 1 (= 2), not position - 1 (= 0)
    // and not position * 1 (= 1, no movement).
    api.on_tab_keydown(&key("b"), &ctrl_keydown(KeyboardKey::ArrowRight));

    assert_eq!(
        recorder.borrow().as_slice(),
        &[Event::ReorderTab {
            tab: key("b"),
            new_index: 2,
        }]
    );
}

#[test]
fn props_builder_round_trips_every_field() {
    let mut disabled = BTreeSet::new();

    disabled.insert(key("x"));

    let props = Props::new()
        .id("tabs-builder")
        .value(Some(Some(key("a"))))
        .default_value(Some(key("a")))
        .orientation(Orientation::Vertical)
        .activation_mode(ActivationMode::Manual)
        .dir(Direction::Rtl)
        .loop_focus(false)
        .disallow_empty_selection(true)
        .lazy_mount(true)
        .unmount_on_exit(true)
        .disabled_keys(disabled.clone())
        .reorderable(true);

    assert_eq!(props.id, "tabs-builder");
    assert_eq!(props.value, Some(Some(key("a"))));
    assert_eq!(props.default_value, Some(key("a")));
    assert_eq!(props.orientation, Orientation::Vertical);
    assert_eq!(props.activation_mode, ActivationMode::Manual);
    assert_eq!(props.dir, Direction::Rtl);
    assert!(!props.loop_focus);
    assert!(props.disallow_empty_selection);
    assert!(props.lazy_mount);
    assert!(props.unmount_on_exit);
    assert_eq!(props.disabled_keys, disabled);
    assert!(props.reorderable);
}

// ────────────────────────────────────────────────────────────────────
// Snapshot tests for the remaining anatomy parts
// ────────────────────────────────────────────────────────────────────

#[test]
fn snapshot_root_horizontal_ltr() {
    let service = service_with_tabs(test_props(), &[]);

    assert_snapshot!(
        "tabs_root_horizontal_ltr",
        snapshot_attrs(&service.connect(&|_| {}).root_attrs())
    );
}

#[test]
fn snapshot_root_vertical_rtl() {
    let service = service_with_tabs(
        Props {
            orientation: Orientation::Vertical,
            dir: Direction::Rtl,
            ..test_props()
        },
        &[],
    );

    assert_snapshot!(
        "tabs_root_vertical_rtl",
        snapshot_attrs(&service.connect(&|_| {}).root_attrs())
    );
}

#[test]
fn snapshot_root_auto_dir() {
    let service = service_with_tabs(
        Props {
            dir: Direction::Auto,
            ..test_props()
        },
        &[],
    );

    assert_snapshot!(
        "tabs_root_auto_dir",
        snapshot_attrs(&service.connect(&|_| {}).root_attrs())
    );
}

#[test]
fn snapshot_tab_indicator() {
    let service = service_with_tabs(test_props(), &[]);

    assert_snapshot!(
        "tabs_tab_indicator",
        snapshot_attrs(&service.connect(&|_| {}).tab_indicator_attrs())
    );
}

#[test]
fn snapshot_root_vertical_ltr() {
    // Covers the vertical+LTR root combo. Vertical orientation is
    // direction-neutral for arrow navigation, but `dir` still flows
    // through to the rendered Root attrs.
    let service = service_with_tabs(
        Props {
            orientation: Orientation::Vertical,
            ..test_props()
        },
        &[],
    );

    assert_snapshot!(
        "tabs_root_vertical_ltr",
        snapshot_attrs(&service.connect(&|_| {}).root_attrs())
    );
}

#[test]
fn snapshot_list_vertical_rtl() {
    // Catches any future change that lets `aria-orientation` track
    // `dir` (it shouldn't — orientation is the layout axis, dir is
    // text direction).
    let service = service_with_tabs(
        Props {
            orientation: Orientation::Vertical,
            dir: Direction::Rtl,
            ..test_props()
        },
        &[],
    );

    assert_snapshot!(
        "tabs_list_vertical_rtl",
        snapshot_attrs(&service.connect(&|_| {}).list_attrs())
    );
}

#[test]
fn snapshot_tab_reorderable_selected_focus_visible() {
    // Combines every output-affecting flag the tab triggers carries
    // (reorderable + selected + focused + keyboard modality). Catches
    // regressions where adding a new flag accidentally drops one of
    // the existing data attrs.
    let mut service = service_with_tabs(
        Props {
            reorderable: true,
            ..test_props()
        },
        &[key("a"), key("b")],
    );

    drop(service.send(Event::Focus(key("a"))));

    assert_snapshot!(
        "tabs_tab_reorderable_selected_focus_visible",
        snapshot_attrs(&service.connect(&|_| {}).tab_attrs(&key("a"), true))
    );
}

#[test]
fn snapshot_tab_reorderable_disabled() {
    // `aria-roledescription="draggable tab"` applies to every tab when
    // reorderable, including disabled ones — disabled tabs are
    // visually present and screen-readers should still discover the
    // affordance. Captures the interaction.
    let mut disabled = BTreeSet::new();

    disabled.insert(key("b"));

    let service = service_with_tabs(
        Props {
            reorderable: true,
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("a"), key("b")],
    );

    assert_snapshot!(
        "tabs_tab_reorderable_disabled",
        snapshot_attrs(&service.connect(&|_| {}).tab_attrs(&key("b"), false))
    );
}

#[test]
fn snapshot_panel_reorderable_selected() {
    // Panels do NOT inherit reorderable's `aria-roledescription` —
    // only tab triggers carry it. Snapshot pins the contract so a
    // future refactor doesn't accidentally leak the attribute.
    let service = service_with_tabs(
        Props {
            reorderable: true,
            ..test_props()
        },
        &[key("a"), key("b")],
    );

    assert_snapshot!(
        "tabs_panel_reorderable_selected",
        snapshot_attrs(&service.connect(&|_| {}).panel_attrs(&key("a"), None))
    );
}

#[test]
fn snapshot_close_trigger_unicode_label() {
    // Stresses `Messages::close_tab_label` with a multi-byte label.
    // Catches any future refactor that pre-computes the label length
    // in bytes vs. graphemes incorrectly.
    let service = service_with_tabs(test_props(), &[]);

    assert_snapshot!(
        "tabs_close_trigger_unicode_label",
        snapshot_attrs(&service.connect(&|_| {}).close_trigger_attrs("受信箱"))
    );
}

// ────────────────────────────────────────────────────────────────────
// Init smoke tests
// ────────────────────────────────────────────────────────────────────

#[test]
fn init_default_value_uncontrolled() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    assert_eq!(service.state(), &State::Idle);
    assert_eq!(service.context().value.get().as_ref(), Some(&key("a")));
    assert_eq!(service.context().focused_tab, None);
    assert_eq!(service.context().ids.part("tablist"), "tabs-tablist");
}

#[test]
fn init_default_value_none_starts_unselected() {
    let service = service_with_tabs(
        Props {
            default_value: None,
            ..test_props()
        },
        &[],
    );

    assert_eq!(service.context().value.get(), &None);
}

#[test]
fn init_controlled_value_overrides_default() {
    let service = service_with_tabs(
        Props {
            value: Some(Some(key("b"))),
            default_value: Some(key("a")),
            ..test_props()
        },
        &[key("a"), key("b")],
    );

    assert_eq!(service.context().value.get().as_ref(), Some(&key("b")));
    assert!(service.context().value.is_controlled());
}

#[test]
fn init_disabled_keys_populate_disabled_tabs_map() {
    let mut disabled = BTreeSet::new();

    disabled.insert(key("a"));
    disabled.insert(key("c"));

    let service = service_with_tabs(
        Props {
            disabled_keys: disabled,
            ..test_props()
        },
        &[],
    );

    assert_eq!(service.context().disabled_tabs.len(), 2);
    assert!(service.context().disabled_tabs.contains(&key("a")));
    assert!(service.context().disabled_tabs.contains(&key("c")));
}

// ────────────────────────────────────────────────────────────────────
// Blur transition
// ────────────────────────────────────────────────────────────────────

#[test]
fn blur_event_returns_to_idle() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    drop(service.send(Event::Focus(key("a"))));

    assert!(matches!(service.state(), State::Focused { .. }));

    let result = service.send(Event::Blur);

    assert!(result.state_changed);
    assert_eq!(service.state(), &State::Idle);
    assert_eq!(service.context().focused_tab, None);
}

#[test]
fn blur_in_idle_with_no_focused_tab_is_no_op() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let result = service.send(Event::Blur);

    assert!(!result.state_changed);
    assert!(!result.context_changed);
}

// ────────────────────────────────────────────────────────────────────
// Bootstrap from Idle
// ────────────────────────────────────────────────────────────────────

#[test]
fn focus_next_from_idle_seeds_to_selected_tab() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b"), key("c")]);

    let result = service.send(Event::FocusNext);

    assert!(result.state_changed);
    assert_eq!(service.context().focused_tab.as_ref(), Some(&key("a")));
    assert!(effect_names(&result).contains(&Effect::FocusFocusedTab));
}

#[test]
fn focus_next_from_idle_no_op_when_value_is_none() {
    let mut service = service_with_tabs(
        Props {
            default_value: None,
            ..test_props()
        },
        &[],
    );

    let result = service.send(Event::FocusNext);

    assert!(!result.state_changed);
}

#[test]
fn focus_next_from_idle_no_op_when_value_points_at_unregistered_key() {
    // Controlled value points at "ghost"; tabs only has [a, b]. The
    // Idle bootstrap arm must reject because the target is not
    // registered. (Catches an `||` → `&&` regression in the
    // not-registered-or-disabled guard.)
    let mut service = Service::<Machine>::new(
        Props {
            value: Some(Some(key("ghost"))),
            ..test_props()
        },
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::SetTabs(vec![
        TabRegistration::new(key("a")),
        TabRegistration::new(key("b")),
    ])));

    // Sanity: controlled value still points at ghost despite snap.
    assert_eq!(service.context().value.get().as_ref(), Some(&key("ghost")));

    let result = service.send(Event::FocusNext);

    assert!(!result.state_changed);
    assert!(!result.context_changed);
}

#[test]
fn focus_next_from_idle_no_op_when_value_points_at_disabled_key() {
    let mut disabled = BTreeSet::new();

    disabled.insert(key("b"));

    let mut service = Service::<Machine>::new(
        Props {
            value: Some(Some(key("b"))),
            disabled_keys: disabled,
            ..test_props()
        },
        &Env::default(),
        &Messages::default(),
    );

    drop(service.send(Event::SetTabs(vec![
        TabRegistration::new(key("a")),
        TabRegistration::new(key("b")),
    ])));

    assert_eq!(service.context().value.get().as_ref(), Some(&key("b")));

    let result = service.send(Event::FocusNext);

    assert!(!result.state_changed);
}

// ────────────────────────────────────────────────────────────────────
// SetTabs — registration + selection invariant
// ────────────────────────────────────────────────────────────────────

#[test]
fn set_tabs_replaces_registered_tab_list() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    drop(service.send(Event::SetTabs(vec![
        TabRegistration::new(key("x")),
        TabRegistration::new(key("y")),
    ])));

    assert_eq!(
        service.context().tabs.as_slice(),
        &[key("x"), key("y")] as &[Key]
    );
}

#[test]
fn set_tabs_populates_closable_tabs_set() {
    let mut service = service_with_tabs(test_props(), &[]);

    drop(service.send(Event::SetTabs(vec![
        TabRegistration::closable(key("a")),
        TabRegistration::new(key("b")),
        TabRegistration::closable(key("c")),
    ])));

    assert!(service.context().closable_tabs.contains(&key("a")));
    assert!(!service.context().closable_tabs.contains(&key("b")));
    assert!(service.context().closable_tabs.contains(&key("c")));
}

#[test]
fn set_tabs_snaps_value_to_first_non_disabled_when_invalid() {
    // value points at "a" but registration only has "b" / "c" → snap to "b".
    let mut service = service_with_tabs(test_props(), &[]);

    drop(service.send(Event::SetTabs(vec![
        TabRegistration::new(key("b")),
        TabRegistration::new(key("c")),
    ])));

    assert_eq!(service.context().value.get().as_ref(), Some(&key("b")));
}

#[test]
fn set_tabs_keeps_value_when_still_valid() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    // Reorder doesn't invalidate "a".
    drop(service.send(Event::SetTabs(vec![
        TabRegistration::new(key("b")),
        TabRegistration::new(key("a")),
    ])));

    assert_eq!(service.context().value.get().as_ref(), Some(&key("a")));
}

#[test]
fn set_tabs_clears_value_when_no_non_disabled_remains() {
    let mut disabled = BTreeSet::new();

    disabled.insert(key("x"));

    let mut service = service_with_tabs(
        Props {
            disabled_keys: disabled,
            ..test_props()
        },
        &[],
    );

    drop(service.send(Event::SetTabs(vec![TabRegistration::new(key("x"))])));

    assert_eq!(
        service.context().value.get(),
        &None,
        "every registered tab is disabled → value snaps to None"
    );
}

#[test]
fn set_tabs_clears_focused_tab_when_no_longer_in_list() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b"), key("c")]);

    drop(service.send(Event::Focus(key("c"))));

    drop(service.send(Event::SetTabs(vec![
        TabRegistration::new(key("a")),
        TabRegistration::new(key("b")),
    ])));

    assert_eq!(service.context().focused_tab, None);
}

#[test]
fn set_tabs_downgrades_state_to_idle_when_focused_tab_removed() {
    // The State→focused_tab invariant: `State::Focused { tab }` must
    // always have `ctx.focused_tab == Some(tab)`. When SetTabs clears
    // the focused tab via the snap, state drops to Idle to keep the
    // invariant intact.
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    drop(service.send(Event::Focus(key("b"))));

    assert!(matches!(service.state(), State::Focused { .. }));

    drop(service.send(Event::SetTabs(vec![TabRegistration::new(key("a"))])));

    assert_eq!(service.state(), &State::Idle);
    assert_eq!(service.context().focused_tab, None);
}

#[test]
fn sync_props_keeps_focused_state_when_focused_tab_remains_valid() {
    // Catches the `==` → `!=` mutation in `sync_props_plan`'s
    // `still_present` check. With a single registered tab `a`,
    // `any(|k| k == a)` is true and `any(|k| k != a)` is false. The
    // mutation would treat `a` as no-longer-present and downgrade
    // state to Idle even when `a` is still registered and enabled.
    let mut service = service_with_tabs(test_props(), &[key("a")]);

    drop(service.send(Event::Focus(key("a"))));

    assert!(matches!(service.state(), State::Focused { .. }));

    // SyncProps with a non-disabled-keys change — state must remain
    // Focused because `a` is still registered and not disabled.
    service.set_props(Props {
        orientation: Orientation::Vertical,
        ..test_props()
    });

    assert!(
        matches!(service.state(), State::Focused { .. }),
        "Focused state must survive SyncProps when the focused tab \
         is still registered and enabled"
    );
    assert_eq!(service.context().focused_tab.as_ref(), Some(&key("a")));
}

#[test]
fn sync_props_downgrades_state_to_idle_when_focused_tab_disabled() {
    // Same invariant as SetTabs: when SyncProps' rebuilt
    // `disabled_tabs` includes the focused tab, snap clears it AND
    // state drops to Idle.
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    drop(service.send(Event::Focus(key("b"))));

    assert!(matches!(service.state(), State::Focused { .. }));

    let mut newly_disabled = BTreeSet::new();

    newly_disabled.insert(key("b"));

    service.set_props(Props {
        disabled_keys: newly_disabled,
        ..test_props()
    });

    assert_eq!(service.state(), &State::Idle);
    assert_eq!(service.context().focused_tab, None);
}

#[test]
fn set_tabs_keeps_focused_state_when_re_registering_same_single_tab() {
    // Catches the `==` → `!=` mutation in `set_tabs_plan`'s
    // `still_present` check. With a single registration matching the
    // focused tab, `any(|r| r.key == tab)` is true and `any(|r| r.key
    // != tab)` is false. The mutation would treat the focused tab as
    // no-longer-present and downgrade state to Idle.
    let mut service = service_with_tabs(test_props(), &[key("a")]);

    drop(service.send(Event::Focus(key("a"))));

    assert!(matches!(service.state(), State::Focused { .. }));

    drop(service.send(Event::SetTabs(vec![TabRegistration::new(key("a"))])));

    assert!(
        matches!(service.state(), State::Focused { .. }),
        "Focused state must survive SetTabs when the focused tab is \
         still in the new registration"
    );
}

#[test]
fn set_tabs_keeps_focused_state_when_focused_tab_still_valid() {
    // Negative case: when the focused tab survives the snap, state
    // stays Focused.
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b"), key("c")]);

    drop(service.send(Event::Focus(key("b"))));

    drop(service.send(Event::SetTabs(vec![
        TabRegistration::new(key("c")),
        TabRegistration::new(key("b")),
        TabRegistration::new(key("a")),
    ])));

    assert!(matches!(service.state(), State::Focused { .. }));
    assert_eq!(service.context().focused_tab.as_ref(), Some(&key("b")));
}

#[test]
fn set_tabs_deduplicates_by_key() {
    let mut service = service_with_tabs(test_props(), &[]);

    drop(service.send(Event::SetTabs(vec![
        TabRegistration::new(key("a")),
        TabRegistration::closable(key("b")),
        TabRegistration::new(key("a")), // duplicate
        TabRegistration::new(key("b")), // duplicate
    ])));

    assert_eq!(
        service.context().tabs.as_slice(),
        &[key("a"), key("b")] as &[Key],
        "duplicate keys deduped, first occurrence wins"
    );

    // The first occurrence of `b` was closable; the duplicate didn't
    // override it.
    assert!(service.context().closable_tabs.contains(&key("b")));
}

// ────────────────────────────────────────────────────────────────────
// Unknown-key guards
// ────────────────────────────────────────────────────────────────────

#[test]
fn select_tab_unknown_key_no_op() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let result = service.send(Event::SelectTab(key("ghost")));

    assert!(!result.state_changed);
    assert!(!result.context_changed);
    assert_eq!(service.context().value.get().as_ref(), Some(&key("a")));
}

#[test]
fn focus_unknown_key_no_op() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let result = service.send(Event::Focus(key("ghost")));

    assert!(!result.state_changed);
    assert!(!result.context_changed);
    assert_eq!(service.context().focused_tab, None);
}

#[test]
fn can_close_tab_unknown_key_returns_false() {
    let service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    let api = service.connect(&|_| {});

    assert!(!api.can_close_tab(&key("ghost")));
}

// ────────────────────────────────────────────────────────────────────
// Tablist tabindex fallback when no tab is selected
// ────────────────────────────────────────────────────────────────────

#[test]
fn tab_attrs_renders_tabindex_zero_for_first_non_disabled_when_value_is_none() {
    // Controlled `value: Some(None)` overrides Bindable's internal so
    // the SetTabs snap (which only updates internal) cannot raise
    // value back to a registered tab — value stays as None at the
    // observable layer, and the fallback anchor must engage.
    let service = service_with_tabs(
        Props {
            value: Some(None),
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    let api = service.connect(&|_| {});

    // Sanity: controlled-None value remains None despite the SetTabs snap.
    assert_eq!(api.selected_tab(), None);

    assert_eq!(
        api.tab_attrs(&key("a"), false).get(&HtmlAttr::TabIndex),
        Some("0"),
        "first non-disabled tab anchors the roving tabindex when value=None"
    );
    assert_eq!(
        api.tab_attrs(&key("b"), false).get(&HtmlAttr::TabIndex),
        Some("-1"),
    );
}

#[test]
fn tab_attrs_fallback_skips_disabled_first_tab() {
    let mut disabled = BTreeSet::new();

    disabled.insert(key("a"));

    let service = service_with_tabs(
        Props {
            value: Some(None),
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    let api = service.connect(&|_| {});

    // Disabled `a` is skipped — `b` anchors the roving tabindex.
    assert_eq!(
        api.tab_attrs(&key("a"), false).get(&HtmlAttr::TabIndex),
        Some("-1"),
    );
    assert_eq!(
        api.tab_attrs(&key("b"), false).get(&HtmlAttr::TabIndex),
        Some("0"),
    );
}

// ────────────────────────────────────────────────────────────────────
// on_props_changed / SyncProps
// ────────────────────────────────────────────────────────────────────

/// Helper: assert `on_props_changed` emits exactly `[Event::SyncProps]`
/// when only the field tweaked by `mutate` differs.
fn assert_sync_props_emitted_when(mutate: impl FnOnce(&mut Props)) {
    let old = test_props();
    let mut new = test_props();

    mutate(&mut new);

    assert_ne!(old, new);

    let events = <Machine as MachineTrait>::on_props_changed(&old, &new);

    assert_eq!(events.as_slice(), &[Event::SyncProps]);
}

#[test]
fn on_props_changed_emits_sync_props_when_orientation_differs() {
    assert_sync_props_emitted_when(|p| p.orientation = Orientation::Vertical);
}

#[test]
fn on_props_changed_emits_sync_props_when_activation_mode_differs() {
    assert_sync_props_emitted_when(|p| p.activation_mode = ActivationMode::Manual);
}

#[test]
fn on_props_changed_emits_sync_props_when_dir_differs() {
    assert_sync_props_emitted_when(|p| p.dir = Direction::Rtl);
}

#[test]
fn on_props_changed_emits_sync_props_when_loop_focus_differs() {
    assert_sync_props_emitted_when(|p| p.loop_focus = false);
}

#[test]
fn on_props_changed_emits_sync_props_when_disabled_keys_differ() {
    assert_sync_props_emitted_when(|p| {
        p.disabled_keys.insert(key("a"));
    });
}

#[test]
fn on_props_changed_no_event_when_props_equal() {
    let props = test_props();

    assert!(<Machine as MachineTrait>::on_props_changed(&props, &props).is_empty());
}

#[test]
fn on_props_changed_no_event_when_only_non_context_props_differ() {
    // `id`, `value`, `default_value`, `lazy_mount`, `unmount_on_exit`,
    // `disallow_empty_selection`, `reorderable` are not context-backed —
    // changing them does NOT emit SyncProps.
    let old = test_props();
    let new = Props {
        id: "different".to_string(),
        lazy_mount: true,
        unmount_on_exit: true,
        disallow_empty_selection: true,
        reorderable: true,
        ..test_props()
    };

    assert!(<Machine as MachineTrait>::on_props_changed(&old, &new).is_empty());
}

#[test]
fn sync_props_replays_orientation_dir_loop_focus_activation_mode() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b")]);

    service.set_props(Props {
        orientation: Orientation::Vertical,
        dir: Direction::Rtl,
        loop_focus: false,
        activation_mode: ActivationMode::Manual,
        ..test_props()
    });

    let ctx = service.context();
    assert_eq!(ctx.orientation, Orientation::Vertical);
    assert_eq!(ctx.dir, Direction::Rtl);
    assert!(!ctx.loop_focus);
    assert_eq!(ctx.activation_mode, ActivationMode::Manual);
}

#[test]
fn sync_props_rebuilds_disabled_tabs_and_snaps_value() {
    let mut service = service_with_tabs(test_props(), &[key("a"), key("b"), key("c")]);

    let mut newly_disabled = BTreeSet::new();

    newly_disabled.insert(key("a"));

    service.set_props(Props {
        disabled_keys: newly_disabled,
        ..test_props()
    });

    // `a` is now disabled and was the selected tab → value snaps to
    // first non-disabled tab `b`.
    assert!(service.context().disabled_tabs.contains(&key("a")));
    assert_eq!(service.context().value.get().as_ref(), Some(&key("b")));
}

// ────────────────────────────────────────────────────────────────────
// Reorderable + disabled
// ────────────────────────────────────────────────────────────────────

#[test]
fn reorder_skips_disabled_tabs() {
    let mut disabled = BTreeSet::new();

    disabled.insert(key("b"));

    let service = service_with_tabs(
        Props {
            reorderable: true,
            disabled_keys: disabled,
            ..test_props()
        },
        &[key("a"), key("b"), key("c")],
    );

    let recorder: EventRecorder = RefCell::new(Vec::new());

    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    // Disabled `b` is not reorderable.
    api.on_tab_keydown(&key("b"), &ctrl_keydown(KeyboardKey::ArrowRight));

    assert!(
        recorder.borrow().is_empty(),
        "Ctrl+Arrow on a disabled tab is a no-op"
    );
}
