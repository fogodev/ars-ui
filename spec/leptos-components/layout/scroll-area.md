---
adapter: leptos
component: scroll-area
category: layout
source: components/layout/scroll-area.md
source_foundation: foundation/08-adapter-leptos.md
---

# ScrollArea — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ScrollArea`](../../components/layout/scroll-area.md) contract onto Leptos `0.8.x` compound components. The adapter preserves native viewport scrolling, custom decorative scrollbars, thumb dragging, track click paging, visibility modes, and RTL normalization while defining observer ownership, refs, and cleanup.

## 2. Public Adapter API

```rust,no_check
pub mod scroll_area {
    #[component]
    pub fn ScrollArea(
        #[prop(optional)] orientation: ScrollOrientation,
        #[prop(optional)] scrollbar_visibility: ScrollbarVisibility,
        #[prop(optional)] hide_delay_ms: u32,
        #[prop(optional)] min_thumb_size: f64,
        #[prop(optional)] dir: Direction,
        #[prop(optional)] aria_label: Option<String>,
        children: Children,
    ) -> impl IntoView

    #[component] pub fn Viewport(children: Children) -> impl IntoView
    #[component] pub fn Content(children: Children) -> impl IntoView
    #[component] pub fn ScrollbarY(children: Children) -> impl IntoView
    #[component] pub fn ThumbY() -> impl IntoView
    #[component] pub fn ScrollbarX(children: Children) -> impl IntoView
    #[component] pub fn ThumbX() -> impl IntoView
    #[component] pub fn CornerSquare() -> impl IntoView
}
```

`ScrollArea` owns the machine, measurement pipeline, and descendant context. `Viewport` remains the native scrolling node.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core scroll-area props, including orientation, visibility mode, thumb sizing, direction, and accessible labeling.
- State parity: full parity with the core `Idle`, `Hovering`, `ScrollActive`, and `ThumbDragging` states.
- Part parity: full parity with `Root`, `Viewport`, `Content`, both scrollbars, both thumbs, and `CornerSquare`.
- Adapter additions: explicit ref ownership, observer lifecycles, drag cleanup, and native-scroll fallback rules.

## 4. Part Mapping

| Core part / structure       | Required?   | Adapter rendering target | Ownership     | Attr source                                           | Notes                                    |
| --------------------------- | ----------- | ------------------------ | ------------- | ----------------------------------------------------- | ---------------------------------------- |
| `Root`                      | required    | `<div>`                  | adapter-owned | `api.root_attrs()`                                    | Owns state attrs and descendant context. |
| `Viewport`                  | required    | `<div>`                  | adapter-owned | `api.viewport_attrs()`                                | Native scrolling node and focus target.  |
| `Content`                   | required    | `<div>`                  | adapter-owned | `api.content_attrs()`                                 | Inner measurement target.                |
| `ScrollbarY` / `ScrollbarX` | conditional | `<div>`                  | adapter-owned | `api.scrollbar_y_attrs()` / `api.scrollbar_x_attrs()` | Decorative tracks only.                  |
| `ThumbY` / `ThumbX`         | conditional | `<div>`                  | adapter-owned | `api.thumb_y_attrs()` / `api.thumb_x_attrs()`         | Decorative thumbs driven by measurement. |
| `CornerSquare`              | conditional | `<div>`                  | adapter-owned | `api.corner_square_attrs()`                           | Visible only when both axes are active.  |

## 5. Attr Merge and Ownership Rules

| Target node       | Core attrs                                                          | Adapter-owned attrs                                       | Consumer attrs                        | Merge order                                        | Ownership notes                        |
| ----------------- | ------------------------------------------------------------------- | --------------------------------------------------------- | ------------------------------------- | -------------------------------------------------- | -------------------------------------- |
| `Root`            | `api.root_attrs()` including state, overflow, and direction markers | pointer-enter and leave handlers plus context publication | wrapper decoration attrs when exposed | core state and overflow attrs win                  | root stays adapter-owned               |
| `Viewport`        | `api.viewport_attrs()` including role, label, and tabindex          | scroll listener and measurement refs                      | viewport decoration attrs             | role, label, tabindex, and scrolling semantics win | viewport remains adapter-owned         |
| tracks and thumbs | corresponding core attrs                                            | drag, pointer, and measurement handlers                   | decoration attrs only                 | decorative semantics and visibility attrs win      | tracks and thumbs remain adapter-owned |

