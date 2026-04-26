---
component: Popover
category: overlay
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: [z-index-stacking]
related: [dialog]
references:
    ark-ui: Popover
    radix-ui: Popover
    react-aria: Popover
---

# Popover

A non-modal overlay anchored to a trigger element for rich content.

Non-modal popovers use `role="group"` to avoid confusing screen readers (JAWS announces 'dialog' and users expect Tab trapping). Reserve `role="dialog"` with `aria-modal="true"` for truly modal popovers that trap focus.

## 1. State Machine

### 1.1 States

```rust
/// The states of the popover.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// The popover is closed.
    Closed,
    /// The popover is open.
    Open,
}
```

### 1.2 Events

```rust
/// The events of the popover.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// The popover is opened.
    Open,
    /// The popover is closed.
    Close,
    /// The popover is toggled.
    Toggle,
    /// The popover is closed on escape.
    CloseOnEscape,
    /// The popover is closed on interact outside.
    CloseOnInteractOutside,
    /// The positioning update is received.
    PositioningUpdate(PositioningResult),
}
```

### 1.3 Context

```rust
/// The context of the popover.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The resolved locale for message formatting.
    pub locale: Locale,
    /// Whether the popover is open.
    pub open: bool,
    /// Whether the popover is modal.
    pub modal: bool,
    /// The ID of the trigger element.
    pub trigger_id: String,
    /// The ID of the content element.
    pub content_id: String,
    /// The ID of the title element.
    pub title_id: Option<String>,
    /// The ID of the description element.
    pub description_id: Option<String>,
    /// Latest positioning result from the positioning engine.
    pub positioning: Option<PositioningResult>,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}
```

### 1.4 Props

```rust
/// The props of the popover.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The ID of the popover.
    pub id: String,
    /// Controlled open state. When `Some`, the consumer owns the open state.
    pub open: Option<bool>,
    /// Default open state for uncontrolled mode. Default: `false`.
    pub default_open: bool,
    /// Whether the popover is modal.
    pub modal: bool,
    /// Whether the popover is closed on escape.
    pub close_on_escape: bool,
    /// Whether the popover is closed on interact outside.
    pub close_on_interact_outside: bool,
    /// The positioning options for the popover.
    pub positioning: PositioningOptions,
    /// Convenience alias that populates `positioning.offset`.
    /// Distance (in pixels) between the trigger and the popover along the main axis.
    /// Default: `0.0`.
    pub offset: f64,
    /// Convenience alias that populates `positioning.cross_axis_offset`.
    /// Distance (in pixels) between the trigger and the popover along the cross axis.
    /// Default: `0.0`.
    pub cross_offset: f64,
    /// When true, the popover content matches the trigger (or anchor) element's width.
    /// Sets `min-width` on the positioner to the trigger's `offsetWidth`.
    /// Useful for dropdown-style popovers that should align with their trigger. Default: false.
    pub same_width: bool,
    /// Whether the popover is a portal.
    pub portal: bool,
    /// When true, popover content is not mounted until first opened. Default: false.
    pub lazy_mount: bool,
    /// Whether the popover content is removed from the DOM after closing.
    /// When true, popover content is removed from the DOM after closing.
    /// Works with Presence for exit animations. Default: false.
    pub unmount_on_exit: bool,
    /// Callback invoked when the popover open state changes.
    /// Fires after the transition with the new open state value (`true` for open, `false` for close).
    pub on_open_change: Option<Callback<bool>>,
    // Change callbacks provided by the adapter layer
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            open: None,
            default_open: false,
            modal: false,
            close_on_escape: true,
            close_on_interact_outside: true,
            positioning: PositioningOptions::default(),
            offset: 0.0,
            cross_offset: 0.0,
            same_width: false,
            portal: true,
            lazy_mount: false,
            unmount_on_exit: false,
            on_open_change: None,
        }
    }
}
```

### 1.5 Click-Outside Race Prevention

When a popover opens, attaching a click-outside listener synchronously creates a race
condition: the same click event that triggered the popover bubbles up to `document` and
immediately closes it. Adapters MUST guard against this.

