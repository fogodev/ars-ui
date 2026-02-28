---
component: ToggleGroup
category: utility
tier: stateful
foundation_deps: [architecture, accessibility, interactions, forms, collections]
shared_deps: []
related: [toggle-button, toggle]
references:
  ark-ui: ToggleGroup
  radix-ui: ToggleGroup
  react-aria: ToggleButtonGroup
---

# ToggleGroup

A `ToggleGroup` is a group of `ToggleButton` instances where zero, one, or many can be active
depending on the `selection_mode`. Common uses: text alignment toolbar (single), tag filter
(multiple).

## 1. State Machine

### 1.1 States

| State                   | Description                          |
| ----------------------- | ------------------------------------ |
| `Idle`                  | No item is focused within the group. |
| `Focused { item: Key }` | An item within the group has focus.  |

### 1.2 Events

| Event          | Payload                        | Description                                          |
| -------------- | ------------------------------ | ---------------------------------------------------- |
| `SelectItem`   | `Key`                          | Activate an item by id.                              |
| `DeselectItem` | `Key`                          | Deactivate an item by id.                            |
| `ToggleItem`   | `Key`                          | Toggle an item's active state by id.                 |
| `Focus`        | `item: Key, is_keyboard: bool` | An item received focus.                              |
| `Blur`         | —                              | Focus left the group.                                |
| `FocusNext`    | —                              | Move focus to the next item (Arrow key).             |
| `FocusPrev`    | —                              | Move focus to the previous item (Arrow key).         |
| `FocusFirst`   | —                              | Move focus to the first item (Home key).             |
| `FocusLast`    | —                              | Move focus to the last item (End key).               |
| `Reset`        | —                              | Restore `value` to `default_value`.                  |
| `SetValue`     | `BTreeSet<Key>`                | Set the controlled value directly (from props sync). |
| `SetProps`     | —                              | Sync context fields from updated props.              |

### 1.3 Context

```rust
/// The state of the `ToggleGroup` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// The group is idle.
    Idle,
    /// The group has focus on an item.
    Focused {
        /// The id of the item that has focus.
        item: Key,
    },
}

/// The events for the `ToggleGroup` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Select an item by id.
    SelectItem(Key),
    /// Deselect an item by id.
    DeselectItem(Key),
    /// Toggle an item's active state by id.
    ToggleItem(Key),
    /// Focus received on an item by id.
    Focus {
        /// The id of the item that has focus.
        item: Key,
        /// True if the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// Focus lost from the group.
    Blur,
    /// Move focus to the next item (Arrow key).
    FocusNext,
    /// Move focus to the previous item (Arrow key).
    FocusPrev,
    /// Move focus to the first item (Home key).
    FocusFirst,
    /// Move focus to the last item (End key).
    FocusLast,
    /// Restore `value` to `default_value`.
    Reset,
    /// Set the controlled value directly (from `on_props_changed`).
    SetValue(BTreeSet<Key>),
    /// Sync context fields from updated props (disabled, orientation, etc.).
    SetProps,
}

/// The selection mode for the `ToggleGroup` component.
///
/// NOTE: This is the ToggleGroup-specific selection mode, distinct from
/// `selection::Mode` used in selection-category components (Select, Listbox, etc.).
/// ToggleGroup uses a simpler enum because it does not support range selection
/// or the full collection-based selection model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SelectionMode {
    /// No items can be selected — the group is a toolbar only.
    None,
    /// Exactly zero or one item selected at a time.
    Single,
    /// Any number of items may be selected simultaneously.
    Multiple,
}

use std::collections::BTreeSet;
use ars_core::Key;
use ars_i18n::{Direction, Orientation};

/// The context of the `ToggleGroup` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Currently selected item(s). Controlled or uncontrolled.
    pub value: Bindable<BTreeSet<Key>>,
    /// The id of the currently focused item (if any).
    /// NOTE: Focused item is tracked in both `State::Focused { item }` and here.
    /// State carries it for transition pattern matching; Context carries it for
    /// the connect API (e.g., `item_attrs()` roving tabindex logic) which only
    /// receives `&Context`, not `&State`.
    pub focused_item: Option<Key>,
    /// Whether focus was received via keyboard (for focus-visible styles).
    pub focus_visible: bool,
    /// The selection mode for the `ToggleGroup` component.
    pub selection_mode: SelectionMode,
    /// Whether the group is disabled.
    pub disabled: bool,
    /// The orientation of the `ToggleGroup` component.
    pub orientation: Orientation,
    /// Text direction — used to swap ArrowLeft/ArrowRight in horizontal groups.
    pub dir: Direction,
    /// When true, only one item has tabindex=0 at a time (roving tabindex).
    /// When false, all items always have tabindex=0 (simpler but less standard).
    pub loop_focus: bool,
    /// When true, only one item has tabindex=0 at a time (roving tabindex).
    pub roving_focus: bool,
    /// When true, prevents deselecting the last selected item.
    pub disallow_empty_selection: bool,
    /// All registered item IDs in DOM/insertion order.
    /// Used for keyboard navigation (FocusNext/Prev/First/Last).
    /// Distinct from `value` which only tracks *selected* items.
    ///
    /// Population: Items register via context-based registration on mount in the
    /// adapter layer. Each `ToggleButton` child calls a registration function
    /// (provided via framework context) during its mount/create lifecycle, passing
    /// its `value` prop. The adapter appends the item ID to this Vec. On unmount,
    /// the adapter removes the item. This keeps `registered_items` in sync with
    /// the actual DOM order.
    pub registered_items: Vec<Key>,
    /// The active locale, inherited from ArsProvider context.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
}

/// Per-item context provided to each `ToggleButton` child via framework context.
/// The adapter sets this when rendering each item so the child toggle button can
/// communicate with the parent group without prop drilling.
///
/// Each item receives `ToggleGroupItemContext` via framework context. The adapter
/// provides this when rendering each item.
pub struct ToggleGroupItemContext {
    /// The group's component ID.
    pub group_id: String,
    /// The group's selection mode.
    pub selection_mode: SelectionMode,
    /// The group's orientation.
    pub orientation: Orientation,
    /// Whether the group is disabled.
    pub disabled: bool,
    /// Whether roving focus is enabled.
    pub roving_focus: bool,
    /// Callback to send events to the group machine.
    pub send: Callback<dyn Fn(Event) + Send + Sync>,
}
```

