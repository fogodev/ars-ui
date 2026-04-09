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

> Matches React Aria's `Autocomplete` (beta)
> and Ark-UI's searchable menu/combobox patterns.
> **ComboBox vs Autocomplete**: `ComboBox` is a composite widget combining a text input with
> a listbox popup — it owns the collection and selection state machine. `Autocomplete` is a
> behavioral enhancement that adds suggestion filtering to an existing input — it does not
> own a collection. `ComboBox` = full selection component (like `Select` with typing).
> `Autocomplete` = input decorator (like `TextField` + suggestions). `ComboBox` uses
> `selection::Machine`; `Autocomplete` uses a simpler suggestion-match algorithm without
> selection state.

An `Autocomplete` pairs a text input (`SearchInput` or `TextField`) with a collection
component (`Menu` or `Listbox`) to enable searchable menus, command palettes, and
searchable selects. Unlike `Combobox`, `Autocomplete` is a **controller** — it coordinates
two independent components rather than owning the popup lifecycle itself.

**Key Differences from `ComboBox`**

|              | `Combobox`                    | `Autocomplete`                                                               |
| ------------ | ----------------------------- | ---------------------------------------------------------------------------- |
| `Popup`      | Owns its own floating listbox | Does NOT own a popup; pairs with inline `Menu`/`Listbox`                     |
| `Collection` | `role="listbox"` only         | Supports both `role="listbox"` and `role="menu"`                             |
| `Selection`  | Value-based selection         | Action-based (menu items) or value-based (listbox)                           |
| Use cases    | Form field with suggestions   | Command palette, searchable menu, searchable select                          |
| ARIA         | `role="combobox"` on input    | `role="combobox"` on input + `role="menu"` or `role="listbox"` on collection |

## 1. State Machine

`Autocomplete` is primarily a coordination layer — it does NOT have its own open/closed state.
Instead, it manages the filter string and keyboard delegation between the input and collection.

```diagram
            +------------------------------------------------------+
            |                     Idle                             |
            |  (input not focused, no filtering active)            |
            +--------------+---------------------------------------+
                           | Focus { is_keyboard }
                           v
            +------------------------------------------------------+
            |                   Focused                            |
            |  (input focused, no active filter)                   |
            |  Entry: set focused=true, focus_visible=is_keyboard  |
            +----------+------------------------+------------------+
                       | InputChange(value)      | Blur
                       v                         v -> Idle
            +------------------------------------------------------+
            |                 Interacting                          |
            |  (user is typing, filter active)                     |
            |  Entry: update input_value, trigger filter           |
            |                                                      |
            |  InputChange -> update filter, stay                  |
            |  DelegateToCollection -> forward to Menu/Listbox     |
            |  ItemActivated -> fire consumer callback             |
            |  Clear -> reset input, transition to Focused         |
            |  Blur -> transition to Idle                          |
            +------------------------------------------------------+
```

**Key invariants:**

- `Autocomplete` never directly mutates the collection's selection or highlight state. It
  forwards navigation events via `DelegateToCollection`, and the paired `Menu` or `Listbox`
  handles them through its own state machine.
- The `InputChange` event triggers filtering via `FilteredCollection::set_filter()` in the
  adapter layer. The `Autocomplete` machine only stores the input value; the adapter is
  responsible for calling the filter and updating the collection's visible items.
- `ItemActivated` is fired by the collection component (`Menu` or `Listbox`) when an item is
  confirmed (Enter, click). The adapter intercepts this and calls the consumer's `on_select`
  or `on_action` callback.

### 1.1 States

```rust
/// The states of the Autocomplete state machine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// Input is not focused.
    Idle,
    /// Input is focused but no active filtering.
    Focused,
    /// User is actively typing / filtering.
    Interacting,
}
```

### 1.2 Events

```rust
/// Events for the Autocomplete state machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Input text changed — triggers filtering.
    InputChange(String),
    /// Arrow key pressed in input — delegate to collection navigation.
    DelegateToCollection(CollectionNavEvent),
    /// Item activated in the collection — fire action or select value.
    ItemActivated(String),
    /// Focus received on the input.
    Focus {
        /// Whether the focus is from a keyboard event.
        is_keyboard: bool,
    },
    /// Focus lost from the entire autocomplete region.
    Blur,
    /// Clear the filter input.
    Clear,
}

pub mod collection_nav {
    /// Events for collection navigation.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub enum Event {
        /// Highlight the next item in the collection.
        HighlightNext,
        /// Highlight the previous item in the collection.
        HighlightPrev,
        /// Highlight the first item in the collection.
        HighlightFirst,
        /// Highlight the last item in the collection.
        HighlightLast,
        /// Confirm the highlighted item in the collection.
        Confirm,
    }
}
```