**Strategy 1 — rAF Delay (recommended):**
Defer the click-outside listener attachment by one `requestAnimationFrame` after the state
machine transitions to `Open`. This ensures the originating click has fully propagated
before the listener becomes active.

**Strategy 2 — Timestamp Comparison (fallback):**
Record the `event.timeStamp` of the triggering click. The click-outside handler ignores
any event whose `timeStamp` is less than or equal to the recorded value.

**Cleanup ordering:** Adapters MUST remove existing click-outside listeners BEFORE
attaching new ones during state transitions. This prevents duplicate listeners from
accumulating during rapid interactions.

**Rapid open/close guard:** If the state transitions to `Closed` before the deferred rAF
callback fires, the pending listener attachment MUST be cancelled. Otherwise a stale
listener attaches to an already-closed popover.

```rust
/// Adapter-level click-outside guard. This is not part of the headless
/// state machine — it lives in the adapter's effect/subscription layer.

/// Strategy 1: rAF-based deferral
struct ClickOutsideGuard {
    /// Handle to the pending rAF callback, used for cancellation.
    pending_raf: Option<RafHandle>,
    /// Handle to the active click-outside listener, used for cleanup.
    active_listener: Option<ListenerHandle>,
}

impl ClickOutsideGuard {
    /// Attach the click-outside listener deferred.
    fn attach_deferred(&mut self, content_el: ElementRef, on_close: impl Fn() + 'static) {
        // Always remove existing listener first (cleanup ordering).
        self.detach();

        let guard_active = Rc::new(Cell::new(true));
        let guard_clone = guard_active.clone();

        self.pending_raf = Some(request_animation_frame(move || {
            if !guard_clone.get() {
                // State transitioned to Closed before rAF fired — bail out.
                return;
            }
            // Now safe to listen: the triggering click has fully propagated.
            let handle = document().add_event_listener("pointerdown", move |e: PointerEvent| {
                if !content_el.contains(e.target_element()) {
                    on_close();
                }
            });
            // Store handle for later cleanup (via detach).
            // In practice the adapter stores this in its own reactive state.
            let _ = handle;
        }));
    }

    /// Detach the click-outside listener.
    fn detach(&mut self) {
        // Cancel pending rAF if state closed before it fired.
        if let Some(raf) = self.pending_raf.take() {
            cancel_animation_frame(raf);
        }
        // Remove active listener.
        if let Some(listener) = self.active_listener.take() {
            listener.remove();
        }
    }
}

/// Strategy 2: Timestamp comparison (fallback for environments without rAF)
struct TimestampClickOutsideGuard {
    /// timeStamp of the pointer event that triggered the open transition.
    trigger_timestamp: f64,
}

impl TimestampClickOutsideGuard {
    /// Create a new timestamp click-outside guard.
    fn new(trigger_event: &PointerEvent) -> Self {
        Self { trigger_timestamp: trigger_event.time_stamp() }
    }

    /// Whether the click-outside event should close the popover.
    fn should_close(&self, outside_event: &PointerEvent) -> bool {
        // Ignore events with the same or earlier timestamp — they are
        // the originating click still propagating.
        outside_event.time_stamp() > self.trigger_timestamp
    }
}
```

### 1.6 Full Machine Implementation

