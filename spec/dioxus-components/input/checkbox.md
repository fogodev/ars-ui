---
adapter: dioxus
component: checkbox
category: input
source: components/input/checkbox.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Checkbox — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Checkbox`](../../components/input/checkbox.md) contract onto a Dioxus 0.7.x component. The adapter must preserve tri-state semantics, hidden-input form participation, and the built-in description and error-message parts.

## 2. Public Adapter API

```rust
#[derive(Props, Clone, PartialEq)]
pub struct CheckboxProps {
    #[props(optional)]
    pub checked: Option<checkbox::State>,
    #[props(optional)]
    pub default_checked: Option<checkbox::State>,
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
pub fn Checkbox(props: CheckboxProps) -> Element
```

The adapter also forwards the shared input props from the core contract, including IDs, locale or messages, and field integration data. Plain props are preferred; wrappers may layer post-mount synchronization when needed.

## 3. Mapping to Core Component Contract

- Props parity: the adapter exposes the full core `Props` surface, including tri-state checked values and form-related props.
- Event parity: `Toggle`, `Focus`, `Blur`, `SetChecked`, `SetDisabled`, `SetReadonly`, `SetInvalid`, and `SetRequired` remain machine-owned.
- Core machine ownership: `use_machine::<checkbox::Machine>(...)` remains the single source of truth for ARIA state, hidden-input state, and label wiring.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target  | Ownership     | Attr source                 | Notes                                       |
| --------------------- | --------- | ------------------------- | ------------- | --------------------------- | ------------------------------------------- |
| `Root`                | required  | `<div>`                   | adapter-owned | `api.root_attrs()`          | Carries state data attrs only.              |
| `Label`               | required  | `<label>`                 | adapter-owned | `api.label_attrs()`         | Must keep `for` linkage to `Control`.       |
| `Control`             | required  | focusable `<div>`         | adapter-owned | `api.control_attrs()`       | Owns `role="checkbox"` and focus semantics. |
| `Indicator`           | required  | `<div>`                   | adapter-owned | `api.indicator_attrs()`     | Decorative only, must remain `aria-hidden`. |
| `HiddenInput`         | required  | `<input type="checkbox">` | adapter-owned | `api.hidden_input_attrs()`  | Owns form submission and reset semantics.   |
| `Description`         | optional  | `<div>`                   | adapter-owned | `api.description_attrs()`   | Render only when provided.                  |
| `ErrorMessage`        | optional  | `<div>`                   | adapter-owned | `api.error_message_attrs()` | Render only when invalid content exists.    |

## 5. Attr Merge and Ownership Rules

- Core ARIA, state, `tabindex`, `id`, `name`, `value`, and form attrs on `Control` or `HiddenInput` always win over consumer decoration.
- `class` and `style` merge additively on `Root`, `Label`, and `Control`; consumer handlers compose after adapter handlers unless the adapter must prevent an invalid toggle.
- Consumers must not remove `aria-hidden` from `Indicator` or `HiddenInput`, and must not repurpose `HiddenInput` as a visible control.

## 6. Composition / Context Contract

`Checkbox` is standalone, but it may optionally consume field or fieldset context for disabled, readonly, described-by, and validation wiring. Group behavior is owned by [`CheckboxGroup`](checkbox-group.md) rather than by implicit child inspection inside `Checkbox`.

## 7. Prop Sync and Event Mapping

| Adapter prop           | Mode       | Sync trigger | Machine event / update path  | Visible effect                                      |
| ---------------------- | ---------- | ------------ | ---------------------------- | --------------------------------------------------- |
| `checked`              | controlled | prop change  | `SetChecked`                 | updates `aria-checked`, indicator, and hidden input |
| `disabled`             | controlled | prop change  | `SetDisabled`                | blocks interaction and updates disabled attrs       |
| `readonly`             | controlled | prop change  | `SetReadonly`                | preserves focusability but blocks mutation          |
| `required` / `invalid` | controlled | prop change  | `SetRequired` / `SetInvalid` | updates ARIA and described-by wiring                |

User activation maps pointer or keyboard activation on `Control` to `Toggle`. The adapter should expose `on_checked_change`-style callbacks only after the machine accepts the transition.

## 8. Registration and Cleanup Contract

- No descendant registry is required.
- The adapter owns only local event listeners on `Control`; no global listeners should survive unmount.
- Hidden-input state must be recomputed from machine state rather than patched imperatively during cleanup.

## 9. Ref and Node Contract

- The interactive node ref belongs to `Control`.
- `Label` and `HiddenInput` refs are optional and exist only for DOM interop; they must not become alternate state sources.
- Focus repair, if needed by wrappers, must target `Control`, never `HiddenInput`.

## 10. State Machine Boundary Rules

- Checked state, including `Indeterminate`, is machine-owned.
- The adapter may normalize pointer or keyboard events, but it must not compute next checked state outside the machine.
- Hidden-input `checked` mirrors only the machine's committed `Checked` state; `Indeterminate` stays an ARIA-only presentation state.

## 11. Callback Payload Contract

- Checked-change callbacks emit the committed `checkbox::State`.
- Focus callbacks may expose `is_keyboard` when the framework wrapper needs focus-visible behavior.
- Field-level validation callbacks must observe machine state after described-by and invalid attrs are settled.

## 12. Failure and Degradation Rules

| Condition                                                       | Policy             | Notes                                                   |
| --------------------------------------------------------------- | ------------------ | ------------------------------------------------------- |
| controlled and uncontrolled checked props are mixed after mount | warn and ignore    | first mode wins                                         |
| label, description, or error content is omitted                 | degrade gracefully | structural part may be absent except for required parts |
| browser-only focus-visible hints are unavailable during SSR     | no-op              | structure still renders correctly                       |

## 13. Identity and Key Policy

The checkbox instance owns a single `Root` / `Control` / `HiddenInput` identity. Hydration must preserve that identity and must not move `HiddenInput` outside the component subtree.

## 14. SSR and Client Boundary Rules

- SSR must render the same structural parts implied by the initial machine state.
- Client-only behavior is limited to event handling and focus-visible detection.
- Hidden-input submission attrs must match between server markup and hydrated markup.

## 15. Performance Constraints

- Derive attrs once per reactive change, not per event callback.
- Do not mirror machine state into separate local signals just to drive `Indicator` or `HiddenInput`.
- Keep focus-visible bookkeeping instance-local.

## 16. Implementation Dependencies

| Dependency | Required?   | Dependency type      | Why it must exist first                                       |
| ---------- | ----------- | -------------------- | ------------------------------------------------------------- |
| `field`    | required    | composition contract | shared label, description, and error wiring must stay uniform |
| `fieldset` | recommended | disabled propagation | group-disabled semantics must match other form controls       |

## 17. Recommended Implementation Sequence

1. Initialize the checkbox machine and derive attrs for all required parts.
2. Render `Root`, `Label`, `Control`, `Indicator`, and `HiddenInput` in the documented order.
3. Wire controlled prop synchronization for `checked`, `disabled`, `readonly`, `required`, and `invalid`.
4. Compose pointer, click, and keyboard activation on `Control`.
5. Add optional field-context integration and final diagnostics.

## 18. Anti-Patterns

- Do not treat `HiddenInput` as the interactive node.
- Do not clear `Indeterminate` inside the adapter unless the controlled parent actually updates the prop.
- Do not let consumer handlers override required ARIA attrs on `Control`.

## 19. Consumer Expectations and Guarantees

- Consumers may assume that `Control` remains the focusable checkbox role owner.
- Consumers may assume that the hidden input remains adapter-owned and form-safe.
- Consumers must not assume undocumented wrapper removal, alternate part order, or implicit checkbox-group delegation.

## 20. Platform Support Matrix

| Capability / behavior                                     | Web          | Desktop      | Mobile       | SSR          | Notes                                   |
| --------------------------------------------------------- | ------------ | ------------ | ------------ | ------------ | --------------------------------------- |
| documented checkbox semantics and hidden-input submission | full support | full support | full support | full support | interactivity activates after hydration |

## 21. Debug Diagnostics and Production Policy

| Condition                                       | Debug build behavior | Production behavior | Notes                                        |
| ----------------------------------------------- | -------------------- | ------------------- | -------------------------------------------- |
| controlled/uncontrolled mode switch after mount | debug warning        | warn and ignore     | preserves current mode                       |
| invalid part composition from a wrapper         | fail fast            | fail fast           | only for adapter-owned structural violations |

## 22. Shared Adapter Helper Notes

| Helper concept      | Required? | Responsibility                                              | Notes                                  |
| ------------------- | --------- | ----------------------------------------------------------- | -------------------------------------- |
| hidden-input helper | required  | mirror checked state and form attrs into the hidden input   | shared with `switch` and `radio-group` |
| field merge helper  | required  | merge described-by, disabled, readonly, and invalid context | reuse utility-layer field integration  |

## 23. Framework-Specific Behavior

Dioxus should prefer plain props for the public surface, `use_context_provider` / `try_use_context` for field integration, and `use_effect` only for the prop synchronization paths that wrappers actually need after mount.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<checkbox::Machine>(props);
let control_attrs = machine.derive(|api| api.control_attrs());
let hidden_input_attrs = machine.derive(|api| api.hidden_input_attrs());

rsx! {
    div { ..machine.derive(|api| api.root_attrs()).read().clone(),
        label { ..machine.derive(|api| api.label_attrs()).read().clone(), {children} }
        div {
            ..control_attrs.read().clone(),
            onclick: move |_| machine.send.call(checkbox::Event::Toggle),
            div { ..machine.derive(|api| api.indicator_attrs()).read().clone() }
        }
        input { ..hidden_input_attrs.read().clone() }
    }
}
```

## 25. Reference Implementation Skeleton

- Build props from explicit props plus merged field or fieldset context.
- Initialize the machine, then set up only the controlled prop watchers that were actually supplied.
- Derive attrs for every structural part, render in stable order, and keep `HiddenInput` fully adapter-owned.

## 26. Adapter Invariants

- `Control` always owns `role="checkbox"` and keyboard activation.
- `HiddenInput` always owns native submission and reset semantics.
- `Indicator` is never announced by assistive technology.
- Controlled `Indeterminate` must survive user interaction until the parent clears it.

## 27. Accessibility and SSR Notes

- `aria-describedby` must include `Description` first and `ErrorMessage` second when both exist.
- `aria-errormessage` must only appear when invalid content is actually rendered.
- SSR markup must not omit `HiddenInput`, because that would change the form-participation contract on hydration.

## 28. Parity Summary and Intentional Deviations

- Matches the core checkbox contract without intentional adapter divergence.
- Promotes hidden-input ownership, focus-visible handling, and controlled-indeterminate behavior from the core spec into Dioxus-facing implementation guidance.

## 29. Test Scenarios

- Controlled `checked` updates move between `Unchecked`, `Checked`, and `Indeterminate` without dropping described-by wiring.
- Keyboard `Space` toggles `Control` but never focuses `HiddenInput`.
- Form submission includes the checkbox value only when the machine state is `Checked`.
- Invalid state renders `ErrorMessage` and updates `aria-describedby` in the correct order.

## 30. Test Oracle Notes

- Inspect the DOM for `role="checkbox"`, `aria-checked`, and hidden-input `checked` synchronization.
- Assert callback order by logging machine-driven checked changes rather than raw click handlers.
- Use a hydration test to verify identical server and client structure for `HiddenInput`.

## 31. Implementation Checklist

- [ ] Render `Root`, `Label`, `Control`, `Indicator`, and `HiddenInput` in stable order.
- [ ] Keep `Control` as the only interactive node.
- [ ] Synchronize controlled props through machine events, not local shadow state.
- [ ] Preserve `Indeterminate` semantics in controlled mode.
- [ ] Keep hidden-input submission and reset behavior adapter-owned.
- [ ] Merge field-derived described-by, disabled, readonly, and invalid state.
