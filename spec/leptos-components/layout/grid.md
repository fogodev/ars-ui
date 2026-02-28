---
adapter: leptos
component: grid
category: layout
source: components/layout/grid.md
source_foundation: foundation/08-adapter-leptos.md
---

# Grid — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Grid`](../../components/layout/grid.md) contract onto a Leptos `0.8.x` component. The adapter preserves the single `Root` part and CSS grid semantics while defining Leptos-facing props, child composition, and token-resolution expectations.

## 2. Public Adapter API

```rust
#[component]
pub fn Grid(
    #[prop(optional)] id: Option<String>,
    #[prop(optional)] columns: Option<u32>,
    #[prop(optional)] auto_columns: Option<Spacing>,
    #[prop(optional)] row_gap: Option<Spacing>,
    #[prop(optional)] column_gap: Option<Spacing>,
    #[prop(optional)] gap: Option<Spacing>,
    #[prop(optional)] align: Option<FlexAlign>,
    #[prop(optional)] stretch: bool,
    children: Children,
) -> impl IntoView
```

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `Props`.
- Part parity: full parity with the core `Root`.
- Adapter additions: explicit child contract and prop-conflict policy when both explicit and auto column strategies are provided.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source        | Notes                                 |
| --------------------- | --------- | ------------------------ | ------------- | ------------------ | ------------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()` | All CSS grid styles live on the root. |

## 5. Attr Merge and Ownership Rules

| Target node | Core attrs                                                | Adapter-owned attrs          | Consumer attrs                        | Merge order                                                                                                                | Ownership notes          |
| ----------- | --------------------------------------------------------- | ---------------------------- | ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- | ------------------------ |
| `Root`      | `api.root_attrs()` including grid-template and gap styles | no additional required attrs | wrapper decoration attrs when exposed | core `data-ars-*` attrs and required grid styles win; additive decoration may extend but not remove required grid behavior | root stays adapter-owned |

- `gap` takes precedence over `row_gap` and `column_gap`, matching the core contract.
- The adapter must not let merged decoration remove `display: grid`.

## 6. Composition / Context Contract

`Grid` is standalone. It provides no context. Optional token-resolution context may be consumed for spacing values, but missing context must degrade gracefully to direct CSS values.

## 7. Prop Sync and Event Mapping

| Adapter prop | Mode       | Sync trigger            | Machine event / update path | Visible effect      | Notes                            |
| ------------ | ---------- | ----------------------- | --------------------------- | ------------------- | -------------------------------- |
| all props    | controlled | rerender with new props | direct attr recomputation   | updates grid styles | no machine or event layer exists |

## 8. Registration and Cleanup Contract

No registration, observers, timers, or cleanup paths are required.

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability                  | Composition rule        | Notes                                   |
| ------------------ | ------------- | ------------- | ---------------------------------- | ----------------------- | --------------------------------------- |
| `Root`             | no            | adapter-owned | always structural, handle optional | no composition required | Imperative node access is not required. |

## 10. State Machine Boundary Rules

- machine-owned state: not applicable.
- adapter-local derived bookkeeping: resolved spacing tokens only.
- forbidden local mirrors: do not mirror grid props into stale local signals.
- allowed snapshot-read contexts: render-time style derivation only.

## 11. Callback Payload Contract

| Callback | Payload source | Payload shape | Timing         | Cancelable? | Notes                        |
| -------- | -------------- | ------------- | -------------- | ----------- | ---------------------------- |
| none     | none           | none          | not applicable | no          | `Grid` exposes no callbacks. |

## 12. Failure and Degradation Rules

| Condition                                  | Policy             | Notes                                                                                    |
| ------------------------------------------ | ------------------ | ---------------------------------------------------------------------------------------- |
| both `columns` and `auto_columns` provided | warn and ignore    | The adapter should follow the core precedence rule instead of inventing a hybrid layout. |
| token context missing                      | degrade gracefully | Resolve spacing directly from provided values.                                           |
| browser-only APIs unavailable during SSR   | no-op              | Grid output is static attrs and styles only.                                             |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                  | Notes                        |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | ---------------------------------------- | ---------------------------- |
| root wrapper                     | instance-derived | not applicable      | not applicable                           | root identity must stay hydration-stable | Single structural node only. |

## 14. SSR and Client Boundary Rules

- SSR renders the same root and computed styles as the client.
- No post-mount listeners or refs are required.

## 15. Performance Constraints

- Recompute grid-template styles from props directly rather than storing mirrored local state.
- Avoid allocating extra wrapper nodes or per-child helper nodes.

## 16. Implementation Dependencies

| Dependency              | Required?   | Dependency type | Why it must exist first                                           | Notes                             |
| ----------------------- | ----------- | --------------- | ----------------------------------------------------------------- | --------------------------------- |
| token-resolution helper | recommended | shared helper   | Keeps spacing resolution consistent with other layout primitives. | Shared with `center` and `stack`. |

## 17. Recommended Implementation Sequence

1. Read props and optional token context.
2. Resolve the effective column strategy.
3. Build the root attr map.
4. Merge wrapper decoration without dropping required grid semantics.
5. Render children inside the root.

## 18. Anti-Patterns

- Do not combine `columns` and `auto_columns` into undocumented hybrid behavior.
- Do not drop `display: grid` or template styles during attr merge.
- Do not add per-child wrappers to emulate gap behavior.

## 19. Consumer Expectations and Guarantees

- Consumers may assume `Grid` renders exactly one structural root.
- Consumers may assume `gap` overrides row and column gap inputs.
- Consumers must not assume the adapter manages item-level accessibility semantics.

## 20. Platform Support Matrix

| Capability / behavior | Browser client | SSR          | Notes                              |
| --------------------- | -------------- | ------------ | ---------------------------------- |
| CSS grid style output | full support   | full support | No browser-only APIs are required. |

## 21. Debug Diagnostics and Production Policy

| Condition                                  | Debug build behavior | Production behavior | Notes                                                       |
| ------------------------------------------ | -------------------- | ------------------- | ----------------------------------------------------------- |
| both `columns` and `auto_columns` provided | debug warning        | warn and ignore     | The adapter should preserve the documented precedence rule. |

## 22. Shared Adapter Helper Notes

| Helper concept          | Required?   | Responsibility                        | Reused by         | Notes                                |
| ----------------------- | ----------- | ------------------------------------- | ----------------- | ------------------------------------ |
| token-resolution helper | recommended | Resolves spacing tokens consistently. | `center`, `stack` | No grid-specific helper is required. |

## 23. Framework-Specific Behavior

Leptos can compute and spread grid attrs during render without effects. `Children` render directly into the grid root.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn Grid(columns: Option<u32>, children: Children) -> impl IntoView {
    let api = grid::Api::new(grid::Props { columns, ..Default::default() }, None);
    let root_attrs = api.root_attrs();
    view! { <div {..root_attrs}>{children()}</div> }
}
```

