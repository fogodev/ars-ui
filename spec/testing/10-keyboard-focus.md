# Keyboard & Focus Testing

> **Alias:** Throughout this file, `PresenceState` is used as a test alias for `presence::State`.
> The canonical state names are: `Unmounted`, `Mounting`, `Mounted`, `UnmountPending`.
> There is NO `Unmounting` variant.
>
> **Harness entrypoints:** `render(...)` and `mount_with_locale(...)` in the
> examples below are imported from the active adapter harness crate
> (`ars_test_harness_leptos` or `ars_test_harness_dioxus`). The core
> `ars-test-harness` crate exposes only `render_with_backend(...)` and
> `render_with_locale_and_backend(...)`.

## 1. Presence and Animation Lifecycle Testing

### 1.1 Mount/Unmount Lifecycle

```rust
#[test]
fn presence_mount_sequence() {
    let harness = render(Presence::new(false));
    assert!(!harness.is_mounted());
    harness.set_present(true);
    assert!(harness.is_mounted());
    assert_eq!(harness.state(), PresenceState::Mounting);
    harness.fire_animation_end();
    assert_eq!(harness.state(), PresenceState::Mounted);
}

#[test]
fn presence_unmount_waits_for_animation() {
    let harness = render(Presence::new(true));
    harness.set_present(false);
    assert_eq!(harness.state(), PresenceState::UnmountPending);
    assert!(harness.is_mounted()); // Still in DOM during animation
    harness.fire_animation_end();
    assert!(!harness.is_mounted()); // Removed after animation
}
```

### 1.2 Rapid Open/Close

```rust
#[test]
fn presence_rapid_toggle_no_stuck_state() {
    let harness = render(Presence::new(false));
    harness.set_present(true);
    harness.set_present(false); // Before mount animation ends
    harness.set_present(true);  // Before unmount animation ends
    harness.fire_animation_end();
    assert!(harness.is_mounted());
    assert_eq!(harness.state(), PresenceState::Mounted);
}

#[test]
fn presence_rapid_toggle_net_absent() {
    let mut svc = Service::new(presence::Props::new().present(true), Env::default(), Default::default());
    // true → false → true → false (net: absent)
    svc.send(presence::Event::SetPresent(false));
    svc.send(presence::Event::SetPresent(true));
    svc.send(presence::Event::SetPresent(false));
    fire_animation_end(&mut svc);
    assert_eq!(*svc.state(), presence::State::Unmounted);
}

#[test]
fn presence_triple_toggle_during_unmount_pending() {
    let mut svc = Service::new(presence::Props::new().present(true), Env::default(), Default::default());
    // Start unmount
    svc.send(presence::Event::SetPresent(false));
    assert_eq!(*svc.state(), presence::State::UnmountPending);
    // Rapid toggles during UnmountPending
    svc.send(presence::Event::SetPresent(true));
    svc.send(presence::Event::SetPresent(false));
    fire_animation_end(&mut svc);
    assert_eq!(*svc.state(), presence::State::Unmounted);
}
```

### 1.3 Zero-Duration Animation

```rust
#[test]
fn presence_zero_duration_skips_animation() {
    let harness = render(Presence::new(false).animation_duration(Duration::ZERO));
    harness.set_present(true);
    // Should immediately transition to Mounted without waiting for animationend
    assert_eq!(harness.state(), PresenceState::Mounted);
}
```

### 1.4 Animation Completion Testing

Components that use CSS animations (Dialog, Toast, Popover) require explicit tests for
`animationend` correctness, timeout fallback behavior, and stuck intermediate states.

**Mocking `animationend`**: Adapter test harnesses MUST provide a mock `animationend` dispatch
mechanism. In unit tests, the harness's `fire_animation_end()` simulates the browser event.
In wasm integration tests, use `dispatchEvent(new AnimationEvent("animationend"))`.

```rust
#[test]
fn dialog_dismiss_animation_fires_cleanup_on_animationend() {
    let harness = render(Dialog::new().open(true).unmount_on_exit(true));
    harness.send(dialog::Event::Close);
    // State transitions to closed (exit animation in progress), but DOM node remains
    assert_eq!(harness.data_attr("state"), "closed");
    assert!(harness.is_mounted());

    // Simulate browser firing animationend
    harness.fire_animation_end();
    assert!(!harness.is_mounted(), "DOM node should be removed after animationend");
}
```

**Timeout fallback**: If `animationend` never fires (e.g., CSS animation removed, browser
quirk, or `display: none` applied mid-animation), the component MUST NOT remain stuck in an
intermediate state. A safety timeout forces cleanup:

```rust
#[test]
fn dialog_dismiss_timeout_fallback_if_animationend_missing() {
    let harness = render(Dialog::new().open(true).animation_timeout(Duration::from_millis(500)));
    harness.send(dialog::Event::Close);
    assert_eq!(harness.data_attr("state"), "closed");

    // Do NOT fire animationend — simulate a stalled animation
    harness.advance_time(Duration::from_millis(500));

    // Timeout fallback must clean up
    assert!(!harness.is_mounted(), "timeout fallback should force unmount");
}
```

**Stuck-state snapshot tests**: Snapshot the `data-ars-state` attribute during intermediate
animation phases to catch regressions where a component gets stuck in `"open"` or
`"closed"` (the two `data-ars-state` tokens defined by Presence):

```rust
#[test]
fn toast_intermediate_state_snapshots() {
    let harness = render(Toast::new().open(false));
    harness.send(toast::Event::Show);
    assert_snapshot!("toast_mounting", harness.snapshot_attrs());

    harness.fire_animation_end();
    assert_snapshot!("toast_mounted", harness.snapshot_attrs());

    harness.send(toast::Event::Dismiss);
    assert_snapshot!("toast_dismissing", harness.snapshot_attrs());

    harness.fire_animation_end();
    assert_snapshot!("toast_dismissed", harness.snapshot_attrs());
}
```

---

## 2. Disabled State Edge Cases

### 2.1 Per-Component Disabled Matrix

Every interactive component MUST be tested for these behaviors when disabled:

- Cannot be focused via Tab
- Cannot be activated via keyboard (Enter/Space)
- Cannot be activated via mouse click
- Cannot be activated via touch
- Has `aria-disabled="true"` (NOT the `disabled` HTML attribute for custom elements)
- Visual appearance changes (opacity, cursor)

The disabled contract distinguishes between `aria-disabled` (custom elements) and the HTML
`disabled` attribute (native form elements). Elements with `aria-disabled="true"` REMAIN
focusable and in the tab order; only the HTML `disabled` attribute removes an element from
the tab order. Both disable activation (click, Enter, Space).

```rust
use ars_core::{
    Bindable,
    ColorChannel,
    DateSegmentKind,
    KeyboardKey,
};


/// Tests that an HTML-natively-disabled element (`<button disabled>`, `<input disabled>`)
/// is removed from the tab order. ONLY for native form elements where HTML `disabled`
/// removes from tab order. For composite widgets using `aria-disabled`, use
/// [`test_aria_disabled!`] instead.
macro_rules! test_html_disabled {
    ($component:ident, $id:expr) => {
        #[test]
        fn html_disabled_removes_from_tab_order() {
            let svc = Service::new($component::Props::new($id).disabled(true), Env::default(), Default::default());
            let api = svc.connect(&|_| {});
            let attrs = api.root_attrs();
            assert!(attrs.contains(&HtmlAttr::Disabled));
            // HTML disabled elements are removed from tab order
            assert_eq!(attrs.get(&HtmlAttr::TabIndex), None);
        }
    };
}

/// Tests for components using `aria-disabled` (composite widgets).
/// aria-disabled elements REMAIN focusable but do not respond to activation.
macro_rules! test_aria_disabled {
    ($component:ident, $id:expr) => {
        #[test]
        fn aria_disabled_remains_focusable() {
            let svc = Service::new($component::Props::new($id).disabled(true), Env::default(), Default::default());
            let api = svc.connect(&|_| {});
            let attrs = api.root_attrs();
            assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
            // aria-disabled elements remain in tab order
            assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("0"));
        }

        #[test]
        fn aria_disabled_blocks_activation() {
            let mut svc = Service::new($component::Props::new($id).disabled(true), Env::default(), Default::default());
            let before = svc.state().clone();
            svc.send($component::Event::Click);
            assert_eq!(svc.state(), &before, "disabled component should not change state on activation");
        }
    };
}

// Native form elements use HTML disabled
test_html_disabled!(Button, "btn1");

// Composite widgets use aria-disabled
test_aria_disabled!(Checkbox, "cb1");
test_aria_disabled!(Switch, "sw1");
test_aria_disabled!(Select, "sel1");
test_aria_disabled!(Combobox, "cmb1");
test_aria_disabled!(Slider, "sl1");
test_aria_disabled!(RadioGroup, "rg1");

/// DOM-level verification that pointer events on `aria-disabled` elements
/// do not trigger state changes.
#[wasm_bindgen_test]
async fn disabled_element_ignores_pointer_events() {
    let props = checkbox::Props { disabled: true, ..Default::default() };
    let harness = render(Checkbox::new(props)).await;
    let control = harness.query("[data-ars-part='control']").expect("control must exist");
    // Dispatch raw pointer events
    let event = web_sys::PointerEvent::new("pointerdown").expect("event creation must succeed");
    control.dispatch_event(&event).expect("dispatch must succeed");
    let event = web_sys::PointerEvent::new("pointerup").expect("event creation must succeed");
    control.dispatch_event(&event).expect("dispatch must succeed");
    harness.tick().await;
    assert_aria_checked(&harness.snapshot_attrs(), "false");
}
```

### 2.2 HoverCard Disabled

```rust
#[test]
fn hovercard_disabled_no_open() {
    let harness = render(HoverCard::new().disabled(true));
    harness.hover_trigger();
    harness.advance_time(Duration::from_millis(500));
    assert!(!harness.is_open());
}
```

### 2.3 Menu Disabled Items

