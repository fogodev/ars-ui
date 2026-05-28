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

| Event                                | Payload        | Description                                                                                                                           |
| ------------------------------------ | -------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `ExpandItem(Key)`                    | item key       | Open a specific item.                                                                                                                 |
| `CollapseItem(Key)`                  | item key       | Close a specific item.                                                                                                                |
| `ToggleItem(Key)`                    | item key       | Open if closed; close if open.                                                                                                        |
| `ExpandAll`                          | —              | Open every registered enabled item (only useful when `multiple=true`).                                                                |
| `CollapseAll`                        | —              | Close every open enabled item; open disabled items remain open.                                                                       |
| `Focus(Key)`                         | item key       | Record a trigger as focused.                                                                                                          |
| `Blur`                               | —              | Clear trigger focus.                                                                                                                  |
| `SetDirection(Direction)`            | direction      | Set the adapter-resolved text direction used by horizontal keyboard navigation.                                                       |
| `FocusNext` / `FocusPrev`            | —              | Move focus intent to the next/previous enabled trigger.                                                                               |
| `FocusFirst` / `FocusLast`           | —              | Move focus intent to the first/last enabled trigger.                                                                                  |
| `SetItems(Vec<ItemRegistration>)`    | registrations  | Replace registered item keys and disabled flags in DOM order.                                                                         |
| `SyncProps`                          | —              | Synchronize prop-backed context fields after render prop changes, including controlled-mode exit and single-mode value normalization. |
| `SyncControlledValue(BTreeSet<Key>)` | open item keys | Push a new controlled open-item set into context, entering controlled mode if needed.                                                 |

Focus-navigation events emit the typed `Effect::FocusFocusedItem` intent. Adapters execute
that effect by focusing their framework-native item handle for `Context::focused_item`
(Leptos `NodeRef`, Dioxus `MountedData` or platform equivalent). Core never calls DOM
focus APIs and does not carry target element ids as event payloads.

### 1.3 Context

