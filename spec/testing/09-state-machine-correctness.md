# State Machine Correctness

## 1. State Machine Correctness Tests

> This section defines systematic tests that detect state machine defects:
> state/context desync, missing handlers, guard gaps, effect callback timing, and API surface
> mismatches.

### 1.1 State ↔ Context Sync Verification

Some machines track `open: bool` on Context alongside a `State::Open` enum
variant. If a transition uses `context_only` when it should use
`TransitionPlan::to(State::Open)`, the two fall out of sync.

**Test pattern — assert sync after every event:**

```rust
/// Invariant: `ctx.open == (*state == State::Open)` after every transition.
/// Components that track an `open` bool on Context must keep it in sync
/// with the State enum. This function is called after every send().
fn assert_state_ctx_sync(svc: &Service<combobox::Machine>) {
    let state_is_open = *svc.state() == combobox::State::Open;
    let ctx_is_open = svc.context().open;
    assert_eq!(
        state_is_open, ctx_is_open,
        "State/ctx.open desync: state={:?} but ctx.open={}",
        svc.state(), ctx_is_open,
    );
}

#[test]
fn combobox_state_ctx_open_sync() {
    let props = combobox::Props {
        open_on_focus: true,
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());

    // Focus with open_on_focus=true must transition State AND ctx.open
    svc.send(combobox::Event::Focus { is_keyboard: true });
    let state_is_open = *svc.state() == combobox::State::Open;
    let ctx_is_open = svc.context().open;
    assert_eq!(state_is_open, ctx_is_open,
        "State/ctx.open desync: state={:?} but ctx.open={}", svc.state(), ctx_is_open);
}
```

**Components requiring this test:** Combobox, Select, Dialog, Popover,
DatePicker, Menu, Tooltip, HoverCard, Accordion — any machine with both a `State::Open` variant and a
`ctx.open` boolean.

### 1.2 Event Enum Exhaustiveness Tests

Every event variant in the `Event` enum MUST have at least one `(State, Event)`
pair where `transition()` returns `Some(...)`. An event that always returns
`None` in all states is a **dead event**.

```rust
/// For each Event variant, verify at least one state produces Some.
#[test]
fn no_dead_events() {
    let props = dialog::Props::default();
    let ctx = dialog::Context::default();
    let all_states = vec![State::Idle, State::Focused];
    let all_events = vec![
        Event::Focus { is_keyboard: false },
        Event::Blur,
        Event::SelectItem(Key::from("test")),
        // ... every variant
    ];

    for event in &all_events {
        let any_handled = all_states.iter().any(|state| {
            Machine::transition(state, event, &ctx, &props).is_some()
        });
        assert!(any_handled,
            "Dead event: {:?} returns None in all states", event);
    }
}
```

**Known exceptions** (document these explicitly in component specs):

- `Dialog::AnimationStart` / `Dialog::AnimationEnd` — reserved for CSS
  animation lifecycle; intentionally no transition.
- `Popover::PositioningUpdate` — adapter-only event; returns None by design.

### 1.3 Focus/Blur Handler Presence

Every machine that declares `Focus` and `Blur` event variants MUST handle them
in the transition function. Missing handlers cause the component to appear
non-interactive.

```rust
#[test]
fn focus_blur_handled() {
    let props = Props::default();
    let (init_state, init_ctx) = Machine::init(&props, &Env::default(), &Default::default());

    // Focus must produce Some from idle/closed state
    let plan = Machine::transition(
        &init_state,
        &Event::Focus { is_keyboard: true },
        &init_ctx,
        &props,
    );
    assert!(plan.is_some(), "Focus event must be handled from initial state");

    // Blur must produce Some from focused/open state
    // (construct the focused state first)
    if let Some(mut plan) = plan {
        let mut ctx = init_ctx.clone();
        if let Some(apply) = plan.apply.take() {
            apply(&mut ctx);
        }
        let focused_state = plan.target.unwrap_or(init_state.clone());
        let blur_plan = Machine::transition(
            &focused_state,
            &Event::Blur,
            &ctx,
            &props,
        );
        assert!(blur_plan.is_some(), "Blur event must be handled from focused state");
    }
}
```

**Flagged by audit:** PinInput declares Focus/Blur in its Event enum but omits
transition handlers for both.

### 1.4 Disabled Guard Completeness

Every interactive component MUST have a disabled guard test suite that verifies:

1. ALL value-changing events return `None` when `ctx.disabled == true`.
2. Focus/Blur events are either blocked or explicitly documented as allowed.
3. The guard is checked BEFORE any state match arms.

```rust
#[test]
fn disabled_blocks_all_interactive_events() {
    let mut ctx = default_ctx();
    ctx.disabled = true;
    let props = Props::default();

    // Every event that modifies state or context must be blocked
    let interactive_events = vec![
        Event::SelectItem(Key::from("a")),
        Event::Open,
        Event::Toggle,
        // ... all value-changing events
    ];

    for event in &interactive_events {
        for state in &all_states() {
            let plan = Machine::transition(state, event, &ctx, &props);
            assert!(plan.is_none(),
                "Disabled guard missed: {:?} in state {:?} returned Some", event, state);
        }
    }
}
```

**Components missing disabled guards (flagged by audit):**

- HoverCard — no `disabled` field or guard at all.
- Menu — no `disabled` field or guard at all.

**Components with intentional disabled-blocks-everything behavior:**

- Calendar, DateField — disabled blocks ALL events including FocusIn/FocusOut.
  Document this decision in the component spec.

### 1.5 Readonly Guard Tests

Readonly components must block value-changing events while allowing navigation
and focus. Test that readonly and disabled behaviors are distinct.

```rust
#[test]
fn readonly_allows_focus_but_blocks_mutation() {
    let mut ctx = default_ctx();
    ctx.readonly = true;
    let props = Props::default();

    // Focus should work
    let plan = Machine::transition(
        &State::Idle,
        &Event::Focus { is_keyboard: true },
        &ctx,
        &props,
    );
    assert!(plan.is_some(), "Focus must work in readonly mode");

    // Value changes should be blocked
    let plan = Machine::transition(
        &State::Focused,
        &Event::SelectItem(Key::from("a")),
        &ctx,
        &props,
    );
    assert!(plan.is_none(), "SelectItem must be blocked in readonly mode");
}
```

**Components missing readonly (flagged by audit):**

- PinInput — has no `readonly` field in Context. Must be added.

### 1.6 Dual-Source Open/Close Tests

