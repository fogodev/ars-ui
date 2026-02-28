---
adapter: leptos
component: search-input
category: input
source: components/input/search-input.md
source_foundation: foundation/08-adapter-leptos.md
---

# SearchInput — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`SearchInput`](../../components/input/search-input.md) contract onto a Leptos 0.8.x component. The adapter must preserve native search-field submission, clear and submit triggers, debounced search state, and IME-aware keyboard propagation.

## 2. Public Adapter API

```rust
#[component]
pub fn SearchInput(
    #[prop(optional, into)] value: Option<Signal<String>>,
    #[prop(optional)] default_value: String,
    #[prop(optional)] placeholder: Option<String>,
    #[prop(optional)] debounce_ms: Option<u32>,
    #[prop(optional)] clearable: bool,
    #[prop(optional)] show_submit_trigger: bool,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] required: Signal<bool>,
    #[prop(optional, into)] invalid: Signal<bool>,
    #[prop(optional)] name: Option<String>,
    #[prop(optional)] form: Option<String>,
    #[prop(optional)] on_submit: Option<Callback<String>>,
    #[prop(optional)] on_clear: Option<Callback<String>>,
    children: Children,
) -> impl IntoView
```

The adapter also forwards locale or messages and other shared field props from the core contract. `on_submit` and `on_clear` are adapter-owned callbacks that fire after the machine accepts `Submit` or `Clear`.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core search-input contract, including debounced searching and loading-indicator behavior.
- Event parity: `Input`, `Focus`, `Blur`, `Clear`, `Submit`, `DebounceExpired`, `CompositionStart`, and `CompositionEnd` remain machine-owned; callbacks are adapter-owned.
- Core machine ownership: `use_machine::<search_input::Machine>(...)` owns value, searching state, busy attrs, and clear-trigger visibility.

## 4. Part Mapping

| Core part / structure | Required?   | Adapter rendering target       | Ownership     | Attr source                     | Notes                               |
| --------------------- | ----------- | ------------------------------ | ------------- | ------------------------------- | ----------------------------------- |
| `Root`                | required    | `<div>`                        | adapter-owned | `api.root_attrs()`              | owns search landmark semantics      |
| `Label`               | required    | `<label>`                      | adapter-owned | `api.label_attrs()`             | linked to `Input`                   |
| `Input`               | required    | native `<input type="search">` | adapter-owned | `api.input_attrs()`             | direct form participant             |
| `ClearTrigger`        | optional    | `<button>`                     | adapter-owned | `api.clear_trigger_attrs()`     | hidden when empty                   |
| `SubmitTrigger`       | optional    | `<button>`                     | adapter-owned | `api.submit_trigger_attrs()`    | optional explicit submit affordance |
| `LoadingIndicator`    | conditional | `<div>`                        | adapter-owned | `api.loading_indicator_attrs()` | shown while searching               |
| `Description`         | optional    | `<div>`                        | adapter-owned | `api.description_attrs()`       | described-by content                |
| `ErrorMessage`        | optional    | `<div>`                        | adapter-owned | `api.error_message_attrs()`     | invalid-only content                |

## 5. Attr Merge and Ownership Rules

- Native `type="search"`, `name`, `form`, `required`, and ARIA attrs on `Input` always win.
- Clear and submit triggers keep adapter-owned labels and `type="button"` semantics.
- Keyboard propagation rules are adapter-owned and must not be overridden by consumer handlers without an explicit higher-level contract.

## 6. Composition / Context Contract

`SearchInput` consumes field, fieldset, and optional form context. When nested inside a higher-level composite such as combobox-like wrappers, the search input still owns its own text-entry and clear or submit behavior while allowing unhandled navigation keys to bubble.

## 7. Prop Sync and Event Mapping

