---
component: Combobox
category: selection
tier: complex
foundation_deps: [architecture, accessibility, interactions, collections]
shared_deps: [selection-patterns]
related: []
references:
    ark-ui: Combobox
    react-aria: ComboBox
---

# Combobox

A text input combined with a dropdown list. The user types to filter options, and selects
from the filtered list.

> **Performance**: For large option lists (>1000 items), combine `Combobox` with `Virtualizer`
> for efficient rendering. See §Collection Virtualization in `06-collections.md`.

Items are stored as a `StaticCollection<combobox::Item>` (from `06-collections.md`). All
navigation uses `Collection` trait methods with `next_enabled_key` / `prev_enabled_key`
for disabled-item awareness. Selection is managed via `select::State` and filtering via
`FilteredCollection` — both from `ars-collections`.

## 1. State Machine

```rust
/// User-facing payload for `Combobox` items. The machine uses Node-level metadata
/// (key, text_value, node_type) for navigation and ARIA; the value `T` is
/// opaque to the machine and consumed only by the adapter for rendering.
#[derive(Clone, Debug)]
pub struct Item {
    /// The label of the item.
    pub label: String,
}
```

Groups (sections with headers) are structural `NodeType::Section` + `NodeType::Header` nodes
in the collection, built via `CollectionBuilder::section()`.

### 1.1 States

```rust
/// The states of the Combobox state machine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The combobox is closed.
    Closed,
    /// The combobox is open.
    Open,
}
```

### 1.2 Events

```rust
/// The events of the Combobox state machine.
#[derive(Clone, Debug)]
pub enum Event {
    /// The input value changed.
    InputChange(String),
    /// The focus received on the input.
    Focus {
        /// Whether the focus is from a keyboard event.
        is_keyboard: bool,
    },
    /// The focus lost from the input.
    Blur,
    /// The combobox is opened.
    Open,
    /// The combobox is closed.
    Close,
    /// The item is selected.
    SelectItem(Key),
    /// Ctrl/Cmd+Click toggle: toggles a single item without replacing the selection.
    /// Only effective when `selection_behavior == Replace` and `selection_mode == Multiple`.
    /// See §Selection Behavior for adapter translation from pointer events.
    SelectItemCtrl(Key),
    /// The item is deselected.
    DeselectItem(Key),
    /// The item is highlighted.
    HighlightItem(Option<Key>),
    /// The first item is highlighted.
    HighlightFirst,
    /// The last item is highlighted.
    HighlightLast,
    /// The next item is highlighted.
    HighlightNext,
    /// The previous item is highlighted.
    HighlightPrev,
    /// The combobox is dismissed.
    Dismiss,
    /// The combobox is clicked outside.
    ClickOutside,
    /// The combobox is cleared.
    Clear,
    /// The item collection is updated.
    /// Dynamically replace the item collection (e.g., async search results).
    UpdateItems(StaticCollection<Item>),
    /// IME composition started (CJK, etc.).
    CompositionStart,
    /// IME composition ended — committed text is in the next InputChange.
    CompositionEnd,
}
```

> **IME composition:** This component tracks `is_composing: bool` in `Context`. During composition (`is_composing == true`), `InputChange` events MUST NOT trigger filtering or item highlighting, and Enter/Tab key handlers MUST be suppressed (they would commit the wrong value). See §IME in `03-accessibility.md`.

### 1.3 Context

```rust
/// The context for the Combobox state machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The full (unfiltered) item collection.
    pub items: StaticCollection<Item>,
    /// Text value of the input field.
    pub input_value: Bindable<String>,
    /// Controlled/uncontrolled selection binding.
    pub selection: Bindable<selection::Set>,
    /// Full selection state: mode, behavior, anchor, disabled_keys.
    pub selection_state: selection::State,
    /// Currently highlighted (keyboard-focused) item key.
    pub highlighted_key: Option<Key>,
    /// Keys that pass the current filter. When `None`, all items are visible (no filter active).
    /// When `Some(keys)`, only items whose key is in the set should be rendered.
    pub visible_keys: Option<BTreeSet<Key>>,
    /// Whether the combobox is open.
    pub open: bool,
    /// Whether the combobox is focused.
    pub focused: bool,
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// Whether the combobox is disabled.
    pub disabled: bool,
    /// Whether the combobox is readonly.
    pub readonly: bool,
    /// Whether the combobox is required.
    pub required: bool,
    /// Whether the combobox is invalid.
    pub invalid: bool,
    /// Whether the combobox allows multiple selections.
    pub multiple: bool,
    /// The filter mode.
    pub filter_mode: FilterMode,
    /// Whether the combobox opens on focus.
    pub open_on_focus: bool,
    /// The name of the combobox.
    pub name: Option<String>,
    /// Whether the combobox loops focus.
    pub loop_focus: bool,
    /// True while an IME composition session is active (between CompositionStart and CompositionEnd).
    pub is_composing: bool,
    /// Whether a description part is rendered (used for aria-describedby wiring).
    pub has_description: bool,
    /// True when running on iOS with VoiceOver active. Determined at init time via
    /// `resolve_focus_strategy()`. When true, the connect code omits
    /// `aria-activedescendant` and uses `aria-selected` on the highlighted option instead.
    pub is_ios: bool,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
    /// The current locale for message resolution.
    pub locale: Locale,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}

/// `FilterMode` — defined in `shared/selection-patterns.md` §1.6.
/// Combobox uses all variants including `None`, `Inline`, and `InlineCompletion`.
```

### 1.4 Props

