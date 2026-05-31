//! `ColorArea` component state machine and connect API.
//!
//! `ColorArea` is a 2D color picker: the thumb position maps to two color
//! channels (x and y). It owns the channel math, value state, keyboard
//! behavior, and ARIA/data attributes. Live area measurement, pointer capture,
//! and coordinate-to-value conversion are adapter concerns: the adapter
//! supplies already-normalized `(x, y)` in `0..=1` via [`Api::on_background_pointer_down`]
//! (drag start) and drives [`Event::DragMove`] / [`Event::DragEnd`] from its own
//! pointer listeners, exactly as the slider does.

use alloc::{format, string::String, vec::Vec};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ColorChannel, ColorValue, ComponentIds,
    ComponentMessages, ComponentPart, ConnectApi, CssProperty, Direction, Env, HtmlAttr,
    KeyboardKey, Locale, MessageFn, PendingEffect, TransitionPlan, channel_range, channel_value,
    no_cleanup, with_channel,
};
use ars_interactions::KeyboardEventData;

/// Label for the area thumb.
type LabelFn = dyn Fn(&Locale) -> String + Send + Sync;

/// Role description for the area thumb.
type RoleDescriptionFn = dyn Fn(&Locale) -> String + Send + Sync;

/// Formats the `aria-valuetext`. Arguments: `x_axis_reading` (channel-aware,
/// e.g. `"saturation 80%"` or `"hue 180°"`), `y_axis_reading`, `color_name`,
/// `locale`. Readings are preformatted per channel so non-fractional channels
/// (hue/RGB) are not mis-rendered as percentages.
type ValueTextFn = dyn Fn(&str, &str, &str, &Locale) -> String + Send + Sync;

/// Consumer callback fired on drag-end / pointer release.
type ChangeEndFn = dyn Fn(ColorValue) + Send + Sync;

/// The states for the `ColorArea` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// No interaction in progress.
    Idle,

    /// The user is dragging the thumb.
    Dragging,
}

/// The events for the `ColorArea` component.
#[derive(Clone, Copy, Debug)]
pub enum Event {
    /// The user started dragging (normalized `x`/`y` in `0..=1` relative to the area).
    DragStart {
        /// Normalized x coordinate (`0..=1`).
        x: f64,

        /// Normalized y coordinate (`0..=1`).
        y: f64,
    },

    /// The user is moving while dragging.
    DragMove {
        /// Normalized x coordinate (`0..=1`).
        x: f64,

        /// Normalized y coordinate (`0..=1`).
        y: f64,
    },

    /// The user released the drag.
    DragEnd,

    /// Increment `x_channel` by `step`.
    IncrementX {
        /// The step amount.
        step: f64,
    },

    /// Decrement `x_channel` by `step`.
    DecrementX {
        /// The step amount.
        step: f64,
    },

    /// Increment `y_channel` by `step`.
    IncrementY {
        /// The step amount.
        step: f64,
    },

    /// Decrement `y_channel` by `step`.
    DecrementY {
        /// The step amount.
        step: f64,
    },

    /// Snap `x_channel` to its minimum.
    SetXToMin,

    /// Snap `x_channel` to its maximum.
    SetXToMax,

    /// Snap `y_channel` to its minimum.
    SetYToMin,

    /// Snap `y_channel` to its maximum.
    SetYToMax,

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

/// Typed identifier for side effects emitted by the `ColorArea` machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Invoke `Props::on_change_end`.
    ChangeEnd,
}

/// The context for the `ColorArea` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The current color value (controlled or uncontrolled).
    pub value: Bindable<ColorValue>,

    /// Which channel the x-axis controls.
    pub x_channel: ColorChannel,

    /// Which channel the y-axis controls.
    pub y_channel: ColorChannel,

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

    /// Large step size (Shift+Arrow).
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

/// The props for the `ColorArea` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled value. When `Some`, the component is controlled.
    pub value: Option<ColorValue>,

    /// Default value for uncontrolled mode.
    pub default_value: ColorValue,

    /// Which channel the x-axis controls.
    pub x_channel: ColorChannel,

    /// Which channel the y-axis controls.
    pub y_channel: ColorChannel,

    /// Step size for keyboard adjustment.
    pub step: f64,

    /// Large step size for Shift+Arrow.
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
            x_channel: ColorChannel::Saturation,
            y_channel: ColorChannel::Lightness,
            step: 0.01,
            large_step: 0.1,
            disabled: false,
            readonly: false,
            dir: Direction::Ltr,
            name: None,
            on_change_end: None,
        }
    }
}