```rust
use ars_core::{Bindable, ComponentIds, Direction, Orientation};
use ars_collections::Key;

/// Context for the `Accordion` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Set of currently open item keys — controlled or uncontrolled.
    pub value: Bindable<BTreeSet<Key>>,
    /// Which item trigger currently holds focus (used for keyboard navigation).
    pub focused_item: Option<Key>,
    /// Allow multiple items to be open simultaneously.
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
    /// Heading level for the wrapper element around each item's trigger button.
    pub heading_level: u8,
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
use ars_core::{Bindable, Direction, Orientation};
use ars_collections::Key;

/// Props for the `Accordion` component.
#[derive(Clone, Debug, PartialEq, Eq, HasId)]
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

**Single-mode initial value normalization**: `init` does not store `value`/`default_value` verbatim. Both the controlled `value` and the uncontrolled `default_value` are passed through `normalize_value_for_mode(.., props.multiple)`, which — when `multiple == false` — trims a multi-key initial set down to a single retained key (the first in `BTreeSet` order). This guarantees the single-mode invariant (at most one open item) holds from mount, even when the consumer supplies an over-broad initial set.

### 1.5 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, PendingEffect, Bindable, AttrMap, Direction, Orientation};
use ars_collections::Key;
use alloc::collections::BTreeSet;

// ── States ───────────────────────────────────────────────────────────────────

/// Design note: `Accordion` uses a single-variant enum rather than `type State = ()`
/// for `Machine` trait conformance — the `Machine` trait requires
/// `State: Clone + Debug + PartialEq` with named variants. All meaningful state
/// lives in `Context` (the `value: Bindable<BTreeSet<Key>>`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum State {
    /// The idle state.
    #[default]
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
    Focus(Key),
    /// Blur the current item.
    Blur,
    /// Set the adapter-resolved text direction used for keyboard navigation.
    SetDirection(Direction),
    /// Focus the next enabled item.
    FocusNext,
    /// Focus the previous enabled item.
    FocusPrev,
    /// Focus the first enabled item.
    FocusFirst,
    /// Focus the last enabled item.
    FocusLast,
    /// Replace registered items in DOM order.
    SetItems(Vec<ItemRegistration>),
    /// Synchronize prop-backed context fields, controlled-mode exit, and
    /// single-mode value normalization.
    SyncProps,
    /// Synchronize a controlled value, entering controlled mode if needed.
    SyncControlledValue(BTreeSet<Key>),
}

/// Adapter-supplied registration data for one rendered accordion item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemRegistration {
    /// Stable item key in DOM order.
    pub key: Key,
    /// Whether this item is disabled.
    pub disabled: bool,
}

/// Typed effect intents emitted by the `Accordion` machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter must move DOM focus to `Context::focused_item`.
    FocusFocusedItem,
}

// ── Machine ──────────────────────────────────────────────────────────────────

/// Machine for the `Accordion` component.
pub struct Machine;

/// This component has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Messages;
impl ComponentMessages for Messages {}

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Messages = Messages;
    type Effect = Effect;
    type Api<'a> = Api<'a>;

    fn init(props: &Props, _env: &Env, _messages: &Messages) -> (State, Context) {
        // Both the controlled and uncontrolled initial sets are normalized for the
        // current mode: when `multiple == false`, a multi-key set is trimmed to a
        // single retained key so the single-mode invariant holds from mount.
        let value = match &props.value {
            Some(v) => Bindable::controlled(normalize_value_for_mode(v.clone(), props.multiple)),
            None    => Bindable::uncontrolled(normalize_value_for_mode(
                props.default_value.clone(),
                props.multiple,
            )),
        };
        (State::Idle, Context {
            value,
            focused_item: None,
            multiple: props.multiple,
            collapsible: props.collapsible,
            disabled: props.disabled,
            orientation: props.orientation,
            dir: props.dir,
            heading_level: props.heading_level,
            items: Vec::new(),
            disabled_items: alloc::collections::BTreeMap::new(),
            ids: ComponentIds::from_id(&props.id),
        })
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // Every event has a dedicated arm; none silently fall through to a no-op.
        // The mutation/focus/sync helpers below own their own disabled, registration,
        // and no-op guards so each arm returns `None` precisely when nothing changes.
        match event {
            Event::ExpandItem(item) => expand_item_plan(ctx, item),

            Event::CollapseItem(item) => collapse_item_plan(ctx, item),

            Event::ToggleItem(item) => toggle_item_plan(ctx, item),

            Event::ExpandAll => expand_all_plan(ctx),

            Event::CollapseAll => collapse_all_plan(ctx),

            Event::Focus(item) => focus_item_plan(ctx, item),

            // ── Blur ──────────────────────────────────────────────────────────
            // No-op when no trigger currently holds focus.
            Event::Blur => {
                ctx.focused_item.as_ref()?;
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focused_item = None;
                }))
            }

            // ── SetDirection ──────────────────────────────────────────────────
            // No-op when the resolved direction is unchanged.
            Event::SetDirection(dir) => {
                let dir = *dir;
                if ctx.dir == dir {
                    return None;
                }
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.dir = dir;
                }))
            }

            // ── Focus movement ───────────────────────────────────────────────
            // Core stores the focused item key and emits a typed effect intent.
            // Adapters resolve the key to their native handle and perform DOM focus.
            // Next/Prev use the anchor + step model; First/Last jump to the
            // edge of the enabled set. Each arm returns `None` when there is no
            // distinct enabled target to move to.
            Event::FocusNext => {
                let current = focus_anchor(ctx)?;
                let next = step_focus(ctx, &current, FocusStep::Next)?;
                Some(focus_item_transition(next))
            }

            Event::FocusPrev => {
                let current = focus_anchor(ctx)?;
                let prev = step_focus(ctx, &current, FocusStep::Prev)?;
                Some(focus_item_transition(prev))
            }

            Event::FocusFirst => {
                let first = enabled_items(ctx).next()?;
                Some(focus_item_transition(first))
            }

            Event::FocusLast => {
                let last = enabled_items(ctx).next_back()?;
                Some(focus_item_transition(last))
            }

            // ── SetItems / SyncProps / SyncControlledValue ────────────────────
            // Prop- and registration-sync events. Each is fully handled — none
            // fall through to a silent no-op.
            Event::SetItems(items) => Some(set_items_plan(items)),

            Event::SyncProps => Some(sync_props_plan(ctx, props)),

            Event::SyncControlledValue(value) => Some(sync_controlled_value_plan(ctx, value)),
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

    /// Translate a prop change into the synchronization events the running
    /// machine must process. Emitted in order: `SetDirection` (when `dir`
    /// changed), `SyncProps` (when any prop-backed field changed), and
    /// `SyncControlledValue` / `SyncProps` for the value transition.
    ///
    /// The value branch covers three cases:
    /// - new props are controlled → push the new controlled set;
    /// - new props are uncontrolled and no other prop changed → `SyncProps`
    ///   so the machine performs the controlled → uncontrolled exit;
    /// - new props are uncontrolled and another prop already changed → the
    ///   `SyncProps` emitted above already handles the exit, so nothing extra.
    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        let mut events = Vec::new();

        let props_changed = old.multiple != new.multiple
            || old.collapsible != new.collapsible
            || old.disabled != new.disabled
            || old.orientation != new.orientation
            || old.heading_level != new.heading_level;

        if old.dir != new.dir {
            events.push(Event::SetDirection(new.dir));
        }

        if props_changed {
            events.push(Event::SyncProps);
        }

        if old.value != new.value {
            if let Some(new_value) = &new.value {
                events.push(Event::SyncControlledValue(new_value.clone()));
            } else if !props_changed {
                // Controlled → uncontrolled exit with no other prop change:
                // SyncProps performs the `sync_controlled(None)` handoff.
                events.push(Event::SyncProps);
            }
        }

        events
    }
}

// ── Transition helpers ────────────────────────────────────────────────────────

/// Direction passed to [`step_focus`] for relative focus movement.
#[derive(Clone, Copy)]
enum FocusStep {
    Next,
    Prev,
}

/// Open `item`. No-op when the root is disabled, the item is disabled, or the
/// item is already open. In single mode the open set is replaced by `item`.
fn expand_item_plan(ctx: &Context, item: &Key) -> Option<TransitionPlan<Machine>> {
    if ctx.disabled || item_disabled(ctx, item) || ctx.value.get().contains(item) {
        return None;
    }
    let item = item.clone();
    let multiple = ctx.multiple;
    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        let mut value = ctx.value.get().clone();
        if multiple {
            value.insert(item);
        } else {
            value.clear();
            value.insert(item);
        }
        ctx.value.set(value);
    }))
}

/// Close `item`. No-op when the root/item is disabled, the item is already
/// closed, or it is the only open item in single non-collapsible mode.
fn collapse_item_plan(ctx: &Context, item: &Key) -> Option<TransitionPlan<Machine>> {
    if ctx.disabled || item_disabled(ctx, item) || !ctx.value.get().contains(item) {
        return None;
    }
    if !ctx.multiple && !ctx.collapsible && ctx.value.get().len() <= 1 {
        return None;
    }
    let item = item.clone();
    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        let mut value = ctx.value.get().clone();
        value.remove(&item);
        ctx.value.set(value);
    }))
}

/// Toggle `item` by delegating to [`collapse_item_plan`] when open and
/// [`expand_item_plan`] when closed; both apply their own guards.
fn toggle_item_plan(ctx: &Context, item: &Key) -> Option<TransitionPlan<Machine>> {
    if ctx.value.get().contains(item) {
        collapse_item_plan(ctx, item)
    } else {
        expand_item_plan(ctx, item)
    }
}

/// Open every registered enabled item. No-op when the root is disabled, when
/// not in `multiple` mode, or when the open set already contains all of them.
fn expand_all_plan(ctx: &Context) -> Option<TransitionPlan<Machine>> {
    if ctx.disabled || !ctx.multiple {
        return None;
    }
    let mut next = ctx.value.get().clone();
    let mut changed = false;
    for item in enabled_items(ctx) {
        changed |= next.insert(item);
    }
    if !changed {
        return None;
    }
    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.value.set(next);
    }))
}

/// Close every open enabled item, retaining any open disabled item. No-op when
/// the root is disabled, when the single non-collapsible guard forbids
/// emptying the set, or when the resulting set equals the current one.
fn collapse_all_plan(ctx: &Context) -> Option<TransitionPlan<Machine>> {
    if ctx.disabled {
        return None;
    }
    if !ctx.multiple && !ctx.collapsible && !ctx.value.get().is_empty() {
        return None;
    }
    let next = ctx
        .value
        .get()
        .iter()
        .filter(|item| item_disabled(ctx, item))
        .cloned()
        .collect::<BTreeSet<_>>();
    if &next == ctx.value.get() {
        return None;
    }
    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.value.set(next);
    }))
}

/// Record `item` as the focused trigger. No-op when the root is disabled, the
/// item is not registered, the item is disabled, or it is already focused.
/// Carries no focus effect — focus is already where the adapter reported it.
fn focus_item_plan(ctx: &Context, item: &Key) -> Option<TransitionPlan<Machine>> {
    if ctx.disabled || !registered(ctx, item) || item_disabled(ctx, item) {
        return None;
    }
    if ctx.focused_item.as_ref() == Some(item) {
        return None;
    }
    let item = item.clone();
    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.focused_item = Some(item);
    }))
}

/// Set `focused_item` to a navigation target and emit the focus effect intent
/// so the adapter moves DOM focus to the resolved trigger.
fn focus_item_transition(item: Key) -> TransitionPlan<Machine> {
    TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.focused_item = Some(item);
    })
    .with_effect(PendingEffect::named(Effect::FocusFocusedItem))
}

/// Replace the registered items and disabled flags in DOM order, deduplicating
/// by key. Clears `focused_item` if it is no longer registered or now disabled.
fn set_items_plan(items: &[ItemRegistration]) -> TransitionPlan<Machine> {
    let items = items.to_vec();
    TransitionPlan::context_only(move |ctx: &mut Context| {
        let mut seen = BTreeSet::new();
        let mut ordered = Vec::with_capacity(items.len());
        let mut disabled = alloc::collections::BTreeMap::new();
        for item in &items {
            if seen.insert(item.key.clone()) {
                ordered.push(item.key.clone());
                disabled.insert(item.key.clone(), item.disabled);
            }
        }
        ctx.items = ordered;
        ctx.disabled_items = disabled;
        if ctx
            .focused_item
            .as_ref()
            .is_some_and(|item| !registered(ctx, item) || item_disabled(ctx, item))
        {
            ctx.focused_item = None;
        }
    })
}

/// Synchronize prop-backed context fields after a render prop change. Updates
/// the scalar fields, then reconciles `value`: a controlled set is pushed
/// (normalized for the new mode); an uncontrolled machine that lost its
/// controlled prop exits controlled mode; otherwise a single-mode downgrade
/// normalizes the existing set. Clears focus that is now disabled.
fn sync_props_plan(ctx: &Context, props: &Props) -> TransitionPlan<Machine> {
    let multiple = props.multiple;
    let collapsible = props.collapsible;
    let disabled = props.disabled;
    let orientation = props.orientation;
    let heading_level = props.heading_level;
    let controlled_value = props
        .value
        .clone()
        .map(|value| normalize_value_for_mode(value, multiple));
    let needs_value_normalize = ctx.multiple && !multiple;

    TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.multiple = multiple;
        ctx.collapsible = collapsible;
        ctx.disabled = disabled;
        ctx.orientation = orientation;
        ctx.heading_level = heading_level;

        if let Some(value) = controlled_value.clone() {
            ctx.value.sync_controlled(Some(value));
        } else if ctx.value.is_controlled() {
            ctx.value.sync_controlled(None);
        } else if needs_value_normalize {
            let normalized = normalize_value_for_mode(ctx.value.get().clone(), false);
            ctx.value.set(normalized);
        }

        if ctx
            .focused_item
            .as_ref()
            .is_some_and(|item| ctx.disabled || item_disabled(ctx, item))
        {
            ctx.focused_item = None;
        }
    })
}

/// Push a new controlled open-item set into context, entering controlled mode
/// if the machine was uncontrolled. The set is normalized for the current mode.
fn sync_controlled_value_plan(ctx: &Context, value: &BTreeSet<Key>) -> TransitionPlan<Machine> {
    let value = normalize_value_for_mode(value.clone(), ctx.multiple);
    TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.value.sync_controlled(Some(value));
    })
}

/// Enforce the single-mode invariant on an open-item set: when `multiple` is
/// false and the set has more than one key, retain only the first (in
/// `BTreeSet` order). In `multiple` mode the set is returned unchanged.
fn normalize_value_for_mode(mut value: BTreeSet<Key>, multiple: bool) -> BTreeSet<Key> {
    if multiple || value.len() <= 1 {
        return value;
    }
    let first = value.pop_first();
    let mut normalized = BTreeSet::new();
    if let Some(first) = first {
        normalized.insert(first);
    }
    normalized
}

/// Registered, enabled item keys in DOM order. Empty when the root is disabled.
fn enabled_items(ctx: &Context) -> impl DoubleEndedIterator<Item = Key> + '_ {
    ctx.items
        .iter()
        .filter(|item| !ctx.disabled && !item_disabled(ctx, item))
        .cloned()
}

/// Resolve the anchor for relative focus movement: the current `focused_item`
/// when it is still registered and enabled, otherwise the first enabled item.
fn focus_anchor(ctx: &Context) -> Option<Key> {
    ctx.focused_item
        .as_ref()
        .filter(|item| registered(ctx, item) && !item_disabled(ctx, item))
        .cloned()
        .or_else(|| enabled_items(ctx).next())
}

/// Step from `current` to the next/previous enabled item, wrapping at the
/// ends. Returns `None` when there is no enabled item or the only candidate is
/// `current` itself (so a single-item set never re-emits a focus effect).
fn step_focus(ctx: &Context, current: &Key, step: FocusStep) -> Option<Key> {
    let enabled = enabled_items(ctx).collect::<Vec<_>>();
    let len = enabled.len();
    if len == 0 {
        return None;
    }
    let index = enabled.iter().position(|item| item == current).unwrap_or(0);
    let next_index = match step {
        FocusStep::Next => (index + 1) % len,
        FocusStep::Prev => {
            if index == 0 {
                len - 1
            } else {
                index - 1
            }
        }
    };
    let next = enabled.get(next_index)?.clone();
    if &next == current {
        return None;
    }
    Some(next)
}

/// Whether `item` appears in the registered item list.
fn registered(ctx: &Context, item: &Key) -> bool {
    ctx.items.iter().any(|registered| registered == item)
}

/// Whether `item` is individually disabled (ignores the root `disabled` flag).
fn item_disabled(ctx: &Context, item: &Key) -> bool {
    ctx.disabled_items.get(item).copied().unwrap_or(false)
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
    ItemTrigger { item_key: Key },
    ItemIndicator { item_key: Key },
    ItemContent { item_key: Key },
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
        attrs.set(HtmlAttr::Dir, self.ctx.dir.as_html_attr());
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

    /// Returns the generated trigger id for an item.
    pub fn trigger_id(&self, item_key: &Key) -> String {
        self.ctx.ids.item("trigger", &dom_safe_key_token(item_key))
    }

    /// Returns the generated content id for an item.
    pub fn content_id(&self, item_key: &Key) -> String {
        self.ctx.ids.item("content", &dom_safe_key_token(item_key))
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
    /// `focus_visible` is the keyboard-modality bit provided by the adapter.
    pub fn item_trigger_attrs(&self, item_key: &Key, focus_visible: bool) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_open     = self.is_item_open(item_key);
        let is_disabled = self.is_item_disabled(item_key);
        let is_focused  = self.ctx.focused_item.as_ref() == Some(item_key);

        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemTrigger { item_key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.trigger_id(item_key));
        attrs.set(HtmlAttr::Data("ars-state"), if is_open { "open" } else { "closed" });
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if is_open { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), self.content_id(item_key));
        if is_disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        } else if is_open && !self.ctx.multiple && !self.ctx.collapsible {
            // The open trigger in single non-collapsible mode cannot collapse,
            // so it advertises `aria-disabled` (without the native `disabled`
            // attribute, which would remove it from the tab sequence). See §3.1.
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        if is_focused && focus_visible {
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
    ///
    /// Disabled triggers do not record focus: the handler is a no-op when the
    /// item is globally or individually disabled, mirroring the `Focus` event
    /// guard in `focus_item_plan`.
    pub fn on_item_trigger_focus(&self, item_key: &Key) {
        if !self.is_item_disabled(item_key) {
            (self.send)(Event::Focus(item_key.clone()));
        }
    }

    /// Handle blur event on the item trigger.
    pub fn on_item_trigger_blur(&self) {
        (self.send)(Event::Blur);
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
        // NOTE: Focus is requested through typed focus events, NOT by carrying
        // target element ids through core. The adapter handles the actual DOM
        // focus in response to `Effect::FocusFocusedItem`.
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
                    if enabled.get(prev_idx).is_some() {
                        (self.send)(Event::FocusPrev);
                    }
                    return;
                } else if resolved == Some("Next") {
                    if enabled.get((idx + 1) % len).is_some() {
                        (self.send)(Event::FocusNext);
                    }
                    return;
                }
                (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight)
            }
        };
        if data.key == next_key {
            if enabled.get((idx + 1) % len).is_some() {
                (self.send)(Event::FocusNext);
            }
        } else if data.key == prev_key {
            let prev_idx = if idx == 0 { len - 1 } else { idx - 1 };
            if enabled.get(prev_idx).is_some() {
                (self.send)(Event::FocusPrev);
            }
        } else if data.key == KeyboardKey::Home {
            if enabled.first().is_some() {
                (self.send)(Event::FocusFirst);
            }
        } else if data.key == KeyboardKey::End {
            if enabled.last().is_some() {
                (self.send)(Event::FocusLast);
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
    /// IDs are derived from the component base id plus a DOM-safe item-key token.
    pub fn item_content_attrs(&self, item_key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_open = self.is_item_open(item_key);
        attrs.set(HtmlAttr::Id, self.content_id(item_key));
        attrs.set(HtmlAttr::Role, "region");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ItemContent { item_key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if is_open { "open" } else { "closed" });
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.trigger_id(item_key));
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
            Part::ItemTrigger { item_key } => self.item_trigger_attrs(item_key, false),
            Part::ItemIndicator { item_key } => self.item_indicator_attrs(item_key),
            Part::ItemContent { item_key } => self.item_content_attrs(item_key),
        }
    }
}
```