Components with both hover and focus as open/close sources (Tooltip, HoverCard)
must test that removing ONE source doesn't close the component when the other
source is still active.

```rust
#[test]
fn hovercard_stays_open_when_one_source_remains() {
    let props = hover_card::Props::default();
    let mut svc = Service::new(props, Env::default(), Default::default());

    // Open via hover
    svc.send(hover_card::Event::TriggerPointerEnter);
    svc.send(hover_card::Event::OpenTimerFired);
    assert_eq!(*svc.state(), State::Open);

    // Also focus the trigger
    // (simulate keyboard focus arriving while pointer is on trigger)
    // After this, both hover_active and focus_active should be true.

    // Remove hover — focus should keep it open
    svc.send(hover_card::Event::TriggerPointerLeave);
    assert_ne!(*svc.state(), State::Closed,
        "HoverCard closed when focus was still active");
}

#[test]
fn hovercard_pointer_leave_updates_hover_active() {
    let props = Props::default();
    let mut svc = Service::new(props, Env::default(), Default::default());

    svc.send(Event::TriggerPointerEnter);
    assert!(svc.context().hover_active);

    svc.send(Event::OpenTimerFired);
    svc.send(Event::TriggerPointerLeave);

    // hover_active must be set to false even if transitioning to ClosePending
    assert!(!svc.context().hover_active,
        "hover_active not cleared on TriggerPointerLeave");
}
```

**Flagged by audit:** HoverCard's `Open→TriggerPointerLeave|TriggerBlur`
handler:

1. Doesn't check if the other source (focus/hover) is still active.
2. Doesn't clear `hover_active`/`focus_active` on the transition.

### 1.7 Effect Callback Timing Tests

Effects that invoke callbacks (e.g., `on_value_complete`, `on_change`) must
invoke them in the **setup** function, not the **cleanup** function.

```rust
#[test]
fn effect_callback_fires_in_setup_not_cleanup() {
    use std::cell::Cell;
    use std::rc::Rc;

    let setup_fired = Rc::new(Cell::new(false));
    let cleanup_fired = Rc::new(Cell::new(false));
    let sf = setup_fired.clone();
    let cf = cleanup_fired.clone();

    // The effect's setup closure should invoke the callback immediately.
    // The cleanup closure should only release resources.
    // Test by checking when the callback fires relative to setup/cleanup.

    let props = pin_input::Props::default();
    let mut svc = Service::new(props, Env::default(), Default::default());

    // Fill all pins to trigger the Complete transition
    for i in 0..svc.context().pin_count {
        svc.send(pin_input::Event::InputChar { index: i, ch: '1' });
    }

    // Run the pending effects — setup fires on_value_complete, cleanup does not.
    let send_fn: Arc<dyn Fn(pin_input::Event) + Send + Sync> = Arc::new(|_| {});

    let result = svc.send(pin_input::Event::InputChar { index: svc.context().pin_count - 1, ch: '1' });
    for effect in result.pending_effects {
        sf.set(true); // Mark setup as fired when we execute the effect
        let cleanup = effect.run(svc.context(), svc.props(), send_fn.clone());
        // Cleanup should not invoke the callback
        cleanup();
        cf.set(true);
    }

    assert!(setup_fired.get(), "Effect setup callback should have fired");
    assert!(cleanup_fired.get(), "Effect cleanup should have run without error");
}
```

**Flagged by audit:** PinInput's Complete effect invokes `on_value_complete`
inside the cleanup function (`Box::new(move || { cb(&combined); })`). The
callback should fire in the setup body, and the cleanup should be a no-op or
release timer resources.

### 1.8 `init()` Trait Compliance Test

Every Machine implementation's `init()` must match the trait signature exactly:
`fn init(props: &Self::Props, _env: &Env, _messages: &Self::Messages) -> (Self::State, Self::Context)`.

```rust
/// Compile-time verified by the trait system, but this test ensures
/// init produces a valid (State, Context) pair from default Props.
#[test]
fn init_returns_valid_pair() {
    let props = Props::default();
    let (state, ctx) = Machine::init(&props, &Env::default(), &Default::default());
    // Verify initial state is the expected variant for this component.
    // Each component's init() test should assert against its documented initial state:
    // - Toggle: State::Off
    // - Dialog: State::Closed
    // - Checkbox: State::Unchecked
    // - Tabs: State::Idle (with first tab selected in context)
    assert_eq!(state, State::Idle, "init should produce the documented initial state");
    // Verify context has valid IDs
    assert!(!ctx.ids.id().is_empty(), "init must set component IDs");
}
```

**Flagged by audit:** Combobox `init()` takes an extra `items:StaticCollection<combobox::Item>` parameter that doesn't match the Machine trait.
Resolution: items must be supplied through Props, through a separate builder, or
through a post-init `UpdateItems` event.

### 1.9 Timer Race Condition Tests

Components with timer effects (Tooltip, HoverCard, Toast, DateField typeahead)
must verify that rapid event sequences don't create orphaned timers or stale
callbacks.

```rust
#[test]
fn rapid_open_close_cancels_pending_timer() {
    let props = tooltip::Props::default();
    let mut svc = Service::new(props, Env::default(), Default::default());

    // Rapidly toggle: pointer enter → leave → enter → leave
    svc.send(Event::PointerEnter);  // → OpenPending, starts timer
    svc.send(Event::PointerLeave);  // → Closed, timer cleanup runs
    svc.send(Event::PointerEnter);  // → OpenPending, NEW timer
    svc.send(Event::PointerLeave);  // → Closed, NEW timer cleanup

    // Now fire what would be the FIRST timer's callback
    // (simulating a timer that fired despite being "cancelled")
    svc.send(Event::OpenTimerFired);
    assert_eq!(*svc.state(), State::Closed,
        "Stale timer callback should be a no-op in Closed state");
}

#[test]
fn named_effect_replaces_previous() {
    // When an effect with the same name fires twice, the adapter must
    // run the old cleanup before setting up the new one.
    // This prevents timer leaks in typeahead, debounce, etc.
    let props = select::Props::default();
    let mut svc = Service::<select::Machine>::new(props);

    let result1 = svc.send(select::Event::TypeaheadSearch('a', 0));
    let result2 = svc.send(select::Event::TypeaheadSearch('b', 100));

    // Both should have a "typeahead_timeout" effect
    assert!(result1.pending_effects.iter().any(|e| e.name == "typeahead_timeout"));
    assert!(result2.pending_effects.iter().any(|e| e.name == "typeahead_timeout"));
    // Adapter must cancel the first before setting up the second.
}

#[test]
fn rapid_events_no_orphaned_timers() {
    let props = tooltip::Props::default();
    let mut svc = Service::new(props, Env::default(), Default::default());
    let mut active_effects: Vec<Box<dyn FnOnce()>> = Vec::new();

    // Rapid sequence: open → close → open → close × 50
    // PendingEffect::run() takes Arc<dyn Fn(M::Event) + Send + Sync> on all targets.
    let send_fn: Arc<dyn Fn(tooltip::Event) + Send + Sync> = Arc::new(|_| {});

    for _ in 0..50 {
        let result = svc.send(tooltip::Event::PointerEnter);
        for effect in result.pending_effects {
            let cleanup = effect.run(svc.context(), svc.props(), send_fn.clone());
            active_effects.push(cleanup);
        }

        // Clean up before next event
        let result = svc.send(tooltip::Event::PointerLeave);
        for cleanup in active_effects.drain(..) {
            cleanup();
        }
    }

    assert!(active_effects.is_empty(), "orphaned timer cleanups detected");
    assert_eq!(*svc.state(), tooltip::State::Closed);
}
```

