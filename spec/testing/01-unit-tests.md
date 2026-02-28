# Unit Tests

> **Note:** Test examples assume component `Props` types implement `Default`. Each component must provide its own `Default` impl for tests to compile.

## 1. Table-Driven Unit Tests for `transition()`

Every state machine must have exhaustive transition coverage via table-driven tests. The pattern eliminates boilerplate and makes it trivial to add new cases.

### 1.1 Pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Test case for table-driven transition tests.
    /// Note: This struct is not `Send` due to the `Box<dyn Fn>` field —
    /// construct test cases per-test, do not share across threads.
    struct TransitionCase {
        name: &'static str,
        initial_state: State,
        initial_ctx: Context,
        props: Props,                        // Use the `default_case()` helper to construct a base case, then override fields as needed
        event: Event,
        expected_state: Option<State>,       // None = no transition
        expected_ctx: Option<Box<dyn Fn(&Context)>>,  // assertion on context after apply (Box<dyn Fn> to allow capturing closures)
        expected_acknowledged: bool,         // default: false — distinguishes notification-only events from rejected transitions
    }

    // TransitionCase cannot derive Default (closure field `expected_ctx` is not Default).
    // Use the `default_case()` helper to construct cases with optional fields set to None/false:
    fn default_case(
        name: &'static str,
        state: State,
        event: Event,
    ) -> TransitionCase {
        TransitionCase {
            name,
            initial_state: state,
            initial_ctx: Context::default(),
            props: Props::default(),
            event,
            expected_state: None,
            expected_ctx: None,
            expected_acknowledged: false,
        }
    }

    fn default_ctx() -> Context {
        let props = Props::default();
        let (_state, ctx) = Machine::init(&props);
        ctx
    }

    #[test]
    fn transition_table() {
        let cases: Vec<TransitionCase> = vec![
            TransitionCase {
                name: "idle_focus_keyboard",
                initial_state: State::Idle,
                initial_ctx: default_ctx(),
                props: Props::default(),
                event: Event::Focus { is_keyboard: true },
                expected_state: Some(State::Focused),
                expected_ctx: Some(Box::new(|ctx: &Context| {
                    assert!(ctx.focused);
                    assert!(ctx.focus_visible);
                })),
                expected_acknowledged: false,
            },
            TransitionCase {
                name: "focused_blur_returns_idle",
                initial_state: State::Focused,
                initial_ctx: {
                    let mut ctx = default_ctx();
                    ctx.focused = true;
                    ctx
                },
                props: Props::default(),
                event: Event::Blur,
                expected_state: Some(State::Idle),
                expected_ctx: Some(Box::new(|ctx: &Context| {
                    assert!(!ctx.focused);
                })),
                expected_acknowledged: false,
            },
            // ... exhaustive cases for every (state, event) pair
        ];

        for case in &cases {
            let plan = Machine::transition(
                &case.initial_state,
                &case.event,
                &case.initial_ctx,
                &case.props,
            );

            match (&case.expected_state, plan) {
                (Some(expected), Some(plan)) => {
                    let target = plan.target.as_ref()
                        .unwrap_or_else(|| panic!("FAIL [{}]: plan should have a target state", case.name));
                    assert_eq!(
                        target, expected,
                        "FAIL [{}]: wrong target state",
                        case.name,
                    );
                    if let Some(ref assert_ctx) = case.expected_ctx {
                        let mut ctx = case.initial_ctx.clone();
                        if let Some(apply) = plan.apply {
                            apply(&mut ctx);
                        }
                        assert_ctx(&ctx);
                    }
                }
                (None, None) => {
                    // Notification-only events (e.g., Click on Button) return `None` but are
                    // semantically different from rejected transitions (e.g., guard failed).
                    // Use `expected_acknowledged` to distinguish: when true the event was
                    // processed as a notification-only action; when false the transition was
                    // genuinely rejected by a guard or unhandled match arm.
                    if case.expected_acknowledged {
                        // Event was acknowledged (notification-only like Click) — no state
                        // change expected, but the event was intentionally handled.
                    }
                    // else: expected no transition, got none — pass
                }
                (Some(expected), None) => {
                    panic!("FAIL [{}]: expected transition to {:?}, got None", case.name, expected);
                }
                (None, Some(plan)) => {
                    panic!("FAIL [{}]: expected None, got transition to {:?}", case.name, plan.target);
                }
            }
        }
    }
}
```

### 1.2 Coverage requirements

- Every `(State, Event)` combination must have at least one test case.
- Guard conditions (e.g. `ctx.disabled`, `ctx.selection_mode`) must have both true and false branches tested.
  When a guard depends on multiple context fields simultaneously, test each combination:

  ```rust
  // Example: compound guard — `ctx.disabled && !ctx.allow_while_disabled`
  TransitionCase {
      name: "disabled AND allow_while_disabled=false → blocks event",
      initial_state: State::Idle,
      initial_ctx: Context { disabled: true, allow_while_disabled: false, ..default() },
      props: Props::default(),
      event: Event::Press,
      expected_state: None, // guard rejects
      expected_ctx: None,
      expected_acknowledged: false,
  },
  TransitionCase {
      name: "disabled AND allow_while_disabled=true → permits event",
      initial_state: State::Idle,
      initial_ctx: Context { disabled: true, allow_while_disabled: true, ..default() },
      props: Props::default(),
      event: Event::Press,
      expected_state: Some(State::Pressed),
      expected_ctx: Some(Box::new(|ctx: &Context| assert!(ctx.pressed))),
      expected_acknowledged: false,
  },
  TransitionCase {
      name: "not disabled → permits event regardless of allow_while_disabled",
      initial_state: State::Idle,
      initial_ctx: Context { disabled: false, allow_while_disabled: false, ..default() },
      props: Props::default(),
      event: Event::Press,
      expected_state: Some(State::Pressed),
      expected_ctx: Some(Box::new(|ctx: &Context| assert!(ctx.pressed))),
      expected_acknowledged: false,
  },
  ```

- Context mutations must be explicitly asserted (not just state transitions).

### 1.3 Handling Large State x Event Matrices

Components with many states and events (e.g., Select, Combobox, DatePicker) produce large
transition tables. Use these strategies to keep tests manageable:

1. **Group by state**: Organize test cases into sub-vectors or separate test functions per
   state. This makes it easy to verify that every event is covered for a given state.

   ```rust
   fn idle_cases() -> Vec<TransitionCase> { vec![/* all (Idle, *) cases */] }
   fn open_cases() -> Vec<TransitionCase> { vec![/* all (Open, *) cases */] }
   fn selecting_cases() -> Vec<TransitionCase> { vec![/* all (Selecting, *) cases */] }

   #[test]
   fn transition_table() {
       let cases: Vec<TransitionCase> = [
           idle_cases(), open_cases(), selecting_cases(),
       ].into_iter().flatten().collect();
       run_transition_table(&cases);
   }
   ```

2. **Document ignored pairs**: When a `(State, Event)` pair is intentionally unhandled
   (returns `None`), include a test case with `expected_state: None` and a comment explaining
   why. This distinguishes "tested and unhandled" from "forgotten".

3. **Helper macros**: For components with many similar transitions, use a macro to reduce
   boilerplate:

   ```rust
   macro_rules! case {
       ($name:expr, $state:expr, $event:expr => $target:expr) => {
           TransitionCase {
               name: $name,
               initial_state: $state,
               initial_ctx: default_ctx(),
               props: Props::default(),
               event: $event,
               expected_state: Some($target),
               expected_ctx: None,
               expected_acknowledged: false,
           }
       };
   }
   ```

4. **Property-based testing with proptest**: For exhaustive coverage of large matrices,
   generate arbitrary `(State, Event)` pairs and assert that `transition()` never panics:

   ```rust
   proptest! {
       #[test]
       fn transition_never_panics(state in arb_state(), event in arb_event()) {
           let ctx = default_ctx();
           let props = Props::default();
           let _plan = Machine::transition(&state, &event, &ctx, &props);
       }
   }
   ```

### 1.4 State/Context synchronization invariant

When a context field mirrors the state machine state (e.g., `ctx.open` tracks whether the
component is in `State::Open`), tests MUST verify that both the state enum and the context field
agree after every transition. A common defect pattern is using `TransitionPlan::context_only()`
to set `ctx.open = true` without transitioning to `State::Open`, which causes state-guarded
handlers (e.g., `(State::Open, Event::Close)`) to become unreachable.

```rust
// BAD: context_only sets ctx.open but state stays Closed
TransitionCase {
    name: "focus with open_on_focus opens the dropdown",
    initial_state: State::Closed,
    initial_ctx: Context { open_on_focus: true, open: false, ..default() },
    props: Props::default(),
    event: Event::Focus { is_keyboard: true },
    expected_state: Some(State::Open), // MUST transition state, not just context
    expected_ctx: Some(Box::new(|ctx: &Context| assert!(ctx.open))),
    expected_acknowledged: false,
},

