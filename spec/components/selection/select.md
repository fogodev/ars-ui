---
component: Select
category: selection
tier: complex
foundation_deps: [architecture, accessibility, interactions, collections]
shared_deps: [selection-patterns]
related: []
references:
    ark-ui: Select
    radix-ui: Select
    react-aria: Select
---

# Select

A single or multi-value selection dropdown. The trigger displays the current value and opens
a floating listbox of options.

## 1. State Machine

Items are stored as a `StaticCollection<Item>` (from `06-collections.md`). All
navigation uses `Collection` trait methods with `next_enabled_key` / `prev_enabled_key`
for disabled-item awareness. Selection is managed via `selection::State` and typeahead via
`typeahead::State` — both from `ars-collections`.

```rust
/// User-facing payload for Select items. The machine uses Node-level metadata
/// (key, text_value, node_type) for navigation and ARIA; the value `T` is
/// opaque to the machine and consumed only by the adapter for rendering.
#[derive(Clone, Debug)]
pub struct Item {
    /// The label of the item.
    pub label: String,
}
```

Groups become structural `NodeType::Section` + `NodeType::Header` nodes in the collection,
built via `CollectionBuilder::section()`.

### 1.1 States

```rust
/// The states of the Select state machine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The select is closed.
    Closed,
    /// The select is open.
    Open,
}
```

### 1.2 Events

```rust
/// The events of the Select state machine.
#[derive(Clone, Debug)]
pub enum Event {
    /// Open the dropdown.
    Open,
    /// Close the dropdown.
    Close,
    /// Toggle open/closed.
    Toggle,
    /// Select an item by key.
    SelectItem(Key),
    /// Deselect an item (multi-select only).
    DeselectItem(Key),
    /// Highlight an item (keyboard/pointer).
    HighlightItem(Option<Key>),
    /// Highlight the first enabled item.
    HighlightFirst,
    /// Highlight the last enabled item.
    HighlightLast,
    /// Highlight the next enabled item.
    HighlightNext,
    /// Highlight the previous enabled item.
    HighlightPrev,
    /// Typeahead search — char + timestamp for typeahead::State.
    /// Suppressed when `ctx.is_composing` is true (IME active).
    TypeaheadSearch(char, u64),
    /// IME composition started (CJK, etc.).
    CompositionStart,
    /// IME composition ended.
    CompositionEnd,
    /// Focus received on trigger.
    Focus {
        /// Whether the focus is from a keyboard event.
        is_keyboard: bool,
    },
    /// Focus lost from the entire component.
    Blur,
    /// Click occurred outside the component.
    ClickOutside,
    /// Clear all selected values.
    Clear,
    /// Clear the typeahead search buffer (fired by timeout).
    ClearTypeahead,
    /// Replace the item collection dynamically.
    UpdateItems(StaticCollection<Item>),
}
```

#### 1.2.1 SetItems Event and Highlight Validation

The `Event::UpdateItems(StaticCollection<Item>)` event MUST be emitted by the adapter whenever the items collection changes (e.g., due to async loading, prop changes, or filtering). This event triggers validation of stale references in the machine context.

**Transition Handler for UpdateItems**:

```rust,no_check
Event::UpdateItems(new_items) => {
    // 1. Update the items collection
    ctx.items = new_items;

    // 2. Validate highlighted_key: reset if no longer present in items
    if let Some(ref key) = ctx.highlighted_key {
        if !ctx.items.iter().any(|item| item.key() == key) {
            ctx.highlighted_key = None; // Stale key, reset
        }
    }

    // 3. Validate selected keys: retain only keys present in new items
    let mut sel = ctx.selection.get().clone();
    sel.retain(|k| ctx.items.iter().any(|item| item.key() == k));
    ctx.selection.set(sel);
    // Note: for controlled selection, emit on_change instead of clearing

    Some(TransitionPlan::context_only(|_ctx| {}))
}
```

**Doc Comment**: "UpdateItems replaces the entire items collection. Any `highlighted_key` or `selected` keys that are not present in the new items are automatically invalidated. This prevents stale references when items are loaded asynchronously or filtered externally."

**Adapter Obligation**: Adapters MUST emit `Event::UpdateItems(...)` whenever the items prop changes. Simply updating the items collection without going through the state machine bypasses highlight/selection validation and leaves stale keys in `Context`.

> **Anchor Key Invalidation**: On `UpdateItems`, if `anchor_key` references a key that has become disabled (present in `disabled_keys`), `anchor_key` must be reset to `None`. The `range_select` operation must validate that the anchor is enabled before initiating range selection — if the anchor is `None` or disabled, range selection starts from the focused item instead.

### 1.3 Context

```rust
/// The context of the Select state machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The resolved locale for message formatting.
    pub locale: Locale,
    /// The item collection. Sections, headers, and separators are structural nodes.
    pub items: StaticCollection<Item>,
    /// Controlled/uncontrolled selection binding.
    pub selection: Bindable<selection::Set>,
    /// Full selection state: mode, behavior, anchor, disabled_keys.
    pub selection_state: selection::State,
    /// Currently highlighted (keyboard-focused) item key.
    pub highlighted_key: Option<Key>,
    /// Type-ahead search state.
    pub typeahead: typeahead::State,
    /// Whether the dropdown is open.
    // Design note: ctx.open duplicates State::Open/Closed. This enables Bindable
    // controlled/uncontrolled pattern but introduces a sync obligation. All
    // transitions that change state MUST also update ctx.open to match.
    // Post-transition assertion: after every transition, the adapter MUST verify
    // `ctx.open == matches!(state, State::Open | State::OpenFocused)`.
    // Debug builds should assert this invariant.
    pub open: bool,
    /// Whether the select is disabled.
    pub disabled: bool,
    /// Whether the select is readonly.
    pub readonly: bool,
    /// Whether the select is required.
    pub required: bool,
    /// Whether the select is invalid.
    pub invalid: bool,
    /// Whether multiple items can be selected.
    pub multiple: bool,
    /// Whether the select is focused.
    pub focused: bool,
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// The name of the select.
    pub name: Option<String>,
    /// Whether keyboard focus wraps around from the last item back to the first (and vice versa).
    /// When `true`, pressing ArrowDown on the last item highlights the first item, and pressing
    /// ArrowUp on the first item highlights the last item. Disable when reaching the end of the
    /// list carries semantic meaning (e.g., search results where the boundary signals "no more
    /// matches").
    pub loop_focus: bool,
    /// True while an IME composition session is active. Suppresses typeahead during composition.
    pub is_composing: bool,
    /// Timer ID for the current typeahead timeout. Used to cancel the previous timeout
    /// before setting a new one, preventing stacked timeouts from clearing the buffer prematurely.
    pub typeahead_timer_id: Option<TimerId>,
    /// Whether a description part is rendered (used for aria-describedby wiring).
    pub has_description: bool,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}
```

