---
component: ScrollArea
category: layout
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: [layout-shared-types]
related: []
references:
    ark-ui: ScrollArea
    radix-ui: ScrollArea
---

# ScrollArea

`ScrollArea` wraps a scrollable region and replaces native OS scrollbars with fully styleable custom scrollbars. The viewport still uses native scroll for accessibility and performance; the custom scrollbars are overlaid and synchronised. Supports vertical, horizontal, and both axes; four visibility modes (`Always`, `Auto`, `Hover`, `Scroll`); drag-to-scroll via thumb dragging; page-scroll via track clicks; and RTL support.

ScrollArea MUST preserve native keyboard scrolling (arrow keys, Page Up/Down, Home/End). Custom scrollbar styling is purely visual.

## 1. State Machine

### 1.1 States

```rust
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum State {
    /// No active interaction.
    #[default]
    Idle,
    /// User is hovering the scroll area (relevant in `Hover` mode).
    Hovering,
    /// Viewport is actively scrolling; hide timer is running.
    ScrollActive,
    /// User is dragging a scrollbar thumb.
    ThumbDragging,
}
```

### 1.2 Events

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The viewport reported a scroll event.
    Scroll { x: f64, y: f64 },
    /// The viewport or content size changed.
    Resize {
        viewport_width: f64,
        viewport_height: f64,
        content_width: f64,
        content_height: f64,
    },
    /// Pointer entered the scroll area.
    MouseEnter,
    /// Pointer left the scroll area.
    MouseLeave,
    /// Pointer entered a scrollbar track.
    MouseEnterScrollbar,
    /// Pointer left a scrollbar track.
    MouseLeaveScrollbar,
    /// Thumb drag started.
    ThumbDragStart { pos: f64, axis: Axis },
    /// Thumb drag moved.
    ThumbDragMove { pos: f64 },
    /// Thumb drag ended.
    ThumbDragEnd,
    /// Click on the scrollbar track (page scroll).
    TrackClick { pos: f64, axis: Axis },
    /// Hide-delay timer fired.
    HideTimeout,
}
```

### 1.3 Context

```rust
/// Which scroll orientation is enabled.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Default)]
pub enum ScrollOrientation {
    #[default]
    Vertical,
    Horizontal,
    Both,
}

/// When scrollbars are visible.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Default)]
pub enum ScrollbarVisibility {
    /// Always visible, whether or not content overflows.
    Always,
    /// Shown only when content overflows the viewport.
    #[default]
    Auto,
    /// Appear when the user hovers the scroll area.
    Hover,
    /// Appear while scrolling and fade after `hide_delay`.
    Scroll,
}

/// Runtime context for `ScrollArea`.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    pub scroll_x: f64,
    pub scroll_y: f64,
    pub viewport_width: f64,
    pub viewport_height: f64,
    pub content_width: f64,
    pub content_height: f64,
    pub scrollbar_x_visible: bool,
    pub scrollbar_y_visible: bool,
    pub hovering_scrollbar: bool,
    pub scrollbar_visibility: ScrollbarVisibility,
    pub min_thumb_size: f64,
    pub hide_delay_ms: u32,
    /// Cross-axis scrollbar thickness (px). Used to shorten track_size when
    /// both scrollbars are visible (the CornerSquare occupies this space).
    pub scrollbar_cross_size: f64,
    // Drag state
    pub drag_start_pointer_pos: f64,
    pub drag_start_thumb_pos: f64,
    pub drag_start_scroll_pos: f64,
    pub drag_axis: Option<Axis>,
    pub ids: ComponentIds,
    /// Resolved text direction. Drives `normalize_scroll_left` and vertical
    /// scrollbar placement (left side in RTL).
    pub dir: Direction,
    /// Resolved locale for message formatting.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
}

impl Context {
    pub fn has_overflow_x(&self) -> bool { self.content_width > self.viewport_width }
    pub fn has_overflow_y(&self) -> bool { self.content_height > self.viewport_height }

