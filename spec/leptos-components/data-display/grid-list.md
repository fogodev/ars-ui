---
adapter: leptos
component: grid-list
category: data-display
source: components/data-display/grid-list.md
source_foundation: foundation/08-adapter-leptos.md
---

# GridList — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`GridList`](../../components/data-display/grid-list.md) contract onto a Leptos 0.8.x component. The adapter must preserve the focus and selection machine, own row and cell rendering, and make empty-state, link-item, and load-more sentinel behavior explicit.

## 2. Public Adapter API

```rust
pub struct GridListRenderItem(pub Callback<grid_list::ItemDef, AnyView>);

#[component]
pub fn GridList(
    #[prop(optional)] id: Option<String>,
    items: StaticCollection<grid_list::ItemDef>,
    columns: usize,
    #[prop(optional, into)] selected_keys: Option<Signal<selection::Set>>,
    #[prop(optional)] selection_mode: Option<selection::Mode>,
    #[prop(optional)] selection_behavior: Option<selection::Behavior>,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] disabled_keys: Option<BTreeSet<Key>>,
    #[prop(optional)] composite: bool,
    #[prop(optional)] loading: bool,
    #[prop(optional)] on_load_more: Option<Callback<()>>,
    #[prop(optional)] on_action: Option<Callback<Key>>,
    render_item: GridListRenderItem,
    #[prop(optional)] empty_state: Option<Children>,
) -> impl IntoView
```

The adapter owns row/cell scaffolding, roving focus, optional empty state, and optional loading sentinel. Consumers provide item content through the render callback only.

## 3. Mapping to Core Component Contract

