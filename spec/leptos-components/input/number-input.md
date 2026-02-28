---
adapter: leptos
component: number-input
category: input
source: components/input/number-input.md
source_foundation: foundation/08-adapter-leptos.md
---

# NumberInput — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`NumberInput`](../../components/input/number-input.md) contract onto a Leptos 0.8.x component. The adapter must preserve native spinbutton semantics, locale-aware parse and format behavior, stepper-trigger ownership, and optional scrub or wheel behavior.

## 2. Public Adapter API

```rust
#[component]
pub fn NumberInput(
    #[prop(optional, into)] value: Option<Signal<f64>>,
    #[prop(optional)] default_value: Option<f64>,
    #[prop(optional, into)] min: Signal<f64>,
    #[prop(optional, into)] max: Signal<f64>,
    #[prop(optional, into)] step: Signal<f64>,
    #[prop(optional)] large_step: Option<f64>,
    #[prop(optional)] allow_mouse_wheel: bool,
    #[prop(optional)] spin_on_press: bool,
    #[prop(optional)] clamp_value_on_blur: bool,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] readonly: Signal<bool>,
    #[prop(optional, into)] invalid: Signal<bool>,
    #[prop(optional)] name: Option<String>,
    #[prop(optional)] form: Option<String>,
    children: Children,
) -> impl IntoView
```

The adapter also forwards locale, messages, and formatting options from the core contract. `value`, `min`, `max`, `step`, `disabled`, `readonly`, and `invalid` are the normal reactive inputs after mount.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core number-input contract, including locale-aware parsing, rounding, wheel behavior, and scrub support.
- Event parity: increment or decrement, large-step, min or max jumps, scrub events, `Input`, `Blur`, `CompositionStart`, and controlled `Set*` events remain machine-owned.
- Core machine ownership: `use_machine::<number_input::Machine>(...)` owns the numeric value, formatted text, and boundary state.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target | Ownership     | Attr source                     | Notes                                   |
| --------------------- | --------- | ------------------------ | ------------- | ------------------------------- | --------------------------------------- |
| `Root`                | required  | `<div>`                  | adapter-owned | `api.root_attrs()`              | structural wrapper                      |
| `Label`               | required  | `<label>`                | adapter-owned | `api.label_attrs()`             | linked to `Input`                       |
| `Input`               | required  | native `<input>`         | adapter-owned | `api.input_attrs()`             | owns `role="spinbutton"` and submission |
| `IncrementTrigger`    | optional  | `<button>`               | adapter-owned | `api.increment_trigger_attrs()` | optional stepper button                 |
| `DecrementTrigger`    | optional  | `<button>`               | adapter-owned | `api.decrement_trigger_attrs()` | optional stepper button                 |
| `Description`         | optional  | `<div>`                  | adapter-owned | `api.description_attrs()`       | described-by content                    |
| `ErrorMessage`        | optional  | `<div>`                  | adapter-owned | `api.error_message_attrs()`     | invalid-only content                    |

## 5. Attr Merge and Ownership Rules

- Native spinbutton attrs, locale-derived `aria-valuetext`, and form attrs on `Input` always win.
- Trigger buttons keep adapter-owned labels, repeat behavior, and `type="button"` semantics.
- Consumer handlers may decorate the input, but parsing, clamping, and rounding remain adapter-owned.

## 6. Composition / Context Contract

`NumberInput` consumes field, fieldset, and optional form context. Scrub handles or custom trigger visuals may be layered by wrappers, but the primary numeric input and stepper triggers remain adapter-owned.

## 7. Prop Sync and Event Mapping

| Adapter prop                               | Mode          | Sync trigger  | Machine event / update path                  | Visible effect                                  |
| ------------------------------------------ | ------------- | ------------- | -------------------------------------------- | ----------------------------------------------- |
| `value`                                    | controlled    | signal change | `SetValue`                                   | updates numeric value, text, and boundary state |
| `min` / `max` / `step`                     | controlled    | signal change | `SetMin` / `SetMax` / `SetStep`              | updates spinbutton bounds and step behavior     |
| `disabled` / `readonly` / `invalid`        | controlled    | signal change | `SetDisabled` / `SetReadonly` / `SetInvalid` | updates guards and ARIA attrs                   |
| input, wheel, scrub, or trigger activation | machine-owned | DOM events    | numeric events                               | updates numeric value through machine           |

