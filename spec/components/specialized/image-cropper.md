---
component: ImageCropper
category: specialized
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: []
references:
    ark-ui: ImageCropper
---

# ImageCropper

An `ImageCropper` lets the user select a rectangular (or circular) region of an image
for cropping. It supports drag-to-move, handle-resize, aspect ratio constraints, and
zoom/rotation.

The following types define the crop data model:

```rust
/// The crop area in normalized coordinates [0.0, 1.0] relative to the image.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CropArea {
    /// Left edge as a fraction of image width.
    pub x: f64,
    /// Top edge as a fraction of image height.
    pub y: f64,
    /// Width as a fraction of image width.
    pub width: f64,
    /// Height as a fraction of image height.
    pub height: f64,
    /// Rotation in degrees.
    pub rotation: f64,
}

impl Default for CropArea {
    fn default() -> Self {
        Self { x: 0.1, y: 0.1, width: 0.8, height: 0.8, rotation: 0.0 }
    }
}

/// Which handle of the crop area the user is interacting with.
///
/// `Hash` is required because `CropHandle` is a field of the `Part::Handle`
/// anatomy variant, and `ComponentPart` derives `Hash`/`Eq` for the part enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CropHandle {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Top,
    Bottom,
    Left,
    Right,
}

impl CropHandle {
    /// The stable kebab-case token for this handle, used in the
    /// `data-ars-position` attribute and the handle's accessible label.
    pub const fn token(self) -> &'static str {
        match self {
            Self::TopLeft => "top-left",
            Self::TopRight => "top-right",
            Self::BottomLeft => "bottom-left",
            Self::BottomRight => "bottom-right",
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::Left => "left",
            Self::Right => "right",
        }
    }

    /// All eight handles, in anatomy order.
    pub const fn all() -> [Self; 8] {
        [
            Self::TopLeft,
            Self::TopRight,
            Self::BottomLeft,
            Self::BottomRight,
            Self::Top,
            Self::Bottom,
            Self::Left,
            Self::Right,
        ]
    }
}

/// Aspect ratio constraint for the crop area.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AspectRatio {
    /// No constraint.
    Free,
    /// Fixed width:height ratio.
    Fixed(f64),
    /// 1:1 square.
    Square,
    /// 4:3 landscape.
    Landscape4x3,
    /// 3:4 portrait.
    Portrait3x4,
    /// 16:9 wide.
    Wide16x9,
}

impl AspectRatio {
    /// Get the ratio as a float, or `None` when the crop is unconstrained.
    ///
    /// A `Fixed` ratio that is not finite and strictly positive (e.g. `0.0`, a
    /// negative number, or `NaN`) is rejected as `None` — treated as
    /// unconstrained — so it can never divide the crop geometry into infinite,
    /// negative, or `NaN` dimensions.
    pub fn as_ratio(&self) -> Option<f64> {
        match self {
            Self::Fixed(r) if r.is_finite() && *r > 0.0 => Some(*r),
            Self::Free | Self::Fixed(_) => None,
            Self::Square => Some(1.0),
            Self::Landscape4x3 => Some(4.0 / 3.0),
            Self::Portrait3x4 => Some(3.0 / 4.0),
            Self::Wide16x9 => Some(16.0 / 9.0),
        }
    }
}

/// Output format for the cropped image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CropOutputFormat {
    /// PNG format.
    Png,
    /// JPEG format.
    Jpeg { quality: u8 },
    /// WebP format.
    WebP { quality: u8 },
}

/// Flip state for the image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlipState {
    /// Horizontal flip.
    pub horizontal: bool,
    /// Vertical flip.
    pub vertical: bool,
}

/// Result of a crop operation.
///
/// All coordinates are normalized to [0.0, 1.0] relative to the original
/// image dimensions. This allows the crop to be applied server-side
/// regardless of the display size used during cropping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CropResult {
    /// Left edge as a fraction of original image width.
    pub x: f64,
    /// Top edge as a fraction of original image height.
    pub y: f64,
    /// Width as a fraction of original image width.
    pub width: f64,
    /// Height as a fraction of original image height.
    pub height: f64,
    /// Rotation in degrees applied to the image before cropping.
    pub rotation: f64,
    /// Scale factor applied (1.0 = no zoom).
    pub scale: f64,
    /// Aspect ratio constraint if one was active (e.g., `Some(16.0 / 9.0)`).
    pub aspect_ratio: Option<f64>,
    /// Flip state applied to the image.
    pub flip: FlipState,
}

impl CropResult {
    /// Build a `CropResult` from a `CropArea` and current zoom/aspect/flip state.
    pub fn from_crop_area(area: &CropArea, zoom: f64, aspect: &AspectRatio, flip: FlipState) -> Self {
        CropResult {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height,
            rotation: area.rotation,
            scale: zoom,
            aspect_ratio: aspect.as_ratio(),
            flip,
        }
    }
}
```

## 1. State Machine

### 1.1 States

```rust
/// The states for the `ImageCropper` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Image loaded, crop area visible, not interacting.
    Idle,
    /// User is dragging the crop area to move it.
    Dragging,
    /// User is resizing via a handle.
    Resizing {
        /// The handle the user is resizing.
        handle: CropHandle,
    },
}
```

### 1.2 Events

