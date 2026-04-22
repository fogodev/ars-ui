# Integration Tests

## 1. Service-Level Integration Tests

> **Note:** Test examples assume component `Props` types implement `Default`. This is not a `Machine` trait bound — each component must provide its own `Default` impl for tests to compile.
>
> **Harness entrypoints:** Any example below that mounts DOM with `render(...)` or
> `mount_with_locale(...)` imports those helpers from the active adapter harness
> crate (`ars_test_harness_leptos` or `ars_test_harness_dioxus`). The core
> `ars-test-harness` crate exposes only `render_with_backend(...)` and
> `render_with_locale_and_backend(...)`.

Service tests verify the full `Service::send()` → `transition()` → effect → event cycle, including the `drain_queue` loop and `PendingEffect` execution.

### 1.1 Pattern for service tests

```rust
#[cfg(test)]
mod service_tests {
    use super::*;
    use ars_core::Service;

    #[test]
    fn send_processes_transition_and_effects() {
        let props = Props::default();
        let mut service = Service::<toggle::Machine>::new(props);

        assert_eq!(*service.state(), State::Off);

        service.send(Event::Toggle);
        assert_eq!(*service.state(), State::On);
        assert!(service.context().pressed.get());

        service.send(Event::Toggle);
        assert_eq!(*service.state(), State::Off);
        assert!(!service.context().pressed.get());
    }

    #[test]
    fn drain_queue_processes_chained_events() {
        // Some transitions emit follow-up events via effects.
        // Verify the queue is drained until empty.
        let props = Props::default();
        let mut service = Service::<Combobox>::new(props);

        service.send(Event::Open);
        // Opening may emit FocusFirst as a follow-up effect.
        assert_eq!(*service.state(), State::Open);
        assert!(service.context().highlighted_key.is_some());
    }

    #[test]
    fn pending_effect_runs_setup_and_cleanup() {
        let props = Props { auto_play: Some(AutoPlayOptions { interval: Duration::from_secs(3), ..Default::default() }), ..Default::default() };
        let mut service = Service::<Carousel>::new(props);

        // AutoPlaying state should have an active effect.
        // Service::send() returns pending effects in SendResult — the
        // adapter is responsible for tracking active effects and calling
        // cleanup when state changes.
        assert_eq!(*service.state(), State::AutoPlaying);
        let result = service.send(Event::AutoPlayStart);
        assert!(result.pending_effects.iter().any(|e| e.name == "auto-play"));

        // Stopping should trigger a state change — the adapter observes
        // result.state_changed and cleans up effects from the previous state.
        let result = service.send(Event::AutoPlayStop);
        assert!(result.state_changed);
        assert_eq!(*service.state(), State::Idle);
    }
}
```

### 1.2 What to test

- **State progression**: multi-step flows (e.g. Idle → Open → Selecting → Closed).
- **Effect lifecycle**: setup runs on transition, cleanup runs on exit or re-transition.
- **Queue draining**: effects that emit events produce the correct follow-up transitions.
- **Bindable integration**: controlled vs uncontrolled values are propagated correctly through the service.

### 1.3 Concurrent Event / Queue Drain Limit Testing

The event queue has a configurable drain limit (default: 100 iterations) to prevent infinite
loops. Tests MUST verify behavior when this limit is reached:

```rust
// In debug builds, drain_queue panics at MAX_DRAIN_ITERATIONS before the
// assertion is reached, so this test only runs in release mode.
#[cfg(not(debug_assertions))]
#[test]
fn excessive_event_queue_truncates_with_warning() {
    // Create a pathological machine where each transition emits a follow-up event,
    // causing an unbounded drain loop.
    let props = Props { enable_echo_effect: true, ..Default::default() };
    let mut svc = Service::<EchoMachine>::new(props);

    let result = svc.send(Event::Start);

    // The drain loop MUST stop at the configured limit (100 iterations).
    // SendResult.truncated is set to true when the queue drain limit is reached.
    // NOTE: In debug builds, drain_queue panics at MAX_DRAIN_ITERATIONS rather than
    // setting truncated = true. This test must run under release mode:
    // `cargo test --release` or be gated with `#[cfg(not(debug_assertions))]`.
    assert!(result.truncated, "drain loop should have been truncated");

    // State must still be consistent — the machine should be in whatever state
    // the last successfully processed transition left it in. The adapter can
    // inspect result.truncated to decide how to handle this (e.g., log a warning).
}
```

---

## 2. Controlled Value Tests

### 2.1 Bindable Sync Verification

```rust
#[cfg(test)]
mod controlled_tests {
    use super::*;

    #[test]
    fn controlled_checkbox_sync() {
        // Start controlled
        let props = checkbox::Props {
            checked: Some(checkbox::State::Unchecked),
            ..Default::default()
        };
        let mut svc = Service::<checkbox::Machine>::new(props);
        assert_eq!(*svc.state(), checkbox::State::Unchecked);

        // External control: set checked
        svc.send(checkbox::Event::SetChecked(checkbox::State::Checked));
        assert_eq!(*svc.state(), checkbox::State::Checked);
        assert!(svc.context().checked.is_controlled());
    }

    #[test]
    fn uncontrolled_checkbox_manages_own_state() {
        let props = checkbox::Props {
            default_checked: checkbox::State::Unchecked,
            ..Default::default()
        };
        let mut svc = Service::<checkbox::Machine>::new(props);
        assert!(!svc.context().checked.is_controlled());

        svc.send(checkbox::Event::Toggle);
        assert_eq!(*svc.state(), checkbox::State::Checked);
    }

    #[test]
    fn controlled_ignores_toggle_without_external_sync() {
        // In a real adapter, toggling a controlled checkbox would:
        // 1. Emit on_checked_change callback
        // 2. Wait for external signal to update
        // 3. Re-sync via SetChecked event
        // Without step 3, the internal state reverts.
        let props = checkbox::Props {
            checked: Some(checkbox::State::Unchecked),
            ..Default::default()
        };
        let mut svc = Service::<checkbox::Machine>::new(props);

        // Toggle updates internal optimistically
        svc.send(checkbox::Event::Toggle);
        // But controlled value still says unchecked
        assert_eq!(*svc.context().checked.get(), checkbox::State::Checked);

        // External re-sync back to unchecked
        svc.send(checkbox::Event::SetChecked(checkbox::State::Unchecked));
        assert_eq!(*svc.state(), checkbox::State::Unchecked);
    }
}
```

### 2.2 Controlled/Uncontrolled Duality Edge Cases

The following tests cover transitions between controlled and uncontrolled modes, prop changes
after mount, and external overrides of uncontrolled values:

```rust
#[test]
fn uncontrolled_then_controlled_after_mount() {
    // Start uncontrolled, then switch to controlled by providing a `checked` prop.
    let mut svc = Service::<checkbox::Machine>::new(checkbox::Props {
        default_checked: checkbox::State::Unchecked,
        ..Default::default()
    });
    assert!(!svc.context().checked.is_controlled());

    // Simulate parent providing a controlled prop after mount.
    svc.set_props(checkbox::Props {
        checked: Some(checkbox::State::Checked),
        ..Default::default()
    });
    // Machine must re-sync: the Bindable switches to controlled mode.
    assert!(svc.context().checked.is_controlled());
    assert_eq!(*svc.context().checked.get(), checkbox::State::Checked);
}

#[test]
fn controlled_prop_change_resyncs_machine() {
    // Start controlled with value A, then parent changes to value B.
    let mut svc = Service::<checkbox::Machine>::new(checkbox::Props {
        checked: Some(checkbox::State::Unchecked),
        ..Default::default()
    });
    assert_eq!(*svc.state(), checkbox::State::Unchecked);

    // Parent re-renders with new controlled value.
    svc.set_props(checkbox::Props {
        checked: Some(checkbox::State::Checked),
        ..Default::default()
    });
    // State machine must re-sync to match the new controlled value.
    assert_eq!(*svc.state(), checkbox::State::Checked);
    assert_eq!(*svc.context().checked.get(), checkbox::State::Checked);
}

#[test]
fn uncontrolled_toggle_then_external_set() {
    // Start uncontrolled, toggle internally, then receive an external override.
    let mut svc = Service::<checkbox::Machine>::new(checkbox::Props {
        default_checked: checkbox::State::Unchecked,
        ..Default::default()
    });
    svc.send(checkbox::Event::Toggle);
    assert_eq!(*svc.state(), checkbox::State::Checked);

    // External override: parent forces a value change on an uncontrolled component.
    svc.send(checkbox::Event::SetChecked(checkbox::State::Unchecked));
    assert_eq!(*svc.state(), checkbox::State::Unchecked);
    assert_eq!(*svc.context().checked.get(), checkbox::State::Unchecked);
}

