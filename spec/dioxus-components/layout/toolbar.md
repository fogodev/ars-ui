---
adapter: dioxus
component: toolbar
category: layout
source: components/layout/toolbar.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Toolbar — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Toolbar`](../../components/layout/toolbar.md) contract onto Dioxus `0.7.x` compound components. The adapter preserves roving tabindex focus management, orientation and direction-aware keyboard navigation, root-disabled behavior, and the `Root`, `Item`, and `Separator` parts.

## 2. Public Adapter API

```rust
pub mod toolbar {
    #[derive(Props, Clone, PartialEq)]
    pub struct ToolbarProps {
        pub orientation: Orientation,
        pub dir: Direction,
        #[props(optional)]
        pub aria_label: Option<String>,
        #[props(default = false)]
        pub disabled: bool,
        pub children: Element,
    }

    #[component]
    pub fn Toolbar(props: ToolbarProps) -> Element

    #[derive(Props, Clone, PartialEq)]
    pub struct ItemProps {
        #[props(default = false)]
        pub disabled: bool,
        pub children: Element,
    }

    #[component]
    pub fn Item(props: ItemProps) -> Element

    #[component]
    pub fn Separator() -> Element
}
```

`Toolbar` owns the machine and descendant registration. `Item` instances register in DOM order and receive roving tabindex attrs from the root context.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `Props`.
- State parity: full parity with the core `Idle` focus-management model.
- Part parity: full parity with `Root`, repeated `Item`, and `Separator`.
- Adapter additions: explicit registration contract for items and focusable node ownership.

## 4. Part Mapping

| Core part / structure | Required?                        | Adapter rendering target                                   | Ownership                                                | Attr source             | Notes                                             |
| --------------------- | -------------------------------- | ---------------------------------------------------------- | -------------------------------------------------------- | ----------------------- | ------------------------------------------------- |
| `Root`                | required                         | `<div>`                                                    | adapter-owned                                            | `api.root_attrs()`      | Owns roving tabindex state and item registration. |
| `Item`                | required for interactive content | consumer-provided focusable child or adapter-owned wrapper | shared between adapter registration and consumer content | `api.item_attrs(index)` | Registration order defines navigation order.      |
| `Separator`           | optional                         | `<div>`                                                    | adapter-owned                                            | `api.separator_attrs()` | Decorative structural separator only.             |

## 5. Attr Merge and Ownership Rules

| Target node | Core attrs                                                                 | Adapter-owned attrs                                        | Consumer attrs                            | Merge order                                                                                                         | Ownership notes                                                               |
| ----------- | -------------------------------------------------------------------------- | ---------------------------------------------------------- | ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `Root`      | `api.root_attrs()` including `role="toolbar"`, orientation, label, and dir | keydown handler and registration context publication       | wrapper decoration attrs when exposed     | core toolbar semantics win                                                                                          | root stays adapter-owned                                                      |
| `Item`      | `api.item_attrs(index)` including roving `tabindex` and disabled markers   | registration identity, focus handlers, and ref composition | consumer item attrs and handlers          | roving tabindex and disabled attrs win; handlers compose adapter before consumer when navigation must be normalized | each item shares ownership between registration and consumer-rendered content |
| `Separator` | `api.separator_attrs()`                                                    | none beyond structural rendering                           | no consumer ownership in the base surface | core separator attrs win                                                                                            | separator remains adapter-owned                                               |

## 6. Composition / Context Contract

- `Toolbar` provides required toolbar context containing machine access, registration state, and item focus helpers.
- `Item` and `Separator` consume required root context and fail fast when rendered outside `toolbar::Toolbar`.
- No optional external context is required, though wrappers may derive default `dir` from environment before constructing root props.

## 7. Prop Sync and Event Mapping

| Adapter prop  | Mode       | Sync trigger            | Machine event / update path | Visible effect                                      | Notes                                      |
| ------------- | ---------- | ----------------------- | --------------------------- | --------------------------------------------------- | ------------------------------------------ |
| `orientation` | controlled | rerender with new props | core prop update            | changes arrow-key mapping and separator orientation | must remain stable with rendered structure |
| `dir`         | controlled | rerender with new props | core prop update            | reverses horizontal navigation in RTL               | applies only to horizontal toolbars        |
| `disabled`    | controlled | prop change after mount | core prop update            | disables navigation and marks items disabled        | roving focus is frozen while disabled      |

| UI event             | Preconditions                     | Machine event / callback path                          | Ordering notes                                                                          | Notes                                    |
| -------------------- | --------------------------------- | ------------------------------------------------------ | --------------------------------------------------------------------------------------- | ---------------------------------------- |
| root or item keydown | toolbar enabled                   | `FocusNext`, `FocusPrev`, `FocusFirst`, or `FocusLast` | keyboard navigation runs before consumer key handlers that rely on the new focus target | Home and End always target enabled items |
| item focus           | item becomes active roving target | `FocusItem(index)`                                     | registration lookup must use current DOM order                                          | keeps `tabindex="0"` on the focused item |
| toolbar blur         | focus leaves the toolbar          | `Blur`                                                 | blur clears transient navigation state after focus exits                                | does not clear item registration         |

## 8. Registration and Cleanup Contract

| Registered entity | Registration trigger | Identity key     | Cleanup trigger | Cleanup action                                               | Notes                                    |
| ----------------- | -------------------- | ---------------- | --------------- | ------------------------------------------------------------ | ---------------------------------------- |
| item registration | `Item` mount         | composite        | `Item` cleanup  | remove the item from the root registry and recompute indices | registration order must follow DOM order |
| root context      | `Root` mount         | instance-derived | `Root` cleanup  | drop toolbar context and registry                            | one registry per toolbar                 |

## 9. Ref and Node Contract

| Target part / node         | Ref required? | Ref owner                                                | Node availability                  | Composition rule                          | Notes                                                   |
| -------------------------- | ------------- | -------------------------------------------------------- | ---------------------------------- | ----------------------------------------- | ------------------------------------------------------- |
| `Root`                     | no            | adapter-owned                                            | always structural, handle optional | no composition required                   | Navigation logic is item-focused.                       |
| each `Item` focusable node | yes           | shared between adapter registration and consumer content | required after mount               | compose adapter ref with any consumer ref | Programmatic roving focus depends on live item handles. |
| `Separator`                | no            | adapter-owned                                            | always structural, handle optional | no composition required                   | Decorative only.                                        |

## 10. State Machine Boundary Rules

- machine-owned state: focused item index, orientation, direction, disabled state, and registered disabled-item metadata.
- adapter-local derived bookkeeping: live item handles and registration ordering.
- forbidden local mirrors: do not track a second focus index outside the machine or registry.
- allowed snapshot-read contexts: keydown handling, item focus, and registry maintenance.

## 11. Callback Payload Contract

| Callback                                        | Payload source           | Payload shape                      | Timing                          | Cancelable? | Notes                        |
| ----------------------------------------------- | ------------------------ | ---------------------------------- | ------------------------------- | ----------- | ---------------------------- |
| focus change callback when exposed by a wrapper | machine-derived snapshot | `{ focused_index: Option<usize> }` | after focus navigation resolves | no          | Wrapper-owned callback only. |

## 12. Failure and Degradation Rules

| Condition                                                 | Policy          | Notes                                                                                        |
| --------------------------------------------------------- | --------------- | -------------------------------------------------------------------------------------------- |
| `Item` or `Separator` rendered outside `toolbar::Toolbar` | fail fast       | Required toolbar context is missing.                                                         |
| registered item has no focusable node after mount         | fail fast       | Roving tabindex cannot function without a live focus target.                                 |
| duplicate or unstable registration order detected         | warn and ignore | The adapter should preserve the last stable registry instead of corrupting navigation state. |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                         | Notes                                                            |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | ----------------------------------------------- | ---------------------------------------------------------------- |
| toolbar root                     | instance-derived | not applicable      | not applicable                           | root structure must stay stable                 | one registry per toolbar                                         |
| items                            | composite        | yes                 | yes                                      | initial server order must match hydration order | registration identity comes from instance plus current DOM order |

## 14. SSR and Client Boundary Rules

- SSR renders the toolbar root, items, and separators with the initial roving tabindex state.
- Live item refs and keyboard listeners are runtime-only.
- Hydration or equivalent client adoption must preserve initial item order so the roving registry aligns with server-rendered tabindex attrs.

## 15. Performance Constraints

- Keep item registration incremental instead of rebuilding the entire registry on every render.
- Avoid listener churn by centralizing root-level keyboard handling where possible.
- Keep live item handles instance-scoped.

## 16. Implementation Dependencies

| Dependency                    | Required?   | Dependency type         | Why it must exist first                                     | Notes                                                 |
| ----------------------------- | ----------- | ----------------------- | ----------------------------------------------------------- | ----------------------------------------------------- |
| registration helper           | required    | shared helper           | The root needs deterministic item registration and cleanup. | Shared with carousel indicators and splitter handles. |
| focus publication helper      | required    | shared helper           | Enables programmatic movement to the current roving item.   | Shared with other roving-focus components.            |
| keyboard normalization helper | recommended | behavioral prerequisite | Centralizes RTL-aware arrow key handling.                   | Shared with navigation components.                    |

## 17. Recommended Implementation Sequence

1. Initialize `Root` and the toolbar machine.
2. Publish toolbar context and registry helpers.
3. Register `Item` descendants and compose their refs.
4. Implement roving keydown behavior and focus dispatch.
5. Add `Separator` rendering and disabled-item handling.

## 18. Anti-Patterns

- Do not hard-code item indices instead of deriving them from registration order.
- Do not let consumer handlers override roving tabindex attrs.
- Do not manage focus with stale refs or duplicate registries.

## 19. Consumer Expectations and Guarantees

- Consumers may assume keyboard navigation follows the documented orientation and direction rules.
- Consumers may assume disabled items are skipped by roving navigation.
- Consumers must not assume items can be reordered without updating registration order.

## 20. Platform Support Matrix

| Capability / behavior                          | Web          | Desktop      | Mobile       | SSR          | Notes                                                               |
| ---------------------------------------------- | ------------ | ------------ | ------------ | ------------ | ------------------------------------------------------------------- |
| roving tabindex and keyboard toolbar semantics | full support | full support | full support | full support | SSR emits initial tabindex state; runtime owns live focus movement. |

## 21. Debug Diagnostics and Production Policy

| Condition                        | Debug build behavior | Production behavior | Notes                                                 |
| -------------------------------- | -------------------- | ------------------- | ----------------------------------------------------- |
| required toolbar context missing | fail fast            | fail fast           | Compound parts must be nested under `Root`.           |
| unstable item registration order | debug warning        | warn and ignore     | Navigation should remain on the last stable registry. |

## 22. Shared Adapter Helper Notes

| Helper concept                | Required?   | Responsibility                                   | Reused by              | Notes                                           |
| ----------------------------- | ----------- | ------------------------------------------------ | ---------------------- | ----------------------------------------------- |
| registration helper           | required    | Tracks item order and cleanup deterministically. | `splitter`, `carousel` | A shared ordered-registry helper is sufficient. |
| focus publication helper      | required    | Stores and focuses the current roving target.    | navigation components  | Must compose with consumer refs.                |
| keyboard normalization helper | recommended | Normalizes RTL-aware arrow behavior.             | `splitter`, `carousel` | Shared direction-aware key mapping.             |

## 23. Framework-Specific Behavior

Dioxus can keep the toolbar registry in context and compose mounted-node ownership with consumer item nodes. Root-level keydown handling can stay stable while item attrs derive reactively from the current machine snapshot.

## 24. Canonical Implementation Sketch

```rust
pub mod toolbar {
    #[derive(Props, Clone, PartialEq)]
    pub struct ToolbarProps {
        pub children: Element,
    }