    pub fn update_visibility(&mut self) {
        match self.scrollbar_visibility {
            ScrollbarVisibility::Always => {
                self.scrollbar_x_visible = true;
                self.scrollbar_y_visible = true;
            }
            ScrollbarVisibility::Auto => {
                self.scrollbar_x_visible = self.has_overflow_x();
                self.scrollbar_y_visible = self.has_overflow_y();
            }
            // Hover and Scroll are managed by state transitions.
            _ => {}
        }
    }
}
```

### 1.4 Props

```rust
/// Detail payload passed to the `on_scroll` callback.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollDetail {
    /// Current scroll offset `(x, y)`.
    pub offset: (f64, f64),
    /// Viewport dimensions `(width, height)`.
    pub viewport_size: (f64, f64),
    /// Content dimensions `(width, height)`.
    pub content_size: (f64, f64),
}

#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the component.
    pub id: String,
    /// Which scroll orientation is enabled. Default: `Vertical`.
    pub orientation: ScrollOrientation,
    /// When scrollbars are visible.
    pub scrollbar_visibility: ScrollbarVisibility,
    /// Minimum thumb size in pixels.
    pub min_thumb_size: Option<f64>,
    /// Delay in milliseconds before scrollbar hides (Scroll mode).
    pub hide_delay_ms: Option<u32>,
    /// Accessible label for the scroll area viewport.
    pub aria_label: Option<String>,
    /// Text/layout direction. Drives RTL scrollbar placement and
    /// `scrollLeft` normalization.
    pub dir: Option<Direction>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            orientation: ScrollOrientation::Vertical,
            scrollbar_visibility: ScrollbarVisibility::Auto,
            min_thumb_size: None,
            hide_delay_ms: None,
            aria_label: None,
            dir: None,
        }
    }
}
```

### 1.5 Thumb Metrics Computation

```rust
/// Compute thumb `(size, position)` for one axis.
///
/// - `viewport_size`: visible extent of the viewport (px)
/// - `content_size`: total scrollable content extent (px)
/// - `scroll_pos`: current scroll offset (px)
/// - `track_size`: length of the scrollbar track (px)
/// - `min_thumb_size`: floor for thumb length (px)
///
/// Returns `(thumb_size, thumb_offset)`.
pub fn compute_thumb_metrics(
    viewport_size: f64,
    content_size: f64,
    scroll_pos: f64,
    track_size: f64,
    min_thumb_size: f64,
) -> (f64, f64) {
    if content_size <= viewport_size {
        return (track_size, 0.0);
    }
    let ratio = viewport_size / content_size;
    let thumb_size = (ratio * track_size).max(min_thumb_size).min(track_size);
    let scrollable_content = content_size - viewport_size;
    let scrollable_track = track_size - thumb_size;
    let thumb_pos = if scrollable_content > 0.0 {
        (scroll_pos / scrollable_content) * scrollable_track
    } else {
        0.0
    };
    (thumb_size, thumb_pos)
}

