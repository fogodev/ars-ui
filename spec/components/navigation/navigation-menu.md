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
    /// Locale override. When `None`, inherits from nearest `ArsProvider` context.
    pub locale: Option<Locale>,
    /// Localizable strings.
    pub messages: Option<Messages>,
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
            locale: None,
            messages: None,
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

### 1.6 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, PendingEffect, Bindable, AttrMap};
use ars_collections::Key;
use ars_i18n::{Orientation, Direction};

// ── Machine ──────────────────────────────────────────────────────────────────

/// Machine for the NavigationMenu component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let (initial_value, bindable) = match &props.value {
            Some(v) => (v.clone(), Bindable::controlled(v.clone())),
            None    => (props.default_value.clone(), Bindable::uncontrolled(props.default_value.clone())),
        };
        let initial_state = match &initial_value {
            Some(key) => State::Open { item: key.clone() },
            None      => State::Idle,
        };
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        let ids = ComponentIds::from_id(&props.id);
        let list_id = ids.part("list");
        (initial_state, Context {
            value: bindable,
            focused_trigger: None,
            focus_visible: false,
            orientation: props.orientation,
            dir: props.dir,
            delay_ms: props.delay_ms,
            skip_delay_ms: props.skip_delay_ms,
            last_close_time: None,
            pointer_in_content: false,
            items: Vec::new(),
            previous_item: None,
            list_id,
            ids,
            locale,
            messages,
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
            (_, Event::Open(key)) => {
                // Guard: already open on this item.
                if matches!(state, State::Open { item } if item == key) {
                    return None;
                }
                let key = key.clone();
                let prev = match state {
                    State::Open { item } => Some(item.clone()),
                    _ => None,
                };
                Some(TransitionPlan::to(State::Open { item: key.clone() })
                    .apply(move |ctx| {
                        ctx.previous_item = prev;
                        ctx.value.set(Some(key));
                    }))
            }

            // ── Close ────────────────────────────────────────────────────────
            (State::Open { .. }, Event::Close(now_ms)) => {
                let now = *now_ms;
                Some(TransitionPlan::to(State::Idle)
                    .apply(move |ctx| {
                        ctx.previous_item = ctx.value.get().clone();
                        ctx.value.set(None);
                        ctx.pointer_in_content = false;
                        ctx.last_close_time = Some(now);
                    }))
            }

            // ── PointerEnter ─────────────────────────────────────────────────
            // Hover a trigger: if within skip-delay window, open immediately;
            // otherwise start the open delay timer.
            (_, Event::PointerEnter(key, now_ms)) => {
                // Guard: already open on this item.
                if matches!(state, State::Open { item } if item == key) {
                    return None;
                }
                let key = key.clone();
                let now = *now_ms;
                let prev = match state {
                    State::Open { item } => Some(item.clone()),
                    _ => None,
                };
                // When another item is already open, skip the delay (moving between triggers).
                let already_open = matches!(state, State::Open { .. });
                if already_open {
                    Some(TransitionPlan::to(State::Open { item: key.clone() })
                        .apply(move |ctx| {
                            ctx.previous_item = prev;
                            ctx.value.set(Some(key));
                            ctx.pointer_in_content = false;
                        }))
                } else if in_skip_delay_window(ctx, now) {
                    // Within skip-delay window: open immediately without timer.
                    Some(TransitionPlan::to(State::Open { item: key.clone() })
                        .apply(move |ctx| {
                            ctx.previous_item = prev;
                            ctx.value.set(Some(key));
                            ctx.pointer_in_content = false;
                        }))
                } else {
                    // Start open delay timer.
                    let delay = ctx.delay_ms;
                    Some(TransitionPlan::context_only(|_| {})
                        .with_effect(PendingEffect::new("open-delay", move |_ctx, _props, send| {
                            let platform = use_platform_effects();
                            let key_clone = key.clone();
                            let handle = platform.set_timeout(delay, Box::new(move || {
                                send(Event::OpenTimerFired(key_clone));
                            }));
                            let pc = platform.clone();
                            Box::new(move || pc.clear_timeout(handle))
                        })))
                }
            }

            // ── PointerLeave ─────────────────────────────────────────────────
            // Start close delay when pointer leaves trigger (but not if pointer
            // moves into content).
            (State::Open { .. }, Event::PointerLeave) => {
                Some(TransitionPlan::context_only(|_| {})
                    .with_effect(PendingEffect::new("close-delay", |ctx, _props, send| {
                        let platform = use_platform_effects();
                        let delay = ctx.delay_ms;
                        let handle = platform.set_timeout(delay, Box::new(move || {
                            let platform_inner = use_platform_effects();
                            send(Event::CloseTimerFired(platform_inner.now_ms()));
                        }));
                        let pc = platform.clone();
                        Box::new(move || pc.clear_timeout(handle))
                    })))
            }

            // When idle and pointer leaves (e.g., from a trigger that hadn't opened yet),
            // cancel any pending open effect by returning a no-op transition.
            (State::Idle, Event::PointerLeave) => {
                Some(TransitionPlan::context_only(|_| {}))
            }

            // ── ContentPointerEnter ──────────────────────────────────────────
            // Cancel any pending close when the pointer moves into content.
            (State::Open { .. }, Event::ContentPointerEnter) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.pointer_in_content = true;
                }))
            }

            // ── ContentPointerLeave ──────────────────────────────────────────
            // Start close delay when pointer leaves the content area.
            (State::Open { .. }, Event::ContentPointerLeave) => {
                Some(TransitionPlan::context_only(|ctx| {
                        ctx.pointer_in_content = false;
                    })
                    .with_effect(PendingEffect::new("close-delay", |ctx, _props, send| {
                        let platform = use_platform_effects();
                        let delay = ctx.delay_ms;
                        let handle = platform.set_timeout(delay, Box::new(move || {
                            let platform_inner = use_platform_effects();
                            send(Event::CloseTimerFired(platform_inner.now_ms()));
                        }));
                        let pc = platform.clone();
                        Box::new(move || pc.clear_timeout(handle))
                    })))
            }

            // ── OpenTimerFired ───────────────────────────────────────────────
            (State::Idle, Event::OpenTimerFired(key)) => {
                let key = key.clone();
                Some(TransitionPlan::to(State::Open { item: key.clone() })
                    .apply(move |ctx| {
                        ctx.previous_item = None;
                        ctx.value.set(Some(key));
                    }))
            }

            // ── CloseTimerFired ──────────────────────────────────────────────
            // Only close if pointer is not currently inside the content.
            (State::Open { .. }, Event::CloseTimerFired(now_ms)) => {
                if ctx.pointer_in_content {
                    return None;
                }
                let now = *now_ms;
                Some(TransitionPlan::to(State::Idle)
                    .apply(move |ctx| {
                        ctx.previous_item = ctx.value.get().clone();
                        ctx.value.set(None);
                        ctx.pointer_in_content = false;
                        ctx.last_close_time = Some(now);
                    }))
            }

            // ── FocusTrigger ─────────────────────────────────────────────────
            (_, Event::FocusTrigger { item, is_keyboard }) => {
                let item = item.clone();
                let is_kb = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused_trigger = Some(item);
                    ctx.focus_visible = is_kb;
                }))
            }

            // ── FocusNext ────────────────────────────────────────────────────
            (_, Event::FocusNext) => {
                let items = ctx.items.clone();
                let total = items.len();
                if total == 0 { return None; }
                let loop_focus = props.loop_focus;
                let idx = ctx.focused_trigger.as_ref()
                    .and_then(|t| items.iter().position(|i| i == t))
                    .unwrap_or(0);
                let next_idx = if loop_focus {
                    (idx + 1) % total
                } else {
                    (idx + 1).min(total.saturating_sub(1))
                };
                if next_idx == idx && !loop_focus { return None; }
                let next = items[next_idx].clone();
                Some(TransitionPlan::context_only(move |ctx| {
                        ctx.focused_trigger = Some(next.clone());
                        ctx.focus_visible = true;
                    })
                    .with_effect(PendingEffect::new("focus-trigger", move |_ctx, _props, send| {
                        let next_id = items[next_idx].to_string();
                        send(Event::RequestFocus { target_id: next_id });
                        no_cleanup()
                    })))
            }

            // ── FocusPrev ────────────────────────────────────────────────────
            (_, Event::FocusPrev) => {
                let items = ctx.items.clone();
                let total = items.len();
                if total == 0 { return None; }
                let loop_focus = props.loop_focus;
                let idx = ctx.focused_trigger.as_ref()
                    .and_then(|t| items.iter().position(|i| i == t))
                    .unwrap_or(0);
                let prev_idx = if loop_focus {
                    if idx == 0 { total - 1 } else { idx - 1 }
                } else {
                    idx.saturating_sub(1)
                };
                if prev_idx == idx && !loop_focus { return None; }
                let prev = items[prev_idx].clone();
                Some(TransitionPlan::context_only(move |ctx| {
                        ctx.focused_trigger = Some(prev.clone());
                        ctx.focus_visible = true;
                    })
                    .with_effect(PendingEffect::new("focus-trigger", move |_ctx, _props, send| {
                        let prev_id = items[prev_idx].to_string();
                        send(Event::RequestFocus { target_id: prev_id });
                        no_cleanup()
                    })))
            }

            // ── FocusFirst ───────────────────────────────────────────────────
            (_, Event::FocusFirst) => {
                let items = ctx.items.clone();
                let first = items.first().cloned();
                if let Some(first) = first {
                    let first_clone = first.clone();
                    Some(TransitionPlan::context_only(move |ctx| {
                            ctx.focused_trigger = Some(first_clone.clone());
                            ctx.focus_visible = true;
                        })
                        .with_effect(PendingEffect::new("focus-trigger", move |_ctx, _props, send| {
                            send(Event::RequestFocus { target_id: first.to_string() });
                            no_cleanup()
                        })))
                } else {
                    None
                }
            }

            // ── FocusLast ────────────────────────────────────────────────────
            (_, Event::FocusLast) => {
                let items = ctx.items.clone();
                let last = items.last().cloned();
                if let Some(last) = last {
                    let last_clone = last.clone();
                    Some(TransitionPlan::context_only(move |ctx| {
                            ctx.focused_trigger = Some(last_clone.clone());
                            ctx.focus_visible = true;
                        })
                        .with_effect(PendingEffect::new("focus-trigger", move |_ctx, _props, send| {
                            send(Event::RequestFocus { target_id: last.to_string() });
                            no_cleanup()
                        })))
                } else {
                    None
                }
            }

            // ── SelectLink ───────────────────────────────────────────────────
            // A link inside the content panel was activated. Close the menu.
            (State::Open { .. }, Event::SelectLink(now_ms)) => {
                let now = *now_ms;
                Some(TransitionPlan::to(State::Idle)
                    .apply(move |ctx| {
                        ctx.previous_item = ctx.value.get().clone();
                        ctx.value.set(None);
                        ctx.pointer_in_content = false;
                        ctx.last_close_time = Some(now);
                    }))
            }

            // ── EscapeKey ────────────────────────────────────────────────────
            // Close the submenu and return focus to its trigger.
            (State::Open { item }, Event::EscapeKey(now_ms)) => {
                let trigger_key = item.clone();
                let now = *now_ms;
                Some(TransitionPlan::to(State::Idle)
                    .apply(move |ctx| {
                        ctx.previous_item = ctx.value.get().clone();
                        ctx.value.set(None);
                        ctx.pointer_in_content = false;
                        ctx.last_close_time = Some(now);
                    })
                    .with_effect(PendingEffect::new("focus-trigger-on-escape", move |_ctx, _props, send| {
                        send(Event::RequestFocus { target_id: trigger_key.to_string() });
                        no_cleanup()
                    })))
            }

            // ── RequestFocus ─────────────────────────────────────────────────
            (_, Event::RequestFocus { target_id }) => {
                let target_id = target_id.clone();
                Some(TransitionPlan::context_only(|_| {})
                    .with_effect(PendingEffect::new("focus-element", move |_ctx, _props, _send| {
                        let platform = use_platform_effects();
                        platform.focus_element_by_id(&target_id);
                        no_cleanup()
                    })))
            }

            // ── SetDirection ─────────────────────────────────────────────────
            (_, Event::SetDirection(dir)) => {
                let dir = *dir;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.dir = dir;
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
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "navigation-menu"]
pub enum Part {
    Root,
    List,
    Item { item_key: Key },
    Trigger { item_key: Key, content_id: String },
    Content { item_key: Key },
    Link { active: bool },
    Indicator,
    Viewport,
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
    /// Get the key of the currently open item, if any.
    pub fn open_item(&self) -> Option<&Key> {
        self.ctx.value.get().as_ref()
    }