- Consumer styling may restyle tracks and thumbs but must not remove `role="none"` or viewport accessibility attrs.

## 6. Composition / Context Contract

- `ScrollArea` provides required scroll-area context containing machine access, node refs, and measurement helpers.
- All other parts consume required context and fail fast when rendered outside `scroll_area::ScrollArea`.
- No optional external context is required beyond `dir` fallback derivation that may happen before `Root` props are built.

## 7. Prop Sync and Event Mapping

| Adapter prop                | Mode       | Sync trigger            | Machine event / update path | Visible effect                                 | Notes                                       |
| --------------------------- | ---------- | ----------------------- | --------------------------- | ---------------------------------------------- | ------------------------------------------- |
| visibility and sizing props | controlled | rerender with new props | core prop update            | changes scrollbar visibility and thumb metrics | no controlled/uncontrolled switching issues |
| `dir`                       | controlled | rerender with new props | core prop update            | changes RTL normalization and track placement  | affects horizontal math only                |

| UI event                         | Preconditions                          | Machine event / callback path                     | Ordering notes                                                         | Notes                                        |
| -------------------------------- | -------------------------------------- | ------------------------------------------------- | ---------------------------------------------------------------------- | -------------------------------------------- |
| viewport scroll                  | viewport mounted                       | `Scroll { x, y }`                                 | measurement reads must happen after the browser updates scroll offsets | native keyboard scroll remains authoritative |
| viewport or content resize       | refs available                         | `Resize { ... }`                                  | adapter batches geometry reads before machine update                   | uses observer-backed measurement             |
| track click                      | track visible                          | `TrackClick { pos, axis }`                        | page scroll dispatch happens before thumb metrics rerender             | no focus should move to decorative tracks    |
| thumb pointer down or move or up | thumb visible and matching axis active | `ThumbDragStart`, `ThumbDragMove`, `ThumbDragEnd` | drag listeners and cleanup are adapter-owned                           | thumbs remain decorative to assistive tech   |

## 8. Registration and Cleanup Contract

| Registered entity              | Registration trigger                | Identity key             | Cleanup trigger                      | Cleanup action                                    | Notes                                   |
| ------------------------------ | ----------------------------------- | ------------------------ | ------------------------------------ | ------------------------------------------------- | --------------------------------------- |
| scroll-area context            | `Root` mount                        | instance-derived         | `Root` cleanup                       | drop provided context and refs                    | one context per root                    |
| viewport and content observers | both refs available                 | instance-derived root id | ref loss or cleanup                  | disconnect observers                              | observer ownership is adapter-local     |
| hide-delay timer               | transition into `ScrollActive` mode | instance-derived root id | timer fire, state change, or cleanup | cancel the outstanding timer                      | at most one hide timer per instance     |
| drag session listeners         | thumb drag start                    | instance-derived root id | drag end or cleanup                  | release listeners and pointer capture bookkeeping | cleanup must run even on pointer cancel |

## 9. Ref and Node Contract

| Target part / node        | Ref required? | Ref owner     | Node availability                  | Composition rule                            | Notes                                               |
| ------------------------- | ------------- | ------------- | ---------------------------------- | ------------------------------------------- | --------------------------------------------------- |
| `Root`                    | no            | adapter-owned | always structural, handle optional | no composition required                     | Root does not need imperative reads.                |
| `Viewport`                | yes           | adapter-owned | required after mount               | no consumer composition in the base surface | Native scroll reads depend on a live viewport node. |
| `Content`                 | yes           | adapter-owned | required after mount               | no consumer composition in the base surface | Content size reads depend on a live node.           |
| visible tracks and thumbs | yes           | adapter-owned | required after mount               | no composition required                     | Drag geometry requires live track or thumb handles. |

## 10. State Machine Boundary Rules

