---
component: HoverCard
category: overlay
tier: complex
foundation_deps: [architecture, accessibility, interactions]
shared_deps: [z-index-stacking]
related: [tooltip, popover]
references:
  ark-ui: HoverCard
  radix-ui: HoverCard
---

# HoverCard

Like a `Popover` but triggered by hover with a longer delay. **Is interactive** (unlike `Tooltip`).

## 1. State Machine

### 1.1 States

```rust
/// The states of the hover card.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// The hover card is closed.
    Closed,
    /// The hover card is pending open.
    OpenPending,
    /// The hover card is open.
    Open,
    /// The hover card is pending close.
    ClosePending,
}
```

### 1.2 Events

```rust
/// The events of the hover card.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The pointer enters the trigger.
    TriggerPointerEnter,
    /// The pointer leaves the trigger.
    TriggerPointerLeave,
    /// The trigger gains keyboard focus.
    TriggerFocus,
    /// The trigger loses keyboard focus.
    TriggerBlur,
    /// The key down event is triggered.
    TriggerKeyDown(KeyboardKey),
    /// The pointer enters the content.
    ContentPointerEnter,
    /// The pointer leaves the content.
    ContentPointerLeave,
    /// The open timer fires.
    OpenTimerFired,
    /// The close timer fires.
    CloseTimerFired,
    /// The close on escape event.
    CloseOnEscape,
    /// The title element mounts.
    TitleMount,
}
```

### 1.3 Context

```rust
/// The context of the hover card.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Whether the hover card is open.
    pub open: bool,
    /// The delay in milliseconds before the hover card opens.
    /// Default: 700ms (intentional longer delay)
    pub open_delay_ms: u32,
    /// The delay in milliseconds before the hover card closes.
    /// Default: 300ms (so pointer can move into card)
    pub close_delay_ms: u32,
    /// The positioning options for the hover card.
    pub positioning: PositioningOptions,
    /// The component IDs for the hover card.
    pub ids: ComponentIds,
    /// The ID of the trigger element.
    pub trigger_id: String,
    /// The ID of the content element.
    pub content_id: String,
    /// The ID of the title element.
    pub title_id: String,
    /// Whether the hover card has a title element.
    /// Tracks whether a title element has been rendered
    pub has_title: bool,
    /// Whether the pointer is currently over the trigger or content.
    /// True while pointer is over trigger or content
    pub hover_active: bool,
    /// Whether the trigger currently has keyboard focus.
    /// True while trigger has keyboard focus
    pub focus_active: bool,
    /// The current locale for message resolution.
    pub locale: Locale,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}
```

### 1.4 Props

```rust
/// The props of the hover card.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The ID of the hover card.
    pub id: String,
    /// Controlled open state. When `Some`, the consumer controls open/close entirely.
    pub open: Option<bool>,
    /// Whether the hover card is open by default (uncontrolled). Default: false.
    pub default_open: bool,
    /// The delay in milliseconds before the hover card opens. Default: 700ms.
    pub open_delay_ms: u32,
    /// The delay in milliseconds before the hover card closes. Default: 300ms.
    pub close_delay_ms: u32,
    /// Whether the hover card is disabled. Default: false.
    pub disabled: bool,
    /// The positioning options for the hover card.
    pub positioning: PositioningOptions,
    /// Callback invoked when the hover card open state changes.
    pub on_open_change: Option<Callback<bool>>,
    /// When true, hover card content is not mounted until first opened. Default: false.
    pub lazy_mount: bool,
    /// When true, hover card content is removed from the DOM after closing. Default: false.
    pub unmount_on_exit: bool,
    /// Localizable messages for the hover card (see §4 Internationalization).
    pub messages: Option<Messages>,
    /// Locale override. When `None`, resolved via `resolve_locale()`.
    pub locale: Option<Locale>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            open: None,
            default_open: false,
            open_delay_ms: 700,
            close_delay_ms: 300,
            disabled: false,
            positioning: PositioningOptions::default(),
            on_open_change: None,
            lazy_mount: false,
            unmount_on_exit: false,
            messages: None,
            locale: None,
        }
    }
}
```

### 1.5 Safe Area (Hover Bridge)

When the pointer moves from the trigger to the floating content, it must cross a gap (the offset
distance). Without a safe area, leaving the trigger's bounding box starts the close delay, and
the content may dismiss before the user reaches it.

**Implementation:**

