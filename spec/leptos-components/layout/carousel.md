---
adapter: leptos
component: carousel
category: layout
source: components/layout/carousel.md
source_foundation: foundation/08-adapter-leptos.md
---

# Carousel — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Carousel`](../../components/layout/carousel.md) contract onto Leptos `0.8.x` compound components. The adapter preserves slide navigation, drag or swipe interaction, autoplay, indicators, and accessible live-region behavior while defining registration, timer ownership, and viewport refs.

## 2. Public Adapter API

```rust,no_check
pub mod carousel {
    #[component]
    pub fn Carousel(
        #[prop(optional, into)] index: Option<Signal<usize>>,
        #[prop(optional)] default_index: usize,
        slide_count: usize,
        #[prop(optional)] loop_nav: bool,
        #[prop(optional)] auto_play: Option<AutoPlayOptions>,
        #[prop(optional)] slides_per_view: f64,
        #[prop(optional)] slides_per_move: usize,
        #[prop(optional)] spacing: f64,
        #[prop(optional)] orientation: Orientation,
        #[prop(optional)] align: SlideAlignment,
        #[prop(optional)] swipe_threshold: f64,
        children: Children,
    ) -> impl IntoView

    #[component] pub fn Viewport(children: Children) -> impl IntoView
    #[component] pub fn ItemGroup(children: Children) -> impl IntoView
    #[component] pub fn Item(index: usize, children: Children) -> impl IntoView
    #[component] pub fn PrevTrigger(children: Children) -> impl IntoView
    #[component] pub fn NextTrigger(children: Children) -> impl IntoView
    #[component] pub fn IndicatorGroup(children: Children) -> impl IntoView
    #[component] pub fn Indicator(index: usize, children: Children) -> impl IntoView
    #[component] pub fn AutoPlayTrigger(children: Children) -> impl IntoView
    #[component] pub fn AutoPlayIndicator() -> impl IntoView
    #[component] pub fn ProgressText() -> impl IntoView
}
```

`Carousel` owns the machine, autoplay timer, and registration contract. `slide_count` must agree with rendered `Item` and `Indicator` parts.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core carousel props, including controlled index, autoplay, spacing, orientation, alignment, and drag threshold.
- State parity: full parity with the core `Idle`, `AutoPlaying`, and `Transitioning` states.
- Part parity: full parity with all documented carousel parts.
- Adapter additions: explicit timer lifecycle, viewport and item-group refs, and item or indicator registration rules.

## 4. Part Mapping

| Core part / structure                                    | Required?   | Adapter rendering target     | Ownership     | Attr source                     | Notes                                                            |
| -------------------------------------------------------- | ----------- | ---------------------------- | ------------- | ------------------------------- | ---------------------------------------------------------------- |
| `Root`                                                   | required    | `<section>`                  | adapter-owned | `api.root_attrs()`              | Owns context, timers, and overall state attrs.                   |
| `Viewport`                                               | required    | `<div>`                      | adapter-owned | `api.viewport_attrs()`          | Drag or swipe interaction starts here.                           |
| `ItemGroup`                                              | required    | `<div>`                      | adapter-owned | `api.item_group_attrs()`        | Live-region policy is derived here.                              |
| each `Item`                                              | required    | `<div>`                      | adapter-owned | `api.item_attrs(index)`         | Hidden items stay in structure but become inert and aria-hidden. |
| triggers, indicator parts, autoplay parts, progress text | conditional | buttons and structural nodes | adapter-owned | corresponding core attr methods | Each control maps directly to a documented core part.            |

## 5. Attr Merge and Ownership Rules

| Target node             | Core attrs                                                            | Adapter-owned attrs                                            | Consumer attrs                      | Merge order                                             | Ownership notes                    |
| ----------------------- | --------------------------------------------------------------------- | -------------------------------------------------------------- | ----------------------------------- | ------------------------------------------------------- | ---------------------------------- |
| `Root`                  | `api.root_attrs()` including region semantics, state, and orientation | keydown, hover, and focus pause hooks plus context publication | wrapper decoration attrs            | root region semantics and state attrs win               | root stays adapter-owned           |
| `Viewport`              | `api.viewport_attrs()` including overflow and touch-action styles     | drag listeners and live ref                                    | viewport decoration attrs           | required clip and touch-action behavior win             | viewport remains adapter-owned     |
| `Item`                  | `api.item_attrs(index)` including role, labeling, and inertness       | registration metadata                                          | item decoration attrs               | role, hidden state, and inertness win                   | item remains adapter-owned         |
| triggers and indicators | corresponding core attrs                                              | click handlers and focus coordination                          | decoration attrs or visual children | required labels, pressed state, and selection attrs win | control parts remain adapter-owned |

