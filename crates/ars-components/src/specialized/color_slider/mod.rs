//! `ColorSlider` component state machine and connect API.
//!
//! `ColorSlider` is a 1D slider that edits a single [`ColorChannel`]. It owns
//! the channel math, value state, orientation, keyboard behavior, and ARIA/data
//! attributes. Live track measurement, pointer capture, and position-to-value
//! conversion are adapter concerns: the adapter supplies an already-normalized
//! position in `0..=1` via [`Api::on_track_pointer_down`] (drag start) and
//! drives [`Event::DragMove`] / [`Event::DragEnd`] from its own pointer
//! listeners, exactly as the slider does.

use alloc::{format, string::String, vec::Vec};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ColorChannel, ColorValue, ComponentIds,
    ComponentMessages, ComponentPart, ConnectApi, CssProperty, Direction, Env, HtmlAttr,
    KeyboardKey, Locale, MessageFn, Orientation, PendingEffect, TransitionPlan, channel_range,
    channel_value, no_cleanup, with_channel,
};
use ars_interactions::KeyboardEventData;

/// Label for the slider thumb.
type LabelFn = dyn Fn(&Locale) -> String + Send + Sync;

/// Formats the `aria-valuetext`. Arguments: `reading` (channel-aware, e.g.
/// `"hue 180°"`) and `color_name` (the perceptual color name), plus `locale`.
type ValueTextFn = dyn Fn(&str, &str, &Locale) -> String + Send + Sync;

/// Consumer callback fired on drag-end / pointer release.
type ChangeEndFn = dyn Fn(ColorValue) + Send + Sync;

/// The states for the `ColorSlider` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// No interaction in progress.
    Idle,

    /// The user is dragging the thumb.
    Dragging,
}

/// The events for the `ColorSlider` component.
#[derive(Clone, Copy, Debug)]
pub enum Event {
    /// The user started dragging (normalized position `0..=1` along the track).
    DragStart {
        /// Normalized track position (`0..=1`).
        position: f64,
    },

    /// The user is moving while dragging.
    DragMove {
        /// Normalized track position (`0..=1`).
        position: f64,
    },

    /// The user released the drag.
    DragEnd,

    /// Increment the channel by `step`.
    Increment {
        /// The step amount.
        step: f64,
    },

    /// Decrement the channel by `step`.
    Decrement {
        /// The step amount.
        step: f64,
    },

    /// Snap the channel to its minimum.
    SetToMin,

    /// Snap the channel to its maximum.
    SetToMax,

    /// Focus entered the thumb.
    Focus {
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },

    /// Focus left the thumb.
    Blur,

    /// Controlled-value sync from the parent after `Service::set_props`.
    SyncValue(Option<ColorValue>),

    /// Refresh cached output props after `Service::set_props`.
    SetProps,
}

/// Typed identifier for side effects emitted by the `ColorSlider` machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Invoke `Props::on_change_end`.
    ChangeEnd,
}

/// The context for the `ColorSlider` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The current color value (controlled or uncontrolled).
    pub value: Bindable<ColorValue>,

    /// The slider's linear channel value in channel units (degrees for hue,
    /// `0..=1` for fractional channels, `0..=255` for RGB).
    ///
    /// This is the source of truth for the thumb position and `aria-valuenow`,
    /// and is kept *unwrapped* so the hue endpoint can reach `360°` distinctly.
    /// [`ColorValue`] normalizes hue into `[0, 360)` (360° stores as 0°/red), so
    /// reading the channel back from the color would otherwise collapse the max
    /// endpoint onto the minimum.
    pub slider_value: f64,

    /// Which channel this slider controls.
    pub channel: ColorChannel,

    /// Slider orientation.
    pub orientation: Orientation,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is read-only.
    pub readonly: bool,

    /// Whether the thumb is focused.
    pub focused: bool,

    /// Whether focus was via keyboard (for the focus-visible ring).
    pub focus_visible: bool,

    /// Step size for keyboard adjustment.
    pub step: f64,

    /// Large step size (Shift+Arrow or PageUp/PageDown).
    pub large_step: f64,

    /// Text direction for RTL-aware keyboard navigation.
    pub dir: Direction,

    /// Locale for internationalized messages.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component instance IDs.
    pub ids: ComponentIds,
}

/// The props for the `ColorSlider` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled value. When `Some`, the component is controlled.
    pub value: Option<ColorValue>,

    /// Default value for uncontrolled mode.
    pub default_value: ColorValue,

    /// Which channel this slider controls.
    pub channel: ColorChannel,

    /// Slider orientation.
    pub orientation: Orientation,

    /// Step size for keyboard adjustment.
    pub step: f64,

    /// Large step size for Shift+Arrow / `PageUp` / `PageDown`.
    pub large_step: f64,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is read-only.
    pub readonly: bool,

    /// Text direction for RTL-aware keyboard navigation.
    pub dir: Direction,

    /// Name attribute for the hidden form input.
    pub name: Option<String>,

    /// Fired on `Event::DragEnd` / pointer release.
    pub on_change_end: Option<Callback<ChangeEndFn>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: ColorValue::default(),
            channel: ColorChannel::Hue,
            orientation: Orientation::Horizontal,
            step: 1.0,
            large_step: 10.0,
            disabled: false,
            readonly: false,
            dir: Direction::Ltr,
            name: None,
            on_change_end: None,
        }
    }
}

