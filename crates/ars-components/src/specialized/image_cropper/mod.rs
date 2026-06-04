//! `ImageCropper` component state machine and connect API.
//!
//! The `ImageCropper` lets the user select a rectangular (or circular) region of
//! an image for cropping. It supports drag-to-move, handle-resize, aspect-ratio
//! constraints, zoom, rotation, flip, and keyboard nudging, and exposes the crop
//! geometry as a resolution-independent [`CropResult`].
//!
//! The agnostic core owns the crop-state model, the transform math (clamping,
//! aspect-ratio enforcement, min-size constraints), keyboard intent, and the
//! ARIA/`data-ars-*` attribute surface. It does **not** touch the live image or
//! canvas: natural-size loading, viewport measurement, pointer capture, and
//! raster export belong to the framework adapters, which feed normalized
//! pointer coordinates back in as [`Event`]s.
//!
//! Like the other component machines, the core only emits typed [`Effect`]
//! intents; the adapter fulfils them. [`Effect::AnnounceCropMoved`] and
//! [`Effect::AnnounceCropResized`] tell the adapter to announce the matching
//! message into a polite `aria-live` region (debounced during continuous
//! pointer movement); [`Effect::CropChange`] fires [`Props::on_crop_change`] so
//! a parent holding a controlled [`Props::crop`] can observe the new geometry.

use alloc::{
    format,
    string::{String, ToString as _},
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, CssProperty, Env, HasId, HtmlAttr, Locale, MessageFn, PendingEffect,
    TransitionPlan, no_cleanup,
};
use ars_interactions::{KeyboardEventData, KeyboardKey};

/// The crop area in normalized coordinates `[0.0, 1.0]` relative to the image.
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
        Self {
            x: 0.1,
            y: 0.1,
            width: 0.8,
            height: 0.8,
            rotation: 0.0,
        }
    }
}

/// Which handle of the crop area the user is interacting with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CropHandle {
    /// Top-left corner handle.
    TopLeft,

    /// Top-right corner handle.
    TopRight,

    /// Bottom-left corner handle.
    BottomLeft,

    /// Bottom-right corner handle.
    BottomRight,

    /// Top edge handle.
    Top,

    /// Bottom edge handle.
    Bottom,

    /// Left edge handle.
    Left,

    /// Right edge handle.
    Right,
}

impl CropHandle {
    /// The stable kebab-case token for this handle, used in the
    /// `data-ars-position` attribute and the handle's accessible label.
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn as_ratio(&self) -> Option<f64> {
        match self {
            Self::Free => None,
            Self::Fixed(ratio) => Some(*ratio),
            Self::Square => Some(1.0),
            Self::Landscape4x3 => Some(4.0 / 3.0),
            Self::Portrait3x4 => Some(3.0 / 4.0),
            Self::Wide16x9 => Some(16.0 / 9.0),
        }
    }
}

/// Output format for the cropped image.
///
/// The agnostic core only describes the desired encoding; the actual
/// rasterization happens in the adapter against a live pixel surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CropOutputFormat {
    /// PNG format.
    Png,

    /// JPEG format with a `0..=100` quality.
    Jpeg {
        /// Encoder quality, `0..=100`.
        quality: u8,
    },

    /// WebP format with a `0..=100` quality.
    WebP {
        /// Encoder quality, `0..=100`.
        quality: u8,
    },
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
/// All coordinates are normalized to `[0.0, 1.0]` relative to the original
/// image dimensions. This allows the crop to be applied server-side regardless
/// of the display size used during cropping.
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
    /// Build a [`CropResult`] from a [`CropArea`] and the current
    /// zoom/aspect/flip state.
    #[must_use]
    pub fn from_crop_area(
        area: &CropArea,
        zoom: f64,
        aspect: &AspectRatio,
        flip: FlipState,
    ) -> Self {
        Self {
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

/// The minimum crop dimension, as a fraction of the image, enforced during
/// resize so a handle drag can never collapse the crop area to nothing.
const MIN_CROP_SIZE: f64 = 0.05;

/// The states for the `ImageCropper` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Image loaded, crop area visible, not interacting.
    Idle,

    /// The user is dragging the crop area to move it.
    Dragging,

    /// The user is resizing via a handle.
    Resizing {
        /// The handle the user is resizing.
        handle: CropHandle,
    },
}

/// The events for the `ImageCropper` component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Start dragging the crop area.
    DragStart {
        /// Normalized x coordinate of the pointer.
        x: f64,

        /// Normalized y coordinate of the pointer.
        y: f64,
    },

    /// Move while dragging.
    DragMove {
        /// Normalized x coordinate of the pointer.
        x: f64,

        /// Normalized y coordinate of the pointer.
        y: f64,
    },

    /// End dragging.
    DragEnd,

    /// Start resizing from a handle.
    ResizeStart {
        /// The handle being grabbed.
        handle: CropHandle,

        /// Normalized x coordinate of the pointer.
        x: f64,

        /// Normalized y coordinate of the pointer.
        y: f64,
    },

    /// Move while resizing.
    ResizeMove {
        /// Normalized x coordinate of the pointer.
        x: f64,

        /// Normalized y coordinate of the pointer.
        y: f64,
    },

    /// End resizing.
    ResizeEnd,

    /// Set the crop area directly.
    SetCropArea(CropArea),

    /// Set the aspect ratio constraint.
    SetAspectRatio(AspectRatio),

    /// Set zoom level.
    SetZoom(f64),

    /// Set rotation in degrees.
    SetRotation(f64),

    /// Flip the image horizontally.
    FlipHorizontal,

    /// Flip the image vertically.
    FlipVertical,

    /// Reset to the default crop area.
    Reset,

    /// Focus entered a part.
    Focus {
        /// The anatomy part that received focus.
        part: Part,
    },

    /// Focus left a part.
    Blur {
        /// The anatomy part that lost focus.
        part: Part,
    },

    /// Keyboard nudge the crop area by a normalized delta.
    NudgeCrop {
        /// Horizontal delta as a fraction of image width.
        dx: f64,

        /// Vertical delta as a fraction of image height.
        dy: f64,
    },

    /// The controlled [`Props::crop`] changed; re-sync the bound crop area.
    /// Dispatched by [`Machine::on_props_changed`](ars_core::Machine::on_props_changed),
    /// not by user interaction, and processed regardless of `disabled`.
    SyncCrop,

    /// A configuration prop (aspect ratio, zoom bounds, disabled, circular,
    /// flip) changed; mirror the new values into the context. Dispatched by
    /// [`Machine::on_props_changed`](ars_core::Machine::on_props_changed), and
    /// processed regardless of `disabled` so the cropper can be re-enabled.
    SyncProps,
}

