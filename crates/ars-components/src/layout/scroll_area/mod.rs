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

use alloc::{string::String, vec::Vec};
use core::{
    fmt::{self, Debug},
    time::Duration,
};

use ars_core::{
    AriaAttr, AttrMap, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Direction, Env,
    HtmlAttr, Locale, MessageFn, PendingEffect, TransitionPlan,
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

    /// Re-sync prop-backed context fields after a props change. Emitted by
    /// [`Machine::on_props_changed`](ars_core::Machine::on_props_changed).
    SyncProps,
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

impl ScrollOrientation {
    /// Whether the horizontal scrollbar is enabled for this orientation.
    #[must_use]
    pub const fn allows_x(self) -> bool {
        matches!(self, Self::Horizontal | Self::Both)
    }

    /// Whether the vertical scrollbar is enabled for this orientation.
    #[must_use]
    pub const fn allows_y(self) -> bool {
        matches!(self, Self::Vertical | Self::Both)
    }
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

    /// Whether the pointer is currently over the root scroll area.
    pub hovering_root: bool,

    /// Which scroll orientation is enabled. Gates which scrollbars may become
    /// visible.
    pub orientation: ScrollOrientation,

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

    /// Resolved text direction. Drives [`normalize_scroll_left_rtl`] and
    /// vertical scrollbar placement (left side in RTL).
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

    /// Clamp the stored scroll offsets into `0..=(content - viewport)` on each
    /// axis. The rest of the machine (thumb metrics, edge queries) assumes
    /// in-range offsets, so any externally-supplied value — overscroll bounce,
    /// a stale programmatic scroll after the content shrank — is clamped before
    /// it is stored.
    fn clamp_offsets(&mut self) {
        self.scroll_x = self
            .scroll_x
            .clamp(0.0, (self.content_width - self.viewport_width).max(0.0));
        self.scroll_y = self
            .scroll_y
            .clamp(0.0, (self.content_height - self.viewport_height).max(0.0));
    }

    /// Whether the horizontal scrollbar is permitted to be visible: the
    /// orientation must enable it and (outside `Always` mode) the content must
    /// overflow.
    #[must_use]
    fn can_show_x(&self) -> bool {
        self.orientation.allows_x()
            && (self.scrollbar_visibility == ScrollbarVisibility::Always || self.has_overflow_x())
    }

    /// Whether the vertical scrollbar is permitted to be visible.
    #[must_use]
    fn can_show_y(&self) -> bool {
        self.orientation.allows_y()
            && (self.scrollbar_visibility == ScrollbarVisibility::Always || self.has_overflow_y())
    }

    /// Recompute scrollbar visibility after a metrics or prop change.
    ///
    /// `Always` and `Auto` derive visibility directly from orientation and
    /// overflow. For `Hover`/`Scroll`, visibility tracks whether a hover/scroll
    /// session is currently `active` (state `Hovering`/`ScrollActive`): when
    /// active, each enabled+overflowing axis shows — so a prop/metrics change
    /// that newly enables an axis turns it on; when inactive, both hide.
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

/// Whether a hover/scroll/drag session is currently active for the given state,
/// i.e. scrollbars should be shown for eligible axes. `ThumbDragging` counts:
/// the thumb being dragged must stay visible even if a resize fires mid-drag.
const fn is_session_active(state: State) -> bool {
    matches!(
        state,
        State::Hovering | State::ScrollActive | State::ThumbDragging
    )
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

// RTL `scrollLeft` normalization is shared with the virtualization layer: a
// sign heuristic cannot tell the negative convention's `0` (inline-start) from
// the positive convention's `0` (inline-end), so the browser convention must be
// detected once by the adapter and passed explicitly. Adapters read the live
// `scrollLeft`, normalize with [`normalize_scroll_left_rtl`], then send the
// result as [`Event::Scroll`]'s `x`.
pub use ars_collections::{RtlScrollMode, normalize_scroll_left_rtl};

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

            // Clamp to zero: a corner gap wider than the viewport (tiny scroll
            // areas) must not yield a negative track length.
            (
                ctx.viewport_width,
                ctx.content_width,
                (ctx.viewport_width - cross).max(0.0),
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
                (ctx.viewport_height - cross).max(0.0),
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
            hovering_root: false,
            orientation: props.orientation,
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
                let active = is_session_active(*state);
                let min_thumb = ctx.min_thumb_size;
                // A drag captured its baseline against the OLD track geometry. To
                // keep tracking continuous, shift `drag_start_thumb_pos` by the
                // change in the current scroll's thumb position from old to new
                // geometry. Capture the old thumb position here (pre-resize); the
                // new one is computed in the closure after the metrics update.
                let drag_rebase = if *state == State::ThumbDragging {
                    ctx.drag_axis.map(|axis| {
                        let scroll = match axis {
                            Axis::X => ctx.scroll_x,
                            Axis::Y => ctx.scroll_y,
                        };
                        let (vp, content, track) = axis_metrics(ctx, axis);
                        let thumb_old =
                            compute_thumb_metrics(vp, content, scroll, track, min_thumb).1;
                        (axis, thumb_old)
                    })
                } else {
                    None
                };

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.viewport_width = vw;
                    ctx.viewport_height = vh;
                    ctx.content_width = cw;
                    ctx.content_height = ch;
                    // Shrinking the content/viewport can leave a previously-valid
                    // offset past the new max; clamp so queries and adapter
                    // scroll-syncs never observe an impossible offset.
                    ctx.clamp_offsets();
                    ctx.update_visibility(active);

                    if let Some((axis, thumb_old)) = drag_rebase {
                        let scroll = match axis {
                            Axis::X => ctx.scroll_x,
                            Axis::Y => ctx.scroll_y,
                        };
                        let (vp, content, track) = axis_metrics(ctx, axis);
                        let thumb_new =
                            compute_thumb_metrics(vp, content, scroll, track, min_thumb).1;
                        // Keep the pointer baseline; only re-anchor the thumb so
                        // the current pointer still maps to `scroll`.
                        ctx.drag_start_thumb_pos += thumb_new - thumb_old;
                        ctx.drag_start_scroll_pos = scroll;
                    }
                }))
            }

            Event::Scroll { x, y } => {
                let (sx, sy) = (*x, *y);

                // A scroll event mid-drag is the browser echoing the offset the
                // adapter just wrote; record it without leaving `ThumbDragging`,
                // or later `ThumbDragMove` events would be dropped by the guard.
                if *state == State::ThumbDragging {
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.scroll_x = sx;
                        ctx.scroll_y = sy;
                        ctx.clamp_offsets();
                    }))
                } else if ctx.scrollbar_visibility == ScrollbarVisibility::Scroll {
                    Some(
                        TransitionPlan::to(State::ScrollActive)
                            .apply(move |ctx: &mut Context| {
                                ctx.scroll_x = sx;
                                ctx.scroll_y = sy;
                                ctx.clamp_offsets();
                                ctx.scrollbar_x_visible = ctx.can_show_x();
                                ctx.scrollbar_y_visible = ctx.can_show_y();
                            })
                            .with_effect(PendingEffect::named(Effect::AutoHide)),
                    )
                } else {
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.scroll_x = sx;
                        ctx.scroll_y = sy;
                        ctx.clamp_offsets();
                    }))
                }
            }

            Event::MouseEnter => {
                // A captured pointer can re-enter the root mid-drag; record the
                // hover flag but never leave `ThumbDragging`, or the drag would
                // stop (mirrors the leave-path guard).
                if ctx.scrollbar_visibility == ScrollbarVisibility::Hover
                    && *state != State::ThumbDragging
                {
                    Some(
                        TransitionPlan::to(State::Hovering).apply(|ctx: &mut Context| {
                            ctx.hovering_root = true;
                            ctx.scrollbar_x_visible = ctx.can_show_x();
                            ctx.scrollbar_y_visible = ctx.can_show_y();
                        }),
                    )
                } else {
                    Some(TransitionPlan::context_only(|ctx: &mut Context| {
                        ctx.hovering_root = true;
                    }))
                }
            }

            Event::MouseLeave => {
                // Hide only once the pointer has left both the root and any
                // overlaid scrollbar track — and never while a thumb drag is in
                // progress (the pointer is captured and may leave the root).
                if ctx.scrollbar_visibility == ScrollbarVisibility::Hover
                    && !ctx.hovering_scrollbar
                    && *state != State::ThumbDragging
                {
                    Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                        ctx.hovering_root = false;
                        ctx.scrollbar_x_visible = false;
                        ctx.scrollbar_y_visible = false;
                    }))
                } else {
                    Some(TransitionPlan::context_only(|ctx: &mut Context| {
                        ctx.hovering_root = false;
                    }))
                }
            }

            Event::MouseEnterScrollbar => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.hovering_scrollbar = true;
                }))
            }

            Event::MouseLeaveScrollbar => {
                // Mirror of `MouseLeave`: if the pointer is no longer over the
                // root either, the overlaid scrollbar must hide now — no further
                // root-leave event will arrive. Never while dragging.
                if ctx.scrollbar_visibility == ScrollbarVisibility::Hover
                    && !ctx.hovering_root
                    && *state != State::ThumbDragging
                {
                    Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                        ctx.hovering_scrollbar = false;
                        ctx.scrollbar_x_visible = false;
                        ctx.scrollbar_y_visible = false;
                    }))
                } else {
                    Some(TransitionPlan::context_only(|ctx: &mut Context| {
                        ctx.hovering_scrollbar = false;
                    }))
                }
            }

            Event::HideTimeout => {
                // Only honour the hide timer while still in Scroll mode and not
                // dragging. A timeout that was already queued before a switch to
                // Always/Auto must be ignored — the adapter can cancel future
                // fires but not retract an already-posted event.
                if ctx.scrollbar_visibility == ScrollbarVisibility::Scroll
                    && *state != State::ThumbDragging
                {
                    Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                        ctx.scrollbar_x_visible = false;
                        ctx.scrollbar_y_visible = false;
                    }))
                } else {
                    None
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
                    TransitionPlan::to(State::ThumbDragging)
                        .apply(move |ctx: &mut Context| {
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
                        })
                        // Explicitly cancel any running Scroll-mode hide timer so
                        // it cannot fire mid-drag; `ThumbDragEnd` starts a fresh
                        // one. A no-op when no AutoHide is active.
                        .cancel_effect(Effect::AutoHide),
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

                // In RTL, `scroll_x` is normalized to distance from inline-start
                // (the right edge), which increases as the physical thumb moves
                // left. Invert the horizontal pointer delta so dragging the thumb
                // right scrolls toward inline-start. Vertical/LTR are unchanged.
                let raw_delta = pointer - drag_start_pointer;
                let delta = if axis == Axis::X && ctx.dir == Direction::Rtl {
                    -raw_delta
                } else {
                    raw_delta
                };

                let (thumb_size, _) = compute_thumb_metrics(
                    viewport_size,
                    content_size,
                    drag_scroll,
                    track_size,
                    min_thumb,
                );

                // Clamp the thumb to the scrollable track so a drag past either
                // end cannot request a scroll offset beyond the content bounds.
                let max_thumb_pos = (track_size - thumb_size).max(0.0);
                let new_thumb_pos = (drag_start_thumb + delta).clamp(0.0, max_thumb_pos);
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
                if ctx.scrollbar_visibility == ScrollbarVisibility::Scroll {
                    // The hide timer was cancelled when the drag began; restart
                    // it so the scrollbar fades after the drag instead of
                    // lingering until the next scroll.
                    Some(
                        TransitionPlan::to(State::ScrollActive)
                            .apply(|ctx: &mut Context| {
                                ctx.drag_axis = None;
                            })
                            .with_effect(PendingEffect::named(Effect::AutoHide)),
                    )
                } else if ctx.scrollbar_visibility == ScrollbarVisibility::Hover {
                    // A drag can end with the pointer off-root (leave events were
                    // suppressed mid-drag). Re-apply the hover rule: stay visible
                    // only while the pointer is still over the root or a track.
                    let still_hovering = ctx.hovering_root || ctx.hovering_scrollbar;
                    let target = if still_hovering {
                        State::Hovering
                    } else {
                        State::Idle
                    };
                    Some(TransitionPlan::to(target).apply(move |ctx: &mut Context| {
                        ctx.drag_axis = None;
                        ctx.update_visibility(still_hovering);
                    }))
                } else {
                    Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                        ctx.drag_axis = None;
                    }))
                }
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

                let max_scroll = (content_size - viewport_size).max(0.0);
                let new_scroll = if click < thumb_pos {
                    (scroll_pos - viewport_size).max(0.0)
                } else if click > thumb_pos + thumb_size {
                    (scroll_pos + viewport_size).min(max_scroll)
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

            Event::SyncProps => {
                let orientation = props.orientation;
                let visibility = props.scrollbar_visibility;
                let min_thumb = props.min_thumb_size.unwrap_or(DEFAULT_MIN_THUMB_SIZE);
                let hide_delay = props.hide_delay;
                let cross = props.scrollbar_cross_size.unwrap_or(0.0);
                let dir = props.dir.unwrap_or(Direction::Ltr);

                // Leaving the visibility mode the current active state belongs
                // to resets the machine to `Idle`: otherwise a stuck
                // `ScrollActive`/`Hovering` state lingers (and, for Scroll, an
                // orphaned `AutoHide` timer could later hide an Always/Auto
                // scrollbar). A `ThumbDragging` session is preserved.
                let leaving_scroll_active =
                    *state == State::ScrollActive && visibility != ScrollbarVisibility::Scroll;
                let leaving_hover =
                    *state == State::Hovering && visibility != ScrollbarVisibility::Hover;
                let reset_state = leaving_scroll_active || leaving_hover;

                // After the (possible) state change, derive visibility against
                // the resulting state so a newly-enabled axis turns on while the
                // session is still active.
                let active = !reset_state && is_session_active(*state);

                let mut plan = if reset_state {
                    TransitionPlan::to(State::Idle)
                } else {
                    TransitionPlan::new()
                };

                plan = plan.apply(move |ctx: &mut Context| {
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

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        let prop_backed_context_changed = old.orientation != new.orientation
            || old.scrollbar_visibility != new.scrollbar_visibility
            || old.min_thumb_size != new.min_thumb_size
            || old.hide_delay != new.hide_delay
            || old.scrollbar_cross_size != new.scrollbar_cross_size
            || old.dir != new.dir;

        if prop_backed_context_changed {
            alloc::vec![Event::SyncProps]
        } else {
            Vec::new()
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

        // `role="none"` is presentational, so ARIA state/property attributes
        // (incl. `aria-orientation`) are not valid on it; the part name already
        // encodes the axis. A `data-ars-orientation` marker is kept for styling.
        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "none")
            .set(HtmlAttr::Data("ars-orientation"), "vertical")
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
            .set(HtmlAttr::Data("ars-orientation"), "horizontal")
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
    ///
    /// `data-ars-visible` is `true` only when both scrollbars are visible, so
    /// adapters can hide the filler when a single axis is showing without
    /// duplicating the core visibility logic.
    #[must_use]
    pub fn corner_square_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CornerSquare.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "none")
            .set_bool(
                HtmlAttr::Data("ars-visible"),
                self.ctx.scrollbar_x_visible && self.ctx.scrollbar_y_visible,
            );

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
                .orientation(ScrollOrientation::Both)
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
        let mut service = service(
            props()
                .orientation(ScrollOrientation::Both)
                .scrollbar_visibility(ScrollbarVisibility::Hover),
        );

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
        // Only `Hover` mode shows scrollbars on pointer enter; other modes only
        // record the root-hover flag without a state change.
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Auto));

        resize(&mut service, 100.0, 100.0, 400.0, 400.0);

        let result = service.send(Event::MouseEnter);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().hovering_root);
    }

    #[test]
    fn hover_scrollbar_outliving_root_hides_on_scrollbar_leave() {
        // root enter -> scrollbar enter -> root leave -> scrollbar leave: the
        // final scrollbar leave must hide, since no further root-leave arrives.
        let mut service = service(
            props()
                .orientation(ScrollOrientation::Both)
                .scrollbar_visibility(ScrollbarVisibility::Hover),
        );
        resize(&mut service, 100.0, 100.0, 400.0, 400.0);

        drop(service.send(Event::MouseEnter));
        drop(service.send(Event::MouseEnterScrollbar));
        drop(service.send(Event::MouseLeave));
        assert_eq!(service.state(), &State::Hovering);
        assert!(service.context().scrollbar_y_visible);

        let result = service.send(Event::MouseLeaveScrollbar);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().scrollbar_x_visible);
        assert!(!service.context().scrollbar_y_visible);
    }

    // --- orientation gating -------------------------------------------------

    #[test]
    fn orientation_gates_which_scrollbars_can_show() {
        let mut vertical = service(props().orientation(ScrollOrientation::Vertical));
        resize(&mut vertical, 100.0, 100.0, 400.0, 400.0);
        assert!(vertical.context().scrollbar_y_visible);
        assert!(
            !vertical.context().scrollbar_x_visible,
            "a vertical scroll area must never expose a horizontal scrollbar",
        );

        let mut horizontal = service(props().orientation(ScrollOrientation::Horizontal));
        resize(&mut horizontal, 100.0, 100.0, 400.0, 400.0);
        assert!(horizontal.context().scrollbar_x_visible);
        assert!(!horizontal.context().scrollbar_y_visible);

        let mut both = service(props().orientation(ScrollOrientation::Both));
        resize(&mut both, 100.0, 100.0, 400.0, 400.0);
        assert!(both.context().scrollbar_x_visible);
        assert!(both.context().scrollbar_y_visible);
    }

    #[test]
    fn always_mode_is_gated_by_orientation() {
        // `Always` shows scrollbars regardless of overflow, but only on enabled
        // axes.
        let service = service(
            props()
                .orientation(ScrollOrientation::Vertical)
                .scrollbar_visibility(ScrollbarVisibility::Always),
        );
        assert!(service.context().scrollbar_y_visible);
        assert!(!service.context().scrollbar_x_visible);
    }

    // --- resize recomputation ----------------------------------------------

    #[test]
    fn resize_clears_stale_visibility_in_scroll_mode() {
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Scroll));
        resize(&mut service, 100.0, 100.0, 100.0, 400.0);
        drop(service.send(Event::Scroll { x: 0.0, y: 50.0 }));
        assert!(service.context().scrollbar_y_visible);

        // Content shrinks so the axis no longer overflows; the previously-shown
        // scrollbar must clear even though Scroll mode is event-driven.
        resize(&mut service, 100.0, 100.0, 100.0, 50.0);
        assert!(!service.context().scrollbar_y_visible);
    }

    #[test]
    fn resize_clears_stale_visibility_in_hover_mode() {
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Hover));
        resize(&mut service, 100.0, 100.0, 100.0, 400.0);
        drop(service.send(Event::MouseEnter));
        assert!(service.context().scrollbar_y_visible);

        resize(&mut service, 100.0, 100.0, 100.0, 50.0);
        assert!(!service.context().scrollbar_y_visible);
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
    fn rtl_horizontal_thumb_drag_inverts_direction() {
        // In RTL, dragging the horizontal thumb right (positive pointer delta)
        // scrolls toward inline-start, i.e. decreases the normalized scroll_x.
        let mut service = service(
            props()
                .orientation(ScrollOrientation::Both)
                .dir(Direction::Rtl),
        );
        resize(&mut service, 100.0, 100.0, 400.0, 100.0);
        drop(service.send(Event::Scroll { x: 150.0, y: 0.0 }));
        drop(service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::X,
        }));

        // +20 pointer delta -> -20 thumb delta: 37.5 - 20 = 17.5 -> scroll 70.
        drop(service.send(Event::ThumbDragMove { pos: 20.0 }));
        assert_close(service.context().scroll_x, 70.0);
        assert!(service.context().scroll_x < 150.0);
    }

    #[test]
    fn resize_mid_drag_preserves_scroll_on_zero_delta_move() {
        // A ResizeObserver firing mid-drag rebases the thumb baseline so the next
        // zero-delta move keeps the current scroll instead of jumping.
        let mut service = service(props());
        resize(&mut service, 100.0, 100.0, 100.0, 400.0);
        drop(service.send(Event::Scroll { x: 0.0, y: 150.0 }));
        drop(service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::Y,
        }));

        // Viewport grows 100 -> 200 mid-drag.
        resize(&mut service, 100.0, 200.0, 100.0, 400.0);

        // Pointer unchanged (pos still 0) -> scroll must stay 150, not jump to 75.
        drop(service.send(Event::ThumbDragMove { pos: 0.0 }));
        assert_close(service.context().scroll_y, 150.0);
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

    #[test]
    fn thumb_drag_clamps_scroll_to_content_bounds() {
        let mut service = service(props());
        resize(&mut service, 100.0, 100.0, 100.0, 400.0);

        drop(service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::Y,
        }));

        // Dragging far past the track end must clamp to the maximum scroll
        // (content 400 - viewport 100 = 300), not overshoot to 800.
        drop(service.send(Event::ThumbDragMove { pos: 200.0 }));
        assert_eq!(service.context().scroll_y, 300.0);
    }

    #[test]
    fn scroll_mode_drag_end_restarts_auto_hide() {
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Scroll));
        resize(&mut service, 100.0, 100.0, 100.0, 400.0);
        drop(service.send(Event::Scroll { x: 0.0, y: 150.0 }));
        drop(service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::Y,
        }));

        let result = service.send(Event::ThumbDragEnd);

        // Drag-end in Scroll mode must restart the adapter-owned hide timer so
        // the scrollbar fades instead of lingering.
        assert_eq!(service.state(), &State::ScrollActive);
        assert_eq!(service.context().drag_axis, None);
        assert!(
            result
                .pending_effects
                .iter()
                .any(|effect| effect.name == Effect::AutoHide),
        );
    }

    #[test]
    fn thumb_drag_survives_negative_track_from_oversized_corner_gap() {
        // A corner gap wider than the viewport must not produce a negative
        // track length and NaN/negative drag math on tiny scroll areas.
        let mut service = service(
            props()
                .orientation(ScrollOrientation::Both)
                .scrollbar_visibility(ScrollbarVisibility::Always)
                .scrollbar_cross_size(50.0),
        );
        resize(&mut service, 10.0, 10.0, 100.0, 100.0);

        drop(service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::Y,
        }));
        assert_eq!(service.context().drag_start_thumb_pos, 0.0);

        drop(service.send(Event::ThumbDragMove { pos: 5.0 }));
        assert!(service.context().scroll_y.is_finite());
        assert_eq!(service.context().scroll_y, 0.0);
    }

    #[test]
    fn hover_leave_during_drag_does_not_cancel_the_drag() {
        let mut service = service(
            props()
                .orientation(ScrollOrientation::Both)
                .scrollbar_visibility(ScrollbarVisibility::Hover),
        );
        resize(&mut service, 100.0, 100.0, 400.0, 400.0);
        drop(service.send(Event::MouseEnter));
        drop(service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::Y,
        }));
        assert_eq!(service.state(), &State::ThumbDragging);

        // Pointer (still captured) leaves the root mid-drag.
        let leave = service.send(Event::MouseLeave);
        assert!(!leave.state_changed);
        assert_eq!(service.state(), &State::ThumbDragging);
        assert_eq!(service.context().drag_axis, Some(Axis::Y));

        // Drag continues to scroll instead of being ignored.
        drop(service.send(Event::ThumbDragMove { pos: 20.0 }));
        assert_close(service.context().scroll_y, 80.0);

        // A scrollbar leave mid-drag is likewise ignored as a hide trigger.
        let scrollbar_leave = service.send(Event::MouseLeaveScrollbar);
        assert!(!scrollbar_leave.state_changed);
        assert_eq!(service.state(), &State::ThumbDragging);
    }

    #[test]
    fn late_hide_timeout_after_mode_change_is_ignored() {
        // A HideTimeout queued before switching away from Scroll must not hide
        // the now-Always scrollbars.
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Scroll));
        resize(&mut service, 100.0, 100.0, 100.0, 400.0);
        drop(service.send(Event::Scroll { x: 0.0, y: 50.0 }));
        drop(
            service.set_props(
                props()
                    .orientation(ScrollOrientation::Both)
                    .scrollbar_visibility(ScrollbarVisibility::Always),
            ),
        );

        let result = service.send(Event::HideTimeout);

        assert!(!result.state_changed);
        assert!(service.context().scrollbar_y_visible);
    }

    #[test]
    fn scroll_event_during_drag_keeps_drag_state() {
        // The adapter writes the computed offset to the viewport; the browser's
        // echoed Scroll event must not abort the drag.
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Scroll));
        resize(&mut service, 100.0, 100.0, 100.0, 400.0);
        drop(service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::Y,
        }));
        assert_eq!(service.state(), &State::ThumbDragging);

        let echoed = service.send(Event::Scroll { x: 0.0, y: 30.0 });
        assert!(!echoed.state_changed);
        assert_eq!(service.state(), &State::ThumbDragging);
        assert_eq!(service.context().scroll_y, 30.0);

        // A subsequent drag move is still honoured.
        drop(service.send(Event::ThumbDragMove { pos: 20.0 }));
        assert_close(service.context().scroll_y, 80.0);
    }

    #[test]
    fn prop_sync_turns_on_newly_enabled_axis_while_hovering() {
        // Hovering in Vertical mode, then orientation -> Both: the horizontal
        // scrollbar must appear immediately while the root is still hovered.
        let mut service = service(
            props()
                .orientation(ScrollOrientation::Vertical)
                .scrollbar_visibility(ScrollbarVisibility::Hover),
        );
        resize(&mut service, 100.0, 100.0, 400.0, 400.0);
        drop(service.send(Event::MouseEnter));
        assert!(service.context().scrollbar_y_visible);
        assert!(!service.context().scrollbar_x_visible);

        drop(
            service.set_props(
                props()
                    .orientation(ScrollOrientation::Both)
                    .scrollbar_visibility(ScrollbarVisibility::Hover),
            ),
        );

        assert_eq!(service.state(), &State::Hovering);
        assert!(service.context().scrollbar_x_visible);
        assert!(service.context().scrollbar_y_visible);
    }

    #[test]
    fn hover_drag_ending_off_root_hides_scrollbars() {
        let mut service = service(
            props()
                .orientation(ScrollOrientation::Both)
                .scrollbar_visibility(ScrollbarVisibility::Hover),
        );
        resize(&mut service, 100.0, 100.0, 400.0, 400.0);
        drop(service.send(Event::MouseEnter));
        drop(service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::Y,
        }));
        // Pointer leaves the root mid-drag (suppressed while dragging).
        drop(service.send(Event::MouseLeave));

        let end = service.send(Event::ThumbDragEnd);

        assert!(end.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().scrollbar_x_visible);
        assert!(!service.context().scrollbar_y_visible);
    }

    #[test]
    fn hover_drag_ending_over_root_stays_visible() {
        let mut service = service(
            props()
                .orientation(ScrollOrientation::Both)
                .scrollbar_visibility(ScrollbarVisibility::Hover),
        );
        resize(&mut service, 100.0, 100.0, 400.0, 400.0);
        drop(service.send(Event::MouseEnter));
        drop(service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::Y,
        }));

        // Pointer still over the root when the drag ends.
        drop(service.send(Event::ThumbDragEnd));

        assert_eq!(service.state(), &State::Hovering);
        assert!(service.context().scrollbar_y_visible);
    }

    #[test]
    fn syncing_away_from_scroll_mode_cancels_stale_auto_hide() {
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Scroll));
        resize(&mut service, 100.0, 100.0, 100.0, 400.0);
        drop(service.send(Event::Scroll { x: 0.0, y: 150.0 }));
        assert_eq!(service.state(), &State::ScrollActive);

        // Switch to Always: the orphaned hide timer must be cancelled and the
        // machine must leave ScrollActive so a late HideTimeout cannot hide the
        // now-always-visible scrollbar.
        let result = service.set_props(
            props()
                .orientation(ScrollOrientation::Both)
                .scrollbar_visibility(ScrollbarVisibility::Always),
        );

        assert_eq!(service.state(), &State::Idle);
        assert!(result.cancel_effects.contains(&Effect::AutoHide));
        assert!(service.context().scrollbar_x_visible);
        assert!(service.context().scrollbar_y_visible);
    }

    #[test]
    fn resize_during_drag_keeps_scrollbar_visible() {
        // A ResizeObserver firing mid-drag must not hide the thumb being dragged.
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Scroll));
        resize(&mut service, 100.0, 100.0, 100.0, 400.0);
        drop(service.send(Event::Scroll { x: 0.0, y: 50.0 }));
        drop(service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::Y,
        }));
        assert!(service.context().scrollbar_y_visible);

        resize(&mut service, 100.0, 100.0, 100.0, 400.0);
        assert_eq!(service.state(), &State::ThumbDragging);
        assert!(service.context().scrollbar_y_visible);
    }

    #[test]
    fn mouse_enter_during_drag_preserves_drag_state() {
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Hover));
        resize(&mut service, 100.0, 100.0, 100.0, 400.0);
        drop(service.send(Event::MouseEnter));
        drop(service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::Y,
        }));
        drop(service.send(Event::MouseLeave));

        // Captured pointer re-enters the root mid-drag.
        let entered = service.send(Event::MouseEnter);
        assert!(!entered.state_changed);
        assert_eq!(service.state(), &State::ThumbDragging);
        assert!(service.context().hovering_root);

        drop(service.send(Event::ThumbDragMove { pos: 20.0 }));
        assert!(service.context().scroll_y > 0.0);
    }

    #[test]
    fn thumb_drag_start_cancels_running_auto_hide_timer() {
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Scroll));
        resize(&mut service, 100.0, 100.0, 100.0, 400.0);
        drop(service.send(Event::Scroll { x: 0.0, y: 50.0 }));

        let result = service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::Y,
        });

        assert_eq!(service.state(), &State::ThumbDragging);
        assert!(result.cancel_effects.contains(&Effect::AutoHide));
    }

    #[test]
    fn leaving_hover_mode_resets_hovering_state() {
        let mut service = service(props().scrollbar_visibility(ScrollbarVisibility::Hover));
        resize(&mut service, 100.0, 100.0, 100.0, 400.0);
        drop(service.send(Event::MouseEnter));
        assert_eq!(service.state(), &State::Hovering);

        // Switch to Auto: the stuck Hovering state must reset to Idle.
        drop(service.set_props(props().scrollbar_visibility(ScrollbarVisibility::Auto)));
        assert_eq!(service.state(), &State::Idle);
    }

    #[test]
    fn drag_end_outside_scroll_mode_returns_to_idle() {
        let mut service = service(props());
        resize(&mut service, 100.0, 100.0, 100.0, 400.0);
        drop(service.send(Event::ThumbDragStart {
            pos: 0.0,
            axis: Axis::Y,
        }));

        let result = service.send(Event::ThumbDragEnd);

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().drag_axis, None);
        assert!(result.pending_effects.is_empty());
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
    fn track_click_forward_clamps_to_nonnegative_max() {
        // Always mode renders a scrollbar even without overflow; a forward-page
        // click must clamp to 0, never a negative offset.
        let mut service = service(
            props()
                .orientation(ScrollOrientation::Both)
                .scrollbar_visibility(ScrollbarVisibility::Always),
        );
        resize(&mut service, 100.0, 100.0, 100.0, 100.0);
        assert!(service.context().scrollbar_y_visible);

        drop(service.send(Event::TrackClick {
            pos: 200.0,
            axis: Axis::Y,
        }));
        assert_eq!(service.context().scroll_y, 0.0);
    }

    #[test]
    fn scroll_event_clamps_out_of_range_offsets() {
        // Overscroll bounce / stale programmatic scroll can report offsets
        // outside 0..=max; they must be clamped before storage.
        let mut service = service(props());
        resize(&mut service, 100.0, 100.0, 100.0, 400.0);

        drop(service.send(Event::Scroll {
            x: -50.0,
            y: 9999.0,
        }));
        assert_eq!(service.context().scroll_x, 0.0);
        assert_eq!(service.context().scroll_y, 300.0); // content 400 - viewport 100
    }

    #[test]
    fn resize_clamps_stored_offset_to_new_bounds() {
        let mut service = service(props());
        resize(&mut service, 100.0, 100.0, 100.0, 400.0);
        drop(service.send(Event::Scroll { x: 0.0, y: 300.0 }));
        assert_eq!(service.context().scroll_y, 300.0);

        // Content shrinks: the old offset (300) now exceeds the new max (100).
        resize(&mut service, 100.0, 100.0, 100.0, 200.0);
        assert_eq!(service.context().scroll_y, 100.0);

        // Content no longer overflows: offset clamps to 0.
        resize(&mut service, 100.0, 100.0, 100.0, 80.0);
        assert_eq!(service.context().scroll_y, 0.0);
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
                .orientation(ScrollOrientation::Both)
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
                .orientation(ScrollOrientation::Both)
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

    #[test]
    fn rtl_scroll_normalization_is_re_exported_with_explicit_mode() {
        // The component re-exports the shared, convention-explicit helper rather
        // than a sign heuristic; `0` differs between the two RTL conventions.
        assert_eq!(
            normalize_scroll_left_rtl(0.0, 400.0, 100.0, RtlScrollMode::Negative),
            0.0
        );
        assert_eq!(
            normalize_scroll_left_rtl(0.0, 400.0, 100.0, RtlScrollMode::Positive),
            300.0
        );
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
            // `role="none"` is presentational: no ARIA attributes belong on it.
            assert!(attrs.get(&HtmlAttr::Aria(AriaAttr::Orientation)).is_none());
        }

        // The axis is conveyed via a data attribute, not ARIA.
        assert_eq!(
            api.scrollbar_y_attrs()
                .get(&HtmlAttr::Data("ars-orientation")),
            Some("vertical")
        );
        assert_eq!(
            api.scrollbar_x_attrs()
                .get(&HtmlAttr::Data("ars-orientation")),
            Some("horizontal")
        );
    }

    #[test]
    fn corner_square_visible_only_when_both_scrollbars_present() {
        let mut both = service(
            props()
                .orientation(ScrollOrientation::Both)
                .scrollbar_visibility(ScrollbarVisibility::Always),
        );
        resize(&mut both, 100.0, 100.0, 400.0, 400.0);
        assert_eq!(
            both.connect(&|_| {})
                .corner_square_attrs()
                .get(&HtmlAttr::Data("ars-visible")),
            Some("true")
        );

        // Vertical-only: corner filler must report hidden.
        let mut single = service(props().orientation(ScrollOrientation::Vertical));
        resize(&mut single, 100.0, 100.0, 400.0, 400.0);
        assert_eq!(
            single
                .connect(&|_| {})
                .corner_square_attrs()
                .get(&HtmlAttr::Data("ars-visible")),
            Some("false")
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

    // --- prop synchronization ----------------------------------------------

    #[test]
    fn on_props_changed_emits_sync_only_for_prop_backed_changes() {
        let base = props();

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(
                &base,
                &props().orientation(ScrollOrientation::Both)
            )
            .as_slice(),
            [Event::SyncProps]
        );
        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(&base, &props().dir(Direction::Rtl))
                .as_slice(),
            [Event::SyncProps]
        );
        assert!(
            <Machine as ars_core::Machine>::on_props_changed(&base, &props().aria_label("Log"))
                .is_empty(),
            "aria_label is read live from props, not synced into context",
        );
        assert!(<Machine as ars_core::Machine>::on_props_changed(&base, &base).is_empty());
    }

    #[test]
    fn set_props_syncs_prop_backed_context_fields() {
        let mut service = service(props());

        drop(
            service.set_props(
                props()
                    .orientation(ScrollOrientation::Both)
                    .scrollbar_visibility(ScrollbarVisibility::Always)
                    .min_thumb_size(50.0)
                    .hide_delay(Duration::from_millis(900))
                    .scrollbar_cross_size(10.0)
                    .dir(Direction::Rtl),
            ),
        );

        let ctx = service.context();
        assert_eq!(ctx.orientation, ScrollOrientation::Both);
        assert_eq!(ctx.scrollbar_visibility, ScrollbarVisibility::Always);
        assert_eq!(ctx.min_thumb_size, 50.0);
        assert_eq!(ctx.hide_delay, Duration::from_millis(900));
        assert_eq!(ctx.scrollbar_cross_size, 10.0);
        assert_eq!(ctx.dir, Direction::Rtl);
        // Visibility recomputed against the new mode/orientation.
        assert!(ctx.scrollbar_x_visible);
        assert!(ctx.scrollbar_y_visible);
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
        let mut both = service(
            props()
                .orientation(ScrollOrientation::Both)
                .scrollbar_visibility(ScrollbarVisibility::Always),
        );

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