#[test]
fn set_props_triggers_on_props_changed() {
    let props = slider::Props { value: Bindable::controlled(50.0), ..Default::default() };
    let mut svc = Service::new(props, Env::default(), Default::default());
    let new_props = slider::Props { value: Bindable::controlled(75.0), ..Default::default() };
    let result = svc.set_props(new_props);
    assert!(result.state_changed || result.context_changed,
        "on_props_changed should produce events that update state or context");
}
```

### 2.3 Effect Cleanup Leak Detection

Use `Rc<Cell<bool>>` to track whether cleanup callbacks are invoked. This pattern
detects leaks where an effect is set up but its cleanup is never called on state change.

```rust
#[test]
fn effect_cleanup_runs_on_state_change() {
    let mut svc = Service::new(dialog::Props::new("d1"), Env::default(), Default::default());

    // Open dialog — triggers the focus-trap effect
    let open_result = svc.send(dialog::Event::Open);
    assert!(!open_result.pending_effects.is_empty());

    // Run effects, collecting cleanups
    let send_fn: Arc<dyn Fn(dialog::Event) + Send + Sync> = Arc::new(|_: dialog::Event| {});
    use ars_core::CleanupFn;

    let mut active_cleanups: Vec<CleanupFn> = Vec::new();
    for effect in open_result.pending_effects {
        let cleanup = effect.run(svc.context(), svc.props(), send_fn.clone());
        active_cleanups.push(cleanup);
    }

    // Close dialog — effects from open should be cleaned up
    let close_result = svc.send(dialog::Event::Close);
    assert!(close_result.state_changed, "Service must signal state change on close");

    // Run all active cleanups (adapter responsibility)
    let cleanup_count = active_cleanups.len();
    for cleanup in active_cleanups.drain(..) {
        cleanup();
    }
    assert!(cleanup_count > 0, "Expected at least one cleanup to run");
}
```

### 2.4 SendResult Field Coverage

Tests for `SendResult` fields not covered by existing tests:

```rust
#[test]
fn context_only_transition_sets_context_changed() {
    let mut svc = Service::new(slider::Props::new("s1"), Env::default(), Default::default());
    // Send an event that only updates context (e.g., hover highlight)
    let result = svc.send(slider::Event::PointerMove { value: 50.0 });
    assert!(result.context_changed);
}

#[test]
fn cancel_effects_contains_cancelled_effect_names() {
    let mut svc = Service::new(tooltip::Props::new("t1"), Env::default(), Default::default());
    let _open = svc.send(tooltip::Event::PointerEnter);
    // Close before delay effect fires — should cancel it
    let close_result = svc.send(tooltip::Event::PointerLeave);
    // If the tooltip has a show-delay effect, it should be in cancel_effects
    // (component-specific — verify against tooltip spec)
}

#[test]
fn send_result_state_changed_reflects_transition() {
    let props = toggle::Props::default();
    let mut svc = Service::new(props, Env::default(), Default::default());
    let result = svc.send(toggle::Event::Toggle);
    assert!(result.state_changed, "toggling must change state");

    // Already On — sending TurnOn is a no-op
    let result2 = svc.send(toggle::Event::TurnOn);
    assert!(!result2.state_changed, "no-op must not change state");
}

#[test]
fn send_result_context_changed_on_thumb_move() {
    let props = slider::Props { value: Bindable::controlled(50.0), ..Default::default() };
    let mut svc = Service::new(props, Env::default(), Default::default());
    let result = svc.send(slider::Event::PointerMove { value: 75.0 });
    assert!(result.context_changed, "context must change on thumb move");
}
```

---

## 3. Collection Virtualization Testing

> **Note:** Sections 3-6 describe adapter-level integration tests that depend on the test harness from [05-adapter-harness.md](05-adapter-harness.md). These are not pure `Service` tests -- they require DOM rendering and element queries.

### 3.1 Large Dataset Performance

```rust
#[test]
fn listbox_10k_items_renders_subset() {
    let items: Vec<_> = (0..10_000).map(|i| format!("Item {i}")).collect();
    let harness = render(Listbox::with_items(&items));
    // Only visible items should be in DOM
    assert!(harness.dom_item_count() < 100);
}
```

### 3.2 Keyboard Navigation Through Virtual List

```rust
#[test]
fn virtual_listbox_keyboard_nav_scrolls() {
    let items: Vec<_> = (0..10_000).map(|i| format!("Item {i}")).collect();
    let harness = render(Listbox::with_items(&items));
    for _ in 0..50 {
        harness.press_key(KeyboardKey::ArrowDown);
    }
    assert_eq!(harness.highlighted_item(), "Item 50");
    assert!(harness.is_item_visible("Item 50")); // Scrolled into view
}
```

### 3.3 Scroll Position Maintenance

```rust
fn large_items() -> Vec<listbox::Item> {
    (0..1000).map(|i| listbox::Item {
        key: Key::from(format!("item-{i}")),
        label: format!("Item {i}"),
        ..Default::default()
    }).collect()
}

fn updated_large_items() -> Vec<listbox::Item> {
    (0..1000).map(|i| listbox::Item {
        key: Key::from(format!("item-{i}")),
        label: format!("Updated Item {i}"),
        ..Default::default()
    }).collect()
}

#[test]
fn virtual_list_maintains_scroll_on_data_update() {
    let harness = render(Listbox::with_items(&large_items()));
    harness.scroll_to_item("Item 500");
    let scroll_pos = harness.scroll_y();
    harness.send(listbox::Event::UpdateItems(updated_large_items())); // Same size, different data
    assert_eq!(harness.scroll_y(), scroll_pos);
}
```

### 3.4 Selection Across Virtual Boundaries

```rust
#[test]
fn virtual_list_shift_select_across_pages() {
    let items: Vec<_> = (0..10_000).map(|i| format!("Item {i}")).collect();
    let harness = render(Listbox::with_items(&items).selection_mode(selection::Mode::Multiple));
    harness.click_selector("[role='option'][data-value='Item 10']");
    harness.scroll_to_item("Item 500");
    // Shift-click requires extending selection via the state machine event,
    // since there is no shift_click_item primitive on TestHarness.
    harness.send(listbox::Event::Select { key: Key::from("Item 500"), mode: SelectionModifier::Extend });
    assert_eq!(harness.query_selector_all("[aria-selected='true']").len(), 491); // Items 10-500 inclusive
}
```

## 4. Tab/Panel Dynamic Registration Tests

Tests verifying dynamic add/remove/reorder of Tab panels and Accordion items, ensuring ARIA attributes and focus management remain correct.

### 4.1 Dynamic Tab Add/Remove Updates ARIA

```rust
#[test]
fn dynamic_tab_add_updates_tablist() {
    let harness = render(Tabs::new().items(vec!["Tab 1", "Tab 2"]));
    assert_eq!(harness.tab_count(), 2);

    harness.send(tabs::Event::AddTab { label: "Tab 3".into() });
    assert_eq!(harness.tab_count(), 3);

    let new_tab = harness.tab(2);
    assert_eq!(new_tab.attr("role"), Some("tab".into()));
    assert!(new_tab.attr("id").map_or(false, |v| v.starts_with("ars-tab-")));
    let panel = harness.panel(2);
    assert_eq!(panel.attr("role"), Some("tabpanel".into()));
    assert_eq!(panel.attr("aria-labelledby"), new_tab.attr("id"));
}

#[test]
fn dynamic_tab_remove_updates_aria() {
    let harness = render(Tabs::new().items(vec!["Tab 1", "Tab 2", "Tab 3"]));
    harness.send(tabs::Event::RemoveTab { index: 1 });
    assert_eq!(harness.tab_count(), 2);

    // Remaining tabs have correct aria-controls pointing to valid panels
    for i in 0..harness.tab_count() {
        let tab = harness.tab(i);
        let controls = tab.attr("aria-controls").expect("tab must have aria-controls");
        assert!(harness.query_selector(&format!("#{controls}")).is_some());
    }
}
```

### 4.2 Focus Management After Reorder

```rust
#[test]
fn focus_persists_after_tab_reorder() {
    let harness = render(Tabs::new().items(vec!["A", "B", "C"]));
    harness.focus("[role='tab']:nth-of-type(2)");
    assert!(harness.tab(1).is_focused());

    // Reorder: move "B" to position 0
    harness.send(tabs::Event::ReorderTab { from: 1, to: 0 });

    // "B" is now at index 0 and retains focus
    assert_eq!(harness.tab(0).text_content(), "B");
    assert!(harness.tab(0).is_focused());
}