/// The messages for the `ColorSlider` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Label for the slider. Default: `"Color channel"`.
    pub label: MessageFn<LabelFn>,

    /// Formats the channel value for `aria-valuetext`.
    pub value_text: MessageFn<ValueTextFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            label: MessageFn::static_str("Color channel"),
            value_text: MessageFn::new(|reading: &str, color_name: &str, _locale: &Locale| {
                format!("{reading}, {color_name}")
            }),
        }
    }
}

impl ComponentMessages for Messages {}

/// Apply a normalized position (`0..=1`) to the channel value.
///
/// A horizontal RTL slider renders the minimum on the right and mirrors its
/// arrow keys, so the incoming physical position is inverted to match (dragging
/// the left edge selects the maximum).
fn apply_slider_position(ctx: &mut Context, position: f64) {
    let (min, max) = channel_range(ctx.channel);

    let clamped = position.clamp(0.0, 1.0);
    let effective = if ctx.orientation == Orientation::Horizontal && ctx.dir == Direction::Rtl {
        1.0 - clamped
    } else {
        clamped
    };

    set_channel_value(ctx, min + effective * (max - min));
}

/// Set the slider's channel value (in channel units) and derive the color.
///
/// `slider_value` is stored unwrapped so the hue endpoint stays distinct; the
/// color is derived via [`with_channel`], which normalizes hue (360° → red).
fn set_channel_value(ctx: &mut Context, value: f64) {
    ctx.slider_value = value;

    let color = *ctx.value.get();
    ctx.value.set(with_channel(&color, ctx.channel, value));
}

/// Build the change-end effect that invokes `Props::on_change_end`.
///
/// Reports the *pending* value (the one staged during the drag) rather than the
/// controlled `get()` value: in controlled mode the parent has not yet synced
/// the new value back through its prop, so `get()` would still return the stale
/// pre-drag color.
fn change_end_effect() -> PendingEffect<Machine> {
    PendingEffect::new(Effect::ChangeEnd, |ctx: &Context, props: &Props, _send| {
        if let Some(callback) = &props.on_change_end {
            callback(*ctx.value.pending());
        }

        no_cleanup()
    })
}

/// The machine for the `ColorSlider` component.
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
        let value = if let Some(v) = &props.value {
            Bindable::controlled(*v)
        } else {
            Bindable::uncontrolled(props.default_value)
        };

        let slider_value = channel_value(value.get(), props.channel);

        let context = Context {
            value,
            slider_value,
            channel: props.channel,
            orientation: props.orientation,
            disabled: props.disabled,
            readonly: props.readonly,
            focused: false,
            focus_visible: false,
            step: props.step,
            large_step: props.large_step,
            dir: props.dir,
            locale: env.locale.clone(),
            messages: messages.clone(),
            ids: ComponentIds::from_id(&props.id),
        };

        (State::Idle, context)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // A disabled slider ignores value-changing input but still tracks focus
        // and accepts parent-driven prop syncs (so it can be re-enabled).
        // `DragEnd` is allowed through so a drag in flight when the parent
        // disabled the control can still terminate cleanly.
        if ctx.disabled {
            match event {
                Event::DragStart { .. }
                | Event::DragMove { .. }
                | Event::Increment { .. }
                | Event::Decrement { .. }
                | Event::SetToMin
                | Event::SetToMax => return None,
                _ => {}
            }
        }

        match (state, event) {
            (State::Idle, Event::DragStart { position }) => {
                if ctx.readonly {
                    return None;
                }

                let pos = *position;
                Some(
                    TransitionPlan::to(State::Dragging).apply(move |ctx: &mut Context| {
                        apply_slider_position(ctx, pos);
                    }),
                )
            }

            (State::Dragging, Event::DragMove { position }) => {
                // Readonly toggled mid-drag must stop further value changes
                // (disabled is already handled by the guard above); DragEnd
                // still terminates the drag.
                if ctx.readonly {
                    return None;
                }

                let pos = *position;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    apply_slider_position(ctx, pos);
                }))
            }

            (State::Dragging, Event::DragEnd) => {
                Some(TransitionPlan::to(State::Idle).with_effect(change_end_effect()))
            }

            (_, Event::Increment { step }) => {
                if ctx.readonly {
                    return None;
                }

                let step = *step;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let (_, max) = channel_range(ctx.channel);
                    set_channel_value(ctx, (ctx.slider_value + step).min(max));
                }))
            }

            (_, Event::Decrement { step }) => {
                if ctx.readonly {
                    return None;
                }

                let step = *step;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let (min, _) = channel_range(ctx.channel);
                    set_channel_value(ctx, (ctx.slider_value - step).max(min));
                }))
            }

            (_, Event::SetToMin) => {
                if ctx.readonly {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    let (min, _) = channel_range(ctx.channel);
                    set_channel_value(ctx, min);
                }))
            }

            (_, Event::SetToMax) => {
                if ctx.readonly {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    let (_, max) = channel_range(ctx.channel);
                    set_channel_value(ctx, max);
                }))
            }

            (_, Event::Focus { is_keyboard }) => {
                let kb = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused = true;
                    ctx.focus_visible = kb;
                }))
            }

            (_, Event::Blur) => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.focused = false;
                ctx.focus_visible = false;
            })),

            (_, Event::SyncValue(value)) => {
                let value = *value;
                Some(TransitionPlan::context_only(
                    move |ctx: &mut Context| match value {
                        Some(color) => {
                            ctx.value.set(color);
                            ctx.value.sync_controlled(Some(color));
                            // Re-derive the slider value from the parent's color.
                            ctx.slider_value = channel_value(&color, ctx.channel);
                        }
                        None => ctx.value.sync_controlled(None),
                    },
                ))
            }

            (_, Event::SetProps) => {
                let props = props.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let channel_changed = ctx.channel != props.channel;

                    ctx.channel = props.channel;
                    ctx.orientation = props.orientation;
                    ctx.step = props.step;
                    ctx.large_step = props.large_step;
                    ctx.disabled = props.disabled;
                    ctx.readonly = props.readonly;
                    ctx.dir = props.dir;

                    // A new channel means the cached slider value refers to the
                    // old channel; re-derive it from the current color.
                    if channel_changed {
                        ctx.slider_value = channel_value(ctx.value.get(), ctx.channel);
                    }
                }))
            }

            _ => None,
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        assert_eq!(
            old.id, new.id,
            "color_slider::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.value != new.value {
            events.push(Event::SyncValue(new.value));
        }

        if props_output_changed(old, new) {
            events.push(Event::SetProps);
        }

        events
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

