---
component: Listbox
category: selection
tier: complex
foundation_deps: [architecture, accessibility, interactions, collections]
shared_deps: [selection-patterns]
related: []
references:
    ark-ui: Listbox
    react-aria: ListBox
---

# Listbox

A visible, always-rendered list for selection. Unlike `Select`, the list is not in a popup —
it is statically positioned in the layout.

Items are stored as a `StaticCollection<Item>` (from `06-collections.md`). All
navigation uses `Collection` trait methods with `next_enabled_key` / `prev_enabled_key`
for disabled-item awareness. Selection is managed via `selection::State` and typeahead via
`typeahead::State` — both from `ars-collections`.

## 1. State Machine

```rust
/// User-facing payload for `Listbox` items. The machine uses Node-level metadata
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
/// The state of the Listbox component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The listbox is idle.
    Idle,
    /// The listbox is focused.
    Focused,
}
```

### 1.2 Events

```rust
/// The events of the Listbox component.
#[derive(Clone, Debug)]
pub enum Event {
    /// The listbox received focus.
    Focus {
        /// Whether the focus is from a keyboard event.
        is_keyboard: bool,
    },
    /// The listbox lost focus.
    Blur,
    /// The listbox selected an item.
    SelectItem(Key),
    /// The listbox deselected an item.
    DeselectItem(Key),
    /// The listbox toggled an item.
    ToggleItem(Key),
    /// The listbox highlighted an item.
    HighlightItem(Option<Key>),
    /// The listbox highlighted the next item.
    HighlightNext,
    /// The listbox highlighted the previous item.
    HighlightPrev,
    /// The listbox highlighted the first item.
    HighlightFirst,
    /// The listbox highlighted the last item.
    HighlightLast,
    /// The listbox selected all items.
    SelectAll,
    /// The listbox deselected all items.
    DeselectAll,
    /// The listbox performed a typeahead search.
    TypeaheadSearch(char, u64),
    /// Replace the item collection dynamically.
    UpdateItems(StaticCollection<Item>),
    /// An item was activated (Enter/double-click) — distinct from selection.
    /// Fires `on_action` callback. Does not affect selection state.
    ItemActivated(Key),
}
```

### 1.3 Context

```rust
/// The context for the Listbox state machine.
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
    /// Whether the listbox is disabled.
    pub disabled: bool,
    /// Whether the listbox is required.
    pub required: bool,
    /// Whether the listbox is invalid.
    pub invalid: bool,
    /// Whether the listbox is focused.
    pub focused: bool,
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// The orientation of the listbox.
    pub orientation: Orientation,
    /// Whether the listbox loops focus.
    pub loop_focus: bool,
    /// True while an IME composition session is active (between CompositionStart
    /// and CompositionEnd). Typeahead is suppressed during composition.
    pub is_composing: bool,
    /// Whether a description part is rendered (used for aria-describedby wiring).
    pub has_description: bool,
    /// Whether a load-more request is currently in flight. Copied from props;
    /// drives `data-ars-loading` on the `LoadingSentinel` part.
    pub loading: bool,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}
```

> **Typeahead IME Handling**: When `is_composing == true`, typeahead character matching is
> skipped — the composing characters are intermediate and not final. The `is_composing` flag
> is set by `Event::CompositionStart` and cleared by `Event::CompositionEnd`. After
> `compositionend`, the typeahead buffer is reset to avoid stale partial matches.
> Cross-reference: `Combobox` IME handling at §2 Key Transitions (~line 1427).

### 1.4 Props