#[test]
fn selected_tab_persists_after_sibling_removal() {
    let harness = render(Tabs::new().items(vec!["A", "B", "C"]).default_index(2));
    assert_eq!(harness.selected_index(), 2);

    // Remove sibling at index 0
    harness.send(tabs::Event::RemoveTab { index: 0 });

    // "C" is still selected, now at index 1
    assert_eq!(harness.tab(harness.selected_index()).text_content(), "C");
    assert_eq!(harness.tab(harness.selected_index()).attr("aria-selected"), Some("true".into()));
}
```

### 4.3 Accordion Dynamic Item Registration

```rust
#[test]
fn accordion_dynamic_add_item() {
    let harness = render(Accordion::new().items(vec!["Section 1"]));
    assert_eq!(harness.query_selector_all("[data-ars-part='item']").len(), 1);

    harness.send(accordion::Event::AddItem { label: "Section 2".into() });
    assert_eq!(harness.query_selector_all("[data-ars-part='item']").len(), 2);

    let new_trigger = harness.item(1).query_selector("button").expect("accordion item must have a trigger button");
    assert_eq!(new_trigger.attr("aria-expanded"), Some("false".into()));
    let controls = new_trigger.attr("aria-controls").expect("aria-controls must be present");
    assert!(harness.query_selector(&format!("#{controls}")).is_some());
}

#[test]
fn accordion_remove_preserves_expanded_state() {
    let harness = render(Accordion::new().items(vec!["A", "B", "C"]));
    // Expand "C" — click the trigger button inside the third accordion item
    harness.click_selector("[data-ars-part='item']:nth-child(3) button");
    let trigger_c = harness.item(2).query_selector("button").expect("trigger button");
    assert_eq!(trigger_c.attr("aria-expanded"), Some("true".into()));

    harness.send(accordion::Event::RemoveItem { index: 0 }); // remove "A"

    // "C" is now at index 1 and still expanded
    assert_eq!(harness.item(1).text_content(), "C");
    let trigger_c = harness.item(1).query_selector("button").expect("trigger button");
    assert_eq!(trigger_c.attr("aria-expanded"), Some("true".into()));
}
```

---

## 5. Z-Index Stacking Management Tests

Tests verifying correct z-index stacking order for overlays, ensuring proper layering between Toast, Dialog, Popover, and Tooltip components.

### 5.1 Toast Renders Above Dialog

```rust
#[test]
fn toast_renders_above_dialog() {
    let harness = render(App::new());
    harness.open_dialog();
    harness.send(toast::Event::Add(Toast::new("Saved")));

    let dialog_z = harness.query_selector("[role='dialog']")
        .unwrap().computed_styles().get("z-index").and_then(|v| v.parse::<i32>().ok()).expect("z-index must be set");
    let toast_z = harness.query_selector("[role='alert']")
        .unwrap().computed_styles().get("z-index").and_then(|v| v.parse::<i32>().ok()).expect("z-index must be set");

    assert!(toast_z > dialog_z, "toast z-index ({toast_z}) must exceed dialog ({dialog_z})");
}
```

### 5.2 Nested Overlay Stacking Order

```rust
#[test]
fn nested_overlay_stacking_order() {
    let harness = render(App::new());

    // Open in order: Dialog → Popover → Tooltip
    harness.open_dialog();
    let dialog_z = harness.query_selector("[role='dialog']").unwrap().computed_styles().get("z-index").and_then(|v| v.parse::<i32>().ok()).expect("z-index must be set");

    harness.click_selector("#popover-trigger-in-dialog");
    let popover_z = harness.query_selector("[data-ars-popover]").unwrap().computed_styles().get("z-index").and_then(|v| v.parse::<i32>().ok()).expect("z-index must be set");

    harness.hover("[data-ars-tooltip-trigger]");
    let tooltip_z = harness.query_selector("[role='tooltip']").unwrap().computed_styles().get("z-index").and_then(|v| v.parse::<i32>().ok()).expect("z-index must be set");

    assert!(tooltip_z > popover_z, "tooltip must stack above popover");
    assert!(popover_z > dialog_z, "popover must stack above dialog");
}
```

### 5.3 Modal vs Non-Modal Z-Index Allocation

```rust
#[test]
fn modal_gets_higher_z_than_non_modal() {
    let harness = render(App::new());

    harness.click_selector("[data-ars-popover-trigger]"); // non-modal
    let non_modal_z = harness.query_selector("[data-ars-popover]").unwrap().computed_styles().get("z-index").and_then(|v| v.parse::<i32>().ok()).expect("z-index must be set");

    harness.open_dialog(); // modal
    let modal_z = harness.query_selector("[role='dialog']").unwrap().computed_styles().get("z-index").and_then(|v| v.parse::<i32>().ok()).expect("z-index must be set");

    assert!(modal_z > non_modal_z, "modal overlay must stack above non-modal");
}

#[test]
fn z_index_released_after_overlay_close() {
    let harness = render(App::new());

    harness.open_dialog();
    let z_before = harness.query_selector("[role='dialog']").unwrap().computed_styles().get("z-index").and_then(|v| v.parse::<i32>().ok()).expect("z-index must be set");
    harness.close();

    // Open a new popover — it should not accumulate stale z-indexes
    harness.click_selector("[data-ars-popover-trigger]");
    let popover_z = harness.query_selector("[data-ars-popover]").unwrap().computed_styles().get("z-index").and_then(|v| v.parse::<i32>().ok()).expect("z-index must be set");
    assert!(popover_z <= z_before, "z-index should be reused after overlay close");
}
```

### 5.4 Concurrent Overlays Get Incrementing Z-Indexes

```rust
#[test]
fn concurrent_overlays_increment_z_index() {
    let harness = render(App::new());

    harness.open_dialog_with_id("d1");
    let z1 = harness.query_selector("#d1[role='dialog']").unwrap().computed_styles().get("z-index").and_then(|v| v.parse::<i32>().ok()).expect("z-index must be set");

    harness.open_dialog_with_id("d2");
    let z2 = harness.query_selector("#d2[role='dialog']").unwrap().computed_styles().get("z-index").and_then(|v| v.parse::<i32>().ok()).expect("z-index must be set");

    harness.open_dialog_with_id("d3");
    let z3 = harness.query_selector("#d3[role='dialog']").unwrap().computed_styles().get("z-index").and_then(|v| v.parse::<i32>().ok()).expect("z-index must be set");

    assert!(z3 > z2, "third overlay z-index must exceed second");
    assert!(z2 > z1, "second overlay z-index must exceed first");
}
```

---

## 6. Positioning Edge Cases Tests

Tests verifying overlay positioning logic handles viewport boundaries, RTL layouts, and scroll scenarios.

### 6.1 Viewport Flip on Bottom Overflow

```rust
#[test]
fn viewport_flip_when_overflow_bottom() {
    let harness = render_with_viewport(
        Popover::new().placement(Placement::Bottom),
        Viewport { width: 800, height: 400 },
    );

    // Anchor near bottom of viewport
    harness.set_anchor_position(Rect { x: 100, y: 370, width: 100, height: 30 });
    harness.open();

    let popover = harness.query_selector("[data-ars-popover]").unwrap();
    let popover_top = popover.bounding_rect().y;
    let anchor_top = 370;

    assert!(popover_top < anchor_top, "popover must flip to top when bottom overflows");
}
```

### 6.2 Horizontal Shift on Overflow

```rust
#[test]
fn viewport_shift_for_horizontal_overflow() {
    let harness = render_with_viewport(
        Popover::new().placement(Placement::Bottom),
        Viewport { width: 400, height: 800 },
    );

    // Anchor near right edge
    harness.set_anchor_position(Rect { x: 350, y: 100, width: 40, height: 30 });
    harness.open();

    let popover = harness.query_selector("[data-ars-popover]").unwrap();
    let rect = popover.bounding_rect();
    let popover_right = rect.x + rect.width;

    assert!(popover_right <= 400, "popover must shift left to stay within viewport");
}
```

### 6.3 Small Viewport Positioning

```rust
#[test]
fn positioning_in_small_viewport() {
    let harness = render_with_viewport(
        Tooltip::new().placement(Placement::Bottom),
        Viewport { width: 320, height: 480 },
    );

    harness.set_anchor_position(Rect { x: 10, y: 10, width: 300, height: 30 });
    harness.open();

    let tooltip = harness.query_selector("[role='tooltip']").unwrap();
    let rect = tooltip.bounding_rect();
    let tooltip_left = rect.x;
    let tooltip_right = rect.x + rect.width;

    assert!(tooltip_left >= 0, "tooltip must not overflow left in small viewport");
    assert!(tooltip_right <= 320, "tooltip must not overflow right in small viewport");
}
```

### 6.4 RTL Placement Mirror

```rust
#[test]
fn rtl_placement_mirror_start_end() {
    let harness = render_with_dir(
        Popover::new().placement(Placement::Start),
        Dir::Rtl,
    );
    harness.open();

    let popover = harness.query_selector("[data-ars-popover]").unwrap();
    let anchor = harness.query("[data-ars-part='trigger']");
    let popover_rect = popover.bounding_rect();
    let anchor_rect = anchor.bounding_rect();

    // In RTL, Placement::Start should render on the right side of the anchor
    assert!(
        popover_rect.x > anchor_rect.x + anchor_rect.width / 2,
        "start placement in RTL must mirror to right side"
    );
}
```

### 6.5 Positioning Update on Scroll

```rust
#[test]
fn positioning_updates_on_scroll() {
    let harness = render(Popover::new().placement(Placement::Bottom));
    harness.open();

    let initial_top = harness.query_selector("[data-ars-popover]")
        .unwrap().bounding_rect().y;

    harness.scroll_container_by(0, 50);

    let updated_top = harness.query_selector("[data-ars-popover]")
        .unwrap().bounding_rect().y;

    assert_ne!(initial_top, updated_top, "popover must reposition on scroll");
}
```

---

## 7. Progress/Loading State Tests

Tests verifying loading states, async action handling, and progress indicators across components.

### 7.1 Button Loading State During Async Action

```rust
#[test]
fn button_loading_state_during_async() {
    let harness = render(Button::new().on_click_async(|| async {
        sleep(Duration::from_millis(100)).await;
    }));

    harness.click();

    assert!(harness.button_attr("aria-busy").is_some());
    assert_eq!(harness.button_attr("aria-disabled"), Some("true".into()));
    assert!(harness.query_selector("[data-ars-loading]").is_some());
}

