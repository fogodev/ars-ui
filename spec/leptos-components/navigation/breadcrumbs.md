---
adapter: leptos
component: breadcrumbs
category: navigation
source: components/navigation/breadcrumbs.md
source_foundation: foundation/08-adapter-leptos.md
---

# Breadcrumbs — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Breadcrumbs`](../../components/navigation/breadcrumbs.md) contract onto a Leptos 0.8.x component. The adapter preserves navigation landmark semantics, ordered list structure, current-page semantics, collapsed-tail ellipsis behavior, and localized separator rendering.

## 2. Public Adapter API

```rust
#[component]
pub fn Breadcrumbs(
    items: Vec<breadcrumbs::ItemDef>,
    #[prop(optional)] max_items: Option<usize>,
    #[prop(optional)] separator: breadcrumbs::Separator,
    #[prop(optional)] nav_label: Option<String>,
    #[prop(optional)] dir: Option<Direction>,
) -> impl IntoView
```

`items` is the ordered breadcrumb trail. The adapter derives whether an entry renders as `Link`, `CurrentPage`, or a collapsed ellipsis expansion point from item position and the core layout algorithm.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core stateless props, including separator, direction, localized nav label, and `max_items`.
- Part parity: full parity with `Root`, `List`, `Item`, `Link`, `CurrentPage`, and `Separator`.
- Adapter additions: explicit Leptos rendering contract for collapsed ellipsis expansion and item iteration.

## 4. Part Mapping

| Core part / structure | Required?                       | Adapter rendering target | Ownership     | Attr source                | Notes                                   |
| --------------------- | ------------------------------- | ------------------------ | ------------- | -------------------------- | --------------------------------------- |
| `Root`                | required                        | `<nav>`                  | adapter-owned | `api.root_attrs()`         | Owns landmark label and direction.      |
| `List`                | required                        | `<ol>`                   | adapter-owned | `api.list_attrs()`         | Ordered list wrapper for all items.     |
| `Item`                | required                        | `<li>`                   | adapter-owned | `api.item_attrs()`         | Repeated structural item wrapper.       |
| `Link`                | conditional                     | `<a>`                    | adapter-owned | `api.link_attrs(href)`     | Rendered for non-current visible items. |
| `CurrentPage`         | required for final visible item | `<span>`                 | adapter-owned | `api.current_page_attrs()` | Carries `aria-current`.                 |
| `Separator`           | repeated                        | `<span>`                 | adapter-owned | `api.separator_attrs()`    | Decorative only.                        |

## 5. Attr Merge and Ownership Rules

| Target node   | Core attrs                                          | Adapter-owned attrs                                        | Consumer attrs                           | Merge order                            | Ownership notes                     |
| ------------- | --------------------------------------------------- | ---------------------------------------------------------- | ---------------------------------------- | -------------------------------------- | ----------------------------------- |
| `Root`        | localized `aria-label`, scope and part attrs, `dir` | none beyond structural rendering                           | wrapper decoration only if later exposed | landmark semantics win                 | `Root` stays adapter-owned          |
| `Link`        | `href`, scope and part attrs                        | collapsed-layout click normalization when ellipsis expands | link decoration attrs if later exposed   | required `href` and semantic attrs win | no router interception happens here |
| `CurrentPage` | `aria-current="page"`                               | none beyond structural rendering                           | text decoration only                     | `aria-current` must remain             | current page is not interactive     |
| `Separator`   | `aria-hidden="true"`                                | none                                                       | no consumer-owned attrs                  | core attrs win                         | decorative only                     |

## 6. Composition / Context Contract

`Breadcrumbs` is standalone. It does not publish required child context and does not consume any required context. Optional locale or direction defaults may come from higher-level environment wrappers before props are constructed.

## 7. Prop Sync and Event Mapping

| Adapter prop                      | Mode       | Sync trigger            | Machine event / update path | Visible effect                             | Notes                                         |
| --------------------------------- | ---------- | ----------------------- | --------------------------- | ------------------------------------------ | --------------------------------------------- |
| `items`                           | controlled | rerender with new props | stateless recomputation     | updates visible crumbs and collapse layout | no local cache                                |
| `max_items`                       | controlled | rerender with new props | stateless recomputation     | switches between full and collapsed layout | ellipsis appears only when layout requires it |
| `separator` / `nav_label` / `dir` | controlled | rerender with new props | stateless recomputation     | updates text, label, and direction attrs   | no post-mount watcher required                |

When collapsed layout is active, clicking the ellipsis expansion trigger is adapter-owned UI behavior that expands the hidden range for the current render pass. No core state machine is introduced.

## 8. Registration and Cleanup Contract

- No descendant registry exists.
- Ellipsis expansion uses ordinary local render state when implemented; no global listeners or timers are allowed.
- Cleanup is normal vnode disposal only.

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability | Composition rule | Notes                                          |
| ------------------ | ------------- | ------------- | ----------------- | ---------------- | ---------------------------------------------- |
| `Root`             | no            | adapter-owned | always structural | no composition   | landmark only                                  |
| `Link`             | no            | adapter-owned | always structural | no composition   | standard anchor semantics                      |
| ellipsis trigger   | no            | adapter-owned | conditional       | no composition   | button-like expansion affordance when rendered |

## 10. State Machine Boundary Rules

- No core machine is introduced.
- Layout derivation, current-page detection, and ellipsis expansion are adapter-owned render concerns only.
- The adapter must not invent pressed, focused, or selection state beyond native semantics.

## 11. Callback Payload Contract

| Callback                          | Payload source  | Payload shape                      | Timing                        | Cancelable? | Notes                        |
| --------------------------------- | --------------- | ---------------------------------- | ----------------------------- | ----------- | ---------------------------- |
| optional ellipsis-expand callback | adapter-derived | `{ replaced_range: Range<usize> }` | after expansion state changes | no          | only if a wrapper exposes it |

