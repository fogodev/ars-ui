---
adapter: leptos
component: select
category: selection
source: components/selection/select.md
source_foundation: foundation/08-adapter-leptos.md
---

# Select — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Select`](../../components/selection/select.md) contract onto Leptos 0.8.x. The adapter must preserve triggered popup selection with typeahead and form participation while making compound trigger/content wiring, keyed item registration, typeahead, hidden-input form bridging, and popup positioning explicit at the framework boundary.

## 2. Public Adapter API

```rust
#[component]
pub fn Select(
    #[prop(optional)] id: Option<String>,
    #[prop(optional, into)] value: Option<Signal<selection::Set>>,
    #[prop(optional)] default_value: selection::Set,
    #[prop(optional)] multiple: bool,
    #[prop(optional)] selection_mode: Option<selection::Mode>,
    #[prop(optional)] disabled_keys: Option<BTreeSet<Key>>,
    #[prop(optional)] name: Option<String>,
    #[prop(optional)] form: Option<String>,
    #[prop(optional)] positioning: Option<PositioningOptions>,
    #[prop(optional)] on_open_change: Option<Callback<bool>>,
    children: Children,
) -> impl IntoView
```

Compound helpers typically include `Trigger`, `ValueText`, `Positioner`, `Content`, `SelectItem`, `ItemGroup`, `HiddenInput`, and optional description or indicator parts.

## 3. Mapping to Core Component Contract

- Props parity: full parity with keyed selection, popup open state, disabled-key policy, form participation, and virtualization guidance.
- Part parity: full parity for trigger, popup, item groups, value text, and hidden-input bridge.
- Traceability note: this spec promotes item registration, range-selection normalization, typeahead timeout cleanup, virtualization-aware focus behavior, and described-by wiring from the agnostic contract.

## 4. Part Mapping

| Core part / structure | Required?                       | Adapter rendering target | Ownership     | Attr source              | Notes                                              |
| --------------------- | ------------------------------- | ------------------------ | ------------- | ------------------------ | -------------------------------------------------- |
| Root                  | required                        | wrapper element          | adapter-owned | api.root_attrs()         | Owns compound context and selection scope.         |
| Trigger               | required                        | native button            | adapter-owned | api.trigger_attrs()      | Primary interaction surface for opening the popup. |
| ValueText             | required                        | inline text host         | adapter-owned | api.value_text_attrs()   | Reflects placeholder or selected text.             |
| Positioner            | required                        | positioned wrapper       | adapter-owned | api.positioner_attrs()   | Receives placement output.                         |
| Content               | required                        | listbox host             | adapter-owned | api.content_attrs()      | Contains keyed items and optional groups.          |
| Item                  | repeated                        | option host              | adapter-owned | api.item_attrs(key)      | One per keyed item.                                |
| HiddenInput           | required when `name` is present | native hidden input      | adapter-owned | api.hidden_input_attrs() | Bridges native form submission.                    |
| Description           | optional                        | descriptive node         | shared        | api.description_attrs()  | Participates in described-by wiring.               |

## 5. Attr Merge and Ownership Rules

- Core attrs win for trigger state, listbox semantics, selected or disabled option state, and `aria-expanded` / `aria-controls` linkage.
- The adapter owns positioning variables, hidden-input serialization, and `aria-describedby` composition for description and error content.
- Consumers may decorate trigger or item content through documented parts, but they must not replace the structural trigger, content, or hidden-input hosts.

## 6. Composition / Context Contract

The root publishes required selection context to trigger, content, item, group, and value-text parts. The adapter consumes environment, field, positioning, and optional virtualization helpers. Missing root context is a fail-fast structural error for child parts.

## 7. Prop Sync and Event Mapping

| Adapter prop / event | Mode                                    | Sync trigger                                         | Machine event / update path                         | Notes                                                              |
| -------------------- | --------------------------------------- | ---------------------------------------------------- | --------------------------------------------------- | ------------------------------------------------------------------ |
| `value`              | controlled                              | signal change after mount                            | selection sync event                                | Updates selected keys, value text, and hidden-input serialization. |
| open state           | machine-owned with callback observation | trigger, keyboard, outside interaction, or prop sync | `Open` / `Close`                                    | Callbacks observe the committed open state only.                   |
| item activation      | adapter event                           | click, Enter, Space, or range-selection modifiers    | `SelectItem`, `ItemCtrl`, or range-selection events | Modifier normalization is adapter-owned.                           |
| typeahead input      | adapter event                           | printable key plus timestamp                         | typeahead transition and timeout scheduling         | IME composition suppresses typeahead until composition ends.       |
| form reset           | adapter bridge                          | native form reset                                    | restore default value and hidden input              | Applies only when the select participates in a form.               |

