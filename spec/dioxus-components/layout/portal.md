---
adapter: dioxus
component: portal
category: layout
source: components/layout/portal.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Portal — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Portal`](../../components/layout/portal.md) contract onto a Dioxus `0.7.x` component. The adapter preserves mount or unmount state, target-container ownership, and SSR inline fallback while defining the Dioxus-facing lifecycle, cleanup, and platform fallback behavior.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct PortalProps {
    #[props(optional)]
    pub id: Option<String>,
    pub container: PortalTarget,
    #[props(default = false)]
    pub ssr_inline: bool,
    pub children: Element,
}

#[component]
pub fn Portal(props: PortalProps) -> Element
```

The adapter surface matches the core props. It does not add a separate adapter-only disable prop.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `Props`.
- State parity: full parity with the core `Unmounted` and `Mounted` lifecycle states.
- Part parity: full parity with the core `Root` mount node.
- Adapter additions: explicit Dioxus target-resolution behavior, cleanup, and non-web fallback policy.

## 4. Part Mapping

| Core part / structure | Required?             | Adapter rendering target                                                                              | Ownership     | Attr source        | Notes                                                                                      |
| --------------------- | --------------------- | ----------------------------------------------------------------------------------------------------- | ------------- | ------------------ | ------------------------------------------------------------------------------------------ |
| `Root`                | required when mounted | detached `<div>` appended to the resolved host container on web, or host fallback container elsewhere | adapter-owned | `api.root_attrs()` | The owned mount node is the Root; do not create a child wrapper with duplicate root attrs. |

## 5. Attr Merge and Ownership Rules

| Target node       | Core attrs                                                                      | Adapter-owned attrs                                | Consumer attrs                                      | Merge order                          | Ownership notes                  |
| ----------------- | ------------------------------------------------------------------------------- | -------------------------------------------------- | --------------------------------------------------- | ------------------------------------ | -------------------------------- |
| portal mount root | `api.root_attrs()` including `id`, `data-ars-portal-id`, owner, and mount state | cleanup bookkeeping and optional boundary metadata | no direct consumer attr ownership at the mount node | core `id` and `data-ars-*` attrs win | mount root remains adapter-owned |

- Consumer children own only the content rendered inside the portal root.
- The adapter must not let consumer content replace or delete the documented mount node.
- `api.root_attrs()` is applied to the owned mount node itself. It is not applied
  to a nested wrapper under `ars_dom::ensure_portal_mount_root(api.owner_id())`.

## 6. Composition / Context Contract

`Portal` is standalone. It provides no context and consumes no required context. Overlay components rendered inside the portal may consume separate overlay or focus-management contexts, but that is outside the portal contract.

## 7. Prop Sync and Event Mapping

| Adapter prop | Mode       | Sync trigger            | Machine event / update path                                                   | Visible effect                                                                      | Notes                                             |
| ------------ | ---------- | ----------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------- |
| `container`  | controlled | prop change after mount | core `on_props_changed` emits `SetContainer`; adapter resolves the new target | moves or recreates the portal root under the new target                             | target switching must clean up the old root first |
| `ssr_inline` | controlled | render-time only        | `Api::should_render_inline()`                                                 | renders children inline during SSR instead of an empty declaration-site placeholder | not a post-mount reactive behavior                |

| UI or lifecycle event | Preconditions                              | Machine event / callback path | Ordering notes                                       | Notes                                                                                         |
| --------------------- | ------------------------------------------ | ----------------------------- | ---------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| component mount       | platform can supply a target container     | `Mount`                       | create root before child content is rendered into it | web uses a detached DOM root; other platforms may fall back to inline or host-native mounting |
| component cleanup     | root currently mounted                     | `Unmount`                     | remove root after child teardown                     | cleanup is adapter-owned                                                                      |
| late target discovery | `PortalTarget::Id` not initially available | `ContainerReady`              | late resolution must not leak orphan roots           | core accepts only the matching ID and marks the portal mounted                                |

## 8. Registration and Cleanup Contract

| Registered entity                   | Registration trigger                                   | Identity key               | Cleanup trigger                    | Cleanup action                                           | Notes                                     |
| ----------------------------------- | ------------------------------------------------------ | -------------------------- | ---------------------------------- | -------------------------------------------------------- | ----------------------------------------- |
| portal mount root                   | first successful mount                                 | instance-derived portal id | component cleanup or target switch | remove or release the mount root from its current parent | exactly one live root per portal instance |
| late-target watcher or retry helper | `PortalTarget::Id` target missing at mount time on web | instance-derived portal id | target found or component cleanup  | stop watching for the missing target                     | watcher scope is per instance only        |

## 9. Ref and Node Contract