```rust
/// Props for the Listbox component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The ID of the listbox.
    pub id: String,
    /// The value of the listbox.
    pub value: Option<selection::Set>,
    /// The default value of the listbox.
    pub default_value: selection::Set,
    /// The selection mode of the listbox.
    pub selection_mode: selection::Mode,
    /// The selection behavior of the listbox.
    pub selection_behavior: selection::Behavior,
    /// The disabled behavior of the listbox.
    pub disabled_behavior: DisabledBehavior,
    /// Whether the listbox is disabled.
    pub disabled: bool,
    /// Whether the listbox is required.
    pub required: bool,
    /// Whether the listbox is invalid.
    pub invalid: bool,
    /// The orientation of the listbox.
    pub orientation: Orientation,
    /// Whether the listbox loops focus.
    pub loop_focus: bool,
    /// When true, prevents deselecting the last selected item
    /// (at least one option must remain selected). Default: false.
    pub disallow_empty_selection: bool,
    /// Keys of items that should be disabled.
    pub disabled_keys: BTreeSet<Key>,
    /// The name of the listbox.
    pub name: Option<String>,
    /// Callback invoked when an item is activated (Enter/click), distinct from
    /// `on_selection_change`. Use for action items that trigger commands rather
    /// than toggling selection state.
    pub on_action: Option<Callback<Key>>,
    /// Callback invoked when the loading sentinel enters the viewport,
    /// signalling that the next page of items should be loaded. When `None`,
    /// the `LoadingSentinel` part is not rendered. See `06-collections.md` §5.3.
    pub on_load_more: Option<Callback<()>>,
    /// Whether a load-more request is currently in flight. When `true`, the
    /// `LoadingSentinel` element receives `data-ars-loading="true"` so the
    /// adapter can display a spinner. Default: `false`.
    pub loading: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: selection::Set::Empty,
            selection_mode: selection::Mode::Single,
            selection_behavior: selection::Behavior::Toggle,
            disabled_behavior: DisabledBehavior::default(),
            disabled: false,
            required: false,
            invalid: false,
            orientation: Orientation::Vertical,
            loop_focus: true,
            disallow_empty_selection: false,
            disabled_keys: BTreeSet::new(),
            name: None,
            on_action: None,
            on_load_more: None,
            loading: false,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
use ars_collections::typeahead::TYPEAHEAD_TIMEOUT_MS;
use ars_core::{use_platform_effects, PendingEffect};

/// Machine for the Listbox component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;
    type Messages = Messages;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let state = State::Idle;
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
            disabled: props.disabled,
            required: props.required,
            invalid: props.invalid,
            focused: false,
            focus_visible: false,
            orientation: props.orientation,
            loop_focus: props.loop_focus,
            has_description: false,
            loading: props.loading,
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
                Event::SelectItem(_) | Event::DeselectItem(_)
                | Event::ToggleItem(_) | Event::SelectAll
                | Event::DeselectAll => return None,
                _ => {}
            }
        }

        match (state, event) {
            // ── Focus / Blur ────────────────────────────────────────
            (State::Idle, Event::Focus { is_keyboard }) => {
                let is_kb = *is_keyboard;
                let first = first_enabled_key(
                    &ctx.items,
                    &ctx.selection_state.disabled_keys,
                    ctx.selection_state.disabled_behavior,
                );

                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = is_kb;
                    if ctx.highlighted_key.is_none() {
                        ctx.highlighted_key = first;
                    }
                }))
            }

            (_, Event::Focus { is_keyboard }) => {
                let is_kb = *is_keyboard;

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = is_kb;
                }))
            }

            (State::Focused, Event::Blur) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                    ctx.typeahead = typeahead::State::default();
                }))
            }

            // ── Highlight navigation ────────────────────────────────
            (State::Focused, Event::HighlightItem(key)) => {
                let key = key.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Focused, Event::HighlightFirst) => {
                let first = first_enabled_key(
                    &ctx.items,
                    &ctx.selection_state.disabled_keys,
                    ctx.selection_state.disabled_behavior,
                );

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.highlighted_key = first;
                }))
            }

            (State::Focused, Event::HighlightLast) => {
                let last = last_enabled_key(
                    &ctx.items,
                    &ctx.selection_state.disabled_keys,
                    ctx.selection_state.disabled_behavior,
                );

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.highlighted_key = last;
                }))
            }

            (State::Focused, Event::HighlightNext) => {
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

            (State::Focused, Event::HighlightPrev) => {
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

            // ── Selection ───────────────────────────────────────────
            (_, Event::SelectItem(key)) => {
                if ctx.selection_state.is_disabled(key) { return None; }
                let new_sel = ctx.selection_state.select(key.clone());
                let new_selection = new_sel.selected_keys.clone();
                let anchor = new_sel.anchor_key.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.selection.set(new_selection);
                    ctx.selection_state.selected_keys = ctx.selection.get().clone();
                    ctx.selection_state.anchor_key = anchor;
                }))
            }

            (_, Event::DeselectItem(key)) => {
                let new_sel = ctx.selection_state.deselect(key);
                let new_selection = new_sel.selected_keys.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.selection.set(new_selection);
                    ctx.selection_state.selected_keys = ctx.selection.get().clone();
                }))
            }

            (_, Event::ToggleItem(key)) => {
                if ctx.selection_state.is_disabled(key) { return None; }
                let new_sel = ctx.selection_state.toggle(key.clone(), &ctx.items);
                let new_selection = new_sel.selected_keys.clone();
                let anchor = new_sel.anchor_key.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.selection.set(new_selection);
                    ctx.selection_state.selected_keys = ctx.selection.get().clone();
                    ctx.selection_state.anchor_key = anchor;
                }))
            }

            (_, Event::SelectAll) => {
                if ctx.selection_state.mode != selection::Mode::Multiple { return None; }
                let new_sel = ctx.selection_state.select_all();
                let new_selection = new_sel.selected_keys.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.selection.set(new_selection);
                    ctx.selection_state.selected_keys = ctx.selection.get().clone();
                }))
            }

            (_, Event::DeselectAll) => {
                let new_sel = ctx.selection_state.clear();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.selection.set(selection::Set::Empty);
                    ctx.selection_state = new_sel;
                }))
            }

            // ── Typeahead ───────────────────────────────────────────
            (State::Focused, Event::TypeaheadSearch(ch, now_ms)) => {
                let (new_ta, found) = ctx.typeahead.process_char(
                    *ch, *now_ms,
                    ctx.highlighted_key.as_ref(),
                    &ctx.items,
                );

                let highlight = found.or_else(|| ctx.highlighted_key.clone());

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.typeahead = new_ta;
                    ctx.highlighted_key = highlight;
                }).cancel_effect("typeahead_timeout")
                  .with_effect(PendingEffect::new("typeahead_timeout", |ctx, _props, send| {
                    let platform = use_platform_effects();
                    let send = send.clone();
                    let handle = platform.set_timeout(TYPEAHEAD_TIMEOUT_MS, Box::new(move || {
                        send(Event::TypeaheadClear);
                    }));
                    let pc = platform.clone();
                    Box::new(move || { pc.clear_timeout(handle); })
                })))
            }

            // ── Typeahead IME Handling ────────────────────────────────
            // When an IME composition is active (CJK input, etc.), typeahead
            // character matching MUST be skipped. The composed characters are
            // not final until compositionend fires.
            (State::Focused, Event::TypeaheadSearch(_, _)) if ctx.is_composing => {
                None // Skip typeahead during IME composition
            }

            // ── UpdateItems ─────────────────────────────────────────
            (_, Event::UpdateItems(items)) => {
                let items = items.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.items = items;
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

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "listbox"]
pub enum Part {
    Root,
    Label,
    Content,
    ItemGroup { key: Key },
    ItemGroupLabel { key: Key },
    Item { key: Key },
    ItemText { key: Key },
    ItemIndicator { key: Key },
    Description,
    LoadingSentinel,
}

/// API for the Listbox component.
pub struct Api<'a> {
    /// The state of the Listbox.
    state: &'a State,
    /// The context of the Listbox.
    ctx: &'a Context,
    /// The props of the Listbox.
    props: &'a Props,
    /// The send function to send events to the Listbox.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Text for the empty state when no options are available.
    /// The adapter renders this as visible text when the item list is empty.
    pub fn empty_label(&self) -> String {
        (self.ctx.messages.empty_label)(&self.ctx.locale)
    }

    /// Announcement for the number of available options.
    /// The adapter can announce this when the listbox receives focus or when
    /// the option count changes (e.g., after filtering).
    pub fn count_announcement(&self, count: usize) -> String {
        (self.ctx.messages.count_announcement)(count, &self.ctx.locale)
    }

    /// The attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
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
        attrs.set(HtmlAttr::TabIndex, if self.ctx.disabled { "-1" } else { "0" });
        if self.ctx.selection_state.mode == selection::Mode::Multiple {
            attrs.set(HtmlAttr::Aria(AriaAttr::MultiSelectable), "true");
        }
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), match self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical => "vertical",
        });
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));
        if let Some(k) = &self.ctx.highlighted_key {
            attrs.set(HtmlAttr::Aria(AriaAttr::ActiveDescendant), self.ctx.ids.item("item", k));
        }
        if self.ctx.required { attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true"); }
        if self.ctx.invalid { attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true"); }
        if self.ctx.disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        // Wire aria-describedby
        let mut describedby_parts: Vec<String> = Vec::new();
        if self.ctx.has_description {
            describedby_parts.push(self.ctx.ids.part("description"));
        }
        if self.ctx.invalid {
            describedby_parts.push(self.ctx.ids.part("error-message"));
        }
        if !describedby_parts.is_empty() {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), describedby_parts.join(" "));
        }
        if self.ctx.focus_visible { attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true); }
        attrs
    }

    /// Build attrs for a single item. Looks up the node from the collection by key.
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
        attrs
    }

    /// The attributes for the item group element.
    pub fn item_group_attrs(&self, section_key: &Key) -> AttrMap {
        let group_label_id = self.ctx.ids.item_part("group", &section_key, "label");
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
        let group_label_id = self.ctx.ids.item_part("group", &section_key, "label");
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ItemGroupLabel { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, group_label_id);
        attrs
    }

    /// The attributes for the item text element.
    pub fn item_text_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemText { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.item_part("item", &key, "text"));
        attrs
    }

    /// The attributes for the item indicator element.
    pub fn item_indicator_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemIndicator { key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let is_selected = self.ctx.selection.get().contains(key);
        if is_selected { attrs.set_bool(HtmlAttr::Data("ars-selected"), true); }
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

    /// Attributes for the loading sentinel element, rendered after the last item
    /// when `on_load_more` is configured. The framework adapter attaches an
    /// `IntersectionObserver` to this element; when it enters the viewport, the
    /// adapter invokes `on_load_more`. See `06-collections.md` §5.3.
    ///
    /// Returns `None` when `on_load_more` is `None` (no sentinel needed).
    pub fn loading_sentinel_attrs(&self) -> Option<AttrMap> {
        if self.props.on_load_more.is_none() { return None; }
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::LoadingSentinel.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs.set(HtmlAttr::TabIndex, "-1");
        if self.ctx.loading {
            attrs.set_bool(HtmlAttr::Data("ars-loading"), true);
        }
        Some(attrs)
    }

    /// Iterate all nodes for rendering (items, sections, headers, separators).
    pub fn items(&self) -> impl Iterator<Item = &Node<Item>> {
        self.ctx.items.nodes()
    }

    // ── Event handlers ──────────────────────────────────────────────

    /// The event handler for the item click event.
    pub fn on_item_click(&self, key: Key) { (self.send)(Event::ToggleItem(key)); }

    /// The event handler for the item hover event.
    pub fn on_item_hover(&self, key: Key) { (self.send)(Event::HighlightItem(Some(key))); }

    /// The event handler for the keydown event.
    pub fn on_keydown(&self, data: &KeyboardEventData, modifiers: Modifiers) {
        match data.key {
            KeyboardKey::ArrowDown | KeyboardKey::ArrowRight => {
                if modifiers.shift {
                    // Shift+Arrow: extend selection range
                    if let Some(ref k) = self.ctx.highlighted_key {
                        if let Some(next) = next_enabled_key(
                            &self.ctx.items, k,
                            &self.ctx.selection_state.disabled_keys,
                            self.ctx.selection_state.disabled_behavior,
                            self.ctx.loop_focus,
                        ) {
                            (self.send)(Event::SelectItem(next.clone()));
                            (self.send)(Event::HighlightItem(Some(next)));
                        }
                    }
                } else {
                    (self.send)(Event::HighlightNext);
                }
            }
            KeyboardKey::ArrowUp | KeyboardKey::ArrowLeft => {
                if modifiers.shift {
                    if let Some(ref k) = self.ctx.highlighted_key {
                        if let Some(prev) = prev_enabled_key(
                            &self.ctx.items, k,
                            &self.ctx.selection_state.disabled_keys,
                            self.ctx.selection_state.disabled_behavior,
                            self.ctx.loop_focus,
                        ) {
                            (self.send)(Event::SelectItem(prev.clone()));
                            (self.send)(Event::HighlightItem(Some(prev)));
                        }
                    }
                } else {
                    (self.send)(Event::HighlightPrev);
                }
            }
            KeyboardKey::Home => (self.send)(Event::HighlightFirst),
            KeyboardKey::End => (self.send)(Event::HighlightLast),
            KeyboardKey::Space | KeyboardKey::Enter => {
                if let Some(ref k) = self.ctx.highlighted_key {
                    (self.send)(Event::ToggleItem(k.clone()));
                }
            }
            KeyboardKey::Escape => (self.send)(Event::DeselectAll),
            _ if data.key == KeyboardKey::Char('a') && modifiers.ctrl_or_meta => {
                (self.send)(Event::SelectAll);
            }
            _ if let Some(ch) = data.character && !modifiers.ctrl_or_meta => {
                // Timestamp obtained from the adapter's clock
                (self.send)(Event::TypeaheadSearch(ch, 0));
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
            Part::Label => self.label_attrs(),
            Part::Content => self.content_attrs(),
            Part::ItemGroup { ref key } => self.item_group_attrs(key),
            Part::ItemGroupLabel { ref key } => self.item_group_label_attrs(key),
            Part::Item { ref key } => self.item_attrs(key),
            Part::ItemText { ref key } => self.item_text_attrs(key),
            Part::ItemIndicator { ref key } => self.item_indicator_attrs(key),
            Part::Description => self.description_attrs(),
            Part::LoadingSentinel => self.loading_sentinel_attrs().unwrap_or_default(),
        }
    }
}
```