/// Whether any cached output prop changed and the context needs refreshing.
///
/// `name` is omitted: it is read live from `Props` in `hidden_input_attrs`
/// rather than cached in the context, so a name-only change needs no resync.
fn props_output_changed(old: &Props, new: &Props) -> bool {
    old.channel != new.channel
        || old.orientation != new.orientation
        || (old.step - new.step).abs() > f64::EPSILON
        || (old.large_step - new.large_step).abs() > f64::EPSILON
        || old.disabled != new.disabled
        || old.readonly != new.readonly
        || old.dir != new.dir
}

/// Structural parts exposed by the `ColorSlider` connect API.
#[derive(ComponentPart)]
#[scope = "color-slider"]
pub enum Part {
    /// Container with `role="group"`.
    Root,

    /// `<label>` whose `for` points at the thumb.
    Label,

    /// Gradient track.
    Track,

    /// Draggable thumb with `role="slider"`.
    Thumb,

    /// `<output>` mirroring the value.
    Output,

    /// `type="hidden"` input that submits the hex value for forms.
    HiddenInput,
}

/// The connect API for the `ColorSlider` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("color_slider::Api")
            .field("state", &self.state)
            .field("ctx", &self.ctx)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Whether the thumb is currently being dragged.
    #[must_use]
    pub const fn is_dragging(&self) -> bool {
        matches!(self.state, State::Dragging)
    }

    /// The current color value.
    #[must_use]
    pub fn value(&self) -> &ColorValue {
        self.ctx.value.get()
    }

    /// The current channel value formatted for display.
    #[must_use]
    pub fn formatted_value(&self) -> String {
        // Use the unwrapped slider value so the hue endpoint reads "360°".
        let val = self.ctx.slider_value;

        match self.ctx.channel {
            ColorChannel::Hue => format!("{val:.0}°"),
            ColorChannel::Red | ColorChannel::Green | ColorChannel::Blue => format!("{val:.0}"),
            _ => format!("{:.0}%", val * 100.0),
        }
    }

    fn orientation_str(&self) -> &'static str {
        if self.ctx.orientation == Orientation::Vertical {
            "vertical"
        } else {
            "horizontal"
        }
    }

    /// Attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "group")
            .set(
                HtmlAttr::Data("ars-channel"),
                format!("{:?}", self.ctx.channel).to_lowercase(),
            )
            .set(HtmlAttr::Data("ars-orientation"), self.orientation_str());

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        if self.is_dragging() {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
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
            .set(HtmlAttr::For, self.ctx.ids.part("thumb"));

        attrs
    }

    /// Attributes for the gradient track element.
    #[must_use]
    pub fn track_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Track.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        // Use the pending color so the gradient matches the in-progress drag
        // position in controlled mode (where `get()` returns the stale prop).
        let color = self.ctx.value.pending();

        let gradient = match self.ctx.channel {
            ColorChannel::Hue => "linear-gradient(to right, \
                hsl(0,100%,50%), hsl(60,100%,50%), hsl(120,100%,50%), \
                hsl(180,100%,50%), hsl(240,100%,50%), hsl(300,100%,50%), \
                hsl(360,100%,50%))"
                .to_string(),

            ColorChannel::Alpha => format!(
                // Fade from the *same* color at alpha 0 to alpha 1, so the track
                // previews only opacity. `transparent` is transparent black and
                // would make non-black colors fade through gray.
                "linear-gradient(to right, {}, {})",
                ColorValue::new(color.hue, color.saturation, color.lightness, 0.0).to_css_hsl(),
                ColorValue::new(color.hue, color.saturation, color.lightness, 1.0).to_css_hsl()
            ),

            _ => {
                let (min, max) = channel_range(self.ctx.channel);

                let start = with_channel(color, self.ctx.channel, min);
                let end = with_channel(color, self.ctx.channel, max);

                format!(
                    "linear-gradient(to right, {}, {})",
                    start.to_css_hsl(),
                    end.to_css_hsl()
                )
            }
        };

        attrs.set_style(CssProperty::Custom("ars-color-slider-track-bg"), gradient);

        attrs
    }

    /// Attributes for the draggable thumb element.
    #[must_use]
    pub fn thumb_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Thumb.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("thumb"))
            .set(HtmlAttr::Role, "slider")
            // A disabled control must stay out of the tab order.
            .set(
                HtmlAttr::TabIndex,
                if self.ctx.disabled { "-1" } else { "0" },
            );

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        // Pending color so the thumb background and the valuetext color name
        // match the in-progress drag (controlled `get()` returns the old prop).
        let color = self.ctx.value.pending();

        // The unwrapped slider value drives aria-valuenow and the thumb position
        // so the hue endpoint stays at 360° instead of wrapping to 0°.
        let val = self.ctx.slider_value;

        let (min, max) = channel_range(self.ctx.channel);

        // Channel-aware reading ("hue 180°") plus the perceptual color name, as
        // required by spec §3.1 (e.g. "hue 180°, dark vibrant blue").
        let channel_name = format!("{:?}", self.ctx.channel).to_lowercase();
        let reading = format!("{channel_name} {}", self.formatted_value());
        let color_name = color.color_name_en();

        attrs
            .set(HtmlAttr::Aria(AriaAttr::ValueNow), format!("{val:.2}"))
            .set(HtmlAttr::Aria(AriaAttr::ValueMin), format!("{min:.2}"))
            .set(HtmlAttr::Aria(AriaAttr::ValueMax), format!("{max:.2}"))
            .set(
                HtmlAttr::Aria(AriaAttr::Orientation),
                self.orientation_str(),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.label)(&self.ctx.locale),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::ValueText),
                (self.ctx.messages.value_text)(&reading, &color_name, &self.ctx.locale),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );

        let mut pct = if (max - min).abs() > f64::EPSILON {
            (val - min) / (max - min) * 100.0
        } else {
            0.0
        };

        // A horizontal RTL slider flips its axis (min on the right), matching the
        // mirrored arrow-key handling in `on_thumb_keydown`.
        if self.ctx.orientation == Orientation::Horizontal && self.ctx.dir == Direction::Rtl {
            pct = 100.0 - pct;
        }

        attrs
            .set_style(
                CssProperty::Custom("ars-color-slider-thumb-position"),
                format!("{pct:.1}%"),
            )
            .set_style(CssProperty::BackgroundColor, color.to_css_hsl());

        if self.is_dragging() {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs
    }

    /// Attributes for the output element.
    #[must_use]
    pub fn output_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Output.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::For, self.ctx.ids.part("thumb"))
            .set(HtmlAttr::Aria(AriaAttr::Live), "off");

        attrs
    }

    /// Attributes for the hidden form input.
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

        attrs.set(HtmlAttr::Value, self.ctx.value.pending().to_hex(true));

        // A disabled control must be omitted from form submission.
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Handles a keydown on the thumb. `shift` selects the large step.
    ///
    /// For a horizontal slider in RTL, the arrow direction is mirrored so
    /// `ArrowLeft` increments and `ArrowRight` decrements. Vertical sliders and
    /// `PageUp` / `PageDown` / Home / End are direction-independent.
    pub fn on_thumb_keydown(&self, data: &KeyboardEventData, shift: bool) {
        let step = if shift {
            self.ctx.large_step
        } else {
            self.ctx.step
        };

        let is_rtl_horizontal =
            self.ctx.orientation == Orientation::Horizontal && self.ctx.dir == Direction::Rtl;

        match data.key {
            KeyboardKey::ArrowRight | KeyboardKey::ArrowLeft => {
                let forward = matches!(data.key, KeyboardKey::ArrowRight) ^ is_rtl_horizontal;

                (self.send)(if forward {
                    Event::Increment { step }
                } else {
                    Event::Decrement { step }
                });
            }

            KeyboardKey::ArrowUp => (self.send)(Event::Increment { step }),

            KeyboardKey::ArrowDown => (self.send)(Event::Decrement { step }),

            KeyboardKey::Home => (self.send)(Event::SetToMin),

            KeyboardKey::End => (self.send)(Event::SetToMax),

            KeyboardKey::PageUp => (self.send)(Event::Increment {
                step: self.ctx.large_step,
            }),

            KeyboardKey::PageDown => (self.send)(Event::Decrement {
                step: self.ctx.large_step,
            }),

            _ => {}
        }
    }

    /// Dispatches a drag-start from an adapter-resolved normalized position.
    pub fn on_track_pointer_down(&self, position: f64) {
        (self.send)(Event::DragStart { position });
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Track => self.track_attrs(),
            Part::Thumb => self.thumb_attrs(),
            Part::Output => self.output_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use ars_core::Service;
    use insta::assert_snapshot;

    use super::*;

    fn service(mut props: Props) -> Service<Machine> {
        if props.id.is_empty() {
            props.id = "color-slider".to_string();
        }

        Service::<Machine>::new(props, &Env::default(), &Messages::default())
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

    #[test]
    fn thumb_position_maps_to_channel() {
        let mut svc = service(Props {
            channel: ColorChannel::Hue,
            ..Props::default()
        });

        drop(svc.send(Event::DragStart { position: 0.5 }));

        // Hue range 0..360, position 0.5 -> 180.
        assert!((svc.connect(&|_| {}).value().hue - 180.0).abs() < 1e-9);
    }

    #[test]
    fn thumb_exposes_slider_role_and_value_range() {
        let svc = service(Props {
            channel: ColorChannel::Hue,
            default_value: ColorValue::from_hsl(90.0, 1.0, 0.5),
            ..Props::default()
        });

        let thumb = svc.connect(&|_| {}).thumb_attrs();

        assert_eq!(thumb.get(&HtmlAttr::Role), Some("slider"));
        assert_eq!(
            thumb.get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("90.00")
        );
        assert_eq!(thumb.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("0.00"));
        assert_eq!(
            thumb.get(&HtmlAttr::Aria(AriaAttr::ValueMax)),
            Some("360.00")
        );
        assert_eq!(
            thumb.get(&HtmlAttr::Aria(AriaAttr::Orientation)),
            Some("horizontal")
        );
    }

    #[test]
    fn vertical_orientation_in_aria_and_data() {
        let svc = service(Props {
            orientation: Orientation::Vertical,
            ..Props::default()
        });

        let api = svc.connect(&|_| {});

        assert_eq!(
            api.thumb_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Orientation)),
            Some("vertical")
        );
        assert_eq!(
            api.root_attrs().get(&HtmlAttr::Data("ars-orientation")),
            Some("vertical")
        );
    }

    #[test]
    fn keyboard_step_increments_and_clamps() {
        let mut svc = service(Props {
            channel: ColorChannel::Saturation,
            step: 0.1,
            default_value: ColorValue::from_hsl(0.0, 0.5, 0.5),
            ..Props::default()
        });

        drop(svc.send(Event::Increment { step: 0.1 }));

        assert!((svc.connect(&|_| {}).value().saturation - 0.6).abs() < 1e-9);

        drop(svc.send(Event::SetToMax));

        assert!((svc.connect(&|_| {}).value().saturation - 1.0).abs() < 1e-9);
    }

    #[test]
    fn rtl_horizontal_mirrors_arrows_but_vertical_does_not() {
        let svc = service(Props {
            dir: Direction::Rtl,
            ..Props::default()
        });

        let captured = core::cell::RefCell::new(Vec::new());
        let send = |event: Event| captured.borrow_mut().push(event);

        svc.connect(&send)
            .on_thumb_keydown(&key(KeyboardKey::ArrowRight), false);

        assert!(matches!(captured.borrow()[0], Event::Decrement { .. }));

        let vsvc = service(Props {
            dir: Direction::Rtl,
            orientation: Orientation::Vertical,
            ..Props::default()
        });

        let vcap = core::cell::RefCell::new(Vec::new());

        let vsend = |event: Event| vcap.borrow_mut().push(event);

        vsvc.connect(&vsend)
            .on_thumb_keydown(&key(KeyboardKey::ArrowUp), false);

        assert!(matches!(vcap.borrow()[0], Event::Increment { .. }));
    }

    #[test]
    fn rtl_horizontal_inverts_pointer_position() {
        // In RTL, the physical left edge (position 0.0) is the visual maximum.
        // Use saturation (0..=1, no hue wrap) so the endpoint reads cleanly.
        let mut rtl = service(Props {
            channel: ColorChannel::Saturation,
            dir: Direction::Rtl,
            default_value: ColorValue::from_hsl(0.0, 0.0, 0.5),
            ..Props::default()
        });
        drop(rtl.send(Event::DragStart { position: 0.0 }));
        assert!(
            (rtl.connect(&|_| {}).value().saturation - 1.0).abs() < 1e-9,
            "RTL position 0.0 must select the maximum"
        );

        // LTR is unchanged: position 0.0 -> minimum.
        let mut ltr = service(Props {
            channel: ColorChannel::Saturation,
            default_value: ColorValue::from_hsl(0.0, 1.0, 0.5),
            ..Props::default()
        });
        drop(ltr.send(Event::DragStart { position: 0.0 }));
        assert!((ltr.connect(&|_| {}).value().saturation - 0.0).abs() < 1e-9);
    }

    #[test]
    fn rtl_horizontal_mirrors_thumb_position() {
        let position = |dir: Direction| {
            let svc = service(Props {
                channel: ColorChannel::Hue,
                dir,
                default_value: ColorValue::from_hsl(0.0, 1.0, 0.5), // hue 0 = min
                ..Props::default()
            });
            svc.connect(&|_| {})
                .thumb_attrs()
                .styles()
                .iter()
                .find(|(p, _)| *p == CssProperty::Custom("ars-color-slider-thumb-position"))
                .map(|(_, value)| value.clone())
                .expect("thumb position style")
        };

        // hue 0 is the minimum: LTR puts it at the left (0%), RTL at the right.
        assert_eq!(position(Direction::Ltr), "0.0%");
        assert_eq!(position(Direction::Rtl), "100.0%");
    }

    #[test]
    fn alpha_track_previews_only_opacity() {
        // A white alpha slider must fade transparent-white -> white, not through
        // black: the zero stop is the same color at alpha 0, never `transparent`.
        let svc = service(Props {
            channel: ColorChannel::Alpha,
            default_value: ColorValue::new(0.0, 0.0, 1.0, 1.0), // white
            ..Props::default()
        });

        let bg = svc
            .connect(&|_| {})
            .track_attrs()
            .styles()
            .iter()
            .find(|(p, _)| *p == CssProperty::Custom("ars-color-slider-track-bg"))
            .map(|(_, value)| value.clone())
            .expect("track bg style");

        assert!(
            !bg.contains("transparent"),
            "must not fade through black: {bg}"
        );
        assert!(
            bg.contains(", 0.00)"),
            "zero stop must be the color at alpha 0: {bg}"
        );
    }

    #[test]
    fn hue_track_uses_rainbow_gradient() {
        let svc = service(Props {
            channel: ColorChannel::Hue,
            ..Props::default()
        });

        let track = svc.connect(&|_| {}).track_attrs();

        let bg = track
            .styles()
            .iter()
            .find(|(p, _)| *p == CssProperty::Custom("ars-color-slider-track-bg"))
            .map(|(_, v)| v.clone())
            .unwrap();

        assert!(bg.contains("hsl(0,100%,50%)") && bg.contains("hsl(360,100%,50%)"));
    }

    #[test]
    fn drag_lifecycle_and_change_end_effect() {
        use alloc::sync::Arc;
        use core::sync::atomic::{AtomicBool, Ordering};

        use ars_core::{StrongSend, callback};

        let fired = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&fired);
        let mut svc = service(Props {
            on_change_end: Some(callback(move |_c: ColorValue| {
                flag.store(true, Ordering::SeqCst);
            })),
            ..Props::default()
        });

        drop(svc.send(Event::DragStart { position: 0.25 }));

        assert_eq!(svc.state(), &State::Dragging);

        let mut end = svc.send(Event::DragEnd);

        assert_eq!(svc.state(), &State::Idle);

        let send: StrongSend<Event> = Arc::new(|_| {});
        for effect in end.pending_effects.drain(..) {
            drop(effect.run(svc.context(), svc.props(), Arc::clone(&send)));
        }

        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn thumb_value_text_includes_channel_reading_and_color_name() {
        let color = ColorValue::from_hsl(180.0, 1.0, 0.5);
        let svc = service(Props {
            channel: ColorChannel::Hue,
            default_value: color,
            ..Props::default()
        });

        let value_text = svc
            .connect(&|_| {})
            .thumb_attrs()
            .get(&HtmlAttr::Aria(AriaAttr::ValueText))
            .expect("value text present")
            .to_string();

        assert!(
            value_text.contains("hue 180°"),
            "channel reading missing from '{value_text}'"
        );
        assert!(
            value_text.contains(&color.color_name_en()),
            "perceptual color name missing from '{value_text}'"
        );
    }

    #[test]
    fn drag_end_reports_pending_value_for_controlled_slider() {
        use alloc::sync::Arc;
        use core::sync::atomic::{AtomicU64, Ordering};

        use ars_core::{StrongSend, callback};

        // Controlled hue slider starting at red (hue 0).
        let reported = Arc::new(AtomicU64::new(u64::MAX));
        let sink = Arc::clone(&reported);
        let mut svc = service(Props {
            channel: ColorChannel::Hue,
            value: Some(ColorValue::from_hsl(0.0, 1.0, 0.5)),
            on_change_end: Some(callback(move |color: ColorValue| {
                sink.store(color.hue.to_bits(), Ordering::SeqCst);
            })),
            ..Props::default()
        });

        // Drag to the far end (hue 360 -> stored as wrapped value) and release.
        drop(svc.send(Event::DragStart { position: 0.5 }));
        drop(svc.send(Event::DragMove { position: 0.75 }));
        let mut end = svc.send(Event::DragEnd);

        let send: StrongSend<Event> = Arc::new(|_| {});
        for effect in end.pending_effects.drain(..) {
            drop(effect.run(svc.context(), svc.props(), Arc::clone(&send)));
        }

        // The callback must receive the dragged hue (270), not the stale
        // controlled hue (0). `get()` would still return 0 here.
        let reported_hue = f64::from_bits(reported.load(Ordering::SeqCst));
        assert!(
            (reported_hue - 270.0).abs() < 1e-9,
            "on_change_end must report the pending dragged hue, got {reported_hue}"
        );
        assert!((svc.connect(&|_| {}).value().hue - 0.0).abs() < 1e-9);
    }

    #[test]
    fn set_props_syncs_controlled_value_and_flags() {
        let mut svc = service(Props {
            channel: ColorChannel::Hue,
            value: Some(ColorValue::from_hsl(0.0, 1.0, 0.5)),
            ..Props::default()
        });

        drop(svc.set_props(Props {
            id: "color-slider".to_string(),
            channel: ColorChannel::Hue,
            value: Some(ColorValue::from_hsl(240.0, 1.0, 0.5)),
            disabled: true,
            ..Props::default()
        }));

        let api = svc.connect(&|_| {});
        assert!(
            (api.value().hue - 240.0).abs() < 1e-9,
            "controlled value must follow the new prop"
        );
        assert!(
            api.root_attrs().contains(&HtmlAttr::Data("ars-disabled")),
            "disabled flag must sync"
        );

        // A re-enable sync must also take effect (regression guard for the
        // disabled-state prop-sync path).
        drop(svc.set_props(Props {
            id: "color-slider".to_string(),
            channel: ColorChannel::Hue,
            value: Some(ColorValue::from_hsl(240.0, 1.0, 0.5)),
            disabled: false,
            ..Props::default()
        }));
        assert!(
            !svc.connect(&|_| {})
                .root_attrs()
                .contains(&HtmlAttr::Data("ars-disabled"))
        );

        // Clearing the controlled value returns the bindable to uncontrolled.
        drop(svc.set_props(Props {
            id: "color-slider".to_string(),
            channel: ColorChannel::Hue,
            value: None,
            ..Props::default()
        }));
    }

    #[test]
    fn controlled_drag_display_uses_pending_color() {
        // Controlled hue slider starting at red. The thumb position uses the
        // pending slider_value, so the thumb background and hidden input must
        // use the pending color too — not the stale controlled red.
        let mut svc = service(Props {
            channel: ColorChannel::Hue,
            value: Some(ColorValue::from_hsl(0.0, 1.0, 0.5)),
            name: Some("hue".to_string()),
            ..Props::default()
        });

        drop(svc.send(Event::DragStart {
            position: 2.0 / 3.0,
        })); // hue 240 (blue)

        let api = svc.connect(&|_| {});
        // value() still reflects the controlled prop (red).
        assert!((api.value().hue - 0.0).abs() < 1e-9);
        // The hidden input submits the pending (blue) color, matching the thumb.
        let hidden = api.hidden_input_attrs();
        let submitted = ColorValue::from_hex(hidden.get(&HtmlAttr::Value).unwrap()).expect("hex");
        assert!(
            (submitted.hue - 240.0).abs() < 1.0,
            "hidden input must carry the pending hue, got {}",
            submitted.hue
        );
    }

    #[test]
    fn hue_max_endpoint_does_not_wrap_to_min() {
        let mut svc = service(Props {
            channel: ColorChannel::Hue,
            default_value: ColorValue::from_hsl(180.0, 1.0, 0.5),
            ..Props::default()
        });

        // End key reaches the 360° endpoint distinctly from the 0° minimum.
        drop(svc.send(Event::SetToMax));

        let api = svc.connect(&|_| {});
        assert_eq!(
            api.thumb_attrs().get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("360.00"),
            "aria-valuenow must stay at the max endpoint, not wrap to 0"
        );
        assert_eq!(api.formatted_value(), "360°");
        // The derived color is red (hue 360° normalizes to 0°).
        assert_eq!(api.value().to_rgb(), (255, 0, 0));

        // Dragging to the far end also lands on 360°, not 0°.
        drop(svc.send(Event::DragStart { position: 1.0 }));
        assert_eq!(
            svc.connect(&|_| {})
                .thumb_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("360.00")
        );
    }

    #[test]
    fn disabled_thumb_leaves_tab_order() {
        let enabled = service(Props::default());
        assert_eq!(
            enabled
                .connect(&|_| {})
                .thumb_attrs()
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );

        let disabled = service(Props {
            disabled: true,
            ..Props::default()
        });
        let thumb = disabled.connect(&|_| {}).thumb_attrs();
        assert_eq!(thumb.get(&HtmlAttr::TabIndex), Some("-1"));
        assert_eq!(thumb.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
    }

    #[test]
    fn readonly_set_mid_drag_blocks_further_moves() {
        let mut svc = service(Props {
            channel: ColorChannel::Hue,
            ..Props::default()
        });

        drop(svc.send(Event::DragStart { position: 0.25 }));
        let after_start = svc.connect(&|_| {}).value().hue;

        // Parent flips readonly during the drag.
        drop(svc.set_props(Props {
            id: "color-slider".to_string(),
            channel: ColorChannel::Hue,
            readonly: true,
            ..Props::default()
        }));

        // Subsequent moves must not change the value.
        drop(svc.send(Event::DragMove { position: 0.9 }));
        assert!(
            (svc.connect(&|_| {}).value().hue - after_start).abs() < 1e-9,
            "readonly drag must not mutate the value"
        );

        // The drag still terminates cleanly.
        drop(svc.send(Event::DragEnd));
        assert_eq!(svc.state(), &State::Idle);
    }

    #[test]
    fn drag_end_terminates_after_mid_drag_disable() {
        let mut svc = service(Props {
            channel: ColorChannel::Hue,
            ..Props::default()
        });

        drop(svc.send(Event::DragStart { position: 0.5 }));
        assert_eq!(svc.state(), &State::Dragging);

        // Parent disables the control mid-drag.
        drop(svc.set_props(Props {
            id: "color-slider".to_string(),
            channel: ColorChannel::Hue,
            disabled: true,
            ..Props::default()
        }));

        // Pointer-up must still exit the drag rather than wedging in Dragging.
        let end = svc.send(Event::DragEnd);
        assert_eq!(svc.state(), &State::Idle);
        assert_eq!(end.pending_effects.len(), 1, "change-end still fires");
        assert!(
            !svc.connect(&|_| {})
                .root_attrs()
                .contains(&HtmlAttr::Data("ars-dragging"))
        );
    }

    #[test]
    fn disabled_slider_omits_hidden_input_from_submission() {
        let svc = service(Props {
            name: Some("hue".to_string()),
            disabled: true,
            ..Props::default()
        });

        let hidden = svc.connect(&|_| {}).hidden_input_attrs();

        assert_eq!(hidden.get(&HtmlAttr::Disabled), Some("true"));
    }

    #[test]
    fn formatted_value_per_channel() {
        let hue = service(Props {
            channel: ColorChannel::Hue,
            default_value: ColorValue::from_hsl(200.0, 1.0, 0.5),
            ..Props::default()
        });

        assert_eq!(hue.connect(&|_| {}).formatted_value(), "200°");

        let alpha = service(Props {
            channel: ColorChannel::Alpha,
            default_value: ColorValue::new(0.0, 1.0, 0.5, 0.4),
            ..Props::default()
        });

        assert_eq!(alpha.connect(&|_| {}).formatted_value(), "40%");
    }

    #[test]
    fn thumb_hue_snapshot() {
        let svc = service(Props {
            id: "cs".to_string(),
            channel: ColorChannel::Hue,
            default_value: ColorValue::from_hsl(120.0, 1.0, 0.5),
            ..Props::default()
        });

        assert_snapshot!(
            "color_slider_thumb_hue",
            snapshot_attrs(&svc.connect(&|_| {}).thumb_attrs())
        );
    }

    #[test]
    fn root_vertical_snapshot() {
        let svc = service(Props {
            id: "cs".to_string(),
            channel: ColorChannel::Alpha,
            orientation: Orientation::Vertical,
            ..Props::default()
        });

        assert_snapshot!(
            "color_slider_root_vertical",
            snapshot_attrs(&svc.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn exhaustive_events_parts_and_helpers() {
        let mut svc = Service::<Machine>::new(
            Props {
                id: "cs".into(),
                value: Some(ColorValue::from_hsl(120.0, 0.5, 0.5)),
                channel: ColorChannel::Alpha,
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        for ev in [
            Event::Focus { is_keyboard: true },
            Event::DragStart { position: 0.2 },
            Event::DragMove { position: 0.7 },
            Event::DragEnd,
            Event::Increment { step: 0.1 },
            Event::Decrement { step: 0.1 },
            Event::SetToMin,
            Event::SetToMax,
            Event::Blur,
        ] {
            drop(svc.send(ev));
        }

        let api = svc.connect(&|_| {});

        for p in [
            Part::Root,
            Part::Label,
            Part::Track,
            Part::Thumb,
            Part::Output,
            Part::HiddenInput,
        ] {
            let _attrs = api.part_attrs(p);
        }

        let _dbg = alloc::format!("{api:?}");

        let _ignored = api.is_dragging();

        // Disabled focus/blur + readonly value guard.
        let mut dis = Service::<Machine>::new(
            Props {
                id: "cs".into(),
                disabled: true,
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(dis.send(Event::Focus { is_keyboard: false }));
        drop(dis.send(Event::Blur));

        let mut ro = Service::<Machine>::new(
            Props {
                id: "cs".into(),
                readonly: true,
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(ro.send(Event::DragStart { position: 0.5 }));
        drop(ro.send(Event::Increment { step: 5.0 }));
        drop(ro.send(Event::SetToMax));

        assert_eq!(ro.state(), &State::Idle);

        // Track-pointer-down dispatch + RGB-channel gradient branch.
        let cap = core::cell::RefCell::new(Vec::new());
        let send = |event: Event| cap.borrow_mut().push(event);

        svc.connect(&send).on_track_pointer_down(0.3);

        assert!(matches!(cap.borrow()[0], Event::DragStart { .. }));

        let red = Service::<Machine>::new(
            Props {
                id: "cs".into(),
                channel: ColorChannel::Red,
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        let _track = red.connect(&|_| {}).track_attrs();
    }

    #[test]
    fn connect_and_guards_cover_both_arms() {
        // Disabled (idle) + keyboard focus.
        let mut disabled = service(Props {
            disabled: true,
            ..Props::default()
        });
        drop(disabled.send(Event::Focus { is_keyboard: true }));
        let disabled_api = disabled.connect(&|_| {});
        for part in [
            Part::Root,
            Part::Label,
            Part::Track,
            Part::Thumb,
            Part::Output,
            Part::HiddenInput,
        ] {
            let _attrs = disabled_api.part_attrs(part);
        }

        // Read-only: every value-changing event is guarded out.
        let mut readonly = service(Props {
            readonly: true,
            ..Props::default()
        });
        for event in [
            Event::DragStart { position: 0.5 },
            Event::Increment { step: 5.0 },
            Event::Decrement { step: 5.0 },
            Event::SetToMin,
            Event::SetToMax,
        ] {
            drop(readonly.send(event));
        }
        let _readonly_root = readonly.connect(&|_| {}).root_attrs();

        // Active drag with keyboard focus.
        let mut active = service(Props::default());
        drop(active.send(Event::Focus { is_keyboard: true }));
        drop(active.send(Event::DragStart { position: 0.5 }));
        let active_api = active.connect(&|_| {});
        let _active_root = active_api.root_attrs();
        let _active_thumb = active_api.thumb_attrs();

        // Idle, no flags.
        let idle = service(Props::default());
        let idle_api = idle.connect(&|_| {});
        let _idle_root = idle_api.root_attrs();
        let _idle_thumb = idle_api.thumb_attrs();
    }
}