### 1.3 Context

```rust
/// Context for the Autocomplete state machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The resolved locale for message formatting.
    pub locale: Locale,
    /// Current filter/search text.
    pub input_value: Bindable<String>,
    /// Whether the input is focused.
    pub focused: bool,
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// Whether the input is disabled.
    pub disabled: bool,
    /// The ID of the associated collection component (`Menu` or `Listbox`).
    pub collection_id: String,
    /// Pending navigation event to forward to the collection component.
    /// The adapter reads this after each transition and, if `Some`, sends the
    /// event to the collection's `Service`, then clears it.
    pub pending_collection_nav: Option<CollectionNavEvent>,
    /// The key of the last activated item. Set by `ItemActivated` so the adapter
    /// can resolve the item's label and update `input_value` (listbox mode) or
    /// invoke the action callback (menu mode).
    pub activated_key: Option<String>,
    /// Component IDs.
    pub ids: ComponentIds,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}
```

### 1.4 Props

```rust
/// Props for the Autocomplete state machine.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the component.
    pub id: String,
    /// Controlled input value. When `Some`, the consumer owns the value.
    pub input_value: Option<String>,
    /// Default input value for uncontrolled mode. Ignored when `input_value` is `Some`.
    pub default_input_value: String,
    /// Whether the input is disabled.
    pub disabled: bool,
    /// ID of the collection component (Menu or Listbox) that receives filtered items.
    pub collection_id: String,
    /// Filter function provided by the consumer. When `None`, uses default contains filter.
    /// The adapter invokes this on every `InputChange` and updates the collection via
    /// `FilteredCollection::set_filter()`.
    pub filter_mode: FilterMode,
    /// When `true`, the input text clears if no matching option is selected when the
    /// component loses focus. Useful for preventing free-form text entry when only
    /// predefined options are valid. Default: `false`.
    pub clear_on_blur: bool,
    // Change callbacks provided by the adapter layer
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            input_value: None,
            default_input_value: String::new(),
            disabled: false,
            collection_id: String::new(),
            filter_mode: FilterMode::default(),
            clear_on_blur: false,
        }
    }
}
```

### 1.5 FilterMode

