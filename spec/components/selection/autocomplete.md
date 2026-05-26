---
component: Autocomplete
category: selection
tier: stateful
foundation_deps: [architecture, accessibility, interactions, collections]
shared_deps: [selection-patterns]
related: []
references:
    react-aria: Autocomplete
---

# Autocomplete

`Autocomplete` is a framework-agnostic selection controller that coordinates an
input with owned suggestion output. It is simpler than `Combobox`: it does not
own popup positioning, live DOM focus, caret management, item scrolling, or
async fetch execution. It does own the semantic state needed by adapters:
input value, suggestion items, filtering, highlight, selection, active
descendant IDs, debounce intent, loading state, empty state, and ARIA/data
attributes.

## 1. State Machine

```diagram
Idle
  Focus -> Focused

Focused
  InputChange -> Interacting
  SetLoading(true) -> Loading
  Blur -> Idle

Interacting
  InputChange -> Interacting
  Highlight* -> Interacting
  SelectItem / SelectHighlighted -> Focused
  SetLoading(true) -> Loading
  Clear -> Focused
  Blur -> Idle

Loading
  SetLoading(false) -> Focused | Interacting | Idle
  InputChange -> Loading
  Highlight* -> Loading
  SelectItem / SelectHighlighted -> Loading
  Blur -> Loading
```

### 1.1 States

```rust
/// The states of the Autocomplete state machine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The input is not focused and no interaction is active.
    #[default]
    Idle,
    /// The input is focused without an active filter change.
    Focused,
    /// The user is typing or navigating filtered suggestions.
    Interacting,
    /// Suggestions are being fetched or refreshed.
    Loading,
}
```

### 1.2 Events

```rust
/// Events accepted by the Autocomplete state machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The input received focus.
    Focus {
        /// Whether focus was initiated by keyboard interaction.
        is_keyboard: bool,
    },
    /// Focus left the autocomplete region.
    Blur,
    /// The input value changed.
    InputChange(String),
    /// A debounce timer expired.
    DebounceExpired,
    /// Cancel the active debounce timer.
    CancelDebounce,
    /// Restart the debounce timer after a debounce prop change.
    RestartDebounce,
    /// Set whether suggestions are loading.
    SetLoading(bool),
    /// Replace the suggestion collection.
    UpdateItems(StaticCollection<Item>),
    /// Highlight a specific item.
    HighlightItem(Option<Key>),
    /// Highlight the first visible item.
    HighlightFirst,
    /// Highlight the last visible item.
    HighlightLast,
    /// Highlight the next visible item.
    HighlightNext,
    /// Highlight the previous visible item.
    HighlightPrev,
    /// Select a suggestion item.
    SelectItem(Key),
    /// Select the currently highlighted suggestion item.
    SelectHighlighted,
    /// Clear the input value and selection.
    Clear,
    /// Synchronize the externally controlled input prop.
    SetInputValue(Option<String>),
    /// Synchronize output-affecting props.
    SyncProps,
}
```

### 1.3 Context

```rust
/// Context held by the Autocomplete state machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Full suggestion item collection.
    pub items: StaticCollection<Item>,
    /// Controlled or uncontrolled input value.
    pub input_value: Bindable<String>,
    /// Keys passing the current built-in filter. `None` means no filter is active.
    pub visible_keys: Option<BTreeSet<Key>>,
    /// Currently highlighted visible suggestion key.
    pub highlighted_key: Option<Key>,
    /// Last selected suggestion key.
    pub selected_key: Option<Key>,
    /// Whether the input has focus.
    pub focused: bool,
    /// Whether focus-visible styling should be emitted.
    pub focus_visible: bool,
    /// Whether suggestions are currently loading.
    pub loading: bool,
    /// Whether an adapter debounce timer is currently pending.
    pub debounce_pending: bool,
    /// Whether interaction is disabled.
    pub disabled: bool,
    /// Filter mode for built-in suggestion filtering.
    pub filter_mode: FilterMode,
    /// ID of the owned listbox content element.
    pub collection_id: String,
    /// Stable component ID derivation helper.
    pub ids: ComponentIds,
    /// Resolved locale for i18n.
    pub locale: Locale,
    /// Resolved localized messages.
    pub messages: Messages,
}
```

