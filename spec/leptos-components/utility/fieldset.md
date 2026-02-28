---
adapter: leptos
component: fieldset
category: utility
source: components/utility/fieldset.md
source_foundation: foundation/08-adapter-leptos.md
---

# Fieldset — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Fieldset`](../../components/utility/fieldset.md) contract and `07-forms.md` fieldset behavior to Leptos 0.8.x compound components.

## 2. Public Adapter API

```rust
#[component] pub fn Fieldset(...) -> impl IntoView
#[component] pub fn Legend(children: Children) -> impl IntoView
#[component] pub fn Description(children: Children) -> impl IntoView
#[component] pub fn ErrorMessage(children: Children) -> impl IntoView
```

The root `Fieldset` component surfaces the full core prop set: `id`, `disabled`, `invalid`, `readonly`, and `dir`.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core fieldset props.
- Part parity: `Root`, `Legend`, `Description`, and `ErrorMessage` are all explicit mapped structures.
- Context parity: descendant field-like controls inherit group state through field context.

## 4. Part Mapping

| Core part / structure | Required?                | Adapter rendering target                      | Ownership                                                | Attr source                 | Notes                                                                                  |
| --------------------- | ------------------------ | --------------------------------------------- | -------------------------------------------------------- | --------------------------- | -------------------------------------------------------------------------------------- |
| `Root`                | required                 | native `<fieldset>`                           | adapter-owned                                            | `api.root_attrs()`          | Group container and propagation boundary.                                              |
| `Legend`              | required structurally    | native `<legend>` via compound subcomponent   | compound subcomponent                                    | `api.legend_attrs()`        | May be omitted only if an alternate accessible name strategy is explicitly documented. |
| `Description`         | conditional              | `<div>` or `<span>` via compound subcomponent | compound subcomponent                                    | `api.description_attrs()`   | Included in group-level `aria-describedby`.                                            |
| `ErrorMessage`        | conditional              | `<span>` via compound subcomponent            | compound subcomponent                                    | `api.error_message_attrs()` | Uses `role="alert"`.                                                                   |
| child fields region   | required structural node | consumer children inside `Root`               | consumer-owned descendants inside adapter-owned fieldset | none                        | Descendants inherit `FieldCtx`.                                                        |

## 5. Attr Merge and Ownership Rules

| Target node                          | Core attrs                                    | Adapter-owned attrs                     | Consumer attrs                    | Merge order                                                                                                                         | Ownership notes                            |
| ------------------------------------ | --------------------------------------------- | --------------------------------------- | --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ |
| `Root`                               | `api.root_attrs()` for `<fieldset>` semantics | wrapper-only structural attrs if needed | consumer attrs on `Fieldset` root | core disabled, describedby, and group semantics win; `class`/`style` merge additively                                               | adapter-owned root                         |
| `Legend`                             | `api.legend_attrs()`                          | none beyond structural part markers     | consumer legend attrs/content     | core naming attrs win                                                                                                               | compound subcomponent owns the legend node |
| `Description` / `ErrorMessage`       | core description/error attrs                  | registration markers if needed          | consumer text/content attrs       | core IDs, roles, and description wiring win                                                                                         | compound subcomponents own these nodes     |
| descendant field inheritance surface | inherited context values, not DOM attrs       | adapter-provided field context          | explicit descendant `Field` props | descendant field props merge on the child field side; inherited values are defaults unless the core contract says they are forceful | no dedicated DOM node                      |

- Native `<fieldset>` and `<legend>` semantics are the default ownership model.
- Descendant fields must receive inherited disabled, readonly, and invalid state through context rather than duplicated DOM mutation.

## 6. Composition / Context Contract

`Fieldset` provides both machine context and inherited `FieldCtx` to descendant field-like controls. Required subparts use `use_context::<Context>().expect(...)`, and descendant fields consume inherited disabled, invalid, and readonly state through the published field context rather than ad hoc DOM inspection.

## 7. Prop Sync and Event Mapping