- Props parity: full parity with explicit adapter slots for item rendering and empty-state rendering.
- Part parity: full parity for `Root`, repeated `Row`, repeated `Cell`, and structural `LoadingSentinel`; `EmptyState` remains adapter-owned.
- Traceability note: this spec promotes empty-state rendering, link-cell semantics, live announcements, load-more sentinel behavior, and RTL key handling from the agnostic spec.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source                    | Notes                                                     |
| --------------------- | --------- | ------------------------ | ------------- | ------------------------------ | --------------------------------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()`             | `role="grid"` in composite mode; list fallback otherwise. |
| `Row`                 | repeated  | `<div>`                  | adapter-owned | `api.row_attrs(row_index)`     | Groups cells into logical rows.                           |
| `Cell`                | repeated  | `<div>` or `<a>`         | adapter-owned | `api.cell_attrs(key)`          | Link items preserve grid semantics on the anchor host.    |
| `LoadingSentinel`     | optional  | `<div>`                  | adapter-owned | `api.loading_sentinel_attrs()` | Render only when `on_load_more` is configured.            |
| `EmptyState`          | optional  | adapter-chosen wrapper   | adapter-owned | adapter-local attrs            | Render when `items` is empty.                             |

## 5. Attr Merge and Ownership Rules

- Core attrs include `role`, selection state, disabled state, `tabindex`, key data attrs, and optional `href`.
- The adapter owns row grouping, roving tabindex, `aria-selected`, and any anchor-vs-div host choice for link items.
- Consumers may decorate rendered item content, but they must not replace row/cell hosts or sentinel structure.

## 6. Composition / Context Contract

`GridList` is self-contained. It may internally provide lightweight item metadata to row or cell helpers, but no public adapter context is required.

## 7. Prop Sync and Event Mapping

| Adapter prop / event            | Mode           | Sync trigger              | Machine event / update path                | Notes                                                    |
| ------------------------------- | -------------- | ------------------------- | ------------------------------------------ | -------------------------------------------------------- |
| `selected_keys`                 | controlled     | signal change after mount | selection-related events                   | Bindable observation remains machine-driven.             |
| focus entry/exit                | adapter event  | root or cell focus        | `Focus` / `Blur`                           | Focus target may default to the first non-disabled cell. |
| arrow/home/end keys             | adapter event  | keyboard interaction      | navigation events                          | RTL reverses horizontal direction at the adapter layer.  |
| click/space selection           | adapter event  | pointer or keyboard       | `Select`, `ToggleSelection`, or equivalent | Behavior depends on selection mode.                      |
| enter/action                    | adapter event  | keyboard or activation    | `ItemAction`                               | Selection and action remain distinct.                    |
| load-more sentinel intersection | adapter effect | observer callback         | adapter callback only                      | Fires `on_load_more`; no machine mutation required.      |

## 8. Registration and Cleanup Contract

- The adapter owns any descendant registration needed to map flat items into row and cell hosts.
- When `on_load_more` is configured, the adapter may register an IntersectionObserver or scroll-threshold listener for the sentinel.
- Cleanup must dispose observers and ignore stale load-more notifications while `loading=true`.

## 9. Ref and Node Contract

The root node requires a live ref for focus entry and optional load-more observation. Individual cell refs are optional and only needed for robust roving focus or scroll-into-view support.

## 10. State Machine Boundary Rules

- Machine-owned state: focused key, selected set, disabled knowledge, and composite-mode navigation state.
- Adapter-owned derived values: row grouping, cell coordinates, sentinel visibility, and empty-state rendering.
- Forbidden mirror: do not maintain a second selected-key set or focused-key index outside the machine.

## 11. Callback Payload Contract

| Callback       | Payload source       | Payload shape | Timing                                             | Cancelable? | Notes                                         |
| -------------- | -------------------- | ------------- | -------------------------------------------------- | ----------- | --------------------------------------------- |
| `on_action`    | adapter event        | `Key`         | after action event resolution                      | no          | Enter activates links or primary item action. |
| `on_load_more` | sentinel observation | `()`          | when the sentinel crosses the configured threshold | no          | Suppressed while `loading=true`.              |

## 12. Failure and Degradation Rules

| Condition                               | Policy             | Notes                                                         |
| --------------------------------------- | ------------------ | ------------------------------------------------------------- |
| `columns == 0`                          | fail fast          | Grid geometry is undefined.                                   |
| sentinel observation unavailable        | degrade gracefully | Keep the sentinel rendered but do not auto-request more data. |
| empty item set with no empty-state slot | degrade gracefully | Render nothing after `Root` semantics.                        |

## 13. Identity and Key Policy

Each cell identity is keyed by item `Key`. Row identities derive from row index plus the ordered child keys. Empty-state and sentinel nodes must occupy stable positions relative to the item list.

## 14. SSR and Client Boundary Rules

- SSR renders the initial item grid or empty state.
- Load-more observation, roving focus, and action callbacks are client-only.
- The composite-vs-list semantic branch must be hydration-stable.

## 15. Performance Constraints

- Row grouping should be derived once from items and columns.
- Sentinel observation must be shared through one observer per component instance.

## 16. Implementation Dependencies

| Dependency        | Required? | Dependency type     | Why it must exist first                                  | Notes                                   |
| ----------------- | --------- | ------------------- | -------------------------------------------------------- | --------------------------------------- |
| collection helper | required  | registration helper | Provides stable key iteration and selection observation. | Shared with `table` and `tag-group`.    |
| load-more helper  | optional  | measurement helper  | Observes the sentinel or scroll threshold.               | Only when `on_load_more` is configured. |

## 17. Recommended Implementation Sequence

1. Initialize the machine from items and selection props.
2. Derive row grouping and semantic branch.
3. Render root, rows, cells, and optional empty state.
4. Wire focus, selection, action, and RTL-aware keyboard handling.
5. Add load-more sentinel observation.

## 18. Anti-Patterns

- Do not let link items drop grid semantics.
- Do not couple load-more observation to selection or focus state.
- Do not render empty-state content as a bare child without documented ownership.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the adapter owns row and cell scaffolding.
- Consumers may assume link items still participate in grid navigation.
- Consumers must not assume empty-state rendering is machine-owned.

## 20. Platform Support Matrix

| Capability / behavior                          | Browser client | SSR            | Notes                                   |
| ---------------------------------------------- | -------------- | -------------- | --------------------------------------- |
| grid semantics, selection, and action handling | full support   | full support   | Server-safe structure and attrs.        |
| load-more sentinel observation                 | client-only    | SSR-safe empty | Requires observers or scroll listeners. |
| RTL horizontal key reversal                    | full support   | full support   | Directional mapping is adapter-owned.   |

## 21. Debug Diagnostics and Production Policy

| Condition                                      | Debug build behavior | Production behavior | Notes                                        |
| ---------------------------------------------- | -------------------- | ------------------- | -------------------------------------------- |
| duplicate item keys                            | fail fast            | fail fast           | Grid identity depends on stable unique keys. |
| load-more sentinel configured without callback | debug warning        | no-op               | Sentinel should not render if unused.        |

## 22. Shared Adapter Helper Notes

| Helper concept    | Required? | Responsibility                                 | Reused by            | Notes                                   |
| ----------------- | --------- | ---------------------------------------------- | -------------------- | --------------------------------------- |
| collection helper | required  | Stable row grouping and selection observation. | `table`, `tag-group` | Keep key identity authoritative.        |
| load-more helper  | optional  | Sentinel threshold observation.                | `table`              | Avoid duplicate requests while loading. |

## 23. Framework-Specific Behavior

Leptos 0.8.x can render rows with keyed iteration and preserve item identity even when cells move between rows after a column-count change. The sentinel observer should be attached in a client-only effect.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn GridList(items: StaticCollection<grid_list::ItemDef>, columns: usize, render_item: GridListRenderItem) -> impl IntoView {
    let machine = use_machine::<grid_list::Machine>(grid_list::Props {
        items: items.clone(),
        columns,
        ..Default::default()
    });

    view! {
        <div {..attr_map_to_leptos(machine.derive(|api| api.root_attrs()).get(), &use_style_strategy(), None).attrs}>
            <For
                each=move || chunk_items(items.clone(), columns)
                key=|row| row.first().map(|item| item.key.clone()).unwrap_or_default()
                children=move |row| view! {
                    <div {..attr_map_to_leptos(machine.derive(|api| api.row_attrs(/* row index */ 0)).get(), &use_style_strategy(), None).attrs}>
                        {row.into_iter().map(|item| view! {
                            <div {..attr_map_to_leptos(machine.derive(move |api| api.cell_attrs(&item.key)).get(), &use_style_strategy(), None).attrs}>
                                {(render_item.0)(item)}
                            </div>
                        }).collect_view()}
                    </div>
                }
            />
        </div>
    }
}
```