| Adapter prop                        | Mode                       | Sync trigger                 | Machine event / update path                  | Visible effect                                               |
| ----------------------------------- | -------------------------- | ---------------------------- | -------------------------------------------- | ------------------------------------------------------------ |
| `value`                             | controlled                 | signal change                | `SetValue`                                   | updates native input value, clear visibility, and busy state |
| `disabled` / `required` / `invalid` | controlled                 | signal change                | `SetDisabled` / `SetRequired` / `SetInvalid` | updates attrs and validation wiring                          |
| native `input`                      | uncontrolled or controlled | DOM input event              | `InputChange`                                | updates machine value and debounce state                     |
| submit or clear trigger             | machine-owned              | click or keyboard activation | `Submit` / `Clear`                           | updates busy state and callback emission                     |

During IME composition, the adapter must suppress eager submit or clear side effects and must not stop propagation for unhandled navigation keys.

## 8. Registration and Cleanup Contract

- Debounce timers are adapter-owned resources that must be cancelled on unmount or when superseded.
- No descendant registry is required.
- No global listeners are required beyond the local input events and timer ownership.

## 9. Ref and Node Contract

- `Input` owns the live node ref.
- Clear and submit triggers may own refs only for focus restoration or measurement.
- Loading indicator never becomes the focus target.

## 10. State Machine Boundary Rules

- The machine owns the committed value, searching state, and busy attrs.
- Debounce scheduling is adapter-owned timing around machine events, not a second state source.
- Submit callbacks must observe the machine-committed value at submit time.

## 11. Callback Payload Contract

- `on_submit` emits the committed current search string after `Submit`.
- `on_clear` emits the cleared pre-transition value or the committed empty value, whichever convention the wrapper documents consistently; the adapter must pick one policy and keep it stable.
- Value-change callbacks emit committed string values and must respect IME suppression rules.

## 12. Failure and Degradation Rules

| Condition                                                     | Policy             | Notes                                              |
| ------------------------------------------------------------- | ------------------ | -------------------------------------------------- |
| controlled and uncontrolled value props are mixed after mount | warn and ignore    | first mode wins                                    |
| debounce scheduling is unavailable                            | degrade gracefully | search still works without delayed searching state |
| submit or clear trigger renders without the input it controls | fail fast          | structural contract violation                      |

## 13. Identity and Key Policy

`Root`, `Input`, clear or submit triggers, and optional loading indicator belong to one search-input instance. Hydration must preserve the input node identity and any running debounce state must restart cleanly on the client.

## 14. SSR and Client Boundary Rules

- SSR renders the same part structure and initial input attrs as the hydrated client.
- Debounce timers, IME handling, and key propagation policy are client-only.
- The native search input remains the direct form participant in both SSR and hydrated modes.

## 15. Performance Constraints

- Keep debounce state instance-local and cancel superseded timers promptly.
- Do not recompute busy or clear-trigger state outside the machine.
- Avoid duplicate submit paths that each emit callbacks.

## 16. Implementation Dependencies

| Dependency         | Required? | Dependency type      | Why it must exist first                                            |
| ------------------ | --------- | -------------------- | ------------------------------------------------------------------ |
| `field`            | required  | composition contract | label, description, and error wiring must stay uniform             |
| composition helper | required  | adapter helper       | IME-aware key propagation must stay aligned with other text inputs |

## 17. Recommended Implementation Sequence

1. Initialize the machine with merged props and contexts.
2. Render `Root`, `Label`, native `Input`, optional triggers, and optional loading indicator in stable order.
3. Wire controlled prop synchronization and native input, focus, blur, and composition events.
4. Add debounce scheduling and clear or submit callback emission.
5. Finish nested-composite keyboard propagation rules and diagnostics.

## 18. Anti-Patterns

- Do not stop propagation for keys the search input does not own.
- Do not emit submit callbacks from raw keydown handlers before the machine transition.
- Do not treat the loading indicator as a second submit trigger.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the native search input remains the form-submitting node.
- Consumers may assume busy state and loading-indicator visibility follow the machine's searching state.
- Consumers must not assume automatic popup or result-list ownership without a separate composite spec.