```rust
/// The events for the `ImageCropper` component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Start dragging the crop area.
    DragStart { x: f64, y: f64 },
    /// Move while dragging.
    DragMove { x: f64, y: f64 },
    /// End dragging.
    DragEnd,
    /// Start resizing from a handle.
    ResizeStart { handle: CropHandle, x: f64, y: f64 },
    /// Move while resizing.
    ResizeMove { x: f64, y: f64 },
    /// End resizing.
    ResizeEnd,
    /// Set the crop area directly.
    SetCropArea(CropArea),
    /// Set the aspect ratio constraint.
    SetAspectRatio(AspectRatio),
    /// Set zoom level.
    SetZoom(f64),
    /// Set rotation.
    SetRotation(f64),
    /// Flip the image horizontally.
    FlipHorizontal,
    /// Flip the image vertically.
    FlipVertical,
    /// Reset to default crop area.
    Reset,
    /// Focus entered a part.
    Focus { part: Part },
    /// Focus left a part.
    Blur { part: Part },
    /// Keyboard nudge the crop area.
    NudgeCrop { dx: f64, dy: f64 },
    /// The controlled `Props::crop` changed; re-sync the bound crop area.
    /// Dispatched by `Machine::on_props_changed`, not by user interaction, and
    /// processed regardless of `disabled`.
    SyncCrop,
    /// A configuration prop (aspect ratio, zoom bounds, disabled, circular,
    /// flip) changed; mirror the new values into the context. Dispatched by
    /// `Machine::on_props_changed`, and processed regardless of `disabled` so
    /// the cropper can be re-enabled.
    SyncProps,
}
```

### 1.3 Context