```rust
/// Props for the Combobox state machine.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the combobox.
    pub id: String,
    /// The input value of the combobox.
    pub input_value: Option<String>,
    /// The default input value of the combobox.
    pub default_input_value: String,
    /// The value of the combobox.
    pub value: Option<selection::Set>,
    /// The default value of the combobox.
    pub default_value: selection::Set,
    /// The selection mode of the combobox.
    pub selection_mode: selection::Mode,
    /// The selection behavior of the combobox.
    pub selection_behavior: selection::Behavior,
    /// The disabled behavior of the combobox.
    pub disabled_behavior: DisabledBehavior,
    /// Whether the combobox is disabled.
    pub disabled: bool,
    /// Whether the combobox is readonly.
    pub readonly: bool,
    /// Whether the combobox is required.
    pub required: bool,
    /// Whether the combobox is invalid.
    pub invalid: bool,
    /// The placeholder of the combobox.
    pub placeholder: Option<String>,
    /// The filter mode of the combobox.
    pub filter_mode: FilterMode,
    /// Whether the combobox opens on focus.
    pub open_on_focus: bool,
    /// Open the listbox when the input is clicked (separate from `open_on_focus`).
    /// Matches Ark-UI's `openOnClick` prop. When `true`, clicking the input opens
    /// the dropdown even if `open_on_focus` is `false`. Default: `false`.
    pub open_on_click: bool,
    /// The name of the combobox.
    pub name: Option<String>,
    /// Whether the combobox loops focus.
    pub loop_focus: bool,
    /// The positioning options of the combobox.
    pub positioning: PositioningOptions,
    /// Initial highlighted key when the combobox first opens.
    /// When `None`, the first enabled item is highlighted. When `Some(key)`, that item is
    /// highlighted on open (useful for search UIs where the best match
    /// should be pre-highlighted).
    pub default_highlighted_key: Option<Key>,
    /// When `true`, the listbox remains open even when the filter produces zero results,
    /// displaying the EmptyState message instead of closing. Default: `false`.
    pub allows_empty_collection: bool,
    /// Callback invoked when the dropdown open state changes.
    /// Fires during Open/Close state transitions with the new open state value (`true` for
    /// opening, `false` for closing). Invoked after the transition completes.
    /// Default: `None`.
    pub on_open_change: Option<Callback<bool>>,
    /// Keys of items that should be disabled.
    pub disabled_keys: BTreeSet<Key>,
    /// Allow the user to enter a value that does not match any filtered option.
    /// When `true`, the raw input text is accepted as the value on blur or Enter,
    /// even if it doesn't match any item in the collection.
    /// When `false` (default), non-matching input is reverted to the last valid selection on blur.
    pub allow_custom_value: bool,
    // Change callbacks provided by the adapter layer
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            input_value: None, default_input_value: String::new(),
            value: None, default_value: selection::Set::Empty,
            selection_mode: selection::Mode::Single,
            selection_behavior: selection::Behavior::Toggle,
            disabled_behavior: DisabledBehavior::Skip,
            disabled: false, readonly: false, required: false, invalid: false,
            placeholder: None, filter_mode: FilterMode::Contains,
            open_on_focus: true, open_on_click: false,
            name: None, loop_focus: true,
            positioning: PositioningOptions::default(),
            default_highlighted_key: None,
            allows_empty_collection: false,
            disabled_keys: BTreeSet::new(),
            on_open_change: None,
            allow_custom_value: false,
        }
    }
}
```

Note: `items` is set during `init()` and does not auto-sync when Props change.
To update items dynamically (e.g., async search results), send an
`Event::UpdateItems(StaticCollection<Item>)` event.

### 1.5 Full Machine Implementation

