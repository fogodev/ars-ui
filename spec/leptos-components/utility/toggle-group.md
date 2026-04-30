---
adapter: leptos
component: toggle-group
category: utility
source: components/utility/toggle-group.md
source_foundation: foundation/08-adapter-leptos.md
---

# ToggleGroup — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ToggleGroup`](../../components/utility/toggle-group.md) machine to Leptos 0.8.x compound components coordinating repeated toggle items and optional indicator structure.

## 2. Public Adapter API

```rust,no_check
#[component] pub fn ToggleGroup(...) -> impl IntoView
#[component] pub fn Item(value: String, children: Children) -> impl IntoView
#[component] pub fn Indicator() -> impl IntoView
```

The group surfaces the full core prop set, including controlled/uncontrolled value, `selection_mode`, `orientation`, `dir`, `loop_focus`, `roving_focus`, and form participation props.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core toggle-group props.
- Part parity: `Root`, repeated `Item`, and optional `Indicator` are all explicit mapped structures.
- Context parity: items consume group context for selection state, roving tabindex, and event dispatch.

## 4. Part Mapping

| Core part / structure | Required?                   | Adapter rendering target           | Ownership                                            | Attr source                                                 | Notes                                              |
| --------------------- | --------------------------- | ---------------------------------- | ---------------------------------------------------- | ----------------------------------------------------------- | -------------------------------------------------- |
| `Root`                | required                    | wrapper `<div>`                    | adapter-owned                                        | `api.root_attrs()`                                          | Role varies by selection mode.                     |
| `Item { value }`      | repeated                    | `ToggleButton`-backed item element | compound subcomponent                                | `api.item_attrs(value)` or `part_attrs(Part::Item { ... })` | Each item owns one logical toggle-group item part. |
| `Indicator`           | optional                    | `<div>` or `<span>` highlight node | compound subcomponent or adapter-owned internal node | `api.indicator_attrs()` or `part_attrs(Part::Indicator)`    | Optional structural selection highlight.           |
| hidden form inputs    | conditional structural node | adapter-owned hidden inputs        | adapter-owned                                        | adapter-owned structural attrs                              | Only when group-level form participation is used.  |

## 5. Attr Merge and Ownership Rules

| Target node        | Core attrs                                       | Adapter-owned attrs                                                 | Consumer attrs                                             | Merge order                                                                            | Ownership notes                                                            |
| ------------------ | ------------------------------------------------ | ------------------------------------------------------------------- | ---------------------------------------------------------- | -------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| `Root`             | `api.root_attrs()`                               | structural group markers and hidden-input container attrs if needed | consumer group root attrs                                  | core role, orientation, and state attrs win; `class`/`style` merge additively          | adapter-owned root                                                         |
| `Item { value }`   | `api.item_attrs(value)` or equivalent part attrs | adapter item wrapper attrs if the adapter wraps `ToggleButton`      | consumer attrs on `Item` or underlying toggle-button child | group-required selection, tabindex, and disabled attrs win; child visual classes merge | compound item owns the actual item root even when backed by `ToggleButton` |
| `Indicator`        | `api.indicator_attrs()`                          | structural indicator attrs                                          | consumer indicator decoration only if exposed              | core indicator attrs win                                                               | adapter-owned or compound-owned highlight node                             |
| hidden form inputs | core form participation data                     | structural hidden-input attrs                                       | none                                                       | core attrs win                                                                         | adapter-owned and group-scoped                                             |

- Group-level attrs always win where selection semantics, roving tabindex, or form participation would otherwise diverge.
- Item-local consumer attrs may decorate appearance, but must not replace group-required state, role, or tabindex attrs.

## 6. Composition / Context Contract

`ToggleGroup` provides typed group context to `Item` and `Indicator`. Standalone toggle-button hidden-input behavior is suppressed when items are inside the group.

## 7. Prop Sync and Event Mapping

Switching between controlled and uncontrolled group value is not supported after mount. `default_value` is init-only. `value` uses effect-based controlled sync.

