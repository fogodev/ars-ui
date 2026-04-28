---
component: Tooltip
category: overlay
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: [z-index-stacking]
related: [hover-card]
references:
    ark-ui: Tooltip
    radix-ui: Tooltip
    react-aria: Tooltip
---

# Tooltip

A brief informational label appearing on hover/focus. Tooltip content is not interactive; use
HoverCard or Popover for focusable floating content.

Per WCAG 1.4.13 (Content on Hover or Focus), hover/focus-triggered content must be:
(a) dismissible without moving pointer or focus (Escape key),
(b) hoverable — the user can move the pointer to the content without it disappearing, and
(c) persistent — remains visible until dismissed, focus lost, or hover ends.
Pressing Escape while the tooltip is visible dismisses it.

## 1. State Machine

### 1.1 States

```rust
/// The states of the tooltip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The tooltip is closed.
    #[default]
    Closed,
    /// The tooltip is waiting for its hover open delay.
    OpenPending,
    /// The tooltip is open.
    Open,
    /// The tooltip is close pending.
    ClosePending,
}
```

### 1.2 Events

```rust
/// The events of the tooltip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// The pointer enters the trigger.
    PointerEnter,
    /// The pointer leaves the trigger.
    PointerLeave,
    /// The trigger gains keyboard focus.
    Focus,
    /// The trigger loses keyboard focus.
    Blur,
    /// The pointer enters the content.
    ContentPointerEnter,
    /// The pointer leaves the content.
    ContentPointerLeave,
    /// The open timer fires.
    OpenTimerFired,
    /// The close timer fires.
    CloseTimerFired,
    /// The close on escape event is triggered.
    CloseOnEscape,
    /// The trigger was clicked/pressed.
    CloseOnClick,
    /// The page was scrolled while the tooltip is open.
    CloseOnScroll,
    /// The tooltip is opened programmatically.
    Open,
    /// The tooltip is closed programmatically.
    Close,
    /// Controlled props synchronized the visible open state.
    SetControlledOpen(bool),
    /// Props changed without changing controlled visible state.
    SyncProps,
    /// Adapter supplied an allocated overlay z-index.
    SetZIndex(u32),
}
```

### 1.3 Context

```rust
use core::time::Duration;

/// The context of the tooltip.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The resolved locale for message formatting.
    pub locale: Locale,
    /// Whether the tooltip is open.
    pub open: bool,
    /// The open delay. (default: 300ms)
    pub open_delay: Duration,
    /// The close delay. (default: 300ms)
    /// Gives pointer time to reach visible tooltip content.
    pub close_delay: Duration,
    /// Whether the tooltip is disabled.
    pub disabled: bool,
    /// Text direction for content rendering.
    pub dir: Direction,
    /// Whether the pointer is currently over the trigger or content.
    pub hover_active: bool,
    /// Whether the trigger currently has keyboard focus.
    pub focus_active: bool,
    /// The positioning options for the tooltip.
    pub positioning: PositioningOptions,
    /// The ID of the trigger element.
    pub trigger_id: String,
    /// The ID of the visible content element.
    pub content_id: String,
    /// The ID of the always-rendered hidden description element.
    pub hidden_description_id: String,
    /// Resolved messages for the tooltip.
    pub messages: Messages,
    /// Adapter-allocated z-index for the positioner.
    pub z_index: Option<u32>,
    /// Touch auto-hide timeout clamped to the accessibility minimum.
    pub touch_auto_hide: Duration,
}
```

### 1.4 Props