## 25. Reference Implementation Skeleton

No expanded skeleton beyond the canonical sketch is required for this stateless component.

## 26. Adapter Invariants

- The adapter always renders exactly one `Root`.
- Required grid layout styles survive attr merging.
- The documented precedence between `gap`, `row_gap`, `column_gap`, `columns`, and `auto_columns` remains intact.

## 27. Accessibility and SSR Notes

- `Grid` is accessibility-neutral; consumers own the semantics of grid content.
- The root structure must stay identical across SSR and hydration.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core `Grid` contract.
- Intentional deviations: none.
- Traceability note: the adapter promotes the core precedence rules for mutually related style props into explicit Leptos merge behavior.

## 29. Test Scenarios

- Render with explicit columns and verify `grid-template-columns`.
- Render with `auto_columns` and verify the auto-fill template.
- Provide `gap`, `row_gap`, and `column_gap` combinations and verify precedence.
- Provide both `columns` and `auto_columns` and verify warning plus stable output.

## 30. Test Oracle Notes

- Grid styles: prefer `DOM attrs`.
- Root stability: prefer `rendered structure`.
- Prop-conflict behavior: prefer diagnostics plus `DOM attrs`.

## 31. Implementation Checklist

- [ ] Render exactly one `Root`.
- [ ] Preserve required grid styles and precedence rules during attr merge.
- [ ] Treat missing token context as a graceful fallback.
- [ ] Keep the component passive and SSR-stable.
