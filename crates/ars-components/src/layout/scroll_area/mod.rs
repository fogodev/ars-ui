//! ScrollArea layout component.
//!
//! `ScrollArea` wraps a scrollable region and replaces native OS scrollbars
//! with fully styleable custom scrollbars. The viewport keeps native scroll for
//! accessibility and performance; the custom scrollbars are overlaid and
//! synchronised from adapter-supplied metrics.
//!
//! This crate owns the DOM-free math and state: thumb size/position, scrollbar
//! visibility, drag state, orientation, and the rendered attribute surface.
//! Framework adapters own live viewport/content/track/thumb handles, the
//! `ResizeObserver`/scroll-listener wiring, the auto-hide timer, and the actual
//! scroll position reads and writes. The machine never calls DOM APIs or looks
//! up elements by id; it consumes adapter-supplied scroll metrics and pointer
//! geometry through [`Event`].

use alloc::string::String;
use core::{
    fmt::{self, Debug},
    time::Duration,
};

use ars_core::{
    AriaAttr, AttrMap, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Direction, Env,
    HtmlAttr, Locale, MessageFn, TransitionPlan, no_cleanup,
};

/// Scrollbar axis. `X` is the horizontal scrollbar, `Y` the vertical one.
///
/// Adapters tag pointer geometry with the axis it belongs to so the machine can
/// route drag/track-click intents to the correct scroll offset without any
/// DOM lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// The horizontal scrollbar (drives `scroll_x`).
    X,

    /// The vertical scrollbar (drives `scroll_y`).
    Y,
}

/// States of the `ScrollArea`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum State {
    /// No active interaction.
    #[default]
    Idle,

    /// The pointer is hovering the scroll area (relevant in `Hover` mode).
    Hovering,

    /// The viewport is actively scrolling; the adapter's hide timer is running.
    ScrollActive,

    /// The user is dragging a scrollbar thumb.
    ThumbDragging,
}

/// Events sent to the `ScrollArea`.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The viewport reported a scroll event with the new (normalized) offset.
    Scroll {
        /// New horizontal scroll offset (px).
        x: f64,

        /// New vertical scroll offset (px).
        y: f64,
    },

    /// The viewport or content size changed.
    Resize {
        /// Visible viewport width (px).
        viewport_width: f64,

        /// Visible viewport height (px).
        viewport_height: f64,

        /// Total scrollable content width (px).
        content_width: f64,

        /// Total scrollable content height (px).
        content_height: f64,
    },

    /// The pointer entered the scroll area.
    MouseEnter,

    /// The pointer left the scroll area.
    MouseLeave,

    /// The pointer entered a scrollbar track.
    MouseEnterScrollbar,

    /// The pointer left a scrollbar track.
    MouseLeaveScrollbar,

    /// A thumb drag started at pointer position `pos` on `axis`.
    ThumbDragStart {
        /// Pointer position along the dragged axis (px, track-relative).
        pos: f64,

        /// Which scrollbar's thumb is being dragged.
        axis: Axis,
    },

    /// A thumb drag moved to pointer position `pos`.
    ThumbDragMove {
        /// Pointer position along the dragged axis (px, track-relative).
        pos: f64,
    },

    /// A thumb drag ended.
    ThumbDragEnd,

    /// A click on the scrollbar track requesting a page scroll.
    TrackClick {
        /// Click position along the track (px, track-relative).
        pos: f64,

        /// Which scrollbar's track was clicked.
        axis: Axis,
    },

    /// The adapter's hide-delay timer fired (`Scroll` visibility mode).
    HideTimeout,
}

/// Which scroll orientation is enabled.
#[derive(Clone, Debug, Copy, PartialEq, Eq, Default)]
pub enum ScrollOrientation {
    /// Only the vertical scrollbar is enabled.
    #[default]
    Vertical,

    /// Only the horizontal scrollbar is enabled.
    Horizontal,

    /// Both scrollbars are enabled.
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
    /// Current horizontal scroll offset (px).
    pub scroll_x: f64,

    /// Current vertical scroll offset (px).
    pub scroll_y: f64,

    /// Visible viewport width (px).
    pub viewport_width: f64,

    /// Visible viewport height (px).
    pub viewport_height: f64,

    /// Total scrollable content width (px).
    pub content_width: f64,

    /// Total scrollable content height (px).
    pub content_height: f64,

    /// Whether the horizontal scrollbar is currently rendered.
    pub scrollbar_x_visible: bool,

    /// Whether the vertical scrollbar is currently rendered.
    pub scrollbar_y_visible: bool,

    /// Whether the pointer is currently over a scrollbar track.
    pub hovering_scrollbar: bool,

    /// Resolved scrollbar visibility mode.
    pub scrollbar_visibility: ScrollbarVisibility,

    /// Minimum thumb size (px).
    pub min_thumb_size: f64,

    /// Delay before the scrollbar hides in `Scroll` mode. Read by adapters that
    /// own the auto-hide timer.
    pub hide_delay: Duration,

    /// Cross-axis scrollbar thickness (px). Used to shorten `track_size` when
    /// both scrollbars are visible (the `CornerSquare` occupies this space).
    pub scrollbar_cross_size: f64,

    /// Pointer position captured when a thumb drag started (px).
    pub drag_start_pointer_pos: f64,

    /// Thumb position captured when a thumb drag started (px).
    pub drag_start_thumb_pos: f64,

    /// Scroll offset captured when a thumb drag started (px).
    pub drag_start_scroll_pos: f64,

    /// Axis of the active thumb drag, if any.
    pub drag_axis: Option<Axis>,

    /// Component identifiers for ARIA attribute generation.
    pub ids: ComponentIds,

    /// Resolved text direction. Drives [`normalize_scroll_left`] and vertical
    /// scrollbar placement (left side in RTL).
    pub dir: Direction,

    /// Resolved locale for message formatting.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,
}

impl Context {
    /// Whether the content overflows the viewport horizontally.
    #[must_use]
    pub fn has_overflow_x(&self) -> bool {
        self.content_width > self.viewport_width
    }

    /// Whether the content overflows the viewport vertically.
    #[must_use]
    pub fn has_overflow_y(&self) -> bool {
        self.content_height > self.viewport_height
    }

    /// Recompute scrollbar visibility for the `Always` and `Auto` modes.
    ///
    /// `Hover` and `Scroll` modes are driven by state transitions and are left
    /// untouched here.
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

            ScrollbarVisibility::Hover | ScrollbarVisibility::Scroll => {}
        }
    }
}