/// The messages for the `ColorArea` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Label for the area thumb. Default: `"Color area"`.
    pub label: MessageFn<LabelFn>,

    /// Role description for screen readers. Default: `"2d color picker"`.
    pub role_description: MessageFn<RoleDescriptionFn>,

    /// Formats both channel values for `aria-valuetext`.
    pub value_text: MessageFn<ValueTextFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            label: MessageFn::static_str("Color area"),
            role_description: MessageFn::static_str("2d color picker"),
            value_text: MessageFn::new(
                |x_reading: &str, y_reading: &str, color_name: &str, _locale: &Locale| {
                    format!("{color_name}, {x_reading}, {y_reading}")
                },
            ),
        }
    }
}

impl ComponentMessages for Messages {}

/// Format a single channel reading for `aria-valuetext`, including the channel
/// name and a channel-appropriate unit (degrees for hue, raw for the 8-bit RGB
/// channels, a percentage for the fractional channels).
fn format_axis_reading(channel: ColorChannel, value: f64) -> String {
    let name = format!("{channel:?}").to_lowercase();

    match channel {
        ColorChannel::Hue => format!("{name} {value:.0}°"),
        ColorChannel::Red | ColorChannel::Green | ColorChannel::Blue => {
            format!("{name} {value:.0}")
        }
        _ => format!("{name} {:.0}%", value * 100.0),
    }
}

/// Apply normalized `(x, y)` coordinates to both channels (y is inverted: top = max).
fn apply_area_position(ctx: &mut Context, x: f64, y: f64) {
    let color = *ctx.value.get();

    let (x_min, x_max) = channel_range(ctx.x_channel);
    let (y_min, y_max) = channel_range(ctx.y_channel);

    let x_val = x_min + x.clamp(0.0, 1.0) * (x_max - x_min);
    let y_val = y_max - y.clamp(0.0, 1.0) * (y_max - y_min);

    let updated = with_channel(&color, ctx.x_channel, x_val);

    ctx.value.set(with_channel(&updated, ctx.y_channel, y_val));
}

/// Build the change-end effect that invokes `Props::on_change_end`.
///
/// Reports the *pending* value staged during the drag rather than the
/// controlled `get()` value, which in controlled mode still holds the stale
/// pre-drag color until the parent syncs the new value back through its prop.
fn change_end_effect() -> PendingEffect<Machine> {
    PendingEffect::new(Effect::ChangeEnd, |ctx: &Context, props: &Props, _send| {
        if let Some(callback) = &props.on_change_end {
            callback(*ctx.value.pending());
        }

        no_cleanup()
    })
}

