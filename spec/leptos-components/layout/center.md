---
adapter: leptos
component: center
category: layout
source: components/layout/center.md
source_foundation: foundation/08-adapter-leptos.md
---

# Center — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Center`](../../components/layout/center.md) contract onto a Leptos `0.8.x` component. The adapter preserves the single `Root` part, CSS-logical centering behavior, and passive layout semantics while defining how Leptos children and optional upstream direction or token context are applied.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn Center(
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] max_width: Option<Spacing>,
    #[prop(optional)] horizontal: bool,
    #[prop(optional)] vertical: bool,
    #[prop(optional)] text_align: Option<TextAlign>,
    children: Children,
) -> impl IntoView
```

The adapter exposes the full core prop set and renders one structural wrapper around the provided children.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `Props`.
- Part parity: full parity with the core `Root`.
- Adapter additions: explicit Leptos child contract and optional context-driven token or direction resolution.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source        | Notes                                           |
| --------------------- | --------- | ------------------------ | ------------- | ------------------ | ----------------------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()` | The root carries all computed centering styles. |

## 5. Attr Merge and Ownership Rules

| Target node | Core attrs                                                       | Adapter-owned attrs          | Consumer attrs                        | Merge order                                                                                                                      | Ownership notes          |
| ----------- | ---------------------------------------------------------------- | ---------------------------- | ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| `Root`      | `api.root_attrs()` including logical sizing and centering styles | no additional required attrs | wrapper decoration attrs when exposed | core `data-ars-*` attrs and required logical styles win; additive `class` and compatible `style` decorations may extend the root | root stays adapter-owned |

- Logical centering styles such as `margin-inline`, flex centering, and computed text alignment must remain intact after attr merge.

## 6. Composition / Context Contract

`Center` is standalone. It provides no context. Optional environment or direction context may be consumed to resolve spacing tokens or RTL-aware text alignment, but missing optional context must degrade gracefully to direct prop values.

## 7. Prop Sync and Event Mapping

| Adapter prop | Mode       | Sync trigger            | Machine event / update path | Visible effect             | Notes                            |
| ------------ | ---------- | ----------------------- | --------------------------- | -------------------------- | -------------------------------- |
| all props    | controlled | rerender with new props | direct attr recomputation   | updates root layout styles | no machine or event layer exists |

No adapter events or callbacks are required.

## 8. Registration and Cleanup Contract

No registration, observers, timers, or cleanup paths are required.

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability                  | Composition rule        | Notes                                  |
| ------------------ | ------------- | ------------- | ---------------------------------- | ----------------------- | -------------------------------------- |
| `Root`             | no            | adapter-owned | always structural, handle optional | no composition required | No imperative node access is required. |

## 10. State Machine Boundary Rules

- machine-owned state: not applicable.
- adapter-local derived bookkeeping: resolved direction and token context inputs only.
- forbidden local mirrors: do not cache style props independently from the latest render.
- allowed snapshot-read contexts: render-time style derivation only.

## 11. Callback Payload Contract

| Callback | Payload source | Payload shape | Timing         | Cancelable? | Notes                          |
| -------- | -------------- | ------------- | -------------- | ----------- | ------------------------------ |
| none     | none           | none          | not applicable | no          | `Center` exposes no callbacks. |

## 12. Failure and Degradation Rules

| Condition                                                         | Policy             | Notes                                                                           |
| ----------------------------------------------------------------- | ------------------ | ------------------------------------------------------------------------------- |
| optional token or direction context missing                       | degrade gracefully | Resolve styles directly from props using default LTR assumptions where needed.  |
| conflicting consumer styles override required centering semantics | warn and ignore    | Wrapper APIs should preserve core layout semantics over conflicting decoration. |
| browser-only APIs unavailable during SSR                          | no-op              | `Center` uses only static attrs and CSS.                                        |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                         | Notes                        |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | ----------------------------------------------- | ---------------------------- |
| root wrapper                     | instance-derived | not applicable      | not applicable                           | root identity must stay stable across hydration | Single structural node only. |

