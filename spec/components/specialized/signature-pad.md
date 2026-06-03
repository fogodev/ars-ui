---
component: SignaturePad
category: specialized
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: []
references:
    ark-ui: SignaturePad
---

# SignaturePad

A canvas-based signature capture component that records pointer/touch strokes as vector
path data. It supports undo, clear, and export to PNG/SVG formats.

The following types define the signature data model:

```rust
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
    /// Check if the signature data is empty.
    pub fn is_empty(&self) -> bool {
        // Point-based, not stroke-count-based: data containing only empty
        // strokes (no points) is blank, matching `to_svg_path`/`point_count`,
        // so externally supplied empty data never looks like a real signature.
        self.strokes.iter().all(|stroke| stroke.points.is_empty())
    }

    /// Convert to SVG path data string.
    pub fn to_svg_path(&self) -> String {
        let mut path = String::new();
        for stroke in &self.strokes {
            let Some(first) = stroke.points.first() else { continue };
            path.push_str(&format!("M{:.1},{:.1}", first.x, first.y));
            for point in &stroke.points[1..] {
                path.push_str(&format!(" L{:.1},{:.1}", point.x, point.y));
            }
        }
        path
    }

    /// Total number of points across all strokes.
    pub fn point_count(&self) -> usize {
        self.strokes.iter().map(|s| s.points.len()).sum()
    }
}

/// Resolution-independent export formats the agnostic core can produce.
///
/// Raster formats (PNG/JPEG/base64-PNG) need a live canvas and live in the
/// adapter layer's own export format enum, not here — keeping this type free of
/// variants the core cannot fulfil (make-invalid-states-unrepresentable).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureFormat {
    /// SVG path markup.
    Svg,
    /// Raw point data.
    Points,
}

/// Exported signature data in one of the `SignatureFormat` variants.
///
/// Mirrors `SignatureFormat`: every variant is something the core can actually
/// return, so `export` is total. Adapters define their own export type for
/// raster output.
#[derive(Clone, Debug, PartialEq)]
pub enum SignatureExport {
    /// SVG markup string.
    Svg(String),
    /// Raw point data for vector reconstruction.
    /// Each inner `Vec` is one continuous stroke.
    Points(Vec<Vec<SignaturePoint>>),
}

impl SignatureData {
    /// Export the signature in one of the resolution-independent formats the
    /// agnostic core can produce.
    ///
    /// For `Svg` the core generates the markup from stroke data. For `Points`
    /// the raw `SignaturePoint` vectors are returned directly, enabling
    /// server-side vector reconstruction regardless of display resolution.
    pub fn export(&self, format: SignatureFormat) -> SignatureExport {
        match format {
            SignatureFormat::Svg => SignatureExport::Svg(self.to_svg_path()),
            SignatureFormat::Points => {
                SignatureExport::Points(
                    self.strokes.iter().map(|s| s.points.clone()).collect()
                )
            }
        }
    }
}
```

**Raster export (PNG/JPEG/WebP).** Raster output needs a pixel surface, which the agnostic core does not have. It
is therefore an **injected platform capability**, modeled exactly like
[`PlatformEffects`](../../foundation/01-architecture.md): `ars-core` defines the
`SignatureRasterizer` trait plus the neutral `RasterPoint` / `RasterSpec` /
`RasterImage` / `RasterFormat` / `RasterError` types (see
`foundation/01-architecture.md` §2.2.8 and `foundation/11-dom-utilities.md` §7),
and the caller supplies an implementation. `ars-dom` provides
`WebSignatureRasterizer` (browser `<canvas>`); `NullSignatureRasterizer` is the
SSR/test no-op.

`SignatureData::export_raster` forwards the strokes — with per-point
**pressure** preserved, so firmer presses render thicker — to the injected
rasterizer. Requiring the rasterizer as an argument means raster export is
impossible to call without a backend (make-invalid-states-unrepresentable), so
there is no panicking or `unimplemented!` arm. `RasterFormat` covers `Png`,
`Jpeg`, and `Webp`; because a browser may not encode WebP (Safari falls back to
PNG), the returned `RasterImage::format` reports the format actually produced,
not the one requested:

```rust
impl SignatureData {
    /// Rasterize into an encoded image (PNG/JPEG) via an injected rasterizer.
    pub fn export_raster(
        &self,
        rasterizer: &dyn SignatureRasterizer,
        spec: &RasterSpec,
    ) -> Result<RasterImage, RasterError> {
        let strokes: Vec<Vec<RasterPoint>> = self
            .strokes
            .iter()
            .map(|stroke| {
                stroke
                    .points
                    .iter()
                    .map(|p| RasterPoint { x: p.x, y: p.y, pressure: p.pressure })
                    .collect()
            })
            .collect();
        rasterizer.rasterize(&strokes, spec)
    }
}
```