### 1.10 `then_send` Chain Tests

`TransitionPlan::then(event)` enqueues a follow-up event. Test that:

1. The follow-up event is processed in the same drain cycle.
2. Chained events respect guards (disabled/readonly).
3. Chains don't create infinite loops (the `truncated` field on `SendResult` detects this).

```rust
use ars_core::{Service, KeyboardKey};

#[test]
fn then_send_chain_fires_followup() {
    let props = calendar::Props::default();
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(calendar::Event::FocusIn);

    // Enter/Space in the calendar fires `.then(Event::SelectDate { ... })`
    svc.send(calendar::Event::KeyDown { key: KeyboardKey::Enter });

    // The SelectDate should have been processed in the same drain cycle
    assert!(svc.context().value.get().is_some(),
        "then_send SelectDate should update value");
}

#[test]
fn then_send_respects_disabled_guard() {
    let props = calendar::Props { disabled: true, ..Default::default() };
    let mut svc = Service::new(props, Env::default(), Default::default());

    // Even if a then_send chain would fire SelectDate,
    // the disabled guard should block it.
    svc.send(calendar::Event::KeyDown { key: KeyboardKey::Enter });
    assert!(svc.context().value.get().is_none(),
        "then_send chain should respect disabled guard");
}
```

### 1.11 Effect Cleanup on Rapid State Transitions Within Drain

When a `then_send` chain causes rapid state transitions within a single `send()`
call, effects from intermediate states must appear in `cancel_effects` so that
the adapter can run their cleanup functions.

```rust
#[test]
fn rapid_transition_cancels_previous_effects() {
    let props = tooltip::Props::default();
    let mut svc = Service::new(props, Env::default(), Default::default());

    // Open tooltip (starts open-delay effect)
    let open_result = svc.send(tooltip::Event::PointerEnter);
    assert!(!open_result.pending_effects.is_empty(), "Opening should produce delay effect");

    // Immediately close before delay completes
    let close_result = svc.send(tooltip::Event::PointerLeave);

    // The open-delay effect should be in cancel_effects
    assert!(
        close_result.cancel_effects.len() > 0,
        "Closing during delay should cancel the pending open effect"
    );
}
```

### 1.11.1 Rapid Transition Cleanup Test

```rust
use ars_core::{Service, CleanupFn};

#[test]
fn rapid_transitions_clean_up_effects() {
    let props = tooltip::Props::new("t1");
    let mut svc = Service::new(props.clone(), Env::default(), Default::default());
    // PendingEffect::run() takes Arc<dyn Fn(M::Event) + Send + Sync> on all targets.
    let send_fn: Arc<dyn Fn(tooltip::Event) + Send + Sync> = Arc::new(|_| {});
    let mut active_cleanups: Vec<CleanupFn> = Vec::new();

    // Rapid open-close-open sequence
    let r1 = svc.send(tooltip::Event::PointerEnter);
    for effect in r1.pending_effects {
        active_cleanups.push(effect.run(svc.context(), svc.props(), send_fn.clone()));
    }

    let r2 = svc.send(tooltip::Event::PointerLeave);
    // Close should cancel open effects
    for cleanup in active_cleanups.drain(..) { cleanup(); }
    for effect in r2.pending_effects {
        active_cleanups.push(effect.run(svc.context(), svc.props(), send_fn.clone()));
    }

    let r3 = svc.send(tooltip::Event::PointerEnter);
    for cleanup in active_cleanups.drain(..) { cleanup(); }
    for effect in r3.pending_effects {
        active_cleanups.push(effect.run(svc.context(), svc.props(), send_fn.clone()));
    }

    // Verify no leaked cleanups — unmount should process remaining
    svc.unmount(active_cleanups);
}
```

### 1.12 `connect()` Field Reference Tests

Every field referenced in `connect()` / API methods must exist on Context. This
is compile-time verified, but snapshot tests ensure attribute stability.

```rust
#[test]
fn connect_produces_all_required_attrs() {
    let props = Props::default();
    let (state, ctx) = Machine::init(&props, &Env::default(), &Default::default());
    let api = Machine::connect(&state, &ctx, &props, &|_| {});

    // Root attrs must always include ars-scope and ars-part
    let root = api.root_attrs();
    assert!(root.get(&HtmlAttr::Data("ars-scope")).is_some());
    assert!(root.get(&HtmlAttr::Data("ars-part")).is_some());

    // Component-specific: verify every part's attrs method
    // doesn't panic and produces expected ARIA attributes.
}

#[test]
fn connect_produces_valid_attrs_in_all_reachable_states() {
    let props = dialog::Props::new("d1");
    let mut svc = Service::new(props.clone(), Env::default(), Default::default());

    // Test connect in initial (Closed) state
    let api = svc.connect(&|_| {});
    assert_role(&api.root_attrs(), "dialog");
    assert_eq!(api.root_attrs().get(&HtmlAttr::Aria(AriaAttr::Expanded)), Some("false"));

    // Transition to Open and test connect
    svc.send(dialog::Event::Open);
    let api = svc.connect(&|_| {});
    assert_eq!(api.root_attrs().get(&HtmlAttr::Aria(AriaAttr::Expanded)), Some("true"));
    assert_eq!(api.root_attrs().get(&HtmlAttr::Aria(AriaAttr::Modal)), Some("true"));
}
```

