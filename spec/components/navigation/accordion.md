---
component: Accordion
category: navigation
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: []
references:
    ark-ui: Accordion
    radix-ui: Accordion
    react-aria: DisclosureGroup
---

# Accordion

An expandable/collapsible panel group. The machine itself is stateless in terms of a discriminated
state enum — all meaningful state lives in `Context`. Each panel's open/closed condition
is derived from the `value` field (the set of currently open item keys).

**Note**: `Accordion` maps to React Aria's `DisclosureGroup`.

## 1. State Machine

### 1.1 States

The accordion machine uses a single `Idle` state. Per-item open/closed status is carried
in context, not in the state discriminant, because the set of items is dynamic.

| State  | Description                                                             |
| ------ | ----------------------------------------------------------------------- |
| `Idle` | The only machine state; item visibility is tracked in `Context::value`. |

### 1.2 Events

| Event                         | Payload       | Description                                                                                                          |
| ----------------------------- | ------------- | -------------------------------------------------------------------------------------------------------------------- |
| `ExpandItem(Key)`             | item key      | Open a specific item.                                                                                                |
| `CollapseItem(Key)`           | item key      | Close a specific item.                                                                                               |
| `ToggleItem(Key)`             | item key      | Open if closed; close if open.                                                                                       |
| `ExpandAll`                   | —             | Open every registered item (only useful when `multiple=true`).                                                       |
| `CollapseAll`                 | —             | Close every open item.                                                                                               |
| `Focus { item, is_keyboard }` | `Key`, `bool` | Move keyboard focus to a trigger button.                                                                             |
| `Blur { item }`               | `Key`         | Remove focus from a trigger button.                                                                                  |
| `RequestFocus { target_id }`  | `String`      | Request the adapter to move DOM focus to the target element. Emits a `PendingEffect`; core never calls DOM directly. |

### 1.3 Context

```rust
use ars_core::Bindable;
use ars_collections::Key;
use ars_i18n::{Orientation, Direction};

/// Context for the `Accordion` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Set of currently open item keys — controlled or uncontrolled.
    pub value: Bindable<BTreeSet<Key>>,
    /// Which item trigger currently holds focus (used for keyboard navigation).
    pub focused_item: Option<Key>,
    /// True when the focused item received focus via keyboard.
    pub focus_visible: bool,
    /// Allow multiple items to be open simultaneously.
    /// When `true`, the Accordion root element sets `aria-multiselectable="true"`.
    pub multiple: bool,
    /// In single mode, allow the open item to be closed (value becomes empty).
    pub collapsible: bool,
    /// Disable all triggers.
    pub disabled: bool,
    /// `Horizontal` renders an accordion whose items stack left-to-right;
    /// `Vertical` (default) stacks top-to-bottom.
    pub orientation: Orientation,
    /// Text direction — used for RTL-aware arrow key handling in horizontal orientation.
    pub dir: Direction,
    /// Registered item keys in DOM order (populated at mount by each Item part).
    pub items: Vec<Key>,
    /// Per-item disabled flags (keyed by item key).
    pub disabled_items: BTreeMap<Key, bool>,
    /// Generated IDs for sub-parts (trigger, content, etc.).
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
use ars_core::Bindable;
use ars_collections::Key;
use ars_i18n::{Orientation, Direction};

/// Props for the `Accordion` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Unique component identifier.
    pub id: String,
    /// Controlled value: set of open item keys.
    pub value: Option<BTreeSet<Key>>,
    /// Initial open items when uncontrolled.
    pub default_value: BTreeSet<Key>,
    /// Allow multiple items open at once.
    pub multiple: bool,
    /// In single mode, allow closing the last open item.
    pub collapsible: bool,
    /// Disable the entire accordion.
    pub disabled: bool,
    /// Layout orientation.
    pub orientation: Orientation,
    /// Text direction (LTR or RTL).
    pub dir: Direction,
    /// When true, content panels are not mounted until the item is first opened.
    /// Reduces initial DOM size for large accordions. Default: false.
    pub lazy_mount: bool,
    /// When true, content panels are removed from the DOM when their item closes.
    /// Works with `Presence` for exit animations. Default: false.
    pub unmount_on_exit: bool,
    /// Heading level for the wrapper element around each item's trigger button.
    /// The adapter renders each trigger inside an `<h{heading_level}>` element to
    /// provide proper document outline structure (WCAG 1.3.1 Info and Relationships).
    /// Valid values: 2-6. Values outside this range are clamped.
    /// Default: `3`.
    /// Alternatively, if a `HeadingLevelProvider` context is available in the component
    /// tree, the accordion consumes its current level and increments for item headers.
    /// An explicit `heading_level` prop takes precedence over the context provider.
    pub heading_level: u8,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: BTreeSet::new(),
            multiple: false,
            collapsible: false,
            disabled: false,
            orientation: Orientation::Vertical,
            dir: Direction::Ltr,
            lazy_mount: false,
            unmount_on_exit: false,
            heading_level: 3,
        }
    }
}
```

