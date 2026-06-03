//! `SignaturePad` component state machine and connect API.
//!
//! The `SignaturePad` captures freehand pointer/touch input as vector path data.
//! It records strokes as point arrays (with pressure and timestamp), supports
//! undo and clear, and exposes the accumulated [`SignatureData`] for form
//! submission (as an SVG path) and export.
//!
//! The agnostic core owns the stroke data model, the drawing state machine,
//! `min_distance` point culling, stroke validation, and the ARIA/`data-ars-*`
//! attribute surface. It does **not** touch the live `<canvas>`: pointer
//! capture, device-pixel-ratio scaling, bounding-rect measurement, and raster
//! rasterization (PNG/JPEG) belong to the framework adapters.
//!
//! Like the other component machines, the core only emits typed [`Effect`]
//! intents; the adapter fulfils them. [`Effect::DrawingListeners`] tells the
//! adapter to attach global pointer-drag listeners (e.g. via
//! `PlatformEffects::track_pointer_drag`) that dispatch [`Event::DrawMove`] and
//! [`Event::DrawEnd`]; [`Effect::AnnounceProvided`] and
//! [`Effect::AnnounceCleared`] tell the adapter to announce the corresponding
//! message into a polite `aria-live` region.

use alloc::{string::String, vec, vec::Vec};
use core::fmt::{self, Debug, Write as _};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, Env, HasId, HtmlAttr, Locale, MessageFn, PendingEffect, RasterError, RasterImage,
    RasterPoint, RasterSpec, SignatureRasterizer, TransitionPlan, no_cleanup,
};

/// A single point in a signature stroke.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SignaturePoint {
    /// The x coordinate of the point.
    pub x: f64,

    /// The y coordinate of the point.
    pub y: f64,

    /// Pressure from 0.0 to 1.0 (for pressure-sensitive input).
    pub pressure: f64,

    /// Timestamp in milliseconds.
    pub timestamp: f64,
}

/// A continuous stroke (pen-down to pen-up).
#[derive(Clone, Debug, PartialEq)]
pub struct SignatureStroke {
    /// The points in the stroke.
    pub points: Vec<SignaturePoint>,
}

/// The complete signature data.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct SignatureData {
    /// The strokes in the signature.
    pub strokes: Vec<SignatureStroke>,
}

impl SignatureData {
    /// Check if the signature data is empty, i.e. it has no recorded points.
    ///
    /// Emptiness is point-based, not stroke-count-based: externally supplied or
    /// deserialized data containing only empty strokes (no points) is treated as
    /// blank, matching [`to_svg_path`](Self::to_svg_path) and
    /// [`point_count`](Self::point_count), so it never looks like a real
    /// signature (guide stays visible, clear/undo stay disabled, init stays
    /// [`Idle`](State::Idle)).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.strokes.iter().all(|stroke| stroke.points.is_empty())
    }

    /// Convert to SVG path data string.
    ///
    /// Each stroke contributes a `moveto` to its first point followed by a
    /// `lineto` to each subsequent point. Strokes are concatenated directly,
    /// so the resulting path describes the whole signature as one `d` value.
    #[must_use]
    pub fn to_svg_path(&self) -> String {
        let mut path = String::new();

        for stroke in &self.strokes {
            let Some(first) = stroke.points.first() else {
                continue;
            };

            let _ = write!(path, "M{:.1},{:.1}", first.x, first.y);

            for point in &stroke.points[1..] {
                let _ = write!(path, " L{:.1},{:.1}", point.x, point.y);
            }
        }
        path
    }

    /// Total number of points across all strokes.
    #[must_use]
    pub fn point_count(&self) -> usize {
        self.strokes.iter().map(|stroke| stroke.points.len()).sum()
    }

    /// Export the signature in one of the resolution-independent formats the
    /// agnostic core can produce.
    ///
    /// For [`SignatureFormat::Svg`] the core generates the markup from stroke
    /// data; for [`SignatureFormat::Points`] the raw [`SignaturePoint`] vectors
    /// are returned directly, enabling server-side vector reconstruction
    /// regardless of display resolution.
    ///
    /// Raster formats (PNG/JPEG/WebP) require a pixel surface, so they are
    /// produced by [`SignatureData::export_raster`] via an injected
    /// [`SignatureRasterizer`] rather than by this method.
    #[must_use]
    pub fn export(&self, format: SignatureFormat) -> SignatureExport {
        match format {
            SignatureFormat::Svg => SignatureExport::Svg(self.to_svg_path()),

            SignatureFormat::Points => SignatureExport::Points(
                self.strokes
                    .iter()
                    .map(|stroke| stroke.points.clone())
                    .collect(),
            ),
        }
    }

    /// Rasterize the signature into an encoded image (PNG/JPEG) via an injected
    /// [`SignatureRasterizer`].
    ///
    /// Raster output needs a pixel surface, which the agnostic core does not
    /// have — so the caller supplies the platform rasterizer (`ars-dom`'s
    /// `WebSignatureRasterizer` in the browser). The per-point
    /// [`pressure`](SignaturePoint::pressure) is forwarded to the rasterizer so
    /// firmer presses render as thicker strokes. Requiring the rasterizer as an
    /// argument keeps raster export impossible to call without a backend
    /// (make-invalid-states-unrepresentable) instead of panicking.
    ///
    /// # Errors
    ///
    /// Propagates the rasterizer's [`RasterError`] — notably
    /// [`RasterError::Unsupported`] under SSR/tests.
    pub fn export_raster(
        &self,
        rasterizer: &dyn SignatureRasterizer,
        spec: &RasterSpec,
    ) -> Result<RasterImage, RasterError> {
        let strokes = self
            .strokes
            .iter()
            .map(|stroke| {
                stroke
                    .points
                    .iter()
                    .map(|&SignaturePoint { x, y, pressure, .. }| RasterPoint { x, y, pressure })
                    .collect()
            })
            .collect::<Vec<_>>();

        rasterizer.rasterize(&strokes, spec)
    }
}

/// Resolution-independent export formats the agnostic core can produce.
///
/// Raster formats (PNG/JPEG/WebP) need a pixel surface and are produced by
/// [`SignatureData::export_raster`] via an injected [`SignatureRasterizer`], not
/// here — keeping this type free of variants the core cannot fulfil
/// (make-invalid-states-unrepresentable, so [`SignatureData::export`] is total
/// with no unreachable arm).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureFormat {
    /// SVG path markup.
    Svg,

    /// Raw point data.
    Points,
}

/// Exported signature data in one of the [`SignatureFormat`] variants.
///
/// Mirrors [`SignatureFormat`]: every variant here is something the core can
/// actually return. Adapters define their own export type for raster output.
#[derive(Clone, Debug, PartialEq)]
pub enum SignatureExport {
    /// SVG markup string.
    Svg(String),

    /// Raw point data for vector reconstruction.
    /// Each inner `Vec` is one continuous stroke.
    Points(Vec<Vec<SignaturePoint>>),
}

/// The states for the `SignaturePad` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// No strokes, canvas is blank.
    Idle,

    /// The user is actively drawing (pointer/touch down).
    Drawing,

    /// At least one stroke has been completed.
    Completed,
}

/// The events for the `SignaturePad` component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Pointer/touch down on the canvas.
    DrawStart {
        /// The x coordinate of the draw start.
        x: f64,

        /// The y coordinate of the draw start.
        y: f64,

        /// The pressure of the draw start.
        pressure: f64,
    },

    /// Pointer/touch move while drawing.
    DrawMove {
        /// The x coordinate of the draw move.
        x: f64,

        /// The y coordinate of the draw move.
        y: f64,

        /// The pressure of the draw move.
        pressure: f64,
    },

    /// Pointer/touch up, ending a stroke.
    DrawEnd,

    /// Undo the last stroke.
    Undo,

    /// Clear all strokes.
    Clear,

    /// Focus entered the canvas.
    Focus,

    /// Focus left the canvas.
    Blur,

    /// The controlled [`Props::data`] changed; re-sync the bound signature data.
    /// Dispatched by [`Machine::on_props_changed`](ars_core::Machine::on_props_changed),
    /// not by user interaction, and processed regardless of disabled/read-only.
    SyncData,

    /// A configuration prop (disabled, read-only, pen color/width, min distance)
    /// changed; mirror the new values into the context. Dispatched by
    /// [`Machine::on_props_changed`](ars_core::Machine::on_props_changed), and
    /// processed regardless of disabled/read-only so the pad can be re-enabled.
    SyncProps,
}