```rust
use core::time::Duration;

use ars_core::{Callback, HasId};
use ars_i18n::Direction;

/// The props of the tooltip.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The ID of the tooltip.
    pub id: String,
    /// Controlled open state. When `Some`, the consumer controls open/close entirely.
    pub open: Option<bool>,
    /// Whether the tooltip is open by default (uncontrolled). Default: false.
    pub default_open: bool,
    /// The delay before the tooltip opens. Default: 300ms.
    pub open_delay: Duration,
    /// The delay before the tooltip closes. Default: 300ms.
    pub close_delay: Duration,
    /// Whether the tooltip is disabled. Default: false.
    pub disabled: bool,
    /// The positioning options for the tooltip.
    pub positioning: PositioningOptions,
    /// Whether the tooltip closes on Escape key. Default: true.
    pub close_on_escape: bool,
    /// Whether the tooltip closes when the trigger is clicked. Default: true.
    /// Useful for tooltips on buttons — the click action should dismiss the tooltip.
    pub close_on_click: bool,
    /// Whether the tooltip closes on page scroll. Default: true.
    /// Prevents stale positioning when the trigger scrolls out of view.
    pub close_on_scroll: bool,
    /// Callback invoked when user interaction requests an open-state change.
    pub on_open_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,
    /// When true, tooltip content is not mounted until first opened. Default: false.
    pub lazy_mount: bool,
    /// When true, tooltip content is removed from the DOM after closing. Default: false.
    pub unmount_on_exit: bool,
    /// Text direction for tooltip content. Default: `Direction::Ltr`.
    pub dir: Direction,
    /// Auto-hide timeout for touch devices. Default: 20s.
    /// Minimum enforced: 5000ms — values below are clamped.
    pub touch_auto_hide: Duration,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            open: None,
            default_open: false,
            open_delay: Duration::from_millis(300),
            close_delay: Duration::from_millis(300),
            disabled: false,
            positioning: PositioningOptions::default(),
            close_on_escape: true,
            close_on_click: true,
            close_on_scroll: true,
            on_open_change: None,
            lazy_mount: false,
            unmount_on_exit: false,
            dir: Direction::Ltr,
            touch_auto_hide: Duration::from_secs(20),
        }
    }
}

impl Props {
    /// Returns Tooltip props with documented default values.
    pub fn new() -> Self { Self::default() }

    /// Sets the component instance id.
    pub fn id(mut self, id: impl Into<String>) -> Self { self.id = id.into(); self }
    /// Sets the controlled open state.
    pub fn open(mut self, value: impl Into<Option<bool>>) -> Self { self.open = value.into(); self }
    /// Sets the initial uncontrolled open state.
    pub const fn default_open(mut self, value: bool) -> Self { self.default_open = value; self }
    /// Sets the hover-triggered open delay.
    pub const fn open_delay(mut self, value: Duration) -> Self { self.open_delay = value; self }
    /// Sets the close delay.
    pub const fn close_delay(mut self, value: Duration) -> Self { self.close_delay = value; self }
    /// Sets whether user interaction is ignored.
    pub const fn disabled(mut self, value: bool) -> Self { self.disabled = value; self }
    /// Sets the adapter-owned positioning configuration.
    pub fn positioning(mut self, value: PositioningOptions) -> Self { self.positioning = value; self }
    /// Sets whether Escape dismisses the tooltip.
    pub const fn close_on_escape(mut self, value: bool) -> Self { self.close_on_escape = value; self }
    /// Sets whether trigger activation dismisses the tooltip.
    pub const fn close_on_click(mut self, value: bool) -> Self { self.close_on_click = value; self }
    /// Sets whether page scroll dismisses the tooltip.
    pub const fn close_on_scroll(mut self, value: bool) -> Self { self.close_on_scroll = value; self }
    /// Registers the open-state change callback.
    pub fn on_open_change<F>(mut self, f: F) -> Self
    where
        F: Fn(bool) + Send + Sync + 'static,
    {
        self.on_open_change = Some(Callback::new(f));
        self
    }
    /// Sets whether content is mounted only after first open.
    pub const fn lazy_mount(mut self, value: bool) -> Self { self.lazy_mount = value; self }
    /// Sets whether content is removed from the DOM after closing.
    pub const fn unmount_on_exit(mut self, value: bool) -> Self { self.unmount_on_exit = value; self }
    /// Sets the text direction for tooltip content.
    pub const fn dir(mut self, value: Direction) -> Self { self.dir = value; self }
    /// Sets the adapter-owned touch auto-hide timeout.
    pub const fn touch_auto_hide(mut self, value: Duration) -> Self { self.touch_auto_hide = value; self }
}
```

### 1.5 Hoverable Visible Content (WCAG 1.4.13 Compliance)

Visible tooltip content is hoverable. Entering visible content cancels a pending close and
leaving visible content restarts the close delay. This prevents accidental dismissal while the
pointer crosses the gap between trigger and tooltip.

- `close_delay` may be `Duration::ZERO`. In that case the tooltip closes immediately only when both the
  trigger and visible content are no longer hovered and the trigger does not have focus.
- When `close_delay` is non-zero, the close timer starts after the pointer leaves the trigger
  or visible content. If the pointer enters the other element before the timer fires, the timer is
  cancelled and the tooltip remains open.

Tooltip does not support buttons, links, or other focusable descendants inside content. Use
HoverCard or Popover when the floating surface must be interactive.

```rust
close_delay: props.close_delay,
```

### 1.6 Full Machine Implementation

