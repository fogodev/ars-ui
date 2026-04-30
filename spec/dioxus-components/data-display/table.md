---
adapter: dioxus
component: table
category: data-display
source: components/data-display/table.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Table — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Table`](../../components/data-display/table.md) contract onto a Dioxus 0.7.x adapter. The adapter must preserve the table selection, expansion, sorting, and focus machine while making semantic host choice, sort announcements, sticky-header ownership, virtualization hooks, and load-more behavior explicit.

## 2. Public Adapter API

```rust,no_check
pub struct TableRenderCell(pub EventHandler<table::CellRenderCtx>);
pub struct TableRenderExpandedRow(pub EventHandler<Key>);

#[derive(Props, Clone, PartialEq)]
pub struct TableProps {
    #[props(optional)]
    pub id: Option<String>,
    pub columns: StaticCollection<table::ColumnDef>,
    pub rows: StaticCollection<table::RowDef>,
    #[props(optional)]
    pub selected_rows: Option<Signal<selection::Set>>,
    #[props(optional)]
    pub expanded_rows: Option<Signal<BTreeSet<Key>>>,
    #[props(optional)]
    pub sort_descriptor: Option<Signal<Option<SortDescriptor<String>>>>,
    #[props(optional)]
    pub selection_mode: Option<selection::Mode>,
    #[props(optional)]
    pub selection_behavior: Option<selection::Behavior>,
    #[props(optional)]
    pub disabled_keys: Option<BTreeSet<Key>>,
    #[props(default = false)]
    pub disallow_empty_selection: bool,
    #[props(default = false)]
    pub interactive: bool,
    #[props(default = false)]
    pub sticky_header: bool,
    #[props(default = false)]
    pub loading: bool,
    #[props(optional)]
    pub on_selection_change: Option<EventHandler<selection::Set>>,
    #[props(optional)]
    pub on_sort_change: Option<EventHandler<Option<SortDescriptor<String>>>>,
    #[props(optional)]
    pub on_row_action: Option<EventHandler<Key>>,
    #[props(optional)]
    pub on_load_more: Option<EventHandler<()>>,
    pub render_cell: TableRenderCell,
    #[props(optional)]
    pub render_expanded_row: Option<TableRenderExpandedRow>,
    pub empty_state: Element,
}