### 1.4 Props

```rust
/// The properties of the Select component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the select.
    pub id: String,
    /// The value of the select.
    pub value: Option<selection::Set>,
    /// The default value of the select.
    pub default_value: selection::Set,
    /// Whether the select allows multiple selections.
    pub multiple: bool,
    /// The mode of the select.
    pub selection_mode: selection::Mode,
    /// The behavior of the select.
    pub selection_behavior: selection::Behavior,
    /// The disabled behavior of the select.
    pub disabled_behavior: DisabledBehavior,
    /// Whether the select is disabled.
    pub disabled: bool,
    /// Whether the select is readonly.
    pub readonly: bool,
    /// Whether the select is required.
    pub required: bool,
    /// Whether the select is invalid.
    pub invalid: bool,
    /// Keys of items that should be disabled.
    pub disabled_keys: BTreeSet<Key>,
    /// The name of the select.
    pub name: Option<String>,
    /// The ID of the form element the component is associated with.
    pub form: Option<String>,
    /// The placeholder of the select.
    pub placeholder: Option<String>,
    /// Whether the select closes on select.
    pub close_on_select: Option<bool>,
    /// Whether the select loops focus.
    pub loop_focus: bool,
    /// The positioning options of the select.
    pub positioning: PositioningOptions,
    /// The autocomplete of the select.
    pub autocomplete: Option<String>,
    /// Prevents deselecting the last remaining selected item. When `true` and the user
    /// attempts to deselect the only selected value, the action is a no-op, ensuring at
    /// least one item is always selected. Commonly used for required single-select fields.
    pub disallow_empty_selection: bool,
    /// Enables a multi-line trigger layout where selected values are displayed as
    /// chips/tags that wrap onto additional lines instead of being truncated or
    /// collapsed into a "+N" badge. Only meaningful when `multiple` is `true`.
    pub multi_line_trigger: bool,
    /// Callback invoked when the dropdown open state changes.
    /// Fires during Open/Close state transitions with the new open state value (`true` for
    /// opening, `false` for closing). Invoked after the transition completes.
    /// Default: `None`.
    pub on_open_change: Option<Callback<bool>>,
    /// Enable virtualized rendering for large option lists.
    /// When `true`, only visible options are rendered in the DOM, with off-screen
    /// items replaced by spacer elements to maintain scroll position.
    /// Requires the adapter to implement virtual scrolling (e.g., via `VirtualList`).
    pub virtualized: bool,
    // Change callbacks provided by the adapter layer
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: selection::Set::Empty,
            multiple: false,
            selection_mode: selection::Mode::Single,
            selection_behavior: selection::Behavior::Toggle,
            disabled_behavior: DisabledBehavior::default(),
            disabled: false,
            readonly: false,
            required: false,
            invalid: false,
            disabled_keys: BTreeSet::new(),
            name: None,
            form: None,
            placeholder: None,
            close_on_select: None,
            loop_focus: true,
            positioning: PositioningOptions::default(),
            autocomplete: None,
            disallow_empty_selection: false,
            multi_line_trigger: false,
            on_open_change: None,
            virtualized: false,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
/// Machine for the Select component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;
    type Messages = Messages;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let state = State::Closed;
        let locale = env.locale.clone();
        let messages = messages.clone();
        let ctx = Context {
            locale,
            items: StaticCollection::default(),
            selection: match &props.value {
                Some(v) => Bindable::controlled(v.clone()),
                None => Bindable::uncontrolled(props.default_value.clone()),
            },
            selection_state: selection::State::new(
                props.selection_mode,
                props.selection_behavior,
            ).with_disabled_behavior(props.disabled_behavior)
             .with_disabled(props.disabled_keys.clone()),
            highlighted_key: None,
            typeahead: typeahead::State::default(),
            open: false,
            disabled: props.disabled,
            readonly: props.readonly,
            required: props.required,
            invalid: props.invalid,
            multiple: props.multiple,
            focused: false,
            focus_visible: false,
            name: props.name.clone(),
            loop_focus: props.loop_focus,
            is_composing: false,
            typeahead_timer_id: None,
            has_description: false,
            ids: ComponentIds::from_id(&props.id),
            messages,
        };

        (state, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled {
            match event {
                Event::Open | Event::Toggle | Event::SelectItem(_)
                | Event::DeselectItem(_) | Event::Clear
                // Guard highlight events in disabled state: a disabled Select
                // MUST NOT change the highlighted item via keyboard input.
                | Event::HighlightFirst | Event::HighlightLast
                | Event::HighlightNext | Event::HighlightPrev
                | Event::HighlightItem(_) => return None,
                _ => {}
            }
        }

        if ctx.readonly {
            match event {
                Event::Open | Event::SelectItem(_)
                | Event::DeselectItem(_) | Event::Clear
                // Guard highlight events in readonly state: a readonly Select
                // MUST NOT change the highlighted item. The user can still open
                // the dropdown to view options but cannot change selection or
                // highlight position.
                | Event::HighlightFirst | Event::HighlightLast
                | Event::HighlightNext | Event::HighlightPrev
                | Event::HighlightItem(_) => return None,
                _ => {}
            }
        }

        match (state, event) {
            // ── Open / Close / Toggle ───────────────────────────────
            // Note: Adapter MUST schedule focus management (e.g., focusing the listbox) as a
            // post-transition effect, not inline during transition. The `ctx.open` flag change
            // is consumed by the adapter's reactive system to trigger DOM updates.
            (State::Closed, Event::Open) => {
                // Highlight first selected item, or first enabled item
                let first_selected = match ctx.selection.get() {
                    selection::Set::Multiple(keys) => keys.iter().next().cloned(),
                    _ => None,
                };
                let first_item = first_enabled_key(
                    &ctx.items,
                    &ctx.selection_state.disabled_keys,
                    ctx.selection_state.disabled_behavior,
                );
                let highlight = first_selected.or(first_item);
                Some(TransitionPlan::to(State::Open).apply(move |ctx| {
                    ctx.open = true;
                    ctx.highlighted_key = highlight;
                }).with_effect(PendingEffect::new("focus_content", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    let content_id = ctx.ids.part("content");
                    platform.focus_element_by_id(&content_id);
                    no_cleanup()
                })))
            }

            (State::Open, Event::Close) => {
                Some(TransitionPlan::to(State::Closed).apply(|ctx| {
                    ctx.open = false;
                    ctx.highlighted_key = None;
                    ctx.typeahead = typeahead::State::default();
                }).with_effect(PendingEffect::new("focus_trigger", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    let trigger_id = ctx.ids.part("trigger");
                    platform.focus_element_by_id(&trigger_id);
                    no_cleanup()
                })))
            }

            // Note: The Service engine processes `then_send` events iteratively via a queue,
            // not recursively via the call stack. This prevents stack overflow when transitions
            // chain multiple events. The Toggle → Open/Close delegation below uses direct
            // Self::transition() which is a single-level delegation, not a then_send chain.
            (_, Event::Toggle) => {
                if ctx.open {
                    Self::transition(state, &Event::Close, ctx, props)
                } else {
                    Self::transition(state, &Event::Open, ctx, props)
                }
            }

            // ── SelectItem ──────────────────────────────────────────
            (State::Open, Event::SelectItem(key)) => {
                if ctx.selection_state.is_disabled(key) { return None; }

                let key = key.clone();
                let is_multiple = ctx.multiple;

                if is_multiple {
                    let new_sel = ctx.selection_state.toggle(key, &ctx.items);
                    let new_selection = new_sel.selected_keys.clone();
                    let anchor = new_sel.anchor_key.clone();
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.selection.set(new_selection);
                        ctx.selection_state.selected_keys = ctx.selection.get().clone();
                        ctx.selection_state.anchor_key = anchor;
                    }))
                } else {
                    let new_sel = ctx.selection_state.select(key);
                    let new_selection = new_sel.selected_keys.clone();
                    Some(TransitionPlan::to(State::Closed).apply(move |ctx| {
                        ctx.selection.set(new_selection);
                        ctx.selection_state.selected_keys = ctx.selection.get().clone();
                        ctx.open = false;
                        ctx.highlighted_key = None;
                        ctx.typeahead = typeahead::State::default();
                    }).with_effect(PendingEffect::new("focus_trigger", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        let trigger_id = ctx.ids.part("trigger");
                        platform.focus_element_by_id(&trigger_id);
                        no_cleanup()
                    })))
                }
            }

            // ── DeselectItem (multi-select) ─────────────────────────
            (_, Event::DeselectItem(key)) => {
                if !ctx.multiple { return None; }
                let new_sel = ctx.selection_state.deselect(key);
                let new_selection = new_sel.selected_keys.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.selection.set(new_selection);
                    ctx.selection_state.selected_keys = ctx.selection.get().clone();
                }))
            }

            // ── Clear ───────────────────────────────────────────────
            (_, Event::Clear) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.selection.set(selection::Set::Empty);
                    ctx.selection_state.selected_keys = selection::Set::Empty;
                    ctx.selection_state.anchor_key = None;
                }))
            }

            // ── Highlight navigation ────────────────────────────────
            (State::Open, Event::HighlightItem(key)) => {
                let key = key.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Open, Event::HighlightFirst) => {
                let first = first_enabled_key(
                    &ctx.items,
                    &ctx.selection_state.disabled_keys,
                    ctx.selection_state.disabled_behavior,
                );
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.highlighted_key = first;
                }))
            }

            (State::Open, Event::HighlightLast) => {
                let last = last_enabled_key(
                    &ctx.items,
                    &ctx.selection_state.disabled_keys,
                    ctx.selection_state.disabled_behavior,
                );
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.highlighted_key = last;
                }))
            }

            (State::Open, Event::HighlightNext) => {
                let next = match &ctx.highlighted_key {
                    Some(k) => next_enabled_key(
                        &ctx.items, k,
                        &ctx.selection_state.disabled_keys,
                        ctx.selection_state.disabled_behavior,
                        ctx.loop_focus,
                    ),
                    None => first_enabled_key(
                        &ctx.items,
                        &ctx.selection_state.disabled_keys,
                        ctx.selection_state.disabled_behavior,
                    ),
                };
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.highlighted_key = next;
                }))
            }

            (State::Open, Event::HighlightPrev) => {
                let prev = match &ctx.highlighted_key {
                    Some(k) => prev_enabled_key(
                        &ctx.items, k,
                        &ctx.selection_state.disabled_keys,
                        ctx.selection_state.disabled_behavior,
                        ctx.loop_focus,
                    ),
                    None => last_enabled_key(
                        &ctx.items,
                        &ctx.selection_state.disabled_keys,
                        ctx.selection_state.disabled_behavior,
                    ),
                };
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.highlighted_key = prev;
                }))
            }

            // ── Typeahead ───────────────────────────────────────────
            (_, Event::TypeaheadSearch(ch, now_ms)) if !ctx.is_composing => {
                let (new_ta, found) = ctx.typeahead.process_char(
                    *ch, *now_ms,
                    ctx.highlighted_key.as_ref(),
                    &ctx.items,
                );
                let should_open = !ctx.open;
                let highlight = found.or_else(|| ctx.highlighted_key.clone());
                if should_open {
                    Some(TransitionPlan::to(State::Open).apply(move |ctx| {
                        ctx.open = true;
                        ctx.typeahead = new_ta;
                        ctx.highlighted_key = highlight;
                    }).cancel_effect("typeahead_timeout")
                      .with_named_effect("typeahead_timeout", |_ctx, _props, send| {
                        let platform = use_platform_effects();
                        let send = send.clone();
                        let handle = platform.set_timeout(TYPEAHEAD_TIMEOUT_MS, Box::new(move || {
                            send.upgrade_and_send(Event::ClearTypeahead);
                        }));
                        let pc = platform.clone();
                        Box::new(move || { pc.clear_timeout(handle); })
                    }))
                } else {
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.typeahead = new_ta;
                        ctx.highlighted_key = highlight;
                    }).cancel_effect("typeahead_timeout")
                      .with_named_effect("typeahead_timeout", |_ctx, _props, send| {
                        let platform = use_platform_effects();
                        let send = send.clone();
                        let handle = platform.set_timeout(TYPEAHEAD_TIMEOUT_MS, Box::new(move || {
                            send.upgrade_and_send(Event::ClearTypeahead);
                        }));
                        let pc = platform.clone();
                        Box::new(move || { pc.clear_timeout(handle); })
                    }))
                }
            }

            // ── Focus / Blur ────────────────────────────────────────
            (_, Event::Focus { is_keyboard }) => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = is_kb;
                }))
            }

            (_, Event::Blur) => {
                let was_open = ctx.open;
                if was_open {
                    Some(TransitionPlan::to(State::Closed).apply(|ctx| {
                        ctx.focused = false;
                        ctx.focus_visible = false;
                        ctx.open = false;
                        ctx.highlighted_key = None;
                        ctx.typeahead = typeahead::State::default();
                    }))
                } else {
                    Some(TransitionPlan::context_only(|ctx| {
                        ctx.focused = false;
                        ctx.focus_visible = false;
                    }))
                }
            }

            // ── ClickOutside ────────────────────────────────────────
            (State::Open, Event::ClickOutside) => {
                Some(TransitionPlan::to(State::Closed).apply(|ctx| {
                    ctx.open = false;
                    ctx.highlighted_key = None;
                    ctx.typeahead = typeahead::State::default();
                }))
            }

            // ── UpdateItems ──────────────────────────────────────────
            (_, Event::UpdateItems(items)) => {
                let items = items.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.items = items;

                    // Validate highlighted_key: reset if no longer present in items
                    if let Some(ref key) = ctx.highlighted_key {
                        if !ctx.items.iter().any(|item| item.key() == key) {
                            ctx.highlighted_key = None;
                        }
                    }

                    // Validate selected keys: retain only keys present in new items
                    let mut sel = ctx.selection.get().clone();
                    sel.retain(|k| ctx.items.iter().any(|item| item.key() == k));
                    ctx.selection.set(sel);

                    // Validate anchor_key: if it points to a removed item, reset to
                    // highlighted_key (or None if that is also invalid).
                    // See invariant in 06-collections.md §L1826.
                    if let Some(ref key) = ctx.selection_state.anchor_key {
                        if !ctx.items.iter().any(|item| item.key() == key) {
                            ctx.selection_state.anchor_key = ctx.highlighted_key.clone()
                                .filter(|hk| ctx.items.iter().any(|item| item.key() == hk));
                        }
                    }
                }))
            }

            // ── ClearTypeahead (fired by timeout) ───────────────────
            (_, Event::ClearTypeahead) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.typeahead = typeahead::State::default();
                    ctx.typeahead_timer_id = None;
                }))
            }

            // ── IME Composition ─────────────────────────────────────
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

/// The API for the Select component.
pub struct Api<'a> {
    /// The state of the Select component.
    state: &'a State,
    /// The context of the Select component.
    ctx: &'a Context,
    /// The properties of the Select component.
    props: &'a Props,
    /// The send function for the Select component.
    send: &'a dyn Fn(Event),
}
impl<'a> Api<'a> {
    /// The handler for the trigger click event.
    pub fn on_trigger_click(&self) { (self.send)(Event::Toggle); }

    /// The handler for the trigger focus event.
    pub fn on_trigger_focus(&self, is_keyboard: bool) { (self.send)(Event::Focus { is_keyboard }); }

    /// The handler for the trigger blur event.
    pub fn on_trigger_blur(&self) { (self.send)(Event::Blur); }

    /// The handler for the item click event.
    pub fn on_item_click(&self, key: Key) { (self.send)(Event::SelectItem(key)); }

    /// The handler for the item hover event.
    pub fn on_item_hover(&self, key: Key) { (self.send)(Event::HighlightItem(Some(key))); }

    /// The handler for the item leave event.
    pub fn on_item_leave(&self) { (self.send)(Event::HighlightItem(None)); }

    /// The handler for the clear click event.
    pub fn on_clear_click(&self) { (self.send)(Event::Clear); }

    /// The display text for the currently selected item (single-select convenience).
    pub fn selected_text(&self) -> Option<&str> {
        match self.ctx.selection.get() {
            selection::Set::Multiple(keys) => {
                keys.iter().next().and_then(|k| self.ctx.items.text_value_of(k))
            }
            _ => None,
        }
    }

    /// Iterate all nodes for rendering (items, sections, headers, separators).
    pub fn items(&self) -> impl Iterator<Item = &Node<Item>> {
        self.ctx.items.nodes()
    }
}
```

