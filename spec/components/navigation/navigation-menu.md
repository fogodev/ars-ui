---
component: NavigationMenu
category: navigation
tier: complex
foundation_deps: [architecture, accessibility, interactions]
shared_deps: [z-index-stacking]
related: [menu, menu-bar, tabs]
references:
    radix-ui: NavigationMenu
---

# NavigationMenu

A horizontal or vertical navigation bar with hover-triggered submenu dropdowns, used for
website main navigation. Each top-level item can either be a direct link or a trigger that
reveals a dropdown content panel on hover (with configurable delay). Moving quickly between
triggers skips the open delay, giving a fluid browsing feel. An optional viewport container
animates smoothly between content sizes, and an indicator tracks the active trigger.

## 1. State Machine

### 1.1 States

```rust
/// State of the NavigationMenu state machine.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// No submenu is open. The menu bar is idle.
    Idle,
    /// A submenu content panel is visible, triggered by hover or keyboard.
    Open {
        /// The key of the item whose content is currently shown.
        item: Key,
    },
}
```

### 1.2 Events

```rust
/// Events for the NavigationMenu state machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Open a specific item's content panel (immediate, used by keyboard).
    Open(Key),
    /// Close the currently open content panel. `now_ms` is the adapter-provided
    /// timestamp (milliseconds since page load) used for skip-delay window tracking.
    Close(u64),
    /// Pointer entered a trigger — starts open delay (or skips it).
    /// `now_ms` is the adapter-provided timestamp for skip-delay window checks.
    PointerEnter(Key, u64),
    /// Pointer left a trigger or content area — starts close sequence.
    PointerLeave,
    /// A trigger received focus via keyboard or programmatic focus.
    FocusTrigger {
        /// The key of the trigger that received focus.
        item: Key,
        /// Whether the focus originated from a keyboard event.
        is_keyboard: bool,
    },
    /// Move keyboard focus to the next trigger in the list.
    FocusNext,
    /// Move keyboard focus to the previous trigger in the list.
    FocusPrev,
    /// Move keyboard focus to the first trigger.
    FocusFirst,
    /// Move keyboard focus to the last trigger.
    FocusLast,
    /// A link inside the content was activated (clicked or Enter pressed).
    /// `now_ms` is the adapter-provided timestamp for skip-delay window tracking.
    SelectLink(u64),
    /// Escape key pressed — close the open submenu and return focus to its trigger.
    /// `now_ms` is the adapter-provided timestamp for skip-delay window tracking.
    EscapeKey(u64),
    /// The open delay timer has fired.
    OpenTimerFired(Key),
    /// The close delay timer has fired. `now_ms` is the adapter-provided timestamp.
    CloseTimerFired(u64),
    /// Pointer entered the content area — cancels pending close.
    ContentPointerEnter,
    /// Pointer left the content area — starts close delay.
    ContentPointerLeave,
    /// Adapter sends this after mount to resolve `Direction::Auto`.
    SetDirection(Direction),
    /// Request the adapter to move DOM focus to the element with `target_id`.
    RequestFocus {
        /// The ID of the element to focus.
        target_id: String,
    },
    /// Replace the registered trigger keys in DOM order. Adapters send this
    /// after triggers mount or reorder so the machine can drive roving focus,
    /// motion direction, and the open-item registration gate. Until the first
    /// `SetItems`, the registry is unsynced and open is permitted optimistically.
    SetItems(Vec<Key>),
    /// Synchronize props-backed context fields (`orientation`, `delay_ms`,
    /// `skip_delay_ms`, and the controlled `value`) after a props change while
    /// the instance is uncontrolled or its scalar config changed.
    SyncProps,
    /// Synchronize the externally controlled open item. Emitted by
    /// `on_props_changed` when `Props::value` changes on a controlled instance.
    SyncControlledValue(Option<Key>),
    /// Synchronize provider-backed locale and messages.
    SyncMessages {
        /// Active provider locale.
        locale: Locale,
        /// Localized `NavigationMenu` messages.
        messages: Messages,
    },
}
```

### 1.3 Context

```rust
use ars_core::Bindable;
use ars_collections::Key;
use ars_i18n::{Orientation, Direction, Locale};

/// Context for the NavigationMenu component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The key of the currently open item (controlled/uncontrolled).
    pub value: Bindable<Option<Key>>,
    /// The trigger that currently has keyboard focus.
    pub focused_trigger: Option<Key>,
    /// Whether the focused trigger received focus via keyboard.
    pub focus_visible: bool,
    /// Layout orientation of the trigger list.
    pub orientation: Orientation,
    /// Text direction — affects arrow key semantics.
    pub dir: Direction,
    /// Delay in milliseconds before a hovered trigger opens its content.
    pub delay_ms: u32,
    /// Window in milliseconds after closing during which re-hovering
    /// another trigger skips the open delay entirely.
    pub skip_delay_ms: u32,
    /// Timestamp (milliseconds since epoch) of the last close event.
    /// Used to determine whether the skip-delay window is active.
    pub last_close_time: Option<u64>,
    /// Whether the pointer is currently inside the content area.
    pub pointer_in_content: bool,
    /// Registered trigger keys in DOM order.
    pub items: Vec<Key>,
    /// Whether adapters have synced the trigger registry at least once.
    /// Until this is `true`, the open-item registration gate is permissive so a
    /// controlled or default value can render before triggers register.
    pub items_registered: bool,
    /// The key of the previously open item (for motion direction calculation).
    pub previous_item: Option<Key>,
    /// ID of the list element (for runtime direction resolution).
    pub list_id: String,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
    /// The resolved locale for this component instance.
    pub locale: Locale,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
    /// Pending hover-open key used by the adapter open-delay timer effect.
    /// Set when `PointerEnter` starts an open delay and cleared on open, close,
    /// registry sync, or controlled-value sync.
    pub pending_open_item: Option<Key>,
    /// Last focus target id requested through `Event::RequestFocus` or a
    /// roving-focus transition, consumed by the `FocusTrigger` effect.
    pub requested_focus_id: Option<String>,
}
```

### 1.4 Props

