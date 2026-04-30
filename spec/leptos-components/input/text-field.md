---
adapter: leptos
component: text-field
category: input
source: components/input/text-field.md
source_foundation: foundation/08-adapter-leptos.md
---

# TextField — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`TextField`](../../components/input/text-field.md) contract onto a Leptos 0.8.x component. The adapter must preserve native text-input submission, decorator slots, clear-trigger behavior, and IME-aware value synchronization.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn TextField(
    #[prop(optional, into)] value: Option<Signal<String>>,
    #[prop(optional)] default_value: String,
    #[prop(optional)] placeholder: Option<String>,
    #[prop(optional)] input_type: text_field::InputType,
    #[prop(optional)] input_mode: Option<InputMode>,
    #[prop(optional)] clearable: bool,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] readonly: Signal<bool>,
    #[prop(optional, into)] required: Signal<bool>,
    #[prop(optional, into)] invalid: Signal<bool>,
    #[prop(optional)] name: Option<String>,
    #[prop(optional)] form: Option<String>,
    children: Children,
) -> impl IntoView
```

The adapter also exposes the shared field props from the core contract, including locale or messages, pattern and length constraints, and optional decorator or clear-trigger slots. `value`, `disabled`, `readonly`, `required`, and `invalid` are the default post-mount reactive inputs.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core text-field contract, including input mode, decorator parts, and clear behavior.
- Event parity: `Input`, `Focus`, `Blur`, `Clear`, `CompositionStart`, `CompositionEnd`, and controlled `Set*` events remain machine-owned.
- Core machine ownership: `use_machine::<text_field::Machine>(...)` remains the source of truth for value, invalid state, and clear-trigger visibility.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source                   | Notes                                       |
| --------------------- | --------- | ------------------------ | ------------- | ----------------------------- | ------------------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()`            | structural wrapper for decorators and input |
| `Label`               | required  | `<label>`                | adapter-owned | `api.label_attrs()`           | must stay linked to `Input`                 |
| `Input`               | required  | native `<input>`         | adapter-owned | `api.input_attrs()`           | direct form-participation owner             |
| `StartDecorator`      | optional  | `<div>`                  | shared        | `api.start_decorator_attrs()` | decorative or semantic prefix               |
| `EndDecorator`        | optional  | `<div>`                  | shared        | `api.end_decorator_attrs()`   | decorative or semantic suffix               |
| `ClearTrigger`        | optional  | `<button>`               | adapter-owned | `api.clear_trigger_attrs()`   | shown only when clearable and non-empty     |
| `Description`         | optional  | `<div>`                  | adapter-owned | `api.description_attrs()`     | described-by content                        |
| `ErrorMessage`        | optional  | `<div>`                  | adapter-owned | `api.error_message_attrs()`   | invalid-only content                        |

## 5. Attr Merge and Ownership Rules

- Native `type`, `inputmode`, `name`, `form`, `required`, `maxlength`, `minlength`, `pattern`, and ARIA attrs on `Input` always win.
- `class` and `style` merge additively across `Root`, decorators, and `Input`.
- Consumer handlers may decorate `Input`, but IME and value-change sequencing remain adapter-owned.

## 6. Composition / Context Contract

`TextField` consumes field, fieldset, and optional form context. Decorator content is consumer-owned, but the structural wrapper order and clear-trigger placement are adapter-owned.

## 7. Prop Sync and Event Mapping

| Adapter prop            | Mode                       | Sync trigger    | Machine event / update path   | Visible effect                                          |
| ----------------------- | -------------------------- | --------------- | ----------------------------- | ------------------------------------------------------- |
| `value`                 | controlled                 | signal change   | `SetValue`                    | updates native input value and clear-trigger visibility |
| `disabled` / `readonly` | controlled                 | signal change   | `SetDisabled` / `SetReadonly` | updates native attrs and guards                         |
| `required` / `invalid`  | controlled                 | signal change   | `SetRequired` / `SetInvalid`  | updates ARIA and validation wiring                      |
| native `input`          | uncontrolled or controlled | DOM input event | `InputChange`                 | updates machine value                                   |

During active IME composition, the adapter must suppress eager callback emission and any typeahead-like wrapper behavior. `ClearTrigger` maps to `Clear`, not to direct DOM mutation.

## 8. Registration and Cleanup Contract

- No descendant registry is required.
- IME state, if mirrored in the adapter for event suppression, is instance-local and dropped on unmount.
- No global listeners are required beyond local input events.

## 9. Ref and Node Contract

- `Input` owns the live node ref.
- Decorators and clear trigger may have refs for layout purposes only.
- Field-level focus repair always targets `Input`.

## 10. State Machine Boundary Rules

- The machine owns the committed value, empty-state detection, and clear-trigger visibility.
- The adapter may normalize raw DOM events, but it must not derive alternate values outside the machine path.
- Placeholder fallback naming is adapter-owned accessibility repair, not a second value source.

## 11. Callback Payload Contract

- Value-change callbacks emit the committed string value.
- Clear callbacks emit after the machine has committed the empty value.
- Focus callbacks may expose `is_keyboard` for focus-visible state.

## 12. Failure and Degradation Rules

| Condition                                                     | Policy             | Notes                                                                  |
| ------------------------------------------------------------- | ------------------ | ---------------------------------------------------------------------- |
| controlled and uncontrolled value props are mixed after mount | warn and ignore    | first mode wins                                                        |
| IME composition events are unavailable                        | degrade gracefully | raw input still works, but composition-specific suppression is reduced |
| clear trigger renders without a reachable input               | fail fast          | structural contract violation                                          |

## 13. Identity and Key Policy

`Root`, `Input`, and any rendered decorators or clear trigger belong to one text-field instance. Hydration must preserve the `Input` node identity.

