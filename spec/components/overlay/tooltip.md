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

A brief informational label appearing on hover/focus. **Not interactive** by default.

Per WCAG 1.4.13 (Content on Hover or Focus), hover/focus-triggered content must be:
(a) dismissible without moving pointer or focus (Escape key),
(b) hoverable — the user can move the pointer to the content without it disappearing, and
(c) persistent — remains visible until dismissed, focus lost, or hover ends.
Pressing Escape while the tooltip is visible dismisses it.

## 1. State Machine

### 1.1 States

```rust
/// The states of the tooltip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The tooltip is closed.
    Closed,
    /// The tooltip is open pending.
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
#[derive(Clone, Copy, Debug, PartialEq)]
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
}
```

### 1.3 Context

```rust
/// The context of the tooltip.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The resolved locale for message formatting.
    pub locale: Locale,
    /// Whether the tooltip is open.
    pub open: bool,
    /// The open delay in milliseconds. (default: 300ms)
    pub open_delay_ms: u32,
    /// The close delay in milliseconds. (default: 300ms)
    /// (matches Ark UI convention; gives pointer time to reach interactive content)
    pub close_delay_ms: u32,
    /// Whether the tooltip is disabled.
    pub disabled: bool,
    /// Whether the tooltip is interactive. (default: false)
    /// Can hover tooltip content
    pub interactive: bool,
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
    /// The ID of the content element.
    pub content_id: String,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}
```

### 1.4 Props

```rust
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
    /// The delay in milliseconds before the tooltip opens. Default: 300ms.
    pub open_delay_ms: u32,
    /// The delay in milliseconds before the tooltip closes. Default: 300ms.
    pub close_delay_ms: u32,
    /// Whether the tooltip is disabled. Default: false.
    pub disabled: bool,
    /// Whether the tooltip content is interactive (hoverable). Default: false.
    pub interactive: bool,
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
    /// Callback invoked when the tooltip open state changes.
    pub on_open_change: Option<Callback<bool>>,
    /// When true, tooltip content is not mounted until first opened. Default: false.
    pub lazy_mount: bool,
    /// When true, tooltip content is removed from the DOM after closing. Default: false.
    pub unmount_on_exit: bool,
    /// Text direction for tooltip content. Default: `Direction::Ltr`.
    pub dir: Direction,
    /// Auto-hide timeout in milliseconds for touch devices. Default: 20000ms.
    /// Minimum enforced: 5000ms — values below are clamped.
    pub touch_auto_hide_ms: u32,
    /// Localizable messages for tooltip (see §4.1 Messages).
    pub messages: Option<Messages>,
    /// Optional locale override. When `None`, resolved from the nearest `ArsProvider` context.
    pub locale: Option<Locale>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            open: None,
            default_open: false,
            open_delay_ms: 300,
            close_delay_ms: 300,
            disabled: false,
            interactive: false,
            positioning: PositioningOptions::default(),
            close_on_escape: true,
            close_on_click: true,
            close_on_scroll: true,
            on_open_change: None,
            lazy_mount: false,
            unmount_on_exit: false,
            dir: Direction::Ltr,
            touch_auto_hide_ms: 20000,
            messages: None,
            locale: None,
        }
    }
}
```

### 1.5 Interactive Tooltip Close Delay (WCAG 1.4.13 Compliance)

When `interactive: true`, the tooltip MUST remain visible long enough for the user to move
their pointer from the trigger into the tooltip content. To satisfy WCAG 1.4.13 criterion (b):

- **`interactive: true`**: Minimum `close_delay_ms` of **200ms** is enforced. If the user
  sets a value below 200ms, the machine clamps it to 200ms. When the pointer leaves the
  trigger, the close timer starts; if the pointer enters the tooltip content before the timer
  fires, the timer is cancelled and the tooltip remains open. When the pointer leaves the
  tooltip content, the close timer restarts.
- **`interactive: false`** (default): `close_delay_ms` may be 0ms. The tooltip closes
  immediately on pointer leave since the user has no reason to move into the content.

The init function enforces this minimum:

```rust
close_delay_ms: if props.interactive {
    props.close_delay_ms.max(200)
} else {
    props.close_delay_ms
},
```