```rust
/// The machine for the `Popover` component.
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
        // Apply convenience offset/cross_offset aliases into positioning options.
        // Explicit Props.offset / Props.cross_offset override the corresponding
        // PositioningOptions fields when non-zero.
        let mut positioning = props.positioning.clone();
        if props.offset != 0.0 { positioning.offset = props.offset; }
        if props.cross_offset != 0.0 { positioning.cross_axis_offset = props.cross_offset; }
        let locale = env.locale.clone();
        let messages = messages.clone();
        let initial_open = props.open.unwrap_or(props.default_open);
        let initial_state = if initial_open { State::Open } else { State::Closed };
        (initial_state, Context {
            locale,
            open: initial_open,
            modal: props.modal,
            trigger_id: ids.part("trigger"),
            content_id: ids.part("content"),
            title_id: None,
            description_id: None,
            messages,
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        _ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (State::Closed, Event::Open | Event::Toggle) => {
                Some(TransitionPlan::to(State::Open)
                    .apply(|ctx| { ctx.open = true; })
                    .with_effect(PendingEffect::new("click-outside", |ctx, _props, send| {
                        // NOTE: The click-outside listener MUST be attached on the
                        // next animation frame (requestAnimationFrame) or next
                        // microtask, NOT synchronously. If attached synchronously,
                        // the same click event that opened the popover will bubble
                        // to the document-level listener and immediately close it.
                        // Alternative: track the opening event's timeStamp and
                        // ignore outside clicks with the same timeStamp.
                        let cleanup = add_click_outside_listener(&ctx.content_id, move || {
                            send(Event::CloseOnInteractOutside);
                        });
                        cleanup
                    })))
            }
            (State::Open, Event::Close | Event::Toggle) => {
                Some(TransitionPlan::to(State::Closed)
                    .apply(|ctx| { ctx.open = false; }))
            }
            (State::Open, Event::CloseOnEscape) if props.close_on_escape => {
                Some(TransitionPlan::to(State::Closed)
                    .apply(|ctx| { ctx.open = false; }))
            }
            (State::Open, Event::CloseOnInteractOutside) if props.close_on_interact_outside => {
                Some(TransitionPlan::to(State::Closed)
                    .apply(|ctx| { ctx.open = false; }))
            }
            // PositioningUpdate — update cached position data (context-only, no state change)
            (State::Open, Event::PositioningUpdate(result)) => {
                let result = result.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.positioning = Some(result);
                }))
            }
            _ => None,
        }
    }
}
```

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "popover"]
pub enum Part {
    Root,
    Anchor,
    Trigger,
    Positioner,
    Content,
    Arrow,
    Title,
    Description,
    CloseTrigger,
}

