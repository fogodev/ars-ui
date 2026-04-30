---
adapter: dioxus
component: autocomplete
category: selection
source: components/selection/autocomplete.md
source_foundation: foundation/09-adapter-dioxus.md
---

# Autocomplete — Dioxus Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Autocomplete`](../../components/selection/autocomplete.md) contract onto Dioxus 0.7.x. The adapter must preserve text entry plus collection filtering and committed option selection while making input ownership, collection filtering, blur reconciliation, and committed selection callbacks explicit at the framework boundary.

## 2. Public Adapter API

```rust,no_check
#[derive(Props, Clone, PartialEq)]
pub struct AutocompleteProps {
    #[props(optional)]
    pub id: Option<String>,
    #[props(optional)]
    pub input_value: Option<Signal<String>>,
    #[props(optional, default)]
    pub default_input_value: String,
    pub collection_id: String,
    #[props(optional)]
    pub filter_mode: Option<autocomplete::FilterMode>,
    #[props(optional, default = false)]
    pub clear_on_blur: bool,
    #[props(optional, default = false)]
    pub disabled: bool,
    #[props(optional)]
    pub on_select: Option<EventHandler<Key>>,
    #[props(optional)]
    pub on_input_value_change: Option<EventHandler<String>>,
    pub children: Element,
}

#[component]
pub fn Autocomplete(props: AutocompleteProps) -> Element

/// Input component: the text input for typing filter text.
#[derive(Props, Clone, PartialEq)]
pub struct InputProps {
    #[props(optional)]
    pub as_child: Option<EventHandler<InputRenderProps, Element>>,
    pub children: Element,
}

#[component]
pub fn Input(props: InputProps) -> Element

/// ClearTrigger component: button that clears pending input text.
#[derive(Props, Clone, PartialEq)]
pub struct ClearTriggerProps {
    pub children: Element,
}

#[component]
pub fn ClearTrigger(props: ClearTriggerProps) -> Element

/// LiveRegion component: announces result count to assistive technology.
#[component]
pub fn LiveRegion() -> Element
```

Compound composition expects a separate collection surface keyed by `collection_id`; the adapter surface is the text input wrapper plus optional clear trigger and live-region helper.

## 3. Mapping to Core Component Contract

- Props parity: full parity with the core `Autocomplete` input props plus explicit adapter callback timing for input-value and committed-selection observation.
- Part parity: the input-side parts are adapter-owned while the filtered collection surface is intentionally shared with `Menu` or `Listbox` rather than duplicated here.
- Traceability note: this spec promotes filter dispatch, blur cleanup, IME suppression, collection linkage, and result-announcement behavior from the agnostic spec into the adapter contract.

## 4. Part Mapping

| Core part / structure | Required? | Adapter rendering target                      | Ownership     | Attr source               | Notes                                                       |
| --------------------- | --------- | --------------------------------------------- | ------------- | ------------------------- | ----------------------------------------------------------- |
| Root                  | required  | wrapper element                               | adapter-owned | api.root_attrs()          | Owns scope attrs and compound context.                      |
| Input                 | required  | native text input                             | adapter-owned | api.input_attrs()         | Remains the active focus target.                            |
| ClearTrigger          | optional  | native button                                 | adapter-owned | api.clear_trigger_attrs() | Clears pending text only.                                   |
| CollectionHost        | required  | consumer-provided `Menu` or `Listbox` subtree | shared        | linked by `collection_id` | Autocomplete does not own option DOM.                       |
| LiveRegion            | optional  | adapter-owned status node                     | adapter-owned | adapter-local attrs       | Announces result count when the product surface enables it. |

## 5. Attr Merge and Ownership Rules

- Core attrs on `Input` win for value, autocomplete semantics, `aria-controls`, and `aria-activedescendant` when a highlighted item exists.
- The adapter owns `aria-describedby` composition, `role="combobox"` repair when the linked collection requires it, and any `aria-live` node for count announcements.
- Consumers may decorate the wrapper or linked collection host, but they must not replace the input node or drop the collection linkage implied by `collection_id`.

## 6. Composition / Context Contract

The adapter publishes local input context and consumes optional environment, field, and collection contracts. Missing linked collection state is a structural error because keyboard navigation and result announcements depend on a resolvable collection host.

## 7. Prop Sync and Event Mapping

| Adapter prop / event           | Mode                       | Sync trigger                           | Machine event / update path                        | Notes                                                                                       |
| ------------------------------ | -------------------------- | -------------------------------------- | -------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `input_value`                  | controlled                 | signal change after mount              | `SetInputValue` or equivalent machine sync         | Updates the rendered input and re-runs filtering.                                           |
| native `input` / paste         | uncontrolled or controlled | DOM event                              | `InputChange`                                      | Adapter updates filtering and callback observation after the machine accepts the change.    |
| `clear_on_blur`                | non-reactive adapter prop  | blur transition                        | adapter-owned blur reconciliation                  | When enabled, unmatched free-form text is cleared on blur.                                  |
| selection from collection host | adapter bridge             | Enter, click, or collection activation | `SelectItem` plus callback observation             | The adapter resolves the selected key from the linked collection before firing `on_select`. |
| IME composition                | adapter event              | compositionstart / compositionend      | composition gate around filtering and commit paths | Intermediate composing text must not trigger committed selection.                           |

