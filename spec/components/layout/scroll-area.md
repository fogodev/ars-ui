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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
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
/// Scrollbar axis. `X` is the horizontal scrollbar, `Y` the vertical one.
///
/// Adapters tag pointer geometry with the axis it belongs to so the machine
/// can route drag/track-click intents to the correct scroll offset without any
/// DOM lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// The horizontal scrollbar (drives `scroll_x`).
    X,
    /// The vertical scrollbar (drives `scroll_y`).
    Y,
}

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
    /// Re-sync prop-backed context fields after a props change. Emitted by
    /// `Machine::on_props_changed`.
    SyncProps,
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

impl ScrollOrientation {
    /// Whether the horizontal scrollbar is enabled for this orientation.
    pub const fn allows_x(self) -> bool { matches!(self, Self::Horizontal | Self::Both) }
    /// Whether the vertical scrollbar is enabled for this orientation.
    pub const fn allows_y(self) -> bool { matches!(self, Self::Vertical | Self::Both) }
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
    /// Whether the pointer is over the root scroll area. Tracked alongside
    /// `hovering_scrollbar` so an overlaid scrollbar that outlives the root
    /// hover still hides on its own leave event.
    pub hovering_root: bool,
    /// Which scroll orientation is enabled. Gates which scrollbars may show.
    pub orientation: ScrollOrientation,
    pub scrollbar_visibility: ScrollbarVisibility,
    pub min_thumb_size: f64,
    pub hide_delay: Duration,
    /// Cross-axis scrollbar thickness (px). Used to shorten track_size when
    /// both scrollbars are visible (the CornerSquare occupies this space).
    pub scrollbar_cross_size: f64,
    // Drag state
    pub drag_start_pointer_pos: f64,
    pub drag_start_thumb_pos: f64,
    pub drag_start_scroll_pos: f64,
    pub drag_axis: Option<Axis>,
    pub ids: ComponentIds,
    /// Resolved text direction. Drives `normalize_scroll_left_rtl` and vertical
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

    /// Whether the horizontal scrollbar may be visible: orientation enables it
    /// and (outside `Always`) the content overflows.
    fn can_show_x(&self) -> bool {
        self.orientation.allows_x()
            && (self.scrollbar_visibility == ScrollbarVisibility::Always || self.has_overflow_x())
    }
    fn can_show_y(&self) -> bool {
        self.orientation.allows_y()
            && (self.scrollbar_visibility == ScrollbarVisibility::Always || self.has_overflow_y())
    }

    /// Recompute visibility after a metrics or prop change. `Always`/`Auto`
    /// derive directly from orientation + overflow. For `Hover`/`Scroll`,
    /// visibility tracks whether a hover/scroll session is `active` (state
    /// `Hovering`/`ScrollActive`): when active each enabled+overflowing axis
    /// shows — so a change that newly enables an axis turns it on; when inactive
    /// both hide.
    pub fn update_visibility(&mut self, active: bool) {
        match self.scrollbar_visibility {
            ScrollbarVisibility::Always | ScrollbarVisibility::Auto => {
                self.scrollbar_x_visible = self.can_show_x();
                self.scrollbar_y_visible = self.can_show_y();
            }
            ScrollbarVisibility::Hover | ScrollbarVisibility::Scroll => {
                self.scrollbar_x_visible = active && self.can_show_x();
                self.scrollbar_y_visible = active && self.can_show_y();
            }
        }
    }
}

/// Whether a hover/scroll/drag session is active (scrollbars should show for
/// eligible axes) for the given state. `ThumbDragging` counts: a resize mid-drag
/// must not hide the thumb being dragged.
const fn is_session_active(state: State) -> bool {
    matches!(state, State::Hovering | State::ScrollActive | State::ThumbDragging)
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
    /// Delay before the scrollbar hides (Scroll mode). Default: `1200ms`.
    pub hide_delay: Duration,
    /// Cross-axis scrollbar thickness in pixels. When both scrollbars are
    /// visible, this is subtracted from each track's length so the thumb does
    /// not overlap the `CornerSquare`. Should match the rendered scrollbar
    /// thickness (e.g. the `--ars-scrollbar-size` CSS custom property).
    /// Default: `0.0` (no corner correction).
    pub scrollbar_cross_size: Option<f64>,
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
            hide_delay: Duration::from_millis(1200),
            scrollbar_cross_size: None,
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
/// Typed side-effect intents emitted by the `ScrollArea` machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Start (or restart) the auto-hide timer in `Scroll` visibility mode.
    ///
    /// The adapter owns the timer: it waits `Context::hide_delay` and then
    /// sends `Event::HideTimeout` back to the machine. The agnostic core
    /// never schedules timers itself.
    AutoHide,
}

pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = Effect;
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
            hovering_root: false,
            orientation: props.orientation,
            scrollbar_visibility: props.scrollbar_visibility,
            min_thumb_size: props.min_thumb_size.unwrap_or(20.0),
            hide_delay: props.hide_delay,
            scrollbar_cross_size: props.scrollbar_cross_size.unwrap_or(0.0),
            drag_start_pointer_pos: 0.0,
            drag_start_thumb_pos: 0.0,
            drag_start_scroll_pos: 0.0,
            drag_axis: None,
            ids: ComponentIds::from_id(&props.id),
            dir: props.dir.unwrap_or(Direction::Ltr),
            locale,
            messages,
        };
        ctx.update_visibility(false);
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
                let active = is_session_active(*state);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.viewport_width = vw; ctx.viewport_height = vh;
                    ctx.content_width = cw; ctx.content_height = ch;
                    // Clamp offsets that the shrunk content/viewport made invalid.
                    ctx.scroll_x = ctx.scroll_x.clamp(0.0, (cw - vw).max(0.0));
                    ctx.scroll_y = ctx.scroll_y.clamp(0.0, (ch - vh).max(0.0));
                    ctx.update_visibility(active);
                }))
            }

            Event::Scroll { x, y } => {
                let (sx, sy) = (*x, *y);
                // A scroll mid-drag is the browser echoing the offset the adapter
                // just wrote; record it without leaving `ThumbDragging`.
                if *state == State::ThumbDragging {
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.scroll_x = sx; ctx.scroll_y = sy;
                    }))
                } else if ctx.scrollbar_visibility == ScrollbarVisibility::Scroll {
                    // The agnostic core does not own timers. It records the
                    // `Effect::AutoHide` intent; the adapter starts a
                    // `ctx.hide_delay` timer and sends `Event::HideTimeout`
                    // back when it fires. See §1.6.1.
                    Some(TransitionPlan::to(State::ScrollActive).apply(move |ctx| {
                        ctx.scroll_x = sx; ctx.scroll_y = sy;
                        ctx.scrollbar_x_visible = ctx.can_show_x();
                        ctx.scrollbar_y_visible = ctx.can_show_y();
                    }).with_effect(PendingEffect::named(Effect::AutoHide)))
                } else {
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.scroll_x = sx; ctx.scroll_y = sy;
                    }))
                }
            }

            // The hover scrollbars hide only once the pointer has left BOTH the
            // root and any overlaid scrollbar track, so each leave event checks
            // the other hover flag before hiding.
            Event::MouseEnter => {
                // A captured pointer can re-enter the root mid-drag; record the
                // hover flag but never leave `ThumbDragging`.
                if ctx.scrollbar_visibility == ScrollbarVisibility::Hover
                    && *state != State::ThumbDragging {
                    Some(TransitionPlan::to(State::Hovering).apply(|ctx| {
                        ctx.hovering_root = true;
                        ctx.scrollbar_x_visible = ctx.can_show_x();
                        ctx.scrollbar_y_visible = ctx.can_show_y();
                    }))
                } else {
                    Some(TransitionPlan::context_only(|ctx| { ctx.hovering_root = true; }))
                }
            }

            // A leave never hides while a thumb drag is active: the pointer is
            // captured and routinely leaves the root mid-drag.
            Event::MouseLeave => {
                if ctx.scrollbar_visibility == ScrollbarVisibility::Hover
                    && !ctx.hovering_scrollbar
                    && *state != State::ThumbDragging {
                    Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                        ctx.hovering_root = false;
                        ctx.scrollbar_x_visible = false; ctx.scrollbar_y_visible = false;
                    }))
                } else {
                    Some(TransitionPlan::context_only(|ctx| { ctx.hovering_root = false; }))
                }
            }

            Event::MouseEnterScrollbar => Some(TransitionPlan::context_only(|ctx| { ctx.hovering_scrollbar = true; })),
            Event::MouseLeaveScrollbar => {
                if ctx.scrollbar_visibility == ScrollbarVisibility::Hover
                    && !ctx.hovering_root
                    && *state != State::ThumbDragging {
                    Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                        ctx.hovering_scrollbar = false;
                        ctx.scrollbar_x_visible = false; ctx.scrollbar_y_visible = false;
                    }))
                } else {
                    Some(TransitionPlan::context_only(|ctx| { ctx.hovering_scrollbar = false; }))
                }
            }

            // Honour the hide timer only while still in Scroll mode and not
            // dragging: a timeout queued before a switch to Always/Auto must be
            // ignored (the adapter can cancel future fires, not an already-posted
            // event).
            Event::HideTimeout => {
                if ctx.scrollbar_visibility == ScrollbarVisibility::Scroll
                    && *state != State::ThumbDragging {
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
                })
                // Cancel any running Scroll-mode hide timer so it cannot fire
                // mid-drag; `ThumbDragEnd` starts a fresh one. No-op otherwise.
                .cancel_effect(Effect::AutoHide))
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
                // Clamp to the scrollable track so a drag past either end cannot
                // request a scroll offset beyond the content bounds.
                let max_thumb_pos = (track_size - thumb_size).max(0.0);
                let new_thumb_pos = (drag_start_thumb + delta).clamp(0.0, max_thumb_pos);
                let new_scroll = thumb_pos_to_scroll(new_thumb_pos, track_size, thumb_size, content_size, viewport_size);
                Some(TransitionPlan::context_only(move |ctx| {
                    match axis { Axis::X => ctx.scroll_x = new_scroll, Axis::Y => ctx.scroll_y = new_scroll }
                }))
            }

            Event::ThumbDragEnd => {
                if ctx.scrollbar_visibility == ScrollbarVisibility::Scroll {
                    // Restart the adapter-owned hide timer cancelled at drag start.
                    Some(TransitionPlan::to(State::ScrollActive)
                        .apply(|ctx| { ctx.drag_axis = None; })
                        .with_effect(PendingEffect::named(Effect::AutoHide)))
                } else if ctx.scrollbar_visibility == ScrollbarVisibility::Hover {
                    // The drag may have ended off-root (leave events were
                    // suppressed mid-drag); re-apply the hover rule.
                    let still_hovering = ctx.hovering_root || ctx.hovering_scrollbar;
                    let target = if still_hovering { State::Hovering } else { State::Idle };
                    Some(TransitionPlan::to(target).apply(move |ctx| {
                        ctx.drag_axis = None;
                        ctx.update_visibility(still_hovering);
                    }))
                } else {
                    Some(TransitionPlan::to(State::Idle).apply(|ctx| { ctx.drag_axis = None; }))
                }
            }

            Event::TrackClick { pos, axis } => {
                let (a, p) = (*axis, *pos);
                let scroll_pos = match a { Axis::X => ctx.scroll_x, Axis::Y => ctx.scroll_y };
                let (viewport_size, content_size, track_size) = axis_metrics(ctx, a);
                let (thumb_size, thumb_pos) = compute_thumb_metrics(viewport_size, content_size, scroll_pos, track_size, ctx.min_thumb_size);
                let max_scroll = (content_size - viewport_size).max(0.0);
                let new_scroll = if p < thumb_pos {
                    (scroll_pos - viewport_size).max(0.0)
                } else if p > thumb_pos + thumb_size {
                    (scroll_pos + viewport_size).min(max_scroll)
                } else { scroll_pos };
                Some(TransitionPlan::context_only(move |ctx| {
                    match a { Axis::X => ctx.scroll_x = new_scroll, Axis::Y => ctx.scroll_y = new_scroll }
                }))
            }

            // Re-derive prop-backed context fields after a controlled prop
            // change, then recompute visibility. Emitted by `on_props_changed`.
            Event::SyncProps => {
                let orientation = props.orientation;
                let visibility = props.scrollbar_visibility;
                let min_thumb = props.min_thumb_size.unwrap_or(20.0);
                let hide_delay = props.hide_delay;
                let cross = props.scrollbar_cross_size.unwrap_or(0.0);
                let dir = props.dir.unwrap_or(Direction::Ltr);

                // Leaving the visibility mode the current active state belongs to
                // resets to Idle (else a stuck ScrollActive/Hovering lingers; for
                // Scroll an orphaned AutoHide could later hide the scrollbar). A
                // ThumbDragging session is preserved.
                let leaving_scroll_active =
                    *state == State::ScrollActive && visibility != ScrollbarVisibility::Scroll;
                let leaving_hover =
                    *state == State::Hovering && visibility != ScrollbarVisibility::Hover;
                let reset_state = leaving_scroll_active || leaving_hover;
                // Derive visibility against the resulting state so a newly-enabled
                // axis turns on while a session is still active.
                let active = !reset_state && is_session_active(*state);
                let mut plan = if reset_state {
                    TransitionPlan::to(State::Idle)
                } else {
                    TransitionPlan::new()
                };
                plan = plan.apply(move |ctx| {
                    ctx.orientation = orientation;
                    ctx.scrollbar_visibility = visibility;
                    ctx.min_thumb_size = min_thumb;
                    ctx.hide_delay = hide_delay;
                    ctx.scrollbar_cross_size = cross;
                    ctx.dir = dir;
                    ctx.update_visibility(active);
                });
                if leaving_scroll_active {
                    plan = plan.cancel_effect(Effect::AutoHide);
                }
                Some(plan)
            }
        }
    }

    fn connect<'a>(
        state: &'a State, ctx: &'a Context, props: &'a Props, send: &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api { state, ctx, props, send }
    }

    /// Sync prop-backed context fields when a controlled prop changes so adapter
    /// rerenders are not ignored until remount. `id`, `locale`, and `aria_label`
    /// are read live and need no sync.
    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        let changed = old.orientation != new.orientation
            || old.scrollbar_visibility != new.scrollbar_visibility
            || old.min_thumb_size != new.min_thumb_size
            || old.hide_delay != new.hide_delay
            || old.scrollbar_cross_size != new.scrollbar_cross_size
            || old.dir != new.dir;
        if changed { vec![Event::SyncProps] } else { Vec::new() }
    }
}

