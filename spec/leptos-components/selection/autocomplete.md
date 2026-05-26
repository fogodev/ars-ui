---
adapter: leptos
component: autocomplete
category: selection
source: components/selection/autocomplete.md
source_foundation: foundation/08-adapter-leptos.md
---

# Autocomplete — Leptos Adapter

## 1. Purpose and Adapter Scope

This spec maps the core [`Autocomplete`](../../components/selection/autocomplete.md)
contract onto Leptos 0.8.x. The adapter renders the owned input and suggestion
output parts from the core machine, observes controlled input props, executes
debounce timers, bridges async item refreshes into `SetLoading` / `UpdateItems`,
and preserves DOM focus on the input while highlight moves through
`aria-activedescendant`.

The adapter does not own popup positioning, viewport scrolling, caret
management, or async fetch execution policy. Those remain adapter or
application responsibilities around the core machine.

## 2. Public Adapter API

```rust,no_check
#[component]
pub fn Autocomplete(
    #[prop(optional)] id: Option<String>,
    items: Signal<StaticCollection<autocomplete::Item>>,
    #[prop(optional, into)] input_value: Option<Signal<String>>,
    #[prop(optional)] default_input_value: String,
    #[prop(optional)] filter_mode: Option<autocomplete::FilterMode>,
    #[prop(optional)] debounce: Option<Duration>,
    #[prop(optional, into)] loading: Signal<bool>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional)] collection_id: Option<String>,
    #[prop(optional)] on_select: Option<Callback<Key>>,
    #[prop(optional)] on_input_value_change: Option<Callback<String>>,
    children: Children,
) -> impl IntoView
```

Compound composition may expose subcomponents for `Input`, `ClearTrigger`,
`Content`, `Item`, `ItemText`, `EmptyState`, `LoadingIndicator`, and
`LiveRegion`, but each subcomponent must derive attrs from the same
`autocomplete::Api` snapshot.

## 3. Mapping to Core Component Contract

- Props parity: full parity with `items`, controlled/uncontrolled input,
  `filter_mode`, `debounce`, `loading`, `disabled`, and `collection_id`.
- Part parity: every core part is adapter-renderable; no linked `Menu` or
  `Listbox` surface is required for the Autocomplete contract.
- Traceability note: the adapter executes timers and DOM work, while the core
  owns filtering, highlight, selection, active descendant IDs, empty/loading
  output, and ARIA/data attrs.

## 4. Part Mapping

| Core part        | Required? | Adapter rendering target     | Ownership     | Attr source                     | Notes                                                    |
| ---------------- | --------- | ---------------------------- | ------------- | ------------------------------- | -------------------------------------------------------- |
| Root             | required  | wrapper element              | adapter-owned | `api.root_attrs()`              | Owns scope attrs and disabled/loading state.             |
| Input            | required  | native text input            | adapter-owned | `api.input_attrs()`             | Remains the active focus target.                         |
| ClearTrigger     | optional  | native button                | adapter-owned | `api.clear_trigger_attrs()`     | Dispatches `Event::Clear`.                               |
| Content          | optional  | suggestion listbox container | adapter-owned | `api.content_attrs()`           | May be positioned by surrounding overlay code.           |
| Item             | repeated  | suggestion option            | adapter-owned | `api.item_attrs(key)`           | Rendered for `api.visible_items()`.                      |
| ItemText         | repeated  | suggestion label span        | adapter-owned | `api.item_text_attrs(key)`      | Text comes from the collection node or user render prop. |
| EmptyState       | optional  | empty-results status         | adapter-owned | `api.empty_state_attrs()`       | Render when `api.empty_visible()`.                       |
| LoadingIndicator | optional  | loading status               | adapter-owned | `api.loading_indicator_attrs()` | Render when loading.                                     |
| LiveRegion       | optional  | result-count status node     | adapter-owned | `api.live_region_attrs()`       | Text from `api.results_announcement()`.                  |

## 5. Attr Merge and Ownership Rules

- Core attrs on `Input`, `Content`, and `Item` win for roles, IDs,
  `aria-controls`, `aria-activedescendant`, `aria-selected`, and loading
  semantics.
- The adapter owns event handler attachment, `NodeRef` / focus retention, and
  any extra `aria-describedby` composition from field context.
- Consumers may decorate rendered parts, but they must not replace the input
  node or remove IDs needed by `aria-controls` and `aria-activedescendant`.

## 6. Composition / Context Contract

The adapter provides one shared machine context to all compound subcomponents.
`Content` and `Item` may be conditionally rendered, but their IDs must remain
stable for a given Autocomplete instance and item key.

## 7. Prop Sync and Event Mapping