```rust
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;
    type Messages = Messages;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let locale = env.locale.clone();
        let messages = messages.clone();
        let selection_state = selection::State {
            mode: props.selection_mode.clone(),
            behavior: props.selection_behavior.clone(),
            disabled_behavior: props.disabled_behavior.clone(),
            disabled_keys: props.disabled_keys.clone(),
            ..Default::default()
        };

        let ctx = Context {
            items: StaticCollection::default(),
            input_value: match &props.input_value {
                Some(v) => Bindable::controlled(v.clone()),
                None => Bindable::uncontrolled(props.default_input_value.clone()),
            },
            selection: match &props.value {
                Some(v) => Bindable::controlled(v.clone()),
                None => Bindable::uncontrolled(props.default_value.clone()),
            },
            selection_state,
            highlighted_key: props.default_highlighted_key.clone(),
            visible_keys: None,
            open: false,
            focused: false,
            focus_visible: false,
            disabled: props.disabled,
            readonly: props.readonly,
            required: props.required,
            invalid: props.invalid,
            multiple: props.selection_mode == selection::Mode::Multiple,
            filter_mode: props.filter_mode.clone(),
            open_on_focus: props.open_on_focus,
            name: props.name.clone(),
            loop_focus: props.loop_focus,
            is_composing: false,
            has_description: false,
            is_ios: resolve_focus_strategy(FocusStrategy::ActiveDescendant) == FocusStrategy::RovingTabindex,
            ids: ComponentIds::from_id(&props.id),
            locale,
            messages,
        };

        (State::Closed, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // Readonly guard: reject mutating events when readonly
        // (Open for browsing is allowed; selecting/deselecting/clearing is not)
        if ctx.readonly {
            match event {
                Event::SelectItem(_) | Event::DeselectItem(_) | Event::Clear | Event::InputChange(_) => return None,
                _ => {}
            }
        }

        match (state, event) {
            // During IME composition, InputChange reflects intermediate composing text.
            // Do not update the filter or trigger selection changes.
            (_, Event::InputChange(_)) if ctx.is_composing => {
                None
            }

            // InputChange: update input, compute visible_keys via FilteredCollection, open if closed
            (_, Event::InputChange(val)) => {
                if ctx.disabled || ctx.readonly { return None; }
                let val = val.clone();
                let was_open = ctx.open;

                // Construct ephemeral FilteredCollection to compute visible keys
                let visible = if val.is_empty() {
                    None // no filter active — show all items
                } else {
                    let filtered = FilteredCollection::new(&ctx.items, |node| {
                        match ctx.filter_mode {
                            FilterMode::Contains => node.text_value.to_lowercase().contains(&val.to_lowercase()),
                            FilterMode::StartsWith => node.text_value.to_lowercase().starts_with(&val.to_lowercase()),
                            FilterMode::None => true,
                            FilterMode::Custom => true, // adapter handles custom filtering
                            FilterMode::InlineCompletion => node.text_value.to_lowercase().starts_with(&val.to_lowercase()),
                        }
                    });
                    let keys: BTreeSet<Key> = filtered.iter().map(|n| n.key.clone()).collect();
                    Some(keys)
                };

                // Highlight first visible enabled item
                let first_key = match &visible {
                    Some(keys) => keys.iter()
                        .find(|k| !ctx.selection_state.is_disabled(k))
                        .cloned(),
                    None => first_enabled_key(&ctx.items, &ctx.selection_state.disabled_keys,
                        ctx.selection_state.disabled_behavior),
                };

                let should_open = !was_open && !val.is_empty();
                if should_open {
                    Some(TransitionPlan::to(State::Open).apply(move |ctx| {
                        ctx.input_value.set(val);
                        ctx.open = true;
                        ctx.visible_keys = visible;
                        ctx.highlighted_key = first_key;
                    }).with_effect(PendingEffect::new("announce_result_count", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        let count = ctx.visible_keys.as_ref().map_or(ctx.items.len(), |k| k.len());
                        let msg = (ctx.messages.results_count)(count, &ctx.locale);
                        platform.announce(&msg);
                        no_cleanup()
                    })))
                } else {
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.input_value.set(val);
                        ctx.visible_keys = visible;
                        ctx.highlighted_key = first_key;
                    }))
                }
            }

            // Open: open the dropdown
            (State::Closed, Event::Open) => {
                if ctx.disabled { return None; }
                let first = first_enabled_key(&ctx.items, &ctx.selection_state.disabled_keys,
                    ctx.selection_state.disabled_behavior);
                Some(TransitionPlan::to(State::Open).apply(move |ctx| {
                    ctx.open = true;
                    ctx.highlighted_key = first;
                }))
            }

            // Close / Dismiss / ClickOutside / Blur: close the dropdown
            (State::Open, Event::Close) | (State::Open, Event::Dismiss) |
            (State::Open, Event::ClickOutside) => {
                Some(TransitionPlan::to(State::Closed).apply(|ctx| {
                    ctx.open = false;
                    ctx.highlighted_key = None;
                }))
            }

            // Focus
            (_, Event::Focus { is_keyboard }) => {
                let should_open = ctx.open_on_focus && !ctx.open && !ctx.disabled;
                let first = if should_open {
                    first_enabled_key(&ctx.items, &ctx.selection_state.disabled_keys,
                        ctx.selection_state.disabled_behavior)
                } else { None };
                if should_open {
                    Some(TransitionPlan::to(State::Open).apply(move |ctx| {
                        ctx.focused = true;
                        ctx.focus_visible = *is_keyboard;
                        ctx.open = true;
                        ctx.highlighted_key = first;
                    }))
                } else {
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.focused = true;
                        ctx.focus_visible = *is_keyboard;
                    }))
                }
            }

            (_, Event::Blur) => {
                Some(TransitionPlan::to(State::Closed).apply(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                    ctx.open = false;
                    ctx.highlighted_key = None;
                }))
            }

            // HighlightNext / HighlightPrev: navigate visible items, skipping disabled
            (_, Event::HighlightNext) => {
                let next = match &ctx.highlighted_key {
                    Some(k) => next_enabled_key(&ctx.items, k, &ctx.selection_state.disabled_keys,
                        ctx.selection_state.disabled_behavior, ctx.loop_focus),
                    None => first_enabled_key(&ctx.items, &ctx.selection_state.disabled_keys,
                        ctx.selection_state.disabled_behavior),
                };
                // Filter to visible keys if active
                let next = match (&next, &ctx.visible_keys) {
                    (Some(k), Some(visible)) if !visible.contains(k) => None,
                    _ => next,
                };
                next.map(|k| TransitionPlan::context_only(move |ctx| { ctx.highlighted_key = Some(k); }))
            }

            (_, Event::HighlightPrev) => {
                let prev = match &ctx.highlighted_key {
                    Some(k) => prev_enabled_key(&ctx.items, k, &ctx.selection_state.disabled_keys,
                        ctx.selection_state.disabled_behavior, ctx.loop_focus),
                    None => last_enabled_key(&ctx.items, &ctx.selection_state.disabled_keys,
                        ctx.selection_state.disabled_behavior),
                };
                let prev = match (&prev, &ctx.visible_keys) {
                    (Some(k), Some(visible)) if !visible.contains(k) => None,
                    _ => prev,
                };
                prev.map(|k| TransitionPlan::context_only(move |ctx| { ctx.highlighted_key = Some(k); }))
            }

            (_, Event::HighlightFirst) => {
                let first = first_enabled_key(&ctx.items, &ctx.selection_state.disabled_keys,
                    ctx.selection_state.disabled_behavior);
                first.map(|k| TransitionPlan::context_only(move |ctx| { ctx.highlighted_key = Some(k); }))
            }

            (_, Event::HighlightLast) => {
                let last = last_enabled_key(&ctx.items, &ctx.selection_state.disabled_keys,
                    ctx.selection_state.disabled_behavior);
                last.map(|k| TransitionPlan::context_only(move |ctx| { ctx.highlighted_key = Some(k); }))
            }

            (_, Event::HighlightItem(key)) => {
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx| { ctx.highlighted_key = key; }))
            }

            // SelectItem: set selection, update input, optionally close
            (State::Open, Event::SelectItem(key)) => {
                if ctx.selection_state.is_disabled(&key) { return None; }
                let key = key.clone();
                let is_multiple = ctx.multiple;

                if is_multiple {
                    if ctx.selection.get().contains(&key) { return None; }
                    Some(TransitionPlan::context_only(move |ctx| {
                        let new_sel = ctx.selection_state.select(key, &ctx.items);
                        ctx.selection.set(new_sel);
                        ctx.input_value.set(String::new());
                        ctx.visible_keys = None; // reset filter
                    }))
                } else {
                    // Look up the label for single-select display
                    let label = ctx.items.get(&key)
                        .map(|n| n.text_value.clone())
                        .unwrap_or_default();
                    Some(TransitionPlan::to(State::Closed).apply(move |ctx| {
                        ctx.selection.set(selection::Set::Multiple(BTreeSet::from([key])));
                        ctx.input_value.set(label);
                        ctx.open = false;
                        ctx.highlighted_key = None;
                        ctx.visible_keys = None; // reset filter
                    }))
                }
            }

            // DeselectItem
            (_, Event::DeselectItem(key)) => {
                if !ctx.selection.get().contains(&key) { return None; }
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let new_sel = ctx.selection_state.deselect(&key);
                    ctx.selection.set(new_sel);
                }))
            }

            // Clear: reset selection and input
            (_, Event::Clear) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.selection.set(selection::Set::Empty);
                    ctx.input_value.set(String::new());
                    ctx.visible_keys = None;
                }))
            }

            // UpdateItems: dynamically replace the item collection
            (_, Event::UpdateItems(new_items)) => {
                let new_items = new_items.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.items = new_items;
                    ctx.visible_keys = None; // reset filter on new items
                }))
            }

            // IME Composition
            (_, Event::CompositionStart) => {
                Some(TransitionPlan::context_only(|ctx| { ctx.is_composing = true; }))
            }
            (_, Event::CompositionEnd) => {
                Some(TransitionPlan::context_only(|ctx| { ctx.is_composing = false; }))
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

### 1.6 Connect / API

The `Api` struct (defined above in the `Machine` block) provides `*_attrs()` methods
returning `AttrMap` values with data attributes and ARIA attributes.

```rust
#[derive(ComponentPart)]
#[scope = "combobox"]
pub enum Part {
    Root,
    Label,
    Control,
    Input,
    Trigger,
    ClearTrigger,
    Positioner,
    Content,
    ItemGroup { key: Key },
    ItemGroupLabel { key: Key },
    Item { key: Key },
    ItemText { key: Key },
    ItemIndicator { key: Key },
    Empty,
    Description,
    ErrorMessage,
    LiveRegion,
}