## 14. SSR and Client Boundary Rules

- SSR renders the same part structure and initial input attrs as the hydrated client.
- IME handling and focus-visible behavior are client-only.
- The input must remain native and directly form-participating on both server and client.

## 15. Performance Constraints

- Avoid shadow state that duplicates the current value outside the machine.
- Keep decorator rendering independent of keystroke-level layout thrash where possible.
- Do not attach both native and synthetic clear paths that each mutate the input.

## 16. Implementation Dependencies

| Dependency | Required?   | Dependency type               | Why it must exist first                                                |
| ---------- | ----------- | ----------------------------- | ---------------------------------------------------------------------- |
| `field`    | required    | composition contract          | label, description, and error wiring must stay uniform                 |
| `form`     | recommended | submission and reset contract | external form association and reset behavior should match other inputs |

## 17. Recommended Implementation Sequence

1. Initialize the machine with the merged core props.
2. Render `Root`, `Label`, `Input`, optional decorators, and optional clear trigger in stable order.
3. Wire controlled prop synchronization.
4. Compose native input, focus, blur, and composition events into machine events.
5. Add clear-trigger behavior and final diagnostics.

## 18. Anti-Patterns

- Do not replace the native input with a non-input host element.
- Do not emit change callbacks during active IME composition unless the wrapper explicitly documents streaming behavior.
- Do not treat decorators as alternate label or described-by owners.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the native `Input` remains the form-submitting element.
- Consumers may assume `ClearTrigger` visibility follows machine value state.
- Consumers must not assume undocumented decorator reordering or automatic popup behavior.

## 20. Platform Support Matrix

| Capability / behavior                                                | Browser client | SSR          | Notes                                          |
| -------------------------------------------------------------------- | -------------- | ------------ | ---------------------------------------------- |
| documented text input semantics, IME handling, and native submission | full support   | full support | composition behavior activates after hydration |

## 21. Debug Diagnostics and Production Policy

| Condition                                       | Debug build behavior | Production behavior | Notes                                         |
| ----------------------------------------------- | -------------------- | ------------------- | --------------------------------------------- |
| controlled/uncontrolled mode switch after mount | debug warning        | warn and ignore     | preserves current mode                        |
| missing accessible label with no valid fallback | debug warning        | degrade gracefully  | placeholder fallback remains last resort only |

## 22. Shared Adapter Helper Notes

| Helper concept     | Required? | Responsibility                                                 | Notes                                                                    |
| ------------------ | --------- | -------------------------------------------------------------- | ------------------------------------------------------------------------ |
| field merge helper | required  | merge IDs, described-by, disabled, readonly, and invalid state | shared by text-entry controls                                            |
| composition helper | required  | suppress eager callbacks during IME composition                | shared with `textarea`, `search-input`, `password-input`, and `editable` |

## 23. Framework-Specific Behavior

Leptos should keep the native input in a `NodeRef`, use `Effect::new` plus `StoredValue` for controlled prop watchers, and keep composition suppression local to the component instance.

## 24. Canonical Implementation Sketch

```rust,no_check
let machine = use_machine::<text_field::Machine>(props);

view! {
    <div {..machine.derive(|api| api.root_attrs()).get()}>
        <label {..machine.derive(|api| api.label_attrs()).get()}>{children()}</label>
        <input
            {..machine.derive(|api| api.input_attrs()).get()}
            on:input=move |ev| send_input(ev, &machine)
            on:compositionstart=move |_| machine.send.run(text_field::Event::CompositionStart)
            on:compositionend=move |_| machine.send.run(text_field::Event::CompositionEnd)
        />
        <Show when=move || show_clear(&machine)>
            <button {..machine.derive(|api| api.clear_trigger_attrs()).get()} />
        </Show>
    </div>
}
```

## 25. Reference Implementation Skeleton

- Merge field context and explicit props before machine initialization.
- Initialize the machine, then register only the controlled value and state watchers that exist.
- Render the structural parts in fixed order and keep all DOM input events flowing through the machine.

## 26. Adapter Invariants

- `Input` always remains a native text input and the only form-submitting node.
- IME composition suppresses eager clear or change callback side effects.
- `ClearTrigger` never mutates the DOM value directly; it always goes through the machine.
- Description and error wiring always target `Input`.

## 27. Accessibility and SSR Notes

- Description-first, error-second ID ordering is mandatory for `aria-describedby`.
- `aria-label` placeholder fallback is a repair path, not the preferred accessible-name path.
- SSR must preserve the same native input attrs used by the hydrated client.

## 28. Parity Summary and Intentional Deviations

- Matches the core text-field contract without intentional divergence.
- Promotes IME handling, clear-trigger ownership, and native-input submission rules into Leptos-facing guidance.

## 29. Test Scenarios

- Controlled value updates synchronize with the native input without losing selection or IME composition state.
- Clear trigger empties the field through the machine and hides itself afterward.
- Invalid state updates `aria-describedby`, `aria-errormessage`, and native constraint attrs correctly.
- Missing explicit label falls back only to the documented accessible-name repair path.

## 30. Test Oracle Notes

- Inspect the DOM for the native input `type`, constraint attrs, and described-by ordering.
- Use composition-event tests to verify callback suppression during IME entry.
- Assert clear behavior from the machine callback log rather than by comparing raw DOM mutations alone.

## 31. Implementation Checklist

- [ ] Keep `Input` native and directly form-participating.
- [ ] Render decorators and clear trigger only in the documented structural positions.
- [ ] Route controlled value and native input changes through machine events.
- [ ] Suppress eager callbacks during IME composition.
- [ ] Preserve description and error-message ordering on `aria-describedby`.
- [ ] Keep clear-trigger visibility machine-driven.
