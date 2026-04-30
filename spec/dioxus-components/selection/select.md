---
adapter: dioxus
component: select
category: selection
source: components/selection/select.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Select — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Select`](../../components/selection/select.md) contract onto Dioxus 0.7.x. The adapter must preserve triggered popup selection with typeahead and form participation while making compound trigger/content wiring, keyed item registration, typeahead, hidden-input form bridging, and popup positioning explicit at the framework boundary.

## 2. Public Adapter API

```rust,no_check
#[derive(Props, Clone, PartialEq)]
pub struct SelectProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub value: Option<Signal<selection::Set>>,
    #[props(optional, default)]
    pub default_value: selection::Set,
    #[props(optional, default = false)]
    pub multiple: bool,
    #[props(optional)]
    pub selection_mode: Option<selection::Mode>,
    #[props(optional)]
    pub disabled_keys: Option<BTreeSet<Key>>,
    #[props(optional)]
    pub name: Option<String>,
    #[props(optional)]
    pub form: Option<String>,
    #[props(optional)]
    pub positioning: Option<PositioningOptions>,
    #[props(optional)]
    pub on_open_change: Option<EventHandler<bool>>,
    pub children: Element,
}

#[component]
pub fn Select(props: SelectProps) -> Element

/// Trigger component: button that opens the select popup.
#[derive(Props, Clone, PartialEq)]
pub struct TriggerProps {
    pub children: Element,
}

#[component]
pub fn Trigger(props: TriggerProps) -> Element

/// ValueText component: displays placeholder or selected item text.
#[derive(Props, Clone, PartialEq)]
pub struct ValueTextProps {
    #[props(optional)]
    pub placeholder: Option<String>,
    pub children: Element,
}

#[component]
pub fn ValueText(props: ValueTextProps) -> Element

/// Positioner component: positioned wrapper for popup content.
#[derive(Props, Clone, PartialEq)]
pub struct PositionerProps {
    pub children: Element,
}

#[component]
pub fn Positioner(props: PositionerProps) -> Element

/// Content component: the listbox host for select options.
#[derive(Props, Clone, PartialEq)]
pub struct ContentProps {
    pub children: Element,
}

#[component]
pub fn Content(props: ContentProps) -> Element

/// SelectItem component: a single keyed option in the select popup.
#[derive(Props, Clone, PartialEq)]
pub struct SelectItemProps {
    pub key: Key,
    #[props(optional, default = false)]
    pub disabled: bool,
    pub children: Element,
}

#[component]
pub fn SelectItem(props: SelectItemProps) -> Element

/// ItemGroup component: a labeled group of select items.
#[derive(Props, Clone, PartialEq)]
pub struct ItemGroupProps {
    pub children: Element,
}

#[component]
pub fn ItemGroup(props: ItemGroupProps) -> Element

/// HiddenInput component: bridges native form submission.
#[component]
pub fn HiddenInput() -> Element

/// Description component: descriptive text wired to aria-describedby.
#[derive(Props, Clone, PartialEq)]
pub struct DescriptionProps {
    pub children: Element,
}

#[component]
pub fn Description(props: DescriptionProps) -> Element
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

| Capability / behavior                                      | Web          | Desktop       | Mobile        | SSR            | Notes                                                     |
| ---------------------------------------------------------- | ------------ | ------------- | ------------- | -------------- | --------------------------------------------------------- |
| trigger, popup, keyed selection, and hidden-input bridging | full support | fallback path | fallback path | full support   | Non-web targets may need documented host-form fallbacks.  |
| typeahead timers and virtualization-aware focus repair     | full support | fallback path | fallback path | SSR-safe empty | Use target-appropriate focus repair when DOM APIs differ. |
| popup positioning                                          | full support | fallback path | fallback path | SSR-safe empty | Non-web targets may use simplified placement.             |

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

Dioxus should isolate controlled selection observation from popup positioning work, avoid long-lived signal guards, and make non-web form or positioning fallbacks explicit rather than implicit.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct SelectProps { /* ... */ }

#[component]
pub fn Select(props: SelectProps) -> Element {
    let machine = use_machine::<select::Machine>(select::Props { /* ... */ });

    rsx! {
        div {
            ..machine.derive(|api| api.root_attrs())(),
            button { ..machine.derive(|api| api.trigger_attrs())() }
            div {
                ..machine.derive(|api| api.positioner_attrs())(),
                div { ..machine.derive(|api| api.content_attrs())(), {props.children} }
            }
            input { ..machine.derive(|api| api.hidden_input_attrs())() }
        }
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