**Canvas element lifecycle:** The Canvas element requires `touch-action: none` (via `class="ars-touch-none"`) to prevent browser scroll/pan during touch drawing. High-DPI scaling must be applied by the adapter: `canvas.width = clientWidth * devicePixelRatio`. Touch/pointer coordinates must be transformed from page space to canvas space accounting for both CSS scaling and devicePixelRatio. The adapter should listen for `webglcontextlost`/`webglcontextrestored` events to handle GPU resource pressure gracefully.

## 1. State Machine

### 1.1 States

```rust
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
```

### 1.2 Events

```rust
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
    /// The controlled `data` prop changed; re-sync the bound signature data.
    /// Dispatched by `on_props_changed`, processed regardless of disabled/read-only.
    SyncData,
    /// A configuration prop (disabled, read-only, pen color/width, min distance)
    /// changed; mirror the new values into the context. Dispatched by
    /// `on_props_changed`, processed regardless of disabled/read-only so the pad
    /// can be re-enabled.
    SyncProps,
}
```

### 1.3 Context

```rust
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
```

### 1.4 Props

```rust
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
    /// controlled `data`: the parent updates its controlled value from this
    /// callback, then feeds it back via props (triggering `Event::SyncData`).
    /// Not fired for parent-driven syncs.
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
```

### 1.5 Full Machine Implementation

The agnostic core never touches the live canvas or the screen reader. It emits
typed [`Effect`] markers and the framework adapter performs the real work:
`DrawingListeners` tells the adapter to attach global pointer-drag listeners
(e.g. `PlatformEffects::track_pointer_drag`) that dispatch `DrawMove`/`DrawEnd`
even when the pointer leaves the canvas, and `AnnounceProvided`/`AnnounceCleared`
tell it to announce the corresponding message into a polite `aria-live` region.