### 1.4 Props

```rust
/// Props for the Autocomplete state machine.
#[derive(Clone, Debug, Default, PartialEq, HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,
    /// Suggestion items.
    pub items: StaticCollection<Item>,
    /// Controlled input value.
    pub input_value: Option<String>,
    /// Initial uncontrolled input value.
    pub default_input_value: String,
    /// Built-in filtering mode.
    pub filter_mode: FilterMode,
    /// Optional debounce interval for search-as-you-type.
    pub debounce: Option<Duration>,
    /// Whether suggestions are loading.
    pub loading: bool,
    /// Whether interaction is disabled.
    pub disabled: bool,
    /// ID for the owned listbox content element.
    pub collection_id: String,
}
```

`collection_id` defaults to the derived content ID when empty. `input_value`
uses controlled semantics: when controlled, selection records `selected_key`
but does not overwrite `input_value` until the adapter/consumer syncs the prop.
`Props` also exposes builder-style methods for every field: `new`, `id`,
`items`, `input_value`, `uncontrolled`, `default_input_value`, `filter_mode`,
`debounce`, `no_debounce`, `loading`, `disabled`, and `collection_id`.

### 1.5 Item and FilterMode

```rust
/// User-facing payload for Autocomplete suggestion items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// The label shown for the suggestion.
    pub label: String,
}
```

`FilterMode` is shared with `Combobox`: `Contains`, `StartsWith`, `None`,
`Inline`, `InlineCompletion`, and `Custom`. Built-in filtering is
case-insensitive and uses each collection node's text value. `Custom`, `None`,
and `Inline` leave `visible_keys` as `None`; adapters may replace `items` for
custom or server-backed filtering.

### 1.6 Effects

```rust
/// Typed identifier for named effect intents emitted by Autocomplete.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter starts or restarts the debounce timer for input changes.
    AutocompleteDebounce,
}
```