- machine-owned state: scroll position, viewport and content geometry, visibility mode state, thumb-drag state, and direction-aware overflow flags.
- adapter-local derived bookkeeping: observer handles, active timer handle, pointer capture state, and raw RTL scroll normalization helpers.
- forbidden local mirrors: do not mirror scroll offsets or visibility state outside the machine.
- allowed snapshot-read contexts: scroll handlers, resize observers, drag handlers, and cleanup.

## 11. Callback Payload Contract

| Callback                                         | Payload source           | Payload shape                                                        | Timing                               | Cancelable? | Notes                        |
| ------------------------------------------------ | ------------------------ | -------------------------------------------------------------------- | ------------------------------------ | ----------- | ---------------------------- |
| scroll change callback when exposed by a wrapper | machine-derived snapshot | `{ scroll_x: f64, scroll_y: f64, progress_x: f64, progress_y: f64 }` | after the machine processes `Scroll` | no          | Wrapper-owned callback only. |

## 12. Failure and Degradation Rules

| Condition                                                         | Policy             | Notes                                                                                                    |
| ----------------------------------------------------------------- | ------------------ | -------------------------------------------------------------------------------------------------------- |
| any compound part rendered outside `scroll_area::ScrollArea`      | fail fast          | Required context is missing.                                                                             |
| viewport or content ref unavailable after mount                   | fail fast          | Measurement and scroll sync cannot work without live nodes.                                              |
| observer APIs unavailable                                         | degrade gracefully | Keep native scrolling and labels, but fall back to less reactive scrollbar visibility and thumb updates. |
| conflicting wrapper attrs remove viewport accessibility semantics | warn and ignore    | `role`, label, and tabindex must remain intact.                                                          |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                          | Notes                                  |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | ------------------------------------------------ | -------------------------------------- |
| root scroll area                 | instance-derived | not applicable      | not applicable                           | the root and viewport structure must stay stable | one measurement pipeline per root      |
| timer and observer resources     | composite        | no                  | not applicable                           | resources belong to one instance at a time       | cleanup must not leak across rerenders |

## 14. SSR and Client Boundary Rules

- SSR renders the full part structure for the chosen orientation and visibility mode but performs no geometry reads.
- Scroll listeners, observers, drag listeners, timers, and RTL scroll normalization are client-only.
- Hydration must preserve root, viewport, content, and conditional scrollbar structure derived from the initial props.

## 15. Performance Constraints

- Batch geometry reads and machine updates to avoid redundant resize and scroll churn.
- Keep at most one hide timer active per instance.
- Do not rebuild observers or drag listeners unless the relevant nodes change.

## 16. Implementation Dependencies

| Dependency                      | Required? | Dependency type         | Why it must exist first                                                     | Notes                                        |
| ------------------------------- | --------- | ----------------------- | --------------------------------------------------------------------------- | -------------------------------------------- |
| measurement helper              | required  | shared helper           | Computes thumb metrics and reads viewport or content geometry consistently. | Shared with carousel and splitter.           |
| observer helper                 | required  | shared helper           | Centralizes `ResizeObserver` ownership and cleanup.                         | Must be instance-scoped.                     |
| RTL scroll normalization helper | required  | behavioral prerequisite | Keeps horizontal scroll math consistent across browsers.                    | Used only for horizontal or both-axis cases. |

## 17. Recommended Implementation Sequence

1. Initialize `Root` and publish the scroll-area context.
2. Wire `Viewport` and `Content` refs and observer setup.
3. Implement machine-driven root and viewport attrs.
4. Add track and thumb rendering plus drag handlers.
5. Add hide-delay timing and graceful observer fallback behavior.

## 18. Anti-Patterns

- Do not replace native viewport scrolling with custom transform-based scrolling.
- Do not expose decorative scrollbars to assistive technology.
- Do not keep duplicate scroll position state outside the machine.

## 19. Consumer Expectations and Guarantees

- Consumers may assume keyboard scrolling remains native on the viewport.
- Consumers may assume custom scrollbars are decorative mirrors of the native scroll position.
- Consumers must not assume track or thumb nodes are focusable or semantic controls.

## 20. Platform Support Matrix

| Capability / behavior                             | Browser client | SSR           | Notes                                                                     |
| ------------------------------------------------- | -------------- | ------------- | ------------------------------------------------------------------------- |
| native viewport plus custom decorative scrollbars | full support   | fallback path | SSR renders structure only; measurement and drag logic start after mount. |