### 1.13 TransitionPlan API Surface Tests

Verify that component code only uses methods that exist on `TransitionPlan`:

| Method                            | Exists | Notes                                           |
| --------------------------------- | ------ | ----------------------------------------------- |
| `TransitionPlan::to(state)`       | Yes    | Transitions to new state                        |
| `TransitionPlan::context_only(f)` | Yes    | Context mutation, no state change               |
| `.apply(f)`                       | Yes    | Mutate context on transition                    |
| `.then(event)`                    | Yes    | Enqueue follow-up event                         |
| `.with_effect(effect)`            | Yes    | Attach a single PendingEffect                   |
| `.with_named_effect(name, f)`     | Yes    | Named effect (replaces previous with same name) |
| `.with_effects()`                 | **No** | Does not exist — use `.with_effect()`           |

**Flagged by audit:** Select typeahead handler calls `.with_effects()` (plural)
which is not part of the TransitionPlan API. Must use `.with_effect()`.

### 1.14 PendingEffect Mutability Contract

`PendingEffect::setup` receives `(&M::Context, &M::Props, Arc<dyn Fn(M::Event) + Send + Sync>)` on all targets.
The Context and Props references are **immutable**. Effects MUST NOT attempt to
write to Context fields. Side effects that need to store state (timer IDs,
listener handles) must use the cleanup return or adapter-managed storage.

```rust
#[test]
fn effect_setup_does_not_require_mut_context() {
    // This is a compile-time guarantee from the PendingEffect signature.
    // But verify at test time that effects don't panic when receiving
    // an immutable context snapshot.
    let props = tooltip::Props::default();
    let mut svc = Service::new(props.clone(), Env::default(), Default::default());

    let result = svc.send(tooltip::Event::PointerEnter);
    // PendingEffect::run() takes Arc<dyn Fn(M::Event) + Send + Sync> on all targets.
    let send_fn: Arc<dyn Fn(tooltip::Event) + Send + Sync> = Arc::new(|_| {});
    for effect in result.pending_effects {
        // This must not panic — context is immutable
        let cleanup = effect.run(svc.context(), svc.props(), send_fn.clone());
        cleanup(); // cleanup must also not panic
    }
}
```

**Flagged by audit:** Select typeahead effect setup writes
`ctx.typeahead_timer_id = Some(...)` through an immutable `&Context` reference.
This won't compile. Timer ID tracking must use adapter-managed storage or named
effect replacement.

### 1.15 IME Composition Tests

Input components that accept text (NumberInput, PinInput) must handle IME
composition events correctly. These tests verify the `is_composing` flag lifecycle
and that intermediate composition input is not prematurely committed.

```rust
#[test]
fn composition_start_sets_is_composing() {
    let props = number_input::Props::default();
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(number_input::Event::FocusIn);

    assert!(!svc.context().is_composing, "initially not composing");

    svc.send(number_input::Event::CompositionStart);
    assert!(svc.context().is_composing, "CompositionStart must set is_composing");

    // Input events during composition are suppressed (not committed)
    svc.send(number_input::Event::InputChange { value: "1".into() });
    let value_during = svc.context().value.clone();

    svc.send(number_input::Event::CompositionEnd { data: "12".into() });
    assert!(!svc.context().is_composing, "CompositionEnd must clear is_composing");
    assert_ne!(svc.context().value, value_during,
        "CompositionEnd should commit the composed text");
}

#[test]
fn disabled_component_rejects_composition_events() {
    let props = pin_input::Props { disabled: true, ..Default::default() };
    let mut svc = Service::new(props, Env::default(), Default::default());

    svc.send(pin_input::Event::CompositionStart);
    assert!(!svc.context().is_composing,
        "disabled component must reject CompositionStart");

    svc.send(pin_input::Event::CompositionEnd { data: "a".into() });
    assert!(svc.context().values.iter().all(|v| v.is_empty()),
        "disabled component must reject CompositionEnd input");
}

#[test]
fn text_field_ime_composition_produces_final_value() {
    let props = text_field::Props::new("tf1");
    let mut svc = Service::new(props.clone(), Env::default(), Default::default());
    svc.send(text_field::Event::CompositionStart);
    // Intermediate composition updates are handled by the adapter (DOM events),
    // not by the state machine. The machine only sees Start and End.
    svc.send(text_field::Event::CompositionEnd);
    // After composition, the adapter fires a Change event with the final value.
    svc.send(text_field::Event::Change("日本".into()));
    assert_eq!(svc.context().value, "日本");
}
```

### 1.16 Collection Trait Integration Tests

The `Collection<T>` trait (and its primary implementation `StaticCollection<T>`)
is used across all list-rendering components. These tests verify the API contract
and integration with navigation and selection patterns.

> `CollectionBuilder` is defined in `ars-collections` and provides a
> fluent API for building item collections used by selection components.
> See spec/foundation/06-collections.md for the full API.

```rust
#[test]
fn static_collection_api_contract() {
    let collection = CollectionBuilder::new()
        .item(combobox::Item { key: "a".into(), label: "Alpha".into(), disabled: false })
        .item(combobox::Item { key: "b".into(), label: "Beta".into(), disabled: true })
        .item(combobox::Item { key: "c".into(), label: "Gamma".into(), disabled: false })
        .build();

    assert_eq!(collection.count(), 3);
    assert_eq!(collection.key_at(0), Some("a".into()));
    assert_eq!(collection.key_at(3), None, "out-of-bounds returns None");
    assert_eq!(collection.index_of("b"), Some(1));
    assert_eq!(collection.index_of("z"), None, "unknown key returns None");
}

#[test]
fn disabled_key_filtering_via_collection() {
    let collection = CollectionBuilder::new()
        .item(listbox::Item { key: "a".into(), label: "Alpha".into(), disabled: false })
        .item(listbox::Item { key: "b".into(), label: "Beta".into(), disabled: true })
        .item(listbox::Item { key: "c".into(), label: "Gamma".into(), disabled: false })
        .build();

    let enabled: Vec<_> = (0..collection.count())
        .filter(|&i| {
            let key = collection.key_at(i).expect("key_at must return Some for valid index");
            !collection.is_disabled(&key)
        })
        .collect();

    assert_eq!(enabled, vec![0, 2], "disabled items must be filterable");
}

#[test]
fn keyboard_navigation_respects_collection_ordering() {
    let props = combobox::Props {
        collection: CollectionBuilder::new()
            .item(combobox::Item { key: "a".into(), label: "Alpha".into(), disabled: false })
            .item(combobox::Item { key: "b".into(), label: "Beta".into(), disabled: true })
            .item(combobox::Item { key: "c".into(), label: "Gamma".into(), disabled: false })
            .build(),
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(combobox::Event::Open);

    // ArrowDown from first item should skip disabled "b" and land on "c"
    svc.send(combobox::Event::HighlightFirst);
    assert_eq!(svc.context().highlighted_key, Some("a".into()));

    svc.send(combobox::Event::HighlightNext);
    assert_eq!(svc.context().highlighted_key, Some("c".into()),
        "HighlightNext must skip disabled items per collection ordering");
}

#[test]
fn empty_collection_produces_accessible_fallback() {
    let props = listbox::Props {
        collection: CollectionBuilder::new().build(),
        ..Default::default()
    };
    let (state, ctx) = listbox::Machine::init(&props, &Env::default(), &Default::default());
    let api = listbox::Machine::connect(&state, &ctx, &props, &|_| {});

    let root = api.root_attrs();
    // An empty listbox must still render a valid ARIA role and announce emptiness
    assert_eq!(root.get(&HtmlAttr::Role), Some("listbox"));
    // Adapter should render the messages.empty_label when collection.count() == 0
}
```