| Adapter prop                                          | Mode                        | Sync trigger            | Machine event / update path | Visible effect                                                | Notes                                                                    |
| ----------------------------------------------------- | --------------------------- | ----------------------- | --------------------------- | ------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `value`                                               | controlled                  | prop change after mount | `SetValue`                  | updates selected items, roving state, and hidden-input output | immediate sync                                                           |
| `default_value`                                       | uncontrolled internal state | initial render only     | initial machine props       | seeds internal selection                                      | read once                                                                |
| `selection_mode`                                      | non-reactive adapter prop   | render time only        | initial/group props         | changes selection behavior and root role                      | post-mount changes should be treated as unsupported unless reinitialized |
| `orientation` / `dir` / `loop_focus` / `roving_focus` | non-reactive adapter prop   | render time only        | initial/group props         | configures keyboard and layout semantics                      | adapter may reinitialize if wrappers want dynamic support                |

| UI event                | Preconditions                    | Machine event / callback path  | Ordering notes                                   | Notes                                                                   |
| ----------------------- | -------------------------------- | ------------------------------ | ------------------------------------------------ | ----------------------------------------------------------------------- |
| item activation         | registered item and not disabled | selection event for item value | normalized before public change callbacks        | selection callback fires after state update unless documented otherwise |
| `Arrow*`, `Home`, `End` | roving focus enabled             | roving-focus navigation path   | focus movement occurs after item registry lookup | DOM order must match registration order                                 |
| form reset              | group participates in a form     | reset path to default value    | reset occurs before notification-only callbacks  | hidden inputs must update in the same turn                              |

## 8. Registration and Cleanup Contract

- Every item registers on mount and unregisters on cleanup.
- Stable identity is the logical item value plus component instance where duplicate values are possible; if duplicate values are not supported, the spec should say so and require uniqueness.
- DOM order and registration order must remain aligned for roving focus.
- Hidden form inputs are group-owned and must be cleaned up with the group instance.

| Registered entity     | Registration trigger                        | Identity key                       | Cleanup trigger                             | Cleanup action                                       | Notes                                 |
| --------------------- | ------------------------------------------- | ---------------------------------- | ------------------------------------------- | ---------------------------------------------------- | ------------------------------------- |
| item registry entry   | `Item` mount                                | item value plus component instance | `Item` cleanup                              | remove item from selection and roving-focus registry | stale items must not keep focus slots |
| indicator anchor data | indicator mount when present                | group instance                     | indicator cleanup or group cleanup          | drop indicator measurement or attachment data        | optional structure                    |
| hidden form inputs    | group mount with form participation enabled | group instance                     | group cleanup or form-participation disable | remove hidden inputs and related bookkeeping         | no leaked form values after unmount   |

## 9. Ref and Node Contract

| Target part / node                      | Ref required?                                                 | Ref owner                                                   | Node availability    | Composition rule                                                   | Notes                                                                        |
| --------------------------------------- | ------------------------------------------------------------- | ----------------------------------------------------------- | -------------------- | ------------------------------------------------------------------ | ---------------------------------------------------------------------------- |
| `Root`                                  | yes when roving focus or measurement depends on the live root | adapter-owned                                               | required after mount | compose only if wrappers expose a root ref                         | Needed for keyboard focus normalization and some form participation helpers. |
| item roots                              | yes                                                           | compound item owns the live node; wrappers may compose refs | required after mount | item ref composition must preserve group ownership of roving focus | Each registered item needs a stable node handle.                             |
| `Indicator` when measured or positioned | yes when the strategy measures the active item                | adapter-owned or compound-owned                             | client-only          | compose only when indicator ownership is exposed                   | Optional depending on indicator strategy.                                    |
| hidden inputs                           | yes when rendered                                             | adapter-owned                                               | required after mount | no composition                                                     | Group-owned form participation nodes.                                        |

## 10. State Machine Boundary Rules

- machine-owned state: selected value(s), roving tabindex, disabled item semantics, indicator targeting, and form participation output.
- adapter-local derived bookkeeping: item node handles, optional indicator measurement handles, and registration bookkeeping that is purely structural.
- forbidden local mirrors: do not keep a separate selected-value list or roving index outside the machine.
- allowed snapshot-read contexts: item registration effects, keyboard handlers, render derivation, and cleanup.

## 11. Callback Payload Contract