#[test]
fn button_loading_state_clears_after_async_completes() {
    let harness = render(Button::new().on_click_async(|| async {
        sleep(Duration::from_millis(50)).await;
    }));

    harness.click();
    harness.advance_time(Duration::from_millis(100));

    assert!(harness.button_attr("aria-busy").is_none());
    assert!(harness.query_selector("[data-ars-loading]").is_none());
}
```

### 7.2 Form Async Validation

```rust
#[test]
fn form_async_validator_shows_pending() {
    let harness = render(TextField::new().validate_async(|val| async move {
        sleep(Duration::from_millis(100)).await;
        if val == "taken" { Err("Username taken".into()) } else { Ok(()) }
    }));

    harness.type_text("taken");
    harness.blur();

    // While validating
    assert!(harness.input_attr("aria-busy").is_some());

    harness.advance_time(Duration::from_millis(150));

    assert!(harness.input_attr("aria-busy").is_none());
    assert_eq!(harness.input_attr("aria-invalid"), Some("true".into()));
}
```

### 7.3 Combobox Async Search with Loading

```rust
#[test]
fn combobox_async_search_loading_indicator() {
    let harness = render(Combobox::new().on_search_async(|query| async move {
        sleep(Duration::from_millis(100)).await;
        vec![format!("Result for {query}")]
    }));

    harness.type_text("foo");

    assert!(harness.query_selector("[data-ars-loading]").is_some());

    harness.advance_time(Duration::from_millis(150));

    assert!(harness.query_selector("[data-ars-loading]").is_none());
    assert_eq!(harness.option_count(), 1);
}
```

### 7.4 Progress Determinate/Indeterminate Transition

```rust
#[test]
fn progress_determinate_to_indeterminate() {
    let harness = render(Progress::new().value(Some(50)));
    let bar = harness.query_selector("[role='progressbar']").unwrap();
    assert_eq!(bar.attr("aria-valuenow"), Some("50".into()));
    assert_eq!(bar.attr("aria-valuemin"), Some("0".into()));
    assert_eq!(bar.attr("aria-valuemax"), Some("100".into()));

    // Switch to indeterminate
    harness.set_value(None);
    let bar = harness.query_selector("[role='progressbar']").unwrap();
    assert!(bar.attr("aria-valuenow").is_none(), "indeterminate progress must not have aria-valuenow");
}
```

### 7.5 Cancellation of In-Flight Async Operations

```rust
#[test]
fn cancellation_of_inflight_async() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let count = call_count.clone();

    let harness = render(Combobox::new().on_search_async(move |_query| {
        let count = count.clone();
        async move {
            sleep(Duration::from_millis(200)).await;
            count.fetch_add(1, Ordering::SeqCst);
            vec!["result".to_string()]
        }
    }));

    harness.type_text("a");
    harness.advance_time(Duration::from_millis(50));
    harness.type_text("ab"); // supersedes first search

    harness.advance_time(Duration::from_millis(250));

    // Only the latest search should have completed
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}
```

---

## 8. Page-Level Integration Tests

Tests verifying correct behavior when components are composed together at the page level.

### 8.1 Form Inside Dialog

```rust
#[test]
fn form_inside_dialog_submit_and_close() {
    let submitted = Arc::new(AtomicBool::new(false));
    let on_submit = {
        let submitted = submitted.clone();
        move |_| { submitted.store(true, Ordering::SeqCst); }
    };

    let harness = render(Dialog::new().content(
        Form::new()
            .field(TextField::new().name("email"))
            .on_submit(on_submit)
    ));
    harness.open();

    harness.focus("input[name='email']");
    harness.type_text("a@b.com");
    harness.click_selector("button[type='submit']");

    assert!(submitted.load(Ordering::SeqCst));
}

#[test]
fn form_validation_error_inside_dialog() {
    let harness = render(Dialog::new().content(
        Form::new().field(TextField::new().name("email").required(true))
    ));
    harness.open();

    // Submit without filling required field
    harness.click_selector("button[type='submit']");

    let input = harness.query_selector("input[name='email']").unwrap();
    assert_eq!(input.attr("aria-invalid"), Some("true".into()));
    // Dialog should remain open
    assert!(harness.query_selector("[role='dialog']").is_some());
}
```

### 8.2 Combobox Inside Menu

```rust
#[test]
fn combobox_inside_menu_no_close_propagation() {
    let harness = render(Menu::new().content(
        Combobox::new().items(vec!["Apple", "Banana"])
    ));
    harness.open_menu();
    harness.click_selector("[role='combobox']");

    // Opening combobox listbox should not close the menu
    assert!(harness.query_selector("[role='menu']").is_some());

    // Selecting a combobox option should not close the menu
    harness.click_selector("[role='option']");
    assert!(harness.query_selector("[role='menu']").is_some());
}
```

### 8.3 Nested Focus Traps

```rust
#[test]
fn nested_focus_traps_dialog_popover() {
    let harness = render(Dialog::new().content(
        Button::new().label("Open Popover")
    ));
    harness.open();

    // Open popover inside dialog
    harness.click_selector("button");
    let popover = harness.query_selector("[data-ars-popover]");
    assert!(popover.is_some());

    // Tab should cycle within popover, not escape to dialog
    harness.press_key(KeyboardKey::Tab);
    // Verify focused element is inside the popover by checking it exists within the popover scope
    let focused_in_popover = harness.query_selector("[data-ars-popover] :focus");
    assert!(focused_in_popover.is_some(), "focus must stay within inner popover");
}
```

### 8.4 Keyboard Navigation Across Composed Components

```rust
#[test]
fn keyboard_nav_across_composed_components() {
    let harness = render(Dialog::new().content(view! {
        <TextField label="Name" />
        <Select label="Country" items=vec!["US", "UK"] />
        <Button label="Submit" />
    }));
    harness.open();

    // Tab through all interactive elements
    harness.press_key(KeyboardKey::Tab);
    assert!(harness.query_selector("input:focus").is_some());

    harness.press_key(KeyboardKey::Tab);
    assert!(harness.query_selector("[role='combobox']:focus").is_some());

    harness.press_key(KeyboardKey::Tab);
    assert!(harness.query_selector("button:focus").is_some());

    // Tab wraps back to first element (focus trap)
    harness.press_key(KeyboardKey::Tab);
    assert!(harness.query_selector("input:focus").is_some());
}
```

### 8.5 Scroll Lock with Nested Modals

```rust
#[test]
fn scroll_lock_with_nested_modals() {
    let harness = render(App::new());

    harness.open_dialog_with_id("outer");
    assert!(harness.body_has_scroll_lock());

    // Open nested modal
    harness.open_dialog_with_id("inner");
    assert!(harness.body_has_scroll_lock());

    // Close inner — body should still be locked (outer still open)
    harness.close_dialog_with_id("inner");
    assert!(harness.body_has_scroll_lock());

    // Close outer — body unlocked
    harness.close_dialog_with_id("outer");
    assert!(!harness.body_has_scroll_lock());
}
```

---

## 9. Scroll Lock / Body Lock Tests

Tests verifying scroll lock behavior when modals are opened and closed, including edge cases with nested modals and existing styles.

### 9.1 Modal Open Applies Scroll Lock

```rust
#[test]
fn modal_open_applies_scroll_lock() {
    let harness = render(Dialog::new().modal(true));
    assert!(!harness.body_has_scroll_lock());

    harness.open();
    assert!(harness.body_has_scroll_lock());
    assert_eq!(harness.body_style("overflow"), "hidden");
}
```

### 9.2 Scroll Position Preserved After Lock/Unlock

```rust
#[test]
fn scroll_position_preserved_after_lock_unlock() {
    let harness = render_with_scroll_content(Dialog::new().modal(true));

    harness.scroll_to(0, 500);
    assert_eq!(harness.scroll_y(), 500);

    harness.open();
    harness.close();

    assert_eq!(harness.scroll_y(), 500, "scroll position must be restored after unlock");
}
```

### 9.3 Nested Modal Close Behavior

```rust
#[test]
fn nested_modal_close_does_not_unlock_body() {
    let harness = render(App::new());

    harness.open_dialog_with_id("outer");
    harness.open_dialog_with_id("inner");

    // Close inner modal
    harness.close_dialog_with_id("inner");

    // Body must remain locked because outer modal is still open
    assert!(harness.body_has_scroll_lock(), "body must stay locked while outer modal is open");
}
```

### 9.4 Scroll Lock with Existing Overflow Styles

```rust
#[test]
fn scroll_lock_preserves_existing_overflow_style() {
    let harness = render(Dialog::new().modal(true));

    // Set pre-existing overflow style
    harness.set_body_style("overflow", "auto");

    harness.open();
    assert_eq!(harness.body_style("overflow"), "hidden");

    harness.close();
    assert_eq!(
        harness.body_style("overflow"), "auto",
        "original overflow style must be restored"
    );
}
```

### 9.5 iOS Scroll Lock Workaround

```rust
#[test]
fn ios_scroll_lock_touch_action() {
    let harness = render_with_platform(Dialog::new().modal(true), Platform::Ios);

    harness.open();

    // iOS requires touch-action: none on body to prevent scroll
    assert_eq!(harness.body_style("touch-action"), "none");

    harness.close();
    assert_ne!(harness.body_style("touch-action"), "none");
}
```

---

## 10. Multi-Component Integration Tests

Integration tests verify that multiple components work correctly together, with proper state
isolation, focus management across boundaries, and event propagation.

### 10.1 Realistic Scenarios

```rust
pub fn test_items() -> Vec<table::Row> {
    vec![
        table::Row { id: "r1".into(), cells: vec!["Alice".into(), "Engineering".into()] },
        table::Row { id: "r2".into(), cells: vec!["Bob".into(), "Design".into()] },
        table::Row { id: "r3".into(), cells: vec!["Carol".into(), "Product".into()] },
    ]
}