/// The context for the `SignaturePad` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The accumulated signature data.
    pub data: Bindable<SignatureData>,

    /// The stroke currently being drawn (None if not drawing).
    pub current_stroke: Option<SignatureStroke>,

    /// Whether the pad is disabled.
    pub disabled: bool,

    /// Whether the pad is read-only (shows existing signature but cannot modify).
    pub readonly: bool,

    /// Stroke color (CSS color string).
    pub pen_color: String,

    /// Stroke width in pixels.
    pub pen_width: f64,

    /// Minimum distance between points to record (for performance).
    pub min_distance: f64,

    /// Whether the canvas has focus.
    pub focused: bool,

    /// Locale for internationalized messages.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component instance IDs.
    pub ids: ComponentIds,
}

/// The props for the `SignaturePad` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled signature data.
    pub data: Option<SignatureData>,

    /// Default data for uncontrolled mode.
    pub default_data: SignatureData,

    /// Disabled state.
    pub disabled: bool,

    /// Read-only state.
    pub readonly: bool,

    /// Pen color.
    pub pen_color: String,

    /// Pen width in pixels.
    pub pen_width: f64,

    /// Minimum distance between recorded points.
    pub min_distance: f64,

    /// Name for form submission.
    pub name: Option<String>,

    /// Fired when the signature data changes through user interaction (a
    /// committed stroke, undo, or clear), carrying the new data. Required for
    /// controlled [`data`](Self::data): the parent updates its controlled value
    /// from this callback, then feeds it back via props (triggering
    /// [`Event::SyncData`]). Not fired for parent-driven syncs.
    pub on_data_change: Option<Callback<dyn Fn(SignatureData) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            data: None,
            default_data: SignatureData::default(),
            disabled: false,
            readonly: false,
            pen_color: "#000000".into(),
            pen_width: 2.0,
            min_distance: 3.0,
            name: None,
            on_data_change: None,
        }
    }
}

impl Props {
    /// Returns a fresh [`Props`] with every field at its [`Default`] value.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id).
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets the controlled [`data`](Self::data).
    #[must_use]
    pub fn data(mut self, data: SignatureData) -> Self {
        self.data = Some(data);
        self
    }