```rust
use ars_collections::Key;
use ars_i18n::{Orientation, Direction};

/// Props for the NavigationMenu component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Unique component identifier.
    pub id: String,
    /// Controlled open item key. When `Some(Some(key))`, that item's content is shown.
    /// When `Some(None)`, all content is closed (controlled). When `None`, uncontrolled.
    pub value: Option<Option<Key>>,
    /// Initial open item when uncontrolled. `None` means all closed initially.
    pub default_value: Option<Key>,
    /// Delay in milliseconds before a hovered trigger opens. Default: 200.
    pub delay_ms: u32,
    /// Window in milliseconds after closing during which hovering a new trigger
    /// skips the delay entirely. Default: 300.
    pub skip_delay_ms: u32,
    /// Layout orientation of the trigger list. Default: Horizontal.
    pub orientation: Orientation,
    /// Text direction. Default: Ltr.
    pub dir: Direction,
    /// Whether focus wraps from the last trigger back to the first. Default: true.
    pub loop_focus: bool,
    /// Callback invoked when the open item changes. Receives `Some(key)` when an item
    /// opens and `None` when all content is closed.
    pub on_value_change: Option<Callback<Option<Key>>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            delay_ms: 200,
            skip_delay_ms: 300,
            orientation: Orientation::Horizontal,
            dir: Direction::Ltr,
            loop_focus: true,
            on_value_change: None,
        }
    }
}
```

### 1.5 Guards

```rust
/// Returns true when the skip-delay window is active.
/// The skip-delay window is the period after closing a submenu during which
/// hovering another trigger opens it immediately (no delay).
fn in_skip_delay_window(ctx: &Context, now_ms: u64) -> bool {
    match ctx.last_close_time {
        Some(t) => now_ms.saturating_sub(t) < ctx.skip_delay_ms as u64,
        None => false,
    }
}
```

The machine is **DOM- and timer-agnostic**: it never touches the platform or
schedules timers directly. Instead it emits a small typed `Effect` enum and the
adapter owns all browser work — the open/close delay timers (it sends
`OpenTimerFired` / `CloseTimerFired` back when they fire), moving DOM focus to
`Context::requested_focus_id`, and invoking `Props::on_value_change`. A
`with_effect(PendingEffect::named(Effect::*))` queues an effect; a
`cancel_effect(Effect::*)` tells the adapter to cancel an in-flight effect of
that kind (used to cancel a stale open/close timer). This mirrors the
typed-`Effect` + adapter-timer convention shared with Popover, Dialog, and
Tooltip.

```rust
/// Typed effect intents emitted by the NavigationMenu machine. Adapters own the
/// corresponding DOM/timer work; the machine only declares intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter starts or refreshes the open-delay timer. When it fires, the
    /// adapter sends `Event::OpenTimerFired(Context::pending_open_item)`.
    OpenDelay,
    /// Adapter starts or refreshes the close-delay timer. When it fires, the
    /// adapter sends `Event::CloseTimerFired(now_ms)`.
    CloseDelay,
    /// Adapter moves DOM focus to `Context::requested_focus_id`.
    FocusTrigger,
    /// Adapter invokes `Props::on_value_change` with the new open item.
    ValueChange,
}
```