| Callback                                         | Payload source             | Payload shape                            | Timing                                                                         | Cancelable?                           | Notes                                      |
| ------------------------------------------------ | -------------------------- | ---------------------------------------- | ------------------------------------------------------------------------------ | ------------------------------------- | ------------------------------------------ | --------------------------------------------------------- |
| selection change callback when exposed           | machine-derived snapshot   | `{ value: string                         | string[], selection_mode: string, triggered_item: string }`                    | after normalized selection transition | no                                         | Must reflect the post-transition machine selection state. |
| item activation callback when separately exposed | normalized adapter payload | `{ item_value: string, disabled: bool }` | after activation normalization and before observational wrappers if documented | no                                    | Must not bypass group selection semantics. |

## 12. Failure and Degradation Rules

| Condition                                         | Policy    | Notes                                                                                                        |
| ------------------------------------------------- | --------- | ------------------------------------------------------------------------------------------------------------ |
| duplicate item values when uniqueness is required | fail fast | Group identity and roving focus depend on stable unique values unless the spec explicitly allows duplicates. |
| item rendered without required group context      | fail fast | Compound items require a parent group contract.                                                              |
| indicator measurement unavailable on the server   | no-op     | Keep structural indicator output and defer measurement until mount.                                          |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source | Duplicates allowed?      | DOM order must match registration order? | SSR/hydration stability                                       | Notes                                                                                           |
| -------------------------------- | --------------- | ------------------------ | ---------------------------------------- | ------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| item registry entries            | composite       | no by default            | yes                                      | item identities and order must remain stable across hydration | Identity is logical item value plus component instance if the implementation must disambiguate. |
| hidden form inputs               | composite       | no duplicate field slots | yes when output ordering matters         | hidden-input structure must stay hydration-stable             | Group instance plus selection slot identity.                                                    |
| indicator target                 | data-derived    | not applicable           | yes relative to active item order        | indicator anchor identity must remain stable                  | Usually based on the selected item value.                                                       |

## 14. SSR and Client Boundary Rules

- SSR must render root structure, repeated items, and hidden inputs or indicator structure required by the initial state.
- Item and root node handles are server-safe absent and required after mount.
- Keyboard navigation, roving focus, and indicator measurement are client-only behaviors.
- Item order and identity must not change between server and client hydration paths.

## 15. Performance Constraints

- Item registration must update incrementally and must not rebuild the whole registry on unrelated rerenders.
- Roving-focus state should derive from the machine plus the current registry, not from expensive DOM scans each interaction.
- Indicator measurement should be instance-scoped and only rerun when the active item or layout changes.
- Hidden-input output should update only when selected value(s) change.

## 16. Implementation Dependencies

| Dependency      | Required? | Dependency type      | Why it must exist first                                                           | Notes                                                           |
| --------------- | --------- | -------------------- | --------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| `toggle-button` | required  | composition contract | Group items rely on the established toggle-button interaction and item semantics. | Required before group-owned selection can be layered correctly. |

## 17. Recommended Implementation Sequence

1. Initialize the toggle-group machine and publish group context.
2. Register items with stable identities and live node handles.
3. Render root, item, indicator, and hidden-input structure.
4. Wire controlled value sync and roving-focus behavior.
5. Verify hidden-input output, unregister cleanup, and test oracles.

## 18. Anti-Patterns

- Do not allow unstable or duplicate item values without an explicit duplicate policy.
- Do not rebuild the entire item registry on unrelated rerenders.
- Do not blur ownership between group attrs and item-local decoration.

## 19. Consumer Expectations and Guarantees

- Consumers may assume item registration order matches DOM order for roving focus and indicator targeting.
- Consumers may assume group-owned hidden inputs reflect the current canonical selection state only.
- Consumers must not assume standalone `ToggleButton` hidden-input semantics remain active inside a group-owned item.

## 20. Platform Support Matrix

| Capability / behavior                | Browser client | SSR          | Notes                                                      |
| ------------------------------------ | -------------- | ------------ | ---------------------------------------------------------- |
| roving focus and keyboard navigation | full support   | client-only  | SSR renders stable item structure but does not move focus. |
| group-owned hidden inputs            | full support   | full support | Hidden inputs may render from initial state on the server. |

## 21. Debug Diagnostics and Production Policy

| Condition                                          | Debug build behavior | Production behavior | Notes                                                                   |
| -------------------------------------------------- | -------------------- | ------------------- | ----------------------------------------------------------------------- |
| duplicate item values where uniqueness is required | fail fast            | fail fast           | Roving focus and hidden-input output depend on canonical item identity. |
| item mounts without required group context         | fail fast            | fail fast           | Compound items require a parent group contract.                         |