/// Detail payload an adapter can surface to its `on_scroll` callback.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollDetail {
    /// Current scroll offset `(x, y)`.
    pub offset: (f64, f64),

    /// Viewport dimensions `(width, height)`.
    pub viewport_size: (f64, f64),

    /// Content dimensions `(width, height)`.
    pub content_size: (f64, f64),
}

/// Configuration props for the `ScrollArea` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// The id of the component.
    pub id: String,

    /// Which scroll orientation is enabled. Default: `Vertical`.
    pub orientation: ScrollOrientation,

    /// When scrollbars are visible. Default: `Auto`.
    pub scrollbar_visibility: ScrollbarVisibility,

    /// Minimum thumb size in pixels. Default: `20.0`.
    pub min_thumb_size: Option<f64>,

    /// Delay before the scrollbar hides (`Scroll` mode).
    /// Default: `1200ms`.
    pub hide_delay: Duration,

    /// Cross-axis scrollbar thickness in pixels. When both scrollbars are
    /// visible, this is subtracted from each track's length so the thumb does
    /// not overlap the `CornerSquare`. Should match the rendered scrollbar
    /// thickness (e.g. the `--ars-scrollbar-size` CSS custom property).
    /// Default: `0.0` (no corner correction).
    pub scrollbar_cross_size: Option<f64>,

    /// Accessible label for the scroll-area viewport.
    pub aria_label: Option<String>,

    /// Text/layout direction. Drives RTL scrollbar placement and `scrollLeft`
    /// normalization. Default: `Ltr`.
    pub dir: Option<Direction>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            orientation: ScrollOrientation::Vertical,
            scrollbar_visibility: ScrollbarVisibility::Auto,
            min_thumb_size: None,
            hide_delay: DEFAULT_HIDE_DELAY,
            scrollbar_cross_size: None,
            aria_label: None,
            dir: None,
        }
    }
}

impl Props {
    /// Returns fresh scroll-area props with documented defaults.
    ///
    /// # Examples
    ///
    /// ```
    /// use ars_components::layout::scroll_area::{Machine, Messages, Props};
    /// use ars_core::{Env, HtmlAttr, Service};
    ///
    /// let service = Service::<Machine>::new(
    ///     Props::new().id("log"),
    ///     &Env::default(),
    ///     &Messages::default(),
    /// );
    /// let attrs = service.connect(&|_| {}).root_attrs();
    ///
    /// assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("idle"));
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the component instance ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`orientation`](Self::orientation).
    #[must_use]
    pub const fn orientation(mut self, orientation: ScrollOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Sets [`scrollbar_visibility`](Self::scrollbar_visibility).
    #[must_use]
    pub const fn scrollbar_visibility(mut self, visibility: ScrollbarVisibility) -> Self {
        self.scrollbar_visibility = visibility;
        self
    }

    /// Sets [`min_thumb_size`](Self::min_thumb_size).
    #[must_use]
    pub const fn min_thumb_size(mut self, min_thumb_size: f64) -> Self {
        self.min_thumb_size = Some(min_thumb_size);
        self
    }

    /// Sets [`hide_delay`](Self::hide_delay).
    #[must_use]
    pub const fn hide_delay(mut self, hide_delay: Duration) -> Self {
        self.hide_delay = hide_delay;
        self
    }

    /// Sets [`scrollbar_cross_size`](Self::scrollbar_cross_size).
    #[must_use]
    pub const fn scrollbar_cross_size(mut self, scrollbar_cross_size: f64) -> Self {
        self.scrollbar_cross_size = Some(scrollbar_cross_size);
        self
    }

    /// Sets [`aria_label`](Self::aria_label).
    #[must_use]
    pub fn aria_label(mut self, aria_label: impl Into<String>) -> Self {
        self.aria_label = Some(aria_label.into());
        self
    }

    /// Sets [`dir`](Self::dir).
    #[must_use]
    pub const fn dir(mut self, dir: Direction) -> Self {
        self.dir = Some(dir);
        self
    }
}

/// Localized messages for [`ScrollArea`](self).
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the scrollable viewport when no `aria_label` prop
    /// is supplied.
    pub viewport_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            viewport_label: MessageFn::static_str("Scrollable content"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed side-effect intents emitted by the `ScrollArea` machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Start (or restart) the auto-hide timer in `Scroll` visibility mode.
    ///
    /// The adapter owns the timer: it waits [`Context::hide_delay`] and then
    /// sends [`Event::HideTimeout`] back to the machine. The agnostic core
    /// never schedules timers itself.
    AutoHide,
}

/// Default minimum thumb size when [`Props::min_thumb_size`] is unset (px).
const DEFAULT_MIN_THUMB_SIZE: f64 = 20.0;

/// Default auto-hide delay for the [`Props::hide_delay`] field.
const DEFAULT_HIDE_DELAY: Duration = Duration::from_millis(1200);

/// Compute thumb `(size, position)` for one axis.
///
/// - `viewport_size`: visible extent of the viewport (px)
/// - `content_size`: total scrollable content extent (px)
/// - `scroll_pos`: current scroll offset (px)
/// - `track_size`: length of the scrollbar track (px)
/// - `min_thumb_size`: floor for thumb length (px)
///
/// Returns `(thumb_size, thumb_offset)`. When the content does not overflow the
/// viewport the thumb fills the track and sits at the origin.
#[must_use]
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

/// Inverse of [`compute_thumb_metrics`]: given a thumb position, compute the
/// scroll offset it corresponds to.
#[must_use]
pub fn thumb_pos_to_scroll(
    thumb_pos: f64,
    track_size: f64,
    thumb_size: f64,
    content_size: f64,
    viewport_size: f64,
) -> f64 {
    let scrollable_track = track_size - thumb_size;
    let scrollable_content = content_size - viewport_size;

    if scrollable_track <= 0.0 {
        return 0.0;
    }

    (thumb_pos / scrollable_track) * scrollable_content
}

/// Normalizes a raw `scrollLeft` value across browser RTL conventions to a
/// 0-to-positive range `[0, scroll_width - client_width]`.
///
/// Adapters read the live `scrollLeft` and call this before sending
/// [`Event::Scroll`] so the machine always works in a single convention:
///
/// - Standard (Chrome, Firefox >= 112): negative values, `0` at the left edge.
/// - Legacy Firefox (< 112): negative values, `0` at the right edge (handled
///   identically to the standard negative convention).
/// - Safari/WebKit: positive values, `0` at the right edge.
///
/// In LTR the raw value is returned unchanged.
#[must_use]
pub fn normalize_scroll_left(raw: f64, scroll_width: f64, client_width: f64, is_rtl: bool) -> f64 {
    if !is_rtl {
        return raw;
    }

    // Modern standard: raw <= 0, normalize to a positive range.
    if raw <= 0.0 {
        raw.abs()
    } else {
        // Safari positive convention: already positive, mirror it.
        scroll_width - client_width - raw
    }
}