A **safe triangle** (or polygon) is computed between the trigger element and the content element.
While the pointer remains inside this polygon, the close delay is suspended — the `HoverCard`
remains open.

```text
 ┌─────────────┐
 │   Trigger   │
 └──────┬───┬──┘
         \   \        ← safe triangle zone
          \   \
     ┌─────┴───┴──────┐
     │    Content     │
     └────────────────┘
```

**Algorithm:**

1. On `TriggerPointerLeave`, compute the convex hull of the trigger rect and the content rect.
   Simplify to a triangle: the pointer's current position, and the two nearest corners of the
   content element.
2. Attach a `pointermove` listener on `document`. On each move:
   - If the pointer is inside the safe polygon: do nothing (keep content open).
   - If the pointer enters the content element: cancel the listener, transition to `Open`.
   - If the pointer exits the safe polygon: remove the listener, start the close delay normally.
3. The safe area listener is cleaned up on close, unmount, or when the pointer enters content.

> **Tooltip**: The same safe-area algorithm applies to interactive `Tooltip`s (`interactive: true`).
> Non-interactive `Tooltip`s do not need a safe area since their content is not a pointer target.

### 1.6 Full Machine Implementation

```rust
/// The machine of the hover card.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let ids = ComponentIds::from_id(&props.id);
        let trigger_id = ids.part("trigger");
        let content_id = ids.part("content");
        let title_id = ids.part("title");
        let initial_open = props.open.unwrap_or(props.default_open);
        let initial_state = if initial_open { State::Open } else { State::Closed };
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        (initial_state, Context {
            open: initial_open,
            open_delay_ms: props.open_delay_ms,
            close_delay_ms: props.close_delay_ms,
            positioning: props.positioning.clone(),
            ids,
            trigger_id,
            content_id,
            title_id,
            has_title: false,
            hover_active: false,
            focus_active: false,
            locale,
            messages,
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if props.disabled { return None; }
        match (state, event) {
            // Pointer-based opening
            (State::Closed, Event::TriggerPointerEnter) => {
                Some(TransitionPlan::to(State::OpenPending)
                    .apply(|ctx| { ctx.hover_active = true; })
                    .with_effect(PendingEffect::new("open-delay", |ctx, _props, send| {
                        let platform = use_platform_effects();
                        let delay = ctx.open_delay_ms;
                        let handle = platform.set_timeout(delay, Box::new(move || send(Event::OpenTimerFired)));
                        let pc = platform.clone();
                        Box::new(move || pc.clear_timeout(handle))
                    })))
            }
            // Keyboard-based opening: Focus starts delay, Enter/Space opens immediately
            (State::Closed, Event::TriggerFocus) => {
                Some(TransitionPlan::to(State::OpenPending)
                    .apply(|ctx| { ctx.focus_active = true; })
                    .with_effect(PendingEffect::new("open-delay", |ctx, _props, send| {
                        let platform = use_platform_effects();
                        let delay = ctx.open_delay_ms;
                        let handle = platform.set_timeout(delay, Box::new(move || send(Event::OpenTimerFired)));
                        let pc = platform.clone();
                        Box::new(move || pc.clear_timeout(handle))
                    })))
            }
            (State::Closed | State::OpenPending, Event::TriggerKeyDown(key))
                if *key == KeyboardKey::Enter || *key == KeyboardKey::Space => {
                Some(TransitionPlan::to(State::Open)
                    .apply(|ctx| { ctx.open = true; }))
            }
            (State::OpenPending, Event::OpenTimerFired) => {
                Some(TransitionPlan::to(State::Open)
                    .apply(|ctx| { ctx.open = true; }))
            }
            // Cancel open on pointer leave — only if focus is not also active
            (State::OpenPending, Event::TriggerPointerLeave) => {
                if ctx.focus_active { return None; } // focus keeps it pending
                Some(TransitionPlan::to(State::Closed)
                    .apply(|ctx| { ctx.hover_active = false; }))
            }
            // Cancel open on focus leave — only if hover is not also active
            (State::OpenPending, Event::TriggerBlur) => {
                if ctx.hover_active { return None; } // hover keeps it pending
                Some(TransitionPlan::to(State::Closed)
                    .apply(|ctx| { ctx.focus_active = false; }))
            }
            // Start close delay on leave
            (State::Open, Event::TriggerPointerLeave | Event::TriggerBlur) => {
                Some(TransitionPlan::to(State::ClosePending)
                    .with_effect(PendingEffect::new("close-delay", |ctx, _props, send| {
                        let platform = use_platform_effects();
                        let delay = ctx.close_delay_ms;
                        let handle = platform.set_timeout(delay, Box::new(move || send(Event::CloseTimerFired)));
                        let pc = platform.clone();
                        Box::new(move || pc.clear_timeout(handle))
                    })))
            }
            // Content pointer leave — start close delay
            (State::Open, Event::ContentPointerLeave) => {
                Some(TransitionPlan::to(State::ClosePending)
                    .with_effect(PendingEffect::new("close-delay", |ctx, _props, send| {
                        let platform = use_platform_effects();
                        let delay = ctx.close_delay_ms;
                        let handle = platform.set_timeout(delay, Box::new(move || (send)(Event::CloseTimerFired)));
                        let pc = platform.clone();
                        Box::new(move || pc.clear_timeout(handle))
                    })))
            }
            // Cancel close when pointer enters content
            (State::ClosePending, Event::ContentPointerEnter | Event::TriggerPointerEnter | Event::TriggerFocus) => {
                Some(TransitionPlan::to(State::Open))
            }
            (State::ClosePending, Event::CloseTimerFired) => {
                Some(TransitionPlan::to(State::Closed)
                    .apply(|ctx| {
                        ctx.open = false;
                        ctx.hover_active = false;
                        ctx.focus_active = false;
                    }))
            }
            // Escape closes immediately
            (State::Open | State::ClosePending, Event::CloseOnEscape) => {
                Some(TransitionPlan::to(State::Closed)
                    .apply(|ctx| {
                        ctx.open = false;
                        ctx.hover_active = false;
                        ctx.focus_active = false;
                    }))
            }
            // Title element mounted — enable aria-labelledby (any state)
            (_, Event::TitleMount) => {
                Some(TransitionPlan::context_only(|ctx| { ctx.has_title = true; }))
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
#[scope = "hover-card"]
pub enum Part {
    Root,
    Trigger,
    Positioner,
    Content,
    Arrow,
    Title,
    DismissButton,
}

/// The API of the hover card.
pub struct Api<'a> {
    /// The state of the hover card.
    state: &'a State,
    /// The context of the hover card.
    ctx: &'a Context,
    /// The props of the hover card.
    props: &'a Props,
    /// The send callback.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Whether the hover card is open.
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

    /// The attributes for the trigger element.
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if self.ctx.open {
            attrs.set(HtmlAttr::Aria(AriaAttr::Controls), self.ctx.ids.part("content"));
        }
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.ctx.open { "true" } else { "false" });
        attrs
    }

    /// The callback for the trigger focus event.
    pub fn on_trigger_focus(&self) { (self.send)(Event::TriggerFocus); }
    /// The callback for the trigger blur event.
    pub fn on_trigger_blur(&self) { (self.send)(Event::TriggerBlur); }
    /// The callback for the trigger keydown event.
    pub fn on_trigger_keydown(&self, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::Enter | KeyboardKey::Space => (self.send)(Event::TriggerKeyDown(data.key)),
            KeyboardKey::Escape => (self.send)(Event::CloseOnEscape),
            _ => {}
        }
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
        attrs.set(HtmlAttr::Role, "dialog");
        attrs.set(HtmlAttr::Data("ars-state"), if self.ctx.open { "open" } else { "closed" });
        // Only reference title_id when a title element is rendered; otherwise
        // aria-labelledby would point at a nonexistent element (dangling ref).
        // Fall back to aria-label from Messages for an accessible name.
        if self.ctx.has_title {
            attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), &self.ctx.title_id);
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.label)(&self.ctx.locale));
        }
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

    /// The attributes for the title element.
    pub fn title_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Title.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.title_id);
        attrs
    }

    /// Call from the adapter when the HoverCard title element mounts.
    pub fn on_title_mount(&self) { (self.send)(Event::TitleMount); }

    /// The attributes for the dismiss button.
    pub fn dismiss_button_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DismissButton.data_attrs();
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
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::Arrow => self.arrow_attrs(),
            Part::Title => self.title_attrs(),
            Part::DismissButton => self.dismiss_button_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
HoverCard
├── Root             (required)
├── Trigger          (required)
├── Positioner       (required)
├── Content          (required — role="dialog")
├── Arrow            (optional)
├── Title            (optional)
└── DismissButton    (optional — visually hidden close for screen readers)
```