Locale-aware parsing and rounding always occur before value callbacks fire. Press-and-hold repeat timers belong to the adapter but dispatch only machine events.

## 8. Registration and Cleanup Contract

- Press-and-hold repeat timers are adapter-owned resources that must be cancelled on pointerup, pointerleave, and unmount.
- Wheel listeners, when enabled, must be local to the input instance and removed on unmount.
- No descendant registry is required.

## 9. Ref and Node Contract

- `Input` owns the primary live ref for native selection, blur, and measurement needs.
- Stepper triggers may own local refs for pointer capture only.
- Any scrub helper must target the same machine-owned input and value path.

## 10. State Machine Boundary Rules

- The machine owns numeric value, formatted text, and boundary logic.
- The adapter may parse raw text or wheel deltas into machine events, but it must not commit numeric state outside the machine.
- Native input submission remains direct; there is no hidden-input bridge.

## 11. Callback Payload Contract

- Value-change callbacks emit the committed numeric value after parsing, clamping, and rounding.
- Invalid-paste or invalid-input diagnostics should reflect the rejected raw text without mutating machine state.
- Focus callbacks may expose `is_keyboard` for focus-visible handling.

## 12. Failure and Degradation Rules

| Condition                                                     | Policy             | Notes                                                   |
| ------------------------------------------------------------- | ------------------ | ------------------------------------------------------- |
| controlled and uncontrolled value props are mixed after mount | warn and ignore    | first mode wins                                         |
| locale parsing helpers are unavailable                        | degrade gracefully | adapter may fall back to canonical numeric parsing only |
| scrub or wheel helpers are unsupported on the current host    | degrade gracefully | text input and stepper buttons still work               |

## 13. Identity and Key Policy

`Root`, `Input`, and both stepper triggers belong to one number-input instance. Hydration must preserve the native input node identity.

## 14. SSR and Client Boundary Rules

- SSR renders the same structure, initial numeric text, and ARIA attrs as the hydrated client.
- Wheel, scrub, and repeat-timer behavior are client-only.
- The native input remains the direct form participant in both SSR and hydrated modes.

## 15. Performance Constraints

- Parse and format only on meaningful input or controlled-prop changes.
- Cancel repeat timers promptly when they are no longer needed.
- Avoid mirroring numeric state into extra local signals outside the machine.

## 16. Implementation Dependencies

| Dependency                   | Required?   | Dependency type      | Why it must exist first                                              |
| ---------------------------- | ----------- | -------------------- | -------------------------------------------------------------------- |
| field merge helper           | required    | composition contract | label, description, and validation wiring must stay uniform          |
| measurement or timer helpers | recommended | adapter helper       | repeat timing, wheel, and scrub behavior should reuse stable helpers |

## 17. Recommended Implementation Sequence

1. Initialize the machine and derive attrs for the input and optional triggers.
2. Render `Root`, `Label`, native `Input`, optional stepper triggers, and optional status parts in stable order.
3. Wire controlled sync for value, min, max, step, and validation-related props.
4. Add input parsing, repeat-timer, wheel, and scrub event handling.
5. Finish locale, rounding, and diagnostics behavior.

## 18. Anti-Patterns

- Do not commit numeric changes directly from raw text without the machine.
- Do not keep repeat timers alive after pointer exit or unmount.
- Do not let trigger buttons bypass locale-aware rounding and clamping rules.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the native input remains the only form-submitting element.
- Consumers may assume stepper triggers and wheel behavior follow machine-owned bounds and rounding rules.
- Consumers must not assume undocumented hidden inputs or wrapper-only numeric state.