/// Inverse: given a thumb position, compute the scroll position.
pub fn thumb_pos_to_scroll(
    thumb_pos: f64,
    track_size: f64,
    thumb_size: f64,
    content_size: f64,
    viewport_size: f64,
) -> f64 {
    let scrollable_track = track_size - thumb_size;
    let scrollable_content = content_size - viewport_size;
    if scrollable_track <= 0.0 { return 0.0; }
    (thumb_pos / scrollable_track) * scrollable_content
}
```

### 1.6 Full Machine Implementation

```rust
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let locale = env.locale.clone();
        let messages = messages.clone();
        let mut ctx = Context {
            scroll_x: 0.0, scroll_y: 0.0,
            viewport_width: 0.0, viewport_height: 0.0,
            content_width: 0.0, content_height: 0.0,
            scrollbar_x_visible: false, scrollbar_y_visible: false,
            hovering_scrollbar: false,
            scrollbar_visibility: props.scrollbar_visibility,
            min_thumb_size: props.min_thumb_size.unwrap_or(20.0),
            hide_delay_ms: props.hide_delay_ms.unwrap_or(1200),
            scrollbar_cross_size: 0.0,
            drag_start_pointer_pos: 0.0,
            drag_start_thumb_pos: 0.0,
            drag_start_scroll_pos: 0.0,
            drag_axis: None,
            ids: ComponentIds::from_id(&props.id),
            dir: props.dir.unwrap_or(Direction::Ltr),
            locale,
            messages,
        };
        ctx.update_visibility();
        (State::Idle, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::Resize { viewport_width, viewport_height, content_width, content_height } => {
                let (vw, vh, cw, ch) = (*viewport_width, *viewport_height, *content_width, *content_height);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.viewport_width = vw; ctx.viewport_height = vh;
                    ctx.content_width = cw; ctx.content_height = ch;
                    ctx.update_visibility();
                }))
            }

            Event::Scroll { x, y } => {
                let (sx, sy) = (*x, *y);
                if ctx.scrollbar_visibility == ScrollbarVisibility::Scroll {
                    let delay = ctx.hide_delay_ms;
                    Some(TransitionPlan::to(State::ScrollActive).apply(move |ctx| {
                        ctx.scroll_x = sx; ctx.scroll_y = sy;
                        ctx.scrollbar_x_visible = ctx.has_overflow_x();
                        ctx.scrollbar_y_visible = ctx.has_overflow_y();
                    }).with_named_effect("auto-hide-scrollbar", move |_ctx, _props, send| {
                        let platform = use_platform_effects();
                        let handle = platform.set_timeout(delay, Box::new(move || send(Event::HideTimeout)));
                        let pc = platform.clone();
                        Box::new(move || pc.clear_timeout(handle))
                    }))
                } else {
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.scroll_x = sx; ctx.scroll_y = sy;
                    }))
                }
            }

            Event::MouseEnter => {
                if ctx.scrollbar_visibility == ScrollbarVisibility::Hover {
                    Some(TransitionPlan::to(State::Hovering).apply(|ctx| {
                        ctx.scrollbar_x_visible = ctx.has_overflow_x();
                        ctx.scrollbar_y_visible = ctx.has_overflow_y();
                    }))
                } else { None }
            }

            Event::MouseLeave => {
                if ctx.scrollbar_visibility == ScrollbarVisibility::Hover && !ctx.hovering_scrollbar {
                    Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                        ctx.scrollbar_x_visible = false; ctx.scrollbar_y_visible = false;
                    }))
                } else { None }
            }

            Event::MouseEnterScrollbar => Some(TransitionPlan::context_only(|ctx| { ctx.hovering_scrollbar = true; })),
            Event::MouseLeaveScrollbar => Some(TransitionPlan::context_only(|ctx| { ctx.hovering_scrollbar = false; })),

            Event::HideTimeout => {
                if *state != State::ThumbDragging {
                    Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                        ctx.scrollbar_x_visible = false; ctx.scrollbar_y_visible = false;
                    }))
                } else { None }
            }

            Event::ThumbDragStart { pos, axis } => {
                let (p, a) = (*pos, *axis);
                let scroll_pos = match a { Axis::X => ctx.scroll_x, Axis::Y => ctx.scroll_y };
                let (viewport_size, content_size, track_size) = axis_metrics(ctx, a);
                let min_thumb = ctx.min_thumb_size;
                Some(TransitionPlan::to(State::ThumbDragging).apply(move |ctx| {
                    ctx.drag_start_pointer_pos = p;
                    let (_, current_thumb_pos) = compute_thumb_metrics(
                        viewport_size, content_size, scroll_pos, track_size, min_thumb,
                    );
                    ctx.drag_start_thumb_pos = current_thumb_pos;
                    ctx.drag_start_scroll_pos = scroll_pos;
                    ctx.drag_axis = Some(a);
                }))
            }

            Event::ThumbDragMove { pos } => {
                if *state != State::ThumbDragging { return None; }
                let axis = ctx.drag_axis?;
                let p = *pos;
                let (drag_start_pointer, drag_start_thumb, drag_scroll, min_thumb) =
                    (ctx.drag_start_pointer_pos, ctx.drag_start_thumb_pos, ctx.drag_start_scroll_pos, ctx.min_thumb_size);
                let (viewport_size, content_size, track_size) = axis_metrics(ctx, axis);
                let delta = p - drag_start_pointer;
                let (thumb_size, _) = compute_thumb_metrics(viewport_size, content_size, drag_scroll, track_size, min_thumb);
                let new_thumb_pos = (drag_start_thumb + delta).max(0.0);
                let new_scroll = thumb_pos_to_scroll(new_thumb_pos, track_size, thumb_size, content_size, viewport_size);
                Some(TransitionPlan::context_only(move |ctx| {
                    match axis { Axis::X => ctx.scroll_x = new_scroll, Axis::Y => ctx.scroll_y = new_scroll }
                }))
            }

            Event::ThumbDragEnd => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| { ctx.drag_axis = None; }))
            }

            Event::TrackClick { pos, axis } => {
                let (a, p) = (*axis, *pos);
                let scroll_pos = match a { Axis::X => ctx.scroll_x, Axis::Y => ctx.scroll_y };
                let (viewport_size, content_size, track_size) = axis_metrics(ctx, a);
                let (thumb_size, thumb_pos) = compute_thumb_metrics(viewport_size, content_size, scroll_pos, track_size, ctx.min_thumb_size);
                let new_scroll = if p < thumb_pos {
                    (scroll_pos - viewport_size).max(0.0)
                } else if p > thumb_pos + thumb_size {
                    (scroll_pos + viewport_size).min(content_size - viewport_size)
                } else { scroll_pos };
                Some(TransitionPlan::context_only(move |ctx| {
                    match a { Axis::X => ctx.scroll_x = new_scroll, Axis::Y => ctx.scroll_y = new_scroll }
                }))
            }
        }
    }

    fn connect<'a>(
        state: &'a State, ctx: &'a Context, props: &'a Props, send: &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api { state, ctx, props, send }
    }
}