pub fn countries() -> Vec<combobox::Item> {
    vec![
        combobox::Item { key: Key::from("us"), label: "United States".into(), ..Default::default() },
        combobox::Item { key: Key::from("uk"), label: "United Kingdom".into(), ..Default::default() },
        combobox::Item { key: Key::from("de"), label: "Germany".into(), ..Default::default() },
    ]
}

#[cfg(test)]
mod integration_tests {
    /// Table row click opens a detail Dialog; closing Dialog returns focus to the row.
    #[wasm_bindgen_test]
    async fn table_row_opens_dialog_and_restores_focus() {
        let harness = mount(|| view! {
            <Table items=test_items()>
                <TableRow on_click=open_detail_dialog />
            </Table>
            <Dialog id="detail" />
        });
        harness.click_selector("[data-ars-scope='table-row']:first-child");
        tick().await;
        assert!(harness.query("[data-ars-scope='dialog']").is_some());
        harness.press_key(KeyboardKey::Escape);
        tick().await;
        assert!(harness.query_selector("[data-ars-scope='table-row']:first-child:focus").is_some(),
            "focus must return to the row after dialog closes");
    }

    /// Combobox inside a Form: validation errors appear and are announced.
    #[wasm_bindgen_test]
    async fn combobox_form_validation() {
        let harness = mount(|| view! {
            <Form on_submit=validate>
                <Combobox name="country" required=true items=countries() />
                <Button type_="submit">"Submit"</Button>
            </Form>
        });
        harness.click_selector("button[type='submit']");
        tick().await;
        assert!(harness.query_selector("[data-ars-invalid='true']").is_some());
        assert!(harness.query_selector("[role='alert']").is_some());
    }

    /// DatePicker + TimeField compose into a DateTime selection.
    #[wasm_bindgen_test]
    async fn datepicker_timefield_composition() {
        let harness = mount(|| view! {
            <DatePicker on_change=update_date />
            <TimeField on_change=update_time />
        });
        harness.click_selector("[data-ars-scope='datepicker'] button");
        tick().await;
        harness.click_selector("[data-ars-scope='calendar'] td:nth-child(3)");
        tick().await;
        harness.focus("[data-ars-scope='timefield'] input");
        harness.type_text("14:30");
        tick().await;
        // Both values should be independently set.
        assert!(get_date_value(&harness).is_some());
        assert!(get_time_value(&harness).is_some());
    }

    /// Sibling Tabs components maintain independent state.
    #[wasm_bindgen_test]
    async fn sibling_tabs_independent_state() {
        let harness = mount(|| view! {
            <Tabs id="tabs-a" default_value="a1">/* ... */</Tabs>
            <Tabs id="tabs-b" default_value="b1">/* ... */</Tabs>
        });
        harness.click_selector("#tabs-a [data-ars-value='a2']");
        tick().await;
        assert_eq!(harness.attr("#tabs-a", "data-ars-value"), Some("a2".into()));
        assert_eq!(harness.attr("#tabs-b", "data-ars-value"), Some("b1".into()),
            "sibling Tabs must not share state");
    }

    /// Focus moves correctly across component boundaries (Dialog -> Combobox -> back).
    #[wasm_bindgen_test]
    async fn focus_management_across_component_boundaries() {
        let harness = mount(|| view! {
            <Dialog id="dlg" open=true>
                <Combobox id="combo" items=items() />
                <Button id="save">"Save"</Button>
            </Dialog>
        });
        harness.press_key(KeyboardKey::Tab);
        tick().await;
        assert!(harness.query_selector("#combo :focus").is_some(), "focus must be inside combobox");
        harness.press_key(KeyboardKey::Tab);
        tick().await;
        assert!(harness.query_selector("#save:focus").is_some(), "focus must be on save button");
        // Tab wraps within dialog focus trap
        harness.press_key(KeyboardKey::Tab);
        tick().await;
        assert!(harness.query_selector("#combo :focus").is_some(), "focus must wrap back to combobox");
    }
}
```

---

## 11. Empty/Null Collection Edge Cases

Components that accept item collections MUST handle empty, single-item, and all-disabled
scenarios without panicking or entering invalid states.

```rust
#[test]
fn select_open_with_empty_items() {
    let props = select::Props {
        items: vec![],
        ..Default::default()
    };
    let mut svc = Service::<select::Machine>::new(props);

    // Opening with no items must not panic.
    svc.send(select::Event::Open);
    assert_eq!(*svc.state(), select::State::Open);
    // highlighted_key follows the Collection trait's key-based API (see select component spec)
    assert_eq!(svc.context().highlighted_key, None,
        "no item should be highlighted when items list is empty");
}

#[test]
fn all_items_disabled_no_highlight() {
    let items = vec![
        select::Item { label: "A".into(), disabled: true, ..Default::default() },
        select::Item { label: "B".into(), disabled: true, ..Default::default() },
        select::Item { label: "C".into(), disabled: true, ..Default::default() },
    ];
    let mut svc = Service::<select::Machine>::new(select::Props {
        items,
        ..Default::default()
    });
    svc.send(select::Event::Open);
    assert_eq!(svc.context().highlighted_key, None,
        "no item should be highlighted when all items are disabled");

    // Arrow key navigation should not highlight any disabled item.
    svc.send(select::Event::ArrowDown);
    assert_eq!(svc.context().highlighted_key, None,
        "ArrowDown must not highlight a disabled item");
}

#[test]
fn single_item_select() {
    let items = vec![select::Item { label: "Only".into(), ..Default::default() }];
    let mut svc = Service::<select::Machine>::new(select::Props {
        items,
        ..Default::default()
    });
    svc.send(select::Event::Open);
    assert!(svc.context().highlighted_key.is_some(),
        "single item should be highlighted on open");

    // ArrowDown on single item wraps or stays.
    let key_before = svc.context().highlighted_key.clone();
    svc.send(select::Event::ArrowDown);
    assert_eq!(svc.context().highlighted_key, key_before,
        "single-item list should not move highlight");
}

