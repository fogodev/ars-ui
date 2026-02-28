---
adapter: leptos
component: marquee
category: data-display
source: components/data-display/marquee.md
source_foundation: foundation/08-adapter-leptos.md
---

# Marquee — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Marquee`](../../components/data-display/marquee.md) contract onto a Leptos 0.8.x component. The adapter must preserve the play/pause machine, duplicate content for seamless looping, honor reduced-motion auto-pause, and make the pause-control contract explicit.

## 2. Public Adapter API

```rust
#[component]
pub fn Marquee(
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] speed: Option<f64>,
    #[prop(optional)] direction: Option<marquee::Direction>,
    #[prop(optional)] gap: Option<f64>,
    #[prop(optional)] pause_on_hover: bool,
    #[prop(optional)] pause_on_focus: bool,
    #[prop(optional)] loop_count: Option<usize>,
    #[prop(optional)] auto_fill: bool,
    #[prop(optional)] delay: Option<f64>,
    #[prop(optional)] auto_play: bool,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] locale: Option<Locale>,
    #[prop(optional)] messages: Option<marquee::Messages>,
    #[prop(optional)] on_pause_change: Option<Callback<bool>>,
    #[prop(optional)] on_loop_complete: Option<Callback<usize>>,
    #[prop(optional)] on_complete: Option<Callback<()>>,
    children: Children,
) -> impl IntoView
```

The adapter always owns content duplication and the pause/play control. Consumer children provide the logical content only once.

## 3. Mapping to Core Component Contract

- Props parity: full parity with explicit adapter callbacks.
- Part parity: full parity for `Root`, duplicated `Content`, `Edge`, and `AutoPlayTrigger`.
- Traceability note: this spec promotes duplicated content ownership, reduced-motion auto-pause, hover/focus pause policy, loop callbacks, and accessibility control requirements from the agnostic spec.

## 4. Part Mapping

| Core part         | Required?            | Adapter rendering target | Ownership     | Attr source                     | Notes                                                |
| ----------------- | -------------------- | ------------------------ | ------------- | ------------------------------- | ---------------------------------------------------- |
| `Root`            | required             | `<div>`                  | adapter-owned | `api.root_attrs()`              | Viewport container and live-region policy host.      |
| `Content`         | required, duplicated | `<div>`                  | adapter-owned | `api.content_attrs()`           | Render original content plus at least one duplicate. |
| `Edge`            | optional             | `<div>`                  | adapter-owned | `api.edge_attrs(side)`          | Decorative gradient overlays.                        |
| `AutoPlayTrigger` | required             | `<button>`               | adapter-owned | `api.auto_play_trigger_attrs()` | Explicit pause/play control.                         |

## 5. Attr Merge and Ownership Rules

- Core attrs include root state, `aria-live`, `aria-disabled`, direction-related custom properties, and trigger pressed state.
- The adapter owns duplicated content wrappers, `aria-hidden="true"` on decorative duplicates, and the trigger semantics.
- Consumer children decorate only the logical content; they do not own duplicate wrappers or edge overlays.

## 6. Composition / Context Contract

`Marquee` is standalone. It does not publish context. It may consume reduced-motion preference helpers but must keep the resulting pause semantics visible in adapter sections rather than hidden in a generic helper.

## 7. Prop Sync and Event Mapping

| Adapter prop / event    | Mode           | Sync trigger                   | Machine event / update path | Notes                                                  |
| ----------------------- | -------------- | ------------------------------ | --------------------------- | ------------------------------------------------------ |
| trigger press           | adapter event  | button click                   | `Play` / `Pause`            | Emits `on_pause_change` after state transition.        |
| hover enter/leave       | adapter event  | pointer interaction            | `HoverIn` / `HoverOut`      | Active only when `pause_on_hover=true`.                |
| focus enter/leave       | adapter event  | focus within root              | `FocusIn` / `FocusOut`      | Active only when `pause_on_focus=true`.                |
| loop completion         | adapter effect | animation or observer boundary | `LoopComplete`              | Emits `on_loop_complete`; may also emit `on_complete`. |
| reduced motion detected | adapter effect | mount or preference change     | `Pause`                     | Auto-pauses without preventing later manual play.      |

## 8. Registration and Cleanup Contract

- The adapter may register animationend, scroll-timeline, or measurement observers for loop completion and auto-fill.
- If `auto_fill=true`, the adapter may measure viewport and content width after mount and whenever relevant layout inputs change.
- Cleanup must cancel observers, media-query listeners, and stale completion callbacks.