// Regression: verify Close/ClickOutside still work after open-on-focus
TransitionCase {
    name: "click outside closes dropdown opened via focus",
    initial_state: State::Open,
    initial_ctx: Context { open: true, ..default() },
    props: Props::default(),
    event: Event::ClickOutside,
    expected_state: Some(State::Closed),
    expected_ctx: Some(Box::new(|ctx: &Context| assert!(!ctx.open))),
    expected_acknowledged: false,
},
```

### 1.5 Props reference coverage

Every `Props` field that influences transition behavior MUST have at least one test that
verifies the field is actually read. A common defect is declaring a prop (e.g.,
`clamp_value_on_blur: bool`) without wiring it into the `transition()` function.

```rust
// Verify that clamp_value_on_blur=false disables clamping
TransitionCase {
    name: "blur with clamp_value_on_blur=false does not clamp",
    initial_state: State::Focused,
    initial_ctx: Context { value: Bindable::uncontrolled(150.0), min: 0.0, max: 100.0, ..default() },
    props: Props { clamp_value_on_blur: false, ..default() },
    event: Event::Blur,
    expected_state: Some(State::Idle),
    expected_ctx: Some(Box::new(|ctx: &Context| assert_eq!(*ctx.value.get(), 150.0))), // NOT clamped
    expected_acknowledged: false,
},
```

### 1.6 Disabled guard completeness

For every component whose `Context` has a `disabled: bool` field, tests MUST verify that all
value-mutating events are blocked when `disabled == true`. The test set should include
**every event variant** from the `Event` enum, asserting that:

- Value-mutating events return `None`
- Focus and Blur behavior when disabled is component-specific. Some components (e.g., Button with HTML `disabled`) remove from tab order entirely. Others (e.g., TextField with `aria-disabled`) remain focusable but do not respond to activation. Consult each component's spec for the exact disabled behavior. Tests should verify the component-specific contract, not assume a universal rule.

### 1.7 Scalability for Large State x Event Matrices

See section 1.3 for complete guidance on large state × event matrices, including proptest, macros, and partitioning strategies.

---

## 2. Layout Component Unit Tests

Concrete unit tests for layout components (Splitter, ScrollArea, Carousel, Stack, AspectRatio).

> **Note:** The functions below (`compute_resize`, `collapse_panel`, `expand_panel`, `compute_thumb_metrics`, `thumb_pos_to_scroll`) are pure helper functions defined in each component's module. They are not part of the core architecture API.

```rust
#[cfg(test)]
mod layout_tests {
    use super::*;
    use crate::splitter::*;
    use crate::scroll_area::*;
    use crate::carousel::*;
    use crate::stack::*;
    use crate::aspect_ratio::*;