```rust
use ars_core::{PendingEffect, TransitionPlan};
use core::time::Duration;

const OPEN_DELAY_EFFECT: &str = "tooltip-open-delay";
const CLOSE_DELAY_EFFECT: &str = "tooltip-close-delay";
const OPEN_CHANGE_EFFECT: &str = "tooltip-open-change";
const ALLOCATE_Z_INDEX_EFFECT: &str = "tooltip-allocate-z-index";
const MIN_TOUCH_AUTO_HIDE: Duration = Duration::from_secs(5);

/// The machine of the tooltip.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let ids = ComponentIds::from_id(&props.id);
        let initial_open = props.open.unwrap_or(props.default_open);
        let initial_state = if initial_open { State::Open } else { State::Closed };
        let content_id = ids.part("content");
        (initial_state, Context {
            locale: env.locale.clone(),
            open: initial_open,
            open_delay: props.open_delay,
            close_delay: props.close_delay,
            disabled: props.disabled,
            dir: props.dir,
            hover_active: false,
            focus_active: false,
            positioning: props.positioning.clone(),
            trigger_id: ids.part("trigger"),
            hidden_description_id: format!("{content_id}-description"),
            content_id,
            messages: messages.clone(),
            z_index: None,
            touch_auto_hide: props.touch_auto_hide.max(MIN_TOUCH_AUTO_HIDE),
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled
            && !matches!(
                event,
                Event::SetControlledOpen(_) | Event::SyncProps | Event::SetZIndex(_)
            )
        {
            return None;
        }

        match (state, event) {
            // Hover opens after the configured delay.
            (State::Closed, Event::PointerEnter) => {
                Some(TransitionPlan::to(State::OpenPending)
                    .apply(|ctx| { ctx.hover_active = true; })
                    .with_effect(PendingEffect::named(OPEN_DELAY_EFFECT)))
            }

            // Keyboard focus opens immediately.
            (State::Closed, Event::Focus) => {
                Some(open_plan(props, |ctx| { ctx.focus_active = true; }))
            }

            // Timer fired -> show
            (State::OpenPending, Event::OpenTimerFired) => {
                Some(open_plan(props, |ctx| { ctx.hover_active = true; })
                    .cancel_effect(OPEN_DELAY_EFFECT))
            }

            // Track hover/focus arriving while show delay is pending
            (State::OpenPending, Event::PointerEnter) => {
                Some(TransitionPlan::context_only(|ctx| { ctx.hover_active = true; }))
            }
            (State::OpenPending, Event::Focus) => {
                Some(open_plan(props, |ctx| { ctx.focus_active = true; }))
            }

            // Cancel show delay only if BOTH hover and focus are gone
            (State::OpenPending, Event::PointerLeave) => {
                if ctx.focus_active {
                    Some(TransitionPlan::context_only(|ctx| { ctx.hover_active = false; }))
                } else {
                    Some(TransitionPlan::to(State::Closed)
                        .apply(|ctx| { ctx.hover_active = false; }))
                }
            }
            (State::OpenPending, Event::Blur) => {
                if ctx.hover_active {
                    Some(TransitionPlan::context_only(|ctx| { ctx.focus_active = false; }))
                } else {
                    Some(TransitionPlan::to(State::Closed)
                        .apply(|ctx| { ctx.focus_active = false; }))
                }
            }

            // Escape dismisses from ShowPending
            (State::OpenPending, Event::CloseOnEscape) if props.close_on_escape => {
                Some(TransitionPlan::to(State::Closed)
                    .apply(|ctx| { ctx.hover_active = false; ctx.focus_active = false; })
                    .cancel_effect(OPEN_DELAY_EFFECT))
            }

            // Track additional hover/focus arriving while open
            (State::Open, Event::PointerEnter | Event::ContentPointerEnter) => {
                Some(TransitionPlan::context_only(|ctx| { ctx.hover_active = true; }))
            }
            (State::Open, Event::Focus) => {
                Some(TransitionPlan::context_only(|ctx| { ctx.focus_active = true; }))
            }

            // Open: start hide delay only if OTHER source is also inactive
            (State::Open, Event::PointerLeave) => {
                if ctx.focus_active {
                    // Keyboard focus still on trigger — stay visible
                    Some(TransitionPlan::context_only(|ctx| { ctx.hover_active = false; }))
                } else if ctx.close_delay == Duration::ZERO {
                    Some(close_now_plan(props).apply(|ctx| { ctx.hover_active = false; }))
                } else {
                    Some(TransitionPlan::to(State::ClosePending)
                        .apply(|ctx| { ctx.hover_active = false; })
                        .with_effect(PendingEffect::named(CLOSE_DELAY_EFFECT)))
                }
            }
            (State::Open, Event::Blur) => {
                if ctx.hover_active {
                    // Pointer still over trigger — stay visible
                    Some(TransitionPlan::context_only(|ctx| { ctx.focus_active = false; }))
                } else if ctx.close_delay == Duration::ZERO {
                    Some(close_now_plan(props).apply(|ctx| { ctx.focus_active = false; }))
                } else {
                    Some(TransitionPlan::to(State::ClosePending)
                        .apply(|ctx| { ctx.focus_active = false; })
                        .with_effect(PendingEffect::named(CLOSE_DELAY_EFFECT)))
                }
            }

            // Visible content is hoverable even though tooltip content is not interactive.
            (State::ClosePending, Event::ContentPointerEnter | Event::PointerEnter) => {
                Some(open_plan(props, |ctx| { ctx.hover_active = true; })
                    .cancel_effect(CLOSE_DELAY_EFFECT))
            }

            // Leaving visible content uses the same close-delay path as leaving the trigger.
            (State::Open, Event::ContentPointerLeave) => {
                if ctx.focus_active {
                    Some(TransitionPlan::context_only(|ctx| { ctx.hover_active = false; }))
                } else {
                    Some(TransitionPlan::to(State::ClosePending)
                        .apply(|ctx| { ctx.hover_active = false; })
                        .with_effect(PendingEffect::named(CLOSE_DELAY_EFFECT)))
                }
            }

            // Escape dismisses from Open
            (State::Open, Event::CloseOnEscape) if props.close_on_escape => {
                Some(close_now_plan(props))
            }

            // Click dismisses tooltip
            (State::Open | State::OpenPending, Event::CloseOnClick) => {
                if !props.close_on_click { return None; }
                Some(TransitionPlan::to(State::Closed)
                    .apply(|ctx| { ctx.open = false; ctx.hover_active = false; ctx.focus_active = false; }))
            }

            // Scroll dismisses tooltip
            (State::Open | State::OpenPending, Event::CloseOnScroll) => {
                if !props.close_on_scroll { return None; }
                Some(TransitionPlan::to(State::Closed)
                    .apply(|ctx| { ctx.open = false; ctx.hover_active = false; ctx.focus_active = false; }))
            }

            (State::ClosePending, Event::CloseTimerFired) => {
                Some(close_now_plan(props).cancel_effect(CLOSE_DELAY_EFFECT))
            }

            // Re-focus before hide delay fires — cancel pending hide
            (State::ClosePending, Event::Focus) => {
                Some(open_plan(props, |ctx| { ctx.focus_active = true; })
                    .cancel_effect(CLOSE_DELAY_EFFECT))
            }

            // Escape dismisses from HidePending
            (State::ClosePending, Event::CloseOnEscape) if props.close_on_escape => {
                Some(close_now_plan(props).cancel_effect(CLOSE_DELAY_EFFECT))
            }

            // Programmatic open — skip delay, show immediately
            (State::Closed | State::OpenPending, Event::Open) => {
                Some(open_plan(props, |_| {}))
            }

            // Programmatic close — skip delay, hide immediately
            (State::Open | State::OpenPending | State::ClosePending, Event::Close) => {
                Some(close_now_plan(props))
            }

            // Controlled prop synchronization owns visible open state.
            (_, Event::SetControlledOpen(open)) => {
                Some(sync_controlled_plan(*open, props))
            }

            // Non-open prop changes update resolved context without changing visibility.
            (_, Event::SyncProps) => {
                Some(sync_props_plan(props))
            }

            // Adapter-owned z-index allocation feeds back into core attrs.
            (_, Event::SetZIndex(z)) => {
                Some(TransitionPlan::context_only(move |ctx| { ctx.z_index = Some(*z); }))
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
    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        let mut events = Vec::new();

        match (old.open, new.open) {
            (old_open, Some(new_open)) if old_open != Some(new_open) => {
                events.push(Event::SetControlledOpen(new_open));
            }
            _ if props_context_changed(old, new) => {
                events.push(Event::SyncProps);
            }
            _ => {}
        }

        events
    }
}

// Helper constructors:
// - `open_plan` sets `ctx.open = true` only for uncontrolled tooltips, emits
//   `tooltip-open-change`, and requests adapter z-index allocation via
//   `tooltip-allocate-z-index`. Controlled open requests emit only
//   `tooltip-open-change`; z-index is allocated when `SetControlledOpen(true)`
//   makes the tooltip visibly open.
// - `close_now_plan` sets `ctx.open = false` only for uncontrolled tooltips and emits
//   `tooltip-open-change`.
// - `sync_controlled_plan` updates visible state from `props.open` without firing
//   `on_open_change`.
// - `sync_props_plan` keeps resolved context fields (`disabled`, delays, `dir`,
//   `positioning`, and `touch_auto_hide`) current when props change.
```

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "tooltip"]
pub enum Part {
    Root,
    Trigger,
    HiddenDescription,
    Positioner,
    Content,
    Arrow,
}