```rust
/// The context for the `ImageCropper` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The current crop area.
    pub crop: Bindable<CropArea>,
    /// Aspect ratio constraint.
    pub aspect_ratio: AspectRatio,
    /// Zoom level (1.0 = no zoom).
    pub zoom: f64,
    /// Minimum zoom.
    pub min_zoom: f64,
    /// Maximum zoom.
    pub max_zoom: f64,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the crop shape is circular.
    pub circular: bool,
    /// Current flip state.
    pub flip: FlipState,
    /// Drag origin for delta calculation.
    pub drag_origin: Option<(f64, f64)>,
    /// Crop area at drag start (for relative movement).
    pub drag_start_crop: Option<CropArea>,
    /// The anatomy part that currently holds focus, if any.
    pub focused_part: Option<Part>,
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
/// The props for the `ImageCropper` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Controlled crop area.
    pub crop: Option<CropArea>,
    /// Default crop area for uncontrolled mode.
    pub default_crop: CropArea,
    /// Image source URL.
    pub src: String,
    /// Aspect ratio constraint.
    pub aspect_ratio: AspectRatio,
    /// Initial zoom.
    pub zoom: f64,
    /// Minimum zoom.
    pub min_zoom: f64,
    /// Maximum zoom.
    pub max_zoom: f64,
    /// Circular crop mask.
    pub circular: bool,
    /// Initial flip state.
    pub flip: FlipState,
    /// Disabled state.
    pub disabled: bool,
    /// Fired when the crop area changes through user interaction (drag, resize,
    /// nudge, direct set, rotation, or reset), carrying the new `CropArea`.
    /// Required for controlled `crop`: the parent updates its controlled value
    /// from this callback, then feeds it back via props (triggering
    /// `Event::SyncCrop`). Not fired for parent-driven syncs.
    pub on_crop_change: Option<Callback<dyn Fn(CropArea) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            crop: None,
            default_crop: CropArea::default(),
            src: String::new(),
            aspect_ratio: AspectRatio::Free,
            zoom: 1.0,
            min_zoom: 1.0,
            max_zoom: 3.0,
            circular: false,
            flip: FlipState::default(),
            disabled: false,
            on_crop_change: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
/// The minimum crop dimension, as a fraction of the image, enforced during
/// resize so a handle drag can never collapse the crop area to nothing.
const MIN_CROP_SIZE: f64 = 0.05;

/// Resize the crop area based on handle position and pointer delta.
/// Enforces aspect ratio constraints and boundary clamping.
fn resize_crop_area(ctx: &mut Context, handle: CropHandle, x: f64, y: f64) {
    if let (Some((ox, oy)), Some(ref start)) = (ctx.drag_origin, &ctx.drag_start_crop) {
        let dx = x - ox;
        let dy = y - oy;
        let mut crop = *start;

        match handle {
            CropHandle::TopLeft => {
                crop.x = (start.x + dx).clamp(0.0, start.x + start.width - MIN_CROP_SIZE);
                crop.y = (start.y + dy).clamp(0.0, start.y + start.height - MIN_CROP_SIZE);
                crop.width = start.width - (crop.x - start.x);
                crop.height = start.height - (crop.y - start.y);
            }
            CropHandle::TopRight => {
                crop.y = (start.y + dy).clamp(0.0, start.y + start.height - MIN_CROP_SIZE);
                crop.width = (start.width + dx).clamp(MIN_CROP_SIZE, 1.0 - start.x);
                crop.height = start.height - (crop.y - start.y);
            }
            CropHandle::BottomLeft => {
                crop.x = (start.x + dx).clamp(0.0, start.x + start.width - MIN_CROP_SIZE);
                crop.width = start.width - (crop.x - start.x);
                crop.height = (start.height + dy).clamp(MIN_CROP_SIZE, 1.0 - start.y);
            }
            CropHandle::BottomRight => {
                crop.width = (start.width + dx).clamp(MIN_CROP_SIZE, 1.0 - start.x);
                crop.height = (start.height + dy).clamp(MIN_CROP_SIZE, 1.0 - start.y);
            }
            CropHandle::Top => {
                crop.y = (start.y + dy).clamp(0.0, start.y + start.height - MIN_CROP_SIZE);
                crop.height = start.height - (crop.y - start.y);
            }
            CropHandle::Bottom => {
                crop.height = (start.height + dy).clamp(MIN_CROP_SIZE, 1.0 - start.y);
            }
            CropHandle::Left => {
                crop.x = (start.x + dx).clamp(0.0, start.x + start.width - MIN_CROP_SIZE);
                crop.width = start.width - (crop.x - start.x);
            }
            CropHandle::Right => {
                crop.width = (start.width + dx).clamp(MIN_CROP_SIZE, 1.0 - start.x);
            }
        }

        if let Some(ratio) = ctx.aspect_ratio.as_ratio() {
            constrain_to_ratio(&mut crop, ratio);
        }

        ctx.crop.set(crop);
    }
}

/// Re-shape `crop` to the given width:height `ratio`, keeping both dimensions
/// within `[MIN_CROP_SIZE, image edge]`. The width drives the result: it is
/// held at or above the floor that keeps *both* sides `>= MIN_CROP_SIZE`
/// (`max(MIN, MIN * ratio)`) and at or below what the right/bottom edges allow,
/// then the height is derived. `.max().min()` (not `f64::clamp`) is deliberate:
/// when the crop sits so close to an edge that the bounds cross, fitting the
/// image wins over the minimum-size floor instead of panicking.
fn constrain_to_ratio(crop: &mut CropArea, ratio: f64) {
    let lower = MIN_CROP_SIZE.max(MIN_CROP_SIZE * ratio);
    let upper = (1.0 - crop.x).min((1.0 - crop.y) * ratio);
    let width = crop.width.max(lower).min(upper);
    crop.width = width;
    crop.height = width / ratio;
}

/// Re-constrain the current crop area to match the current aspect ratio.
///
/// The constrained value is written to the internal slot and, in controlled
/// mode, mirrored into the controlled slot too — otherwise `get()` (and every
/// rendered attr/API value) would keep returning the parent's unconstrained
/// crop until the parent next changes `crop`, even though the ratio just
/// changed (e.g. via `SetAspectRatio` or a `SyncProps` aspect change).
fn enforce_aspect_ratio(ctx: &mut Context) {
    if let Some(ratio) = ctx.aspect_ratio.as_ratio() {
        let mut crop = *ctx.crop.pending();
        constrain_to_ratio(&mut crop, ratio);
        ctx.crop.set(crop);
        if ctx.crop.is_controlled() {
            ctx.crop.sync_controlled(Some(crop));
        }
    }
}

/// Normalize a rotation in degrees into the `[-180, 180]` range the rotation
/// slider advertises, so repeated `r`/`R` rotations wrap instead of running
/// past the declared `aria-valuemin`/`aria-valuemax`. A non-finite input
/// collapses to `0.0` so a stray `NaN`/`inf` can never reach the rendered
/// `aria-valuenow`/CSS.
fn normalize_rotation(degrees: f64) -> f64 {
    if !degrees.is_finite() {
        return 0.0;
    }
    (degrees + 180.0).rem_euclid(360.0) - 180.0
}

/// Clamp `zoom` into `[min, max]` without ever panicking. `f64::clamp` panics
/// when `min > max` or a bound is `NaN` — both reachable through the public
/// `min_zoom`/`max_zoom` props — so the bounds are ordered first and `.max()`/
/// `.min()` are used (yielding the value unchanged when a bound is `NaN`).
const fn clamp_zoom(zoom: f64, min: f64, max: f64) -> f64 {
    let lower = min.min(max);
    let upper = min.max(max);
    zoom.max(lower).min(upper)
}

/// Coerce an externally-supplied crop area into a renderable, in-bounds value:
/// non-finite fields fall back to the default crop, width/height are floored at
/// `MIN_CROP_SIZE` and capped at the image, the origin is pulled inside so
/// `x + width <= 1` and `y + height <= 1`, and rotation is normalized. Keeps
/// `SetCropArea`/init/`SyncCrop` from storing geometry that renders invalid CSS
/// or later inverts a resize clamp and panics.
fn sanitize_crop(area: CropArea) -> CropArea {
    let default = CropArea::default();
    let finite = |value: f64, fallback: f64| if value.is_finite() { value } else { fallback };

    let width = finite(area.width, default.width).clamp(MIN_CROP_SIZE, 1.0);
    let height = finite(area.height, default.height).clamp(MIN_CROP_SIZE, 1.0);
    let x = finite(area.x, default.x).clamp(0.0, 1.0 - width);
    let y = finite(area.y, default.y).clamp(0.0, 1.0 - height);

    CropArea { x, y, width, height, rotation: normalize_rotation(area.rotation) }
}

/// Typed effect intents emitted by the image-cropper machine.
///
/// The agnostic core never touches the live image or the screen reader; it
/// emits these markers and the framework adapter performs the real work. (The
/// `&'static str`-named effects of earlier drafts could not work: the core has
/// no `use_platform_effects()` — that hook is adapter-only — so announcements
/// must be deferred to the adapter through a typed `Effect`.)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter announces `Messages::crop_moved` into a polite `aria-live`
    /// region. Emitted on a drag-move or keyboard nudge. The adapter should
    /// debounce/throttle this (e.g. at most once per 500ms) so continuous
    /// pointer movement does not flood the screen reader.
    AnnounceCropMoved,
    /// Adapter announces `Messages::crop_resized` into a polite `aria-live`
    /// region. Emitted on a handle resize-move; debounce/throttle as above.
    AnnounceCropResized,
    /// The crop geometry changed through user interaction. Fires
    /// `Props::on_crop_change` with the new `CropArea` so a parent holding a
    /// controlled `Props::crop` can observe it. Not emitted for
    /// `Event::SyncCrop` (that change originates from the parent).
    CropChange,
}

/// Builds the `Effect::CropChange` effect that notifies `Props::on_crop_change`
/// with the new crop area. Reads the bound value's *pending* (internal) crop —
/// the value just committed by the transition.
fn crop_change_effect() -> PendingEffect<Machine> {
    PendingEffect::new(Effect::CropChange, |ctx: &Context, props: &Props, _send| {
        if let Some(callback) = &props.on_crop_change {
            callback(*ctx.crop.pending());
        }
        no_cleanup()
    })
}