    fn two_panels() -> Vec<PanelDef> {
        vec![
            PanelDef {
                id: "a".into(),
                min_size: 10.0,
                default_size: 50.0,
                ..PanelDef::default()
            },
            PanelDef {
                id: "b".into(),
                min_size: 10.0,
                default_size: 50.0,
                ..PanelDef::default()
            },
        ]
    }

    // ── Splitter ─────────────────────────────────────────────────────────

    #[test]
    fn resize_respects_min_size() {
        let panels = two_panels();
        let result = compute_resize(&[50.0, 50.0], 0, 200.0, &panels);
        assert_eq!(result[1], 10.0, "panel b at min");
        assert_eq!(result[0], 90.0, "panel a gets remainder");
    }

    #[test]
    fn resize_negative_delta_respects_left_min() {
        let panels = two_panels();
        let result = compute_resize(&[50.0, 50.0], 0, -200.0, &panels);
        assert_eq!(result[0], 10.0, "panel a at min");
        assert_eq!(result[1], 90.0, "panel b gets remainder");
    }

    #[test]
    fn collapse_and_expand_round_trip() {
        let panels = vec![
            PanelDef {
                id: "a".into(),
                collapsible: true,
                min_size: 20.0,
                default_size: 40.0,
                ..PanelDef::default()
            },
            PanelDef {
                id: "b".into(),
                min_size: 20.0,
                default_size: 60.0,
                ..PanelDef::default()
            },
        ];
        let mut sizes = vec![40.0, 60.0];
        collapse_panel(&mut sizes, 0, &panels);
        assert_eq!(sizes[0], 0.0, "collapsed to 0");
        assert_eq!(sizes[1], 100.0, "b absorbed freed space");

        expand_panel(&mut sizes, 0, &panels);
        assert_eq!(sizes[0], 40.0, "restored to default_size");
        assert_eq!(sizes[1], 60.0, "b gave back space");
    }