`FilterMode` — defined in [`shared/selection-patterns.md` §1.6](../../shared/selection-patterns.md#16-filtermode). Autocomplete typically uses `Contains` (default) or `StartsWith`.

### 1.6 Full Machine Implementation

```rust
/// The Autocomplete state machine.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Api<'a> = Api<'a>;
    type Messages = Messages;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let ids = ComponentIds::from_id(&props.id);
        let locale = env.locale.clone();
        let messages = messages.clone();
        (State::Idle, Context {
            locale,
            input_value: match &props.input_value {
                Some(v) => Bindable::controlled(v.clone()),
                None    => Bindable::uncontrolled(props.default_input_value.clone()),
            },
            focused: false,
            focus_visible: false,
            disabled: props.disabled,
            collection_id: props.collection_id.clone(),
            ids,
            messages,
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled {
            if let Event::Blur = event {
                return Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }));
            }
            return None;
        }

        match (state, event) {
            // -- Focus lifecycle --------------------------------------------------
            (State::Idle, Event::Focus { is_keyboard }) => {
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = *is_keyboard;
                }))
            }
            (_, Event::Blur) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }))
            }

            // -- Input changes ----------------------------------------------------
            (_, Event::InputChange(value)) => {
                Some(TransitionPlan::to(State::Interacting).apply(move |ctx| {
                    ctx.input_value.set(value.clone());
                }))
            }

            // -- Clear ------------------------------------------------------------
            (_, Event::Clear) => {
                Some(TransitionPlan::to(State::Focused).apply(|ctx| {
                    ctx.input_value.set(String::new());
                }))
            }

            // -- Collection navigation delegation ---------------------------------
            (_, Event::DelegateToCollection(nav_event)) => {
                // Store the pending navigation so the adapter can forward it
                // to the paired collection component (Menu or Listbox).
                // No state change in Autocomplete — the collection manages
                // its own highlighted item state.
                let nav = nav_event.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.pending_collection_nav = Some(nav);
                }))
            }

            // -- Item activation --------------------------------------------------
            (State::Interacting | State::Focused, Event::ItemActivated(key)) => {
                // Transition to Focused closes the dropdown (leaves Interacting state).
                // Store the activated key so the adapter can resolve the item's label
                // and update ctx.input_value (listbox mode) or invoke the action
                // callback (menu mode).
                let key = key.clone();
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.activated_key = Some(key);
                }))
            }

            _ => None,
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "autocomplete"]
pub enum Part {
    Root,
    Input,
    ClearTrigger,
}

/// The API for the Autocomplete component.
pub struct Api<'a> {
    /// The state of the component.
    state: &'a State,
    /// The context of the component.
    ctx: &'a Context,
    /// The props of the component.
    props: &'a Props,
    /// The send function to send events to the component.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Root container attributes.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        attrs
    }

    /// Returns a live-region announcement string for the current results count.
    /// The adapter renders this inside a `role="status"` `aria-live="polite"`
    /// element that updates when the filtered collection count changes.
    pub fn results_announcement(&self, count: usize) -> String {
        (self.ctx.messages.results_count)(count, &self.ctx.locale)
    }

    /// Input element attributes. Applied to the `<input>` element.
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("input"));
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "combobox");
        attrs.set(HtmlAttr::Aria(AriaAttr::AutoComplete), "list");
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), &self.ctx.collection_id);
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.ctx.focused { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.input_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::Value, &self.ctx.input_value.get());

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        attrs
    }

    /// Clear trigger button attributes.
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("clear-trigger"));
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ClearTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.clear_label)(&self.ctx.locale));

        let is_empty = self.ctx.input_value.get().is_empty();
        if is_empty || self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }
        attrs
    }

    /// Whether the input value is empty.
    pub fn is_empty(&self) -> bool {
        self.ctx.input_value.get().is_empty()
    }

    /// Current input value.
    pub fn input_value(&self) -> &str {
        &self.ctx.input_value.get()
    }

    /// ID of the associated collection component.
    pub fn collection_id(&self) -> &str {
        &self.ctx.collection_id
    }

    /// Handle keydown events on the input.
    pub fn on_input_keydown(&self, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::ArrowDown => (self.send)(Event::DelegateToCollection(CollectionNavEvent::HighlightNext)),
            KeyboardKey::ArrowUp   => (self.send)(Event::DelegateToCollection(CollectionNavEvent::HighlightPrev)),
            KeyboardKey::Home      => (self.send)(Event::DelegateToCollection(CollectionNavEvent::HighlightFirst)),
            KeyboardKey::End       => (self.send)(Event::DelegateToCollection(CollectionNavEvent::HighlightLast)),
            KeyboardKey::Enter     => (self.send)(Event::DelegateToCollection(CollectionNavEvent::Confirm)),
            KeyboardKey::Escape    => {
                if !self.ctx.input_value.get().is_empty() {
                    (self.send)(Event::Clear);
                }
            }
            _ => {}
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Input => self.input_attrs(),
            Part::ClearTrigger => self.clear_trigger_attrs(),
        }
    }
}
```

## 2. Anatomy

| Part           | Selector                                                         | Element    |
| -------------- | ---------------------------------------------------------------- | ---------- |
| `Root`         | `[data-ars-scope="autocomplete"][data-ars-part="root"]`          | `<div>`    |
| `Input`        | `[data-ars-scope="autocomplete"][data-ars-part="input"]`         | `<input>`  |
| `ClearTrigger` | `[data-ars-scope="autocomplete"][data-ars-part="clear-trigger"]` | `<button>` |

The collection (`Menu` or `Listbox`) is rendered as a **sibling or descendant**, not owned by
`Autocomplete`. `Autocomplete` provides `aria-controls` pointing to the collection's ID.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property                | Element | Value                                                                                                                                                |
| ----------------------- | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `role`                  | Input   | `combobox`                                                                                                                                           |
| `aria-expanded`         | Input   | `true` when collection is visible                                                                                                                    |
| `aria-haspopup`         | Input   | `menu` or `listbox` (depends on collection type)                                                                                                     |
| `aria-controls`         | Input   | Collection element ID                                                                                                                                |
| `aria-autocomplete`     | Input   | `list`                                                                                                                                               |
| `aria-activedescendant` | Input   | Currently highlighted item ID in the collection (only set when a valid item is highlighted; **omit attribute entirely** when no item is highlighted) |

### 3.2 Keyboard Interaction

| Key       | Action                                                   |
| --------- | -------------------------------------------------------- |
| Typing    | Filters the collection                                   |
| ArrowDown | Highlight next item in collection (focus stays in input) |
| ArrowUp   | Highlight previous item in collection                    |
| Enter     | Activate/select highlighted item                         |
| Escape    | Clear input or dismiss                                   |
| Tab       | Move focus away                                          |

## 4. Internationalization

### 4.1 Messages

```rust
/// Messages for the Autocomplete component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible label for the input field.
    pub input_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Accessible label for the clear button.
    pub clear_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Announcement when filtered results count changes.
    /// Receives the number of matching results.
    pub results_count: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            input_label: MessageFn::static_str("Search"),
            clear_label: MessageFn::static_str("Clear search"),
            results_count: MessageFn::new(|count, _locale| match count {
                0 => "No results found".to_string(),
                1 => "1 result available".to_string(),
                n => format!("{} results available", n),
            }),
        }
    }
}

impl ComponentMessages for Messages {}
```

## 5. Integration with `FilteredCollection`

`Autocomplete` integrates with `FilteredCollection<T>` from `06-collections.md` §3:

```rust
// Adapter-level wiring (Leptos example):
let collection = create_filtered_collection(all_items, |input, item| {
    item.label.to_lowercase().contains(&input.to_lowercase())
});

// On InputChange, update the filter:
Effect::new(move || {
    let input = autocomplete.ctx.input_value.get();
    collection.set_filter(&input);
});
```

## 6. Usage Patterns

**Searchable `Menu` (Command Palette)**:

```tsx
<Autocomplete>
    <SearchInput />
    <Menu items={filtered_commands} on_select={run_command} />
</Autocomplete>
```

**Searchable `Select`**:

```tsx
<Autocomplete>
    <TextField />
    <Listbox items={filtered_options} selection_mode="single" />
</Autocomplete>
```

## 7. Library Parity

> Compared against: React Aria (`Autocomplete`).

Note: React Aria's `Autocomplete` is a beta/utility component that wraps a SearchField/TextField with a collection (Menu/ListBox/TagGroup/GridList/Table). ars-ui's `Autocomplete` follows the same controller pattern.

### 7.1 Props

| Feature                       | ars-ui                                | React Aria                         | Notes                                                  |
| ----------------------------- | ------------------------------------- | ---------------------------------- | ------------------------------------------------------ |
| Controlled/uncontrolled input | `input_value` / `default_input_value` | `inputValue` / `defaultInputValue` | --                                                     |
| Disabled                      | `disabled`                            | --                                 | ars-ui exclusive                                       |
| Filter mode                   | `filter_mode`                         | `filter` (custom function)         | React Aria uses a callback; ars-ui uses enum + adapter |
| Collection ID                 | `collection_id`                       | implicit (child composition)       | ars-ui uses explicit ID; React Aria uses JSX children  |
| Clear on blur                 | `clear_on_blur`                       | --                                 | ars-ui exclusive                                       |
| Disable auto-focus first      | --                                    | `disableAutoFocusFirst`            | React Aria exclusive                                   |
| Disable virtual focus         | --                                    | `disableVirtualFocus`              | React Aria exclusive                                   |

**Gaps:** None. React Aria's `disableAutoFocusFirst` and `disableVirtualFocus` are low-level focus management knobs that are handled internally by ars-ui's focus strategy resolution.

### 7.2 Anatomy

| Part         | ars-ui                        | React Aria                                     | Notes                              |
| ------------ | ----------------------------- | ---------------------------------------------- | ---------------------------------- |
| Root         | `Root`                        | `Autocomplete`                                 | --                                 |
| Input        | `Input`                       | `SearchField` / `TextField`                    | ars-ui uses a dedicated input part |
| ClearTrigger | `ClearTrigger`                | --                                             | --                                 |
| Collection   | delegated to `Menu`/`Listbox` | `Menu`/`ListBox`/`TagGroup`/`GridList`/`Table` | --                                 |

**Gaps:** None.

### 7.3 Events

| Callback       | ars-ui                 | React Aria              | Notes |
| -------------- | ---------------------- | ----------------------- | ----- |
| Input change   | `Event::InputChange`   | `onInputChange`         | --    |
| Item activated | `Event::ItemActivated` | delegated to collection | --    |

**Gaps:** None.

### 7.4 Features

| Feature             | ars-ui                     | React Aria              |
| ------------------- | -------------------------- | ----------------------- |
| Filter coordination | Yes                        | Yes                     |
| Keyboard delegation | Yes                        | Yes                     |
| Works with Menu     | Yes                        | Yes                     |
| Works with Listbox  | Yes                        | Yes                     |
| Custom filter       | Yes (`FilterMode::Custom`) | Yes (`filter` callback) |
| Virtual focus       | Yes (via collection)       | Yes                     |

**Gaps:** None.

### 7.5 Summary

- **Overall:** Full parity -- no gaps identified.
- **Divergences:** (1) ars-ui uses explicit `collection_id` prop to link the input with the collection component; React Aria uses JSX child composition; (2) ars-ui uses `FilterMode` enum for built-in filter strategies; React Aria requires a custom filter callback; (3) React Aria supports `GridList` and `Table` as collection targets; ars-ui targets `Menu` and `Listbox`.
- **Recommended additions:** None.
