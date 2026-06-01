//! `ColorPicker` complex orchestrator state machine and connect API.
//!
//! `ColorPicker` composes a popover-anchored color-editing surface: a 2D
//! saturation/lightness area, a hue channel slider, an optional alpha slider,
//! per-channel text inputs, a hex input, runtime format switching
//! (hex/rgb/hsl/hsb), runtime color-space switching, preset swatches, and an
//! optional browser `EyeDropper` integration. The agnostic core owns the
//! composite color value, the channel math, the popover open/drag lifecycle,
//! and every ARIA / `data-ars-*` attribute.
//!
//! Live element measurement, pointer capture, popup positioning, click-outside
//! containment, focus restoration, and the EyeDropper browser API all belong to
//! the framework adapter. The adapter supplies already-normalized `(x, y)` in
//! `0..=1` through [`Api::on_area_pointer_down`] / [`Event::DragMove`], reports
//! browser `EyeDropper` support back via [`Event::SetEyedropperSupported`], and
//! resolves the typed [`Effect`] intents emitted on the open/close lifecycle.
//! The string IDs in [`Context`] exist purely for ARIA wiring and
//! hydration-stable `id` attributes, never as a substitute for live handles.
//!
//! # Examples
//!
//! ```
//! use ars_components::specialized::color_picker::{Event, Machine, Messages, Props};
//! use ars_core::{ColorValue, Env, Service};
//!
//! let mut picker = Service::<Machine>::new(
//!     Props { id: "demo".into(), ..Props::default() },
//!     &Env::default(),
//!     &Messages::default(),
//! );
//!
//! // Open the popover; the trigger now reports `aria-expanded="true"`.
//! drop(picker.send(Event::Open));
//! assert!(picker.connect(&|_| {}).is_open());
//!
//! // Set a color (e.g. from the hex input) and read it back in the active format.
//! drop(picker.send(Event::SetColor(ColorValue::from_hsl(0.0, 1.0, 0.5))));
//! assert_eq!(picker.connect(&|_| {}).value_as_string(), "#ff0000");
//! ```

use alloc::{
    format,
    string::{String, ToString as _},
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ColorChannel, ColorFormat, ColorSpace, ColorValue,
    ComponentIds, ComponentMessages, ComponentPart, ConnectApi, CssProperty, Direction, DragTarget,
    Env, HtmlAttr, Locale, MessageFn, PendingEffect, TransitionPlan, channel_range,
    channel_step_default, channel_value, format_color_string, no_cleanup, with_channel,
};
use ars_interactions::{KeyboardEventData, KeyboardKey};

use crate::overlay::positioning::{Offset, Placement, PositioningOptions};

// ────────────────────────────────────────────────────────────────────
// Message function type aliases
// ────────────────────────────────────────────────────────────────────

/// A locale-only label message (`Fn(&Locale) -> String`).
type LabelFn = dyn Fn(&Locale) -> String + Send + Sync;

/// A swatch / color-name message (`Fn(&ColorValue, &Locale) -> String`).
type ColorLabelFn = dyn Fn(&ColorValue, &Locale) -> String + Send + Sync;

/// The debounced color announcement (`Fn(&ColorValue, ColorFormat, &Locale) -> String`).
type ColorAnnouncementFn = dyn Fn(&ColorValue, ColorFormat, &Locale) -> String + Send + Sync;

/// Channel `aria-valuetext` formatter (`Fn(label, value, unit, &Locale) -> String`).
type ChannelValueTextFn = dyn Fn(&str, &str, &str, &Locale) -> String + Send + Sync;

/// Color-space-switch announcement (`Fn(space, &Locale) -> String`).
type ColorSpaceSwitchedFn = dyn Fn(&str, &Locale) -> String + Send + Sync;

/// Consumer callback fired once when a drag interaction ends.
type ChangeEndFn = dyn Fn(ColorValue) + Send + Sync;

// ────────────────────────────────────────────────────────────────────
// State / Event / Effect
// ────────────────────────────────────────────────────────────────────

/// The states for the `ColorPicker` component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// Picker is closed (trigger visible, content hidden).
    Closed,

    /// Picker is open, user is not actively dragging.
    Open,

    /// User is dragging a thumb (area or channel slider).
    Dragging {
        /// The target of the drag.
        target: DragTarget,
    },
}

/// The events for the `ColorPicker` component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Open the picker popover.
    Open,

    /// Close the picker popover.
    Close,

    /// Toggle open/closed.
    Toggle,

    /// User started dragging a thumb (area or channel slider). `x`/`y` are
    /// adapter-normalized to `0..=1` relative to the dragged surface.
    DragStart {
        /// The target of the drag.
        target: DragTarget,

        /// Normalized x coordinate (`0..=1`).
        x: f64,

        /// Normalized y coordinate (`0..=1`).
        y: f64,
    },

    /// User is moving while dragging (adapter-normalized coordinates).
    DragMove {
        /// Normalized x coordinate (`0..=1`).
        x: f64,

        /// Normalized y coordinate (`0..=1`).
        y: f64,
    },

    /// User released the drag.
    DragEnd,

    /// Set the color value directly (e.g., from text input or a swatch).
    SetColor(ColorValue),

    /// Set an individual channel value (from a channel input).
    SetChannel {
        /// The channel to update.
        channel: ColorChannel,

        /// The new channel value, in channel units.
        value: f64,
    },

    /// Switch the displayed text format.
    SetFormat(ColorFormat),

    /// Switch the active color space. Triggers value recomputation.
    ChangeColorSpace(ColorSpace),

    /// Eyedropper sampling requested by the user.
    EyedropperRequest,

    /// Eyedropper result reported by the adapter (`None` on cancel).
    EyedropperResult(Option<ColorValue>),

    /// Adapter-reported browser `EyeDropper` API availability.
    SetEyedropperSupported(bool),

    /// Focus entered a part.
    Focus {
        /// The part name that received focus.
        part: &'static str,
    },

    /// Focus left a part.
    Blur {
        /// The part name that lost focus.
        part: &'static str,
    },

    /// Keyboard channel adjustment (increment).
    ChannelIncrement {
        /// The channel to increment.
        channel: ColorChannel,

        /// The step amount, in channel units.
        step: f64,
    },

    /// Keyboard channel adjustment (decrement).
    ChannelDecrement {
        /// The channel to decrement.
        channel: ColorChannel,

        /// The step amount, in channel units.
        step: f64,
    },

    /// Close on interact outside (suppressed while dragging).
    CloseOnInteractOutside,

    /// Close on Escape.
    CloseOnEscape,

    /// Controlled-value sync from the parent after `Service::set_props`.
    SyncValue(Option<ColorValue>),

    /// Refresh cached output props after `Service::set_props`.
    SetProps,
}

/// Typed identifier for the named side effects emitted by the `ColorPicker`
/// machine. Adapters dispatch on these names exhaustively; the agnostic core
/// never touches the DOM or browser APIs itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Invoke [`Props::on_change_end`] with the final color (fired on `DragEnd`).
    ChangeEnd,

    /// Attach the click-outside listener (fired on `Closed → Open` and a
    /// non-`Closed` initial mount). The adapter dispatches
    /// [`Event::CloseOnInteractOutside`] when an outside interaction occurs.
    AttachClickOutside,

    /// Detach the click-outside listener (fired on `Open → Closed`).
    DetachClickOutside,

    /// Detect browser `EyeDropper` support (fired on `Closed → Open` and a
    /// non-`Closed` initial mount). The adapter performs the
    /// `"EyeDropper" in window` check and reports the result via
    /// [`Event::SetEyedropperSupported`].
    DetectEyedropper,

    /// Open the browser `EyeDropper` (fired on `EyedropperRequest`). The adapter
    /// calls `EyeDropper.open()` from the originating user gesture and reports
    /// the outcome via [`Event::EyedropperResult`].
    InvokeEyedropper,

    /// Announce the active color space change via an `aria-live` region. The
    /// adapter reads the announcement text from [`Api::color_space_announcement`].
    AnnounceColorSpace,
}

// ────────────────────────────────────────────────────────────────────
// Context / Props / Messages
// ────────────────────────────────────────────────────────────────────

/// The context for the `ColorPicker` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The current color value (controlled or uncontrolled).
    pub value: Bindable<ColorValue>,

    /// Whether the picker popover is open (controlled or uncontrolled).
    pub open: Bindable<bool>,

    /// The currently displayed text format.
    pub format: ColorFormat,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is read-only.
    pub readonly: bool,

    /// Whether to close the picker when the user interacts outside.
    pub close_on_interact_outside: bool,

    /// Whether to close the picker when Escape is pressed.
    pub close_on_escape: bool,

    /// Whether to show the alpha channel slider and input.
    pub show_alpha: bool,

    /// Whether the browser `EyeDropper` API is available (adapter-detected).
    pub eyedropper_supported: bool,

    /// Currently focused part name (or `None`).
    pub focused_part: Option<&'static str>,

    /// Positioning options for the popover.
    pub positioning: PositioningOptions,

    /// Keyboard step for channel adjustments.
    pub channel_step: f64,

    /// Large step (Shift+Arrow or PageUp/PageDown).
    pub channel_large_step: f64,

    /// Active color space for the picker controls.
    pub color_space: ColorSpace,

    /// Preset swatch colors rendered in the swatch group.
    pub swatches: Vec<ColorValue>,

    /// Text direction for RTL-aware keyboard navigation and layout.
    pub dir: Direction,

    /// Locale for internationalized messages.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component instance IDs.
    pub ids: ComponentIds,
}

impl Context {
    /// Returns the channels available in the current color space, in display
    /// order. Used by adapters to lay out the per-channel inputs.
    #[must_use]
    pub const fn channels(&self) -> &'static [ColorChannel] {
        match self.color_space {
            ColorSpace::Rgb => &[ColorChannel::Red, ColorChannel::Green, ColorChannel::Blue],

            ColorSpace::Hsl | ColorSpace::Hwb => &[
                ColorChannel::Hue,
                ColorChannel::Saturation,
                ColorChannel::Lightness,
            ],

            ColorSpace::Hsb => &[
                ColorChannel::Hue,
                ColorChannel::Saturation,
                ColorChannel::Brightness,
            ],
        }
    }
}

/// The props for the `ColorPicker` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled value. When `Some`, the component is controlled.
    pub value: Option<ColorValue>,

    /// Default value for uncontrolled mode.
    pub default_value: ColorValue,

    /// Controlled open state. When `Some`, open state is controlled.
    pub open: Option<bool>,

    /// Default open state for uncontrolled mode.
    pub default_open: bool,

    /// Disabled state.
    pub disabled: bool,

    /// Read-only state.
    pub readonly: bool,

    /// Close on interact outside the popover.
    pub close_on_interact_outside: bool,

    /// Close on Escape.
    pub close_on_escape: bool,

    /// Show the alpha channel slider and input.
    pub show_alpha: bool,

    /// Initial format for the text inputs.
    pub default_format: ColorFormat,

    /// Positioning options for the popover.
    pub positioning: PositioningOptions,

    /// Step size for keyboard channel adjustment.
    pub channel_step: f64,

    /// Large step size for keyboard channel adjustment (Shift+Arrow).
    pub channel_large_step: f64,

    /// Color space for the picker controls. Default: [`ColorSpace::Hsl`].
    pub color_space: ColorSpace,

    /// Preset swatch colors rendered in the swatch group.
    pub swatches: Vec<ColorValue>,

    /// Text direction for RTL-aware keyboard navigation and layout.
    pub dir: Direction,

    /// Name attribute for the hidden form input.
    pub name: Option<String>,

    /// Callback fired once when a drag interaction ends (pointer release on the
    /// area or a channel slider). Use for expensive operations like persisting.
    pub on_change_end: Option<Callback<ChangeEndFn>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: ColorValue::default(),
            open: None,
            default_open: false,
            disabled: false,
            readonly: false,
            close_on_interact_outside: true,
            close_on_escape: true,
            show_alpha: true,
            default_format: ColorFormat::Hex,
            positioning: PositioningOptions {
                placement: Placement::BottomStart,
                offset: Offset {
                    main_axis: 4.0,
                    cross_axis: 0.0,
                },
                ..Default::default()
            },
            channel_step: 1.0,
            channel_large_step: 10.0,
            color_space: ColorSpace::default(),
            swatches: Vec::new(),
            dir: Direction::Ltr,
            name: None,
            on_change_end: None,
        }
    }
}

