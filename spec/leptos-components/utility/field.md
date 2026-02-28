---
adapter: leptos
component: field
category: utility
source: components/utility/field.md
source_foundation: foundation/08-adapter-leptos.md
---

# Field — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Field`](../../components/utility/field.md) contract and `07-forms.md` field behavior to Leptos 0.8.x compound components.

## 2. Public Adapter API

```rust
#[component] pub fn Field(...) -> impl IntoView
#[component] pub fn Label(children: Children) -> impl IntoView
#[component] pub fn Description(children: Children) -> impl IntoView
#[component] pub fn ErrorMessage(children: Children) -> impl IntoView
```

The root `Field` component surfaces the full core prop set: `id`, `required`, `disabled`, `readonly`, `invalid`, and `dir`.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core field props.
- Part parity: all documented field parts must be mapped, including structural parts not backed by a visible subcomponent.
- Context parity: descendant input-like controls consume field attrs from the field machine context.

## 4. Part Mapping

| Core part / structure | Required?                   | Adapter rendering target                      | Ownership                               | Attr source                                   | Notes                                                                |
| --------------------- | --------------------------- | --------------------------------------------- | --------------------------------------- | --------------------------------------------- | -------------------------------------------------------------------- |
| `Root`                | required                    | wrapper `<div>` from `Field`                  | adapter-owned compound root             | `api.root_attrs()`                            | Hosts all field subparts.                                            |
| `Label`               | conditional                 | `<label>` from `field::Label`                 | compound subcomponent                   | `api.label_attrs()`                           | Optional if the consumer omits a label.                              |
| `Input`               | required structurally       | consumer input-like child inside `Field`      | consumer-owned node                     | `api.input_attrs()` consumed by child control | No dedicated DOM wrapper; attrs are forwarded to the actual control. |
| `Description`         | conditional                 | `<span>` or similar from `field::Description` | compound subcomponent                   | `api.description_attrs()`                     | Mount state affects `aria-describedby`.                              |
| `ErrorMessage`        | conditional                 | `<span>` from `field::ErrorMessage`           | compound subcomponent                   | `api.error_message_attrs()`                   | Uses `role="alert"`.                                                 |
| `RequiredIndicator`   | conditional structural node | `<span>` inside `Label`                       | compound or consumer-owned substructure | adapter-owned structural attrs                | Decorative marker, typically rendered only when `required=true`.     |

## 5. Attr Merge and Ownership Rules

| Target node                    | Core attrs                                              | Adapter-owned attrs                                               | Consumer attrs                                                     | Merge order                                                                                                                                                                                           | Ownership notes                                                                  |
| ------------------------------ | ------------------------------------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- |
| `Root`                         | `api.root_attrs()`                                      | wrapper-only `data-ars-part` helpers if needed                    | consumer attrs on `Field` root                                     | core state, ID, and relationship attrs win; `class`/`style` merge additively                                                                                                                          | root remains adapter-owned                                                       |
| `Label`                        | `api.label_attrs()`                                     | optional required-indicator wrapper attrs                         | consumer label attrs or label slot content                         | core `for`, `id`, and labeling attrs win; visual classes merge                                                                                                                                        | compound subcomponent owns the actual `<label>`                                  |
| actual input-like control      | `api.input_attrs()`                                     | none unless the concrete input adapter adds structural data attrs | consumer input attrs from the concrete control component           | core `id`, `aria-*`, `disabled`, `readonly`, and describedby attrs win; consumer `class`/`style` merge; consumer handlers compose after machine-normalized handlers unless guard logic must run first | the input DOM node is consumer-owned but must consume the core attr set directly |
| `Description` / `ErrorMessage` | `api.description_attrs()` / `api.error_message_attrs()` | registration markers if needed                                    | consumer text/content attrs on the subcomponent root               | core `id`, `role`, and announcement attrs win                                                                                                                                                         | compound subcomponents own their nodes                                           |
| `RequiredIndicator`            | none from core beyond field state                       | structural wrapper attrs such as `aria-hidden="true"`             | consumer decoration only if the adapter exposes the indicator slot | adapter-owned attrs win because the indicator must remain decorative by default                                                                                                                       | usually rendered inside `Label`                                                  |

- Child input-like controls must merge their own attrs onto the same DOM node that receives `api.input_attrs()`.
- Parent field context may contribute disabled, readonly, invalid, and naming defaults, but an explicit local field prop must follow the precedence policy documented by the core spec.
- Consumers must not replace IDs or ARIA relationship attrs in ways that sever label, description, or error associations.

## 6. Composition / Context Contract

`Field` provides a typed machine context with `provide_context`. Required subparts use `use_context::<Context>().expect(...)`. Optional parent field state such as inherited disabled/invalid values is read via `use_context::<FieldCtx>()`.