| Target part / node           | Ref required? | Ref owner     | Node availability                  | Composition rule                          | Notes                                                                       |
| ---------------------------- | ------------- | ------------- | ---------------------------------- | ----------------------------------------- | --------------------------------------------------------------------------- |
| declaration-site placeholder | no            | adapter-owned | always structural, handle optional | no composition required                   | Only needed when the render branch leaves a placeholder in tree position.   |
| portal mount root            | yes           | adapter-owned | required after mount               | no consumer composition at the mount node | The adapter needs a live handle to move and remove portaled content safely. |

## 10. State Machine Boundary Rules

- machine-owned state: mounted or unmounted state and the logical target stored in core context.
- adapter-local derived bookkeeping: resolved host target handle and any late-target watcher handles.
- forbidden local mirrors: do not mirror mount state in a separate local signal that can diverge from the core lifecycle.
- allowed snapshot-read contexts: mount effects, cleanup, and target-switch handling only.

## 11. Callback Payload Contract

| Callback | Payload source | Payload shape | Timing         | Cancelable? | Notes                                                         |
| -------- | -------------- | ------------- | -------------- | ----------- | ------------------------------------------------------------- |
| none     | none           | none          | not applicable | no          | `Portal` exposes no public callbacks in this adapter surface. |

## 12. Failure and Degradation Rules

| Condition                                             | Policy             | Notes                                                                                         |
| ----------------------------------------------------- | ------------------ | --------------------------------------------------------------------------------------------- |
| target container is missing at first mount            | degrade gracefully | Keep the portal unmounted while retrying late resolution for ID-based web targets.            |
| true teleportation is unavailable on the current host | degrade gracefully | Fall back to inline or host-native rendering while preserving the logical portal contract.    |
| target container never appears                        | warn and ignore    | The declaration-site tree remains stable; the adapter must not leak watchers or orphan roots. |
| host APIs unavailable during SSR                      | no-op              | SSR follows the `ssr_inline` render branch and performs no host reparenting.                  |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                                                         | Notes                         |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | ------------------------------------------------------------------------------- | ----------------------------- |
| portal mount root                | instance-derived | no                  | not applicable                           | portal id and mount ownership must stay stable for the lifetime of the instance | One root per portal instance. |

## 14. SSR and Client Boundary Rules

- When `ssr_inline=true`, SSR renders children at the declaration site and the runtime may reattach them into a portal mount after mount where supported.
- When `ssr_inline=false`, SSR may render an empty declaration-site placeholder and wait for the runtime mount.
- Host target resolution, root creation, and cleanup are runtime-only behaviors.
- Hydration or equivalent client adoption must preserve the declaration-site branch chosen by `ssr_inline`.

## 15. Performance Constraints

- Keep target watchers and root cleanup instance-scoped.
- Avoid remounting the portal root unless the target actually changes.
- Do not perform repeated target queries once a stable live target handle exists.

## 16. Implementation Dependencies

| Dependency                 | Required? | Dependency type | Why it must exist first                                                                  | Notes                                |
| -------------------------- | --------- | --------------- | ---------------------------------------------------------------------------------------- | ------------------------------------ |
| portal target resolver     | required  | shared helper   | Converts `PortalTarget` into a live host container and centralizes late-target behavior. | Shared by other teleported overlays. |
| cleanup helper             | required  | shared helper   | Ensures mount nodes are always removed on unmount or target switch.                      | Must be instance-scoped.             |
| platform capability helper | required  | shared helper   | Distinguishes full web teleport from host fallback behavior.                             | Drives the platform matrix.          |

## 17. Recommended Implementation Sequence

1. Initialize the core portal machine and derive the portal id.
2. Branch SSR behavior from runtime behavior using `Api::should_render_inline()`.
3. Resolve the target container and create the mount root when the host can support it.
4. Render children into the mount root or documented fallback path.
5. Handle target switches and cleanup without leaking roots or watchers.

## 18. Anti-Patterns

- Do not expose adapter-only props that bypass the core portal contract.
- Do not render portal children directly into the target container without the documented mount root when teleportation is available.
- Do not leak mount nodes or late-target watchers after cleanup.

## 19. Consumer Expectations and Guarantees

- Consumers may assume each portal instance owns exactly one logical mount root when mounted.
- Consumers may assume SSR behavior follows the documented `ssr_inline` branch.
- Consumers must not assume portaled content keeps its declaration-site host position after runtime mount on supported targets.

## 20. Platform Support Matrix

| Capability / behavior                  | Web          | Desktop       | Mobile        | SSR           | Notes                                                                                             |
| -------------------------------------- | ------------ | ------------- | ------------- | ------------- | ------------------------------------------------------------------------------------------------- |
| target resolution and teleported mount | full support | fallback path | fallback path | fallback path | Web can use a detached DOM mount; other targets may need inline or host-native fallback behavior. |

## 21. Debug Diagnostics and Production Policy

| Condition                                       | Debug build behavior | Production behavior | Notes                                                                       |
| ----------------------------------------------- | -------------------- | ------------------- | --------------------------------------------------------------------------- |
| target container missing for too long           | debug warning        | warn and ignore     | The adapter should surface the unresolved target without leaking resources. |
| duplicate mount root detected for one portal id | fail fast            | fail fast           | Multiple live roots for one instance break cleanup and ownership rules.     |