## 8. Registration and Cleanup Contract

- The adapter owns keyed item registration, typeahead timeout cleanup, hidden-input synchronization, and popup positioning handles.
- Virtualization helpers must unregister off-screen items cleanly while preserving key identity.
- Any form-reset listener or observer must be released on unmount.

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner     | Node availability                  | Composition rule                         | Notes                                                                 |
| ------------------ | ------------- | ------------- | ---------------------------------- | ---------------------------------------- | --------------------------------------------------------------------- |
| Trigger            | yes           | adapter-owned | required after mount               | compose only through a documented helper | Needed for open-state focus return and positioning fallback anchor.   |
| Content            | yes           | adapter-owned | required after mount               | no composition by default                | Needed for popup focus management and virtualization-aware scrolling. |
| HiddenInput        | no            | adapter-owned | always structural, handle optional | no composition                           | Exists only for form participation.                                   |

## 10. State Machine Boundary Rules

- Machine-owned state: selected set, open state, highlighted key, and typeahead buffer.
- Adapter-local derived bookkeeping: keyed item registration, typeahead timeout handle, hidden-input serialization, and virtualization focus strategy.
- Forbidden local mirrors: do not keep a second selected set, highlighted key, or open flag outside the machine.
- Allowed snapshot reads: trigger handlers, item activation handlers, positioning callbacks, form-reset listeners, and timeout cleanup.

## 11. Callback Payload Contract

| Callback         | Payload source           | Payload shape | Timing                       | Cancelable? | Notes                                 |
| ---------------- | ------------------------ | ------------- | ---------------------------- | ----------- | ------------------------------------- |
| `on_open_change` | machine-derived snapshot | `bool`        | after open-state transitions | no          | Fires after the transition completes. |

## 12. Failure and Degradation Rules

| Condition                                         | Policy             | Notes                                                                                        |
| ------------------------------------------------- | ------------------ | -------------------------------------------------------------------------------------------- |
| trigger or content missing from the compound tree | fail fast          | Select cannot satisfy popup semantics without both structural nodes.                         |
| positioning helper unavailable                    | degrade gracefully | Render inline popup content with documented fallback behavior.                               |
| hidden-input bridge unavailable while form-bound  | degrade gracefully | Interactive selection still works, but native submission falls back to non-bridged behavior. |

## 13. Identity and Key Policy

- Items are data-derived by `Key`; registration order must match rendered order for stable navigation.
- Trigger, content, and hidden-input nodes are instance-derived and must remain hydration-stable.
- Typeahead timeout and positioning resources are instance-derived and cleanup-scoped to one select instance.

## 14. SSR and Client Boundary Rules

- SSR renders the root, trigger, value text, and any hydration-stable popup shell plus hidden input when present.
- Positioning, virtualization scrolling, typeahead timers, and form reset listeners are client-only.
- When the popup is SSR-rendered open, the same item order and value text must hydrate on the client.

## 15. Performance Constraints

- Do not rebuild keyed item registration on every selection change when the item set is stable.
- Typeahead timers must be single-instance and replaced rather than stacked.
- Virtualization and popup positioning work must run only while the popup is open.

## 16. Implementation Dependencies

| Dependency               | Required? | Dependency type | Why it must exist first                                                          | Notes                                                     |
| ------------------------ | --------- | --------------- | -------------------------------------------------------------------------------- | --------------------------------------------------------- |
| item registration helper | required  | shared helper   | Stable keyed item lookup is required for selection, highlighting, and typeahead. | Shared with `listbox`, `combobox`, and `menu`.            |
| hidden-input helper      | required  | shared helper   | Form submission and reset semantics are adapter-owned.                           | Shared with `segment-group` and `tags-input`.             |
| positioning helper       | required  | shared helper   | Popup placement belongs to the adapter contract.                                 | Shared with popup-based selection and overlay components. |

## 17. Recommended Implementation Sequence

1. Initialize machine props, publish root context, and render trigger plus value-text shell.
2. Render popup positioner, content, keyed items, and optional groups in stable order.
3. Wire selection, range-selection modifiers, typeahead, and open-state callbacks.
4. Add hidden-input synchronization, form reset handling, virtualization behavior, and cleanup checks.

## 18. Anti-Patterns

- Do not mirror selection state in a separate adapter-local store.
- Do not let `aria-activedescendant` reference an unmounted or virtualized-away item id.
- Do not serialize placeholder text into the hidden input when no item is selected.

## 19. Consumer Expectations and Guarantees

- Consumers may assume selection, highlighting, and popup open state are machine-owned.
- Consumers may assume form participation uses the hidden-input bridge when `name` is provided.
- Consumers must not assume range-selection or modifier-key normalization is handled implicitly by the browser.

## 20. Platform Support Matrix