## 21. Debug Diagnostics and Production Policy

| Condition                                    | Debug build behavior | Production behavior | Notes                                        |
| -------------------------------------------- | -------------------- | ------------------- | -------------------------------------------- |
| required scroll-area context missing         | fail fast            | fail fast           | Compound parts must be nested correctly.     |
| viewport or content node missing after mount | fail fast            | fail fast           | Measurement pipeline cannot function safely. |
| observer APIs unavailable                    | debug warning        | degrade gracefully  | Native scrolling still works.                |

## 22. Shared Adapter Helper Notes

| Helper concept           | Required? | Responsibility                                 | Reused by                 | Notes                                   |
| ------------------------ | --------- | ---------------------------------------------- | ------------------------- | --------------------------------------- |
| measurement helper       | required  | Computes track and thumb metrics and progress. | `splitter`, `carousel`    | Avoids duplicated geometry math.        |
| observer helper          | required  | Owns resize observer setup and cleanup.        | measurement-heavy widgets | Must disconnect on cleanup.             |
| RTL normalization helper | required  | Normalizes horizontal scroll offsets in RTL.   | horizontal scrollers      | Browser differences stay isolated here. |

## 23. Framework-Specific Behavior

Leptos can compose `NodeRef` ownership for viewport, content, tracks, and thumbs while keeping machine-derived attrs reactive. Observer and timer setup belong in client effects and cleanup hooks.

## 24. Canonical Implementation Sketch

```rust
pub mod scroll_area {
    #[component]
    pub fn ScrollArea(children: Children) -> impl IntoView {
        let machine = use_machine::<scroll_area::Machine>(scroll_area::Props::default());
        provide_context(Context::from_machine(machine));
        view! { <div>{children()}</div> }
    }
}
```

## 25. Reference Implementation Skeleton

```rust
pub mod scroll_area {
    #[component]
    pub fn ScrollArea(children: Children) -> impl IntoView {
        let machine = use_machine::<scroll_area::Machine>(scroll_area::Props::default());
        let viewport_ref = NodeRef::<html::Div>::new();
        let content_ref = NodeRef::<html::Div>::new();
        provide_context(Context::new(machine, viewport_ref, content_ref));
        view! { <div {..machine.derive(|api| api.root_attrs()).get()}>{children()}</div> }
    }
}
```

## 26. Adapter Invariants

- `Viewport` always remains the native scrolling node.
- Decorative scrollbars never replace native keyboard scroll behavior or viewport labeling.
- Observers, timers, and drag listeners are always cleaned up with the instance.

## 27. Accessibility and SSR Notes

- `Viewport` keeps `role="region"`, an accessible label, and `tabindex="0"` even when custom scrollbars are hidden.
- Tracks and thumbs remain `role="none"` so assistive technology stays focused on the native viewport.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core `ScrollArea` contract.
- Intentional deviations: none.
- Traceability note: adapter-owned observers, timers, drag cleanup, and RTL normalization are promoted into explicit Leptos rules.

## 29. Test Scenarios

- Scroll the viewport and verify scrollbar visibility, thumb position, and progress values.
- Resize viewport and content and verify recalculated thumb metrics.
- Drag a thumb and verify native scroll position updates plus cleanup on pointer release.
- Verify RTL horizontal normalization and left-side track placement.
- Render a part outside `scroll_area::ScrollArea` and verify failure behavior.

## 30. Test Oracle Notes

- Viewport and decorative-part attrs: prefer `DOM attrs`.
- Scroll and drag state: prefer `machine state`.
- Ordered setup and cleanup of observers or timers: prefer `cleanup side effects`.
- Compound registration: prefer `context registration`.

## 31. Implementation Checklist

- [ ] Publish one required scroll-area context from `Root`.
- [ ] Keep `Viewport` as the native scrolling node and preserve its accessibility attrs.
- [ ] Wire observers, drag listeners, and hide-delay timers with explicit cleanup.
- [ ] Normalize horizontal RTL scroll math.
- [ ] Fail fast when required descendants or live nodes are missing.
