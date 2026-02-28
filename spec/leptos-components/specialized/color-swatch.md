---
adapter: leptos
component: color-swatch
category: specialized
source: components/specialized/color-swatch.md
source_foundation: foundation/08-adapter-leptos.md
---

# ColorSwatch — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ColorSwatch`](../../components/specialized/color-swatch.md) contract onto Leptos `0.8.x`. The adapter preserves the stateless visual swatch contract, perceptual color naming, optional alpha visualization, and non-interactive semantics.

## 2. Public Adapter API

```rust
#[component]
pub fn ColorSwatch(
    value: ColorValue,
    #[prop(optional)] alpha_grid: bool,
    #[prop(optional)] aria_label: Option<String>,
    #[prop(optional)] class: Option<String>,
) -> impl IntoView
```

The adapter exposes a single semantic surface. It does not publish writable signals or callbacks because the core component is stateless and display-only.

## 3. Mapping to Core Component Contract

- Props parity: full parity with color value, alpha visualization, and accessible naming override.
- Part parity: full parity with `Root` and `Inner`.
- Adapter additions: none beyond Leptos attr merge semantics.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source         | Notes                              |
| --------------------- | --------- | ------------------------ | ------------- | ------------------- | ---------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()`  | Carries `role="img"` and labeling. |
| `Inner`               | required  | `<div>`                  | adapter-owned | `api.inner_attrs()` | Purely visual fill surface.        |

## 5. Attr Merge and Ownership Rules

| Target node | Core attrs                              | Adapter-owned attrs | Consumer attrs            | Merge order             | Ownership notes                                            |
| ----------- | --------------------------------------- | ------------------- | ------------------------- | ----------------------- | ---------------------------------------------------------- |
| `Root`      | role, label, scope, part, alpha markers | Leptos class merge  | `class`, decorative attrs | core semantic attrs win | Consumer styling must not remove `role="img"` or labeling. |
| `Inner`     | visual color attrs and CSS vars         | none                | decoration only           | core visual vars win    | Inner remains non-semantic.                                |

## 6. Composition / Context Contract

`ColorSwatch` is context-free. When embedded inside `ColorSwatchPicker`, the picker owns selection and focus semantics while the swatch remains display-only.

## 7. Prop Sync and Event Mapping

| Adapter prop                        | Mode       | Sync trigger | Machine event / update path | Visible effect                               | Notes                      |
| ----------------------------------- | ---------- | ------------ | --------------------------- | -------------------------------------------- | -------------------------- |
| `value`, `alpha_grid`, `aria_label` | controlled | rerender     | stateless recomputation     | updates label, CSS color, alpha presentation | no post-mount side effects |

## 8. Registration and Cleanup Contract

No registration, timers, listeners, observers, or cleanup paths are required.

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability | Composition rule | Notes                      |
| ------------------ | ------------- | ------------- | ----------------- | ---------------- | -------------------------- |
| `Root`             | no            | adapter-owned | always structural | no composition   | Static semantic node only. |
| `Inner`            | no            | adapter-owned | always structural | no composition   | No imperative reads.       |

## 10. State Machine Boundary Rules

- machine-owned state: none; this component is stateless.
- adapter-local derived bookkeeping: none.
- forbidden local mirrors: do not introduce hover, pressed, or selected state here.

## 11. Callback Payload Contract

No public callbacks are part of this adapter surface.

## 12. Failure and Degradation Rules

| Condition                                                    | Policy             | Notes                                                    |
| ------------------------------------------------------------ | ------------------ | -------------------------------------------------------- |
| invalid or non-displayable color payload reaches the adapter | fail fast          | The core color type should already guarantee validity.   |
| alpha-grid styling asset unavailable                         | degrade gracefully | Render the color swatch without checkerboard decoration. |

## 13. Identity and Key Policy

`ColorSwatch` owns no repeated registration or keyed descendants. Identity is the component instance only.

## 14. SSR and Client Boundary Rules

- SSR renders the same root and inner structure as the client.
- No client-only listeners or measurements are required.
- Hydration must preserve only the stateless CSS and accessible label output.

## 15. Performance Constraints

- Recompute the accessible label only when `value` or `aria_label` changes.
- Avoid allocating browser-only helpers for this component.

## 16. Implementation Dependencies

| Dependency           | Required? | Dependency type | Why it must exist first                                  | Notes                       |
| -------------------- | --------- | --------------- | -------------------------------------------------------- | --------------------------- |
| color-name formatter | required  | helper          | Accessible labeling depends on a stable color-name path. | Shared with `color-picker`. |

## 17. Recommended Implementation Sequence

1. Compute the accessible color label.
2. Render `Root` and `Inner`.
3. Merge any decorative consumer attrs without weakening semantics.

## 18. Anti-Patterns

- Do not make the swatch focusable or interactive.
- Do not move selection semantics from `ColorSwatchPicker` into this component.

## 19. Consumer Expectations and Guarantees

- Consumers may assume `ColorSwatch` is display-only.
- Consumers may assume the root remains labeled as an image.
- Consumers must not assume picker semantics, focus management, or button behavior.

## 20. Platform Support Matrix

| Capability / behavior           | Browser client | SSR          | Notes                        |
| ------------------------------- | -------------- | ------------ | ---------------------------- |
| semantic image swatch rendering | full support   | full support | No client-only dependencies. |
| alpha checkerboard decoration   | full support   | full support | Pure CSS presentation.       |

## 21. Debug Diagnostics and Production Policy

| Condition                                                   | Debug build behavior | Production behavior | Notes                                 |
| ----------------------------------------------------------- | -------------------- | ------------------- | ------------------------------------- |
| semantic attrs are overridden destructively by wrapper glue | debug warning        | warn and ignore     | Keep the core image semantics intact. |

## 22. Shared Adapter Helper Notes

| Helper concept    | Required?   | Responsibility                                            | Reused by              | Notes                                                  |
| ----------------- | ----------- | --------------------------------------------------------- | ---------------------- | ------------------------------------------------------ |
| attr merge helper | recommended | preserves semantic attrs while merging decoration classes | semantic-only surfaces | no special helper beyond normal attr merge is required |

## 23. Framework-Specific Behavior

Leptos may merge `class` and `style` additively, but the rendered root remains adapter-owned and semantic.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn ColorSwatch(value: ColorValue) -> impl IntoView {
    let api = color_swatch::Api::new(&color_swatch::Props { value, ..Default::default() });
    view! {
        <div {..api.root_attrs()}>
            <div {..api.inner_attrs()} />
        </div>
    }
}
```

## 25. Reference Implementation Skeleton

- Build the core API from the incoming value.
- Render the root and inner parts directly.
- Expose no local state, listeners, or effects.

## 26. Adapter Invariants

- The root always remains `role="img"` with an accessible label.
- The inner fill remains decorative only.
- The adapter never adds interactivity.

## 27. Accessibility and SSR Notes

Accessible color naming belongs on the root. The inner node must remain hidden from assistive technology by omission of extra semantics rather than extra roles.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- semantic root labeling
- alpha-grid on/off rendering
- use inside `ColorSwatchPicker` without duplicate interactivity

## 30. Test Oracle Notes

| Behavior            | Preferred oracle type | Notes                                                               |
| ------------------- | --------------------- | ------------------------------------------------------------------- |
| root label and role | DOM attrs             | assert `role="img"` plus resolved label                             |
| alpha rendering     | rendered structure    | assert checkerboard marker classes or vars rather than pixel output |

## 31. Implementation Checklist

- [ ] `Root` and `Inner` are rendered exactly once.
- [ ] The root remains labeled and non-interactive.
- [ ] Alpha decoration degrades gracefully when styling support is absent.