### 1.6 Connect / API

The `Api` struct (defined above in the `Machine` block) provides `*_attrs()` methods
returning `AttrMap` values with data attributes and ARIA attributes.

```rust
#[derive(ComponentPart)]
#[scope = "select"]
pub enum Part {
    Root,
    Label,
    Control,
    Trigger,
    ValueText,
    Indicator,
    ClearTrigger,
    Positioner,
    Content,
    ItemGroup { key: Key },
    ItemGroupLabel { key: Key },
    Item { key: Key },
    ItemText { key: Key },
    ItemIndicator { key: Key },
    HiddenInput,
    Description,
    ErrorMessage,
}

impl<'a> Api<'a> {
    /// The attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if self.ctx.open { "open" } else { "closed" });
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.invalid { attrs.set_bool(HtmlAttr::Data("ars-invalid"), true); }
        attrs
    }

    /// The attributes for the label element.
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("label"));
        attrs.set(HtmlAttr::For, self.ctx.ids.part("trigger"));
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

    /// The attributes for the trigger element.
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("trigger"));
        attrs.set(HtmlAttr::Role, "combobox");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.ctx.open { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "listbox");
        attrs.set(HtmlAttr::Aria(AriaAttr::AutoComplete), "none");
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), self.ctx.ids.part("content"));
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));
        if let Some(k) = &self.ctx.highlighted_key {
            attrs.set(HtmlAttr::Aria(AriaAttr::ActiveDescendant), self.ctx.ids.item("item", k));
        }
        // For multi-select, announce the number of selected items.
        let count = self.ctx.selection.get().len();
        if count > 0 && self.ctx.selection_mode == selection::Mode::Multiple {
            attrs.set(HtmlAttr::Aria(AriaAttr::Description),
                (self.ctx.messages.selected_count)(count, &self.ctx.locale));
        }
        if self.ctx.required { attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true"); }
        if self.ctx.invalid { attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true"); }
        if self.ctx.disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        if self.ctx.readonly { attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true"); }
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
        attrs.set(HtmlAttr::TabIndex, if self.ctx.disabled { "-1" } else { "0" });
        if self.ctx.focus_visible { attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true); }
        attrs
    }

    // NOTE: `role="combobox"` follows the APG "select-only combobox" pattern for
    // non-editable single-select triggers. React Aria uses plain `role="button"`
    // with `aria-haspopup="listbox"` instead. Both patterns are valid APG.
    // We chose combobox because it provides `aria-activedescendant` support,
    // giving screen readers real-time feedback as the user arrows through options.

    // Handler methods on_trigger_click, on_trigger_focus, on_trigger_blur
    // are defined in the Machine block above.

    /// The handler for the trigger keydown event.
    pub fn on_trigger_keydown(&self, data: &KeyboardEventData, ctrl: bool, meta: bool) {
        match data.key {
            KeyboardKey::ArrowDown | KeyboardKey::ArrowUp => (self.send)(Event::Open),
            KeyboardKey::Enter | KeyboardKey::Space => (self.send)(Event::Toggle),
            KeyboardKey::Home => (self.send)(Event::HighlightFirst),
            KeyboardKey::End => (self.send)(Event::HighlightLast),
            KeyboardKey::Escape => { if self.ctx.open { (self.send)(Event::Close); } }
            _ if let Some(ch) = data.character && !ctrl && !meta => {
                // Timestamp obtained from the adapter's clock
                (self.send)(Event::TypeaheadSearch(ch, 0));
            }
            _ => {}
        }
    }

    /// The attributes for the value text element.
    pub fn value_text_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ValueText.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if self.ctx.selection.get().is_empty() {
            attrs.set_bool(HtmlAttr::Data("ars-placeholder"), true);
        }
        attrs
    }

    /// Returns the placeholder text to display when no option is selected.
    /// The adapter renders this inside the `ValueText` part when `data-ars-placeholder` is set.
    pub fn placeholder_text(&self) -> String {
        (self.ctx.messages.placeholder)(&self.ctx.locale)
    }

    /// Returns the accessible label for the trigger button.
    /// Used as a fallback `aria-label` when no explicit `Label` part is rendered.
    pub fn trigger_label(&self) -> String {
        (self.ctx.messages.trigger_label)(&self.ctx.locale)
    }

    /// The attributes for the indicator element.
    pub fn indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Indicator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs.set(HtmlAttr::Data("ars-state"), if self.ctx.open { "open" } else { "closed" });
        attrs
    }

    /// The attributes for the clear trigger element.
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ClearTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.clear_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs
    }

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

    /// Returns the empty-state text when no options are available.
    /// The adapter renders this inside the `Content` part when the item collection is empty.
    pub fn empty_text(&self) -> String {
        (self.ctx.messages.empty)(&self.ctx.locale)
    }

    /// The handler for the content keydown event.
    pub fn on_content_keydown(&self, data: &KeyboardEventData, ctrl: bool, meta: bool) {
        match data.key {
            KeyboardKey::ArrowDown => (self.send)(Event::HighlightNext),
            KeyboardKey::ArrowUp => (self.send)(Event::HighlightPrev),
            KeyboardKey::Home => (self.send)(Event::HighlightFirst),
            KeyboardKey::End => (self.send)(Event::HighlightLast),
            KeyboardKey::Enter | KeyboardKey::Space => {
                if let Some(ref k) = self.ctx.highlighted_key {
                    (self.send)(Event::SelectItem(k.clone()));
                }
            }
            KeyboardKey::Escape => (self.send)(Event::Close),
            _ if let Some(ch) = data.character && !ctrl && !meta => {
                // Timestamp obtained from the adapter's clock
                (self.send)(Event::TypeaheadSearch(ch, 0));
            }
            _ => {}
        }
    }

    /// The attributes for the hidden input element.
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "hidden");
        attrs.set(HtmlAttr::Name, self.ctx.name.as_deref().unwrap_or(""));
        // Serialize selected keys as comma-separated string values
        let value_str = match self.ctx.selection.get() {
            selection::Set::Multiple(keys) => keys.iter().map(|k| k.to_string()).collect::<Vec<_>>().join(","),
            selection::Set::All => "all".to_string(),
            selection::Set::Empty => String::new(),
        };
        attrs.set(HtmlAttr::Value, value_str);
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
        if self.ctx.required { attrs.set_bool(HtmlAttr::Required, true); }
        if let Some(ac) = &self.props.autocomplete {
            attrs.set(HtmlAttr::AutoComplete, ac.as_str());
        }
        if let Some(ref form) = self.props.form {
            attrs.set(HtmlAttr::Form, form);
        }
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

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

    /// Build attrs for a single item within the content. Call per item.
    pub fn item_attrs(&self, key: &Key) -> AttrMap {
        let node = self.ctx.items.get(key);
        let item_id = self.ctx.ids.item("item", key);
        let is_selected = self.ctx.selection.get().contains(key);
        let is_highlighted = self.ctx.highlighted_key.as_ref() == Some(key);
        let is_disabled = self.ctx.disabled || self.ctx.selection_state.is_disabled(key);

        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Item { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, item_id);
        attrs.set(HtmlAttr::Role, "option");
        attrs.set(HtmlAttr::Aria(AriaAttr::Selected), if is_selected { "true" } else { "false" });
        if is_disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        attrs.set(HtmlAttr::Data("ars-state"), if is_selected { "selected" } else { "unselected" });
        if is_highlighted { attrs.set_bool(HtmlAttr::Data("ars-highlighted"), true); }
        if is_disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if let Some(n) = node { attrs.set(HtmlAttr::Data("ars-value"), &n.text_value); }
        attrs
    }

    // on_item_click, on_item_hover, on_item_leave are defined in the Machine block above.

    /// The attributes for the item text element.
    pub fn item_text_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemText { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.item_part("item", key, "text"));
        attrs
    }

    /// The attributes for the item indicator element.
    pub fn item_indicator_attrs(&self, key: &Key) -> AttrMap {
        let is_selected = self.ctx.selection.get().contains(key);
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ItemIndicator { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs.set(HtmlAttr::Data("ars-state"), if is_selected { "selected" } else { "unselected" });
        attrs
    }

    /// The attributes for the item group element.
    pub fn item_group_attrs(&self, section_key: &Key) -> AttrMap {
        let group_label_id = self.ctx.ids.item_part("group", section_key, "label");
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ItemGroup { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "group");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), group_label_id);
        attrs
    }

    /// The attributes for the item group label element.
    pub fn item_group_label_attrs(&self, section_key: &Key) -> AttrMap {
        let group_label_id = self.ctx.ids.item_part("group", section_key, "label");
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ItemGroupLabel { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, group_label_id);
        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Control => self.control_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::ValueText => self.value_text_attrs(),
            Part::Indicator => self.indicator_attrs(),
            Part::ClearTrigger => self.clear_trigger_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::ItemGroup { ref key } => self.item_group_attrs(key),
            Part::ItemGroupLabel { ref key } => self.item_group_label_attrs(key),
            Part::Item { ref key } => self.item_attrs(key),
            Part::ItemText { ref key } => self.item_text_attrs(key),
            Part::ItemIndicator { ref key } => self.item_indicator_attrs(key),
            Part::HiddenInput => self.hidden_input_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}
```

