---
adapter: leptos
component: toggle-button
category: utility
source: components/utility/toggle-button.md
source_foundation: foundation/08-adapter-leptos.md
---

# ToggleButton — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`ToggleButton`](../../components/utility/toggle-button.md) machine to Leptos 0.8.x.

## 2. Public Adapter API

```rust
#[component] pub fn ToggleButton(...) -> impl IntoView
```

The adapter surfaces the full core prop set including pressed state, form props, value, locale, and hover callbacks.

## 3. Mapping to Core Component Contract

- Props parity: full parity.
- Event parity: press, release, toggle, reset, disabled sync, and hover callbacks are adapter-driven.

## 4. Part Mapping

| Core part / structure | Required?                   | Adapter rendering target | Ownership     | Attr source                                 | Notes                                               |
| --------------------- | --------------------------- | ------------------------ | ------------- | ------------------------------------------- | --------------------------------------------------- |
| `Root`                | required                    | native `<button>`        | adapter-owned | `api.part_attrs(toggle_button::Part::Root)` | Main interactive root.                              |
| hidden input          | conditional structural node | hidden `<input>`         | adapter-owned | adapter-owned structural attrs              | Only when standalone form participation is enabled. |

## 5. Attr Merge and Ownership Rules

| Target node  | Core attrs                                            | Adapter-owned attrs                                           | Consumer attrs                                     | Merge order                                                                                                                 | Ownership notes                                                |
| ------------ | ----------------------------------------------------- | ------------------------------------------------------------- | -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| `Root`       | `api.part_attrs(Part::Root)` or equivalent root attrs | hover-state markers and structural `data-*` helpers if needed | consumer root attrs from the hosting component     | core pressed/disabled/form semantics win; `class`/`style` merge additively; handlers compose around normalized toggle logic | adapter-owned native button                                    |
| hidden input | core form-participation attrs when standalone         | adapter-owned hidden-input structural attrs                   | no direct consumer attrs unless explicitly exposed | core name/value/checked attrs win                                                                                           | hidden input is adapter-owned and omitted inside `ToggleGroup` |

- Inside `ToggleGroup`, group context owns selection semantics and standalone hidden-input ownership is suppressed.
- Consumers must not override `aria-pressed`, hidden-input value, or disabled semantics in ways that break the contract.

## 6. Composition / Context Contract

May consume `ToggleGroup` context; standalone hidden input is suppressed when group-owned.
When form participation is enabled, the adapter should also integrate with surrounding field or form context. If the button is required and no hidden input is rendered, the adapter must register a `RequiredValidator`-equivalent path with `FormContext` so required validation still observes the unpressed state.

## 7. Prop Sync and Event Mapping

Switching between controlled and uncontrolled pressed state is not supported after mount. `default_pressed` is init-only. `pressed` and `disabled` use immediate effect-based sync when reactive.

| Adapter prop                                  | Mode                        | Sync trigger                                               | Machine event / update path                                      | Visible effect                                                                 | Notes                                                                                 |
| --------------------------------------------- | --------------------------- | ---------------------------------------------------------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- |
| `pressed`                                     | controlled                  | prop change after mount                                    | `SetPressed` or equivalent                                       | updates pressed state, `aria-pressed`, and hidden-input checkedness            | explicit controlled mode                                                              |
| `default_pressed`                             | uncontrolled internal state | initial render only                                        | initial machine props                                            | seeds internal pressed state                                                   | read once at initialization                                                           |
| `disabled`                                    | controlled                  | prop change after mount                                    | `SetDisabled`                                                    | blocks toggle activation and updates disabled semantics                        | immediate sync                                                                        |
| `name` / `value`                              | non-reactive adapter prop   | render time only                                           | included in root/hidden-input props                              | controls form participation payload                                            | post-mount changes should be treated as unsupported unless reinitialized              |
| `required` with standalone form participation | controlled                  | prop change after mount or initial standalone registration | hidden-input validity wiring or `RequiredValidator` registration | unpressed required buttons remain visible to form validation                   | hidden input and context-based validator are alternate adapter-owned validation paths |
| toggle-group context                          | derived from context        | group registration and selection updates                   | group-driven selection path                                      | suppresses standalone hidden-input ownership and local pressed source of truth | group wins over standalone form participation                                         |