### 1.6 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, PendingEffect, AttrMap};

/// The machine of the tooltip.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let ids = ComponentIds::from_id(&props.id);
        let close_delay = if props.interactive {
            props.close_delay_ms.max(200)
        } else {
            props.close_delay_ms
        };
        let initial_open = props.open.unwrap_or(props.default_open);
        let initial_state = if initial_open { State::Open } else { State::Closed };
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        (initial_state, Context {
            locale,
            open: initial_open,
            open_delay_ms: props.open_delay_ms,
            close_delay_ms: close_delay,
            disabled: props.disabled,
            interactive: props.interactive,
            dir: props.dir,
            hover_active: false,
            focus_active: false,
            positioning: props.positioning.clone(),
            trigger_id: ids.part("trigger"),
            content_id: ids.part("content"),
            messages,
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled { return None; }

        match (state, event) {
            // Start show delay on hover
            (State::Closed, Event::PointerEnter) => {
                Some(TransitionPlan::to(State::OpenPending)
                    .apply(|ctx| { ctx.hover_active = true; })
                    .with_effect(PendingEffect::new("show-delay", |ctx, _props, send| {
                        let platform = use_platform_effects();
                        let delay = ctx.open_delay_ms;
                        let handle = platform.set_timeout(delay, Box::new(move || {
                            send(Event::OpenTimerFired);
                        }));
                        let pc = platform.clone();
                        Box::new(move || pc.clear_timeout(handle))
                    })))
            }

            // Start show delay on focus
            (State::Closed, Event::Focus) => {
                Some(TransitionPlan::to(State::OpenPending)
                    .apply(|ctx| { ctx.focus_active = true; })
                    .with_effect(PendingEffect::new("show-delay", |ctx, _props, send| {
                        let platform = use_platform_effects();
                        let delay = ctx.open_delay_ms;
                        let handle = platform.set_timeout(delay, Box::new(move || {
                            send(Event::OpenTimerFired);
                        }));
                        let pc = platform.clone();
                        Box::new(move || pc.clear_timeout(handle))
                    })))
            }

            // Timer fired -> show
            (State::OpenPending, Event::OpenTimerFired) => {
                Some(TransitionPlan::to(State::Open)
                    .apply(|ctx| { ctx.open = true; }))
            }

            // Track hover/focus arriving while show delay is pending
            (State::OpenPending, Event::PointerEnter) => {
                Some(TransitionPlan::context_only(|ctx| { ctx.hover_active = true; }))
            }
            (State::OpenPending, Event::Focus) => {
                Some(TransitionPlan::context_only(|ctx| { ctx.focus_active = true; }))
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
            (State::OpenPending, Event::CloseOnEscape) => {
                Some(TransitionPlan::to(State::Closed)
                    .apply(|ctx| { ctx.hover_active = false; ctx.focus_active = false; }))
            }

            // Track additional hover/focus arriving while open
            (State::Open, Event::PointerEnter) => {
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
                } else if ctx.close_delay_ms == 0 {
                    Some(TransitionPlan::to(State::Closed)
                        .apply(|ctx| { ctx.open = false; ctx.hover_active = false; }))
                } else {
                    Some(TransitionPlan::to(State::ClosePending)
                        .apply(|ctx| { ctx.hover_active = false; })
                        .with_effect(PendingEffect::new("hide-delay", |ctx, _props, send| {
                            let platform = use_platform_effects();
                            let delay = ctx.close_delay_ms;
                            let handle = platform.set_timeout(delay, Box::new(move || {
                                send(Event::CloseTimerFired);
                            }));
                            let pc = platform.clone();
                            Box::new(move || pc.clear_timeout(handle))
                        })))
                }
            }
            (State::Open, Event::Blur) => {
                if ctx.hover_active {
                    // Pointer still over trigger — stay visible
                    Some(TransitionPlan::context_only(|ctx| { ctx.focus_active = false; }))
                } else if ctx.close_delay_ms == 0 {
                    Some(TransitionPlan::to(State::Closed)
                        .apply(|ctx| { ctx.open = false; ctx.focus_active = false; }))
                } else {
                    Some(TransitionPlan::to(State::ClosePending)
                        .apply(|ctx| { ctx.focus_active = false; })
                        .with_effect(PendingEffect::new("hide-delay", |ctx, _props, send| {
                            let platform = use_platform_effects();
                            let delay = ctx.close_delay_ms;
                            let handle = platform.set_timeout(delay, Box::new(move || {
                                send(Event::CloseTimerFired);
                            }));
                            let pc = platform.clone();
                            Box::new(move || pc.clear_timeout(handle))
                        })))
                }
            }

            // Interactive tooltip: cancel hide when pointer enters content
            (State::ClosePending, Event::ContentPointerEnter) if ctx.interactive => {
                Some(TransitionPlan::to(State::Open)
                    .apply(|ctx| { ctx.hover_active = true; }))
            }

            // Interactive tooltip: start hide delay when pointer leaves content
            (State::Open, Event::ContentPointerLeave) if ctx.interactive => {
                if ctx.focus_active {
                    Some(TransitionPlan::context_only(|ctx| { ctx.hover_active = false; }))
                } else {
                    Some(TransitionPlan::to(State::ClosePending)
                        .apply(|ctx| { ctx.hover_active = false; })
                        .with_effect(PendingEffect::new("hide-delay", |ctx, _props, send| {
                            let platform = use_platform_effects();
                            let delay = ctx.close_delay_ms;
                            let handle = platform.set_timeout(delay, Box::new(move || {
                                send(Event::CloseTimerFired);
                            }));
                            let pc = platform.clone();
                            Box::new(move || pc.clear_timeout(handle))
                        })))
                }
            }

            // Escape dismisses from Open
            (State::Open, Event::CloseOnEscape) => {
                Some(TransitionPlan::to(State::Closed)
                    .apply(|ctx| { ctx.open = false; ctx.hover_active = false; ctx.focus_active = false; }))
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
                Some(TransitionPlan::to(State::Closed)
                    .apply(|ctx| { ctx.open = false; }))
            }

            // Re-enter before hide delay fires — cancel pending hide
            (State::ClosePending, Event::PointerEnter) => {
                Some(TransitionPlan::to(State::Open)
                    .apply(|ctx| { ctx.hover_active = true; }))
            }
            (State::ClosePending, Event::Focus) => {
                Some(TransitionPlan::to(State::Open)
                    .apply(|ctx| { ctx.focus_active = true; }))
            }

            // Escape dismisses from HidePending
            (State::ClosePending, Event::CloseOnEscape) => {
                Some(TransitionPlan::to(State::Closed)
                    .apply(|ctx| { ctx.open = false; ctx.hover_active = false; ctx.focus_active = false; }))
            }

            // Programmatic open — skip delay, show immediately
            (State::Closed | State::OpenPending, Event::Open) => {
                Some(TransitionPlan::to(State::Open)
                    .apply(|ctx| { ctx.open = true; }))
            }

            // Programmatic close — skip delay, hide immediately
            (State::Open | State::OpenPending | State::ClosePending, Event::Close) => {
                Some(TransitionPlan::to(State::Closed)
                    .apply(|ctx| { ctx.open = false; ctx.hover_active = false; ctx.focus_active = false; }))
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
        attrs
    }

    /// The attributes for the trigger element.
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        // Always point to the hidden description span, not just when open
        attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), format!("{}-description", self.ctx.content_id));
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

    /// Returns `true` if the event was handled (Escape dismissed the tooltip).
    /// Adapters MUST call `event.stopPropagation()` when this returns `true`.
    /// This prevents Escape from closing both the Tooltip and a parent Dialog
    /// when the tooltip is open inside a dialog.
    pub fn on_trigger_keydown(&self, data: &KeyboardEventData) -> bool {
        if data.key == KeyboardKey::Escape {
            (self.send)(Event::CloseOnEscape);
            true
        } else {
            false
        }
    }

    /// The attributes for the hidden description span.
    /// Always rendered (regardless of open state) for screen reader access.
    pub fn hidden_description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenDescription.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, format!("{}-description", self.ctx.content_id));
        // Visually hidden but accessible to screen readers
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
        attrs.set(HtmlAttr::Id, &self.ctx.content_id);
        attrs.set(HtmlAttr::Role, "tooltip");
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        attrs.set(HtmlAttr::Dir, match self.ctx.dir {
            Direction::Ltr => "ltr",
            Direction::Rtl => "rtl",
        });
        attrs
    }

    /// The attributes for the arrow element.
    pub fn arrow_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Arrow.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
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