## 8. Registration and Cleanup Contract

- The adapter owns any live-region timer or debounce handle used for result announcements.
- Collection linkage subscriptions and blur-reconciliation observers must be dropped on unmount.
- No hidden-input bridge exists; form participation belongs to the surrounding form contract only when a higher-level component wraps Autocomplete.

## 9. Ref and Node Contract

| Target part / node | Ref required? | Ref owner      | Node availability                  | Composition rule                                       | Notes                                                           |
| ------------------ | ------------- | -------------- | ---------------------------------- | ------------------------------------------------------ | --------------------------------------------------------------- |
| Input              | yes           | adapter-owned  | required after mount               | compose only if the consumer explicitly forwards a ref | Needed for focus retention and `aria-activedescendant` updates. |
| Root               | no            | not applicable | always structural, handle optional | not applicable                                         | Pure wrapper node.                                              |
| LiveRegion         | no            | adapter-owned  | client-only                        | no composition                                         | Only needed when announcements are rendered.                    |

## 10. State Machine Boundary Rules

- Machine-owned state: input value, focused state, and highlighted selection candidate.
- Adapter-local derived bookkeeping: linked collection availability, live-region throttling, and blur-reconciliation policy.
- Forbidden local mirrors: do not keep a second mutable copy of the input text or selected key outside the machine context.
- Allowed snapshot reads: input handlers, blur cleanup, linked collection activation handlers, and result-announcement effects.

## 11. Callback Payload Contract

| Callback                | Payload source           | Payload shape | Timing                      | Cancelable? | Notes                                             |
| ----------------------- | ------------------------ | ------------- | --------------------------- | ----------- | ------------------------------------------------- |
| `on_input_value_change` | machine-derived snapshot | `String`      | after `InputChange` commits | no          | Fires after IME gating and machine normalization. |
| `on_select`             | machine-derived snapshot | `Key`         | after committed selection   | no          | Never fires for transient highlight movement.     |

## 12. Failure and Degradation Rules

| Condition                                       | Policy             | Notes                                                           |
| ----------------------------------------------- | ------------------ | --------------------------------------------------------------- |
| missing linked collection host                  | fail fast          | The component contract requires a resolvable `collection_id`.   |
| result-announcement helper unavailable          | degrade gracefully | Filtering and selection still work without spoken count output. |
| controlled/uncontrolled mode switch after mount | warn and ignore    | The first mode wins for the life of the instance.               |

## 13. Identity and Key Policy

- Input instance identity is instance-derived and must remain stable across hydration.
- Linked collection items remain data-derived by `Key`; the adapter never rewrites item identity.
- Announcement resources are instance-derived and cleanup-scoped to the adapter instance.

## 14. SSR and Client Boundary Rules

- SSR renders the wrapper and input with initial value and collection linkage attrs only.
- Filtering effects, live announcements, and IME handling are client-only.
- Hydration must preserve the same input node and `collection_id` linkage used on the server.

## 15. Performance Constraints

- Do not rebuild collection linkage on every keystroke when only the input value changes.
- Announcement work should be incremental and avoid duplicate emissions for the same visible count.
- Keep IME bookkeeping instance-local and avoid using it as a general debounce state store.

## 16. Implementation Dependencies

| Dependency                 | Required?   | Dependency type      | Why it must exist first                                                                    | Notes                                                                                  |
| -------------------------- | ----------- | -------------------- | ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------- |
| linked collection contract | required    | composition contract | Autocomplete depends on `Menu` or `Listbox` exposing a stable keyed collection host.       | Reuse the selection collection contract rather than inventing a second option surface. |
| IME helper                 | required    | shared helper        | Text composition suppression must stay aligned with other text-entry selection components. | Shared with `Combobox` and `TagsInput`.                                                |
| announcement helper        | recommended | shared helper        | Result-count announcements should share one cleanup-safe helper path.                      | Optional when the product surface omits announcements.                                 |

## 17. Recommended Implementation Sequence

1. Initialize the machine and linked collection lookup from the documented props.
2. Render the wrapper, input, optional clear trigger, and consumer children in stable order.
3. Wire input, blur, clear, and IME events before adding callback observation.
4. Attach optional result-announcement behavior and verify cleanup on unmount.

## 18. Anti-Patterns

- Do not duplicate the option list inside the Autocomplete adapter; it consumes an external selection surface.
- Do not commit selection from highlight movement alone.
- Do not run blur cleanup against intermediate IME composition text.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the input remains the primary focus target even when a linked collection is open.
- Consumers may assume `on_select` fires only for committed item activation.
- Consumers must not assume Autocomplete owns popup positioning or option rendering by itself.