| UI event                         | Preconditions                      | Machine event / callback path        | Ordering notes                                                                                | Notes                                         |
| -------------------------------- | ---------------------------------- | ------------------------------------ | --------------------------------------------------------------------------------------------- | --------------------------------------------- |
| `pointerdown`                    | interactive and not disabled       | `Press`                              | runs before hover/consumer notification callbacks                                             | may also set pointer modality                 |
| `pointerup`                      | matching active press              | `Release`                            | must preserve blur/release ordering                                                           | no-op when the press was suppressed           |
| `click` or normalized activation | not disabled and not group-blocked | `Toggle` then public change callback | change callback fires after state update unless documented otherwise                          | Enter on native button uses this path         |
| `focus` / `blur`                 | root focus changes                 | `Focus { is_keyboard }` / `Blur`     | focus normalization runs before notification-only callbacks                                   | required for focus-visible logic              |
| hover entry/exit                 | root pointer hover changes         | hover callbacks only                 | hover callbacks are notification-only and must not precede the normalized toggle state change | root state does not change selection on hover |

## 8. Registration and Cleanup Contract

- Standalone form participation owns one hidden input that mounts with the button and unmounts with it.
- Inside `ToggleGroup`, the item registers with group context and must not keep a standalone hidden input alive.
- Form reset participation must be cleaned up together with any hidden input bookkeeping.
- When standalone validation uses `FormContext` instead of a hidden input, the adapter must register a `RequiredValidator`-equivalent entry on mount and remove it during cleanup.

| Registered entity               | Registration trigger                                               | Identity key                               | Cleanup trigger                                                              | Cleanup action                                                      | Notes                                                                |
| ------------------------------- | ------------------------------------------------------------------ | ------------------------------------------ | ---------------------------------------------------------------------------- | ------------------------------------------------------------------- | -------------------------------------------------------------------- |
| hidden input                    | standalone mount with form participation enabled                   | button instance plus form field name/value | disabled standalone participation or component cleanup                       | remove hidden input and clear form bookkeeping                      | omitted when group-owned                                             |
| required validator registration | standalone required mount without hidden-input validation fallback | button instance plus field identity        | hidden-input takeover, disabled required participation, or component cleanup | unregister validator from `FormContext` or equivalent form registry | preserves required validation when no native hidden input handles it |
| group item registration         | mount inside `ToggleGroup` context                                 | item value plus component instance         | item cleanup or group cleanup                                                | unregister item from group selection and roving-focus bookkeeping   | prevents stale selected items                                        |

## 9. Ref and Node Contract

| Target part / node                                         | Ref required? | Ref owner     | Node availability    | Composition rule                                      | Notes                                                                                        |
| ---------------------------------------------------------- | ------------- | ------------- | -------------------- | ----------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `Root` button                                              | yes           | adapter-owned | required after mount | compose only if the hosting component exposes the ref | Needed for normalized focus/press behavior.                                                  |
| hidden input when standalone form participation is enabled | yes           | adapter-owned | required after mount | no composition by default                             | The hidden input participates in form reset/submission and must have a stable node identity. |

## 10. State Machine Boundary Rules

- machine-owned state: pressed state, disabled state, form participation, and any group-owned selection semantics.
- adapter-local derived bookkeeping: hover state, pointer-modality tracking, and hidden-input node handles.
- forbidden local mirrors: do not keep an unsynchronized pressed flag separate from the machine or group context.
- allowed snapshot-read contexts: render derivation, pointer/focus handlers, form-reset hooks, and cleanup.

## 11. Callback Payload Contract

| Callback                     | Payload source             | Payload shape                                      | Timing                                    | Cancelable? | Notes                                                         |
| ---------------------------- | -------------------------- | -------------------------------------------------- | ----------------------------------------- | ----------- | ------------------------------------------------------------- |
| change callback when exposed | machine-derived snapshot   | `{ pressed: bool, value?: string, name?: string }` | after normalized pressed-state transition | no          | Must reflect group-owned semantics when inside `ToggleGroup`. |
| hover callbacks when exposed | normalized adapter payload | `{ is_hovering: bool }`                            | after hover bookkeeping updates           | no          | Observational only; must not imply a pressed-state change.    |