## 22. Shared Adapter Helper Notes

| Helper concept                         | Required?   | Responsibility                                                                      | Reused by                       | Notes                                                             |
| -------------------------------------- | ----------- | ----------------------------------------------------------------------------------- | ------------------------------- | ----------------------------------------------------------------- |
| registry helper for repeated items     | required    | Maintain canonical item identity, DOM order, roving focus, and unregister cleanup.  | `toggle-group`, `action-group`  | The registry is the source of truth for item ordering.            |
| hidden-input form participation helper | required    | Generate and reconcile group-owned hidden inputs from canonical selection state.    | `toggle-group`, `toggle-button` | Suppress standalone toggle-button hidden inputs inside the group. |
| measurement helper                     | recommended | Position the indicator relative to the active item without consumer-owned geometry. | `toggle-group`, `action-group`  | Only needed when the indicator strategy measures the active item. |

## 23. Framework-Specific Behavior

Leptos uses compound components and stable item registration. Item order must remain consistent with DOM order for roving focus.

## 24. Canonical Implementation Sketch

```rust
use leptos::prelude::*;

#[component]
pub fn ToggleGroup(children: Children) -> impl IntoView {
    let machine = use_machine::<toggle_group::Machine>(toggle_group::Props::default());
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
let machine = use_machine::<toggle_group::Machine>(props);
let registry = create_item_registry_helper();
let hidden_input_helper = create_group_hidden_input_helper();
let indicator_helper = create_optional_indicator_measurement_helper();

publish_group_context(machine, registry);
render_root_items_indicator_and_hidden_inputs();
register_items_with_stable_identity(registry);
sync_controlled_value(machine, props.value);
wire_roving_focus_and_selection(machine, registry);
update_hidden_inputs_and_indicator(machine, hidden_input_helper, indicator_helper);

on_cleanup(|| {
    registry.release_all();
    hidden_input_helper.release_all();
    indicator_helper.release();
});
```

## 26. Adapter Invariants

- Repeated item identity must remain stable and keyed so roving focus and selection state do not drift across reorders.
- Roving focus ownership must remain explicit and must not be split across unrelated wrappers.
- Selection-mode-specific hidden input or form participation rules must remain explicit wherever the core contract defines them.
- Controlled value synchronization must define how external updates reconcile with registered items.
- Disabled items must preserve the discoverability and focus rules defined by the core contract.

## 27. Accessibility and SSR Notes

- `Root` must preserve `role="group"` or `role="radiogroup"` as required.
- Repeated items must preserve per-item selected/focus attrs.
- Hidden form participation, when present, must stay synchronized across SSR and hydration.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, repeated-part, and context parity.

Intentional deviations: none.

Traceability note: This adapter spec now makes explicit the core adapter-owned concerns for stable item identity, roving-focus ownership, hidden-input or form participation output, and cleanup-safe item unregister behavior.

## 29. Test Scenarios

- explicit root, repeated item, and indicator mapping
- roving focus and loop behavior
- single vs multiple selection
- hidden input participation when configured

## 30. Test Oracle Notes

| Behavior                       | Preferred oracle type | Notes                                                           |
| ------------------------------ | --------------------- | --------------------------------------------------------------- |
| selection and roving state     | machine state         | Assert canonical selected value(s) and roving focus state.      |
| item tabindex/selection attrs  | DOM attrs             | Assert per-item roving and selected attrs on actual item roots. |
| item mount/unmount bookkeeping | context registration  | Verify item registration and unregister behavior.               |
| unregister cleanup             | cleanup side effects  | Assert stale item handles and hidden-input state are removed.   |

Cheap verification recipe:

1. Render a keyed item set, then assert registration order and per-item tabindex before checking any selection callback.
2. Drive both controlled-value updates and user selection, then verify machine state and hidden-input output stay synchronized.
3. Unmount one item and assert registry cleanup removes stale handles or form-participation output before reordering the remaining items.

## 31. Implementation Checklist

- [ ] Item registration and unregister behavior is keyed by stable item identity.
- [ ] Root/item attrs and roving tabindex match the documented contract.
- [ ] Controlled value sync and hidden-input output are verified.
- [ ] Cleanup removes stale item handles and group-owned form participation state.