When `Props::debounce` is `Some`, `InputChange` cancels the previous
`AutocompleteDebounce`, schedules a new one, and sets `debounce_pending`.
`DebounceExpired`, `CancelDebounce`, `Clear`, and successful selection clear the
pending flag. Adapters execute timers and async fetching; the core only emits
the named effect intent and exposes `SetLoading`.

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "autocomplete"]
pub enum Part {
    Root,
    Input,
    ClearTrigger,
    Content,
    Item { key: Key },
    ItemText { key: Key },
    EmptyState,
    LoadingIndicator,
    LiveRegion,
}
```

`Machine` is the public framework-agnostic machine marker implementing
`ars_core::Machine` for the `State`, `Event`, `Context`, `Props`, `Messages`,
`Effect`, and `Api` surfaces above.

`Api` exposes `root_attrs`, `input_attrs`, `clear_trigger_attrs`,
`content_attrs`, `item_attrs`, `item_text_attrs`, `empty_state_attrs`,
`loading_indicator_attrs`, and `live_region_attrs`. It also exposes
`visible_items`, `visible_count`, `empty_visible`, `input_value`,
`selected_key`, `highlighted_key`, `collection_id`, `loading_text`,
`empty_text`, `results_announcement`, and event helpers for input changes,
clear, item selection, hover, leave, and input keydown.

## 2. Anatomy

| Part               | Selector                                                             | Element    |
| ------------------ | -------------------------------------------------------------------- | ---------- |
| `Root`             | `[data-ars-scope="autocomplete"][data-ars-part="root"]`              | `<div>`    |
| `Input`            | `[data-ars-scope="autocomplete"][data-ars-part="input"]`             | `<input>`  |
| `ClearTrigger`     | `[data-ars-scope="autocomplete"][data-ars-part="clear-trigger"]`     | `<button>` |
| `Content`          | `[data-ars-scope="autocomplete"][data-ars-part="content"]`           | `<div>`    |
| `Item`             | `[data-ars-scope="autocomplete"][data-ars-part="item"]`              | `<div>`    |
| `ItemText`         | `[data-ars-scope="autocomplete"][data-ars-part="item-text"]`         | `<span>`   |
| `EmptyState`       | `[data-ars-scope="autocomplete"][data-ars-part="empty-state"]`       | `<div>`    |
| `LoadingIndicator` | `[data-ars-scope="autocomplete"][data-ars-part="loading-indicator"]` | `<div>`    |
| `LiveRegion`       | `[data-ars-scope="autocomplete"][data-ars-part="live-region"]`       | `<div>`    |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property                | Element                                | Value                                                               |
| ----------------------- | -------------------------------------- | ------------------------------------------------------------------- |
| `role`                  | Input                                  | `combobox`                                                          |
| `aria-expanded`         | Input                                  | `true` when focused, interacting, or loading                        |
| `aria-haspopup`         | Input                                  | `listbox`                                                           |
| `aria-controls`         | Input                                  | Content element ID                                                  |
| `aria-autocomplete`     | Input                                  | `list`                                                              |
| `aria-activedescendant` | Input                                  | Highlighted item ID; omitted when no valid visible highlight exists |
| `role`                  | Content                                | `listbox`                                                           |
| `aria-busy`             | Root/Content                           | `true` while loading                                                |
| `role`                  | Item                                   | `option`                                                            |
| `aria-selected`         | Item                                   | `true` for the selected key                                         |
| `role`                  | EmptyState/LoadingIndicator            | `status`                                                            |
| `aria-live`             | EmptyState/LoadingIndicator/LiveRegion | `polite`                                                            |

### 3.2 Keyboard Interaction

| Key       | Action                                                              |
| --------- | ------------------------------------------------------------------- |
| Typing    | Updates input value, filters suggestions, and may schedule debounce |
| ArrowDown | Highlights the next visible suggestion                              |
| ArrowUp   | Highlights the previous visible suggestion                          |
| Home      | Highlights the first visible suggestion                             |
| End       | Highlights the last visible suggestion                              |
| Enter     | Selects the highlighted suggestion                                  |
| Escape    | Clears the input when non-empty                                     |
| Tab       | Leaves native focus handling to the adapter/browser                 |

## 4. Internationalization

### 4.1 Messages

```rust
/// Locale-specific labels for the Autocomplete component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the input.
    pub input_label: MessageFn<LocaleMessage>,
    /// Accessible label for the clear trigger.
    pub clear_label: MessageFn<LocaleMessage>,
    /// Accessible label for the suggestion listbox.
    pub listbox_label: MessageFn<LocaleMessage>,
    /// Loading indicator text.
    pub loading_label: MessageFn<LocaleMessage>,
    /// Empty-state text.
    pub empty_label: MessageFn<LocaleMessage>,
    /// Live-region announcement for visible result count.
    pub results_count: MessageFn<ResultCountMessage>,
}
```

Default English messages are "Search", "Clear search", "Suggestions",
"Loading suggestions", "No results found", and pluralized result-count
announcements.

## 5. Integration Boundaries

Adapters wire `AutocompleteDebounce` to wall-clock timers and may start async
fetches when debounced input settles. Adapters then send `SetLoading(true)`,
`UpdateItems`, and `SetLoading(false)` as data arrives. Core string IDs are
valid for `aria-controls`, `aria-activedescendant`, hydration-stable markup,
and `data-ars-*` hooks. They are not a substitute for live element handles.

## 6. Usage Patterns

### 6.1 Searchable command list

```tsx
<Autocomplete>
    <Autocomplete.Input />
    <Autocomplete.Content>
        <Autocomplete.Item />
    </Autocomplete.Content>
</Autocomplete>
```

### 6.2 Server-backed suggestions

```tsx
<Autocomplete debounce={200} loading={isFetching} items={suggestions} />
```

## 7. Library Parity

React Aria's `Autocomplete` is a controller utility layered over text input and
collection components. ars-ui follows that pattern but exposes explicit
framework-agnostic state and `AttrMap` output so Leptos and Dioxus adapters can
own DOM handles and async effects without duplicating selection semantics.