```rust,no_check
use ars_core::{TransitionPlan, PendingEffect, Bindable, AttrMap, no_cleanup};
use ars_collections::Key;
use ars_i18n::{Orientation, Direction, Locale};

// ── Machine ──────────────────────────────────────────────────────────────────

/// Machine for the NavigationMenu component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Messages = Messages;
    type Effect = Effect;
    type Api<'a> = Api<'a>;

    fn init(props: &Props, env: &Env, messages: &Messages) -> (State, Context) {
        let initial_value = match &props.value {
            Some(v) => v.clone(),
            None    => props.default_value.clone(),
        };
        let value = match &props.value {
            Some(v) => Bindable::controlled(v.clone()),
            None    => Bindable::uncontrolled(props.default_value.clone()),
        };
        let initial_state = match &initial_value {
            Some(key) => State::Open { item: key.clone() },
            None      => State::Idle,
        };
        let ids = ComponentIds::from_id(&props.id);
        let list_id = ids.part("list");
        (initial_state, Context {
            value,
            focused_trigger: None,
            focus_visible: false,
            orientation: props.orientation,
            dir: props.dir,
            delay_ms: props.delay_ms,
            skip_delay_ms: props.skip_delay_ms,
            last_close_time: None,
            pointer_in_content: false,
            items: Vec::new(),
            items_registered: false,
            previous_item: None,
            list_id,
            ids,
            locale: env.locale.clone(),
            messages: messages.clone(),
            pending_open_item: None,
            requested_focus_id: None,
        })
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {

            // ── Open (keyboard / programmatic) ───────────────────────────────
            // Gated on the registration check: an unregistered (or stale
            // controlled) key cannot open a phantom panel. `open_item_plan`
            // returns `None` when the key is unknown or already open.
            (_, Event::Open(item)) => open_item_plan(state, ctx, item.clone()),

            // ── Close / SelectLink ───────────────────────────────────────────
            // `close_plan` cancels the open/close timers, records the close
            // timestamp, and emits `Effect::ValueChange`.
            (_, Event::Close(now_ms) | Event::SelectLink(now_ms))
                if effective_open_item(state, ctx).is_some() =>
            {
                Some(close_plan(*now_ms))
            }

            // ── PointerEnter ─────────────────────────────────────────────────
            // Hover a trigger. Re-entering the open trigger cancels a pending
            // close. When another item is open or the skip-delay window is
            // active, open immediately; otherwise stage `pending_open_item` and
            // start the open-delay timer.
            (_, Event::PointerEnter(item, now_ms)) => {
                let rendered_open = effective_open_item(state, ctx);

                if rendered_open == Some(item) {
                    return Some(
                        TransitionPlan::context_only(|ctx: &mut Context| {
                            ctx.pending_open_item = None;
                            ctx.pointer_in_content = false;
                        })
                        .cancel_effect(Effect::CloseDelay),
                    );
                }

                if rendered_open.is_some() || in_skip_delay_window(ctx, *now_ms) {
                    open_item_plan(state, ctx, item.clone())
                        .map(|plan| plan.cancel_effect(Effect::CloseDelay))
                } else {
                    let item = item.clone();
                    Some(
                        TransitionPlan::context_only(move |ctx: &mut Context| {
                            ctx.pending_open_item = Some(item);
                        })
                        .with_effect(PendingEffect::named(Effect::OpenDelay)),
                    )
                }
            }

            // ── PointerLeave / ContentPointerLeave ───────────────────────────
            // While open, start the close-delay timer.
            (_, Event::PointerLeave | Event::ContentPointerLeave)
                if effective_open_item(state, ctx).is_some() =>
            {
                Some(
                    TransitionPlan::context_only(|ctx: &mut Context| {
                        ctx.pointer_in_content = false;
                    })
                    .with_effect(PendingEffect::named(Effect::CloseDelay)),
                )
            }

            // When idle and pointer leaves a trigger that had not opened yet,
            // clear the staged key and cancel the pending open-delay timer.
            (State::Idle, Event::PointerLeave) => Some(
                TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.pending_open_item = None;
                })
                .cancel_effect(Effect::OpenDelay),
            ),

            // ── ContentPointerEnter ──────────────────────────────────────────
            // Cancel any pending close when the pointer moves into content.
            (_, Event::ContentPointerEnter) if effective_open_item(state, ctx).is_some() => Some(
                TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.pointer_in_content = true;
                })
                .cancel_effect(Effect::CloseDelay),
            ),

            // ── OpenTimerFired ───────────────────────────────────────────────
            // Only honour the timer if it still matches the staged key.
            (State::Idle, Event::OpenTimerFired(item)) => {
                if ctx.pending_open_item.as_ref() != Some(item) {
                    return None;
                }
                Some(
                    open_to_plan(None, item.clone())
                        .apply(|ctx: &mut Context| {
                            ctx.pending_open_item = None;
                        })
                        .cancel_effect(Effect::OpenDelay),
                )
            }

            // ── CloseTimerFired ──────────────────────────────────────────────
            // Only close if the pointer is not currently inside the content.
            (_, Event::CloseTimerFired(now_ms)) if effective_open_item(state, ctx).is_some() => {
                if ctx.pointer_in_content {
                    return None;
                }
                Some(close_plan(*now_ms).cancel_effect(Effect::CloseDelay))
            }

            // ── FocusTrigger ─────────────────────────────────────────────────
            (_, Event::FocusTrigger { item, is_keyboard }) => {
                let item = item.clone();
                let is_keyboard = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused_trigger = Some(item);
                    ctx.focus_visible = is_keyboard;
                }))
            }

            // ── Roving focus ─────────────────────────────────────────────────
            // Each plan records the DOM-safe target id in `requested_focus_id`
            // (via `trigger_dom_id`) and queues `Effect::FocusTrigger` so the
            // adapter moves DOM focus.
            (_, Event::FocusNext) => focus_by_offset_plan(ctx, props, 1),
            (_, Event::FocusPrev) => focus_by_offset_plan(ctx, props, -1),
            (_, Event::FocusFirst) => focus_absolute_plan(ctx, 0),
            (_, Event::FocusLast) => {
                if ctx.items.is_empty() {
                    None
                } else {
                    focus_absolute_plan(ctx, ctx.items.len() - 1)
                }
            }

            // ── EscapeKey ────────────────────────────────────────────────────
            // Close the open panel and return focus to its trigger.
            (_, Event::EscapeKey(now_ms)) => {
                let item = effective_open_item(state, ctx)?.clone();
                Some(
                    close_plan(*now_ms)
                        .apply(move |ctx: &mut Context| {
                            ctx.focused_trigger = Some(item);
                            ctx.focus_visible = true;
                        })
                        .with_effect(PendingEffect::named(Effect::FocusTrigger)),
                )
            }

            // ── SetDirection ─────────────────────────────────────────────────
            (_, Event::SetDirection(dir)) if ctx.dir != *dir => {
                let dir = *dir;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.dir = dir;
                }))
            }

            // ── RequestFocus ─────────────────────────────────────────────────
            // Record the target id and queue the focus effect for the adapter.
            (_, Event::RequestFocus { target_id }) => {
                let target_id = target_id.clone();
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.requested_focus_id = Some(target_id);
                    })
                    .with_effect(PendingEffect::named(Effect::FocusTrigger)),
                )
            }

            // ── SetItems ─────────────────────────────────────────────────────
            // Register the trigger keys in DOM order. If the currently open key
            // is no longer present it is closed (with `Effect::ValueChange`).
            // Cancels any in-flight open/close timer.
            (_, Event::SetItems(items)) => {
                let items = dedupe_keys(items);
                let open_removed = ctx
                    .value
                    .get()
                    .as_ref()
                    .is_some_and(|item| !items.iter().any(|candidate| candidate == item));

                let plan = if open_removed {
                    TransitionPlan::to(State::Idle).with_effect(value_change_effect(None))
                } else {
                    TransitionPlan::new()
                };

                Some(
                    plan.apply(move |ctx: &mut Context| {
                        ctx.items = items;
                        ctx.items_registered = true;
                        ctx.pending_open_item = None;
                        if let Some(focused) = &ctx.focused_trigger
                            && !ctx.items.iter().any(|item| item == focused)
                        {
                            ctx.focused_trigger = None;
                            ctx.focus_visible = false;
                        }
                        if open_removed {
                            ctx.previous_item = ctx.value.get().clone();
                            ctx.value.set(None);
                            ctx.pointer_in_content = false;
                        }
                    })
                    .cancel_effect(Effect::OpenDelay)
                    .cancel_effect(Effect::CloseDelay),
                )
            }

            // ── SyncProps ────────────────────────────────────────────────────
            (_, Event::SyncProps) => {
                let orientation = props.orientation;
                let delay_ms = props.delay_ms;
                let skip_delay_ms = props.skip_delay_ms;
                let value = props.value.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.orientation = orientation;
                    ctx.delay_ms = delay_ms;
                    ctx.skip_delay_ms = skip_delay_ms;
                    ctx.value.sync_controlled(value);
                }))
            }

            // ── SyncControlledValue ──────────────────────────────────────────
            (_, Event::SyncControlledValue(value)) => {
                let value = value.clone();
                let next_state = match &value {
                    Some(item) => State::Open { item: item.clone() },
                    None => State::Idle,
                };
                Some(
                    TransitionPlan::to(next_state)
                        .apply(move |ctx: &mut Context| {
                            ctx.previous_item = ctx.value.get().clone();
                            ctx.pending_open_item = None;
                            ctx.pointer_in_content = false;
                            ctx.value.sync_controlled(Some(value));
                        })
                        .cancel_effect(Effect::OpenDelay)
                        .cancel_effect(Effect::CloseDelay),
                )
            }

            // ── SyncMessages ─────────────────────────────────────────────────
            (_, Event::SyncMessages { locale, messages }) => {
                let locale = locale.clone();
                let messages = messages.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.locale = locale;
                    ctx.messages = messages;
                }))
            }

            _ => None,
        }
    }

    fn connect<'a>(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api { state, ctx, props, send }
    }
}

// ── Open-item registration gate and shared transition helpers ─────────────────

/// Returns true when the key may open: either the registry has not synced yet,
/// or the key is present in it. Prevents an unregistered or stale controlled key
/// from opening a phantom panel.
fn item_is_registered(ctx: &Context, item: &Key) -> bool {
    !ctx.items_registered || ctx.items.iter().any(|candidate| candidate == item)
}

/// Resolves the open item that should actually render: the controlled/bindable
/// value if registered, otherwise the state's open item if registered.
fn effective_open_item<'a>(state: &'a State, ctx: &'a Context) -> Option<&'a Key> {
    ctx.value
        .get()
        .as_ref()
        .filter(|item| item_is_registered(ctx, item))
        .or_else(|| state_open_item(state).filter(|item| item_is_registered(ctx, item)))
}

/// Builds an open plan, gated on registration and a no-op when already open.
fn open_item_plan(state: &State, ctx: &Context, item: Key) -> Option<TransitionPlan<Machine>> {
    if !item_is_registered(ctx, &item) {
        return None;
    }
    if effective_open_item(state, ctx) == Some(&item) {
        None
    } else {
        let previous = ctx.value.get().clone().or_else(|| state_open_item(state).cloned());
        Some(open_to_plan(previous, item))
    }
}

/// Transition to `Open`, recording the previous item and emitting `ValueChange`.
fn open_to_plan(previous: Option<Key>, item: Key) -> TransitionPlan<Machine> {
    let next_value = Some(item.clone());
    TransitionPlan::to(State::Open { item: item.clone() })
        .apply(move |ctx: &mut Context| {
            ctx.previous_item = previous;
            ctx.value.set(Some(item));
            ctx.pointer_in_content = false;
            ctx.pending_open_item = None;
        })
        .with_effect(value_change_effect(next_value))
}

/// Transition to `Idle`, cancelling timers and emitting `ValueChange(None)`.
fn close_plan(now_ms: u64) -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Idle)
        .apply(move |ctx: &mut Context| {
            ctx.previous_item = ctx.value.get().clone();
            ctx.value.set(None);
            ctx.pointer_in_content = false;
            ctx.pending_open_item = None;
            ctx.last_close_time = Some(now_ms);
        })
        .cancel_effect(Effect::OpenDelay)
        .cancel_effect(Effect::CloseDelay)
        .with_effect(value_change_effect(None))
}

/// `Effect::ValueChange` carrying the new open item to `Props::on_value_change`.
fn value_change_effect(value: Option<Key>) -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::ValueChange,
        move |_ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_value_change {
                callback(value.clone());
            }
            no_cleanup()
        },
    )
}

/// Roving focus by relative offset, honouring `loop_focus`.
fn focus_by_offset_plan(ctx: &Context, props: &Props, offset: isize) -> Option<TransitionPlan<Machine>> {
    if ctx.items.is_empty() {
        return None;
    }
    let current = ctx
        .focused_trigger
        .as_ref()
        .and_then(|focused| ctx.items.iter().position(|item| item == focused))
        .unwrap_or(0);
    let len = ctx.items.len();
    let next = if offset.is_positive() {
        if current + 1 >= len {
            if props.loop_focus { 0 } else { current }
        } else {
            current + 1
        }
    } else if current == 0 {
        if props.loop_focus { len - 1 } else { current }
    } else {
        current - 1
    };
    if next == current && !props.loop_focus {
        None
    } else {
        focus_absolute_plan(ctx, next)
    }
}

/// Roving focus to an absolute index; records the DOM-safe target id.
fn focus_absolute_plan(ctx: &Context, index: usize) -> Option<TransitionPlan<Machine>> {
    let item = ctx.items.get(index)?.clone();
    let target_id = trigger_dom_id(&ctx.ids, &item);
    Some(
        TransitionPlan::context_only(move |ctx: &mut Context| {
            ctx.focused_trigger = Some(item);
            ctx.focus_visible = true;
            ctx.requested_focus_id = Some(target_id);
        })
        .with_effect(PendingEffect::named(Effect::FocusTrigger)),
    )
}
```