| Part          | Element    | Key Attributes                                         |
| ------------- | ---------- | ------------------------------------------------------ |
| Root          | `<div>`    | `data-ars-scope="hover-card"`, `data-ars-state`        |
| Trigger       | any        | `aria-expanded`, `aria-controls` (when open)           |
| Positioner    | `<div>`    | `data-ars-scope="hover-card"`                          |
| Content       | `<div>`    | `role="dialog"`, `aria-labelledby` or `aria-label`     |
| Arrow         | `<div>`    | `data-ars-scope="hover-card"`, `data-ars-part="arrow"` |
| Title         | any        | `id` for `aria-labelledby` wiring                      |
| DismissButton | `<button>` | Visually hidden, screen reader close mechanism         |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part    | Property          | Value                                  |
| ------- | ----------------- | -------------------------------------- |
| Content | `role`            | `"dialog"`                             |
| Content | `aria-labelledby` | Title ID (when title rendered)         |
| Content | `aria-label`      | From Messages (fallback when no title) |
| Trigger | `aria-expanded`   | `"true"` / `"false"`                   |
| Trigger | `aria-controls`   | Content ID (when open)                 |

> **Screen reader behavior:** HoverCard content is supplementary visual enrichment. The content area does NOT receive an ARIA role (`role="dialog"` or `role="group"`) and is NOT announced to screen readers. Only the trigger element itself is accessible — it should have meaningful accessible text. Users who cannot hover (keyboard, screen reader, touch) access the linked content directly via the trigger's `href` or action.