## 6. Composition / Context Contract

- `Carousel` provides required carousel context containing machine access, autoplay helpers, and item or indicator registration.
- All descendant parts consume required context and fail fast when rendered outside `carousel::Carousel`.
- No optional external context is required beyond higher-level locale or direction derivation before root props are built.

## 7. Prop Sync and Event Mapping

| Adapter prop               | Mode       | Sync trigger                         | Machine event / update path                                                        | Visible effect                                                   | Notes                                                        |
| -------------------------- | ---------- | ------------------------------------ | ---------------------------------------------------------------------------------- | ---------------------------------------------------------------- | ------------------------------------------------------------ |
| `index`                    | controlled | signal change after mount            | `GoToSlide { index }`                                                              | updates active item, indicator selection, and live-region policy | controlled/uncontrolled switching is unsupported after mount |
| `auto_play`                | controlled | rerender and timer lifecycle changes | `AutoPlayStart`, `AutoPlayPause`, `AutoPlayResume`, `AutoPlayStop`, `AutoPlayTick` | starts or stops autoplay and affects live-region mode            | timer ownership is adapter-local                             |
| orientation and drag props | controlled | rerender with new props              | core prop update                                                                   | changes drag axis, key mapping, and transition behavior          | affects gesture math only                                    |

| UI event                        | Preconditions        | Machine event / callback path                              | Ordering notes                                                   | Notes                                               |
| ------------------------------- | -------------------- | ---------------------------------------------------------- | ---------------------------------------------------------------- | --------------------------------------------------- |
| prev or next trigger activation | navigation available | `GoToPrev` or `GoToNext`                                   | transition starts before wrapper callbacks observe the new slide | disabled state must block activation                |
| indicator activation            | indicator exists     | `GoToSlide { index }`                                      | indicator selection updates with the same transition path        | indicator count must match slides                   |
| viewport pointer drag           | viewport mounted     | `PointerDown`, `PointerMove`, `PointerUp`, `PointerCancel` | adapter computes axis-relative gesture values before dispatch    | drag must not bypass machine-owned transition rules |
| root hover or focus transitions | autoplay configured  | `AutoPlayPause` or `AutoPlayResume`                        | pause happens before live-region attrs are read                  | follows the core autoplay rules                     |

## 8. Registration and Cleanup Contract

| Registered entity               | Registration trigger                   | Identity key             | Cleanup trigger                            | Cleanup action                                     | Notes                                   |
| ------------------------------- | -------------------------------------- | ------------------------ | ------------------------------------------ | -------------------------------------------------- | --------------------------------------- |
| carousel context                | `Root` mount                           | instance-derived         | `Root` cleanup                             | drop provided context and live refs                | one context per carousel                |
| autoplay timer                  | autoplay becomes active                | instance-derived root id | pause, stop, transition change, or cleanup | cancel the active timer                            | at most one autoplay timer per instance |
| viewport drag listeners         | pointer drag start                     | instance-derived root id | drag end or cleanup                        | release drag listeners and transient gesture state | cleanup must run on cancel too          |
| item and indicator registration | each `Item` or `Indicator` render path | composite                | part cleanup                               | remove stale registrations and recompute order     | counts must align with `slide_count`    |

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability                  | Composition rule        | Notes                                                         |
| ------------------ | ------------- | ------------- | ---------------------------------- | ----------------------- | ------------------------------------------------------------- |
| `Root`             | no            | adapter-owned | always structural, handle optional | no composition required | State lives in the machine.                                   |
| `Viewport`         | yes           | adapter-owned | required after mount               | no composition required | Drag and focus-pause behavior depend on a live viewport node. |
| `ItemGroup`        | yes           | adapter-owned | required after mount               | no composition required | Transition and transform application need a live group node.  |
| each `Item`        | no            | adapter-owned | always structural, handle optional | no composition required | Registration uses indices rather than imperative item refs.   |