`on_props_changed` translates a controlled/config props change into the sync
events above: a changed controlled `value` emits `SyncControlledValue`, an
uncontrolled value change or a changed `orientation`/`delay_ms`/`skip_delay_ms`
emits `SyncProps`, and a changed `dir` emits `SetDirection`. The adapter emits
`SetItems` when triggers mount or reorder, and `SyncMessages` when the provider
locale or messages change.

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "navigation-menu"]
pub enum Part {
    /// The outer navigation landmark element.
    Root,
    /// The menubar list container.
    List,
    /// A top-level item wrapper.
    Item { item_key: Key },
    /// A trigger that opens associated content.
    Trigger { item_key: Key, content_id: String },
    /// Dropdown content for a trigger.
    Content { item_key: Key },
    /// A navigation link inside content.
    Link { active: bool },
    /// Visual active-trigger indicator.
    Indicator,
    /// Optional animated content viewport.
    Viewport,
    /// Root element for a nested navigation menu inside content.
    Sub,
    /// Menubar list container for a nested navigation menu.
    SubList,
    /// Nested item wrapper.
    SubItem { item_key: Key },
    /// Nested trigger that opens associated nested content.
    SubTrigger { item_key: Key, content_id: String },
    /// Dropdown content for a nested trigger.
    SubContent { item_key: Key },
    /// Visual active-trigger indicator for a nested menu.
    SubIndicator,
    /// Optional animated content viewport for a nested menu.
    SubViewport,
}