## 2. Anatomy

| Part             | Selector                                                      | Element                                                   | Notes                 |
| ---------------- | ------------------------------------------------------------- | --------------------------------------------------------- | --------------------- |
| `Root`           | `[data-ars-scope="select"][data-ars-part="root"]`             | `<div>`                                                   | Container             |
| `Label`          | `[data-ars-scope="select"][data-ars-part="label"]`            | `<label>`                                                 | Text label            |
| `Control`        | `[data-ars-scope="select"][data-ars-part="control"]`          | `<div>`                                                   | Wraps trigger + clear |
| `Trigger`        | `[data-ars-scope="select"][data-ars-part="trigger"]`          | `<button>`                                                | Opens dropdown        |
| `ValueText`      | `[data-ars-scope="select"][data-ars-part="value-text"]`       | `<span>`                                                  | Displays selection    |
| `Indicator`      | `[data-ars-scope="select"][data-ars-part="indicator"]`        | `<div>`                                                   | Arrow icon            |
| `ClearTrigger`   | `[data-ars-scope="select"][data-ars-part="clear-trigger"]`    | `<button>`                                                | Clears selection      |
| `Positioner`     | `[data-ars-scope="select"][data-ars-part="positioner"]`       | `<div>`                                                   | Positions content     |
| `Content`        | `[data-ars-scope="select"][data-ars-part="content"]`          | `<div>`                                                   | Dropdown listbox      |
| `ItemGroup`      | `[data-ars-scope="select"][data-ars-part="item-group"]`       | `<div>`                                                   | Group container       |
| `ItemGroupLabel` | `[data-ars-scope="select"][data-ars-part="item-group-label"]` | `<div>`                                                   | Group heading         |
| `Item`           | `[data-ars-scope="select"][data-ars-part="item"]`             | `<div>`                                                   | Option item           |
| `ItemText`       | `[data-ars-scope="select"][data-ars-part="item-text"]`        | `<span>`                                                  | Item label            |
| `ItemIndicator`  | `[data-ars-scope="select"][data-ars-part="item-indicator"]`   | `<div>`                                                   | Check mark            |
| `HiddenInput`    | `[data-ars-scope="select"][data-ars-part="hidden-input"]`     | `<input>`                                                 | Form value            |
| `Description`    | `[data-ars-scope="select"][data-ars-part="description"]`      | `<div>`                                                   | Help text             |
| `ErrorMessage`   | `[data-ars-scope="select"][data-ars-part="error-message"]`    | `<div>`                                                   | Error text            |
| **EmptyState**   | `<div>`                                                       | Message displayed when the listbox has no matching items. |                       |