/// The API for the Combobox state machine.
pub struct Api<'a> {
    /// The current state of the Combobox.
    state: &'a State,
    /// The context of the Combobox.
    ctx: &'a Context,
    /// The props of the Combobox.
    props: &'a Props,
    /// The send function to send events to the Combobox state machine.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// The attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        attrs.set(HtmlAttr::Data("ars-state"), if self.ctx.open { "open" } else { "closed" });
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.invalid { attrs.set_bool(HtmlAttr::Data("ars-invalid"), true); }
        if self.ctx.readonly { attrs.set_bool(HtmlAttr::Data("ars-readonly"), true); }
        attrs
    }

    /// The attributes for the label element.
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("label"));
        attrs.set(HtmlAttr::For, self.ctx.ids.part("input"));
        attrs
    }

    /// The attributes for the control element.
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// The attributes for the input element.
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("input"));
        attrs.set(HtmlAttr::Role, "combobox");
        attrs.set(HtmlAttr::Type, "text");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.ctx.open { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "listbox");
        if self.ctx.open {
            attrs.set(HtmlAttr::Aria(AriaAttr::Controls), self.ctx.ids.part("content"));
        }
        // aria-autocomplete modes (WAI-ARIA 1.2 §5.2.1.2):
        //   "list"   — filter only (Contains, StartsWith, Custom)
        //   "inline" — completion only (text is completed in the input, no filtering)
        //   "both"   — filter + inline completion combined
        attrs.set(HtmlAttr::Aria(AriaAttr::AutoComplete), match self.ctx.filter_mode {
            FilterMode::None => "none",
            FilterMode::InlineCompletion => "both",
            FilterMode::Inline => "inline",
            _ => "list",
        });
        // iOS VoiceOver fallback: omit aria-activedescendant entirely.
        // Instead, the highlighted option receives aria-selected="true" (see item_attrs).
        if !self.ctx.is_ios {
            if let Some(k) = &self.ctx.highlighted_key {
                attrs.set(HtmlAttr::Aria(AriaAttr::ActiveDescendant), self.ctx.ids.item("item", k));
            }
        }
        if self.ctx.required { attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true"); }
        if self.ctx.invalid { attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true"); }
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
        if self.ctx.readonly { attrs.set_bool(HtmlAttr::ReadOnly, true); }
        // Wire aria-describedby to error-message (first) and description parts.
        // Error-first ordering ensures screen readers announce the error before help text.
        let mut describedby_parts: Vec<String> = Vec::new();
        if self.ctx.invalid {
            describedby_parts.push(self.ctx.ids.part("error-message"));
        }
        if self.ctx.has_description {
            describedby_parts.push(self.ctx.ids.part("description"));
        }
        if !describedby_parts.is_empty() {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), describedby_parts.join(" "));
        }
        if let Some(ph) = &self.props.placeholder {
            attrs.set(HtmlAttr::Placeholder, ph);
        }
        attrs.set(HtmlAttr::EnterKeyHint, match self.ctx.filter_mode {
            FilterMode::None => "done",
            _ => "search",
        });
        attrs
    }

    /// The event handler for the input change event.
    pub fn on_input_change(&self, value: String) {
        (self.send)(Event::InputChange(value));
    }

    /// The event handler for the input keydown event.
    pub fn on_input_keydown(&self, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::ArrowDown if data.alt_key => (self.send)(Event::Open),
            KeyboardKey::ArrowDown => {
                if !self.ctx.open { (self.send)(Event::Open); }
                (self.send)(Event::HighlightNext);
            }
            KeyboardKey::ArrowUp if data.alt_key => (self.send)(Event::Close),
            KeyboardKey::ArrowUp => (self.send)(Event::HighlightPrev),
            KeyboardKey::Home if data.alt_key => (self.send)(Event::HighlightFirst),
            KeyboardKey::End if data.alt_key => (self.send)(Event::HighlightLast),
            KeyboardKey::Enter => {
                if !self.ctx.is_composing {
                    if let Some(k) = &self.ctx.highlighted_key {
                        (self.send)(Event::SelectItem(k.clone()));
                    }
                }
            }
            KeyboardKey::Escape => (self.send)(Event::Close),
            _ => {}
        }
    }

    /// The attributes for the trigger element.
    /// Text to display when the filter produces zero results.
    /// The adapter renders this as visible text in the empty state area.
    pub fn no_results_text(&self) -> String {
        (self.ctx.messages.no_results)(&self.ctx.locale)
    }

    /// Text to display while async options are loading.
    /// The adapter renders this as visible text in the loading state area.
    pub fn loading_text(&self) -> String {
        (self.ctx.messages.loading)(&self.ctx.locale)
    }

    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.trigger_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), self.ctx.ids.part("content"));
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
        attrs
    }

    /// The event handler for the trigger click event.
    pub fn on_trigger_click(&self) { (self.send)(Event::Open); }

    /// The attributes for the clear trigger element.
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ClearTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.clear_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::TabIndex, "-1");
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
        attrs
    }

    /// The event handler for the clear trigger click event.
    pub fn on_clear_click(&self) { (self.send)(Event::Clear); }

    /// The attributes for the positioner element.
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// The attributes for the content element.
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("content"));
        attrs.set(HtmlAttr::Role, "listbox");
        if self.ctx.multiple { attrs.set(HtmlAttr::Aria(AriaAttr::MultiSelectable), "true"); }
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));
        attrs
    }

    /// The attributes for the item element.
    pub fn item_attrs(&self, key: &Key) -> AttrMap {
        let node = self.ctx.items.get(key);
        let item_id = self.ctx.ids.item("item", &key.to_string());
        let is_selected = self.ctx.selection.get().contains(key);
        let is_highlighted = self.ctx.highlighted_key.as_ref() == Some(key);
        let is_disabled = self.ctx.disabled || self.ctx.selection_state.is_disabled(key);

        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Item { key: Default::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, item_id);
        attrs.set(HtmlAttr::Role, "option");
        // iOS VoiceOver fallback: set aria-selected="true" on the highlighted item
        // so VoiceOver announces it as the active option (since aria-activedescendant
        // is omitted from the input on iOS). On non-iOS, aria-selected reflects
        // selection state only.
        let aria_selected = if self.ctx.is_ios {
            is_selected || is_highlighted
        } else {
            is_selected
        };
        attrs.set(HtmlAttr::Aria(AriaAttr::Selected), if aria_selected { "true" } else { "false" });
        if is_disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        attrs.set(HtmlAttr::Data("ars-state"), if is_selected { "selected" } else { "unselected" });
        if is_highlighted { attrs.set_bool(HtmlAttr::Data("ars-highlighted"), true); }
        if is_disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if let Some(n) = node { attrs.set(HtmlAttr::Data("ars-value"), &n.text_value); }
        attrs
    }

    /// The attributes for the item text element.
    pub fn item_text_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ItemText { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.item_part("item", &key.to_string(), "text"));
        attrs
    }

    /// The attributes for the item indicator element.
    pub fn item_indicator_attrs(&self, key: &Key) -> AttrMap {
        let is_selected = self.ctx.selection.get().contains(key);
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ItemIndicator { key: Default::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs.set(HtmlAttr::Data("ars-state"), if is_selected { "selected" } else { "unselected" });
        attrs
    }

    /// The attributes for the item group element.
    pub fn item_group_attrs(&self, section_key: &Key) -> AttrMap {
        let group_label_id = self.ctx.ids.item_part("group", &section_key, "label");
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ItemGroup { key: Default::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "group");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), group_label_id);
        attrs
    }

    /// The attributes for the item group label element.
    pub fn item_group_label_attrs(&self, section_key: &Key) -> AttrMap {
        let group_label_id = self.ctx.ids.item_part("group", &section_key, "label");
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ItemGroupLabel { key: Default::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, group_label_id);
        attrs
    }

    /// The event handler for the item click event.
    pub fn on_item_click(&self, key: Key) { (self.send)(Event::SelectItem(key)); }

    /// The event handler for the item hover event.
    pub fn on_item_hover(&self, key: Key) { (self.send)(Event::HighlightItem(Some(key))); }

    /// The event handler for the item leave event.
    pub fn on_item_leave(&self) { (self.send)(Event::HighlightItem(None)); }

    /// The attributes for the description element.
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));
        attrs
    }

    /// The attributes for the error message element.
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("error-message"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        attrs
    }

    /// The attributes for the live region element.
    pub fn live_region_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::LiveRegion.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        attrs.set(HtmlAttr::Aria(AriaAttr::Atomic), "true");
        attrs
    }

    /// The attributes for the empty state element.
    /// Displayed when the filter produces zero results and `allows_empty_collection` is true.
    /// Uses `role="none"` because Combobox has a LiveRegion sibling that handles announcements.
    pub fn empty_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Empty.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("empty"));
        attrs.set(HtmlAttr::Role, "none");
        attrs
    }

    /// Returns an iterator over visible items (filtered if a filter is active).
    pub fn visible_items(&self) -> impl Iterator<Item = &Node<Item>> {
        self.ctx.items.iter().filter(|n| {
            n.node_type == NodeType::Item
                && self.ctx.visible_keys.as_ref().map_or(true, |keys| keys.contains(&n.key))
        })
    }

    /// Returns an iterator over all nodes (including structural) for full rendering.
    pub fn items(&self) -> impl Iterator<Item = &Node<Item>> {
        self.ctx.items.iter()
    }

    /// Convenience: text of the first selected item (for display in single-select mode).
    pub fn selected_text(&self) -> Option<&str> {
        match self.ctx.selection.get() {
            selection::Set::Multiple(keys) => keys.iter().next()
                .and_then(|k| self.ctx.items.get(k))
                .map(|n| n.text_value.as_str()),
            _ => None,
        }
    }

    /// Result count for live region announcement.
    pub fn visible_count(&self) -> usize {
        match &self.ctx.visible_keys {
            Some(keys) => keys.len(),
            None => self.ctx.items.iter().filter(|n| n.node_type == NodeType::Item).count(),
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Control => self.control_attrs(),
            Part::Input => self.input_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::ClearTrigger => self.clear_trigger_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::ItemGroup { ref key } => self.item_group_attrs(key),
            Part::ItemGroupLabel { ref key } => self.item_group_label_attrs(key),
            Part::Item { ref key } => self.item_attrs(key),
            Part::ItemText { ref key } => self.item_text_attrs(key),
            Part::ItemIndicator { ref key } => self.item_indicator_attrs(key),
            Part::Empty => self.empty_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::LiveRegion => self.live_region_attrs(),
        }
    }
}
```

> **Async Loading**: When integrated with `AsyncCollection<T>` (see `06-collections.md` §5), `Combobox` supports server-driven search with `loading_state: LoadingState` and `LoadMore` event. The adapter shows a loading spinner in the listbox and sets `aria-busy="true"` on the listbox during loading.

## 2. Anatomy

| Part             | Selector                                                        | Element                                                   |
| ---------------- | --------------------------------------------------------------- | --------------------------------------------------------- | ---------------------------------------------------------------- |
| `Root`           | `[data-ars-scope="combobox"][data-ars-part="root"]`             | `<div>`                                                   |
| `Label`          | `[data-ars-scope="combobox"][data-ars-part="label"]`            | `<label>`                                                 |
| `Control`        | `[data-ars-scope="combobox"][data-ars-part="control"]`          | `<div>`                                                   |
| `Input`          | `[data-ars-scope="combobox"][data-ars-part="input"]`            | `<input>`                                                 |
| `Trigger`        | `[data-ars-scope="combobox"][data-ars-part="trigger"]`          | `<button>`                                                |
| `ClearTrigger`   | `[data-ars-scope="combobox"][data-ars-part="clear-trigger"]`    | `<button>`                                                |
| `Positioner`     | `[data-ars-scope="combobox"][data-ars-part="positioner"]`       | `<div>`                                                   |
| `Content`        | `[data-ars-scope="combobox"][data-ars-part="content"]`          | `<div>`                                                   |
| `ItemGroup`      | `[data-ars-scope="combobox"][data-ars-part="item-group"]`       | `<div>`                                                   |
| `ItemGroupLabel` | `[data-ars-scope="combobox"][data-ars-part="item-group-label"]` | `<div>`                                                   |
| `Item`           | `[data-ars-scope="combobox"][data-ars-part="item"]`             | `<div>`                                                   |
| `ItemText`       | `[data-ars-scope="combobox"][data-ars-part="item-text"]`        | `<span>`                                                  |
| `ItemIndicator`  | `[data-ars-scope="combobox"][data-ars-part="item-indicator"]`   | `<div>`                                                   |
| `Description`    | `[data-ars-scope="combobox"][data-ars-part="description"]`      | `<div>`                                                   |
| `ErrorMessage`   | `[data-ars-scope="combobox"][data-ars-part="error-message"]`    | `<div>`                                                   |
| `LiveRegion`     | `[data-ars-scope="combobox"][data-ars-part="live-region"]`      | `<div>`                                                   | `aria-live="polite" aria-atomic="true"` — announces result count |
| **EmptyState**   | `<div>`                                                         | Message displayed when the listbox has no matching items. |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property                | Element    | Value                                                                                                                                                                                               |
| ----------------------- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| `role`                  | Input      | `combobox`                                                                                                                                                                                          |
| `aria-expanded`         | Input      | `true` when open                                                                                                                                                                                    |
| `aria-haspopup`         | Input      | `listbox`                                                                                                                                                                                           |
| `aria-controls`         | Input      | Content id                                                                                                                                                                                          |
| `aria-autocomplete`     | Input      | `list` (filter only), `inline` (completion only), or `both` (filter + completion)                                                                                                                   |
| `aria-activedescendant` | Input      | Highlighted item id (only set when a valid item is highlighted; **omit attribute entirely** when `highlighted_key` is `None` — setting it to an empty string or non-existent ID violates ARIA spec) |
| `role`                  | Content    | `listbox`                                                                                                                                                                                           |
| `role`                  | Item       | `option`                                                                                                                                                                                            |
| `aria-selected`         | Item       | `"true"` when selected, `"false"` when unselected (must be explicitly set, not omitted)                                                                                                             |
| `role`                  | EmptyState | `"status"`                                                                                                                                                                                          | Implicit `aria-live="polite"` and `aria-atomic="true"` |
| `aria-atomic`           | EmptyState | `"true"`                                                                                                                                                                                            | Entire message announced as a unit                     |

> **Double announcement resolution**: The `Combobox` has a dedicated `LiveRegion` part
> (`[data-ars-part="live-region"]`) that announces result counts via `aria-live="polite"`.
> When EmptyState is rendered inside a component with an existing `LiveRegion`, EmptyState
> MUST use `role="none"` (not `role="status"`) to avoid double-announcement. The existing
> LiveRegion part is responsible for announcing the empty state message. The `connect()` code
> MUST detect whether a LiveRegion sibling exists and conditionally set the EmptyState role:
>
> - **With LiveRegion sibling** (`Combobox`): EmptyState gets `role="none"`; `LiveRegion` announces the empty message.
> - **Without LiveRegion sibling** (`Select`): EmptyState gets `role="status"` with implicit `aria-live="polite"`.

Additional accessibility notes:

- Trigger sets `aria-controls` pointing to the listbox ID
- `aria-expanded` changes immediately on state change (not after animation)
- Trigger sets `aria-activedescendant` to the focused option ID

### 3.2 Keyboard Interaction

| Key       | Action                                                                                                                                |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| Typing    | Filters list, opens if closed                                                                                                         |
| ArrowDown | Open or highlight next                                                                                                                |
| ArrowUp   | Highlight previous                                                                                                                    |
| Enter     | Select highlighted item                                                                                                               |
| Escape    | 3-phase (per APG inline autocomplete): (1) clear inline completion text if present, (2) close dropdown if open, (3) clear input value |

> **Inline Autocomplete Announcements**: When using `FilterMode::InlineCompletion`,
> the live region MUST announce both the total matching item count and the currently
> highlighted/auto-completed item. Format: `"{N} results available. {item_label}
highlighted."` (localized). The auto-selected (inline completion) text that appears
> in the input field is **visual only** — it does not require special ARIA attributes
> beyond the existing `aria-activedescendant` pointing to the highlighted option.
> The selected text range (completion portion) is indicated via the browser's native
> text selection, which screen readers can detect.
> | Home/End | Move cursor in input; with Alt, highlight first/last |
> | Tab | Close + move focus |