/// The API of the `Popover` component.
pub struct Api<'a> {
    /// The state of the popover.
    state: &'a State,
    /// The context of the popover.
    ctx: &'a Context,
    /// The props of the popover.
    props: &'a Props,
    /// The send callback.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Whether the popover is open.
    pub fn is_open(&self) -> bool { *self.state == State::Open }

    /// The attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        attrs
    }

    /// The attributes for the anchor element.
    pub fn anchor_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Anchor.data_attrs();
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
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.is_open() { "true" } else { "false" });
        if self.is_open() {
            attrs.set(HtmlAttr::Aria(AriaAttr::Controls), &self.ctx.content_id);
        }
        attrs
    }

    /// The handler for the trigger click event.
    pub fn on_trigger_click(&self) {
        (self.send)(Event::Toggle);
    }

    /// The attributes for the positioner element.
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if let Some(pos) = &self.ctx.positioning {
            attrs.set_style(CssProperty::Position, "absolute");
            attrs.set_style(CssProperty::Left, format!("{}px", pos.x));
            attrs.set_style(CssProperty::Top, format!("{}px", pos.y));
            attrs.set(HtmlAttr::Data("ars-placement"), pos.actual_placement.as_str());
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
        if self.ctx.modal {
            attrs.set(HtmlAttr::Role, "dialog");
            attrs.set(HtmlAttr::Aria(AriaAttr::Modal), "true");
        } else {
            attrs.set(HtmlAttr::Role, "group");
        }
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        attrs.set(HtmlAttr::TabIndex, "-1");
        if let Some(title_id) = &self.ctx.title_id {
            attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), title_id);
        }
        if let Some(desc_id) = &self.ctx.description_id {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), desc_id);
        }
        attrs
    }

    /// The handler for the content keydown event.
    pub fn on_content_keydown(&self, data: &KeyboardEventData) {
        if data.key == KeyboardKey::Escape {
            (self.send)(Event::CloseOnEscape);
        }
    }

    /// The attributes for the arrow element.
    pub fn arrow_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Arrow.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if let Some(pos) = &self.ctx.positioning {
            if let Some(ax) = pos.arrow_x {
                attrs.set_style(CssProperty::Left, format!("{ax}px"));
            }
            if let Some(ay) = pos.arrow_y {
                attrs.set_style(CssProperty::Top, format!("{ay}px"));
            }
        }
        attrs
    }

    /// The attributes for the title element.
    pub fn title_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Title.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if let Some(title_id) = &self.ctx.title_id {
            attrs.set(HtmlAttr::Id, title_id);
        }
        attrs
    }

    /// The attributes for the description element.
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if let Some(desc_id) = &self.ctx.description_id {
            attrs.set(HtmlAttr::Id, desc_id);
        }
        attrs
    }

    /// The attributes for the close trigger element.
    pub fn close_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CloseTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.dismiss_label)(&self.ctx.locale));
        attrs
    }

    /// The handler for the close trigger click event.
    pub fn on_close_trigger_click(&self) {
        (self.send)(Event::Close);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Anchor => self.anchor_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::Arrow => self.arrow_attrs(),
            Part::Title => self.title_attrs(),
            Part::Description => self.description_attrs(),
            Part::CloseTrigger => self.close_trigger_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Popover
├── Root                 (required)
├── Trigger              (required)
├── Anchor               (optional — alternative positioning reference)
├── Positioner           (required)
│   ├── Arrow            (optional)
│   └── Content          (required)
│       ├── Title        (optional)
│       ├── Description  (optional)
│       └── CloseTrigger (optional)
```

| Part         | Element    | Key Attributes                                                  |
| ------------ | ---------- | --------------------------------------------------------------- |
| Root         | `<div>`    | `data-ars-scope="popover"`, `data-ars-state`                    |
| Anchor       | any        | `data-ars-scope="popover"`, `data-ars-part="anchor"`            |
| Trigger      | `<button>` | `aria-expanded`, `aria-controls`                                |
| Positioner   | `<div>`    | `data-ars-scope="popover"`, `data-ars-part="positioner"`        |
| Content      | `<div>`    | `role="group"` or `role="dialog"`, `tabindex="-1"`              |
| Arrow        | `<div>`    | `data-ars-scope="popover"`, `data-ars-part="arrow"`             |
| Title        | any        | `data-ars-scope="popover"`, `data-ars-part="title"`, `id`       |
| Description  | any        | `data-ars-scope="popover"`, `data-ars-part="description"`, `id` |
| CloseTrigger | `<button>` | `data-ars-scope="popover"`, `data-ars-part="close-trigger"`     |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part    | Property           | Value                                              |
| ------- | ------------------ | -------------------------------------------------- |
| Content | `role`             | `"group"` (non-modal) or `"dialog"` (modal)        |
| Content | `aria-modal`       | `"true"` (modal only)                              |
| Content | `aria-labelledby`  | Title part ID (when title is rendered)             |
| Content | `aria-describedby` | Description part ID (when description is rendered) |
| Content | `tabindex`         | `"-1"` (allows programmatic focus)                 |
| Trigger | `aria-expanded`    | `"true"` / `"false"`                               |
| Trigger | `aria-controls`    | Content part ID (when open)                        |

- No `aria-modal` for non-modal popovers.
- Return focus to trigger on close.
- Tab cycles through interactive content but can leave (non-modal).

### 3.2 Focus Management

When `modal=false` (default for Popover, HoverCard):

- **On open**: focus moves to the first tabbable element inside the popover content. If no tabbable element exists, focus moves to the content container itself (which should have `tabindex="-1"`)
- Focus is NOT trapped — Tab moves to the next element in document order (natural tab flow). After the last tabbable element inside the popover, Tab continues to the next element in the page
- `FocusScope::popover()` preset is used: `contain=false`, `restore_focus=true`
- Clicking outside closes the popover and focus returns to trigger
- Escape closes the popover and restores focus to trigger
- `DismissButton` provides screen reader close mechanism
- Content is NOT rendered with `aria-modal="true"`
- Background is NOT made inert

### 3.3 DismissButton

A visually hidden button placed at the start and/or end of non-modal overlay
content (Popover, HoverCard, Tooltip with interactive content). It allows
screen reader users to dismiss the overlay without relying on Escape key
discovery.

```rust
/// Returns the attributes for the dismiss button.
pub fn dismiss_button_attrs(label: &str) -> AttrMap {
    let mut p = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = dismissable::Part::DismissButton.data_attrs();
    p.set(scope_attr, scope_val);
    p.set(part_attr, part_val);
    p.set(HtmlAttr::Role, "button");
    p.set(HtmlAttr::TabIndex, "0");
    p.set(HtmlAttr::Aria(AriaAttr::Label), label); // Caller provides localized label from Messages struct
    p.set_bool(HtmlAttr::Data("ars-visually-hidden"), true);
    p
}
```

Adapters render this as a `<button>` with visually-hidden styling that calls
the overlay's close handler on click.

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Dismiss button label for screen readers (default: "Dismiss popover")
    pub dismiss_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self { dismiss_label: MessageFn::static_str("Dismiss popover") }
    }
}

impl ComponentMessages for Messages {}
```

## 5. Differences from Dialog

| Feature                | Dialog           | Popover                           |
| ---------------------- | ---------------- | --------------------------------- |
| Modal                  | Yes (by default) | No                                |
| Focus trap             | Yes              | No (Tab moves through, can leave) |
| Backdrop               | Yes              | No                                |
| Anchored to trigger    | No (centered)    | Yes                               |
| Close on click outside | Optional         | Yes (by default)                  |

## 6. Library Parity

> Compared against: Ark UI (`Popover`), Radix UI (`Popover`), React Aria (`Popover`).

### 6.1 Props

| Feature                | ars-ui                      | Ark UI                   | Radix UI             | React Aria                         | Notes                                                 |
| ---------------------- | --------------------------- | ------------------------ | -------------------- | ---------------------------------- | ----------------------------------------------------- |
| Controlled open        | `open`                      | `open`                   | `open`               | `isOpen`                           | All libraries                                         |
| Default open           | `default_open`              | `defaultOpen`            | `defaultOpen`        | `defaultOpen`                      | All libraries                                         |
| Modal mode             | `modal`                     | `modal`                  | `modal`              | `isNonModal` (inverse)             | All libraries                                         |
| Close on Escape        | `close_on_escape`           | `closeOnEscape`          | (onEscapeKeyDown)    | `isKeyboardDismissDisabled`        | All libraries                                         |
| Close on outside click | `close_on_interact_outside` | `closeOnInteractOutside` | (onInteractOutside)  | `shouldCloseOnInteractOutside`     | All libraries                                         |
| Positioning options    | `positioning`               | `positioning`            | (side/align/offset)  | `placement`/`offset`/`crossOffset` | ars-ui unified; Radix/React Aria use individual props |
| Offset                 | `offset`                    | (in positioning)         | `sideOffset`         | `offset`                           | Convenience alias                                     |
| Cross offset           | `cross_offset`              | (in positioning)         | `alignOffset`        | `crossOffset`                      | Convenience alias                                     |
| Same width             | `same_width`                | (in positioning)         | --                   | --                                 | Dropdown-style alignment                              |
| Auto focus             | (implicit)                  | `autoFocus`              | (onOpenAutoFocus)    | (implicit)                         | Ark UI has explicit prop                              |
| Initial focus el       | --                          | `initialFocusEl`         | (onOpenAutoFocus)    | --                                 | Ark UI only                                           |
| Portal                 | `portal`                    | `portalled`              | (Portal part)        | `UNSTABLE_portalContainer`         | All libraries                                         |
| Lazy mount             | `lazy_mount`                | `lazyMount`              | --                   | --                                 | Ark UI parity                                         |
| Unmount on exit        | `unmount_on_exit`           | `unmountOnExit`          | (forceMount inverse) | --                                 | Ark UI parity                                         |
| Open change callback   | `on_open_change`            | `onOpenChange`           | `onOpenChange`       | `onOpenChange`                     | All libraries                                         |
| Should flip            | (in positioning)            | (in positioning)         | `avoidCollisions`    | `shouldFlip`                       | All libraries via positioning engine                  |
| Container padding      | (in positioning)            | (in positioning)         | `collisionPadding`   | `containerPadding`                 | All libraries                                         |
| Max height             | (in positioning)            | --                       | --                   | `maxHeight`                        | React Aria only                                       |
| Arrow padding          | (in positioning)            | (in positioning)         | `arrowPadding`       | `arrowBoundaryOffset`              | All libraries                                         |
| Hide when detached     | (in positioning)            | --                       | `hideWhenDetached`   | --                                 | Radix only                                            |

**Gaps:** None. Positioning features are handled through the unified `PositioningOptions` struct.

### 6.2 Anatomy

| Part         | ars-ui       | Ark UI       | Radix UI | React Aria   | Notes                       |
| ------------ | ------------ | ------------ | -------- | ------------ | --------------------------- |
| Root         | Root         | Root         | Root     | --           | Container                   |
| Trigger      | Trigger      | Trigger      | Trigger  | --           | Open button                 |
| Anchor       | Anchor       | Anchor       | Anchor   | (triggerRef) | Alternative positioning ref |
| Positioner   | Positioner   | Positioner   | --       | --           | Ark UI parity               |
| Content      | Content      | Content      | Content  | Popover      | Main content                |
| Arrow        | Arrow        | Arrow        | Arrow    | OverlayArrow | All libraries               |
| Title        | Title        | Title        | --       | --           | ars-ui/Ark UI               |
| Description  | Description  | Description  | --       | --           | ars-ui/Ark UI               |
| CloseTrigger | CloseTrigger | CloseTrigger | Close    | --           | ars-ui/Ark UI/Radix         |
| Indicator    | --           | Indicator    | --       | --           | Ark UI open-state indicator |

**Gaps:** None. Ark UI's `Indicator` part is purely visual and covered by `data-ars-state` attribute on Root/Trigger.

### 6.3 Events

| Callback             | ars-ui                          | Ark UI                 | Radix UI               | React Aria     | Notes                                       |
| -------------------- | ------------------------------- | ---------------------- | ---------------------- | -------------- | ------------------------------------------- |
| Open change          | `on_open_change`                | `onOpenChange`         | `onOpenChange`         | `onOpenChange` | All libraries                               |
| Escape key           | (via close_on_escape)           | `onEscapeKeyDown`      | `onEscapeKeyDown`      | --             | ars-ui uses boolean; Ark/Radix use callback |
| Outside interaction  | (via close_on_interact_outside) | `onInteractOutside`    | `onInteractOutside`    | --             | ars-ui uses boolean prop                    |
| Focus outside        | --                              | `onFocusOutside`       | `onFocusOutside`       | --             | Subsumed by interact outside                |
| Pointer down outside | --                              | `onPointerDownOutside` | `onPointerDownOutside` | --             | Subsumed by interact outside                |
| Exit complete        | (Presence)                      | `onExitComplete`       | --                     | --             | Handled by Presence composition             |

**Gaps:** None.

### 6.4 Features

| Feature                       | ars-ui         | Ark UI        | Radix UI         | React Aria         |
| ----------------------------- | -------------- | ------------- | ---------------- | ------------------ |
| Non-modal (default)           | Yes            | Yes (default) | Yes (default)    | Yes                |
| Modal mode                    | Yes            | Yes           | Yes              | Yes                |
| Focus management              | Yes            | Yes           | Yes              | Yes                |
| Anchored positioning          | Yes            | Yes           | Yes              | Yes                |
| Arrow                         | Yes            | Yes           | Yes              | Yes                |
| Click-outside close           | Yes            | Yes           | Yes              | Yes                |
| Click-outside race prevention | Yes            | --            | --               | --                 |
| Animation support             | Yes (Presence) | Yes           | Yes (forceMount) | Yes (render props) |
| DismissButton                 | Yes            | --            | --               | Yes (implicit)     |
| Lazy mount                    | Yes            | Yes           | --               | --                 |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity.
- **Divergences:** (1) ars-ui uses a unified `PositioningOptions` struct instead of individual `side`/`align`/`sideOffset` props like Radix; convenience aliases (`offset`, `cross_offset`) provide a simpler API. (2) Dismiss interception uses boolean props (`close_on_escape`, `close_on_interact_outside`) rather than preventable callbacks. (3) Click-outside race prevention is explicitly specified with two strategies (rAF delay and timestamp comparison).
- **Recommended additions:** None.