### 1.4 Props

```rust
/// Props for the `ToggleGroup` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Controlled selected item(s).
    pub value: Option<BTreeSet<Key>>,
    /// Default selected item(s) for uncontrolled mode.
    pub default_value: BTreeSet<Key>,
    /// The selection mode for the `ToggleGroup` component.
    pub selection_mode: SelectionMode,
    /// Whether the group is disabled.
    pub disabled: bool,
    /// The orientation of the `ToggleGroup` component.
    pub orientation: Orientation,
    /// Text direction for RTL-aware arrow key navigation.
    pub dir: Direction,
    /// When true, focus loops to the first item when the last item is focused.
    pub loop_focus: bool,
    /// When true, focus roves to the next item when the last item is focused.
    pub roving_focus: bool,
    /// Accessible label for the group. Required — either `aria_label` or `aria_labelledby` must be set.
    /// Groups with `role="radiogroup"` or `role="group"` need an accessible name.
    pub aria_label: Option<String>,
    /// ID of the element that labels this group (alternative to `aria_label`).
    pub aria_labelledby: Option<String>,
    /// When true, prevents deselecting the last selected item. Default: false.
    pub disallow_empty_selection: bool,
    /// Form field name. When set, hidden `<input>` element(s) are rendered for
    /// native HTML form submission. Single mode: one hidden input with the selected
    /// value. Multiple mode: one hidden input per selected value.
    pub name: Option<String>,
    /// Whether the current value is invalid (set by form validation).
    pub invalid: bool,
    /// Whether a selection is required (set by form validation).
    pub required: bool,
    /// Locale override. When `None`, inherits from nearest `ArsProvider` context.
    pub locale: Option<Locale>,
    /// Associates the group's hidden inputs with a `<form>` element by `id`,
    /// even if the group is not a descendant of that form. Threaded to
    /// `HiddenInputConfig::form_id`.
    pub form: Option<String>,
    /// Whether the group is read-only.
    /// Read-only blocks SelectItem/DeselectItem/ToggleItem events but allows
    /// Focus/Blur/navigation. Hidden input still submits values.
    pub read_only: bool,
    /// Per-item disabled set. Items whose value is in this set are individually
    /// disabled: `aria-disabled="true"`, press handlers skipped, excluded from
    /// roving tabindex navigation.
    pub disabled_items: BTreeSet<Key>,
    /// Translatable messages. When `None`, resolved via `resolve_messages()`.
    pub messages: Option<Messages>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: BTreeSet::new(),
            selection_mode: SelectionMode::Single,
            disabled: false,
            orientation: Orientation::Horizontal,
            dir: Direction::Ltr,
            loop_focus: true,
            roving_focus: true,
            aria_label: None,
            aria_labelledby: None,
            disallow_empty_selection: false,
            name: None,
            invalid: false,
            required: false,
            locale: None,
            form: None,
            read_only: false,
            disabled_items: BTreeSet::new(),
            messages: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
/// The machine for the `ToggleGroup` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let ids = ComponentIds::from_id(&props.id);
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);

        let value = match &props.value {
            Some(v) => Bindable::controlled(v.clone()),
            None    => Bindable::uncontrolled(props.default_value.clone()),
        };

        let ctx = Context {
            value,
            focused_item: None,
            focus_visible: false,
            selection_mode: props.selection_mode,
            disabled: props.disabled,
            orientation: props.orientation,
            dir: props.dir,
            loop_focus: props.loop_focus,
            roving_focus: props.roving_focus,
            disallow_empty_selection: props.disallow_empty_selection,
            registered_items: Vec::new(),
            locale,
            messages,
        };

        (State::Idle, ctx)
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        // Disabled groups still allow keyboard navigation for screen reader discoverability (WAI-ARIA)
        if ctx.disabled && matches!(event,
            Event::SelectItem(_) | Event::DeselectItem(_) | Event::ToggleItem(_) | Event::Reset
        ) {
            return None;
        }

        // Read-only guard: blocks value-changing events but allows Focus/Blur/navigation.
        if props.read_only && matches!(event, Event::SelectItem(_) | Event::DeselectItem(_) | Event::ToggleItem(_)) {
            return None;
        }

        match (state, event) {
            // ── SetValue (controlled value sync from on_props_changed) ───────
            (_, Event::SetValue(new_value)) => {
                let new_value = new_value.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(new_value);
                }))
            }

            // ── SetProps (sync context fields from props) ────────────────────
            (_, Event::SetProps) => {
                let disabled = props.disabled;
                let selection_mode = props.selection_mode;
                let orientation = props.orientation;
                let dir = props.dir;
                let loop_focus = props.loop_focus;
                let roving_focus = props.roving_focus;
                let disallow_empty_selection = props.disallow_empty_selection;
                let locale = resolve_locale(props.locale.as_ref());
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.disabled = disabled;
                    ctx.selection_mode = selection_mode;
                    ctx.orientation = orientation;
                    ctx.dir = dir;
                    ctx.loop_focus = loop_focus;
                    ctx.roving_focus = roving_focus;
                    ctx.disallow_empty_selection = disallow_empty_selection;
                    ctx.locale = locale;
                }))
            }
            // ── SelectItem ───────────────────────────────────────────────────
            (_, Event::SelectItem(id)) => {
                let id = id.clone();
                let mode = ctx.selection_mode;
                Some(TransitionPlan::context_only(move |ctx| {
                    match mode {
                        SelectionMode::Single => {
                            ctx.value.set(BTreeSet::from([id]));
                        }
                        SelectionMode::Multiple => {
                            if !ctx.value.get().contains(&id) {
                                let mut v = ctx.value.get().clone();
                                v.insert(id);
                                ctx.value.set(v);
                            }
                        }
                        SelectionMode::None => {}
                    }
                }))
            }

            // ── DeselectItem ─────────────────────────────────────────────────
            // When `disallow_empty_selection` is true, the last selected item cannot be deselected.
            (_, Event::DeselectItem(id)) => {
                if props.disallow_empty_selection && ctx.value.get().len() <= 1 { return None; }
                let id = id.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut v = ctx.value.get().clone();
                    v.remove(&id);
                    ctx.value.set(v);
                }))
            }

            // ── ToggleItem ───────────────────────────────────────────────────
            // Inlined select/deselect logic to avoid recursive Machine::transition calls.
            (_, Event::ToggleItem(id)) => {
                let id = id.clone();
                let mode = ctx.selection_mode;
                let currently_selected = ctx.value.get().contains(&id);
                if currently_selected {
                    // Deselect: guard against empty selection if disallowed.
                    if props.disallow_empty_selection && ctx.value.get().len() <= 1 {
                        return None;
                    }
                    Some(TransitionPlan::context_only(move |ctx| {
                        let mut v = ctx.value.get().clone();
                        v.remove(&id);
                        ctx.value.set(v);
                    }))
                } else {
                    // Select
                    Some(TransitionPlan::context_only(move |ctx| {
                        match mode {
                            SelectionMode::Single => {
                                ctx.value.set(BTreeSet::from([id]));
                            }
                            SelectionMode::Multiple => {
                                let mut v = ctx.value.get().clone();
                                v.insert(id);
                                ctx.value.set(v);
                            }
                            SelectionMode::None => {}
                        }
                    }))
                }
            }

            // ── Focus ────────────────────────────────────────────────────────
            (_, Event::Focus { item, is_keyboard }) => {
                let item = item.clone();
                let is_keyboard = *is_keyboard;
                Some(TransitionPlan::to(State::Focused { item: item.clone() })
                    .apply(move |ctx| {
                        ctx.focused_item = Some(item);
                        ctx.focus_visible = is_keyboard;
                    }))
            }

            // ── Blur ─────────────────────────────────────────────────────────
            (_, Event::Blur) => {
                Some(TransitionPlan::to(State::Idle)
                    .apply(|ctx| {
                        ctx.focused_item = None;
                        ctx.focus_visible = false;
                    }))
            }

            // ── FocusNext from Idle → delegate to FocusFirst ────────────────
            (State::Idle, Event::FocusNext) => {
                let items = ctx.registered_items.clone();
                let first = items.first().cloned();
                let first_for_state = first.clone();
                Some(TransitionPlan::to(State::Focused {
                    item: first_for_state.unwrap_or_default(),
                }).apply(move |ctx| {
                    ctx.focused_item = first;
                    ctx.focus_visible = true;
                }))
            }

            // ── FocusPrev from Idle → delegate to FocusLast ────────────────
            (State::Idle, Event::FocusPrev) => {
                let items = ctx.registered_items.clone();
                let last = items.last().cloned();
                let last_for_state = last.clone();
                Some(TransitionPlan::to(State::Focused {
                    item: last_for_state.unwrap_or_default(),
                }).apply(move |ctx| {
                    ctx.focused_item = last;
                    ctx.focus_visible = true;
                }))
            }

            // ── FocusNext ────────────────────────────────────────────────────
            (State::Focused { item }, Event::FocusNext) => {
                // Roving tabindex: move focus to next item.
                // Compute the next item from the registered items list (all items
                // in DOM/insertion order), NOT from value (which is selected items only).
                let items = ctx.registered_items.clone();
                let current = item.clone();
                let loop_focus = ctx.loop_focus;
                let next = {
                    let idx = items.iter().position(|k| k == &current);
                    match idx {
                        Some(i) if i + 1 < items.len() => Some(items[i + 1].clone()),
                        Some(i) if loop_focus && !items.is_empty() => Some(items[0].clone()),
                        _ if items.is_empty() => None,
                        _ => None,
                    }
                };
                let next_for_state = next.clone();
                Some(TransitionPlan::to(State::Focused {
                    item: next_for_state.unwrap_or_else(|| current.clone()),
                }).apply(move |ctx| {
                    if let Some(next_item) = next {
                        ctx.focused_item = Some(next_item);
                    }
                    ctx.focus_visible = true;
                }))
            }

            // ── FocusPrev ────────────────────────────────────────────────────
            (State::Focused { item }, Event::FocusPrev) => {
                // Compute the previous item from the registered items list.
                let items = ctx.registered_items.clone();
                let current = item.clone();
                let loop_focus = ctx.loop_focus;
                let prev = {
                    let idx = items.iter().position(|k| k == &current);
                    match idx {
                        Some(0) if loop_focus && !items.is_empty() => Some(items[items.len() - 1].clone()),
                        Some(i) if i > 0 => Some(items[i - 1].clone()),
                        _ if items.is_empty() => None,
                        _ => None,
                    }
                };
                let prev_for_state = prev.clone();
                Some(TransitionPlan::to(State::Focused {
                    item: prev_for_state.unwrap_or_else(|| current.clone()),
                }).apply(move |ctx| {
                    if let Some(prev_item) = prev {
                        ctx.focused_item = Some(prev_item);
                    }
                    ctx.focus_visible = true;
                }))
            }

            // ── FocusFirst ───────────────────────────────────────────────────
            (_, Event::FocusFirst) => {
                let items = ctx.registered_items.clone();
                let first = items.first().cloned();
                let first_for_state = first.clone();
                Some(TransitionPlan::to(State::Focused {
                    item: first_for_state.unwrap_or_default(),
                }).apply(move |ctx| {
                    ctx.focused_item = first;
                    ctx.focus_visible = true;
                }))
            }

            // ── FocusLast ────────────────────────────────────────────────────
            (_, Event::FocusLast) => {
                let items = ctx.registered_items.clone();
                let last = items.last().cloned();
                let last_for_state = last.clone();
                Some(TransitionPlan::to(State::Focused {
                    item: last_for_state.unwrap_or_default(),
                }).apply(move |ctx| {
                    ctx.focused_item = last;
                    ctx.focus_visible = true;
                }))
            }

            // ── Reset ────────────────────────────────────────────────────────
            (_, Event::Reset) => {
                let default = props.default_value.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(default);
                }))
            }

            _ => None,
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        let mut events = Vec::new();
        if old.value != new.value {
            if let Some(ref new_value) = new.value {
                events.push(Event::SetValue(new_value.clone()));
            }
        }
        if old.disabled != new.disabled
            || old.orientation != new.orientation
            || old.dir != new.dir
            || old.loop_focus != new.loop_focus
            || old.roving_focus != new.roving_focus
            || old.selection_mode != new.selection_mode
            || old.read_only != new.read_only
            || old.disallow_empty_selection != new.disallow_empty_selection
            || old.locale != new.locale
        {
            events.push(Event::SetProps);
        }
        events
    }

    fn connect<'a>(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.6 Indicator Part

The `Indicator` part provides an animated sliding selection highlight, similar to
Ark UI's SegmentGroup Indicator. It is positioned via CSS custom properties set
by the connect layer based on the selected item's DOM measurements.

**CSS Custom Properties** (set as inline styles on the indicator element):

| Property                             | Description                                                             |
| ------------------------------------ | ----------------------------------------------------------------------- |
| `--ars-indicator-inset-inline-start` | Inline-start offset of the indicator (uses `LogicalRect.inline_start`). |
| `--ars-indicator-top`                | Vertical offset of the indicator from the group root.                   |
| `--ars-indicator-width`              | Width of the indicator (matches the selected item).                     |
| `--ars-indicator-height`             | Height of the indicator (matches the selected item).                    |

> **SSR behaviour:** During SSR, render the indicator element with `display: none` inline style. On hydration, the adapter measures item positions and replaces the inline style with CSS custom properties (`--ars-indicator-inset-inline-start`, `--ars-indicator-top`, `--ars-indicator-width`, `--ars-indicator-height`).
>
> **Dioxus Desktop note:** The indicator positioning relies on `getBoundingClientRect()` which is
> web-only. Dioxus Desktop needs a platform-agnostic measurement API to compute indicator position.
> Until Dioxus provides a native layout measurement primitive, the indicator part is web-only.
> Desktop adapters should either omit the indicator or use a CSS-only highlight approach (e.g.,
> background color on `[data-ars-selected]`) instead of absolute positioning.

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "toggle-group"]
pub enum Part {
    Root,
    Item { value: Key },
    Indicator,
}

/// The API for the `ToggleGroup` component.
pub struct Api<'a> {
    /// The state of the `ToggleGroup` component.
    state: &'a State,
    /// The context of the `ToggleGroup` component.
    ctx: &'a Context,
    /// The props of the `ToggleGroup` component.
    props: &'a Props,
    /// The send callback for the `ToggleGroup` component.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Whether the item with the given id is selected.
    pub fn is_selected(&self, item_id: &Key) -> bool {
        self.ctx.value.get().contains(item_id)
    }

    /// The attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        // Use radiogroup semantics for single-selection mode.
        let role = match self.ctx.selection_mode {
            SelectionMode::Single => "radiogroup",
            _ => "group",
        };
        attrs.set(HtmlAttr::Role, role);
        let orientation_str = match self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical => "vertical",
        };
        attrs.set(HtmlAttr::Data("ars-orientation"), orientation_str);
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), orientation_str);
        // aria-label or aria-labelledby is REQUIRED on group/radiogroup roles.
        // Priority: aria_labelledby > aria_label > messages.group_label fallback.
        if let Some(ref labelledby) = self.props.aria_labelledby {
            attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), labelledby.as_str());
        } else if let Some(ref label) = self.props.aria_label {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label.as_str());
        } else {
            // Fallback to messages — always ensure an accessible name exists.
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.group_label)(&self.ctx.locale));
            #[cfg(debug_assertions)]
            log::warn!("ToggleGroup: No accessible name provided. Set aria_label or aria_labelledby.");
        }
        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        if self.props.read_only {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }
        if self.props.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
        }
        if self.props.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }
        attrs
    }

    /// The attributes for the item element.
    pub fn item_attrs(&self, item_id: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item { value: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-value"), item_id.to_string());
        let selected = self.is_selected(item_id);
        // Focused item data attribute for CSS-based focus styling.
        let is_focused = match &self.state {
            State::Focused { item } => item == item_id,
            State::Idle => false,
        };
        if is_focused {
            attrs.set_bool(HtmlAttr::Data("ars-focused"), true);
        }

        match self.ctx.selection_mode {
            SelectionMode::Single => {
                // In radiogroup mode, items use role="radio" with aria-checked.
                attrs.set(HtmlAttr::Role, "radio");
                attrs.set(HtmlAttr::Aria(AriaAttr::Checked), if selected { "true" } else { "false" });
            }
            SelectionMode::Multiple => {
                // In multiple mode, items use aria-pressed.
                attrs.set(HtmlAttr::Role, "button");
                attrs.set(HtmlAttr::Aria(AriaAttr::Pressed), if selected { "true" } else { "false" });
            }
            SelectionMode::None => {
                // In none mode (toolbar only), no selection semantics.
                attrs.set(HtmlAttr::Role, "button");
            }
        }

        if selected {
            attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
        }

        // Roving tabindex: only the selected item (or first item if nothing selected)
        // has tabindex=0; others get tabindex=-1.
        if self.ctx.roving_focus {
            let is_focus_target = match &self.state {
                State::Focused { item } => item == item_id,
                State::Idle => {
                    if selected {
                        true
                    } else if self.ctx.value.get().is_empty() {
                        // When nothing is selected, the first registered item
                        // gets tabindex=0 so the group is reachable via Tab.
                        self.ctx.registered_items.first() == Some(item_id)
                    } else {
                        false
                    }
                }
            };
            attrs.set(HtmlAttr::TabIndex, if is_focus_target { "0" } else { "-1" });
        }

        let item_disabled = self.ctx.disabled || self.props.disabled_items.contains(item_id);
        if item_disabled {
            // Use aria-disabled instead of the HTML disabled attribute so items
            // remain focusable and discoverable by screen readers.
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
            // Per-item disabled items are excluded from roving tabindex navigation
            // and their press handlers are skipped by the adapter.
        }

        // Event handlers (click, focus, blur, keydown for navigation) are typed methods on the Api struct.

        attrs
    }

    /// Attributes for the optional animated selection indicator.
    /// The adapter measures the selected item's position/size and sets
    /// CSS custom properties so consumers can animate the indicator
    /// via CSS transitions or animations.
    pub fn indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Indicator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        // Position/size set by adapter using layout measurement
        // of the currently selected item element.
        if let Some(selected) = self.ctx.value.get().iter().next() {
            attrs.set(HtmlAttr::Data("ars-active-value"), selected.to_string());
        }
        // The adapter layer measures the selected item's bounding rect
        // relative to the group root and sets these CSS custom properties:
        //   --ars-indicator-inset-inline-start, --ars-indicator-top,
        //   --ars-indicator-width, --ars-indicator-height
        // The inline-start value uses LogicalRect.inline_start (not physical left)
        // for correct RTL behavior. These are set dynamically via inline styles
        // by the adapter. CSS consumers use `inset-inline-start` instead of `left`.
        attrs
    }

    /// Handle keydown on a toggle-group item.
    /// Arrow keys navigate between items with RTL-aware direction swapping.
    pub fn on_item_keydown(&self, data: &KeyboardEventData) {
        let is_horizontal = self.ctx.orientation == Orientation::Horizontal;
        let is_rtl = self.ctx.dir == Direction::Rtl;

        match data.key {
            KeyboardKey::ArrowRight if is_horizontal => {
                if is_rtl { (self.send)(Event::FocusPrev) }
                else { (self.send)(Event::FocusNext) }
            }
            KeyboardKey::ArrowLeft if is_horizontal => {
                if is_rtl { (self.send)(Event::FocusNext) }
                else { (self.send)(Event::FocusPrev) }
            }
            KeyboardKey::ArrowDown if !is_horizontal => (self.send)(Event::FocusNext),
            KeyboardKey::ArrowUp if !is_horizontal => (self.send)(Event::FocusPrev),
            KeyboardKey::Home => (self.send)(Event::FocusFirst),
            KeyboardKey::End => (self.send)(Event::FocusLast),
            _ => {}
        }
    }

    /// Hidden input configuration for native HTML form submission.
    /// Returns `None` when `name` is not set.
    /// When the group is disabled, returns `Some(HiddenInputConfig { disabled: true, .. })`
    /// so the adapter can decide whether to render disabled hidden inputs.
    /// For Single mode: one hidden input with the selected value.
    /// For Multiple mode: one `HiddenInputConfig` with `HiddenInputValue::Multiple`.
    pub fn hidden_input_config(&self) -> Option<HiddenInputConfig> {
        let name = self.props.name.as_ref()?;

        let selected = self.ctx.value.get();
        let value = if selected.is_empty() {
            HiddenInputValue::None
        } else {
            match self.ctx.selection_mode {
                SelectionMode::Single => {
                    HiddenInputValue::Single(selected.iter().next().expect("non-empty set").to_string())
                }
                SelectionMode::Multiple => {
                    HiddenInputValue::Multiple(selected.iter().map(|k| k.to_string()).collect())
                }
                SelectionMode::None => HiddenInputValue::None,
            }
        };

        Some(HiddenInputConfig {
            name: name.clone(),
            value,
            form_id: self.props.form.clone(),
            disabled: self.ctx.disabled,
        })
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Item { ref value } => self.item_attrs(value),
            Part::Indicator => self.indicator_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
toggle-group
├── Root       <div>    (required — role="group" | role="radiogroup")
├── Item (×N)  <button> (required — aria-pressed | role="radio" aria-checked)
└── Indicator  <div>    (optional — aria-hidden="true", animated selection highlight)
```