### 3.3 Screen Reader Announcements

**Filtered Result Announcement**: When the listbox filters (after input debounce completes),
a live region announces the result count. Format: `'{count} results available'` (localized,
pluralized via `ars-i18n`). Timing: announcement fires once after the debounce period, not
during typing. If count is 0: `'No results found'`. The live region uses `aria-live='polite'`
and `aria-atomic='true'`.

**Screen Reader Compatibility**: Result count announcements use an assertive live region
visually hidden element (not `aria-live` on the listbox itself) to maximize cross-screen-reader
compatibility (NVDA, JAWS, VoiceOver). The announcement element is a dedicated `<div>` with
`role='status'` outside the listbox DOM.

**Live Region**: When filter results change, announce count:
`"{N} results available"` or `"No results found"`.

> **LiveRegion Timing**: The results count announcement fires 500ms after the last `InputChange` event (debounced), not on every keystroke. This prevents screen reader announcement spam, particularly on NVDA+Firefox which aggressively interrupts with polite announcements. The debounce timer resets on each keystroke. On `Open` with an initial query, the count fires immediately.

## 4. Internationalization

- **Filter matching**: Uses `Collator` for locale-aware case/accent folding.
- **Result count announcement**: Localized, with plural rules from `PluralCategory`.
  Result count announcement uses ICU4X PluralRules for locale-correct pluralization:
  "1 result" (en), "2 results" (en), "1 résultat" (fr), "2 résultats" (fr).
  Uses the `count` field from `Messages` with `{count}` placeholder.