```rust
/// Typed effect intents emitted by the signature-pad machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter attaches global pointer-drag listeners that dispatch
    /// `Event::DrawMove`/`Event::DrawEnd`. Emitted on entry to `Drawing`.
    DrawingListeners,
    /// Adapter announces `messages.signature_provided`. Emitted on entry to
    /// `Completed`.
    AnnounceProvided,
    /// Adapter announces `messages.signature_cleared`. Emitted on `Clear`.
    AnnounceCleared,
    /// Signature data changed through user interaction (committed stroke, undo,
    /// or clear); fires `Props::on_data_change` with the new data. Lets a parent
    /// holding controlled `data` observe the change. Not emitted for `SyncData`.
    DataChange,
}

/// The machine for the `SignaturePad` component.
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
        let data = match &props.data {
            Some(d) => Bindable::controlled(d.clone()),
            None => Bindable::uncontrolled(props.default_data.clone()),
        };

        let state = if data.get().is_empty() {
            State::Idle
        } else {
            State::Completed
        };

        let ids = ComponentIds::from_id(&props.id);
        let locale = env.locale.clone();
        let messages = messages.clone();

        (state, Context {
            data,
            current_stroke: None,
            disabled: props.disabled,
            readonly: props.readonly,
            pen_color: props.pen_color.clone(),
            pen_width: props.pen_width,
            min_distance: props.min_distance,
            focused: false,
            locale,
            messages,
            ids,
        })
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(old.id, new.id, "signature_pad::Props.id must remain stable after init");

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
                return Some(TransitionPlan::context_only(|ctx| ctx.focused = true));
            }
            Event::Blur => {
                return Some(TransitionPlan::context_only(|ctx| ctx.focused = false));
            }
            Event::SyncProps => {
                let disabled = props.disabled;
                let readonly = props.readonly;
                let pen_color = props.pen_color.clone();
                let pen_width = props.pen_width;
                let min_distance = props.min_distance;
                // Becoming non-interactive mid-stroke must not strand the machine
                // in `Drawing` (the gate below would reject the trailing
                // `DrawEnd`/`Clear`); cancel the in-flight stroke and leave it.
                let cancel_drawing = (disabled || readonly) && matches!(state, State::Drawing);
                let target = if cancel_drawing {
                    if ctx.data.get().is_empty() { State::Idle } else { State::Completed }
                } else {
                    *state
                };
                return Some(TransitionPlan::to(target).apply(move |ctx| {
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
                let empty_after = match &new_data {
                    Some(data) => data.is_empty(),
                    None => ctx.data.pending().is_empty(),
                };
                // Reconcile the displayed state with the synced data, but never
                // reorder an in-flight stroke.
                let plan = if matches!(state, State::Drawing) {
                    TransitionPlan::new()
                } else {
                    TransitionPlan::to(if empty_after { State::Idle } else { State::Completed })
                };
                return Some(plan.apply(move |ctx| {
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
            // Start drawing
            (State::Idle | State::Completed, Event::DrawStart { x, y, pressure }) => {
                let x = *x;
                let y = *y;
                let pressure = *pressure;
                Some(TransitionPlan::to(State::Drawing).apply(move |ctx| {
                    ctx.current_stroke = Some(SignatureStroke {
                        points: vec![SignaturePoint {
                            x, y, pressure, timestamp: 0.0,
                        }],
                    });
                }).with_effect(PendingEffect::named(Effect::DrawingListeners)))
            }

            // Continue drawing
            (State::Drawing, Event::DrawMove { x, y, pressure }) => {
                let x = *x;
                let y = *y;
                let pressure = *pressure;
                Some(TransitionPlan::context_only(move |ctx| {
                    if let Some(ref mut stroke) = ctx.current_stroke {
                        if let Some(last) = stroke.points.last() {
                            let dx = x - last.x;
                            let dy = y - last.y;
                            let dist = (dx * dx + dy * dy).sqrt();
                            if dist >= ctx.min_distance {
                                stroke.points.push(SignaturePoint {
                                    x, y, pressure, timestamp: 0.0,
                                });
                            }
                        }
                    }
                }))
            }

            // End drawing. A stroke too short to commit (a stray tap) leaves the
            // signature unchanged, so the pad returns to `Idle` when the canvas
            // is still blank and only announces when a stroke is actually added.
            (State::Drawing, Event::DrawEnd) => {
                let commits = ctx
                    .current_stroke
                    .as_ref()
                    .is_some_and(|stroke| stroke.points.len() >= 2);
                let target = if commits || !ctx.data.get().is_empty() {
                    State::Completed
                } else {
                    State::Idle
                };
                let mut plan = TransitionPlan::to(target).apply(|ctx| {
                    if let Some(stroke) = ctx.current_stroke.take() {
                        if stroke.points.len() >= 2 {
                            let mut data = ctx.data.get().clone();
                            data.strokes.push(stroke);
                            ctx.data.set(data);
                        }
                    }
                });
                if commits {
                    plan = plan
                        .with_effect(PendingEffect::named(Effect::AnnounceProvided))
                        .with_effect(data_change_effect());
                }
                Some(plan)
            }

            // Undo last stroke. Removing the final stroke returns to `Idle`
            // (a blank canvas is `Idle`, never `Completed`).
            (State::Completed, Event::Undo) => {
                let target = if ctx.data.get().strokes.len() <= 1 {
                    State::Idle
                } else {
                    State::Completed
                };
                Some(TransitionPlan::to(target).apply(|ctx| {
                    let mut data = ctx.data.get().clone();
                    data.strokes.pop();
                    ctx.data.set(data);
                }).with_effect(data_change_effect()))
            }

            // Clear all strokes. Notify the parent only when there was data.
            (_, Event::Clear) => {
                let had_data = !ctx.data.get().is_empty();
                let mut plan = TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.data.set(SignatureData::default());
                    ctx.current_stroke = None;
                }).with_effect(PendingEffect::named(Effect::AnnounceCleared));
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
        Api { state, ctx, props, send }
    }
}

/// Builds the `Effect::DataChange` effect, notifying `Props::on_data_change`
/// with the value just committed (`pending()`, which in controlled mode is the
/// new data even though `get()` still returns the stale parent-owned value).
fn data_change_effect() -> PendingEffect<Machine> {
    PendingEffect::new(Effect::DataChange, |ctx, props, _send| {
        if let Some(callback) = &props.on_data_change {
            callback(ctx.data.pending().clone());
        }
        no_cleanup()
    })
}
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "signature-pad"]
pub enum Part {
    Root,
    Canvas,
    ClearTrigger,
    UndoTrigger,
    Label,
    Guide,
    HiddenInput,
}

pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    pub fn is_drawing(&self) -> bool { matches!(self.state, State::Drawing) }
    pub fn is_empty(&self) -> bool { self.ctx.data.get().is_empty() }
    pub fn data(&self) -> &SignatureData { self.ctx.data.get() }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        let state_str = match self.state {
            State::Idle => "idle",
            State::Drawing => "drawing",
            State::Completed => "completed",
        };
        attrs.set(HtmlAttr::Data("ars-state"), state_str);
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.readonly { attrs.set_bool(HtmlAttr::Data("ars-readonly"), true); }
        attrs
    }

    pub fn canvas_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Canvas.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("canvas"));
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs.set(HtmlAttr::Class, "ars-touch-none");

        // Role depends on state: application when interactive, img when completed+readonly
        if self.ctx.readonly || self.ctx.disabled {
            attrs.set(HtmlAttr::Role, "img");
        } else {
            attrs.set(HtmlAttr::Role, "application");
        }

        attrs.set(HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.canvas_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy),
            self.ctx.ids.part("label"));

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

    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ClearTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("clear"));
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.clear_label)(&self.ctx.locale));
        if self.is_empty() || self.ctx.disabled || self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        attrs
    }

    pub fn undo_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::UndoTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("undo"));
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.undo_label)(&self.ctx.locale));
        if self.is_empty() || self.ctx.disabled || self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        attrs
    }

    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("label"));
        attrs
    }

    pub fn guide_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Guide.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        // Hide guide when strokes are present
        if !self.is_empty() {
            attrs.set_bool(HtmlAttr::Data("ars-hidden"), true);
        }
        attrs
    }

    /// Returns the guide placeholder text (e.g., "Sign here").
    /// The adapter renders this inside the `Guide` part.
    pub fn guide_text(&self) -> String {
        (self.ctx.messages.guide_text)(&self.ctx.locale)
    }

    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(ref name) = self.props.name {
            attrs.set(HtmlAttr::Name, name);
        }
        // Value is SVG path data for form submission
        attrs.set(HtmlAttr::Value, self.ctx.data.get().to_svg_path());
        // A disabled control is excluded from submission; mirror that so a
        // disabled pad cannot submit stale signature data.
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        attrs
    }

    pub fn on_canvas_pointer_down(&self, x: f64, y: f64, pressure: f64) {
        (self.send)(Event::DrawStart { x, y, pressure });
    }

    pub fn on_clear(&self) {
        (self.send)(Event::Clear);
    }

    pub fn on_undo(&self) {
        (self.send)(Event::Undo);
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
```