## 3. Accessibility

| Property                | Element    | Value                                                                                                                                                                                               |
| ----------------------- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------- |
| `role`                  | Trigger    | `combobox`                                                                                                                                                                                          |
| `aria-expanded`         | Trigger    | `true` when open                                                                                                                                                                                    |
| `aria-haspopup`         | Trigger    | `listbox`                                                                                                                                                                                           |
| `aria-controls`         | Trigger    | Content id                                                                                                                                                                                          |
| `aria-activedescendant` | Trigger    | Highlighted item id (only set when a valid item is highlighted; **omit attribute entirely** when `highlighted_key` is `None` — setting it to an empty string or non-existent ID violates ARIA spec) |
| `aria-labelledby`       | Trigger    | Label ID                                                                                                                                                                                            | Points to the label element                               |
| `role`                  | Content    | `listbox`                                                                                                                                                                                           |
| `aria-multiselectable`  | Content    | Present for multi-select                                                                                                                                                                            |
| `role`                  | Item       | `option`                                                                                                                                                                                            |
| `aria-selected`         | Item       | `true` when selected                                                                                                                                                                                |
| `role`                  | ItemGroup  | `group`                                                                                                                                                                                             |
| `aria-labelledby`       | ItemGroup  | Group label id                                                                                                                                                                                      |
| `role`                  | EmptyState | `"status"`                                                                                                                                                                                          | Implicit `aria-live="polite"` and `aria-atomic="true"`    |
| `aria-atomic`           | EmptyState | `"true"`                                                                                                                                                                                            | Entire message announced as a unit (explicit for clarity) |