| Part      | Element    | Key Attributes                                                         |
| --------- | ---------- | ---------------------------------------------------------------------- |
| Root      | `<div>`    | `role="group"\|"radiogroup"`, `aria-orientation`, `aria-label`         |
| Item      | `<button>` | `aria-pressed` or `role="radio"` + `aria-checked`, `data-ars-selected` |
| Indicator | `<div>`    | `aria-hidden="true"`, `data-ars-active-value`                          |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part            | Role                    | Properties                                                                                           |
| --------------- | ----------------------- | ---------------------------------------------------------------------------------------------------- |
| Root            | `group` or `radiogroup` | `aria-label`/`aria-labelledby`, `aria-orientation`, `aria-disabled`, `aria-invalid`, `aria-required` |
| Item (single)   | `radio`                 | `aria-checked="true\|false"`, `aria-disabled`                                                        |
| Item (multiple) | `button`                | `aria-pressed="true\|false"`, `aria-disabled`                                                        |
| Item (none)     | `button`                | `aria-disabled`                                                                                      |
| Indicator       | (none)                  | `aria-hidden="true"`                                                                                 |

- Root: `role="group"` with `aria-label` or `aria-labelledby` identifying the group purpose.
  For single-selection, `role="radiogroup"`.
- Single selection items: `role="radio"` + `aria-checked="true|false"`. Roving tabindex applies.
- Multiple selection items: `role="button"` + `aria-pressed="true|false"`. All items tab-focusable
  unless `roving_focus=true`.