## 7. Prop Sync and Event Mapping

Controlled/uncontrolled switching is not applicable to the compound shell itself. Root state comes from explicit field props plus inherited field context. Presence-sensitive subparts update machine state through mount/unmount registration.

| Adapter prop         | Mode                        | Sync trigger                       | Machine event / update path                            | Visible effect                                              | Notes                                                                     |
| -------------------- | --------------------------- | ---------------------------------- | ------------------------------------------------------ | ----------------------------------------------------------- | ------------------------------------------------------------------------- |
| `disabled`           | controlled                  | root prop change after mount       | `SetDisabled` or equivalent prop-to-machine sync       | disables the input attr set and descendant affordances      | local prop participates in merge with parent context                      |
| `readonly`           | controlled                  | root prop change after mount       | `SetReadonly` or equivalent prop-to-machine sync       | updates input attrs and read-only affordances               | merge with inherited readonly follows explicit precedence                 |
| `invalid`            | controlled                  | root prop change after mount       | `SetInvalid` or equivalent prop-to-machine sync        | updates error attrs and validation styling                  | merged with parent invalid state                                          |
| `required`           | non-reactive adapter prop   | render time only                   | included in machine props                              | toggles required semantics and required indicator rendering | changes after mount should be treated as unsupported unless reinitialized |
| parent field context | derived from context        | parent context update or recompute | merge into effective field props before deriving attrs | inherited disabled/invalid state reaches descendants        | explicit local props must win where the core contract says so             |
| description presence | uncontrolled internal state | `Description` mount/unmount        | `SetHasDescription(true/false)`                        | updates `aria-describedby`                                  | registration-driven                                                       |
| error presence       | uncontrolled internal state | `ErrorMessage` mount/unmount       | `SetHasErrorMessage(true/false)` or equivalent         | updates `aria-describedby` and alert structure              | registration-driven                                                       |

| UI event                     | Preconditions                          | Machine event / callback path                 | Ordering notes                                                                              | Notes                                                        |
| ---------------------------- | -------------------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| compound subpart render      | inside a valid `Field` context         | attr derivation only                          | context lookup must happen before attrs are derived                                         | missing context is a contract failure for required subparts  |
| input-like descendant events | child control consumes `input_attrs()` | handled by the concrete child control adapter | child control normalizes events first, then field relationships update through shared attrs | `Field` does not steal ownership of the input event pipeline |

## 8. Registration and Cleanup Contract

- Presence-sensitive parts must register on mount and unregister on cleanup.
- Registration order is not semantically meaningful for `Description` versus `ErrorMessage`, but both must be reflected in the current `aria-describedby` set.
- Missing-context failures for `Label`, `Description`, and `ErrorMessage` should fail fast rather than silently no-op.

| Registered entity                    | Registration trigger              | Identity key                                   | Cleanup trigger      | Cleanup action                                                          | Notes                                            |
| ------------------------------------ | --------------------------------- | ---------------------------------------------- | -------------------- | ----------------------------------------------------------------------- | ------------------------------------------------ |
| `Description` part                   | `Description` subcomponent mount  | component instance plus derived description ID | subcomponent cleanup | dispatch `SetHasDescription(false)` and drop any stored ID contribution | must not leave stale describedby tokens          |
| `ErrorMessage` part                  | `ErrorMessage` subcomponent mount | component instance plus derived error ID       | subcomponent cleanup | dispatch error-presence removal and drop any stored ID contribution     | alert relationship must disappear when unmounted |
| inherited field context subscription | root `Field` mount                | field instance                                 | root cleanup         | drop inherited-state subscription                                       | parent updates must not outlive the child field  |

## 9. Ref and Node Contract

| Target part / node                             | Ref required?                                            | Ref owner                                                                   | Node availability                  | Composition rule                                                                               | Notes                                                                    |
| ---------------------------------------------- | -------------------------------------------------------- | --------------------------------------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| actual input-like control                      | yes                                                      | consumer-owned but composed with the field contract through `input_attrs()` | required after mount               | compose consumer control refs with any adapter-level input ref needed by higher-level wrappers | The input node, not a wrapper, is the authoritative form-control handle. |
| `Root`, `Label`, `Description`, `ErrorMessage` | no unless a wrapper needs measurement or focus targeting | adapter-owned                                                               | always structural, handle optional | no composition by default                                                                      | These nodes do not replace the actual input ref.                         |

## 10. State Machine Boundary Rules