/// API for the NavigationMenu component.
pub struct Api<'a> {
    /// Current machine state.
    state: &'a State,
    /// Current context.
    ctx:   &'a Context,
    /// Current props.
    props: &'a Props,
    /// Event dispatcher.
    send:  &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Get the key of the currently open item, if any. Gated on the
    /// registration check so a stale controlled key reports no open item until
    /// its trigger registers.
    pub fn open_item(&self) -> Option<&Key> {
        let item = self.ctx.value.get().as_ref()?;
        item_is_registered(self.ctx, item).then_some(item)
    }

    /// Check whether a specific item's content is currently showing.
    pub fn is_item_open(&self, item_key: &Key) -> bool {
        self.open_item() == Some(item_key)
    }

    /// Compute the motion direction for content animation.
    /// Returns `Some("from-end")` when the current trigger is after the previous
    /// one, `Some("from-start")` when it is before. Returns `None` when there is
    /// no previous item (first open) or when the current and previous triggers
    /// resolve to the same index (no direction to animate).
    fn motion_direction(&self, item_key: &Key) -> Option<&'static str> {
        let prev = self.ctx.previous_item.as_ref()?;
        let items = &self.ctx.items;
        let prev_idx = items.iter().position(|k| k == prev)?;
        let curr_idx = items.iter().position(|k| k == item_key)?;
        if curr_idx > prev_idx {
            Some("from-end")
        } else if curr_idx < prev_idx {
            Some("from-start")
        } else {
            None
        }
    }

    /// Attrs for the outer navigation landmark element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "navigation");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.navigation_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::Data("ars-orientation"), match self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical   => "vertical",
        });
        attrs.set(HtmlAttr::Dir, match self.ctx.dir {
            Direction::Ltr  => "ltr",
            Direction::Rtl  => "rtl",
            Direction::Auto => "auto",
        });
        attrs
    }

    /// Attrs for the menubar list container.
    pub fn list_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::List.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.list_id);
        attrs.set(HtmlAttr::Role, "menubar");
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), match self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical   => "vertical",
        });
        attrs
    }

    /// Attrs for an item wrapper (wraps trigger + content).
    pub fn item_attrs(&self, item_key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Item { item_key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Attrs for a trigger button that opens/closes a content panel.
    ///
    /// `item_key`   -- unique key for this item.
    /// `content_id` -- ID of the associated content panel (for `aria-controls`).
    pub fn trigger_attrs(&self, item_key: &Key, content_id: &str) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_open    = self.is_item_open(item_key);
        let is_focused = self.ctx.focused_trigger.as_ref() == Some(item_key);

        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Trigger { item_key: Key::default(), content_id: String::new() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, trigger_dom_id(&self.ctx.ids, item_key));
        attrs.set(HtmlAttr::Role, "menuitem");
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "true");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if is_open { "true" } else { "false" });
        if is_open {
            attrs.set(HtmlAttr::Aria(AriaAttr::Controls), content_id);
        }
        attrs.set(HtmlAttr::Data("ars-state"), if is_open { "open" } else { "closed" });
        if is_focused && self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        attrs.set(HtmlAttr::TabIndex, self.trigger_tab_index(item_key));
        attrs
    }

    /// Roving tabindex: the focused trigger (or the first trigger when none is
    /// focused) gets `"0"`; all others get `"-1"`.
    fn trigger_tab_index(&self, item_key: &Key) -> &'static str {
        if self.ctx.focused_trigger.as_ref() == Some(item_key)
            || (self.ctx.focused_trigger.is_none() && self.ctx.items.first() == Some(item_key))
        {
            "0"
        } else {
            "-1"
        }
    }

    /// Attrs for a content panel revealed when its trigger is active.
    pub fn content_attrs(&self, item_key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_open = self.is_item_open(item_key);

        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Content { item_key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, content_dom_id(&self.ctx.ids, item_key));
        attrs.set(HtmlAttr::Data("ars-state"), if is_open { "open" } else { "closed" });
        if let Some(motion) = self.motion_direction(item_key) {
            attrs.set(HtmlAttr::Data("ars-motion"), motion);
        }
        if !is_open {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }
        attrs
    }

    /// Attrs for a navigation link inside a content panel.
    ///
    /// `active` -- whether this link represents the current page.
    pub fn link_attrs(&self, active: bool) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Link { active: false }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if active {
            attrs.set(HtmlAttr::Aria(AriaAttr::Current), "page");
            attrs.set_bool(HtmlAttr::Data("ars-active"), true);
        }
        attrs
    }

    /// Attrs for the visual indicator that tracks the active trigger.
    pub fn indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Indicator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        let is_visible = self.open_item().is_some();
        attrs.set(HtmlAttr::Data("ars-state"), if is_visible { "visible" } else { "hidden" });
        attrs
    }

    /// Attrs for the optional viewport container that animates between content sizes.
    pub fn viewport_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Viewport.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if self.open_item().is_some() { "open" } else { "closed" });
        // CSS custom properties for viewport sizing are set as inline styles by the adapter.
        // --ars-viewport-width: width of the currently active content panel.
        // --ars-viewport-height: height of the currently active content panel.
        attrs
    }

    // ── Sub-menu parts (see §5) ────────────────────────────────────────────────
    // A `Sub` reuses the root anatomy/attrs with `sub-*` part tokens. Triggers
    // and content share the root DOM-safe id helpers and the same open/focus
    // semantics, scoped to a second `Machine` instance per §5.

    /// Attrs for the root element of a nested navigation menu.
    pub fn sub_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Sub.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-orientation"), match self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical   => "vertical",
        });
        attrs.set(HtmlAttr::Dir, match self.ctx.dir {
            Direction::Ltr  => "ltr",
            Direction::Rtl  => "rtl",
            Direction::Auto => "auto",
        });
        attrs
    }

    /// Attrs for the menubar list container of a nested navigation menu.
    pub fn sub_list_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::SubList.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "menubar");
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), match self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical   => "vertical",
        });
        attrs
    }

    /// Attrs for a nested item wrapper.
    pub fn sub_item_attrs(&self, _item_key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::SubItem { item_key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Attrs for a nested trigger that opens/closes a nested content panel.
    pub fn sub_trigger_attrs(&self, item_key: &Key, content_id: &str) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_open    = self.is_item_open(item_key);
        let is_focused = self.ctx.focused_trigger.as_ref() == Some(item_key);
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::SubTrigger { item_key: Key::default(), content_id: String::new() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, trigger_dom_id(&self.ctx.ids, item_key));
        attrs.set(HtmlAttr::Role, "menuitem");
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "true");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if is_open { "true" } else { "false" });
        attrs.set(HtmlAttr::Data("ars-state"), if is_open { "open" } else { "closed" });
        attrs.set(HtmlAttr::TabIndex, self.trigger_tab_index(item_key));
        if is_open {
            attrs.set(HtmlAttr::Aria(AriaAttr::Controls), content_id);
        }
        if is_focused && self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        attrs
    }

    /// Attrs for a nested content panel revealed when its trigger is active.
    pub fn sub_content_attrs(&self, item_key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_open = self.is_item_open(item_key);
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::SubContent { item_key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, content_dom_id(&self.ctx.ids, item_key));
        attrs.set(HtmlAttr::Data("ars-state"), if is_open { "open" } else { "closed" });
        if let Some(motion) = self.motion_direction(item_key) {
            attrs.set(HtmlAttr::Data("ars-motion"), motion);
        }
        if !is_open {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }
        attrs
    }

    /// Attrs for the visual indicator of a nested navigation menu.
    pub fn sub_indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::SubIndicator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs.set(HtmlAttr::Data("ars-state"), if self.open_item().is_some() { "visible" } else { "hidden" });
        attrs
    }

    /// Attrs for the optional viewport container of a nested navigation menu.
    pub fn sub_viewport_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::SubViewport.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if self.open_item().is_some() { "open" } else { "closed" });
        attrs
    }

    // ── Event handlers ───────────────────────────────────────────────────────

    /// Handle pointer enter on a trigger.
    /// `now_ms` — adapter-provided timestamp from the pointer event.
    pub fn on_trigger_pointer_enter(&self, item_key: &Key, now_ms: u64) {
        (self.send)(Event::PointerEnter(item_key.clone(), now_ms));
    }

    /// Handle pointer leave on a trigger.
    pub fn on_trigger_pointer_leave(&self) {
        (self.send)(Event::PointerLeave);
    }

    /// Handle focus on a trigger.
    pub fn on_trigger_focus(&self, item_key: &Key, is_keyboard: bool) {
        (self.send)(Event::FocusTrigger { item: item_key.clone(), is_keyboard });
    }

    /// Handle keydown on a trigger.
    /// `now_ms` — adapter-provided timestamp from the keyboard event, forwarded
    /// to `EscapeKey` so the machine never reaches into the platform itself.
    pub fn on_trigger_keydown(&self, item_key: &Key, data: &KeyboardEventData, now_ms: u64) {
        let (prev_key, next_key) = match (&self.ctx.orientation, &self.ctx.dir) {
            (Orientation::Horizontal, Direction::Rtl)  => (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft),
            (Orientation::Horizontal, _)               => (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight),
            (Orientation::Vertical,   _)               => (KeyboardKey::ArrowUp, KeyboardKey::ArrowDown),
        };
        if data.key == next_key {
            (self.send)(Event::FocusNext);
        } else if data.key == prev_key {
            (self.send)(Event::FocusPrev);
        } else if data.key == KeyboardKey::Home {
            (self.send)(Event::FocusFirst);
        } else if data.key == KeyboardKey::End {
            (self.send)(Event::FocusLast);
        } else if data.key == KeyboardKey::Enter || data.key == KeyboardKey::Space {
            (self.send)(Event::Open(item_key.clone()));
        } else if data.key == KeyboardKey::Escape {
            (self.send)(Event::EscapeKey(now_ms));
        } else if self.ctx.orientation == Orientation::Horizontal
            && (data.key == KeyboardKey::ArrowDown || data.key == KeyboardKey::ArrowUp)
        {
            // In horizontal mode, ArrowDown/ArrowUp open the content panel.
            (self.send)(Event::Open(item_key.clone()));
        }
    }

    /// Handle pointer enter on the content area.
    pub fn on_content_pointer_enter(&self) {
        (self.send)(Event::ContentPointerEnter);
    }

    /// Handle pointer leave on the content area.
    pub fn on_content_pointer_leave(&self) {
        (self.send)(Event::ContentPointerLeave);
    }

    /// Handle keydown inside the content area.
    /// `now_ms` — adapter-provided timestamp from the keyboard event.
    pub fn on_content_keydown(&self, data: &KeyboardEventData, now_ms: u64) {
        if data.key == KeyboardKey::Escape {
            (self.send)(Event::EscapeKey(now_ms));
        }
    }

    /// Handle click on a link inside the content.
    /// `now_ms` — adapter-provided timestamp from the click event.
    pub fn on_link_select(&self, now_ms: u64) {
        (self.send)(Event::SelectLink(now_ms));
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::List => self.list_attrs(),
            Part::Item { item_key } => self.item_attrs(&item_key),
            Part::Trigger { item_key, content_id } => self.trigger_attrs(&item_key, &content_id),
            Part::Content { item_key } => self.content_attrs(&item_key),
            Part::Link { active } => self.link_attrs(active),
            Part::Indicator => self.indicator_attrs(),
            Part::Viewport => self.viewport_attrs(),
            Part::Sub => self.sub_attrs(),
            Part::SubList => self.sub_list_attrs(),
            Part::SubItem { item_key } => self.sub_item_attrs(&item_key),
            Part::SubTrigger { item_key, content_id } => self.sub_trigger_attrs(&item_key, &content_id),
            Part::SubContent { item_key } => self.sub_content_attrs(&item_key),
            Part::SubIndicator => self.sub_indicator_attrs(),
            Part::SubViewport => self.sub_viewport_attrs(),
        }
    }
}