## 2. Anatomy

```text
SignaturePad
├── Root            (required)
├── Label           (required -- describes the signature area)
├── Canvas          (required -- the drawing surface)
├── Guide           (optional -- guide line or placeholder text)
├── ClearTrigger    (optional -- button to clear all strokes)
├── UndoTrigger     (optional -- button to undo the last stroke)
└── HiddenInput     (optional -- for form submission, value = SVG path data)
```

| Part         | Element    | Key Attributes                                                    |
| ------------ | ---------- | ----------------------------------------------------------------- |
| Root         | `<div>`    | `data-ars-state`, `data-ars-disabled`, `data-ars-readonly`        |
| Label        | `<label>`  | Labels the canvas area                                            |
| Canvas       | `<canvas>` | `role="application"` or `"img"`, `tabindex="0"`, `ars-touch-none` |
| Guide        | `<div>`    | Hidden when strokes are present                                   |
| ClearTrigger | `<button>` | `aria-label="Clear signature"`, disabled when empty               |
| UndoTrigger  | `<button>` | `aria-label="Undo last stroke"`, disabled when empty              |
| HiddenInput  | `<input>`  | `type="hidden"`, `name`, `value` (SVG path data)                  |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Attribute                | Element      | Value                                       |
| ------------------------ | ------------ | ------------------------------------------- |
| `role="application"`     | Canvas       | Custom interaction model (when interactive) |
| `role="img"`             | Canvas       | Visual content (when disabled/readonly)     |
| `aria-label`             | Canvas       | From `messages.canvas_label`                |
| `aria-labelledby`        | Canvas       | Label element ID                            |
| `aria-disabled="true"`   | Canvas       | When disabled                               |
| `aria-readonly="true"`   | Canvas       | When read-only                              |
| `aria-label`             | ClearTrigger | From `messages.clear_label`                 |
| `aria-label`             | UndoTrigger  | From `messages.undo_label`                  |
| `tabindex="0"`           | Canvas       | Keyboard focusable                          |
| `data-ars-focus-visible` | Canvas       | When focused via keyboard                   |

### 3.2 Keyboard Interaction

The canvas uses `role="application"` which opts out of standard screen reader virtual
cursor interaction, allowing direct pointer/touch capture. The ClearTrigger and
UndoTrigger buttons are standard focusable buttons reachable via Tab.

### 3.3 Screen Reader Announcements

The `SignaturePad` includes a visually-hidden live region (`aria-live="polite"`) that announces state changes:

- "Signature provided" -- after drawing ends (pointer up after strokes)
- "Signature cleared" -- after clear action

## 4. Internationalization

### 4.1 Messages

