---
adapter: leptos
component: stack
category: layout
source: components/layout/stack.md
source_foundation: foundation/08-adapter-leptos.md
---

# Stack — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Stack`](../../components/layout/stack.md) contract onto a Leptos `0.8.x` component. The adapter preserves the single `Root` part, flex-layout semantics, and RTL-aware direction resolution while defining the Leptos child contract and divider expectations.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn Stack(
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] direction: StackDirection,
    #[prop(optional)] spacing: Option<Spacing>,
    #[prop(optional)] align: FlexAlign,
    #[prop(optional)] justify: FlexJustify,
    #[prop(optional)] wrap: bool,
    #[prop(optional)] divider: bool,
    #[prop(optional)] full_width: bool,
    #[prop(optional)] full_height: bool,
    children: Children,
) -> impl IntoView
```

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `Props`.
- Part parity: full parity with the core `Root`.
- Adapter additions: explicit divider rendering policy and Leptos child composition rules.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source        | Notes                                    |
| --------------------- | --------- | ------------------------ | ------------- | ------------------ | ---------------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()` | All flex layout styles live on the root. |

## 5. Attr Merge and Ownership Rules

| Target node | Core attrs                                                                   | Adapter-owned attrs                                                                           | Consumer attrs                        | Merge order                                                                                                                  | Ownership notes          |
| ----------- | ---------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| `Root`      | `api.root_attrs()` including flex-direction, gap, and width or height styles | optional divider wrapper markers when the adapter injects separators between child boundaries | wrapper decoration attrs when exposed | core `data-ars-*` attrs and required flex styles win; additive decoration may extend but not remove required stack semantics | root stays adapter-owned |

- Divider support may inject visual children between consumer children, but the root contract remains unchanged.
- Direction resolution must preserve RTL-aware `RowLogical` behavior from the core spec.

## 6. Composition / Context Contract

`Stack` is standalone. It provides no context. Optional direction or token context may be consumed to resolve logical direction and spacing tokens, but missing context must degrade gracefully.

## 7. Prop Sync and Event Mapping

| Adapter prop | Mode       | Sync trigger            | Machine event / update path | Visible effect                                            | Notes                            |
| ------------ | ---------- | ----------------------- | --------------------------- | --------------------------------------------------------- | -------------------------------- |
| all props    | controlled | rerender with new props | direct attr recomputation   | updates root layout styles and optional divider rendering | no machine or event layer exists |

## 8. Registration and Cleanup Contract

No registration, observers, timers, or cleanup paths are required.

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability                  | Composition rule        | Notes                                   |
| ------------------ | ------------- | ------------- | ---------------------------------- | ----------------------- | --------------------------------------- |
| `Root`             | no            | adapter-owned | always structural, handle optional | no composition required | Imperative node access is not required. |

## 10. State Machine Boundary Rules

- machine-owned state: not applicable.
- adapter-local derived bookkeeping: resolved direction and divider insertion plan only.
- forbidden local mirrors: do not cache stack props independently from render inputs.
- allowed snapshot-read contexts: render-time attr and divider derivation only.

## 11. Callback Payload Contract

| Callback | Payload source | Payload shape | Timing         | Cancelable? | Notes                         |
| -------- | -------------- | ------------- | -------------- | ----------- | ----------------------------- |
| none     | none           | none          | not applicable | no          | `Stack` exposes no callbacks. |

## 12. Failure and Degradation Rules

| Condition                                              | Policy             | Notes                                                                         |
| ------------------------------------------------------ | ------------------ | ----------------------------------------------------------------------------- |
| optional direction or token context missing            | degrade gracefully | Resolve logical direction with default LTR assumptions and direct CSS values. |
| divider injection would require invalid child mutation | warn and ignore    | The adapter may skip divider output rather than corrupting child structure.   |
| browser-only APIs unavailable during SSR               | no-op              | Stack behavior is encoded entirely in structure and styles.                   |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                                                     | Notes                             |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | --------------------------------------------------------------------------- | --------------------------------- |
| root wrapper                     | instance-derived | not applicable      | not applicable                           | root identity must stay hydration-stable                                    | Single structural root only.      |
| optional divider nodes           | composite        | yes                 | yes                                      | divider insertion order must remain deterministic between SSR and hydration | Only present when `divider=true`. |