## 22. Shared Adapter Helper Notes

| Helper concept         | Required? | Responsibility                                                 | Reused by              | Notes                                   |
| ---------------------- | --------- | -------------------------------------------------------------- | ---------------------- | --------------------------------------- |
| portal target resolver | required  | Resolves `PortalTarget` and handles late container appearance. | overlay adapters       | Centralizes target lookup semantics.    |
| cleanup helper         | required  | Removes old mount roots on cleanup and target switches.        | overlay adapters       | Must tolerate already-removed nodes.    |
| capability helper      | required  | Selects full teleport versus documented fallback path.         | `scroll-area`, `frame` | Needed for multi-platform parity notes. |

## 23. Framework-Specific Behavior

Dioxus web can create and own a detached mount node. Non-web targets may need inline or host-native fallback behavior, but the adapter still owns lifecycle, cleanup, and target-resolution policy.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct PortalProps {
    pub container: PortalTarget,
    #[props(default = false)]
    pub ssr_inline: bool,
    pub children: Element,
}

#[component]
pub fn Portal(props: PortalProps) -> Element {
    let machine = use_machine::<portal::Machine>(portal::Props {
        container: props.container,
        ssr_inline: props.ssr_inline,
        ..Default::default()
    });
    let render_inline = machine.derive(|api| api.should_render_inline());

    if render_inline() {
        rsx! { {props.children} }
    } else {
        rsx! {}
    }
}
```

## 25. Reference Implementation Skeleton

```rust
#[derive(Props, Clone, PartialEq)]
pub struct PortalProps {
    pub container: PortalTarget,
    #[props(default = false)]
    pub ssr_inline: bool,
    pub children: Element,
}

#[component]
pub fn Portal(props: PortalProps) -> Element {
    let machine = use_machine::<portal::Machine>(portal::Props {
        container: props.container.clone(),
        ssr_inline: props.ssr_inline,
        ..Default::default()
    });
    let mut mount_root = use_signal(|| None::<HostNode>);
    let render_inline = machine.derive(|api| api.should_render_inline());

    use_effect(move || {
        if render_inline() {
            return;
        }

        if let Some(target) = resolve_portal_target(&props.container) {
            let owner_id = machine.with_api_snapshot(|api| api.owner_id().to_string());
            let mount = ars_dom::ensure_portal_mount_root(&owner_id);
            apply_attrs(&mount, machine.derive(|api| api.root_attrs()));
            move_mount_to_target(&mount, target);
            render_children_into_mount(&mount);
            mount_root.set(Some(mount));
        }
    });

    use_drop(move || remove_portal_root(mount_root.read().clone()));

    if render_inline() {
        rsx! { {props.children} }
    } else {
        rsx! {}
    }
}
```

## 26. Adapter Invariants

- Each portal instance owns at most one live mount root.
- Cleanup always removes the owned mount root and watcher resources.
- SSR never performs target resolution or host reparenting.

## 27. Accessibility and SSR Notes

- Portal itself adds no ARIA role; the portaled content remains responsible for semantics and focus management.
- The adapter should preserve a stable portal owner id so overlay consumers can bridge outside-interaction boundaries when needed.
- On web, when the resolved target is the shared portal root, the adapter MUST delegate mount-node creation to `ars_dom::ensure_portal_mount_root(owner_id)` so the mount node carries the required `data-ars-portal-owner` marker.

## 28. Parity Summary and Intentional Deviations

- Parity summary: full parity with the core `Portal` contract on web, with documented host fallbacks elsewhere.
- Intentional deviations: non-web targets may use an inline or host-native fallback instead of true teleportation.
- Traceability note: adapter-owned mount creation, late target resolution, fallback selection, and cleanup are promoted into explicit Dioxus lifecycle rules.

## 29. Test Scenarios

- Mount into the default portal root and verify the portal root attrs plus target placement.
- Use `PortalTarget::Id` with a late-appearing target and verify late container resolution.
- Toggle `ssr_inline` and verify SSR declaration-site behavior versus runtime mount behavior.
- Unmount or switch targets and verify cleanup.
- Verify non-web fallback policy does not leak extra mount roots.

## 30. Test Oracle Notes

- Mount-root attrs: prefer `DOM attrs`.
- Mount placement and declaration-site branch: prefer `rendered structure` and `hydration structure`.
- Target switching and cleanup: prefer `cleanup side effects`.
- Host fallback policy: prefer `rendered structure`.

## 31. Implementation Checklist

- [ ] Keep the public API aligned with the core portal props.
- [ ] Create exactly one logical mount root per instance.
- [ ] Handle late target discovery without leaking roots or watchers.
- [ ] Honor `ssr_inline` exactly during SSR and runtime adoption.
- [ ] Remove mount roots on cleanup and target switches.