```rust
#[test]
fn menu_disabled_item_focusable_but_not_selectable() {
    let harness = render(Menu::with_items(vec![
        menu::Item::new("a"),
        menu::Item::new("b").disabled(true),
        menu::Item::new("c"),
    ]).disabled_behavior(DisabledBehavior::FocusOnly));
    harness.open();
    harness.press_key(KeyboardKey::ArrowDown); // Focus "a"
    harness.press_key(KeyboardKey::ArrowDown); // Focus "b" (disabled but focusable)
    assert_eq!(harness.highlighted_item(), "b");
    harness.press_key(KeyboardKey::Enter);
    assert!(harness.is_open()); // Not selected, menu stays open
}
```

### 2.4 Calendar Disabled Dates

```rust
#[test]
fn calendar_disabled_date_not_selectable() {
    let harness = render(Calendar::new().is_date_disabled(|d| d.day() == 15));
    harness.send(calendar::Event::NavigateToDate(
        Date::try_new_iso(2026, 3, 15).expect("valid date"),
    ));
    harness.press_key(KeyboardKey::Enter);
    assert_ne!(harness.data_attr("selected-date"), "2026-03-15");
}
```

## 3. Disabled State Guard Matrix

Comprehensive verification that disabled components reject all interactions and emit correct ARIA/HTML attributes.

### 3.1 Per-Component Disabled Template

> **Relationship to section 2.1:** The `test_disabled_guard!` macro in this section tests disabled behavior at the Machine/Service level (guard enforcement, event rejection). Section 2.1's `test_html_disabled!` and `test_aria_disabled!` test the DOM attribute output. These are complementary — use section 2.1 macros for attribute verification, section 3.1 macros for state machine guard verification.

```rust
macro_rules! test_disabled_guard {
    ($mod:ident, $component:ident, $events:expr) => {
        mod $mod {
            use super::*;

            #[test]
            fn disabled_ignores_all_events() {
                let props = $component::Props { disabled: true, ..Default::default() };
                let mut svc = Service::<$component::Machine>::new(props);
                let initial_state = svc.state().clone();
                let initial_ctx = svc.context().clone();

                for event in $events {
                    svc.send(event);
                    assert_eq!(
                        *svc.state(), initial_state,
                        "disabled {} must not transition on {:?}",
                        stringify!($component), event,
                    );
                    assert_eq!(
                        svc.context(), &initial_ctx,
                        "disabled {} context must not change on {:?}",
                        stringify!($component), event,
                    );
                }
            }

            #[test]
            fn disabled_emits_aria_disabled() {
                let props = $component::Props { disabled: true, ..Default::default() };
                let (state, ctx) = $component::Machine::init(&props, &Env::default(), &Default::default());
                let api = $component::Machine::connect(&state, &ctx, &props, &|_| {});
                let root = api.root_attrs();
                assert_eq!(
                    root.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"),
                    "{} must set aria-disabled=\"true\"", stringify!($component),
                );
            }

            #[test]
            fn disabled_emits_disabled_attribute() {
                let props = $component::Props { disabled: true, ..Default::default() };
                let (state, ctx) = $component::Machine::init(&props, &Env::default(), &Default::default());
                let api = $component::Machine::connect(&state, &ctx, &props, &|_| {});
                let control = api.control_attrs();
                // Native form elements use `disabled`, custom elements use `aria-disabled`
                assert!(
                    control.get(&HtmlAttr::Disabled).is_some()
                        || control.get(&HtmlAttr::Aria(AriaAttr::Disabled)) == Some("true"),
                    "{} must emit disabled or aria-disabled on control element",
                    stringify!($component),
                );
            }
        }
    };
}

/// Like `test_disabled_guard!` but checks the native HTML `disabled` attribute
/// instead of `aria-disabled`. Use for components that render native HTML
/// form elements (e.g., Button, input-based components).
macro_rules! test_html_disabled_guard {
    ($name:ident, $module:ident, $events:expr) => {
        mod $name {
            use super::*;

            #[test]
            fn disabled_blocks_events() {
                let props = $module::Props { disabled: true, ..Default::default() };
                let (state, ctx) = $module::Machine::init(&props, &Env::default(), &Default::default());
                for event in $events {
                    let plan = $module::Machine::transition(&state, &event, &ctx, &props);
                    assert!(
                        plan.is_none(),
                        "disabled component must reject {:?}",
                        event
                    );
                }
            }

            #[test]
            fn disabled_emits_html_disabled_attribute() {
                let props = $module::Props { disabled: true, ..Default::default() };
                let (state, ctx) = $module::Machine::init(&props, &Env::default(), &Default::default());
                let api = $module::Machine::connect(&state, &ctx, &props, &|_: $module::Event| {});
                let attrs = api.root_attrs();
                assert_eq!(
                    attrs.get(&HtmlAttr::Disabled),
                    Some("true"),
                    "native HTML disabled must be set"
                );
            }
        }
    };
}

// Button uses native HTML `disabled` attribute, not `aria-disabled`.
test_html_disabled_guard!(button_disabled, button, vec![
    button::Event::Click,
    button::Event::Press,
]);

test_disabled_guard!(textfield_disabled, text_field, vec![
    text_field::Event::Change("test".into()),
    text_field::Event::Clear,
]);

test_disabled_guard!(select_disabled, select, vec![
    select::Event::Open,
    select::Event::Toggle,
    select::Event::HighlightNext,
    select::Event::SelectItem(Key::from("a")),
]);

test_disabled_guard!(checkbox_disabled, checkbox, vec![
    checkbox::Event::Toggle,
    checkbox::Event::Check,
    checkbox::Event::Uncheck,
]);

test_disabled_guard!(radio_group_disabled, radio_group, vec![
    radio_group::Event::SelectValue(Key::from("a")),
    radio_group::Event::FocusNext,
]);

test_disabled_guard!(slider_disabled, slider, vec![
    slider::Event::Increment,
    slider::Event::Decrement,
    slider::Event::PointerDown { value: 0.5 },
]);

test_disabled_guard!(switch_disabled, switch, vec![
    switch::Event::Toggle,
    switch::Event::TurnOn,
    switch::Event::TurnOff,
]);

// ── Expanded Disabled/Readonly Guard Tests ──────────────────────────────────

// The following components MUST also have disabled guard tests using the
// test_disabled_guard! macro. Each test verifies that ALL state-changing
// events are blocked when disabled=true, while query-only events (e.g.,
// Focus for screen reader discoverability) still function.

test_disabled_guard!(accordion_disabled, accordion, vec![
    accordion::Event::ToggleItem(Key::from("item-1")),
    accordion::Event::ExpandAll,
    accordion::Event::CollapseAll,
]);

test_disabled_guard!(tabs_disabled, tabs, vec![
    tabs::Event::SelectTab(Key::from("tab-1")),
    tabs::Event::FocusNext,
    tabs::Event::FocusPrev,
    tabs::Event::CloseTab(Key::from("tab-1")),
]);

// NOTE: Dialog does not support a `disabled` prop — it is opened/closed
// by external triggers whose disabled state is tested at the trigger level.
// See button_disabled / test_html_disabled_guard! for trigger-level tests.

test_disabled_guard!(tooltip_disabled, tooltip, vec![
    tooltip::Event::Open,
    tooltip::Event::Close,
]);

test_disabled_guard!(datefield_disabled, date_field, vec![
    date_field::Event::IncrementSegment(DateSegmentKind::Day),
    date_field::Event::DecrementSegment(DateSegmentKind::Day),
    date_field::Event::TypeIntoSegment(DateSegmentKind::Day, '1'),
    date_field::Event::ClearSegment(DateSegmentKind::Day),
]);

test_disabled_guard!(combobox_disabled, combobox, vec![
    combobox::Event::Open,
    combobox::Event::InputChange("test".into()),
    combobox::Event::SelectItem(Key::from("a")),
]);

test_disabled_guard!(number_input_disabled, number_input, vec![
    number_input::Event::Increment,
    number_input::Event::Decrement,
    number_input::Event::Change("5".into()),
]);

test_disabled_guard!(color_picker_disabled, color_picker, vec![
    color_picker::Event::Open,
    color_picker::Event::DragStart { target: DragTarget::Area, x: 0.5, y: 0.5 },
    color_picker::Event::SetChannel { channel: ColorChannel::Hue, value: 180.0 },
]);

// Query-only events that MUST still work when disabled:
#[test]
fn disabled_components_allow_query_events() {
    // Focus events should still work for screen reader discoverability.
    // The component remains focusable (no `disabled` HTML attribute)
    // but aria-disabled="true" is set.
    let props = button::Props { disabled: true, ..Default::default() };
    let mut svc = Service::<button::Machine>::new(props);
    svc.send(button::Event::Focus { is_keyboard: true });
    assert!(svc.context().focus_visible,
        "Focus should still work on disabled components for a11y");
}
```

---

## 4. Readonly State Tests

Readonly differs from disabled: the control is focusable and its value is readable,
but the value cannot be changed.

### 4.1 PinInput Readonly

```rust
#[test]
fn pin_input_readonly_shows_value_but_rejects_input() {
    let harness = render(PinInput::new(4).value("1234").read_only(true));
    assert_eq!(harness.value(), "1234");
    harness.focus("[data-ars-part='input']:nth-of-type(1)");
    assert!(harness.focused_element().is_some()); // Focusable
    harness.type_text("5");
    assert_eq!(harness.value(), "1234"); // Unchanged
}
```

### 4.2 Switch Readonly

```rust
#[test]
fn switch_readonly_preserves_state() {
    let harness = render(Switch::new(true).read_only(true));
    assert!(harness.is_checked());
    harness.click();
    assert!(harness.is_checked()); // Still checked
    harness.press_key(KeyboardKey::Space);
    assert!(harness.is_checked()); // Still checked
}
```

### 4.3 Checkbox Readonly

```rust
#[test]
fn checkbox_readonly_no_toggle() {
    let harness = render(Checkbox::new(true).read_only(true));
    harness.click();
    assert!(harness.is_checked());
}
```

### 4.4 Slider Readonly