/// The API of the tooltip.
pub struct Api<'a> {
    /// The state of the tooltip.
    state: &'a State,
    /// The context of the tooltip.
    ctx: &'a Context,
    /// The props of the tooltip.
    props: &'a Props,
    /// The send function for the tooltip.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Returns `true` if the tooltip is open.
    pub fn is_open(&self) -> bool { self.ctx.open }

    /// The attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        if self.ctx.disabled {
            attrs.set(HtmlAttr::Data("ars-disabled"), "true");
        }
        attrs
    }

    /// The attributes for the trigger element.
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(HtmlAttr::Id, &self.ctx.trigger_id);
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        // Always point to the hidden description span, not just when open
        attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), &self.ctx.hidden_description_id);
        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        attrs
    }

    /// The handler for the trigger pointer enter event.
    pub fn on_trigger_pointer_enter(&self) {
        (self.send)(Event::PointerEnter);
    }

    /// The handler for the trigger pointer leave event.
    pub fn on_trigger_pointer_leave(&self) {
        (self.send)(Event::PointerLeave);
    }

    /// The handler for the trigger focus event.
    pub fn on_trigger_focus(&self) {
        (self.send)(Event::Focus);
    }

    /// The handler for the trigger blur event.
    pub fn on_trigger_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// The handler for trigger activation dismissal.
    pub fn on_trigger_click(&self) {
        if self.props.close_on_click {
            (self.send)(Event::CloseOnClick);
        }
    }

    /// The handler for scroll dismissal.
    pub fn on_scroll(&self) {
        if self.props.close_on_scroll {
            (self.send)(Event::CloseOnScroll);
        }
    }

    /// Returns `true` if the event was handled (Escape dismissed the tooltip).
    /// Adapters MUST call `event.stopPropagation()` when this returns `true`.
    /// This prevents Escape from closing both the Tooltip and a parent Dialog
    /// when the tooltip is open inside a dialog.
    pub fn on_trigger_keydown(&self, data: &KeyboardEventData) -> bool {
        if data.key == KeyboardKey::Escape
            && self.props.close_on_escape
            && matches!(self.state, State::OpenPending | State::Open | State::ClosePending)
        {
            (self.send)(Event::CloseOnEscape);
            true
        } else {
            false
        }
    }

    /// The handler for visible-content pointer enter.
    pub fn on_content_pointer_enter(&self) {
        (self.send)(Event::ContentPointerEnter);
    }

    /// The handler for visible-content pointer leave.
    pub fn on_content_pointer_leave(&self) {
        (self.send)(Event::ContentPointerLeave);
    }

    /// The attributes for the hidden description span.
    /// Always rendered (regardless of open state) for screen reader access.
    pub fn hidden_description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenDescription.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.hidden_description_id);
        attrs.set(HtmlAttr::Data("ars-visually-hidden"), "true");
        // Visually hidden but accessible to screen readers
        attrs
    }

    /// The attributes for the positioner element.
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        attrs.set(HtmlAttr::Data("ars-placement"), self.ctx.positioning.placement.as_str());
        if let Some(z_index) = self.ctx.z_index {
            attrs.set_style(CssProperty::Custom("ars-z-index"), z_index.to_string());
        }
        attrs
    }

    /// The attributes for the content element.
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.content_id);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        attrs.set(HtmlAttr::Dir, self.ctx.dir.as_html_attr());
        attrs
    }

    /// The attributes for the arrow element.
    pub fn arrow_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Arrow.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-placement"), self.ctx.positioning.placement.as_str());
        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::HiddenDescription => self.hidden_description_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::Arrow => self.arrow_attrs(),
        }
    }
}
```

**Core/adapter boundary:** Tooltip core stores `PositioningOptions` and emits
`data-ars-placement`; framework adapters run the `ars-dom` positioning engine and write geometry
CSS custom properties such as `--ars-x`, `--ars-y`, and `--ars-transform-origin`. Tooltip core emits
`tooltip-allocate-z-index` when the tooltip becomes visibly open; adapters allocate with
`ZIndexAllocator`/`next_z_index()` and feed the value back with `Event::SetZIndex(u32)`.

### 1.8 Positioning Options

`ars_components::overlay::positioning` is the canonical DOM-free positioning intent model for
agnostic overlay machines. It is pure data: no DOM references, measurement, viewport reads, or
`ars-dom` dependency. Adapters translate these fields into the concrete DOM positioning engine.

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Placement {
    Bottom,
    BottomStart,
    BottomEnd,
    Top,
    TopStart,
    TopEnd,
    Left,
    LeftStart,
    LeftEnd,
    Right,
    RightStart,
    RightEnd,
    Auto,
    AutoStart,
    AutoEnd,
    Start,
    End,
    StartTop,
    StartBottom,
    EndTop,
    EndBottom,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Offset {
    pub main_axis: f64,
    pub cross_axis: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PositioningOptions {
    pub placement: Placement,
    pub offset: Offset,
    pub flip: bool,
    pub shift: bool,
    pub shift_padding: f64,
    pub arrow_padding: f64,
    pub auto_max_size: bool,
    pub fallback_placements: Vec<Placement>,
    pub keyboard_aware: bool,
    pub auto_placement: bool,
}
```