### 3.2 Keyboard Interaction

| Key           | Action                               |
| ------------- | ------------------------------------ |
| Enter / Space | Open the hover card immediately      |
| Escape        | Close the hover card                 |
| Tab           | Can enter card content (interactive) |

### 3.3 Focus Management

- HoverCard content IS interactive — Tab can enter the card
- `FocusScope::popover()` preset: `contain=false`, `restore_focus=true`
- DismissButton provides screen reader close mechanism
- Content is NOT `aria-modal` — background is NOT made inert

## 4. Internationalization

### 4.1 Messages

```rust
/// The messages of the hover card.
#[derive(Clone, Debug)]
pub struct Messages {
    /// The fallback accessible name if no title element. (default: "Additional information")
    pub label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self { label: MessageFn::static_str("Additional information") }
    }
}

impl ComponentMessages for Messages {}
```

## 5. Key Differences from Tooltip

| Feature             | Tooltip               | HoverCard                            |
| ------------------- | --------------------- | ------------------------------------ |
| Interactive content | No                    | Yes                                  |
| Open delay          | Short (300ms)         | Long (700ms)                         |
| Close delay         | Immediate             | Delayed (300ms, pointer can move in) |
| ARIA role           | tooltip               | dialog                               |
| Focus behavior      | Stays on trigger      | Tab can enter card                   |
| Safe area           | Only when interactive | Always                               |

## 6. Variant: ContextualHelp

A small popover triggered by a help icon button, typically placed next to a form field label or section heading. Composes with `Popover` internally.

### 6.1 Additional Props

```rust
/// The visual style of the ContextualHelp trigger icon and popover.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Variant {
    /// Question-mark icon — general guidance or explanation.
    #[default]
    Help,
    /// Info icon — supplementary detail or tips.
    Info,
}

/// The props of the contextual help.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The ID of the contextual help.
    pub id: String,
    /// The variant of the contextual help.
    /// Controls the icon and semantic styling. Emits `data-ars-variant="help"` or `"info"`.
    pub variant: Variant,
}
```

The adapter emits `data-ars-variant` on the root element so CSS can style the trigger icon and popover chrome differently for `Help` vs `Info`:

- `data-ars-variant="help"` → question-mark icon (?)
- `data-ars-variant="info"` → info icon (ⓘ)

### 6.2 Anatomy Additions

```text
ContextualHelp
├── Trigger    (button with icon, aria-label from messages)
├── Popover
│   ├── Heading (optional)
│   ├── Body    (slot for content)
│   └── Footer  (optional, e.g., "Learn more" link)
└── DismissButton  (visually hidden)
```

## 7. Library Parity

> Compared against: Ark UI (`HoverCard`), Radix UI (`HoverCard`).

### 7.1 Props

