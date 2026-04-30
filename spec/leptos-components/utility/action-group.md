---
adapter: leptos
component: action-group
category: utility
source: components/utility/action-group.md
source_foundation: foundation/08-adapter-leptos.md
---

# ActionGroup — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ActionGroup`](../../components/utility/action-group.md) machine to Leptos 0.8.x compound components, including repeated action items and optional overflow structure.

## 2. Public Adapter API

```rust,no_check
#[component] pub fn ActionGroup(...) -> impl IntoView
#[component] pub fn Item(item_id: Key, children: Children) -> impl IntoView
#[component] pub fn OverflowTrigger(children: Children) -> impl IntoView
```

The adapter surfaces the full core prop set, including orientation, overflow mode, variant, selection mode, disabled state, disabled items, max visible actions, and localized messages.

## 3. Mapping to Core Component Contract

- Props parity: full parity with core action-group props.
- Part parity: `Root`, repeated `Item`, and `OverflowTrigger` are explicit mapped parts.
- Structural parity: overflow menu content is a documented structural node when overflow mode uses a menu.

## 4. Part Mapping

| Core part / structure | Required?                   | Adapter rendering target               | Ownership                                            | Attr source                                                           | Notes                                                   |
| --------------------- | --------------------------- | -------------------------------------- | ---------------------------------------------------- | --------------------------------------------------------------------- | ------------------------------------------------------- |
| `Root`                | required                    | wrapper `<div>` or toolbar root        | adapter-owned                                        | `api.root_attrs()`                                                    | Carries toolbar semantics.                              |
| `Item { item_id }`    | repeated                    | button-like item node                  | compound subcomponent                                | `api.item_attrs(item_id)` or `part_attrs(Part::Item { ... })`         | Each visible action maps to one repeated part instance. |
| `OverflowTrigger`     | conditional                 | trigger `<button>`                     | compound subcomponent or adapter-owned internal node | `api.overflow_trigger_attrs()` or `part_attrs(Part::OverflowTrigger)` | Only when overflow mode requires a trigger.             |
| overflow menu content | conditional structural node | menu root/content for overflowed items | adapter-owned                                        | adapter-owned structural attrs                                        | Structural node, not a core enum part.                  |

## 5. Attr Merge and Ownership Rules

| Target node         | Core attrs                                 | Adapter-owned attrs                                          | Consumer attrs                              | Merge order                                                                                                                                                    | Ownership notes                                                    |
| ------------------- | ------------------------------------------ | ------------------------------------------------------------ | ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| `Root`              | `api.root_attrs()`                         | measurement markers and overflow bookkeeping attrs if needed | consumer root attrs                         | core state/role attrs win; `class`/`style` merge additively                                                                                                    | adapter-owned root                                                 |
| repeated item nodes | per-item core attrs                        | adapter registration or measurement attrs                    | consumer item attrs/content                 | core activation and disabled attrs win; consumer decoration remains additive; disabled items must not receive adapter-owned click or keydown activation wiring | item nodes are compound-owned even when consumer content is nested |
| overflow trigger    | core trigger attrs if owned by the adapter | adapter overflow-control attrs                               | consumer trigger decoration only if exposed | core/menu-control attrs win                                                                                                                                    | trigger may be adapter-owned or a documented compound part         |
| overflow content    | core overflow container attrs if any       | portal or measurement attrs                                  | consumer overflow content attrs if exposed  | structural containment attrs win                                                                                                                               | adapter-owned overflow container                                   |

## 6. Composition / Context Contract

`ActionGroup` provides typed context to repeated items and any overflow trigger/menu helpers. Stable keys are required.

## 7. Prop Sync and Event Mapping

Action-group configuration props are generally non-reactive after mount unless a wrapper explicitly remeasures and reinitializes. Responsive overflow state is internally derived from measurement and item registration.

| Adapter prop            | Mode                      | Sync trigger                | Machine event / update path                      | Visible effect                         | Notes                                                                                |
| ----------------------- | ------------------------- | --------------------------- | ------------------------------------------------ | -------------------------------------- | ------------------------------------------------------------------------------------ |
| item collection / order | controlled                | render of repeated children | item registration and measurement reconciliation | determines visible vs overflowed items | DOM order is significant                                                             |
| overflow strategy props | non-reactive adapter prop | render time only            | initial/group props                              | determines overflow behavior           | post-mount changes should be treated as unsupported unless the adapter reinitializes |