    /// Sets [`default_data`](Self::default_data) for uncontrolled mode.
    #[must_use]
    pub fn default_data(mut self, default_data: SignatureData) -> Self {
        self.default_data = default_data;
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets [`readonly`](Self::readonly).
    #[must_use]
    pub const fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    /// Sets [`pen_color`](Self::pen_color).
    #[must_use]
    pub fn pen_color(mut self, pen_color: impl Into<String>) -> Self {
        self.pen_color = pen_color.into();
        self
    }

    /// Sets [`pen_width`](Self::pen_width).
    #[must_use]
    pub const fn pen_width(mut self, pen_width: f64) -> Self {
        self.pen_width = pen_width;
        self
    }

    /// Sets [`min_distance`](Self::min_distance).
    #[must_use]
    pub const fn min_distance(mut self, min_distance: f64) -> Self {
        self.min_distance = min_distance;
        self
    }

    /// Sets [`name`](Self::name) for form submission.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets [`on_data_change`](Self::on_data_change).
    #[must_use]
    pub fn on_data_change(
        mut self,
        callback: Callback<dyn Fn(SignatureData) + Send + Sync>,
    ) -> Self {
        self.on_data_change = Some(callback);
        self
    }
}

/// The messages for the `SignaturePad` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the canvas.
    pub canvas_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the clear button.
    pub clear_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the undo button.
    pub undo_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Screen reader announcement when signature is provided.
    pub signature_provided: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Screen reader announcement when signature is cleared.
    pub signature_cleared: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Guide placeholder text.
    pub guide_text: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            canvas_label: MessageFn::static_str("Signature pad"),
            clear_label: MessageFn::static_str("Clear signature"),
            undo_label: MessageFn::static_str("Undo last stroke"),
            signature_provided: MessageFn::static_str("Signature provided"),
            signature_cleared: MessageFn::static_str("Signature cleared"),
            guide_text: MessageFn::static_str("Sign here"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed effect intents emitted by the signature-pad machine.
///
/// The agnostic core never touches the live canvas or the screen reader; it
/// emits these markers and the framework adapter performs the real work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter attaches global pointer-drag listeners (e.g. via
    /// `PlatformEffects::track_pointer_drag`) that dispatch [`Event::DrawMove`]
    /// while the pointer moves and [`Event::DrawEnd`] on pointer up, so a stroke
    /// continues even when the pointer leaves the canvas. Emitted on the
    /// transition into [`State::Drawing`].
    DrawingListeners,

    /// Adapter announces [`Messages::signature_provided`] into a polite
    /// `aria-live` region. Emitted on the transition into [`State::Completed`]
    /// (pointer up after a valid stroke).
    AnnounceProvided,

    /// Adapter announces [`Messages::signature_cleared`] into a polite
    /// `aria-live` region. Emitted on the [`Event::Clear`] transition.
    AnnounceCleared,

    /// The signature data changed through user interaction (a committed stroke,
    /// undo, or clear). Fires [`Props::on_data_change`] with the new data so a
    /// parent holding controlled [`Props::data`] can update it — without this,
    /// controlled mode would never observe the change. Not emitted for
    /// [`Event::SyncData`] (that change originates from the parent).
    DataChange,
}

/// The machine for the `SignaturePad` component.
///
/// # Examples
///
/// Draw a single stroke and complete it. In a real app the adapter dispatches
/// [`Event::DrawMove`]/[`Event::DrawEnd`] from the pointer-drag listeners that
/// [`Effect::DrawingListeners`] sets up; here we send them directly:
///
/// ```
/// use ars_components::specialized::signature_pad::{Event, Machine, Messages, Props, State};
/// use ars_core::{Env, Service};
///
/// let mut pad = Service::<Machine>::new(
///     Props::new().id("sig").min_distance(0.0),
///     &Env::default(),
///     &Messages::default(),
/// );
/// assert_eq!(pad.state(), &State::Idle);
///
/// drop(pad.send(Event::DrawStart { x: 0.0, y: 0.0, pressure: 0.5 }));
/// assert_eq!(pad.state(), &State::Drawing);
///
/// drop(pad.send(Event::DrawMove { x: 10.0, y: 10.0, pressure: 0.5 }));
/// drop(pad.send(Event::DrawEnd));
/// assert_eq!(pad.state(), &State::Completed);
/// assert_eq!(pad.context().data.get().strokes.len(), 1);
/// ```
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

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (State, Context) {
        let data = if let Some(data) = &props.data {
            Bindable::controlled(data.clone())
        } else {
            Bindable::uncontrolled(props.default_data.clone())
        };

        let state = if data.get().is_empty() {
            State::Idle
        } else {
            State::Completed
        };

        (
            state,
            Context {
                data,
                current_stroke: None,
                disabled: props.disabled,
                readonly: props.readonly,
                pen_color: props.pen_color.clone(),
                pen_width: props.pen_width,
                min_distance: props.min_distance,
                focused: false,
                locale: env.locale.clone(),
                messages: messages.clone(),
                ids: ComponentIds::from_id(&props.id),
            },
        )
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        assert_eq!(
            old.id, new.id,
            "signature_pad::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.data != new.data {
            events.push(Event::SyncData);
        }

        if old.disabled != new.disabled
            || old.readonly != new.readonly
            || old.pen_color != new.pen_color
            || old.pen_width != new.pen_width
            || old.min_distance != new.min_distance
        {
            events.push(Event::SyncProps);
        }

        events
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // Focus/Blur and the prop-sync events always pass through, regardless of
        // disabled/read-only (prop sync must be able to re-enable the pad).
        match event {
            Event::Focus => {
                return Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focused = true;
                }));
            }

            Event::Blur => {
                return Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focused = false;
                }));
            }

            Event::SyncProps => {
                let disabled = props.disabled;
                let readonly = props.readonly;
                let pen_color = props.pen_color.clone();
                let pen_width = props.pen_width;
                let min_distance = props.min_distance;

                // Becoming non-interactive mid-stroke must not strand the machine
                // in `Drawing` — the disabled/read-only gate below would then
                // reject the trailing `DrawEnd`/`Clear`, leaving an in-flight
                // stroke that could later commit stale. Cancel it and leave
                // `Drawing` (blank canvas -> `Idle`, otherwise `Completed`).
                let cancel_drawing = (disabled || readonly) && matches!(state, State::Drawing);
                let target = if cancel_drawing {
                    if ctx.data.pending().is_empty() {
                        State::Idle
                    } else {
                        State::Completed
                    }
                } else {
                    *state
                };

                return Some(TransitionPlan::to(target).apply(move |ctx: &mut Context| {
                    ctx.disabled = disabled;
                    ctx.readonly = readonly;
                    ctx.pen_color = pen_color;
                    ctx.pen_width = pen_width;
                    ctx.min_distance = min_distance;
                    if cancel_drawing {
                        ctx.current_stroke = None;
                    }
                }));
            }

            Event::SyncData => {
                let new_data = props.data.clone();
                // Emptiness after the sync: the new controlled value, or — when
                // dropping to uncontrolled — the retained local value.

                let empty_after = if let Some(data) = &new_data {
                    data.is_empty()
                } else {
                    ctx.data.pending().is_empty()
                };

                // Reconcile the displayed state with the synced data, but never
                // reorder an in-flight stroke.
                let plan = if matches!(state, State::Drawing) {
                    TransitionPlan::new()
                } else {
                    TransitionPlan::to(if empty_after {
                        State::Idle
                    } else {
                        State::Completed
                    })
                };

                return Some(plan.apply(move |ctx: &mut Context| {
                    if let Some(data) = new_data {
                        ctx.data.set(data.clone());
                        ctx.data.sync_controlled(Some(data));
                    } else {
                        ctx.data.sync_controlled(None);
                    }
                }));
            }

            _ => {}
        }

        if ctx.disabled || ctx.readonly {
            return None;
        }

        match (state, event) {
            // Start drawing: seed a new stroke and ask the adapter to wire up
            // the global pointer-drag listeners that feed DrawMove/DrawEnd.
            (State::Idle | State::Completed, Event::DrawStart { x, y, pressure }) => {
                let point = SignaturePoint {
                    x: *x,
                    y: *y,
                    pressure: *pressure,
                    timestamp: 0.0,
                };

                Some(
                    TransitionPlan::to(State::Drawing)
                        .apply(move |ctx: &mut Context| {
                            ctx.current_stroke = Some(SignatureStroke {
                                points: vec![point],
                            });
                        })
                        .with_effect(PendingEffect::named(Effect::DrawingListeners)),
                )
            }

            // Continue drawing: append the point only when it is at least
            // `min_distance` away from the previous one (cull jitter).
            (State::Drawing, Event::DrawMove { x, y, pressure }) => {
                let point = SignaturePoint {
                    x: *x,
                    y: *y,
                    pressure: *pressure,
                    timestamp: 0.0,
                };

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let min_distance = ctx.min_distance;

                    if let Some(stroke) = ctx.current_stroke.as_mut()
                        && let Some(last) = stroke.points.last()
                    {
                        let dx = point.x - last.x;
                        let dy = point.y - last.y;

                        if (dx * dx + dy * dy).sqrt() >= min_distance {
                            stroke.points.push(point);
                        }
                    }
                }))
            }

            // End drawing: commit the in-flight stroke if it has enough points.
            // A stroke that is too short to commit (a stray tap) leaves the
            // signature unchanged, so the pad returns to `Idle` when the canvas
            // is still blank and only announces when a stroke is actually added.
            (State::Drawing, Event::DrawEnd) => {
                let commits = ctx
                    .current_stroke
                    .as_ref()
                    .is_some_and(|stroke| stroke.points.len() >= 2);

                // Build on the *pending* (internal) data, not `get()`: in
                // controlled mode `get()` returns the stale parent value until it
                // round-trips through `on_data_change`, so reading it would drop
                // earlier un-synced strokes when several are drawn in quick
                // succession.
                let target = if commits || !ctx.data.pending().is_empty() {
                    State::Completed
                } else {
                    State::Idle
                };

                let mut plan = TransitionPlan::to(target).apply(|ctx: &mut Context| {
                    if let Some(stroke) = ctx.current_stroke.take()
                        && stroke.points.len() >= 2
                    {
                        let mut data = ctx.data.pending().clone();

                        data.strokes.push(stroke);

                        ctx.data.set(data);
                    }
                });

                if commits {
                    plan = plan
                        .with_effect(PendingEffect::named(Effect::AnnounceProvided))
                        .with_effect(data_change_effect());
                }

                Some(plan)
            }

            // Undo the last completed stroke. The post-pop emptiness — not the
            // stroke count — decides the target, so data carrying empty-stroke
            // entries can never strand the pad in `Completed` while it is blank.
            // Built on the pending working copy to preserve un-synced controlled
            // edits.
            (State::Completed, Event::Undo) => {
                let mut data = ctx.data.pending().clone();
                data.strokes.pop();

                let target = if data.is_empty() {
                    State::Idle
                } else {
                    State::Completed
                };

                Some(
                    TransitionPlan::to(target)
                        .apply(move |ctx: &mut Context| {
                            ctx.data.set(data);
                        })
                        .with_effect(data_change_effect()),
                )
            }

            // Clear all strokes from any state.
            (_, Event::Clear) => {
                // Notify the parent only when there was something to clear, so a
                // clear on an already-blank pad does not emit a spurious change.
                let had_data = !ctx.data.pending().is_empty();

                let mut plan = TransitionPlan::to(State::Idle)
                    .apply(|ctx: &mut Context| {
                        ctx.data.set(SignatureData::default());
                        ctx.current_stroke = None;
                    })
                    .with_effect(PendingEffect::named(Effect::AnnounceCleared));

                if had_data {
                    plan = plan.with_effect(data_change_effect());
                }

                Some(plan)
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
        Api {
            state,
            ctx,
            props,
            send,
        }
    }
}

/// Builds the [`Effect::DataChange`] effect that notifies
/// [`Props::on_data_change`] with the new signature data.
///
/// Reads the bound value's *pending* (internal) data, which is the value just
/// committed by the transition — in controlled mode `get()` would still return
/// the stale parent-owned value until it round-trips back through props.
fn data_change_effect() -> PendingEffect<Machine> {
    PendingEffect::new(Effect::DataChange, |ctx: &Context, props: &Props, _send| {
        if let Some(callback) = &props.on_data_change {
            callback(ctx.data.pending().clone());
        }

        no_cleanup()
    })
}

/// DOM parts of the `SignaturePad` component.
#[derive(ComponentPart)]
#[scope = "signature-pad"]
pub enum Part {
    /// Root wrapper element.
    Root,

    /// The `<canvas>` drawing surface.
    Canvas,

    /// Button that clears all strokes.
    ClearTrigger,

    /// Button that undoes the last stroke.
    UndoTrigger,