## 10. State Machine Boundary Rules

- machine-owned state: current index, autoplay status, transition status, drag state, and accessibility-facing labels or aria-live mode.
- adapter-local derived bookkeeping: timer handle, drag listener cleanup, live viewport and item-group refs, and registration order.
- forbidden local mirrors: do not track a second active index outside the machine and controlled prop path.
- allowed snapshot-read contexts: trigger handlers, drag gestures, timer callbacks, and cleanup.

## 11. Callback Payload Contract

| Callback                                        | Payload source           | Payload shape                            | Timing                                          | Cancelable? | Notes                        |
| ----------------------------------------------- | ------------------------ | ---------------------------------------- | ----------------------------------------------- | ----------- | ---------------------------- |
| slide change callback when exposed by a wrapper | machine-derived snapshot | `{ index: usize, user_initiated: bool }` | after the machine resolves the new active slide | no          | Wrapper-owned callback only. |

## 12. Failure and Degradation Rules

| Condition                                                     | Policy             | Notes                                                               |
| ------------------------------------------------------------- | ------------------ | ------------------------------------------------------------------- |
| any compound part rendered outside `carousel::Carousel`       | fail fast          | Required carousel context is missing.                               |
| viewport or item-group ref unavailable after mount            | fail fast          | Drag and transition ownership cannot work correctly.                |
| rendered item or indicator count disagrees with `slide_count` | warn and ignore    | Preserve the last stable registration set and surface the mismatch. |
| timer APIs unavailable                                        | degrade gracefully | Render the carousel with manual navigation only.                    |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                 | Notes                                  |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | --------------------------------------- | -------------------------------------- |
| carousel root                    | instance-derived | not applicable      | not applicable                           | root structure must stay stable         | one carousel context per root          |
| items and indicators             | composite        | no                  | yes                                      | server order must match hydration order | indices determine active slide mapping |
| autoplay timer                   | instance-derived | no                  | not applicable                           | timer belongs to one carousel instance  | never keep more than one live timer    |

## 14. SSR and Client Boundary Rules

- SSR renders the full carousel structure for the initial slide and autoplay state without starting timers or drag listeners.
- Timers, drag listeners, hover pause, and focus pause are client-only.
- Hydration must preserve item and indicator order exactly so registration matches the server-rendered active index.

## 15. Performance Constraints

- Keep at most one autoplay timer active per instance.
- Reuse live viewport and item-group refs rather than rebuilding them on every render.
- Avoid re-registering every item when only the active index changes.

## 16. Implementation Dependencies

| Dependency              | Required? | Dependency type | Why it must exist first                                         | Notes                                  |
| ----------------------- | --------- | --------------- | --------------------------------------------------------------- | -------------------------------------- |
| ordered registry helper | required  | shared helper   | Keeps rendered items and indicators aligned with `slide_count`. | Shared with toolbar and splitter.      |
| gesture helper          | required  | shared helper   | Normalizes drag or swipe coordinates and velocity.              | Shared with drag-driven widgets.       |
| timer helper            | required  | shared helper   | Owns autoplay scheduling and cleanup.                           | Must tolerate pause and resume cycles. |

## 17. Recommended Implementation Sequence

1. Initialize `Root`, the carousel machine, and required context.
2. Register items and indicators in stable order.
3. Capture `Viewport` and `ItemGroup` refs.
4. Implement trigger and indicator navigation.
5. Add drag gestures and autoplay timer ownership.

## 18. Anti-Patterns

- Do not keep a second active-slide index outside the machine.
- Do not expose hidden slides to assistive technology while marking them inactive visually.
- Do not leave autoplay timers running after pause, stop, or cleanup.

## 19. Consumer Expectations and Guarantees

- Consumers may assume hidden slides remain structurally present but inert and aria-hidden.
- Consumers may assume autoplay pauses according to the documented focus and hover rules.
- Consumers must not assume mismatched slide counts will be silently corrected.

## 20. Platform Support Matrix

| Capability / behavior                             | Browser client | SSR           | Notes                                                                    |
| ------------------------------------------------- | -------------- | ------------- | ------------------------------------------------------------------------ |
| triggers, indicators, drag gestures, and autoplay | full support   | fallback path | SSR renders structure only; timers and drag listeners start after mount. |