## 2. Anatomy

```text
Tooltip
├── Root               (required)
├── Trigger            (required — always has aria-describedby pointing to hidden description span)
├── HiddenDescription  (required — visually-hidden span, always rendered, contains tooltip text)
├── Positioner         (required — floating positioning, visible only when open)
├── Content            (required — role="tooltip", aria-hidden="true" — visual presentation only)
└── Arrow              (optional)
```

| Part              | Element  | Key Attributes                                           |
| ----------------- | -------- | -------------------------------------------------------- |
| Root              | `<div>`  | `data-ars-scope="tooltip"`, `data-ars-state`             |
| Trigger           | any      | `aria-describedby` pointing to HiddenDescription         |
| HiddenDescription | `<span>` | Visually hidden, always rendered, stable ID              |
| Positioner        | `<div>`  | `data-ars-scope="tooltip"`, `data-ars-part="positioner"` |
| Content           | `<div>`  | `role="tooltip"`, `data-ars-state`, `dir`                |
| Arrow             | `<div>`  | `data-ars-scope="tooltip"`, `data-ars-part="arrow"`      |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part              | Property           | Value                                               |
| ----------------- | ------------------ | --------------------------------------------------- |
| Content           | `role`             | `"tooltip"`                                         |
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

When `interactive: true`, a `DismissButton` SHOULD be rendered inside the tooltip content for screen reader users who cannot hover/focus away.