    /// Label describing the signature area.
    Label,

    /// Guide line or placeholder text (hidden once strokes exist).
    Guide,

    /// Hidden input carrying the SVG path data for form submission.
    HiddenInput,
}

/// API for the `SignaturePad` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("signature_pad::Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Whether the user is actively drawing a stroke.
    #[must_use]
    pub const fn is_drawing(&self) -> bool {
        matches!(self.state, State::Drawing)
    }

    /// Whether the signature is empty (no committed strokes).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ctx.data.get().is_empty()
    }

    /// The accumulated signature data.
    #[must_use]
    pub fn data(&self) -> &SignatureData {
        self.ctx.data.get()
    }

    /// Exports the current signature in a resolution-independent format.
    ///
    /// Convenience delegate to [`SignatureData::export`].
    #[must_use]
    pub fn export(&self, format: SignatureFormat) -> SignatureExport {
        self.ctx.data.get().export(format)
    }

    /// Rasterizes the current signature into an encoded image via an injected
    /// [`SignatureRasterizer`].
    ///
    /// Convenience delegate to [`SignatureData::export_raster`].
    ///
    /// # Errors
    ///
    /// Propagates the rasterizer's [`RasterError`].
    pub fn export_raster(
        &self,
        rasterizer: &dyn SignatureRasterizer,
        spec: &RasterSpec,
    ) -> Result<RasterImage, RasterError> {
        self.ctx.data.get().export_raster(rasterizer, spec)
    }

    /// Root element attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), self.state_token());

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        attrs
    }

    /// Canvas element attributes.
    #[must_use]
    pub fn canvas_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Canvas.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("canvas"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::TabIndex, "0")
            .set(HtmlAttr::Class, "ars-touch-none");

        // `application` opts the canvas out of the screen-reader virtual cursor
        // so it can capture raw pointer/touch input; a non-interactive pad is a
        // static image instead.
        if self.ctx.readonly || self.ctx.disabled {
            attrs.set(HtmlAttr::Role, "img");
        } else {
            attrs.set(HtmlAttr::Role, "application");
        }

        attrs
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.canvas_label)(&self.ctx.locale),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }

        if self.ctx.focused {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs
    }

    /// Clear-trigger button attributes.
    #[must_use]
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ClearTrigger.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("clear"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.clear_label)(&self.ctx.locale),
            );

        if self.is_empty() || self.ctx.disabled || self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Undo-trigger button attributes.
    #[must_use]
    pub fn undo_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::UndoTrigger.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("undo"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.undo_label)(&self.ctx.locale),
            );

        if self.is_empty() || self.ctx.disabled || self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Label element attributes.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("label"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        attrs
    }

    /// Guide element attributes. The guide is hidden once strokes are present.
    #[must_use]
    pub fn guide_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Guide.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if !self.is_empty() {
            attrs.set_bool(HtmlAttr::Data("ars-hidden"), true);
        }

        attrs
    }

    /// Returns the guide placeholder text (e.g. "Sign here").
    ///
    /// The adapter renders this inside the [`Part::Guide`] element.
    #[must_use]
    pub fn guide_text(&self) -> String {
        (self.ctx.messages.guide_text)(&self.ctx.locale)
    }

    /// Hidden-input attributes. The value is the signature's SVG path data,
    /// suitable for form submission.
    #[must_use]
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "hidden");

        if let Some(name) = &self.props.name {
            attrs.set(HtmlAttr::Name, name.clone());
        }

        attrs.set(HtmlAttr::Value, self.ctx.data.get().to_svg_path());

        // A disabled control is excluded from form submission; mirror that on
        // the hidden input so a disabled pad cannot submit stale signature data.
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Dispatches pointer-down intent on the canvas, beginning a stroke.
    pub fn on_canvas_pointer_down(&self, x: f64, y: f64, pressure: f64) {
        (self.send)(Event::DrawStart { x, y, pressure });
    }

    /// Dispatches clear intent.
    pub fn on_clear(&self) {
        (self.send)(Event::Clear);
    }

    /// Dispatches undo intent.
    pub fn on_undo(&self) {
        (self.send)(Event::Undo);
    }

    const fn state_token(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::Drawing => "drawing",
            State::Completed => "completed",
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Canvas => self.canvas_attrs(),
            Part::ClearTrigger => self.clear_trigger_attrs(),
            Part::UndoTrigger => self.undo_trigger_attrs(),
            Part::Label => self.label_attrs(),
            Part::Guide => self.guide_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, vec, vec::Vec};

    use ars_core::{AttrMap, Env, Machine as _, NullSignatureRasterizer, RasterFormat, Service};
    use insta::assert_snapshot;

    use super::*;

    // ───────────────────────── helpers ─────────────────────────

    fn point(x: f64, y: f64) -> SignaturePoint {
        SignaturePoint {
            x,
            y,
            pressure: 0.5,
            timestamp: 0.0,
        }
    }

    fn stroke(points: &[(f64, f64)]) -> SignatureStroke {
        SignatureStroke {
            points: points.iter().map(|&(x, y)| point(x, y)).collect(),
        }
    }

    /// Signature data with `n` two-point strokes.
    fn data_with(n: usize) -> SignatureData {
        SignatureData {
            strokes: (0..n)
                .map(|i| {
                    let base = i as f64;

                    stroke(&[(base, base), (base + 1.0, base + 1.0)])
                })
                .collect(),
        }
    }

    fn test_props() -> Props {
        Props::new().id("sig")
    }

    fn fresh_service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn effect_names(result: &ars_core::SendResult<Machine>) -> Vec<Effect> {
        result
            .pending_effects
            .iter()
            .map(|effect| effect.name)
            .collect()
    }

    /// Leaks a fully-formed [`Api`] for attribute/accessor assertions.
    fn api_for(
        state: State,
        data: SignatureData,
        disabled: bool,
        readonly: bool,
        focused: bool,
    ) -> Api<'static> {
        let props = Box::leak(Box::new(
            Props::new()
                .id("sig")
                .disabled(disabled)
                .readonly(readonly)
                .name("signature"),
        ));

        let messages = Messages::default();

        let (_, mut ctx) = Machine::init(props, &Env::default(), &messages);

        ctx.data.set(data);
        ctx.focused = focused;

        let ctx = Box::leak(Box::new(ctx));

        let state = Box::leak(Box::new(state));

        let send = Box::leak(Box::new(|_: Event| {}));

        Api {
            state,
            ctx,
            props,
            send,
        }
    }

    fn empty_api(state: State) -> Api<'static> {
        api_for(state, SignatureData::default(), false, false, false)
    }

    fn filled_api(state: State) -> Api<'static> {
        api_for(state, data_with(1), false, false, false)
    }

    // ───────────────────────── data model ─────────────────────────

    #[test]
    fn signature_data_default_is_empty() {
        let data = SignatureData::default();

        assert!(data.is_empty());
        assert_eq!(data.point_count(), 0);
        assert_eq!(data.to_svg_path(), "");
    }

    #[test]
    fn signature_data_with_only_empty_strokes_is_empty() {
        // Strokes that carry no points are blank even though the stroke vec is
        // non-empty — emptiness is point-based, consistent with to_svg_path.
        let data = SignatureData {
            strokes: vec![
                SignatureStroke { points: Vec::new() },
                SignatureStroke { points: Vec::new() },
            ],
        };

        assert!(data.is_empty());
        assert_eq!(data.point_count(), 0);
        assert_eq!(data.to_svg_path(), "");
    }

    #[test]
    fn init_with_only_empty_strokes_starts_idle() {
        let service = fresh_service(test_props().default_data(SignatureData {
            strokes: vec![SignatureStroke { points: Vec::new() }],
        }));

        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().data.get().is_empty());
    }

    #[test]
    fn signature_data_to_svg_path_emits_moveto_lineto() {
        let data = SignatureData {
            strokes: vec![stroke(&[(0.0, 0.0), (1.5, 2.25), (3.0, 4.0)])],
        };

        assert_eq!(data.to_svg_path(), "M0.0,0.0 L1.5,2.2 L3.0,4.0");
    }

    #[test]
    fn signature_data_to_svg_path_concatenates_strokes() {
        let data = SignatureData {
            strokes: vec![
                stroke(&[(0.0, 0.0), (1.0, 1.0)]),
                stroke(&[(5.0, 5.0), (6.0, 6.0)]),
            ],
        };

        assert_eq!(data.to_svg_path(), "M0.0,0.0 L1.0,1.0M5.0,5.0 L6.0,6.0");
    }

    #[test]
    fn signature_data_to_svg_path_skips_empty_strokes() {
        let data = SignatureData {
            strokes: vec![
                SignatureStroke { points: Vec::new() },
                stroke(&[(2.0, 2.0), (3.0, 3.0)]),
            ],
        };

        assert_eq!(data.to_svg_path(), "M2.0,2.0 L3.0,3.0");
    }

    #[test]
    fn signature_data_point_count_sums_all_strokes() {
        let data = SignatureData {
            strokes: vec![stroke(&[(0.0, 0.0), (1.0, 1.0)]), stroke(&[(2.0, 2.0)])],
        };

        assert_eq!(data.point_count(), 3);
    }

    /// Records the strokes and spec it was handed and returns canned bytes, so
    /// tests can assert `export_raster` forwards pressure-weighted points.
    struct StubRasterizer {
        seen: std::sync::Mutex<Option<(Vec<Vec<RasterPoint>>, RasterSpec)>>,
    }

    impl StubRasterizer {
        fn new() -> Self {
            Self {
                seen: std::sync::Mutex::new(None),
            }
        }
    }

    impl SignatureRasterizer for StubRasterizer {
        fn rasterize(
            &self,
            strokes: &[Vec<RasterPoint>],
            spec: &RasterSpec,
        ) -> Result<RasterImage, RasterError> {
            *self.seen.lock().unwrap() = Some((strokes.to_vec(), spec.clone()));

            Ok(RasterImage {
                format: spec.format,
                bytes: vec![strokes.len() as u8],
            })
        }
    }

    #[test]
    fn export_raster_forwards_pressure_weighted_points_to_backend() {
        let data = SignatureData {
            strokes: vec![SignatureStroke {
                points: vec![
                    SignaturePoint {
                        x: 1.0,
                        y: 2.0,
                        pressure: 0.25,
                        timestamp: 0.0,
                    },
                    SignaturePoint {
                        x: 3.0,
                        y: 4.0,
                        pressure: 0.75,
                        timestamp: 0.0,
                    },
                ],
            }],
        };

        let rasterizer = StubRasterizer::new();

        let spec = RasterSpec::new(120, 60).format(RasterFormat::Png);

        let image = data
            .export_raster(&rasterizer, &spec)
            .expect("stub rasterizer succeeds");

        assert_eq!(image.format, RasterFormat::Png);
        assert_eq!(image.bytes, vec![1]);

        let (seen_strokes, seen_spec) = rasterizer.seen.lock().unwrap().clone().unwrap();

        assert_eq!(seen_strokes.len(), 1);
        assert_eq!(
            seen_strokes[0],
            vec![
                RasterPoint {
                    x: 1.0,
                    y: 2.0,
                    pressure: 0.25,
                },
                RasterPoint {
                    x: 3.0,
                    y: 4.0,
                    pressure: 0.75,
                },
            ]
        );
        assert_eq!(seen_spec, spec);
    }

    #[test]
    fn export_raster_propagates_backend_unsupported() {
        let result = data_with(1).export_raster(&NullSignatureRasterizer, &RasterSpec::new(10, 10));

        assert_eq!(result, Err(RasterError::Unsupported));
    }

    #[test]
    fn api_export_delegates_to_data() {
        let api = filled_api(State::Completed);

        assert_eq!(
            api.export(SignatureFormat::Svg),
            api.data().export(SignatureFormat::Svg)
        );
        assert_eq!(
            api.export_raster(&NullSignatureRasterizer, &RasterSpec::new(10, 10)),
            Err(RasterError::Unsupported)
        );
    }

    #[test]
    fn export_svg_matches_to_svg_path() {
        let data = data_with(1);

        assert_eq!(
            data.export(SignatureFormat::Svg),
            SignatureExport::Svg(data.to_svg_path())
        );
    }

    #[test]
    fn export_points_returns_per_stroke_vectors() {
        let data = data_with(2);

        assert_eq!(
            data.export(SignatureFormat::Points),
            SignatureExport::Points(vec![
                data.strokes[0].points.clone(),
                data.strokes[1].points.clone(),
            ])
        );
    }

    // ───────────────────────── props / init ─────────────────────────

    #[test]
    fn props_default_matches_spec() {
        let props = Props::default();

        assert_eq!(props.id, "");
        assert_eq!(props.data, None);
        assert_eq!(props.default_data, SignatureData::default());
        assert!(!props.disabled);
        assert!(!props.readonly);
        assert_eq!(props.pen_color, "#000000");
        assert_eq!(props.pen_width, 2.0);
        assert_eq!(props.min_distance, 3.0);
        assert_eq!(props.name, None);
    }

    #[test]
    fn props_builder_sets_expected_fields() {
        let props = Props::new()
            .id("sig")
            .default_data(data_with(1))
            .disabled(true)
            .readonly(true)
            .pen_color("#ff0000")
            .pen_width(4.0)
            .min_distance(1.0)
            .name("autograph");

        assert_eq!(props.id, "sig");
        assert_eq!(props.default_data, data_with(1));
        assert!(props.disabled);
        assert!(props.readonly);
        assert_eq!(props.pen_color, "#ff0000");
        assert_eq!(props.pen_width, 4.0);
        assert_eq!(props.min_distance, 1.0);
        assert_eq!(props.name.as_deref(), Some("autograph"));
    }

    #[test]
    fn init_empty_data_starts_idle_uncontrolled() {
        let service = fresh_service(test_props());

        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().data.get().is_empty());
        assert!(!service.context().data.is_controlled());
        assert_eq!(service.context().pen_color, "#000000");
        assert_eq!(service.context().pen_width, 2.0);
        assert_eq!(service.context().min_distance, 3.0);
        assert!(!service.context().focused);
        assert_eq!(service.context().ids.id(), "sig");
    }

    #[test]
    fn init_with_default_data_starts_completed() {
        let service = fresh_service(test_props().default_data(data_with(1)));

        assert_eq!(service.state(), &State::Completed);
        assert_eq!(service.context().data.get().strokes.len(), 1);
    }

    #[test]
    fn init_with_controlled_data_starts_completed_and_controlled() {
        let service = fresh_service(test_props().data(data_with(2)));

        assert_eq!(service.state(), &State::Completed);
        assert!(service.context().data.is_controlled());
        assert_eq!(service.context().data.get().strokes.len(), 2);
    }

    #[test]
    fn init_mirrors_pen_customization_into_context() {
        let service = fresh_service(test_props().pen_color("#0000ff").pen_width(5.0));

        assert_eq!(service.context().pen_color, "#0000ff");
        assert_eq!(service.context().pen_width, 5.0);
    }

    // ───────────────────────── transitions ─────────────────────────

    #[test]
    fn draw_start_enters_drawing_and_seeds_stroke() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::DrawStart {
            x: 1.0,
            y: 2.0,
            pressure: 0.7,
        });

        assert_eq!(service.state(), &State::Drawing);

        let stroke = service
            .context()
            .current_stroke
            .as_ref()
            .expect("drawing seeds a current stroke");

        assert_eq!(stroke.points.len(), 1);
        assert_eq!(stroke.points[0].x, 1.0);
        assert_eq!(stroke.points[0].y, 2.0);
        assert_eq!(stroke.points[0].pressure, 0.7);
        assert_eq!(effect_names(&result), vec![Effect::DrawingListeners]);
    }

    #[test]
    fn draw_move_appends_point_beyond_min_distance() {
        let mut service = fresh_service(test_props().min_distance(3.0));

        drop(service.send(Event::DrawStart {
            x: 0.0,
            y: 0.0,
            pressure: 0.5,
        }));
        drop(service.send(Event::DrawMove {
            x: 10.0,
            y: 0.0,
            pressure: 0.5,
        }));

        let stroke = service.context().current_stroke.as_ref().unwrap();

        assert_eq!(stroke.points.len(), 2);
    }

    #[test]
    fn draw_move_culls_points_within_min_distance() {
        let mut service = fresh_service(test_props().min_distance(3.0));

        drop(service.send(Event::DrawStart {
            x: 0.0,
            y: 0.0,
            pressure: 0.5,
        }));
        // 1px away: below the 3px threshold, so dropped.
        drop(service.send(Event::DrawMove {
            x: 1.0,
            y: 0.0,
            pressure: 0.5,
        }));

        let stroke = service.context().current_stroke.as_ref().unwrap();

        assert_eq!(stroke.points.len(), 1);
    }

    #[test]
    fn draw_end_commits_multi_point_stroke_and_announces() {
        let mut service = fresh_service(test_props().min_distance(0.0));

        drop(service.send(Event::DrawStart {
            x: 0.0,
            y: 0.0,
            pressure: 0.5,
        }));
        drop(service.send(Event::DrawMove {
            x: 5.0,
            y: 5.0,
            pressure: 0.5,
        }));

        let result = service.send(Event::DrawEnd);

        assert_eq!(service.state(), &State::Completed);
        assert_eq!(service.context().data.get().strokes.len(), 1);
        assert!(service.context().current_stroke.is_none());
        assert_eq!(
            effect_names(&result),
            vec![Effect::AnnounceProvided, Effect::DataChange]
        );
    }

    #[test]
    fn draw_end_discards_single_point_stroke_and_returns_to_idle() {
        let mut service = fresh_service(test_props().min_distance(100.0));

        drop(service.send(Event::DrawStart {
            x: 0.0,
            y: 0.0,
            pressure: 0.5,
        }));
        // Move is culled (within min_distance), leaving a 1-point stroke.
        drop(service.send(Event::DrawMove {
            x: 1.0,
            y: 0.0,
            pressure: 0.5,
        }));

        let result = service.send(Event::DrawEnd);

        // A blank canvas after a discarded tap is `Idle`, not `Completed`, and
        // nothing was provided to announce.
        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().data.get().is_empty());
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn draw_end_discarded_tap_keeps_prior_signature_completed() {
        let mut service =
            fresh_service(test_props().default_data(data_with(1)).min_distance(100.0));

        assert_eq!(service.state(), &State::Completed);

        drop(service.send(Event::DrawStart {
            x: 0.0,
            y: 0.0,
            pressure: 0.5,
        }));
        // Culled move leaves a 1-point stroke; the tap is discarded but the
        // earlier signature remains, so the pad stays `Completed`.
        drop(service.send(Event::DrawMove {
            x: 1.0,
            y: 0.0,
            pressure: 0.5,
        }));

        let result = service.send(Event::DrawEnd);

        assert_eq!(service.state(), &State::Completed);
        assert_eq!(service.context().data.get().strokes.len(), 1);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn undo_pops_last_stroke_and_stays_completed_while_strokes_remain() {
        let mut service = fresh_service(test_props().default_data(data_with(2)));

        assert_eq!(service.state(), &State::Completed);

        drop(service.send(Event::Undo));

        assert_eq!(service.context().data.get().strokes.len(), 1);
        assert_eq!(service.state(), &State::Completed);
    }

    #[test]
    fn undo_removing_last_stroke_returns_to_idle() {
        let mut service = fresh_service(test_props().default_data(data_with(1)));

        assert_eq!(service.state(), &State::Completed);

        drop(service.send(Event::Undo));

        assert!(service.context().data.get().is_empty());
        assert_eq!(service.state(), &State::Idle);
    }

    #[test]
    fn undo_returns_to_idle_when_only_empty_strokes_remain() {
        // `[empty_stroke, real_stroke]` is non-blank, so init is Completed; undo
        // pops the real stroke leaving only an empty stroke, which is blank — so
        // the target is Idle (computed from post-pop emptiness, not stroke count).
        let mut service = fresh_service(test_props().default_data(SignatureData {
            strokes: vec![
                SignatureStroke { points: Vec::new() },
                stroke(&[(0.0, 0.0), (1.0, 1.0)]),
            ],
        }));
        assert_eq!(service.state(), &State::Completed);

        drop(service.send(Event::Undo));

        assert!(service.context().data.get().is_empty());
        assert_eq!(service.state(), &State::Idle);
    }

    #[test]
    fn undo_ignored_when_idle() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::Undo);

        assert_eq!(service.state(), &State::Idle);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn clear_resets_to_idle_and_announces() {
        let mut service = fresh_service(test_props().default_data(data_with(2)));

        let result = service.send(Event::Clear);

        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().data.get().is_empty());
        assert!(service.context().current_stroke.is_none());
        assert_eq!(
            effect_names(&result),
            vec![Effect::AnnounceCleared, Effect::DataChange]
        );
    }

    #[test]
    fn clear_works_while_drawing() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::DrawStart {
            x: 0.0,
            y: 0.0,
            pressure: 0.5,
        }));

        assert_eq!(service.state(), &State::Drawing);

        drop(service.send(Event::Clear));

        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().current_stroke.is_none());
    }

    #[test]
    fn focus_and_blur_update_focused_flag() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Focus));

        assert!(service.context().focused);

        drop(service.send(Event::Blur));

        assert!(!service.context().focused);
    }

    #[test]
    fn focus_passes_through_when_disabled() {
        let mut service = fresh_service(test_props().disabled(true));

        drop(service.send(Event::Focus));

        assert!(service.context().focused);
    }

    #[test]
    fn disabled_pad_ignores_draw_start() {
        let mut service = fresh_service(test_props().disabled(true));

        let result = service.send(Event::DrawStart {
            x: 0.0,
            y: 0.0,
            pressure: 0.5,
        });

        assert_eq!(service.state(), &State::Idle);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn readonly_pad_ignores_draw_start() {
        let mut service = fresh_service(test_props().readonly(true));

        drop(service.send(Event::DrawStart {
            x: 0.0,
            y: 0.0,
            pressure: 0.5,
        }));

        assert_eq!(service.state(), &State::Idle);
    }

    // ───────────────────────── prop sync ─────────────────────────

    #[test]
    fn set_props_syncs_config_into_context() {
        let mut service = fresh_service(test_props());

        drop(
            service.set_props(
                test_props()
                    .disabled(true)
                    .readonly(true)
                    .pen_color("#abcdef")
                    .pen_width(7.0)
                    .min_distance(9.0),
            ),
        );

        assert!(service.context().disabled);
        assert!(service.context().readonly);
        assert_eq!(service.context().pen_color, "#abcdef");
        assert_eq!(service.context().pen_width, 7.0);
        assert_eq!(service.context().min_distance, 9.0);
    }

    #[test]
    fn set_props_can_reenable_a_disabled_pad() {
        let mut service = fresh_service(test_props().disabled(true));

        assert!(service.context().disabled);

        drop(service.set_props(test_props().disabled(false)));

        assert!(!service.context().disabled);

        // And drawing works again afterwards.
        drop(service.send(Event::DrawStart {
            x: 0.0,
            y: 0.0,
            pressure: 0.5,
        }));

        assert_eq!(service.state(), &State::Drawing);
    }

    #[test]
    fn set_props_syncs_controlled_data_and_reconciles_state() {
        let mut service = fresh_service(test_props().data(SignatureData::default()));

        assert_eq!(service.state(), &State::Idle);

        // Parent pushes a non-empty controlled signature: state becomes Completed.
        drop(service.set_props(test_props().data(data_with(2))));

        assert_eq!(service.state(), &State::Completed);
        assert_eq!(service.context().data.get().strokes.len(), 2);

        // Parent clears the controlled signature: state returns to Idle.
        drop(service.set_props(test_props().data(SignatureData::default())));

        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().data.get().is_empty());
    }

    #[test]
    fn set_props_switching_to_uncontrolled_retains_value_and_reconciles_state() {
        let mut service = fresh_service(test_props().data(data_with(1)));

        assert!(service.context().data.is_controlled());
        assert_eq!(service.state(), &State::Completed);

        // Dropping to uncontrolled keeps the last value (so the signature does
        // not vanish) and the state stays consistent with it.
        drop(service.set_props(test_props()));

        assert!(!service.context().data.is_controlled());
        assert_eq!(service.context().data.get().strokes.len(), 1);
        assert_eq!(service.state(), &State::Completed);
    }

    #[test]
    fn set_props_does_not_reorder_state_mid_stroke() {
        let mut service = fresh_service(test_props().data(SignatureData::default()));

        drop(service.send(Event::DrawStart {
            x: 0.0,
            y: 0.0,
            pressure: 0.5,
        }));

        assert_eq!(service.state(), &State::Drawing);

        // A controlled-data sync arriving mid-stroke updates data but keeps the
        // active drawing state.
        drop(service.set_props(test_props().data(data_with(1))));

        assert_eq!(service.state(), &State::Drawing);
    }

    #[test]
    fn set_props_disabling_mid_stroke_cancels_drawing() {
        let mut service = fresh_service(test_props());
        drop(service.send(Event::DrawStart {
            x: 0.0,
            y: 0.0,
            pressure: 0.5,
        }));
        assert_eq!(service.state(), &State::Drawing);

        // Disabling while drawing must cancel the in-flight stroke and leave
        // Drawing, not strand the machine where the disabled gate rejects DrawEnd.
        drop(service.set_props(test_props().disabled(true)));
        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().current_stroke.is_none());

        // The (now disabled) trailing DrawEnd is a no-op and cannot commit a
        // stale stroke.
        drop(service.send(Event::DrawEnd));
        assert!(service.context().data.get().is_empty());
    }

    #[test]
    fn set_props_readonly_mid_stroke_with_prior_data_returns_to_completed() {
        let mut service = fresh_service(test_props().default_data(data_with(1)));
        drop(service.send(Event::DrawStart {
            x: 9.0,
            y: 9.0,
            pressure: 0.5,
        }));
        assert_eq!(service.state(), &State::Drawing);

        drop(service.set_props(test_props().default_data(data_with(1)).readonly(true)));
        assert_eq!(service.state(), &State::Completed);
        assert!(service.context().current_stroke.is_none());
    }

    // ───────────────────────── on_data_change ─────────────────────────

    fn recording_pad() -> (
        Service<Machine>,
        std::sync::Arc<std::sync::Mutex<Vec<SignatureData>>>,
    ) {
        use std::sync::{Arc, Mutex};

        let log: Arc<Mutex<Vec<SignatureData>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&log);
        let service = Service::<Machine>::new(
            test_props()
                .min_distance(0.0)
                .on_data_change(ars_core::callback(move |data: SignatureData| {
                    sink.lock().unwrap().push(data);
                })),
            &Env::default(),
            &Messages::default(),
        );
        (service, log)
    }

    /// Sends an event and runs the resulting pending effects, so effect-backed
    /// callbacks (like `on_data_change`) actually fire — `Service::send` only
    /// returns the effects for the adapter to run.
    fn send_run(service: &mut Service<Machine>, event: Event) {
        use std::sync::Arc;

        let mut result = service.send(event);
        let send: ars_core::StrongSend<Event> = Arc::new(|_| {});
        for effect in result.pending_effects.drain(..) {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }
    }

    fn draw_one_stroke(service: &mut Service<Machine>) {
        send_run(
            service,
            Event::DrawStart {
                x: 0.0,
                y: 0.0,
                pressure: 0.5,
            },
        );
        send_run(
            service,
            Event::DrawMove {
                x: 5.0,
                y: 5.0,
                pressure: 0.5,
            },
        );
        send_run(service, Event::DrawEnd);
    }

    #[test]
    fn on_data_change_fires_with_new_data_on_commit_undo_and_clear() {
        let (mut service, log) = recording_pad();

        draw_one_stroke(&mut service);
        assert_eq!(log.lock().unwrap().len(), 1);
        assert_eq!(log.lock().unwrap()[0].strokes.len(), 1);

        send_run(&mut service, Event::Undo);
        assert_eq!(log.lock().unwrap().len(), 2);
        assert!(log.lock().unwrap()[1].is_empty());

        // Drawing again then clearing fires once more with the emptied data.
        draw_one_stroke(&mut service);
        send_run(&mut service, Event::Clear);
        let calls = log.lock().unwrap();
        assert_eq!(calls.len(), 4);
        assert!(calls[3].is_empty());
    }

    #[test]
    fn clear_on_blank_pad_does_not_fire_data_change() {
        let (mut service, log) = recording_pad();
        send_run(&mut service, Event::Clear);
        assert!(log.lock().unwrap().is_empty());
    }

    #[test]
    fn controlled_data_change_round_trips_through_callback_and_sync() {
        // In controlled mode the parent owns `data`: the component proposes the
        // new value via `on_data_change`, the parent feeds it back through props,
        // and only then does `api.data()` reflect it.
        let mut service = Service::<Machine>::new(
            test_props()
                .min_distance(0.0)
                .data(SignatureData::default()),
            &Env::default(),
            &Messages::default(),
        );

        draw_one_stroke(&mut service);
        // The committed stroke is staged but the controlled value still reads empty.
        assert!(service.context().data.get().is_empty());
        let pending = service.context().data.pending().clone();
        assert_eq!(pending.strokes.len(), 1);

        // Parent applies the proposed data back through props.
        drop(service.set_props(test_props().min_distance(0.0).data(pending)));
        assert_eq!(service.context().data.get().strokes.len(), 1);
        assert_eq!(service.state(), &State::Completed);
    }

    #[test]
    fn controlled_rapid_strokes_accumulate_in_pending() {
        use std::sync::{Arc, Mutex};

        // Controlled, with the parent not yet round-tripping: a second stroke
        // drawn before the sync must build on the pending value, not the stale
        // controlled one, so the first stroke is not lost.
        let log: Arc<Mutex<Vec<SignatureData>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&log);
        let mut service = Service::<Machine>::new(
            test_props()
                .min_distance(0.0)
                .data(SignatureData::default())
                .on_data_change(ars_core::callback(move |data: SignatureData| {
                    sink.lock().unwrap().push(data);
                })),
            &Env::default(),
            &Messages::default(),
        );

        draw_one_stroke(&mut service); // stroke A
        draw_one_stroke(&mut service); // stroke B, before any parent sync

        let calls = log.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].strokes.len(), 1, "first callback reports [A]");
        assert_eq!(
            calls[1].strokes.len(),
            2,
            "second callback reports [A, B], not just [B]"
        );
        assert_eq!(service.context().data.pending().strokes.len(), 2);
    }

    // ───────────────────────── Api accessors ─────────────────────────

    #[test]
    fn api_query_methods_reflect_state_and_data() {
        let drawing = empty_api(State::Drawing);

        assert!(drawing.is_drawing());
        assert!(drawing.is_empty());

        let completed = filled_api(State::Completed);

        assert!(!completed.is_drawing());
        assert!(!completed.is_empty());
        assert_eq!(completed.data().strokes.len(), 1);
    }

    #[test]
    fn api_guide_text_uses_messages() {
        assert_eq!(empty_api(State::Idle).guide_text(), "Sign here");
    }

    #[test]
    fn connect_builds_api_and_debug_renders() {
        let service = fresh_service(test_props());

        let api = service.connect(&|_| {});

        assert!(api.is_empty());
        assert!(!api.is_drawing());

        drop(api.root_attrs());

        assert!(format!("{api:?}").contains("signature_pad::Api"));
    }

    #[test]
    fn api_event_dispatchers_send_events() {
        use std::sync::{Arc, Mutex};

        let captured: Arc<Mutex<Vec<Event>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&captured);
        let send: Box<dyn Fn(Event)> = Box::new(move |event| sink.lock().unwrap().push(event));
        let send = Box::leak(send);

        let props = Box::leak(Box::new(test_props()));

        let messages = Messages::default();

        let (state, ctx) = Machine::init(props, &Env::default(), &messages);

        let ctx = Box::leak(Box::new(ctx));

        let state = Box::leak(Box::new(state));

        let api = Api {
            state,
            ctx,
            props,
            send,
        };

        api.on_canvas_pointer_down(3.0, 4.0, 0.9);
        api.on_clear();
        api.on_undo();

        let events = captured.lock().unwrap();

        assert_eq!(
            *events,
            vec![
                Event::DrawStart {
                    x: 3.0,
                    y: 4.0,
                    pressure: 0.9
                },
                Event::Clear,
                Event::Undo,
            ]
        );
    }

    #[test]
    fn canvas_role_is_application_when_interactive() {
        let api = empty_api(State::Idle);

        let attrs = api.canvas_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("application"));
    }

    #[test]
    fn canvas_role_is_img_when_readonly_or_disabled() {
        let readonly = api_for(State::Completed, data_with(1), false, true, false);

        assert_eq!(readonly.canvas_attrs().get(&HtmlAttr::Role), Some("img"));

        let disabled = api_for(State::Idle, SignatureData::default(), true, false, false);

        assert_eq!(disabled.canvas_attrs().get(&HtmlAttr::Role), Some("img"));
    }

    #[test]
    fn clear_and_undo_triggers_disabled_when_empty() {
        let api = empty_api(State::Idle);

        assert!(api.clear_trigger_attrs().contains(&HtmlAttr::Disabled));
        assert!(api.undo_trigger_attrs().contains(&HtmlAttr::Disabled));
    }

    #[test]
    fn clear_and_undo_triggers_enabled_when_filled() {
        let api = filled_api(State::Completed);

        assert!(!api.clear_trigger_attrs().contains(&HtmlAttr::Disabled));
        assert!(!api.undo_trigger_attrs().contains(&HtmlAttr::Disabled));
    }

    #[test]
    fn guide_hidden_when_strokes_present() {
        assert!(
            !empty_api(State::Idle)
                .guide_attrs()
                .contains(&HtmlAttr::Data("ars-hidden"))
        );
        assert!(
            filled_api(State::Completed)
                .guide_attrs()
                .contains(&HtmlAttr::Data("ars-hidden"))
        );
    }

    #[test]
    fn hidden_input_carries_name_and_svg_path() {
        let api = filled_api(State::Completed);

        let attrs = api.hidden_input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Name), Some("signature"));
        assert_eq!(
            attrs.get(&HtmlAttr::Value).map(str::to_string),
            Some(data_with(1).to_svg_path())
        );
        // Enabled pad: the input participates in submission.
        assert!(!attrs.contains(&HtmlAttr::Disabled));
    }

    #[test]
    fn hidden_input_disabled_when_pad_disabled() {
        let api = api_for(State::Completed, data_with(1), true, false, false);
        // A disabled pad must not submit its (stale) signature value.
        assert!(api.hidden_input_attrs().contains(&HtmlAttr::Disabled));
    }

    #[test]
    fn connect_api_dispatch_matches_direct_attrs() {
        let api = filled_api(State::Completed);

        for part in [
            Part::Root,
            Part::Canvas,
            Part::ClearTrigger,
            Part::UndoTrigger,
            Part::Label,
            Part::Guide,
            Part::HiddenInput,
        ] {
            let direct = match part {
                Part::Root => api.root_attrs(),
                Part::Canvas => api.canvas_attrs(),
                Part::ClearTrigger => api.clear_trigger_attrs(),
                Part::UndoTrigger => api.undo_trigger_attrs(),
                Part::Label => api.label_attrs(),
                Part::Guide => api.guide_attrs(),
                Part::HiddenInput => api.hidden_input_attrs(),
            };

            assert_eq!(
                snapshot_attrs(&api.part_attrs(part)),
                snapshot_attrs(&direct)
            );
        }
    }

    // ───────────────────────── snapshots ─────────────────────────

    #[test]
    fn snapshot_root_idle() {
        assert_snapshot!(snapshot_attrs(&empty_api(State::Idle).root_attrs()));
    }

    #[test]
    fn snapshot_root_drawing() {
        assert_snapshot!(snapshot_attrs(&empty_api(State::Drawing).root_attrs()));
    }

    #[test]
    fn snapshot_root_completed() {
        assert_snapshot!(snapshot_attrs(&filled_api(State::Completed).root_attrs()));
    }

    #[test]
    fn snapshot_root_disabled() {
        let api = api_for(State::Idle, SignatureData::default(), true, false, false);

        assert_snapshot!(snapshot_attrs(&api.root_attrs()));
    }

    #[test]
    fn snapshot_root_readonly() {
        let api = api_for(State::Completed, data_with(1), false, true, false);

        assert_snapshot!(snapshot_attrs(&api.root_attrs()));
    }

    #[test]
    fn snapshot_canvas_interactive() {
        assert_snapshot!(snapshot_attrs(&empty_api(State::Idle).canvas_attrs()));
    }

    #[test]
    fn snapshot_canvas_focused() {
        let api = api_for(State::Idle, SignatureData::default(), false, false, true);

        assert_snapshot!(snapshot_attrs(&api.canvas_attrs()));
    }

    #[test]
    fn snapshot_canvas_disabled() {
        let api = api_for(State::Idle, SignatureData::default(), true, false, false);

        assert_snapshot!(snapshot_attrs(&api.canvas_attrs()));
    }

    #[test]
    fn snapshot_canvas_readonly() {
        let api = api_for(State::Completed, data_with(1), false, true, false);

        assert_snapshot!(snapshot_attrs(&api.canvas_attrs()));
    }

    #[test]
    fn snapshot_clear_trigger_empty() {
        assert_snapshot!(snapshot_attrs(
            &empty_api(State::Idle).clear_trigger_attrs()
        ));
    }

    #[test]
    fn snapshot_clear_trigger_filled() {
        assert_snapshot!(snapshot_attrs(
            &filled_api(State::Completed).clear_trigger_attrs()
        ));
    }

    #[test]
    fn snapshot_undo_trigger_empty() {
        assert_snapshot!(snapshot_attrs(&empty_api(State::Idle).undo_trigger_attrs()));
    }

    #[test]
    fn snapshot_undo_trigger_filled() {
        assert_snapshot!(snapshot_attrs(
            &filled_api(State::Completed).undo_trigger_attrs()
        ));
    }

    #[test]
    fn snapshot_label() {
        assert_snapshot!(snapshot_attrs(&empty_api(State::Idle).label_attrs()));
    }

    #[test]
    fn snapshot_guide_visible() {
        assert_snapshot!(snapshot_attrs(&empty_api(State::Idle).guide_attrs()));
    }

    #[test]
    fn snapshot_guide_hidden() {
        assert_snapshot!(snapshot_attrs(&filled_api(State::Completed).guide_attrs()));
    }

    #[test]
    fn snapshot_hidden_input() {
        assert_snapshot!(snapshot_attrs(
            &filled_api(State::Completed).hidden_input_attrs()
        ));
    }
}