/// Helper: get (viewport_size, content_size, track_size) for an axis,
/// accounting for the cross-axis scrollbar's CornerSquare gap.
fn axis_metrics(ctx: &Context, axis: Axis) -> (f64, f64, f64) {
    match axis {
        Axis::X => {
            let cross = if ctx.scrollbar_y_visible { ctx.scrollbar_cross_size } else { 0.0 };
            (ctx.viewport_width, ctx.content_width, ctx.viewport_width - cross)
        }
        Axis::Y => {
            let cross = if ctx.scrollbar_x_visible { ctx.scrollbar_cross_size } else { 0.0 };
            (ctx.viewport_height, ctx.content_height, ctx.viewport_height - cross)
        }
    }
}
```

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "scroll-area"]
pub enum Part {
    Root,
    Viewport,
    Content,
    ScrollbarY,
    ThumbY,
    ScrollbarX,
    ThumbX,
    CornerSquare,
}

pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Whether the viewport is scrolled to the top edge.
    pub fn is_at_top(&self) -> bool { self.ctx.scroll_y <= 0.0 }

    /// Whether the viewport is scrolled to the bottom edge.
    pub fn is_at_bottom(&self) -> bool {
        self.ctx.scroll_y >= (self.ctx.content_height - self.ctx.viewport_height).max(0.0)
    }

    /// Whether the viewport is scrolled to the left edge.
    pub fn is_at_left(&self) -> bool { self.ctx.scroll_x <= 0.0 }

    /// Whether the viewport is scrolled to the right edge.
    pub fn is_at_right(&self) -> bool {
        self.ctx.scroll_x >= (self.ctx.content_width - self.ctx.viewport_width).max(0.0)
    }

    /// Current scroll progress as `(x, y)` in the range `0.0..=1.0`.
    pub fn scroll_progress(&self) -> (f64, f64) {
        let px = if self.ctx.content_width > self.ctx.viewport_width {
            self.ctx.scroll_x / (self.ctx.content_width - self.ctx.viewport_width)
        } else { 0.0 };
        let py = if self.ctx.content_height > self.ctx.viewport_height {
            self.ctx.scroll_y / (self.ctx.content_height - self.ctx.viewport_height)
        } else { 0.0 };
        (px.clamp(0.0, 1.0), py.clamp(0.0, 1.0))
    }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Idle => "idle",
            State::Hovering => "hovering",
            State::ScrollActive => "scroll-active",
            State::ThumbDragging => "thumb-dragging",
        });
        attrs.set_bool(HtmlAttr::Data("ars-overflow-x"), self.ctx.has_overflow_x());
        attrs.set_bool(HtmlAttr::Data("ars-overflow-y"), self.ctx.has_overflow_y());
        if self.ctx.dir == Direction::Rtl {
            attrs.set(HtmlAttr::Data("ars-dir"), "rtl");
        }
        attrs
    }

    pub fn viewport_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Viewport.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "region");
        attrs.set(HtmlAttr::TabIndex, "0");
        if let Some(label) = &self.props.aria_label {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label.as_str());
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.viewport_label)(&self.ctx.locale));
        }
        attrs
    }

    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn scrollbar_y_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ScrollbarY.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "none");
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), "vertical");
        attrs.set_bool(HtmlAttr::Data("ars-visible"), self.ctx.scrollbar_y_visible);
        attrs
    }

    pub fn thumb_y_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ThumbY.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "none");
        attrs
    }

    pub fn scrollbar_x_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ScrollbarX.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "none");
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), "horizontal");
        attrs.set_bool(HtmlAttr::Data("ars-visible"), self.ctx.scrollbar_x_visible);
        attrs
    }

    pub fn thumb_x_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ThumbX.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "none");
        attrs
    }

    pub fn corner_square_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CornerSquare.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "none");
        attrs
    }

    pub fn on_viewport_scroll(&self, x: f64, y: f64) { (self.send)(Event::Scroll { x, y }); }
    pub fn on_root_mouseenter(&self) { (self.send)(Event::MouseEnter); }
    pub fn on_root_mouseleave(&self) { (self.send)(Event::MouseLeave); }
    pub fn on_scrollbar_mouseenter(&self) { (self.send)(Event::MouseEnterScrollbar); }
    pub fn on_scrollbar_mouseleave(&self) { (self.send)(Event::MouseLeaveScrollbar); }
    pub fn on_thumb_pointerdown(&self, pos: f64, axis: Axis) { (self.send)(Event::ThumbDragStart { pos, axis }); }
    pub fn on_thumb_pointermove(&self, pos: f64) { (self.send)(Event::ThumbDragMove { pos }); }
    pub fn on_thumb_pointerup(&self) { (self.send)(Event::ThumbDragEnd); }
    pub fn on_track_click(&self, pos: f64, axis: Axis) { (self.send)(Event::TrackClick { pos, axis }); }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Viewport => self.viewport_attrs(),
            Part::Content => self.content_attrs(),
            Part::ScrollbarY => self.scrollbar_y_attrs(),
            Part::ThumbY => self.thumb_y_attrs(),
            Part::ScrollbarX => self.scrollbar_x_attrs(),
            Part::ThumbX => self.thumb_x_attrs(),
            Part::CornerSquare => self.corner_square_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
ScrollArea
├── Root           <div>   data-ars-state data-ars-overflow-x data-ars-overflow-y
│   ├── Viewport   <div>   role="region" tabindex="0" (native scroll)
│   │   └── Content <div>  (inner content wrapper)
│   ├── ScrollbarY <div>   role="none" (vertical track)
│   │   └── ThumbY <div>   role="none" (vertical thumb)
│   ├── ScrollbarX <div>   role="none" (horizontal track)
│   │   └── ThumbX <div>   role="none" (horizontal thumb)
│   └── CornerSquare <div> role="none" (gap filler when both axes)
```

