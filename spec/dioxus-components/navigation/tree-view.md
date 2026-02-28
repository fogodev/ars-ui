---
adapter: dioxus
component: tree-view
category: navigation
source: components/navigation/tree-view.md
source_foundation: foundation/09-adapter-dioxus.md
---

# TreeView — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`TreeView`](../../components/navigation/tree-view.md) contract onto Dioxus 0.7.x. The adapter preserves collection-backed ordering, expanded and selected state sync, branch or leaf rendering, scroll-into-view behavior, lazy-load hooks, rename flows, `aria-owns` fallback for non-nested rendering, and descendant cleanup.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct TreeViewProps {
    pub items: TreeCollection<TreeItem>,
    #[props(optional)]
    pub selected: Option<selection::Set>,
    #[props(optional)]
    pub expanded: Option<BTreeSet<Key>>,
    pub selection_mode: selection::Mode,
    pub selection_behavior: selection::Behavior,
    #[props(optional)]
    pub load_children: Option<tree_view::LoadChildren>,
    #[props(default = false)]
    pub renamable: bool,
    pub children: Element,
}

#[component]
pub fn TreeView(props: TreeViewProps) -> Element
```

The adapter owns visible-node iteration, descendant registration, branch or leaf part selection, scroll-into-view sequencing, and adapter-level lazy-load or rename plumbing.

## 3. Mapping to Core Component Contract

- Props parity: full parity with collection data, selected set, expanded set, selection behavior, and lazy-load or rename variants.
- State parity: full parity with focused node, expanded nodes, selected nodes, and focus-visible state.
- Part parity: full parity with `Root`, repeated `Branch`, `BranchControl`, `BranchIndicator`, `BranchText`, `BranchContent`, repeated `Leaf`, and `LeafText`.
- Adapter additions: explicit collection-backed rendering, live node registration, scroll-into-view sequencing, and `aria-owns` fallback when structure is not nested.

## 4. Part Mapping

| Core part / structure | Required?            | Adapter rendering target  | Ownership     | Attr source                       | Notes                                  |
| --------------------- | -------------------- | ------------------------- | ------------- | --------------------------------- | -------------------------------------- |
| `Root`                | required             | `<ul>` or `<div>`         | adapter-owned | `api.root_attrs()`                | owns tree role and descendant registry |
| `Branch`              | repeated             | `<li>` or `<div>`         | adapter-owned | `api.branch_attrs(key)`           | `role="treeitem"` with expand state    |
| `BranchControl`       | repeated             | `<div>` or `<a>`          | adapter-owned | `api.branch_control_attrs(key)`   | clickable row surface                  |
| `BranchIndicator`     | conditional repeated | `<span>`                  | adapter-owned | `api.branch_indicator_attrs(key)` | decorative expand affordance           |
| `BranchText`          | repeated             | `<span>`                  | adapter-owned | `api.branch_text_attrs()`         | textual label surface                  |
| `BranchContent`       | conditional repeated | `<ul>` or `<div>`         | adapter-owned | `api.branch_content_attrs(key)`   | `role="group"`                         |
| `Leaf`                | repeated             | `<li>`, `<div>`, or `<a>` | adapter-owned | `api.leaf_attrs(key)`             | treeitem with optional href            |
| `LeafText`            | repeated             | `<span>`                  | adapter-owned | `api.leaf_text_attrs()`           | textual label surface                  |

## 5. Attr Merge and Ownership Rules

| Target node              | Core attrs                                                       | Adapter-owned attrs                     | Consumer attrs                         | Merge order                          | Ownership notes                                       |
| ------------------------ | ---------------------------------------------------------------- | --------------------------------------- | -------------------------------------- | ------------------------------------ | ----------------------------------------------------- |
| `Root`                   | tree role, selection attrs, scope attrs                          | descendant registry publication         | wrapper decoration only                | tree semantics win                   | registry belongs to root                              |
| branch or leaf treeitems | expanded, selected, disabled, level, setsize, and posinset attrs | live node registration and focus hooks  | decoration attrs and trailing handlers | accessibility and ordering attrs win | branch or leaf remains semantic owner                 |
| `BranchContent`          | group role and hidden state                                      | nested or non-nested ownership strategy | decoration attrs                       | group linkage wins                   | may require `aria-owns` fallback only when non-nested |

## 6. Composition / Context Contract

`TreeView` publishes required root context containing machine access, visible-node registration, live node handles, lazy-load helpers, and rename helpers. Repeated branch or leaf surfaces consume that context and fail fast when rendered outside the root.

## 7. Prop Sync and Event Mapping

| Adapter prop                                                         | Mode       | Sync trigger            | Machine event / update path          | Visible effect                           | Notes                                            |
| -------------------------------------------------------------------- | ---------- | ----------------------- | ------------------------------------ | ---------------------------------------- | ------------------------------------------------ |
| `selected`                                                           | controlled | prop change after mount | selection sync event                 | updates selected attrs and announcements | no controlled/uncontrolled switching after mount |
| `expanded`                                                           | controlled | prop change after mount | expansion sync event                 | updates visible range and branch content | expansion order stays collection-derived         |
| `selection_mode`, `selection_behavior`, `load_children`, `renamable` | controlled | rerender with new props | core prop rebuild plus adapter hooks | updates behavior variants                | lazy-load and rename remain explicit             |

| UI event               | Preconditions                   | Machine event / callback path          | Ordering notes                                     | Notes                                         |
| ---------------------- | ------------------------------- | -------------------------------------- | -------------------------------------------------- | --------------------------------------------- |
| branch toggle          | branch has or can load children | expand or collapse event               | focus repair may precede collapse animation        | lazy load may run before expansion commits    |
| arrow navigation       | visible nodes exist             | focus-next, prev, parent, child events | scroll-into-view runs after committed focus move   | logical semantics do not swap in RTL          |
| selection activation   | selection mode allows it        | select or toggle selection event       | selection callbacks observe committed state        | link-host nodes still preserve tree semantics |
| rename start or commit | `renamable=true`                | rename variant events                  | node identity must remain stable across edit cycle | adapter owns edit surface cleanup             |

## 8. Registration and Cleanup Contract

- Each visible node registers a live handle keyed by node key on mount.
- Cleanup removes hidden, collapsed, renamed, or unmounted nodes from the registry immediately.
- Lazy-load handles, rename edit surfaces, and queued scroll work must be canceled on cleanup.

## 9. Ref and Node Contract

| Target part / node               | Ref required? | Ref owner                                        | Node availability    | Composition rule                                 | Notes                                                        |
| -------------------------------- | ------------- | ------------------------------------------------ | -------------------- | ------------------------------------------------ | ------------------------------------------------------------ |
| each visible branch or leaf node | yes           | shared between adapter and optional wrapper refs | required after mount | compose adapter node handle with any exposed ref | programmatic focus and scroll depend on live nodes           |
| `BranchContent` group            | recommended   | adapter-owned                                    | conditional          | no composition                                   | useful for `aria-owns` fallback when structure is non-nested |

## 10. State Machine Boundary Rules

- Expanded nodes, selected nodes, focused node, and typeahead or selection semantics remain core-owned.
- Visible-node iteration, live node registration, scroll-into-view sequencing, lazy-load bridge hooks, rename editor lifecycle, and `aria-owns` fallback remain adapter-owned.
- The adapter must not compute selection or expansion from stale DOM state.

## 11. Callback Payload Contract

| Callback                  | Payload source           | Payload shape                  | Timing                                        | Cancelable? | Notes                                   |
| ------------------------- | ------------------------ | ------------------------------ | --------------------------------------------- | ----------- | --------------------------------------- |
| selection-change callback | machine-derived snapshot | `{ selected: selection::Set }` | after committed selection change              | no          | wrapper-owned only                      |
| expanded-change callback  | machine-derived snapshot | `{ expanded: BTreeSet<Key> }`  | after committed expansion change              | no          | wrapper-owned only                      |
| lazy-load callback        | adapter bridge           | `{ key: Key }`                 | before or during expansion path as documented | yes         | adapter-owned bridge into async loading |

## 12. Failure and Degradation Rules

| Condition                                                | Policy             | Notes                                                                   |
| -------------------------------------------------------- | ------------------ | ----------------------------------------------------------------------- |
| node handle unavailable during adapter-driven focus move | degrade gracefully | keep logical focus state and skip DOM focus or scroll                   |
| non-nested branch rendering without usable group id      | degrade gracefully | render nested structure instead of broken `aria-owns`                   |
| lazy-load callback fails                                 | warn and ignore    | keep branch collapsed and expose error state if the variant requires it |

## 13. Identity and Key Policy

Node identity is always the collection key. Visible ordering derives from the `TreeCollection`, not from incidental DOM insertion order. Server and client must preserve initial collection order, expansion, and selection state.

## 14. SSR and Client Boundary Rules

- SSR renders the same visible node set implied by the initial collection and expanded state.
- Scroll-into-view, lazy-load, rename edit surfaces, and live node registration are client-only.
- If the adapter chooses nested DOM structure on the server, the client must preserve that branch rather than switching to `aria-owns` indirection on hydration.

## 15. Performance Constraints

- Iterate only visible nodes, not the full collection, for rendering and registry updates.
- Run scroll-into-view only after adapter-driven focus changes that actually target off-screen nodes.
- Avoid rebuilding the entire visible-node registry when only one branch expands or collapses.

## 16. Implementation Dependencies

| Dependency                  | Required?   | Dependency type     | Why it must exist first                                                       | Notes                                  |
| --------------------------- | ----------- | ------------------- | ----------------------------------------------------------------------------- | -------------------------------------- |
| collection helper           | required    | collection helper   | visible-order, level, setsize, and posinset come from the collection contract | core dependency                        |
| ordered registration helper | required    | registration helper | live node lookup depends on visible-node registration                         | shared with composites                 |
| scroll helper               | recommended | platform helper     | consistent nearest-edge scrolling is adapter-owned                            | especially for iOS Safari focus repair |

## 17. Recommended Implementation Sequence

1. Initialize the core tree-view machine with the collection data.
2. Publish root context and register visible nodes.
3. Render branch or leaf surfaces from the visible collection view.
4. Add adapter-driven focus and scroll sequencing.
5. Add lazy-load, rename, and `aria-owns` fallback paths.

## 18. Anti-Patterns

- Do not swap left or right semantics in RTL for logical tree navigation.
- Do not compute `aria-level`, `aria-setsize`, or `aria-posinset` from ad hoc DOM counting.
- Do not let collapsed descendants remain registered as visible focus targets.

## 19. Consumer Expectations and Guarantees

- Consumers may assume visible ordering comes from the collection contract.
- Consumers may assume focus and selection stay attached to stable node keys.
- Consumers must not assume all children are nested in the DOM if the adapter documents an `aria-owns` fallback path.

## 20. Platform Support Matrix

| Capability / behavior                                              | Web           | Desktop       | Mobile        | SSR           | Notes                                                        |
| ------------------------------------------------------------------ | ------------- | ------------- | ------------- | ------------- | ------------------------------------------------------------ |
| tree semantics, visible collection order, selection, and expansion | full support  | full support  | full support  | full support  | baseline tree behavior                                       |
| adapter-driven scroll-into-view                                    | full support  | full support  | full support  | client-only   | no browser scrolling on server                               |
| lazy-load and rename variants                                      | full support  | full support  | full support  | client-only   | structural shells may SSR, interactive flows are client-side |
| `aria-owns` fallback for non-nested rendering                      | fallback path | fallback path | fallback path | fallback path | use only when nested structure is not possible               |

## 21. Debug Diagnostics and Production Policy

| Condition                                       | Debug build behavior | Production behavior | Notes                             |
| ----------------------------------------------- | -------------------- | ------------------- | --------------------------------- |
| visible node key missing from collection lookup | fail fast            | fail fast           | contract violation                |
| stale visible-node registry after collapse      | debug warning        | warn and ignore     | cleanup must converge immediately |

## 22. Shared Adapter Helper Notes

| Helper concept              | Required?   | Responsibility                                     | Reused by                 | Notes                        |
| --------------------------- | ----------- | -------------------------------------------------- | ------------------------- | ---------------------------- |
| collection helper           | required    | derives visible nodes and ARIA positional metadata | collection-backed widgets | core source of truth         |
| ordered registration helper | required    | stores live handles by visible node key            | roving composites         | cleanup critical on collapse |
| scroll helper               | recommended | performs nearest-edge scroll with iOS focus repair | focus-moving widgets      | only on adapter-driven moves |

## 23. Framework-Specific Behavior

Dioxus should keep the visible-node registry in context, compose each node handle with any wrapper ref, and sync controlled selected or expanded values through normal prop updates after mount.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<tree_view::Machine>(props);
use_context_provider(|| Context::from_machine(machine));

rsx! {
    ul { ..machine.derive(|api| api.root_attrs()),
        // render visible branches and leaves from the collection
    }
}
```