- When all items are deselected in `Single` mode, the first item should still have `tabindex=0`
  so the group is reachable via Tab.

### 3.2 Keyboard Interaction

- Arrow keys navigate between items; Tab moves focus out of the group entirely.
- Space or Enter activates the focused item.

| Key                         | Action                  |
| --------------------------- | ----------------------- |
| ArrowRight (horizontal LTR) | Focus next item         |
| ArrowLeft (horizontal LTR)  | Focus previous item     |
| ArrowRight (horizontal RTL) | Focus previous item     |
| ArrowLeft (horizontal RTL)  | Focus next item         |
| ArrowDown (vertical)        | Focus next item         |
| ArrowUp (vertical)          | Focus previous item     |
| Home                        | Focus first item        |
| End                         | Focus last item         |
| Space / Enter               | Toggle the focused item |

### 3.3 Forced-Colors Mode

In Windows High Contrast Mode (`@media (forced-colors: active)`), the indicator part may become
invisible. Selected items MUST have a visible border or outline fallback:

```css
@media (forced-colors: active) {
  [data-ars-selected="true"] {
    outline: 2px solid Highlight;
    outline-offset: -2px;
  }
}
```

### 3.4 Screen Reader Test Expectations

**Single selection (radiogroup):**

- Tab to group: '[group label], radio group'
- Focus item: '[item label], radio button, checked/not checked, 1 of N'
- Arrow to next: '[item label], radio button, not checked, 2 of N'