## 2. Anatomy

```text
Tooltip
├── Root               (required)
├── Trigger            (required — always has aria-describedby pointing to hidden description span)
├── HiddenDescription  (required — visually-hidden span, always rendered, contains tooltip text)
├── Positioner         (required — floating positioning, visible only when open)
├── Content            (required — aria-hidden="true" — visual presentation only)
└── Arrow              (optional)
```

| Part              | Element  | Key Attributes                                           |
| ----------------- | -------- | -------------------------------------------------------- |
| Root              | `<div>`  | `data-ars-scope="tooltip"`, `data-ars-state`             |
| Trigger           | any      | `aria-describedby` pointing to HiddenDescription         |
| HiddenDescription | `<span>` | Visually hidden, always rendered, stable ID              |
| Positioner        | `<div>`  | `data-ars-scope="tooltip"`, `data-ars-part="positioner"` |
| Content           | `<div>`  | `aria-hidden="true"`, `data-ars-state`, `dir`            |
| Arrow             | `<div>`  | `data-ars-scope="tooltip"`, `data-ars-part="arrow"`      |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part              | Property           | Value                                               |
| ----------------- | ------------------ | --------------------------------------------------- |
| Trigger           | `aria-describedby` | HiddenDescription ID (always, not just when open)   |
| HiddenDescription | (visually hidden)  | Contains tooltip text, always in accessibility tree |
| Content           | `aria-hidden`      | `"true"` (visual presentation only)                 |