- machine-owned state: field IDs, disabled/readonly/invalid/required semantics, label and description relationships, and mounted-part presence.
- adapter-local derived bookkeeping: optional wrapper refs and consumer-control integration glue only.
- forbidden local mirrors: do not mirror field IDs, `aria-describedby`, invalid state, or required state outside the machine/context merge.
- allowed snapshot-read contexts: render derivation, mounted-part registration effects, and cleanup for mounted-part removal.

## 11. Callback Payload Contract

| Callback                                                 | Payload source             | Payload shape                                                              | Timing                            | Cancelable?                           | Notes                                                                                                                   |
| -------------------------------------------------------- | -------------------------- | -------------------------------------------------------------------------- | --------------------------------- | ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| field-level callbacks exposed by concrete child controls | normalized adapter payload | child-control-specific payload enriched with canonical field IDs/relations | after child-control normalization | depends on the child control contract | `Field` itself does not invent independent payloads; it enriches downstream control behavior through attrs and context. |

## 12. Failure and Degradation Rules

| Condition                                                                     | Policy             | Notes                                                                       |
| ----------------------------------------------------------------------------- | ------------------ | --------------------------------------------------------------------------- |
| `Label`, `Description`, or `ErrorMessage` used without required field context | fail fast          | Compound parts require a parent `Field` contract.                           |
| consumer control fails to apply `input_attrs()` to the actual input node      | fail fast          | Wrapper-node-only application breaks canonical form semantics.              |
| generated IDs unavailable during SSR-safe rendering                           | degrade gracefully | Render the structure but preserve deterministic ID generation on hydration. |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source | Duplicates allowed?                               | DOM order must match registration order? | SSR/hydration stability                     | Notes                                                      |
| -------------------------------- | --------------- | ------------------------------------------------- | ---------------------------------------- | ------------------------------------------- | ---------------------------------------------------------- |
| field root and input association | composite       | no                                                | not applicable                           | IDs must be stable across SSR and hydration | Identity is field instance plus generated/stable field ID. |
| description/error registrations  | composite       | yes for distinct nodes; duplicate IDs not allowed | not applicable                           | registered IDs must be hydration-stable     | Identity is subpart instance plus derived ID.              |

## 14. SSR and Client Boundary Rules

- SSR must preserve stable field, label, description, and error IDs.
- The actual input node handle is server-safe absent and required after mount if wrappers need imperative focus/measurement.
- Description and error registration effects are client-safe but must preserve the same derived IDs generated on the server.

## 15. Performance Constraints

- Description and error presence must only register/unregister on mount-state changes, not every rerender.
- `input_attrs()` should be derived once per machine/context change and merged at the input node, not reconstructed ad hoc in nested wrappers.
- Parent-context inheritance should update deterministically without forcing unrelated subparts to re-register.

## 16. Implementation Dependencies

| Dependency | Required?   | Dependency type  | Why it must exist first                                                                   | Notes                                                       |
| ---------- | ----------- | ---------------- | ----------------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| `form`     | recommended | context contract | Field registration and reset behavior must align with form-level coordination.            | Required when the field participates in form-managed flows. |
| `fieldset` | recommended | context contract | Inherited disabled/readonly/invalid state must merge consistently with field-local props. | Important for grouped fields.                               |

## 17. Recommended Implementation Sequence

1. Initialize the field machine and publish field context.
2. Render root, label, description, error, and required-indicator structure.
3. Wire `input_attrs()` consumption into the actual input-like control.
4. Register mount-sensitive description and error parts.
5. Verify inherited-context merge behavior and cleanup of mounted-part registrations.

## 18. Anti-Patterns

- Do not apply `input_attrs()` to a wrapper instead of the actual input node.
- Do not allow description or error registration state to survive unmount.
- Do not generate unstable field-related IDs across SSR and hydration.

## 19. Consumer Expectations and Guarantees

- Consumers may assume generated label, description, and error IDs remain stable across hydration and point at the actual input node.
- Consumers may assume published field context reflects inherited disabled, invalid, and readonly state before descendant attrs are derived.
- Consumers must not assume wrapper nodes own `input_attrs()`; the actual input-like control remains the ownership target.

## 20. Platform Support Matrix

| Capability / behavior                     | Browser client | SSR          | Notes                                                                                         |
| ----------------------------------------- | -------------- | ------------ | --------------------------------------------------------------------------------------------- |
| documented structural and state semantics | full support   | full support | This utility does not have additional platform variance beyond its existing SSR/client rules. |

## 21. Debug Diagnostics and Production Policy

| Condition                                           | Debug build behavior | Production behavior | Notes                                                      |
| --------------------------------------------------- | -------------------- | ------------------- | ---------------------------------------------------------- |
| required compound part mounts without field context | fail fast            | fail fast           | Label, description, and error parts require field context. |
| actual input never receives `input_attrs()`         | fail fast            | fail fast           | Applying input attrs to a wrapper is a contract violation. |