## 12. Failure and Degradation Rules

| Condition                                                                                                   | Policy    | Notes                                                                                       |
| ----------------------------------------------------------------------------------------------------------- | --------- | ------------------------------------------------------------------------------------------- |
| standalone hidden-input participation combined with group-owned selection semantics                         | fail fast | The adapter must choose one ownership model.                                                |
| hidden-input node missing after mount when standalone form participation is enabled                         | fail fast | Form reset/submission semantics depend on the hidden input.                                 |
| required standalone validation requested with neither hidden input nor `FormContext` validator registration | fail fast | Required validation must not be silently dropped for an unpressed standalone toggle button. |
| SSR-only absence of browser interaction APIs                                                                | no-op     | Render the structural nodes and defer interaction.                                          |

## 13. Identity and Key Policy

| Registered or repeated structure | Identity source | Duplicates allowed?                                                        | DOM order must match registration order? | SSR/hydration stability                                    | Notes                                                   |
| -------------------------------- | --------------- | -------------------------------------------------------------------------- | ---------------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------- |
| standalone hidden input          | composite       | no for the same field slot                                                 | not applicable                           | hidden-input structure must remain stable across hydration | Identity is button instance plus field name/value pair. |
| group item registration          | composite       | depends on group duplicate policy; duplicates should default to disallowed | yes when group roving order applies      | group/item identity must remain stable across hydration    | Item identity must align with `ToggleGroup` rules.      |

## 14. SSR and Client Boundary Rules

- SSR must preserve root structure and any hidden input required by the initial standalone form-participation state.
- Root and hidden-input refs are server-safe absent and required after mount.
- Hover and pointer normalization are client-only.

## 15. Performance Constraints

- Hidden-input creation/removal must only follow participation changes, not ordinary rerenders.
- Hover bookkeeping must remain instance-local and must not allocate global listeners.
- Group registration should update incrementally instead of tearing down and recreating the item on every render.

## 16. Implementation Dependencies

| Dependency     | Required?   | Dependency type         | Why it must exist first                                                            | Notes                                                           |
| -------------- | ----------- | ----------------------- | ---------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| `button`       | required    | behavioral prerequisite | Toggle-button builds on button-like activation, focus, and accessibility behavior. | Use button semantics rather than re-inventing root interaction. |
| `field`        | recommended | context contract        | Standalone form participation aligns with field-like naming/value semantics.       | Relevant when hidden-input form participation is enabled.       |
| `toggle-group` | recommended | composition contract    | Group ownership overrides standalone selection and hidden-input behavior.          | Needed to avoid conflicting ownership models.                   |

## 17. Recommended Implementation Sequence

1. Initialize the toggle-button machine and root interaction behavior.
2. Choose the ownership model: standalone hidden input or group-owned selection.
3. Wire controlled pressed-state sync and disabled semantics.
4. Add hover, focus, press, and change callback wiring.
5. Verify form reset behavior and cleanup for hidden-input/group registration.

## 18. Anti-Patterns

- Do not keep standalone hidden-input participation and group-owned selection active at the same time.
- Do not fork pressed state outside the machine or group context.
- Do not treat hover callbacks as state-changing toggle callbacks.

## 19. Consumer Expectations and Guarantees

- Consumers may assume documented adapter-owned structural nodes and attrs remain the canonical implementation surface.
- Consumers may assume framework-specific divergence is called out explicitly rather than hidden in generic prose.
- Consumers must not assume unspecified fallback behavior, cleanup ordering, or helper ownership beyond what this adapter spec documents.

## 20. Platform Support Matrix

| Capability / behavior                     | Browser client | SSR          | Notes                                                                                         |
| ----------------------------------------- | -------------- | ------------ | --------------------------------------------------------------------------------------------- |
| documented structural and state semantics | full support   | full support | This utility does not have additional platform variance beyond its existing SSR/client rules. |

