---
adapter: dioxus
component: timer
category: specialized
source: components/specialized/timer.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Timer — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Timer`](../../components/specialized/timer.md) contract onto Dioxus `0.7.x`. The adapter preserves countdown and stopwatch behavior, action triggers, formatted display, optional progress reporting, and timer-effect ownership across supported runtimes.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct TimerProps {
    pub mode: TimerMode,
    #[props(optional)]
    pub duration_ms: Option<u64>,
    #[props(optional)]
    pub auto_start: Option<bool>,
    #[props(optional)]
    pub tick_interval_ms: Option<u32>,
    #[props(optional)]
    pub label: Option<String>,
    #[props(optional)]
    pub show_progress: Option<bool>,
}

#[component]
pub fn Timer(props: TimerProps) -> Element
```

`StartTrigger`, `PauseTrigger`, `ResetTrigger`, `Display`, and optional `Progress` are all rendered by the adapter-owned surface.

## 3. Mapping to Core Component Contract

- Props parity: full parity with mode, duration, auto-start, tick cadence, and labeling.
- Part parity: full parity with `Root`, `Label`, `Display`, `Progress`, `StartTrigger`, `PauseTrigger`, `ResetTrigger`, and `Separator`.
- Adapter additions: explicit interval ownership, visibility cleanup, and SSR idle policy.

## 4. Part Mapping

| Core part / structure | Required?         | Adapter rendering target | Ownership     | Attr source                      | Notes                                   |
| --------------------- | ----------------- | ------------------------ | ------------- | -------------------------------- | --------------------------------------- |
| `Root`                | required          | `<div>`                  | adapter-owned | `api.root_attrs()`               | `role="timer"` live region root.        |
| `Label`               | optional          | `<label>`                | adapter-owned | `api.label_attrs()`              | Render when `label` exists.             |
| `Display`             | required          | `<span>`                 | adapter-owned | `api.display_attrs()`            | Visible formatted time.                 |
| `Progress`            | optional          | `<div>`                  | adapter-owned | `api.progress_attrs()`           | Render when `show_progress=true`.       |
| action triggers       | required          | `<button>`               | adapter-owned | `api.start_trigger_attrs()` etc. | Buttons stay separate structural parts. |
| `Separator`           | optional repeated | `<span>`                 | adapter-owned | `api.separator_attrs()`          | Decorative time delimiters only.        |

## 5. Attr Merge and Ownership Rules

| Target node     | Core attrs                                 | Adapter-owned attrs             | Consumer attrs   | Merge order                              | Ownership notes                      |
| --------------- | ------------------------------------------ | ------------------------------- | ---------------- | ---------------------------------------- | ------------------------------------ |
| `Root`          | timer role, live-region attrs, scope, part | none beyond wrapper class merge | decoration attrs | timer semantics win                      | root remains semantic                |
| action triggers | labels, disabled markers                   | click handlers                  | decoration only  | accessibility and disabled semantics win | trigger ownership stays adapter-side |
| `Progress`      | progressbar attrs                          | none                            | decoration only  | valuenow/min/max win                     | omit entirely when disabled by props |

## 6. Composition / Context Contract

`Timer` is context-free. The adapter does not publish timer controls through separate context because all public parts are rendered within the single component surface.

## 7. Prop Sync and Event Mapping

| Adapter prop                                            | Mode       | Sync trigger | Machine event / update path | Visible effect                            | Notes                                |
| ------------------------------------------------------- | ---------- | ------------ | --------------------------- | ----------------------------------------- | ------------------------------------ |
| `mode`, `duration_ms`, `tick_interval_ms`, `auto_start` | controlled | rerender     | prop rebuild                | changes timing rules and initial behavior | timer effect must read latest values |

| UI event              | Preconditions             | Machine event / callback path | Ordering notes                                     | Notes                                         |
| --------------------- | ------------------------- | ----------------------------- | -------------------------------------------------- | --------------------------------------------- |
| start or resume click | timer not already running | `Start` or `Resume`           | timer effect begins after transition               | no timer starts during SSR                    |
| pause click           | running                   | `Pause`                       | clear active interval before leaving running state | display freezes at current value              |
| reset click           | any non-idle state        | `Reset`                       | clear active interval before resetting display     | countdown returns to original duration        |
| interval tick         | running                   | `Tick`                        | generated by platform timer effect                 | completion transition may cancel the interval |

## 8. Registration and Cleanup Contract

| Registered entity    | Registration trigger | Identity key       | Cleanup trigger                                   | Cleanup action | Notes                       |
| -------------------- | -------------------- | ------------------ | ------------------------------------------------- | -------------- | --------------------------- |
| active tick interval | enter running state  | component instance | pause, reset, completion, prop change, or cleanup | clear interval | at most one active interval |

## 9. Ref and Node Contract

No live node refs are required. The component is timer-driven, not measurement-driven.

## 10. State Machine Boundary Rules

- machine-owned state: idle, running, paused, and completed timing semantics.
- adapter-local derived bookkeeping: active interval handle only.
- forbidden local mirrors: do not mirror elapsed or remaining time outside the machine.

