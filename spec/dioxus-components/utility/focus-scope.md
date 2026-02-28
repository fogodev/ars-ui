---
adapter: dioxus
component: focus-scope
category: utility
source: components/utility/focus-scope.md
source_foundation: foundation/09-adapter-dioxus.md
---

# FocusScope — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`FocusScope`](../../components/utility/focus-scope.md) machine to Dioxus 0.7.x.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct FocusScopeProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(default = false)]
    pub trapped: bool,
    #[props(default = false)]
    pub contain: bool,
    #[props(optional, default = true)]
    pub auto_focus: bool,
    #[props(optional, default = true)]
    pub restore_focus: bool,
    pub children: Element,
}

#[component]
pub fn FocusScope(props: FocusScopeProps) -> Element
```

The adapter exposes the full core focus-scope prop set for trapping, looping, autofocus, and restore-focus behavior.

## 3. Mapping to Core Component Contract

- Props parity: full parity.
- Event parity: focus containment and restoration are adapter-driven on top of the core machine.

## 4. Part Mapping

| Core part / structure      | Required?                | Adapter rendering target                   | Ownership      | Attr source                                    | Notes                                        |
| -------------------------- | ------------------------ | ------------------------------------------ | -------------- | ---------------------------------------------- | -------------------------------------------- |
| `Container`                | required                 | wrapper `<div>` or any chosen root element | adapter-owned  | `api.part_attrs(focus_scope::Part::Container)` | Focus trap boundary.                         |
| trapped descendants region | required structural node | consumer children inside container         | consumer-owned | none                                           | Focus stays within this region while active. |

## 5. Attr Merge and Ownership Rules

| Target node           | Core attrs                                 | Adapter-owned attrs                                     | Consumer attrs      | Merge order                                                     | Ownership notes                                         |
| --------------------- | ------------------------------------------ | ------------------------------------------------------- | ------------------- | --------------------------------------------------------------- | ------------------------------------------------------- |
| `Container`           | `api.part_attrs(Part::Container)`          | focus-sentinel or guard attrs if the strategy uses them | consumer root attrs | core focus-boundary attrs win; `class`/`style` merge additively | adapter-owned scope container                           |
| helper nodes (if any) | none from core unless modeled as structure | sentinel, guard, or helper attrs                        | none                | adapter attrs apply directly                                    | helper nodes are adapter-owned and never consumer-owned |

## 6. Composition / Context Contract

May compose with `ArsProvider` for scoped DOM queries and with `Dismissable` inside the same container.
When an adapter chooses to expose programmatic navigation, it should publish a `FocusManager`-style context with `focus_next`, `focus_previous`, `focus_first`, and `focus_last` helpers for descendants that need imperative focus movement without bypassing the scope contract.

## 7. Prop Sync and Event Mapping

Focus-scope behavior is mostly interaction-driven. Configuration props are usually init-only unless a wrapper explicitly reinitializes the scope.

| Adapter prop                        | Mode                      | Sync trigger     | Machine event / update path | Visible effect                               | Notes                                               |
| ----------------------------------- | ------------------------- | ---------------- | --------------------------- | -------------------------------------------- | --------------------------------------------------- |
| trap / loop / restore configuration | non-reactive adapter prop | render time only | initial machine props       | determines trapping and restoration behavior | dynamic changes require documented reinitialization |

| UI event            | Preconditions                           | Machine event / callback path | Ordering notes                                   | Notes                                               |
| ------------------- | --------------------------------------- | ----------------------------- | ------------------------------------------------ | --------------------------------------------------- |
| initial activation  | client-side mount with trapping enabled | scope-activation path         | activation occurs after the container ref exists | no SSR focus work                                   |
| `Tab` / `Shift+Tab` | active scope and trapping enabled       | normalized focus-wrap path    | scope registry lookup precedes focus movement    | keeps focus inside the container                    |
| cleanup / unmount   | scope active                            | restore-focus path            | nested scopes unwind in stack order              | restore fallback applies if prior target is missing |

## 8. Registration and Cleanup Contract

- Each active focus scope registers in a scope stack on mount.
- Nested scopes must unwind in last-in, first-out order.
- Helper nodes or sentinels, if used, are owned by the scope instance and cleaned up before focus restoration completes.

| Registered entity        | Registration trigger                    | Identity key                        | Cleanup trigger | Cleanup action                                 | Notes               |
| ------------------------ | --------------------------------------- | ----------------------------------- | --------------- | ---------------------------------------------- | ------------------- |
| active scope stack entry | client-side mount with scope activation | scope instance                      | scope cleanup   | remove stack entry and run restore-focus logic | no SSR registration |
| helper nodes / sentinels | helper-node render                      | scope instance plus helper identity | scope cleanup   | remove helper nodes and drop refs              | optional strategy   |

## 9. Ref and Node Contract

| Target part / node                  | Ref required?                           | Ref owner                      | Node availability    | Composition rule                                                    | Notes                                                                            |
| ----------------------------------- | --------------------------------------- | ------------------------------ | -------------------- | ------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `Container`                         | yes                                     | adapter-owned                  | required after mount | compose only when wrappers expose the scope container               | The trap boundary requires a concrete node.                                      |
| helper nodes or sentinels when used | yes                                     | adapter-owned                  | client-only          | no consumer composition unless the strategy explicitly exposes them | These handles are part of the trap implementation, not consumer-owned structure. |
| restore-focus target                | yes if a previously focused node exists | adapter-owned transient handle | client-only          | not compositional                                                   | Stored as a transient node handle, not a rendered part.                          |

## 10. State Machine Boundary Rules

- machine-owned state: active scope status, nesting behavior, trap/loop semantics, and restore-focus decision rules.
- adapter-local derived bookkeeping: transient previous-focus handle, helper-node refs, and active-scope stack handles.
- forbidden local mirrors: do not fork a parallel active/inactive scope flag separate from the machine/scope stack.
- allowed snapshot-read contexts: mount activation effects, Tab handlers, restore-focus cleanup, and render derivation.

## 11. Callback Payload Contract

| Callback                                                                     | Payload source | Payload shape | Timing         | Cancelable? | Notes                                                                                          |
| ---------------------------------------------------------------------------- | -------------- | ------------- | -------------- | ----------- | ---------------------------------------------------------------------------------------------- |
| no public adapter-specific callback unless a wrapper exposes lifecycle hooks | none           | none          | not applicable | no          | Focus restoration and trap behavior remain internal unless a wrapper explicitly surfaces them. |

## 12. Failure and Degradation Rules

| Condition                                | Policy             | Notes                                                                    |
| ---------------------------------------- | ------------------ | ------------------------------------------------------------------------ |
| scope container ref missing after mount  | fail fast          | Focus trapping requires the container node.                              |
| previous focus target missing on restore | degrade gracefully | Fall back to the documented restore target behavior instead of erroring. |
| browser focus APIs absent during SSR     | no-op              | Render the structure and defer focus logic to the client.                |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed?   | DOM order must match registration order? | SSR/hydration stability                             | Notes                                        |
| -------------------------------- | ---------------- | --------------------- | ---------------------------------------- | --------------------------------------------------- | -------------------------------------------- |
| active scope stack entry         | instance-derived | yes for nested scopes | stack order must match activation order  | scope structure must remain stable across hydration | Nested scopes are keyed by scope instance.   |
| helper-node identities           | composite        | yes                   | not applicable                           | helper nodes are client-only                        | Identity is scope instance plus helper role. |

## 14. SSR and Client Boundary Rules

- SSR must render the scope container and trapped descendants region.
- Scope activation, helper-node setup, and restore-focus logic are client-only.
- Container/helper refs are server-safe absent and required after mount.
- Nested-scope structural ownership must remain consistent across hydration.

## 15. Performance Constraints

- Active-scope registration must only change on mount/unmount or activation changes, not every render.
- Helper-node setup should be instance-scoped and must not churn when the trapped children rerender.
- Restore-focus cleanup should run once per scope cleanup and should not scan the DOM repeatedly when a stored target exists.

## 16. Implementation Dependencies

| Dependency     | Required?   | Dependency type         | Why it must exist first                                                   | Notes                                         |
| -------------- | ----------- | ----------------------- | ------------------------------------------------------------------------- | --------------------------------------------- |
| `ars-provider` | recommended | context contract        | Scoped DOM access can simplify focus queries in complex trees.            | Helpful for advanced composed consumers.      |
| `dismissable`  | recommended | behavioral prerequisite | Overlay shells often combine focus trapping with dismissable containment. | Not required to implement the utility itself. |

## 17. Recommended Implementation Sequence

1. Establish the scope container ref and any helper-node strategy.
2. Activate the scope on mount using the documented machine rules.
3. Publish optional `FocusManager` context after the scope boundary and activation path are stable.
4. Wire Tab/Shift+Tab trapping and nested-scope stack behavior.
5. Add restore-focus cleanup and fallback behavior.
6. Verify client-only activation, optional context publication, and cleanup test oracles.

## 18. Anti-Patterns

- Do not activate focus trapping during SSR.
- Do not store restore targets as IDs only when the contract requires a live node handle.
- Do not let nested scopes unwind out of stack order.

## 19. Consumer Expectations and Guarantees

- Consumers may assume focus trapping and restore-focus ordering follow the active scope stack.
- Consumers may assume optional focus-manager publication, when exposed, already respects the current scope boundary.
- Consumers must not assume focus logic runs during SSR or before the container ref exists.

## 20. Platform Support Matrix

| Capability / behavior              | Web          | Desktop       | Mobile        | SSR            | Notes                                                      |
| ---------------------------------- | ------------ | ------------- | ------------- | -------------- | ---------------------------------------------------------- |
| focus trapping and restore         | full support | fallback path | fallback path | client-only    | Host focus APIs can differ outside the web DOM.            |
| optional focus-manager publication | full support | full support  | full support  | SSR-safe empty | Context may exist without live focus effects before mount. |

## 21. Debug Diagnostics and Production Policy

| Condition                                                           | Debug build behavior | Production behavior | Notes                                                                        |
| ------------------------------------------------------------------- | -------------------- | ------------------- | ---------------------------------------------------------------------------- |
| documented platform capability is unavailable on the active runtime | debug warning        | degrade gracefully  | Use the documented fallback path instead of inventing browser-only behavior. |

## 22. Shared Adapter Helper Notes

| Helper concept                    | Required?   | Responsibility                                                                                               | Reused by                                 | Notes                                                         |
| --------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------ | ----------------------------------------- | ------------------------------------------------------------- |
| focus-manager publication helper  | recommended | Publish `focus_next`, `focus_previous`, `focus_first`, and `focus_last` without leaking raw focus internals. | `focus-scope`, composed overlay utilities | Only needed when the adapter exposes programmatic navigation. |
| node-boundary registration helper | required    | Track the active scope container, helper nodes, and restore target handles.                                  | `focus-scope`, `dismissable`              | Focus restoration depends on live-node ownership.             |

## 23. Framework-Specific Behavior

Dioxus uses hook cleanup for focus restoration cleanup. When programmatic navigation is exposed, the adapter should publish a `FocusManager` through `use_context_provider(|| FocusManager { ... })` after the container ref and scope activation path exist.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct FocusScopeSketchProps {
    pub children: Element,
}

#[component]
pub fn FocusScope(props: FocusScopeSketchProps) -> Element {
    let machine = use_machine::<focus_scope::Machine>(focus_scope::Props::default());
    let attrs = machine.derive(|api| api.part_attrs(focus_scope::Part::Container));
    rsx! { div { ..attrs.read().clone(), {props.children} } }
}
```