- **RTL**: Input text direction follows locale. Dropdown flips.
- **Placeholder**: User-provided, localized.

> **EmptyState i18n**: The EmptyState message text is sourced from `Messages.no_results` (see `04-internationalization.md` §7.1). Embedded user input in the message must use Unicode directional isolate characters for BiDi safety.

### 4.1 Messages

```rust
/// Messages for the Combobox component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible label for the open trigger button. Default: `"Show suggestions"`.
    pub trigger_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Accessible label for the clear button. Default: `"Clear value"`.
    pub clear_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Text shown when no results match the filter. Default: `"No results found"`.
    pub no_results: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Text shown while async options are loading. Default: `"Loading options…"`.
    pub loading: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Live region announcement for filtered result count.
    pub results_count: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            trigger_label: MessageFn::static_str("Show suggestions"),
            clear_label: MessageFn::static_str("Clear value"),
            no_results: MessageFn::static_str("No results found"),
            loading: MessageFn::static_str("Loading options…"),
            results_count: MessageFn::new(|n, _locale| format!("{} results available", n)),
        }
    }
}

impl ComponentMessages for Messages {}
```

## 5. Variant: Custom Filtering

The built-in `FilterMode` variants (`Contains`, `StartsWith`, `InlineCompletion`) cover the
most common cases, but applications may need domain-specific filtering — for example, fuzzy
matching, server-side search, or filtering on hidden metadata.