    #[test]
    fn max_size_constraint_is_respected() {
        let panels = vec![
            PanelDef {
                id: "a".into(),
                min_size: 10.0,
                max_size: Some(60.0),
                default_size: 50.0,
                ..PanelDef::default()
            },
            PanelDef {
                id: "b".into(),
                min_size: 10.0,
                default_size: 50.0,
                ..PanelDef::default()
            },
        ];
        let result = compute_resize(&[50.0, 50.0], 0, 30.0, &panels);
        assert_eq!(result[0], 60.0, "panel a at max_size");
        assert_eq!(result[1], 40.0, "panel b reduced");
    }

    // ── ScrollArea ───────────────────────────────────────────────────────

    #[test]
    fn thumb_at_start_of_track() {
        let (size, pos) = compute_thumb_metrics(500.0, 1000.0, 0.0, 500.0, 20.0);
        assert!((size - 250.0).abs() < 0.01, "thumb = half track");
        assert!((pos - 0.0).abs() < 0.01, "thumb at top");
    }

    #[test]
    fn thumb_at_end_of_track() {
        let (size, pos) = compute_thumb_metrics(500.0, 1000.0, 500.0, 500.0, 20.0);
        assert!((size - 250.0).abs() < 0.01);
        assert!((pos - 250.0).abs() < 0.01, "thumb at bottom: {pos}");
    }

    #[test]
    fn thumb_at_middle_of_track() {
        let (size, pos) = compute_thumb_metrics(500.0, 1000.0, 250.0, 500.0, 20.0);
        assert!((size - 250.0).abs() < 0.01);
        assert!((pos - 125.0).abs() < 0.01, "thumb at midpoint: {pos}");
    }

    #[test]
    fn min_thumb_size_applied() {
        // Very small viewport relative to content.
        let (size, _) = compute_thumb_metrics(10.0, 10000.0, 0.0, 500.0, 30.0);
        assert!(size >= 30.0, "thumb must be at least min_thumb_size");
    }

    #[test]
    fn no_overflow_fills_track() {
        let (size, pos) = compute_thumb_metrics(500.0, 400.0, 0.0, 500.0, 20.0);
        assert_eq!(size, 500.0, "no overflow: thumb fills track");
        assert_eq!(pos, 0.0);
    }

    #[test]
    fn thumb_pos_to_scroll_round_trips() {
        let viewport = 500.0;
        let content  = 1000.0;
        let track    = 500.0;
        let min_thumb = 20.0;
        let scroll_pos = 200.0;

        let (thumb_size, thumb_pos) =
            compute_thumb_metrics(viewport, content, scroll_pos, track, min_thumb);
        let recovered = thumb_pos_to_scroll(thumb_pos, track, thumb_size, content, viewport);
        assert!((recovered - scroll_pos).abs() < 0.01, "round-trip: {recovered}");
    }

    // ── Carousel ─────────────────────────────────────────────────────────

    fn carousel_ctx(index: usize, count: usize, loop_nav: bool) -> Context {
        let props = carousel::Props::new("test-carousel");
        let (_state, mut ctx) = carousel::Machine::init(&props);
        ctx.index = Bindable::uncontrolled(index);
        ctx.slide_count = NonZero::new(count).expect("count must be non-zero");
        ctx.loop_nav = loop_nav;
        ctx
    }

    #[test]
    fn carousel_loop_prev_wraps() {
        let ctx = carousel_ctx(0, 3, true);
        assert_eq!(ctx.clamp_index(-1), 2);
    }

    #[test]
    fn carousel_loop_next_wraps() {
        let ctx = carousel_ctx(2, 3, true);
        assert_eq!(ctx.clamp_index(3), 0);
    }

    #[test]
    fn carousel_no_loop_clamps_at_boundaries() {
        let ctx = carousel_ctx(0, 3, false);
        assert_eq!(ctx.clamp_index(-1), 0, "cannot go before 0");
        let ctx2 = carousel_ctx(2, 3, false);
        assert_eq!(ctx2.clamp_index(3), 2, "cannot go past last");
    }

