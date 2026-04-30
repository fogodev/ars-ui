---
adapter: dioxus
component: password-input
category: input
source: components/input/password-input.md
source_foundation: foundation/09-adapter-dioxus.md
---

# PasswordInput — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`PasswordInput`](../../components/input/password-input.md) contract onto a Dioxus 0.7.x component. The adapter must preserve native password submission, visibility toggling, and IME-aware text entry.

## 2. Public Adapter API

```rust,no_check
#[derive(Props, Clone, PartialEq)]
pub struct PasswordInputProps {
    #[props(optional)]
    pub value: Option<String>,
    #[props(optional)]
    pub default_value: Option<String>,
    #[props(default = false)]
    pub default_visible: bool,
    #[props(optional)]
    pub autocomplete: Option<String>,
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
    pub form: Option<String>,
    pub children: Element,
}

#[component]
pub fn PasswordInput(props: PasswordInputProps) -> Element
```

The adapter also exposes the shared field props from the core contract, including locale or messages and `on_visibility_change`-style callbacks. Value and validation props are the normal reactive surface; visibility remains machine-owned unless a wrapper documents a controlled bridge.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core password-input contract, including visibility state, autocomplete, and form props.
- Event parity: `Input`, `ToggleVisibility`, `Focus`, `Blur`, `CompositionStart`, `CompositionEnd`, and controlled `Set*` events remain machine-owned.
- Core machine ownership: `use_machine::<password_input::Machine>(...)` owns value, visible or masked state, and toggle labeling.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source                 | Notes                              |
| --------------------- | --------- | ------------------------ | ------------- | --------------------------- | ---------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()`          | structural wrapper                 |
| `Label`               | required  | `<label>`                | adapter-owned | `api.label_attrs()`         | linked to `Input`                  |
| `Input`               | required  | native `<input>`         | adapter-owned | `api.input_attrs()`         | `type="password"` or `type="text"` |
| `Toggle`              | required  | `<button>`               | adapter-owned | `api.toggle_attrs()`        | toggles visible state              |
| `Description`         | optional  | `<div>`                  | adapter-owned | `api.description_attrs()`   | described-by content               |
| `ErrorMessage`        | optional  | `<div>`                  | adapter-owned | `api.error_message_attrs()` | invalid-only content               |

## 5. Attr Merge and Ownership Rules

- Native `type`, `autocomplete`, `name`, `form`, and ARIA attrs on `Input` always win.
- `Toggle` keeps adapter-owned labels and `type="button"` semantics even when consumers decorate it.
- Consumer handlers may decorate the input, but IME handling and visibility toggling remain adapter-owned.

## 6. Composition / Context Contract

`PasswordInput` consumes field, fieldset, and optional form context. The toggle is adapter-owned and must remain within the component subtree so label, described-by, and visibility semantics stay consistent.

## 7. Prop Sync and Event Mapping

| Adapter prop            | Mode          | Sync trigger                 | Machine event / update path   | Visible effect                       |
| ----------------------- | ------------- | ---------------------------- | ----------------------------- | ------------------------------------ |
| `value`                 | controlled    | prop change                  | `SetValue`                    | updates native input value           |
| `disabled` / `readonly` | controlled    | prop change                  | `SetDisabled` / `SetReadonly` | guards editing and toggling          |
| `required` / `invalid`  | controlled    | prop change                  | `SetRequired` / `SetInvalid`  | updates ARIA and described-by wiring |
| toggle press            | machine-owned | click or keyboard activation | `ToggleVisibility`            | flips `Input` type and toggle label  |

IME composition follows the same suppression rules as `TextField`. Toggle presses must not blur the input unexpectedly unless the native host does so and the wrapper documents it.

## 8. Registration and Cleanup Contract

- No descendant registry is required.
- No global listeners are required.
- Visibility state is machine-owned and resets by normal component teardown only.

## 9. Ref and Node Contract

- `Input` owns the primary live node ref.
- `Toggle` may own a secondary ref for focus restoration, but it never replaces the input as the value owner.
- Wrappers that preserve caret position on visibility changes must use the input ref only.

## 10. State Machine Boundary Rules

- The machine owns both the committed value and visible or masked state.
- The adapter must not toggle `type` directly without the machine transition.
- Native password submission remains direct; there is no hidden-input bridge.

## 11. Callback Payload Contract

- Value-change callbacks emit the committed string value.
- Visibility-change callbacks emit the committed visible boolean after the machine transition.
- Focus callbacks may include `is_keyboard` for focus-visible behavior.

## 12. Failure and Degradation Rules

| Condition                                                     | Policy             | Notes                         |
| ------------------------------------------------------------- | ------------------ | ----------------------------- |
| controlled and uncontrolled value props are mixed after mount | warn and ignore    | first mode wins               |
| toggle renders without the input it controls                  | fail fast          | structural contract violation |
| IME composition events are unavailable                        | degrade gracefully | raw input still works         |

## 13. Identity and Key Policy

`Root`, `Input`, and `Toggle` belong to one password-input instance. Hydration must preserve the native input node so visibility changes do not recreate it unnecessarily.

## 14. SSR and Client Boundary Rules

- SSR renders the same part structure and initial masked or visible state as the client.
- IME and focus-visible behavior are client-only layers over stable markup.
- The native input remains the direct form participant in both SSR and hydrated modes.

## 15. Performance Constraints

- Toggling visibility should update only the native input attrs and toggle label, not rebuild unrelated structure.
- Do not mirror password value into extra local state outside the machine.
- Avoid duplicate toggle handlers that each attempt to flip visibility.

## 16. Implementation Dependencies

| Dependency         | Required?   | Dependency type       | Why it must exist first                                             |
| ------------------ | ----------- | --------------------- | ------------------------------------------------------------------- |
| `field`            | required    | composition contract  | label, description, and error wiring must stay uniform              |
| text-entry helpers | recommended | shared adapter helper | IME and native input handling should stay aligned with `text-field` |

## 17. Recommended Implementation Sequence

1. Initialize the machine with merged props and contexts.
2. Render `Root`, `Label`, `Input`, `Toggle`, and optional field-status parts in stable order.
3. Wire controlled prop synchronization and input or composition events.
4. Compose toggle activation into `ToggleVisibility`.
5. Finish diagnostics, caret-preservation policy, and SSR checks.

## 18. Anti-Patterns

- Do not store a second password copy outside the machine.
- Do not toggle input `type` directly from the button without the machine event.
- Do not blur the input on every toggle unless that divergence is explicitly documented.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the native `Input` remains the form-submitting element.
- Consumers may assume the toggle label and state follow the machine's visible or masked state.
- Consumers must not assume undocumented controlled visibility props beyond the documented adapter bridge.

## 20. Platform Support Matrix

| Capability / behavior                                                       | Web          | Desktop      | Mobile       | SSR          | Notes                                  |
| --------------------------------------------------------------------------- | ------------ | ------------ | ------------ | ------------ | -------------------------------------- |
| documented password input semantics, toggle behavior, and native submission | full support | full support | full support | full support | IME behavior activates after hydration |

## 21. Debug Diagnostics and Production Policy

| Condition                                       | Debug build behavior | Production behavior | Notes                            |
| ----------------------------------------------- | -------------------- | ------------------- | -------------------------------- |
| controlled/uncontrolled mode switch after mount | debug warning        | warn and ignore     | preserves current mode           |
| toggle cannot resolve a controlled input        | fail fast            | fail fast           | protects visible-state ownership |

## 22. Shared Adapter Helper Notes

| Helper concept     | Required? | Responsibility                                                 | Notes                                 |
| ------------------ | --------- | -------------------------------------------------------------- | ------------------------------------- |
| field merge helper | required  | merge IDs, described-by, disabled, readonly, and invalid state | shared by text-entry controls         |
| composition helper | required  | suppress eager callbacks during IME composition                | shared with other text-entry controls |

## 23. Framework-Specific Behavior

Dioxus should use the same input and composition wiring as `TextField`, keep the toggle as a local button with `type="button"`, and preserve input node identity across visibility changes.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<password_input::Machine>(props);

rsx! {
    div { ..machine.derive(|api| api.root_attrs()).read().clone(),
        label { ..machine.derive(|api| api.label_attrs()).read().clone(), {children} }
        input { ..machine.derive(|api| api.input_attrs()).read().clone() }
        button {
            ..machine.derive(|api| api.toggle_attrs()).read().clone(),
            onclick: move |_| machine.send.call(password_input::Event::ToggleVisibility),
        }
    }
}
```