/// Typed effect intents emitted by the image-cropper machine.
///
/// The agnostic core never touches the live image or the screen reader; it
/// emits these markers and the framework adapter performs the real work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter announces [`Messages::crop_moved`] into a polite `aria-live`
    /// region. Emitted on a drag-move or keyboard nudge. The adapter should
    /// debounce/throttle this (e.g. at most once per 500ms) so continuous
    /// pointer movement does not flood the screen reader.
    AnnounceCropMoved,

    /// Adapter announces [`Messages::crop_resized`] into a polite `aria-live`
    /// region. Emitted on a handle resize-move. The adapter should
    /// debounce/throttle this as for [`Effect::AnnounceCropMoved`].
    AnnounceCropResized,

    /// The crop geometry changed through user interaction (drag, resize, nudge,
    /// direct set, rotation, or reset). Fires [`Props::on_crop_change`] with the
    /// new [`CropArea`] so a parent holding a controlled [`Props::crop`] can
    /// update it — without this, controlled mode would never observe the
    /// change. Not emitted for [`Event::SyncCrop`] (that change originates from
    /// the parent).
    CropChange,
}

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

    /// Drag origin for delta calculation (normalized pointer coordinates).
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
    /// nudge, direct set, rotation, or reset), carrying the new [`CropArea`].
    /// Required for controlled [`crop`](Self::crop): the parent updates its
    /// controlled value from this callback, then feeds it back via props
    /// (triggering [`Event::SyncCrop`]). Not fired for parent-driven syncs.
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

    /// Sets the controlled [`crop`](Self::crop).
    #[must_use]
    pub const fn crop(mut self, crop: CropArea) -> Self {
        self.crop = Some(crop);
        self
    }

    /// Sets [`default_crop`](Self::default_crop) for uncontrolled mode.
    #[must_use]
    pub const fn default_crop(mut self, default_crop: CropArea) -> Self {
        self.default_crop = default_crop;
        self
    }

    /// Sets [`src`](Self::src).
    #[must_use]
    pub fn src(mut self, src: impl Into<String>) -> Self {
        self.src = src.into();
        self
    }

    /// Sets [`aspect_ratio`](Self::aspect_ratio).
    #[must_use]
    pub const fn aspect_ratio(mut self, aspect_ratio: AspectRatio) -> Self {
        self.aspect_ratio = aspect_ratio;
        self
    }

    /// Sets [`zoom`](Self::zoom).
    #[must_use]
    pub const fn zoom(mut self, zoom: f64) -> Self {
        self.zoom = zoom;
        self
    }

    /// Sets [`min_zoom`](Self::min_zoom).
    #[must_use]
    pub const fn min_zoom(mut self, min_zoom: f64) -> Self {
        self.min_zoom = min_zoom;
        self
    }

    /// Sets [`max_zoom`](Self::max_zoom).
    #[must_use]
    pub const fn max_zoom(mut self, max_zoom: f64) -> Self {
        self.max_zoom = max_zoom;
        self
    }

    /// Sets [`circular`](Self::circular).
    #[must_use]
    pub const fn circular(mut self, circular: bool) -> Self {
        self.circular = circular;
        self
    }

    /// Sets [`flip`](Self::flip).
    #[must_use]
    pub const fn flip(mut self, flip: FlipState) -> Self {
        self.flip = flip;
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets [`on_crop_change`](Self::on_crop_change).
    #[must_use]
    pub fn on_crop_change(mut self, callback: Callback<dyn Fn(CropArea) + Send + Sync>) -> Self {
        self.on_crop_change = Some(callback);
        self
    }
}

/// Message function that builds a resize-handle label from its position token
/// (e.g. `"top-left"`) and the active [`Locale`].
pub type HandleLabelFn = dyn Fn(&str, &Locale) -> String + Send + Sync;