When `lazy_mount` is true, the adapter wraps each item's content in a conditional
that checks whether the item has _ever_ been opened. When `unmount_on_exit` is true,
the adapter composes the `Presence` utility to animate the exit before removing
the content from the DOM. Both props can be combined: `lazy_mount` defers the first
mount, and `unmount_on_exit` removes it again after closing.

**Per-item disabled state**: To disable an individual item, include it in the `disabled_items` map in Context: `disabled_items: BTreeMap::from([(Key::String("item-2".into()), true)])`. Disabled items cannot be expanded or collapsed, and their triggers are skipped during keyboard navigation.

### 1.5 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, PendingEffect, Bindable, AttrMap};
use ars_collections::Key;
use ars_i18n::{Orientation, Direction};
use alloc::collections::BTreeSet;

// ── States ───────────────────────────────────────────────────────────────────

/// Design note: `Accordion` uses a single-variant enum rather than `type State = ()`
/// for `Machine` trait conformance. The `Machine` trait requires `State: Clone + Debug + PartialEq`
/// with named variants for potential future extension (e.g., an Animating state).
/// All meaningful state lives in `Context` (the `value: Bindable<BTreeSet<Key>>`).
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// The idle state.
    Idle,
}

// ── Events ───────────────────────────────────────────────────────────────────

/// Events for the `Accordion` component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Expand an item.
    ExpandItem(Key),
    /// Collapse an item.
    CollapseItem(Key),
    /// Toggle an item.
    ToggleItem(Key),
    /// Expand all items.
    ExpandAll,
    /// Collapse all items.
    CollapseAll,
    /// Focus an item.
    Focus {
        /// The item to focus.
        item: Key,
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// Blur an item.
    Blur {
        /// The item to blur.
        item: Key,
    },
    /// Request the adapter to move DOM focus to the element with `target_id`.
    /// The core machine MUST NOT call DOM methods directly; focus is an adapter effect.
    RequestFocus {
        /// The ID of the target element to focus.
        target_id: String,
    },
}

// ── Machine ──────────────────────────────────────────────────────────────────

/// Machine for the `Accordion` component.
pub struct Machine;