## 12. Failure and Degradation Rules

| Condition                                    | Policy             | Notes                                                      |
| -------------------------------------------- | ------------------ | ---------------------------------------------------------- |
| `max_items < 2`                              | warn and ignore    | fall back to full layout                                   |
| item data omits href for a non-current crumb | degrade gracefully | render plain text item instead of broken link              |
| locale-specific separator cannot be resolved | degrade gracefully | use the explicit separator prop or a simple slash fallback |

## 13. Identity and Key Policy

Visible crumbs use stable item identity from the ordered `items` vector. Server and client must preserve the same visible order for the initial collapsed or expanded layout.

## 14. SSR and Client Boundary Rules

- SSR renders the same `Root`, `List`, visible `Item`s, and `CurrentPage` structure implied by the initial props.
- Collapsed layout may SSR either the collapsed ellipsis or the expanded tail, but server and client must agree on the initial branch.
- No client-only API is required for baseline rendering.

## 15. Performance Constraints

- Recompute the visible layout from props; do not keep a second derived breadcrumb list outside render state.
- Avoid rebuilding separator markup in multiple places; reuse the same derived separator text.
- Keep ellipsis expansion instance-local.

## 16. Implementation Dependencies

| Dependency | Required?   | Dependency type | Why it must exist first                                     | Notes                                    |
| ---------- | ----------- | --------------- | ----------------------------------------------------------- | ---------------------------------------- |
| `link`     | recommended | semantic helper | Reuses the documented anchor contract for breadcrumb links. | Keep router-specific behavior in `Link`. |

## 17. Recommended Implementation Sequence

1. Render `Root` and `List`.
2. Derive visible layout from `items` and `max_items`.
3. Render repeated `Item`s with `Link` or `CurrentPage`.
4. Insert separators in the documented order.
5. Add collapsed ellipsis expansion behavior and final diagnostics.

## 18. Anti-Patterns

- Do not render the current page as an anchor by default.
- Do not expose separators to assistive technology.
- Do not reorder crumbs in the DOM for RTL; use `dir` instead.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the last visible current item receives `aria-current`.
- Consumers may assume collapsed layout preserves item order when expanded.
- Consumers must not assume undocumented global disabled state or router interception.

## 20. Platform Support Matrix

| Capability / behavior                                     | Browser client | SSR          | Notes                                                            |
| --------------------------------------------------------- | -------------- | ------------ | ---------------------------------------------------------------- |
| navigation landmark, ordered list, current-page semantics | full support   | full support | no client-only behavior required                                 |
| collapsed ellipsis expansion                              | full support   | full support | initial expanded vs collapsed branch must match across hydration |

## 21. Debug Diagnostics and Production Policy

| Condition                                     | Debug build behavior | Production behavior | Notes                   |
| --------------------------------------------- | -------------------- | ------------------- | ----------------------- |
| invalid `max_items` value                     | debug warning        | warn and ignore     | full layout fallback    |
| multiple items marked current in wrapper data | debug warning        | warn and ignore     | final visible item wins |

## 22. Shared Adapter Helper Notes

| Helper concept                   | Required?   | Responsibility                                   | Reused by            | Notes                                    |
| -------------------------------- | ----------- | ------------------------------------------------ | -------------------- | ---------------------------------------- |
| button-or-anchor semantic helper | recommended | normalizes ellipsis trigger vs plain link output | `pagination`, `link` | only needed when ellipsis is interactive |

## 23. Framework-Specific Behavior

Leptos can keep ellipsis expansion as local signal state and render the ordered list directly with `For` or standard iterators. No context publication or `NodeRef` composition is required for the baseline adapter.

## 24. Canonical Implementation Sketch

```rust
let api = breadcrumbs::Api::new(props);
let layout = api.layout(items.len());

view! {
    <nav {..api.root_attrs()}>
        <ol {..api.list_attrs()}>
            // render visible items from `layout`
        </ol>
    </nav>
}
```

## 25. Reference Implementation Skeleton

- Build the stateless API from the current props.
- Derive the visible layout once per render.
- Render `Link` for visible non-current items, `CurrentPage` for the final visible item, and `Separator` between visible items only.

## 26. Adapter Invariants

- `Root` always renders as a navigation landmark.
- `CurrentPage` is never hidden behind a separator or ellipsis trigger.
- `Separator` remains decorative and `aria-hidden`.
- DOM order always stays first-crumb-first even in RTL.

## 27. Accessibility and SSR Notes

- The localized breadcrumb label belongs on `Root`, not `List`.
- `aria-current` belongs only on the rendered current item.
- SSR must not emit different current-page semantics than the hydrated client tree.

## 28. Parity Summary and Intentional Deviations

- Matches the core breadcrumb contract without intentional adapter divergence.
- Promotes landmark labeling, collapsed ellipsis behavior, separator hiding, and current-page rendering into explicit Leptos-facing rules.

## 29. Test Scenarios

- full breadcrumb trail with plain separators
- collapsed tail with ellipsis expansion
- current-page item rendered as text with `aria-current="page"`
- RTL root rendering without DOM reordering

## 30. Test Oracle Notes

- Inspect the DOM for `<nav>`, `<ol>`, and `aria-current` placement.
- Verify separator nodes remain `aria-hidden`.
- Use a hydration test to confirm the initial collapsed or expanded branch matches server output.

## 31. Implementation Checklist

- [ ] Render `Root`, `List`, repeated `Item`s, and `CurrentPage` in the documented order.
- [ ] Keep `Separator` decorative and hidden from assistive technology.
- [ ] Preserve DOM order in RTL.
- [ ] Keep ellipsis expansion instance-local with no global listeners.