/// Helper: get (viewport_size, content_size, track_size) for an axis,
/// accounting for the cross-axis scrollbar's CornerSquare gap. The track length
/// is clamped to zero so a corner gap wider than the viewport (tiny scroll
/// areas) cannot yield a negative track and NaN thumb math.
fn axis_metrics(ctx: &Context, axis: Axis) -> (f64, f64, f64) {
    match axis {
        Axis::X => {
            let cross = if ctx.scrollbar_y_visible { ctx.scrollbar_cross_size } else { 0.0 };
            (ctx.viewport_width, ctx.content_width, (ctx.viewport_width - cross).max(0.0))
        }
        Axis::Y => {
            let cross = if ctx.scrollbar_x_visible { ctx.scrollbar_cross_size } else { 0.0 };
            (ctx.viewport_height, ctx.content_height, (ctx.viewport_height - cross).max(0.0))
        }
    }
}
```

#### 1.6.1 Adapter Contract: Auto-Hide Timer

Timers are a platform concern, so the agnostic machine never calls
`set_timeout`/`clear_timeout` itself. `Effect::AutoHide` is emitted as a bare
`PendingEffect::named(Effect::AutoHide)` **marker** (the same pattern Tooltip
uses for `OpenDelay`/`CloseDelay`): its `run()` is a no-op, so the marker on its
own schedules nothing. **The scroll-area adapter component is responsible for
translating the marker into a real timer** — inspecting the emitted
`pending_effects` for `Effect::AutoHide` rather than relying solely on the generic
`use_machine` cleanup pass, since that pass only runs the (no-op) setup closure.
This split is deliberate: the agnostic core cannot reach a platform from a
transition, and adapter timer wiring is out of scope for the core crate (a
separate adapter task, exactly as for Tooltip).

The adapter must:

1. Start (or restart) a timer for `Context::hide_delay` when it observes a
   pending `Effect::AutoHide`.
2. Send `Event::HideTimeout` back to the machine when the timer fires.
3. Cancel the outstanding timer when it observes `Effect::AutoHide` in the
   `cancel_effects` list (emitted on `ThumbDragStart`) or on any state change.

The machine's `HideTimeout` handler then hides the scrollbars and returns to
`State::Idle` (only while still in `Scroll` mode and not dragging). Because each
new `Scroll` event re-emits the marker, the adapter treats a fresh intent as a
"reset the timer" signal, keeping the scrollbar visible while scrolling
continues.

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
#[derive(Clone, Debug, PartialEq)]
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

**Horizontal scroll normalization:** RTL browsers use different `scrollLeft`
conventions, and the two cannot be told apart from a single sample (both report
`0` at one edge). The adapter therefore detects the browser's convention once and
normalizes through the shared, convention-explicit helper from `ars-collections`
— re-exported from this module — before sending `Event::Scroll`:

```rust,no_check
pub use ars_collections::{normalize_scroll_left_rtl, RtlScrollMode};

// Adapter, on each horizontal scroll event (RTL only):
let normalized_x = normalize_scroll_left_rtl(raw_scroll_left, scroll_width, client_width, mode);
// `mode: RtlScrollMode` is `Negative` (Chrome/Edge/Firefox) or `Positive`
// (Safari), detected once at startup. LTR `scrollLeft` is already `0..max` and
// needs no normalization. The machine always works in the normalized
// inline-start `0..max` range.
```

## 5. Library Parity

> Compared against: Ark UI (`ScrollArea`), Radix UI (`ScrollArea`).

### 5.1 Props

| Feature                   | ars-ui                           | Ark UI                  | Radix UI                | Notes                                            |
| ------------------------- | -------------------------------- | ----------------------- | ----------------------- | ------------------------------------------------ |
| Scrollbar visibility mode | `scrollbar_visibility` (4 modes) | --                      | `type` (4 modes)        | Same semantics, different naming                 |
| Hide delay                | `hide_delay`                     | --                      | `scrollHideDelay`       | Same feature                                     |
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