### 3.2 Keyboard Interaction

| Key    | Action                              |
| ------ | ----------------------------------- |
| Escape | Dismiss the tooltip                 |
| Tab    | Move focus away (tooltip may close) |

### 3.3 Touch Device Behavior

On touch devices (detected via `pointerType === "touch"`), the Tooltip adjusts its behavior:

- Opens on tap (via `pointerenter` + `focus` from the tap)
- Remains open until the user taps outside the trigger
- The adapter composes `InteractOutside` (§13 in `05-interactions.md`) to detect outside taps when the tooltip is visible and the pointer type was touch
- Auto-hides after a configurable timeout (default **20000ms**) on touch devices, since the user cannot hover to keep it open. The minimum of 20 seconds satisfies WCAG 2.2.1 (Timing Adjustable) which requires content to remain visible long enough for users to read it. The previous default of 5000ms was insufficient for users with cognitive or reading disabilities.
- The auto-hide timeout is configurable via the `touch_auto_hide_ms: u32` prop (default: `20000`). Minimum enforced value: `5000` (values below are clamped).
- **Screen reader pause**: When a screen reader's virtual cursor (focus) enters the tooltip's live region content, the auto-hide timer is paused. The timer resumes when screen reader focus leaves the tooltip. This ensures assistive technology users are not interrupted mid-read.
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

When multiple tooltips exist on a page, a **tooltip provider** coordinates their open timing to provide a smoother experience. Without coordination, each tooltip enforces its full `open_delay_ms` even when the user is scanning across multiple triggers in quick succession.

**Behavior:**

- When a tooltip closes and the user moves to another tooltip trigger within the **cooldown window**, the second tooltip opens **instantly** (skipping `open_delay_ms`).
- After the cooldown window expires without tooltip activity, the next tooltip opening uses the full `open_delay_ms` again.