```rust
#[test]
fn slider_readonly_no_drag() {
    let harness = render(Slider::new(50.0).read_only(true));
    harness.drag_thumb_to(75.0);
    assert_eq!(harness.value(), 50.0);
    harness.press_key(KeyboardKey::ArrowRight);
    assert_eq!(harness.value(), 50.0);
}
```

## 5. Drag and Drop Testing

### 5.1 Slider Thumb Drag

```rust
#[test]
fn slider_drag_updates_value_continuously() {
    let harness = render(Slider::new(0.0).min(0.0).max(100.0));
    let values = harness.record_values(|| {
        harness.drag_thumb(point(0, 50), point(50, 50), 10); // 10 intermediate steps
    });
    assert!(values.len() >= 10); // Continuous updates
    assert!(values.last().unwrap() > &0.0);
}
```

### 5.2 FloatingPanel Drag

```rust
#[test]
fn floating_panel_drag_repositions() {
    let harness = render(FloatingPanel::new().position(point(100, 100)));
    harness.pointer_down_at(100.0, 100.0);
    harness.pointer_move_to(200.0, 200.0);
    harness.pointer_up();
    let pos = harness.query_part("panel").expect("panel").bounding_rect();
    assert_eq!((pos.left, pos.top), (200.0, 200.0));
}
```

### 5.3 Drawer Snap Points

```rust
#[test]
fn drawer_snaps_to_nearest_point() {
    let harness = render(Drawer::new()
        .snap_points(vec![0.25, 0.5, 0.75])
        .placement(Placement::Bottom)
    );
    harness.open();
    // Drag to ~40% — should snap to 0.5
    // Simulate drag via pointer sequence (bottom placement, viewport height assumed)
    let viewport_h = 800.0;
    let start_y = viewport_h; // bottom edge
    let target_y = viewport_h * (1.0 - 0.4); // drag to 40%
    harness.pointer_down_at(200.0, start_y);
    harness.pointer_move_to(200.0, target_y);
    harness.pointer_up();
    let drawer_rect = harness.query_part("drawer").expect("drawer").bounding_rect();
    let snap = drawer_rect.top / viewport_h;
    assert!((snap - 0.5).abs() < 0.05, "drawer should snap to 0.5");
}
```

### 5.4 Keyboard Reorder

```rust
#[test]
fn sortable_list_keyboard_reorder() {
    let harness = render(SortableList::with_items(vec!["A", "B", "C"]));
    harness.focus("[data-value='B']");
    harness.press_key(KeyboardKey::Space); // Enter drag mode
    harness.press_key(KeyboardKey::ArrowUp); // Move up
    harness.press_key(KeyboardKey::Space); // Drop
    assert_eq!(harness.query_selector_all("[data-ars-part='item']").iter().map(|el| el.text_content()).collect::<Vec<_>>(), vec!["B", "A", "C"]);
}
```

### 5.5 Drag Accessibility

```rust
#[test]
fn drag_announces_position() {
    let harness = render(SortableList::with_items(vec!["A", "B", "C"]));
    harness.focus("[data-value='A']");
    harness.press_key(KeyboardKey::Space);
    assert!(harness.query_selector("[aria-live]").expect("live region").text_content().contains("Grabbed A"));
    harness.press_key(KeyboardKey::ArrowDown);
    assert!(harness.query_selector("[aria-live]").expect("live region").text_content().contains("A moved to position 2"));
}
```

## 6. Keyboard Navigation Matrix

Every interactive component MUST have a completed keyboard navigation matrix that exhaustively tests Tab, Arrow keys, Home, End, Escape, Enter, and Space interactions across all component states.

### 6.1 Template

| Component | Key        | State   | Expected Behavior                             |
| --------- | ---------- | ------- | --------------------------------------------- |
| Dialog    | Escape     | Open    | Close dialog, return focus to trigger         |
| Select    | ArrowDown  | Open    | Highlight next enabled item                   |
| Select    | ArrowDown  | Closed  | Open and highlight first item                 |
| Tabs      | ArrowRight | Focused | Move focus to next tab                        |
| Tabs      | Home       | Focused | Move focus to first tab                       |
| Tabs      | End        | Focused | Move focus to last tab                        |
| Combobox  | Escape     | Open    | Close listbox, clear input if `clearOnEscape` |
| Slider    | ArrowRight | Focused | Increment by step                             |
| Slider    | Home       | Focused | Set to minimum value                          |
| DateField | Left/Right | Focused | Move between segments                         |
| DateField | Up/Down    | Focused | Increment/decrement segment value             |
| DateField | Tab        | Focused | Exit field                                    |
| DateField | Enter      | Focused | Confirm and move to next segment              |

### 6.2 Per-Component Matrix Requirement

Every interactive component MUST have a completed key matrix covering all keys and all states. Missing entries are CI failures.

### 6.3 Test Pattern

```rust
/// Test utility: force a service into a specific state for keyboard matrix testing.
/// Requires `Service` to expose a `#[cfg(test)]` method `set_state_for_test`.
fn force_state<M: Machine>(svc: &mut Service<M>, state: M::State) {
    svc.set_state_for_test(state);
}

struct KeyMatrixEntry {
    key: KeyboardKey,
    initial_state: State,
    expected_state: Option<State>,
    expected_ctx: Option<Box<dyn Fn(&Context)>>,
    description: &'static str,
}

#[test]
fn keyboard_matrix_exhaustive() {
    let keys = [
        KeyboardKey::Tab, KeyboardKey::ArrowUp, KeyboardKey::ArrowDown,
        KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight,
        KeyboardKey::Home, KeyboardKey::End, KeyboardKey::Escape,
        KeyboardKey::Enter, KeyboardKey::Space,
    ];
    let states = State::ALL_VARIANTS; // Every state the component defines

    let matrix: Vec<KeyMatrixEntry> = vec![
        // ... one entry per (key, state) pair ...
    ];

    // Verify completeness: every (key, state) pair is covered
    for state in &states {
        for key in &keys {
            assert!(
                matrix.iter().any(|e| e.key == *key && e.initial_state == *state),
                "Missing matrix entry for ({key:?}, {state:?})"
            );
        }
    }

    // Run each entry
    for entry in &matrix {
        let props = Props::default();
        let mut svc = Service::<Machine>::new(props);
        force_state(&mut svc, entry.initial_state.clone());

        let result = svc.send(Event::KeyDown { key: entry.key.clone() });
        match &entry.expected_state {
            Some(expected) => assert_eq!(svc.state(), expected, "{}", entry.description),
            None => { /* no transition expected */ }
        }
        if let Some(check_ctx) = &entry.expected_ctx {
            check_ctx(svc.context());
        }
    }
}
```

### 6.4 Extended Keyboard Navigation Entries

The following components MUST also have complete keyboard matrix entries:

| Component   | Key         | State   | Expected Behavior                                 |
| ----------- | ----------- | ------- | ------------------------------------------------- |
| Listbox     | ArrowDown   | Focused | Move highlight to next enabled item               |
| Listbox     | ArrowUp     | Focused | Move highlight to previous enabled item           |
| Listbox     | Home        | Focused | Move highlight to first enabled item              |
| Listbox     | End         | Focused | Move highlight to last enabled item               |
| Listbox     | PageDown    | Focused | Jump highlight down by page size                  |
| Listbox     | PageUp      | Focused | Jump highlight up by page size                    |
| Listbox     | _typeahead_ | Focused | NFC-normalized prefix match, 500ms timeout        |
| Menu        | ArrowDown   | Open    | Move highlight to next enabled menuitem           |
| Menu        | ArrowUp     | Open    | Move highlight to previous enabled menuitem       |
| Menu        | ArrowRight  | Open    | Open submenu (if current item has submenu)        |
| Menu        | ArrowLeft   | Submenu | Close submenu, return focus to parent menuitem    |
| Menu        | Escape      | Open    | Close menu, return focus to trigger               |
| MenuBar     | ArrowRight  | Focused | Move focus to next top-level menu                 |
| MenuBar     | ArrowLeft   | Focused | Move focus to previous top-level menu             |
| MenuBar     | ArrowDown   | Focused | Open menu and focus first item                    |
| Tree        | ArrowDown   | Focused | Move focus to next visible treeitem               |
| Tree        | ArrowUp     | Focused | Move focus to previous visible treeitem           |
| Tree        | ArrowRight  | Focused | Expand node (if collapsed) or move to first child |
| Tree        | ArrowLeft   | Focused | Collapse node (if expanded) or move to parent     |
| Tree        | Home        | Focused | Move focus to first treeitem                      |
| Tree        | End         | Focused | Move focus to last visible treeitem               |
| Tree        | PageDown    | Focused | Jump focus down by page size                      |
| Tree        | PageUp      | Focused | Jump focus up by page size                        |
| Grid        | ArrowDown   | Focused | Move focus to cell below                          |
| Grid        | ArrowUp     | Focused | Move focus to cell above                          |
| Grid        | ArrowRight  | Focused | Move focus to next cell in row                    |
| Grid        | ArrowLeft   | Focused | Move focus to previous cell in row                |
| Grid        | Home        | Focused | Move focus to first cell in row                   |
| Grid        | End         | Focused | Move focus to last cell in row                    |
| Grid        | PageDown    | Focused | Jump focus down by page size of rows              |
| Grid        | PageUp      | Focused | Jump focus up by page size of rows                |
| Grid        | Ctrl+Home   | Focused | Move focus to first cell of first row             |
| Grid        | Ctrl+End    | Focused | Move focus to last cell of last row               |
| Accordion   | ArrowDown   | Focused | Move focus to next accordion header               |
| Accordion   | ArrowUp     | Focused | Move focus to previous accordion header           |
| Accordion   | Home        | Focused | Move focus to first accordion header              |
| Accordion   | End         | Focused | Move focus to last accordion header               |
| Accordion   | Enter       | Focused | Toggle expand/collapse of focused section         |
| Accordion   | Space       | Focused | Toggle expand/collapse of focused section         |
| RadioGroup  | ArrowDown   | Focused | Select next radio (wraps)                         |
| RadioGroup  | ArrowUp     | Focused | Select previous radio (wraps)                     |
| RadioGroup  | ArrowRight  | Focused | Select next radio (wraps)                         |
| RadioGroup  | ArrowLeft   | Focused | Select previous radio (wraps)                     |
| NumberField | Up          | Focused | Increment value by step                           |
| NumberField | Down        | Focused | Decrement value by step                           |
| NumberField | Home        | Focused | Set to minimum value                              |
| NumberField | End         | Focused | Set to maximum value                              |
| NumberField | Tab         | Focused | Move focus to next focusable element              |
| TagsInput   | Backspace   | Focused | Remove last tag (when input is empty)             |
| TagsInput   | Delete      | Focused | Remove focused tag                                |
| TagsInput   | ArrowLeft   | Focused | Navigate to previous tag                          |
| TagsInput   | ArrowRight  | Focused | Navigate to next tag                              |

#### 6.4.1 Cross-Component Key Summary Matrix

| Key        | Listbox    | Tree            | Grid       | Menu       | Tabs       | RadioGroup  | Accordion    |
| ---------- | ---------- | --------------- | ---------- | ---------- | ---------- | ----------- | ------------ |
| ArrowDown  | Next item  | Next visible    | Next row   | Next item  | —          | —           | Next header  |
| ArrowUp    | Prev item  | Prev visible    | Prev row   | Prev item  | —          | —           | Prev header  |
| ArrowRight | —          | Expand/child    | Next cell  | Submenu    | Next tab   | Next radio  | —            |
| ArrowLeft  | —          | Collapse/parent | Prev cell  | Close sub  | Prev tab   | Prev radio  | —            |
| Home       | First item | First visible   | First cell | First item | First tab  | First radio | First header |
| End        | Last item  | Last visible    | Last cell  | Last item  | Last tab   | Last radio  | Last header  |
| PageDown   | Jump down  | Jump down       | Jump rows  | —          | —          | —           | —            |
| PageUp     | Jump up    | Jump up         | Jump rows  | —          | —          | —           | —            |
| Enter      | Select     | Activate        | Activate   | Activate   | —          | Select      | Toggle       |
| Space      | Select     | —               | —          | Activate   | Select tab | Select      | Toggle       |
| Escape     | Close      | —               | —          | Close      | —          | —           | —            |

### 6.5 Virtualized Keyboard Navigation

When a collection uses the `Virtualizer` from [06-collections.md](../foundation/06-collections.md) §6, only a subset of items are rendered in the DOM. Keyboard navigation must handle transitions to unrendered items by triggering scroll-to-focus behavior.

```rust
use ars_core::{Key, KeyboardKey};