**Multiple selection (group + aria-pressed):**

- Tab to group: '[group label], group'
- Focus item: '[item label], toggle button, pressed/not pressed'
- Arrow to next: '[item label], toggle button, not pressed'

## 4. Internationalization

- In RTL layouts (`dir: Direction::Rtl`), ArrowLeft and ArrowRight meanings reverse for
  horizontal groups: ArrowLeft focuses the next item, ArrowRight focuses the previous item.
- The `on_item_keydown()` method reads `ctx.dir` and swaps `FocusNext`/`FocusPrev` for
  horizontal orientations in RTL automatically.

**RTL Arrow Keys:** In RTL mode (detected from nearest `ArsProvider` or document direction), horizontal arrow keys are flipped: ArrowLeft moves to the next item, ArrowRight moves to the previous item. This flipping is handled at the adapter level (same as RadioGroup). The machine always uses abstract 'Next'/'Previous' navigation; adapters map physical arrow keys to abstract directions based on document direction.

### 4.1 Messages

```rust
/// Messages for the `ToggleGroup` component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Label for the group root. Used as `aria-label` when no `aria_label`
    /// or `aria_labelledby` prop is provided.
    pub group_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            group_label: MessageFn::static_str("Toggle group"),
        }
    }
}

impl ComponentMessages for Messages {}
```