## 21. Debug Diagnostics and Production Policy

| Condition                                      | Debug build behavior | Production behavior | Notes                                                         |
| ---------------------------------------------- | -------------------- | ------------------- | ------------------------------------------------------------- |
| required carousel context missing              | fail fast            | fail fast           | Compound parts must be nested under `Root`.                   |
| viewport or item-group ref missing after mount | fail fast            | fail fast           | Drag and transition ownership cannot recover safely.          |
| item or indicator count mismatch               | debug warning        | warn and ignore     | The adapter should preserve the last stable registration set. |

## 22. Shared Adapter Helper Notes

| Helper concept          | Required? | Responsibility                                             | Reused by                        | Notes                              |
| ----------------------- | --------- | ---------------------------------------------------------- | -------------------------------- | ---------------------------------- |
| ordered registry helper | required  | Keeps item and indicator order aligned with `slide_count`. | `toolbar`, `splitter`            | Supports deterministic navigation. |
| gesture helper          | required  | Normalizes drag deltas and velocity.                       | drag-driven widgets              | Avoids ad hoc axis math.           |
| timer helper            | required  | Owns autoplay timer lifecycle.                             | widgets with delayed transitions | Must cancel on cleanup.            |

## 23. Framework-Specific Behavior

Leptos can keep carousel context, item registration, and live refs in component-owned state while timers and drag listeners live in client effects. Machine-derived attrs stay reactive through `derive(...)`.

## 24. Canonical Implementation Sketch

```rust
pub mod carousel {
    #[component]
    pub fn Carousel(slide_count: usize, children: Children) -> impl IntoView {
        let machine = use_machine::<carousel::Machine>(carousel::Props { slide_count, ..Default::default() });
        provide_context(Context::from_machine(machine));
        view! { <section>{children()}</section> }
    }
}
```

## 25. Reference Implementation Skeleton

```rust
pub mod carousel {
    #[component]
    pub fn Carousel(slide_count: usize, children: Children) -> impl IntoView {
        let machine = use_machine::<carousel::Machine>(carousel::Props { slide_count, ..Default::default() });
        let viewport_ref = NodeRef::<html::Div>::new();
        let item_group_ref = NodeRef::<html::Div>::new();
        provide_context(Context::new(machine, viewport_ref, item_group_ref));
        view! { <section {..machine.derive(|api| api.root_attrs()).get()}>{children()}</section> }
    }
}
```

## 26. Adapter Invariants

- `Carousel` owns exactly one ordered item and indicator registration set.
- Hidden slides remain inert and aria-hidden while the active slide stays perceivable.
- Autoplay timer ownership is single-instance and always cleaned up.

## 27. Accessibility and SSR Notes

- `Root` must preserve region labeling and `aria-roledescription="carousel"`.
- `ItemGroup` live-region mode must follow autoplay state so announcements remain helpful rather than noisy.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core `Carousel` contract.
- Intentional deviations: none.
- Traceability note: adapter-owned timers, drag cleanup, registration order, and hidden-slide accessibility rules are promoted into explicit Leptos requirements.

## 29. Test Scenarios

- Navigate with previous or next triggers and verify active slide and indicator attrs.
- Drag the viewport and verify slide changes plus drag cleanup.
- Start, pause, resume, and stop autoplay and verify live-region mode plus timer cleanup.
- Render mismatched slide or indicator counts and verify warning behavior.
- Render a part outside `carousel::Carousel` and verify failure behavior.

## 30. Test Oracle Notes

- Root, slide, and indicator semantics: prefer `DOM attrs`.
- Active index changes: prefer `machine state`.
- Timer ownership and cleanup: prefer `cleanup side effects`.
- Ordered registration: prefer `context registration`.

## 31. Implementation Checklist

- [ ] Publish one required carousel context from `Root`.
- [ ] Keep item and indicator registrations aligned with `slide_count`.
- [ ] Capture live `Viewport` and `ItemGroup` refs before enabling drag or autoplay logic.
- [ ] Own exactly one autoplay timer per carousel instance and clean it up.
- [ ] Fail fast when required descendants or live nodes are missing.