/// The machine for the `ColorArea` component.
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
            x_channel: props.x_channel,
            y_channel: props.y_channel,
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
        // A disabled area ignores value-changing input but still tracks focus
        // and accepts parent-driven prop syncs (so it can be re-enabled).
        // `DragEnd` is allowed through so a drag in flight when the parent
        // disabled the control can still terminate cleanly.
        if ctx.disabled {
            match event {
                Event::DragStart { .. }
                | Event::DragMove { .. }
                | Event::IncrementX { .. }
                | Event::DecrementX { .. }
                | Event::IncrementY { .. }
                | Event::DecrementY { .. }
                | Event::SetXToMin
                | Event::SetXToMax
                | Event::SetYToMin
                | Event::SetYToMax => return None,
                _ => {}
            }
        }

        match (state, event) {
            (State::Idle, Event::DragStart { x, y }) => {
                if ctx.readonly {
                    return None;
                }

                let (x, y) = (*x, *y);
                Some(
                    TransitionPlan::to(State::Dragging).apply(move |ctx: &mut Context| {
                        apply_area_position(ctx, x, y);
                    }),
                )
            }

            (State::Dragging, Event::DragMove { x, y }) => {
                let (x, y) = (*x, *y);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    apply_area_position(ctx, x, y);
                }))
            }

            (State::Dragging, Event::DragEnd) => {
                Some(TransitionPlan::to(State::Idle).with_effect(change_end_effect()))
            }

            (_, Event::IncrementX { step }) => {
                if ctx.readonly {
                    return None;
                }

                let step = *step;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let color = *ctx.value.get();

                    let current = channel_value(&color, ctx.x_channel);

                    let (_, max) = channel_range(ctx.x_channel);

                    ctx.value.set(with_channel(
                        &color,
                        ctx.x_channel,
                        (current + step).min(max),
                    ));
                }))
            }

            (_, Event::DecrementX { step }) => {
                if ctx.readonly {
                    return None;
                }

                let step = *step;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let color = *ctx.value.get();
                    let current = channel_value(&color, ctx.x_channel);
                    let (min, _) = channel_range(ctx.x_channel);

                    ctx.value.set(with_channel(
                        &color,
                        ctx.x_channel,
                        (current - step).max(min),
                    ));
                }))
            }

            (_, Event::IncrementY { step }) => {
                if ctx.readonly {
                    return None;
                }

                let step = *step;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let color = *ctx.value.get();
                    let current = channel_value(&color, ctx.y_channel);
                    let (_, max) = channel_range(ctx.y_channel);

                    ctx.value.set(with_channel(
                        &color,
                        ctx.y_channel,
                        (current + step).min(max),
                    ));
                }))
            }

            (_, Event::DecrementY { step }) => {
                if ctx.readonly {
                    return None;
                }

                let step = *step;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let color = *ctx.value.get();
                    let current = channel_value(&color, ctx.y_channel);
                    let (min, _) = channel_range(ctx.y_channel);

                    ctx.value.set(with_channel(
                        &color,
                        ctx.y_channel,
                        (current - step).max(min),
                    ));
                }))
            }

            (_, Event::SetXToMin) => {
                if ctx.readonly {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    let color = *ctx.value.get();
                    let (min, _) = channel_range(ctx.x_channel);

                    ctx.value.set(with_channel(&color, ctx.x_channel, min));
                }))
            }

            (_, Event::SetXToMax) => {
                if ctx.readonly {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    let color = *ctx.value.get();
                    let (_, max) = channel_range(ctx.x_channel);

                    ctx.value.set(with_channel(&color, ctx.x_channel, max));
                }))
            }

            (_, Event::SetYToMin) => {
                if ctx.readonly {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    let color = *ctx.value.get();
                    let (min, _) = channel_range(ctx.y_channel);

                    ctx.value.set(with_channel(&color, ctx.y_channel, min));
                }))
            }

            (_, Event::SetYToMax) => {
                if ctx.readonly {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    let color = *ctx.value.get();
                    let (_, max) = channel_range(ctx.y_channel);

                    ctx.value.set(with_channel(&color, ctx.y_channel, max));
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
                        }
                        None => ctx.value.sync_controlled(None),
                    },
                ))
            }

            (_, Event::SetProps) => {
                let props = props.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.x_channel = props.x_channel;
                    ctx.y_channel = props.y_channel;
                    ctx.step = props.step;
                    ctx.large_step = props.large_step;
                    ctx.disabled = props.disabled;
                    ctx.readonly = props.readonly;
                    ctx.dir = props.dir;
                }))
            }

            _ => None,
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        assert_eq!(
            old.id, new.id,
            "color_area::Props.id must remain stable after init"
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
/// rather than cached in the context.
fn props_output_changed(old: &Props, new: &Props) -> bool {
    old.x_channel != new.x_channel
        || old.y_channel != new.y_channel
        || (old.step - new.step).abs() > f64::EPSILON
        || (old.large_step - new.large_step).abs() > f64::EPSILON
        || old.disabled != new.disabled
        || old.readonly != new.readonly
        || old.dir != new.dir
}

/// Structural parts exposed by the `ColorArea` connect API.
#[derive(ComponentPart)]
#[scope = "color-area"]
pub enum Part {
    /// Container with `role="group"`.
    Root,

    /// 2D gradient background.
    Background,

    /// Draggable thumb with `role="application"`.
    Thumb,

    /// `type="hidden"` input that submits the hex value for forms.
    HiddenInput,
}

/// The connect API for the `ColorArea` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("color_area::Api")
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

    /// Attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "group");

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

    /// Attributes for the gradient background element.
    #[must_use]
    pub fn background_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Background.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        let color = self.ctx.value.get();

        attrs.set_style(
            CssProperty::Custom("ars-color-area-bg"),
            format!("hsl({:.0}, 100%, 50%)", color.hue),
        );

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
            .set(HtmlAttr::Role, "application")
            .set(
                HtmlAttr::Aria(AriaAttr::RoleDescription),
                (self.ctx.messages.role_description)(&self.ctx.locale),
            )
            // A disabled control must stay out of the tab order.
            .set(
                HtmlAttr::TabIndex,
                if self.ctx.disabled { "-1" } else { "0" },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.label)(&self.ctx.locale),
            );

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        let color = self.ctx.value.get();

        let x_val = channel_value(color, self.ctx.x_channel);
        let y_val = channel_value(color, self.ctx.y_channel);

        let (x_min, x_max) = channel_range(self.ctx.x_channel);
        let (y_min, y_max) = channel_range(self.ctx.y_channel);

        let x_reading = format_axis_reading(self.ctx.x_channel, x_val);
        let y_reading = format_axis_reading(self.ctx.y_channel, y_val);
        let color_name = color.color_name_en();

        attrs.set(
            HtmlAttr::Aria(AriaAttr::ValueText),
            (self.ctx.messages.value_text)(&x_reading, &y_reading, &color_name, &self.ctx.locale),
        );

        let x_pct = if (x_max - x_min).abs() > f64::EPSILON {
            (x_val - x_min) / (x_max - x_min) * 100.0
        } else {
            0.0
        };

        let y_pct = if (y_max - y_min).abs() > f64::EPSILON {
            (1.0 - (y_val - y_min) / (y_max - y_min)) * 100.0
        } else {
            0.0
        };

        attrs
            .set_style(
                CssProperty::Custom("ars-color-area-thumb-x"),
                format!("{x_pct:.1}%"),
            )
            .set_style(
                CssProperty::Custom("ars-color-area-thumb-y"),
                format!("{y_pct:.1}%"),
            )
            .set_style(CssProperty::BackgroundColor, color.to_css_hsl())
            .set(
                HtmlAttr::Aria(AriaAttr::KeyShortcuts),
                "ArrowUp ArrowDown ArrowLeft ArrowRight",
            );

        if self.is_dragging() {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

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

        // A disabled control must be omitted from form submission.
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Handles a keydown on the thumb. `shift` selects the large step.
    ///
    /// In RTL, the x-axis arrow keys are mirrored so `ArrowLeft` increments and
    /// `ArrowRight` decrements; the y-axis and Home/End/PageUp/PageDown are
    /// direction-independent.
    pub fn on_thumb_keydown(&self, data: &KeyboardEventData, shift: bool) {
        let step = if shift {
            self.ctx.large_step
        } else {
            self.ctx.step
        };

        let rtl = self.ctx.dir == Direction::Rtl;

        match data.key {
            KeyboardKey::ArrowRight => (self.send)(if rtl {
                Event::DecrementX { step }
            } else {
                Event::IncrementX { step }
            }),

            KeyboardKey::ArrowLeft => (self.send)(if rtl {
                Event::IncrementX { step }
            } else {
                Event::DecrementX { step }
            }),

            KeyboardKey::ArrowUp => (self.send)(Event::IncrementY { step }),

            KeyboardKey::ArrowDown => (self.send)(Event::DecrementY { step }),

            KeyboardKey::Home => (self.send)(Event::SetXToMin),

            KeyboardKey::End => (self.send)(Event::SetXToMax),

            KeyboardKey::PageUp => (self.send)(Event::SetYToMax),

            KeyboardKey::PageDown => (self.send)(Event::SetYToMin),

            _ => {}
        }
    }

    /// Dispatches a drag-start from an adapter-resolved normalized `(x, y)`.
    pub fn on_background_pointer_down(&self, x: f64, y: f64) {
        (self.send)(Event::DragStart { x, y });
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Background => self.background_attrs(),
            Part::Thumb => self.thumb_attrs(),
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
            props.id = "color-area".to_string();
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
    fn thumb_position_maps_to_two_channels() {
        let mut svc = service(Props {
            default_value: ColorValue::from_hsl(200.0, 0.5, 0.5),
            ..Props::default()
        });

        // Top-right corner: x=1 (saturation max), y=0 (lightness max).
        drop(svc.send(Event::DragStart { x: 1.0, y: 0.0 }));

        let value = *svc.connect(&|_| {}).value();

        assert!((value.saturation - 1.0).abs() < 1e-9);
        assert!((value.lightness - 1.0).abs() < 1e-9);
    }

    #[test]
    fn pointer_drag_updates_both_channels_and_state() {
        let mut svc = service(Props::default());

        drop(svc.send(Event::DragStart { x: 0.0, y: 1.0 }));

        assert_eq!(svc.state(), &State::Dragging);

        drop(svc.send(Event::DragMove { x: 0.25, y: 0.75 }));

        let value = *svc.connect(&|_| {}).value();

        assert!((value.saturation - 0.25).abs() < 1e-9);
        assert!((value.lightness - 0.25).abs() < 1e-9);

        drop(svc.send(Event::DragEnd));

        assert_eq!(svc.state(), &State::Idle);
    }

    #[test]
    fn keyboard_arrows_adjust_channels() {
        let svc = service(Props {
            default_value: ColorValue::from_hsl(0.0, 0.5, 0.5),
            ..Props::default()
        });

        let captured = core::cell::RefCell::new(Vec::new());
        let send = |event: Event| captured.borrow_mut().push(event);

        let api = svc.connect(&send);

        api.on_thumb_keydown(&key(KeyboardKey::ArrowRight), false);
        api.on_thumb_keydown(&key(KeyboardKey::ArrowUp), true);

        let events = captured.borrow();

        assert!(matches!(events[0], Event::IncrementX { step } if (step - 0.01).abs() < 1e-9));
        assert!(matches!(events[1], Event::IncrementY { step } if (step - 0.1).abs() < 1e-9));
    }

    #[test]
    fn rtl_mirrors_x_axis_arrows() {
        let svc = service(Props {
            dir: Direction::Rtl,
            ..Props::default()
        });

        let captured = core::cell::RefCell::new(Vec::new());
        let send = |event: Event| captured.borrow_mut().push(event);

        svc.connect(&send)
            .on_thumb_keydown(&key(KeyboardKey::ArrowRight), false);

        assert!(matches!(captured.borrow()[0], Event::DecrementX { .. }));
    }

    #[test]
    fn increment_clamps_at_channel_max() {
        let mut svc = service(Props {
            default_value: ColorValue::from_hsl(0.0, 1.0, 0.5),
            ..Props::default()
        });

        drop(svc.send(Event::IncrementX { step: 0.5 }));

        assert!((svc.connect(&|_| {}).value().saturation - 1.0).abs() < 1e-9);
    }

    #[test]
    fn change_end_callback_fires_on_drag_end() {
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

        drop(svc.send(Event::DragStart { x: 0.5, y: 0.5 }));

        let mut end = svc.send(Event::DragEnd);

        assert_eq!(end.pending_effects.len(), 1);

        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in end.pending_effects.drain(..) {
            drop(effect.run(svc.context(), svc.props(), Arc::clone(&send)));
        }

        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn disabled_ignores_value_events_but_tracks_focus() {
        let mut svc = service(Props {
            disabled: true,
            default_value: ColorValue::from_hsl(0.0, 0.2, 0.2),
            ..Props::default()
        });

        drop(svc.send(Event::DragStart { x: 1.0, y: 1.0 }));

        assert_eq!(svc.state(), &State::Idle);

        let before = *svc.connect(&|_| {}).value();

        drop(svc.send(Event::IncrementX { step: 0.5 }));

        assert_eq!(*svc.connect(&|_| {}).value(), before);

        drop(svc.send(Event::Focus { is_keyboard: true }));

        assert!(
            svc.connect(&|_| {})
                .thumb_attrs()
                .contains(&HtmlAttr::Data("ars-focus-visible"))
        );
    }

    #[test]
    fn thumb_value_text_includes_perceptual_color_name() {
        let color = ColorValue::from_hsl(120.0, 0.75, 0.4);
        let svc = service(Props {
            default_value: color,
            ..Props::default()
        });

        let value_text = svc
            .connect(&|_| {})
            .thumb_attrs()
            .get(&HtmlAttr::Aria(AriaAttr::ValueText))
            .expect("value text present")
            .to_string();

        let perceptual = color.color_name_en();
        assert!(
            value_text.contains(&perceptual),
            "value text '{value_text}' must include perceptual color name '{perceptual}'"
        );
    }

    #[test]
    fn value_text_is_channel_aware_for_non_fractional_axes() {
        // With a Hue x-axis the reading must be degrees, never a percentage like
        // "18000%". RGB axes render as raw 0-255 values.
        let svc = service(Props {
            x_channel: ColorChannel::Hue,
            y_channel: ColorChannel::Lightness,
            default_value: ColorValue::from_hsl(180.0, 1.0, 0.5),
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
            "hue axis must read in degrees, got '{value_text}'"
        );
        assert!(
            !value_text.contains('%') || value_text.contains("lightness"),
            "hue must not be rendered as a percentage: '{value_text}'"
        );
        assert!(!value_text.contains("18000"));
    }

    #[test]
    fn drag_end_reports_pending_value_for_controlled_area() {
        use alloc::sync::Arc;
        use core::sync::atomic::{AtomicU64, Ordering};

        use ars_core::{StrongSend, callback};

        let reported = Arc::new(AtomicU64::new(u64::MAX));
        let sink = Arc::clone(&reported);
        let mut svc = service(Props {
            // Controlled at saturation 0; a drag must report the new saturation.
            value: Some(ColorValue::from_hsl(0.0, 0.0, 0.5)),
            on_change_end: Some(callback(move |color: ColorValue| {
                sink.store(color.saturation.to_bits(), Ordering::SeqCst);
            })),
            ..Props::default()
        });

        drop(svc.send(Event::DragStart { x: 1.0, y: 0.5 }));
        let mut end = svc.send(Event::DragEnd);

        let send: StrongSend<Event> = Arc::new(|_| {});
        for effect in end.pending_effects.drain(..) {
            drop(effect.run(svc.context(), svc.props(), Arc::clone(&send)));
        }

        let reported_saturation = f64::from_bits(reported.load(Ordering::SeqCst));
        assert!(
            (reported_saturation - 1.0).abs() < 1e-9,
            "on_change_end must report the pending saturation, got {reported_saturation}"
        );
    }

    #[test]
    fn set_props_syncs_controlled_value_and_disabled() {
        let mut svc = service(Props {
            value: Some(ColorValue::from_hsl(0.0, 0.2, 0.2)),
            ..Props::default()
        });

        drop(svc.set_props(Props {
            id: "color-area".to_string(),
            value: Some(ColorValue::from_hsl(0.0, 0.9, 0.8)),
            disabled: true,
            ..Props::default()
        }));

        let api = svc.connect(&|_| {});
        assert!((api.value().saturation - 0.9).abs() < 1e-9);
        assert!(api.root_attrs().contains(&HtmlAttr::Data("ars-disabled")));

        drop(svc.set_props(Props {
            id: "color-area".to_string(),
            value: Some(ColorValue::from_hsl(0.0, 0.9, 0.8)),
            disabled: false,
            ..Props::default()
        }));
        assert!(
            !svc.connect(&|_| {})
                .root_attrs()
                .contains(&HtmlAttr::Data("ars-disabled"))
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
    fn drag_end_terminates_after_mid_drag_disable() {
        let mut svc = service(Props::default());

        drop(svc.send(Event::DragStart { x: 0.5, y: 0.5 }));
        assert_eq!(svc.state(), &State::Dragging);

        drop(svc.set_props(Props {
            id: "color-area".to_string(),
            disabled: true,
            ..Props::default()
        }));

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
    fn disabled_area_omits_hidden_input_from_submission() {
        let svc = service(Props {
            name: Some("swatch".to_string()),
            disabled: true,
            ..Props::default()
        });

        assert_eq!(
            svc.connect(&|_| {})
                .hidden_input_attrs()
                .get(&HtmlAttr::Disabled),
            Some("true")
        );
    }

    #[test]
    fn root_dragging_snapshot() {
        let mut svc = service(Props {
            id: "ca".to_string(),
            ..Props::default()
        });

        drop(svc.send(Event::DragStart { x: 0.5, y: 0.5 }));

        assert_snapshot!(
            "color_area_root_dragging",
            snapshot_attrs(&svc.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn thumb_snapshot() {
        let svc = service(Props {
            id: "ca".to_string(),
            default_value: ColorValue::from_hsl(120.0, 0.75, 0.4),
            ..Props::default()
        });

        assert_snapshot!(
            "color_area_thumb",
            snapshot_attrs(&svc.connect(&|_| {}).thumb_attrs())
        );
    }

    #[test]
    fn hidden_input_snapshot() {
        let svc = service(Props {
            id: "ca".to_string(),
            name: Some("swatch".to_string()),
            default_value: ColorValue::from_hsl(120.0, 0.75, 0.4),
            ..Props::default()
        });

        assert_snapshot!(
            "color_area_hidden_input",
            snapshot_attrs(&svc.connect(&|_| {}).hidden_input_attrs())
        );
    }

    #[test]
    fn exhaustive_events_parts_and_helpers() {
        // Controlled construction + RTL keyboard path.
        let mut svc = Service::<Machine>::new(
            Props {
                id: "ca".into(),
                value: Some(ColorValue::from_hsl(10.0, 0.5, 0.5)),
                dir: Direction::Rtl,
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        for ev in [
            Event::Focus { is_keyboard: true },
            Event::DragStart { x: 0.2, y: 0.3 },
            Event::DragMove { x: 0.4, y: 0.6 },
            Event::DragEnd,
            Event::IncrementX { step: 0.05 },
            Event::DecrementX { step: 0.05 },
            Event::IncrementY { step: 0.05 },
            Event::DecrementY { step: 0.05 },
            Event::SetXToMin,
            Event::SetXToMax,
            Event::SetYToMin,
            Event::SetYToMax,
            Event::Blur,
        ] {
            drop(svc.send(ev));
        }

        let api = svc.connect(&|_| {});

        for p in [Part::Root, Part::Background, Part::Thumb, Part::HiddenInput] {
            let _attrs = api.part_attrs(p);
        }

        let _dbg = alloc::format!("{api:?}");

        // Disabled blur + readonly value-events return without changing state.
        let mut dis = Service::<Machine>::new(
            Props {
                id: "ca".into(),
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
                id: "ca".into(),
                readonly: true,
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(ro.send(Event::DragStart { x: 0.5, y: 0.5 }));
        drop(ro.send(Event::IncrementX { step: 0.1 }));
        drop(ro.send(Event::SetXToMin));

        assert_eq!(ro.state(), &State::Idle);

        // Pointer-down dispatch helper.
        let cap = core::cell::RefCell::new(Vec::new());
        let send = |event: Event| cap.borrow_mut().push(event);

        svc.connect(&send).on_background_pointer_down(0.1, 0.2);

        assert!(matches!(cap.borrow()[0], Event::DragStart { .. }));
    }

    #[test]
    fn connect_and_guards_cover_both_arms() {
        // Disabled (idle) + keyboard focus: root marks disabled, thumb marks focus-visible.
        let mut disabled = service(Props {
            disabled: true,
            ..Props::default()
        });
        drop(disabled.send(Event::Focus { is_keyboard: true }));
        let disabled_api = disabled.connect(&|_| {});
        for part in [Part::Root, Part::Background, Part::Thumb, Part::HiddenInput] {
            let _attrs = disabled_api.part_attrs(part);
        }

        // Read-only: every value-changing event is guarded out.
        let mut readonly = service(Props {
            readonly: true,
            ..Props::default()
        });
        for event in [
            Event::DragStart { x: 0.5, y: 0.5 },
            Event::IncrementX { step: 0.1 },
            Event::DecrementX { step: 0.1 },
            Event::IncrementY { step: 0.1 },
            Event::DecrementY { step: 0.1 },
            Event::SetXToMin,
            Event::SetXToMax,
            Event::SetYToMin,
            Event::SetYToMax,
        ] {
            drop(readonly.send(event));
        }
        let _readonly_root = readonly.connect(&|_| {}).root_attrs();

        // Active drag with keyboard focus: root + thumb mark dragging/focus-visible.
        let mut active = service(Props::default());
        drop(active.send(Event::Focus { is_keyboard: true }));
        drop(active.send(Event::DragStart { x: 0.5, y: 0.5 }));
        let active_api = active.connect(&|_| {});
        let _active_root = active_api.root_attrs();
        let _active_thumb = active_api.thumb_attrs();

        // Idle, no flags: the false arm of every conditional.
        let idle = service(Props::default());
        let idle_api = idle.connect(&|_| {});
        let _idle_root = idle_api.root_attrs();
        let _idle_thumb = idle_api.thumb_attrs();
    }
}