## 20. Platform Support Matrix

| Capability / behavior                                               | Browser client | SSR          | Notes                                            |
| ------------------------------------------------------------------- | -------------- | ------------ | ------------------------------------------------ |
| documented search semantics, debounce timers, and native submission | full support   | full support | timers and IME behavior activate after hydration |

## 21. Debug Diagnostics and Production Policy

| Condition                                       | Debug build behavior | Production behavior | Notes                               |
| ----------------------------------------------- | -------------------- | ------------------- | ----------------------------------- |
| controlled/uncontrolled mode switch after mount | debug warning        | warn and ignore     | preserves current mode              |
| trigger wiring without an input owner           | fail fast            | fail fast           | protects clear and submit ownership |

## 22. Shared Adapter Helper Notes

| Helper concept     | Required? | Responsibility                                               | Notes                              |
| ------------------ | --------- | ------------------------------------------------------------ | ---------------------------------- |
| debounce helper    | required  | schedule and cancel search timers                            | shared with search-like composites |
| composition helper | required  | suppress submit or clear side effects during IME composition | shared with text-entry controls    |

## 23. Framework-Specific Behavior

Leptos should use a local timer cleanup path for debounce scheduling, keep the input in a `NodeRef`, and ensure unhandled navigation keys bubble naturally when the search input is nested inside larger composites.

## 24. Canonical Implementation Sketch

```rust
let machine = use_machine::<search_input::Machine>(props);

view! {
    <div {..machine.derive(|api| api.root_attrs()).get()}>
        <label {..machine.derive(|api| api.label_attrs()).get()}>{children()}</label>
        <input
            {..machine.derive(|api| api.input_attrs()).get()}
            on:input=move |ev| schedule_search(ev, &machine)
            on:keydown=move |ev| forward_owned_keys(ev, &machine)
        />
        <Show when=move || show_clear(&machine)>
            <button {..machine.derive(|api| api.clear_trigger_attrs()).get()} />
        </Show>
    </div>
}
```

## 25. Reference Implementation Skeleton

- Merge field context and explicit props before machine initialization.
- Initialize the machine, then register controlled watchers and one debounce helper if needed.
- Keep all submit and clear paths machine-driven and cancel timers on unmount.

## 26. Adapter Invariants

- `Input` always remains native and directly form-participating.
- Debounce timers never outlive the component instance.
- Submit and clear callbacks fire only after the machine accepts the transition.
- Unhandled navigation keys continue bubbling to parent composites.

## 27. Accessibility and SSR Notes

- Description-first, error-second ordering is mandatory for `aria-describedby`.
- Busy state must be reflected on `Root` and any loading indicator must remain non-focusable.
- SSR must preserve the same search input attrs and trigger structure used by the client.

## 28. Parity Summary and Intentional Deviations

- Matches the core search-input contract without intentional divergence.
- Promotes debounce ownership, callback timing, and nested keyboard-propagation rules into Leptos-facing guidance.

## 29. Test Scenarios

- Debounced input transitions into searching state and back without leaking timers.
- Clear and submit triggers fire the correct callbacks with machine-committed values.
- Nested-composite key propagation stops only for keys the search input actually owns.
- Invalid state updates busy, described-by, and native constraint attrs correctly.

## 30. Test Oracle Notes

- Inspect busy attrs, loading-indicator visibility, and native search input attrs in the DOM.
- Use fake timers to verify debounce scheduling and cancellation.
- Assert submit and clear ordering from machine-driven callback logs rather than raw keydown or click handlers.

## 31. Implementation Checklist

- [ ] Keep `Input` native and directly form-participating.
- [ ] Route clear and submit behavior through machine events.
- [ ] Cancel debounce timers on unmount or replacement.
- [ ] Suppress eager submit or clear side effects during IME composition.
- [ ] Bubble unhandled navigation keys to parent composites.
- [ ] Preserve description and error-message ordering on `aria-describedby`.