| Capability / behavior                                      | Browser client | SSR            | Notes                                                    |
| ---------------------------------------------------------- | -------------- | -------------- | -------------------------------------------------------- |
| trigger, popup, keyed selection, and hidden-input bridging | full support   | full support   | The form bridge and structural popup shell are SSR-safe. |
| typeahead timers and virtualization-aware focus repair     | client-only    | SSR-safe empty | Timers and DOM scroll repair require mounted nodes.      |
| popup positioning                                          | client-only    | SSR-safe empty | Positioning activates after hydration.                   |

## 21. Debug Diagnostics and Production Policy

| Condition                                   | Debug build behavior | Production behavior | Notes                                                 |
| ------------------------------------------- | -------------------- | ------------------- | ----------------------------------------------------- |
| duplicate item keys in one select instance  | fail fast            | fail fast           | Stable selection and typeahead depend on unique keys. |
| typeahead timeout not cleaned up on unmount | debug warning        | warn and ignore     | Cleanup must remain instance-scoped.                  |

## 22. Shared Adapter Helper Notes

| Helper concept           | Required? | Responsibility                                 | Reused by                          | Notes                                      |
| ------------------------ | --------- | ---------------------------------------------- | ---------------------------------- | ------------------------------------------ |
| item registration helper | required  | Track keyed items, groups, and disabled state. | `listbox`, `combobox`, `menu`      | Keep order aligned with rendering.         |
| hidden-input helper      | required  | Serialize the committed selection for forms.   | `segment-group`, `tags-input`      | Never serialize placeholder-only state.    |
| positioning helper       | required  | Apply popup placement and size output.         | `combobox`, `menu`, `context-menu` | Shared popup behavior across the category. |

## 23. Framework-Specific Behavior

Leptos should keep item registration and timeout cleanup in `provide_context` plus local effect teardown, and use `Signal<T>` only for live post-mount controlled selection or open state.

## 24. Canonical Implementation Sketch

```rust
#[component]
pub fn Select(/* props */ children: Children) -> impl IntoView {
    let machine = use_machine::<select::Machine>(select::Props { /* ... */ });
    provide_context(Context::from_machine(machine));

    view! {
        <div {..machine.derive(|api| api.root_attrs()).get()}>
            <button {..machine.derive(|api| api.trigger_attrs()).get()} />
            <div {..machine.derive(|api| api.positioner_attrs()).get()}>
                <div {..machine.derive(|api| api.content_attrs()).get()}>{children()}</div>
            </div>
            <input {..machine.derive(|api| api.hidden_input_attrs()).get()} />
        </div>
    }
}
```

## 25. Reference Implementation Skeleton

Keep one machine, one keyed item-registration helper, one positioning handle, one hidden-input helper, and one typeahead timeout path. Selection and open-state callbacks always observe the committed snapshot after the machine transition.

## 26. Adapter Invariants

- Trigger, content, item registration, and hidden-input ownership remain explicit adapter responsibilities.
- Typeahead and range-selection modifier normalization are machine-driven with adapter-owned timeout and event translation.
- Virtualization never leaves `aria-activedescendant` pointing at an absent node.

## 27. Accessibility and SSR Notes

- Description and error linkage must include only rendered ids and remain synchronized with the trigger or content semantics required by the core spec.
- If active-descendant focus is used, the highlighted item id must always reference a mounted option; otherwise the attribute must be omitted.
- VoiceOver or virtualization fallback strategies must be documented explicitly when the default focus strategy is not reliable.

## 28. Parity Summary and Intentional Deviations

- Parity status: full parity with explicit adapter ownership of popup composition, form bridging, and keyed item registration.
- Intentional deviations: non-web positioning or form participation may use documented fallback paths rather than browser-specific behavior.

## 29. Test Scenarios

1. Opening and closing the popup updates trigger attrs and callback timing correctly.
2. Selecting, toggling, and range-selecting items updates value text and hidden-input serialization.
3. Typeahead highlights enabled items only and clears its timeout state on unmount.
4. Virtualized or off-screen items never leave stale focus or active-descendant references behind.

## 30. Test Oracle Notes

- Preferred oracle for selection and popup state: `machine state` plus `DOM attrs` on trigger, content, and items.
- Preferred oracle for form participation: hidden-input `DOM attrs` and `cleanup side effects` on reset listeners.
- Preferred oracle for typeahead and virtualization: `callback order` and `rendered structure` after highlight movement.

## 31. Implementation Checklist

- [ ] Popup composition, keyed item registration, and hidden-input bridging are explicit and cleanup-safe.
- [ ] Typeahead, range-selection modifiers, and virtualization focus behavior are captured in invariants and tests.
- [ ] Form reset and fallback platform behavior are documented where relevant.