/// Returns `(viewport_size, content_size, track_size)` for an axis, accounting
/// for the cross-axis scrollbar's `CornerSquare` gap.
fn axis_metrics(ctx: &Context, axis: Axis) -> (f64, f64, f64) {
    match axis {
        Axis::X => {
            let cross = if ctx.scrollbar_y_visible {
                ctx.scrollbar_cross_size
            } else {
                0.0
            };

            (
                ctx.viewport_width,
                ctx.content_width,
                ctx.viewport_width - cross,
            )
        }

        Axis::Y => {
            let cross = if ctx.scrollbar_x_visible {
                ctx.scrollbar_cross_size
            } else {
                0.0
            };

            (
                ctx.viewport_height,
                ctx.content_height,
                ctx.viewport_height - cross,
            )
        }
    }
}

/// The machine for the `ScrollArea` component.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = Effect;
    type Api<'a> = Api<'a>;

    fn init(
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        let mut ctx = Context {
            scroll_x: 0.0,
            scroll_y: 0.0,
            viewport_width: 0.0,
            viewport_height: 0.0,
            content_width: 0.0,
            content_height: 0.0,
            scrollbar_x_visible: false,
            scrollbar_y_visible: false,
            hovering_scrollbar: false,
            scrollbar_visibility: props.scrollbar_visibility,
            min_thumb_size: props.min_thumb_size.unwrap_or(DEFAULT_MIN_THUMB_SIZE),
            hide_delay: props.hide_delay,
            scrollbar_cross_size: props.scrollbar_cross_size.unwrap_or(0.0),
            drag_start_pointer_pos: 0.0,
            drag_start_thumb_pos: 0.0,
            drag_start_scroll_pos: 0.0,
            drag_axis: None,
            ids: ComponentIds::from_id(&props.id),
            dir: props.dir.unwrap_or(Direction::Ltr),
            locale: env.locale.clone(),
            messages: messages.clone(),
        };

        ctx.update_visibility();

        (State::Idle, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::Resize {
                viewport_width,
                viewport_height,
                content_width,
                content_height,
            } => {
                let (vw, vh, cw, ch) = (
                    *viewport_width,
                    *viewport_height,
                    *content_width,
                    *content_height,
                );

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.viewport_width = vw;
                    ctx.viewport_height = vh;
                    ctx.content_width = cw;
                    ctx.content_height = ch;
                    ctx.update_visibility();
                }))
            }

            Event::Scroll { x, y } => {
                let (sx, sy) = (*x, *y);

                if ctx.scrollbar_visibility == ScrollbarVisibility::Scroll {
                    Some(
                        TransitionPlan::to(State::ScrollActive)
                            .apply(move |ctx: &mut Context| {
                                ctx.scroll_x = sx;
                                ctx.scroll_y = sy;
                                ctx.scrollbar_x_visible = ctx.has_overflow_x();
                                ctx.scrollbar_y_visible = ctx.has_overflow_y();
                            })
                            .with_named_effect(Effect::AutoHide, |_ctx, _props, _send| {
                                no_cleanup()
                            }),
                    )
                } else {
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.scroll_x = sx;
                        ctx.scroll_y = sy;
                    }))
                }
            }

            Event::MouseEnter => {
                if ctx.scrollbar_visibility == ScrollbarVisibility::Hover {
                    Some(
                        TransitionPlan::to(State::Hovering).apply(|ctx: &mut Context| {
                            ctx.scrollbar_x_visible = ctx.has_overflow_x();
                            ctx.scrollbar_y_visible = ctx.has_overflow_y();
                        }),
                    )
                } else {
                    None
                }
            }

            Event::MouseLeave => {
                if ctx.scrollbar_visibility == ScrollbarVisibility::Hover && !ctx.hovering_scrollbar
                {
                    Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                        ctx.scrollbar_x_visible = false;
                        ctx.scrollbar_y_visible = false;
                    }))
                } else {
                    None
                }
            }

            Event::MouseEnterScrollbar => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.hovering_scrollbar = true;
                }))
            }

            Event::MouseLeaveScrollbar => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.hovering_scrollbar = false;
                }))
            }

            Event::HideTimeout => {
                // The adapter-owned hide timer must not collapse an in-progress
                // thumb drag.
                if *state == State::ThumbDragging {
                    None
                } else {
                    Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                        ctx.scrollbar_x_visible = false;
                        ctx.scrollbar_y_visible = false;
                    }))
                }
            }

            Event::ThumbDragStart { pos, axis } => {
                let (pointer, axis) = (*pos, *axis);

                let scroll_pos = match axis {
                    Axis::X => ctx.scroll_x,
                    Axis::Y => ctx.scroll_y,
                };

                let (viewport_size, content_size, track_size) = axis_metrics(ctx, axis);

                let min_thumb = ctx.min_thumb_size;

                Some(
                    TransitionPlan::to(State::ThumbDragging).apply(move |ctx: &mut Context| {
                        ctx.drag_start_pointer_pos = pointer;

                        let (_, current_thumb_pos) = compute_thumb_metrics(
                            viewport_size,
                            content_size,
                            scroll_pos,
                            track_size,
                            min_thumb,
                        );

                        ctx.drag_start_thumb_pos = current_thumb_pos;
                        ctx.drag_start_scroll_pos = scroll_pos;
                        ctx.drag_axis = Some(axis);
                    }),
                )
            }

            Event::ThumbDragMove { pos } => {
                if *state != State::ThumbDragging {
                    return None;
                }

                let axis = ctx.drag_axis?;
                let pointer = *pos;

                let (drag_start_pointer, drag_start_thumb, drag_scroll, min_thumb) = (
                    ctx.drag_start_pointer_pos,
                    ctx.drag_start_thumb_pos,
                    ctx.drag_start_scroll_pos,
                    ctx.min_thumb_size,
                );

                let (viewport_size, content_size, track_size) = axis_metrics(ctx, axis);

                let delta = pointer - drag_start_pointer;

                let (thumb_size, _) = compute_thumb_metrics(
                    viewport_size,
                    content_size,
                    drag_scroll,
                    track_size,
                    min_thumb,
                );

                let new_thumb_pos = (drag_start_thumb + delta).max(0.0);
                let new_scroll = thumb_pos_to_scroll(
                    new_thumb_pos,
                    track_size,
                    thumb_size,
                    content_size,
                    viewport_size,
                );

                Some(TransitionPlan::context_only(
                    move |ctx: &mut Context| match axis {
                        Axis::X => ctx.scroll_x = new_scroll,
                        Axis::Y => ctx.scroll_y = new_scroll,
                    },
                ))
            }

            Event::ThumbDragEnd => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                    ctx.drag_axis = None;
                }))
            }

            Event::TrackClick { pos, axis } => {
                let (axis, click) = (*axis, *pos);

                let scroll_pos = match axis {
                    Axis::X => ctx.scroll_x,
                    Axis::Y => ctx.scroll_y,
                };

                let (viewport_size, content_size, track_size) = axis_metrics(ctx, axis);

                let (thumb_size, thumb_pos) = compute_thumb_metrics(
                    viewport_size,
                    content_size,
                    scroll_pos,
                    track_size,
                    ctx.min_thumb_size,
                );

                let new_scroll = if click < thumb_pos {
                    (scroll_pos - viewport_size).max(0.0)
                } else if click > thumb_pos + thumb_size {
                    (scroll_pos + viewport_size).min(content_size - viewport_size)
                } else {
                    scroll_pos
                };

                Some(TransitionPlan::context_only(
                    move |ctx: &mut Context| match axis {
                        Axis::X => ctx.scroll_x = new_scroll,
                        Axis::Y => ctx.scroll_y = new_scroll,
                    },
                ))
            }
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api {
            state,
            ctx,
            props,
            send,
        }
    }
}