| Key                        | Default (en-US)  | Purpose                              |
| -------------------------- | ---------------- | ------------------------------------ |
| `toggle_group.group_label` | `"Toggle group"` | Fallback `aria-label` for group root |

The `root_attrs()` method applies the accessible name in the following priority:

1. `props.aria_labelledby` (explicit association)
2. `props.aria_label` (explicit label)
3. `messages.group_label` (fallback)

In development builds (`cfg(debug_assertions)`), if neither `aria_label` nor
`aria_labelledby` is set, `root_attrs()` emits a console warning:
`"ToggleGroup: No accessible name provided. Set aria_label or aria_labelledby."`.

## 5. Form Integration

When the `name` prop is set, `ToggleGroup` participates in native HTML form submission via hidden
`<input>` elements, following the `HiddenInputConfig` pattern from `07-forms.md` section 7.

- **Single mode**: One hidden input with `name` and the selected item's value string.
- **Multiple mode**: One hidden input per selected value, all sharing the same `name`
  (standard HTML multi-value pattern, like `<select multiple>`).
- **None mode**: No hidden inputs rendered (toolbar-only, no form value).
- **Disabled**: When the group is disabled, `hidden_input_config()` returns
  `Some(HiddenInputConfig { disabled: true, .. })` so the adapter can decide whether to render
  disabled hidden inputs (rather than returning `None` and losing the field entirely).