#[component]
pub fn Table(props: TableProps) -> Element
```

The adapter owns table scaffolding, `colgroup`, sticky-header mechanics, sort announcements, empty state, loading rows, and load-more observation. Consumers provide cell and optional expanded-row content through render callbacks only.

## 3. Mapping to Core Component Contract

- Props parity: full parity with adapter render slots and callbacks.
- Part parity: full parity for root table anatomy plus optional select-all, column-resize, empty-state, skeleton-row, and loading-sentinel structures.
- Traceability note: this spec promotes semantic `table` vs interactive `grid` rendering, live sort announcements, load-more wiring, virtualization hooks, row-link handling, and column-width ownership from the agnostic spec.

## 4. Part Mapping

| Core part / structure             | Required? | Adapter rendering target                      | Ownership     | Attr source                                   | Notes                                                              |
| --------------------------------- | --------- | --------------------------------------------- | ------------- | --------------------------------------------- | ------------------------------------------------------------------ |
| `Root` / `Table`                  | required  | `<table>` inside an optional scroll container | adapter-owned | `api.root_attrs()`                            | `role="table"` by default, `role="grid"` when `interactive=true`.  |
| `Caption`                         | optional  | `<caption>`                                   | adapter-owned | `api.caption_attrs()`                         | Stable descriptive text.                                           |
| `ColumnHeader` / header row       | repeated  | `<th>` inside `<thead>`                       | adapter-owned | `api.column_header_attrs(column)`             | Sort state and resize handles live here.                           |
| `Row`                             | repeated  | `<tr>`                                        | adapter-owned | `api.row_attrs(key)`                          | Selection, expansion, disabled, and link affordances are explicit. |
| `Cell`                            | repeated  | `<td>`                                        | adapter-owned | `api.cell_attrs(cell)`                        | Roving tabindex only in interactive mode.                          |
| `ExpandedRow`                     | optional  | `<tr>`                                        | adapter-owned | adapter-local attrs plus core expansion state | Rendered after its owning row.                                     |
| `SelectAll`                       | optional  | `<input>` or adapter-owned control            | adapter-owned | `api.select_all_attrs()`                      | Only in multiple selection mode.                                   |
| `ColumnResizeHandle`              | optional  | `<div>`                                       | adapter-owned | `api.column_resize_handle_attrs(column)`      | Only when resizing is enabled.                                     |
| `EmptyState`                      | optional  | adapter-owned row or wrapper                  | adapter-owned | adapter-local attrs                           | Rendered when there are no rows.                                   |
| `SkeletonRow` / `LoadingSentinel` | optional  | adapter-owned rows or sentinel div            | adapter-owned | adapter-local attrs                           | Adapter concerns for loading and infinite scroll.                  |

## 5. Attr Merge and Ownership Rules

- Core attrs include row selection, expansion, sort, disabled, focus, and resize state.
- The adapter owns table vs grid semantics, `colgroup` widths, sticky-header styles, live-sort-region wiring, and row-link affordances.
- Consumer decoration must not replace `<table>`, `<tr>`, `<th>`, or `<td>` hosts or remove required ARIA and data attrs.
- Column widths are adapter-owned once resizing or sticky-header alignment is active.

## 6. Composition / Context Contract

`Table` is self-contained. It may internally provide lightweight row or cell metadata to render helpers, but it does not expose a public descendant context contract.

## 7. Prop Sync and Event Mapping

| Adapter prop / event     | Mode           | Sync trigger                 | Machine event / update path         | Notes                                                              |
| ------------------------ | -------------- | ---------------------------- | ----------------------------------- | ------------------------------------------------------------------ |
| `selected_rows`          | controlled     | signal change after mount    | selection events                    | Supports `selection::Set::All`.                                    |
| `expanded_rows`          | controlled     | signal change after mount    | expansion events                    | Adapter must keep expanded-row structure stable.                   |
| `sort_descriptor`        | controlled     | signal change after mount    | `SortColumn` and direct observation | Sort state lives in context, not a separate sorting state variant. |
| header activation        | adapter event  | click or keyboard            | `SortColumn`                        | Follow single- or multi-sort rules from core state.                |
| row selection and action | adapter event  | click or keyboard            | selection events and `RowAction`    | Space selects, Enter activates.                                    |
| focus navigation         | adapter event  | grid keyboard movement       | `Focus` / `Blur`                    | Only when `interactive=true`.                                      |
| load-more threshold      | adapter effect | sentinel or scroll threshold | adapter callback only               | Suppressed while `loading=true`.                                   |

## 8. Registration and Cleanup Contract

- The adapter owns registration of rows and cells for roving focus, resize handles, and optional virtualization.
- Sticky-header and column-resize helpers may register observers or pointer listeners; cleanup must remove them on unmount.
- If load-more or virtualization is enabled, sentinel or scroll listeners must be disposed cleanly and ignore stale callbacks.

## 9. Ref and Node Contract

The table container requires a live ref for sticky-header alignment, load-more thresholds, and virtualization hooks. Header and cell refs are optional but required for keyboard grid focus or column-resize implementations that need live geometry.

## 10. State Machine Boundary Rules

- Machine-owned state: selected rows, expanded rows, sort descriptor, focused cell, and row focus state.
- Adapter-owned derived values: column widths, sticky offsets, virtualized visible range, exclusion sets for paginated `selection::Set::All`, and sort-announcement text.
- Forbidden mirror: do not keep a second source of truth for sort state outside `sort_descriptor`.

## 11. Callback Payload Contract

| Callback              | Payload source              | Payload shape                    | Timing                                   | Cancelable? | Notes                                     |
| --------------------- | --------------------------- | -------------------------------- | ---------------------------------------- | ----------- | ----------------------------------------- |
| `on_selection_change` | adapter observation         | `selection::Set`                 | after machine selection updates          | no          | Preserve `All` semantics when applicable. |
| `on_sort_change`      | adapter observation         | `Option<SortDescriptor<String>>` | after sort descriptor changes            | no          | Emit the post-update descriptor.          |
| `on_row_action`       | adapter event               | `Key`                            | after row action resolution              | no          | Distinct from selection.                  |
| `on_load_more`        | sentinel/scroll observation | `()`                             | when the configured threshold is crossed | no          | Suppressed while loading.                 |

## 12. Failure and Degradation Rules

| Condition                                            | Policy             | Notes                                                       |
| ---------------------------------------------------- | ------------------ | ----------------------------------------------------------- |
| sticky header unsupported in the current environment | fallback path      | Render a normal scrolling table without sticky positioning. |
| virtualization helper unavailable                    | degrade gracefully | Render the full row set.                                    |
| live sort announcer unavailable                      | warn and ignore    | Keep visual sort state but skip announcement callback text. |
| row-link affordance requested without stable row key | fail fast          | Link and action semantics require unique row identity.      |

## 13. Identity and Key Policy

Row identity is keyed by row `Key`. Expanded rows derive identity from their owning row key. Column widths and resize handles key off stable column identifiers, not header display labels.

## 14. SSR and Client Boundary Rules

- SSR renders the initial semantic branch (`table` vs `grid`), row set, caption, and any empty or loading state.
- Sticky positioning, resize listeners, load-more thresholds, and virtualization hooks start only after mount.
- Skeleton rows and empty state must occupy stable positions between server and client.

## 15. Performance Constraints

- Reuse derived row metadata and column width maps across renders.
- Keep one observer or scroll-listener path per instance for load more and virtualization.
- Sort announcements should derive from changed descriptors only, not every render.

## 16. Implementation Dependencies

| Dependency               | Required?   | Dependency type        | Why it must exist first                                   | Notes                                    |
| ------------------------ | ----------- | ---------------------- | --------------------------------------------------------- | ---------------------------------------- |
| collection helper        | required    | selection/focus helper | Provides stable row identity and selection integration.   | Shared with `grid-list` and `tag-group`. |
| live announcement helper | recommended | accessibility helper   | Announces sort changes and bulk updates.                  | Reuse a single hidden region.            |
| measurement helper       | recommended | layout helper          | Drives sticky headers, column sizing, and virtualization. | Instance-scoped only.                    |

## 17. Recommended Implementation Sequence

1. Initialize the machine and controlled prop sync.
2. Render the semantic table structure and cell callbacks.
3. Add sort, selection, expansion, and row-action wiring.
4. Layer sticky-header, resize, and live-sort announcement behavior.
5. Add empty state, loading rows, load-more observation, and virtualization hooks.

## 18. Anti-Patterns

- Do not render a div-based grid when semantic table markup is still viable.
- Do not split sort state between local adapter flags and the machine descriptor.
- Do not make row links bypass selection or row-action rules without documenting the precedence.

## 19. Consumer Expectations and Guarantees

- Consumers may assume table scaffolding, sticky alignment, and column-width ownership are adapter concerns.
- Consumers may assume sort announcements and selection callbacks reflect post-machine state.
- Consumers must not assume virtualization or sticky headers are required for correctness.

## 20. Platform Support Matrix

| Capability / behavior                         | Web           | Desktop       | Mobile        | SSR            | Notes                                                   |
| --------------------------------------------- | ------------- | ------------- | ------------- | -------------- | ------------------------------------------------------- |
| semantic table and interactive grid rendering | full support  | full support  | full support  | full support   | Structural branch must be hydration-stable.             |
| sticky header and column resize               | client-only   | client-only   | client-only   | SSR-safe empty | Requires live layout measurement and pointer listeners. |
| live sort announcements                       | client-only   | client-only   | client-only   | SSR-safe empty | Hidden announcer helper runs after mount.               |
| virtualization and load-more observation      | fallback path | fallback path | fallback path | SSR-safe empty | May degrade to full render and manual pagination.       |

## 21. Debug Diagnostics and Production Policy

| Condition                                            | Debug build behavior | Production behavior | Notes                              |
| ---------------------------------------------------- | -------------------- | ------------------- | ---------------------------------- |
| duplicate row or column keys                         | fail fast            | fail fast           | Identity is foundational.          |
| sort callback or announcer observes stale descriptor | debug warning        | no-op               | Post-update snapshots only.        |
| sticky header enabled without measurable container   | debug warning        | degrade gracefully  | Fall back to non-sticky rendering. |

## 22. Shared Adapter Helper Notes

| Helper concept           | Required?   | Responsibility                                         | Reused by                | Notes                                               |
| ------------------------ | ----------- | ------------------------------------------------------ | ------------------------ | --------------------------------------------------- |
| collection helper        | required    | Stable selection, focus, and row identity management.  | `grid-list`, `tag-group` | Prefer key-based state.                             |
| measurement helper       | recommended | Sticky alignment, resize widths, virtualization range. | measured layouts         | Keep observers batched and cleanup-safe.            |
| live announcement helper | recommended | Sort and bulk-update announcements.                    | `tag-group`, `meter`     | One hidden region per component instance is enough. |

## 23. Framework-Specific Behavior

Dioxus 0.7.x should keep row and column derivations memoized while wiring sticky-header and virtualization observers through client-only effects. When `selection::Set::All` is used against paginated data, the adapter may maintain an exclusion set alongside the canonical selection state for server interactions, but it must not replace the machine's exposed selection descriptor.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct TableSketchProps {
    pub columns: StaticCollection<table::ColumnDef>,
    pub rows: StaticCollection<table::RowDef>,
    pub render_cell: TableRenderCell,
}

#[component]
pub fn Table(props: TableSketchProps) -> Element {
    let machine = use_machine::<table::Machine>(table::Props {
        columns: props.columns.clone(),
        rows: props.rows.clone(),
        ..Default::default()
    });
    let strategy = use_style_strategy();

    rsx! {
        table {
            ..attr_map_to_dioxus(machine.derive(|api| api.root_attrs())(), &strategy, None).attrs,
            thead {
                tr {
                    for column in props.columns.iter() {
                        th { ..attr_map_to_dioxus(machine.derive(move |api| api.column_header_attrs(column))(), &strategy, None).attrs }
                    }
                }
            }
            tbody {
                for row in props.rows.iter() {
                    tr {
                        ..attr_map_to_dioxus(machine.derive(move |api| api.row_attrs(&row.key))(), &strategy, None).attrs,
                        for cell in row.cells.iter() {
                            td {
                                ..attr_map_to_dioxus(machine.derive(move |api| api.cell_attrs(cell))(), &strategy, None).attrs,
                                props.render_cell.0.call(cell.clone())
                            }
                        }
                    }
                }
            }
        }
    }
}
```