Controlled/uncontrolled switching is not applicable. Fieldset state is driven by explicit props and the presence of description or error subparts.

| Adapter prop         | Mode                        | Sync trigger                 | Machine event / update path                    | Visible effect                                        | Notes                                                                    |
| -------------------- | --------------------------- | ---------------------------- | ---------------------------------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------ |
| `disabled`           | controlled                  | prop change after mount      | `SetDisabled` or equivalent fieldset sync path | disables descendant field context and root semantics  | inherited by descendant fields unless locally overridden per core policy |
| `readonly`           | controlled                  | prop change after mount      | `SetReadonly` or equivalent                    | updates inherited readonly context                    | context-derived effect                                                   |
| `invalid`            | controlled                  | prop change after mount      | `SetInvalid` or equivalent                     | updates inherited invalid context and error semantics | context-derived effect                                                   |
| description presence | uncontrolled internal state | `Description` mount/unmount  | `SetHasDescription(true/false)` or equivalent  | updates group-level describedby                       | registration-driven                                                      |
| error presence       | uncontrolled internal state | `ErrorMessage` mount/unmount | `SetHasErrorMessage(true/false)` or equivalent | updates group-level describedby and alert semantics   | registration-driven                                                      |

| UI event                | Preconditions             | Machine event / callback path          | Ordering notes                                                        | Notes                                                    |
| ----------------------- | ------------------------- | -------------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------- |
| descendant field render | inside `Fieldset` context | inherited state read via context merge | descendant field merges inherited state before deriving its own attrs | fieldset itself does not normalize descendant DOM events |

## 8. Registration and Cleanup Contract

- Description and error subparts register on mount and unregister on cleanup.
- Descendant fields consume inherited context for as long as the fieldset is mounted.
- Cleanup must remove inherited-state availability before descendant orphan updates can occur.

| Registered entity          | Registration trigger | Identity key                          | Cleanup trigger      | Cleanup action                                  | Notes                                                 |
| -------------------------- | -------------------- | ------------------------------------- | -------------------- | ----------------------------------------------- | ----------------------------------------------------- |
| `Description` part         | subcomponent mount   | fieldset instance plus description ID | subcomponent cleanup | clear description registration from the machine | prevents stale group describedby references           |
| `ErrorMessage` part        | subcomponent mount   | fieldset instance plus error ID       | subcomponent cleanup | clear error registration from the machine       | removes stale alert relationships                     |
| inherited fieldset context | root mount           | fieldset instance                     | root cleanup         | drop context provider / subscription linkage    | descendant fields must not read stale inherited state |

## 9. Ref and Node Contract

| Target part / node   | Ref required?                                                                            | Ref owner     | Node availability                  | Composition rule                                            | Notes                                                             |
| -------------------- | ---------------------------------------------------------------------------------------- | ------------- | ---------------------------------- | ----------------------------------------------------------- | ----------------------------------------------------------------- |
| `Root` fieldset node | no by default; yes only when wrappers require measurement or imperative focus management | adapter-owned | always structural, handle optional | compose only when a wrapper explicitly exposes the root ref | Native fieldset semantics do not inherently require a stored ref. |
| `Legend`             | no by default                                                                            | adapter-owned | always structural, handle optional | no composition unless separately exposed                    | Descendants should not depend on a legend ref for semantics.      |

## 10. State Machine Boundary Rules

- machine-owned state: inherited disabled/readonly/invalid values, describedby relationships, and fieldset semantics.
- adapter-local derived bookkeeping: optional wrapper refs only.
- forbidden local mirrors: do not duplicate inherited descendant state outside fieldset context propagation.
- allowed snapshot-read contexts: render derivation and mounted-part registration effects.

## 11. Callback Payload Contract

| Callback                                                          | Payload source | Payload shape | Timing         | Cancelable? | Notes                                                  |
| ----------------------------------------------------------------- | -------------- | ------------- | -------------- | ----------- | ------------------------------------------------------ |
| no public adapter-specific callback beyond descendant composition | none           | none          | not applicable | no          | Descendant fields own their own interaction callbacks. |