## 20. Platform Support Matrix

| Capability / behavior                                                  | Browser client | SSR          | Notes                                            |
| ---------------------------------------------------------------------- | -------------- | ------------ | ------------------------------------------------ |
| documented spinbutton semantics, locale parsing, and direct submission | full support   | full support | wheel and scrub helpers activate after hydration |

## 21. Debug Diagnostics and Production Policy

| Condition                                       | Debug build behavior | Production behavior | Notes                                    |
| ----------------------------------------------- | -------------------- | ------------------- | ---------------------------------------- |
| controlled/uncontrolled mode switch after mount | debug warning        | warn and ignore     | preserves current mode                   |
| unsupported locale parsing path                 | debug warning        | degrade gracefully  | falls back to reduced parsing capability |

## 22. Shared Adapter Helper Notes

| Helper concept                 | Required? | Responsibility                                                     | Notes                                            |
| ------------------------------ | --------- | ------------------------------------------------------------------ | ------------------------------------------------ |
| timer helper                   | required  | own press-and-hold repeat timers                                   | shared with other repeating triggers             |
| composition and parsing helper | required  | suppress IME side effects and perform locale-aware parse or format | shared with other numeric or text-entry controls |

## 23. Framework-Specific Behavior

Leptos should keep prop watchers for `min`, `max`, and `step` narrow, attach wheel listeners only when enabled, and cancel repeat timers via `on_cleanup` or pointer-end paths owned by this instance.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<number_input::Machine>(props);

view! {
    <div {..machine.derive(|api| api.root_attrs()).get()}>
        <label {..machine.derive(|api| api.label_attrs()).get()}>{children()}</label>
        <input {..machine.derive(|api| api.input_attrs()).get()} />
        <button {..machine.derive(|api| api.increment_trigger_attrs()).get()} />
        <button {..machine.derive(|api| api.decrement_trigger_attrs()).get()} />
    </div>
}
```

## 25. Reference Implementation Skeleton

- Merge field context and explicit props before machine initialization.
- Register only the controlled watchers that exist for value and numeric constraints.
- Keep parse, clamp, rounding, repeat timers, wheel behavior, and scrub translation all feeding the same machine events.

## 26. Adapter Invariants

- `Input` always remains native and directly form-participating.
- Parsing, clamping, and rounding happen before value callbacks fire.
- Repeat timers are always cancelled on pointer-end and unmount.
- Stepper triggers and scrub helpers always dispatch through machine events only.

## 27. Accessibility and SSR Notes

- Description-first, error-second ordering is mandatory for `aria-describedby`.
- `aria-valuetext` must reflect locale-aware formatting whenever available.
- SSR must preserve the initial formatted value and spinbutton attrs used by the client.

## 28. Parity Summary and Intentional Deviations

- Matches the core number-input contract without intentional divergence.
- Promotes locale parsing, repeat-timer ownership, and machine-only numeric mutation into Leptos-facing guidance.

## 29. Test Scenarios

- Controlled value, min, max, and step updates synchronize the spinbutton without stale boundary state.
- Press-and-hold triggers repeat until pointer release and then stop cleanly.
- Locale-specific pasted text parses, clamps, rounds, and updates the input correctly.
- Wheel or scrub helpers degrade gracefully when unsupported.

## 30. Test Oracle Notes

- Inspect spinbutton ARIA attrs and direct form-submission attrs on the native input.
- Use fake timers to verify repeat-trigger scheduling and cancellation.
- Assert value callbacks from machine-driven state transitions rather than from raw input text alone.

## 31. Implementation Checklist

- [ ] Keep `Input` native and directly form-participating.
- [ ] Route parsing, rounding, and value changes through machine events only.
- [ ] Synchronize controlled value and numeric-constraint props through dedicated watchers.
- [ ] Cancel repeat timers on pointer-end and unmount.
- [ ] Preserve locale-aware `aria-valuetext` and parsing behavior.
- [ ] Keep trigger buttons adapter-owned and semantically stable.
