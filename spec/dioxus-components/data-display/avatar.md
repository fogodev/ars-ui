---
adapter: dioxus
component: avatar
category: data-display
source: components/data-display/avatar.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Avatar — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Avatar`](../../components/data-display/avatar.md) contract onto a Dioxus 0.7.x component. The adapter must preserve the loading/error/fallback state machine, own image event wiring, and keep the accessible name stable while switching between image and fallback content.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct AvatarProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub src: Option<Signal<Option<String>>>,
    pub name: String,
    #[props(optional)]
    pub shape: Option<avatar::Shape>,
    #[props(optional)]
    pub size: Option<avatar::Size>,
    #[props(optional)]
    pub fallback_delay: Option<u32>,
    #[props(optional)]
    pub locale: Option<Locale>,
    #[props(optional)]
    pub messages: Option<avatar::Messages>,
    pub fallback: Element,
    #[props(optional)]
    pub on_load: Option<EventHandler<()>>,
    #[props(optional)]
    pub on_error: Option<EventHandler<()>>,
}

#[component]
pub fn Avatar(props: AvatarProps) -> Element
```

`src` is the primary reactive input. If no `fallback` slot is provided, the adapter renders locale-aware initials from `name`.

## 3. Mapping to Core Component Contract

- Props parity: full parity with adapter callbacks for image load and error notifications.
- Part parity: full parity for `Root`, `Image`, and `Fallback`.
- Traceability note: this spec promotes image event wiring, accessible-name preservation, fallback timing, and stale-event handling from the agnostic spec.

## 4. Part Mapping

| Core part  | Required?   | Adapter rendering target | Ownership     | Attr source            | Notes                                       |
| ---------- | ----------- | ------------------------ | ------------- | ---------------------- | ------------------------------------------- |
| `Root`     | required    | `<span>` by default      | adapter-owned | `api.root_attrs()`     | Carries shape, size, and accessible naming. |
| `Image`    | conditional | `<img>`                  | adapter-owned | `api.image_attrs()`    | Hidden when fallback is active.             |
| `Fallback` | conditional | `<span>`                 | adapter-owned | `api.fallback_attrs()` | Slot content or derived initials.           |

## 5. Attr Merge and Ownership Rules

- Core attrs include `data-ars-state`, `data-ars-shape`, `data-ars-size`, and visibility-related attrs.
- The adapter owns `alt`, root accessible naming, image visibility, and fallback hiding rules.
- Consumer classes merge additively, but consumers must not replace the image or fallback hosts.
- The root accessible name is derived from `name`; fallback content stays decorative when the name already labels the root.

## 6. Composition / Context Contract

`Avatar` is standalone. It may resolve locale, messages, and initials logic from the nearest `ArsProvider`, but it does not publish adapter context.

## 7. Prop Sync and Event Mapping

| Adapter prop / event | Mode           | Sync trigger              | Machine event / update path | Notes                                                                              |
| -------------------- | -------------- | ------------------------- | --------------------------- | ---------------------------------------------------------------------------------- |
| `src`                | controlled     | signal change after mount | `SetSrc`                    | Restarts loading and hides fallback until outcome is known.                        |
| `<img onload>`       | adapter event  | native image load         | `ImageLoad`                 | Fires `on_load` after machine transition.                                          |
| `<img onerror>`      | adapter event  | native image error        | `ImageError`                | Fires `on_error` after machine transition.                                         |
| `fallback_delay`     | initialization | render time               | machine context setup       | Delay ownership stays adapter-visible even when timing uses CSS or effect helpers. |

## 8. Registration and Cleanup Contract

- The adapter may allocate a fallback-delay timer when a delayed fallback policy is implemented.
- Cleanup must cancel the delay timer and ignore stale image events after unmount or `src` replacement.

## 9. Ref and Node Contract

No live ref is required for the base avatar. Image state is driven by native load/error events instead of node inspection.

## 10. State Machine Boundary Rules

- Machine-owned state: loading status, fallback visibility, and visible state label.
- Adapter-owned derived values: slot fallback content and initials text.
- Forbidden mirror: do not track separate local booleans for image visibility outside the machine.

## 11. Callback Payload Contract

| Callback   | Payload source      | Payload shape | Timing                        | Cancelable? | Notes                                                             |
| ---------- | ------------------- | ------------- | ----------------------------- | ----------- | ----------------------------------------------------------------- |
| `on_load`  | adapter image event | `()`          | after `ImageLoad` transition  | no          | Must not fire for stale `src` values.                             |
| `on_error` | adapter image event | `()`          | after `ImageError` transition | no          | Must not fire if the component already switched to a newer `src`. |

## 12. Failure and Degradation Rules

| Condition                                   | Policy             | Notes                                                             |
| ------------------------------------------- | ------------------ | ----------------------------------------------------------------- |
| image fails to load                         | fallback path      | Show fallback immediately and preserve accessible name on `Root`. |
| empty `name` and no explicit external label | debug warning      | Rendering still succeeds, but accessible naming is incomplete.    |
| `src` changes during an in-flight load      | degrade gracefully | Ignore stale completion events.                                   |

## 13. Identity and Key Policy

The root identity is stable. `Image` and `Fallback` may toggle visibility, but their relative order must remain stable across hydration and src changes.

## 14. SSR and Client Boundary Rules

- SSR may render the root with the initial machine state, but actual load success or failure is client-only.
- Hydration must preserve the root and any fallback placeholder structure implied by initial props.
- Delayed fallback timers and native image events run only after mount.