    /// Check whether a specific item's content is currently showing.
    pub fn is_item_open(&self, item_key: &Key) -> bool {
        self.ctx.value.get().as_ref() == Some(item_key)
    }

    /// Compute the motion direction for content animation.
    /// Returns `Some("from-start")`, `Some("from-end")`, `Some("to-start")`, or `Some("to-end")`.
    /// Returns `None` if no previous item exists (first open, no animation direction).
    fn motion_direction(&self, item_key: &Key) -> Option<&'static str> {
        let prev = self.ctx.previous_item.as_ref()?;
        let items = &self.ctx.items;
        let prev_idx = items.iter().position(|k| k == prev)?;
        let curr_idx = items.iter().position(|k| k == item_key)?;
        if curr_idx > prev_idx {
            Some("from-end")
        } else {
            Some("from-start")
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
        attrs.set(HtmlAttr::Id, item_key.to_string());
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
        // Roving tabindex: first trigger gets tabindex="0" unless another is focused.
        let is_first = self.ctx.items.first() == Some(item_key);
        let has_focus = self.ctx.focused_trigger.is_some();
        let tab_index = if is_focused {
            "0"
        } else if !has_focus && is_first {
            "0"
        } else {
            "-1"
        };
        attrs.set(HtmlAttr::TabIndex, tab_index);
        attrs
    }