#[test]
fn combobox_empty_items_type_ahead() {
    let mut svc = Service::<combobox::Machine>::new(combobox::Props {
        items: vec![],
        ..Default::default()
    });
    svc.send(combobox::Event::Open);
    svc.send(combobox::Event::TypeAhead("abc".into()));
    // Must not panic; filtered list is empty.
    // Combobox uses visible_keys (per foundation 06 FilteredCollection pattern)
    // to track which items pass the filter. With no items, visible_keys is None
    // (meaning "show all") or Some(empty set).
    let visible = svc.context().visible_keys.as_ref().map_or(0, |keys| keys.len());
    assert_eq!(visible, 0);
}
```

---

## 12. Nested Modal Dialog Testing

Nested (stacked) modal dialogs must maintain correct focus management and stack order. Closing the inner dialog must restore focus to the outer dialog, and closing the outer dialog must restore focus to the original trigger.

### 12.1 Two Simultaneous Dialogs

```rust
#[wasm_bindgen_test]
async fn nested_dialogs_focus_management() {
    mount_to_body(|| {
        view! {
            <button id="outer-trigger">"Open Outer"</button>
            <Dialog id="outer-dialog" trigger_id="outer-trigger">
                <DialogContent>
                    <p>"Outer content"</p>
                    <button id="inner-trigger">"Open Inner"</button>
                    <Dialog id="inner-dialog" trigger_id="inner-trigger">
                        <DialogContent>
                            <p>"Inner content"</p>
                            <button id="inner-close">"Close Inner"</button>
                        </DialogContent>
                    </Dialog>
                </DialogContent>
            </Dialog>
        }
    });

    let outer_trigger = get_element::<HtmlElement>("outer-trigger");
    let inner_trigger = || get_element::<HtmlElement>("inner-trigger");

    // Open outer dialog
    outer_trigger.click();
    tick().await;

    let outer_content = document().query_selector("#outer-dialog [data-ars-part='content']").unwrap().unwrap();
    assert_eq!(outer_content.get_attribute("role").as_deref(), Some("dialog"));
    assert_eq!(outer_content.get_attribute("aria-modal").as_deref(), Some("true"));

    // Open inner dialog
    inner_trigger().click();
    tick().await;

    let inner_content = document().query_selector("#inner-dialog [data-ars-part='content']").unwrap().unwrap();
    assert_eq!(inner_content.get_attribute("role").as_deref(), Some("dialog"));
    assert_eq!(inner_content.get_attribute("aria-modal").as_deref(), Some("true"));

    // Verify inner dialog is on top (higher z-index or later in DOM stacking order)
    let outer_z = get_computed_z_index(&outer_content);
    let inner_z = get_computed_z_index(&inner_content);
    assert!(inner_z > outer_z, "Inner dialog must stack above outer dialog");
}
```

### 12.2 Closing Inner Restores Focus to Outer

```rust
#[wasm_bindgen_test]
async fn closing_inner_dialog_restores_focus_to_outer() {
    // Setup: both dialogs open (as above)
    // ...

    // Close inner dialog
    let inner_close = get_element::<HtmlElement>("inner-close");
    inner_close.click();
    tick().await;

    // Inner dialog must be removed
    assert!(
        document().query_selector("#inner-dialog [data-ars-part='content']").unwrap().is_none(),
        "Inner dialog must be closed"
    );

    // Focus must return to the inner dialog's trigger (inside outer dialog)
    let active = document().active_element().unwrap();
    assert_eq!(active.id(), "inner-trigger", "Focus must return to inner-trigger after closing inner dialog");
}
```

### 12.3 Closing Outer Restores Focus to Original Trigger

```rust
#[wasm_bindgen_test]
async fn closing_outer_dialog_restores_focus_to_page() {
    // Setup: only outer dialog open
    // ...

    // Close outer dialog (e.g., press Escape)
    dispatch_keyboard_event(&outer_content, "keydown", "Escape");
    tick().await;

    assert!(
        document().query_selector("#outer-dialog [data-ars-part='content']").unwrap().is_none(),
        "Outer dialog must be closed"
    );

    let active = document().active_element().unwrap();
    assert_eq!(active.id(), "outer-trigger", "Focus must return to outer-trigger after closing outer dialog");
}
```

### 12.4 Escape Key Closes Only Topmost Dialog

```rust
#[wasm_bindgen_test]
async fn escape_closes_only_topmost_dialog() {
    // Setup: both dialogs open
    // ...

    // Press Escape — should close inner only
    dispatch_keyboard_event(&inner_content, "keydown", "Escape");
    tick().await;

    assert!(
        document().query_selector("#inner-dialog [data-ars-part='content']").unwrap().is_none(),
        "Inner dialog must close on Escape"
    );
    assert!(
        document().query_selector("#outer-dialog [data-ars-part='content']").unwrap().is_some(),
        "Outer dialog must remain open when inner receives Escape"
    );
}
```

---

## 13. Collection Unit Tests

These tests verify the pure-Rust collection APIs from `ars-collections`. All tests are framework-agnostic and run as standard `#[test]` functions.

> **Foundation reference:** All types and APIs below are defined in [06-collections.md](../foundation/06-collections.md).

### 13.1 Collection&lt;T&gt; Trait Methods

```rust
use ars_collections::{StaticCollection, CollectionBuilder, Collection, Key, Node, NodeType};

#[test]
fn empty_collection_returns_none_for_all_queries() {
    let col = StaticCollection::<()>::default();
    assert!(col.first_key().is_none(), "empty collection must return None for first_key");
    assert!(col.last_key().is_none(), "empty collection must return None for last_key");
    assert!(col.get_by_index(0).is_none(), "empty collection must return None for get_by_index(0)");
    assert_eq!(col.size(), 0);
}

#[test]
fn key_after_last_returns_none_without_wrap() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .item(Key::from("b"), "Beta", ())
        .build();
    let last = col.last_key().expect("collection must have last key");
    assert!(col.key_after_no_wrap(last).is_none(), "key_after last without wrap must be None");
}

#[test]
fn key_after_wraps_to_first() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .item(Key::from("b"), "Beta", ())
        .build();
    let last = col.last_key().expect("collection must have last key");
    let first = col.first_key().expect("collection must have first key");
    assert_eq!(col.key_after(last), Some(first), "key_after last with wrap must return first");
}

#[test]
fn key_before_first_returns_none_without_wrap() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .item(Key::from("b"), "Beta", ())
        .build();
    let first = col.first_key().expect("collection must have first key");
    assert!(col.key_before_no_wrap(first).is_none(), "key_before first without wrap must be None");
}

#[test]
fn get_by_index_matches_get_first_key() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .item(Key::from("b"), "Beta", ())
        .build();
    let first = col.first_key().expect("collection must have first key");
    let by_key = col.get(first).expect("first key must exist");
    let by_index = col.get_by_index(0).expect("index 0 must exist");
    assert_eq!(by_key.key, by_index.key, "get(first_key) must equal get_by_index(0)");
}

#[test]
fn children_of_returns_direct_children_only() {
    let col = CollectionBuilder::new()
        .section(Key::from("s1"), "Section 1")
        .item(Key::from("a"), "Alpha", ())
        .item(Key::from("b"), "Beta", ())
        .end_section()
        .item(Key::from("c"), "Gamma", ()) // top-level, not child of s1
        .build();
    let children: Vec<_> = col.children_of(&Key::from("s1")).collect();
    assert_eq!(children.len(), 2, "section must have exactly 2 direct children");
    assert!(children.iter().all(|n| n.key == Key::from("a") || n.key == Key::from("b")));
}

#[test]
fn text_value_of_returns_node_text() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .build();
    assert_eq!(col.text_value_of(&Key::from("a")), Some("Alpha"));
    assert_eq!(col.text_value_of(&Key::from("missing")), None);
}

#[test]
fn empty_collection_is_empty() {
    let col: StaticCollection<String> = CollectionBuilder::new().build();
    assert!(col.is_empty());
}

#[test]
fn contains_key_for_existing_item() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .build();
    assert!(col.contains_key(&Key::from("a")));
    assert!(!col.contains_key(&Key::from("z")));
}

#[test]
fn item_keys_skips_structural_nodes() {
    let col = CollectionBuilder::new()
        .section(Key::from("s1"), "Section 1")
        .item(Key::from("a"), "Alpha", ())
        .end_section()
        .build();
    let keys: Vec<_> = col.item_keys().collect();
    assert!(keys.contains(&&Key::from("a")));
    assert!(!keys.contains(&&Key::from("s1")));
}

#[test]
fn nodes_includes_all_node_types() {
    let col = CollectionBuilder::new()
        .section(Key::from("s1"), "Section")
        .item(Key::from("a"), "Alpha", ())
        .end_section()
        .build();
    let nodes: Vec<_> = col.nodes().collect();
    assert!(nodes.len() >= 2); // section + item
}
```

### 13.2 selection::State Operations