## 12. Failure and Degradation Rules

| Condition                                                             | Policy          | Notes                                                                                 |
| --------------------------------------------------------------------- | --------------- | ------------------------------------------------------------------------------------- |
| non-native fallback structure used without documented ARIA repair     | fail fast       | Fieldset semantics must not silently degrade.                                         |
| missing description/error context during descendant inheritance       | warn and ignore | Preserve root semantics while omitting the missing inherited contribution.            |
| non-native fallback omits `aria-labelledby` pointing to the legend ID | fail fast       | A generic fallback container must preserve the legend naming relationship explicitly. |

## 13. Identity and Key Policy

| Registered or repeated structure     | Identity source | Duplicates allowed?                               | DOM order must match registration order? | SSR/hydration stability                             | Notes                                                  |
| ------------------------------------ | --------------- | ------------------------------------------------- | ---------------------------------------- | --------------------------------------------------- | ------------------------------------------------------ |
| fieldset root and legend association | composite       | no                                                | not applicable                           | root/legend IDs must remain stable across hydration | Identity is fieldset instance plus stable IDs.         |
| description/error registrations      | composite       | yes for distinct nodes; duplicate IDs not allowed | not applicable                           | IDs must remain hydration-stable                    | Same policy as field-level describedby/error identity. |

## 14. SSR and Client Boundary Rules

- SSR must preserve native `<fieldset>` / `<legend>` structure whenever that is the documented rendering target.
- Description and error nodes must keep stable IDs across hydration.
- No client-only mutation should replace native fieldset semantics with generic wrappers after mount.

## 15. Performance Constraints

- Inherited-state propagation should update through context/state derivation, not by walking descendants imperatively.
- Description/error registration should only react to mount-state changes.
- Avoid redundant recomputation of descendant inheritance when unrelated fieldset props are unchanged.

## 16. Implementation Dependencies

| Dependency | Required? | Dependency type  | Why it must exist first                                                                              | Notes                                           |
| ---------- | --------- | ---------------- | ---------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| `field`    | required  | context contract | Fieldset inheritance exists to shape descendant field behavior and must match field merge semantics. | Required for meaningful descendant propagation. |

## 17. Recommended Implementation Sequence

1. Render the native fieldset and legend structure.
2. Establish fieldset machine/context for inherited state.
3. Render and register description/error structural parts.
4. Verify descendant field merge behavior and cleanup of inherited-state registrations.

## 18. Anti-Patterns

- Do not replace native `<fieldset>` and `<legend>` with generic nodes without explicit ARIA repair.
- Do not duplicate inherited-state propagation imperatively on every descendant render.

## 19. Consumer Expectations and Guarantees

- Consumers may assume descendant fields receive inherited `FieldCtx` values from the nearest fieldset boundary.
- Consumers may assume native `<fieldset>` and `<legend>` semantics remain the default rendering contract.
- Consumers must not assume a non-native fallback preserves legend naming unless the adapter also emits the documented `aria-labelledby` repair.

## 20. Platform Support Matrix

| Capability / behavior                     | Browser client | SSR          | Notes                                                                                         |
| ----------------------------------------- | -------------- | ------------ | --------------------------------------------------------------------------------------------- |
| documented structural and state semantics | full support   | full support | This utility does not have additional platform variance beyond its existing SSR/client rules. |

## 21. Debug Diagnostics and Production Policy

| Condition                                                                    | Debug build behavior | Production behavior | Notes                                                                                 |
| ---------------------------------------------------------------------------- | -------------------- | ------------------- | ------------------------------------------------------------------------------------- |
| non-native fallback omits `aria-labelledby` repair to the legend ID          | fail fast            | fail fast           | Generic fallback containers must preserve the fieldset accessible name explicitly.    |
| missing description or error contribution during inherited state propagation | warn and ignore      | warn and ignore     | Root semantics stay intact while the missing contribution is surfaced diagnostically. |

## 22. Shared Adapter Helper Notes