| UI event                          | Preconditions                                 | Machine event / callback path | Ordering notes                                                                           | Notes                                                    |
| --------------------------------- | --------------------------------------------- | ----------------------------- | ---------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| item activation                   | item visible or in overflow and not disabled  | item activation path          | normalized before public item callbacks                                                  | selection semantics must stay separate from activation   |
| disabled item interaction attempt | item is disabled by group or item-local state | no-op                         | click and keydown activation wiring must be absent or suppressed before public callbacks | disabled items must not synthesize activation work       |
| overflow trigger activation       | overflow exists                               | open/close overflow path      | measurement state must already be current                                                | trigger callbacks run after normalized open state change |
| resize / remeasure                | client-only measurement available             | overflow recompute path       | recompute occurs after current DOM order is known                                        | no SSR measurement                                       |

## 8. Registration and Cleanup Contract

- Repeated items register in DOM order.
- Overflow bookkeeping registers measurement observers or resize listeners only on platforms that support them.
- Cleanup must remove stale item registrations before tearing down overflow state.

| Registered entity             | Registration trigger               | Identity key                                                | Cleanup trigger                       | Cleanup action                                     | Notes                               |
| ----------------------------- | ---------------------------------- | ----------------------------------------------------------- | ------------------------------------- | -------------------------------------------------- | ----------------------------------- |
| item registry entry           | item mount                         | stable item identity from data source or component instance | item cleanup                          | remove item from ordering and overflow bookkeeping | reorders must update registry order |
| overflow measurement observer | first client-side measurement pass | action-group instance                                       | overflow disable or component cleanup | disconnect observer/listener                       | client-only                         |
| overflow container state      | overflow open                      | action-group instance                                       | close or component cleanup            | clear tracked open/placement state                 | no stale detached overflow nodes    |

## 9. Ref and Node Contract

| Target part / node                    | Ref required?                                                            | Ref owner                                                   | Node availability    | Composition rule                                       | Notes                                                   |
| ------------------------------------- | ------------------------------------------------------------------------ | ----------------------------------------------------------- | -------------------- | ------------------------------------------------------ | ------------------------------------------------------- |
| `Root`                                | yes when overflow measurement or roving behavior uses the live container | adapter-owned                                               | required after mount | compose only when wrappers expose the root             | Root measurement belongs to the action-group instance.  |
| repeated item nodes                   | yes                                                                      | compound item owns the live node; wrappers may compose refs | required after mount | preserve adapter registration ownership when composing | Needed for measurement and focus bookkeeping.           |
| overflow trigger and overflow content | yes when overflow mode is enabled                                        | adapter-owned or compound-owned depending on exposure       | client-only          | compose only if separately exposed                     | Trigger/content refs support measurement and placement. |

## 10. State Machine Boundary Rules

- machine-owned state: visible item set, overflowed item set, activation semantics, disabled item semantics, and overflow open state when modeled by the utility.
- adapter-local derived bookkeeping: measurement observer handles, DOM rect caches, and item node refs.
- forbidden local mirrors: do not keep an unsynchronized visible/overflow split separate from the normalized overflow recompute path.
- allowed snapshot-read contexts: measurement effects, render derivation, activation handlers, and cleanup.

## 11. Callback Payload Contract

| Callback                               | Payload source             | Payload shape                                        | Timing                                 | Cancelable? | Notes                                                      |
| -------------------------------------- | -------------------------- | ---------------------------------------------------- | -------------------------------------- | ----------- | ---------------------------------------------------------- |
| item activation callback when exposed  | normalized adapter payload | `{ item_id: Key, overflowed: bool, disabled: bool }` | after normalized activation handling   | no          | Activation remains separate from selection-like semantics. |
| overflow trigger callback when exposed | machine-derived snapshot   | `{ open: bool, overflow_count: usize }`              | after normalized open-state transition | no          | Observational callback for overflow toggling.              |

## 12. Failure and Degradation Rules

| Condition                                                                          | Policy             | Notes                                                                                                                                  |
| ---------------------------------------------------------------------------------- | ------------------ | -------------------------------------------------------------------------------------------------------------------------------------- |
| duplicate item IDs or unstable repeated-item identity                              | fail fast          | Overflow bookkeeping and focus order depend on stable identities.                                                                      |
| measurement unsupported on the current platform                                    | degrade gracefully | Preserve structure and fall back to a non-measured overflow/default presentation if documented.                                        |
| overflow trigger/content ref missing after mount while overflow mode requires them | fail fast          | The overflow interaction model depends on concrete nodes.                                                                              |
| toolbar root rendered without an accessible name                                   | warn and ignore    | Preserve the toolbar structure, but emit a debug-oriented warning because `role="toolbar"` requires `aria-label` or `aria-labelledby`. |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source  | Duplicates allowed? | DOM order must match registration order? | SSR/hydration stability                                         | Notes                                     |
| -------------------------------- | ---------------- | ------------------- | ---------------------------------------- | --------------------------------------------------------------- | ----------------------------------------- |
| repeated item registry           | data-derived     | no                  | yes                                      | repeated item order and IDs must remain stable across hydration | Item ID is the canonical identity source. |
| overflow trigger/content pair    | instance-derived | not applicable      | not applicable                           | structural identity must stay stable if present on the server   | Owned by the action-group instance.       |