/// The messages for the `ImageCropper` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the cropper root.
    pub label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Role description for the cropper root (default: "cropper").
    pub role_description: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label template for resize handles (receives the position token).
    pub handle_label: MessageFn<HandleLabelFn>,

    /// Accessible label for the zoom slider (default: "Zoom").
    pub zoom_slider_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the rotation slider (default: "Rotation").
    pub rotation_slider_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label for the reset button.
    pub reset_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Announcement when the crop area is moved.
    pub crop_moved: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Announcement when the crop area is resized.
    pub crop_resized: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            label: MessageFn::static_str("Crop image"),
            role_description: MessageFn::static_str("cropper"),
            handle_label: MessageFn::new(|position: &str, _locale: &Locale| {
                format!("Resize {position}")
            }),
            zoom_slider_label: MessageFn::static_str("Zoom"),
            rotation_slider_label: MessageFn::static_str("Rotation"),
            reset_label: MessageFn::static_str("Reset crop"),
            crop_moved: MessageFn::static_str("Crop area moved"),
            crop_resized: MessageFn::static_str("Crop area resized"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Resize the crop area based on handle position and pointer delta.
///
/// Enforces the [`MIN_CROP_SIZE`] floor, boundary clamping, and the active
/// aspect-ratio constraint. Builds on the drag-start snapshot so the resize is
/// always relative to where the handle was first grabbed.
fn resize_crop_area(ctx: &mut Context, handle: CropHandle, x: f64, y: f64) {
    if let (Some((origin_x, origin_y)), Some(start)) = (ctx.drag_origin, ctx.drag_start_crop) {
        let dx = x - origin_x;
        let dy = y - origin_y;

        let mut crop = start;

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

        // Enforce aspect ratio if set, re-clamping height against the bottom
        // edge so the constrained crop never overflows the image.
        if let Some(ratio) = ctx.aspect_ratio.as_ratio() {
            crop.height = crop.width / ratio;

            if crop.y + crop.height > 1.0 {
                crop.height = 1.0 - crop.y;
                crop.width = crop.height * ratio;
            }
        }

        ctx.crop.set(crop);
    }
}

/// Re-constrain the current crop area to match the current aspect ratio.
fn enforce_aspect_ratio(ctx: &mut Context) {
    if let Some(ratio) = ctx.aspect_ratio.as_ratio() {
        let mut crop = *ctx.crop.pending();

        crop.height = crop.width / ratio;

        if crop.y + crop.height > 1.0 {
            crop.height = 1.0 - crop.y;
            crop.width = crop.height * ratio;
        }

        ctx.crop.set(crop);
    }
}

/// Builds the [`Effect::CropChange`] effect that notifies
/// [`Props::on_crop_change`] with the new crop area.
///
/// Reads the bound value's *pending* (internal) crop, which is the value just
/// committed by the transition — in controlled mode `get()` would still return
/// the stale parent-owned value until it round-trips back through props.
fn crop_change_effect() -> PendingEffect<Machine> {
    PendingEffect::new(Effect::CropChange, |ctx: &Context, props: &Props, _send| {
        if let Some(callback) = &props.on_crop_change {
            callback(*ctx.crop.pending());
        }

        no_cleanup()
    })
}

/// The machine for the `ImageCropper` component.
///
/// # Examples
///
/// Drag the crop area to a new position. In a real app the adapter dispatches
/// [`Event::DragMove`]/[`Event::DragEnd`] from pointer listeners; here we send
/// them directly:
///
/// ```
/// use ars_components::specialized::image_cropper::{Event, Machine, Messages, Props, State};
/// use ars_core::{Env, Service};
///
/// let mut cropper = Service::<Machine>::new(
///     Props::new().id("cropper"),
///     &Env::default(),
///     &Messages::default(),
/// );
/// assert_eq!(cropper.state(), &State::Idle);
///
/// drop(cropper.send(Event::DragStart { x: 0.5, y: 0.5 }));
/// assert_eq!(cropper.state(), &State::Dragging);
///
/// drop(cropper.send(Event::DragMove { x: 0.6, y: 0.5 }));
/// drop(cropper.send(Event::DragEnd));
/// assert_eq!(cropper.state(), &State::Idle);
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
        let crop = if let Some(crop) = props.crop {
            Bindable::controlled(crop)
        } else {
            Bindable::uncontrolled(props.default_crop)
        };

        (
            State::Idle,
            Context {
                crop,
                aspect_ratio: props.aspect_ratio,
                zoom: props.zoom,
                min_zoom: props.min_zoom,
                max_zoom: props.max_zoom,
                disabled: props.disabled,
                circular: props.circular,
                flip: props.flip,
                drag_origin: None,
                drag_start_crop: None,
                focused_part: None,
                locale: env.locale.clone(),
                messages: messages.clone(),
                ids: ComponentIds::from_id(&props.id),
            },
        )
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        assert_eq!(
            old.id, new.id,
            "image_cropper::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.crop != new.crop {
            events.push(Event::SyncCrop);
        }

        if old.aspect_ratio != new.aspect_ratio
            || old.zoom != new.zoom
            || old.min_zoom != new.min_zoom
            || old.max_zoom != new.max_zoom
            || old.disabled != new.disabled
            || old.circular != new.circular
            || old.flip != new.flip
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
        // the disabled gate (prop sync must be able to re-enable the cropper).
        match event {
            Event::Focus { part } => {
                let part = *part;
                return Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused_part = Some(part);
                }));
            }

            Event::Blur { .. } => {
                return Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focused_part = None;
                }));
            }

            Event::SyncCrop => {
                let new_crop = props.crop;
                return Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    if let Some(crop) = new_crop {
                        ctx.crop.set(crop);
                    }

                    ctx.crop.sync_controlled(new_crop);
                }));
            }

            Event::SyncProps => {
                let aspect_ratio = props.aspect_ratio;
                let zoom = props.zoom.clamp(props.min_zoom, props.max_zoom);
                let min_zoom = props.min_zoom;
                let max_zoom = props.max_zoom;
                let disabled = props.disabled;
                let circular = props.circular;
                let flip = props.flip;

                // Becoming disabled mid-interaction must not strand the machine
                // in `Dragging`/`Resizing` — the disabled gate below would then
                // reject the trailing `DragEnd`/`ResizeEnd`. Cancel and return to
                // `Idle`.
                let cancel = disabled && !matches!(state, State::Idle);

                let target = if cancel { State::Idle } else { *state };

                return Some(TransitionPlan::to(target).apply(move |ctx: &mut Context| {
                    ctx.aspect_ratio = aspect_ratio;
                    ctx.zoom = zoom;
                    ctx.min_zoom = min_zoom;
                    ctx.max_zoom = max_zoom;
                    ctx.disabled = disabled;
                    ctx.circular = circular;
                    ctx.flip = flip;

                    if cancel {
                        ctx.drag_origin = None;
                        ctx.drag_start_crop = None;
                    }

                    enforce_aspect_ratio(ctx);
                }));
            }

            _ => {}
        }

        if ctx.disabled {
            return None;
        }

        match (state, event) {
            (State::Idle, Event::DragStart { x, y }) => {
                let (x, y) = (*x, *y);
                Some(
                    TransitionPlan::to(State::Dragging).apply(move |ctx: &mut Context| {
                        ctx.drag_origin = Some((x, y));
                        ctx.drag_start_crop = Some(*ctx.crop.pending());
                    }),
                )
            }

            (State::Dragging, Event::DragMove { x, y }) => {
                let (x, y) = (*x, *y);
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        if let (Some((origin_x, origin_y)), Some(start)) =
                            (ctx.drag_origin, ctx.drag_start_crop)
                        {
                            let dx = x - origin_x;
                            let dy = y - origin_y;

                            let mut new_crop = start;

                            new_crop.x = (start.x + dx).clamp(0.0, 1.0 - start.width);
                            new_crop.y = (start.y + dy).clamp(0.0, 1.0 - start.height);

                            ctx.crop.set(new_crop);
                        }
                    })
                    .with_effect(PendingEffect::named(Effect::AnnounceCropMoved))
                    .with_effect(crop_change_effect()),
                )
            }

            (State::Dragging, Event::DragEnd) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                    ctx.drag_origin = None;
                    ctx.drag_start_crop = None;
                }))
            }

            (State::Idle, Event::ResizeStart { handle, x, y }) => {
                let (handle, x, y) = (*handle, *x, *y);
                Some(TransitionPlan::to(State::Resizing { handle }).apply(
                    move |ctx: &mut Context| {
                        ctx.drag_origin = Some((x, y));
                        ctx.drag_start_crop = Some(*ctx.crop.pending());
                    },
                ))
            }

            (State::Resizing { handle }, Event::ResizeMove { x, y }) => {
                let (handle, x, y) = (*handle, *x, *y);
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        resize_crop_area(ctx, handle, x, y);
                    })
                    .with_effect(PendingEffect::named(Effect::AnnounceCropResized))
                    .with_effect(crop_change_effect()),
                )
            }

            (State::Resizing { .. }, Event::ResizeEnd) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                    ctx.drag_origin = None;
                    ctx.drag_start_crop = None;
                }))
            }

            (_, Event::SetCropArea(area)) => {
                let area = *area;
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.crop.set(area);
                    })
                    .with_effect(crop_change_effect()),
                )
            }

            (_, Event::SetAspectRatio(ratio)) => {
                let ratio = *ratio;
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.aspect_ratio = ratio;

                        enforce_aspect_ratio(ctx);
                    })
                    .with_effect(crop_change_effect()),
                )
            }

            (_, Event::SetZoom(zoom)) => {
                let zoom = zoom.clamp(ctx.min_zoom, ctx.max_zoom);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.zoom = zoom;
                }))
            }

            (_, Event::SetRotation(rotation)) => {
                let rotation = *rotation;
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        let mut crop = *ctx.crop.pending();

                        crop.rotation = rotation;

                        ctx.crop.set(crop);
                    })
                    .with_effect(crop_change_effect()),
                )
            }

            (_, Event::FlipHorizontal) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.flip.horizontal = !ctx.flip.horizontal;
                }))
            }

            (_, Event::FlipVertical) => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.flip.vertical = !ctx.flip.vertical;
            })),

            (_, Event::Reset) => Some(
                TransitionPlan::to(State::Idle)
                    .apply(|ctx: &mut Context| {
                        ctx.crop.set(CropArea::default());
                        ctx.zoom = 1.0;
                        ctx.flip = FlipState::default();
                        ctx.drag_origin = None;
                        ctx.drag_start_crop = None;
                    })
                    .with_effect(crop_change_effect()),
            ),

            (_, Event::NudgeCrop { dx, dy }) => {
                let (dx, dy) = (*dx, *dy);
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        let mut crop = *ctx.crop.pending();

                        crop.x = (crop.x + dx).clamp(0.0, 1.0 - crop.width);
                        crop.y = (crop.y + dy).clamp(0.0, 1.0 - crop.height);

                        ctx.crop.set(crop);
                    })
                    .with_effect(PendingEffect::named(Effect::AnnounceCropMoved))
                    .with_effect(crop_change_effect()),
                )
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

/// DOM parts of the `ImageCropper` component.
///
/// `Handle { position }` is parameterized by a [`CropHandle`], so this enum
/// cannot derive [`Default`]; [`CropHandle::TopLeft`] is the canonical
/// placeholder used by [`ComponentPart::all`].
#[derive(ComponentPart, Copy)]
#[scope = "image-cropper"]
pub enum Part {
    /// Root wrapper element (`role="application"`).
    Root,

    /// The source image, possibly transformed by zoom/rotation/flip.
    Image,

    /// Semi-transparent mask outside the crop area.
    Overlay,

    /// The selected crop region, focusable for keyboard nudging.
    CropArea,

    /// Rule-of-thirds grid lines inside the crop area.
    Grid,

    /// A resize handle on a corner or edge.
    Handle {
        /// Which handle this is.
        #[part(default = CropHandle::TopLeft)]
        position: CropHandle,
    },

    /// Zoom control slider.
    ZoomSlider,

    /// Rotation control slider.
    RotationSlider,

    /// Button that resets the crop to its default.
    ResetTrigger,

    /// Label describing the cropper.
    Label,
}

