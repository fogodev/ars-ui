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
        self.strokes.is_empty()
    }

    /// Convert to SVG path data string.
    pub fn to_svg_path(&self) -> String {
        let mut path = String::new();
        for stroke in &self.strokes {
            if stroke.points.is_empty() { continue; }
            path.push_str(format!("M{:.1},{:.1}", stroke.points[0].x, stroke.points[0].y));
            for point in &stroke.points[1..] {
                path.push_str(format!(" L{:.1},{:.1}", point.x, point.y));
            }
        }
        path
    }

    /// Total number of points across all strokes.
    pub fn point_count(&self) -> usize {
        self.strokes.iter().map(|s| s.points.len()).sum()
    }
}

/// Export format for the signature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureFormat {
    /// PNG format.
    Png,
    /// SVG format.
    Svg,
    /// JPEG format.
    Jpeg,
    /// Base64-encoded PNG format.
    Base64Png,
    /// Raw point data.
    Points,
}

/// Exported signature data in the requested format.
#[derive(Clone, Debug, PartialEq)]
pub enum SignatureExport {
    /// PNG image bytes.
    Png(Vec<u8>),
    /// SVG markup string.
    Svg(String),
    /// JPEG image bytes.
    Jpeg(Vec<u8>),
    /// Base64-encoded PNG string (suitable for `data:` URIs).
    Base64(String),
    /// Raw point data for vector reconstruction.
    /// Each inner `Vec` is one continuous stroke.
    Points(Vec<Vec<SignaturePoint>>),
}

impl SignatureData {
    /// Export the signature in the requested format.
    ///
    /// For `Png`, `Jpeg`, and `Base64Png`, the adapter renders strokes onto
    /// an off-screen canvas and encodes the result. For `Svg`, the core
    /// library generates the markup from stroke data. For `Points`, the raw
    /// `SignaturePoint` vectors are returned directly, enabling server-side
    /// vector reconstruction regardless of display resolution.
    pub fn export(&self, format: SignatureFormat) -> SignatureExport {
        match format {
            SignatureFormat::Svg => SignatureExport::Svg(self.to_svg_path()),
            SignatureFormat::Points => {
                SignatureExport::Points(
                    self.strokes.iter().map(|s| s.points.clone()).collect()
                )
            }
            // Png, Jpeg, Base64Png require adapter-side canvas rendering.
            _ => unimplemented!("Raster export is handled by the adapter layer"),
        }
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
    /// Locale override. When `None`, resolved via `resolve_locale()`.
    pub locale: Option<Locale>,
    /// Translatable messages. When `None`, resolved via `resolve_messages()`.
    pub messages: Option<Messages>,
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
            locale: None,
            messages: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
/// The machine for the `SignaturePad` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
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
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);

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

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        _props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        // Focus/Blur always pass through regardless of disabled/readonly.
        match event {
            Event::Focus => {
                return Some(TransitionPlan::context_only(|ctx| ctx.focused = true));
            }
            Event::Blur => {
                return Some(TransitionPlan::context_only(|ctx| ctx.focused = false));
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
                }).with_named_effect("drawing-listeners", |_ctx, _props, send| {
                    let platform = use_platform_effects();
                    let send_move = send.clone();
                    let send_up = send.clone();
                    platform.track_pointer_drag(
                        Box::new(move |x, y| { send_move.call_if_alive(Event::DrawMove { x, y, pressure: 0.5 }); }),
                        Box::new(move || { send_up.call_if_alive(Event::DrawEnd); }),
                    )
                }))
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

            // End drawing
            (State::Drawing, Event::DrawEnd) => {
                Some(TransitionPlan::to(State::Completed).apply(|ctx| {
                    if let Some(stroke) = ctx.current_stroke.take() {
                        if stroke.points.len() >= 2 {
                            let mut data = ctx.data.get().clone();
                            data.strokes.push(stroke);
                            ctx.data.set(data);
                        }
                    }
                }).with_named_effect("announce-provided", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    platform.announce(&(ctx.messages.signature_provided)(&ctx.locale));
                    no_cleanup()
                }))
            }

            // Undo last stroke
            (State::Completed, Event::Undo) => {
                Some(TransitionPlan::context_only(|ctx| {
                    let mut data = ctx.data.get().clone();
                    data.strokes.pop();
                    ctx.data.set(data);
                }))
            }

            // Clear all strokes
            (_, Event::Clear) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.data.set(SignatureData::default());
                    ctx.current_stroke = None;
                }).with_named_effect("announce-cleared", |ctx, _props, _send| {
                    let platform = use_platform_effects();
                    platform.announce(&(ctx.messages.signature_cleared)(&ctx.locale));
                    no_cleanup()
                }))
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
            attrs.set(HtmlAttr::Disabled, "true");
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
            attrs.set(HtmlAttr::Disabled, "true");
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
#[derive(Clone, Debug)]
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