## 20. Platform Support Matrix

| Capability / behavior                              | Web          | Desktop       | Mobile        | SSR            | Notes                                                                    |
| -------------------------------------------------- | ------------ | ------------- | ------------- | -------------- | ------------------------------------------------------------------------ |
| input ownership, filtering, and collection linkage | full support | full support  | full support  | full support   | Collection linkage is adapter-owned across supported targets.            |
| IME composition handling                           | full support | fallback path | fallback path | SSR-safe empty | Desktop and mobile may need platform-specific composition normalization. |
| live result announcements                          | client-only  | fallback path | fallback path | SSR-safe empty | Use a platform-appropriate announcement helper when available.           |

## 21. Debug Diagnostics and Production Policy

| Condition                                       | Debug build behavior | Production behavior | Notes                                                                                   |
| ----------------------------------------------- | -------------------- | ------------------- | --------------------------------------------------------------------------------------- |
| collection host missing for `collection_id`     | fail fast            | fail fast           | Autocomplete cannot satisfy keyboard or selection behavior without a linked collection. |
| controlled/uncontrolled mode switch after mount | debug warning        | warn and ignore     | Preserve the first mode.                                                                |

## 22. Shared Adapter Helper Notes

| Helper concept           | Required?   | Responsibility                                               | Reused by                | Notes                                                |
| ------------------------ | ----------- | ------------------------------------------------------------ | ------------------------ | ---------------------------------------------------- |
| IME helper               | required    | Suppress filtering and commit paths during composition.      | `combobox`, `tags-input` | One shared policy keeps text-entry behavior aligned. |
| collection bridge helper | required    | Resolve the linked collection host and selected key lookups. | `combobox`               | Do not duplicate keyed collection logic.             |
| announcement helper      | recommended | Throttle and announce result counts.                         | `combobox`, `listbox`    | Only when announcements are enabled.                 |

## 23. Framework-Specific Behavior

Dioxus should keep controlled input sync in props or signal observation, avoid holding signal guards across async work, and normalize composition events per target before dispatching machine events.

## 24. Canonical Implementation Sketch

```rust
#[derive(Props, Clone, PartialEq)]
pub struct AutocompleteProps { /* ... */ }

#[component]
pub fn Autocomplete(props: AutocompleteProps) -> Element {
    let machine = use_machine::<autocomplete::Machine>(autocomplete::Props { /* ... */ });

    rsx! {
        div {
            ..machine.derive(|api| api.root_attrs())(),
            input {
                ..machine.derive(|api| api.input_attrs())(),
                oninput: move |ev| { /* dispatch InputChange */ },
            }
            {props.children}
        }
    }
}
```

## 25. Reference Implementation Skeleton

Keep one machine, one collection-bridge helper, and one optional announcement helper. Input events always flow through the machine first; callback observation and blur cleanup read the post-transition snapshot only.

## 26. Adapter Invariants

- The input node remains adapter-owned and is never replaced by consumer composition.
- `collection_id` always points at a keyed selection surface when the component is mounted.
- Committed selection callbacks never fire for transient highlight movement or composing text.

## 27. Accessibility and SSR Notes

- When a highlighted item exists, `aria-activedescendant` must reference a valid live DOM id; otherwise the attribute must be omitted entirely.
- If announcements are rendered, they should describe the filtered result count without duplicating the active-option announcement already provided by the collection surface.
- SSR should keep the input and wrapper structure stable even when announcements are client-only.

## 28. Parity Summary and Intentional Deviations

- Parity status: full input-side parity with explicit adapter ownership of collection linkage and announcement policy.
- Intentional deviation: popup rendering stays delegated to `Menu` or `Listbox` rather than being duplicated inside Autocomplete.

## 29. Test Scenarios

1. Typing updates the input value and re-runs filtering without firing committed selection callbacks.
2. IME composition suppresses premature filtering commits and resumes normal behavior on composition end.
3. Blur cleanup clears unmatched text only when `clear_on_blur=true`.
4. Committed item activation resolves a `Key` from the linked collection and fires `on_select` once.

## 30. Test Oracle Notes

- Preferred oracle for input ownership: `DOM attrs` on the input plus machine snapshot checks for committed value transitions.
- Preferred oracle for collection linkage: `callback order` and `machine state` after Enter or click activates an item in the linked collection.
- Preferred oracle for IME behavior: `callback order` showing that composing input does not emit committed selection.

## 31. Implementation Checklist

- [ ] Input ownership, IME gating, and linked collection resolution are documented explicitly.
- [ ] The component never duplicates popup or option rendering responsibilities already owned by the linked collection surface.
- [ ] Announcements, blur cleanup, and callback timing are cleanup-safe and machine-driven.