## 21. Debug Diagnostics and Production Policy

| Condition                                                            | Debug build behavior | Production behavior | Notes                                                                               |
| -------------------------------------------------------------------- | -------------------- | ------------------- | ----------------------------------------------------------------------------------- |
| no component-specific diagnostics beyond documented failure policies | not applicable       | not applicable      | Use the `Failure and Degradation Rules` section as the full runtime policy surface. |

## 22. Shared Adapter Helper Notes

| Helper concept                         | Required?      | Responsibility                                                               | Reused by                                            | Notes                                                    |
| -------------------------------------- | -------------- | ---------------------------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------- |
| hidden-input form participation helper | required       | Choose between hidden-input output and context-based validator registration. | `toggle-button`, `toggle-group`, field-like controls | Must never leave required validation silently unhandled. |
| debug-warning helper                   | not applicable | No extra helper beyond documented failure policies is required.              | not applicable                                       | Use the standard failure-policy surface.                 |

## 23. Framework-Specific Behavior

Leptos uses local signals for hover and pointer-modality bookkeeping.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn ToggleButton(children: Children) -> impl IntoView {
    let machine = use_machine::<toggle_button::Machine>(toggle_button::Props::default());
    let root_attrs = machine.derive(|api| api.part_attrs(toggle_button::Part::Root));
    view! { <button {..root_attrs.get()}>{children()}</button> }
}
```

## 25. Reference Implementation Skeleton

```rust
let machine = use_machine::<toggle_button::Machine>(props);
let root_ref = create_root_ref();
let form_helper = create_hidden_input_or_validator_helper();
let group_ctx = try_consume_group_context();

render_root(root_ref);
choose_group_owned_or_standalone_participation(group_ctx, form_helper);
sync_pressed_disabled_and_required_props(machine, props);
wire_press_focus_hover_and_change_callbacks(machine);
wire_form_reset_and_required_validation_paths(form_helper, machine);

on_cleanup(|| form_helper.release_all());
```

## 26. Adapter Invariants

- Native button activation must not be double-handled by custom keyboard wiring.
- Hidden input or form-participation structure, when part of the contract, must be explicit and hydration-safe.
- Required standalone validation must use either hidden-input validity semantics or a `FormContext` `RequiredValidator`-equivalent registration.
- Controlled and uncontrolled synchronization rules must state whether switching modes is supported.
- Form reset handling must define how the adapter restores unchecked or checked state.
- Blur and release ordering must remain aligned with the normalized interaction contract.
- Hover and press callbacks must document whether they fire from normalized machine events or raw DOM listeners.

## 27. Accessibility and SSR Notes

Must preserve `aria-pressed` and hidden-input semantics when enabled.

## 28. Parity Summary and Intentional Deviations

Parity summary: full core parity.

Intentional deviations: none.

## 29. Test Scenarios

- root mapping
- hidden-input structural node
- hover callback behavior
- form reset behavior
- required standalone toggle button participates in validation when unpressed
- hidden-input fallback path and context-validator path remain mutually exclusive and cleanup-safe

## 30. Test Oracle Notes

| Behavior                           | Preferred oracle type | Notes                                                                                                 |
| ---------------------------------- | --------------------- | ----------------------------------------------------------------------------------------------------- |
| pressed and hidden-input semantics | DOM attrs             | Assert `aria-pressed` and hidden-input value/checked output.                                          |
| change and hover timing            | callback order        | Verify state-changing callbacks and observational hover callbacks are ordered correctly.              |
| form participation cleanup         | cleanup side effects  | Assert hidden-input teardown and group unregister behavior.                                           |
| required standalone validation     | context registration  | Verify the required-validator registration exists when validation is not delegated to a hidden input. |

## 31. Implementation Checklist

- [ ] Root attrs and pressed semantics match the documented button-derived behavior.
- [ ] Hidden-input ownership is correct for standalone mode and absent in group mode.
- [ ] Controlled pressed sync and callback order are verified.
- [ ] Required standalone validation uses either hidden-input semantics or a documented `FormContext` validator path.
- [ ] Form reset behavior and cleanup side effects are covered.
