//! `AngleSlider` component state machine and connect API.
//!
//! `AngleSlider` is a circular angle input over a bare `f64` value in degrees
//! (not a [`ColorValue`](ars_core::ColorValue)). It owns the angle math
//! (snapping, wrapping), value state, keyboard behavior, and ARIA/data
//! attributes. Live track measurement and pointer capture are adapter concerns;
//! the adapter converts pointer coordinates to an angle with [`compute_angle`]
//! and feeds it via [`Api::on_track_pointer_down`] (drag start), then drives
//! [`Event::DragMove`] / [`Event::DragEnd`] from its own pointer listeners.
//! Angular direction is universal, so arrow keys are not mirrored for RTL.

use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::{
    fmt::{self, Debug},
    hash::{Hash, Hasher},
};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, CssProperty, Env, HtmlAttr, KeyboardKey, Locale, MessageFn, PendingEffect,
    TransitionPlan, no_cleanup,
};
// `f64::atan2`/`round`/`rem_euclid` are std-only; use the libm-backed `core_maths`
// versions so the module compiles under `#![no_std]` (matching `ars-core::color`).
use core_maths::CoreFloat;

/// Formats the angle value for `aria-valuetext`.
type ValueTextFn = dyn Fn(f64, &Locale) -> String + Send + Sync;
/// Accessible label for the slider.
type LabelFn = dyn Fn(&Locale) -> String + Send + Sync;
/// Consumer callback fired on drag-end / pointer release.
type ChangeEndFn = dyn Fn(f64) + Send + Sync;

/// Compute an angle from a pointer position relative to the center of the track.
///
/// Returns degrees with `0` at the top (12 o'clock), increasing clockwise. This
/// is a pure helper for adapters to convert pointer coordinates before calling
/// [`Api::on_track_pointer_down`].
#[must_use]
pub fn compute_angle(center: (f64, f64), pointer: (f64, f64)) -> f64 {
    let dx = pointer.0 - center.0;
    let dy = pointer.1 - center.1;

    let degrees = CoreFloat::atan2(dy, dx) * (180.0 / core::f64::consts::PI);

    // Normalize to 0..360, with 0 at the top (12 o'clock).
    CoreFloat::rem_euclid(degrees + 90.0, 360.0)
}

/// Snap an angle to the nearest multiple of `step`.
fn snap_to_step(angle: f64, step: f64) -> f64 {
    if step <= 0.0 || !step.is_finite() {
        return angle;
    }

    CoreFloat::round(angle / step) * step
}

/// Wrap a value into the range `[min, max)`.
fn wrap_value(value: f64, min: f64, max: f64) -> f64 {
    let range = max - min;

    if !range.is_finite() || range <= 0.0 {
        return min;
    }

    CoreFloat::rem_euclid(value - min, range) + min
}

/// Clamp `value` to `[min, max]` without panicking on malformed bounds.
///
/// `min`/`max` are public props with no enforced invariant, so a `min > max`
/// or non-finite bound would make [`f64::clamp`] panic. In that case the value
/// is returned unclamped rather than crashing the component.
fn clamp_to_range(value: f64, min: f64, max: f64) -> f64 {
    if !min.is_finite() || !max.is_finite() || min > max {
        return value;
    }

    value.clamp(min, max)
}

/// The states for the `AngleSlider` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// No interaction in progress and not focused.
    Idle,

    /// The user is dragging the thumb.
    Dragging,

    /// The thumb is focused for keyboard interaction.
    Focused,
}

/// The events for the `AngleSlider` component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Pointer down on the track or thumb. Angle in degrees (0-360).
    DragStart {
        /// The angle in degrees (0-360).
        angle: f64,
    },

    /// Pointer move during drag. Angle in degrees (0-360).
    DragMove {
        /// The angle in degrees (0-360).
        angle: f64,
    },

    /// Pointer released.
    DragEnd,

    /// Increase the value by `step`.
    Increment,

    /// Decrease the value by `step`.
    Decrement,

    /// Set the value to a specific angle.
    SetValue {
        /// The angle in degrees (0-360).
        angle: f64,
    },

    /// Focus received.
    Focus {
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },

    /// Focus lost.
    Blur,

    /// A keyboard key was pressed on the focused thumb.
    KeyDown {
        /// The key that was pressed.
        key: KeyboardKey,
    },

    /// Controlled-value sync from the parent after `Service::set_props`.
    SyncValue(Option<f64>),

    /// Refresh cached output props after `Service::set_props`.
    SetProps,
}

/// Typed identifier for side effects emitted by the `AngleSlider` machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Invoke `Props::on_change_end`.
    ChangeEnd,
}