| Part         | Element | Key Attributes                                      |
| ------------ | ------- | --------------------------------------------------- |
| Root         | `<div>` | `data-ars-state`, `data-ars-overflow-x/y`           |
| Viewport     | `<div>` | `role="region"`, `tabindex="0"`, `aria-label`       |
| Content      | `<div>` | Inner content wrapper                               |
| ScrollbarY   | `<div>` | `role="none"`, `data-ars-visible`                   |
| ThumbY       | `<div>` | `role="none"`, sized/positioned by thumb metrics    |
| ScrollbarX   | `<div>` | `role="none"`, `data-ars-visible`                   |
| ThumbX       | `<div>` | `role="none"`, sized/positioned by thumb metrics    |
| CornerSquare | `<div>` | `role="none"`, visible when both scrollbars present |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Element           | Attribute    | Value                                                   |
| ----------------- | ------------ | ------------------------------------------------------- |
| Viewport          | `role`       | `"region"`                                              |
| Viewport          | `aria-label` | Consumer-provided label                                 |
| Viewport          | `tabindex`   | `"0"` (focusable for keyboard scrolling)                |
| Scrollbars/Thumbs | `role`       | `"none"` (decorative; screen readers use native scroll) |

- Custom scrollbar tracks and thumbs are decorative duplicates of the native scroll mechanism. They use `role="none"` (ARIA 1.2) to be excluded from the accessibility tree.
- Keyboard users scroll the viewport using standard browser key behaviours (arrow keys, Page Up/Down, Space).
- No `tabindex` is placed on scrollbar elements.

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    pub viewport_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self { viewport_label: MessageFn::static_str("Scrollable content") }
    }
}