| Feature              | ars-ui            | Ark UI          | Radix UI                | Notes                                       |
| -------------------- | ----------------- | --------------- | ----------------------- | ------------------------------------------- |
| Controlled open      | `open`            | `open`          | `open`                  | Both libraries                              |
| Default open         | `default_open`    | `defaultOpen`   | `defaultOpen`           | Both libraries                              |
| Open delay           | `open_delay_ms`   | `openDelay`     | `openDelay`             | Both; 700ms default matches Radix           |
| Close delay          | `close_delay_ms`  | `closeDelay`    | `closeDelay`            | Both; 300ms default matches both            |
| Disabled             | `disabled`        | `disabled`      | --                      | Ark UI only                                 |
| Positioning          | `positioning`     | `positioning`   | (side/sideOffset/align) | ars-ui unified; Radix uses individual props |
| Lazy mount           | `lazy_mount`      | `lazyMount`     | --                      | Ark UI parity                               |
| Unmount on exit      | `unmount_on_exit` | `unmountOnExit` | (forceMount inverse)    | Ark UI parity                               |
| Open change callback | `on_open_change`  | `onOpenChange`  | `onOpenChange`          | Both libraries                              |
| Hide when detached   | (in positioning)  | --              | `hideWhenDetached`      | Radix only; handled in PositioningOptions   |
| Collision boundary   | (in positioning)  | --              | `collisionBoundary`     | Radix only; handled in PositioningOptions   |

**Gaps:** None.

### 7.2 Anatomy

| Part          | ars-ui        | Ark UI     | Radix UI | Notes                                    |
| ------------- | ------------- | ---------- | -------- | ---------------------------------------- |
| Root          | Root          | Root       | Root     | Container                                |
| Trigger       | Trigger       | Trigger    | Trigger  | Hover target                             |
| Positioner    | Positioner    | Positioner | --       | Ark UI parity                            |
| Content       | Content       | Content    | Content  | Main content                             |
| Arrow         | Arrow         | Arrow      | Arrow    | Both libraries                           |
| ArrowTip      | --            | ArrowTip   | --       | Ark UI only; decorative inner element    |
| Title         | Title         | --         | --       | ars-ui addition for aria-labelledby      |
| DismissButton | DismissButton | --         | --       | ars-ui addition for screen reader access |

**Gaps:** None. Ark UI's `ArrowTip` is a purely decorative inner element within the arrow, not semantically distinct.

### 7.3 Events

| Callback             | ars-ui           | Ark UI                 | Radix UI       | Notes               |
| -------------------- | ---------------- | ---------------------- | -------------- | ------------------- |
| Open change          | `on_open_change` | `onOpenChange`         | `onOpenChange` | Both libraries      |
| Focus outside        | --               | `onFocusOutside`       | --             | Ark UI only         |
| Interact outside     | --               | `onInteractOutside`    | --             | Ark UI only         |
| Pointer down outside | --               | `onPointerDownOutside` | --             | Ark UI only         |
| Exit complete        | (Presence)       | `onExitComplete`       | --             | Handled by Presence |

**Gaps:** None. Ark UI's outside-interaction callbacks are not needed for HoverCard since it closes on pointer leave, not outside interaction.

### 7.4 Features

| Feature                       | ars-ui         | Ark UI        | Radix UI         |
| ----------------------------- | -------------- | ------------- | ---------------- |
| Hover trigger                 | Yes            | Yes           | Yes              |
| Open/close delay              | Yes            | Yes           | Yes              |
| Safe area (hover bridge)      | Yes            | Yes (via Zag) | --               |
| Interactive content           | Yes            | Yes           | Yes              |
| Keyboard access (Enter/Space) | Yes            | --            | --               |
| Focus-based opening           | Yes            | --            | --               |
| Arrow positioning             | Yes            | Yes           | Yes              |
| Animation support             | Yes (Presence) | Yes           | Yes (forceMount) |
| DismissButton                 | Yes            | --            | --               |
| Title with aria-labelledby    | Yes            | --            | --               |

**Gaps:** None.

### 7.5 Summary

- **Overall:** Full parity.
- **Divergences:** (1) ars-ui adds keyboard accessibility (Enter/Space to open, Escape to close, Tab into content) which reference libraries lack for HoverCard. (2) ars-ui adds a `Title` part for `aria-labelledby` wiring and a `DismissButton` for screen reader users, improving accessibility beyond both references. (3) Safe area (hover bridge) algorithm is explicitly specified for pointer movement from trigger to content.
- **Recommended additions:** None.