## 11. Callback Payload Contract

No dedicated public callback is required by this adapter surface.

## 12. Failure and Degradation Rules

| Condition                       | Policy             | Notes                                                                   |
| ------------------------------- | ------------------ | ----------------------------------------------------------------------- |
| timer API unavailable           | degrade gracefully | render the static display and controls without automatic ticking        |
| runtime throttling delays ticks | degrade gracefully | recompute from machine time source rather than trusting delayed cadence |

## 13. Identity and Key Policy

Timer identity is the component instance. Decorative separators do not own independent identity beyond stable render order.

## 14. SSR and Client Boundary Rules

- SSR renders the full idle structure with the initial formatted display.
- Tick intervals are client-only.
- Hydration must preserve the same initial display before any auto-start logic runs.

## 15. Performance Constraints

- Keep only one interval active.
- Avoid rerendering more structure than the changing display and progress parts.
- Cancel intervals promptly when the timer leaves running state.

## 16. Implementation Dependencies

| Dependency         | Required? | Dependency type | Why it must exist first                                         | Notes                   |
| ------------------ | --------- | --------------- | --------------------------------------------------------------- | ----------------------- |
| timer helper       | required  | platform helper | owns interval creation and cleanup                              | adapter-owned effect    |
| duration formatter | required  | helper          | display and live-region wording depend on consistent formatting | shared with status text |

## 17. Recommended Implementation Sequence

1. Render the semantic root, display, and action triggers.
2. Wire start, pause, and reset events into the machine.
3. Add interval ownership and cleanup.
4. Add optional progress rendering and completion announcement.

## 18. Anti-Patterns

- Do not keep the interval alive while paused or completed.
- Do not derive display text from an unsynchronized local clock.
- Do not start intervals during SSR.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the display stays mounted across state changes.
- Consumers may assume only one running interval exists.
- Consumers must not assume background-runtime tick cadence is exact.

## 20. Platform Support Matrix

| Capability / behavior      | Web          | Desktop      | Mobile       | SSR          | Notes                                     |
| -------------------------- | ------------ | ------------ | ------------ | ------------ | ----------------------------------------- |
| timer display and controls | full support | full support | full support | full support | SSR renders initial state only.           |
| running interval           | full support | full support | full support | client-only  | interval starts after hydration or mount. |
| progress bar               | full support | full support | full support | full support | static on SSR, reactive on client.        |

## 21. Debug Diagnostics and Production Policy

| Condition                          | Debug build behavior | Production behavior | Notes                     |
| ---------------------------------- | -------------------- | ------------------- | ------------------------- |
| timer helper unavailable           | debug warning        | degrade gracefully  | preserve static rendering |
| multiple active intervals detected | fail fast            | fail fast           | timer ownership bug       |

## 22. Shared Adapter Helper Notes

| Helper concept     | Required? | Responsibility                         | Reused by            | Notes                           |
| ------------------ | --------- | -------------------------------------- | -------------------- | ------------------------------- |
| timer helper       | required  | starts and clears the running interval | timer-backed widgets | one interval per instance       |
| duration formatter | required  | formats display and announcement text  | `timer` only         | keep locale behavior consistent |

## 23. Framework-Specific Behavior

Dioxus should keep the interval handle in instance-local state or effects and cancel it eagerly on state changes so no background task survives disposal.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct TimerSketchProps {
    pub mode: TimerMode,
}

#[component]
pub fn Timer(props: TimerSketchProps) -> Element {
    let machine = use_machine::<timer::Machine>(timer::Props { mode: props.mode, ..Default::default() });
    let root_attrs = machine.derive(|api| api.root_attrs());
    rsx! { div { ..root_attrs.read().clone() } }
}
```

## 25. Reference Implementation Skeleton

- Initialize the machine with the chosen mode and duration.
- Render the permanent display and action triggers.
- Start and clear the interval from state-driven effects only.
- Derive progress and completion text from the machine snapshot.

## 26. Adapter Invariants

- At most one interval is active at a time.
- Reset and pause always clear the active interval first.
- SSR never starts background timing work.

## 27. Accessibility and SSR Notes

The root `role="timer"` live region is authoritative. The visible display should remain `aria-hidden` if the root already announces the formatted time.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- countdown start, pause, resume, and reset
- stopwatch start and reset
- auto-start after hydration
- completion transition clears the interval
- progress bar updates with countdown mode

## 30. Test Oracle Notes

| Behavior             | Preferred oracle type | Notes                                               |
| -------------------- | --------------------- | --------------------------------------------------- |
| ticking behavior     | fake timer            | drive the interval deterministically                |
| interval cleanup     | cleanup side effects  | assert no more ticks after pause, reset, or unmount |
| accessibility output | DOM attrs             | assert timer and progress semantics                 |

## 31. Implementation Checklist

- [ ] Interval ownership is explicit and singular.
- [ ] Pause, reset, and completion all cancel the interval.
- [ ] SSR renders idle structure without starting timing work.