#[wasm_bindgen_test]
async fn arrow_down_into_unrendered_item_scrolls_and_focuses() {
    let items: Vec<_> = (0..100).map(|i| (Key::from(format!("item-{i}")), format!("Item {i}"))).collect();
    let harness = render(VirtualizedListbox::new("vl1", items, /* viewport_height */ 200.0)).await;
    tick().await;

    // Focus last visible item
    let last_visible = harness.query("[data-ars-key='item-4']")
        .expect("last visible item must be in DOM");
    last_visible.focus();
    tick().await;

    // Press ArrowDown — should scroll to reveal item-5 and focus it
    harness.press_key(KeyboardKey::ArrowDown);
    tick().await;

    let newly_focused = harness.query("[data-ars-key='item-5']")
        .expect("item-5 must be rendered after scroll");
    assert_eq!(
        document().active_element().as_ref(),
        Some(newly_focused.as_ref()),
        "item-5 must receive focus after scroll"
    );
}

#[wasm_bindgen_test]
async fn arrow_up_into_unrendered_item_scrolls_backward() {
    let items: Vec<_> = (0..100).map(|i| (Key::from(format!("item-{i}")), format!("Item {i}"))).collect();
    let harness = render(VirtualizedListbox::new("vl1", items, 200.0)).await;
    tick().await;

    // Scroll down so item-0 is no longer rendered
    harness.scroll_to(0, 400); // items 10-14 visible
    tick().await;

    // Focus first visible item
    let first_visible = harness.query("[role='option']:first-child")
        .expect("first visible item must be in DOM");
    first_visible.focus();
    harness.press_key(KeyboardKey::ArrowUp);
    tick().await;

    // Must scroll back and focus the previous item
    let focused = document().active_element().expect("something must be focused");
    assert!(focused.get_attribute("data-ars-key").is_some(),
        "focused element must be a collection item");
}
```

---

## 7. Focus Restoration

When an overlay closes, focus must return to the element that triggered the overlay.
If the trigger is no longer available, focus falls back through a chain: parent element,
then `document.body`.

```rust
#[wasm_bindgen_test]
async fn focus_restores_to_trigger_on_dialog_close() {
    mount_to_body(|| {
        view! {
            <Button id="trigger">"Open"</Button>
            <Dialog id="dlg" trigger_id="trigger">"Content"</Dialog>
        }
    });

    // Open dialog (focus moves to dialog)
    let trigger = document().get_element_by_id("trigger").unwrap();
    trigger.dyn_ref::<HtmlElement>().unwrap().click();
    tick().await;
    assert!(document().query_selector("[role='dialog']").unwrap().is_some());

    // Close dialog
    dispatch_keyboard_event_on_active("keydown", "Escape");
    tick().await;

    // Assert focus is on trigger button
    let active = document().active_element().unwrap();
    assert_eq!(active.id(), "trigger");
}

#[wasm_bindgen_test]
async fn focus_falls_back_to_parent_when_trigger_removed() {
    mount_to_body(|| {
        view! {
            <div id="container" tabindex="-1">
                <Button id="trigger">"Open"</Button>
            </div>
            <Dialog id="dlg" trigger_id="trigger">"Content"</Dialog>
        }
    });

    // Open dialog, then remove trigger from DOM
    let trigger = document().get_element_by_id("trigger").unwrap();
    trigger.dyn_ref::<HtmlElement>().unwrap().click();
    tick().await;
    trigger.remove();

    // Close dialog
    dispatch_keyboard_event_on_active("keydown", "Escape");
    tick().await;

    // Assert focus falls back to the container (parent fallback)
    let active = document().active_element().unwrap();
    assert_eq!(active.id(), "container");
}

#[wasm_bindgen_test]
async fn focus_falls_back_to_body_when_no_fallback() {
    mount_to_body(|| {
        view! {
            <div id="container">
                <Button id="trigger">"Open"</Button>
            </div>
            <Dialog id="dlg" trigger_id="trigger">"Content"</Dialog>
        }
    });

    // Open dialog, then remove both trigger and container from DOM
    let trigger = document().get_element_by_id("trigger").unwrap();
    trigger.dyn_ref::<HtmlElement>().unwrap().click();
    tick().await;
    document().get_element_by_id("container").unwrap().remove();

    // Close dialog
    dispatch_keyboard_event_on_active("keydown", "Escape");
    tick().await;

    // Assert focus falls back to document.body
    let active = document().active_element().unwrap();
    assert_eq!(active.tag_name().to_lowercase(), "body");
}
```

### 7.1 Zero-Focusable-Children Modal

When a modal dialog contains no focusable children (no buttons, inputs, or links),
focus must move to the dialog container itself. Tab must not escape the dialog.

```rust
#[wasm_bindgen_test]
async fn focus_trap_with_no_focusable_children() {
    mount_to_body(|| {
        view! {
            <Button id="trigger">"Open"</Button>
            <Dialog id="dlg" trigger_id="trigger">
                <p>"This dialog has only text content — no interactive elements."</p>
            </Dialog>
        }
    });

    // Open dialog
    let trigger = document().get_element_by_id("trigger").unwrap();
    trigger.dyn_ref::<HtmlElement>().unwrap().click();
    tick().await;

    // Assert focus is on the dialog element itself (the container)
    let active = document().active_element().unwrap();
    assert_eq!(active.get_attribute("role").as_deref(), Some("dialog"));

    // Press Tab — focus should NOT leave the dialog
    dispatch_keyboard_event_on_active("keydown", "Tab");
    tick().await;
    let active = document().active_element().unwrap();
    assert_eq!(
        active.get_attribute("role").as_deref(),
        Some("dialog"),
        "Tab must not move focus out of a modal with no focusable children"
    );
}
```

### 7.2 Nested Overlay Focus Restoration Chain

When overlays are nested, closing them must restore focus through the chain in order.

```rust
#[wasm_bindgen_test]
async fn nested_overlay_focus_restoration_chain() {
    mount_to_body(|| {
        view! {
            <Button id="page-btn">"Open Dialog A"</Button>
            <Dialog id="dialog-a" trigger_id="page-btn">
                <Button id="dialog-a-btn">"Open Dialog B"</Button>
                <Dialog id="dialog-b" trigger_id="dialog-a-btn">
                    <p>"Nested content"</p>
                    <Button id="dialog-b-close">"Close B"</Button>
                </Dialog>
            </Dialog>
        }
    });

    // Open Dialog A (focus → Dialog A)
    let page_btn = document().get_element_by_id("page-btn").unwrap();
    page_btn.dyn_ref::<HtmlElement>().unwrap().click();
    tick().await;

    // Open Dialog B from within A (focus → Dialog B)
    let dialog_a_btn = document().get_element_by_id("dialog-a-btn").unwrap();
    dialog_a_btn.dyn_ref::<HtmlElement>().unwrap().click();
    tick().await;

    // Close Dialog B (focus → Dialog A's button)
    dispatch_keyboard_event_on_active("keydown", "Escape");
    tick().await;
    let active = document().active_element().unwrap();
    assert_eq!(active.id(), "dialog-a-btn");

    // Close Dialog A (focus → Page's button)
    dispatch_keyboard_event_on_active("keydown", "Escape");
    tick().await;
    let active = document().active_element().unwrap();
    assert_eq!(active.id(), "page-btn");
}
```

### 7.3 Focus Restoration with Conditionally-Rendered Trigger Parent

```rust
#[wasm_bindgen_test]
async fn focus_restores_when_trigger_parent_conditionally_hidden() {
    // Use render() to mount the dialog with a conditionally-visible parent.
    let harness = render(Dialog::new(dialog::Props {
        open: false,
        modal: true,
        ..Default::default()
    }))
    .await;

    let trigger = harness.query("[data-ars-part='trigger']").expect("trigger must exist");
    trigger.click();
    harness.tick().await;
    assert!(harness.is_open(), "dialog must be open");

    // Hide the trigger's parent while dialog is open
    // Hide the trigger's parent via DOM style (no TestHarness method for this)
    harness.set_body_style("display", "none");
    harness.tick().await;

    // Close dialog — trigger no longer in DOM
    harness.press_key(KeyboardKey::Escape);
    harness.tick().await;

    // Focus should fall back to a focusable ancestor or body
    let active = document().active_element().expect("something must be focused");
    assert!(active != trigger, "focus must not remain on removed trigger");
}
```

---

## 8. Focus Visible Tests

These tests verify the `data-ars-focus-visible` attribute correctly distinguishes keyboard focus from mouse focus, enabling CSS-only focus ring styling.

### 8.1 Keyboard Tab Sets Focus Visible

```rust
#[test]
fn tab_sets_data_focus_visible() {
    let harness = render(Button::new("Click me"));
    harness.press_key(KeyboardKey::Tab);
    assert_eq!(harness.data_attr("focus-visible"), "true");
}