## 25. Reference Implementation Skeleton

- Merge field context and explicit props before machine initialization.
- Initialize the machine, then register only the controlled watchers that exist.
- Keep the native input mounted across visibility toggles and route toggle activation through the machine only.

## 26. Adapter Invariants

- `Input` always remains native and directly form-participating.
- `Toggle` always flips visibility through the machine path.
- IME composition suppresses eager side effects.
- Description and error wiring always target `Input`.

## 27. Accessibility and SSR Notes

- Description-first, error-second ordering is mandatory for `aria-describedby`.
- Toggle labels must reflect the post-transition visible state and remain localized.
- SSR must preserve initial masked or visible markup to avoid hydration drift.

## 28. Parity Summary and Intentional Deviations

- Matches the core password-input contract without intentional divergence.
- Promotes toggle ownership, input-node identity, and IME handling into Dioxus-facing guidance.

## 29. Test Scenarios

- Toggle activation flips the input `type` and localized toggle label without losing the current value.
- Controlled value updates synchronize with the native input during masked and visible states.
- Invalid state updates described-by ordering and native constraint attrs correctly.
- IME composition suppresses eager callbacks and resumes normal updates afterward.

## 30. Test Oracle Notes

- Inspect the native input `type`, `autocomplete`, and described-by attrs directly.
- Assert visibility changes from machine-driven callback logs instead of raw click counts.
- Verify the input node identity persists across visibility toggles.

## 31. Implementation Checklist

- [ ] Keep `Input` native and directly form-participating.
- [ ] Keep `Toggle` adapter-owned and `type="button"`.
- [ ] Route visibility changes through the machine only.
- [ ] Preserve input node identity across masked and visible states.
- [ ] Suppress eager callbacks during IME composition.
- [ ] Preserve description and error-message ordering on `aria-describedby`.