## 14. SSR and Client Boundary Rules

- SSR must render root, visible items, and any documented overflow structure required by the initial non-measured layout contract.
- Measurement, resize observers, and placement logic are client-only.
- Root/item/overflow refs are server-safe absent and required after mount when measurement is enabled.
- The server/client item order must remain stable for hydration.

## 15. Performance Constraints

- Resize observers and measurement listeners must not be recreated on every render.
- Overflow recomputation should run only when item identity/order or container size changes.
- Item registration should be incremental and keyed by stable item identity.
- Cleanup must remove observer/listener ownership in one pass per group instance.

## 16. Implementation Dependencies

| Dependency          | Required?   | Dependency type         | Why it must exist first                                                           | Notes                                                                       |
| ------------------- | ----------- | ----------------------- | --------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `button`            | recommended | behavioral prerequisite | Action items typically rely on button-like activation semantics.                  | Reuse normalized item activation behavior.                                  |
| `z-index-allocator` | recommended | shared helper           | Overflow content may later participate in layered overlay or menu infrastructure. | Especially relevant when overflow content escapes normal stacking contexts. |

## 17. Recommended Implementation Sequence

1. Initialize the action-group machine and item registry.
2. Render root plus repeated item structure and overflow trigger/content nodes.
3. Establish measurement or overflow bookkeeping after structure exists.
4. Wire item activation and overflow trigger behavior.
5. Verify observer cleanup, item unregister behavior, and callback order.

## 18. Anti-Patterns

- Do not depend on measurement before the overflow structure exists.
- Do not recreate observers or measurement listeners on every render.
- Do not treat overflow content as implicit or optional when the documented structure requires it.

## 19. Consumer Expectations and Guarantees

- Consumers may assume registered item identity remains attached to the same logical action across overflow recomputation.
- Consumers may assume disabled items never synthesize normalized activation.
- Consumers must not assume overflow measurement or selected-item geometry is consumer-owned layout state.

## 20. Platform Support Matrix

| Capability / behavior               | Browser client | SSR            | Notes                                                                                  |
| ----------------------------------- | -------------- | -------------- | -------------------------------------------------------------------------------------- |
| overflow measurement and reflow     | full support   | client-only    | Responsive measurement is client-owned; SSR renders the stable non-measured structure. |
| toolbar accessible-name diagnostics | full support   | not applicable | Diagnostics are authoring-time only and do not change SSR structure.                   |

## 21. Debug Diagnostics and Production Policy

| Condition                                                | Debug build behavior | Production behavior | Notes                                                                                 |
| -------------------------------------------------------- | -------------------- | ------------------- | ------------------------------------------------------------------------------------- |
| toolbar semantics lack `aria-label` or `aria-labelledby` | debug warning        | warn and ignore     | `role="toolbar"` still renders, but the accessible-name requirement remains explicit. |
| duplicate item IDs or unstable registry identity         | fail fast            | fail fast           | Registration and overflow bookkeeping depend on stable identities.                    |

## 22. Shared Adapter Helper Notes

| Helper concept                     | Required?   | Responsibility                                                                         | Reused by                                      | Notes                                                                      |
| ---------------------------------- | ----------- | -------------------------------------------------------------------------------------- | ---------------------------------------------- | -------------------------------------------------------------------------- |
| registry helper for repeated items | required    | Keep stable item identity, DOM order, and overflow bookkeeping aligned.                | `action-group`, `toggle-group`                 | Do not rebuild the whole registry on unrelated rerenders.                  |
| measurement helper                 | required    | Own overflow and selected-item geometry measurements plus fallback strategy selection. | `action-group`, `toggle-group`                 | Measurement remains adapter-owned state, never consumer-owned layout data. |
| debug-warning helper               | recommended | Emit toolbar accessible-name diagnostics in debug builds.                              | `action-group`, `as-child`, `download-trigger` | Warnings stay diagnostic-only.                                             |

## 23. Framework-Specific Behavior

