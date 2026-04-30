---
adapter: dioxus
component: marquee
category: data-display
source: components/data-display/marquee.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Marquee — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Marquee`](../../components/data-display/marquee.md) contract onto a Dioxus 0.7.x component. The adapter must preserve the play/pause machine, duplicate content for seamless looping, honor reduced-motion auto-pause, and make the pause-control contract explicit across Dioxus renderers.

## 2. Public Adapter API

```rust,no_check
#[derive(Props, Clone, PartialEq)]
pub struct MarqueeProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub speed: Option<f64>,
    #[props(optional)]
    pub direction: Option<marquee::Direction>,
    #[props(optional)]
    pub gap: Option<f64>,
    #[props(default = false)]
    pub pause_on_hover: bool,
    #[props(default = false)]
    pub pause_on_focus: bool,
    #[props(optional)]
    pub loop_count: Option<usize>,
    #[props(default = false)]
    pub auto_fill: bool,
    #[props(optional)]
    pub delay: Option<f64>,
    #[props(default = false)]
    pub auto_play: bool,
    #[props(default = false)]
    pub disabled: bool,
    #[props(optional)]
    pub locale: Option<Locale>,
    #[props(optional)]
    pub messages: Option<marquee::Messages>,
    #[props(optional)]
    pub on_pause_change: Option<EventHandler<bool>>,
    #[props(optional)]
    pub on_loop_complete: Option<EventHandler<usize>>,
    #[props(optional)]
    pub on_complete: Option<EventHandler<()>>,
    pub children: Element,
}

#[component]
pub fn Marquee(props: MarqueeProps) -> Element
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

| Capability / behavior                          | Web           | Desktop       | Mobile        | SSR            | Notes                                     |
| ---------------------------------------------- | ------------- | ------------- | ------------- | -------------- | ----------------------------------------- |
| duplicated scrolling content and pause control | full support  | full support  | full support  | full support   | Structural output is server-safe.         |
| reduced-motion auto-pause                      | client-only   | client-only   | client-only   | SSR-safe empty | Preference evaluation starts after mount. |
| auto-fill measurement                          | client-only   | client-only   | client-only   | SSR-safe empty | Requires live layout metrics.             |
| loop completion callbacks                      | fallback path | fallback path | fallback path | SSR-safe empty | Depends on reliable boundary observation. |

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

Dioxus 0.7.x should keep measurement observers and loop-boundary logic outside the render path, with memoized attr derivation for root, duplicates, and trigger state.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct MarqueeSketchProps {
    pub children: Element,
}

#[component]
pub fn Marquee(props: MarqueeSketchProps) -> Element {
    let machine = use_machine::<marquee::Machine>(marquee::Props::default());
    let strategy = use_style_strategy();

    rsx! {
        div {
            ..attr_map_to_dioxus(machine.derive(|api| api.root_attrs())(), &strategy, None).attrs,
            onmouseenter: move |_| machine.send.call(marquee::Event::HoverIn),
            onmouseleave: move |_| machine.send.call(marquee::Event::HoverOut),
            onfocusin: move |_| machine.send.call(marquee::Event::FocusIn),
            onfocusout: move |_| machine.send.call(marquee::Event::FocusOut),
            div { ..attr_map_to_dioxus(machine.derive(|api| api.edge_attrs(marquee::EdgeSide::Start))(), &strategy, None).attrs }
            div { ..attr_map_to_dioxus(machine.derive(|api| api.content_attrs())(), &strategy, None).attrs, {props.children.clone()} }
            div { ..attr_map_to_dioxus(machine.derive(|api| api.content_attrs())(), &strategy, None).attrs, aria_hidden: "true", {props.children} }
            div { ..attr_map_to_dioxus(machine.derive(|api| api.edge_attrs(marquee::EdgeSide::End))(), &strategy, None).attrs }
            button { ..attr_map_to_dioxus(machine.derive(|api| api.auto_play_trigger_attrs())(), &strategy, None).attrs }
        }
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