## 25. Reference Implementation Skeleton

The implementation should keep one machine, one controlled-prop sync layer, one sort-announcement helper, one measurement helper for sticky headers and resize widths, and one optional virtualization or load-more observer path. Empty state and skeleton rows should be rendered from dedicated adapter branches outside `render_cell`.

## 26. Adapter Invariants

- Semantic table markup remains the default.
- Sort announcements reflect descriptor changes, not inferred DOM state.
- Sticky-header and resize behavior never become the only source of column-width truth.

## 27. Accessibility and SSR Notes

- `interactive=true` upgrades semantics to grid behavior and roving focus.
- Sort changes should be announced through a polite live region using localized column names.
- Empty state, skeleton rows, and expanded rows must preserve valid table structure on the server.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit adapter ownership of empty state, load more, sticky headers, and virtualization hooks.
- Intentional deviations: virtualization, row-link affordances, and column-resize helpers remain adapter-level implementation concerns rather than core DOM parts.

## 29. Test Scenarios

1. Interactive mode renders grid semantics and roving focus, while non-interactive mode remains a table.
2. Sort changes update header attrs, callback payloads, and live-announcement text together.
3. Sticky header, skeleton rows, empty state, and load-more sentinel occupy the documented structural positions.

## 30. Test Oracle Notes

- Preferred oracle for semantics: inspect table, row, and cell roles plus focus attrs.
- Preferred oracle for sort behavior: assert descriptor change, callback payload, and live-announcement text from one interaction.
- Verification recipe: toggle loading, empty, and paginated states and confirm structural branches stay valid and cleanup-safe.

## 31. Implementation Checklist

- [ ] Table vs grid semantics are explicit and hydration-stable.
- [ ] Sort state comes only from `sort_descriptor`.
- [ ] Sticky header and colgroup widths are adapter-owned.
- [ ] Empty state, skeleton rows, and load-more sentinel are explicit adapter branches.
- [ ] Tests cover semantics, sort announcements, loading, and virtualization fallback.