## 2. Anatomy

| Part              | Selector                                                       | Element                                                                       |
| ----------------- | -------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `Root`            | `[data-ars-scope="listbox"][data-ars-part="root"]`             | `<div>`                                                                       |
| `Label`           | `[data-ars-scope="listbox"][data-ars-part="label"]`            | `<label>`                                                                     |
| `Content`         | `[data-ars-scope="listbox"][data-ars-part="content"]`          | `<div>`                                                                       |
| `ItemGroup`       | `[data-ars-scope="listbox"][data-ars-part="item-group"]`       | `<div>`                                                                       |
| `ItemGroupLabel`  | `[data-ars-scope="listbox"][data-ars-part="item-group-label"]` | `<div>`                                                                       |
| `Item`            | `[data-ars-scope="listbox"][data-ars-part="item"]`             | `<div>`                                                                       |
| `ItemText`        | `[data-ars-scope="listbox"][data-ars-part="item-text"]`        | `<span>`                                                                      |
| `ItemIndicator`   | `[data-ars-scope="listbox"][data-ars-part="item-indicator"]`   | `<div>`                                                                       |
| `Description`     | `[data-ars-scope="listbox"][data-ars-part="description"]`      | `<div>`                                                                       |
| `LoadingSentinel` | `[data-ars-scope="listbox"][data-ars-part="loading-sentinel"]` | `<div>` (optional, after last Item; rendered only when `on_load_more` is set) |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property                | Element         | Value                                                                                                                                                                                               |
| ----------------------- | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `role`                  | Content         | `listbox`                                                                                                                                                                                           |
| `tabindex`              | Content         | `"0"` (required for `aria-activedescendant` to function; the Content element must be focusable)                                                                                                     |
| `aria-multiselectable`  | Content         | Present for multi-select                                                                                                                                                                            |
| `aria-orientation`      | Content         | `horizontal` or `vertical`                                                                                                                                                                          |
| `aria-activedescendant` | Content         | Highlighted item id (only set when a valid item is highlighted; **omit attribute entirely** when `highlighted_key` is `None` — setting it to an empty string or non-existent ID violates ARIA spec) |
| `role`                  | Item            | `option`                                                                                                                                                                                            |
| `aria-selected`         | Item            | `"true"` when selected, `"false"` when unselected (must be explicitly set, not omitted)                                                                                                             |
| `aria-hidden`           | LoadingSentinel | `"true"` — the sentinel carries no content for screen readers; loading progress is announced via a `LiveAnnouncer` (see `06-collections.md` §5.3)                                                   |
| `tabindex`              | LoadingSentinel | `"-1"` — not focusable via keyboard                                                                                                                                                                 |