/// The machine for the `ImageCropper` component.
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
        // Sanitize and ratio-constrain the initial crop *before* building the
        // `Bindable`, so the constraint is reflected in the controlled slot that
        // `get()` returns (not just the internal value), and a malformed
        // `default_crop`/controlled `crop` can never seed out-of-bounds geometry.
        let mut initial_crop = sanitize_crop(props.crop.unwrap_or(props.default_crop));
        if let Some(ratio) = props.aspect_ratio.as_ratio() {
            constrain_to_ratio(&mut initial_crop, ratio);
        }

        let crop = if props.crop.is_some() {
            Bindable::controlled(initial_crop)
        } else {
            Bindable::uncontrolled(initial_crop)
        };

        let ids = ComponentIds::from_id(&props.id);
        let locale = env.locale.clone();
        let messages = messages.clone();

        let ctx = Context {
            crop,
            aspect_ratio: props.aspect_ratio,
            // Clamp the initial zoom so the first render never advertises a value
            // outside `min_zoom..=max_zoom` (matches `SetZoom`/`SyncProps`).
            zoom: clamp_zoom(props.zoom, props.min_zoom, props.max_zoom),
            min_zoom: props.min_zoom,
            max_zoom: props.max_zoom,
            disabled: props.disabled,
            circular: props.circular,
            flip: props.flip,
            drag_origin: None,
            drag_start_crop: None,
            focused_part: None,
            locale,
            messages,
            ids,
        };

        (State::Idle, ctx)
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        assert_eq!(old.id, new.id, "image_cropper::Props.id must remain stable after init");

        let mut events = Vec::new();

        if old.crop != new.crop {
            events.push(Event::SyncCrop);
        }

        // `zoom` and `flip` are *initial* (uncontrolled) props that become
        // user-editable state after mount — like `default_crop`. Changing them
        // post-mount is intentionally ignored so a re-render that only touches an
        // unrelated prop never clobbers the user's current zoom/flip.
        if old.aspect_ratio != new.aspect_ratio
            || old.min_zoom != new.min_zoom
            || old.max_zoom != new.max_zoom
            || old.disabled != new.disabled
            || old.circular != new.circular
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
        // Focus/Blur always pass through.
        match event {
            Event::Focus { part } => {
                let p = *part;
                return Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused_part = Some(p);
                }));
            }
            Event::Blur { .. } => {
                return Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused_part = None;
                }));
            }
            Event::SyncCrop => {
                // Sanitize and ratio-constrain the parent's value, then store the
                // constrained crop in *both* the internal and controlled slots so
                // the rendered geometry (read via `get()`) satisfies the active
                // aspect ratio — consistent with init/SetCropArea.
                let aspect_ratio = props.aspect_ratio;
                let new_crop = props.crop.map(|crop| {
                    let mut crop = sanitize_crop(crop);
                    if let Some(ratio) = aspect_ratio.as_ratio() {
                        constrain_to_ratio(&mut crop, ratio);
                    }
                    crop
                });
                return Some(TransitionPlan::context_only(move |ctx| {
                    if let Some(crop) = new_crop {
                        ctx.crop.set(crop);
                    }
                    ctx.crop.sync_controlled(new_crop);
                }));
            }
            Event::SyncProps => {
                let aspect_ratio = props.aspect_ratio;
                let min_zoom = props.min_zoom;
                let max_zoom = props.max_zoom;
                let disabled = props.disabled;
                let circular = props.circular;

                // Becoming disabled mid-interaction must not strand the machine
                // in `Dragging`/`Resizing`; cancel and return to `Idle`.
                let cancel = disabled && !matches!(state, State::Idle);
                let target = if cancel { State::Idle } else { *state };

                return Some(TransitionPlan::to(target).apply(move |ctx| {
                    ctx.aspect_ratio = aspect_ratio;
                    ctx.min_zoom = min_zoom;
                    ctx.max_zoom = max_zoom;
                    ctx.disabled = disabled;
                    ctx.circular = circular;
                    // `zoom`/`flip` are user-editable initial values, not synced
                    // here; only re-clamp the *current* zoom into the (possibly
                    // new) bounds so it never sits outside the slider range.
                    ctx.zoom = clamp_zoom(ctx.zoom, min_zoom, max_zoom);
                    if cancel {
                        ctx.drag_origin = None;
                        ctx.drag_start_crop = None;
                    }
                    enforce_aspect_ratio(ctx);
                }));
            }
            _ => {}
        }

        if ctx.disabled { return None; }

        match (state, event) {
            (State::Idle, Event::DragStart { x, y }) => {
                let x = *x; let y = *y;
                Some(TransitionPlan::to(State::Dragging).apply(move |ctx| {
                    ctx.drag_origin = Some((x, y));
                    // Anchor the gesture to the rendered (controlled) crop, not an
                    // un-accepted internal edit.
                    ctx.drag_start_crop = Some(*ctx.crop.get());
                }))
            }

            (State::Dragging, Event::DragMove { x, y }) => {
                let x = *x; let y = *y;
                // The adapter debounces/throttles `AnnounceCropMoved` (e.g. at
                // most once per 500ms) so continuous pointer movement does not
                // flood the screen reader.
                Some(TransitionPlan::context_only(move |ctx| {
                    if let (Some((ox, oy)), Some(start)) =
                        (ctx.drag_origin, ctx.drag_start_crop)
                    {
                        let dx = x - ox;
                        let dy = y - oy;
                        let mut new_crop = start;
                        new_crop.x = (start.x + dx).clamp(0.0, 1.0 - start.width);
                        new_crop.y = (start.y + dy).clamp(0.0, 1.0 - start.height);
                        ctx.crop.set(new_crop);
                    }
                })
                .with_effect(PendingEffect::named(Effect::AnnounceCropMoved))
                .with_effect(crop_change_effect()))
            }

            (State::Dragging, Event::DragEnd) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.drag_origin = None;
                    ctx.drag_start_crop = None;
                }))
            }

            (State::Idle, Event::ResizeStart { handle, x, y }) => {
                let handle = *handle; let x = *x; let y = *y;
                Some(TransitionPlan::to(State::Resizing { handle }).apply(move |ctx| {
                    ctx.drag_origin = Some((x, y));
                    // Anchor the gesture to the rendered (controlled) crop, not an
                    // un-accepted internal edit.
                    ctx.drag_start_crop = Some(*ctx.crop.get());
                }))
            }

            (State::Resizing { handle }, Event::ResizeMove { x, y }) => {
                let handle = *handle; let x = *x; let y = *y;
                // The adapter debounces/throttles `AnnounceCropResized` as for
                // `AnnounceCropMoved` above.
                Some(TransitionPlan::context_only(move |ctx| {
                    resize_crop_area(ctx, handle, x, y);
                })
                .with_effect(PendingEffect::named(Effect::AnnounceCropResized))
                .with_effect(crop_change_effect()))
            }

            (State::Resizing { .. }, Event::ResizeEnd) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.drag_origin = None;
                    ctx.drag_start_crop = None;
                }))
            }

            (_, Event::SetCropArea(area)) => {
                // Sanitize the external geometry into the image before storing,
                // then apply the active aspect-ratio constraint.
                let area = sanitize_crop(*area);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.crop.set(area);
                    enforce_aspect_ratio(ctx);
                })
                .with_effect(crop_change_effect()))
            }

            (_, Event::SetAspectRatio(ratio)) => {
                let ratio = *ratio;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.aspect_ratio = ratio;
                    enforce_aspect_ratio(ctx);
                })
                .with_effect(crop_change_effect()))
            }

            (_, Event::SetZoom(zoom)) => {
                let zoom = clamp_zoom(*zoom, ctx.min_zoom, ctx.max_zoom);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.zoom = zoom;
                }))
            }

            (_, Event::SetRotation(rotation)) => {
                // Wrap into the slider's advertised `[-180, 180]` range so
                // repeated rotations never report an out-of-range `aria-valuenow`.
                let rotation = normalize_rotation(*rotation);
                Some(TransitionPlan::context_only(move |ctx| {
                    // Base off the rendered crop; `pending()` is for reporting.
                    let mut crop = *ctx.crop.get();
                    crop.rotation = rotation;
                    ctx.crop.set(crop);
                })
                .with_effect(crop_change_effect()))
            }

            (_, Event::FlipHorizontal) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.flip.horizontal = !ctx.flip.horizontal;
                }))
            }

            (_, Event::FlipVertical) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.flip.vertical = !ctx.flip.vertical;
                }))
            }

            (_, Event::Reset) => {
                // Reset returns to the *configured* initial state — the declared
                // `default_crop`, initial `zoom`, and initial `flip` — not the
                // library's hard-coded defaults.
                let default_crop = sanitize_crop(props.default_crop);
                let zoom = clamp_zoom(props.zoom, props.min_zoom, props.max_zoom);
                let flip = props.flip;
                Some(TransitionPlan::to(State::Idle).apply(move |ctx| {
                    ctx.crop.set(default_crop);
                    ctx.zoom = zoom;
                    ctx.flip = flip;
                    ctx.drag_origin = None;
                    ctx.drag_start_crop = None;
                    // Keep the reset crop consistent with an active ratio.
                    enforce_aspect_ratio(ctx);
                })
                .with_effect(crop_change_effect()))
            }

            (_, Event::NudgeCrop { dx, dy }) => {
                let dx = *dx; let dy = *dy;
                // Arrow-key nudge: discrete, so `AnnounceCropMoved` need not be
                // throttled the way the continuous drag/resize moves are.
                Some(TransitionPlan::context_only(move |ctx| {
                    // Base off the rendered crop; `pending()` is for reporting.
                    let mut crop = *ctx.crop.get();
                    crop.x = (crop.x + dx).clamp(0.0, 1.0 - crop.width);
                    crop.y = (crop.y + dy).clamp(0.0, 1.0 - crop.height);
                    ctx.crop.set(crop);
                })
                .with_effect(PendingEffect::named(Effect::AnnounceCropMoved))
                .with_effect(crop_change_effect()))
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
```

### 1.6 Connect / API

```rust
// `Copy` is added to the derive list: every field is `Copy` (the unit variants
// plus `Handle`'s `CropHandle`), and `ComponentPart` supplies the `Clone` impl
// that `Copy` requires. This lets `Event::Focus`/`Blur` carry a `Part` while
// staying `Copy`.
#[derive(ComponentPart, Copy)]
#[scope = "image-cropper"]
pub enum Part {
    Root,
    Image,
    Overlay,
    CropArea,
    Grid,
    Handle { position: CropHandle },
    ZoomSlider,
    RotationSlider,
    ResetTrigger,
    Label,
}

pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    pub fn is_dragging(&self) -> bool { matches!(self.state, State::Dragging) }
    pub fn is_resizing(&self) -> bool { matches!(self.state, State::Resizing { .. }) }
    pub fn crop(&self) -> CropArea { *self.ctx.crop.get() }
    pub fn zoom(&self) -> f64 { self.ctx.zoom }
    pub fn flip(&self) -> FlipState { self.ctx.flip }

    /// The resolution-independent crop result for the current geometry.
    pub fn result(&self) -> CropResult {
        CropResult::from_crop_area(
            self.ctx.crop.get(),
            self.ctx.zoom,
            &self.ctx.aspect_ratio,
            self.ctx.flip,
        )
    }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        attrs.set(HtmlAttr::Role, "application");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.label)(&self.ctx.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::RoleDescription), (self.ctx.messages.role_description)(&self.ctx.locale));
        let state_str = match self.state {
            State::Idle => "idle",
            State::Dragging => "dragging",
            State::Resizing { .. } => "resizing",
        };
        attrs.set(HtmlAttr::Data("ars-state"), state_str);
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
            // Crop area / sliders stay keyboard-reachable while disabled, so
            // expose the inoperable state semantically too.
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        if self.ctx.circular { attrs.set_bool(HtmlAttr::Data("ars-circular"), true); }
        attrs
    }

    pub fn image_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Image.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("image"));
        attrs.set(HtmlAttr::Src, self.props.src.clone());
        // The source image is decorative (the cropper conveys the crop via ARIA);
        // empty alt marks it so for axe/validators and SR fallbacks.
        attrs.set(HtmlAttr::Alt, "");
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs.set_style(CssProperty::Custom("ars-crop-zoom"), format!("{}", self.ctx.zoom));
        let crop = self.ctx.crop.get();
        attrs.set_style(CssProperty::Custom("ars-crop-rotation"),
            format!("{}deg", crop.rotation));
        let scale_x = if self.ctx.flip.horizontal { -1 } else { 1 };
        let scale_y = if self.ctx.flip.vertical { -1 } else { 1 };
        attrs.set_style(CssProperty::Custom("ars-crop-flip-x"), scale_x.to_string());
        attrs.set_style(CssProperty::Custom("ars-crop-flip-y"), scale_y.to_string());
        attrs
    }

    pub fn overlay_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Overlay.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("overlay"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    pub fn crop_area_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CropArea.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("crop-area"));
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs.set(HtmlAttr::Class, "ars-touch-none");

        let crop = self.ctx.crop.get();
        attrs.set_style(CssProperty::Custom("ars-crop-x"), format!("{:.4}", crop.x));
        attrs.set_style(CssProperty::Custom("ars-crop-y"), format!("{:.4}", crop.y));
        attrs.set_style(CssProperty::Custom("ars-crop-width"), format!("{:.4}", crop.width));
        attrs.set_style(CssProperty::Custom("ars-crop-height"), format!("{:.4}", crop.height));

        if self.is_dragging() { attrs.set_bool(HtmlAttr::Data("ars-dragging"), true); }
        if self.ctx.focused_part == Some(Part::CropArea) {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        attrs
    }

    pub fn grid_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Grid.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    pub fn handle_attrs(&self, position: CropHandle) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Handle { position }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Class, "ars-touch-none");

        let pos_str = position.token();
        attrs.set(HtmlAttr::Data("ars-position"), pos_str);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.handle_label)(pos_str, &self.ctx.locale));

        if matches!(self.state, State::Resizing { handle } if handle == position) {
            attrs.set_bool(HtmlAttr::Data("ars-active"), true);
        }
        attrs
    }

    pub fn zoom_slider_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ZoomSlider.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "slider");
        // Advertise *ordered* bounds (same ordering `clamp_zoom` applies) so
        // inverted `min_zoom`/`max_zoom` props never expose `valuemin > valuemax`.
        let lower = self.ctx.min_zoom.min(self.ctx.max_zoom);
        let upper = self.ctx.min_zoom.max(self.ctx.max_zoom);
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), format!("{:.2}", self.ctx.zoom));
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), format!("{lower:.2}"));
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), format!("{upper:.2}"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.zoom_slider_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs
    }

    pub fn rotation_slider_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::RotationSlider.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "slider");
        let crop = self.ctx.crop.get();
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), format!("{:.0}", crop.rotation));
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), "-180");
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), "180");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.rotation_slider_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs
    }

    pub fn reset_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ResetTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.reset_label)(&self.ctx.locale));
        if self.ctx.disabled {
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

    // `is_rtl` flips horizontal arrows so `ArrowRight` always moves toward the
    // inline-end (see §4 RTL note). `KeyboardKey` carries no printable-character
    // variant, so named arrow keys come from `data.key`.
    pub fn on_crop_area_keydown(&self, data: &KeyboardEventData, shift: bool, is_rtl: bool) {
        // Arrow keys steer the IME candidate list during composition; don't also
        // nudge the crop.
        if data.is_composing { return; }
        let nudge = if shift { 0.1 } else { 0.01 };
        let inline = if is_rtl { -nudge } else { nudge };
        match data.key {
            KeyboardKey::ArrowRight => (self.send)(Event::NudgeCrop { dx: inline, dy: 0.0 }),
            KeyboardKey::ArrowLeft => (self.send)(Event::NudgeCrop { dx: -inline, dy: 0.0 }),
            KeyboardKey::ArrowDown => (self.send)(Event::NudgeCrop { dx: 0.0, dy: nudge }),
            KeyboardKey::ArrowUp => (self.send)(Event::NudgeCrop { dx: 0.0, dy: -nudge }),
            _ => {}
        }
    }

    // Printable shortcuts read `data.character` (`KeyboardKey` excludes
    // printable characters by design — the character lives in its own field).
    pub fn on_root_keydown(&self, data: &KeyboardEventData) {
        // A printable key produced mid-IME-composition belongs to the composed
        // text, not to a cropper shortcut.
        if data.is_composing { return; }
        let Some(character) = data.character else { return; };
        match character {
            '+' | '=' => (self.send)(Event::SetZoom(self.ctx.zoom + 0.1)),
            '-' => (self.send)(Event::SetZoom(self.ctx.zoom - 0.1)),
            'r' => (self.send)(Event::SetRotation(self.ctx.crop.get().rotation + 90.0)),
            'R' => (self.send)(Event::SetRotation(self.ctx.crop.get().rotation - 90.0)),
            'h' => (self.send)(Event::FlipHorizontal),
            'v' => (self.send)(Event::FlipVertical),
            _ => {}
        }
    }

    pub fn on_crop_area_pointer_down(&self, x: f64, y: f64) {
        (self.send)(Event::DragStart { x, y });
    }

    pub fn on_handle_pointer_down(&self, handle: CropHandle, x: f64, y: f64) {
        (self.send)(Event::ResizeStart { handle, x, y });
    }

    pub fn on_reset(&self) {
        (self.send)(Event::Reset);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Image => self.image_attrs(),
            Part::Overlay => self.overlay_attrs(),
            Part::CropArea => self.crop_area_attrs(),
            Part::Grid => self.grid_attrs(),
            Part::Handle { position } => self.handle_attrs(position),
            Part::ZoomSlider => self.zoom_slider_attrs(),
            Part::RotationSlider => self.rotation_slider_attrs(),
            Part::ResetTrigger => self.reset_trigger_attrs(),
            Part::Label => self.label_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
ImageCropper
├── Root              (required -- role="application")
├── Image             (required -- the source image, possibly transformed)
├── Overlay           (required -- semi-transparent mask outside crop area)
├── CropArea          (required -- the selected region, focusable)
│   ├── Grid          (optional -- rule-of-thirds grid lines)
│   └── Handle x 8    (required -- resize handles on corners and edges)
├── ZoomSlider        (optional -- zoom control)
├── RotationSlider    (optional -- rotation control)
├── ResetTrigger      (optional -- reset to default crop)
└── Label             (required -- describes the cropper)
```

| Part           | Element    | Key Attributes                                                   |
| -------------- | ---------- | ---------------------------------------------------------------- |
| Root           | `<div>`    | `role="application"`, `aria-roledescription="cropper"`           |
| Image          | `<img>`    | `aria-hidden="true"`, zoom/rotation CSS custom properties        |
| Overlay        | `<div>`    | `aria-hidden="true"`, semi-transparent mask                      |
| CropArea       | `<div>`    | `tabindex="0"`, `ars-touch-none`, position CSS custom properties |
| Grid           | `<div>`    | `aria-hidden="true"`, rule-of-thirds overlay                     |
| Handle         | `<div>`    | `aria-label="Resize {position}"`, `ars-touch-none`               |
| ZoomSlider     | `<input>`  | `role="slider"`, `aria-valuenow/min/max`                         |
| RotationSlider | `<input>`  | `role="slider"`, `aria-valuenow/min/max`                         |
| ResetTrigger   | `<button>` | `aria-label` from messages                                       |
| Label          | `<label>`  | Labels the cropper                                               |

**`touch-action: none` requirement:** The CropArea element and all resize Handle elements
MUST include `class="ars-touch-none"` from the companion stylesheet. Without this,
touch-initiated dragging and resizing on mobile devices triggers browser scroll/pan
instead of producing pointer events.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Attribute              | Element              | Value                                         |
| ---------------------- | -------------------- | --------------------------------------------- |
| `role="application"`   | Root                 | Custom interaction model                      |
| `aria-label`           | Root                 | From `messages.label` (default: "Crop image") |
| `aria-roledescription` | Root                 | `"cropper"` -- generic type descriptor        |
| `aria-disabled="true"` | Root                 | When `disabled` (widget stays focusable)      |
| `role="slider"`        | ZoomSlider           | Zoom control                                  |
| `role="slider"`        | RotationSlider       | Rotation control                              |
| `aria-label`           | Handle               | `"Resize {position}"` from messages           |
| `tabindex="0"`         | CropArea             | Keyboard focusable for arrow-key nudging      |
| `aria-hidden="true"`   | Image, Overlay, Grid | Decorative/structural elements                |

### 3.2 Keyboard Interaction

| Key         | Element  | Action                    |
| ----------- | -------- | ------------------------- |
| Arrow keys  | CropArea | Nudge crop area (1% step) |
| Shift+Arrow | CropArea | Large nudge (10% step)    |
| +/-         | Root     | Zoom in/out               |
| r/R         | Root     | Rotate +/-90 degrees      |
| h           | Root     | Flip horizontal           |
| v           | Root     | Flip vertical             |

### 3.3 Screen Reader Announcements

The cropper includes a visually-hidden live region (`aria-live="polite"`) that
announces crop area changes. The machine emits `Effect::AnnounceCropMoved` /
`Effect::AnnounceCropResized` and the adapter performs the announcement,
debouncing/throttling the continuous pointer-driven cases:

- "Crop area moved" (`Effect::AnnounceCropMoved`) -- after arrow-key nudging or
  a pointer drag-move.
- "Crop area resized" (`Effect::AnnounceCropResized`) -- after a handle
  resize-move.

## 4. Internationalization

### 4.1 Messages

```rust
/// The messages for the `ImageCropper` component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible label for the cropper root.
    pub label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Role description for the cropper root (default: "cropper").
    pub role_description: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label template for resize handles (receives position name).
    pub handle_label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
    /// Accessible label for the zoom slider (default: "Zoom").
    pub zoom_slider_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Accessible label for the rotation slider (default: "Rotation").
    pub rotation_slider_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the reset button.
    pub reset_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Announcement when crop area is moved.
    pub crop_moved: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Announcement when crop area is resized.
    pub crop_resized: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            label: MessageFn::static_str("Crop image"),
            role_description: MessageFn::static_str("cropper"),
            handle_label: MessageFn::new(|pos, _locale| format!("Resize {}", pos)),
            zoom_slider_label: MessageFn::static_str("Zoom"),
            rotation_slider_label: MessageFn::static_str("Rotation"),
            reset_label: MessageFn::static_str("Reset crop"),
            crop_moved: MessageFn::static_str("Crop area moved"),
            crop_resized: MessageFn::static_str("Crop area resized"),
        }
    }
}