- **Reset**: The `Reset` event restores `ctx.value` to `props.default_value`, which also
  updates the hidden input values accordingly.
- **Validation**: `aria-invalid` and `aria-required` on the root element communicate
  validation state to assistive technology. The `invalid` and `required` props are typically
  set by the `Field` component wrapping the group.

The adapter renders hidden inputs using `hidden_input_attrs()` or `multi_hidden_input_attrs()`
from `ars-forms`, based on the `HiddenInputConfig` returned by `Api::hidden_input_config()`.

**FieldCtx merge:** When used inside a `Field`, the adapter merges `disabled`/`invalid`/`required`
from `FieldCtx` (per `07-forms.md` §12.6), identical to the pattern documented for ToggleButton.

## 6. Library Parity

> Compared against: Ark UI (`ToggleGroup`), Radix UI (`ToggleGroup`), React Aria (`ToggleButtonGroup`).

### 6.1 Props

| Feature           | ars-ui                          | Ark UI                   | Radix UI                     | React Aria               | Notes                                                |
| ----------------- | ------------------------------- | ------------------------ | ---------------------------- | ------------------------ | ---------------------------------------------------- |
| Controlled value  | `value: Option<BTreeSet<Key>>`  | `value`                  | `value`                      | `selectedKeys`           | ars-ui uses `BTreeSet<Key>`, RA uses `Iterable<Key>` |
| Default value     | `default_value`                 | `defaultValue`           | `defaultValue`               | `defaultSelectedKeys`    | Same concept, different naming                       |
| Selection mode    | `selection_mode: SelectionMode` | `multiple: bool`         | `type: "single"\|"multiple"` | `selectionMode`          | ars-ui adds `None` mode for toolbar-only groups      |
| Disabled          | `disabled`                      | `disabled`               | `disabled`                   | `isDisabled`             | All libraries                                        |
| Orientation       | `orientation`                   | `orientation`            | `orientation`                | `orientation`            | All libraries                                        |
| Loop focus        | `loop_focus`                    | `loopFocus`              | `loop`                       | --                       | React Aria does not expose this                      |
| Roving focus      | `roving_focus`                  | `rovingFocus`            | `rovingFocus`                | --                       | React Aria uses roving focus implicitly              |
| Disallow empty    | `disallow_empty_selection`      | `deselectable` (inverse) | --                           | `disallowEmptySelection` | Ark uses inverse boolean naming                      |
| Dir               | `dir`                           | --                       | `dir`                        | --                       | Ark does not expose dir directly                     |
| Per-item disabled | `disabled_items: BTreeSet<Key>` | --                       | --                           | --                       | ars-ui addition for granular control                 |
| Read-only         | `read_only`                     | --                       | --                           | --                       | ars-ui addition                                      |
| Form name         | `name`                          | --                       | --                           | --                       | ars-ui form integration                              |
| Invalid/Required  | `invalid`, `required`           | --                       | --                           | --                       | ars-ui form integration                              |