> **Loading sentinel accessibility**: The `LoadingSentinel` element is `aria-hidden="true"` and excluded from the tab order. It serves purely as an `IntersectionObserver` target. Loading state changes (loading started/completed, new items available) are communicated to screen readers via a `LiveAnnouncer` managed by the adapter layer, not via ARIA attributes on the sentinel itself.
>
> **VoiceOver iOS compatibility**: VoiceOver on iOS does not support `aria-activedescendant` reliably. On iOS, the adapter MUST fall back to **roving tabindex** — moving DOM focus directly to the highlighted item (`option` element with `tabindex="0"`) rather than relying on `aria-activedescendant` on the `listbox` container. The adapter SHOULD detect iOS via `navigator.userAgent` (or `navigator.platform`) and switch focus strategy at initialization time. When using roving tabindex, the `Listbox` Content element sets `tabindex="-1"` and each highlighted option gets `tabindex="0"` with `element.focus()`. Non-highlighted options get `tabindex="-1"`. This is the same pattern used by MenuBar (§6) and TreeView (§3 in `13-navigation-components.md`).
>
> **iOS VoiceOver Compatibility Note**
>
> On iOS, VoiceOver does not support `aria-activedescendant` for announcing the
> currently focused option. The `Listbox` implementation uses roving `tabindex`
> (moving `tabindex="0"` to the active option and `tabindex="-1"` to all others)
> as the primary focus management strategy, which provides reliable announcements
> across all screen readers including iOS VoiceOver.
>
> When `aria-activedescendant` is used (e.g., in `Combobox` where focus must remain
> on the input), an additional `aria-selected="true"` attribute is set on the
> active option to improve iOS VoiceOver announcements. Test with VoiceOver on
> actual iOS devices to verify announcement quality.