#[test]
fn keyboard_focus_on_select() {
    let harness = render(Select::new());
    harness.press_key(KeyboardKey::Tab); // Focus trigger via keyboard
    assert_eq!(harness.trigger_attr("data-ars-focus-visible"), "true");
}
```

### 8.2 Mouse Click Does NOT Set Focus Visible

```rust
#[test]
fn mouse_click_no_focus_visible() {
    let harness = render(Button::new("Click me"));
    harness.click();
    assert!(harness.data_attr("focus-visible").is_empty()
        || harness.data_attr("focus-visible") == "false");
}

#[test]
fn mouse_focus_on_checkbox() {
    let harness = render(Checkbox::new(false));
    harness.click();
    assert_ne!(harness.data_attr("focus-visible"), "true");
}
```

### 8.3 Focus Visible Toggles on Focus/Blur

```rust
#[test]
fn focus_visible_clears_on_blur() {
    let harness = render(Button::new("Click me"));
    harness.press_key(KeyboardKey::Tab);
    assert_eq!(harness.data_attr("focus-visible"), "true");
    harness.blur();
    assert_ne!(harness.data_attr("focus-visible"), "true");
}
```

### 8.4 CSS Pseudo-Class Consistency

```rust
#[test]
fn focus_visible_css_matches_data_attribute() {
    let harness = render(Button::new("Click me"));
    harness.press_key(KeyboardKey::Tab);
    // The data-ars-focus-visible attribute should be set whenever
    // :focus-visible would match, allowing CSS selectors like:
    //   [data-ars-focus-visible="true"] { outline: 2px solid blue; }
    assert_eq!(harness.data_attr("focus-visible"), "true");
    // Verified via data-ars-focus-visible attribute above
    // (matches_css requires web_sys Element::matches(), not a TestHarness method)
}
```

### 8.5 FocusStrategy Behavioral Parity

```rust
#[wasm_bindgen_test]
async fn roving_tabindex_moves_dom_focus_and_updates_tabindex() {
    // RovingTabindex: one item has tabindex="0", rest have "-1"
    let harness = render(Listbox::new(vec![
        ListboxItem::new(Key::from("a"), "Alpha"),
        ListboxItem::new(Key::from("b"), "Beta"),
        ListboxItem::new(Key::from("c"), "Gamma"),
    ]).focus_strategy(FocusStrategy::RovingTabindex)).await;

    // First item has tabindex="0" by default
    assert_eq!(harness.item(0).attr("tabindex"), Some("0".into()));
    assert_eq!(harness.item(1).attr("tabindex"), Some("-1".into()));

    // Arrow down moves focus and swaps tabindex
    harness.press_key(KeyboardKey::ArrowDown).await;
    assert_eq!(harness.item(0).attr("tabindex"), Some("-1".into()));
    assert_eq!(harness.item(1).attr("tabindex"), Some("0".into()));
    assert!(harness.item(1).is_focused(), "DOM focus must move to second item");
}