- **Always-available description**: A visually-hidden `<span>` with a stable ID
  (`{content_id}-description`) MUST always be rendered containing the tooltip text,
  regardless of open state. The trigger MUST always have `aria-describedby` pointing
  to this hidden span. This ensures screen reader users (especially touch-only
  VoiceOver/TalkBack users) always hear the tooltip description when focusing the trigger.
- The visible floating tooltip is a **separate element** (`aria-hidden="true"`) used
  for visual presentation only. It does not carry the accessible description.
- `Tooltip` appears on **both hover and keyboard focus**
- `Tooltip` content is **not interactive** (no buttons, links, or focusable elements inside)
- `Tooltip` does NOT prevent focus from moving to other elements
- Pressing Escape while the tooltip is visible (or pending) dismisses it (WCAG 1.4.13)

### 3.2 Keyboard Interaction

| Key    | Action                              |
| ------ | ----------------------------------- |
| Escape | Dismiss the tooltip                 |
| Tab    | Move focus away (tooltip may close) |

### 3.3 Touch Device Behavior

On touch devices (detected by adapters via pointer event metadata), adapters adjust behavior while
keeping the core DOM-free:

- Opens on tap (via `pointerenter` + `focus` from the tap)
- Remains open until the user taps outside the trigger
- The adapter composes `InteractOutside` (§13 in `05-interactions.md`) to detect outside taps when the tooltip is visible and the pointer type was touch.
- The adapter may auto-hide after the configured timeout (default **20s**) on touch devices, since the user cannot hover to keep it open. Core stores the clamped `touch_auto_hide` value and does not run timers itself.
- The auto-hide timeout is configurable via the `touch_auto_hide: Duration` prop (default: `Duration::from_secs(20)`). Minimum enforced value: `Duration::from_secs(5)` (values below are clamped).
- **Screen reader pause**: When an adapter implements touch auto-hide and can detect assistive-technology reading state, it must pause the timer while the description is being read. Core does not inspect DOM focus or virtual cursor state.
- Escape key dismisses (available on iPad with keyboard)

## 4. Internationalization

### 4.1 Messages

```rust
/// Localizable messages for Tooltip.
#[derive(Clone, Default, Debug)]
pub struct Messages {
    // Currently minimal — provided for consistency with other component
    // Messages structs and future extensibility.
}

impl ComponentMessages for Messages {}
```