### 3.2 Keyboard Interaction

| Key                  | Action                                |
| -------------------- | ------------------------------------- |
| ArrowDown/ArrowRight | Highlight next                        |
| ArrowUp/ArrowLeft    | Highlight previous                    |
| Home                 | Highlight first                       |
| End                  | Highlight last                        |
| Space / Enter        | Select/deselect highlighted           |
| Ctrl+A               | Select all (multi-select)             |
| Shift+ArrowDown      | Extend selection range (multi-select) |
| Escape               | Deselect all                          |
| a-z                  | Typeahead highlight                   |

#### 3.2.1 RTL Arrow Key Handling

When the `Listbox` has `orientation: Horizontal` and `dir: Rtl`, the meanings of `ArrowLeft`
and `ArrowRight` MUST be swapped to match the physical layout direction:

```rust
// In on_keydown, resolve horizontal arrow keys with RTL awareness:
let resolved_key = match (data.key, self.ctx.orientation, self.ctx.dir) {
    (KeyboardKey::ArrowRight, Orientation::Horizontal, Direction::Rtl) => KeyboardKey::ArrowLeft,
    (KeyboardKey::ArrowLeft, Orientation::Horizontal, Direction::Rtl) => KeyboardKey::ArrowRight,
    _ => data.key,
};
// Then match on resolved_key for highlight navigation.
```