If a placeholder option exists (e.g., "Choose..."), it should have `aria-hidden="true"`
or be implemented as the trigger's label text rather than a selectable option.

> **Keyboard Interaction**

| Key           | Closed                                             | Open                               |
| ------------- | -------------------------------------------------- | ---------------------------------- |
| Enter / Space | Open dropdown                                      | Select highlighted item            |
| ArrowDown     | Open dropdown                                      | Highlight next item                |
| ArrowUp       | Open dropdown                                      | Highlight previous item            |
| Home          | —                                                  | Highlight first item               |
| End           | —                                                  | Highlight last item                |
| Escape        | —                                                  | Close dropdown                     |
| a-z           | Typeahead: open dropdown + highlight matching item | Typeahead: highlight matching item |
| Tab           | Move focus away                                    | Close + move focus                 |

> **Closed-state keyboard behavior.** When the dropdown is **closed**, `TypeaheadSearch` and
> `ArrowDown`/`ArrowUp` events **open the dropdown** as part of the transition (see state
> machine: `(State::Closed, Event::TypeaheadSearch(..))` transitions to `State::Open` with the
> matched item highlighted). This follows the `aria-activedescendant` pattern where the user
> can begin navigating without an explicit open gesture. `Home`, `End`, and `Escape` are
> no-ops in the closed state.
> **Type-ahead search behavior.** The `Select` component implements multi-character type-ahead
> with the following rules:
>
> - **Timeout**: 500ms between keystrokes before the search buffer is reset. Each new keystroke
>   restarts the timeout timer (see Appendix: Shared Selection Patterns §Typeahead).
> - **Multi-character prefix match**: Typing multiple characters within the timeout window
>   performs a prefix match against item text labels. For example, typing "ca" matches
>   "California" before "Colorado".
> - **Case-insensitive matching**: All comparisons use `ars_i18n::Collator` for locale-aware,
>   case-insensitive, accent-insensitive matching.
> - **On timeout**: The search buffer is cleared. The next keystroke starts a new search from
>   scratch. The currently highlighted item is NOT reset (see TypeaheadClear Semantics in the
>   Appendix).
> - **No match found**: If the typed prefix does not match any enabled item, the highlighted
>   selection does not change. The buffer continues accumulating until timeout.