| Adapter prop / event   | Mode                        | Sync trigger                | Machine event / update path                        | Notes                                                              |
| ---------------------- | --------------------------- | --------------------------- | -------------------------------------------------- | ------------------------------------------------------------------ |
| `items`                | controlled by adapter input | signal change               | `UpdateItems` / props sync                         | Re-runs filtering against the current input.                       |
| `input_value`          | controlled                  | signal change after mount   | `SetInputValue`                                    | Updates rendered input and filter state.                           |
| native `input` / paste | uncontrolled or controlled  | DOM event                   | `InputChange`                                      | Callback observation happens after the machine accepts the change. |
| `debounce`             | optional                    | prop change or input change | `RestartDebounce` / `AutocompleteDebounce`         | Adapter owns the timer handle and cancellation.                    |
| `loading`              | controlled                  | signal change               | `SetLoading` / props sync                          | Exposes `aria-busy` and loading indicator attrs.                   |
| item click / Enter     | adapter event               | committed activation        | `SelectItem` / `SelectHighlighted`                 | Fires `on_select` only after committed selection.                  |
| hover / pointer leave  | adapter event               | pointer movement            | `HighlightItem(Some(key))` / `HighlightItem(None)` | Does not change selection.                                         |
| input keydown          | adapter event               | keydown                     | `api.on_input_keydown(...)`                        | Focus remains on the input.                                        |

## 8. Registration and Cleanup Contract

- Debounce timer handles must be canceled on unmount and whenever the machine
  emits a cancel effect for `AutocompleteDebounce`.
- Async fetch tasks are application-owned; adapters only reflect their state
  through `loading` and refreshed `items`.
- No hidden-input bridge exists; form participation belongs to a higher-level
  field wrapper.

## 9. Ref and Node Contract

| Target node | Ref required? | Ref owner      | Node availability    | Notes                                                    |
| ----------- | ------------- | -------------- | -------------------- | -------------------------------------------------------- |
| Input       | yes           | adapter-owned  | required after mount | Needed for focus retention and caret preservation.       |
| Root        | no            | not applicable | structural           | Wrapper node.                                            |
| Content     | optional      | adapter-owned  | conditional          | Positioning and scrolling are outside the core contract. |
| LiveRegion  | no            | adapter-owned  | client-only          | Only needed when announcements are rendered.             |

## 10. State Machine Boundary Rules

- Machine-owned state: input value, items, visible keys, highlighted key,
  selected key, loading flag, disabled flag, and debounce-pending flag.
- Adapter-owned work: DOM focus, caret, popup positioning, scroll-into-view,
  timer handles, and async fetch execution.
- Forbidden local mirrors: do not keep a second mutable selected key,
  highlighted key, or filtered item set outside the machine context.

## 11. Callback Payload Contract

| Callback                | Payload source   | Payload shape | Timing                      | Cancelable? | Notes                                         |
| ----------------------- | ---------------- | ------------- | --------------------------- | ----------- | --------------------------------------------- |
| `on_input_value_change` | machine snapshot | `String`      | after `InputChange` commits | no          | Fires after any adapter IME gating.           |
| `on_select`             | machine snapshot | `Key`         | after committed selection   | no          | Never fires for transient highlight movement. |

## 12. Failure and Degradation Rules

| Condition                                       | Policy                 | Notes                                                           |
| ----------------------------------------------- | ---------------------- | --------------------------------------------------------------- |
| debounce runtime unavailable                    | degrade gracefully     | Input still updates; no delayed fetch trigger is emitted.       |
| result-announcement node omitted                | degrade gracefully     | Filtering and selection still work without spoken count output. |
| controlled/uncontrolled mode switch after mount | follow core props sync | The machine owns controlled value reconciliation.               |

## 13. Identity and Key Policy

- Input, content, live-region, and generated item IDs are derived from the
  stable component `id`.
- Item identity remains data-derived by `Key`; the adapter never rewrites item
  identity.
- `collection_id` overrides the generated content ID when provided.

## 14. SSR and Client Boundary Rules

- SSR renders root, input, and any static suggestion output with initial attrs.
- Debounce timers, live announcements, focus retention, and async fetches are
  client-only.
- Hydration must preserve generated IDs so `aria-controls` and
  `aria-activedescendant` remain valid.

## 15. Performance Constraints

- Do not rebuild static suggestion markup when only highlight changes unless the
  rendered attrs for that item changed.
- Debounce work should cancel previous timers instead of accumulating tasks.
- Result announcements should avoid duplicate emissions for the same visible
  count.

## 16. Implementation Dependencies