## 14. SSR and Client Boundary Rules

- SSR renders the same single root and computed styles as the client.
- No post-mount listeners, refs, or effects are required.

## 15. Performance Constraints

- Derive styles from props directly instead of storing mirrored local state.
- Avoid unnecessary wrapper nodes around the documented `Root`.

## 16. Implementation Dependencies

| Dependency              | Required?   | Dependency type | Why it must exist first                                                              | Notes                                           |
| ----------------------- | ----------- | --------------- | ------------------------------------------------------------------------------------ | ----------------------------------------------- |
| token-resolution helper | recommended | shared helper   | Keeps spacing and text-alignment resolution consistent with other layout primitives. | May be provided by environment-level utilities. |

## 17. Recommended Implementation Sequence

1. Read props and any optional direction or token context.
2. Build the root attr map from the core API.
3. Merge wrapper decoration without dropping required centering styles.
4. Render children inside the root.

## 18. Anti-Patterns

- Do not add extra wrappers for horizontal or vertical centering modes.
- Do not replace CSS logical properties with hard-coded left or right behavior.
- Do not require context for behavior that can be derived directly from props.

## 19. Consumer Expectations and Guarantees

- Consumers may assume `Center` renders exactly one structural root.
- Consumers may assume RTL-aware text alignment follows the resolved direction input.
- Consumers must not assume the adapter will manage the accessibility semantics of child content.

## 20. Platform Support Matrix

| Capability / behavior                  | Browser client | SSR          | Notes                              |
| -------------------------------------- | -------------- | ------------ | ---------------------------------- |
| CSS centering and logical style output | full support   | full support | No browser-only APIs are required. |

## 21. Debug Diagnostics and Production Policy

| Condition                                                        | Debug build behavior | Production behavior | Notes                               |
| ---------------------------------------------------------------- | -------------------- | ------------------- | ----------------------------------- |
| conflicting wrapper style overrides required centering semantics | debug warning        | warn and ignore     | Required layout semantics must win. |

## 22. Shared Adapter Helper Notes

| Helper concept          | Required?   | Responsibility                                           | Reused by       | Notes                                                |
| ----------------------- | ----------- | -------------------------------------------------------- | --------------- | ---------------------------------------------------- |
| token-resolution helper | recommended | Resolves spacing and text-alignment tokens consistently. | `grid`, `stack` | Generic environment-backed resolution is sufficient. |

## 23. Framework-Specific Behavior

Leptos can resolve optional context during render and apply the resulting attr map directly to the root node. No client effect is required.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn Center(horizontal: bool, vertical: bool, children: Children) -> impl IntoView {
    let api = center::Api::new(center::Props { horizontal, vertical, ..Default::default() }, false, None);
    let root_attrs = api.root_attrs();
    view! { <div {..root_attrs}>{children()}</div> }
}
```

## 25. Reference Implementation Skeleton

No expanded skeleton beyond the canonical sketch is required for this stateless component.

## 26. Adapter Invariants

- The adapter always renders exactly one `Root`.
- Required centering and logical-property styles survive attr merging.
- Missing optional context never blocks rendering.

## 27. Accessibility and SSR Notes

- `Center` is accessibility-neutral; child content remains responsible for semantics.
- The static root structure must remain identical between SSR and hydration.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core `Center` contract.
- Intentional deviations: none.
- Traceability note: optional direction or token inputs are promoted into explicit adapter fallback rules.

## 29. Test Scenarios

- Render horizontal, vertical, and combined centering modes and verify root styles.
- Render with optional direction-dependent text alignment and verify resolved CSS output.
- Merge wrapper decoration and verify required centering styles remain present.

## 30. Test Oracle Notes

- Layout attrs: prefer `DOM attrs`.
- Root stability: prefer `rendered structure`.
- Context fallback: prefer `DOM attrs`.

## 31. Implementation Checklist

- [ ] Render exactly one `Root`.
- [ ] Preserve required centering and logical-property styles during attr merge.
- [ ] Treat optional direction or token context as graceful fallback input only.
- [ ] Keep the component passive and SSR-stable.