Leptos client-only measurement can drive overflow behavior, but the spec must still document the overflow menu content as a concrete structural node when present. Any selected-item or indicator-position geometry used for overflow, underline, or active-item presentation remains adapter-owned measurement state rather than consumer-managed layout data. When the root uses toolbar semantics, the adapter should emit a debug-oriented warning if neither `aria-label` nor `aria-labelledby` is present.

## 24. Canonical Implementation Sketch

```rust
use leptos::prelude::*;

#[component]
pub fn ActionGroup(children: Children) -> impl IntoView {
    let machine = use_machine::<action_group::Machine>(action_group::Props::default());
    let root_attrs = machine.derive(|api| api.root_attrs());
    provide_context(Context(machine));

    view! {
        <div {..root_attrs.get()}>
            {children()}
        </div>
    }
}
```

## 25. Reference Implementation Skeleton

```rust,no_check
let machine = use_machine_or_adapter_controller(props);
let refs = create_required_refs_and_helpers();
let registry = create_optional_registration_or_cleanup_helper();

publish_required_contexts(machine, refs, registry);
render_documented_structure_and_attrs(machine);
sync_controlled_props_and_runtime_capabilities(machine, props);
wire_normalized_events_and_callbacks(machine, refs, registry);

on_cleanup(|| registry.release_all());
```

## 26. Adapter Invariants

- Item registration must remain stable across reordering so action state and measurements stay attached to the right item.
- Overflow trigger and overflow content must remain structurally distinct when the adapter owns those nodes.
- Activation semantics must stay separate from selection or overflow-management semantics.
- Disabled items must not receive click or keydown activation handlers that could synthesize normalized activation.
- Toolbar semantics must not be rendered without an accessible name requirement being documented and verified.
- Responsive measurement observers, resize handlers, or overflow bookkeeping must be cleaned up on unmount.
- Platform limitations must be documented in framework-specific behavior rather than hidden in the invariant list.

## 27. Accessibility and SSR Notes

- `Root` must preserve toolbar semantics.
- `Root` must provide an accessible name through `aria-label` or `aria-labelledby` when toolbar semantics are used.
- Repeated items must remain focus-addressable by key.
- Overflow trigger and overflow content must be structurally explicit when present.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop and repeated-part parity, with explicit overflow structural mapping.

Intentional deviations: responsive measurement strategy is adapter-owned, but not the overflow structure contract.

Traceability note: This adapter spec now makes explicit the core adapter-owned concerns for stable repeated-item registration, disabled-item activation suppression, overflow measurement ownership, accessible toolbar naming, and cleanup-safe overflow or indicator geometry handling.

## 29. Test Scenarios

- explicit root, repeated item, and overflow trigger mapping
- overflow menu structural presence
- item registration order
- activation vs selection behavior
- disabled items suppress activation wiring and do not produce normalized click or keydown activation
- selected-item or indicator geometry measurement remains adapter-owned and cleanup-safe where documented
- toolbar root without `aria-label` or `aria-labelledby` emits the documented warning

## 30. Test Oracle Notes

| Behavior                        | Preferred oracle type | Notes                                                                                                               |
| ------------------------------- | --------------------- | ------------------------------------------------------------------------------------------------------------------- |
| visible vs overflowed structure | rendered structure    | Assert the documented overflow trigger/content structure explicitly.                                                |
| repeated item bookkeeping       | context registration  | Verify stable item registration order and removal.                                                                  |
| observer/listener teardown      | cleanup side effects  | Assert measurement resources are disconnected on cleanup.                                                           |
| activation timing               | callback order        | Verify item and overflow trigger callbacks follow normalized transitions.                                           |
| disabled item suppression       | callback order        | Verify disabled items do not invoke normalized activation callbacks from click or keydown paths.                    |
| toolbar accessible-name warning | cleanup side effects  | Verify the documented debug-oriented warning appears when toolbar semantics lack `aria-label` or `aria-labelledby`. |

Cheap verification recipe:

1. Render a named toolbar with three items, then force overflow and assert the trigger or overflow content structure before checking any callback behavior.
2. Attempt click and keydown activation on a disabled item and verify no normalized activation callback fires.
3. Unmount the group and assert observer or measurement cleanup before removing the final registry assertions.

## 31. Implementation Checklist

- [ ] Repeated item identities are stable and registry order matches DOM order.
- [ ] Overflow trigger and overflow content render when required.
- [ ] Measurement setup and cleanup are instance-scoped.
- [ ] Disabled items do not receive activation wiring from click or keydown paths.
- [ ] Toolbar semantics include an accessible name, or the documented debug warning path is verified.
- [ ] Item and overflow callback ordering is verified.