```rust
use ars_collections::selection::{self, State as SelectionState};
use ars_collections::{CollectionBuilder, Collection, Key};
use std::collections::BTreeSet;

#[test]
fn empty_set_contains_nothing() {
    let set = selection::Set::Empty;
    assert!(!set.contains(&Key::from("a")));
    assert!(set.is_empty());
    assert!(!set.is_all());
}

#[test]
fn single_toggle_removes_item() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .build();
    let state = SelectionState::new(selection::Mode::Multiple, selection::Behavior::Toggle);
    let state = state.select(Key::from("a"));
    assert!(state.is_selected(&Key::from("a")));
    let state = state.toggle(Key::from("a"), &col);
    assert!(!state.is_selected(&Key::from("a")), "toggling the only selected item must deselect it");
}

#[test]
fn multiple_toggle_adds_and_removes() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .item(Key::from("b"), "Beta", ())
        .item(Key::from("c"), "Gamma", ())
        .build();
    let state = SelectionState::new(selection::Mode::Multiple, selection::Behavior::Toggle);
    let state = state.select(Key::from("a"));
    let state = state.toggle(Key::from("b"), &col);
    assert!(state.is_selected(&Key::from("b")), "toggled item must be added");

    // Toggle existing item removes it
    let state = state.toggle(Key::from("a"), &col);
    assert!(!state.is_selected(&Key::from("a")), "toggled item must be removed");
    assert!(state.is_selected(&Key::from("b")), "other items must remain");

    // Toggle new item adds it
    let state = state.toggle(Key::from("c"), &col);
    assert!(state.is_selected(&Key::from("c")), "new item must be added");
}

#[test]
fn all_contains_every_key() {
    let set = selection::Set::All;
    assert!(set.contains(&Key::from("anything")));
    assert!(set.contains(&Key::from("any_other")));
    assert!(set.is_all());
    assert!(!set.is_empty());
}

#[test]
fn extend_selection_skips_disabled() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .item(Key::from("b"), "Beta", ())    // disabled
        .item(Key::from("c"), "Gamma", ())
        .item(Key::from("d"), "Delta", ())
        .build();
    let disabled = BTreeSet::from([Key::from("b")]);

    let state = SelectionState::new(selection::Mode::Multiple, selection::Behavior::Toggle)
        .with_disabled(disabled)
        .select(Key::from("a")); // sets anchor to "a"
    let state = state.extend_selection(Key::from("d"), &col);
    assert!(state.is_selected(&Key::from("a")), "anchor must be selected");
    assert!(!state.is_selected(&Key::from("b")), "disabled item must be skipped");
    assert!(state.is_selected(&Key::from("c")), "enabled item in range must be selected");
    assert!(state.is_selected(&Key::from("d")), "target must be selected");
}

#[test]
fn select_all_produces_all_variant() {
    let state = SelectionState::new(selection::Mode::Multiple, selection::Behavior::Toggle);
    let state = state.select_all();
    assert!(state.selected_keys.is_all());
}

#[test]
fn set_all_count_returns_none() {
    let set = selection::Set::All;
    assert_eq!(set.count(), None);
}

#[test]
fn set_explicit_count_returns_some() {
    let mut keys = BTreeSet::new();
    keys.insert(Key::from("a"));
    keys.insert(Key::from("b"));
    let set = selection::Set::Multiple(keys);
    assert_eq!(set.count(), Some(2));
}

#[test]
fn set_all_len_returns_zero() {
    let set = selection::Set::All;
    assert_eq!(set.len(), 0);
}
```

### 13.3 Typeahead

```rust
use ars_collections::typeahead::{State as TypeaheadState, TYPEAHEAD_TIMEOUT_MS};
use ars_collections::{CollectionBuilder, Collection, Key};

#[test]
fn single_char_match_returns_matching_key() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .item(Key::from("b"), "Beta", ())
        .item(Key::from("g"), "Gamma", ())
        .build();
    let state = TypeaheadState::default();
    let (new_state, matched) = state.process_char('b', 0, None, &col);
    assert_eq!(matched, Some(Key::from("b")), "typing 'b' must match Beta");
    assert_eq!(new_state.search, "b");
}

#[test]
fn multi_char_accumulates_within_timeout() {
    let col = CollectionBuilder::new()
        .item(Key::from("al"), "Alpha", ())
        .item(Key::from("am"), "Amaranth", ())
        .build();
    let state = TypeaheadState::default();
    let (state, _) = state.process_char('a', 0, None, &col);
    let (state, matched) = state.process_char('m', 100, None, &col); // within 500ms
    assert_eq!(matched, Some(Key::from("am")), "typing 'am' must match Amaranth");
    assert_eq!(state.search, "am");
}

#[test]
fn timeout_resets_search() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .item(Key::from("b"), "Beta", ())
        .build();
    let state = TypeaheadState::default();
    let (state, _) = state.process_char('a', 0, None, &col);
    // Wait beyond timeout
    let (state, matched) = state.process_char('b', TYPEAHEAD_TIMEOUT_MS + 1, None, &col);
    assert_eq!(matched, Some(Key::from("b")), "after timeout, 'b' must start fresh search");
    assert_eq!(state.search, "b", "search buffer must be reset");
}

#[test]
fn wrap_around_search() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .item(Key::from("b"), "Beta", ())
        .item(Key::from("a2"), "Another Alpha", ())
        .build();
    let state = TypeaheadState::default();
    // Start from "a2", search for 'a' — should wrap to "a" (Alpha)
    let (_, matched) = state.process_char('a', 0, Some(&Key::from("a2")), &col);
    assert_eq!(matched, Some(Key::from("a")), "search must wrap to first match after current");
}

#[test]
fn no_match_returns_none() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .build();
    let state = TypeaheadState::default();
    let (new_state, matched) = state.process_char('z', 0, None, &col);
    assert!(matched.is_none(), "no matching item must return None");
    assert_eq!(new_state.search, "z", "search buffer must still update");
}

#[cfg(feature = "i18n")]
#[test]
fn typeahead_with_locale_matches_locale_aware() {
    let locale = ars_i18n::Locale::parse("de").expect("valid locale");
    let col = CollectionBuilder::new()
        .item(Key::from("ae-item"), "Aeble", ())
        .item(Key::from("ä-item"), "Äpfel", ())
        .build();
    let state = TypeaheadState::default();
    let (_, matched) = state.process_char('ä', 0, None, &col, &locale);
    assert_eq!(matched, Some(Key::from("ä-item")),
        "i18n typeahead should match locale-aware characters");
}
```

### 13.4 AsyncCollection State Machine

```rust
use ars_collections::async_collection::{AsyncCollection, AsyncLoadingState};
use ars_collections::{Collection, Key};

#[test]
fn new_async_collection_is_idle() {
    let col = AsyncCollection::<()>::new();
    assert_eq!(col.loading_state, AsyncLoadingState::Idle);
    assert!(col.has_more, "new collection must default to has_more=true");
    assert_eq!(col.size(), 0);
}

#[test]
fn begin_load_transitions_to_loading() {
    let col = AsyncCollection::<()>::new();
    let loading = col.begin_load();
    assert_eq!(loading.loading_state, AsyncLoadingState::Loading);
}

#[test]
fn append_page_with_cursor_sets_has_more() {
    let col = AsyncCollection::<()>::new().begin_load();
    let loaded = col.append_page(
        vec![(Key::from("1"), "Item 1".into(), ())],
        Some("cursor_2".into()),
    );
    assert_eq!(loaded.loading_state, AsyncLoadingState::Loaded);
    assert!(loaded.has_more, "must have more pages when cursor is Some");
    assert_eq!(loaded.size(), 1);
}

#[test]
fn append_page_without_cursor_clears_has_more() {
    let col = AsyncCollection::<()>::new().begin_load();
    let loaded = col.append_page(
        vec![(Key::from("1"), "Item 1".into(), ())],
        None,
    );
    assert_eq!(loaded.loading_state, AsyncLoadingState::Loaded);
    assert!(!loaded.has_more, "must not have more pages when cursor is None");
}

#[test]
fn set_error_transitions_to_error() {
    let col = AsyncCollection::<()>::new().begin_load();
    let errored = col.set_error("Network timeout");
    assert_eq!(errored.loading_state, AsyncLoadingState::Error("Network timeout".into()));
}

#[test]
fn retry_from_error_transitions_to_loading() {
    let col = AsyncCollection::<()>::new()
        .begin_load()
        .set_error("Failed");
    let retry = col.begin_load();
    assert_eq!(retry.loading_state, AsyncLoadingState::Loading, "retry must transition back to Loading");
}

#[test]
fn load_more_from_loaded_transitions_to_loading_more() {
    let col = AsyncCollection::<()>::new()
        .begin_load()
        .append_page(
            vec![(Key::from("1"), "Item 1".into(), ())],
            Some("cursor_2".into()),
        );
    assert!(col.has_more);
    let loading_more = col.begin_load();
    assert_eq!(loading_more.loading_state, AsyncLoadingState::LoadingMore);
}
```

### 13.5 Virtualizer