    #[component]
    pub fn Toolbar(props: ToolbarProps) -> Element {
        let machine = use_machine::<toolbar::Machine>(toolbar::Props::default());
        use_context_provider(|| Context::from_machine(machine));
        rsx! { div { {props.children} } }
    }
}
```

## 25. Reference Implementation Skeleton

```rust
pub mod toolbar {
    #[derive(Props, Clone, PartialEq)]
    pub struct ToolbarProps {
        pub children: Element,
    }

    #[component]
    pub fn Toolbar(props: ToolbarProps) -> Element {
        let machine = use_machine::<toolbar::Machine>(toolbar::Props::default());
        use_context_provider(|| Context::from_machine(machine));
        rsx! { div { ..machine.derive(|api| api.root_attrs()), {props.children} } }
    }

    #[derive(Props, Clone, PartialEq)]
    pub struct ItemProps {
        pub children: Element,
    }

    #[component]
    pub fn Item(props: ItemProps) -> Element {
        let ctx = try_use_context::<Context>()
            .expect("toolbar::Item must be used inside Toolbar");
        let registration = ctx.register_item();
        rsx! { button { ..ctx.item_attrs(registration.index), {props.children} } }
    }
}
```

## 26. Adapter Invariants

- `Toolbar` owns exactly one ordered item registry.
- Each registered item has one live focusable node after mount.
- Roving tabindex attrs always reflect the machine's focused index.

## 27. Accessibility and SSR Notes

- `aria-label` should remain consumer-provided and localized outside the toolbar adapter.
- Separator orientation must remain perpendicular to the toolbar orientation.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core `Toolbar` contract.
- Intentional deviations: none.
- Traceability note: adapter-owned registration, ref composition, and roving focus repair are promoted into explicit Dioxus rules.

## 29. Test Scenarios

- Register multiple items and verify roving tabindex attrs plus initial focus target.
- Verify horizontal LTR, horizontal RTL, and vertical keyboard navigation.
- Verify disabled items are skipped and disabled roots suppress navigation.
- Render `Item` outside `toolbar::Toolbar` and verify failure behavior.

## 30. Test Oracle Notes

- Toolbar semantics and tabindex attrs: prefer `DOM attrs`.
- Focus navigation: prefer `callback order` or live focus assertions.
- Registration behavior: prefer `context registration`.
- Missing context or missing refs: prefer `fail fast` behavior.

## 31. Implementation Checklist

- [ ] Publish one required toolbar context from `Root`.
- [ ] Register items deterministically in DOM order and clean them up on unmount.
- [ ] Compose live refs for each focusable item node.
- [ ] Preserve roving tabindex attrs and RTL-aware keyboard navigation.
- [ ] Fail fast when compound descendants are rendered outside `toolbar::Toolbar`.