// ── DOM-safe id helpers and registration gate ─────────────────────────────────

/// Returns true when the key may be treated as open: the registry has not synced
/// yet, or the key is present in it.
fn item_is_registered(ctx: &Context, item: &Key) -> bool {
    !ctx.items_registered || ctx.items.iter().any(|candidate| candidate == item)
}

/// DOM id for a trigger element, e.g. `nav-trigger-s-6d61696e`.
fn trigger_dom_id(ids: &ComponentIds, key: &Key) -> String {
    ids.item("trigger", &dom_safe_key_token(key))
}

/// DOM id for a content element, e.g. `nav-content-s-6d61696e`.
fn content_dom_id(ids: &ComponentIds, key: &Key) -> String {
    ids.item("content", &dom_safe_key_token(key))
}

/// Encodes a `Key` into a DOM-id-safe token. Integer and UUID keys map to
/// `i-{n}` / `u-{uuid}`; string keys are hex-encoded as `s-{hex}` so arbitrary
/// user strings can never produce an invalid or colliding DOM id.
fn dom_safe_key_token(key: &Key) -> String {
    match key {
        Key::Int(value) => format!("i-{value}"),
        #[cfg(feature = "uuid")]
        Key::Uuid(value) => format!("u-{value}"),
        Key::String(value) => {
            let mut token = String::from("s-");
            for byte in value.as_bytes() {
                write!(token, "{byte:02x}").expect("writing to a String cannot fail");
            }
            token
        }
    }
}
```

## 2. Anatomy

```text
NavigationMenu
├── Root                 <nav>  role="navigation", aria-label
│   ├── List             <ul>   role="menubar"
│   │   ├── Item (xN)   <li>   wrapper for trigger + content
│   │   │   ├── Trigger  <button> role="menuitem", aria-expanded, aria-haspopup
│   │   │   └── Content  <div>    dropdown panel (hidden when closed)
│   │   │       └── Link (xN) <a> aria-current="page" when active
│   │   └── Indicator    <span>   aria-hidden, visual active-trigger marker
│   └── Viewport         <div>    optional animated content container
```

| Part        | Element    | Key Attributes                                                                                            |
| ----------- | ---------- | --------------------------------------------------------------------------------------------------------- |
| `Root`      | `<nav>`    | `role="navigation"`, `aria-label`, `data-ars-scope="navigation-menu"`, `data-ars-orientation`, `dir`      |
| `List`      | `<ul>`     | `role="menubar"`, `aria-orientation`, `data-ars-part="list"`                                              |
| `Item`      | `<li>`     | `data-ars-part="item"` (wrapper, no ARIA role)                                                            |
| `Trigger`   | `<button>` | `role="menuitem"`, `aria-haspopup="true"`, `aria-expanded`, `aria-controls`, `tabindex`, `data-ars-state` |
| `Content`   | `<div>`    | `data-ars-state="open\|closed"`, `data-ars-motion`, `hidden` (when closed)                                |
| `Link`      | `<a>`      | `aria-current="page"` (when active), `data-ars-active`                                                    |
| `Indicator` | `<span>`   | `aria-hidden="true"`, `data-ars-state="visible\|hidden"`                                                  |
| `Viewport`  | `<div>`    | `data-ars-state="open\|closed"`, CSS vars `--ars-viewport-width`, `--ars-viewport-height`                 |

### 2.1 Viewport Part

The `Viewport` is an optional container placed outside the list that holds the currently
active content panel. The adapter portal-mounts the active content into the viewport and
sets CSS custom properties for smooth size transitions:

| Property                | Description                                               |
| ----------------------- | --------------------------------------------------------- |
| `--ars-viewport-width`  | Width of the currently active content panel (in pixels).  |
| `--ars-viewport-height` | Height of the currently active content panel (in pixels). |

Consumers apply CSS `width: var(--ars-viewport-width)` and `height: var(--ars-viewport-height)`
with CSS transitions to animate size changes when switching between content panels.

### 2.2 Indicator Part

The `Indicator` tracks the position of the currently active (or hovered) trigger. The adapter
measures the trigger's bounding rect relative to the list and sets CSS custom properties as
inline styles:

| Property                 | Description                                            |
| ------------------------ | ------------------------------------------------------ |
| `--ars-indicator-left`   | Horizontal offset of the indicator from the list root. |
| `--ars-indicator-top`    | Vertical offset of the indicator from the list root.   |
| `--ars-indicator-width`  | Width of the indicator (matches the active trigger).   |
| `--ars-indicator-height` | Height of the indicator (matches the active trigger).  |

### 2.3 Content Motion

Content panels receive a `data-ars-motion` attribute indicating the animation direction,
enabling CSS-based entry/exit animations:

| Value        | Meaning                                      |
| ------------ | -------------------------------------------- |
| `from-start` | Content enters from the start (left in LTR). |
| `from-end`   | Content enters from the end (right in LTR).  |

The direction is computed by comparing the index of the newly opened item to the previously
open item. If the new item appears later in the list, motion is `from-end`; if earlier, `from-start`.
On the first open (no previous item), no motion attribute is set.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part      | Role / Property    | Value                                                |
| --------- | ------------------ | ---------------------------------------------------- |
| `Root`    | `role`             | `"navigation"` (landmark)                            |
| `Root`    | `aria-label`       | From `Messages.navigation_label` (e.g., "Main")      |
| `List`    | `role`             | `"menubar"`                                          |
| `List`    | `aria-orientation` | `"horizontal"` or `"vertical"`                       |
| `Trigger` | `role`             | `"menuitem"`                                         |
| `Trigger` | `aria-haspopup`    | `"true"`                                             |
| `Trigger` | `aria-expanded`    | `"true"` when content is open, `"false"` when closed |
| `Trigger` | `aria-controls`    | Content panel ID (only when open)                    |
| `Link`    | `aria-current`     | `"page"` when the link represents the current page   |

Items that are direct links (no dropdown content) use a simple `<a>` element with
`role="menuitem"` instead of a trigger+content pair.

### 3.2 Keyboard Interaction

| Key                           | Behavior                                                             |
| ----------------------------- | -------------------------------------------------------------------- |
| `ArrowRight` (horizontal LTR) | Move focus to next trigger.                                          |
| `ArrowLeft` (horizontal LTR)  | Move focus to previous trigger.                                      |
| `ArrowRight` (horizontal RTL) | Move focus to previous trigger (reversed).                           |
| `ArrowLeft` (horizontal RTL)  | Move focus to next trigger (reversed).                               |
| `ArrowDown` (vertical)        | Move focus to next trigger.                                          |
| `ArrowUp` (vertical)          | Move focus to previous trigger.                                      |
| `ArrowDown` (horizontal)      | Open the content panel for the focused trigger.                      |
| `ArrowUp` (horizontal)        | Open the content panel for the focused trigger.                      |
| `Home`                        | Move focus to the first trigger.                                     |
| `End`                         | Move focus to the last trigger.                                      |
| `Enter` / `Space`             | Open the content panel for the focused trigger (or activate a link). |
| `Escape`                      | Close the open content panel and return focus to its trigger.        |
| `Tab`                         | Move focus into the content panel (or out of the navigation menu).   |

> **RTL Handling**: Horizontal keyboard navigation follows the canonical RTL matrix defined
> in `03-accessibility.md` section "Canonical RTL Keyboard Navigation Matrix". In RTL,
> `ArrowRight` moves to the previous trigger and `ArrowLeft` moves to the next trigger.

### 3.3 Focus Management

- **Roving tabindex**: Only one trigger participates in the Tab order at a time
  (`tabindex="0"`). All others have `tabindex="-1"`. Arrow keys move focus between triggers
  without leaving the menubar.
- **Content focus**: When a content panel opens via keyboard (`ArrowDown` or `Enter`/`Space`),
  focus does NOT automatically move into the content. The user presses `Tab` to enter the
  content area. This preserves the menubar keyboard model.
- **Escape restoration**: Pressing `Escape` inside the content or on a trigger closes the
  open panel and returns focus to the trigger that owned it.
- **Focus wraps**: When `loop_focus` is true (default), focus wraps from the last trigger
  to the first and vice versa.

## 4. Internationalization

### 4.1 Messages

```rust
/// Localizable strings for the NavigationMenu component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible label for the navigation landmark (default: "Main").
    pub navigation_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            navigation_label: MessageFn::static_str("Main"),
        }
    }
}