## 2. Anatomy

```text
Accordion
├── Root                   data-ars-scope="accordion" data-ars-part="root"
└── Item (×N)              data-ars-scope="accordion" data-ars-part="item"
    ├── ItemHeader         data-ars-scope="accordion" data-ars-part="item-header"
    │   └── ItemTrigger    data-ars-scope="accordion" data-ars-part="item-trigger"
    │   └── ItemIndicator  data-ars-scope="accordion" data-ars-part="item-indicator"
    └── ItemContent        data-ars-scope="accordion" data-ars-part="item-content"
```

| Part            | Element    | Key Attributes                                                                                                           |
| --------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------ |
| `Root`          | `<div>`    | `data-ars-scope="accordion"`, `data-ars-part="root"`, `data-ars-orientation`                                             |
| `Item`          | `<div>`    | `data-ars-scope="accordion"`, `data-ars-part="item"`, `data-ars-state="open\|closed"`, `data-ars-disabled`               |
| `ItemHeader`    | `<h2-h6>`  | `data-ars-scope="accordion"`, `data-ars-part="item-header"`                                                              |
| `ItemTrigger`   | `<button>` | `data-ars-scope="accordion"`, `data-ars-part="item-trigger"`, `aria-expanded`, `aria-controls`, `data-ars-focus-visible` |
| `ItemIndicator` | `<span>`   | `data-ars-scope="accordion"`, `data-ars-part="item-indicator"`, `aria-hidden="true"`                                     |
| `ItemContent`   | `<div>`    | `data-ars-scope="accordion"`, `data-ars-part="item-content"`, `role="region"`, `aria-labelledby`                         |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part          | Role              | Properties                                                                                                                                                           |
| ------------- | ----------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Root`        | (none / `<div>`)  | `aria-orientation` when horizontal layout matters                                                                                                                    |
| `ItemTrigger` | `button` (native) | `aria-expanded="true\|false"`, `aria-controls="{content-id}"`, `aria-disabled` when disabled or when the open trigger cannot collapse in single non-collapsible mode |
| `ItemContent` | `region`          | `aria-labelledby="{trigger-id}"`, `hidden` when closed                                                                                                               |

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
  handled by the keyboard handler reading `ctx.dir` (see `resolve_horizontal_key()` in §1.6).
- **Text direction**: The Root part should propagate `dir` to the DOM element so nested text
  renders correctly.
- **No locale-specific strings** are emitted by `Accordion` itself; all visible labels are
  provided by the consumer.

> **Content height animation:** To animate content height, adapters SHOULD use CSS `grid-template-rows: 0fr` → `1fr` transition (Chrome 117+, Firefox 117+, Safari 17.2+) which avoids JavaScript measurement. Alternatively, batch all `scrollHeight` reads before any style writes in a single frame to avoid layout thrashing. For browsers supporting `interpolate-size: allow-keywords`, prefer this zero-JS solution.

## 5. Disclosure Pattern

A **Disclosure** is a single expandable/collapsible section — equivalent to React Aria's `useDisclosure`. Rather than defining a separate component, ars-ui implements Disclosure as a constrained Accordion configuration:

```rust,no_check
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
- **No `aria-multiselectable`**: Accordion root never sets `aria-multiselectable`; each trigger exposes its own expanded state through `aria-expanded`.
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