| Dependency              | Required?   | Dependency type | Why it must exist first                                     | Notes                                   |
| ----------------------- | ----------- | --------------- | ----------------------------------------------------------- | --------------------------------------- |
| `autocomplete::Machine` | required    | core machine    | Owns filtering, highlight, selection, and attrs.            | This task provides it.                  |
| timer cleanup helper    | required    | adapter helper  | Executes and cancels debounce effects.                      | May be shared with search input.        |
| IME helper              | recommended | shared helper   | Suppresses premature committed input during composition.    | Shared with `Combobox` and `TagsInput`. |
| positioning helper      | optional    | adapter helper  | Positions `Content` when the product renders it as a popup. | Out of core scope.                      |

## 17. Recommended Implementation Sequence

1. Initialize the machine from `items`, input props, `filter_mode`, debounce,
   loading, disabled, and optional `collection_id`.
2. Render root, input, clear trigger, content, visible items, empty/loading
   output, and live region from `Api` attrs.
3. Wire input, clear, item activation, hover, and keydown events.
4. Add debounce timer execution and cleanup.
5. Add optional async-item refresh integration through `loading` and `items`.

## 18. Anti-Patterns

- Do not delegate the suggestion list to a separate `Menu` or `Listbox` state
  machine for this component.
- Do not commit selection from highlight movement alone.
- Do not implement async fetch logic in the core or require a specific fetch
  abstraction in the adapter.

## 19. Consumer Expectations and Guarantees

- Consumers may assume the input remains the primary focus target.
- Consumers may assume `on_select` fires only for committed item activation.
- Consumers must not assume Autocomplete owns popup positioning or scrolling.

## 20. Platform Support Matrix

| Capability / behavior         | Browser client | SSR              | Notes                                         |
| ----------------------------- | -------------- | ---------------- | --------------------------------------------- |
| input ownership and filtering | full support   | initial attrs    | Interactive filtering begins after hydration. |
| active-descendant highlight   | full support   | structural attrs | Live DOM focus remains on the input.          |
| debounce and async refresh    | client-only    | SSR-safe empty   | Timers and fetches are client work.           |
| live result announcements     | client-only    | SSR-safe empty   | Announcements require a live region.          |

## 21. Debug Diagnostics and Production Policy

| Condition                                      | Debug build behavior | Production behavior | Notes                                         |
| ---------------------------------------------- | -------------------- | ------------------- | --------------------------------------------- |
| item key missing during activation             | ignore and warn      | ignore              | The core only selects visible focusable keys. |
| duplicate rendered IDs from custom composition | warn                 | best effort         | Consumers must preserve provided IDs.         |

## 22. Shared Adapter Helper Notes

| Helper concept     | Required?   | Responsibility                                         | Reused by                              | Notes                                                |
| ------------------ | ----------- | ------------------------------------------------------ | -------------------------------------- | ---------------------------------------------------- |
| debounce helper    | required    | Execute and cancel named debounce effects.             | `SearchInput`, future text controllers | Keeps timer cleanup consistent.                      |
| IME helper         | recommended | Suppress premature committed input during composition. | `combobox`, `tags-input`               | One shared policy keeps text-entry behavior aligned. |
| positioning helper | optional    | Position `Content` when rendered as a popup.           | overlays                               | Not part of core Autocomplete.                       |

## 23. Framework-Specific Behavior

Leptos should use `Signal<T>` only for live post-mount props, use `prop:value`
for reactive input value binding, keep the input node in a local `NodeRef` when
DOM focus repair is needed, and execute debounce timers from effects that clean
up on dependency change or unmount.

## 24. Canonical Implementation Sketch

```rust,no_check
#[component]
pub fn Autocomplete(/* props */ children: Children) -> impl IntoView {
    let machine = use_machine::<autocomplete::Machine>(autocomplete::Props { /* ... */ });

    view! {
        <div {..machine.derive(|api| api.root_attrs()).get()}>
            <input
                {..machine.derive(|api| api.input_attrs()).get()}
                on:input=move |ev| { /* dispatch InputChange */ }
                on:keydown=move |ev| { /* api.on_input_keydown */ }
            />
            {children()}
        </div>
    }
}
```

## 25. Test Scenarios

1. Typing updates the input value, filters visible items, and schedules debounce
   when configured.
2. Loading state renders `aria-busy` plus loading indicator attrs.
3. Empty filtered results render `EmptyState`.
4. Arrow/Home/End keydown keeps DOM focus on the input and updates
   `aria-activedescendant`.
5. Committed item activation resolves a `Key` and fires `on_select` once.

## 26. Implementation Checklist

- [ ] All core parts can render from `autocomplete::Api`.
- [ ] Debounce effect cleanup is tied to machine cancel effects and unmount.
- [ ] The adapter does not duplicate filtering, highlight, or selection state.