### 1.17 Follow-up Event Cycle Detection Tests

Follow-up events (`then_send` / `follow_up`) can introduce infinite cycles when
two states ping-pong events between each other. The drain queue
(the drain queue's maximum iteration limit) prevents runaway loops, but tests must verify that
the safeguard works correctly and that the machine remains valid after truncation.
The `SendResult::truncated` field indicates when this limit is hit.

#### 1.17.1 Test 1: Cyclic `then_send` Detection

```rust
/// Two states continuously send follow-up events to each other.
/// The drain queue must hit its maximum iteration limit and set `truncated`.
/// Note: The `truncated` field on `SendResult` indicates the drain queue
/// hit its maximum iteration limit. Warnings are logged via the `log`
/// facade, not returned in `SendResult`.
#[test]
fn cyclic_follow_up_is_detected_and_bounded() {
    // Minimal machine: StateA transitions to StateB with follow_up(GoB),
    // StateB transitions to StateA with follow_up(GoA).
    let props = cycle_test::Props::default();
    let mut svc = Service::new(props, Env::default(), Default::default());

    // Trigger the cycle
    let result = svc.send(cycle_test::Event::GoB);

    // The `truncated` field indicates the drain queue hit its maximum iteration limit.
    assert!(
        result.truncated,
        "expected drain queue to be truncated (hit max iteration limit)",
    );
}
```

#### 1.17.2 Test 2: Controlled Prop Sync Cycle Guard

```rust
/// Parent updates a controlled prop inside on_change, which would trigger
/// another on_change. The guard (skip if value unchanged) breaks the cycle.
#[test]
fn controlled_prop_sync_cycle_is_broken_by_guard() {
    let call_count = Rc::new(Cell::new(0u32));
    let count = call_count.clone();

    let props = search_input::Props {
        value: Some("initial".into()),
        on_change: Some(Box::new(move |_: &str| { count.set(count.get() + 1); })),
        ..Default::default()
    };
    let mut svc = Service::new(props, Env::default(), Default::default());

    // Simulate: on_change sets the controlled value back to the same value.
    // The machine must detect no actual change and skip re-emission.
    svc.send(search_input::Event::Change("updated".into()));
    assert_eq!(call_count.get(), 1, "on_change should fire once for the first change");
    let after_first = svc.context().value.get().clone();

    // Sending the same value again must not trigger another transition
    let result = svc.send(search_input::Event::Change(after_first.clone()));
    assert!(
        !result.state_changed,
        "re-setting identical value must not produce a state transition"
    );
    assert!(
        !result.context_changed,
        "re-setting identical value must not change context"
    );
    assert_eq!(call_count.get(), 1, "on_change must not fire again for identical value");
}
```

#### 1.17.3 Test 3: Post-Truncation State Validity

```rust
/// After the drain queue is truncated, the machine must be in a valid,
/// consistent state — not corrupted or half-transitioned.
#[test]
fn machine_is_valid_after_drain_truncation() {
    let props = cycle_test::Props::default();
    let mut svc = Service::new(props, Env::default(), Default::default());

    // Trigger the cycle (will be truncated)
    let result = svc.send(cycle_test::Event::GoB);
    assert!(result.truncated, "cycle must trigger drain truncation");

    // Machine must be in one of its declared states (not a partial transition)
    let state = svc.state();
    assert!(
        matches!(state, cycle_test::State::A | cycle_test::State::B),
        "machine must be in a valid declared state after truncation, got: {:?}",
        state,
    );

    // Machine must still accept new events normally after truncation
    let result = svc.send(cycle_test::Event::Reset);
    assert_eq!(svc.state(), &cycle_test::State::A);
    assert!(!result.truncated, "normal event after truncation should not truncate");
}
```

### 1.18 Props Change Re-evaluation

Tests verify that `Service::set_props()` triggers `Machine::on_props_changed()` and enqueues the returned events.

```rust
use ars_core::Bindable;

#[test]
fn set_props_triggers_on_props_changed() {
    let initial_props = slider::Props { min: 0.0, max: 100.0, value: Bindable::controlled(50.0), ..Default::default() };
    let mut svc = Service::new(initial_props, Env::default(), Default::default());
    assert_eq!(svc.context().value, 50.0);

    // Update props with new controlled value
    let new_props = slider::Props { min: 0.0, max: 100.0, value: Bindable::controlled(75.0), ..Default::default() };
    svc.set_props(new_props);

    // on_props_changed should have returned a SetValue event that was processed
    assert_eq!(svc.context().value, 75.0, "set_props must trigger controlled value sync");
}

#[test]
fn set_props_no_change_produces_no_events() {
    let props = checkbox::Props::default();
    let mut svc = Service::new(props.clone(), Env::default(), Default::default());
    let state_before = svc.state().clone();
    let ctx_before = svc.context().clone();

    svc.set_props(props.clone());

    assert_eq!(*svc.state(), state_before, "identical props must not change state");
    assert_eq!(*svc.context(), ctx_before, "identical props must not change context");
}

#[test]
fn on_props_changed_with_identical_props_returns_empty() {
    let props = checkbox::Props::default();
    let events = checkbox::Machine::on_props_changed(&props, &props);
    assert!(
        events.is_empty(),
        "on_props_changed with identical props must return empty Vec"
    );
}
```

### 1.19 Guard Ordering Verification

Tests verify that guard conditions (disabled, readonly, loading, errored) are checked at the top of `transition()` before any state-specific match arms. A guard that only appears inside specific states creates a gap.

```rust
use ars_core::{Service, Machine, Key};

/// Verifies that disabled guard rejects events in ALL states, not just specific ones.
#[test]
fn disabled_guard_applies_to_all_states() {
    let props = combobox::Props { disabled: true, ..Default::default() };
    for state in combobox::State::iter_all() {
        let ctx = combobox::Machine::init(&props, &Env::default(), &Default::default()).1;
        let plan = combobox::Machine::transition(&state, &combobox::Event::Open, &ctx, &props);
        assert!(
            plan.is_none() || plan.as_ref().map_or(false, |p| p.target.is_none()),
            "disabled guard must reject Open event in state {state:?}"
        );
    }
}

/// Verifies that readonly guard allows navigation but blocks mutation in all states.
#[test]
fn readonly_guard_blocks_mutation_in_all_states() {
    let props = combobox::Props { readonly: true, ..Default::default() };
    let mutation_events = [
        combobox::Event::SelectItem(Key::from("a")),
        combobox::Event::Clear,
    ];
    for state in combobox::State::iter_all() {
        let ctx = combobox::Machine::init(&props, &Env::default(), &Default::default()).1;
        for event in &mutation_events {
            let plan = combobox::Machine::transition(&state, event, &ctx, &props);
            assert!(
                plan.is_none() || plan.as_ref().map_or(false, |p| p.target.is_none()),
                "readonly guard must reject {event:?} in state {state:?}"
            );
        }
    }
}

/// Guard test: readonly state blocks mutation events but allows navigation
#[test]
fn readonly_combobox_blocks_mutation() {
    let props = combobox::Props { readonly: true, ..Default::default() };
    let mut svc = Service::new(props, Env::default(), Default::default());
    let result = svc.send(combobox::Event::InputChange("test".into()));
    assert!(!result.state_changed,
        "readonly guard must prevent InputChange from changing state");
    // Navigation should still work
    let result = svc.send(combobox::Event::HighlightNext);
    // HighlightNext updates context but may not change state
    assert!(result.context_changed || !result.state_changed);
}
```

## 2. Property-Based Testing

State machines have well-defined invariants that must hold for **all** event sequences, not just hand-picked ones. Property-based testing explores random event sequences to find invariant violations that table-driven tests miss.

### 2.1 Pattern with `proptest`

```rust
use proptest::prelude::*;

fn arb_toggle_event() -> impl Strategy<Value = toggle::Event> {
    prop_oneof![
        Just(toggle::Event::Toggle),
        Just(toggle::Event::SetPressed(true)),
        Just(toggle::Event::SetPressed(false)),
    ]
}

proptest! {
    /// No event sequence should ever cause a panic in the state machine.
    #[test]
    fn toggle_never_panics(events in prop::collection::vec(arb_toggle_event(), 0..100)) {
        let props = toggle::Props { id: "prop-test".into(), ..Default::default() };
        let mut svc = Service::new(props, Env::default(), Default::default());
        for event in events {
            let _ = svc.send(event);
        }
    }

    /// ARIA attributes must always be valid after any event sequence.
    #[test]
    fn toggle_aria_always_valid(events in prop::collection::vec(arb_toggle_event(), 0..50)) {
        let props = toggle::Props { id: "prop-test".into(), ..Default::default() };
        let mut svc = Service::new(props, Env::default(), Default::default());
        for event in events {
            svc.send(event);
        }
        let api = svc.connect(&|_| {});
        let attrs = api.root_attrs();

        // Invariant: role is always "button"
        assert_eq!(attrs.get(&HtmlAttr::Role), Some(&AttrValue::String("button".into())));
        // Invariant: aria-pressed is always "true" or "false"
        let pressed = attrs.get(&HtmlAttr::Aria(AriaAttr::Pressed));
        assert!(
            pressed == Some(&AttrValue::String("true".into()))
            || pressed == Some(&AttrValue::String("false".into())),
            "aria-pressed must be 'true' or 'false', got {:?}", pressed
        );
    }

    /// The drain_queue loop must never truncate (no infinite cycles).
    #[test]
    fn drain_queue_never_truncates(events in prop::collection::vec(arb_toggle_event(), 0..200)) {
        let props = toggle::Props { id: "prop-test".into(), ..Default::default() };
        let mut svc = Service::new(props, Env::default(), Default::default());
        for event in events {
            let result = svc.send(event);
            assert!(!result.truncated, "drain_queue truncated on event {:?}", event);
        }
    }
}
```

### 2.2 Arbitrary event generators per component

Each component SHOULD provide an `arb_{component}_event()` function that generates all valid event variants, including boundary values:

```rust
// Generators MUST draw parametric values (item keys, indices) from provided
// collections, not random strings. Using random strings causes the state machine
// to handle items that don't exist in its collection, which is not a valid
// real-world scenario and produces false-positive "failures" that obscure real bugs.
fn arb_select_event(items: &[Key]) -> impl Strategy<Value = select::Event> + '_ {
    let item_keys = items.to_vec();
    prop_oneof![
        Just(select::Event::Open),
        Just(select::Event::Close),
        Just(select::Event::Toggle),
        (any::<char>(), any::<u64>()).prop_map(|(c, t)| select::Event::TypeaheadSearch(c, t)),
        // Draw highlight/select keys from the fixed item set
        proptest::sample::select(item_keys.clone())
            .prop_map(|k| select::Event::HighlightItem(Some(k))),
        Just(select::Event::HighlightItem(None)),
        proptest::sample::select(item_keys.clone())
            .prop_map(select::Event::SelectItem),
        Just(select::Event::HighlightFirst),
        Just(select::Event::HighlightLast),
        Just(select::Event::HighlightNext),
        Just(select::Event::HighlightPrev),
        Just(select::Event::Clear),
        Just(select::Event::Blur),
        prop::bool::ANY.prop_map(|is_keyboard| select::Event::Focus { is_keyboard }),
    ]
}
```

### 2.3 Key invariants to verify

| Component | Invariant                                                               |
| --------- | ----------------------------------------------------------------------- |
| All       | `transition()` never panics                                             |
| All       | `connect()` produces valid `role` attribute                             |
| All       | `drain_queue` never truncates                                           |
| Toggle    | `aria-pressed` is always `"true"` or `"false"`                          |
| Checkbox  | `aria-checked` is always `"true"`, `"false"`, or `"mixed"`              |
| Dialog    | When state is `Open`, `aria-modal` is `"true"`                          |
| Select    | `aria-expanded` matches state (`Open` → `"true"`, `Closed` → `"false"`) |
| Accordion | At least one item is open when `collapsible == false`                   |
| Tabs      | Exactly one tab has `aria-selected="true"`                              |

### 2.4 Component-Specific State Machine Invariants

Property-based tests (using `proptest` or `quickcheck`) MUST verify that state machine invariants hold under arbitrary event sequences. Each component MUST define its invariants and test them:

| Component   | Invariant                                                      | Test Property                                                   |
| ----------- | -------------------------------------------------------------- | --------------------------------------------------------------- |
| Checkbox    | `checked` ∈ {Checked, Unchecked, Indeterminate}                | After any event sequence, `ctx.checked` is a valid enum variant |
| Slider      | `min <= value <= max`                                          | After any event, `ctx.value` satisfies bounds                   |
| RangeSlider | `min <= start_value <= end_value <= max`                       | After any event, both values satisfy ordering                   |
| Select      | `highlighted_key` ∈ items ∪ {None}                             | Highlighted key always references an existing item or is None   |
| Combobox    | `highlighted_key` ∈ filtered_items ∪ {None}                    | Highlight is always within filtered results                     |
| Dialog      | If `State::Open`, focus is trapped within dialog content       | No focusable element outside dialog receives focus while open   |
| NumberInput | `value` matches `precision` (decimal places ≤ `ctx.precision`) | After any event, value has correct decimal precision            |
| DateField   | Each segment value is within calendar bounds                   | Month 1-12 (or 1-13 Hebrew), day within month range             |
| Table       | `sort_descriptor` column exists in column definitions          | Sort column always references a defined column                  |

**Example Test Pattern**:

```rust
use proptest::prelude::*;

fn arb_checkbox_event() -> impl Strategy<Value = CheckboxEvent> {
    prop_oneof![
        Just(CheckboxEvent::Toggle),
        Just(CheckboxEvent::SetChecked(Checked)),
        Just(CheckboxEvent::SetChecked(Unchecked)),
        Just(CheckboxEvent::SetChecked(Indeterminate)),
        Just(CheckboxEvent::SetDisabled(true)),
        Just(CheckboxEvent::SetDisabled(false)),
    ]
}

proptest! {
    #[test]
    fn checkbox_invariant_holds(events in prop::collection::vec(arb_checkbox_event(), 0..100)) {
        let mut svc = Service::new(checkbox::Props::default(), Env::default(), Default::default());
        for event in events {
            svc.send(event);
            // Invariant: checked is always a valid state
            assert!(matches!(svc.context().checked, Checked | Unchecked | Indeterminate));
        }
    }
}
```

Each component's test module MUST include at least one `proptest!` block verifying its invariants against 1000+ random event sequences.

### 2.5 Property-Based Testing Summary

1. All machines must pass a random-event-sequence fuzz test: 1000 random valid events must not cause panics.
2. Invalid event/state combinations must be handled gracefully (ignored or logged, never panic).
3. Use `proptest` crate for generating random `Bindable` values and event sequences.
4. Fuzz tests run in CI on nightly builds.

---

## 3. Fuzz Testing

Fuzz testing complements property-based testing by exploring event sequences that a human or property generator might not consider. Use `cargo-fuzz` with `libfuzzer`.

### 3.1 Setup

```toml
# fuzz/Cargo.toml
[package]
name = "ars-fuzz"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
libfuzzer-sys = "0.4"
ars-core = { path = "../crates/ars-core" }
arbitrary = { version = "1", features = ["derive"] }
```

### 3.2 Fuzz target example

```rust
// fuzz/fuzz_targets/fuzz_dialog.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;

#[derive(Debug, Arbitrary)]
enum FuzzEvent {
    Open,
    Close,
    Toggle,
    CloseOnEscape,
    CloseOnBackdropClick,
    RegisterTitle,
    RegisterDescription,
}

fn to_event(fe: &FuzzEvent) -> dialog::Event {
    match fe {
        FuzzEvent::Open => dialog::Event::Open,
        FuzzEvent::Close => dialog::Event::Close,
        FuzzEvent::Toggle => dialog::Event::Toggle,
        FuzzEvent::CloseOnEscape => dialog::Event::CloseOnEscape,
        FuzzEvent::CloseOnBackdropClick => dialog::Event::CloseOnBackdropClick,
        FuzzEvent::RegisterTitle => dialog::Event::RegisterTitle,
        FuzzEvent::RegisterDescription => dialog::Event::RegisterDescription,
    }
}

fuzz_target!(|data: Vec<FuzzEvent>| {
    let props = dialog::Props { id: "fuzz".into(), ..Default::default() };
    let mut svc = Service::new(props, Env::default(), Default::default());
    for fe in &data {
        let result = svc.send(to_event(fe));
        assert!(!result.truncated, "drain_queue overflow");
        // Verify component-specific invariants after EACH fuzz iteration,
        // not just at the end. This catches transient invariant violations
        // that self-heal by the end of the sequence.
        verify_invariants(&svc);
    }
    // Post-sequence: verify connect() doesn't panic
    let api = svc.connect(&|_| {});
    let _ = api.root_attrs();
    let _ = api.content_attrs();
});

/// Component-specific invariant checker. Each component MUST define its own
/// `verify_invariants()` that asserts structural correctness after every event.
///
/// Example invariants by component:
/// - **Select/Combobox**: `highlighted_key` exists in the item collection or is `None`.
/// - **Accordion**: when `collapsible == false`, at least one panel is expanded.
/// - **Tabs**: exactly one tab has `selected == true`.
/// - **TreeView**: `focused_key` is not a disabled item.
/// - **TreeView**: expanded items do not exceed `max_expanded` (if set).
/// - **Slider**: `min <= value <= max`.
/// - **Dialog**: if state is `Open`, `ctx.open == true`.
fn verify_invariants(svc: &Service<dialog::Machine>) {
    let state = svc.state();
    let ctx = svc.context();

    // Dialog invariant: state ↔ context sync
    match state {
        dialog::State::Open => {
            assert!(ctx.open, "Dialog State::Open but ctx.open is false");
        }
        dialog::State::Closed => {
            assert!(!ctx.open, "Dialog State::Closed but ctx.open is true");
        }
        _ => {}
    }

    // Dialog invariant: title_id and description_id are non-empty when registered
    // (registration events were sent)
}
```

### 3.3 Running fuzz tests

```bash
cargo +nightly fuzz run fuzz_dialog -- -max_total_time=300
```

### 3.4 What to fuzz

Every P0 and P1 component should have a fuzz target:

- Dialog, Popover, Menu, Select, Combobox, Tabs, Accordion
- Toast (especially after the state machine overhaul)
- DateField, Calendar (complex transition logic)

---

## 4. Disabled State Guard Matrix

Comprehensive verification that disabled components reject all interactions and emit correct ARIA/HTML attributes.

### 4.1 Per-Component Disabled Template

```rust
use ars_core::{Service, Machine, KeyboardKey, date::DateSegmentKind};

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
                        "disabled guard: {} context must not change on event {:?}",
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

test_disabled_guard!(button_disabled, button, vec![
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

test_disabled_guard!(dialog_disabled, dialog, vec![
    dialog::Event::Open,
    dialog::Event::Close,
]);

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

## 5. State Machine Completeness

These tests verify that every (State, Event) pair has a defined outcome, that context stays in sync after transitions, and that timer race conditions are handled.

### 5.1 State×Event Matrix Template

````rust
/// Components must provide exhaustive variant lists for property-based testing.
/// These can be derived via `#[derive(AllVariants)]` from `ars-derive`, or
/// manually implemented:
///
/// ```rust
/// impl toggle::Event {
///     pub const ALL_VARIANTS: &[Self] = &[
///         Self::Toggle,
///         Self::Focus,
///         Self::Blur,
///     ];
/// }
/// ```

#[test]
fn state_event_matrix_completeness() {
    let all_states = State::ALL_VARIANTS;
    let all_events = Event::ALL_VARIANTS;

    let mut results: Vec<(State, Event, Option<State>)> = Vec::new();

    for state in &all_states {
        for event in &all_events {
            let props = Props::default();
            let (_, ctx) = Machine::init(&props, &Env::default(), &Default::default());
            let plan = Machine::transition(state, event, &ctx, &props);
            let next_state = plan.as_ref().map(|p| p.target.clone());
            results.push((state.clone(), event.clone(), next_state));
        }
    }

    // Every (state, event) pair must be accounted for in the matrix.
    // None means "no transition" (event ignored in this state) — which is valid.
    assert_eq!(results.len(), all_states.len() * all_events.len());

    // Print matrix for documentation
    for (state, event, next) in &results {
        let outcome = match next {
            Some(s) => format!("→ {s:?}"),
            None => "— (ignored)".to_string(),
        };
        println!("{state:?} × {event:?} {outcome}");
    }
}
````

### 5.2 Context Sync Verification

```rust
#[test]
fn context_matches_state_after_every_transition() {
    let props = Props::default();
    let mut svc = Service::<Machine>::new(props);

    for event in Event::ALL_VARIANTS {
        let state_before = svc.state().clone();
        svc.send(event.clone());
        let state_after = svc.state();
        let ctx = svc.context();

        // Component-specific invariant: context fields must agree with state.
        // Example for Select:
        //   State::Open  → ctx.open == true
        //   State::Closed → ctx.open == false
        match state_after {
            State::Open => assert!(ctx.open, "Open state but ctx.open is false"),
            State::Closed => assert!(!ctx.open, "Closed state but ctx.open is true"),
            _ => {}
        }
    }
}
```

### 5.3 Example: Select State×Event Matrix

| State  | Event         | Expected                  |
| ------ | ------------- | ------------------------- |
| Closed | Open          | → Open                    |
| Closed | Close         | — (ignored)               |
| Closed | Toggle        | → Open                    |
| Open   | Open          | — (ignored)               |
| Open   | Close         | → Closed                  |
| Open   | Toggle        | → Closed                  |
| Open   | SelectItem    | → Closed (with value set) |
| Open   | HighlightNext | → Open (highlight moves)  |
| Open   | Escape        | → Closed                  |

## 6. Form Machine Correctness

These tests verify the `form_submit::Machine` interactions, specifically the requirement that async validators ALWAYS run even when sync validation fails (to show all errors at once).

> **Foundation reference:** See [07-forms.md](../foundation/07-forms.md) §8 (`form_submit::Machine`) and lines 2138-2143 for the async+sync merge requirement.
>
> **Canonical executable example:** See [11-form-validation.md §6.1](11-form-validation.md#61-async-validators-run-even-when-sync-fails).
> That section owns the full Service-based scenario and assertions.
>
> **State-machine relevance:** The requirement still belongs in this checklist because
> `form_submit::Machine` must remain in `Validating` until async validation completes,
> even when sync validation has already failed. Use the canonical form-validation test
> as the executable example for this behavior.

```rust
use ars_core::{Service, Machine};

#[test]
fn form_submit_idle_to_validating_to_submitting() {
    let props = form_submit::Props {
        id: "test-submit".into(),
        validation_mode: ValidationMode::default(),
        spawn_async_validation: Callback::new(|(validators, send)| {
            no_cleanup()
        }),
        schedule_microtask: Callback::new(|f| f()),
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    assert_eq!(*svc.state(), form_submit::State::Idle);

    svc.send(form_submit::Event::Submit);
    assert_eq!(*svc.state(), form_submit::State::Validating);

    svc.send(form_submit::Event::ValidationPassed);
    assert_eq!(*svc.state(), form_submit::State::Submitting);

    svc.send(form_submit::Event::SubmitComplete);
    assert_eq!(*svc.state(), form_submit::State::Succeeded);
}

#[test]
fn form_submit_reset_returns_to_idle() {
    let props = form_submit::Props {
        id: "test-submit".into(),
        validation_mode: ValidationMode::default(),
        spawn_async_validation: Callback::new(|(validators, send)| {
            no_cleanup()
        }),
        schedule_microtask: Callback::new(|f| f()),
    };
    let mut svc = Service::new(props, Env::default(), Default::default());
    svc.send(form_submit::Event::Submit);
    svc.send(form_submit::Event::ValidationFailed);
    assert_eq!(*svc.state(), form_submit::State::ValidationFailed);

    svc.send(form_submit::Event::Reset);
    assert_eq!(*svc.state(), form_submit::State::Idle);
}
```