## 9. Ref and Node Contract

The root and content nodes require live refs for measurement, duplicate sizing, and optional loop-completion detection. These refs are adapter-owned and are not replaceable with ID strings alone.

## 10. State Machine Boundary Rules

- Machine-owned state: playing vs paused, loop counter, disabled state, and pause source flags.
- Adapter-owned derived values: duplicate count, measured content extent, and paused play-state CSS custom property.
- Forbidden mirror: do not track a second paused state outside the machine.

## 11. Callback Payload Contract

| Callback           | Payload source            | Payload shape | Timing                                          | Cancelable? | Notes                                |
| ------------------ | ------------------------- | ------------- | ----------------------------------------------- | ----------- | ------------------------------------ |
| `on_pause_change`  | adapter observation       | `bool`        | after state changes                             | no          | `true` means paused.                 |
| `on_loop_complete` | `LoopComplete` transition | `usize`       | after loop count increments                     | no          | Payload is the completed loop count. |
| `on_complete`      | adapter observation       | `()`          | after finite loop exhaustion pauses the marquee | no          | Fire once per exhaustion cycle.      |

## 12. Failure and Degradation Rules

| Condition                           | Policy             | Notes                                                                         |
| ----------------------------------- | ------------------ | ----------------------------------------------------------------------------- |
| reduced motion enabled              | fallback path      | Auto-pause on mount and keep manual play control available.                   |
| auto-fill measurement unavailable   | degrade gracefully | Render a single duplicate pair and accept a visible seam rather than failing. |
| loop boundary detection unavailable | warn and ignore    | Keep marquee playable even if completion callbacks cannot fire reliably.      |

## 13. Identity and Key Policy

The root identity is stable. Duplicated content nodes must keep deterministic ordering: original first, decorative duplicates after it.

## 14. SSR and Client Boundary Rules

- SSR renders the root, at least one content copy, edges if configured, and the trigger.
- Measurement-driven duplicate expansion and reduced-motion detection start after mount.
- The initial play/pause state must be hydration-stable for the same props; reduced-motion auto-pause may transition immediately after mount.

## 15. Performance Constraints

- Reuse measured duplicate counts until geometry inputs actually change.
- Avoid per-frame JS animation loops; prefer CSS animation plus lightweight boundary observation.

## 16. Implementation Dependencies

| Dependency            | Required?   | Dependency type      | Why it must exist first                                        | Notes                                   |
| --------------------- | ----------- | -------------------- | -------------------------------------------------------------- | --------------------------------------- |
| reduced-motion helper | required    | accessibility helper | Auto-pauses on mount when motion should be suppressed.         | Shared policy, explicit local contract. |
| measurement helper    | recommended | layout helper        | Supports `auto_fill` and reliable loop completion observation. | Keep instance-scoped.                   |

## 17. Recommended Implementation Sequence

1. Initialize the machine and render base structure.
2. Wire trigger, hover, and focus events.
3. Add reduced-motion auto-pause.
4. Measure for auto-fill and duplicate counts.
5. Observe loop completion and emit callbacks.

## 18. Anti-Patterns

- Do not expose decorative duplicate content to assistive technology.
- Do not hide or omit the pause/play control.
- Do not auto-resume after reduced-motion auto-pause without explicit user action.

## 19. Consumer Expectations and Guarantees

- Consumers may assume content duplication is adapter-owned.
- Consumers may assume the marquee can always be paused manually.
- Consumers must not assume loop callbacks fire when measurement or animation boundaries are unavailable on the platform.

## 20. Platform Support Matrix

| Capability / behavior                          | Browser client | SSR            | Notes                                      |
| ---------------------------------------------- | -------------- | -------------- | ------------------------------------------ |
| duplicated scrolling content and pause control | full support   | full support   | Structural output is server-safe.          |
| reduced-motion auto-pause                      | client-only    | SSR-safe empty | Media-query evaluation starts after mount. |
| auto-fill measurement                          | client-only    | SSR-safe empty | Requires live DOM geometry.                |
| loop completion callbacks                      | fallback path  | SSR-safe empty | Depends on reliable boundary observation.  |

## 21. Debug Diagnostics and Production Policy

| Condition                                         | Debug build behavior | Production behavior | Notes                                              |
| ------------------------------------------------- | -------------------- | ------------------- | -------------------------------------------------- |
| no pause control rendered                         | fail fast            | fail fast           | Accessibility contract violation.                  |
| duplicate measurement unavailable for `auto_fill` | debug warning        | degrade gracefully  | Keep the marquee running with minimal duplication. |

