---
component: Marquee
category: data-display
tier: stateful
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
  ark-ui: Marquee
---

# Marquee

A component that displays scrolling content in a continuous loop, commonly used
for announcements, news tickers, or decorative text. Content is duplicated to
create seamless looping. Respects `prefers-reduced-motion` by auto-pausing.

## 1. State Machine

### 1.1 States

```rust
/// States for the Marquee component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// The marquee is currently scrolling.
    Playing,
    /// The marquee is paused.
    Paused,
}
```

### 1.2 Events

```rust
/// Events for the Marquee component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Start or resume scrolling.
    Play,
    /// Pause scrolling.
    Pause,
    /// Pointer entered the root element.
    HoverIn,
    /// Pointer left the root element.
    HoverOut,
    /// Focus moved into the root element.
    FocusIn,
    /// Focus moved out of the root element.
    FocusOut,
    /// One full loop of the content completed.
    LoopComplete,
}
```

### 1.3 Context

```rust
/// Scroll direction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    /// The marquee is scrolling from left to right.
    Left,
    /// The marquee is scrolling from right to left.
    Right,
    /// The marquee is scrolling from top to bottom.
    Up,
    /// The marquee is scrolling from bottom to top.
    Down,
}

/// Context for the Marquee component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Scroll speed in pixels per second.
    pub speed: f64,
    /// Scroll direction.
    pub direction: Direction,
    /// Gap in pixels between the original and duplicated content.
    pub gap: f64,
    /// Whether to pause on pointer hover.
    pub pause_on_hover: bool,
    /// Whether to pause when the component receives focus.
    pub pause_on_focus: bool,
    /// Maximum number of loops. `None` means infinite.
    pub loop_count: Option<usize>,
    /// Whether to automatically duplicate content to fill the viewport.
    pub auto_fill: bool,
    /// Delay in seconds before the animation starts.
    pub delay: f64,
    /// Number of completed loops.
    pub current_loop: usize,
    /// Whether the pause was triggered by a hover event.
    pub paused_by_hover: bool,
    /// Whether the pause was triggered by a focus event.
    pub paused_by_focus: bool,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Resolved locale for message formatting.
    pub locale: Locale,
    /// Resolved messages for the marquee.
    pub messages: Messages,
    /// Component instance IDs.
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
/// Props for the Marquee component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Scroll speed in pixels per second.
    pub speed: f64,
    /// Scroll direction.
    pub direction: Direction,
    /// Gap in pixels between the original and duplicated content.
    pub gap: f64,
    /// Whether to pause on pointer hover.
    pub pause_on_hover: bool,
    /// Whether to pause when the component receives focus.
    pub pause_on_focus: bool,
    /// Maximum number of loops. `None` means infinite.
    pub loop_count: Option<usize>,
    /// Whether to automatically duplicate content to fill the viewport.
    /// When `true`, the adapter measures the viewport and duplicates content
    /// enough times to ensure seamless looping with no visible gap.
    pub auto_fill: bool,
    /// Delay in seconds before the animation starts.
    pub delay: f64,
    /// Whether scrolling starts automatically.
    pub auto_play: bool,
    /// Whether the component is disabled (paused and non-interactive).
    pub disabled: bool,
    /// Optional locale override. When `None`, resolved from the nearest
    /// `ArsProvider` context.
    pub locale: Option<Locale>,
    /// Internationalization messages.
    pub messages: Option<Messages>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            speed: 50.0,
            direction: Direction::Left,
            gap: 40.0,
            pause_on_hover: true,
            pause_on_focus: true,
            loop_count: None,
            auto_fill: false,
            delay: 0.0,
            auto_play: true,
            disabled: false,
            locale: None,
            messages: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, ComponentIds, AttrMap, Bindable};

/// Machine for the Marquee component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        let ids = ComponentIds::from_id(&props.id);
        let initial = if props.auto_play && !props.disabled {
            State::Playing
        } else {
            State::Paused
        };
        (initial, Context {
            speed: props.speed,
            direction: props.direction.clone(),
            gap: props.gap,
            pause_on_hover: props.pause_on_hover,
            pause_on_focus: props.pause_on_focus,
            loop_count: props.loop_count,
            auto_fill: props.auto_fill,
            delay: props.delay,
            current_loop: 0,
            paused_by_hover: false,
            paused_by_focus: false,
            disabled: props.disabled,
            locale,
            messages,
            ids,
        })
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        _props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled {
            return None;
        }

        match (state, event) {
            // Explicit play/pause
            (State::Paused, Event::Play) => {
                Some(TransitionPlan::to(State::Playing).apply(|ctx| {
                    ctx.paused_by_hover = false;
                    ctx.paused_by_focus = false;
                }))
            }
            (State::Playing, Event::Pause) => {
                Some(TransitionPlan::to(State::Paused))
            }

            // Hover-triggered pause
            (State::Playing, Event::HoverIn) if ctx.pause_on_hover => {
                Some(TransitionPlan::to(State::Paused).apply(|ctx| {
                    ctx.paused_by_hover = true;
                }))
            }
            (State::Paused, Event::HoverOut) if ctx.paused_by_hover => {
                Some(TransitionPlan::to(State::Playing).apply(|ctx| {
                    ctx.paused_by_hover = false;
                }))
            }

            // Focus-triggered pause
            (State::Playing, Event::FocusIn) if ctx.pause_on_focus => {
                Some(TransitionPlan::to(State::Paused).apply(|ctx| {
                    ctx.paused_by_focus = true;
                }))
            }
            (State::Paused, Event::FocusOut) if ctx.paused_by_focus => {
                Some(TransitionPlan::to(State::Playing).apply(|ctx| {
                    ctx.paused_by_focus = false;
                }))
            }

            // Loop completion
            (State::Playing, Event::LoopComplete) => {
                let exhausted = ctx.loop_count
                    .map(|max| ctx.current_loop + 1 >= max)
                    .unwrap_or(false);
                if exhausted {
                    Some(TransitionPlan::to(State::Paused).apply(|ctx| {
                        ctx.current_loop += 1;
                    }))
                } else {
                    Some(TransitionPlan::context_only(|ctx| {
                        ctx.current_loop += 1;
                    }))
                }
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

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "marquee"]
pub enum Part {
    Root,
    Content,
    Edge { side: EdgeSide },
    AutoPlayTrigger,
}

/// Which edge of the marquee viewport a gradient overlay is placed on.
#[derive(Clone, Debug, PartialEq)]
pub enum EdgeSide {
    /// The start edge (left in LTR, right in RTL, top for vertical).
    Start,
    /// The end edge (right in LTR, left in RTL, bottom for vertical).
    End,
}

/// API for the Marquee component.
pub struct Api<'a> {
    /// Current state of the marquee.
    state: &'a State,
    /// Current context of the marquee.
    ctx: &'a Context,
    /// Current props of the marquee.
    props: &'a Props,
    /// Send event to the marquee.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Whether the marquee is currently scrolling.
    pub fn is_playing(&self) -> bool {
        *self.state == State::Playing
    }

    /// Whether the marquee is paused.
    pub fn is_paused(&self) -> bool {
        *self.state == State::Paused
    }

    /// Start or resume scrolling.
    pub fn play(&self) {
        (self.send)(Event::Play);
    }

    /// Pause scrolling.
    pub fn pause(&self) {
        (self.send)(Event::Pause);
    }

    /// Root element attributes. The root wraps the scrolling viewport.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "marquee");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.region_label)(&self.ctx.locale));
        let state_str = match self.state {
            State::Playing => "playing",
            State::Paused  => "paused",
        };
        attrs.set(HtmlAttr::Data("ars-state"), state_str);
        // Suppress screen reader announcements while scrolling.
        if self.is_playing() {
            attrs.set(HtmlAttr::Aria(AriaAttr::Live), "off");
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        }
        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        attrs
    }

    /// Content element attributes. The content is duplicated for seamless looping.
    /// CSS custom properties are set for adapter animation.
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        // CSS custom properties for animation control.
        let direction_str = match &self.ctx.direction {
            Direction::Left  => "left",
            Direction::Right => "right",
            Direction::Up    => "up",
            Direction::Down  => "down",
        };
        attrs.set_style(CssProperty::Custom("ars-marquee-speed"), format!("{}px", self.ctx.speed));
        attrs.set_style(CssProperty::Custom("ars-marquee-direction"), direction_str);
        attrs.set_style(CssProperty::Custom("ars-marquee-gap"), format!("{}px", self.ctx.gap));
        if self.ctx.delay > 0.0 {
            attrs.set_style(CssProperty::Custom("ars-marquee-delay"), format!("{}s", self.ctx.delay));
        }
        if self.is_paused() {
            attrs.set_style(CssProperty::Custom("ars-marquee-play-state"), "paused");
        } else {
            attrs.set_style(CssProperty::Custom("ars-marquee-play-state"), "running");
        }
        attrs
    }

    /// Edge gradient overlay attributes. Each edge renders a fade-out gradient
    /// at the start or end of the marquee viewport to visually soften the content
    /// boundary. Purely decorative (`aria-hidden="true"`).
    pub fn edge_attrs(&self, side: &EdgeSide) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Edge { side: side.clone() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs.set(HtmlAttr::Data("ars-side"), match side {
            EdgeSide::Start => "start",
            EdgeSide::End   => "end",
        });
        attrs
    }

    /// Auto-play trigger (play/pause button) attributes.
    /// The label toggles based on current state: shows "pause" when playing,
    /// "play" when paused.
    pub fn auto_play_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::AutoPlayTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        let label = if self.is_playing() {
            (self.ctx.messages.pause_label)(&self.ctx.locale)
        } else {
            (self.ctx.messages.play_label)(&self.ctx.locale)
        };
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        attrs.set(HtmlAttr::Aria(AriaAttr::Pressed), if self.is_playing() { "true" } else { "false" });
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match &part {
            Part::Root => self.root_attrs(),
            Part::Content => self.content_attrs(),
            Part::Edge { side } => self.edge_attrs(side),
            Part::AutoPlayTrigger => self.auto_play_trigger_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Marquee
├── Root               role="marquee", aria-live, aria-label
│   ├── Edge (start)   gradient overlay; aria-hidden="true"
│   ├── Content        duplicated for seamless looping
│   ├── Edge (end)     gradient overlay; aria-hidden="true"
│   └── AutoPlayTrigger  play/pause button
```