    /// Attrs for a content panel revealed when its trigger is active.
    pub fn content_attrs(&self, item_key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_open = self.is_item_open(item_key);

        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Content { item_key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.item("content", item_key));
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
        let is_visible = matches!(self.state, State::Open { .. });
        attrs.set(HtmlAttr::Data("ars-state"), if is_visible { "visible" } else { "hidden" });
        attrs
    }

    /// Attrs for the optional viewport container that animates between content sizes.
    pub fn viewport_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Viewport.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Open { .. } => "open",
            State::Idle => "closed",
        });
        // CSS custom properties for viewport sizing are set as inline styles by the adapter.
        // --ars-viewport-width: width of the currently active content panel.
        // --ars-viewport-height: height of the currently active content panel.
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
    pub fn on_trigger_keydown(&self, item_key: &Key, data: &KeyboardEventData) {
        let (prev_key, next_key) = match (&self.ctx.orientation, &self.ctx.dir) {
            (Orientation::Horizontal, Direction::Ltr)  => (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight),
            (Orientation::Horizontal, Direction::Rtl)  => (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft),
            (Orientation::Horizontal, Direction::Auto) => (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight),
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
            // Adapter provides now_ms from platform.now_ms() at dispatch time.
            let platform = use_platform_effects();
            (self.send)(Event::EscapeKey(platform.now_ms()));
        } else if data.key == KeyboardKey::ArrowDown
            && self.ctx.orientation == Orientation::Horizontal {
            // In horizontal mode, ArrowDown opens the content panel.
            (self.send)(Event::Open(item_key.clone()));
        } else if data.key == KeyboardKey::ArrowUp
            && self.ctx.orientation == Orientation::Horizontal {
            // ArrowUp in horizontal mode also opens (for symmetry with ArrowDown).
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
    pub fn on_content_keydown(&self, data: &KeyboardEventData) {
        if data.key == KeyboardKey::Escape {
            // Adapter provides now_ms from platform.now_ms() at dispatch time.
            let platform = use_platform_effects();
            (self.send)(Event::EscapeKey(platform.now_ms()));
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
        match &part {
            Part::Root => self.root_attrs(),
            Part::List => self.list_attrs(),
            Part::Item { item_key } => self.item_attrs(item_key),
            Part::Trigger { item_key, content_id } => self.trigger_attrs(item_key, content_id),
            Part::Content { item_key } => self.content_attrs(item_key),
            Part::Link { active } => self.link_attrs(*active),
            Part::Indicator => self.indicator_attrs(),
            Part::Viewport => self.viewport_attrs(),
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