#[wasm_bindgen_test]
async fn active_descendant_updates_aria_without_moving_dom_focus() {
    // ActiveDescendant: container keeps focus, aria-activedescendant points to highlighted item
    let harness = render(Combobox::new(vec![
        ComboboxItem::new(Key::from("a"), "Alpha"),
        ComboboxItem::new(Key::from("b"), "Beta"),
    ]).focus_strategy(FocusStrategy::ActiveDescendant)).await;

    harness.open().await;
    let combobox = harness.query("[role='combobox']");
    let first_id = harness.item(0).attr("id").expect("item must have id");
    assert_eq!(combobox.attr("aria-activedescendant"), Some(first_id.clone()));

    harness.press_key(KeyboardKey::ArrowDown).await;
    let second_id = harness.item(1).attr("id").expect("item must have id");
    assert_eq!(combobox.attr("aria-activedescendant"), Some(second_id),
        "aria-activedescendant must update to second item");
    // DOM focus stays on the combobox input, not on the item
    assert!(harness.query("[role='combobox']").is_focused(),
        "DOM focus must remain on combobox input");
}
```

---

## 9. Disabled Item Navigation Edge Cases

Selection components must correctly skip disabled items during keyboard navigation. Test `HighlightNext`, `HighlightPrev`, `HighlightFirst`, and `HighlightLast` with various `disabled_keys` configurations.

### 9.1 HighlightNext Skipping Disabled Items

```rust
#[test]
fn highlight_next_skips_disabled_items() {
    let collection = CollectionBuilder::new()
        .item(Key::from("a"), select::Item { label: "A".into() })
        .item(Key::from("b"), select::Item { label: "B".into() })
        .item(Key::from("c"), select::Item { label: "C".into() })
        .item(Key::from("d"), select::Item { label: "D".into() })
        .disabled(vec![Key::from("b"), Key::from("c")])
        .build();
    let props = select::Props::default();
    let mut svc = Service::<select::Machine>::new(props);
    svc.send(select::Event::UpdateItems(collection));
    svc.send(select::Event::Open);
    svc.send(select::Event::HighlightFirst); // highlights "a"
    assert_eq!(svc.context().highlighted_key, Some(Key::from("a")));

    svc.send(select::Event::HighlightNext); // skips "b" and "c", highlights "d"
    assert_eq!(svc.context().highlighted_key, Some(Key::from("d")));
}
```

### 9.2 HighlightFirst When First N Items Disabled

```rust,no_check
#[test]
fn highlight_first_skips_leading_disabled_items() {
    let collection = CollectionBuilder::new()
        .item(Key::from("a"), select::Item { label: "A".into() })
        .item(Key::from("b"), select::Item { label: "B".into() })
        .item(Key::from("c"), select::Item { label: "C".into() })
        .disabled(vec![Key::from("a"), Key::from("b")])
        .build();
    let props = select::Props::default();
    let mut svc = Service::<select::Machine>::new(props);
    svc.send(select::Event::UpdateItems(collection));
    svc.send(select::Event::Open);
    svc.send(select::Event::HighlightFirst);
    assert_eq!(
        svc.context().highlighted_key,
        Some(Key::from("c")),
        "HighlightFirst must skip disabled items a and b"
    );
}
```

### 9.3 HighlightLast When Last Item Disabled

```rust
#[test]
fn highlight_last_skips_trailing_disabled_items() {
    let collection = CollectionBuilder::new()
        .item(Key::from("a"), select::Item { label: "A".into() })
        .item(Key::from("b"), select::Item { label: "B".into() })
        .item(Key::from("c"), select::Item { label: "C".into() })
        .disabled(vec![Key::from("c")])
        .build();
    let props = select::Props::default();
    let mut svc = Service::<select::Machine>::new(props);
    svc.send(select::Event::UpdateItems(collection));
    svc.send(select::Event::Open);
    svc.send(select::Event::HighlightLast);
    assert_eq!(
        svc.context().highlighted_key,
        Some(Key::from("b")),
        "HighlightLast must skip disabled item c"
    );
}
```

### 9.4 HighlightPrev Skipping Disabled Items

```rust,no_check
#[test]
fn highlight_prev_skips_disabled_items() {
    let collection = CollectionBuilder::new()
        .item(Key::from("a"), select::Item { label: "A".into() })
        .item(Key::from("b"), select::Item { label: "B".into() })
        .item(Key::from("c"), select::Item { label: "C".into() })
        .item(Key::from("d"), select::Item { label: "D".into() })
        .disabled(vec![Key::from("b"), Key::from("c")])
        .build();
    let props = select::Props::default();
    let mut svc = Service::<select::Machine>::new(props);
    svc.send(select::Event::UpdateItems(collection));
    svc.send(select::Event::Open);
    svc.send(select::Event::HighlightLast); // highlights "d"
    svc.send(select::Event::HighlightPrev); // skips "c" and "b", highlights "a"
    assert_eq!(svc.context().highlighted_key, Some(Key::from("a")));
}
```

### 9.5 All Items Disabled

```rust
#[test]
fn highlight_noop_when_all_items_disabled() {
    let collection = CollectionBuilder::new()
        .item(Key::from("a"), select::Item { label: "A".into() })
        .item(Key::from("b"), select::Item { label: "B".into() })
        .disabled(vec![Key::from("a"), Key::from("b")])
        .build();
    let props = select::Props::default();
    let mut svc = Service::<select::Machine>::new(props);
    svc.send(select::Event::UpdateItems(collection));
    svc.send(select::Event::Open);
    svc.send(select::Event::HighlightFirst);
    assert_eq!(
        svc.context().highlighted_key,
        None,
        "No item should be highlighted when all items are disabled"
    );
}
```

### 9.6 Disabled-Item Navigation for Listbox, Menu, Combobox, Tree

The disabled-item skip behavior tested for Select above MUST also be verified for all
selection components that support `disabled_keys`.

```rust
#[test]
fn listbox_skips_disabled_items_on_arrow() {
    let props = listbox::Props {
        items: vec!["A", "B", "C"],
        disabled_keys: vec!["B".into()].into_iter().collect(),
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(listbox::Event::Open);
    svc.send(listbox::Event::HighlightFirst); // highlights "A"
    svc.send(listbox::Event::HighlightNext); // should skip "B", highlight "C"
    assert_eq!(
        svc.context().highlighted_key,
        Some(Key::from("C")),
        "Listbox HighlightNext must skip disabled item B"
    );
}

#[test]
fn menu_skips_disabled_items_on_arrow() {
    let props = menu::Props {
        items: vec![
            menu::Item::new("cut"),
            menu::Item::new("copy").disabled(true),
            menu::Item::new("paste"),
        ],
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(menu::Event::Open);
    svc.send(menu::Event::HighlightFirst); // highlights "cut"
    svc.send(menu::Event::HighlightNext); // should skip "copy", highlight "paste"
    assert_eq!(
        svc.context().highlighted_key,
        Some(Key::from("paste")),
        "Menu HighlightNext must skip disabled item copy"
    );
}

#[test]
fn combobox_skips_disabled_items_on_arrow() {
    let props = combobox::Props {
        items: vec!["Apple", "Banana", "Cherry"],
        disabled_keys: vec!["Banana".into()].into_iter().collect(),
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(combobox::Event::Open);
    svc.send(combobox::Event::HighlightFirst); // highlights "Apple"
    svc.send(combobox::Event::HighlightNext); // should skip "Banana", highlight "Cherry"
    assert_eq!(
        svc.context().highlighted_key,
        Some(Key::from("Cherry")),
        "Combobox HighlightNext must skip disabled item Banana"
    );
}

#[test]
fn tree_skips_disabled_items_on_arrow() {
    let tree_data = vec![
        tree::Node::new("a", "Node A"),
        tree::Node::new("b", "Node B"), // will be disabled
        tree::Node::new("c", "Node C"),
    ];
    let props = tree_view::Props {
        data: tree_data,
        disabled_keys: vec!["b".into()].into_iter().collect(),
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(tree_view::Event::FocusFirst); // focuses "a"
    svc.send(tree_view::Event::FocusNext); // should skip "b", focus "c"
    assert_eq!(
        svc.context().focused_node,
        Some(Key::from("c")),
        "TreeView FocusNext must skip disabled item b"
    );
}
```

---

## 10. RTL Keyboard Navigation

Components that use arrow keys for navigation MUST reverse horizontal arrows in RTL
(`dir: Direction::Rtl`). Affected components: RadioGroup, Tabs, Accordion (horizontal),
MenuBar, Slider, Splitter.

```rust
use ars_core::{KeyboardKey, Bindable};

#[test]
fn tabs_rtl_arrow_keys_reversed() {
    let props = tabs::Props { dir: Direction::Rtl, ..Default::default() };
    let mut svc = Service::new(props, Env::default(), Default::default());
    // In LTR, ArrowRight moves to next tab. In RTL, ArrowLeft moves to next tab.
    svc.send(tabs::Event::KeyDown(KeyboardKey::ArrowLeft));
    assert_eq!(svc.context().focused_index, 1, "ArrowLeft should move to NEXT tab in RTL");
}

#[test]
fn tabs_rtl_arrow_right_moves_prev() {
    let props = tabs::Props { dir: Direction::Rtl, ..Default::default() };
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(tabs::Event::KeyDown(KeyboardKey::ArrowLeft)); // Move to index 1
    svc.send(tabs::Event::KeyDown(KeyboardKey::ArrowRight)); // Move back to index 0
    assert_eq!(svc.context().focused_index, 0, "ArrowRight should move to PREVIOUS tab in RTL");
}

#[test]
fn slider_rtl_arrow_keys_reversed() {
    let props = slider::Props {
        dir: Direction::Rtl,
        value: Bindable::controlled(50.0),
        min: 0.0,
        max: 100.0,
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    let initial_value = svc.context().value;
    // In RTL: ArrowLeft should INCREASE value (reversed from LTR)
    svc.send(slider::Event::KeyDown(KeyboardKey::ArrowLeft));
    assert!(svc.context().value > initial_value, "ArrowLeft should INCREASE value in RTL");
}

#[test]
fn slider_rtl_arrow_right_decreases() {
    let props = slider::Props {
        dir: Direction::Rtl,
        value: Bindable::controlled(50.0),
        min: 0.0,
        max: 100.0,
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    let initial_value = svc.context().value;
    // In RTL: ArrowRight should DECREASE value (reversed from LTR)
    svc.send(slider::Event::KeyDown(KeyboardKey::ArrowRight));
    assert!(svc.context().value < initial_value, "ArrowRight should DECREASE value in RTL");
}

#[test]
fn radiogroup_rtl_arrow_keys_reversed() {
    let props = radio_group::Props {
        dir: Direction::Rtl,
        items: vec!["a", "b", "c"],
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    // ArrowLeft moves to next in RTL
    svc.send(radio_group::Event::KeyDown(KeyboardKey::ArrowLeft));
    assert_eq!(svc.context().focused_index, 1);
}

#[test]
fn splitter_rtl_arrow_keys_reversed() {
    let props = splitter::Props {
        dir: Direction::Rtl,
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    let initial_size = svc.context().primary_size;
    // In RTL, ArrowLeft increases the primary panel size
    svc.send(splitter::Event::KeyDown(KeyboardKey::ArrowLeft));
    assert!(svc.context().primary_size > initial_size,
        "ArrowLeft should INCREASE primary panel size in RTL");
}

#[test]
fn menubar_rtl_arrow_keys_reversed() {
    let props = menu_bar::Props {
        dir: Direction::Rtl,
        menus: vec!["File", "Edit", "View"],
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    // In RTL, ArrowLeft moves to the NEXT menu (reversed from LTR)
    svc.send(menu_bar::Event::KeyDown(KeyboardKey::ArrowLeft));
    assert_eq!(svc.context().focused_index, 1,
        "ArrowLeft should move to NEXT menu in RTL");
}

#[test]
fn accordion_horizontal_rtl_arrow_keys_reversed() {
    let props = accordion::Props {
        dir: Direction::Rtl,
        orientation: Orientation::Horizontal,
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    // In RTL horizontal, ArrowLeft moves to next trigger
    svc.send(accordion::Event::KeyDown(KeyboardKey::ArrowLeft));
    assert_eq!(svc.context().focused_index, 1,
        "ArrowLeft should move to NEXT trigger in RTL horizontal accordion");
}
```

---

## 11. Type-Ahead Navigation

Selection components (Listbox, Select, Menu, Tree) support type-ahead:
NFC-normalized matching, 500ms timeout, buffer clears after timeout.

```rust
#[test]
fn listbox_typeahead_prefix_match() {
    let props = listbox::Props {
        items: vec!["Apple", "Banana", "Blueberry", "Cherry"],
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(listbox::Event::Open);
    // Type "ba" within 500ms → focuses first item starting with "ba"
    svc.send(listbox::Event::TypeaheadSearch('b', 0));
    svc.send(listbox::Event::TypeaheadSearch('a', 0));
    assert_eq!(
        svc.context().highlighted_key,
        Some(Key::from("Banana")),
        "Type-ahead 'ba' should match 'Banana'"
    );
}

#[test]
fn listbox_typeahead_timeout_clears_buffer() {
    let props = listbox::Props {
        items: vec!["Apple", "Banana", "Cherry"],
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(listbox::Event::Open);
    // Type "b", wait 600ms (past 500ms timeout), then type "a"
    svc.send(listbox::Event::TypeaheadSearch('b', 0));
    svc.advance_time(Duration::from_millis(600));
    svc.send(listbox::Event::TypeaheadSearch('a', 0));
    // Should search for "a" not "ba" — matches "Apple"
    assert_eq!(
        svc.context().highlighted_key,
        Some(Key::from("Apple")),
        "After timeout, buffer should clear — 'a' matches 'Apple' not 'ba' for 'Banana'"
    );
}

#[test]
fn listbox_typeahead_repeated_char_cycles() {
    let props = listbox::Props {
        items: vec!["Alpha", "Apex", "Atom", "Beta"],
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(listbox::Event::Open);
    // Type "a" three times → cycles through items starting with "a"
    svc.send(listbox::Event::TypeaheadSearch('a', 0));
    assert_eq!(svc.context().highlighted_key, Some(Key::from("Alpha")));
    svc.send(listbox::Event::TypeaheadSearch('a', 0));
    assert_eq!(svc.context().highlighted_key, Some(Key::from("Apex")));
    svc.send(listbox::Event::TypeaheadSearch('a', 0));
    assert_eq!(svc.context().highlighted_key, Some(Key::from("Atom")));
    // Wraps around
    svc.send(listbox::Event::TypeaheadSearch('a', 0));
    assert_eq!(svc.context().highlighted_key, Some(Key::from("Alpha")));
}

#[test]
fn listbox_typeahead_skips_disabled_items() {
    let props = listbox::Props {
        items: vec!["Apple", "Avocado", "Apricot"],
        disabled_keys: vec!["Avocado".into()].into_iter().collect(),
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(listbox::Event::Open);
    svc.send(listbox::Event::TypeaheadSearch('a', 0));
    assert_eq!(svc.context().highlighted_key, Some(Key::from("Apple")));
    // Next "a" should skip disabled "Avocado" and go to "Apricot"
    svc.send(listbox::Event::TypeaheadSearch('a', 0));
    assert_eq!(
        svc.context().highlighted_key,
        Some(Key::from("Apricot")),
        "Type-ahead must skip disabled items"
    );
}
```

### 11.1 IME Composition Suppresses Type-Ahead

Per [05-interactions.md](../foundation/05-interactions.md) §11.5, components MUST check `is_composing` and suppress character-keyed interactions during IME composition.

```rust
#[test]
fn ime_composition_suppresses_typeahead() {
    let props = listbox::Props::default();
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(listbox::Event::Focus);

    // Begin IME composition — should NOT trigger type-ahead
    svc.send(listbox::Event::TypeaheadSearch('あ', 0));

    // Type-ahead buffer must remain empty during composition
    assert!(
        svc.context().typeahead.buffer().is_empty(),
        "type-ahead buffer must not update during IME composition (is_composing=true)"
    );
}
```

---

## 12. Long-Press Interaction

Long-press has a 500ms threshold (configurable), a move dead-zone of 5-10px,
and states: Idle -> Timing -> LongPressed.

```rust
#[test]
fn long_press_triggers_after_threshold() {
    let props = long_press::Props {
        threshold: Duration::from_millis(500),
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(long_press::Event::PointerDown { x: 100.0, y: 100.0 });
    assert_eq!(*svc.state(), long_press::State::Timing);
    svc.advance_time(Duration::from_millis(500));
    assert_eq!(*svc.state(), long_press::State::LongPressed);
}

#[test]
fn long_press_cancels_on_pointer_up_before_threshold() {
    let props = long_press::Props {
        threshold: Duration::from_millis(500),
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(long_press::Event::PointerDown { x: 100.0, y: 100.0 });
    assert_eq!(*svc.state(), long_press::State::Timing);
    svc.advance_time(Duration::from_millis(200));
    svc.send(long_press::Event::PointerUp);
    assert_eq!(*svc.state(), long_press::State::Idle);
}

#[test]
fn long_press_cancels_on_move_beyond_dead_zone() {
    let props = long_press::Props {
        threshold: Duration::from_millis(500),
        move_dead_zone: 10.0,
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(long_press::Event::PointerDown { x: 100.0, y: 100.0 });
    assert_eq!(*svc.state(), long_press::State::Timing);
    // Move pointer > 10px from origin
    svc.send(long_press::Event::PointerMove { x: 115.0, y: 100.0 });
    assert_eq!(
        *svc.state(),
        long_press::State::Idle,
        "Moving beyond dead zone must cancel long-press"
    );
}

#[wasm_bindgen_test]
async fn long_press_element_has_keyboard_alternative_description() {
    let callback = Callback::new(|_| {});
    let props = button::Props { on_long_press: Some(callback), ..Default::default() };
    let harness = render(Button::new(props)).await;
    let btn = harness.query_selector("[data-ars-part='root']")
        .expect("query must not error").expect("button must exist");
    let described_by = btn.attr("aria-describedby")
        .expect("long-press element must have aria-describedby");
    let desc_el = harness.query_selector(&format!("#{described_by}"))
        .expect("query must not error").expect("description element must exist");
    assert!(!desc_el.text_content().unwrap_or_default().is_empty(),
        "keyboard alternative description must not be empty");
}
```

### 12.3 LongPress Accessibility Description

```rust
#[test]
fn long_press_description_attrs_returns_aria_describedby_link() {
    let ids = ComponentIds::from_id("btn-1");
    let config = LongPressConfig {
        accessibility_description: Some("Long press for more options".into()),
        ..Default::default()
    };
    let result = use_long_press(config);

    let desc_attrs = result.description_attrs(&ids)
        .expect("description_attrs must return Some when accessibility_description is set");
    assert_eq!(desc_attrs.get(&HtmlAttr::Id), Some("btn-1-long-press-desc"),
        "description element must have predictable ID");
}

#[test]
fn long_press_no_description_returns_none() {
    let ids = ComponentIds::from_id("btn-2");
    let config = LongPressConfig::default();
    let result = use_long_press(config);

    assert!(result.description_attrs(&ids).is_none(),
        "description_attrs must return None when no accessibility_description");
}
```

---

## 13. Move Interaction

Move interaction translates keyboard arrow keys into deltas. Base delta is 1 unit;
Shift modifier increases to 10 units. RTL reverses horizontal direction.
Used by: Slider, ColorPicker, Splitter.

```rust
#[test]
fn move_interaction_keyboard_delta() {
    let mut move_ctx = move_interaction::Context::new(Direction::Ltr);
    let delta = move_ctx.handle_key(KeyboardKey::ArrowRight, Modifiers::empty());
    assert_eq!(delta, Some(MoveDelta { dx: 1.0, dy: 0.0 }));

    let delta = move_ctx.handle_key(KeyboardKey::ArrowRight, Modifiers::SHIFT);
    assert_eq!(delta, Some(MoveDelta { dx: 10.0, dy: 0.0 }));

    let delta = move_ctx.handle_key(KeyboardKey::ArrowUp, Modifiers::empty());
    assert_eq!(delta, Some(MoveDelta { dx: 0.0, dy: -1.0 }));

    let delta = move_ctx.handle_key(KeyboardKey::ArrowDown, Modifiers::SHIFT);
    assert_eq!(delta, Some(MoveDelta { dx: 0.0, dy: 10.0 }));
}

#[test]
fn move_interaction_rtl_reverses_horizontal() {
    let mut move_ctx = move_interaction::Context::new(Direction::Rtl);
    // In RTL: ArrowRight → negative delta (reversed from LTR)
    let delta = move_ctx.handle_key(KeyboardKey::ArrowRight, Modifiers::empty());
    assert_eq!(delta, Some(MoveDelta { dx: -1.0, dy: 0.0 }));

    // ArrowLeft → positive delta in RTL
    let delta = move_ctx.handle_key(KeyboardKey::ArrowLeft, Modifiers::empty());
    assert_eq!(delta, Some(MoveDelta { dx: 1.0, dy: 0.0 }));

    // Vertical keys are NOT affected by RTL
    let delta = move_ctx.handle_key(KeyboardKey::ArrowUp, Modifiers::empty());
    assert_eq!(delta, Some(MoveDelta { dx: 0.0, dy: -1.0 }));
}
```

### 13.1 Scroll Lock Tests

Tests verify that body scroll is locked when modal overlays are open, per [foundation/11-dom-utilities.md section 5](../foundation/11-dom-utilities.md#5-scroll-locking).

```rust
fn popover_props() -> popover::Props { popover::Props::default() }
fn dialog_props() -> dialog::Props {
    dialog::Props { modal: false, ..Default::default() }
}
fn modal_dialog_props() -> dialog::Props {
    dialog::Props { modal: true, ..Default::default() }
}

#[wasm_bindgen_test]
async fn dialog_locks_body_scroll_on_open() {
    let harness = render(Dialog::new(dialog_props())).await;
    harness.open();
    harness.tick().await;
    let body_style = document().body().expect("body must exist").style();
    assert_eq!(body_style.get_property_value("overflow").expect("overflow must be readable"), "hidden");
}

#[wasm_bindgen_test]
async fn dialog_unlocks_body_scroll_on_close() {
    let harness = render(Dialog::new(dialog_props())).await;
    harness.open();
    harness.tick().await;
    harness.close();
    harness.tick().await;
    let body_style = document().body().expect("body must exist").style();
    assert_ne!(body_style.get_property_value("overflow").expect("overflow must be readable"), "hidden");
}

#[wasm_bindgen_test]
async fn nested_overlay_maintains_scroll_lock() {
    let harness = render(Dialog::new(dialog_props())).await;
    harness.open();
    harness.tick().await;
    // Open nested popover inside dialog
    let inner = harness.query("[data-ars-part='popover-trigger']").expect("inner trigger must exist");
    inner.click();
    harness.tick().await;
    // Close inner popover — scroll should stay locked (dialog still open)
    harness.press_key(KeyboardKey::Escape);
    harness.tick().await;
    let body_style = document().body().expect("body must exist").style();
    assert_eq!(body_style.get_property_value("overflow").expect("overflow must be readable"), "hidden");
}
```

### 13.2 InteractOutside Tests

Tests verify overlay dismissal on outside interaction, per [foundation/05-interactions.md section 12](../foundation/05-interactions.md#12-interactoutside-interaction).

```rust
#[wasm_bindgen_test]
async fn outside_click_closes_non_modal_popover() {
    let harness = render(Popover::new(popover_props())).await;
    harness.open();
    harness.tick().await;
    // Click outside the popover
    document().body().expect("body must exist").click();
    harness.tick().await;
    assert!(!harness.is_open(), "popover must close on outside click");
}

#[wasm_bindgen_test]
async fn outside_click_does_not_close_modal_dialog() {
    let harness = render(Dialog::new(modal_dialog_props())).await;
    harness.open();
    harness.tick().await;
    // Click on the backdrop (not outside — modal captures all interaction)
    document().body().expect("body must exist").click();
    harness.tick().await;
    assert!(harness.is_open(), "modal dialog must NOT close on outside click");
}

#[wasm_bindgen_test]
async fn outside_focus_closes_non_modal_overlay() {
    // Set up an outside element for focus-outside detection
    let outside = document().create_element("input").expect("create element");
    outside.set_attribute("data-testid", "outside-input").expect("set attr");
    document().body().expect("body").append_child(&outside).expect("append");

    let harness = render(Popover::new(popover_props())).await;
    harness.open();
    harness.tick().await;
    // Focus an element outside the popover
    let outside = document().query_selector("[data-testid='outside-input']")
        .expect("query must not error").expect("outside input must exist");
    outside.dyn_ref::<web_sys::HtmlElement>().expect("outside is HtmlElement").focus().expect("focus");
    harness.tick().await;
    assert!(!harness.is_open(), "popover must close on outside focus");
}
```

### 13.3 Popover Focus Management

Popover uses non-modal focus semantics: focus moves to popover on open, Tab can leave freely (no trap), Escape closes and restores focus.

```rust
#[wasm_bindgen_test]
async fn popover_moves_focus_on_open() {
    let harness = render(Popover::new(popover_props())).await;
    harness.open();
    harness.tick().await;
    let active = document().active_element().expect("an element must be focused");
    let popover = harness.query("[data-ars-part='content']").expect("popover content must exist");
    assert!(popover.contains(Some(&active)), "focus must be inside popover on open");
}

#[wasm_bindgen_test]
async fn popover_allows_tab_out() {
    let harness = render(Popover::new(popover_props())).await;
    harness.open();
    harness.tick().await;
    // Tab through all focusable elements — focus should eventually leave popover
    for _ in 0..10 {
        harness.press_key(KeyboardKey::Tab);
        harness.tick().await;
    }
    let active = document().active_element().expect("an element must be focused");
    let popover = harness.query("[data-ars-part='content']").expect("popover content must exist");
    assert!(!popover.contains(Some(&active)), "focus must be able to leave popover (no trap)");
}

#[wasm_bindgen_test]
async fn popover_escape_closes_and_restores_focus() {
    let harness = render(Popover::new(popover_props())).await;
    let trigger = harness.query("[data-ars-part='trigger']").expect("trigger must exist");
    trigger.click();
    harness.tick().await;
    harness.press_key(KeyboardKey::Escape);
    harness.tick().await;
    assert!(!harness.is_open(), "popover must close on Escape");
    let active = document().active_element().expect("an element must be focused");
    assert_eq!(active, trigger, "focus must restore to trigger after Escape");
}
```

### 13.4 Hover Interaction

Tests for the Hover Interaction pattern from foundation 05-interactions.md section 3.

```rust
/// Create a PointerEvent with the given pointer type and event name.
fn pointer_event(pointer_type: &str, event_name: &str) -> web_sys::PointerEvent {
    let init = web_sys::PointerEventInit::new();
    init.set_pointer_type(pointer_type);
    init.set_bubbles(true);
    web_sys::PointerEvent::new_with_pointer_event_init_dict(event_name, &init)
        .expect("PointerEvent construction must succeed")
}

#[wasm_bindgen_test]
async fn hover_sets_data_attribute_on_pointer_enter() {
    let props = button::Props::default();
    let harness = render(Button::new(props)).await;
    let btn = harness.query_selector("[data-ars-part='root']")
        .expect("query must not error").expect("button must exist");

    btn.dispatch_event(&pointer_event("mouse", "pointerenter")).expect("dispatch");
    assert!(btn.attr("data-ars-hovered").is_some(),
        "pointer enter must set data-ars-hovered");

    btn.dispatch_event(&pointer_event("mouse", "pointerleave")).expect("dispatch");
    assert!(btn.attr("data-ars-hovered").is_none(),
        "pointer leave must clear data-ars-hovered");
}

#[wasm_bindgen_test]
async fn hover_suppressed_during_active_press() {
    let props = button::Props::default();
    let harness = render(Button::new(props)).await;
    let btn = harness.query_selector("[data-ars-part='root']")
        .expect("query must not error").expect("button must exist");

    btn.dispatch_event(&pointer_event("mouse", "pointerdown")).expect("dispatch");
    // Move pointer out and back during press
    btn.dispatch_event(&pointer_event("mouse", "pointerleave")).expect("dispatch");
    btn.dispatch_event(&pointer_event("mouse", "pointerenter")).expect("dispatch");
    // Hover should not re-activate during press
    assert!(btn.attr("data-ars-hovered").is_none(),
        "hover must not re-activate during active press");
    btn.dispatch_event(&pointer_event("mouse", "pointerup")).expect("dispatch");
}

#[wasm_bindgen_test]
async fn touch_pointer_does_not_trigger_hover() {
    let props = button::Props::default();
    let harness = render(Button::new(props)).await;
    let btn = harness.query_selector("[data-ars-part='root']")
        .expect("query must not error").expect("button must exist");

    btn.dispatch_event(&pointer_event("touch", "pointerenter")).expect("dispatch");
    assert_eq!(btn.attr("data-ars-hovered"), None,
        "touch pointer type must not trigger hover");
}
```

## 14. Page Visibility Timer Pausing

Per [05-interactions.md](../foundation/05-interactions.md) §12.5.1, components with timers or animations MUST pause when the page is hidden (`visibilityState === "hidden"`) and resume when visibility is restored.

```rust
#[wasm_bindgen_test]
async fn tooltip_delay_pauses_during_page_hidden() {
    let harness = render(Tooltip::new("t1", "Help text")).await;
    tick().await;

    // Trigger hover to start tooltip delay
    harness.hover("[data-ars-part='trigger']");
    tick().await;

    // Simulate page becoming hidden via send event
    harness.send(visibility::Event::PageHidden);

    // Advance time past the normal tooltip delay
    harness.advance_time(Duration::from_millis(1000));
    tick().await;

    // Tooltip should NOT have opened — timer was paused
    assert!(harness.query("[role='tooltip']").is_none(),
        "tooltip must not open while page is hidden");

    // Restore visibility
    harness.send(visibility::Event::PageVisible);
    // Remaining delay should resume
    harness.advance_time(Duration::from_millis(500));
    tick().await;

    assert!(harness.query("[role='tooltip']").is_some(),
        "tooltip must open after visibility restored and remaining delay elapsed");
}

#[wasm_bindgen_test]
async fn toast_auto_dismiss_pauses_during_page_hidden() {
    let harness = render(Toast::new("toast1", "Message", /* auto_dismiss_ms */ 3000)).await;
    tick().await;

    assert!(harness.query("[role='status']").is_some(), "toast must be visible initially");

    // Simulate page hidden after 1000ms
    harness.advance_time(Duration::from_millis(1000));
    harness.send(visibility::Event::PageHidden);

    // Advance past total dismiss time while hidden
    harness.advance_time(Duration::from_millis(5000));
    tick().await;

    // Toast must still be visible — timer paused
    assert!(harness.query("[role='status']").is_some(),
        "toast must not auto-dismiss while page is hidden");

    // Restore visibility — remaining 2000ms should count down
    harness.send(visibility::Event::PageVisible);
    harness.advance_time(Duration::from_millis(2000));
    tick().await;

    assert!(harness.query("[role='status']").is_none(),
        "toast must auto-dismiss after remaining time elapses");
}
```

---

## 15. FocusWithin Tests

`FocusWithin` tracks whether any descendant has focus and produces `data-ars-focus-within`
and `data-ars-focus-within-visible` data attributes.

```rust
// mount_component wraps TestHarness::mount() — see 05-adapter-harness.md §3 for definition.
// async fn mount_component<C: Component>(c: C) -> TestHarness { TestHarness::mount(c).await }

#[wasm_bindgen_test]
async fn focus_within_set_on_child_focus() {
    let harness = mount_component(TextField::new("tf1"));
    tick().await;
    let group = harness.query("[data-ars-scope='tf1']");
    assert_eq!(group.get_attribute("data-ars-focus-within"), None);

    harness.focus("[data-ars-scope='tf1'] input");
    tick().await;
    assert_eq!(group.get_attribute("data-ars-focus-within"), Some("true".into()));
}

#[wasm_bindgen_test]
async fn focus_within_clears_on_blur() {
    let harness = mount_component(TextField::new("tf1"));
    tick().await;
    harness.focus("[data-ars-scope='tf1'] input");
    tick().await;
    harness.blur(); // Blur the currently focused element
    tick().await;
    let group = harness.query("[data-ars-scope='tf1']");
    assert_eq!(group.get_attribute("data-ars-focus-within"), None);
}

#[wasm_bindgen_test]
async fn focus_within_visible_on_keyboard_focus() {
    let harness = mount_component(TextField::new("tf1"));
    tick().await;
    harness.press_key(KeyboardKey::Tab); // keyboard focus into input
    tick().await;
    let group = harness.query("[data-ars-scope='tf1']");
    assert_eq!(group.get_attribute("data-ars-focus-within"), Some("true".into()));
    assert_eq!(group.get_attribute("data-ars-focus-within-visible"), Some("true".into()));
}
```

---

## 16. Presence State Naming and Timeout

### 16.1 Presence Unmount Timeout Fallback

If `animationend` never fires, a safety timeout must force transition to `Unmounted`.

```rust
#[test]
fn presence_unmount_timeout_fallback() {
    let harness = render(Presence::new(true).animation_timeout(Duration::from_millis(5000)));
    assert_eq!(harness.state(), PresenceState::Mounted);

    // Trigger unmount
    harness.set_present(false);
    assert_eq!(harness.state(), PresenceState::UnmountPending);
    assert!(harness.is_mounted()); // Still in DOM during animation

    // Do NOT fire animationend — simulate a stalled animation
    harness.advance_time(Duration::from_millis(5000));

    // Timeout must force cleanup
    assert_eq!(
        harness.state(),
        PresenceState::Unmounted,
        "Timeout fallback must transition to Unmounted when animationend never fires"
    );
    assert!(!harness.is_mounted());
}
```

---

## 17. Extended Disabled Guard Components

The following components MUST also have `test_disabled_guard!` invocations to verify
that all state-changing events are blocked when disabled.

```rust
test_disabled_guard!(listbox_disabled, listbox, vec![
    listbox::Event::Open,
    listbox::Event::HighlightNext,
    listbox::Event::HighlightPrev,
    listbox::Event::SelectHighlighted,
]);

test_disabled_guard!(menu_component_disabled, menu, vec![
    menu::Event::Open,
    menu::Event::Close,
    menu::Event::HighlightNext,
    menu::Event::SelectHighlighted,
]);

test_disabled_guard!(tags_input_disabled, tags_input, vec![
    tags_input::Event::AddTag("new".into()),
    tags_input::Event::RemoveTag("existing".into()),
    tags_input::Event::InputChange("test".into()),
]);

test_disabled_guard!(pin_input_disabled, pin_input, vec![
    pin_input::Event::InputChar { index: 0, char: '1' },
    pin_input::Event::Backspace,
    pin_input::Event::Paste("1234".into()),
]);

test_disabled_guard!(file_upload_disabled, file_upload, vec![
    file_upload::Event::FilesDropped(vec![]),
    file_upload::Event::RemoveFile("file.txt".into()),
    file_upload::Event::OpenFilePicker,
]);

test_disabled_guard!(signature_pad_disabled, signature_pad, vec![
    signature_pad::Event::DrawStart { x: 0.0, y: 0.0 },
    signature_pad::Event::DrawMove { x: 10.0, y: 10.0 },
    signature_pad::Event::DrawEnd,
    signature_pad::Event::Clear,
]);
```
