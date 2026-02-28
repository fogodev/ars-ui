---
adapter: dioxus
component: switch
category: input
source: components/input/switch.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Switch — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Switch`](../../components/input/switch.md) contract onto a Dioxus 0.7.x component. The adapter must preserve switch semantics on the `Control` button, hidden-input submission, and root-label composition.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct SwitchProps {
    #[props(optional)]
    pub checked: Option<bool>,
    #[props(optional)]
    pub default_checked: Option<bool>,
    #[props(default = false)]
    pub disabled: bool,
    #[props(default = false)]
    pub readonly: bool,
    #[props(default = false)]
    pub required: bool,
    #[props(default = false)]
    pub invalid: bool,
    #[props(optional)]
    pub name: Option<String>,
    #[props(optional)]
    pub value: Option<String>,
    #[props(optional)]
    pub form: Option<String>,
    pub children: Element,
}

#[component]
pub fn Switch(props: SwitchProps) -> Element
```

Shared input props such as IDs, locale or messages, and field integration remain available. Plain props are preferred; wrappers may add synchronization when they need a controlled external store.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core switch contract, including form props and RTL-aware root direction.
- Event parity: `Toggle`, `Focus`, `Blur`, and controlled `Set*` events remain machine-owned.
- Core machine ownership: `use_machine::<switch::Machine>(...)` owns `aria-checked`, hidden-input state, and thumb state tokens.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target  | Ownership     | Attr source                 | Notes                                    |
| --------------------- | --------- | ------------------------- | ------------- | --------------------------- | ---------------------------------------- |
| `Root`                | required  | `<label>`                 | adapter-owned | `api.root_attrs()`          | root-label click area remains structural |
| `Label`               | required  | `<span>`                  | adapter-owned | `api.label_attrs()`         | visible text label                       |
| `Control`             | required  | `<button>`                | adapter-owned | `api.control_attrs()`       | owns `role="switch"` and focus           |
| `Thumb`               | required  | `<span>`                  | adapter-owned | `api.thumb_attrs()`         | always `aria-hidden`                     |
| `HiddenInput`         | required  | `<input type="checkbox">` | adapter-owned | `api.hidden_input_attrs()`  | native submission bridge                 |
| `Description`         | optional  | `<div>`                   | adapter-owned | `api.description_attrs()`   | linked through `aria-describedby`        |
| `ErrorMessage`        | optional  | `<div>`                   | adapter-owned | `api.error_message_attrs()` | validation content                       |

## 5. Attr Merge and Ownership Rules

- Required ARIA, `type="button"`, state, and form attrs on `Control` or `HiddenInput` always win.
- `class` and `style` merge additively on `Root`, `Control`, and `Thumb`.
- Consumer handlers may decorate click or key handling, but must not suppress adapter toggling unless they intentionally cancel activation before it reaches the machine.

## 6. Composition / Context Contract

`Switch` is standalone but may consume field or fieldset context for disabled, readonly, validation, and described-by state. The root-label structure is adapter-owned and must not be collapsed into the `Control` button.

## 7. Prop Sync and Event Mapping

| Adapter prop            | Mode       | Sync trigger | Machine event / update path   | Visible effect                                       |
| ----------------------- | ---------- | ------------ | ----------------------------- | ---------------------------------------------------- |
| `checked`               | controlled | prop change  | `SetChecked`                  | updates `aria-checked`, thumb position, hidden input |
| `disabled` / `readonly` | controlled | prop change  | `SetDisabled` / `SetReadonly` | blocks toggling and updates attrs                    |
| `required` / `invalid`  | controlled | prop change  | `SetRequired` / `SetInvalid`  | updates ARIA and described-by wiring                 |

Pointer, `Space`, and `Enter` activation on `Control` all map to `Toggle`. Callback payloads should reflect the committed boolean state after the machine transition.

## 8. Registration and Cleanup Contract

- No descendant registry is required.
- No global listeners are required.
- Hidden-input and thumb state are derived, not imperatively cleaned up.

## 9. Ref and Node Contract

- `Control` owns the live interactive ref.
- `Root` may receive a ref for layout or style hooks, but it does not replace `Control` as the focus target.
- `HiddenInput` must remain non-focusable and visually hidden.

## 10. State Machine Boundary Rules

- The machine owns the on/off state and disabled or readonly guards.
- The adapter may normalize button activation, but it must not compute the next boolean outside the machine.
- `Thumb` is purely representational; it must not become an alternate event target that bypasses `Control`.

## 11. Callback Payload Contract

- Checked-change callbacks emit the committed `bool`.
- Focus callbacks may include `is_keyboard` for focus-visible styling.
- Validation callbacks should observe state after the machine settles the invalid or required attrs.

## 12. Failure and Degradation Rules

| Condition                                                       | Policy          | Notes                                                |
| --------------------------------------------------------------- | --------------- | ---------------------------------------------------- |
| controlled and uncontrolled checked props are mixed after mount | warn and ignore | first mode wins                                      |
| thumb decoration is omitted by a wrapper                        | fail fast       | `Thumb` is a required structural part                |
| SSR cannot determine focus-visible modality                     | no-op           | only post-hydration focus-visible styling is skipped |

## 13. Identity and Key Policy

The switch instance owns a single `Root` / `Control` / `Thumb` / `HiddenInput` identity. Hydration must preserve that order.

## 14. SSR and Client Boundary Rules