## 4. Internationalization

- **Typeahead**: Character matching must be locale-aware — uses `Collator` from `ars-i18n`
  for case-insensitive, accent-insensitive comparison.
- **RTL**: Dropdown positioning flips horizontally. Arrow icon animates in mirrored direction.
- **Placeholder text**: User-provided, localized by consumer.
- **"Clear selection"** label: Localized via `Messages.clear_label`.

### 4.1 Messages

```rust
/// Messages for the Select component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Placeholder text shown when no option is selected.
    pub placeholder: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Empty state text (default: "No options available")
    pub empty: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Announcement for multi-select: how many options are selected.
    pub selected_count: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,
    /// Clear trigger label (default: "Clear selection")
    pub clear_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Accessible label for the dropdown trigger.
    pub trigger_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            placeholder: MessageFn::static_str("Select an option"),
            empty: MessageFn::static_str("No options available"),
            selected_count: MessageFn::new(|n, _locale| match n {
                1 => "1 option selected".into(),
                _ => format!("{} options selected", n),
            }),
            clear_label: MessageFn::static_str("Clear selection"),
            trigger_label: MessageFn::static_str("Open dropdown"),
        }
    }
}

impl ComponentMessages for Messages {}
```

The `clear_trigger_attrs()` method MUST use `&self.messages.clear_label` instead of the hardcoded string.

> **EmptyState i18n**: The `EmptyState` message text is sourced from `Messages.empty` (see `04-internationalization.md` §7.1). When the message includes embedded user input (e.g., "No results for '{query}'"), the interpolated query text must be wrapped in Unicode directional isolate characters (`U+2068`/`U+2069`) to prevent BiDi reordering in RTL layouts.
>
> **Async Loading**: When integrated with `AsyncCollection<T>` (see `06-collections.md` §5), Select gains `loading_state: LoadingState` (Idle | Loading | LoadingMore | Error) and fires `LoadMore` when the user scrolls near the end of the listbox. The adapter renders a loading indicator in the listbox footer and announces loading state changes via `aria-busy`.
>
> **Virtual Scroll Integration**: When the option collection is large (hundreds or thousands of items), the listbox SHOULD integrate with a Virtualizer (see `06-collections.md` §Virtualizer) to render only the visible items plus a small overscan buffer. The virtualizer provides `aria-setsize` and `aria-posinset` on each rendered option so screen readers can report the total list size even when most items are not in the DOM. The same applies to Combobox (§2) and Listbox (§3) when they display large collections.

## 5. Library Parity

> Compared against: Ark UI (`Select`), Radix UI (`Select`), React Aria (`Select`).

### 5.1 Props

| Feature                       | ars-ui                        | Ark UI                                  | Radix UI                                | React Aria                                | Notes                                                 |
| ----------------------------- | ----------------------------- | --------------------------------------- | --------------------------------------- | ----------------------------------------- | ----------------------------------------------------- |
| Controlled/uncontrolled value | `value` / `default_value`     | `value` / `defaultValue`                | `value` / `defaultValue`                | `selectedKey` / `defaultSelectedKey`      | ars-ui uses `selection::Set` for multi-select support |
| Open state control            | `on_open_change` callback     | `open` / `defaultOpen` / `onOpenChange` | `open` / `defaultOpen` / `onOpenChange` | `isOpen` / `defaultOpen` / `onOpenChange` | ars-ui uses callback-only; open state is internal     |
| Multiple selection            | `multiple` + `selection_mode` | `multiple`                              | --                                      | `selectionMode`                           | Radix is single-select only                           |
| Disabled                      | `disabled`                    | `disabled`                              | `disabled`                              | `isDisabled`                              | --                                                    |
| Read-only                     | `readonly`                    | `readOnly`                              | --                                      | --                                        | ars-ui and Ark UI only                                |
| Required                      | `required`                    | `required`                              | `required`                              | `isRequired`                              | --                                                    |
| Invalid                       | `invalid`                     | `invalid`                               | --                                      | `isInvalid`                               | --                                                    |
| Disabled keys                 | `disabled_keys`               | --                                      | per-item `disabled`                     | `disabledKeys`                            | Ark UI disables per-item in collection                |
| Placeholder                   | `placeholder`                 | via `ValueText.placeholder`             | via `Value.placeholder`                 | `placeholder`                             | --                                                    |
| Close on select               | `close_on_select`             | `closeOnSelect`                         | --                                      | --                                        | Radix/React Aria always close on single-select        |
| Loop focus                    | `loop_focus`                  | `loopFocus`                             | --                                      | `shouldFocusWrap`                         | Different naming                                      |
| Positioning                   | `positioning`                 | `positioning`                           | `Content.position` + `side`/`align`     | via `Popover`                             | --                                                    |
| Form name                     | `name`                        | `name`                                  | `name`                                  | `name`                                    | --                                                    |
| Form ID                       | `form`                        | `form`                                  | --                                      | `form`                                    | --                                                    |
| Autocomplete                  | `autocomplete`                | `autoComplete`                          | --                                      | `autoComplete`                            | --                                                    |
| Disallow empty selection      | `disallow_empty_selection`    | `deselectable` (inverse)                | --                                      | --                                        | Inverted boolean semantics                            |
| Virtualized                   | `virtualized`                 | `scrollToIndexFn`                       | --                                      | --                                        | Different approach to same need                       |
| Selection behavior            | `selection_behavior`          | --                                      | --                                      | --                                        | ars-ui exclusive (toggle vs replace)                  |
| Disabled behavior             | `disabled_behavior`           | --                                      | --                                      | --                                        | ars-ui exclusive (skip vs focus-only)                 |
| Multi-line trigger            | `multi_line_trigger`          | --                                      | --                                      | --                                        | ars-ui exclusive (chip/tag display)                   |
| Validate                      | --                            | --                                      | --                                      | `validate` / `validationBehavior`         | React Aria exclusive; form-level in ars-ui            |