When `filter_mode: FilterMode::Custom` is set, the machine **skips all built-in filtering**
(the `InputChange` transition sets `visible_keys = None`, showing all items). Instead, the
consumer provides a custom filter at the adapter level:

```rust
/// Signature of the custom filter callback provided via the adapter.
/// Not stored in Context (closures are not `Clone`/`PartialEq`).
/// Uses `Rc` so the adapter can clone the callback into multiple closures.
pub type FilterFn = Rc<dyn Fn(&str, &StaticCollection<Item>) -> BTreeSet<Key>>;
```

**Integration with `FilteredCollection`** — The adapter can construct a `FilteredCollection`
to compute filtered keys, then send `Event::UpdateItems` or directly set `visible_keys`:

1. Set `filter_mode: FilterMode::Custom` to disable built-in filtering.
2. On each `InputChange`, the adapter constructs `FilteredCollection::new(&items, predicate)`
   and collects the visible keys.
3. For async/server-driven search, the adapter sends `Event::UpdateItems(new_collection)` to
   replace the entire collection with server results.

See `06-collections.md` §3 for `FilteredCollection` API details.

### 5.1 Behavior

The state machine maintains `filter_text: String` in `Context` (updated by `InputChange`).
The adapter is responsible for wiring the actual `FilteredCollection` filtering:

- The machine stores the filter text but does NOT directly execute the filter predicate
  (predicates are closures, which are not `Clone`/`PartialEq`).
- The adapter reads `ctx.filter_text` and calls `FilteredCollection::set_filter()` or
  constructs a new `FilteredCollection` with the appropriate predicate on each change.
- After filtering, the adapter calls `apply_filter()` and sends `Event::UpdateVisibleKeys`
  to sync the machine's `visible_keys` with the filtered results.
- **Highlighted item invariant**: After filtering, the machine checks whether
  `ctx.highlighted_key` is still within the filtered results. If not, it resets to
  the first visible item (or `None` if no results match).
- This split ensures the machine remains pure (no closures in `Context`) while the
  adapter handles framework-specific reactivity.

## 6. Library Parity

> Compared against: Ark UI (`Combobox`), React Aria (`ComboBox`).

### 6.1 Props