impl ComponentMessages for Messages {}
```

### 4.2 RTL

- `dir="rtl"` on the Root element reverses the visual layout of the trigger list.
- Arrow key semantics reverse per the RTL keyboard matrix: `ArrowLeft` advances forward and
  `ArrowRight` moves backward.
- Vertical orientation is direction-neutral; `ArrowUp`/`ArrowDown` are not affected by `dir`.
- The `data-ars-motion` attribute values (`from-start`, `from-end`) are logical (not physical).
  CSS authors use these in combination with the `dir` attribute to determine physical animation
  direction.

## 5. Variant: Sub-menus

Items in a NavigationMenu can themselves contain nested navigation groups via the `Sub` part.
A `Sub` acts like a scoped `NavigationMenu` embedded within a content panel — it has its own
list of triggers and content panels.

### 5.1 Additional Props

```rust
/// Props for a Sub navigation menu embedded inside a content panel.
/// Inherits `delay_ms`, `skip_delay_ms`, `orientation`, and `dir` from the parent.
#[derive(Clone, Debug, PartialEq)]
pub struct SubProps {
    /// Controlled open item within this sub-menu.
    pub value: Option<Option<Key>>,
    /// Initial open item when uncontrolled.
    pub default_value: Option<Key>,
    /// Callback invoked when the open item changes within this sub-menu.
    pub on_value_change: Option<Callback<Option<Key>>>,
}