## 14. SSR and Client Boundary Rules

- SSR renders the same root and optional divider structure as the client.
- Divider insertion must be deterministic from props and child count alone.

## 15. Performance Constraints

- Derive direction and divider placement from current props instead of storing mirrored local state.
- Avoid wrapping every child when simple adjacent divider insertion is sufficient.

## 16. Implementation Dependencies

| Dependency              | Required?   | Dependency type | Why it must exist first                                        | Notes                            |
| ----------------------- | ----------- | --------------- | -------------------------------------------------------------- | -------------------------------- |
| token-resolution helper | recommended | shared helper   | Keeps spacing resolution aligned with other layout primitives. | Shared with `center` and `grid`. |

## 17. Recommended Implementation Sequence

1. Resolve direction, spacing, and sizing props.
2. Build the root attr map.
3. Decide whether divider insertion is needed.
4. Render children with deterministic divider placement.

## 18. Anti-Patterns

- Do not replace logical-direction handling with hard-coded left or right assumptions.
- Do not let divider rendering alter the meaning of the root or drop child order.
- Do not remove required flex styles during attr merge.

## 19. Consumer Expectations and Guarantees

- Consumers may assume `Stack` renders one structural root.
- Consumers may assume logical row direction is RTL-aware when the resolved direction input requires it.
- Consumers must not assume divider rendering adds semantics beyond visual separation.

## 20. Platform Support Matrix

| Capability / behavior             | Browser client | SSR          | Notes                              |
| --------------------------------- | -------------- | ------------ | ---------------------------------- |
| flex layout and divider structure | full support   | full support | No browser-only APIs are required. |

## 21. Debug Diagnostics and Production Policy

| Condition                                                       | Debug build behavior | Production behavior | Notes                                                                  |
| --------------------------------------------------------------- | -------------------- | ------------------- | ---------------------------------------------------------------------- |
| divider insertion cannot preserve deterministic child structure | debug warning        | warn and ignore     | The adapter should prefer stable child structure over ad hoc mutation. |

## 22. Shared Adapter Helper Notes

| Helper concept          | Required?   | Responsibility                        | Reused by        | Notes                                 |
| ----------------------- | ----------- | ------------------------------------- | ---------------- | ------------------------------------- |
| token-resolution helper | recommended | Resolves spacing tokens consistently. | `center`, `grid` | No stack-specific helper is required. |

## 23. Framework-Specific Behavior

Leptos can resolve children and optional divider placement during render. No client effect is required for this passive layout primitive.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn Stack(direction: StackDirection, children: Children) -> impl IntoView {
    let api = stack::Api::new(stack::Props { direction, ..Default::default() }, false, None);
    let root_attrs = api.root_attrs();
    view! { <div {..root_attrs}>{children()}</div> }
}
```

## 25. Reference Implementation Skeleton

No expanded skeleton beyond the canonical sketch is required for this stateless component.

## 26. Adapter Invariants

- The adapter always renders one `Root`.
- Required flex-direction, spacing, and sizing styles survive attr merging.
- Divider rendering remains deterministic and does not reorder consumer children.

## 27. Accessibility and SSR Notes

- `Stack` is accessibility-neutral unless a higher-level wrapper adds semantics.
- Divider nodes should remain presentational unless a wrapper explicitly assigns semantics outside this spec.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core `Stack` contract.
- Intentional deviations: none.
- Traceability note: divider insertion and logical-direction fallback are promoted into explicit adapter rules.

## 29. Test Scenarios

- Render row, column, and RTL-aware logical row directions and verify root styles.
- Render with `divider=true` and verify deterministic divider placement.
- Toggle spacing and full-width or full-height props and verify root styles.

## 30. Test Oracle Notes

- Layout attrs: prefer `DOM attrs`.
- Divider placement: prefer `rendered structure`.
- Direction fallback: prefer `DOM attrs`.

## 31. Implementation Checklist

- [ ] Render exactly one `Root`.
- [ ] Preserve required flex and logical-direction styles during attr merge.
- [ ] Keep divider placement deterministic.
- [ ] Treat optional context as graceful fallback input only.