| Feature                       | ars-ui                                                                     | Ark UI                                                  | React Aria                           | Notes                                      |
| ----------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------- | ------------------------------------ | ------------------------------------------ |
| Controlled/uncontrolled value | `value` / `default_value`                                                  | `value` / `defaultValue`                                | `selectedKey` / `defaultSelectedKey` | --                                         |
| Input value                   | `input_value` / `default_input_value`                                      | `inputValue` / `defaultInputValue`                      | `inputValue` / `defaultInputValue`   | --                                         |
| Multiple selection            | `selection_mode`                                                           | `multiple`                                              | `selectionMode`                      | --                                         |
| Disabled                      | `disabled`                                                                 | `disabled`                                              | `isDisabled`                         | --                                         |
| Read-only                     | `readonly`                                                                 | `readOnly`                                              | `isReadOnly`                         | --                                         |
| Required                      | `required`                                                                 | `required`                                              | `isRequired`                         | --                                         |
| Invalid                       | `invalid`                                                                  | `invalid`                                               | `isInvalid`                          | --                                         |
| Disabled keys                 | `disabled_keys`                                                            | --                                                      | `disabledKeys`                       | --                                         |
| Placeholder                   | `placeholder`                                                              | `placeholder`                                           | `placeholder`                        | --                                         |
| Filter mode                   | `filter_mode` (`Contains`/`StartsWith`/`InlineCompletion`/`Custom`/`None`) | `inputBehavior` (`none`/`autohighlight`/`autocomplete`) | `defaultFilter` + `menuTrigger`      | Different abstraction granularity          |
| Allow custom value            | `allow_custom_value`                                                       | `allowCustomValue`                                      | `allowsCustomValue`                  | --                                         |
| Open on focus                 | `open_on_focus`                                                            | `openOnKeyPress`                                        | `menuTrigger: 'focus'`               | --                                         |
| Open on click                 | `open_on_click`                                                            | `openOnClick`                                           | --                                   | --                                         |
| Loop focus                    | `loop_focus`                                                               | `loopFocus`                                             | `shouldFocusWrap`                    | --                                         |
| Positioning                   | `positioning`                                                              | `positioning`                                           | via `Popover`                        | --                                         |
| Form name                     | `name`                                                                     | `name`                                                  | `name`                               | --                                         |
| Allows empty collection       | `allows_empty_collection`                                                  | --                                                      | `allowsEmptyCollection`              | --                                         |
| Default highlighted key       | `default_highlighted_key`                                                  | `defaultHighlightedValue`                               | --                                   | --                                         |
| Selection behavior (input)    | `selection_behavior`                                                       | `selectionBehavior` (`clear`/`replace`/`preserve`)      | --                                   | What happens to input text on selection    |
| Open change callback          | `on_open_change`                                                           | `onOpenChange`                                          | `onOpenChange`                       | --                                         |
| Validate                      | --                                                                         | --                                                      | `validate` / `validationBehavior`    | React Aria exclusive; form-level in ars-ui |
| Form value type               | --                                                                         | --                                                      | `formValue` (`text`/`key`)           | React Aria exclusive                       |

**Gaps:** None. React Aria's `formValue` and `validate` are form-level concerns handled outside the Combobox machine in ars-ui.

### 6.2 Anatomy

| Part           | ars-ui           | Ark UI           | React Aria                     | Notes                                         |
| -------------- | ---------------- | ---------------- | ------------------------------ | --------------------------------------------- |
| Root           | `Root`           | `Root`           | `ComboBox`                     | --                                            |
| Label          | `Label`          | `Label`          | `Label`                        | --                                            |
| Control        | `Control`        | `Control`        | --                             | Wrapper for input + trigger                   |
| Input          | `Input`          | `Input`          | `Input`                        | --                                            |
| Trigger        | `Trigger`        | `Trigger`        | `Button`                       | --                                            |
| ClearTrigger   | `ClearTrigger`   | `ClearTrigger`   | --                             | --                                            |
| Positioner     | `Positioner`     | `Positioner`     | `Popover`                      | --                                            |
| Content        | `Content`        | `Content`        | `ListBox`                      | --                                            |
| ItemGroup      | `ItemGroup`      | `ItemGroup`      | `ComboBoxSection`              | --                                            |
| ItemGroupLabel | `ItemGroupLabel` | `ItemGroupLabel` | `Header`                       | --                                            |
| Item           | `Item`           | `Item`           | `ListBoxItem` / `ComboBoxItem` | --                                            |
| ItemText       | `ItemText`       | `ItemText`       | --                             | --                                            |
| ItemIndicator  | `ItemIndicator`  | `ItemIndicator`  | --                             | --                                            |
| Empty          | `Empty`          | `Empty`          | --                             | Empty state display                           |
| Description    | `Description`    | --               | `Text[slot=description]`       | --                                            |
| ErrorMessage   | `ErrorMessage`   | --               | `FieldError`                   | --                                            |
| LiveRegion     | `LiveRegion`     | --               | --                             | ars-ui exclusive; screen reader announcements |

**Gaps:** None.

### 6.3 Events

| Callback         | ars-ui                         | Ark UI               | React Aria                             | Notes |
| ---------------- | ------------------------------ | -------------------- | -------------------------------------- | ----- |
| Value change     | via `Bindable`                 | `onValueChange`      | `onSelectionChange`                    | --    |
| Input change     | `Event::InputChange`           | `onInputValueChange` | `onInputChange`                        | --    |
| Open change      | `on_open_change`               | `onOpenChange`       | `onOpenChange`                         | --    |
| Highlight change | via `Context.highlighted_key`  | `onHighlightChange`  | --                                     | --    |
| Focus/blur       | `Event::Focus` / `Event::Blur` | --                   | `onFocus` / `onBlur` / `onFocusChange` | --    |

**Gaps:** None.

### 6.4 Features

| Feature                       | ars-ui                               | Ark UI                                | React Aria            |
| ----------------------------- | ------------------------------------ | ------------------------------------- | --------------------- |
| Filter (contains/starts-with) | Yes                                  | Yes                                   | Yes                   |
| Inline autocomplete           | Yes (`FilterMode::InlineCompletion`) | Yes (`inputBehavior: 'autocomplete'`) | Yes                   |
| Custom filter                 | Yes (`FilterMode::Custom`)           | --                                    | Yes (`defaultFilter`) |
| Allow custom value            | Yes                                  | Yes                                   | Yes                   |
| Multiple selection            | Yes                                  | Yes                                   | Yes                   |
| Disabled items                | Yes                                  | Yes                                   | Yes                   |
| Item groups                   | Yes                                  | Yes                                   | Yes                   |
| Keyboard navigation           | Yes                                  | Yes                                   | Yes                   |
| IME composition handling      | Yes                                  | --                                    | Yes                   |
| iOS VoiceOver fallback        | Yes                                  | --                                    | Yes                   |
| Result count announcements    | Yes (LiveRegion)                     | --                                    | Yes                   |
| RTL support                   | Yes                                  | Yes                                   | Yes                   |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity -- no gaps identified.
- **Divergences:** (1) ars-ui uses `FilterMode` enum instead of separate `inputBehavior`/`menuTrigger` props; (2) ars-ui has an explicit `LiveRegion` part for screen reader announcements (not present in reference libraries which use framework-specific mechanisms); (3) ars-ui has iOS VoiceOver detection built into the machine context.
- **Recommended additions:** None.