impl ComponentMessages for Messages {}
```

### 4.2 RTL Support

**Vertical scrollbar position:** In `dir="rtl"`, `data-ars-dir="rtl"` is set on Root. CSS targets this to move the vertical scrollbar to the left side:

```css
[data-ars-dir="rtl"] [data-ars-part="scrollbar-y"] {
    right: auto;
    left: 0;
}
```

**Horizontal scroll normalization:** RTL browsers use different `scrollLeft` conventions. The machine normalizes to a 0-to-positive range:

```rust
/// Normalizes `scrollLeft` across browser RTL conventions.
/// - Standard (Chrome, Firefox >= 112): negative values, 0 at left edge
/// - Legacy Firefox (< 112): negative values, 0 at right edge
/// - Safari/WebKit: positive values, 0 at right edge
///
/// Returns normalized value in range [0, scrollWidth - clientWidth].
fn normalize_scroll_left(raw: f64, scroll_width: f64, client_width: f64, is_rtl: bool) -> f64 {
    if !is_rtl {
        return raw;
    }
    // Detect convention by checking sign of scrollLeft at initial position
    // Modern standard: raw <= 0, normalize to positive range
    if raw <= 0.0 {
        raw.abs()
    } else {
        // Safari positive convention: already positive
        scroll_width - client_width - raw
    }
}
```

## 5. Library Parity

> Compared against: Ark UI (`ScrollArea`), Radix UI (`ScrollArea`).

### 5.1 Props

| Feature                   | ars-ui                           | Ark UI                  | Radix UI                | Notes                                            |
| ------------------------- | -------------------------------- | ----------------------- | ----------------------- | ------------------------------------------------ |
| Scrollbar visibility mode | `scrollbar_visibility` (4 modes) | --                      | `type` (4 modes)        | Same semantics, different naming                 |
| Hide delay                | `hide_delay_ms`                  | --                      | `scrollHideDelay`       | Same feature                                     |
| Direction (RTL)           | `dir`                            | --                      | `dir`                   | Same feature                                     |
| CSP nonce                 | --                               | --                      | `nonce`                 | Adapter-level concern in ars-ui; not a core prop |
| Orientation               | `orientation`                    | Scrollbar `orientation` | Scrollbar `orientation` | ars-ui sets at Root; refs set per-scrollbar      |
| Min thumb size            | `min_thumb_size`                 | --                      | --                      | ars-ui addition                                  |
| Accessible label          | `aria_label`                     | --                      | --                      | ars-ui addition                                  |

**Gaps:** None. `nonce` is handled at the adapter layer.

### 5.2 Anatomy

| Part                   | ars-ui         | Ark UI      | Radix UI    | Notes                               |
| ---------------------- | -------------- | ----------- | ----------- | ----------------------------------- |
| Root                   | `Root`         | `Root`      | `Root`      | --                                  |
| Viewport               | `Viewport`     | `Viewport`  | `Viewport`  | --                                  |
| Content                | `Content`      | `Content`   | --          | Radix nests content inside Viewport |
| Scrollbar (vertical)   | `ScrollbarY`   | `Scrollbar` | `Scrollbar` | ars-ui splits per-axis              |
| Scrollbar (horizontal) | `ScrollbarX`   | `Scrollbar` | `Scrollbar` | ars-ui splits per-axis              |
| Thumb (vertical)       | `ThumbY`       | `Thumb`     | `Thumb`     | ars-ui splits per-axis              |
| Thumb (horizontal)     | `ThumbX`       | `Thumb`     | `Thumb`     | ars-ui splits per-axis              |
| Corner                 | `CornerSquare` | `Corner`    | `Corner`    | Same feature                        |

**Gaps:** None.

### 5.3 Events

| Callback        | ars-ui                    | Ark UI | Radix UI | Notes                |
| --------------- | ------------------------- | ------ | -------- | -------------------- |
| Scroll position | `Event::Scroll`           | --     | --       | State machine event  |
| Resize          | `Event::Resize`           | --     | --       | State machine event  |
| Thumb drag      | `ThumbDragStart/Move/End` | --     | --       | State machine events |
| Track click     | `Event::TrackClick`       | --     | --       | State machine event  |

**Gaps:** None. Ark UI and Radix UI handle these internally without exposing callbacks.

### 5.4 Features

| Feature                   | ars-ui                                           | Ark UI                                           | Radix UI |
| ------------------------- | ------------------------------------------------ | ------------------------------------------------ | -------- |
| Custom scrollbar styling  | Yes                                              | Yes                                              | Yes      |
| Four visibility modes     | Yes                                              | --                                               | Yes      |
| Drag-to-scroll (thumb)    | Yes                                              | Yes                                              | Yes      |
| Track click (page scroll) | Yes                                              | Yes                                              | Yes      |
| RTL support               | Yes                                              | --                                               | Yes      |
| Scroll position queries   | `is_at_top/bottom/left/right`, `scroll_progress` | `isAtTop/Bottom/Left/Right`, `getScrollProgress` | --       |
| Scroll-to APIs            | --                                               | `scrollToEdge`, `scrollTo`                       | --       |

**Gaps:** Ark UI exposes `scrollToEdge` and `scrollTo` imperative APIs. These are adapter-level operations (calling `element.scrollTo()`) rather than state machine concerns. Adapters can provide these as utility methods on the framework wrapper without core spec changes.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui splits scrollbars into per-axis parts (`ScrollbarY`/`ScrollbarX`) instead of parameterized `Scrollbar(orientation)`. This is more explicit and avoids runtime orientation checks.
- **Recommended additions:** None.