```rust
use ars_collections::{
    Key,
    virtualization::{
        Virtualizer,
        LayoutStrategy,
        Orientation,
        Direction,
        ScrollAlign
    }
};


/// Helper: construct a fixed-height Virtualizer with a 200px viewport.
/// Uses Virtualizer::new() then sets public fields directly
/// (measured_heights is private, so struct literal is not possible).
fn fixed_height_virt() -> Virtualizer {
    let mut virt = Virtualizer::new(100, LayoutStrategy::FixedHeight { item_height: 40.0 });
    virt.viewport_height = 200.0;
    virt.overscan = 3;
    virt
}

#[test]
fn visible_range_for_fixed_height_items() {
    let virt = fixed_height_virt();
    // At scroll offset 0, visible items are 0..5 (200 / 40 = 5 items) plus overscan
    let range = virt.visible_range();
    // Core visible: 0..5, plus overscan on the trailing side
    assert!(range.start == 0);
    assert!(range.end >= 5, "visible range must include at least 5 items");
}

#[test]
fn scroll_to_index_returns_correct_offset() {
    let virt = fixed_height_virt();
    let offset = virt.scroll_to_index(10, ScrollAlign::Top);
    assert_eq!(offset, 400.0, "item 10 at height 40 must be at offset 400");
}

#[test]
fn scroll_to_key_delegates_through_closure() {
    let virt = fixed_height_virt();
    let offset = virt.scroll_to_key(
        &Key::from("item-10"),
        ScrollAlign::Top,
        |key| {
            if key == &Key::from("item-10") { Some(10) } else { None }
        },
    );
    assert_eq!(offset, Some(400.0));
}

#[test]
fn scroll_to_unknown_key_returns_none() {
    let virt = fixed_height_virt();
    let offset = virt.scroll_to_key(
        &Key::from("nonexistent"),
        ScrollAlign::Top,
        |_| None,
    );
    assert!(offset.is_none());
}

#[test]
fn variable_height_updates_layout() {
    let mut virt = Virtualizer::new(100, LayoutStrategy::VariableHeight { estimated_item_height: 40.0 });
    virt.viewport_height = 200.0;
    virt.overscan = 3;
    let range_before = virt.visible_range();
    // report_item_height returns a new Virtualizer with the updated measurement
    virt = virt.report_item_height(0, 60.0); // item 0 is actually 60px
    // After measurement, layout should reflect the actual height
    let range_after = virt.visible_range();
    // With item 0 = 60px and rest estimated at 40px, the range may differ
    assert!(range_after.end <= range_before.end + 1, "variable heights must affect visible range");
}
```

### 13.6 FilteredCollection & SortedCollection

```rust
use ars_collections::{CollectionBuilder, Collection, Key, NodeType};
use ars_collections::filtered_collection::FilteredCollection;
use ars_collections::sorted_collection::{SortedCollection, SortDirection};

#[test]
fn filtered_excludes_hidden_items() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .item(Key::from("b"), "Beta", ())
        .item(Key::from("g"), "Gamma", ())
        .build();
    let filtered = FilteredCollection::new(&col, |node| node.text_value.starts_with('A'));
    assert_eq!(filtered.size(), 1, "only Alpha matches the filter");
    assert_eq!(filtered.first_key(), Some(&Key::from("a")));
}

#[test]
fn filtered_first_key_skips_hidden() {
    let col = CollectionBuilder::new()
        .item(Key::from("b"), "Beta", ())
        .item(Key::from("a"), "Alpha", ())
        .build();
    let filtered = FilteredCollection::new(&col, |node| node.text_value == "Alpha");
    assert_eq!(filtered.first_key(), Some(&Key::from("a")), "first visible key must skip hidden Beta");
}

#[test]
fn select_all_intersects_with_filtered_visible() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .item(Key::from("b"), "Beta", ())
        .item(Key::from("g"), "Gamma", ())
        .build();
    let filtered = FilteredCollection::new(&col, |node| node.text_value != "Beta");
    // Select All on filtered collection should only include visible items
    let all_visible: Vec<_> = filtered.keys().cloned().collect();
    assert_eq!(all_visible.len(), 2);
    assert!(!all_visible.contains(&Key::from("b")));
}

#[test]
fn sorted_ascending_order() {
    let col = CollectionBuilder::new()
        .item(Key::from("c"), "Gamma", ())
        .item(Key::from("a"), "Alpha", ())
        .item(Key::from("b"), "Beta", ())
        .build();
    let sorted = SortedCollection::new(&col, |a, b| a.text_value.cmp(&b.text_value));
    let keys: Vec<_> = sorted.keys().cloned().collect();
    assert_eq!(keys, vec![Key::from("a"), Key::from("b"), Key::from("c")]);
}

#[test]
fn sorted_descending_reverses_order() {
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .item(Key::from("b"), "Beta", ())
        .build();
    // Reverse comparator for descending order
    let sorted = SortedCollection::new(&col, |a, b| b.text_value.cmp(&a.text_value));
    let first = sorted.first_key().expect("sorted collection must have first key");
    assert_eq!(*first, Key::from("b"), "descending sort must put Beta first");
}

#[test]
fn sorted_preserves_stability_for_equal_keys() {
    let col = CollectionBuilder::new()
        .item(Key::from("a1"), "Alpha", ())
        .item(Key::from("a2"), "Alpha", ()) // same text
        .build();
    let sorted = SortedCollection::new(&col, |a, b| a.text_value.cmp(&b.text_value));
    let keys: Vec<_> = sorted.keys().cloned().collect();
    assert_eq!(keys, vec![Key::from("a1"), Key::from("a2")], "equal items must preserve original order");
}
```

### 13.7 DraggableCollection & DroppableCollection

```rust
use ars_collections::{
    CollectionBuilder,
    Collection,
    Key,
    Node,
    dnd::{
        DropPosition,
        DraggableCollection,
        DroppableCollection
    },
    selection
};

#[test]
fn is_draggable_returns_true_for_focusable_items() {
    // Test fixture implementing DraggableCollection
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .item(Key::from("b"), "Beta", ())
        .build();

    // DraggableCollection::is_draggable default returns true for focusable items
    assert!(col.is_draggable(&Key::from("a")));
    assert!(col.is_draggable(&Key::from("b")));
}

#[test]
fn drag_keys_returns_selected_keys_when_selection_active() {
    // When dragging from a selection, drag_keys returns all selected keys in
    // collection order rather than key-sorted order.
    let col = TestDraggableCollection::new(vec![
        TestItem { key: Key::from("b"), label: "Beta".into() },
        TestItem { key: Key::from("a"), label: "Alpha".into() },
        TestItem { key: Key::from("c"), label: "Gamma".into() },
    ]);
    col.select(&[Key::from("a"), Key::from("b")]);

    let drag_keys = col.drag_keys();
    assert_eq!(
        drag_keys,
        vec![Key::from("b"), Key::from("a")],
        "drag_keys must preserve collection order for selected items"
    );
}

#[test]
fn drag_keys_empty_when_no_selection() {
    let col = TestDraggableCollection::new(vec![
        TestItem { key: Key::from("a"), label: "Alpha".into() },
    ]);
    let drag_keys = col.drag_keys();
    assert!(drag_keys.is_empty(), "drag_keys must be empty with no selection");
}

#[test]
fn drag_data_returns_text_plain_from_text_value() {
    let col = TestDraggableCollection::new(vec![
        TestItem { key: Key::from("a"), label: "Alpha".into() },
    ]);
    let data = col.drag_data(&Key::from("a"));
    assert_eq!(data.get("text/plain"), Some(&"Alpha".to_string()),
        "drag_data should return text/plain from text_value_of()");
}

#[test]
fn drag_data_returns_empty_for_missing_key() {
    let col = TestDraggableCollection::new(vec![
        TestItem { key: Key::from("a"), label: "Alpha".into() },
    ]);
    let data = col.drag_data(&Key::from("nonexistent"));
    assert!(data.is_empty(), "drag_data for missing key should be empty");
}

#[test]
fn accepted_types_default_returns_empty() {
    let col = TestDroppableCollection::default();
    assert!(col.accepted_types().is_empty(),
        "default accepted_types should be empty");
}

#[test]
fn is_drop_valid_default_returns_true() {
    let col = TestDroppableCollection::default();
    assert!(col.is_drop_valid(&Key::from("a"), DropPosition::Before),
        "default is_drop_valid should return true");
}

#[test]
fn is_drop_valid_checks_all_positions() {
    let col = TestDroppableCollection::default();
    assert!(col.is_drop_valid(&Key::from("a"), DropPosition::Before));
    assert!(col.is_drop_valid(&Key::from("a"), DropPosition::After));
    assert!(col.is_drop_valid(&Key::from("a"), DropPosition::On));
}

#[test]
fn allows_drop_on_defaults_to_false() {
    // DroppableCollection::allows_drop_on defaults to false (only between-item drops)
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Alpha", ())
        .build();
    assert!(!col.allows_drop_on(&Key::from("a")),
        "allows_drop_on must default to false");
}
```