/// Structural parts exposed by the `ScrollArea` connect API.
#[derive(ComponentPart)]
#[scope = "scroll-area"]
pub enum Part {
    /// The root scroll-area container.
    Root,

    /// The natively-scrolling viewport.
    Viewport,

    /// The inner content wrapper.
    Content,

    /// The vertical scrollbar track.
    ScrollbarY,

    /// The vertical scrollbar thumb.
    ThumbY,

    /// The horizontal scrollbar track.
    ScrollbarX,

    /// The horizontal scrollbar thumb.
    ThumbX,

    /// The gap filler shown when both scrollbars are present.
    CornerSquare,
}

/// Connected `ScrollArea` API.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish()
    }
}

impl Api<'_> {
    /// Whether the viewport is scrolled to the top edge.
    #[must_use]
    pub fn is_at_top(&self) -> bool {
        self.ctx.scroll_y <= 0.0
    }

    /// Whether the viewport is scrolled to the bottom edge.
    #[must_use]
    pub fn is_at_bottom(&self) -> bool {
        self.ctx.scroll_y >= (self.ctx.content_height - self.ctx.viewport_height).max(0.0)
    }

    /// Whether the viewport is scrolled to the left edge.
    #[must_use]
    pub fn is_at_left(&self) -> bool {
        self.ctx.scroll_x <= 0.0
    }

    /// Whether the viewport is scrolled to the right edge.
    #[must_use]
    pub fn is_at_right(&self) -> bool {
        self.ctx.scroll_x >= (self.ctx.content_width - self.ctx.viewport_width).max(0.0)
    }

    /// Current scroll progress as `(x, y)` in the range `0.0..=1.0`.
    #[must_use]
    pub fn scroll_progress(&self) -> (f64, f64) {
        let px = if self.ctx.content_width > self.ctx.viewport_width {
            self.ctx.scroll_x / (self.ctx.content_width - self.ctx.viewport_width)
        } else {
            0.0
        };

        let py = if self.ctx.content_height > self.ctx.viewport_height {
            self.ctx.scroll_y / (self.ctx.content_height - self.ctx.viewport_height)
        } else {
            0.0
        };

        (px.clamp(0.0, 1.0), py.clamp(0.0, 1.0))
    }

    /// Attributes for the root container element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Data("ars-state"),
                match self.state {
                    State::Idle => "idle",
                    State::Hovering => "hovering",
                    State::ScrollActive => "scroll-active",
                    State::ThumbDragging => "thumb-dragging",
                },
            )
            .set_bool(HtmlAttr::Data("ars-overflow-x"), self.ctx.has_overflow_x())
            .set_bool(HtmlAttr::Data("ars-overflow-y"), self.ctx.has_overflow_y());

        if self.ctx.dir == Direction::Rtl {
            attrs.set(HtmlAttr::Data("ars-dir"), "rtl");
        }

        attrs
    }

    /// Attributes for the natively-scrolling viewport element.
    #[must_use]
    pub fn viewport_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Viewport.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "region")
            .set(HtmlAttr::TabIndex, "0");

        if let Some(label) = &self.props.aria_label {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label.as_str());
        } else {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.viewport_label)(&self.ctx.locale),
            );
        }

        attrs
    }

    /// Attributes for the inner content wrapper.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Attributes for the vertical scrollbar track.
    #[must_use]
    pub fn scrollbar_y_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ScrollbarY.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "none")
            .set(HtmlAttr::Aria(AriaAttr::Orientation), "vertical")
            .set_bool(HtmlAttr::Data("ars-visible"), self.ctx.scrollbar_y_visible);

        attrs
    }

    /// Attributes for the vertical scrollbar thumb.
    #[must_use]
    pub fn thumb_y_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ThumbY.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "none");

        attrs
    }

    /// Attributes for the horizontal scrollbar track.
    #[must_use]
    pub fn scrollbar_x_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ScrollbarX.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "none")
            .set(HtmlAttr::Aria(AriaAttr::Orientation), "horizontal")
            .set_bool(HtmlAttr::Data("ars-visible"), self.ctx.scrollbar_x_visible);

        attrs
    }

    /// Attributes for the horizontal scrollbar thumb.
    #[must_use]
    pub fn thumb_x_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ThumbX.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "none");

        attrs
    }

    /// Attributes for the corner gap shown when both scrollbars are present.
    #[must_use]
    pub fn corner_square_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CornerSquare.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "none");

        attrs
    }

    /// Adapter handler: the viewport reported a new scroll offset.
    pub fn on_viewport_scroll(&self, x: f64, y: f64) {
        (self.send)(Event::Scroll { x, y });
    }

    /// Adapter handler: the pointer entered the root.
    pub fn on_root_mouseenter(&self) {
        (self.send)(Event::MouseEnter);
    }

    /// Adapter handler: the pointer left the root.
    pub fn on_root_mouseleave(&self) {
        (self.send)(Event::MouseLeave);
    }

    /// Adapter handler: the pointer entered a scrollbar track.
    pub fn on_scrollbar_mouseenter(&self) {
        (self.send)(Event::MouseEnterScrollbar);
    }

    /// Adapter handler: the pointer left a scrollbar track.
    pub fn on_scrollbar_mouseleave(&self) {
        (self.send)(Event::MouseLeaveScrollbar);
    }

    /// Adapter handler: a thumb drag started.
    pub fn on_thumb_pointerdown(&self, pos: f64, axis: Axis) {
        (self.send)(Event::ThumbDragStart { pos, axis });
    }

    /// Adapter handler: a thumb drag moved.
    pub fn on_thumb_pointermove(&self, pos: f64) {
        (self.send)(Event::ThumbDragMove { pos });
    }

    /// Adapter handler: a thumb drag ended.
    pub fn on_thumb_pointerup(&self) {
        (self.send)(Event::ThumbDragEnd);
    }

    /// Adapter handler: a scrollbar track was clicked.
    pub fn on_track_click(&self, pos: f64, axis: Axis) {
        (self.send)(Event::TrackClick { pos, axis });
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
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

#[cfg(test)]
mod tests {
    use alloc::{format, string::String, vec::Vec};
    use core::cell::RefCell;

    use ars_core::{AttrMap, Env, HtmlAttr, Locale, MessageFn, Service};
    use insta::assert_snapshot;

    use super::*;

    fn props() -> Props {
        Props::new().id("scroll")
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    /// Asserts two scroll offsets are equal within floating-point tolerance.
    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}",
        );
    }

    /// Renders an `AttrMap` as a deterministic, sorted `attr=value` block for
    /// snapshotting.
    fn snapshot_attrs(attrs: &AttrMap) -> String {
        let mut entries = attrs.iter().collect::<Vec<_>>();

        entries.sort_by_key(|(attr, _)| attr.to_string());

        entries
            .into_iter()
            .map(|(attr, value)| format!("{}={}", attr, value.as_str().unwrap_or("<reactive>")))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Drives the machine to known viewport/content metrics.
    fn resize(
        service: &mut Service<Machine>,
        viewport_width: f64,
        viewport_height: f64,
        content_width: f64,
        content_height: f64,
    ) {
        drop(service.send(Event::Resize {
            viewport_width,
            viewport_height,
            content_width,
            content_height,
        }));
    }

    // --- thumb metrics math -------------------------------------------------

    #[test]
    fn thumb_size_is_proportional_to_viewport_content_ratio() {
        // viewport is 1/4 of content -> thumb is 1/4 of the track.
        assert_eq!(
            compute_thumb_metrics(100.0, 400.0, 0.0, 100.0, 20.0),
            (25.0, 0.0)
        );
    }

    #[test]
    fn thumb_size_is_floored_at_min_thumb_size() {
        // ratio 0.05 * 100 = 5, floored to the 20px minimum.
        assert_eq!(
            compute_thumb_metrics(100.0, 2000.0, 0.0, 100.0, 20.0).0,
            20.0
        );
    }

    #[test]
    fn thumb_fills_track_when_content_fits() {
        assert_eq!(
            compute_thumb_metrics(100.0, 100.0, 0.0, 100.0, 20.0),
            (100.0, 0.0)
        );
        assert_eq!(
            compute_thumb_metrics(100.0, 50.0, 0.0, 100.0, 20.0),
            (100.0, 0.0)
        );
    }

    #[test]
    fn thumb_position_tracks_scroll_offset() {
        // scroll halfway through 300px of scrollable content -> halfway down
        // the 75px of scrollable track.
        assert_eq!(
            compute_thumb_metrics(100.0, 400.0, 150.0, 100.0, 20.0),
            (25.0, 37.5)
        );
    }

    #[test]
    fn thumb_pos_to_scroll_is_inverse_of_compute() {
        let (thumb_size, thumb_pos) = compute_thumb_metrics(100.0, 400.0, 150.0, 100.0, 20.0);

        let scroll = thumb_pos_to_scroll(thumb_pos, 100.0, thumb_size, 400.0, 100.0);

        assert_eq!(scroll, 150.0);
    }

    #[test]
    fn thumb_pos_to_scroll_guards_against_zero_scrollable_track() {
        assert_eq!(thumb_pos_to_scroll(10.0, 100.0, 100.0, 400.0, 100.0), 0.0);
    }

    // --- init / defaults ----------------------------------------------------

    #[test]
    fn init_applies_documented_defaults() {
        let service = service(props());

        let ctx = service.context();

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(ctx.min_thumb_size, DEFAULT_MIN_THUMB_SIZE);
        assert_eq!(ctx.hide_delay, DEFAULT_HIDE_DELAY);
        assert_eq!(ctx.dir, Direction::Ltr);
        assert!(!ctx.scrollbar_x_visible);
        assert!(!ctx.scrollbar_y_visible);
    }

    #[test]
    fn init_honours_prop_overrides() {
        let service = service(
            props()
                .min_thumb_size(40.0)
                .hide_delay(Duration::from_millis(500))
                .dir(Direction::Rtl)
                .scrollbar_visibility(ScrollbarVisibility::Always),
        );

        let ctx = service.context();

        assert_eq!(ctx.min_thumb_size, 40.0);
        assert_eq!(ctx.hide_delay, Duration::from_millis(500));
        assert_eq!(ctx.dir, Direction::Rtl);
        // `Always` mode shows both scrollbars immediately, even before resize.
        assert!(ctx.scrollbar_x_visible);
        assert!(ctx.scrollbar_y_visible);
    }

    // --- visibility modes ---------------------------------------------------

    #[test]
    fn auto_mode_shows_scrollbars_only_for_overflowing_axes() {
        let mut service = service(props());

        resize(&mut service, 100.0, 100.0, 100.0, 400.0);

        let ctx = service.context();

        assert!(ctx.has_overflow_y());
        assert!(!ctx.has_overflow_x());
        assert!(ctx.scrollbar_y_visible);
        assert!(!ctx.scrollbar_x_visible);
    }

    #[test]
    fn hover_mode_toggles_visibility_on_enter_and_leave() {
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Hover));

        resize(&mut service, 100.0, 100.0, 400.0, 400.0);

        // Auto/Always logic does not pre-show in Hover mode.
        assert!(!service.context().scrollbar_y_visible);

        let entered = service.send(Event::MouseEnter);

        assert!(entered.state_changed);
        assert_eq!(service.state(), &State::Hovering);
        assert!(service.context().scrollbar_x_visible);
        assert!(service.context().scrollbar_y_visible);

        let left = service.send(Event::MouseLeave);

        assert!(left.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().scrollbar_y_visible);
    }

    #[test]
    fn hover_mode_keeps_scrollbars_while_pointer_is_over_track() {
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Hover));

        resize(&mut service, 100.0, 100.0, 400.0, 400.0);

        drop(service.send(Event::MouseEnter));
        drop(service.send(Event::MouseEnterScrollbar));

        assert!(service.context().hovering_scrollbar);

        // Leaving the root while still over the scrollbar must not hide it.
        let left = service.send(Event::MouseLeave);

        assert!(!left.state_changed);
        assert_eq!(service.state(), &State::Hovering);
        assert!(service.context().scrollbar_y_visible);

        drop(service.send(Event::MouseLeaveScrollbar));

        assert!(!service.context().hovering_scrollbar);
    }

    #[test]
    fn mouse_enter_outside_hover_mode_is_ignored() {
        // Only `Hover` mode reacts to pointer enter; other modes no-op.
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Auto));

        resize(&mut service, 100.0, 100.0, 400.0, 400.0);

        let result = service.send(Event::MouseEnter);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Idle);
    }

    // --- auto-hide (Scroll mode) -------------------------------------------

    #[test]
    fn scroll_mode_shows_scrollbar_and_emits_auto_hide_intent() {
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Scroll));

        resize(&mut service, 100.0, 100.0, 100.0, 400.0);

        let result = service.send(Event::Scroll { x: 0.0, y: 150.0 });

        assert_eq!(service.state(), &State::ScrollActive);
        assert_eq!(service.context().scroll_y, 150.0);
        assert!(service.context().scrollbar_y_visible);
        assert!(
            result
                .pending_effects
                .iter()
                .any(|effect| effect.name == Effect::AutoHide),
            "Scroll in Scroll mode must emit the adapter-owned AutoHide intent",
        );
    }

    #[test]
    fn hide_timeout_hides_scrollbars_and_returns_to_idle() {
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Scroll));

        resize(&mut service, 100.0, 100.0, 100.0, 400.0);

        drop(service.send(Event::Scroll { x: 0.0, y: 150.0 }));

        let result = service.send(Event::HideTimeout);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().scrollbar_y_visible);
    }

    #[test]
    fn hide_timeout_is_ignored_while_dragging() {
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Scroll));

        resize(&mut service, 100.0, 100.0, 100.0, 400.0);

        drop(service.send(Event::Scroll { x: 0.0, y: 150.0 }));
        drop(service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::Y,
        }));

        assert_eq!(service.state(), &State::ThumbDragging);

        let result = service.send(Event::HideTimeout);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::ThumbDragging);
    }

    #[test]
    fn scroll_in_non_scroll_mode_updates_offset_without_state_change() {
        let mut service = service(props());

        resize(&mut service, 100.0, 100.0, 400.0, 400.0);

        let result = service.send(Event::Scroll { x: 30.0, y: 150.0 });

        assert!(!result.state_changed);
        assert_eq!(service.context().scroll_x, 30.0);
        assert_eq!(service.context().scroll_y, 150.0);
        assert!(result.pending_effects.is_empty());
    }

    // --- thumb dragging -----------------------------------------------------

    #[test]
    fn vertical_thumb_drag_produces_scroll_from_track_geometry() {
        let mut service = service(props());

        resize(&mut service, 100.0, 100.0, 100.0, 400.0);

        drop(service.send(Event::Scroll { x: 0.0, y: 150.0 }));

        let start = service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::Y,
        });

        assert!(start.state_changed);
        assert_eq!(service.state(), &State::ThumbDragging);
        assert_eq!(service.context().drag_axis, Some(Axis::Y));
        assert_eq!(service.context().drag_start_scroll_pos, 150.0);
        assert_eq!(service.context().drag_start_thumb_pos, 37.5);

        // Moving the pointer +20px down the 75px scrollable track maps to a
        // +80px scroll: (57.5 / 75) * 300 = 230.
        drop(service.send(Event::ThumbDragMove { pos: 20.0 }));

        assert_close(service.context().scroll_y, 230.0);

        let end = service.send(Event::ThumbDragEnd);

        assert!(end.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().drag_axis, None);
    }

    #[test]
    fn horizontal_thumb_drag_produces_scroll_from_track_geometry() {
        let mut service = service(props().orientation(ScrollOrientation::Horizontal));

        resize(&mut service, 100.0, 100.0, 400.0, 100.0);

        drop(service.send(Event::Scroll { x: 150.0, y: 0.0 }));

        drop(service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::X,
        }));

        assert_eq!(service.context().drag_axis, Some(Axis::X));

        drop(service.send(Event::ThumbDragMove { pos: 20.0 }));

        assert_close(service.context().scroll_x, 230.0);
    }

    #[test]
    fn thumb_drag_move_is_ignored_when_not_dragging() {
        let mut service = service(props());

        resize(&mut service, 100.0, 100.0, 100.0, 400.0);

        let result = service.send(Event::ThumbDragMove { pos: 20.0 });

        assert!(!result.state_changed);
        assert!(!result.context_changed);
        assert_eq!(service.context().scroll_y, 0.0);
    }

    // --- track click --------------------------------------------------------

    #[test]
    fn track_click_above_thumb_pages_backward() {
        let mut service = service(props());

        resize(&mut service, 100.0, 100.0, 100.0, 400.0);

        drop(service.send(Event::Scroll { x: 0.0, y: 150.0 }));

        // Click at 10px (< thumb_pos 37.5) -> page up by one viewport: 150-100.
        drop(service.send(Event::TrackClick {
            pos: 10.0,
            axis: Axis::Y,
        }));

        assert_eq!(service.context().scroll_y, 50.0);
    }

    #[test]
    fn track_click_below_thumb_pages_forward() {
        let mut service = service(props());

        resize(&mut service, 100.0, 100.0, 100.0, 400.0);

        drop(service.send(Event::Scroll { x: 0.0, y: 150.0 }));

        // Click at 80px (> thumb_pos+size 62.5) -> page down by one viewport,
        // clamped to content-viewport (300): 150+100 = 250.
        drop(service.send(Event::TrackClick {
            pos: 80.0,
            axis: Axis::Y,
        }));

        assert_eq!(service.context().scroll_y, 250.0);
    }

    #[test]
    fn track_click_on_thumb_does_not_scroll() {
        let mut service = service(props());

        resize(&mut service, 100.0, 100.0, 100.0, 400.0);

        drop(service.send(Event::Scroll { x: 0.0, y: 150.0 }));

        // Click at 50px lands on the thumb (37.5..=62.5) -> offset unchanged.
        drop(service.send(Event::TrackClick {
            pos: 50.0,
            axis: Axis::Y,
        }));

        assert_eq!(service.context().scroll_y, 150.0);
    }

    #[test]
    fn both_axes_track_shortened_by_cross_scrollbar_gap() {
        // With both scrollbars visible, axis_metrics subtracts the cross-axis
        // thickness from each track so the thumb does not overlap the corner.
        let mut service = service(
            props()
                .scrollbar_visibility(ScrollbarVisibility::Always)
                .scrollbar_cross_size(8.0),
        );

        resize(&mut service, 100.0, 100.0, 400.0, 400.0);

        assert!(service.context().scrollbar_x_visible);
        assert!(service.context().scrollbar_y_visible);

        drop(service.send(Event::Scroll { x: 0.0, y: 150.0 }));

        // Track shortened to 100 - 8 = 92. thumb_size = (100/400)*92 = 23,
        // thumb_pos = (150/300)*(92-23) = 34.5 (vs 37.5 with a full track).
        drop(service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::Y,
        }));

        assert_close(service.context().drag_start_thumb_pos, 34.5);
    }

    #[test]
    fn track_click_on_horizontal_axis_pages_scroll() {
        // Exercises the `Axis::X` arms and the X-axis cross-size branch of
        // axis_metrics (vertical scrollbar visible -> horizontal track shortened).
        let mut service = service(
            props()
                .scrollbar_visibility(ScrollbarVisibility::Always)
                .scrollbar_cross_size(8.0),
        );

        resize(&mut service, 100.0, 100.0, 400.0, 400.0);

        assert!(service.context().scrollbar_y_visible);

        drop(service.send(Event::Scroll { x: 150.0, y: 0.0 }));

        // Click left of the thumb pages backward one viewport: 150 - 100 = 50.
        drop(service.send(Event::TrackClick {
            pos: 1.0,
            axis: Axis::X,
        }));

        assert_eq!(service.context().scroll_x, 50.0);
    }

    #[test]
    fn cross_size_defaults_to_zero_when_unset() {
        // Without the prop, the cross-axis gap is 0, so the track keeps its
        // full length and the thumb position is unshortened.
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Always));

        resize(&mut service, 100.0, 100.0, 400.0, 400.0);

        drop(service.send(Event::Scroll { x: 0.0, y: 150.0 }));

        drop(service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::Y,
        }));

        assert_close(service.context().drag_start_thumb_pos, 37.5);
    }

    // --- normalize_scroll_left ---------------------------------------------

    #[test]
    fn normalize_scroll_left_is_identity_in_ltr() {
        assert_eq!(normalize_scroll_left(50.0, 400.0, 100.0, false), 50.0);
    }

    #[test]
    fn normalize_scroll_left_flips_modern_rtl_negative_convention() {
        assert_eq!(normalize_scroll_left(-50.0, 400.0, 100.0, true), 50.0);
        assert_eq!(normalize_scroll_left(0.0, 400.0, 100.0, true), 0.0);
    }

    #[test]
    fn normalize_scroll_left_mirrors_safari_positive_rtl_convention() {
        // scroll_width - client_width - raw = 400 - 100 - 50 = 250.
        assert_eq!(normalize_scroll_left(50.0, 400.0, 100.0, true), 250.0);
    }

    // --- query methods ------------------------------------------------------

    #[test]
    fn edge_queries_and_progress_track_scroll_offset() {
        let mut service = service(props());

        resize(&mut service, 100.0, 100.0, 400.0, 400.0);

        {
            let api = service.connect(&|_| {});

            assert!(api.is_at_top());
            assert!(api.is_at_left());
            assert!(!api.is_at_bottom());
            assert!(!api.is_at_right());
            assert_eq!(api.scroll_progress(), (0.0, 0.0));
        }

        drop(service.send(Event::Scroll { x: 150.0, y: 300.0 }));

        {
            let api = service.connect(&|_| {});

            assert!(!api.is_at_top());
            assert!(api.is_at_bottom());
            // Half-way horizontally (150 of 300 scrollable) -> not at the right.
            assert!(!api.is_at_right());
            assert_eq!(api.scroll_progress(), (0.5, 1.0));
        }

        drop(service.send(Event::Scroll { x: 300.0, y: 300.0 }));

        {
            let api = service.connect(&|_| {});

            assert!(api.is_at_right());
            assert_eq!(api.scroll_progress(), (1.0, 1.0));
        }
    }

    #[test]
    fn scroll_progress_is_zero_without_overflow() {
        let mut service = service(props());

        resize(&mut service, 100.0, 100.0, 100.0, 100.0);

        let api = service.connect(&|_| {});

        assert_eq!(api.scroll_progress(), (0.0, 0.0));
    }

    // --- connect / ARIA -----------------------------------------------------

    #[test]
    fn viewport_exposes_region_role_and_native_keyboard_focus() {
        // The viewport's role="region" + tabindex="0" is what enables native
        // keyboard scrolling; the machine has no keyboard events of its own.
        let service = service(props());

        let attrs = service.connect(&|_| {}).viewport_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("region"));
        assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("0"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Scrollable content")
        );
    }

    #[test]
    fn viewport_uses_custom_aria_label_when_provided() {
        let service = service(props().aria_label("Activity log"));

        let attrs = service.connect(&|_| {}).viewport_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Activity log")
        );
    }

    #[test]
    fn scrollbars_and_thumbs_are_decorative() {
        let service = service(props());

        let api = service.connect(&|_| {});

        for attrs in [
            api.scrollbar_y_attrs(),
            api.thumb_y_attrs(),
            api.scrollbar_x_attrs(),
            api.thumb_x_attrs(),
            api.corner_square_attrs(),
        ] {
            assert_eq!(attrs.get(&HtmlAttr::Role), Some("none"));
        }

        assert_eq!(
            api.scrollbar_y_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Orientation)),
            Some("vertical")
        );
        assert_eq!(
            api.scrollbar_x_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Orientation)),
            Some("horizontal")
        );
    }

    #[test]
    fn root_exposes_state_overflow_and_rtl() {
        let mut service = service(props().dir(Direction::Rtl));

        resize(&mut service, 100.0, 100.0, 400.0, 400.0);

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("idle"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-overflow-x")), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-overflow-y")), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-dir")), Some("rtl"));
    }

    #[test]
    fn root_state_token_reflects_each_state() {
        let idle = service(props());

        assert_eq!(
            idle.connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Data("ars-state")),
            Some("idle")
        );

        let mut hovering = service(props().scrollbar_visibility(ScrollbarVisibility::Hover));

        resize(&mut hovering, 100.0, 100.0, 400.0, 400.0);

        drop(hovering.send(Event::MouseEnter));

        assert_eq!(
            hovering
                .connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Data("ars-state")),
            Some("hovering")
        );

        let mut scrolling = service(props().scrollbar_visibility(ScrollbarVisibility::Scroll));

        resize(&mut scrolling, 100.0, 100.0, 100.0, 400.0);

        drop(scrolling.send(Event::Scroll { x: 0.0, y: 10.0 }));

        assert_eq!(
            scrolling
                .connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Data("ars-state")),
            Some("scroll-active")
        );

        let mut dragging = service(props());

        resize(&mut dragging, 100.0, 100.0, 100.0, 400.0);

        drop(dragging.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::Y,
        }));

        assert_eq!(
            dragging
                .connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Data("ars-state")),
            Some("thumb-dragging")
        );
    }

    #[test]
    fn data_visible_reflects_scrollbar_visibility() {
        let mut service = service(props());

        resize(&mut service, 100.0, 100.0, 100.0, 400.0);

        let api = service.connect(&|_| {});

        assert_eq!(
            api.scrollbar_y_attrs().get(&HtmlAttr::Data("ars-visible")),
            Some("true")
        );
        assert_eq!(
            api.scrollbar_x_attrs().get(&HtmlAttr::Data("ars-visible")),
            Some("false")
        );
    }

    // --- adapter event handlers --------------------------------------------

    #[test]
    fn api_handlers_forward_typed_events() {
        let events = RefCell::new(Vec::new());
        let send = |event| events.borrow_mut().push(event);

        let service = service(props());

        let api = service.connect(&send);

        api.on_viewport_scroll(5.0, 6.0);
        api.on_root_mouseenter();
        api.on_root_mouseleave();
        api.on_scrollbar_mouseenter();
        api.on_scrollbar_mouseleave();
        api.on_thumb_pointerdown(12.0, Axis::Y);
        api.on_thumb_pointermove(15.0);
        api.on_thumb_pointerup();
        api.on_track_click(40.0, Axis::X);

        assert_eq!(
            events.into_inner(),
            [
                Event::Scroll { x: 5.0, y: 6.0 },
                Event::MouseEnter,
                Event::MouseLeave,
                Event::MouseEnterScrollbar,
                Event::MouseLeaveScrollbar,
                Event::ThumbDragStart {
                    pos: 12.0,
                    axis: Axis::Y
                },
                Event::ThumbDragMove { pos: 15.0 },
                Event::ThumbDragEnd,
                Event::TrackClick {
                    pos: 40.0,
                    axis: Axis::X
                },
            ]
        );
    }

    #[test]
    fn api_debug_renders_struct_name() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert!(format!("{api:?}").contains("Api"));
    }

    #[test]
    fn part_attrs_matches_direct_methods() {
        let mut service = service(props());

        resize(&mut service, 100.0, 100.0, 400.0, 400.0);

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Viewport), api.viewport_attrs());
        assert_eq!(api.part_attrs(Part::Content), api.content_attrs());
        assert_eq!(api.part_attrs(Part::ScrollbarY), api.scrollbar_y_attrs());
        assert_eq!(api.part_attrs(Part::ThumbY), api.thumb_y_attrs());
        assert_eq!(api.part_attrs(Part::ScrollbarX), api.scrollbar_x_attrs());
        assert_eq!(api.part_attrs(Part::ThumbX), api.thumb_x_attrs());
        assert_eq!(
            api.part_attrs(Part::CornerSquare),
            api.corner_square_attrs()
        );
    }

    // --- snapshots ----------------------------------------------------------

    #[test]
    fn snapshot_all_output_affecting_branches() {
        let mut both = service(props().scrollbar_visibility(ScrollbarVisibility::Always));

        resize(&mut both, 100.0, 100.0, 400.0, 400.0);

        let mut rtl = service(props().dir(Direction::Rtl));

        resize(&mut rtl, 100.0, 100.0, 400.0, 400.0);

        let mut scroll_active = service(props().scrollbar_visibility(ScrollbarVisibility::Scroll));

        resize(&mut scroll_active, 100.0, 100.0, 100.0, 400.0);

        drop(scroll_active.send(Event::Scroll { x: 0.0, y: 10.0 }));

        let custom_label = service(props().aria_label("Activity log"));

        let custom_messages = Service::<Machine>::new(
            props(),
            &Env::default(),
            &Messages {
                viewport_label: MessageFn::new(|_locale: &Locale| "Contenido".to_string()),
            },
        );

        let both_api = both.connect(&|_| {});

        assert_snapshot!(
            "scroll_area_root_idle",
            snapshot_attrs(&both_api.root_attrs())
        );
        assert_snapshot!(
            "scroll_area_viewport_default",
            snapshot_attrs(&both_api.viewport_attrs())
        );
        assert_snapshot!(
            "scroll_area_content",
            snapshot_attrs(&both_api.content_attrs())
        );
        assert_snapshot!(
            "scroll_area_scrollbar_y_visible",
            snapshot_attrs(&both_api.scrollbar_y_attrs())
        );
        assert_snapshot!(
            "scroll_area_thumb_y",
            snapshot_attrs(&both_api.thumb_y_attrs())
        );
        assert_snapshot!(
            "scroll_area_scrollbar_x_visible",
            snapshot_attrs(&both_api.scrollbar_x_attrs())
        );
        assert_snapshot!(
            "scroll_area_thumb_x",
            snapshot_attrs(&both_api.thumb_x_attrs())
        );
        assert_snapshot!(
            "scroll_area_corner_square",
            snapshot_attrs(&both_api.corner_square_attrs())
        );

        assert_snapshot!(
            "scroll_area_root_rtl",
            snapshot_attrs(&rtl.connect(&|_| {}).root_attrs())
        );
        assert_snapshot!(
            "scroll_area_root_scroll_active",
            snapshot_attrs(&scroll_active.connect(&|_| {}).root_attrs())
        );
        assert_snapshot!(
            "scroll_area_scrollbar_x_hidden",
            snapshot_attrs(&scroll_active.connect(&|_| {}).scrollbar_x_attrs())
        );
        assert_snapshot!(
            "scroll_area_viewport_custom_label",
            snapshot_attrs(&custom_label.connect(&|_| {}).viewport_attrs())
        );
        assert_snapshot!(
            "scroll_area_viewport_custom_message",
            snapshot_attrs(&custom_messages.connect(&|_| {}).viewport_attrs())
        );
    }
}