## 22. Shared Adapter Helper Notes

| Helper concept        | Required?   | Responsibility                                       | Reused by                           | Notes                                                     |
| --------------------- | ----------- | ---------------------------------------------------- | ----------------------------------- | --------------------------------------------------------- |
| measurement helper    | recommended | Measure viewport and content extent for duplication. | scrolling or measured widgets       | Keep reads batched and instance-scoped.                   |
| reduced-motion helper | required    | Detect motion suppression preference.                | `skeleton` and motion-heavy widgets | Must not hide the resulting pause behavior from the spec. |

## 23. Framework-Specific Behavior

Leptos 0.8.x can derive root and content attr maps reactively while keeping duplicate-count measurement in an effect. `Children` should be invoked once for the logical content and cloned into decorative wrappers without mutating the original vnode tree.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn Marquee(children: Children) -> impl IntoView {
    let machine = use_machine::<marquee::Machine>(marquee::Props::default());

    view! {
        <div
            {..attr_map_to_leptos(machine.derive(|api| api.root_attrs()).get(), &use_style_strategy(), None).attrs}
            on:mouseenter=move |_| machine.send.run(marquee::Event::HoverIn)
            on:mouseleave=move |_| machine.send.run(marquee::Event::HoverOut)
            on:focusin=move |_| machine.send.run(marquee::Event::FocusIn)
            on:focusout=move |_| machine.send.run(marquee::Event::FocusOut)
        >
            <div {..attr_map_to_leptos(machine.derive(|api| api.edge_attrs(marquee::EdgeSide::Start)).get(), &use_style_strategy(), None).attrs} />
            <div {..attr_map_to_leptos(machine.derive(|api| api.content_attrs()).get(), &use_style_strategy(), None).attrs}>{children()}</div>
            <div {..attr_map_to_leptos(machine.derive(|api| api.content_attrs()).get(), &use_style_strategy(), None).attrs} aria-hidden="true">{children()}</div>
            <div {..attr_map_to_leptos(machine.derive(|api| api.edge_attrs(marquee::EdgeSide::End)).get(), &use_style_strategy(), None).attrs} />
            <button
                {..attr_map_to_leptos(machine.derive(|api| api.auto_play_trigger_attrs()).get(), &use_style_strategy(), None).attrs}
                on:click=move |_| if machine.derive(|api| api.is_playing()).get() {
                    machine.send.run(marquee::Event::Pause)
                } else {
                    machine.send.run(marquee::Event::Play)
                }
            />
        </div>
    }
}
```

## 25. Reference Implementation Skeleton

The implementation should keep one machine, one reduced-motion watcher, one optional measurement effect for `auto_fill`, and one boundary observer for loop completion. Duplicate nodes should be produced from a measured count, not ad hoc conditional cloning spread across render branches.

## 26. Adapter Invariants

- Decorative duplicates remain hidden from AT.
- The pause/play trigger is always present and semantically correct.
- Reduced-motion auto-pause never removes the ability to resume manually.

## 27. Accessibility and SSR Notes

- `aria-live` is `"off"` while playing and `"polite"` while paused.
- The trigger labels must be localized.
- SSR should emit a stable initial structure even when post-mount measurement adds more duplicates.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit adapter duplication and measurement rules.
- Intentional deviations: none beyond adapter-owned pause control and duplicate-measurement policy already implied by the agnostic spec.

## 29. Test Scenarios

1. Hover and focus pause policies send the correct machine events and update `aria-live`.
2. Reduced-motion mode auto-pauses on mount while preserving manual resume.
3. Decorative duplicate content remains hidden from AT and loop callbacks fire only when boundaries are observed.

## 30. Test Oracle Notes

- Preferred oracle for semantics: inspect root `aria-live`, trigger `aria-pressed`, and duplicate `aria-hidden`.
- Preferred oracle for callbacks: record pause and loop callback order around machine transitions.
- Verification recipe: mount with reduced motion enabled, then manually resume and confirm the trigger plus callback behavior remains coherent.

## 31. Implementation Checklist

- [ ] The base structure includes root, duplicate content, and trigger.
- [ ] Reduced-motion auto-pause is explicit.
- [ ] Auto-fill measurement is instance-scoped and optional.
- [ ] Loop callbacks tolerate unavailable boundary detection.
- [ ] Tests cover hover/focus pause, reduced motion, and duplicate accessibility.