## 25. Reference Implementation Skeleton

```rust
let machine = use_machine::<focus_scope::Machine>(props);
let container_ref = create_container_ref();
let scope_stack = create_scope_registration_helper();
let restore_target = store_previous_focus_handle();
let focus_manager = maybe_publish_focus_manager(machine, container_ref);

render_scope_container_and_optional_helper_nodes(container_ref);
activate_scope_on_mount(scope_stack, container_ref);
wire_tab_looping_and_nested_scope_order(machine, scope_stack);

on_cleanup(|| {
    scope_stack.unregister();
    restore_focus_if_needed(restore_target);
    release_helper_nodes();
});
```

## 26. Adapter Invariants

- All trapped descendants must remain inside the owned scope container used as the focus boundary.
- Nested scope activation and restore-focus ordering must remain deterministic.
- Restore-focus fallback behavior must be explicit when the previously focused element no longer exists.
- Focus sentinels, guards, or helper dismiss controls must remain inside the trapped structure whenever they are rendered.
- Client-only focus trapping logic must not execute during SSR.

## 27. Accessibility and SSR Notes

Trap behavior is client-only, but the container structure must still be documented and rendered consistently.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, part, and lifecycle parity.

Intentional deviations: none.

Traceability note: This adapter spec now makes explicit the core adapter-owned concerns for live restore-target validation, nested scope ordering, client-only trap activation, helper-node ownership, and optional focus-manager publication.