**Gaps:** None. ars-ui is a superset of all three references.

### 6.2 Anatomy

| Part      | ars-ui           | Ark UI | Radix UI | React Aria          | Notes                                          |
| --------- | ---------------- | ------ | -------- | ------------------- | ---------------------------------------------- |
| Root      | `Root`           | `Root` | `Root`   | `ToggleButtonGroup` | All libraries                                  |
| Item      | `Item { value }` | `Item` | `Item`   | `ToggleButton`      | All libraries                                  |
| Indicator | `Indicator`      | --     | --       | --                  | ars-ui addition (animated selection highlight) |

**Gaps:** None.

### 6.3 Events

| Callback     | ars-ui                         | Ark UI          | Radix UI        | React Aria          | Notes                        |
| ------------ | ------------------------------ | --------------- | --------------- | ------------------- | ---------------------------- |
| Value change | `Bindable` change notification | `onValueChange` | `onValueChange` | `onSelectionChange` | ars-ui uses Bindable pattern |

**Gaps:** None.

### 6.4 Features

| Feature             | ars-ui | Ark UI | Radix UI | React Aria |
| ------------------- | ------ | ------ | -------- | ---------- |
| Single selection    | Yes    | Yes    | Yes      | Yes        |
| Multiple selection  | Yes    | Yes    | Yes      | Yes        |
| None mode (toolbar) | Yes    | --     | --       | --         |
| Roving tabindex     | Yes    | Yes    | Yes      | Yes        |
| Loop focus          | Yes    | Yes    | Yes      | --         |
| RTL support         | Yes    | --     | Yes      | Yes        |
| Form integration    | Yes    | --     | --       | --         |
| Indicator part      | Yes    | --     | --       | --         |
| Keyboard navigation | Yes    | Yes    | Yes      | Yes        |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity -- ars-ui is a superset.
- **Divergences:** ars-ui uses `SelectionMode` enum (None/Single/Multiple) vs Ark's `multiple: bool` / Radix's `type: "single"|"multiple"`. ars-ui adds `None` mode for toolbar-only use. ars-ui uses `BTreeSet<Key>` instead of `string[]`.
- **Recommended additions:** None.