## 22. Shared Adapter Helper Notes

| Helper concept    | Required?      | Responsibility                                                                          | Reused by      | Notes                                                         |
| ----------------- | -------------- | --------------------------------------------------------------------------------------- | -------------- | ------------------------------------------------------------- |
| attr merge helper | not applicable | No special helper beyond the documented attr derivation and merge contract is required. | not applicable | Use the normal machine attr derivation path for this utility. |

## 23. Framework-Specific Behavior

Leptos compound components are the adapter representation of `Label`, `Description`, and `ErrorMessage`. `Input` remains consumer-owned but must consume the field attrs from context.

## 24. Canonical Implementation Sketch

```rust
use leptos::prelude::*;

#[component]
pub fn Field(children: Children) -> impl IntoView {
    let machine = use_machine::<field::FieldComponentMachine>(field::Props::default());
    let root_attrs = machine.derive(|api| api.root_attrs());
    provide_context(Context(machine));

    view! {
        <div {..root_attrs.get()}>
            {children()}
        </div>
    }
}

#[component]
pub fn Label(children: Children) -> impl IntoView {
    let machine = use_context::<Context>().expect("field::Label requires Field");
    let attrs = machine.derive(|api| api.label_attrs());
    view! { <label {..attrs.get()}>{children()}</label> }
}

#[component]
pub fn Description(children: Children) -> impl IntoView {
    let machine = use_context::<Context>().expect("field::Description requires Field");
    machine.send.run(field::Event::SetHasDescription(true));
    on_cleanup(move || machine.send.run(field::Event::SetHasDescription(false)));
    let attrs = machine.derive(|api| api.description_attrs());
    view! { <span {..attrs.get()}>{children()}</span> }
}
```

## 25. Reference Implementation Skeleton

```rust
let machine = use_machine::<field::Machine>(props);
let ids = derive_stable_field_ids(machine);
let machine_ctx = publish_field_machine_context(machine);
let inherited_ctx = publish_field_context(machine, parent_ctx);

render_root_label_and_optional_parts(ids);
ensure_input_attrs_land_on_the_actual_control(machine);
register_description_and_error_parts_on_mount(machine_ctx);
merge_inherited_and_local_state_before_descendant_attr_derivation();

on_cleanup(|| unregister_description_error_and_context_bookkeeping());
```

## 26. Adapter Invariants

- The actual input-like control must own the input attrs produced by the core API rather than an outer wrapper.
- Mounted `Description` and `ErrorMessage` parts must update `aria-describedby` registration as they appear and disappear.
- The required indicator must remain decorative unless the core contract explicitly promotes it into the accessible name.
- Parent and child field context merge order must be explicit and deterministic.
- Generated IDs and label or description associations must remain stable across SSR and hydration.
- Compound subcomponents that require field context must define an explicit missing-context failure contract.

## 27. Accessibility and SSR Notes

- `Input` attrs must remain attached to the actual form control.
- `Description` and `ErrorMessage` presence must affect `aria-describedby` correctly.
- SSR must preserve stable IDs for all associated nodes.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core prop, structure, and context parity.

Intentional deviations: none.

Traceability note: This adapter spec now makes explicit the core adapter-owned concerns for actual-control attr ownership, stable generated IDs, description or error registration, and fail-fast compound-context handling.

## 29. Test Scenarios

- full mapping of root, label, input, description, error, and required indicator
- context inheritance from parent field containers
- presence-driven `aria-describedby` updates
- consumer-owned input attr consumption

## 30. Test Oracle Notes

| Behavior                            | Preferred oracle type | Notes                                                                         |
| ----------------------------------- | --------------------- | ----------------------------------------------------------------------------- |
| actual control attr ownership       | DOM attrs             | Assert the canonical attrs on the real input-like DOM node, not on a wrapper. |
| description/error presence tracking | context registration  | Assert registration and deregistration when subparts mount/unmount.           |
| ID stability across SSR/hydration   | hydration structure   | Assert the same generated IDs and references survive hydration.               |

Cheap verification recipe:

1. Render the field with description and error parts, then assert canonical attrs on the actual input-like node rather than the wrapper.
2. Mount and unmount description or error parts and verify `aria-describedby` changes through registration rather than ad-hoc string concatenation.
3. Hydrate the same structure and confirm the generated IDs and label or description references remain stable.

## 31. Implementation Checklist

- [ ] The actual input-like control owns the canonical field attrs.
- [ ] Label, description, error, and required-indicator structure matches the documented mapping.
- [ ] Description/error registration updates `aria-describedby` correctly.
- [ ] Missing-context failures for compound parts are covered.
- [ ] SSR/hydration preserves stable IDs and references.