- SSR must render `Root`, `Label`, `Control`, `Thumb`, and `HiddenInput` in the same order used on the client.
- Only activation and focus-visible behavior are client-only.
- Hidden-input attrs must match on server and client.

## 15. Performance Constraints

- Keep thumb position purely attr- or class-driven.
- Do not mirror checked state into extra local signals.
- Avoid attaching duplicate click and key handlers that both toggle.

## 16. Implementation Dependencies

| Dependency | Required?   | Dependency type      | Why it must exist first                            |
| ---------- | ----------- | -------------------- | -------------------------------------------------- |
| `field`    | required    | composition contract | keeps label, description, and error wiring uniform |
| `fieldset` | recommended | disabled propagation | shared group-disabled behavior must stay aligned   |

## 17. Recommended Implementation Sequence

1. Initialize the machine and derive attrs for all required parts.
2. Render the root-label structure and keep `Control` and `Thumb` nested as documented.
3. Synchronize controlled props and field-derived state.
4. Compose `Space`, `Enter`, and pointer activation onto `Control`.
5. Finish diagnostics and hydration checks.

## 18. Anti-Patterns

- Do not move the hidden input outside the component subtree.
- Do not attach toggle handlers directly to `Thumb`.
- Do not let consumer `type` attrs override the required button semantics on `Control`.

## 19. Consumer Expectations and Guarantees

- Consumers may assume that `Control` always owns `role="switch"`.
- Consumers may assume that `Thumb` remains a required structural part.
- Consumers must not assume undocumented wrapper elision or label-to-button reassignment.

## 20. Platform Support Matrix

| Capability / behavior                                   | Web          | Desktop      | Mobile       | SSR          | Notes                                |
| ------------------------------------------------------- | ------------ | ------------ | ------------ | ------------ | ------------------------------------ |
| documented switch semantics and hidden-input submission | full support | full support | full support | full support | interactivity starts after hydration |

## 21. Debug Diagnostics and Production Policy

| Condition                                       | Debug build behavior | Production behavior | Notes                     |
| ----------------------------------------------- | -------------------- | ------------------- | ------------------------- |
| controlled/uncontrolled mode switch after mount | debug warning        | warn and ignore     | preserves current mode    |
| required structural part omitted by a wrapper   | fail fast            | fail fast           | protects switch semantics |

## 22. Shared Adapter Helper Notes

| Helper concept      | Required? | Responsibility                                              | Notes                              |
| ------------------- | --------- | ----------------------------------------------------------- | ---------------------------------- |
| hidden-input helper | required  | sync checked state, form attrs, and reset semantics         | shared with checkbox-like controls |
| field merge helper  | required  | merge described-by, disabled, readonly, and invalid context | shared across form-bound inputs    |

## 23. Framework-Specific Behavior

Dioxus should prefer plain props for the public surface, keep attrs in `derive(...)` outputs, and use `use_effect` only for the controlled synchronization paths that wrappers actually need after mount.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<switch::Machine>(props);

rsx! {
    label { ..machine.derive(|api| api.root_attrs()).read().clone(),
        span { ..machine.derive(|api| api.label_attrs()).read().clone(), {children} }
        button {
            ..machine.derive(|api| api.control_attrs()).read().clone(),
            onclick: move |_| machine.send.call(switch::Event::Toggle),
            span { ..machine.derive(|api| api.thumb_attrs()).read().clone() }
        }
        input { ..machine.derive(|api| api.hidden_input_attrs()).read().clone() }
    }
}
```

## 25. Reference Implementation Skeleton

- Build props from explicit values plus merged field or fieldset state.
- Initialize the machine and set up only the controlled sync paths that exist.
- Derive attrs for `Root`, `Label`, `Control`, `Thumb`, and `HiddenInput`, then render them in stable order.

## 26. Adapter Invariants

- `Control` always owns `role="switch"` and all activation behavior.
- `Thumb` is always present and always `aria-hidden`.
- `HiddenInput` remains adapter-owned and non-focusable.
- `Enter` and `Space` both map to the same machine toggle path.

## 27. Accessibility and SSR Notes

- `aria-describedby` must preserve description-first, error-second ordering.
- `aria-invalid` and `aria-errormessage` must only appear when invalid content is actually rendered.
- SSR must keep the same label and control structure to avoid hydration drift.

## 28. Parity Summary and Intentional Deviations

- Matches the core switch contract without intentional adapter divergence.
- Promotes hidden-input ownership, root-label composition, and keyboard activation rules into Dioxus-facing guidance.

## 29. Test Scenarios

- `Space` and `Enter` both toggle the switch and update `aria-checked`.
- Form submission includes the switch value only when checked.
- Invalid and required state update both ARIA attrs and described-by ordering.
- Disabled or readonly state blocks state changes but preserves the documented focus behavior.

## 30. Test Oracle Notes

- Inspect `role="switch"`, `aria-checked`, and hidden-input `checked` in the DOM.
- Assert that only `Control` receives focus during keyboard interaction.
- Use hydration tests to confirm stable label, button, thumb, and hidden-input ordering.

## 31. Implementation Checklist

- [ ] Render `Root`, `Label`, `Control`, `Thumb`, and `HiddenInput` in stable order.
- [ ] Keep `Control` as the only interactive node.
- [ ] Map `Space`, `Enter`, and pointer activation to the machine toggle path.
- [ ] Keep `Thumb` decorative and `HiddenInput` adapter-owned.
- [ ] Merge field-derived described-by, disabled, readonly, and invalid state.
- [ ] Preserve SSR and hydration parity for all structural parts.