**Gaps:** None. React Aria's `validate`/`validationBehavior` are form-level concerns handled outside the Select machine in ars-ui.

### 5.2 Anatomy

| Part             | ars-ui           | Ark UI           | Radix UI               | React Aria               | Notes                                        |
| ---------------- | ---------------- | ---------------- | ---------------------- | ------------------------ | -------------------------------------------- |
| Root             | `Root`           | `Root`           | `Root`                 | `Select`                 | --                                           |
| Label            | `Label`          | `Label`          | --                     | `Label`                  | Radix uses external label                    |
| Control          | `Control`        | `Control`        | --                     | --                       | Wrapper for trigger + clear                  |
| Trigger          | `Trigger`        | `Trigger`        | `Trigger`              | `Button`                 | --                                           |
| ValueText        | `ValueText`      | `ValueText`      | `Value`                | `SelectValue`            | --                                           |
| Indicator        | `Indicator`      | `Indicator`      | `Icon`                 | --                       | Arrow/chevron icon                           |
| ClearTrigger     | `ClearTrigger`   | `ClearTrigger`   | --                     | --                       | --                                           |
| Positioner       | `Positioner`     | `Positioner`     | `Portal`               | `Popover`                | --                                           |
| Content          | `Content`        | `Content`        | `Content` + `Viewport` | `ListBox`                | Radix splits content/viewport                |
| ItemGroup        | `ItemGroup`      | `ItemGroup`      | `Group`                | `ListBoxSection`         | --                                           |
| ItemGroupLabel   | `ItemGroupLabel` | `ItemGroupLabel` | `Label`                | `Header`                 | --                                           |
| Item             | `Item`           | `Item`           | `Item`                 | `ListBoxItem`            | --                                           |
| ItemText         | `ItemText`       | `ItemText`       | `ItemText`             | --                       | --                                           |
| ItemIndicator    | `ItemIndicator`  | `ItemIndicator`  | `ItemIndicator`        | --                       | --                                           |
| HiddenInput      | `HiddenInput`    | `HiddenSelect`   | --                     | --                       | Form submission                              |
| Description      | `Description`    | --               | --                     | `Text[slot=description]` | --                                           |
| ErrorMessage     | `ErrorMessage`   | --               | --                     | `FieldError`             | --                                           |
| Separator        | --               | --               | `Separator`            | --                       | ars-ui uses structural `NodeType::Separator` |
| ScrollUpButton   | --               | --               | `ScrollUpButton`       | --                       | Radix item-aligned mode                      |
| ScrollDownButton | --               | --               | `ScrollDownButton`     | --                       | Radix item-aligned mode                      |

**Gaps:** None. Radix's `ScrollUpButton`/`ScrollDownButton` and `Separator` are Radix-specific rendering patterns; ars-ui uses collection-level structural nodes for separators and CSS-based scrolling.

### 5.3 Events

| Callback         | ars-ui                         | Ark UI              | Radix UI        | React Aria                             | Notes                                    |
| ---------------- | ------------------------------ | ------------------- | --------------- | -------------------------------------- | ---------------------------------------- |
| Value change     | via `Bindable`                 | `onValueChange`     | `onValueChange` | `onSelectionChange`                    | --                                       |
| Open change      | `on_open_change`               | `onOpenChange`      | `onOpenChange`  | `onOpenChange`                         | --                                       |
| Highlight change | via `Context.highlighted_key`  | `onHighlightChange` | --              | --                                     | ars-ui exposes via context, not callback |
| Focus/blur       | `Event::Focus` / `Event::Blur` | --                  | --              | `onFocus` / `onBlur` / `onFocusChange` | --                                       |

**Gaps:** None.

### 5.4 Features

| Feature              | ars-ui                   | Ark UI                  | Radix UI          | React Aria |
| -------------------- | ------------------------ | ----------------------- | ----------------- | ---------- |
| Single select        | Yes                      | Yes                     | Yes               | Yes        |
| Multi select         | Yes                      | Yes                     | --                | Yes        |
| Typeahead            | Yes                      | Yes                     | Yes               | Yes        |
| Disabled items       | Yes                      | Yes                     | Yes               | Yes        |
| Item groups          | Yes                      | Yes                     | Yes               | Yes        |
| Keyboard navigation  | Yes                      | Yes                     | Yes               | Yes        |
| Positioning/floating | Yes                      | Yes                     | Yes (popper mode) | Yes        |
| Form integration     | Yes                      | Yes                     | Yes               | Yes        |
| Virtualization       | Yes (`virtualized` prop) | Yes (`scrollToIndexFn`) | --                | --         |
| RTL support          | Yes                      | Yes                     | Yes (`dir`)       | Yes        |
| IME composition      | Yes                      | --                      | --                | --         |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity -- no gaps identified.
- **Divergences:** (1) ars-ui uses `selection::Set` enum for multi-select values instead of `string[]`; (2) ars-ui uses `disallow_empty_selection` (positive) vs Ark UI's `deselectable` (negative); (3) ars-ui tracks open state internally with a callback rather than exposing controlled `open`/`defaultOpen` props; (4) Radix is single-select only.
- **Recommended additions:** None.