| Part            | Selector                                                        | Element    |
| --------------- | --------------------------------------------------------------- | ---------- |
| Root            | `[data-ars-scope="marquee"][data-ars-part="root"]`              | `<div>`    |
| Content         | `[data-ars-scope="marquee"][data-ars-part="content"]`           | `<div>`    |
| Edge            | `[data-ars-scope="marquee"][data-ars-part="edge"]`              | `<div>`    |
| AutoPlayTrigger | `[data-ars-scope="marquee"][data-ars-part="auto-play-trigger"]` | `<button>` |

**4 parts total.**

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part | Role      | Properties                                            |
| ---- | --------- | ----------------------------------------------------- |
| Root | `marquee` | `aria-live`, `aria-disabled`, `aria-label` (optional) |

- **`role="marquee"`**: The root element uses the ARIA `marquee` role to identify
  scrolling content to assistive technologies. Screen readers interpret this as
  auto-updating content and may suppress continuous re-reading.
- **`aria-live`**: Set to `"off"` while playing to prevent screen readers from
  constantly re-reading moving content. Set to `"polite"` when paused so the
  current content is announced.
- **`aria-disabled`**: Present when the component is disabled.
- **Accessible label**: The root element should have an `aria-label` provided via
  `Messages::region_label` (default: "Scrolling content") to give context to
  screen reader users.