/// This component has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Messages;
impl ComponentMessages for Messages {}

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Props, _env: &Env, _messages: &Messages) -> (State, Context) {
        let value = match &props.value {
            Some(v) => Bindable::controlled(v.clone()),
            None    => Bindable::uncontrolled(props.default_value.clone()),
        };
        (State::Idle, Context {
            value,
            focused_item: None,
            focus_visible: false,
            multiple: props.multiple,
            collapsible: props.collapsible,
            disabled: props.disabled,
            orientation: props.orientation,
            dir: props.dir,
            items: Vec::new(),
            disabled_items: alloc::collections::BTreeMap::new(),
            ids: ComponentIds::from_id(&props.id),
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // Machine-level disabled guard for mutation events
        match event {
            Event::ExpandItem(id) | Event::CollapseItem(id) | Event::ToggleItem(id) => {
                if ctx.disabled || *ctx.disabled_items.get(id).unwrap_or(&false) {
                    return None;
                }
            }
            _ => {}
        }

        match (state, event) {

            // ── ExpandItem ────────────────────────────────────────────────────
            (State::Idle, Event::ExpandItem(id)) => {
                let id = id.clone();
                let multiple = ctx.multiple;
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut current = ctx.value.get().clone();
                    if multiple {
                        current.insert(id);
                    } else {
                        // Single mode: replace with the new item.
                        current = BTreeSet::from([id]);
                    }
                    ctx.value.set(current);
                }))
            }

            // ── CollapseItem ──────────────────────────────────────────────────
            // Idempotent: collapsing an already-closed item returns None (no-op).
            // This is intentional — callers need not check open state before sending
            // CollapseItem. The transition is a no-op rather than an error.
            (State::Idle, Event::CollapseItem(id)) => {
                // Guard: item is not currently open — idempotent no-op.
                if !ctx.value.get().contains(id) {
                    return None;
                }
                let id = id.clone();
                let collapsible = ctx.collapsible;
                let multiple    = ctx.multiple;
                let current_len = ctx.value.get().len();
                // Guard: in single mode with collapsible=false, do nothing when
                //        the target item is the only open item.
                if !multiple && !collapsible && current_len <= 1 {
                    return None;
                }
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut current = ctx.value.get().clone();
                    current.remove(&id);
                    ctx.value.set(current);
                }))
            }

            // ── ToggleItem ────────────────────────────────────────────────────
            (State::Idle, Event::ToggleItem(id)) => {
                // Guard: disabled items return no-op instead of producing an
                // empty transition plan (defense-in-depth; the top-level guard
                // already covers this, but an explicit check here prevents
                // accidental fall-through if the top guard is refactored).
                if ctx.disabled_items.contains_key(id)
                    && *ctx.disabled_items.get(id).unwrap_or(&false)
                {
                    return None;
                }
                let is_open = ctx.value.get().contains(id);
                let id = id.clone();
                if is_open {
                    // Delegate to collapse logic: respect collapsible guard.
                    let collapsible = ctx.collapsible;
                    let multiple    = ctx.multiple;
                    let current_len = ctx.value.get().len();
                    if !multiple && !collapsible && current_len <= 1 {
                        return None;
                    }
                    Some(TransitionPlan::context_only(move |ctx| {
                        let mut current = ctx.value.get().clone();
                        current.remove(&id);
                        ctx.value.set(current);
                    }))
                } else {
                    let multiple = ctx.multiple;
                    Some(TransitionPlan::context_only(move |ctx| {
                        let mut current = ctx.value.get().clone();
                        if multiple {
                            current.insert(id);
                        } else {
                            current = BTreeSet::from([id]);
                        }
                        ctx.value.set(current);
                    }))
                }
            }

            // ── ExpandAll ─────────────────────────────────────────────────────
            // Bulk operations respect both global and per-item disabled state.
            (State::Idle, Event::ExpandAll) => {
                if ctx.disabled { return None; }
                if !ctx.multiple { return None; }
                // Filter out items that are individually disabled.
                let expandable_items: BTreeSet<Key> = ctx.items.iter()
                    .filter(|id| !*ctx.disabled_items.get(*id).unwrap_or(&false))
                    .cloned()
                    .collect();
                // Merge with currently open items (disabled items that are already
                // open remain open; we just don't add new disabled items).
                let current = ctx.value.get().clone();
                let merged: BTreeSet<Key> = current.union(&expandable_items).cloned().collect();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(merged);
                }))
            }

            // ── CollapseAll ───────────────────────────────────────────────────
            // Bulk operations respect both global and per-item disabled state.
            (State::Idle, Event::CollapseAll) => {
                if ctx.disabled { return None; }
                if !ctx.multiple && !ctx.collapsible && !ctx.value.get().is_empty() {
                    return None; // cannot collapse the last item when collapsible is false
                }
                // Filter out disabled items from collapsing — they remain in their
                // current open/closed state.
                let disabled_and_open: BTreeSet<Key> = ctx.value.get().iter()
                    .filter(|id| *ctx.disabled_items.get(*id).unwrap_or(&false))
                    .cloned()
                    .collect();
                Some(TransitionPlan::context_only(move |ctx| {
                    // Keep disabled items that were open; close everything else.
                    ctx.value.set(disabled_and_open);
                }))
            }

            // ── Focus ─────────────────────────────────────────────────────────
            (State::Idle, Event::Focus { item, is_keyboard }) => {
                let item       = item.clone();
                let is_keyboard = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused_item  = Some(item);
                    ctx.focus_visible = is_keyboard;
                }))
            }

            // ── Blur ──────────────────────────────────────────────────────────
            (State::Idle, Event::Blur { .. }) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused_item  = None;
                    ctx.focus_visible = false;
                }))
            }

            // ── RequestFocus ─────────────────────────────────────────────────
            // Core machine does NOT call DOM methods. Instead, it emits
            // a PendingEffect that the adapter executes to move focus.
            (_, Event::RequestFocus { target_id }) => {
                let target_id = target_id.clone();
                Some(TransitionPlan::context_only(|_| {})
                    .with_effect(PendingEffect::new("focus-element", move |_ctx, _props, _send| {
                        let platform = use_platform_effects();
                        platform.focus_element_by_id(&target_id);
                        no_cleanup()
                    })))
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
#[scope = "accordion"]
pub enum Part {
    Root,
    Item { item_key: Key },
    ItemHeader { item_key: Key },
    ItemTrigger { item_key: Key, content_id: String },
    ItemIndicator { item_key: Key },
    ItemContent { item_key: Key, content_id: String, trigger_id: String },
}

/// API for the `Accordion` component.
pub struct Api<'a> {
    /// The state of the `Accordion` component.
    state: &'a State,
    /// The context of the `Accordion` component.
    ctx:   &'a Context,
    /// The props of the `Accordion` component.
    props: &'a Props,
    /// The send function for the `Accordion` component.
    send:  &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Returns true if the item with the given key is currently open.
    pub fn is_item_open(&self, item_key: &Key) -> bool {
        self.ctx.value.get().contains(item_key)
    }

    /// Returns true if the given item is disabled (either globally or individually).
    pub fn is_item_disabled(&self, item_key: &Key) -> bool {
        self.ctx.disabled
            || *self.ctx.disabled_items.get(item_key).unwrap_or(&false)
    }

    /// Attrs for the root container element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-orientation"), match self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical   => "vertical",
        });
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        attrs
    }

    /// Attrs for an individual item wrapper element.
    ///
    /// `item_key` is the unique identifier for this accordion item.
    pub fn item_attrs(&self, item_key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_open = self.is_item_open(item_key);
        let is_disabled = self.is_item_disabled(item_key);
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item { item_key: Default::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if is_open { "open" } else { "closed" });
        if is_disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        attrs
    }

    /// Returns the heading level for item trigger wrapper elements.
    /// The adapter wraps each trigger `<button>` inside an `<h{level}>` element.
    /// Resolves from: explicit prop > HeadingLevelProvider context > default (3).
    pub fn heading_level(&self) -> u8 {
        self.props.heading_level.clamp(2, 6)
    }

    /// Attrs for the heading wrapper element around each item trigger.
    /// The adapter renders this as `<h{heading_level()}>` with these attributes.
    pub fn item_header_attrs(&self, item_key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemHeader { item_key: Default::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Attrs for the trigger `<button>` inside an item.
    ///
    /// `item_key` — the item this trigger belongs to.
    /// `content_id` — the ID of the associated content region (for `aria-controls`).
    pub fn item_trigger_attrs(&self, item_key: &Key, content_id: &str) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_open     = self.is_item_open(item_key);
        let is_disabled = self.is_item_disabled(item_key);
        let is_focused  = self.ctx.focused_item.as_ref() == Some(item_key);

        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemTrigger { item_key: Key::default(), content_id: String::new() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if is_open { "open" } else { "closed" });
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if is_open { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), content_id);
        if is_disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        if is_focused && self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        attrs
    }

    /// Handle click event on the item trigger.
    pub fn on_item_trigger_click(&self, item_key: &Key) {
        if !self.is_item_disabled(item_key) {
            (self.send)(Event::ToggleItem(item_key.clone()));
        }
    }

    /// Handle focus event on the item trigger.
    pub fn on_item_trigger_focus(&self, item_key: &Key, is_keyboard: bool) {
        (self.send)(Event::Focus { item: item_key.clone(), is_keyboard });
    }

    /// Handle blur event on the item trigger.
    pub fn on_item_trigger_blur(&self, item_key: &Key) {
        (self.send)(Event::Blur { item: item_key.clone() });
    }

    // RTL-aware arrow key resolution for horizontal orientation.
    // When `dir` is RTL, ArrowLeft and ArrowRight are swapped.
    fn resolve_horizontal_key(key: KeyboardKey, is_rtl: bool) -> Option<&'static str> {
        match (key, is_rtl) {
            (KeyboardKey::ArrowLeft, false) | (KeyboardKey::ArrowRight, true) => Some("Prev"),
            (KeyboardKey::ArrowRight, false) | (KeyboardKey::ArrowLeft, true) => Some("Next"),
            _ => None,
        }
    }

    /// Returns the enabled items (filters out disabled triggers).
    fn enabled_items(&self) -> Vec<&Key> {
        self.ctx.items.iter()
            .filter(|id| !self.is_item_disabled(id))
            .collect()
    }

    pub fn on_item_trigger_keydown(&self, item_key: &Key, data: &KeyboardEventData) {
        // Keyboard navigation (ArrowUp/ArrowDown, Home/End) MUST skip disabled
        // triggers. When computing the next/previous trigger index, filter out
        // items where disabled == true. If all items are disabled, navigation
        // is a no-op.
        //
        // NOTE: Focus is requested via Event::RequestFocus, NOT by calling DOM
        // methods directly. The adapter handles the actual DOM focus in the
        // resulting PendingEffect.
        let enabled = self.enabled_items();
        if enabled.is_empty() { return; }
        let idx = enabled.iter().position(|x| **x == *item_key).unwrap_or(0);
        let len = enabled.len();
        let is_rtl = self.ctx.dir == Direction::Rtl;
        let (prev_key, next_key) = match self.ctx.orientation {
            Orientation::Vertical   => (KeyboardKey::ArrowUp, KeyboardKey::ArrowDown),
            Orientation::Horizontal => {
                let resolved = Self::resolve_horizontal_key(data.key, is_rtl);
                if resolved == Some("Prev") {
                    let prev_idx = if idx == 0 { len - 1 } else { idx - 1 };
                    if let Some(prev) = enabled.get(prev_idx) {
                        (self.send)(Event::RequestFocus { target_id: prev.to_string() });
                        (self.send)(Event::Focus { item: (*prev).clone(), is_keyboard: true });
                    }
                    return;
                } else if resolved == Some("Next") {
                    if let Some(next) = enabled.get((idx + 1) % len) {
                        (self.send)(Event::RequestFocus { target_id: next.to_string() });
                        (self.send)(Event::Focus { item: (*next).clone(), is_keyboard: true });
                    }
                    return;
                }
                (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight)
            }
        };
        if data.key == next_key {
            if let Some(next) = enabled.get((idx + 1) % len) {
                (self.send)(Event::RequestFocus { target_id: next.to_string() });
                (self.send)(Event::Focus { item: (*next).clone(), is_keyboard: true });
            }
        } else if data.key == prev_key {
            let prev_idx = if idx == 0 { len - 1 } else { idx - 1 };
            if let Some(prev) = enabled.get(prev_idx) {
                (self.send)(Event::RequestFocus { target_id: prev.to_string() });
                (self.send)(Event::Focus { item: (*prev).clone(), is_keyboard: true });
            }
        } else if data.key == KeyboardKey::Home {
            if let Some(first) = enabled.first() {
                (self.send)(Event::RequestFocus { target_id: first.to_string() });
                (self.send)(Event::Focus { item: (*first).clone(), is_keyboard: true });
            }
        } else if data.key == KeyboardKey::End {
            if let Some(last) = enabled.last() {
                (self.send)(Event::RequestFocus { target_id: last.to_string() });
                (self.send)(Event::Focus { item: (*last).clone(), is_keyboard: true });
            }
        }
    }

    /// Attrs for the chevron/indicator element inside a trigger.
    pub fn item_indicator_attrs(&self, item_key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_open = self.is_item_open(item_key);
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemIndicator { item_key: Default::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if is_open { "open" } else { "closed" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attrs for the collapsible content region.
    ///
    /// `item_key` — the item this content belongs to.
    /// `content_id` — the element ID (must match `aria-controls` in trigger).
    /// `trigger_id` — the element ID of the associated trigger (for `aria-labelledby`).
    pub fn item_content_attrs(&self, item_key: &Key, content_id: &str, trigger_id: &str) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_open = self.is_item_open(item_key);
        attrs.set(HtmlAttr::Id, content_id);
        attrs.set(HtmlAttr::Role, "region");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemContent { item_key: Key::default(), content_id: String::new(), trigger_id: String::new() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if is_open { "open" } else { "closed" });
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), trigger_id);
        if !is_open {
            // `hidden="until-found"` enables browser find-in-page to reveal
            // collapsed accordion content. However, it is only supported in
            // Chromium-based browsers (Chrome 102+, Edge 102+). For Firefox
            // and Safari, the adapter MUST fall back to the boolean `hidden`
            // attribute, which fully hides the content from display.
            //
            // Feature detection: check if `HTMLElement.prototype` has a
            // `'until-found'`-aware `hidden` setter, or use:
            //   typeof document.createElement('div').hidden === 'string'
            // after setting el.hidden = 'until-found'. If the browser
            // collapses it to boolean `true`, use `hidden` (boolean) instead.
            //
            // When using boolean `hidden`, the `beforematch` event will not
            // fire, so find-in-page will not auto-expand collapsed sections.
            attrs.set(HtmlAttr::Hidden, "until-found");
        }
        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match &part {
            Part::Root => self.root_attrs(),
            Part::Item { item_key } => self.item_attrs(item_key),
            Part::ItemHeader { item_key } => self.item_header_attrs(item_key),
            Part::ItemTrigger { item_key, content_id } => self.item_trigger_attrs(item_key, content_id),
            Part::ItemIndicator { item_key } => self.item_indicator_attrs(item_key),
            Part::ItemContent { item_key, content_id, trigger_id } => self.item_content_attrs(item_key, content_id, trigger_id),
        }
    }
}
```

## 2. Anatomy

```text
Accordion
├── Root                   data-ars-scope="accordion" data-ars-part="root"
└── Item (×N)              data-ars-scope="accordion" data-ars-part="item"
    ├── ItemTrigger        data-ars-scope="accordion" data-ars-part="item-trigger"
    │   └── ItemIndicator  data-ars-scope="accordion" data-ars-part="item-indicator"
    └── ItemContent        data-ars-scope="accordion" data-ars-part="item-content"