/// The context for the `AngleSlider` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current angle value (`0.0`-`360.0` by default).
    pub value: Bindable<f64>,

    /// Step size for keyboard increments.
    pub step: f64,

    /// Minimum angle value.
    pub min: f64,

    /// Maximum angle value.
    pub max: f64,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is read-only.
    pub readonly: bool,

    /// Whether the component is focused.
    pub focused: bool,

    /// Whether focus was received via keyboard.
    pub focus_visible: bool,

    /// Locale for internationalized messages.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component instance IDs.
    pub ids: ComponentIds,
}

/// The props for the `AngleSlider` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled value. When `Some`, the component is controlled.
    pub value: Option<f64>,

    /// Default value for uncontrolled mode.
    pub default_value: f64,

    /// Step size for keyboard increments.
    pub step: f64,

    /// Minimum angle value.
    pub min: f64,

    /// Maximum angle value.
    pub max: f64,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is read-only.
    pub readonly: bool,

    /// The name for form submission.
    pub name: Option<String>,

    /// The ID of the form element the component is associated with.
    pub form: Option<String>,

    /// Fired on `Event::DragEnd` / pointer release.
    pub on_change_end: Option<Callback<ChangeEndFn>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: 0.0,
            step: 1.0,
            min: 0.0,
            max: 360.0,
            disabled: false,
            readonly: false,
            name: None,
            form: None,
            on_change_end: None,
        }
    }
}