impl ComponentMessages for Messages {}
```

| Key                                   | Default (en-US)       | Purpose                    |
| ------------------------------------- | --------------------- | -------------------------- |
| `image_cropper.label`                 | `"Crop image"`        | Root aria-label            |
| `image_cropper.role_description`      | `"cropper"`           | Root aria-roledescription  |
| `image_cropper.handle_label`          | `"Resize {position}"` | Handle labels              |
| `image_cropper.zoom_slider_label`     | `"Zoom"`              | Zoom slider aria-label     |
| `image_cropper.rotation_slider_label` | `"Rotation"`          | Rotation slider aria-label |
| `image_cropper.reset_label`           | `"Reset crop"`        | Reset button label         |
| `image_cropper.crop_moved`            | `"Crop area moved"`   | SR announcement            |
| `image_cropper.crop_resized`          | `"Crop area resized"` | SR announcement            |

RTL: Handle positions map correctly (`Left` <-> `Right`). Arrow key nudging
reverses horizontal direction so that `ArrowRight` always moves toward the inline-end.

## 5. Library Parity

> Compared against: Ark UI (`ImageCropper`).

### 5.1 Props

| Feature                        | ars-ui                           | Ark UI                                           | Notes                                         |
| ------------------------------ | -------------------------------- | ------------------------------------------------ | --------------------------------------------- |
| `crop` / `defaultCrop`         | `crop` / `default_crop`          | `initialCrop`                                    | Equivalent; ars-ui adds controlled mode       |
| `aspectRatio`                  | `aspect_ratio`                   | `aspectRatio`                                    | Equivalent                                    |
| `cropShape`                    | `circular`                       | `cropShape` (circle/rectangle)                   | Equivalent                                    |
| `zoom` / `minZoom` / `maxZoom` | `zoom` / `min_zoom` / `max_zoom` | `zoom` / `minZoom` / `maxZoom`                   | Equivalent                                    |
| `rotation`                     | via `SetRotation` event          | `rotation` / `defaultRotation`                   | Equivalent                                    |
| `flip`                         | `flip`                           | `flip` / `defaultFlip`                           | Equivalent                                    |
| `fixedCropArea`                | --                               | `fixedCropArea`                                  | Ark prevents crop area resizing; niche        |
| `nudgeStep`                    | 0.01 (hardcoded)                 | `nudgeStep` / `nudgeStepShift` / `nudgeStepCtrl` | Ark has configurable nudge steps              |
| `zoomStep`                     | 0.1 (hardcoded)                  | `zoomStep`                                       | Ark has configurable zoom step                |
| `zoomSensitivity`              | --                               | `zoomSensitivity`                                | Ark has pinch zoom sensitivity                |
| `minWidth` / `minHeight`       | 0.05 (5% min)                    | `minWidth` / `minHeight`                         | Ark has pixel-based min; ars-ui uses fraction |
| `maxWidth` / `maxHeight`       | --                               | `maxWidth` / `maxHeight`                         | Ark has max crop dimensions                   |
| `src`                          | `src`                            | --                                               | ars-ui has image source prop                  |
| `disabled`                     | `disabled`                       | --                                               | ars-ui has disabled state                     |
| `translations`                 | `messages`                       | `translations`                                   | Equivalent                                    |

**Gaps:** None critical. `fixedCropArea` and granular nudge/zoom step props are minor configurability that adapters can override.

### 5.2 Anatomy

| Part           | ars-ui                 | Ark UI      | Notes                       |
| -------------- | ---------------------- | ----------- | --------------------------- |
| Root           | `Root`                 | `Root`      | Equivalent                  |
| Image          | `Image`                | `Image`     | Equivalent                  |
| Overlay        | `Overlay`              | --          | ars-ui has mask overlay     |
| CropArea       | `CropArea`             | `Selection` | Equivalent (different name) |
| Grid           | `Grid`                 | `Grid`      | Equivalent                  |
| Handle         | `Handle` (8 positions) | `Handle`    | Equivalent                  |
| ZoomSlider     | `ZoomSlider`           | --          | ars-ui has zoom control     |
| RotationSlider | `RotationSlider`       | --          | ars-ui has rotation control |
| ResetTrigger   | `ResetTrigger`         | --          | ars-ui has reset button     |
| Label          | `Label`                | --          | ars-ui has label            |
| Viewport       | --                     | `Viewport`  | Ark has viewport wrapper    |

**Gaps:** None. `Viewport` is a layout wrapper handled by the adapter.

### 5.3 Events

| Callback        | ars-ui                           | Ark UI             | Notes      |
| --------------- | -------------------------------- | ------------------ | ---------- |
| Crop change     | `on_crop_change` + `Bindable`    | `onCropChange`     | Equivalent |
| Zoom change     | `Event::SetZoom`                 | `onZoomChange`     | Equivalent |
| Rotation change | `Event::SetRotation`             | `onRotationChange` | Equivalent |
| Flip change     | `Event::FlipHorizontal/Vertical` | `onFlipChange`     | Equivalent |

**Gaps:** None.

### 5.4 Features

| Feature                    | ars-ui                       | Ark UI |
| -------------------------- | ---------------------------- | ------ |
| Drag to move crop          | Yes                          | Yes    |
| Handle resize              | Yes (8 handles)              | Yes    |
| Aspect ratio constraint    | Yes (multiple presets)       | Yes    |
| Zoom control               | Yes                          | Yes    |
| Rotation control           | Yes                          | Yes    |
| Flip (horizontal/vertical) | Yes                          | Yes    |
| Keyboard interaction       | Yes (arrows, +/-, r/R, h, v) | Yes    |
| Circular crop              | Yes                          | Yes    |
| Reset                      | Yes                          | Yes    |
| Touch support              | Yes (ars-touch-none)         | Yes    |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** Ark UI has more granular step configuration props (`nudgeStep`, `zoomStep`, `zoomSensitivity`); ars-ui uses sensible defaults. Ark has `fixedCropArea` to prevent resizing; ars-ui does not restrict this.
- **Recommended additions:** None.