## 15. Performance Constraints

- Ignore duplicate load/error events after the machine leaves `Loading`.
- Initials derivation should be memoized from `name`, locale, and messages.

## 16. Implementation Dependencies

| Dependency        | Required?   | Dependency type | Why it must exist first                | Notes                           |
| ----------------- | ----------- | --------------- | -------------------------------------- | ------------------------------- |
| formatting helper | recommended | i18n helper     | Derives initials from localized names. | Shared logic, not a public API. |

## 17. Recommended Implementation Sequence

1. Initialize the avatar machine from `src`.
2. Wire reactive `src` sync into `SetSrc`.
3. Render root, image, and fallback structure with stable ordering.
4. Attach load and error handlers.
5. Add delayed fallback cleanup if the implementation uses timers.

## 18. Anti-Patterns

- Do not hide both `Image` and `Fallback` at the same time.
- Do not move the accessible name from `Root` onto transient image-only markup.
- Do not let stale image events mutate current state.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the avatar always exposes one stable accessible identity.
- Consumers may assume fallback content appears when no image is available.
- Consumers must not assume the fallback text is announced separately from the root name.

## 20. Platform Support Matrix

| Capability / behavior              | Web          | Desktop       | Mobile        | SSR            | Notes                                                      |
| ---------------------------------- | ------------ | ------------- | ------------- | -------------- | ---------------------------------------------------------- |
| image load/error state transitions | full support | fallback path | fallback path | SSR-safe empty | Non-web renderers may need platform image-status plumbing. |
| fallback rendering and initials    | full support | full support  | full support  | full support   | Server-safe structural output.                             |
| fallback delay timer               | client-only  | client-only   | client-only   | SSR-safe empty | Timer setup starts after mount.                            |

## 21. Debug Diagnostics and Production Policy

| Condition                            | Debug build behavior | Production behavior | Notes                               |
| ------------------------------------ | -------------------- | ------------------- | ----------------------------------- |
| missing accessible name context      | debug warning        | warn and ignore     | Root still renders.                 |
| stale image event after `src` change | debug warning        | no-op               | Production ignores outdated events. |

## 22. Shared Adapter Helper Notes

| Helper concept  | Required?   | Responsibility                         | Reused by                      | Notes                        |
| --------------- | ----------- | -------------------------------------- | ------------------------------ | ---------------------------- |
| initials helper | recommended | Locale-aware fallback text derivation. | other identity-display widgets | Keep pure and deterministic. |

## 23. Framework-Specific Behavior

Dioxus 0.7.x should sync `src` via a watched signal and avoid recreating the machine on each parent render. Optional fallback slot content should not perturb hook ordering or root identity.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct AvatarSketchProps {
    #[props(optional)]
    pub src: Option<Signal<Option<String>>>,
    pub name: String,
}

#[component]
pub fn Avatar(props: AvatarSketchProps) -> Element {
    let machine = use_machine::<avatar::Machine>(avatar::Props {
        src: props.src.as_ref().map(|s| s.peek().clone()).flatten(),
        name: Some(props.name.clone()),
        ..Default::default()
    });
    let strategy = use_style_strategy();

    use_effect(move || {
        if let Some(src) = props.src.as_ref() {
            if let Some(value) = src.read().clone() {
                machine.send.call(avatar::Event::SetSrc(value));
            }
        }
    });

    rsx! {
        span {
            ..attr_map_to_dioxus(machine.derive(|api| api.root_attrs())(), &strategy, None).attrs,
            img {
                ..attr_map_to_dioxus(machine.derive(|api| api.image_attrs())(), &strategy, None).attrs,
                onload: move |_| machine.send.call(avatar::Event::ImageLoad),
                onerror: move |_| machine.send.call(avatar::Event::ImageError),
            }
            span { ..attr_map_to_dioxus(machine.derive(|api| api.fallback_attrs())(), &strategy, None).attrs }
        }
    }
}
```

## 25. Reference Implementation Skeleton

The implementation should keep one machine instance, one `src` watcher, and one optional fallback-delay cleanup handle. No other local state is required.

## 26. Adapter Invariants

- The root accessible name remains stable through image success or failure.
- Only current `src` events may affect the machine.
- Fallback rendering remains available whenever the image is absent or broken.

## 27. Accessibility and SSR Notes

- Use `name` as the root accessible name.
- Keep fallback content decorative when the root already exposes the same identity.
- SSR must not guess image success; that transition is client-only.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit adapter load/error callbacks.
- Intentional deviations: none beyond adapter callback timing and optional fallback-slot rendering.

## 29. Test Scenarios

1. `src` load success hides fallback and fires `on_load`.
2. `src` failure shows fallback and fires `on_error`.
3. `src` replacement ignores stale events from the previous image request.

## 30. Test Oracle Notes

- Preferred oracle for state: inspect `data-ars-state` plus image and fallback visibility attrs.
- Preferred oracle for callbacks: record event order around `ImageLoad` and `ImageError`.
- Verification recipe: swap `src` twice quickly and confirm only the last request wins.

## 31. Implementation Checklist

- [ ] `src` changes map to `SetSrc`.
- [ ] Image load/error handlers fire after machine transitions.
- [ ] Fallback visibility is machine-owned.
- [ ] Accessible naming does not regress when the image disappears.
- [ ] Tests cover success, error, and stale-event cases.