/// The messages for the `AngleSlider` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Value text formatter. Receives the current angle. Default: `"{v} degrees"`.
    pub value_text: MessageFn<ValueTextFn>,

    /// Accessible label for the slider. Default: `"Angle"`.
    pub label: MessageFn<LabelFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            value_text: MessageFn::new(|degrees: f64, _locale: &Locale| {
                format!("{} degrees", degrees as i32)
            }),
            label: MessageFn::static_str("Angle"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Build the change-end effect that invokes `Props::on_change_end`.
///
/// Reports the *pending* value staged during the drag rather than the
/// controlled `get()` value, which in controlled mode still holds the stale
/// pre-drag angle until the parent syncs the new value back through its prop.
fn change_end_effect() -> PendingEffect<Machine> {
    PendingEffect::new(Effect::ChangeEnd, |ctx: &Context, props: &Props, _send| {
        if let Some(callback) = &props.on_change_end {
            callback(*ctx.value.pending());
        }

        no_cleanup()
    })
}

/// The machine for the `AngleSlider` component.
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
        let value = if let Some(v) = props.value {
            Bindable::controlled(v)
        } else {
            Bindable::uncontrolled(props.default_value)
        };

        let context = Context {
            value,
            step: props.step,
            min: props.min,
            max: props.max,
            disabled: props.disabled,
            readonly: props.readonly,
            focused: false,
            focus_visible: false,
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
        // Parent-driven prop syncs always apply, even when disabled/readonly,
        // so the control can be re-enabled and its controlled value updated.
        match event {
            Event::SyncValue(value) => {
                let value = *value;
                return Some(TransitionPlan::context_only(
                    move |ctx: &mut Context| match value {
                        Some(angle) => {
                            ctx.value.set(angle);
                            ctx.value.sync_controlled(Some(angle));
                        }
                        None => ctx.value.sync_controlled(None),
                    },
                ));
            }

            Event::SetProps => {
                let props = props.clone();
                return Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.step = props.step;
                    ctx.min = props.min;
                    ctx.max = props.max;
                    ctx.disabled = props.disabled;
                    ctx.readonly = props.readonly;

                    // Re-clamp the current value to the new bounds so an
                    // out-of-range angle is not exposed via aria-valuenow / the
                    // hidden input until the next interaction.
                    let clamped = clamp_to_range(*ctx.value.get(), ctx.min, ctx.max);
                    ctx.value.set(clamped);
                    if ctx.value.is_controlled() {
                        ctx.value.sync_controlled(Some(clamped));
                    }
                }));
            }

            _ => {}
        }

        // Disabled and read-only both block value-changing input. Focus/Blur
        // (handled in the main match) and `DragEnd` still pass through so a drag
        // in flight when the control was disabled can terminate cleanly.
        if ctx.disabled || ctx.readonly {
            match event {
                Event::DragStart { .. }
                | Event::DragMove { .. }
                | Event::Increment
                | Event::Decrement
                | Event::SetValue { .. }
                | Event::KeyDown { .. } => return None,
                _ => {}
            }
        }

        match (state, event) {
            (State::Idle | State::Focused, Event::DragStart { angle }) => {
                let snapped = clamp_to_range(snap_to_step(*angle, ctx.step), ctx.min, ctx.max);
                Some(
                    TransitionPlan::to(State::Dragging).apply(move |ctx: &mut Context| {
                        ctx.value.set(snapped);
                    }),
                )
            }

            (State::Dragging, Event::DragMove { angle }) => {
                let snapped = clamp_to_range(snap_to_step(*angle, ctx.step), ctx.min, ctx.max);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.value.set(snapped);
                }))
            }

            (State::Dragging, Event::DragEnd) => {
                let next_state = if ctx.focused {
                    State::Focused
                } else {
                    State::Idle
                };

                Some(TransitionPlan::to(next_state).with_effect(change_end_effect()))
            }

            (_, Event::Focus { is_keyboard }) => {
                let ik = *is_keyboard;
                Some(
                    TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
                        ctx.focused = true;
                        ctx.focus_visible = ik;
                    }),
                )
            }

            (_, Event::Blur) => Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                ctx.focused = false;
                ctx.focus_visible = false;
            })),

            (_, Event::Increment) => {
                // Accumulate from the pending value so repeated controlled steps
                // before a parent `SyncValue` advance instead of recomputing from
                // the stale prop.
                let new_val = wrap_value(ctx.value.pending() + ctx.step, ctx.min, ctx.max);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.value.set(new_val);
                }))
            }

            (_, Event::Decrement) => {
                let new_val = wrap_value(ctx.value.pending() - ctx.step, ctx.min, ctx.max);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.value.set(new_val);
                }))
            }

            (_, Event::SetValue { angle }) => {
                let clamped = clamp_to_range(*angle, ctx.min, ctx.max);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.value.set(clamped);
                }))
            }

            (State::Focused, Event::KeyDown { key }) => {
                let large_step = ctx.step * 10.0;

                // Accumulate relative steps from the pending value (see Increment).
                let current = *ctx.value.pending();

                let new_val = match key {
                    KeyboardKey::ArrowRight | KeyboardKey::ArrowUp => {
                        wrap_value(current + ctx.step, ctx.min, ctx.max)
                    }

                    KeyboardKey::ArrowLeft | KeyboardKey::ArrowDown => {
                        wrap_value(current - ctx.step, ctx.min, ctx.max)
                    }

                    KeyboardKey::Home => ctx.min,

                    KeyboardKey::End => ctx.max,

                    KeyboardKey::PageUp => wrap_value(current + large_step, ctx.min, ctx.max),

                    KeyboardKey::PageDown => wrap_value(current - large_step, ctx.min, ctx.max),

                    _ => return None,
                };

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.value.set(new_val);
                }))
            }

            _ => None,
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        assert_eq!(
            old.id, new.id,
            "angle_slider::Props.id must remain stable after init"
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
/// `name`/`form` are omitted: they are read live from `Props` in
/// `hidden_input_attrs` rather than cached in the context.
fn props_output_changed(old: &Props, new: &Props) -> bool {
    (old.step - new.step).abs() > f64::EPSILON
        || (old.min - new.min).abs() > f64::EPSILON
        || (old.max - new.max).abs() > f64::EPSILON
        || old.disabled != new.disabled
        || old.readonly != new.readonly
}

/// Structural parts exposed by the `AngleSlider` connect API.
///
/// `Marker { value }` is parameterized by an `f64` angle, so this enum cannot
/// derive `Eq`/`Hash`; the manual impls below compare and hash that field via
/// [`f64::to_bits`], matching the `Slider` convention.
#[derive(Clone, Debug)]
pub enum Part {
    /// Container with `role="group"` and `data-ars-state`.
    Root,

    /// Positioning wrapper for the circular control.
    Control,

    /// Circular track background.
    Track,

    /// Filled arc indicating the current angle.
    Range,

    /// Draggable thumb with `role="slider"`.
    Thumb,

    /// Live value-text output.
    ValueText,

    /// Presentational group holding the markers.
    MarkerGroup,

    /// A marker at a specific angle.
    Marker {
        /// The marker angle in degrees.
        value: f64,
    },

    /// `type="hidden"` input for form submission.
    HiddenInput,
}

impl PartialEq for Part {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Marker { value: left }, Self::Marker { value: right }) => {
                left.to_bits() == right.to_bits()
            }

            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

impl Eq for Part {}

impl Hash for Part {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);

        if let Self::Marker { value } = self {
            value.to_bits().hash(state);
        }
    }
}

impl ComponentPart for Part {
    const ROOT: Self = Self::Root;