/// API for the `ImageCropper` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("image_cropper::Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Whether the user is dragging the crop area.
    #[must_use]
    pub const fn is_dragging(&self) -> bool {
        matches!(self.state, State::Dragging)
    }

    /// Whether the user is resizing via a handle.
    #[must_use]
    pub const fn is_resizing(&self) -> bool {
        matches!(self.state, State::Resizing { .. })
    }

    /// The current crop area.
    #[must_use]
    pub fn crop(&self) -> CropArea {
        *self.ctx.crop.get()
    }

    /// The current zoom level.
    #[must_use]
    pub const fn zoom(&self) -> f64 {
        self.ctx.zoom
    }

    /// The current flip state.
    #[must_use]
    pub const fn flip(&self) -> FlipState {
        self.ctx.flip
    }

    /// The resolution-independent crop result for the current geometry.
    #[must_use]
    pub fn result(&self) -> CropResult {
        CropResult::from_crop_area(
            self.ctx.crop.get(),
            self.ctx.zoom,
            &self.ctx.aspect_ratio,
            self.ctx.flip,
        )
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
            .set(HtmlAttr::Role, "application")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.label)(&self.ctx.locale),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::RoleDescription),
                (self.ctx.messages.role_description)(&self.ctx.locale),
            )
            .set(HtmlAttr::Data("ars-state"), self.state_token());

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.circular {
            attrs.set_bool(HtmlAttr::Data("ars-circular"), true);
        }

        attrs
    }

    /// Image element attributes.
    #[must_use]
    pub fn image_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Image.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("image"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Src, self.props.src.clone())
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        let crop = self.ctx.crop.get();

        attrs
            .set_style(
                CssProperty::Custom("ars-crop-zoom"),
                self.ctx.zoom.to_string(),
            )
            .set_style(
                CssProperty::Custom("ars-crop-rotation"),
                format!("{}deg", crop.rotation),
            );

        let scale_x = if self.ctx.flip.horizontal { -1 } else { 1 };
        let scale_y = if self.ctx.flip.vertical { -1 } else { 1 };

        attrs
            .set_style(CssProperty::Custom("ars-crop-flip-x"), scale_x.to_string())
            .set_style(CssProperty::Custom("ars-crop-flip-y"), scale_y.to_string());

        attrs
    }

    /// Overlay (mask) element attributes.
    #[must_use]
    pub fn overlay_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Overlay.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("overlay"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Crop-area element attributes.
    #[must_use]
    pub fn crop_area_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CropArea.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("crop-area"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::TabIndex, "0")
            .set(HtmlAttr::Class, "ars-touch-none");

        let crop = self.ctx.crop.get();

        attrs
            .set_style(CssProperty::Custom("ars-crop-x"), format!("{:.4}", crop.x))
            .set_style(CssProperty::Custom("ars-crop-y"), format!("{:.4}", crop.y))
            .set_style(
                CssProperty::Custom("ars-crop-width"),
                format!("{:.4}", crop.width),
            )
            .set_style(
                CssProperty::Custom("ars-crop-height"),
                format!("{:.4}", crop.height),
            );

        if self.is_dragging() {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }

        if self.ctx.focused_part == Some(Part::CropArea) {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs
    }

    /// Grid (rule-of-thirds) element attributes.
    #[must_use]
    pub fn grid_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Grid.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Resize-handle element attributes for the given `position`.
    #[must_use]
    pub fn handle_attrs(&self, position: CropHandle) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Handle { position }.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Class, "ars-touch-none");

        let position_token = position.token();

        attrs
            .set(HtmlAttr::Data("ars-position"), position_token)
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.handle_label)(position_token, &self.ctx.locale),
            );

        if matches!(self.state, State::Resizing { handle } if *handle == position) {
            attrs.set_bool(HtmlAttr::Data("ars-active"), true);
        }

        attrs
    }

    /// Zoom-slider element attributes.
    #[must_use]
    pub fn zoom_slider_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ZoomSlider.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "slider")
            .set(
                HtmlAttr::Aria(AriaAttr::ValueNow),
                format!("{:.2}", self.ctx.zoom),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::ValueMin),
                format!("{:.2}", self.ctx.min_zoom),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::ValueMax),
                format!("{:.2}", self.ctx.max_zoom),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.zoom_slider_label)(&self.ctx.locale),
            )
            .set(HtmlAttr::TabIndex, "0");

        attrs
    }

    /// Rotation-slider element attributes.
    #[must_use]
    pub fn rotation_slider_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::RotationSlider.data_attrs();

        let crop = self.ctx.crop.get();
        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "slider")
            .set(
                HtmlAttr::Aria(AriaAttr::ValueNow),
                format!("{:.0}", crop.rotation),
            )
            .set(HtmlAttr::Aria(AriaAttr::ValueMin), "-180")
            .set(HtmlAttr::Aria(AriaAttr::ValueMax), "180")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.rotation_slider_label)(&self.ctx.locale),
            )
            .set(HtmlAttr::TabIndex, "0");

        attrs
    }

    /// Reset-trigger button attributes.
    #[must_use]
    pub fn reset_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ResetTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.reset_label)(&self.ctx.locale),
            );

        if self.ctx.disabled {
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

    /// Dispatches arrow-key nudging intent for the crop area.
    ///
    /// `shift` selects the large 10% step over the default 1% step. Direction
    /// is inline-aware via the supplied `is_rtl` flag: under RTL, `ArrowRight`
    /// moves toward the inline-end (decreasing x) so the crop always tracks the
    /// arrow's reading-direction meaning.
    pub fn on_crop_area_keydown(&self, data: &KeyboardEventData, shift: bool, is_rtl: bool) {
        let nudge = if shift { 0.1 } else { 0.01 };
        let inline = if is_rtl { -nudge } else { nudge };

        match data.key {
            KeyboardKey::ArrowRight => (self.send)(Event::NudgeCrop {
                dx: inline,
                dy: 0.0,
            }),

            KeyboardKey::ArrowLeft => (self.send)(Event::NudgeCrop {
                dx: -inline,
                dy: 0.0,
            }),

            KeyboardKey::ArrowDown => (self.send)(Event::NudgeCrop { dx: 0.0, dy: nudge }),

            KeyboardKey::ArrowUp => (self.send)(Event::NudgeCrop {
                dx: 0.0,
                dy: -nudge,
            }),

            _ => {}
        }
    }

    /// Dispatches root-level keyboard shortcuts: `+`/`=` zoom in, `-` zoom out,
    /// `r`/`R` rotate +/-90 degrees, `h` flip horizontal, `v` flip vertical.
    pub fn on_root_keydown(&self, data: &KeyboardEventData) {
        let Some(character) = data.character else {
            return;
        };

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

    /// Dispatches pointer-down intent on the crop area, beginning a drag.
    pub fn on_crop_area_pointer_down(&self, x: f64, y: f64) {
        (self.send)(Event::DragStart { x, y });
    }

    /// Dispatches pointer-down intent on a resize handle, beginning a resize.
    pub fn on_handle_pointer_down(&self, handle: CropHandle, x: f64, y: f64) {
        (self.send)(Event::ResizeStart { handle, x, y });
    }

    /// Dispatches reset intent.
    pub fn on_reset(&self) {
        (self.send)(Event::Reset);
    }

    const fn state_token(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::Dragging => "dragging",
            State::Resizing { .. } => "resizing",
        }
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

#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use std::sync::{Arc, Mutex};

    use ars_core::{Env, Machine as _, Service, StrongSend};
    use ars_interactions::{KeyboardEventData, KeyboardKey};
    use insta::assert_snapshot;

    use super::*;

    // ───────────────────────── helpers ─────────────────────────

    fn test_props() -> Props {
        Props::new().id("cropper").src("photo.jpg")
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

    /// Sends an event and runs the resulting pending effects, so effect-backed
    /// callbacks (like `on_crop_change`) actually fire — `Service::send` only
    /// returns the effects for the adapter to run.
    fn send_run(service: &mut Service<Machine>, event: Event) {
        let mut result = service.send(event);

        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in result.pending_effects.drain(..) {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }
    }

    fn key(k: KeyboardKey) -> KeyboardEventData {
        KeyboardEventData {
            key: k,
            character: None,
            code: String::new(),
            shift_key: false,
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            repeat: false,
            is_composing: false,
        }
    }

    fn char_key(c: char) -> KeyboardEventData {
        KeyboardEventData {
            character: Some(c),
            ..key(KeyboardKey::Unidentified)
        }
    }

    /// A leaked-`Api` fixture for attribute/accessor assertions.
    struct Fixture {
        state: State,
        crop: CropArea,
        zoom: f64,
        disabled: bool,
        circular: bool,
        flip: FlipState,
        focused_part: Option<Part>,
    }

    impl Default for Fixture {
        fn default() -> Self {
            Self {
                state: State::Idle,
                crop: CropArea::default(),
                zoom: 1.0,
                disabled: false,
                circular: false,
                flip: FlipState::default(),
                focused_part: None,
            }
        }
    }

    impl Fixture {
        fn api(self) -> Api<'static> {
            let props = Box::leak(Box::new(
                Props::new()
                    .id("cropper")
                    .src("photo.jpg")
                    .disabled(self.disabled)
                    .circular(self.circular),
            ));

            let (_, mut ctx) = Machine::init(props, &Env::default(), &Messages::default());

            ctx.crop.set(self.crop);
            ctx.zoom = self.zoom;
            ctx.flip = self.flip;
            ctx.focused_part = self.focused_part;

            let ctx = Box::leak(Box::new(ctx));
            let state = Box::leak(Box::new(self.state));
            let send = Box::leak(Box::new(|_: Event| {}));

            Api {
                state,
                ctx,
                props,
                send,
            }
        }
    }

    // ───────────────────────── data model ─────────────────────────

    #[test]
    fn crop_area_default_is_centered_inset() {
        let crop = CropArea::default();

        assert_eq!(crop.x, 0.1);
        assert_eq!(crop.y, 0.1);
        assert_eq!(crop.width, 0.8);
        assert_eq!(crop.height, 0.8);
        assert_eq!(crop.rotation, 0.0);
    }

    #[test]
    fn aspect_ratio_as_ratio_maps_each_preset() {
        assert_eq!(AspectRatio::Free.as_ratio(), None);
        assert_eq!(AspectRatio::Fixed(2.5).as_ratio(), Some(2.5));
        assert_eq!(AspectRatio::Square.as_ratio(), Some(1.0));
        assert_eq!(AspectRatio::Landscape4x3.as_ratio(), Some(4.0 / 3.0));
        assert_eq!(AspectRatio::Portrait3x4.as_ratio(), Some(3.0 / 4.0));
        assert_eq!(AspectRatio::Wide16x9.as_ratio(), Some(16.0 / 9.0));
    }

    #[test]
    fn crop_handle_token_and_all_cover_eight_positions() {
        let tokens: Vec<&str> = CropHandle::all().iter().map(|h| h.token()).collect();

        assert_eq!(
            tokens,
            vec![
                "top-left",
                "top-right",
                "bottom-left",
                "bottom-right",
                "top",
                "bottom",
                "left",
                "right",
            ]
        );
    }

    #[test]
    fn crop_output_format_variants_carry_quality() {
        assert_eq!(CropOutputFormat::Png, CropOutputFormat::Png);
        assert_eq!(
            CropOutputFormat::Jpeg { quality: 80 },
            CropOutputFormat::Jpeg { quality: 80 }
        );
        assert_ne!(
            CropOutputFormat::Jpeg { quality: 80 },
            CropOutputFormat::WebP { quality: 80 }
        );

        let CropOutputFormat::WebP { quality } = (CropOutputFormat::WebP { quality: 60 }) else {
            panic!("expected WebP");
        };

        assert_eq!(quality, 60);
    }

    #[test]
    fn crop_result_from_crop_area_carries_all_transform_state() {
        let crop = CropArea {
            x: 0.2,
            y: 0.3,
            width: 0.5,
            height: 0.4,
            rotation: 45.0,
        };

        let flip = FlipState {
            horizontal: true,
            vertical: false,
        };

        let result = CropResult::from_crop_area(&crop, 1.5, &AspectRatio::Wide16x9, flip);

        assert_eq!(result.x, 0.2);
        assert_eq!(result.y, 0.3);
        assert_eq!(result.width, 0.5);
        assert_eq!(result.height, 0.4);
        assert_eq!(result.rotation, 45.0);
        assert_eq!(result.scale, 1.5);
        assert_eq!(result.aspect_ratio, Some(16.0 / 9.0));
        assert_eq!(result.flip, flip);
    }

    // ───────────────────────── init ─────────────────────────

    #[test]
    fn init_uncontrolled_uses_default_crop_and_starts_idle() {
        let service = fresh_service(test_props());

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(*service.context().crop.get(), CropArea::default());
        assert!(!service.context().crop.is_controlled());
    }

    #[test]
    fn init_controlled_uses_supplied_crop() {
        let crop = CropArea {
            x: 0.25,
            y: 0.25,
            width: 0.5,
            height: 0.5,
            rotation: 0.0,
        };

        let service = fresh_service(test_props().crop(crop));

        assert_eq!(*service.context().crop.get(), crop);
        assert!(service.context().crop.is_controlled());
    }

    #[test]
    fn props_builders_set_default_crop_and_flip() {
        let crop = CropArea {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            rotation: 0.0,
        };

        let flip = FlipState {
            horizontal: true,
            vertical: true,
        };

        let service = fresh_service(test_props().default_crop(crop).flip(flip));

        assert_eq!(*service.context().crop.get(), crop);
        assert_eq!(service.context().flip, flip);
    }

    #[test]
    fn init_copies_config_props_into_context() {
        let service = fresh_service(
            test_props()
                .aspect_ratio(AspectRatio::Square)
                .zoom(2.0)
                .min_zoom(0.5)
                .max_zoom(4.0)
                .circular(true),
        );

        let ctx = service.context();

        assert_eq!(ctx.aspect_ratio, AspectRatio::Square);
        assert_eq!(ctx.zoom, 2.0);
        assert_eq!(ctx.min_zoom, 0.5);
        assert_eq!(ctx.max_zoom, 4.0);
        assert!(ctx.circular);
    }

    // ───────────────────────── transitions: drag ─────────────────────────

    #[test]
    fn drag_start_enters_dragging_and_snapshots_crop() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::DragStart { x: 0.5, y: 0.5 }));

        assert_eq!(service.state(), &State::Dragging);
        assert_eq!(service.context().drag_origin, Some((0.5, 0.5)));
        assert_eq!(service.context().drag_start_crop, Some(CropArea::default()));
    }

    #[test]
    fn drag_move_translates_crop_by_pointer_delta() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::DragStart { x: 0.5, y: 0.5 }));
        drop(service.send(Event::DragMove { x: 0.55, y: 0.45 }));

        let crop = *service.context().crop.get();

        assert!((crop.x - 0.15).abs() < 1e-9);
        assert!((crop.y - 0.05).abs() < 1e-9);
        // size preserved during a move
        assert_eq!(crop.width, 0.8);
        assert_eq!(crop.height, 0.8);
    }

    #[test]
    fn drag_move_clamps_crop_within_image_bounds() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::DragStart { x: 0.0, y: 0.0 }));
        // Push far beyond the right/bottom edge.
        drop(service.send(Event::DragMove { x: 1.0, y: 1.0 }));

        let crop = *service.context().crop.get();

        // width is 0.8, so x maxes at 1.0 - 0.8 = 0.2
        assert!((crop.x - 0.2).abs() < 1e-9);
        assert!((crop.y - 0.2).abs() < 1e-9);
    }

    #[test]
    fn drag_end_returns_to_idle_and_clears_drag_state() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::DragStart { x: 0.5, y: 0.5 }));
        drop(service.send(Event::DragEnd));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().drag_origin, None);
        assert_eq!(service.context().drag_start_crop, None);
    }

    #[test]
    fn drag_move_emits_announce_and_crop_change_effects() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::DragStart { x: 0.5, y: 0.5 }));

        let result = service.send(Event::DragMove { x: 0.6, y: 0.5 });

        assert_eq!(
            effect_names(&result),
            vec![Effect::AnnounceCropMoved, Effect::CropChange]
        );
    }

    // ───────────────────────── transitions: resize ─────────────────────────

    #[test]
    fn resize_start_enters_resizing_with_handle() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::ResizeStart {
            handle: CropHandle::BottomRight,
            x: 0.9,
            y: 0.9,
        }));

        assert_eq!(
            service.state(),
            &State::Resizing {
                handle: CropHandle::BottomRight
            }
        );
    }

    #[test]
    fn resize_bottom_right_grows_width_and_height() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::ResizeStart {
            handle: CropHandle::BottomRight,
            x: 0.9,
            y: 0.9,
        }));
        // default crop x=0.1 width=0.8 -> right edge at 0.9; shrink inward
        drop(service.send(Event::ResizeMove { x: 0.8, y: 0.8 }));

        let crop = *service.context().crop.get();

        assert!((crop.width - 0.7).abs() < 1e-9);
        assert!((crop.height - 0.7).abs() < 1e-9);
        // origin unchanged for bottom-right
        assert_eq!(crop.x, 0.1);
        assert_eq!(crop.y, 0.1);
    }

    #[test]
    fn resize_top_left_moves_origin_and_shrinks() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::ResizeStart {
            handle: CropHandle::TopLeft,
            x: 0.1,
            y: 0.1,
        }));
        drop(service.send(Event::ResizeMove { x: 0.2, y: 0.2 }));

        let crop = *service.context().crop.get();

        assert!((crop.x - 0.2).abs() < 1e-9);
        assert!((crop.y - 0.2).abs() < 1e-9);
        assert!((crop.width - 0.7).abs() < 1e-9);
        assert!((crop.height - 0.7).abs() < 1e-9);
    }

    #[test]
    fn resize_enforces_minimum_crop_size() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::ResizeStart {
            handle: CropHandle::BottomRight,
            x: 0.9,
            y: 0.9,
        }));
        // Collapse far past the origin.
        drop(service.send(Event::ResizeMove { x: 0.0, y: 0.0 }));

        let crop = *service.context().crop.get();

        assert!((crop.width - MIN_CROP_SIZE).abs() < 1e-9);
        assert!((crop.height - MIN_CROP_SIZE).abs() < 1e-9);
    }

    #[test]
    fn resize_each_handle_keeps_geometry_in_bounds() {
        for handle in CropHandle::all() {
            let mut service = fresh_service(test_props());

            drop(service.send(Event::ResizeStart {
                handle,
                x: 0.5,
                y: 0.5,
            }));
            drop(service.send(Event::ResizeMove { x: 0.55, y: 0.55 }));

            let crop = *service.context().crop.get();

            assert!(crop.x >= 0.0, "{handle:?}: x below 0");
            assert!(crop.y >= 0.0, "{handle:?}: y below 0");
            assert!(
                crop.width >= MIN_CROP_SIZE - 1e-9,
                "{handle:?}: width too small"
            );
            assert!(
                crop.height >= MIN_CROP_SIZE - 1e-9,
                "{handle:?}: height too small"
            );
            assert!(crop.x + crop.width <= 1.0 + 1e-9, "{handle:?}: overflow x");
            assert!(crop.y + crop.height <= 1.0 + 1e-9, "{handle:?}: overflow y");
        }
    }

    #[test]
    fn resize_with_aspect_reclamps_when_height_overflows_bottom() {
        // Portrait3x4 (ratio 0.75) makes height taller than width; growing the
        // width pushes the constrained height past the bottom edge, exercising
        // the re-clamp branch in `resize_crop_area`.
        let mut service = fresh_service(test_props().aspect_ratio(AspectRatio::Portrait3x4));

        drop(service.send(Event::ResizeStart {
            handle: CropHandle::BottomRight,
            x: 0.9,
            y: 0.9,
        }));
        drop(service.send(Event::ResizeMove { x: 0.85, y: 0.9 }));

        let crop = *service.context().crop.get();

        // Re-clamped so the bottom edge stays within the image.
        assert!(crop.y + crop.height <= 1.0 + 1e-9);
        // Ratio still honored after the re-clamp: width == height * 0.75.
        assert!((crop.width - crop.height * 0.75).abs() < 1e-9);
    }

    #[test]
    fn set_aspect_ratio_reclamps_when_height_overflows_bottom() {
        // Default crop (y=0.1, width=0.8) under Portrait3x4 would compute
        // height = 0.8 / 0.75 = 1.067, overflowing the bottom and hitting the
        // re-clamp branch in `enforce_aspect_ratio`.
        let mut service = fresh_service(test_props());

        drop(service.send(Event::SetAspectRatio(AspectRatio::Portrait3x4)));

        let crop = *service.context().crop.get();

        assert!(crop.y + crop.height <= 1.0 + 1e-9);
        assert!((crop.width - crop.height * 0.75).abs() < 1e-9);
    }

    #[test]
    fn unhandled_event_for_state_is_ignored() {
        // `DragMove` has no handler outside `Dragging`; it falls through to the
        // catch-all and leaves state and geometry untouched.
        let mut service = fresh_service(test_props());

        let before = *service.context().crop.get();

        drop(service.send(Event::DragMove { x: 0.5, y: 0.5 }));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(*service.context().crop.get(), before);
    }

    #[test]
    fn resize_locks_aspect_ratio_when_set() {
        let mut service = fresh_service(test_props().aspect_ratio(AspectRatio::Square));

        drop(service.send(Event::ResizeStart {
            handle: CropHandle::BottomRight,
            x: 0.9,
            y: 0.9,
        }));
        drop(service.send(Event::ResizeMove { x: 0.6, y: 0.9 }));

        let crop = *service.context().crop.get();

        // Square: height follows width.
        assert!((crop.width - crop.height).abs() < 1e-9);
    }

    #[test]
    fn resize_move_emits_announce_resized_and_crop_change() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::ResizeStart {
            handle: CropHandle::Right,
            x: 0.9,
            y: 0.5,
        }));

        let result = service.send(Event::ResizeMove { x: 0.8, y: 0.5 });

        assert_eq!(
            effect_names(&result),
            vec![Effect::AnnounceCropResized, Effect::CropChange]
        );
    }

    #[test]
    fn resize_end_returns_to_idle() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::ResizeStart {
            handle: CropHandle::Top,
            x: 0.5,
            y: 0.1,
        }));
        drop(service.send(Event::ResizeEnd));

        assert_eq!(service.state(), &State::Idle);
    }

    // ───────────────────────── transitions: setters ─────────────────────────

    #[test]
    fn set_crop_area_replaces_geometry() {
        let mut service = fresh_service(test_props());

        let area = CropArea {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            rotation: 0.0,
        };

        drop(service.send(Event::SetCropArea(area)));

        assert_eq!(*service.context().crop.get(), area);
    }

    #[test]
    fn set_aspect_ratio_reconstrains_height() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::SetAspectRatio(AspectRatio::Square)));

        let crop = *service.context().crop.get();

        assert_eq!(service.context().aspect_ratio, AspectRatio::Square);
        assert!((crop.width - crop.height).abs() < 1e-9);
    }

    #[test]
    fn set_zoom_clamps_to_bounds() {
        let mut service = fresh_service(test_props().min_zoom(1.0).max_zoom(3.0));

        drop(service.send(Event::SetZoom(10.0)));

        assert_eq!(service.context().zoom, 3.0);

        drop(service.send(Event::SetZoom(0.1)));

        assert_eq!(service.context().zoom, 1.0);
    }

    #[test]
    fn set_rotation_updates_crop_rotation() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::SetRotation(90.0)));

        assert_eq!(service.context().crop.get().rotation, 90.0);
    }

    #[test]
    fn flip_horizontal_and_vertical_toggle() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::FlipHorizontal));

        assert!(service.context().flip.horizontal);

        drop(service.send(Event::FlipHorizontal));

        assert!(!service.context().flip.horizontal);

        drop(service.send(Event::FlipVertical));

        assert!(service.context().flip.vertical);
    }

    #[test]
    fn reset_restores_defaults_and_returns_idle() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::SetZoom(2.0)));
        drop(service.send(Event::FlipHorizontal));
        drop(service.send(Event::SetRotation(45.0)));

        drop(service.send(Event::Reset));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(*service.context().crop.get(), CropArea::default());
        assert_eq!(service.context().zoom, 1.0);
        assert_eq!(service.context().flip, FlipState::default());
    }

    // ───────────────────────── transitions: nudge ─────────────────────────

    #[test]
    fn nudge_crop_moves_and_clamps() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::NudgeCrop {
            dx: 0.01,
            dy: -0.01,
        }));

        let crop = *service.context().crop.get();

        assert!((crop.x - 0.11).abs() < 1e-9);
        assert!((crop.y - 0.09).abs() < 1e-9);
    }

    #[test]
    fn nudge_crop_emits_announce_moved_and_crop_change() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::NudgeCrop { dx: 0.01, dy: 0.0 });

        assert_eq!(
            effect_names(&result),
            vec![Effect::AnnounceCropMoved, Effect::CropChange]
        );
    }

    // ───────────────────────── focus / disabled ─────────────────────────

    #[test]
    fn focus_and_blur_track_focused_part() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Focus {
            part: Part::CropArea,
        }));

        assert_eq!(service.context().focused_part, Some(Part::CropArea));

        drop(service.send(Event::Blur {
            part: Part::CropArea,
        }));

        assert_eq!(service.context().focused_part, None);
    }

    #[test]
    fn disabled_blocks_interaction_but_not_focus() {
        let mut service = fresh_service(test_props().disabled(true));

        // Interaction is ignored.
        drop(service.send(Event::DragStart { x: 0.5, y: 0.5 }));

        assert_eq!(service.state(), &State::Idle);

        // Focus still tracked.
        drop(service.send(Event::Focus {
            part: Part::CropArea,
        }));

        assert_eq!(service.context().focused_part, Some(Part::CropArea));
    }

    // ───────────────────────── prop sync ─────────────────────────

    #[test]
    fn sync_crop_updates_controlled_value_on_props_change() {
        let initial = CropArea {
            x: 0.1,
            y: 0.1,
            width: 0.5,
            height: 0.5,
            rotation: 0.0,
        };

        let mut service = fresh_service(test_props().crop(initial));

        let next = CropArea {
            x: 0.2,
            y: 0.2,
            width: 0.6,
            height: 0.6,
            rotation: 0.0,
        };

        drop(service.set_props(test_props().crop(next)));

        assert_eq!(*service.context().crop.get(), next);
    }

    #[test]
    fn sync_props_mirrors_config_and_cancels_active_interaction() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::DragStart { x: 0.5, y: 0.5 }));

        assert_eq!(service.state(), &State::Dragging);

        // Disable mid-drag via props.
        drop(service.set_props(test_props().disabled(true).zoom(2.0)));

        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().disabled);
        assert_eq!(service.context().zoom, 2.0);
        assert_eq!(service.context().drag_origin, None);
    }

    #[test]
    fn on_crop_change_fires_with_new_crop() {
        let log: Arc<Mutex<Vec<CropArea>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&log);
        let mut service = fresh_service(test_props().on_crop_change(ars_core::callback(
            move |crop: CropArea| {
                sink.lock().unwrap().push(crop);
            },
        )));

        send_run(&mut service, Event::NudgeCrop { dx: 0.01, dy: 0.0 });

        let recorded = log.lock().unwrap();

        assert_eq!(recorded.len(), 1);
        assert!((recorded[0].x - 0.11).abs() < 1e-9);
    }

    // ───────────────────────── Api accessors / handlers ─────────────────────────

    #[test]
    fn connect_exposes_api_accessors() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::SetZoom(2.0)));
        drop(service.send(Event::FlipHorizontal));

        let api = service.connect(&|_| {});

        assert_eq!(api.crop(), CropArea::default());
        assert_eq!(api.zoom(), 2.0);
        assert!(api.flip().horizontal);

        // The Debug impl is exercised so it stays compilable for diagnostics.
        assert!(format!("{api:?}").contains("image_cropper::Api"));
    }

    #[test]
    fn api_state_accessors_reflect_state() {
        assert!(
            Fixture {
                state: State::Dragging,
                ..Default::default()
            }
            .api()
            .is_dragging()
        );
        assert!(
            Fixture {
                state: State::Resizing {
                    handle: CropHandle::Top
                },
                ..Default::default()
            }
            .api()
            .is_resizing()
        );
    }

    #[test]
    fn api_result_reflects_zoom_and_flip() {
        let result = Fixture {
            zoom: 2.0,
            flip: FlipState {
                horizontal: true,
                vertical: false,
            },
            ..Default::default()
        }
        .api()
        .result();

        assert_eq!(result.scale, 2.0);
        assert!(result.flip.horizontal);
    }

    #[test]
    fn crop_area_keydown_nudges_with_step_and_rtl() {
        let log: Arc<Mutex<Vec<Event>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&log);
        let props = Box::leak(Box::new(test_props()));

        let (_, ctx) = Machine::init(props, &Env::default(), &Messages::default());

        let ctx = Box::leak(Box::new(ctx));

        let state = Box::leak(Box::new(State::Idle));

        let send = Box::leak(Box::new(move |event: Event| {
            sink.lock().unwrap().push(event);
        }));

        let api = Api {
            state,
            ctx,
            props,
            send,
        };

        api.on_crop_area_keydown(&key(KeyboardKey::ArrowRight), false, false);
        api.on_crop_area_keydown(&key(KeyboardKey::ArrowUp), true, false);
        api.on_crop_area_keydown(&key(KeyboardKey::ArrowRight), false, true);
        api.on_crop_area_keydown(&key(KeyboardKey::ArrowLeft), false, false);
        api.on_crop_area_keydown(&key(KeyboardKey::ArrowDown), false, false);
        // A non-arrow key is ignored.
        api.on_crop_area_keydown(&key(KeyboardKey::Enter), false, false);

        let events = log.lock().unwrap();

        assert_eq!(events.len(), 5);
        assert_eq!(events[0], Event::NudgeCrop { dx: 0.01, dy: 0.0 });
        assert_eq!(events[1], Event::NudgeCrop { dx: 0.0, dy: -0.1 });
        // RTL flips ArrowRight to the inline-end (negative dx).
        assert_eq!(events[2], Event::NudgeCrop { dx: -0.01, dy: 0.0 });
        assert_eq!(events[3], Event::NudgeCrop { dx: -0.01, dy: 0.0 });
        assert_eq!(events[4], Event::NudgeCrop { dx: 0.0, dy: 0.01 });
    }

    #[test]
    fn root_keydown_maps_character_shortcuts() {
        let log: Arc<Mutex<Vec<Event>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&log);

        let props = Box::leak(Box::new(test_props()));

        let (_, ctx) = Machine::init(props, &Env::default(), &Messages::default());

        let ctx = Box::leak(Box::new(ctx));

        let state = Box::leak(Box::new(State::Idle));

        let send = Box::leak(Box::new(move |event: Event| {
            sink.lock().unwrap().push(event);
        }));

        let api = Api {
            state,
            ctx,
            props,
            send,
        };

        api.on_root_keydown(&char_key('+'));
        api.on_root_keydown(&char_key('-'));
        api.on_root_keydown(&char_key('r'));
        api.on_root_keydown(&char_key('R'));
        api.on_root_keydown(&char_key('h'));
        api.on_root_keydown(&char_key('v'));
        // An unmapped character is ignored.
        api.on_root_keydown(&char_key('z'));
        // A named key with no character is ignored.
        api.on_root_keydown(&key(KeyboardKey::Enter));

        let events = log.lock().unwrap();

        assert_eq!(events.len(), 6);
        assert!(matches!(events[0], Event::SetZoom(_)));
        assert!(matches!(events[2], Event::SetRotation(rotation) if rotation > 0.0));
        assert!(matches!(events[3], Event::SetRotation(rotation) if rotation < 0.0));
        assert_eq!(events[4], Event::FlipHorizontal);
        assert_eq!(events[5], Event::FlipVertical);
    }

    #[test]
    fn pointer_and_reset_dispatchers_emit_events() {
        let log: Arc<Mutex<Vec<Event>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&log);

        let props = Box::leak(Box::new(test_props()));

        let (_, ctx) = Machine::init(props, &Env::default(), &Messages::default());

        let ctx = Box::leak(Box::new(ctx));

        let state = Box::leak(Box::new(State::Idle));

        let send = Box::leak(Box::new(move |event: Event| {
            sink.lock().unwrap().push(event);
        }));

        let api = Api {
            state,
            ctx,
            props,
            send,
        };

        api.on_crop_area_pointer_down(0.3, 0.4);
        api.on_handle_pointer_down(CropHandle::Top, 0.5, 0.1);
        api.on_reset();

        let events = log.lock().unwrap();

        assert_eq!(events[0], Event::DragStart { x: 0.3, y: 0.4 });
        assert_eq!(
            events[1],
            Event::ResizeStart {
                handle: CropHandle::Top,
                x: 0.5,
                y: 0.1,
            }
        );
        assert_eq!(events[2], Event::Reset);
    }

    #[test]
    fn part_attrs_dispatches_every_part() {
        let api = Fixture::default().api();

        // Every anatomy part dispatches through `ConnectApi::part_attrs` without
        // panicking and emits the component scope marker.
        for part in Part::all() {
            let attrs = api.part_attrs(part);

            assert_eq!(
                attrs.get(&HtmlAttr::Data("ars-scope")),
                Some("image-cropper")
            );
        }
    }

    // ───────────────────────── snapshots ─────────────────────────

    #[test]
    fn snapshot_root_idle() {
        assert_snapshot!(snapshot_attrs(&Fixture::default().api().root_attrs()));
    }

    #[test]
    fn snapshot_root_dragging() {
        assert_snapshot!(snapshot_attrs(
            &Fixture {
                state: State::Dragging,
                ..Default::default()
            }
            .api()
            .root_attrs()
        ));
    }

    #[test]
    fn snapshot_root_resizing() {
        assert_snapshot!(snapshot_attrs(
            &Fixture {
                state: State::Resizing {
                    handle: CropHandle::BottomRight
                },
                ..Default::default()
            }
            .api()
            .root_attrs()
        ));
    }

    #[test]
    fn snapshot_root_disabled() {
        assert_snapshot!(snapshot_attrs(
            &Fixture {
                disabled: true,
                ..Default::default()
            }
            .api()
            .root_attrs()
        ));
    }

    #[test]
    fn snapshot_root_circular() {
        assert_snapshot!(snapshot_attrs(
            &Fixture {
                circular: true,
                ..Default::default()
            }
            .api()
            .root_attrs()
        ));
    }

    #[test]
    fn snapshot_image_default() {
        assert_snapshot!(snapshot_attrs(&Fixture::default().api().image_attrs()));
    }

    #[test]
    fn snapshot_image_transformed() {
        assert_snapshot!(snapshot_attrs(
            &Fixture {
                zoom: 2.0,
                crop: CropArea {
                    rotation: 90.0,
                    ..CropArea::default()
                },
                flip: FlipState {
                    horizontal: true,
                    vertical: true,
                },
                ..Default::default()
            }
            .api()
            .image_attrs()
        ));
    }

    #[test]
    fn snapshot_overlay() {
        assert_snapshot!(snapshot_attrs(&Fixture::default().api().overlay_attrs()));
    }

    #[test]
    fn snapshot_crop_area_idle() {
        assert_snapshot!(snapshot_attrs(&Fixture::default().api().crop_area_attrs()));
    }

    #[test]
    fn snapshot_crop_area_dragging() {
        assert_snapshot!(snapshot_attrs(
            &Fixture {
                state: State::Dragging,
                ..Default::default()
            }
            .api()
            .crop_area_attrs()
        ));
    }

    #[test]
    fn snapshot_crop_area_focused() {
        assert_snapshot!(snapshot_attrs(
            &Fixture {
                focused_part: Some(Part::CropArea),
                ..Default::default()
            }
            .api()
            .crop_area_attrs()
        ));
    }

    #[test]
    fn snapshot_grid() {
        assert_snapshot!(snapshot_attrs(&Fixture::default().api().grid_attrs()));
    }

    #[test]
    fn snapshot_handle_top_left_idle() {
        assert_snapshot!(snapshot_attrs(
            &Fixture::default().api().handle_attrs(CropHandle::TopLeft)
        ));
    }

    #[test]
    fn snapshot_handle_active_when_resizing() {
        assert_snapshot!(snapshot_attrs(
            &Fixture {
                state: State::Resizing {
                    handle: CropHandle::BottomRight
                },
                ..Default::default()
            }
            .api()
            .handle_attrs(CropHandle::BottomRight)
        ));
    }

    #[test]
    fn snapshot_zoom_slider() {
        assert_snapshot!(snapshot_attrs(
            &Fixture {
                zoom: 1.5,
                ..Default::default()
            }
            .api()
            .zoom_slider_attrs()
        ));
    }

    #[test]
    fn snapshot_rotation_slider() {
        assert_snapshot!(snapshot_attrs(
            &Fixture {
                crop: CropArea {
                    rotation: 45.0,
                    ..CropArea::default()
                },
                ..Default::default()
            }
            .api()
            .rotation_slider_attrs()
        ));
    }

    #[test]
    fn snapshot_reset_trigger_enabled() {
        assert_snapshot!(snapshot_attrs(
            &Fixture::default().api().reset_trigger_attrs()
        ));
    }

    #[test]
    fn snapshot_reset_trigger_disabled() {
        assert_snapshot!(snapshot_attrs(
            &Fixture {
                disabled: true,
                ..Default::default()
            }
            .api()
            .reset_trigger_attrs()
        ));
    }

    #[test]
    fn snapshot_label() {
        assert_snapshot!(snapshot_attrs(&Fixture::default().api().label_attrs()));
    }
}