## 29. Test Scenarios

- container mapping
- trap activation
- nested scope behavior
- focus restore fallback
- descendants can access focus navigation helpers through context when the adapter exposes them

## 30. Test Oracle Notes

| Behavior                           | Preferred oracle type | Notes                                                                                             |
| ---------------------------------- | --------------------- | ------------------------------------------------------------------------------------------------- |
| nested scope stack behavior        | machine state         | Assert active-scope stack order and restoration policy.                                           |
| focus restoration                  | cleanup side effects  | Verify focus lands on the documented restore target or fallback after cleanup.                    |
| scope container/helper structure   | rendered structure    | Assert the container and any helper nodes remain correctly placed.                                |
| optional focus-manager publication | context registration  | Verify descendants can read the published focus navigation helpers when the adapter exposes them. |

Cheap verification recipe:

1. Render a scope with a focusable child, activate trapping on the client, and assert the container plus helper structure before moving focus.
2. Open nested scopes, then close them in reverse order and verify focus restores to the documented live target or fallback.
3. If the adapter publishes `FocusManager`, read it from a descendant and verify the context handle exists without using it as proof of trap activation by itself.

## 31. Implementation Checklist

- [ ] Container ref and any helper-node refs exist before trap activation.
- [ ] Nested scope activation and restore order match the documented rules.
- [ ] Restore-focus fallback behavior is verified.
- [ ] Optional `FocusManager` publication is documented and verified when the adapter exposes programmatic navigation.
- [ ] Cleanup and test oracles cover focus restoration and structure.