| Helper concept    | Required?      | Responsibility                                                                          | Reused by      | Notes                                                         |
| ----------------- | -------------- | --------------------------------------------------------------------------------------- | -------------- | ------------------------------------------------------------- |
| attr merge helper | not applicable | No special helper beyond the documented attr derivation and merge contract is required. | not applicable | Use the normal machine attr derivation path for this utility. |

## 23. Framework-Specific Behavior

Leptos represents `Legend`, `Description`, and `ErrorMessage` as compound subcomponents. Descendant `Field` components merge inherited state with their own props. If a non-native fallback such as `<div role="group">` is ever used instead of `<fieldset>`, the adapter must set `aria-labelledby` to the rendered legend ID and continue publishing `FieldCtx` via `provide_context(...)`.

## 24. Canonical Implementation Sketch

```rust
use leptos::prelude::*;

#[component]
pub fn Fieldset(children: Children) -> impl IntoView {
    let machine = use_machine::<fieldset::Machine>(fieldset::Props::default());
    let root_attrs = machine.derive(|api| api.root_attrs());

    provide_context(Context(machine));
    provide_context(FieldCtx {
        name: None,
        disabled: machine.derive(|api| api.is_disabled()),
        invalid: machine.derive(|api| api.is_invalid()),
    });

    view! {
        <fieldset {..root_attrs.get()}>
            {children()}
        </fieldset>
    }
}
```

## 25. Reference Implementation Skeleton

```rust
let machine = use_machine::<fieldset::Machine>(props);
let ids = derive_stable_fieldset_ids(machine);
publish_fieldset_machine_context(machine);
publish_inherited_field_ctx(machine);

render_native_fieldset_and_legend(ids);
render_and_register_description_and_error_parts(ids);
if using_non_native_fallback() {
    repair_group_semantics_with_aria_labelledby(ids.legend_id);
}

on_cleanup(|| unregister_description_error_and_drop_inherited_ctx());
```

## 26. Adapter Invariants

- Native `<fieldset>` and `<legend>` semantics should be preserved unless the spec explicitly documents a fallback structure.
- Group-level description and error parts must participate in naming and description wiring for descendant controls.
- Inherited disabled, readonly, and invalid state propagation must remain explicit and deterministic.
- Any non-native fallback structure must document the ARIA repair work needed to preserve fieldset semantics.
- Any non-native fallback root must set `aria-labelledby` to the legend ID instead of relying on implicit naming behavior.

## 27. Accessibility and SSR Notes

- `Root` must remain a native `<fieldset>`.
- `Legend` must remain a native `<legend>`.
- Group-level description and error associations must be stable across SSR and hydration.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, structure, and context parity.

Intentional deviations: none.

## 29. Test Scenarios

- explicit root, legend, description, and error mapping
- inherited disabled and invalid propagation
- group-level `aria-describedby`
- native fieldset semantics
- non-native fallback preserves legend naming through explicit `aria-labelledby`

## 30. Test Oracle Notes

| Behavior                                   | Preferred oracle type | Notes                                                                                                       |
| ------------------------------------------ | --------------------- | ----------------------------------------------------------------------------------------------------------- |
| native fieldset and legend semantics       | rendered structure    | Assert the documented structural nodes directly.                                                            |
| inherited semantics and describedby wiring | DOM attrs             | Assert descendant-facing inherited attrs and fieldset-level references.                                     |
| descendant inheritance propagation         | context registration  | Verify descendant fields consume updated inherited state.                                                   |
| non-native fallback naming repair          | DOM attrs             | Assert fallback roots point `aria-labelledby` at the legend ID when native fieldset semantics are not used. |

## 31. Implementation Checklist

- [ ] Native fieldset and legend structure is preserved unless a documented fallback is intentionally used.
- [ ] Inherited disabled, readonly, and invalid state propagates through context.
- [ ] Description/error wiring and cleanup are correct.
- [ ] Any non-native fallback root sets `aria-labelledby` to the legend ID and still publishes `FieldCtx`.
- [ ] SSR/hydration preserves stable fieldset semantics and IDs.