- `dir` prop on Props ensures correct rendering in mixed-direction pages (e.g., RTL app with LTR tooltip).
- `data-ars-state` values (`open`, `closed`) are stable API tokens, not localized.

## 5. Variant: Tooltip Provider

### 5.1 Warmup/Cooldown Coordination

When multiple tooltips exist on a page, a **tooltip provider** coordinates their open timing to provide a smoother experience. Without coordination, each tooltip enforces its full `open_delay` even when the user is scanning across multiple triggers in quick succession.

**Behavior:**

- When a tooltip closes and the user moves to another tooltip trigger within the **cooldown window**, the second tooltip opens **instantly** (skipping `open_delay`).
- After the cooldown window expires without tooltip activity, the next tooltip opening uses the full `open_delay` again.

```rust
use core::time::Duration;

/// Global tooltip coordination state.
/// Shared across all Tooltip instances within a TooltipProvider scope.
pub struct TooltipGroup {
    /// Timestamp (performance.now()) of the last tooltip close event.
    pub last_close_at: Option<f64>,
    /// Duration after closing during which the next tooltip opens instantly.
    /// Default: 500ms.
    pub cooldown: Duration,
    /// ID of the currently open tooltip (if any). Only one tooltip may be open at a time.
    pub active_tooltip_id: Option<String>,
}

impl TooltipGroup {
    /// Whether a tooltip should skip its open delay (warm start).
    pub fn is_warm(&self) -> bool {
        match self.last_close_at {
            Some(closed_at) => {
                let elapsed = performance_now() - closed_at;
                elapsed < self.cooldown.as_secs_f64() * 1000.0
            }
            None => false,
        }
    }

    /// Record that a tooltip just closed.
    pub fn record_close(&mut self) {
        self.last_close_at = Some(performance_now());
        self.active_tooltip_id = None;
    }

    /// Record that a tooltip just opened.
    pub fn record_open(&mut self, tooltip_id: &str) {
        self.active_tooltip_id = Some(tooltip_id.to_string());
    }
}
```

**Integration with the state machine:**

When the Tooltip machine receives `PointerEnter` or `Focus` in the `Closed` state, it checks `TooltipGroup::is_warm()`:

- If warm: transition directly to `Open` (skip `OpenPending` and the delay timer)
- If cold: transition to `OpenPending` with the normal delay

**Single-open enforcement:** When a tooltip opens, the provider closes any other currently-open tooltip by sending `Event::Close` to it. Only one tooltip may be visible at a time within a provider scope.

**Keyboard behavior:** Tab-focusing a trigger always opens the tooltip without delay, regardless of warmup state. This matches React Aria's behavior where keyboard users get instant feedback.

## 6. Library Parity

> Compared against: Ark UI (`Tooltip`), Radix UI (`Tooltip`), React Aria (`Tooltip`).

### 6.1 Props

| Feature               | ars-ui               | Ark UI               | Radix UI                        | React Aria                         | Notes                                                          |
| --------------------- | -------------------- | -------------------- | ------------------------------- | ---------------------------------- | -------------------------------------------------------------- |
| Controlled open       | `open`               | `open`               | `open`                          | `isOpen`                           | All libraries                                                  |
| Default open          | `default_open`       | `defaultOpen`        | `defaultOpen`                   | `defaultOpen`                      | All libraries                                                  |
| Open delay            | `open_delay`         | `openDelay`          | `delayDuration`                 | `delay`                            | All libraries; different defaults                              |
| Close delay           | `close_delay`        | `closeDelay`         | --                              | `closeDelay`                       | Ark UI/React Aria; Radix uses provider-level skipDelayDuration |
| Disabled              | `disabled`           | `disabled`           | --                              | `isDisabled`                       | All except Radix                                               |
| Close on Escape       | `close_on_escape`    | `closeOnEscape`      | (onEscapeKeyDown)               | `isKeyboardDismissDisabled`        | All libraries                                                  |
| Close on click        | `close_on_click`     | `closeOnClick`       | --                              | --                                 | Ark UI parity                                                  |
| Close on pointer down | --                   | `closeOnPointerDown` | --                              | --                                 | Subsumed by `close_on_click`                                   |
| Close on scroll       | `close_on_scroll`    | `closeOnScroll`      | --                              | --                                 | Ark UI parity                                                  |
| Positioning           | `positioning`        | `positioning`        | (side/sideOffset/align)         | `placement`/`offset`/`crossOffset` | ars-ui unified struct                                          |
| Dir                   | `dir`                | --                   | --                              | --                                 | ars-ui addition for mixed-direction                            |
| Lazy mount            | `lazy_mount`         | `lazyMount`          | --                              | --                                 | Ark UI parity                                                  |
| Unmount on exit       | `unmount_on_exit`    | `unmountOnExit`      | (forceMount inverse)            | --                                 | Ark UI parity                                                  |
| Open change callback  | `on_open_change`     | `onOpenChange`       | `onOpenChange`                  | `onOpenChange`                     | All libraries                                                  |
| Touch auto-hide       | `touch_auto_hide`    | --                   | --                              | --                                 | Adapter-owned ars-ui addition                                  |
| `aria-label`          | (HiddenDescription)  | `aria-label`         | `aria-label`                    | --                                 | ars-ui uses always-rendered hidden span                        |
| Skip delay duration   | (TooltipGroup)       | --                   | `skipDelayDuration`             | --                                 | Provider-level warmup/cooldown                                 |
| Focus-only trigger    | --                   | --                   | --                              | `trigger="focus"`                  | React Aria only                                                |