```

| Part            | Element    | Key Attributes                                                                                                           |
| --------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------ |
| `Root`          | `<div>`    | `data-ars-scope="accordion"`, `data-ars-part="root"`, `data-ars-orientation`                                             |
| `Item`          | `<div>`    | `data-ars-scope="accordion"`, `data-ars-part="item"`, `data-ars-state="open\|closed"`, `data-ars-disabled`               |
| `ItemTrigger`   | `<button>` | `data-ars-scope="accordion"`, `data-ars-part="item-trigger"`, `aria-expanded`, `aria-controls`, `data-ars-focus-visible` |
| `ItemIndicator` | `<span>`   | `data-ars-scope="accordion"`, `data-ars-part="item-indicator"`, `aria-hidden="true"`                                     |
| `ItemContent`   | `<div>`    | `data-ars-scope="accordion"`, `data-ars-part="item-content"`, `role="region"`, `aria-labelledby`                         |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part          | Role              | Properties                                                                                   |
| ------------- | ----------------- | -------------------------------------------------------------------------------------------- |
| `Root`        | (none / `<div>`)  | `aria-orientation` when horizontal layout matters                                            |
| `ItemTrigger` | `button` (native) | `aria-expanded="true\|false"`, `aria-controls="{content-id}"`, `aria-disabled` when disabled |
| `ItemContent` | `region`          | `aria-labelledby="{trigger-id}"`, `hidden` when closed                                       |

### 3.2 Keyboard Interaction

| Key                            | Behavior                                           |
| ------------------------------ | -------------------------------------------------- |
| `Enter` / `Space`              | Toggle the focused item open or closed.            |
| `ArrowDown` (vertical)         | Move focus to the next trigger; wraps to first.    |
| `ArrowUp` (vertical)           | Move focus to the previous trigger; wraps to last. |
| `ArrowRight` (horizontal, LTR) | Move focus to the next trigger.                    |
| `ArrowLeft` (horizontal, LTR)  | Move focus to the previous trigger.                |
| `ArrowRight` (horizontal, RTL) | Move focus to the **previous** trigger (swapped).  |
| `ArrowLeft` (horizontal, RTL)  | Move focus to the **next** trigger (swapped).      |
| `Home`                         | Move focus to the first trigger.                   |
| `End`                          | Move focus to the last trigger.                    |

Focus moves between trigger buttons only. The content region itself is not in the tab sequence
(it receives focus via standard tabbing of its children when open).

### 3.3 Scroll Position Preservation

When keyboard focus moves between `Accordion` triggers (or Tab triggers — see §2), the browser's default `focus()` call may scroll the viewport unexpectedly. To preserve the user's scroll context:

1. **Save** `window.scrollX` and `window.scrollY` (or the scroll container's `scrollTop`/`scrollLeft`) **before** calling `element.focus()`.
2. **After** focus completes, check whether the newly focused element is still within the visible viewport (via `getBoundingClientRect()`).
3. If the element **is** in the viewport, **restore** the saved scroll position to undo any browser-initiated scroll.
4. If the element is **not** in the viewport (e.g., `Accordion` trigger below the fold), use `element.scrollIntoView({ block: 'nearest', inline: 'nearest' })` to scroll minimally.

**Conflict with native anchor scroll:** When `Accordion`/`Tabs` items have `id` attributes that match URL hash fragments, the browser may auto-scroll on page load. Adapters MUST NOT call `scrollIntoView` during initial mount if the focus was triggered by hash navigation — defer to the browser's native behavior.

> **RTL Handling**: Horizontal keyboard navigation follows the canonical RTL matrix defined in `03-accessibility.md` section "Canonical RTL Keyboard Navigation Matrix". Vertical accordion is unaffected by text direction.

## 4. Internationalization

- **RTL**: When `dir="rtl"`, the `ArrowLeft`/`ArrowRight` meanings for horizontal accordions are
  swapped via `if ctx.dir.is_rtl() { swap }` so that `ArrowRight` moves to the visually
  previous trigger and `ArrowLeft` moves to the visually next trigger. This is the canonical
  RTL rule: in RTL horizontal layouts, ArrowRight/ArrowLeft meanings flip to match physical
  layout. The `data-ars-orientation` attribute remains `horizontal`; the direction flip is
  handled by the keyboard handler reading `ctx.dir` (see `resolve_horizontal_key()` at line 482).
- **Text direction**: The Root part should propagate `dir` to the DOM element so nested text
  renders correctly.
- **No locale-specific strings** are emitted by `Accordion` itself; all visible labels are
  provided by the consumer.

> **Content height animation:** To animate content height, adapters SHOULD use CSS `grid-template-rows: 0fr` → `1fr` transition (Chrome 117+, Firefox 117+, Safari 17.2+) which avoids JavaScript measurement. Alternatively, batch all `scrollHeight` reads before any style writes in a single frame to avoid layout thrashing. For browsers supporting `interpolate-size: allow-keywords`, prefer this zero-JS solution.

## 5. Disclosure Pattern

A **Disclosure** is a single expandable/collapsible section — equivalent to React Aria's `useDisclosure`. Rather than defining a separate component, ars-ui implements Disclosure as a constrained Accordion configuration:

```rust
/// Create a Disclosure by configuring Accordion with a single item.
let disclosure_props = accordion::Props {
    id: "my-disclosure".into(),
    multiple: false,
    collapsible: true, // single item can be closed
    default_value: BTreeSet::new(), // starts collapsed
    ..Default::default()
};
```

**Key differences from full `Accordion`:**

- **Single item only**: The consumer registers exactly one item. The `Accordion` machine handles this naturally — no special casing is needed.
- **`aria-expanded`**: The single trigger button carries `aria-expanded="true|false"`, which the `Accordion` trigger already emits.
- **No `aria-multiselectable`**: Since `multiple` is false, the root does not set `aria-multiselectable`.
- **Programmatic control**: Use `Event::ExpandItem(id)` / `Event::CollapseItem(id)` to programmatically open/close.

A **DisclosureGroup** is simply an `Accordion` with `multiple: false` and `collapsible: true` — only one item can be open at a time, and the open item can be closed. This maps directly to React Aria's `useDisclosureGroup`.

## 6. Library Parity

> Compared against: Ark UI (`Accordion`), Radix UI (`Accordion`), React Aria (`DisclosureGroup`).

### 6.1 Props

| Feature           | ars-ui                     | Ark UI              | Radix UI            | React Aria                  | Notes                                 |
| ----------------- | -------------------------- | ------------------- | ------------------- | --------------------------- | ------------------------------------- |
| Controlled value  | `value`                    | `value`             | `value`             | `expandedKeys`              | Same concept                          |
| Default value     | `default_value`            | `defaultValue`      | `defaultValue`      | `defaultExpandedKeys`       | Same concept                          |
| Multiple          | `multiple`                 | `multiple`          | `type="multiple"`   | `allowsMultipleExpanded`    | Radix uses `type` prop instead        |
| Collapsible       | `collapsible`              | `collapsible`       | `collapsible`       | --                          | React Aria always collapsible         |
| Disabled (global) | `disabled`                 | `disabled`          | `disabled`          | `isDisabled`                | Full match                            |
| Orientation       | `orientation`              | `orientation`       | `orientation`       | --                          | React Aria has no orientation         |
| Dir               | `dir`                      | --                  | `dir`               | --                          | ars-ui and Radix have RTL             |
| Lazy mount        | `lazy_mount`               | `lazyMount`         | --                  | --                          | Radix uses `forceMount` per-content   |
| Unmount on exit   | `unmount_on_exit`          | `unmountOnExit`     | `forceMount`        | --                          | Inverse semantics on Radix            |
| Heading level     | `heading_level`            | --                  | -- (Header part)    | --                          | ars-ui prop; Radix has Header anatomy |
| Per-item disabled | `disabled_items` (Context) | per-item `disabled` | per-item `disabled` | per-Disclosure `isDisabled` | All libraries support this            |

**Gaps:** None. ars-ui covers all behaviorally meaningful props.

### 6.2 Anatomy

| Part           | ars-ui          | Ark UI          | Radix UI  | React Aria         | Notes                    |
| -------------- | --------------- | --------------- | --------- | ------------------ | ------------------------ |
| Root           | `Root`          | `Root`          | `Root`    | `DisclosureGroup`  | Full match               |
| Item           | `Item`          | `Item`          | `Item`    | `Disclosure`       | Full match               |
| Item header    | `ItemHeader`    | --              | `Header`  | `DisclosureHeader` | Full match               |
| Item trigger   | `ItemTrigger`   | `ItemTrigger`   | `Trigger` | (button in Header) | Full match               |
| Item indicator | `ItemIndicator` | `ItemIndicator` | --        | --                 | ars-ui and Ark have this |
| Item content   | `ItemContent`   | `ItemContent`   | `Content` | `DisclosurePanel`  | Full match               |

**Gaps:** None.

### 6.3 Events

| Callback     | ars-ui                | Ark UI          | Radix UI        | React Aria         | Notes                        |
| ------------ | --------------------- | --------------- | --------------- | ------------------ | ---------------------------- |
| Value change | `Bindable` onChange   | `onValueChange` | `onValueChange` | `onExpandedChange` | ars-ui uses Bindable pattern |
| Focus change | `Focus`/`Blur` events | `onFocusChange` | --              | --                 | ars-ui and Ark track focus   |

**Gaps:** None.

### 6.4 Features

| Feature                  | ars-ui                      | Ark UI                 | Radix UI               | React Aria            |
| ------------------------ | --------------------------- | ---------------------- | ---------------------- | --------------------- |
| Single/Multiple mode     | Yes                         | Yes                    | Yes                    | Yes                   |
| Collapsible single       | Yes                         | Yes                    | Yes                    | Yes (always)          |
| Per-item disabled        | Yes                         | Yes                    | Yes                    | Yes                   |
| Global disabled          | Yes                         | Yes                    | Yes                    | Yes                   |
| Keyboard navigation      | Yes                         | Yes                    | Yes                    | Yes                   |
| RTL support              | Yes                         | Yes                    | Yes                    | No                    |
| Orientation              | Yes                         | Yes                    | Yes                    | No                    |
| Lazy mount / unmount     | Yes                         | Yes                    | forceMount             | No                    |
| Heading level control    | Yes                         | No                     | Header part            | No                    |
| Content height animation | CSS vars guidance           | CSS vars               | CSS vars               | No                    |
| Disclosure pattern       | Yes (single-item Accordion) | Collapsible (separate) | Collapsible (separate) | Disclosure (separate) |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui models Disclosure as a constrained Accordion configuration rather than a separate component (Ark has `Collapsible`, Radix has `Collapsible`, React Aria has `Disclosure`). Radix uses `type="single"|"multiple"` rather than a boolean `multiple` prop.
- **Recommended additions:** None.

Adapters MAY provide a `<Disclosure>` convenience component that wraps `<Accordion>` with the appropriate defaults and a simplified single-item API.