## 25. Reference Implementation Skeleton

The implementation should keep one machine, one derived row-grouping helper, optional sentinel observation, and one observer path for controlled selection callbacks. Empty state and sentinel rendering should be siblings after the item rows, never hidden inside the render callback.

## 26. Adapter Invariants

- Row and cell hosts remain adapter-owned.
- Link items retain grid semantics and native link activation.
- Empty state and sentinel rendering stay outside the item renderer.

## 27. Accessibility and SSR Notes

- Composite mode uses grid semantics with a single tab stop.
- Non-composite mode may use list semantics, but that branch must still document how selection and action behavior degrade.
- Empty-state changes that replace prior content should be announced through a shared live-region helper when the product surface requires it.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit adapter ownership of empty state and load-more rendering.
- Intentional deviations: empty-state and section/group rendering remain adapter-level concerns rather than core-machine DOM parts.

## 29. Test Scenarios

1. Arrow keys and RTL direction move focus to the correct cell.
2. Link items preserve grid semantics while activating native navigation on Enter.
3. Empty state and load-more sentinel render in the documented positions.

## 30. Test Oracle Notes

- Preferred oracle for semantics: inspect root, row, and cell roles plus `tabindex` behavior.
- Preferred oracle for load-more: simulate sentinel entry and assert callback suppression while `loading=true`.
- Verification recipe: rerender between non-empty and empty item sets and confirm empty-state placement plus root identity remain stable.

## 31. Implementation Checklist

- [ ] Row grouping and cell identity derive from keys and columns.
- [ ] Link items keep grid semantics.
- [ ] Empty state is adapter-owned and explicit.
- [ ] Sentinel observation is optional and cleanup-safe.
- [ ] Tests cover focus navigation, link items, empty state, and load-more behavior.