    fn scope() -> &'static str {
        "angle-slider"
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Root => "root",
            Self::Control => "control",
            Self::Track => "track",
            Self::Range => "range",
            Self::Thumb => "thumb",
            Self::ValueText => "value-text",
            Self::MarkerGroup => "marker-group",
            Self::Marker { .. } => "marker",
            Self::HiddenInput => "hidden-input",
        }
    }

    fn all() -> Vec<Self> {
        vec![
            Self::Root,
            Self::Control,
            Self::Track,
            Self::Range,
            Self::Thumb,
            Self::ValueText,
            Self::MarkerGroup,
            Self::Marker { value: 0.0 },
            Self::HiddenInput,
        ]
    }
}

/// The connect API for the `AngleSlider` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("angle_slider::Api")
            .field("state", &self.state)
            .field("ctx", &self.ctx)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// The current angle value (the controlled prop, when controlled).
    #[must_use]
    pub fn value(&self) -> f64 {
        *self.ctx.value.get()
    }

    /// The angle to render (ARIA + rotation).
    ///
    /// Uses the *pending* value so a controlled slider visibly and accessibly
    /// moves during a drag / keyboard adjustment, before the parent round-trips
    /// the new value back through `SyncValue`. In uncontrolled mode this equals
    /// [`value`](Self::value).
    #[must_use]
    const fn display_value(&self) -> f64 {
        *self.ctx.value.pending()
    }

    /// Sets the angle value programmatically.
    pub fn set_value(&self, angle: f64) {
        (self.send)(Event::SetValue { angle });
    }

    /// Whether the slider is currently being dragged.
    #[must_use]
    pub const fn is_dragging(&self) -> bool {
        matches!(self.state, State::Dragging)
    }

    /// Whether the slider is focused.
    #[must_use]
    pub const fn is_focused(&self) -> bool {
        self.ctx.focused
    }

    /// Formatted value text (e.g., `"45 degrees"`).
    #[must_use]
    pub fn formatted_value(&self) -> String {
        (self.ctx.messages.value_text)(self.display_value(), &self.ctx.locale)
    }

    const fn state_str(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::Dragging => "dragging",
            State::Focused => "focused",
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
            .set(HtmlAttr::Id, self.ctx.ids.id().to_string())
            .set(HtmlAttr::Role, "group")
            .set(HtmlAttr::Data("ars-state"), self.state_str());

        if self.ctx.disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        if self.is_dragging() {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }

        attrs
    }

    /// Attributes for the control wrapper element.
    #[must_use]
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Attributes for the circular track element.
    #[must_use]
    pub fn track_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Track.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Attributes for the filled-arc range element.
    #[must_use]
    pub fn range_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Range.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

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
            .set(HtmlAttr::Role, "slider")
            // A disabled control must stay out of the tab order.
            .set(
                HtmlAttr::TabIndex,
                if self.ctx.disabled { "-1" } else { "0" },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::ValueNow),
                self.display_value().to_string(),
            )
            .set(HtmlAttr::Aria(AriaAttr::ValueMin), self.ctx.min.to_string())
            .set(HtmlAttr::Aria(AriaAttr::ValueMax), self.ctx.max.to_string())
            .set(HtmlAttr::Aria(AriaAttr::ValueText), self.formatted_value())
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.label)(&self.ctx.locale),
            )
            .set_style(
                CssProperty::Custom("ars-angle-value"),
                self.display_value().to_string(),
            )
            .set_style(
                CssProperty::Custom("ars-angle-thumb-rotation"),
                format!("{}deg", self.display_value()),
            );

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        if self.is_dragging() {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }

        attrs
    }

    /// Attributes for the live value-text element.
    #[must_use]
    pub fn value_text_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ValueText.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Live), "off");

        attrs
    }

    /// Attributes for the presentational marker group.
    #[must_use]
    pub fn marker_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::MarkerGroup.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "presentation");

        attrs
    }

    /// Attributes for a marker at the given angle.
    #[must_use]
    pub fn marker_attrs(&self, value: f64) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Marker { value }.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set_style(
                CssProperty::Custom("ars-angle-marker-rotation"),
                format!("{value}deg"),
            );

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

        // Pending value so the submitted angle matches the in-progress drag in
        // controlled mode (the thumb already renders the pending value).
        attrs.set(HtmlAttr::Value, self.ctx.value.pending().to_string());

        if let Some(form) = &self.props.form {
            attrs.set(HtmlAttr::Form, form.clone());
        }

        // A disabled control must be omitted from form submission.
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
            .set(HtmlAttr::TabIndex, "-1")
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Dispatches a keydown on the focused thumb.
    pub fn on_thumb_keydown(&self, key: KeyboardKey) {
        (self.send)(Event::KeyDown { key });
    }

    /// Dispatches a drag-start from an adapter-resolved angle (see [`compute_angle`]).
    pub fn on_track_pointer_down(&self, angle: f64) {
        (self.send)(Event::DragStart { angle });
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Control => self.control_attrs(),
            Part::Track => self.track_attrs(),
            Part::Range => self.range_attrs(),
            Part::Thumb => self.thumb_attrs(),
            Part::ValueText => self.value_text_attrs(),
            Part::MarkerGroup => self.marker_group_attrs(),
            Part::Marker { value } => self.marker_attrs(value),
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
            props.id = "angle-slider".to_string();
        }

        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn compute_angle_zero_at_top_clockwise() {
        // Pointer directly above center -> 0 degrees.
        assert!((compute_angle((0.0, 0.0), (0.0, -1.0)) - 0.0).abs() < 1e-9);
        // Pointer to the right -> 90 degrees.
        assert!((compute_angle((0.0, 0.0), (1.0, 0.0)) - 90.0).abs() < 1e-9);
        // Pointer below -> 180 degrees.
        assert!((compute_angle((0.0, 0.0), (0.0, 1.0)) - 180.0).abs() < 1e-9);
    }

    #[test]
    fn drag_start_snaps_and_enters_dragging() {
        let mut svc = service(Props {
            step: 15.0,
            ..Props::default()
        });

        drop(svc.send(Event::DragStart { angle: 47.0 }));

        assert_eq!(svc.state(), &State::Dragging);
        // 47 snaps to nearest 15 -> 45.
        assert!((svc.connect(&|_| {}).value() - 45.0).abs() < 1e-9);
    }

    #[test]
    fn keyboard_step_requires_focus_and_wraps() {
        let mut svc = service(Props {
            step: 10.0,
            default_value: 5.0,
            ..Props::default()
        });

        // KeyDown without focus is ignored.
        drop(svc.send(Event::KeyDown {
            key: KeyboardKey::ArrowDown,
        }));

        assert!((svc.connect(&|_| {}).value() - 5.0).abs() < 1e-9);

        // Focus, then ArrowDown wraps 5 - 10 -> 355.
        drop(svc.send(Event::Focus { is_keyboard: true }));
        drop(svc.send(Event::KeyDown {
            key: KeyboardKey::ArrowDown,
        }));

        assert!((svc.connect(&|_| {}).value() - 355.0).abs() < 1e-9);
    }

    #[test]
    fn home_end_snap_to_min_max() {
        let mut svc = service(Props {
            min: 0.0,
            max: 270.0,
            default_value: 100.0,
            ..Props::default()
        });

        drop(svc.send(Event::Focus { is_keyboard: true }));
        drop(svc.send(Event::KeyDown {
            key: KeyboardKey::End,
        }));

        assert!((svc.connect(&|_| {}).value() - 270.0).abs() < 1e-9);

        drop(svc.send(Event::KeyDown {
            key: KeyboardKey::Home,
        }));

        assert!((svc.connect(&|_| {}).value() - 0.0).abs() < 1e-9);
    }

    #[test]
    fn custom_min_max_clamps_set_value() {
        let mut svc = service(Props {
            min: 30.0,
            max: 150.0,
            ..Props::default()
        });

        drop(svc.send(Event::SetValue { angle: 200.0 }));

        assert!((svc.connect(&|_| {}).value() - 150.0).abs() < 1e-9);

        drop(svc.send(Event::SetValue { angle: 10.0 }));

        assert!((svc.connect(&|_| {}).value() - 30.0).abs() < 1e-9);
    }

    #[test]
    fn thumb_exposes_slider_role_and_aria_values() {
        let svc = service(Props {
            default_value: 45.0,
            ..Props::default()
        });

        let thumb = svc.connect(&|_| {}).thumb_attrs();

        assert_eq!(thumb.get(&HtmlAttr::Role), Some("slider"));
        assert_eq!(thumb.get(&HtmlAttr::Aria(AriaAttr::ValueNow)), Some("45"));
        assert_eq!(thumb.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("0"));
        assert_eq!(thumb.get(&HtmlAttr::Aria(AriaAttr::ValueMax)), Some("360"));
        assert_eq!(
            thumb.get(&HtmlAttr::Aria(AriaAttr::ValueText)),
            Some("45 degrees")
        );
        assert!(!thumb.contains(&HtmlAttr::Aria(AriaAttr::Orientation)));
    }

    #[test]
    fn root_exposes_state_data_attr() {
        let mut svc = service(Props::default());

        assert_eq!(
            svc.connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Data("ars-state")),
            Some("idle")
        );

        drop(svc.send(Event::Focus { is_keyboard: false }));

        assert_eq!(
            svc.connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Data("ars-state")),
            Some("focused")
        );
    }

    #[test]
    fn change_end_effect_fires_on_drag_end() {
        use alloc::sync::Arc;
        use core::sync::atomic::{AtomicU32, Ordering};

        use ars_core::{StrongSend, callback};

        let last = Arc::new(AtomicU32::new(0));
        let sink = Arc::clone(&last);
        let mut svc = service(Props {
            step: 1.0,
            on_change_end: Some(callback(move |degrees: f64| {
                sink.store(degrees as u32, Ordering::SeqCst);
            })),
            ..Props::default()
        });

        drop(svc.send(Event::DragStart { angle: 90.0 }));

        let mut end = svc.send(Event::DragEnd);
        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in end.pending_effects.drain(..) {
            drop(effect.run(svc.context(), svc.props(), Arc::clone(&send)));
        }

        assert_eq!(last.load(Ordering::SeqCst), 90);
    }

    #[test]
    fn drag_end_reports_pending_value_for_controlled_slider() {
        use alloc::sync::Arc;
        use core::sync::atomic::{AtomicU32, Ordering};

        use ars_core::{StrongSend, callback};

        let last = Arc::new(AtomicU32::new(u32::MAX));
        let sink = Arc::clone(&last);
        let mut svc = service(Props {
            value: Some(0.0),
            step: 1.0,
            on_change_end: Some(callback(move |degrees: f64| {
                sink.store(degrees as u32, Ordering::SeqCst);
            })),
            ..Props::default()
        });

        drop(svc.send(Event::DragStart { angle: 120.0 }));
        let mut end = svc.send(Event::DragEnd);

        let send: StrongSend<Event> = Arc::new(|_| {});
        for effect in end.pending_effects.drain(..) {
            drop(effect.run(svc.context(), svc.props(), Arc::clone(&send)));
        }

        // The dragged angle (120) must reach the callback, not the stale
        // controlled value (0) that `get()` still returns.
        assert_eq!(last.load(Ordering::SeqCst), 120);
    }

    #[test]
    fn set_props_syncs_controlled_value_and_disabled() {
        let mut svc = service(Props {
            value: Some(30.0),
            ..Props::default()
        });

        drop(svc.set_props(Props {
            id: "angle-slider".to_string(),
            value: Some(200.0),
            disabled: true,
            ..Props::default()
        }));

        let api = svc.connect(&|_| {});
        assert!((api.value() - 200.0).abs() < 1e-9);
        assert_eq!(
            api.root_attrs().get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true")
        );

        drop(svc.set_props(Props {
            id: "angle-slider".to_string(),
            value: Some(200.0),
            disabled: false,
            ..Props::default()
        }));
        assert!(
            !svc.connect(&|_| {})
                .root_attrs()
                .contains(&HtmlAttr::Aria(AriaAttr::Disabled))
        );
    }

    #[test]
    fn set_props_clamps_value_to_new_bounds() {
        let mut svc = service(Props {
            default_value: 300.0,
            ..Props::default()
        });
        assert!((svc.connect(&|_| {}).value() - 300.0).abs() < 1e-9);

        // Shrink the range below the current value without changing `value`.
        drop(svc.set_props(Props {
            id: "angle-slider".to_string(),
            default_value: 300.0,
            min: 0.0,
            max: 180.0,
            ..Props::default()
        }));

        assert!(
            (svc.connect(&|_| {}).value() - 180.0).abs() < 1e-9,
            "value must clamp into the new bounds"
        );
        assert_eq!(
            svc.connect(&|_| {})
                .thumb_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("180")
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
    fn malformed_bounds_do_not_panic() {
        // min > max must not panic f64::clamp via SetValue / drag.
        let mut inverted = service(Props {
            min: 300.0,
            max: 30.0,
            default_value: 100.0,
            ..Props::default()
        });
        drop(inverted.send(Event::SetValue { angle: 150.0 }));
        drop(inverted.send(Event::DragStart { angle: 200.0 }));
        // Reaching here without panicking is the assertion; value stays finite.
        assert!(inverted.connect(&|_| {}).value().is_finite());

        // A NaN bound likewise must not panic.
        let mut nan_bound = service(Props {
            max: f64::NAN,
            default_value: 45.0,
            ..Props::default()
        });
        drop(nan_bound.send(Event::SetValue { angle: 90.0 }));
        assert!(nan_bound.connect(&|_| {}).value().is_finite());
    }

    #[test]
    fn controlled_keyboard_steps_accumulate_from_pending() {
        // Controlled slider at 0°; two Increments before a parent sync must
        // accumulate (10° -> 20°), not recompute from the stale prop each time.
        let mut svc = service(Props {
            value: Some(0.0),
            step: 10.0,
            ..Props::default()
        });

        drop(svc.send(Event::Increment));
        drop(svc.send(Event::Increment));

        // The pending value (rendered + submitted) is 20°.
        assert_eq!(
            svc.connect(&|_| {})
                .thumb_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("20")
        );
    }

    #[test]
    fn controlled_interaction_renders_pending_angle() {
        // Controlled slider at 0°; a drag stages 90° internally. The thumb must
        // render the pending angle even though value() still returns the prop.
        let mut svc = service(Props {
            value: Some(0.0),
            ..Props::default()
        });

        drop(svc.send(Event::DragStart { angle: 90.0 }));

        let api = svc.connect(&|_| {});
        assert_eq!(
            api.thumb_attrs().get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("90"),
            "thumb must render the pending angle during a controlled drag"
        );
        // The public value() still reflects the controlled prop.
        assert!((api.value() - 0.0).abs() < 1e-9);
    }

    #[test]
    fn drag_end_terminates_after_mid_drag_disable() {
        let mut svc = service(Props::default());

        drop(svc.send(Event::DragStart { angle: 90.0 }));
        assert_eq!(svc.state(), &State::Dragging);

        drop(svc.set_props(Props {
            id: "angle-slider".to_string(),
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
    fn disabled_slider_omits_hidden_input_from_submission() {
        let svc = service(Props {
            name: Some("angle".to_string()),
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
    fn hidden_input_submits_value_with_name_and_form() {
        let svc = service(Props {
            name: Some("angle".to_string()),
            form: Some("form-1".to_string()),
            default_value: 120.0,
            ..Props::default()
        });

        let hidden = svc.connect(&|_| {}).hidden_input_attrs();

        assert_eq!(hidden.get(&HtmlAttr::Type), Some("hidden"));
        assert_eq!(hidden.get(&HtmlAttr::Name), Some("angle"));
        assert_eq!(hidden.get(&HtmlAttr::Form), Some("form-1"));
        assert_eq!(hidden.get(&HtmlAttr::Value), Some("120"));
        assert_eq!(hidden.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
    }

    #[test]
    fn marker_attrs_carry_rotation() {
        let svc = service(Props::default());
        let marker = svc.connect(&|_| {}).marker_attrs(90.0);

        assert!(marker.styles().contains(&(
            CssProperty::Custom("ars-angle-marker-rotation"),
            "90deg".to_string()
        )));
    }

    #[test]
    fn disabled_blocks_value_but_focuses() {
        let mut svc = service(Props {
            disabled: true,
            default_value: 10.0,
            ..Props::default()
        });

        drop(svc.send(Event::Increment));

        assert!((svc.connect(&|_| {}).value() - 10.0).abs() < 1e-9);

        drop(svc.send(Event::Focus { is_keyboard: true }));

        assert_eq!(svc.state(), &State::Focused);
        assert_eq!(
            svc.connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true")
        );
    }

    #[test]
    fn thumb_snapshot() {
        let svc = service(Props {
            id: "as".to_string(),
            default_value: 135.0,
            ..Props::default()
        });

        assert_snapshot!(
            "angle_slider_thumb",
            snapshot_attrs(&svc.connect(&|_| {}).thumb_attrs())
        );
    }

    #[test]
    fn root_idle_snapshot() {
        let svc = service(Props {
            id: "as".to_string(),
            ..Props::default()
        });

        assert_snapshot!(
            "angle_slider_root_idle",
            snapshot_attrs(&svc.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn root_dragging_snapshot() {
        let mut svc = service(Props {
            id: "as".to_string(),
            ..Props::default()
        });

        drop(svc.send(Event::DragStart { angle: 90.0 }));

        assert_snapshot!(
            "angle_slider_root_dragging",
            snapshot_attrs(&svc.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn root_focused_snapshot() {
        let mut svc = service(Props {
            id: "as".to_string(),
            ..Props::default()
        });

        drop(svc.send(Event::Focus { is_keyboard: true }));

        assert_snapshot!(
            "angle_slider_root_focused",
            snapshot_attrs(&svc.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn exhaustive_events_parts_and_helpers() {
        let mut svc = Service::<Machine>::new(
            Props {
                id: "as".into(),
                value: Some(45.0),
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(svc.send(Event::Focus { is_keyboard: true }));

        for ev in [
            Event::DragStart { angle: 30.0 },
            Event::DragMove { angle: 60.0 },
            Event::DragEnd,
            Event::Increment,
            Event::Decrement,
            Event::SetValue { angle: 90.0 },
            Event::KeyDown {
                key: KeyboardKey::PageUp,
            },
            Event::KeyDown {
                key: KeyboardKey::PageDown,
            },
            Event::KeyDown {
                key: KeyboardKey::Tab,
            }, // unhandled key -> None
            Event::Blur,
        ] {
            drop(svc.send(ev));
        }

        // KeyDown while idle (not focused) is ignored by the (Focused, KeyDown) arm.
        drop(svc.send(Event::KeyDown {
            key: KeyboardKey::ArrowUp,
        }));

        let api = svc.connect(&|_| {});

        for p in [
            Part::Root,
            Part::Control,
            Part::Track,
            Part::Range,
            Part::Thumb,
            Part::ValueText,
            Part::MarkerGroup,
            Part::Marker { value: 90.0 },
            Part::HiddenInput,
        ] {
            let _attrs = api.part_attrs(p);
        }

        let _dbg = alloc::format!("{api:?}");
        let _focused = api.is_focused();

        // Disabled + readonly only allow focus/blur.
        let mut dis = Service::<Machine>::new(
            Props {
                id: "as".into(),
                disabled: true,
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(dis.send(Event::Focus { is_keyboard: false }));
        drop(dis.send(Event::Blur));
        drop(dis.send(Event::Increment));

        let mut ro = Service::<Machine>::new(
            Props {
                id: "as".into(),
                readonly: true,
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(ro.send(Event::Focus { is_keyboard: true }));
        drop(ro.send(Event::DragStart { angle: 10.0 }));
        drop(ro.send(Event::Blur));

        // Dispatch helpers.
        let cap = core::cell::RefCell::new(Vec::new());
        let send = |event: Event| cap.borrow_mut().push(event);

        let dapi = svc.connect(&send);

        dapi.on_thumb_keydown(KeyboardKey::ArrowUp);
        dapi.on_track_pointer_down(120.0);
        dapi.set_value(200.0);

        let events = cap.borrow();

        assert!(matches!(events[0], Event::KeyDown { .. }));
        assert!(matches!(events[1], Event::DragStart { .. }));
        assert!(matches!(events[2], Event::SetValue { .. }));
    }

    #[test]
    fn connect_and_guards_cover_both_arms() {
        let all_parts = [
            Part::Root,
            Part::Control,
            Part::Track,
            Part::Range,
            Part::Thumb,
            Part::ValueText,
            Part::MarkerGroup,
            Part::Marker { value: 45.0 },
            Part::HiddenInput,
        ];

        // Disabled + keyboard focus: aria-disabled true arms, focus-visible true arm.
        let mut disabled = service(Props {
            disabled: true,
            ..Props::default()
        });
        drop(disabled.send(Event::Focus { is_keyboard: true }));
        drop(disabled.send(Event::Increment)); // guarded out
        let disabled_api = disabled.connect(&|_| {});
        for part in &all_parts {
            let _attrs = disabled_api.part_attrs(part.clone());
        }

        // Read-only + focus: aria-readonly true arm; drag guarded out.
        let mut readonly = service(Props {
            readonly: true,
            ..Props::default()
        });
        drop(readonly.send(Event::Focus { is_keyboard: true }));
        drop(readonly.send(Event::DragStart { angle: 10.0 }));
        let readonly_api = readonly.connect(&|_| {});
        let _readonly_root = readonly_api.root_attrs();
        let _readonly_thumb = readonly_api.thumb_attrs();

        // Dragging (state=dragging, focus-visible false).
        let mut dragging = service(Props::default());
        drop(dragging.send(Event::DragStart { angle: 90.0 }));
        let dragging_api = dragging.connect(&|_| {});
        let _dragging_root = dragging_api.root_attrs();
        let _dragging_thumb = dragging_api.thumb_attrs();

        // Idle, no flags: every false arm + state=idle.
        let idle = service(Props::default());
        let idle_api = idle.connect(&|_| {});
        for part in &all_parts {
            let _attrs = idle_api.part_attrs(part.clone());
        }
    }

    #[test]
    fn degenerate_step_and_range_hit_defensive_guards() {
        // `step <= 0.0` short-circuits `snap_to_step` (returns the angle as-is).
        let mut zero_step = service(Props {
            step: 0.0,
            default_value: 40.0,
            ..Props::default()
        });
        drop(zero_step.send(Event::DragStart { angle: 47.0 }));
        assert!((zero_step.connect(&|_| {}).value() - 47.0).abs() < 1e-9);

        // `min == max` makes the range zero, so `wrap_value` returns `min`.
        let mut zero_range = service(Props {
            min: 90.0,
            max: 90.0,
            default_value: 90.0,
            ..Props::default()
        });
        drop(zero_range.send(Event::Focus { is_keyboard: true }));
        drop(zero_range.send(Event::Increment));
        assert!((zero_range.connect(&|_| {}).value() - 90.0).abs() < 1e-9);
    }
}