impl Default for SubProps {
    fn default() -> Self {
        Self {
            value: None,
            default_value: None,
            on_value_change: None,
        }
    }
}
```

### 5.2 Anatomy Additions

```text
Content
└── Sub
    ├── SubList        role="menubar"
    │   ├── SubItem (xN)
    │   │   ├── SubTrigger  role="menuitem"
    │   │   └── SubContent
    │   └── SubIndicator
    └── SubViewport
```

The `Sub` part reuses the same state machine as the root `NavigationMenu`. The adapter
instantiates a second `Machine` for the sub-menu with its own `SubProps`. The parent content
panel remains open while the sub-menu is interacted with.

### 5.3 Behavior

- Hovering a `SubTrigger` opens its `SubContent` using the same delay logic as the root.
- `Escape` inside a `SubContent` closes the sub-menu first. A second `Escape` closes the
  parent content panel.
- Arrow keys within the sub-menu's list navigate between sub-triggers.
- The `Sub` part inherits `orientation`, `dir`, `delay_ms`, and `skip_delay_ms` from the
  parent `Props` unless the consumer explicitly overrides them.

### 5.4 Accessibility

| Part         | Role / Property    | Value                             |
| ------------ | ------------------ | --------------------------------- |
| `SubList`    | `role`             | `"menubar"`                       |
| `SubList`    | `aria-orientation` | Inherited from parent orientation |
| `SubTrigger` | `role`             | `"menuitem"`                      |
| `SubTrigger` | `aria-haspopup`    | `"true"`                          |
| `SubTrigger` | `aria-expanded`    | `"true"` / `"false"`              |

Keyboard interaction within the sub-menu mirrors the root menu's keyboard patterns.

## 6. Library Parity

> Compared against: Radix UI (`NavigationMenu`).

### 6.1 Props

| Feature             | ars-ui               | Radix UI            | Notes                              |
| ------------------- | -------------------- | ------------------- | ---------------------------------- |
| Controlled value    | `value`              | `value`             | Full match                         |
| Default value       | `default_value`      | `defaultValue`      | Full match                         |
| Delay duration      | `delay_ms`           | `delayDuration`     | Full match (ars-ui: 200ms default) |
| Skip delay duration | `skip_delay_ms`      | `skipDelayDuration` | Full match (ars-ui: 300ms default) |
| Dir                 | `dir`                | `dir`               | Full match                         |
| Orientation         | `orientation`        | `orientation`       | Full match                         |
| Loop focus          | `loop_focus`         | --                  | ars-ui addition                    |
| On value change     | `on_value_change`    | `onValueChange`     | Full match                         |
| Locale / i18n       | `locale`, `messages` | --                  | ars-ui addition                    |

**Gaps:** None. ars-ui covers all Radix props and adds `loop_focus` and i18n.

### 6.2 Anatomy

| Part      | ars-ui           | Radix UI    | Notes      |
| --------- | ---------------- | ----------- | ---------- |
| Root      | `Root` (`<nav>`) | `Root`      | Full match |
| List      | `List`           | `List`      | Full match |
| Item      | `Item`           | `Item`      | Full match |
| Trigger   | `Trigger`        | `Trigger`   | Full match |
| Content   | `Content`        | `Content`   | Full match |
| Link      | `Link`           | `Link`      | Full match |
| Indicator | `Indicator`      | `Indicator` | Full match |
| Viewport  | `Viewport`       | `Viewport`  | Full match |
| Sub       | `Sub` (variant)  | `Sub`       | Full match |

**Gaps:** None.

### 6.3 Events

| Callback                  | ars-ui                                  | Radix UI               | Notes           |
| ------------------------- | --------------------------------------- | ---------------------- | --------------- |
| Value change              | `Bindable` onChange / `on_value_change` | `onValueChange`        | Full match      |
| Content: Escape key       | `EscapeKey` event                       | `onEscapeKeyDown`      | Full match      |
| Content: Pointer outside  | `ContentPointerLeave`                   | `onPointerDownOutside` | Similar concept |
| Content: Focus outside    | --                                      | `onFocusOutside`       | Radix-specific  |
| Content: Interact outside | --                                      | `onInteractOutside`    | Radix-specific  |
| Link: Select              | `SelectLink`                            | `onSelect` (on Link)   | Full match      |

**Gaps:** None. Radix's `onFocusOutside` and `onInteractOutside` are dismiss-layer callbacks that ars-ui handles through its close-delay timer mechanism and the `ContentPointerLeave`/`PointerLeave` events. The behavior is equivalent.

### 6.4 Features

| Feature                           | ars-ui                  | Radix UI                                   |
| --------------------------------- | ----------------------- | ------------------------------------------ | ---------------------------- |
| Hover-triggered open with delay   | Yes                     | Yes                                        |
| Skip-delay window                 | Yes                     | Yes                                        |
| Immediate switch between triggers | Yes                     | Yes                                        |
| Content motion direction          | Yes (`data-ars-motion`) | Yes (`data-motion`)                        |
| Viewport size animation           | Yes (CSS vars)          | Yes (CSS vars)                             |
| Indicator tracking                | Yes (CSS vars)          | Yes                                        |
| Sub-menus                         | Yes (variant)           | Yes (`Sub`)                                |
| Keyboard navigation (arrows)      | Yes                     | Yes                                        |
| Escape closes + focus restore     | Yes                     | Yes                                        |
| RTL support                       | Yes                     | Yes                                        |
| ForceMount                        | --                      | `forceMount` on Content/Indicator/Viewport | ars-ui uses Presence utility |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity.
- **Divergences:** Radix uses `forceMount` for animation control; ars-ui uses the `Presence` utility. Radix fires granular dismiss-layer callbacks (`onFocusOutside`, `onInteractOutside`); ars-ui uses timer-based close logic.
- **Recommended additions:** None.
