//! `ColorSlider` component state machine and connect API.
//!
//! `ColorSlider` is a 1D slider that edits a single [`ColorChannel`]. It owns
//! the channel math, value state, orientation, keyboard behavior, and ARIA/data
//! attributes. Live track measurement, pointer capture, and position-to-value
//! conversion are adapter concerns: the adapter supplies an already-normalized
//! position in `0..=1` via [`Api::on_track_pointer_down`] (drag start) and
//! drives [`Event::DragMove`] / [`Event::DragEnd`] from its own pointer
//! listeners, exactly as the slider does.

use alloc::{format, string::String};
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

/// Formats the channel value for `aria-valuetext`.
type ValueTextFn = dyn Fn(f64, &Locale) -> String + Send + Sync;

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
            value_text: MessageFn::new(|val: f64, _locale: &Locale| format!("{val:.0}")),
        }
    }
}

impl ComponentMessages for Messages {}

/// Apply a normalized position (`0..=1`) to the channel value.
fn apply_slider_position(ctx: &mut Context, position: f64) {
    let color = *ctx.value.get();

    let (min, max) = channel_range(ctx.channel);

    let value = min + position.clamp(0.0, 1.0) * (max - min);

    ctx.value.set(with_channel(&color, ctx.channel, value));
}

/// Build the change-end effect that invokes `Props::on_change_end`.
fn change_end_effect() -> PendingEffect<Machine> {
    PendingEffect::new(Effect::ChangeEnd, |ctx: &Context, props: &Props, _send| {
        if let Some(callback) = &props.on_change_end {
            callback(*ctx.value.get());
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

        let context = Context {
            value,
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
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled {
            return match event {
                Event::Focus { is_keyboard } => {
                    let kb = *is_keyboard;
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.focused = true;
                        ctx.focus_visible = kb;
                    }))
                }

                Event::Blur => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                })),

                _ => None,
            };
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
                    let color = *ctx.value.get();
                    let current = channel_value(&color, ctx.channel);
                    let (_, max) = channel_range(ctx.channel);

                    ctx.value
                        .set(with_channel(&color, ctx.channel, (current + step).min(max)));
                }))
            }

            (_, Event::Decrement { step }) => {
                if ctx.readonly {
                    return None;
                }

                let step = *step;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let color = *ctx.value.get();
                    let current = channel_value(&color, ctx.channel);
                    let (min, _) = channel_range(ctx.channel);

                    ctx.value
                        .set(with_channel(&color, ctx.channel, (current - step).max(min)));
                }))
            }

            (_, Event::SetToMin) => {
                if ctx.readonly {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    let color = *ctx.value.get();
                    let (min, _) = channel_range(ctx.channel);
                    ctx.value.set(with_channel(&color, ctx.channel, min));
                }))
            }

            (_, Event::SetToMax) => {
                if ctx.readonly {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    let color = *ctx.value.get();
                    let (_, max) = channel_range(ctx.channel);
                    ctx.value.set(with_channel(&color, ctx.channel, max));
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
        let color = self.ctx.value.get();

        let val = channel_value(color, self.ctx.channel);

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

        let color = self.ctx.value.get();

        let gradient = match self.ctx.channel {
            ColorChannel::Hue => "linear-gradient(to right, \
                hsl(0,100%,50%), hsl(60,100%,50%), hsl(120,100%,50%), \
                hsl(180,100%,50%), hsl(240,100%,50%), hsl(300,100%,50%), \
                hsl(360,100%,50%))"
                .to_string(),

            ColorChannel::Alpha => format!(
                "linear-gradient(to right, transparent, {})",
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
            .set(HtmlAttr::TabIndex, "0");

        let color = self.ctx.value.get();

        let val = channel_value(color, self.ctx.channel);

        let (min, max) = channel_range(self.ctx.channel);

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
                (self.ctx.messages.value_text)(val, &self.ctx.locale),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );

        let pct = if (max - min).abs() > f64::EPSILON {
            (val - min) / (max - min) * 100.0
        } else {
            0.0
        };

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

        attrs.set(HtmlAttr::Value, self.ctx.value.get().to_hex(true));

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