```rust
/// Global tooltip coordination state.
/// Shared across all Tooltip instances within a TooltipProvider scope.
pub struct TooltipGroup {
    /// Timestamp (performance.now()) of the last tooltip close event.
    pub last_close_at: Option<f64>,
    /// Duration (ms) after closing during which the next tooltip opens instantly.
    /// Default: 500ms.
    pub cooldown_ms: u32,
    /// ID of the currently open tooltip (if any). Only one tooltip may be open at a time.
    pub active_tooltip_id: Option<String>,
}

impl TooltipGroup {
    /// Whether a tooltip should skip its open delay (warm start).
    pub fn is_warm(&self) -> bool {
        match self.last_close_at {
            Some(closed_at) => {
                let elapsed = performance_now() - closed_at;
                elapsed < self.cooldown_ms as f64
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
| Open delay            | `open_delay_ms`      | `openDelay`          | `delayDuration`                 | `delay`                            | All libraries; different defaults                              |
| Close delay           | `close_delay_ms`     | `closeDelay`         | --                              | `closeDelay`                       | Ark UI/React Aria; Radix uses provider-level skipDelayDuration |
| Disabled              | `disabled`           | `disabled`           | --                              | `isDisabled`                       | All except Radix                                               |
| Interactive           | `interactive`        | `interactive`        | `disableHoverableContent` (inv) | --                                 | Ark UI parity; Radix inverts semantics                         |
| Close on Escape       | `close_on_escape`    | `closeOnEscape`      | (onEscapeKeyDown)               | `isKeyboardDismissDisabled`        | All libraries                                                  |
| Close on click        | `close_on_click`     | `closeOnClick`       | --                              | --                                 | Ark UI parity                                                  |
| Close on pointer down | --                   | `closeOnPointerDown` | --                              | --                                 | Subsumed by `close_on_click`                                   |
| Close on scroll       | `close_on_scroll`    | `closeOnScroll`      | --                              | --                                 | Ark UI parity                                                  |
| Positioning           | `positioning`        | `positioning`        | (side/sideOffset/align)         | `placement`/`offset`/`crossOffset` | ars-ui unified struct                                          |
| Dir                   | `dir`                | --                   | --                              | --                                 | ars-ui addition for mixed-direction                            |
| Lazy mount            | `lazy_mount`         | `lazyMount`          | --                              | --                                 | Ark UI parity                                                  |
| Unmount on exit       | `unmount_on_exit`    | `unmountOnExit`      | (forceMount inverse)            | --                                 | Ark UI parity                                                  |
| Open change callback  | `on_open_change`     | `onOpenChange`       | `onOpenChange`                  | `onOpenChange`                     | All libraries                                                  |
| Touch auto-hide       | `touch_auto_hide_ms` | --                   | --                              | --                                 | ars-ui addition for WCAG compliance                            |
| `aria-label`          | (HiddenDescription)  | `aria-label`         | `aria-label`                    | --                                 | ars-ui uses always-rendered hidden span                        |
| Skip delay duration   | (TooltipGroup)       | --                   | `skipDelayDuration`             | --                                 | Provider-level warmup/cooldown                                 |
| Focus-only trigger    | --                   | --                   | --                              | `trigger="focus"`                  | React Aria only                                                |

**Gaps:** None. React Aria's `trigger="focus"` (show only on focus, not hover) is a niche use case not needed for ars-ui's standard tooltip.

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
| Interactive content           | Yes                     | Yes    | Yes (disableHoverableContent) | --                 |
| WCAG 1.4.13 compliance        | Yes                     | --     | --                            | Yes                |
| Touch device handling         | Yes                     | --     | --                            | Yes                |
| Warmup/cooldown               | Yes (TooltipGroup)      | --     | Yes (Provider)                | Yes (built-in)     |
| Single-open enforcement       | Yes (TooltipGroup)      | --     | --                            | Yes                |
| Always-accessible description | Yes (HiddenDescription) | --     | --                            | --                 |
| Close on scroll               | Yes                     | Yes    | --                            | --                 |
| Close on click                | Yes                     | Yes    | --                            | --                 |
| Animation support             | Yes (Presence)          | Yes    | Yes (forceMount)              | Yes (render props) |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity.
- **Divergences:** (1) ars-ui renders a permanently-visible `HiddenDescription` span for screen readers instead of relying on `aria-describedby` pointing to the conditionally-rendered tooltip content; this ensures touch/screen reader users always have access to tooltip text. (2) Warmup/cooldown is provided by `TooltipGroup` struct instead of a React context provider. (3) `touch_auto_hide_ms` is an ars-ui-specific addition for WCAG 2.2.1 compliance on touch devices.
- **Recommended additions:** None.