This is the canonical RTL rule applied consistently across all horizontal-arrow-key components
(`Accordion`, `Tabs`, `Listbox`, `Splitter`, `MenuBar`, `Carousel`, `Toolbar`). See `03-accessibility.md`
for the global rule: "In RTL horizontal layouts, ArrowRight/ArrowLeft meanings flip to match
physical layout."

## 4. Internationalization

Same as `Select`: locale-aware typeahead via `Collator`, RTL direction reversal.

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Text for the empty state when no options are available.
    pub empty_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Live region announcement for the number of available options.
    pub count_announcement: MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            empty_label: MessageFn::static_str("No options available"),
            count_announcement: MessageFn::new(|n, _locale| match n {
                0 => "No options available".to_string(),
                1 => "1 option available".to_string(),
                n => format!("{n} options available"),
            }),
        }
    }
}

impl ComponentMessages for Messages {}
```

> **Drag and Drop Reorder**: When `dnd_enabled: true` is set, `Listbox` integrates with `DraggableCollection<T>` and `DroppableCollection<T>` from `06-collections.md` §7. This adds optional `DragHandle` and `DropIndicator` anatomy parts, and fires `CollectionDndEvent::Reorder` when items are drag-reordered.

## 5. Library Parity

> Compared against: Ark UI (`Listbox`), React Aria (`ListBox`).

### 5.1 Props

| Feature                       | ars-ui                       | Ark UI                                           | React Aria                             | Notes                                             |
| ----------------------------- | ---------------------------- | ------------------------------------------------ | -------------------------------------- | ------------------------------------------------- |
| Controlled/uncontrolled value | `value` / `default_value`    | `value` / `defaultValue`                         | `selectedKeys` / `defaultSelectedKeys` | --                                                |
| Selection mode                | `selection_mode`             | `selectionMode` (`single`/`multiple`/`extended`) | `selectionMode` (`single`/`multiple`)  | Ark UI adds `extended` mode                       |
| Selection behavior            | `selection_behavior`         | --                                               | --                                     | ars-ui exclusive (toggle vs replace)              |
| Disabled                      | `disabled`                   | `disabled`                                       | --                                     | --                                                |
| Disabled keys                 | `disabled_keys`              | --                                               | `disabledKeys`                         | --                                                |
| Required                      | `required`                   | --                                               | --                                     | ars-ui exclusive                                  |
| Invalid                       | `invalid`                    | --                                               | --                                     | ars-ui exclusive                                  |
| Orientation                   | `orientation`                | `orientation`                                    | `orientation`                          | --                                                |
| Loop focus                    | `loop_focus`                 | `loopFocus`                                      | `shouldFocusWrap`                      | --                                                |
| Disallow empty selection      | `disallow_empty_selection`   | `deselectable` (inverse)                         | `disallowEmptySelection`               | --                                                |
| Disabled behavior             | `disabled_behavior`          | --                                               | --                                     | ars-ui exclusive (skip vs focus-only)             |
| On action                     | `on_action`                  | --                                               | `onAction`                             | Item activation distinct from selection           |
| Typeahead                     | Yes (via `typeahead::State`) | `typeahead`                                      | Yes (built-in)                         | --                                                |
| Select on highlight           | --                           | `selectOnHighlight`                              | --                                     | Ark UI exclusive; niche pattern                   |
| Disallow select all           | --                           | `disallowSelectAll`                              | --                                     | Ark UI exclusive                                  |
| Layout (grid)                 | --                           | --                                               | `layout` (`stack`/`grid`)              | React Aria exclusive                              |
| Escape key behavior           | --                           | --                                               | `escapeKeyBehavior`                    | React Aria exclusive                              |
| Auto-focus on hover           | --                           | --                                               | `shouldFocusOnHover`                   | React Aria exclusive                              |
| Render empty state            | --                           | --                                               | `renderEmptyState`                     | React Aria uses render prop; ars-ui uses Messages |
| Drag and drop                 | DnD integration              | --                                               | `dragAndDropHooks`                     | --                                                |
| Load more (infinite scroll)   | `on_load_more` / `loading`   | `scrollToIndexFn`                                | `ListBoxLoadMoreItem`                  | Different approaches                              |
| Highlighted value control     | --                           | `highlightedValue` / `defaultHighlightedValue`   | --                                     | Ark UI exclusive                                  |

**Gaps:** None. Ark UI's `selectOnHighlight` and `disallowSelectAll` are niche patterns. React Aria's `layout: 'grid'` maps to GridList which is a separate component concept. React Aria's `escapeKeyBehavior` and `shouldFocusOnHover` are minor behavioral tweaks not essential for core functionality.

### 5.2 Anatomy

| Part            | ars-ui            | Ark UI           | React Aria               | Notes                                                    |
| --------------- | ----------------- | ---------------- | ------------------------ | -------------------------------------------------------- |
| Root            | `Root`            | `Root`           | `ListBox`                | --                                                       |
| Label           | `Label`           | `Label`          | --                       | External label in React Aria                             |
| Content         | `Content`         | `Content`        | --                       | ars-ui wraps items in a content container                |
| ItemGroup       | `ItemGroup`       | `ItemGroup`      | `ListBoxSection`         | --                                                       |
| ItemGroupLabel  | `ItemGroupLabel`  | `ItemGroupLabel` | `Header`                 | --                                                       |
| Item            | `Item`            | `Item`           | `ListBoxItem`            | --                                                       |
| ItemText        | `ItemText`        | `ItemText`       | `Text[slot=label]`       | --                                                       |
| ItemIndicator   | `ItemIndicator`   | `ItemIndicator`  | --                       | --                                                       |
| Empty           | --                | `Empty`          | render prop              | ars-ui uses Messages-based empty text                    |
| LoadingSentinel | `LoadingSentinel` | --               | `ListBoxLoadMoreItem`    | --                                                       |
| Description     | `Description`     | --               | `Text[slot=description]` | --                                                       |
| ErrorMessage    | `ErrorMessage`    | --               | `FieldError`             | --                                                       |
| Input (filter)  | --                | `Input`          | --                       | Ark UI exclusive; ars-ui uses Autocomplete for filtering |
| ValueText       | --                | `ValueText`      | --                       | Ark UI exclusive                                         |

**Gaps:** None.

### 5.3 Events

| Callback         | ars-ui                        | Ark UI              | React Aria          | Notes |
| ---------------- | ----------------------------- | ------------------- | ------------------- | ----- |
| Value change     | via `Bindable`                | `onValueChange`     | `onSelectionChange` | --    |
| Highlight change | via `Context.highlighted_key` | `onHighlightChange` | --                  | --    |
| Item action      | `on_action`                   | --                  | `onAction`          | --    |
| Load more        | `on_load_more`                | --                  | `onLoadMore`        | --    |

**Gaps:** None.

### 5.4 Features

| Feature             | ars-ui | Ark UI | React Aria |
| ------------------- | ------ | ------ | ---------- |
| Single select       | Yes    | Yes    | Yes        |
| Multi select        | Yes    | Yes    | Yes        |
| Typeahead           | Yes    | Yes    | Yes        |
| Disabled items      | Yes    | Yes    | Yes        |
| Item groups         | Yes    | Yes    | Yes        |
| Keyboard navigation | Yes    | Yes    | Yes        |
| Orientation (h/v)   | Yes    | Yes    | Yes        |
| IME composition     | Yes    | --     | --         |
| Infinite scroll     | Yes    | --     | Yes        |
| DnD reorder         | Yes    | --     | Yes        |
| RTL support         | Yes    | Yes    | Yes        |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity -- no gaps identified.
- **Divergences:** (1) ars-ui provides `on_action` for item activation (matching React Aria) while Ark UI does not distinguish action from selection; (2) ars-ui uses `LoadingSentinel` part for infinite scroll while React Aria uses `ListBoxLoadMoreItem`; (3) Ark UI includes an inline `Input` part for filtering; ars-ui uses the separate `Autocomplete` component for this purpose.
- **Recommended additions:** None.