```rust
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
```

| Key                                | Default (en-US)        | Purpose                    |
| ---------------------------------- | ---------------------- | -------------------------- |
| `signature_pad.canvas_label`       | `"Signature pad"`      | Canvas aria-label          |
| `signature_pad.clear_label`        | `"Clear signature"`    | Clear button label         |
| `signature_pad.undo_label`         | `"Undo last stroke"`   | Undo button label          |
| `signature_pad.guide_text`         | `"Sign here"`          | Guide placeholder          |
| `signature_pad.signature_provided` | `"Signature provided"` | Screen reader announcement |
| `signature_pad.signature_cleared`  | `"Signature cleared"`  | Screen reader announcement |

RTL: Canvas drawing is direction-agnostic (pointer coordinates are absolute).
The layout of Clear/Undo buttons and Label reverses in RTL.

## 5. Library Parity

> Compared against: Ark UI (`SignaturePad`).

### 5.1 Props

| Feature                | ars-ui                          | Ark UI                     | Notes                                                    |
| ---------------------- | ------------------------------- | -------------------------- | -------------------------------------------------------- |
| `data` / `defaultData` | `data` / `default_data`         | `paths` / `defaultPaths`   | Equivalent (ars-ui uses richer `SignatureData` type)     |
| `disabled`             | `disabled`                      | `disabled`                 | Equivalent                                               |
| `readOnly`             | `readonly`                      | `readOnly`                 | Equivalent                                               |
| `required`             | --                              | `required`                 | Ark-only; form-level concern                             |
| `penColor`             | `pen_color`                     | `drawing.size`             | ars-ui has explicit pen color                            |
| `penWidth`             | `pen_width`                     | `drawing.size`             | Equivalent                                               |
| `simulatePressure`     | (via `SignaturePoint.pressure`) | `drawing.simulatePressure` | ars-ui records real pressure; simulation is adapter-side |
| `minDistance`          | `min_distance`                  | --                         | ars-ui has distance threshold                            |
| `name`                 | `name`                          | `name`                     | Equivalent                                               |
| `translations`         | `messages`                      | `translations`             | Equivalent                                               |

**Gaps:** None.

### 5.2 Anatomy

| Part         | ars-ui         | Ark UI          | Notes                                                 |
| ------------ | -------------- | --------------- | ----------------------------------------------------- |
| Root         | `Root`         | `Root`          | Equivalent                                            |
| Label        | `Label`        | `Label`         | Equivalent                                            |
| Canvas       | `Canvas`       | `Segment` (SVG) | Different rendering: ars-ui uses canvas, Ark uses SVG |
| ClearTrigger | `ClearTrigger` | `ClearTrigger`  | Equivalent                                            |
| UndoTrigger  | `UndoTrigger`  | --              | ars-ui has undo                                       |
| Guide        | `Guide`        | `Guide`         | Equivalent                                            |
| HiddenInput  | `HiddenInput`  | `HiddenInput`   | Equivalent                                            |
| Control      | --             | `Control`       | Ark has wrapper part                                  |

**Gaps:** None. Ark's `Control` is a layout wrapper.

### 5.3 Events

| Callback | ars-ui                      | Ark UI      | Notes      |
| -------- | --------------------------- | ----------- | ---------- |
| Draw     | `Event::DrawStart/Move/End` | `onDraw`    | Equivalent |
| Draw end | `Event::DrawEnd`            | `onDrawEnd` | Equivalent |

**Gaps:** None.

### 5.4 Features

| Feature              | ars-ui                                  | Ark UI               |
| -------------------- | --------------------------------------- | -------------------- |
| Stroke capture       | Yes (full path data with pressure/time) | Yes (SVG paths)      |
| Undo last stroke     | Yes                                     | No                   |
| Clear all            | Yes                                     | Yes                  |
| Export to SVG        | Yes                                     | Yes (via getDataUrl) |
| Export to PNG/JPEG   | Yes (adapter-side)                      | Yes (via getDataUrl) |
| Pressure sensitivity | Yes (recorded per point)                | Yes (simulated)      |
| Read-only display    | Yes                                     | Yes                  |
| Form submission      | Yes (SVG path data)                     | Yes                  |
| SR announcements     | Yes (signature provided/cleared)        | No                   |

**Gaps:** None. ars-ui exceeds Ark UI with undo support and richer data model.

### 5.5 Summary

- **Overall:** Full parity, with additional features.
- **Divergences:** ars-ui uses `<canvas>` rendering with `SignatureData` (rich point data with pressure/time); Ark uses `<svg>` with SVG path strings. ars-ui has undo support.
- **Recommended additions:** None.