## 25. Reference Implementation Skeleton

- Initialize the machine from current collection props.
- Register visible nodes by key and remove them immediately when visibility changes.
- Route focus moves through the core machine first, then perform adapter-owned DOM focus and scrolling.
- Keep nested DOM structure as the default and use `aria-owns` only for documented non-nested fallbacks.

## 26. Adapter Invariants

- Visible-node order always follows the collection.
- Collapsed descendants are never left registered as focusable visible nodes.
- Adapter-driven focus changes scroll only as needed and preserve logical focus when DOM focus cannot move.
- `aria-owns` is used only when nested rendering is not possible.

## 27. Accessibility and SSR Notes

- `Root` owns `role="tree"` and `aria-multiselectable` when appropriate.
- Branches and leaves keep logical tree semantics in both LTR and RTL.
- SSR must preserve the same visible set and nesting strategy the client hydrates.

## 28. Parity Summary and Intentional Deviations

- Matches the core tree-view contract without intentional adapter divergence.
- Promotes collection-backed rendering, visible-node registration, scroll sequencing, lazy-load hooks, rename lifecycle, and `aria-owns` fallback into explicit Dioxus-facing rules.

## 29. Test Scenarios

- collection-backed visible-node rendering with correct positional ARIA attrs
- branch expand or collapse with descendant cleanup
- logical arrow navigation in both LTR and RTL
- adapter-driven scroll-into-view after off-screen focus move
- lazy-load branch expansion
- rename lifecycle with stable node key identity

## 30. Test Oracle Notes

- Inspect DOM attrs for `role="tree"`, `role="treeitem"`, `role="group"`, `aria-expanded`, `aria-selected`, `aria-level`, `aria-setsize`, and `aria-posinset`.
- Verify collapsed descendants are removed from the live registry before the next focus move.
- Use browser tests to confirm off-screen focus moves call nearest-edge scrolling rather than full-page jumps.
- Assert `aria-owns` appears only in the documented non-nested fallback branch.

## 31. Implementation Checklist

- [ ] Render visible nodes from the collection rather than ad hoc DOM traversal.
- [ ] Register and clean up live node handles by visible node key.
- [ ] Preserve logical left/right tree navigation in RTL.
- [ ] Apply scroll-into-view only after adapter-driven focus changes.
- [ ] Keep lazy-load and rename flows explicit and cleanup-safe.
- [ ] Use `aria-owns` only for documented non-nested rendering fallbacks.