    #[test]
    fn carousel_can_go_prev_no_loop() {
        assert!(!carousel_ctx(0, 3, false).can_go_prev());
        assert!(carousel_ctx(1, 3, false).can_go_prev());
    }

    #[test]
    fn carousel_can_go_next_no_loop() {
        assert!(carousel_ctx(0, 3, false).can_go_next());
        assert!(!carousel_ctx(2, 3, false).can_go_next());
    }

    // ── Stack ─────────────────────────────────────────────────────────────

    #[test]
    fn stack_direction_ltr_logical() {
        assert_eq!(
            StackDirection::RowLogical.resolve(false),
            StackDirection::Row
        );
    }

    #[test]
    fn stack_direction_rtl_logical() {
        assert_eq!(
            StackDirection::RowLogical.resolve(true),
            StackDirection::RowReverse
        );
    }

    #[test]
    fn stack_direction_physical_unchanged_in_rtl() {
        assert_eq!(
            StackDirection::Column.resolve(true),
            StackDirection::Column
        );
    }

    #[test]
    fn stack_style_contains_flex() {
        let props = Props {
            direction: StackDirection::Column,
            ..Default::default()
        };
        let style = props.to_style(false, None);
        assert!(style.contains("display: flex"));
        assert!(style.contains("flex-direction: column"));
    }

    // ── AspectRatio ───────────────────────────────────────────────────────

    #[test]
    fn aspect_ratio_16_9() {
        let props = Props { ratio: 16.0 / 9.0 };
        let pct = props.padding_top_percent();
        assert!((pct - 56.25).abs() < 0.01, "16:9 = 56.25%: {pct}");
    }

    #[test]
    fn aspect_ratio_square() {
        let props = Props { ratio: 1.0 };
        assert!((props.padding_top_percent() - 100.0).abs() < 0.01);
    }

    #[test]
    fn aspect_ratio_portrait() {
        let props = Props { ratio: 2.0 / 3.0 };
        let pct = props.padding_top_percent();
        assert!((pct - 150.0).abs() < 0.01, "2:3 = 150%: {pct}");
    }
}
```

## 3. Invalid Props and Boundary Testing

State machines MUST handle invalid or edge-case props gracefully. `Machine::init()` and
`Machine::transition()` must NEVER panic, regardless of prop values.

### 3.1 Boundary Value Table

| Prop Type         | Invalid / Edge Values                        | Expected Behavior                                 |
| ----------------- | -------------------------------------------- | ------------------------------------------------- |
| `id: String`      | `""` (empty string)                          | Machine initializes; adapter may log warning      |
| `min / max`       | `min > max`, `min == max`, `NaN`, `Infinity` | Clamp or normalize; no panic                      |
| `value`           | Out of `[min, max]` range                    | Clamp to range on blur (if `clamp_value_on_blur`) |
| `items: Vec<T>`   | Empty `vec![]`                               | Machine initializes in idle; no selection         |
| `default_value`   | Value not in items list                      | Ignore; start with no selection                   |
| Conflicting props | `disabled=true` + `auto_focus=true`          | `disabled` wins; no focus                         |
| `step`            | `0`, negative, `NaN`                         | Treat as `1` (default); no panic                  |

### 3.2 Property Fuzzing with `proptest`

```rust
use proptest::prelude::*;

/// Generate arbitrary Props values to verify no panics.
impl Arbitrary for slider::Props {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_: ()) -> Self::Strategy {
        (
            any::<f64>(),           // min
            any::<f64>(),           // max
            any::<Option<f64>>(),   // value
            any::<f64>(),           // step
            any::<bool>(),          // disabled
            ".*",                   // id (arbitrary string)
        ).prop_map(|(min, max, value, step, disabled, id)| {
            slider::Props { min, max, value, step, disabled, id, ..Default::default() }
        }).boxed()
    }
}