/// The translatable messages for the `ColorPicker` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// `aria-label` for the trigger button. Default: `"Pick a color"`.
    pub trigger_label: MessageFn<LabelFn>,

    /// `aria-label` for the 2D area thumb. Default: `"Color area selector"`.
    pub area_label: MessageFn<LabelFn>,

    /// `aria-roledescription` for the area thumb. Default: `"color area"`.
    pub area_role_description: MessageFn<LabelFn>,

    /// Label for the hue channel slider. Default: `"Hue"`.
    pub hue_label: MessageFn<LabelFn>,

    /// Label for the alpha channel slider. Default: `"Alpha"`.
    pub alpha_label: MessageFn<LabelFn>,

    /// Label for the saturation channel. Default: `"Saturation"`.
    pub saturation_label: MessageFn<LabelFn>,

    /// Label for the lightness channel. Default: `"Lightness"`.
    pub lightness_label: MessageFn<LabelFn>,

    /// `aria-label` for the eyedropper trigger. Default: `"Pick color from screen"`.
    pub eyedropper_label: MessageFn<LabelFn>,

    /// `aria-label` for the format selector. Default: `"Toggle color format"`.
    pub format_toggle_label: MessageFn<LabelFn>,

    /// `aria-label` for a preset swatch. Default: `"Select color {hex}"`.
    pub swatch_label: MessageFn<ColorLabelFn>,

    /// Human-readable color name for accessibility. Default: English perceptual name.
    pub color_name: MessageFn<ColorLabelFn>,

    /// Debounced color announcement for keyboard adjustments. Default: `"Color: {value}"`.
    pub color_announcement: MessageFn<ColorAnnouncementFn>,

    /// Formats a channel `aria-valuetext` from `(label, value, unit)`.
    pub channel_value_text: MessageFn<ChannelValueTextFn>,

    /// Color-space-switch announcement. Default: `"Switched to {space} color space"`.
    pub color_space_switched: MessageFn<ColorSpaceSwitchedFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            trigger_label: MessageFn::static_str("Pick a color"),
            area_label: MessageFn::static_str("Color area selector"),
            area_role_description: MessageFn::static_str("color area"),
            hue_label: MessageFn::static_str("Hue"),
            alpha_label: MessageFn::static_str("Alpha"),
            saturation_label: MessageFn::static_str("Saturation"),
            lightness_label: MessageFn::static_str("Lightness"),
            eyedropper_label: MessageFn::static_str("Pick color from screen"),
            format_toggle_label: MessageFn::static_str("Toggle color format"),
            swatch_label: MessageFn::new(|color: &ColorValue, _locale: &Locale| {
                format!("Select color {}", color.to_hex(false))
            }),
            color_name: MessageFn::new(|color: &ColorValue, _locale: &Locale| {
                color.color_name_en()
            }),
            color_announcement: MessageFn::new(
                |color: &ColorValue, format: ColorFormat, _locale: &Locale| {
                    format!("Color: {}", format_color_string(color, format))
                },
            ),
            channel_value_text: MessageFn::new(
                |label: &str, value: &str, _unit: &str, _locale: &Locale| {
                    format!("{label}: {value}")
                },
            ),
            color_space_switched: MessageFn::new(|space: &str, _locale: &Locale| {
                format!("Switched to {space} color space")
            }),
        }
    }
}

impl ComponentMessages for Messages {}

// ────────────────────────────────────────────────────────────────────
// Machine helpers
// ────────────────────────────────────────────────────────────────────

/// Apply an adapter-normalized pointer position to the color value for the
/// given drag target.
///
/// For [`DragTarget::Area`] the x-axis maps to saturation `[0, 1]` and the
/// y-axis maps to lightness `[1, 0]` (top = lightest). For
/// [`DragTarget::Channel`] the x-axis maps across the channel's full range.
/// Both base the new color on the *pending* value so a controlled drag-in-flight
/// accumulates rather than re-reading the stale controlled prop.
fn apply_drag_position(ctx: &mut Context, target: DragTarget, x: f64, y: f64) {
    let current = *ctx.value.pending();

    match target {
        DragTarget::Area => {
            let saturation = x.clamp(0.0, 1.0);
            let lightness = (1.0 - y).clamp(0.0, 1.0);

            ctx.value.set(ColorValue::new(
                current.hue,
                saturation,
                lightness,
                current.alpha,
            ));
        }

        DragTarget::Channel(channel) => {
            let (min, max) = channel_range(channel);
            let value = min + x.clamp(0.0, 1.0) * (max - min);

            ctx.value.set(with_channel(&current, channel, value));
        }
    }
}

/// Build the change-end effect that invokes [`Props::on_change_end`] with the
/// pending color value (the color staged during the drag).
fn change_end_effect() -> PendingEffect<Machine> {
    PendingEffect::new(Effect::ChangeEnd, |ctx: &Context, props: &Props, _send| {
        if let Some(callback) = &props.on_change_end {
            callback(*ctx.value.pending());
        }

        no_cleanup()
    })
}

/// The named effect intents produced by the open lifecycle. Shared by
/// `open_plan` (the `Closed → Open` path) and `Machine::initial_effects` (the
/// booted-open path) so the two entry points stay in lock-step.
fn open_lifecycle_effects() -> [PendingEffect<Machine>; 2] {
    [
        PendingEffect::named(Effect::AttachClickOutside),
        PendingEffect::named(Effect::DetectEyedropper),
    ]
}

/// The `Closed → Open` transition plan, shared by `Open` and `Toggle`.
fn open_plan() -> TransitionPlan<Machine> {
    let mut plan = TransitionPlan::to(State::Open).apply(|ctx: &mut Context| {
        ctx.open.set(true);
    });

    for effect in open_lifecycle_effects() {
        plan = plan.with_effect(effect);
    }

    plan
}

/// The `Open → Closed` transition plan, shared by every close path.
fn close_plan() -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Closed)
        .apply(|ctx: &mut Context| {
            ctx.open.set(false);
        })
        .with_effect(PendingEffect::named(Effect::DetachClickOutside))
}

/// Whether any context-backed prop that `SetProps` owns changed and the context
/// needs refreshing. Excludes `value`/`open` (driven by `SyncValue`/`Open`/
/// `Close`) and `color_space` (driven by `ChangeColorSpace`), each of which has
/// its own dedicated sync path so an unrelated `SetProps` cannot clobber a
/// runtime change.
fn context_relevant_props_changed(old: &Props, new: &Props) -> bool {
    old.disabled != new.disabled
        || old.readonly != new.readonly
        || old.close_on_interact_outside != new.close_on_interact_outside
        || old.close_on_escape != new.close_on_escape
        || old.show_alpha != new.show_alpha
        || old.swatches != new.swatches
        || old.dir != new.dir
        || old.positioning != new.positioning
        || (old.channel_step - new.channel_step).abs() > f64::EPSILON
        || (old.channel_large_step - new.channel_large_step).abs() > f64::EPSILON
}

// ────────────────────────────────────────────────────────────────────
// Machine
// ────────────────────────────────────────────────────────────────────