- **Pause control**: A play/pause button must always be available so users can
  stop the animation. Labels are provided via `Messages`. The button is a standard
  `<button>` and receives `aria-pressed` reflecting the current state.
- **Pause on hover**: When `pause_on_hover: true` (default), pointer hover pauses
  the animation. This prevents motion-sensitive users from being unable to read the
  content.
- **Pause on focus**: When `pause_on_focus: true` (default), focus within the marquee
  pauses the animation. This ensures keyboard users can interact with any focusable
  content inside the marquee without it scrolling away.
- **`prefers-reduced-motion`**: When `prefers-reduced-motion: reduce` is active, the
  adapter MUST auto-pause the marquee on mount and set
  `--ars-marquee-play-state: paused`. The user can still manually start playback via
  the play button. The adapter detects this via
  `window.matchMedia('(prefers-reduced-motion: reduce)')` and passes the result to
  the machine as an initial `Event::Pause` if active.
- **Keyboard**: The pause/play control is a standard button -- keyboard accessible
  by default. No special key bindings are needed on the marquee itself.

## 4. Internationalization

### 4.1 Messages

```rust
/// Localizable strings for the `Marquee` component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Label for the pause control.
    pub pause_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the play control.
    pub play_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the scrolling content region.
    pub region_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            pause_label: MessageFn::static_str("Pause scrolling"),
            play_label: MessageFn::static_str("Start scrolling"),
            region_label: MessageFn::static_str("Scrolling content"),
        }
    }
}

impl ComponentMessages for Messages {}
```