proptest! {
    /// Machine::init() must never panic for any Props combination.
    #[test]
    fn init_never_panics(props in any::<slider::Props>()) {
        let (_state, _ctx) = slider::Machine::init(&props);
    }

    /// Machine::transition() must never panic for any (state, event, props) combination.
    #[test]
    fn transition_never_panics(
        props in any::<slider::Props>(),
        event in arb_slider_event(),
    ) {
        let (state, ctx) = slider::Machine::init(&props);
        let _plan = slider::Machine::transition(&state, &event, &ctx, &props);
    }
}
```

---

## 4. Floating Point Precision Tests

Slider step clamping and color conversion round-trips must not accumulate floating point errors.

### 4.1 Slider Step Clamping Precision

```rust
use proptest::prelude::*;

proptest! {
    /// Slider value must always be an exact multiple of the step, relative to min.
    #[test]
    fn slider_drag_does_not_accumulate_error(
        min in -1000.0f64..0.0,
        max in 1.0f64..1000.0,
        step in 0.01f64..10.0,
        drags in prop::collection::vec(0.0f64..1.0, 1..50),
    ) {
        let props = slider::Props {
            min,
            max,
            step,
            ..Default::default()
        };
        let mut svc = Service::<slider::Machine>::new(props);

        for fraction in drags {
            let raw_value = min + fraction * (max - min);
            svc.send(slider::Event::PointerMove { value: raw_value });
            let value = *svc.context().value.get();

            // Value must be clamped to [min, max]
            assert!(value >= min && value <= max, "value {} out of range [{}, {}]", value, min, max);

            // Value must be an exact step multiple relative to min
            let steps_from_min = (value - min) / step;
            let rounded_steps = steps_from_min.round();
            let expected = min + rounded_steps * step;
            let expected_clamped = expected.min(max).max(min);
            assert!(
                (value - expected_clamped).abs() < f64::EPSILON * 100.0,
                "value {} is not an exact step multiple (expected {}), step={}, min={}",
                value, expected_clamped, step, min,
            );
        }
    }
}
```

### 4.2 Accumulated Error with Small Steps

```rust
#[test]
fn slider_small_steps_no_drift() {
    let props = slider::Props {
        min: 0.0,
        max: 1.0,
        step: 0.1,
        ..Default::default()
    };
    let mut svc = Service::<slider::Machine>::new(props);

    // Increment 10 times by step
    for i in 1..=10 {
        svc.send(slider::Event::Increment);
        let value = *svc.context().value.get();
        let expected = (i as f64) * 0.1;
        assert!(
            (value - expected).abs() < f64::EPSILON * 10.0,
            "After {} increments: got {}, expected {}",
            i, value, expected,
        );
    }
    // Final value must be exactly max
    assert_eq!(*svc.context().value.get(), 1.0);
}
```

### 4.3 Color Conversion Round-Trip Accuracy

```rust
use proptest::prelude::*;

proptest! {
    /// RGB → HSL → RGB round-trip must not drift more than ±1 per channel (0–255).
    #[test]
    fn color_rgb_hsl_roundtrip(
        r in 0u8..=255,
        g in 0u8..=255,
        b in 0u8..=255,
    ) {
        let original = Rgb { r, g, b };
        let hsl = original.to_hsl();
        let roundtrip = hsl.to_rgb();

        assert!(
            (roundtrip.r as i16 - original.r as i16).abs() <= 1
            && (roundtrip.g as i16 - original.g as i16).abs() <= 1
            && (roundtrip.b as i16 - original.b as i16).abs() <= 1,
            "RGB round-trip drift too large: {:?} → {:?} → {:?}",
            original, hsl, roundtrip,
        );
    }

    /// RGB → HSV → RGB round-trip accuracy.
    #[test]
    fn color_rgb_hsv_roundtrip(
        r in 0u8..=255,
        g in 0u8..=255,
        b in 0u8..=255,
    ) {
        let original = Rgb { r, g, b };
        let hsv = original.to_hsv();
        let roundtrip = hsv.to_rgb();

        assert!(
            (roundtrip.r as i16 - original.r as i16).abs() <= 1
            && (roundtrip.g as i16 - original.g as i16).abs() <= 1
            && (roundtrip.b as i16 - original.b as i16).abs() <= 1,
            "RGB→HSV round-trip drift too large: {:?} → {:?} → {:?}",
            original, hsv, roundtrip,
        );
    }
}
```

---

## 5. Controlled Component Test Template

Every `Bindable` prop must have tests verifying:

1. `on_change` callback fires with correct value on user interaction.
2. Updating the controlling signal causes the machine context to reflect the new value on next render.
3. Stale prop values (prop changes between event dispatch and processing) do not cause state/context mismatch — the latest prop value wins.

---