/// The machine for the `ColorPicker` component.
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
        let value = if let Some(color) = props.value {
            Bindable::controlled(color)
        } else {
            Bindable::uncontrolled(props.default_value)
        };

        let open = if let Some(open) = props.open {
            Bindable::controlled(open)
        } else {
            Bindable::uncontrolled(props.default_open)
        };

        let state = if *open.get() {
            State::Open
        } else {
            State::Closed
        };

        let context = Context {
            value,
            open,
            format: props.default_format,
            disabled: props.disabled,
            readonly: props.readonly,
            close_on_interact_outside: props.close_on_interact_outside,
            close_on_escape: props.close_on_escape,
            show_alpha: props.show_alpha,
            eyedropper_supported: false,
            focused_part: None,
            positioning: props.positioning.clone(),
            channel_step: props.channel_step,
            channel_large_step: props.channel_large_step,
            color_space: props.color_space,
            swatches: props.swatches.clone(),
            dir: props.dir,
            locale: env.locale.clone(),
            messages: messages.clone(),
            ids: ComponentIds::from_id(&props.id),
        };

        (state, context)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // A disabled picker ignores user interaction but still tracks focus and
        // accepts parent-driven syncs (so it can be re-enabled). Controlled
        // `open` changes arrive as `Open`/`Close` (see `on_props_changed`), so
        // those pass through too — otherwise a disabled, controlled picker could
        // never be opened or closed by its parent.
        if ctx.disabled {
            match event {
                Event::Open
                | Event::Close
                | Event::Focus { .. }
                | Event::Blur { .. }
                | Event::SyncValue(_)
                | Event::SetProps => {}
                _ => return None,
            }
        }

        match (state, event) {
            // --- Open / close lifecycle ---
            (State::Closed, Event::Open | Event::Toggle) => Some(open_plan()),

            // An explicit close request is honored from `Dragging` too: a
            // parent-controlled `open: false`, `Api::close()`, `Toggle`, or
            // Escape must abandon the in-flight drag and close rather than leave
            // the picker stuck open. (Interact-outside stays suppressed during
            // pointer capture — see the dedicated arm below.)
            (State::Open | State::Dragging { .. }, Event::Close | Event::Toggle) => {
                Some(close_plan())
            }

            (State::Open, Event::CloseOnInteractOutside) if ctx.close_on_interact_outside => {
                Some(close_plan())
            }

            // While dragging, interact-outside is suppressed: the user is still
            // interacting with the picker through pointer capture. This arm is
            // kept explicit (rather than folded into the catch-all) so the
            // suppression is a documented, intentional decision.
            #[expect(
                clippy::match_same_arms,
                reason = "explicit drag-suppression arm documents intent; sharing a body with the catch-all is incidental"
            )]
            (State::Dragging { .. }, Event::CloseOnInteractOutside) => None,

            (State::Open | State::Dragging { .. }, Event::CloseOnEscape) if ctx.close_on_escape => {
                Some(close_plan())
            }

            // --- Drag lifecycle ---
            (State::Open, Event::DragStart { target, x, y }) => {
                if ctx.readonly {
                    return None;
                }

                let (target, x, y) = (*target, *x, *y);
                Some(TransitionPlan::to(State::Dragging { target }).apply(
                    move |ctx: &mut Context| {
                        apply_drag_position(ctx, target, x, y);
                    },
                ))
            }

            (State::Dragging { target }, Event::DragMove { x, y }) => {
                if ctx.readonly {
                    return None;
                }

                let (target, x, y) = (*target, *x, *y);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    apply_drag_position(ctx, target, x, y);
                }))
            }

            (State::Dragging { .. }, Event::DragEnd) => {
                Some(TransitionPlan::to(State::Open).with_effect(change_end_effect()))
            }

            // --- Value editing (valid in any state so text inputs work) ---
            (_, Event::SetColor(color)) => {
                if ctx.readonly {
                    return None;
                }

                let color = *color;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.value.set(color);
                }))
            }

            (State::Open, Event::SetChannel { channel, value }) => {
                if ctx.readonly {
                    return None;
                }

                let (channel, value) = (*channel, *value);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let color = *ctx.value.pending();
                    ctx.value.set(with_channel(&color, channel, value));
                }))
            }

            (_, Event::SetFormat(format)) => {
                let format = *format;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.format = format;
                }))
            }

            (_, Event::ChangeColorSpace(new_space)) => {
                let new_space = *new_space;
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.color_space = new_space;
                    })
                    .with_effect(PendingEffect::named(Effect::AnnounceColorSpace)),
                )
            }

            (State::Open, Event::ChannelIncrement { channel, step }) => {
                if ctx.readonly {
                    return None;
                }

                let (channel, step) = (*channel, *step);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let color = *ctx.value.pending();
                    let (_, max) = channel_range(channel);
                    let next = (channel_value(&color, channel) + step).min(max);

                    ctx.value.set(with_channel(&color, channel, next));
                }))
            }

            (State::Open, Event::ChannelDecrement { channel, step }) => {
                if ctx.readonly {
                    return None;
                }

                let (channel, step) = (*channel, *step);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let color = *ctx.value.pending();
                    let (min, _) = channel_range(channel);
                    let next = (channel_value(&color, channel) - step).max(min);

                    ctx.value.set(with_channel(&color, channel, next));
                }))
            }

            // --- Eyedropper ---
            (State::Open, Event::EyedropperRequest) => {
                if !ctx.eyedropper_supported || ctx.readonly {
                    return None;
                }

                Some(
                    TransitionPlan::context_only(|_ctx: &mut Context| {})
                        .with_effect(PendingEffect::named(Effect::InvokeEyedropper)),
                )
            }

            (_, Event::EyedropperResult(Some(color))) => {
                if ctx.readonly {
                    return None;
                }

                let color = *color;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.value.set(color);
                }))
            }

            (_, Event::SetEyedropperSupported(supported)) => {
                let supported = *supported;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.eyedropper_supported = supported;
                }))
            }

            // --- Focus tracking ---
            (_, Event::Focus { part }) => {
                let part = *part;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused_part = Some(part);
                }))
            }

            (_, Event::Blur { .. }) => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.focused_part = None;
            })),

            // --- Parent-driven syncs ---
            (_, Event::SyncValue(value)) => {
                let value = *value;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    if let Some(color) = value {
                        ctx.value.set(color);
                    }

                    ctx.value.sync_controlled(value);
                }))
            }

            (_, Event::SetProps) => {
                let props = props.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.disabled = props.disabled;
                    ctx.readonly = props.readonly;
                    ctx.close_on_interact_outside = props.close_on_interact_outside;
                    ctx.close_on_escape = props.close_on_escape;
                    ctx.show_alpha = props.show_alpha;
                    // `color_space` is intentionally NOT synced here — it is
                    // owned by `ChangeColorSpace` (prop changes are routed
                    // through that event in `on_props_changed`) so an unrelated
                    // prop update cannot revert a runtime color-space switch.
                    ctx.swatches = props.swatches;
                    ctx.dir = props.dir;
                    ctx.positioning = props.positioning;
                    ctx.channel_step = props.channel_step;
                    ctx.channel_large_step = props.channel_large_step;
                }))
            }

            _ => None,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        // The id is baked into Context::ids (and every aria-* relationship that
        // points at it) at init; allowing it to change would silently break the
        // ARIA wiring. Every stateful component enforces this invariant.
        assert_eq!(
            old.id, new.id,
            "color_picker::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        // A controlled `open` flip drives the same Open/Close transition the
        // user would, so the lifecycle effects fire identically.
        if let (was, Some(now)) = (old.open, new.open)
            && was != Some(now)
        {
            events.push(if now { Event::Open } else { Event::Close });
        }

        if old.value != new.value {
            events.push(Event::SyncValue(new.value));
        }

        // A controlled `color_space` prop change is routed through the same
        // event a runtime switch uses, so it announces and remaps consistently
        // and `SetProps` never has to touch `color_space` (which would clobber a
        // runtime switch on any unrelated prop update).
        if old.color_space != new.color_space {
            events.push(Event::ChangeColorSpace(new.color_space));
        }

        if context_relevant_props_changed(old, new) {
            events.push(Event::SetProps);
        }

        events
    }

    fn initial_effects(
        state: &Self::State,
        _context: &Self::Context,
        _props: &Self::Props,
    ) -> Vec<PendingEffect<Self>> {
        // A `default_open`/controlled-open boot returns `State::Open` directly
        // from `init`, so the `Closed → Open` plan never runs. Mirror its
        // lifecycle effects here so adapters drive identical wiring on first
        // mount via `Service::take_initial_effects`.
        if matches!(state, State::Open) {
            open_lifecycle_effects().into_iter().collect()
        } else {
            Vec::new()
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

// ────────────────────────────────────────────────────────────────────
// Part
// ────────────────────────────────────────────────────────────────────

/// Structural parts exposed by the `ColorPicker` connect API.
#[derive(ComponentPart)]
#[scope = "color-picker"]
pub enum Part {
    /// Container element.
    Root,

    /// Text label for the picker (`for` the trigger).
    Label,

    /// Container for the trigger / preview.
    Control,

    /// Button that opens/closes the popover.
    Trigger,

    /// The popover panel (`role="dialog"`).
    Content,

    /// 2D saturation/lightness gradient (`role="group"`).
    Area,

    /// Draggable thumb inside the area (`role="application"`).
    AreaThumb,

    /// A channel slider container (`role="group"`), parameterized by channel.
    ChannelSlider {
        /// The channel this slider controls.
        channel: ColorChannel,
    },

    /// A channel slider thumb (`role="slider"`), parameterized by channel.
    ChannelSliderThumb {
        /// The channel this thumb controls.
        channel: ColorChannel,
    },

    /// The alpha channel slider container (`role="group"`).
    AlphaSlider,

    /// Container for the preset swatches (`role="group"`).
    SwatchGroup,

    /// A preset swatch button, parameterized by index into `Context::swatches`.
    Swatch {
        /// The index into `Context::swatches`.
        index: usize,
    },

    /// The format selector (`<select>`/`<button>`).
    FormatSelect,

    /// A channel text input, parameterized by channel and display index.
    ChannelInput {
        /// The channel this input edits.
        channel: ColorChannel,

        /// The display index (0-based).
        index: usize,
    },

    /// The hex color text input.
    HexInput,

    /// The browser eyedropper trigger button.
    EyeDropperTrigger,

    /// The `type="hidden"` input for form submission.
    HiddenInput,
}

/// The kebab-case `data-ars-channel` token for a channel.
const fn channel_token(channel: ColorChannel) -> &'static str {
    match channel {
        ColorChannel::Hue => "hue",
        ColorChannel::Saturation => "saturation",
        ColorChannel::Lightness => "lightness",
        ColorChannel::Brightness => "brightness",
        ColorChannel::Alpha => "alpha",
        ColorChannel::Red => "red",
        ColorChannel::Green => "green",
        ColorChannel::Blue => "blue",
    }
}

// ────────────────────────────────────────────────────────────────────
// Api
// ────────────────────────────────────────────────────────────────────

/// The connect API for the `ColorPicker` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("color_picker::Api")
            .field("state", &self.state)
            .field("ctx", &self.ctx)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    // --- Computed state ---

    /// Whether the popover is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        !matches!(self.state, State::Closed)
    }

    /// Whether a thumb is currently being dragged.
    #[must_use]
    pub const fn is_dragging(&self) -> bool {
        matches!(self.state, State::Dragging { .. })
    }

    /// The current color value (the pending value, so a controlled
    /// drag-in-flight is reflected).
    #[must_use]
    pub const fn value(&self) -> &ColorValue {
        self.ctx.value.pending()
    }

    /// The current color value formatted as a string in the active format.
    #[must_use]
    pub fn value_as_string(&self) -> String {
        let color = self.ctx.value.pending();

        match self.ctx.format {
            ColorFormat::Hex => color.to_hex(self.ctx.show_alpha),

            ColorFormat::Hsl => color.to_css_hsl(),

            ColorFormat::Rgb => {
                let (red, green, blue) = color.to_rgb();

                if self.ctx.show_alpha && color.alpha < 1.0 {
                    format!("rgba({red}, {green}, {blue}, {:.2})", color.alpha)
                } else {
                    format!("rgb({red}, {green}, {blue})")
                }
            }

            ColorFormat::Hsb => {
                let (hue, saturation, brightness) = color.to_hsb();

                format!(
                    "hsb({hue:.0}, {:.1}%, {:.1}%)",
                    saturation * 100.0,
                    brightness * 100.0
                )
            }
        }
    }

    /// The active text format.
    #[must_use]
    pub const fn format(&self) -> ColorFormat {
        self.ctx.format
    }

    /// The active color space.
    #[must_use]
    pub const fn color_space(&self) -> ColorSpace {
        self.ctx.color_space
    }

    /// A human-readable name for the current color (e.g. `"dark vibrant blue"`).
    #[must_use]
    pub fn color_name(&self) -> String {
        (self.ctx.messages.color_name)(self.ctx.value.pending(), &self.ctx.locale)
    }

    /// The debounced `aria-live` announcement text for the current color.
    #[must_use]
    pub fn color_announcement(&self) -> String {
        (self.ctx.messages.color_announcement)(
            self.ctx.value.pending(),
            self.ctx.format,
            &self.ctx.locale,
        )
    }

    /// The `aria-live` announcement text for the active color space, used by the
    /// [`Effect::AnnounceColorSpace`] adapter handler.
    #[must_use]
    pub fn color_space_announcement(&self) -> String {
        (self.ctx.messages.color_space_switched)(
            &format!("{:?}", self.ctx.color_space),
            &self.ctx.locale,
        )
    }

    // --- Imperative actions ---

    /// Open the popover.
    pub fn open(&self) {
        (self.send)(Event::Open);
    }

    /// Close the popover.
    pub fn close(&self) {
        (self.send)(Event::Close);
    }

    /// Set the color value directly.
    pub fn set_value(&self, color: ColorValue) {
        (self.send)(Event::SetColor(color));
    }

    /// Set the active text format.
    pub fn set_format(&self, format: ColorFormat) {
        (self.send)(Event::SetFormat(format));
    }

    // --- Part attrs ---

    /// Attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Data("ars-state"),
                if self.is_open() { "open" } else { "closed" },
            );

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        attrs
    }

    /// Attributes for the label element.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("label"))
            .set(HtmlAttr::For, self.ctx.ids.part("trigger"));

        attrs
    }

    /// Attributes for the control container.
    #[must_use]
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Attributes for the trigger button.
    #[must_use]
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("trigger"))
            // `type="button"` so a trigger rendered as a real `<button>` inside
            // a form toggles the popover instead of submitting the form (the
            // HTML default). Mirrors the sibling popover trigger.
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.trigger_label)(&self.ctx.locale),
            )
            .set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog")
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                if self.is_open() { "true" } else { "false" },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.ctx.ids.part("content"),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );

        if self.ctx.disabled {
            attrs
                .set_bool(HtmlAttr::Data("ars-disabled"), true)
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Attributes for the popover content panel.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("content"))
            .set(HtmlAttr::Role, "dialog")
            .set(
                HtmlAttr::Data("ars-state"),
                if self.is_open() { "open" } else { "closed" },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );

        attrs
    }

    /// Attributes for the 2D area container.
    #[must_use]
    pub fn area_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Area.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("area"))
            .set(HtmlAttr::Role, "group");

        let color = self.ctx.value.pending();

        attrs.set_style(
            CssProperty::Custom("ars-color-picker-area-bg"),
            format!("hsl({:.0}, 100%, 50%)", color.hue),
        );

        attrs
    }

    /// Attributes for the area thumb (the 2D saturation/lightness handle).
    ///
    /// Uses `role="application"` because `role="slider"` is one-dimensional;
    /// `aria-roledescription` and `aria-valuetext` give screen-reader users the
    /// 2D orientation, and `aria-keyshortcuts` documents the arrow controls.
    #[must_use]
    pub fn area_thumb_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::AreaThumb.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("area-thumb"))
            .set(HtmlAttr::Role, "application")
            .set(
                HtmlAttr::Aria(AriaAttr::RoleDescription),
                (self.ctx.messages.area_role_description)(&self.ctx.locale),
            )
            .set(
                HtmlAttr::TabIndex,
                if self.ctx.disabled { "-1" } else { "0" },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.area_label)(&self.ctx.locale),
            );

        let color = self.ctx.value.pending();

        let saturation_text = (self.ctx.messages.channel_value_text)(
            &(self.ctx.messages.saturation_label)(&self.ctx.locale),
            &format!("{:.0}%", (color.saturation * 100.0).round()),
            "",
            &self.ctx.locale,
        );

        let lightness_text = (self.ctx.messages.channel_value_text)(
            &(self.ctx.messages.lightness_label)(&self.ctx.locale),
            &format!("{:.0}%", (color.lightness * 100.0).round()),
            "",
            &self.ctx.locale,
        );

        attrs.set(
            HtmlAttr::Aria(AriaAttr::ValueText),
            format!("{saturation_text}, {lightness_text}"),
        );

        attrs
            .set_style(
                CssProperty::Custom("ars-color-picker-area-thumb-x"),
                format!("{:.1}%", color.saturation * 100.0),
            )
            .set_style(
                CssProperty::Custom("ars-color-picker-area-thumb-y"),
                format!("{:.1}%", (1.0 - color.lightness) * 100.0),
            )
            .set_style(CssProperty::BackgroundColor, color.to_css_hsl())
            .set(
                HtmlAttr::Aria(AriaAttr::KeyShortcuts),
                "ArrowUp ArrowDown ArrowLeft ArrowRight",
            );

        if self.is_dragging() {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }

        attrs
    }

    /// Attributes for a channel slider container.
    #[must_use]
    pub fn channel_slider_attrs(&self, channel: ColorChannel) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ChannelSlider { channel }.data_attrs();

        let slider_id = match channel {
            ColorChannel::Alpha => self.ctx.ids.part("alpha-slider"),
            _ => self.ctx.ids.part("hue-slider"),
        };

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, slider_id)
            .set(HtmlAttr::Role, "group")
            .set(HtmlAttr::Data("ars-channel"), channel_token(channel));

        attrs
    }

    /// Attributes for a channel slider thumb (the draggable handle).
    #[must_use]
    pub fn channel_slider_thumb_attrs(&self, channel: ColorChannel) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ChannelSliderThumb { channel }.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "slider")
            // A disabled control must stay out of the tab order, mirroring the
            // area thumb, so keyboard / AT users cannot focus a slider whose
            // events the machine drops.
            .set(
                HtmlAttr::TabIndex,
                if self.ctx.disabled { "-1" } else { "0" },
            )
            .set(HtmlAttr::Data("ars-channel"), channel_token(channel));

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        let color = self.ctx.value.pending();
        let value = channel_value(color, channel);
        let (min, max) = channel_range(channel);

        let label = match channel {
            ColorChannel::Alpha => (self.ctx.messages.alpha_label)(&self.ctx.locale),
            _ => (self.ctx.messages.hue_label)(&self.ctx.locale),
        };

        attrs
            .set(HtmlAttr::Aria(AriaAttr::Label), label)
            .set(HtmlAttr::Aria(AriaAttr::ValueNow), format!("{value:.0}"))
            .set(HtmlAttr::Aria(AriaAttr::ValueMin), format!("{min:.0}"))
            .set(HtmlAttr::Aria(AriaAttr::ValueMax), format!("{max:.0}"))
            .set(HtmlAttr::Aria(AriaAttr::Orientation), "horizontal");

        let pct = if (max - min).abs() > f64::EPSILON {
            (value - min) / (max - min) * 100.0
        } else {
            0.0
        };

        attrs.set_style(
            CssProperty::Custom("ars-color-picker-channel-thumb-position"),
            format!("{pct:.1}%"),
        );

        if matches!(self.state, State::Dragging { target: DragTarget::Channel(dragged) } if *dragged == channel)
        {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }

        attrs
    }

    /// Attributes for the alpha slider container.
    #[must_use]
    pub fn alpha_slider_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::AlphaSlider.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("alpha-slider"))
            .set(HtmlAttr::Role, "group")
            .set(HtmlAttr::Data("ars-channel"), "alpha");

        attrs
    }

    /// Attributes for the swatch group container.
    #[must_use]
    pub fn swatch_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::SwatchGroup.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "group");

        attrs
    }

    /// Attributes for a preset swatch button. The swatch color is resolved from
    /// [`Context::swatches`] at `index`; an out-of-range index yields the base
    /// attributes only (no color styling or selection).
    #[must_use]
    pub fn swatch_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Swatch { index }.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "button")
            // A disabled picker keeps its swatches out of the tab order too,
            // matching the area and channel thumbs.
            .set(
                HtmlAttr::TabIndex,
                if self.ctx.disabled { "-1" } else { "0" },
            )
            .set(HtmlAttr::Data("ars-index"), index.to_string());

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if let Some(color) = self.ctx.swatches.get(index) {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.swatch_label)(color, &self.ctx.locale),
            );

            if self.ctx.value.pending() == color {
                attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
            }

            attrs.set_style(CssProperty::Custom("ars-swatch-color"), color.to_css_hsl());
        }

        attrs
    }

    /// Attributes for the format selector.
    #[must_use]
    pub fn format_select_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::FormatSelect.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("format-select"))
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.format_toggle_label)(&self.ctx.locale),
            );

        attrs
    }

    /// Attributes for a channel text input.
    #[must_use]
    pub fn channel_input_attrs(&self, channel: ColorChannel, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ChannelInput { channel, index }.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.item("channel", &index))
            .set(HtmlAttr::Type, "text")
            .set(HtmlAttr::InputMode, "numeric")
            .set(HtmlAttr::Data("ars-channel"), channel_token(channel))
            .set(HtmlAttr::Data("ars-channel-index"), index.to_string());

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::ReadOnly, true);
        }

        attrs
    }

    /// Attributes for the hex text input.
    #[must_use]
    pub fn hex_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HexInput.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "text")
            .set(HtmlAttr::InputMode, "text");

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::ReadOnly, true);
        }

        attrs
    }

    /// Attributes for the eyedropper trigger button. Hidden from both trees when
    /// the browser `EyeDropper` API is unavailable.
    #[must_use]
    pub fn eye_dropper_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::EyeDropperTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.eyedropper_label)(&self.ctx.locale),
            );

        if !self.ctx.eyedropper_supported {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        if self.ctx.disabled || self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Attributes for the hidden form input. The submitted value is the canonical
    /// hex string (8-digit when `show_alpha` and the color is translucent).
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

        attrs.set(
            HtmlAttr::Value,
            self.ctx.value.pending().to_hex(self.ctx.show_alpha),
        );

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    // --- Typed event dispatch helpers ---

    /// Toggle the popover (trigger click / Enter / Space).
    pub fn on_trigger_click(&self) {
        (self.send)(Event::Toggle);
    }

    /// Handle a keydown on the trigger: Enter/Space toggles the popover.
    pub fn on_trigger_keydown(&self, data: &KeyboardEventData) {
        if matches!(data.key, KeyboardKey::Enter | KeyboardKey::Space) {
            (self.send)(Event::Toggle);
        }
    }

    /// Handle a keydown on the content panel: Escape requests a close.
    pub fn on_content_keydown(&self, data: &KeyboardEventData) {
        if matches!(data.key, KeyboardKey::Escape) {
            (self.send)(Event::CloseOnEscape);
        }
    }

    /// Dispatch an area drag-start from adapter-resolved normalized `(x, y)`.
    pub fn on_area_pointer_down(&self, x: f64, y: f64) {
        (self.send)(Event::DragStart {
            target: DragTarget::Area,
            x,
            y,
        });
    }

    /// Dispatch a channel-slider drag-start from adapter-resolved normalized `x`.
    pub fn on_channel_slider_pointer_down(&self, channel: ColorChannel, x: f64) {
        (self.send)(Event::DragStart {
            target: DragTarget::Channel(channel),
            x,
            y: 0.0,
        });
    }

    /// Handle arrow/Home/End keydown on the area thumb. Left/Right adjust
    /// saturation, Up/Down adjust lightness; `shift` selects the large step. In
    /// RTL the saturation (x-axis) arrows are mirrored.
    pub fn on_area_thumb_keydown(&self, data: &KeyboardEventData, shift: bool) {
        let saturation_step = self.keyboard_step(ColorChannel::Saturation, shift);
        let lightness_step = self.keyboard_step(ColorChannel::Lightness, shift);

        let rtl = self.ctx.dir == Direction::Rtl;

        match data.key {
            KeyboardKey::ArrowRight => (self.send)(if rtl {
                Event::ChannelDecrement {
                    channel: ColorChannel::Saturation,
                    step: saturation_step,
                }
            } else {
                Event::ChannelIncrement {
                    channel: ColorChannel::Saturation,
                    step: saturation_step,
                }
            }),

            KeyboardKey::ArrowLeft => (self.send)(if rtl {
                Event::ChannelIncrement {
                    channel: ColorChannel::Saturation,
                    step: saturation_step,
                }
            } else {
                Event::ChannelDecrement {
                    channel: ColorChannel::Saturation,
                    step: saturation_step,
                }
            }),

            KeyboardKey::ArrowUp => (self.send)(Event::ChannelIncrement {
                channel: ColorChannel::Lightness,
                step: lightness_step,
            }),

            KeyboardKey::ArrowDown => (self.send)(Event::ChannelDecrement {
                channel: ColorChannel::Lightness,
                step: lightness_step,
            }),

            _ => {}
        }
    }

    /// Handle arrow/Home/End keydown on a channel slider thumb. Arrows adjust the
    /// channel value (`shift` = large step); Home/End jump to min/max.
    pub fn on_channel_slider_keydown(
        &self,
        channel: ColorChannel,
        data: &KeyboardEventData,
        shift: bool,
    ) {
        let step = self.keyboard_step(channel, shift);
        let (min, max) = channel_range(channel);

        let rtl = self.ctx.dir == Direction::Rtl;

        match data.key {
            KeyboardKey::ArrowRight | KeyboardKey::ArrowUp => {
                let event = if rtl && matches!(data.key, KeyboardKey::ArrowRight) {
                    Event::ChannelDecrement { channel, step }
                } else {
                    Event::ChannelIncrement { channel, step }
                };

                (self.send)(event);
            }

            KeyboardKey::ArrowLeft | KeyboardKey::ArrowDown => {
                let event = if rtl && matches!(data.key, KeyboardKey::ArrowLeft) {
                    Event::ChannelIncrement { channel, step }
                } else {
                    Event::ChannelDecrement { channel, step }
                };

                (self.send)(event);
            }

            KeyboardKey::Home => (self.send)(Event::SetChannel {
                channel,
                value: min,
            }),

            KeyboardKey::End => (self.send)(Event::SetChannel {
                channel,
                value: max,
            }),

            _ => {}
        }
    }

    /// Select the preset swatch at `index` (click / Enter / Space).
    pub fn on_swatch_click(&self, index: usize) {
        if let Some(color) = self.ctx.swatches.get(index) {
            (self.send)(Event::SetColor(*color));
        }
    }

    /// Request an eyedropper sample.
    pub fn on_eyedropper_click(&self) {
        (self.send)(Event::EyedropperRequest);
    }

    /// Compute the per-channel keyboard step. Fractional channels (saturation,
    /// lightness, brightness, alpha) step by `channel_step_default` so a single
    /// arrow press is a perceptible 1%/10% nudge rather than snapping the whole
    /// `0..1` range; the configured `channel_step` / `channel_large_step` apply
    /// to the wider hue and RGB ranges.
    fn keyboard_step(&self, channel: ColorChannel, shift: bool) -> f64 {
        match channel {
            ColorChannel::Hue | ColorChannel::Red | ColorChannel::Green | ColorChannel::Blue => {
                if shift {
                    self.ctx.channel_large_step
                } else {
                    self.ctx.channel_step
                }
            }

            _ => {
                let base = channel_step_default(channel);

                if shift { base * 10.0 } else { base }
            }
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Control => self.control_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Content => self.content_attrs(),
            Part::Area => self.area_attrs(),
            Part::AreaThumb => self.area_thumb_attrs(),
            Part::ChannelSlider { channel } => self.channel_slider_attrs(channel),
            Part::ChannelSliderThumb { channel } => self.channel_slider_thumb_attrs(channel),
            Part::AlphaSlider => self.alpha_slider_attrs(),
            Part::SwatchGroup => self.swatch_group_attrs(),
            Part::Swatch { index } => self.swatch_attrs(index),
            Part::FormatSelect => self.format_select_attrs(),
            Part::ChannelInput { channel, index } => self.channel_input_attrs(channel, index),
            Part::HexInput => self.hex_input_attrs(),
            Part::EyeDropperTrigger => self.eye_dropper_trigger_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;
    use core::{
        cell::RefCell,
        sync::atomic::{AtomicU64, Ordering},
    };

    use ars_core::{Service, StrongSend, callback};
    use insta::assert_snapshot;

    use super::*;

    fn service(mut props: Props) -> Service<Machine> {
        if props.id.is_empty() {
            props.id = "color-picker".to_string();
        }

        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    /// A service that boots open, draining the initial effects so subsequent
    /// `send` calls observe only their own effects.
    fn open_service(mut props: Props) -> Service<Machine> {
        props.default_open = true;

        let mut svc = service(props);

        drop(svc.take_initial_effects());

        svc
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn key(key: KeyboardKey) -> KeyboardEventData {
        KeyboardEventData {
            key,
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

    /// Run every pending effect of a `SendResult`, returning the effect names.
    fn run_effects(
        svc: &Service<Machine>,
        result: &mut ars_core::SendResult<Machine>,
    ) -> Vec<Effect> {
        let send: StrongSend<Event> = Arc::new(|_| {});

        let mut names = Vec::new();

        for effect in result.pending_effects.drain(..) {
            names.push(effect.name);

            drop(effect.run(svc.context(), svc.props(), Arc::clone(&send)));
        }

        names
    }

    // ── Lifecycle ──────────────────────────────────────────────────

    #[test]
    fn open_close_toggle_drive_state_and_open_flag() {
        let mut svc = service(Props::default());

        assert_eq!(svc.state(), &State::Closed);

        drop(svc.send(Event::Open));

        assert_eq!(svc.state(), &State::Open);
        assert!(svc.connect(&|_| {}).is_open());

        drop(svc.send(Event::Close));

        assert_eq!(svc.state(), &State::Closed);

        drop(svc.send(Event::Toggle));

        assert_eq!(svc.state(), &State::Open);

        drop(svc.send(Event::Toggle));

        assert_eq!(svc.state(), &State::Closed);
    }

    #[test]
    fn open_emits_click_outside_and_eyedropper_detection_effects() {
        let mut svc = service(Props::default());

        let mut result = svc.send(Event::Open);

        let names = run_effects(&svc, &mut result);

        assert!(names.contains(&Effect::AttachClickOutside));
        assert!(names.contains(&Effect::DetectEyedropper));

        let mut close = svc.send(Event::Close);

        let close_names = run_effects(&svc, &mut close);

        assert!(close_names.contains(&Effect::DetachClickOutside));
    }

    #[test]
    fn initial_effects_mirror_open_lifecycle_when_default_open() {
        let mut svc = service(Props {
            default_open: true,
            ..Props::default()
        });

        let names: Vec<Effect> = svc
            .take_initial_effects()
            .into_iter()
            .map(|effect| effect.name)
            .collect();

        assert!(names.contains(&Effect::AttachClickOutside));
        assert!(names.contains(&Effect::DetectEyedropper));
        // Fires exactly once.
        assert!(svc.take_initial_effects().is_empty());
    }

    #[test]
    fn closed_picker_has_no_initial_effects() {
        let mut svc = service(Props::default());

        assert!(svc.take_initial_effects().is_empty());
    }

    #[test]
    fn escape_and_interact_outside_close_when_enabled() {
        let mut svc = open_service(Props::default());

        drop(svc.send(Event::CloseOnEscape));

        assert_eq!(svc.state(), &State::Closed);

        let mut svc = open_service(Props::default());

        drop(svc.send(Event::CloseOnInteractOutside));

        assert_eq!(svc.state(), &State::Closed);
    }

    #[test]
    fn escape_and_interact_outside_respect_disabled_policies() {
        let mut svc = open_service(Props {
            close_on_escape: false,
            close_on_interact_outside: false,
            ..Props::default()
        });

        drop(svc.send(Event::CloseOnEscape));
        drop(svc.send(Event::CloseOnInteractOutside));

        assert_eq!(svc.state(), &State::Open);
    }

    #[test]
    fn dragging_suppresses_interact_outside() {
        let mut svc = open_service(Props::default());

        drop(svc.send(Event::DragStart {
            target: DragTarget::Area,
            x: 0.5,
            y: 0.5,
        }));

        assert!(matches!(svc.state(), State::Dragging { .. }));

        drop(svc.send(Event::CloseOnInteractOutside));

        assert!(
            matches!(svc.state(), State::Dragging { .. }),
            "interact-outside must be suppressed during a drag"
        );
    }

    // ── Area + channel dragging ────────────────────────────────────

    #[test]
    fn area_drag_updates_saturation_and_lightness() {
        let mut svc = open_service(Props {
            default_value: ColorValue::from_hsl(200.0, 0.2, 0.2),
            ..Props::default()
        });

        // Top-right corner: x = 1 (saturation max), y = 0 (lightness max).
        drop(svc.send(Event::DragStart {
            target: DragTarget::Area,
            x: 1.0,
            y: 0.0,
        }));

        assert_eq!(
            svc.state(),
            &State::Dragging {
                target: DragTarget::Area
            }
        );

        let value = *svc.connect(&|_| {}).value();

        assert!((value.saturation - 1.0).abs() < 1e-9);
        assert!((value.lightness - 1.0).abs() < 1e-9);
        // Hue is preserved by an area drag.
        assert!((value.hue - 200.0).abs() < 1e-9);

        drop(svc.send(Event::DragMove { x: 0.25, y: 0.75 }));

        let value = *svc.connect(&|_| {}).value();

        assert!((value.saturation - 0.25).abs() < 1e-9);
        assert!((value.lightness - 0.25).abs() < 1e-9);

        drop(svc.send(Event::DragEnd));

        assert_eq!(svc.state(), &State::Open);
    }

    #[test]
    fn channel_drag_updates_single_channel() {
        let mut svc = open_service(Props::default());

        drop(svc.send(Event::DragStart {
            target: DragTarget::Channel(ColorChannel::Hue),
            x: 0.5,
            y: 0.0,
        }));

        let value = *svc.connect(&|_| {}).value();

        // Hue range is 0..360, so x = 0.5 -> 180°.
        assert!((value.hue - 180.0).abs() < 1e-9);
    }

    #[test]
    fn change_end_callback_fires_with_pending_value_on_drag_end() {
        let reported = Arc::new(AtomicU64::new(u64::MAX));

        let sink = Arc::clone(&reported);

        let mut svc = open_service(Props {
            value: Some(ColorValue::from_hsl(0.0, 0.0, 0.5)),
            on_change_end: Some(callback(move |color: ColorValue| {
                sink.store(color.saturation.to_bits(), Ordering::SeqCst);
            })),
            ..Props::default()
        });

        drop(svc.send(Event::DragStart {
            target: DragTarget::Area,
            x: 1.0,
            y: 0.5,
        }));

        let mut end = svc.send(Event::DragEnd);

        let names = run_effects(&svc, &mut end);

        assert!(names.contains(&Effect::ChangeEnd));

        let reported_saturation = f64::from_bits(reported.load(Ordering::SeqCst));

        assert!(
            (reported_saturation - 1.0).abs() < 1e-9,
            "on_change_end must report the pending saturation, got {reported_saturation}"
        );
    }

    // ── Channels / color space ─────────────────────────────────────

    #[test]
    fn channels_track_color_space() {
        let hsl = service(Props::default());

        assert_eq!(
            hsl.context().channels(),
            &[
                ColorChannel::Hue,
                ColorChannel::Saturation,
                ColorChannel::Lightness
            ]
        );

        let rgb = service(Props {
            color_space: ColorSpace::Rgb,
            ..Props::default()
        });

        assert_eq!(
            rgb.context().channels(),
            &[ColorChannel::Red, ColorChannel::Green, ColorChannel::Blue]
        );

        let hsb = service(Props {
            color_space: ColorSpace::Hsb,
            ..Props::default()
        });

        assert_eq!(
            hsb.context().channels(),
            &[
                ColorChannel::Hue,
                ColorChannel::Saturation,
                ColorChannel::Brightness
            ]
        );
    }

    #[test]
    fn change_color_space_updates_context_and_announces() {
        let mut svc = service(Props::default());

        let mut result = svc.send(Event::ChangeColorSpace(ColorSpace::Rgb));

        let names = run_effects(&svc, &mut result);

        assert!(names.contains(&Effect::AnnounceColorSpace));
        assert_eq!(svc.connect(&|_| {}).color_space(), ColorSpace::Rgb);
        assert!(
            svc.connect(&|_| {})
                .color_space_announcement()
                .contains("Rgb")
        );
    }

    // ── Format switching ───────────────────────────────────────────

    #[test]
    fn set_format_changes_value_as_string() {
        let mut svc = service(Props {
            // Opaque pure red.
            default_value: ColorValue::from_hsl(0.0, 1.0, 0.5),
            ..Props::default()
        });

        assert_eq!(svc.connect(&|_| {}).value_as_string(), "#ff0000");

        drop(svc.send(Event::SetFormat(ColorFormat::Rgb)));

        assert_eq!(svc.connect(&|_| {}).value_as_string(), "rgb(255, 0, 0)");

        drop(svc.send(Event::SetFormat(ColorFormat::Hsl)));

        assert_eq!(
            svc.connect(&|_| {}).value_as_string(),
            "hsl(0, 100.0%, 50.0%)"
        );

        drop(svc.send(Event::SetFormat(ColorFormat::Hsb)));

        assert!(svc.connect(&|_| {}).value_as_string().starts_with("hsb("));
        assert_eq!(svc.connect(&|_| {}).format(), ColorFormat::Hsb);
    }

    // ── Channel inputs / keyboard ──────────────────────────────────

    #[test]
    fn set_channel_updates_value_when_open() {
        let mut svc = open_service(Props::default());

        drop(svc.send(Event::SetChannel {
            channel: ColorChannel::Hue,
            value: 120.0,
        }));

        assert!((svc.connect(&|_| {}).value().hue - 120.0).abs() < 1e-9);
    }

    #[test]
    fn channel_increment_and_decrement_clamp_to_range() {
        // A linear channel (Red, 0..255) shows the clamp cleanly. Hue is
        // intentionally excluded: `ColorValue` wraps hue into `[0, 360)`, so an
        // increment past the top circles back to 0 rather than pinning at 360.
        let mut svc = open_service(Props {
            default_value: ColorValue::from_rgb(250, 0, 0),
            ..Props::default()
        });

        drop(svc.send(Event::ChannelIncrement {
            channel: ColorChannel::Red,
            step: 10.0,
        }));

        assert_eq!(svc.connect(&|_| {}).value().to_rgb().0, 255);

        drop(svc.send(Event::ChannelDecrement {
            channel: ColorChannel::Red,
            step: 1000.0,
        }));

        assert_eq!(svc.connect(&|_| {}).value().to_rgb().0, 0);
    }

    #[test]
    fn area_thumb_keyboard_adjusts_saturation_and_lightness() {
        let svc = open_service(Props::default());

        let captured = RefCell::new(Vec::new());

        let send = |event: Event| captured.borrow_mut().push(event);

        let api = svc.connect(&send);

        api.on_area_thumb_keydown(&key(KeyboardKey::ArrowRight), false);
        api.on_area_thumb_keydown(&key(KeyboardKey::ArrowUp), true);

        let events = captured.borrow();

        assert!(matches!(
            events[0],
            Event::ChannelIncrement {
                channel: ColorChannel::Saturation,
                ..
            }
        ));
        assert!(matches!(
            events[1],
            Event::ChannelIncrement {
                channel: ColorChannel::Lightness,
                ..
            }
        ));
    }

    #[test]
    fn area_thumb_keyboard_mirrors_saturation_in_rtl() {
        let svc = open_service(Props {
            dir: Direction::Rtl,
            ..Props::default()
        });

        let captured = RefCell::new(Vec::new());

        let send = |event: Event| captured.borrow_mut().push(event);

        let api = svc.connect(&send);

        api.on_area_thumb_keydown(&key(KeyboardKey::ArrowRight), false);
        api.on_area_thumb_keydown(&key(KeyboardKey::ArrowLeft), false);

        let events = captured.borrow();

        // RTL: ArrowRight decrements saturation, ArrowLeft increments it.
        assert!(matches!(
            events[0],
            Event::ChannelDecrement {
                channel: ColorChannel::Saturation,
                ..
            }
        ));
        assert!(matches!(
            events[1],
            Event::ChannelIncrement {
                channel: ColorChannel::Saturation,
                ..
            }
        ));
    }

    #[test]
    fn channel_slider_keyboard_home_end_jump_to_bounds() {
        let svc = open_service(Props::default());

        let captured = RefCell::new(Vec::new());

        let send = |event: Event| captured.borrow_mut().push(event);

        let api = svc.connect(&send);

        api.on_channel_slider_keydown(ColorChannel::Hue, &key(KeyboardKey::Home), false);
        api.on_channel_slider_keydown(ColorChannel::Hue, &key(KeyboardKey::End), false);

        let events = captured.borrow();

        assert_eq!(
            events[0],
            Event::SetChannel {
                channel: ColorChannel::Hue,
                value: 0.0
            }
        );
        assert_eq!(
            events[1],
            Event::SetChannel {
                channel: ColorChannel::Hue,
                value: 360.0
            }
        );
    }

    // ── Eyedropper ─────────────────────────────────────────────────

    #[test]
    fn eyedropper_request_gated_by_support_and_emits_effect() {
        let mut svc = open_service(Props::default());

        // Unsupported by default: the request is a no-op.
        let mut blocked = svc.send(Event::EyedropperRequest);

        assert!(run_effects(&svc, &mut blocked).is_empty());

        drop(svc.send(Event::SetEyedropperSupported(true)));

        let mut allowed = svc.send(Event::EyedropperRequest);

        assert!(run_effects(&svc, &mut allowed).contains(&Effect::InvokeEyedropper));
    }

    #[test]
    fn eyedropper_result_sets_value() {
        let mut svc = open_service(Props::default());

        let sampled = ColorValue::from_rgb(0x33, 0x66, 0xff);

        drop(svc.send(Event::EyedropperResult(Some(sampled))));

        assert_eq!(*svc.connect(&|_| {}).value(), sampled);

        // Cancellation is a no-op.
        drop(svc.send(Event::EyedropperResult(None)));

        assert_eq!(*svc.connect(&|_| {}).value(), sampled);
    }

    // ── Swatches ───────────────────────────────────────────────────

    #[test]
    fn swatch_click_sets_color_and_marks_selection() {
        let red = ColorValue::from_hsl(0.0, 1.0, 0.5);
        let blue = ColorValue::from_hsl(240.0, 1.0, 0.5);

        let mut svc = open_service(Props {
            default_value: red,
            swatches: alloc::vec![red, blue],
            ..Props::default()
        });

        // Swatch 0 (red) starts selected.
        assert!(
            svc.connect(&|_| {})
                .swatch_attrs(0)
                .contains(&HtmlAttr::Data("ars-selected"))
        );
        assert!(
            !svc.connect(&|_| {})
                .swatch_attrs(1)
                .contains(&HtmlAttr::Data("ars-selected"))
        );

        let captured = RefCell::new(Vec::new());

        let send = |event: Event| captured.borrow_mut().push(event);

        svc.connect(&send).on_swatch_click(1);

        assert_eq!(captured.borrow()[0], Event::SetColor(blue));

        drop(svc.send(Event::SetColor(blue)));

        assert!(
            svc.connect(&|_| {})
                .swatch_attrs(1)
                .contains(&HtmlAttr::Data("ars-selected"))
        );
    }

    #[test]
    fn swatch_attrs_out_of_range_is_safe() {
        let svc = service(Props::default());

        let attrs = svc.connect(&|_| {}).swatch_attrs(99);

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-index")), Some("99"));
        assert!(!attrs.contains(&HtmlAttr::Data("ars-selected")));
    }

    // ── Trigger / content ARIA ─────────────────────────────────────

    #[test]
    fn trigger_exposes_dialog_popup_relationship() {
        let svc = service(Props::default());

        let trigger = svc.connect(&|_| {}).trigger_attrs();

        assert_eq!(
            trigger.get(&HtmlAttr::Aria(AriaAttr::HasPopup)),
            Some("dialog")
        );
        assert_eq!(
            trigger.get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("false")
        );
        assert_eq!(
            trigger.get(&HtmlAttr::Aria(AriaAttr::Controls)),
            Some("color-picker-content")
        );
        assert_eq!(
            trigger.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("color-picker-label")
        );

        let open = open_service(Props::default());

        assert_eq!(
            open.connect(&|_| {})
                .trigger_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("true")
        );
    }

    #[test]
    fn content_is_a_labelled_dialog() {
        let svc = open_service(Props::default());

        let content = svc.connect(&|_| {}).content_attrs();

        assert_eq!(content.get(&HtmlAttr::Role), Some("dialog"));
        assert_eq!(
            content.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("color-picker-label")
        );
        assert_eq!(content.get(&HtmlAttr::Data("ars-state")), Some("open"));
    }

    #[test]
    fn root_and_content_reflect_state_data_attr() {
        let closed = service(Props::default());

        assert_eq!(
            closed
                .connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Data("ars-state")),
            Some("closed")
        );

        let open = open_service(Props::default());

        assert_eq!(
            open.connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Data("ars-state")),
            Some("open")
        );
    }

    // ── Alpha gating ───────────────────────────────────────────────

    #[test]
    fn hidden_input_value_honors_show_alpha() {
        let translucent = ColorValue::new(0.0, 1.0, 0.5, 0.5);

        let with_alpha = service(Props {
            name: Some("color".to_string()),
            default_value: translucent,
            show_alpha: true,
            ..Props::default()
        });

        let attrs = with_alpha.connect(&|_| {}).hidden_input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Name), Some("color"));
        assert_eq!(attrs.get(&HtmlAttr::Value).map(str::len), Some(9)); // #rrggbbaa

        let without_alpha = service(Props {
            default_value: translucent,
            show_alpha: false,
            ..Props::default()
        });

        assert_eq!(
            without_alpha
                .connect(&|_| {})
                .hidden_input_attrs()
                .get(&HtmlAttr::Value)
                .map(str::len),
            Some(7) // #rrggbb
        );
    }

    // ── Disabled / readonly ────────────────────────────────────────

    #[test]
    fn disabled_picker_ignores_user_interaction_but_tracks_focus() {
        let mut svc = service(Props {
            disabled: true,
            ..Props::default()
        });

        // User-initiated toggle is ignored while disabled.
        drop(svc.send(Event::Toggle));
        assert_eq!(
            svc.state(),
            &State::Closed,
            "disabled picker ignores a user toggle"
        );

        drop(svc.send(Event::Focus { part: "trigger" }));
        assert_eq!(svc.context().focused_part, Some("trigger"));

        let trigger = svc.connect(&|_| {}).trigger_attrs();
        assert_eq!(
            trigger.get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true")
        );
    }

    #[test]
    fn disabled_picker_still_honors_controlled_open_sync() {
        // A controlled, disabled picker must reflect parent-driven open changes
        // (which arrive as `Open`/`Close`) even though user interaction is
        // blocked — otherwise it would be stuck until re-enabled.
        let mut svc = service(Props {
            disabled: true,
            open: Some(true),
            ..Props::default()
        });
        assert_eq!(svc.state(), &State::Open);

        drop(svc.set_props(Props {
            id: "color-picker".to_string(),
            disabled: true,
            open: Some(false),
            ..Props::default()
        }));
        assert_eq!(svc.state(), &State::Closed);

        drop(svc.set_props(Props {
            id: "color-picker".to_string(),
            disabled: true,
            open: Some(true),
            ..Props::default()
        }));
        assert_eq!(svc.state(), &State::Open);
    }

    #[test]
    fn readonly_blocks_value_edits() {
        let mut svc = open_service(Props {
            readonly: true,
            default_value: ColorValue::from_hsl(10.0, 0.5, 0.5),
            ..Props::default()
        });

        let before = *svc.connect(&|_| {}).value();

        drop(svc.send(Event::DragStart {
            target: DragTarget::Area,
            x: 1.0,
            y: 1.0,
        }));
        drop(svc.send(Event::SetChannel {
            channel: ColorChannel::Hue,
            value: 200.0,
        }));
        drop(svc.send(Event::SetColor(ColorValue::default())));

        assert_eq!(*svc.connect(&|_| {}).value(), before);
        assert_eq!(svc.state(), &State::Open);
    }

    // ── Controlled syncs ───────────────────────────────────────────

    #[test]
    fn controlled_open_prop_drives_transition() {
        let mut svc = service(Props {
            open: Some(false),
            ..Props::default()
        });

        assert_eq!(svc.state(), &State::Closed);

        drop(svc.set_props(Props {
            id: "color-picker".to_string(),
            open: Some(true),
            ..Props::default()
        }));

        assert_eq!(svc.state(), &State::Open);
    }

    #[test]
    fn controlled_value_prop_syncs() {
        let mut svc = service(Props {
            value: Some(ColorValue::from_hsl(0.0, 0.2, 0.2)),
            ..Props::default()
        });

        drop(svc.set_props(Props {
            id: "color-picker".to_string(),
            value: Some(ColorValue::from_hsl(120.0, 0.9, 0.8)),
            ..Props::default()
        }));

        assert!((svc.connect(&|_| {}).value().hue - 120.0).abs() < 1e-9);
    }

    #[test]
    fn set_props_syncs_context_flags() {
        let mut svc = service(Props::default());

        drop(svc.set_props(Props {
            id: "color-picker".to_string(),
            disabled: true,
            color_space: ColorSpace::Rgb,
            show_alpha: false,
            ..Props::default()
        }));

        let ctx = svc.context();

        assert!(ctx.disabled);
        assert_eq!(ctx.color_space, ColorSpace::Rgb);
        assert!(!ctx.show_alpha);
    }

    #[test]
    fn focus_blur_tracks_focused_part() {
        let mut svc = open_service(Props::default());

        drop(svc.send(Event::Focus { part: "area-thumb" }));

        assert_eq!(svc.context().focused_part, Some("area-thumb"));

        drop(svc.send(Event::Blur { part: "area-thumb" }));

        assert_eq!(svc.context().focused_part, None);
    }

    // ── Imperative + dispatch helpers (coverage) ───────────────────

    #[test]
    fn imperative_and_dispatch_helpers_emit_expected_events() {
        let svc = open_service(Props {
            swatches: alloc::vec![ColorValue::default()],
            ..Props::default()
        });

        let captured = RefCell::new(Vec::new());

        let send = |event: Event| captured.borrow_mut().push(event);

        let api = svc.connect(&send);

        api.open();
        api.close();
        api.set_value(ColorValue::default());
        api.set_format(ColorFormat::Rgb);
        api.on_trigger_click();
        api.on_trigger_keydown(&key(KeyboardKey::Enter));
        api.on_content_keydown(&key(KeyboardKey::Escape));
        api.on_area_pointer_down(0.3, 0.4);
        api.on_channel_slider_pointer_down(ColorChannel::Hue, 0.7);
        api.on_eyedropper_click();

        let events = captured.borrow();

        assert!(matches!(events[0], Event::Open));
        assert!(matches!(events[1], Event::Close));
        assert!(matches!(events[2], Event::SetColor(_)));
        assert!(matches!(events[3], Event::SetFormat(ColorFormat::Rgb)));
        assert!(matches!(events[4], Event::Toggle));
        assert!(matches!(events[5], Event::Toggle));
        assert!(matches!(events[6], Event::CloseOnEscape));
        assert!(matches!(
            events[7],
            Event::DragStart {
                target: DragTarget::Area,
                ..
            }
        ));
        assert!(matches!(
            events[8],
            Event::DragStart {
                target: DragTarget::Channel(ColorChannel::Hue),
                ..
            }
        ));
        assert!(matches!(events[9], Event::EyedropperRequest));
    }

    #[test]
    fn exhaustive_events_and_parts_walk() {
        let mut svc = service(Props {
            id: "cp".into(),
            value: Some(ColorValue::from_hsl(10.0, 0.5, 0.5)),
            show_alpha: true,
            swatches: alloc::vec![ColorValue::default()],
            dir: Direction::Rtl,
            ..Props::default()
        });

        for event in [
            Event::SetEyedropperSupported(true),
            Event::Focus { part: "trigger" },
            Event::Open,
            Event::DragStart {
                target: DragTarget::Channel(ColorChannel::Alpha),
                x: 0.4,
                y: 0.0,
            },
            Event::DragMove { x: 0.6, y: 0.0 },
            Event::DragEnd,
            Event::SetChannel {
                channel: ColorChannel::Saturation,
                value: 0.7,
            },
            Event::ChannelIncrement {
                channel: ColorChannel::Lightness,
                step: 0.05,
            },
            Event::ChannelDecrement {
                channel: ColorChannel::Lightness,
                step: 0.05,
            },
            Event::SetFormat(ColorFormat::Hsb),
            Event::ChangeColorSpace(ColorSpace::Hsb),
            Event::EyedropperRequest,
            Event::EyedropperResult(Some(ColorValue::default())),
            Event::Blur { part: "trigger" },
            Event::Close,
        ] {
            let mut result = svc.send(event);

            drop(run_effects(&svc, &mut result));
        }

        let api = svc.connect(&|_| {});

        for part in [
            Part::Root,
            Part::Label,
            Part::Control,
            Part::Trigger,
            Part::Content,
            Part::Area,
            Part::AreaThumb,
            Part::ChannelSlider {
                channel: ColorChannel::Hue,
            },
            Part::ChannelSliderThumb {
                channel: ColorChannel::Hue,
            },
            Part::AlphaSlider,
            Part::SwatchGroup,
            Part::Swatch { index: 0 },
            Part::FormatSelect,
            Part::ChannelInput {
                channel: ColorChannel::Red,
                index: 0,
            },
            Part::HexInput,
            Part::EyeDropperTrigger,
            Part::HiddenInput,
        ] {
            let _attrs = api.part_attrs(part);
        }

        let _name = api.color_name();
        let _announcement = api.color_announcement();

        let _dbg = format!("{api:?}");

        // Disabled walk: user interaction blocked, focus tracked, controlled
        // open/value syncs pass through.
        let mut disabled = service(Props {
            id: "cp".into(),
            disabled: true,
            ..Props::default()
        });

        drop(disabled.send(Event::Toggle)); // user toggle ignored
        assert_eq!(disabled.state(), &State::Closed);
        drop(disabled.send(Event::Focus { part: "trigger" }));
        drop(disabled.send(Event::Blur { part: "trigger" }));
        drop(disabled.send(Event::SyncValue(Some(ColorValue::default()))));

        // Controlled open sync is honored even while disabled.
        drop(disabled.send(Event::Open));
        assert_eq!(disabled.state(), &State::Open);
        drop(disabled.send(Event::Close));
        assert_eq!(disabled.state(), &State::Closed);
    }

    // ── Coverage: remaining attr/keyboard/guard branches ──────────

    #[test]
    fn value_as_string_emits_rgba_for_translucent_rgb() {
        let mut svc = service(Props {
            default_value: ColorValue::new(0.0, 1.0, 0.5, 0.5),
            show_alpha: true,
            ..Props::default()
        });

        drop(svc.send(Event::SetFormat(ColorFormat::Rgb)));

        let string = svc.connect(&|_| {}).value_as_string();

        assert!(string.starts_with("rgba("), "got {string}");
    }

    #[test]
    fn channel_token_covers_every_channel() {
        let svc = open_service(Props::default());

        let api = svc.connect(&|_| {});

        for (channel, token) in [
            (ColorChannel::Hue, "hue"),
            (ColorChannel::Saturation, "saturation"),
            (ColorChannel::Lightness, "lightness"),
            (ColorChannel::Brightness, "brightness"),
            (ColorChannel::Alpha, "alpha"),
            (ColorChannel::Red, "red"),
            (ColorChannel::Green, "green"),
            (ColorChannel::Blue, "blue"),
        ] {
            let attrs = api.channel_input_attrs(channel, 0);

            assert_eq!(attrs.get(&HtmlAttr::Data("ars-channel")), Some(token));
        }
    }

    #[test]
    fn channel_slider_attrs_use_channel_specific_id() {
        let svc = open_service(Props {
            id: "cp".into(),
            ..Props::default()
        });

        let api = svc.connect(&|_| {});

        assert_eq!(
            api.channel_slider_attrs(ColorChannel::Hue)
                .get(&HtmlAttr::Id),
            Some("cp-hue-slider")
        );
        assert_eq!(
            api.channel_slider_attrs(ColorChannel::Alpha)
                .get(&HtmlAttr::Id),
            Some("cp-alpha-slider")
        );
    }

    #[test]
    fn eyedropper_request_blocked_when_readonly_even_if_supported() {
        let mut svc = open_service(Props {
            readonly: true,
            ..Props::default()
        });

        drop(svc.send(Event::SetEyedropperSupported(true)));

        let mut result = svc.send(Event::EyedropperRequest);

        assert!(run_effects(&svc, &mut result).is_empty());
    }

    #[test]
    fn sync_value_none_clears_controlled_binding_without_panic() {
        // value Some -> None emits `SyncValue(None)`, exercising the `None` arm.
        let mut svc = service(Props {
            value: Some(ColorValue::from_hsl(0.0, 0.5, 0.5)),
            ..Props::default()
        });

        drop(svc.set_props(Props {
            id: "color-picker".to_string(),
            value: None,
            ..Props::default()
        }));

        // The last controlled value is retained as the uncontrolled baseline.
        assert!((svc.connect(&|_| {}).value().saturation - 0.5).abs() < 1e-9);
    }

    #[test]
    fn controlled_open_true_to_false_closes() {
        let mut svc = service(Props {
            open: Some(true),
            ..Props::default()
        });

        assert_eq!(svc.state(), &State::Open);

        drop(svc.set_props(Props {
            id: "color-picker".to_string(),
            open: Some(false),
            ..Props::default()
        }));

        assert_eq!(svc.state(), &State::Closed);
    }

    #[test]
    fn value_as_string_rgb_drops_alpha_when_show_alpha_false() {
        let mut svc = service(Props {
            default_value: ColorValue::new(0.0, 1.0, 0.5, 0.5),
            show_alpha: false,
            ..Props::default()
        });

        drop(svc.send(Event::SetFormat(ColorFormat::Rgb)));

        assert_eq!(svc.connect(&|_| {}).value_as_string(), "rgb(255, 0, 0)");
    }

    #[test]
    fn disabled_area_thumb_leaves_tab_order() {
        let enabled = open_service(Props::default());

        assert_eq!(
            enabled
                .connect(&|_| {})
                .area_thumb_attrs()
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );

        let disabled = service(Props {
            disabled: true,
            ..Props::default()
        });

        assert_eq!(
            disabled
                .connect(&|_| {})
                .area_thumb_attrs()
                .get(&HtmlAttr::TabIndex),
            Some("-1")
        );
    }

    #[test]
    fn channel_thumb_marks_dragging_only_for_its_channel() {
        let mut svc = open_service(Props::default());

        drop(svc.send(Event::DragStart {
            target: DragTarget::Channel(ColorChannel::Alpha),
            x: 0.5,
            y: 0.0,
        }));

        let api = svc.connect(&|_| {});

        assert!(
            api.channel_slider_thumb_attrs(ColorChannel::Alpha)
                .contains(&HtmlAttr::Data("ars-dragging"))
        );
        assert!(
            !api.channel_slider_thumb_attrs(ColorChannel::Hue)
                .contains(&HtmlAttr::Data("ars-dragging"))
        );
    }

    #[test]
    fn set_props_detects_a_late_chain_prop_change() {
        // Changing only the last operand of `context_relevant_props_changed`
        // forces every earlier `||` operand to be evaluated (all equal/false),
        // then applies the sync.
        let mut svc = service(Props::default());

        drop(svc.set_props(Props {
            id: "color-picker".to_string(),
            channel_large_step: 99.0,
            ..Props::default()
        }));

        assert!((svc.context().channel_large_step - 99.0).abs() < 1e-9);
    }

    #[test]
    fn set_props_detects_each_context_relevant_prop_individually() {
        // Each case changes exactly one context-relevant prop from the default,
        // so that operand is the first `||` term to be `true` — exercising the
        // "changed" side of every term in `context_relevant_props_changed`.
        let resync = |mutate: &dyn Fn(&mut Props)| {
            let mut svc = service(Props::default());

            let mut next = Props {
                id: "color-picker".to_string(),
                ..Props::default()
            };

            mutate(&mut next);

            let result = svc.set_props(next);

            (svc, result.state_changed)
        };

        let (svc, _) = resync(&|props| props.close_on_interact_outside = false);

        assert!(!svc.context().close_on_interact_outside);

        let (svc, _) = resync(&|props| props.close_on_escape = false);

        assert!(!svc.context().close_on_escape);

        let (svc, _) = resync(&|props| props.show_alpha = false);

        assert!(!svc.context().show_alpha);

        let (svc, _) = resync(&|props| props.color_space = ColorSpace::Rgb);

        assert_eq!(svc.context().color_space, ColorSpace::Rgb);

        let (svc, _) = resync(&|props| props.swatches = alloc::vec![ColorValue::default()]);

        assert_eq!(svc.context().swatches.len(), 1);

        let (svc, _) = resync(&|props| props.dir = Direction::Rtl);

        assert_eq!(svc.context().dir, Direction::Rtl);

        let (svc, _) = resync(&|props| props.positioning.placement = Placement::TopEnd);

        assert_eq!(svc.context().positioning.placement, Placement::TopEnd);

        let (svc, _) = resync(&|props| props.channel_step = 5.0);

        assert!((svc.context().channel_step - 5.0).abs() < 1e-9);
    }

    #[test]
    fn set_props_with_unchanged_open_does_not_retrigger_transition() {
        // Controlled-open held at `Some(true)` across a set_props that changes a
        // different prop: the `was != Some(now)` guard is `false`, so no
        // Open/Close is emitted and the picker stays open.
        let mut svc = service(Props {
            open: Some(true),
            ..Props::default()
        });

        assert_eq!(svc.state(), &State::Open);

        drop(svc.set_props(Props {
            id: "color-picker".to_string(),
            open: Some(true),
            channel_step: 3.0,
            ..Props::default()
        }));

        assert_eq!(svc.state(), &State::Open);
        assert!((svc.context().channel_step - 3.0).abs() < 1e-9);
    }

    #[test]
    fn on_swatch_click_out_of_range_dispatches_nothing() {
        let svc = open_service(Props::default()); // no swatches

        let captured = RefCell::new(Vec::new());

        let send = |event: Event| captured.borrow_mut().push(event);

        svc.connect(&send).on_swatch_click(0);

        assert!(captured.borrow().is_empty());
    }

    #[test]
    fn input_parts_reflect_disabled_and_readonly() {
        let disabled = service(Props {
            disabled: true,
            ..Props::default()
        });

        let api = disabled.connect(&|_| {});

        assert!(
            api.channel_input_attrs(ColorChannel::Red, 0)
                .contains(&HtmlAttr::Disabled)
        );
        assert!(api.hex_input_attrs().contains(&HtmlAttr::Disabled));
        assert!(
            api.eye_dropper_trigger_attrs()
                .contains(&HtmlAttr::Disabled)
        );
        assert!(api.hidden_input_attrs().contains(&HtmlAttr::Disabled));

        let readonly = service(Props {
            readonly: true,
            ..Props::default()
        });

        let api = readonly.connect(&|_| {});

        assert!(
            api.channel_input_attrs(ColorChannel::Red, 0)
                .contains(&HtmlAttr::ReadOnly)
        );
        assert!(api.hex_input_attrs().contains(&HtmlAttr::ReadOnly));
        // The eyedropper trigger is also disabled while read-only.
        assert!(
            api.eye_dropper_trigger_attrs()
                .contains(&HtmlAttr::Disabled)
        );
    }

    #[test]
    fn readonly_blocks_drag_move_and_eyedropper_result() {
        // Start a drag, then toggle read-only mid-drag: further moves are ignored.
        let mut svc = open_service(Props::default());

        drop(svc.send(Event::DragStart {
            target: DragTarget::Area,
            x: 0.25,
            y: 0.25,
        }));

        let mid = *svc.connect(&|_| {}).value();

        drop(svc.set_props(Props {
            id: "color-picker".to_string(),
            default_open: true,
            readonly: true,
            ..Props::default()
        }));
        drop(svc.send(Event::DragMove { x: 0.9, y: 0.9 }));

        assert_eq!(*svc.connect(&|_| {}).value(), mid);

        // A read-only picker also ignores an eyedropper result.
        let mut ro = open_service(Props {
            readonly: true,
            default_value: ColorValue::from_hsl(10.0, 0.5, 0.5),
            ..Props::default()
        });

        let before = *ro.connect(&|_| {}).value();

        drop(ro.send(Event::EyedropperResult(Some(ColorValue::default()))));

        assert_eq!(*ro.connect(&|_| {}).value(), before);
    }

    #[test]
    fn readonly_blocks_channel_increment_and_decrement() {
        let mut svc = open_service(Props {
            readonly: true,
            default_value: ColorValue::from_hsl(100.0, 0.5, 0.5),
            ..Props::default()
        });

        let before = *svc.connect(&|_| {}).value();

        drop(svc.send(Event::ChannelIncrement {
            channel: ColorChannel::Hue,
            step: 10.0,
        }));
        drop(svc.send(Event::ChannelDecrement {
            channel: ColorChannel::Hue,
            step: 10.0,
        }));

        assert_eq!(*svc.connect(&|_| {}).value(), before);
    }

    #[test]
    fn area_thumb_keyboard_covers_left_down_and_ignored_keys() {
        let svc = open_service(Props::default());

        let captured = RefCell::new(Vec::new());

        let send = |event: Event| captured.borrow_mut().push(event);

        let api = svc.connect(&send);

        api.on_area_thumb_keydown(&key(KeyboardKey::ArrowLeft), false);
        api.on_area_thumb_keydown(&key(KeyboardKey::ArrowDown), false);
        api.on_area_thumb_keydown(&key(KeyboardKey::Tab), false); // ignored

        let events = captured.borrow();

        assert_eq!(events.len(), 2, "Tab must not dispatch an event");
        assert!(matches!(
            events[0],
            Event::ChannelDecrement {
                channel: ColorChannel::Saturation,
                ..
            }
        ));
        assert!(matches!(
            events[1],
            Event::ChannelDecrement {
                channel: ColorChannel::Lightness,
                ..
            }
        ));
    }

    #[test]
    fn channel_slider_keyboard_covers_arrows_large_step_and_ignored_keys() {
        let svc = open_service(Props {
            channel_step: 1.0,
            channel_large_step: 15.0,
            ..Props::default()
        });

        let captured = RefCell::new(Vec::new());

        let send = |event: Event| captured.borrow_mut().push(event);

        let api = svc.connect(&send);

        // Shift selects the configured large step for the wide hue range.
        api.on_channel_slider_keydown(ColorChannel::Hue, &key(KeyboardKey::ArrowUp), true);
        api.on_channel_slider_keydown(ColorChannel::Hue, &key(KeyboardKey::ArrowDown), false);
        api.on_channel_slider_keydown(ColorChannel::Hue, &key(KeyboardKey::Tab), false); // ignored

        let events = captured.borrow();

        assert_eq!(events.len(), 2, "Tab must not dispatch an event");
        // Shift on the wide hue range uses the configured large step (15.0).
        assert_eq!(
            events[0],
            Event::ChannelIncrement {
                channel: ColorChannel::Hue,
                step: 15.0
            }
        );
        assert_eq!(
            events[1],
            Event::ChannelDecrement {
                channel: ColorChannel::Hue,
                step: 1.0
            }
        );
    }

    #[test]
    fn channel_slider_keyboard_rtl_mirrors_horizontal_arrows() {
        let svc = open_service(Props {
            dir: Direction::Rtl,
            ..Props::default()
        });

        let captured = RefCell::new(Vec::new());

        let send = |event: Event| captured.borrow_mut().push(event);

        let api = svc.connect(&send);

        api.on_channel_slider_keydown(ColorChannel::Hue, &key(KeyboardKey::ArrowRight), false);
        api.on_channel_slider_keydown(ColorChannel::Hue, &key(KeyboardKey::ArrowLeft), false);

        let events = captured.borrow();

        // RTL flips the horizontal arrows: ArrowRight decrements, ArrowLeft increments.
        assert!(matches!(events[0], Event::ChannelDecrement { .. }));
        assert!(matches!(events[1], Event::ChannelIncrement { .. }));
    }

    // ── Codex review #706: lifecycle / prop-sync / a11y fixes ──────

    #[test]
    fn runtime_color_space_survives_unrelated_set_props() {
        // A runtime ChangeColorSpace must not be reverted by a later SetProps
        // triggered by an unrelated prop change (e.g. `dir`).
        let mut svc = service(Props::default()); // default color_space = Hsl
        drop(svc.send(Event::ChangeColorSpace(ColorSpace::Rgb)));
        assert_eq!(svc.connect(&|_| {}).color_space(), ColorSpace::Rgb);

        drop(svc.set_props(Props {
            id: "color-picker".to_string(),
            dir: Direction::Rtl, // unrelated change; color_space prop unchanged (Hsl)
            ..Props::default()
        }));
        assert_eq!(
            svc.connect(&|_| {}).color_space(),
            ColorSpace::Rgb,
            "runtime color-space switch must survive an unrelated prop sync"
        );
        assert_eq!(svc.context().dir, Direction::Rtl);
    }

    #[test]
    fn controlled_color_space_prop_change_applies_and_announces() {
        let mut svc = service(Props::default());
        let mut result = svc.set_props(Props {
            id: "color-picker".to_string(),
            color_space: ColorSpace::Hsb,
            ..Props::default()
        });
        let names = run_effects(&svc, &mut result);
        assert_eq!(svc.connect(&|_| {}).color_space(), ColorSpace::Hsb);
        assert!(
            names.contains(&Effect::AnnounceColorSpace),
            "a controlled color-space prop change announces like a runtime switch"
        );
    }

    #[test]
    fn explicit_close_requests_resolve_from_dragging_state() {
        // Close
        let mut svc = open_service(Props::default());
        drop(svc.send(Event::DragStart {
            target: DragTarget::Area,
            x: 0.5,
            y: 0.5,
        }));
        assert!(matches!(svc.state(), State::Dragging { .. }));
        let mut closed = svc.send(Event::Close);
        assert_eq!(svc.state(), &State::Closed);
        assert!(
            run_effects(&svc, &mut closed).contains(&Effect::DetachClickOutside),
            "closing from a drag still emits the close-lifecycle effects"
        );

        // Escape
        let mut svc = open_service(Props::default());
        drop(svc.send(Event::DragStart {
            target: DragTarget::Channel(ColorChannel::Hue),
            x: 0.5,
            y: 0.0,
        }));
        drop(svc.send(Event::CloseOnEscape));
        assert_eq!(svc.state(), &State::Closed);

        // Parent-controlled open -> false while dragging.
        let mut svc = service(Props {
            open: Some(true),
            ..Props::default()
        });
        drop(svc.send(Event::DragStart {
            target: DragTarget::Area,
            x: 0.2,
            y: 0.2,
        }));
        assert!(matches!(svc.state(), State::Dragging { .. }));
        drop(svc.set_props(Props {
            id: "color-picker".to_string(),
            open: Some(false),
            ..Props::default()
        }));
        assert_eq!(svc.state(), &State::Closed);
    }

    #[test]
    fn trigger_is_type_button_to_avoid_form_submission() {
        let svc = service(Props::default());
        assert_eq!(
            svc.connect(&|_| {}).trigger_attrs().get(&HtmlAttr::Type),
            Some("button")
        );
    }

    #[test]
    fn disabled_thumbs_and_swatches_leave_tab_order() {
        let svc = service(Props {
            disabled: true,
            swatches: alloc::vec![ColorValue::default()],
            ..Props::default()
        });
        let api = svc.connect(&|_| {});

        let hue_thumb = api.channel_slider_thumb_attrs(ColorChannel::Hue);
        assert_eq!(hue_thumb.get(&HtmlAttr::TabIndex), Some("-1"));
        assert_eq!(
            hue_thumb.get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true")
        );

        let swatch = api.swatch_attrs(0);
        assert_eq!(swatch.get(&HtmlAttr::TabIndex), Some("-1"));
        assert_eq!(
            swatch.get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true")
        );

        // Enabled controls remain focusable.
        let enabled = open_service(Props::default());
        assert_eq!(
            enabled
                .connect(&|_| {})
                .channel_slider_thumb_attrs(ColorChannel::Hue)
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );
    }

    // ── Snapshots: every anatomy part + output-affecting branches ──

    #[test]
    fn snapshot_root_closed() {
        let svc = service(Props {
            id: "cp".into(),
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_root_closed",
            snapshot_attrs(&svc.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_root_open() {
        let svc = open_service(Props {
            id: "cp".into(),
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_root_open",
            snapshot_attrs(&svc.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_root_disabled() {
        let svc = service(Props {
            id: "cp".into(),
            disabled: true,
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_root_disabled",
            snapshot_attrs(&svc.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_root_readonly() {
        let svc = service(Props {
            id: "cp".into(),
            readonly: true,
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_root_readonly",
            snapshot_attrs(&svc.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_label() {
        let svc = service(Props {
            id: "cp".into(),
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_label",
            snapshot_attrs(&svc.connect(&|_| {}).label_attrs())
        );
    }

    #[test]
    fn snapshot_control() {
        let svc = service(Props {
            id: "cp".into(),
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_control",
            snapshot_attrs(&svc.connect(&|_| {}).control_attrs())
        );
    }

    #[test]
    fn snapshot_trigger_closed() {
        let svc = service(Props {
            id: "cp".into(),
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_trigger_closed",
            snapshot_attrs(&svc.connect(&|_| {}).trigger_attrs())
        );
    }

    #[test]
    fn snapshot_trigger_open() {
        let svc = open_service(Props {
            id: "cp".into(),
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_trigger_open",
            snapshot_attrs(&svc.connect(&|_| {}).trigger_attrs())
        );
    }

    #[test]
    fn snapshot_trigger_disabled() {
        let svc = service(Props {
            id: "cp".into(),
            disabled: true,
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_trigger_disabled",
            snapshot_attrs(&svc.connect(&|_| {}).trigger_attrs())
        );
    }

    #[test]
    fn snapshot_content() {
        let svc = open_service(Props {
            id: "cp".into(),
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_content",
            snapshot_attrs(&svc.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_area() {
        let svc = open_service(Props {
            id: "cp".into(),
            default_value: ColorValue::from_hsl(200.0, 0.6, 0.5),
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_area",
            snapshot_attrs(&svc.connect(&|_| {}).area_attrs())
        );
    }

    #[test]
    fn snapshot_area_thumb_idle() {
        let svc = open_service(Props {
            id: "cp".into(),
            default_value: ColorValue::from_hsl(120.0, 0.75, 0.4),
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_area_thumb_idle",
            snapshot_attrs(&svc.connect(&|_| {}).area_thumb_attrs())
        );
    }

    #[test]
    fn snapshot_area_thumb_dragging() {
        let mut svc = open_service(Props {
            id: "cp".into(),
            default_value: ColorValue::from_hsl(120.0, 0.75, 0.4),
            ..Props::default()
        });

        drop(svc.send(Event::DragStart {
            target: DragTarget::Area,
            x: 0.5,
            y: 0.5,
        }));

        assert_snapshot!(
            "color_picker_area_thumb_dragging",
            snapshot_attrs(&svc.connect(&|_| {}).area_thumb_attrs())
        );
    }

    #[test]
    fn snapshot_channel_slider_hue() {
        let svc = open_service(Props {
            id: "cp".into(),
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_channel_slider_hue",
            snapshot_attrs(&svc.connect(&|_| {}).channel_slider_attrs(ColorChannel::Hue))
        );
    }

    #[test]
    fn snapshot_channel_slider_thumb_hue() {
        let svc = open_service(Props {
            id: "cp".into(),
            default_value: ColorValue::from_hsl(180.0, 1.0, 0.5),
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_channel_slider_thumb_hue",
            snapshot_attrs(
                &svc.connect(&|_| {})
                    .channel_slider_thumb_attrs(ColorChannel::Hue)
            )
        );
    }

    #[test]
    fn snapshot_channel_slider_thumb_alpha_dragging() {
        let mut svc = open_service(Props {
            id: "cp".into(),
            default_value: ColorValue::new(180.0, 1.0, 0.5, 0.5),
            ..Props::default()
        });

        drop(svc.send(Event::DragStart {
            target: DragTarget::Channel(ColorChannel::Alpha),
            x: 0.5,
            y: 0.0,
        }));

        assert_snapshot!(
            "color_picker_channel_slider_thumb_alpha_dragging",
            snapshot_attrs(
                &svc.connect(&|_| {})
                    .channel_slider_thumb_attrs(ColorChannel::Alpha)
            )
        );
    }

    #[test]
    fn snapshot_alpha_slider() {
        let svc = open_service(Props {
            id: "cp".into(),
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_alpha_slider",
            snapshot_attrs(&svc.connect(&|_| {}).alpha_slider_attrs())
        );
    }

    #[test]
    fn snapshot_swatch_group() {
        let svc = open_service(Props {
            id: "cp".into(),
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_swatch_group",
            snapshot_attrs(&svc.connect(&|_| {}).swatch_group_attrs())
        );
    }

    #[test]
    fn snapshot_swatch_selected() {
        let red = ColorValue::from_hsl(0.0, 1.0, 0.5);

        let svc = open_service(Props {
            id: "cp".into(),
            default_value: red,
            swatches: alloc::vec![red, ColorValue::from_hsl(240.0, 1.0, 0.5)],
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_swatch_selected",
            snapshot_attrs(&svc.connect(&|_| {}).swatch_attrs(0))
        );
    }

    #[test]
    fn snapshot_swatch_unselected() {
        let red = ColorValue::from_hsl(0.0, 1.0, 0.5);

        let svc = open_service(Props {
            id: "cp".into(),
            default_value: red,
            swatches: alloc::vec![red, ColorValue::from_hsl(240.0, 1.0, 0.5)],
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_swatch_unselected",
            snapshot_attrs(&svc.connect(&|_| {}).swatch_attrs(1))
        );
    }

    #[test]
    fn snapshot_format_select() {
        let svc = open_service(Props {
            id: "cp".into(),
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_format_select",
            snapshot_attrs(&svc.connect(&|_| {}).format_select_attrs())
        );
    }

    #[test]
    fn snapshot_channel_input_rgb() {
        let svc = open_service(Props {
            id: "cp".into(),
            color_space: ColorSpace::Rgb,
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_channel_input_rgb",
            snapshot_attrs(
                &svc.connect(&|_| {})
                    .channel_input_attrs(ColorChannel::Red, 0)
            )
        );
    }

    #[test]
    fn snapshot_hex_input() {
        let svc = open_service(Props {
            id: "cp".into(),
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_hex_input",
            snapshot_attrs(&svc.connect(&|_| {}).hex_input_attrs())
        );
    }

    #[test]
    fn snapshot_eye_dropper_supported() {
        let mut svc = open_service(Props {
            id: "cp".into(),
            ..Props::default()
        });

        drop(svc.send(Event::SetEyedropperSupported(true)));

        assert_snapshot!(
            "color_picker_eye_dropper_supported",
            snapshot_attrs(&svc.connect(&|_| {}).eye_dropper_trigger_attrs())
        );
    }

    #[test]
    fn snapshot_eye_dropper_unsupported() {
        let svc = open_service(Props {
            id: "cp".into(),
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_eye_dropper_unsupported",
            snapshot_attrs(&svc.connect(&|_| {}).eye_dropper_trigger_attrs())
        );
    }

    #[test]
    fn snapshot_hidden_input() {
        let svc = service(Props {
            id: "cp".into(),
            name: Some("color".to_string()),
            default_value: ColorValue::from_hsl(120.0, 0.75, 0.4),
            ..Props::default()
        });

        assert_snapshot!(
            "color_picker_hidden_input",
            snapshot_attrs(&svc.connect(&|_| {}).hidden_input_attrs())
        );
    }
}