## 5. Library Parity

> Compared against: Ark UI (`Marquee`).

### 5.1 Props

| Feature                             | ars-ui                                | Ark UI                                 | Notes                                       |
| ----------------------------------- | ------------------------------------- | -------------------------------------- | ------------------------------------------- |
| `speed`                             | `f64` (px/s)                          | `number` (px/s, default 50)            | Equivalent                                  |
| `direction`                         | `Direction` enum (Left/Right/Up/Down) | `side` (`Side`, default `'start'`)     | ars-ui uses explicit direction names        |
| `gap`                               | `f64` (px)                            | `spacing` (`string`, default `'1rem'`) | ars-ui uses numeric px; Ark uses CSS string |
| `pause_on_hover` / `pause_on_focus` | `bool` / `bool`                       | `pauseOnInteraction` (single flag)     | ars-ui splits hover and focus control       |
| `loop_count`                        | `Option<usize>`                       | `loopCount` (`number`, 0=infinite)     | Equivalent                                  |
| `auto_fill`                         | `bool`                                | `autoFill` (`boolean`)                 | Added from Ark UI                           |
| `delay`                             | `f64` (seconds)                       | `delay` (`number`, seconds)            | Added from Ark UI                           |
| `auto_play`                         | `bool`                                | Via `defaultPaused` / `paused`         | Different representation, same behavior     |
| `disabled`                          | `bool`                                | --                                     | ars-ui original                             |
| `reverse`                           | Via `Direction::Right`                | `reverse` (`boolean`)                  | ars-ui uses direction enum instead          |

**Gaps:** None.

### 5.2 Anatomy

| Part            | ars-ui            | Ark UI     | Notes                                      |
| --------------- | ----------------- | ---------- | ------------------------------------------ |
| Root            | `Root`            | `Root`     | --                                         |
| Content         | `Content`         | `Content`  | --                                         |
| Edge            | `Edge { side }`   | `Edge`     | Added from Ark UI                          |
| AutoPlayTrigger | `AutoPlayTrigger` | --         | ars-ui original for accessibility          |
| Viewport        | --                | `Viewport` | ars-ui uses Root as viewport container     |
| Item            | --                | `Item`     | ars-ui treats items as children of Content |

**Gaps:** None.

### 5.3 Events

| Callback           | ars-ui                                               | Ark UI           | Notes                                                               |
| ------------------ | ---------------------------------------------------- | ---------------- | ------------------------------------------------------------------- |
| `on_pause_change`  | State transitions (Playing/Paused)                   | `onPauseChange`  | Adapter derives from state changes                                  |
| `on_loop_complete` | `LoopComplete` event                                 | `onLoopComplete` | Equivalent                                                          |
| `on_complete`      | Handled in LoopComplete transition (exhausted check) | `onComplete`     | Adapter can derive from state transition to Paused after exhaustion |

**Gaps:** None.

### 5.4 Features

| Feature                | ars-ui                  | Ark UI                    |
| ---------------------- | ----------------------- | ------------------------- |
| Speed control          | Yes                     | Yes                       |
| Direction control      | Yes (4 directions)      | Yes (side + reverse)      |
| Gap/spacing            | Yes                     | Yes                       |
| Pause on hover         | Yes                     | Yes                       |
| Pause on focus         | Yes                     | Yes (combined with hover) |
| Loop count             | Yes                     | Yes                       |
| Auto-fill              | Yes                     | Yes                       |
| Animation delay        | Yes                     | Yes                       |
| Play/pause control     | Yes (`AutoPlayTrigger`) | No explicit button part   |
| Edge gradients         | Yes (`Edge`)            | Yes (`Edge`)              |
| prefers-reduced-motion | Yes                     | Yes                       |
| CSS custom properties  | Yes                     | Yes                       |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui splits `pauseOnInteraction` into separate `pause_on_hover` and `pause_on_focus` props for finer control. ars-ui uses an explicit `Direction` enum instead of Ark's `side` + `reverse` combination. ars-ui adds an `AutoPlayTrigger` button part for accessibility that Ark UI does not provide.
- **Recommended additions:** None.