**Intentional divergences:** Tooltip does not expose interactive content support. Interactive
floating content belongs in HoverCard or Popover. React Aria's `trigger="focus"` (show only on
focus, not hover) is a niche use case not needed for ars-ui's standard tooltip.

### 6.2 Anatomy

| Part              | ars-ui            | Ark UI     | Radix UI | React Aria     | Notes                                      |
| ----------------- | ----------------- | ---------- | -------- | -------------- | ------------------------------------------ |
| Root              | Root              | Root       | Root     | --             | Container                                  |
| Trigger           | Trigger           | Trigger    | Trigger  | TooltipTrigger | Hover/focus target                         |
| HiddenDescription | HiddenDescription | --         | --       | --             | ars-ui addition for always-accessible text |
| Positioner        | Positioner        | Positioner | --       | --             | Ark UI parity                              |
| Content           | Content           | Content    | Content  | Tooltip        | Visual tooltip                             |
| Arrow             | Arrow             | Arrow      | Arrow    | OverlayArrow   | All libraries                              |
| Provider          | TooltipGroup      | --         | Provider | --             | Warmup/cooldown coordination               |

**Gaps:** None.

### 6.3 Events

| Callback             | ars-ui                | Ark UI           | Radix UI               | React Aria     | Notes                           |
| -------------------- | --------------------- | ---------------- | ---------------------- | -------------- | ------------------------------- |
| Open change          | `on_open_change`      | `onOpenChange`   | `onOpenChange`         | `onOpenChange` | All libraries                   |
| Exit complete        | (Presence)            | `onExitComplete` | --                     | --             | Handled by Presence composition |
| Escape key down      | (via close_on_escape) | --               | `onEscapeKeyDown`      | --             | Radix Content-level callback    |
| Pointer down outside | --                    | --               | `onPointerDownOutside` | --             | Radix Content-level callback    |

**Gaps:** None.

### 6.4 Features

| Feature                       | ars-ui                  | Ark UI | Radix UI                      | React Aria         |
| ----------------------------- | ----------------------- | ------ | ----------------------------- | ------------------ |
| Hover + focus triggers        | Yes                     | Yes    | Yes                           | Yes                |
| Open/close delay              | Yes                     | Yes    | Yes (provider)                | Yes                |
| Interactive content           | No                      | Yes    | Yes (disableHoverableContent) | --                 |
| WCAG 1.4.13 compliance        | Yes                     | --     | --                            | Yes                |
| Touch device handling         | Adapter-owned           | --     | --                            | Yes                |
| Warmup/cooldown               | Yes (TooltipGroup)      | --     | Yes (Provider)                | Yes (built-in)     |
| Single-open enforcement       | Yes (TooltipGroup)      | --     | --                            | Yes                |
| Always-accessible description | Yes (HiddenDescription) | --     | --                            | --                 |
| Close on scroll               | Yes                     | Yes    | --                            | --                 |
| Close on click                | Yes                     | Yes    | --                            | --                 |
| Animation support             | Yes (Presence)          | Yes    | Yes (forceMount)              | Yes (render props) |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity for non-interactive tooltip behavior, with intentional divergence from libraries that allow interactive tooltip content.
- **Divergences:** (1) ars-ui renders a permanently-visible `HiddenDescription` span for screen readers instead of relying on `aria-describedby` pointing to the conditionally-rendered tooltip content; this ensures touch/screen reader users always have access to tooltip text. (2) Warmup/cooldown is provided by `TooltipGroup` struct instead of a React context provider. (3) `touch_auto_hide` is an adapter-owned ars-ui-specific addition for touch devices. (4) Interactive floating content is handled by HoverCard/Popover instead of Tooltip.
- **Recommended additions:** None.
