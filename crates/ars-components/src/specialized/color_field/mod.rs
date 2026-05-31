//! `ColorField` component state machine and connect API.
//!
//! `ColorField` is a text input for color values. In whole-color mode it parses
//! and formats complete color strings (`#rrggbb`, `rgb(...)`, `hsl(...)`,
//! `hsb(...)`); in channel mode it edits a single [`ColorChannel`] as an ARIA
//! spinbutton with keyboard stepping. All color parsing, formatting, and
//! validation is delegated to the shared color helpers in
//! [`ars_core::color`]; the component owns only the value/edit state, the
//! ARIA/data attributes, and IME-composition suppression.

use alloc::{format, string::String, vec::Vec};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, Bindable, ColorChannel, ColorFormat, ColorValue, ComponentIds,
    ComponentMessages, ComponentPart, ConnectApi, Env, HtmlAttr, KeyboardKey, Locale, MessageFn,
    NoEffect, TransitionPlan, channel_range, channel_step_default, channel_value,
    format_color_string, parse_color_string, with_channel,
};
use ars_interactions::KeyboardEventData;

/// Labels a single channel input (e.g. `"Hue"`).
type ChannelLabelFn = dyn Fn(ColorChannel, &Locale) -> String + Send + Sync;

/// Formats a channel value for `aria-valuetext`.
type ChannelValueTextFn = dyn Fn(ColorChannel, f64, &Locale) -> String + Send + Sync;

/// Returns the whole-color-mode `aria-label`.
type ColorLabelFn = dyn Fn(&Locale) -> String + Send + Sync;

/// Returns the parse-failure message text.
type InvalidMessageFn = dyn Fn(&Locale) -> String + Send + Sync;

/// The states for the `ColorField` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Input is not focused.
    Idle,

    /// Input is focused; the user may be editing text.
    Focused,
}

/// The events for the `ColorField` component.
#[derive(Clone, Debug)]
pub enum Event {
    /// Input received focus.
    Focus {
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },

    /// Input lost focus — triggers a commit.
    Blur,

    /// Raw text changed (keystroke or paste). No parsing until commit.
    Change(String),

    /// Enter key — parse and commit without leaving `Focused`.
    Commit,

    /// Programmatic value update from the parent.
    SetValue(ColorValue),

    /// Programmatic invalid-state update.
    SetInvalid(bool),
    /// Adapter signal that a description part is (or is no longer) rendered,
    /// toggling whether the input's `aria-describedby` references it.
    SetHasDescription(bool),

    /// Channel mode: increment by `step` (`ArrowUp`).
    Increment,

    /// Channel mode: decrement by `step` (`ArrowDown`).
    Decrement,

    /// Channel mode: increment by `large_step` (`PageUp`).
    IncrementLarge,

    /// Channel mode: decrement by `large_step` (`PageDown`).
    DecrementLarge,

    /// Channel mode: snap to max (End).
    IncrementToMax,

    /// Channel mode: snap to min (Home).
    DecrementToMin,

    /// IME composition started.
    CompositionStart,

    /// IME composition ended.
    CompositionEnd,

    /// Controlled-value sync from the parent after `Service::set_props`.
    SyncValue(Option<ColorValue>),

    /// Refresh cached output props after `Service::set_props`.
    SetProps,
}

/// The context for the `ColorField` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The current color value (controlled or uncontrolled). `None` when empty.
    pub value: Bindable<Option<ColorValue>>,

    /// Raw text in the input. Diverges from `value` while editing.
    pub input_text: String,

    /// If `Some`, the field edits a single channel (numeric spinbutton);
    /// if `None`, the field accepts whole color strings.
    pub channel: Option<ColorChannel>,

    /// Display format for formatting `value` → text. Default: `Hex`.
    pub color_format: ColorFormat,

    /// Step size for channel-mode keyboard adjustment.
    pub step: f64,

    /// Large step size for channel-mode `PageUp` / `PageDown`.
    pub large_step: f64,

    /// Whether the input is focused.
    pub focused: bool,

    /// Whether focus was via keyboard (for the focus-visible ring).
    pub focus_visible: bool,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is read-only.
    pub readonly: bool,

    /// Whether the value is invalid per *external* validation (the `invalid`
    /// prop / `SetInvalid`). Kept separate from [`parse_error`](Self::parse_error)
    /// so a prop refresh cannot clear a parser-derived error.
    pub invalid: bool,

    /// Whether the last commit failed to parse (or an empty required field was
    /// committed). Parser-derived, owned by the machine — `SetProps` must not
    /// clear it. The effective invalid state is `invalid || parse_error`.
    pub parse_error: bool,

    /// Whether a value is required.
    pub required: bool,

    /// Whether IME composition is in progress.
    pub is_composing: bool,

    /// Whether a description part is rendered.
    pub has_description: bool,

    /// Form submission name.
    pub name: Option<String>,

    /// Resolved locale for message formatting.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component instance IDs.
    pub ids: ComponentIds,
}

/// The props for the `ColorField` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled value. When `Some`, the component is controlled.
    pub value: Option<ColorValue>,

    /// Default value for uncontrolled mode.
    pub default_value: Option<ColorValue>,

    /// If `Some`, the field edits a single channel (numeric spinbutton);
    /// if `None`, the field accepts whole color strings.
    pub channel: Option<ColorChannel>,

    /// Display format for whole-color mode. Default: `Hex`.
    pub color_format: ColorFormat,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is read-only.
    pub readonly: bool,

    /// Whether the value is invalid (external validation).
    pub invalid: bool,

    /// Whether a value is required.
    pub required: bool,

    /// Form submission name.
    pub name: Option<String>,

    /// Step size for channel-mode keyboard adjustment.
    /// Default: `channel_step_default(ch)` when a channel is set.
    pub step: Option<f64>,

    /// Large step size for channel-mode `PageUp` / `PageDown`. Default: `step * 10`.
    pub large_step: Option<f64>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            channel: None,
            color_format: ColorFormat::Hex,
            disabled: false,
            readonly: false,
            invalid: false,
            required: false,
            name: None,
            step: None,
            large_step: None,
        }
    }
}

/// The messages for the `ColorField` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Label for a channel input. Default: channel name (e.g., `"Hue"`).
    pub channel_label: MessageFn<ChannelLabelFn>,

    /// Formatted channel value for `aria-valuetext`.
    pub channel_value_text: MessageFn<ChannelValueTextFn>,

    /// Label for whole-color mode. Default: `"Color value"`.
    pub color_label: MessageFn<ColorLabelFn>,

    /// Message shown when parsing fails. Default: `"Invalid color value"`.
    pub invalid_message: MessageFn<InvalidMessageFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            channel_label: MessageFn::new(|ch: ColorChannel, _locale: &Locale| format!("{ch:?}")),
            channel_value_text: MessageFn::new(|ch: ColorChannel, val: f64, _locale: &Locale| {
                match ch {
                    ColorChannel::Hue => format!("{val:.0}°"),
                    ColorChannel::Red | ColorChannel::Green | ColorChannel::Blue => {
                        format!("{val:.0}")
                    }
                    _ => format!("{:.0}%", val * 100.0),
                }
            }),
            color_label: MessageFn::static_str("Color value"),
            invalid_message: MessageFn::static_str("Invalid color value"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Whether a channel is presented to the user as a `0..=100` percentage of its
/// underlying `0..=1` fractional range.
///
/// Hue (degrees) and the 8-bit RGB channels are shown and entered as their raw
/// numeric value; saturation, lightness, brightness, and alpha are shown and
/// entered as percentages. Display formatting and commit parsing must agree on
/// this scaling, otherwise typed percentages clamp directly into the fractional
/// range (e.g. `50` -> `50.clamp(0, 1) == 1.0`).
const fn channel_is_percentage(channel: ColorChannel) -> bool {
    !matches!(
        channel,
        ColorChannel::Hue | ColorChannel::Red | ColorChannel::Green | ColorChannel::Blue
    )
}

/// Format a color value for display in the input.
fn format_value(
    color: &ColorValue,
    channel: Option<ColorChannel>,
    color_format: ColorFormat,
) -> String {
    if let Some(ch) = channel {
        let val = channel_value(color, ch);

        if channel_is_percentage(ch) {
            format!("{:.0}", val * 100.0)
        } else {
            format!("{val:.0}")
        }
    } else {
        format_color_string(color, color_format)
    }
}

/// Parse `input_text` and update `value`; reset `input_text` to the formatted
/// value. Sets the parser-derived `parse_error` flag when parsing fails.
fn commit_input(ctx: &mut Context) {
    if let Some(ch) = ctx.channel {
        // Channel mode: parse as f64. Reject non-finite input (`NaN`/`inf`),
        // which `f64::parse` accepts but must not flow into a `ColorValue`.
        if let Some(raw) = ctx
            .input_text
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
        {
            let (min, max) = channel_range(ch);

            // Percentage channels are entered as 0-100 but stored as 0..=1.
            let scaled = if channel_is_percentage(ch) {
                raw / 100.0
            } else {
                raw
            };

            let clamped = scaled.clamp(min, max);

            if let Some(color) = ctx.value.pending() {
                let new_color = with_channel(color, ch, clamped);

                ctx.value.set(Some(new_color));
                ctx.input_text = format_value(&new_color, ctx.channel, ctx.color_format);
                ctx.parse_error = false;
            }
        } else {
            ctx.parse_error = true;
        }
    } else {
        // Whole-color mode: parse via parse_color_string.
        if ctx.input_text.trim().is_empty() {
            ctx.value.set(None);
            ctx.parse_error = ctx.required;

            return;
        }

        if let Some(color) = parse_color_string(&ctx.input_text) {
            ctx.value.set(Some(color));
            ctx.input_text = format_color_string(&color, ctx.color_format);
            ctx.parse_error = false;
        } else {
            ctx.parse_error = true;
        }
    }
}

/// Adjust the channel value by `delta` (positive or negative), clamped to range.
///
/// Reads the *pending* color so repeated controlled steps accumulate (each press
/// builds on the last staged value, not the stale controlled prop).
fn adjust_channel(ctx: &mut Context, delta: f64) {
    if let (Some(ch), Some(color)) = (ctx.channel, *ctx.value.pending()) {
        let color = &color;
        let current = channel_value(color, ch);

        let (min, max) = channel_range(ch);

        let new_val = (current + delta).clamp(min, max);

        let new_color = with_channel(color, ch, new_val);

        ctx.value.set(Some(new_color));
        ctx.input_text = format_value(&new_color, ctx.channel, ctx.color_format);
        ctx.parse_error = false;
    }
}

/// Snap the channel to `min` or `max` and refresh the input text.
fn snap_channel(ctx: &mut Context, to_max: bool) {
    if let (Some(ch), Some(color)) = (ctx.channel, *ctx.value.pending()) {
        let color = &color;
        let (min, max) = channel_range(ch);

        let target = if to_max { max } else { min };

        let new_color = with_channel(color, ch, target);

        ctx.value.set(Some(new_color));
        ctx.input_text = format_value(&new_color, ctx.channel, ctx.color_format);
        ctx.parse_error = false;
    }
}

/// The machine for the `ColorField` component.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = NoEffect;
    type Api<'a> = Api<'a>;

    fn init(
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        let value = if let Some(v) = &props.value {
            Bindable::controlled(Some(*v))
        } else {
            // A channel field is an ARIA spinbutton: it needs a base color to
            // expose a valid value and to accept keyboard steps / commits.
            // Seed an initial color in channel mode when none was supplied.
            let seed = props
                .default_value
                .or_else(|| props.channel.map(|_| ColorValue::default()));

            Bindable::uncontrolled(seed)
        };

        let step = props
            .step
            .unwrap_or_else(|| props.channel.map_or(1.0, channel_step_default));

        let large_step = props.large_step.unwrap_or(step * 10.0);

        let input_text = if let Some(c) = value.get() {
            format_value(c, props.channel, props.color_format)
        } else {
            String::new()
        };

        let context = Context {
            value,
            input_text,
            channel: props.channel,
            color_format: props.color_format,
            step,
            large_step,
            focused: false,
            focus_visible: false,
            disabled: props.disabled,
            readonly: props.readonly,
            invalid: props.invalid,
            parse_error: false,
            required: props.required,
            is_composing: false,
            has_description: false,
            name: props.name.clone(),
            locale: env.locale.clone(),
            messages: messages.clone(),
            ids: ComponentIds::from_id(&props.id),
        };

        (State::Idle, context)
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // During IME composition, suppress all keyboard shortcuts. Parent-driven
        // prop syncs (`SyncValue`/`SetProps`) fall through to the main match so a
        // controlled field stays in step with its props even mid-composition.
        if ctx.is_composing {
            match event {
                Event::CompositionEnd => {
                    return Some(TransitionPlan::context_only(|ctx: &mut Context| {
                        ctx.is_composing = false;
                    }));
                }

                Event::Change(text) => {
                    // An inert (read-only/disabled) field must not accept edits,
                    // even mid-composition where this branch runs before the
                    // guards below.
                    if ctx.readonly || ctx.disabled {
                        return None;
                    }

                    let next_text = text.clone();
                    return Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.input_text = next_text;
                    }));
                }

                Event::SyncValue(_) | Event::SetProps | Event::SetHasDescription(_) => {}

                _ => return None,
            }
        }

        // A disabled field tracks focus but ignores edits. Prop syncs still apply
        // (so the field can be re-enabled), falling through to the main match.
        if ctx.disabled {
            match event {
                Event::Focus { is_keyboard } => {
                    let kb = *is_keyboard;
                    return Some(TransitionPlan::to(State::Focused).apply(
                        move |ctx: &mut Context| {
                            ctx.focused = true;
                            ctx.focus_visible = kb;
                        },
                    ));
                }

                Event::Blur => {
                    return Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                        ctx.focused = false;
                        ctx.focus_visible = false;
                    }));
                }

                Event::SyncValue(_) | Event::SetProps | Event::SetHasDescription(_) => {}

                _ => return None,
            }
        }

        match event {
            Event::SyncValue(value) => {
                let value = *value;
                Some(TransitionPlan::context_only(
                    move |ctx: &mut Context| match value {
                        Some(color) => {
                            ctx.value.set(Some(color));
                            ctx.value.sync_controlled(Some(Some(color)));

                            if !ctx.focused {
                                ctx.input_text =
                                    format_value(&color, ctx.channel, ctx.color_format);
                            }

                            // A programmatic value is valid: clear both the
                            // external and parser-derived invalid state.
                            ctx.invalid = false;
                            ctx.parse_error = false;
                        }
                        None => ctx.value.sync_controlled(None),
                    },
                ))
            }

            Event::SetProps => {
                let props = props.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    let step = props
                        .step
                        .unwrap_or_else(|| props.channel.map_or(1.0, channel_step_default));

                    ctx.channel = props.channel;
                    ctx.color_format = props.color_format;
                    ctx.step = step;
                    ctx.large_step = props.large_step.unwrap_or(step * 10.0);
                    ctx.disabled = props.disabled;
                    ctx.readonly = props.readonly;
                    ctx.invalid = props.invalid;
                    ctx.required = props.required;
                    ctx.name = props.name.clone();

                    // Switching an empty field into channel mode at runtime needs
                    // the same base-color seed as `init`, so the spinbutton stays
                    // usable and accessible (mirrors the seed in `init`).
                    if ctx.channel.is_some() && ctx.value.pending().is_none() {
                        ctx.value.set(Some(ColorValue::default()));
                    }

                    // Re-render the input in the new channel/format unless the
                    // user is mid-edit. `SyncValue` (which runs before this on a
                    // combined update) formatted with the old representation.
                    if !ctx.focused {
                        ctx.input_text = (*ctx.value.pending())
                            .map(|color| format_value(&color, ctx.channel, ctx.color_format))
                            .unwrap_or_default();
                    }
                }))
            }

            Event::Focus { is_keyboard } => {
                let kb = *is_keyboard;
                Some(
                    TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
                        ctx.focused = true;
                        ctx.focus_visible = kb;
                    }),
                )
            }

            Event::Blur => Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                // A read-only field must never mutate its value on blur.
                if !ctx.readonly {
                    commit_input(ctx);
                }

                ctx.focused = false;
                ctx.focus_visible = false;
            })),

            Event::Change(text) => {
                // Read-only fields ignore edits so input text cannot diverge from
                // the committed value (and cannot be committed later on blur).
                if ctx.readonly {
                    return None;
                }

                let next_text = text.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.input_text = next_text;
                }))
            }

            Event::Commit => {
                if ctx.readonly {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    commit_input(ctx);
                }))
            }

            Event::SetValue(color) => {
                let new_value = *color;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    if !ctx.focused {
                        ctx.input_text = format_value(&new_value, ctx.channel, ctx.color_format);
                    }

                    ctx.value.set(Some(new_value));
                    ctx.invalid = false;
                    ctx.parse_error = false;
                }))
            }

            Event::SetInvalid(inv) => {
                let inv = *inv;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.invalid = inv;
                }))
            }

            Event::SetHasDescription(has_description) => {
                let has_description = *has_description;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.has_description = has_description;
                }))
            }

            Event::Increment => {
                if ctx.readonly || ctx.channel.is_none() {
                    return None;
                }

                let step = ctx.step;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    adjust_channel(ctx, step);
                }))
            }

            Event::Decrement => {
                if ctx.readonly || ctx.channel.is_none() {
                    return None;
                }

                let step = -ctx.step;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    adjust_channel(ctx, step);
                }))
            }

            Event::IncrementLarge => {
                if ctx.readonly || ctx.channel.is_none() {
                    return None;
                }

                let step = ctx.large_step;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    adjust_channel(ctx, step);
                }))
            }

            Event::DecrementLarge => {
                if ctx.readonly || ctx.channel.is_none() {
                    return None;
                }

                let step = -ctx.large_step;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    adjust_channel(ctx, step);
                }))
            }

            Event::IncrementToMax => {
                if ctx.readonly || ctx.channel.is_none() {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    snap_channel(ctx, true);
                }))
            }

            Event::DecrementToMin => {
                if ctx.readonly || ctx.channel.is_none() {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    snap_channel(ctx, false);
                }))
            }

            Event::CompositionStart => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.is_composing = true;
            })),

            Event::CompositionEnd => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.is_composing = false;
            })),
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        assert_eq!(
            old.id, new.id,
            "color_field::Props.id must remain stable after init"
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
        _props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, send }
    }
}

/// Whether any cached output prop changed and the context needs refreshing.
fn props_output_changed(old: &Props, new: &Props) -> bool {
    old.channel != new.channel
        || old.color_format != new.color_format
        || old.disabled != new.disabled
        || old.readonly != new.readonly
        || old.invalid != new.invalid
        || old.required != new.required
        || old.name != new.name
        || option_f64_changed(old.step, new.step)
        || option_f64_changed(old.large_step, new.large_step)
}

/// Compares two optional `f64` step values without a direct float `==`.
fn option_f64_changed(old: Option<f64>, new: Option<f64>) -> bool {
    match (old, new) {
        (Some(old), Some(new)) => (old - new).abs() > f64::EPSILON,
        (None, None) => false,
        _ => true,
    }
}

/// Structural parts exposed by the `ColorField` connect API.
#[derive(ComponentPart)]
#[scope = "color-field"]
pub enum Part {
    /// Container with state/validity data attributes.
    Root,

    /// `<label>` whose `for` points at the input.
    Label,

    /// The text or spinbutton `<input>`.
    Input,

    /// Optional helper text referenced by `aria-describedby`.
    Description,

    /// Error display (`role="alert"`) referenced by `aria-describedby`.
    ErrorMessage,

    /// `type="hidden"` input that submits the hex value for forms.
    HiddenInput,
}

/// The connect API for the `ColorField` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("color_field::Api")
            .field("state", &self.state)
            .field("ctx", &self.ctx)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Whether the component is currently focused.
    #[must_use]
    pub const fn is_focused(&self) -> bool {
        matches!(self.state, State::Focused)
    }

    /// The effective invalid state: external validation (`invalid` prop /
    /// `SetInvalid`) OR a parser-derived error from the last commit.
    #[must_use]
    pub const fn is_invalid(&self) -> bool {
        self.ctx.invalid || self.ctx.parse_error
    }

    /// The current value of the component.
    ///
    /// Reports the *pending* value so a committed edit is reflected consistently
    /// (matching the displayed text, channel ARIA, and the hidden input) even in
    /// controlled mode, where the controlled prop only updates via `SyncValue`.
    #[must_use]
    pub const fn value(&self) -> Option<&ColorValue> {
        self.ctx.value.pending().as_ref()
    }

    /// The current raw input text of the component.
    #[must_use]
    pub fn input_text(&self) -> &str {
        &self.ctx.input_text
    }

    /// The attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        if self.is_invalid() {
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
        }

        if self.ctx.focused {
            attrs.set_bool(HtmlAttr::Data("ars-focused"), true);
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs
    }

    /// The attributes for the label element.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("label"))
            .set(HtmlAttr::For, self.ctx.ids.part("input"));

        attrs
    }

    /// The attributes for the input element.
    #[must_use]
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("input"))
            .set(HtmlAttr::Type, "text")
            .set(HtmlAttr::Value, self.ctx.input_text.clone());

        if let Some(ch) = self.ctx.channel {
            // Channel mode: numeric spinbutton.
            attrs
                .set(HtmlAttr::Role, "spinbutton")
                .set(HtmlAttr::InputMode, "numeric");

            if let Some(color) = self.ctx.value.pending() {
                let val = channel_value(color, ch);
                let (min, max) = channel_range(ch);

                // Percentage channels display/commit as 0-100, so the spinbutton
                // ARIA range must match (the stored channel range is 0..=1).
                let scale = if channel_is_percentage(ch) {
                    100.0
                } else {
                    1.0
                };

                attrs
                    .set(
                        HtmlAttr::Aria(AriaAttr::ValueNow),
                        format!("{:.2}", val * scale),
                    )
                    .set(
                        HtmlAttr::Aria(AriaAttr::ValueMin),
                        format!("{:.2}", min * scale),
                    )
                    .set(
                        HtmlAttr::Aria(AriaAttr::ValueMax),
                        format!("{:.2}", max * scale),
                    )
                    .set(
                        HtmlAttr::Aria(AriaAttr::ValueText),
                        (self.ctx.messages.channel_value_text)(ch, val, &self.ctx.locale),
                    );
            }

            attrs.set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.channel_label)(ch, &self.ctx.locale),
            );
        } else {
            // Whole-color mode: standard text input.
            attrs.set(HtmlAttr::InputMode, "text").set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.color_label)(&self.ctx.locale),
            );
        }

        attrs.set(
            HtmlAttr::Aria(AriaAttr::LabelledBy),
            self.ctx.ids.part("label"),
        );

        if self.is_invalid() {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }

        if self.ctx.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if self.ctx.readonly {
            attrs
                .set_bool(HtmlAttr::ReadOnly, true)
                .set_bool(HtmlAttr::Aria(AriaAttr::ReadOnly), true);
        }

        if self.ctx.disabled {
            attrs
                .set_bool(HtmlAttr::Disabled, true)
                .set_bool(HtmlAttr::Aria(AriaAttr::Disabled), true);
        }

        // describedby: description + error message
        let mut describedby = Vec::new();

        if self.ctx.has_description {
            describedby.push(self.ctx.ids.part("description"));
        }

        if self.is_invalid() {
            describedby.push(self.ctx.ids.part("error-message"));
        }

        if !describedby.is_empty() {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), describedby.join(" "));
        }

        attrs
    }

    /// The attributes for the description element.
    #[must_use]
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("description"));

        attrs
    }

    /// Returns the error message text shown when color parsing fails.
    ///
    /// The adapter renders this inside the `ErrorMessage` part.
    #[must_use]
    pub fn invalid_message(&self) -> String {
        (self.ctx.messages.invalid_message)(&self.ctx.locale)
    }

    /// The attributes for the error message element.
    #[must_use]
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("error-message"))
            .set(HtmlAttr::Role, "alert");

        attrs
    }

    /// The attributes for the hidden input element.
    #[must_use]
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "hidden");

        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name.clone());
        }

        // Only submit a value when the field is valid. While invalid, the stored
        // color is the last *valid* value, which no longer matches the visible
        // input — submitting it would send a stale color.
        if let Some(color) = (*self.ctx.value.pending()).filter(|_| !self.is_invalid()) {
            attrs.set(HtmlAttr::Value, color.to_hex(true));
        }

        // A disabled control is omitted from form submission — and so is an
        // invalid one, so the field round-trips as absent rather than as an
        // empty `name=` carrying the stale last-valid color.
        if self.ctx.disabled || self.is_invalid() {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    // --- Event dispatch helpers ---

    /// Dispatches an input focus event.
    pub fn on_input_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Dispatches an input blur event.
    pub fn on_input_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Dispatches a raw text change.
    pub fn on_input_change(&self, text: String) {
        (self.send)(Event::Change(text));
    }

    /// Handles a keydown on the input element.
    ///
    /// Shortcuts are suppressed while an IME composition is in progress — both
    /// when the machine already knows (`ctx.is_composing`) and when the event
    /// itself reports composing (`data.is_composing`), in case the keydown
    /// arrives before the `CompositionStart` event reaches the machine.
    pub fn on_input_keydown(&self, data: &KeyboardEventData) {
        if self.ctx.is_composing || data.is_composing {
            return;
        }

        let has_channel = self.ctx.channel.is_some();

        match data.key {
            KeyboardKey::Enter => (self.send)(Event::Commit),
            KeyboardKey::ArrowUp if has_channel => (self.send)(Event::Increment),
            KeyboardKey::ArrowDown if has_channel => (self.send)(Event::Decrement),
            KeyboardKey::PageUp if has_channel => (self.send)(Event::IncrementLarge),
            KeyboardKey::PageDown if has_channel => (self.send)(Event::DecrementLarge),
            KeyboardKey::Home if has_channel => (self.send)(Event::DecrementToMin),
            KeyboardKey::End if has_channel => (self.send)(Event::IncrementToMax),
            _ => {}
        }
    }

    /// Dispatches an IME composition-start event.
    pub fn on_composition_start(&self) {
        (self.send)(Event::CompositionStart);
    }

    /// Dispatches an IME composition-end event.
    pub fn on_composition_end(&self) {
        (self.send)(Event::CompositionEnd);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Input => self.input_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use ars_core::{ColorValue, Service};
    use insta::assert_snapshot;

    use super::*;

    fn service(mut props: Props) -> Service<Machine> {
        if props.id.is_empty() {
            props.id = "color-field".to_string();
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
    fn whole_color_input_parses_on_commit() {
        let mut svc = service(Props {
            id: "fg".to_string(),
            ..Props::default()
        });

        drop(svc.send(Event::Change("#3366ff".to_string())));
        drop(svc.send(Event::Commit));

        assert_eq!(
            svc.connect(&|_| {}).value().unwrap().to_hex(false),
            "#3366ff"
        );
    }

    #[test]
    fn parses_hex_rgb_hsl_formats() {
        for input in ["#00ff00", "rgb(0, 255, 0)", "hsl(120, 100%, 50%)"] {
            let mut svc = service(Props::default());

            drop(svc.send(Event::Change(input.to_string())));
            drop(svc.send(Event::Commit));

            let api = svc.connect(&|_| {});

            assert_eq!(api.value().unwrap().to_rgb(), (0, 255, 0), "input {input}");
            assert!(
                !api.input_attrs()
                    .contains(&HtmlAttr::Aria(AriaAttr::Invalid))
            );
        }
    }

    #[test]
    fn invalid_input_sets_invalid_flag_and_aria() {
        let mut svc = service(Props::default());

        drop(svc.send(Event::Change("not a color".to_string())));
        drop(svc.send(Event::Commit));

        let api = svc.connect(&|_| {});

        assert_eq!(
            api.input_attrs().get(&HtmlAttr::Aria(AriaAttr::Invalid)),
            Some("true")
        );
        assert_eq!(api.invalid_message(), "Invalid color value");
    }

    #[test]
    fn empty_required_input_is_invalid() {
        let mut svc = service(Props {
            required: true,
            default_value: Some(ColorValue::from_rgb(0, 0, 0)),
            ..Props::default()
        });

        drop(svc.send(Event::Change(String::new())));
        drop(svc.send(Event::Commit));

        let api = svc.connect(&|_| {});

        assert!(api.value().is_none());
        assert_eq!(
            api.input_attrs().get(&HtmlAttr::Aria(AriaAttr::Invalid)),
            Some("true")
        );
    }

    #[test]
    fn connect_api_whole_color_input_has_text_label() {
        let svc = service(Props::default());

        let api = svc.connect(&|_| {});

        let input = api.input_attrs();

        assert_eq!(input.get(&HtmlAttr::InputMode), Some("text"));
        assert_eq!(
            input.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Color value")
        );
        assert!(!input.contains(&HtmlAttr::Role));
    }

    #[test]
    fn channel_mode_is_spinbutton_with_value_range() {
        let svc = service(Props {
            channel: Some(ColorChannel::Hue),
            default_value: Some(ColorValue::from_hsl(180.0, 1.0, 0.5)),
            ..Props::default()
        });

        let api = svc.connect(&|_| {});

        let input = api.input_attrs();

        assert_eq!(input.get(&HtmlAttr::Role), Some("spinbutton"));
        assert_eq!(input.get(&HtmlAttr::InputMode), Some("numeric"));
        assert_eq!(
            input.get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("180.00")
        );
        assert_eq!(input.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("0.00"));
        assert_eq!(
            input.get(&HtmlAttr::Aria(AriaAttr::ValueMax)),
            Some("360.00")
        );
        assert_eq!(
            input.get(&HtmlAttr::Aria(AriaAttr::ValueText)),
            Some("180°")
        );
        assert_eq!(input.get(&HtmlAttr::Aria(AriaAttr::Label)), Some("Hue"));
    }

    #[test]
    fn percentage_channel_spinbutton_aria_is_scaled() {
        // Saturation displays/commits as 0-100, so the spinbutton ARIA range
        // must be 0..100 (not the stored 0..1) to match the visible value.
        let svc = service(Props {
            channel: Some(ColorChannel::Saturation),
            default_value: Some(ColorValue::from_hsl(0.0, 0.5, 0.5)),
            ..Props::default()
        });

        let input = svc.connect(&|_| {}).input_attrs();
        assert_eq!(
            input.get(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            Some("50.00")
        );
        assert_eq!(input.get(&HtmlAttr::Aria(AriaAttr::ValueMin)), Some("0.00"));
        assert_eq!(
            input.get(&HtmlAttr::Aria(AriaAttr::ValueMax)),
            Some("100.00")
        );

        // Raw channels (hue) keep their native range.
        let hue = service(Props {
            channel: Some(ColorChannel::Hue),
            default_value: Some(ColorValue::from_hsl(180.0, 1.0, 0.5)),
            ..Props::default()
        });
        assert_eq!(
            hue.connect(&|_| {})
                .input_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ValueMax)),
            Some("360.00")
        );
    }

    #[test]
    fn channel_mode_keyboard_steps_value() {
        let mut svc = service(Props {
            channel: Some(ColorChannel::Hue),
            default_value: Some(ColorValue::from_hsl(180.0, 1.0, 0.5)),
            ..Props::default()
        });

        drop(svc.send(Event::Increment));

        assert_eq!(svc.connect(&|_| {}).value().unwrap().hue, 181.0);

        drop(svc.send(Event::DecrementToMin));

        assert_eq!(svc.connect(&|_| {}).value().unwrap().hue, 0.0);

        drop(svc.send(Event::IncrementToMax));

        // Hue wraps: 360 stores as 0.
        assert_eq!(svc.connect(&|_| {}).value().unwrap().hue, 0.0);
    }

    #[test]
    fn on_input_keydown_dispatches_channel_events() {
        let svc = service(Props {
            channel: Some(ColorChannel::Saturation),
            default_value: Some(ColorValue::from_hsl(0.0, 0.5, 0.5)),
            ..Props::default()
        });

        let captured = core::cell::RefCell::new(Vec::new());
        let send = |event: Event| captured.borrow_mut().push(event);

        let api = svc.connect(&send);

        api.on_input_keydown(&key(KeyboardKey::ArrowUp));
        api.on_input_keydown(&key(KeyboardKey::PageDown));
        api.on_input_keydown(&key(KeyboardKey::Enter));

        let events = captured.borrow();

        assert!(matches!(events[0], Event::Increment));
        assert!(matches!(events[1], Event::DecrementLarge));
        assert!(matches!(events[2], Event::Commit));
    }

    #[test]
    fn focus_blur_transitions_and_commits() {
        let mut svc = service(Props::default());

        drop(svc.send(Event::Focus { is_keyboard: true }));

        assert_eq!(svc.state(), &State::Focused);

        drop(svc.send(Event::Change("#ff0000".to_string())));
        drop(svc.send(Event::Blur));

        assert_eq!(svc.state(), &State::Idle);
        assert_eq!(svc.connect(&|_| {}).value().unwrap().to_rgb(), (255, 0, 0));
    }

    #[test]
    fn ime_composition_suppresses_commit() {
        let mut svc = service(Props::default());

        drop(svc.send(Event::CompositionStart));
        // Enter during composition must not commit.
        drop(svc.send(Event::Commit));
        drop(svc.send(Event::Change("#abcdef".to_string())));

        assert!(svc.connect(&|_| {}).value().is_none());

        drop(svc.send(Event::CompositionEnd));
        drop(svc.send(Event::Commit));

        assert_eq!(
            svc.connect(&|_| {}).value().unwrap().to_hex(false),
            "#abcdef"
        );
    }

    #[test]
    fn hidden_input_submits_hex_with_name() {
        let svc = service(Props {
            name: Some("color".to_string()),
            default_value: Some(ColorValue::new(0.0, 1.0, 0.5, 0.5)),
            ..Props::default()
        });

        let hidden = svc.connect(&|_| {}).hidden_input_attrs();

        assert_eq!(hidden.get(&HtmlAttr::Type), Some("hidden"));
        assert_eq!(hidden.get(&HtmlAttr::Name), Some("color"));
        assert_eq!(hidden.get(&HtmlAttr::Value), Some("#ff000080"));
    }

    #[test]
    fn disabled_field_ignores_value_edits_but_tracks_focus() {
        let mut svc = service(Props {
            disabled: true,
            ..Props::default()
        });

        drop(svc.send(Event::Focus { is_keyboard: false }));

        assert_eq!(svc.state(), &State::Focused);

        drop(svc.send(Event::Change("#ffffff".to_string())));
        drop(svc.send(Event::Commit));

        assert!(svc.connect(&|_| {}).value().is_none());
    }

    #[test]
    fn percentage_channel_commit_does_not_inflate_value() {
        // Saturation is displayed as a 0-100 percentage but stored as a 0..=1
        // fraction. Init formats 0.5 -> "50"; focusing and blurring without an
        // edit must commit 0.5 again, not clamp the raw "50" to the channel
        // range (which previously yielded 1.0 / 100%).
        let mut svc = service(Props {
            channel: Some(ColorChannel::Saturation),
            default_value: Some(ColorValue::from_hsl(120.0, 0.5, 0.5)),
            ..Props::default()
        });

        drop(svc.send(Event::Focus { is_keyboard: false }));
        drop(svc.send(Event::Blur));

        let saturation = svc.connect(&|_| {}).value().unwrap().saturation;
        assert!(
            (saturation - 0.5).abs() < 1e-9,
            "saturation should round-trip at 0.5, got {saturation}"
        );

        // Typing a fresh percentage commits the scaled fraction.
        drop(svc.send(Event::Change("75".to_string())));
        drop(svc.send(Event::Commit));

        let saturation = svc.connect(&|_| {}).value().unwrap().saturation;
        assert!(
            (saturation - 0.75).abs() < 1e-9,
            "saturation should commit 0.75, got {saturation}"
        );
    }

    #[test]
    fn raw_channel_commit_is_not_scaled() {
        // Hue is displayed and committed as raw degrees (no percentage scaling).
        let mut svc = service(Props {
            channel: Some(ColorChannel::Hue),
            default_value: Some(ColorValue::from_hsl(120.0, 1.0, 0.5)),
            ..Props::default()
        });

        drop(svc.send(Event::Change("210".to_string())));
        drop(svc.send(Event::Commit));

        assert_eq!(svc.connect(&|_| {}).value().unwrap().hue, 210.0);
    }

    #[test]
    fn readonly_field_sets_native_attr_and_blocks_blur_commit() {
        let mut svc = service(Props {
            readonly: true,
            default_value: Some(ColorValue::from_rgb(255, 0, 0)),
            ..Props::default()
        });

        assert!(
            svc.connect(&|_| {})
                .input_attrs()
                .contains(&HtmlAttr::ReadOnly),
            "native readonly attribute must be set"
        );

        drop(svc.send(Event::Focus { is_keyboard: false }));
        drop(svc.send(Event::Change("#0000ff".to_string())));
        drop(svc.send(Event::Blur));

        assert_eq!(
            svc.connect(&|_| {}).value().unwrap().to_rgb(),
            (255, 0, 0),
            "a readonly field must not commit edits on blur"
        );
    }

    #[test]
    fn set_props_syncs_controlled_value_and_flags() {
        let mut svc = service(Props {
            value: Some(ColorValue::from_rgb(255, 0, 0)),
            ..Props::default()
        });

        drop(svc.set_props(Props {
            id: "color-field".to_string(),
            value: Some(ColorValue::from_rgb(0, 0, 255)),
            disabled: true,
            ..Props::default()
        }));

        let api = svc.connect(&|_| {});
        assert_eq!(
            api.value().expect("controlled value present").to_rgb(),
            (0, 0, 255),
            "controlled value must follow the new prop"
        );
        assert_eq!(api.input_text(), "#0000ff", "input text reformats on sync");
        assert!(
            api.input_attrs().contains(&HtmlAttr::Disabled),
            "disabled flag must sync"
        );

        drop(svc.set_props(Props {
            id: "color-field".to_string(),
            value: Some(ColorValue::from_rgb(0, 0, 255)),
            disabled: false,
            ..Props::default()
        }));
        assert!(
            !svc.connect(&|_| {})
                .input_attrs()
                .contains(&HtmlAttr::Disabled)
        );
    }

    #[test]
    fn channel_field_without_value_is_a_usable_spinbutton() {
        // channel set, no value/default_value: must still seed a base color so
        // the spinbutton exposes value metadata and keyboard steps work.
        let mut svc = service(Props {
            channel: Some(ColorChannel::Hue),
            ..Props::default()
        });

        let input = svc.connect(&|_| {}).input_attrs();
        assert_eq!(input.get(&HtmlAttr::Role), Some("spinbutton"));
        assert!(
            input.contains(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            "spinbutton must expose aria-valuenow even without an explicit value"
        );
        assert_eq!(
            input.get(&HtmlAttr::Aria(AriaAttr::ValueMax)),
            Some("360.00")
        );

        // Keyboard stepping must work (not no-op for lack of a color).
        drop(svc.send(Event::Increment));
        assert!((svc.connect(&|_| {}).value().unwrap().hue - 1.0).abs() < 1e-9);

        // Whole-color mode with no value stays empty (unchanged behavior).
        let whole = service(Props::default());
        assert!(whole.connect(&|_| {}).value().is_none());
    }

    #[test]
    fn controlled_channel_steps_accumulate_from_pending() {
        // Controlled hue channel at 0°; two Increments before a parent sync must
        // accumulate (0 -> 1 -> 2), not recompute from the stale prop each time.
        let mut svc = service(Props {
            channel: Some(ColorChannel::Hue),
            value: Some(ColorValue::from_hsl(0.0, 1.0, 0.5)),
            ..Props::default()
        });

        drop(svc.send(Event::Increment));
        drop(svc.send(Event::Increment));

        assert!(
            (svc.connect(&|_| {}).value().unwrap().hue - 2.0).abs() < 1e-9,
            "controlled channel steps must accumulate from the pending value"
        );
    }

    #[test]
    fn keydown_ignores_event_level_composition() {
        // A keydown whose own `is_composing` flag is set must be ignored even
        // before the CompositionStart event reaches the machine.
        let svc = service(Props {
            channel: Some(ColorChannel::Hue),
            value: Some(ColorValue::from_hsl(0.0, 1.0, 0.5)),
            ..Props::default()
        });

        let captured = core::cell::RefCell::new(Vec::new());
        let send = |event: Event| captured.borrow_mut().push(event);
        let api = svc.connect(&send);

        let mut composing = key(KeyboardKey::Enter);
        composing.is_composing = true;
        api.on_input_keydown(&composing);

        assert!(
            captured.borrow().is_empty(),
            "a composing keydown must not dispatch shortcuts"
        );
    }

    #[test]
    fn controlled_commit_reports_pending_value() {
        // A controlled field that commits a new color must report it consistently
        // through value() and the hidden input, matching the displayed text, even
        // before the parent syncs the prop.
        let mut svc = service(Props {
            value: Some(ColorValue::from_rgb(255, 0, 0)),
            name: Some("color".to_string()),
            ..Props::default()
        });

        drop(svc.send(Event::Change("#0000ff".to_string())));
        drop(svc.send(Event::Commit));

        let api = svc.connect(&|_| {});
        assert_eq!(
            api.value().expect("value").to_rgb(),
            (0, 0, 255),
            "value() must report the committed (pending) color"
        );
        assert_eq!(
            api.hidden_input_attrs().get(&HtmlAttr::Value),
            Some("#0000ff"),
            "hidden input must submit the committed color, not the stale prop"
        );
    }

    #[test]
    fn set_props_seeds_channel_value_when_switching_from_empty() {
        // An empty whole-color field switched into channel mode at runtime must
        // seed a base color so the spinbutton exposes value metadata.
        let mut svc = service(Props::default());
        assert!(svc.connect(&|_| {}).value().is_none());

        drop(svc.set_props(Props {
            id: "color-field".to_string(),
            channel: Some(ColorChannel::Hue),
            ..Props::default()
        }));

        let api = svc.connect(&|_| {});
        assert!(api.value().is_some(), "channel mode must seed a base color");
        assert!(
            api.input_attrs()
                .contains(&HtmlAttr::Aria(AriaAttr::ValueNow)),
            "spinbutton must expose aria-valuenow after the switch"
        );
    }

    #[test]
    fn set_props_reformats_input_text_when_display_props_change() {
        // An unfocused whole-color field switched to channel mode must re-render
        // its text in the new representation, not leave the old hex string in a
        // now-numeric spinbutton.
        let mut svc = service(Props {
            value: Some(ColorValue::from_hsl(120.0, 1.0, 0.5)),
            ..Props::default()
        });
        assert_eq!(svc.connect(&|_| {}).input_text(), "#00ff00");

        drop(svc.set_props(Props {
            id: "color-field".to_string(),
            value: Some(ColorValue::from_hsl(120.0, 1.0, 0.5)),
            channel: Some(ColorChannel::Hue),
            ..Props::default()
        }));

        assert_eq!(
            svc.connect(&|_| {}).input_text(),
            "120",
            "text must reformat to the hue channel representation"
        );

        // A combined value + channel change (SyncValue then SetProps) must end
        // up formatted with the new channel, not the old one.
        drop(svc.set_props(Props {
            id: "color-field".to_string(),
            value: Some(ColorValue::from_hsl(0.0, 0.5, 0.5)),
            channel: Some(ColorChannel::Saturation),
            ..Props::default()
        }));
        assert_eq!(
            svc.connect(&|_| {}).input_text(),
            "50",
            "saturation 0.5 renders as 50%"
        );
    }

    #[test]
    fn invalid_field_omits_stale_hidden_value() {
        let mut svc = service(Props {
            name: Some("color".to_string()),
            default_value: Some(ColorValue::from_rgb(255, 0, 0)),
            ..Props::default()
        });

        // Valid state submits the value.
        assert_eq!(
            svc.connect(&|_| {})
                .hidden_input_attrs()
                .get(&HtmlAttr::Value),
            Some("#ff0000")
        );

        // Commit an invalid string: value stays at the old red, invalid is set.
        drop(svc.send(Event::Change("not a color".to_string())));
        drop(svc.send(Event::Commit));

        let hidden = svc.connect(&|_| {}).hidden_input_attrs();
        assert!(
            !hidden.contains(&HtmlAttr::Value),
            "an invalid field must not submit the last valid color"
        );
        // The input is disabled while invalid, so the browser omits it from
        // submission entirely (rather than sending an empty `name=`).
        assert_eq!(hidden.get(&HtmlAttr::Disabled), Some("true"));
    }

    #[test]
    fn readonly_field_ignores_ime_composition_edits() {
        let mut svc = service(Props {
            readonly: true,
            default_value: Some(ColorValue::from_rgb(255, 0, 0)),
            ..Props::default()
        });

        let before = svc.connect(&|_| {}).input_text().to_string();

        drop(svc.send(Event::CompositionStart));
        drop(svc.send(Event::Change("#0000ff".to_string())));
        drop(svc.send(Event::CompositionEnd));

        assert_eq!(
            svc.connect(&|_| {}).input_text(),
            before,
            "a read-only field must not accept IME composition edits"
        );
    }

    #[test]
    fn prop_refresh_preserves_parser_invalid_state() {
        // Commit an unparseable value, then a prop refresh with invalid=false
        // must NOT clear the parser-derived error while the bad text is shown.
        let mut svc = service(Props {
            name: Some("color".to_string()),
            default_value: Some(ColorValue::from_rgb(255, 0, 0)),
            ..Props::default()
        });

        drop(svc.send(Event::Change("not-a-color".to_string())));
        drop(svc.send(Event::Commit));
        assert_eq!(
            svc.connect(&|_| {})
                .input_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Invalid)),
            Some("true")
        );

        // External prop refresh (invalid stays false) — must not clear it.
        drop(svc.set_props(Props {
            id: "color-field".to_string(),
            name: Some("color".to_string()),
            default_value: Some(ColorValue::from_rgb(255, 0, 0)),
            invalid: false,
            ..Props::default()
        }));

        let api = svc.connect(&|_| {});
        assert_eq!(
            api.input_attrs().get(&HtmlAttr::Aria(AriaAttr::Invalid)),
            Some("true"),
            "parser-derived invalid must survive a prop refresh"
        );
        assert_eq!(
            api.hidden_input_attrs().get(&HtmlAttr::Disabled),
            Some("true"),
            "an invalid field must not become submittable after a prop refresh"
        );

        // A successful commit clears the parser error.
        drop(svc.send(Event::Change("#00ff00".to_string())));
        drop(svc.send(Event::Commit));
        assert!(
            !svc.connect(&|_| {})
                .input_attrs()
                .contains(&HtmlAttr::Aria(AriaAttr::Invalid))
        );
    }

    #[test]
    fn disabled_field_still_tracks_description() {
        // A disabled field with helper text must still wire aria-describedby.
        let mut svc = service(Props {
            disabled: true,
            ..Props::default()
        });

        drop(svc.send(Event::SetHasDescription(true)));

        let describedby = svc
            .connect(&|_| {})
            .input_attrs()
            .get(&HtmlAttr::Aria(AriaAttr::DescribedBy))
            .map(ToString::to_string)
            .expect("disabled field still references its description");
        assert!(describedby.contains("description"));
    }

    #[test]
    fn disabled_field_omits_hidden_input_from_submission() {
        let svc = service(Props {
            name: Some("color".to_string()),
            default_value: Some(ColorValue::from_rgb(255, 0, 0)),
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
    fn root_focused_invalid_snapshot() {
        let mut svc = service(Props {
            id: "fg".to_string(),
            invalid: true,
            ..Props::default()
        });

        drop(svc.send(Event::Focus { is_keyboard: true }));

        assert_snapshot!(
            "color_field_root_focused_invalid",
            snapshot_attrs(&svc.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn input_channel_mode_snapshot() {
        let svc = service(Props {
            id: "hue".to_string(),
            channel: Some(ColorChannel::Hue),
            default_value: Some(ColorValue::from_hsl(210.0, 1.0, 0.5)),
            ..Props::default()
        });

        assert_snapshot!(
            "color_field_input_channel_hue",
            snapshot_attrs(&svc.connect(&|_| {}).input_attrs())
        );
    }

    #[test]
    fn input_whole_color_invalid_describedby_snapshot() {
        let svc = service(Props {
            id: "fg".to_string(),
            invalid: true,
            required: true,
            ..Props::default()
        });

        assert_snapshot!(
            "color_field_input_whole_color_invalid",
            snapshot_attrs(&svc.connect(&|_| {}).input_attrs())
        );
    }

    #[test]
    fn exhaustive_events_parts_and_helpers() {
        // Controlled construction in channel mode exercises every event arm.
        let mut svc = service(Props {
            value: Some(ColorValue::from_hsl(120.0, 0.5, 0.5)),
            channel: Some(ColorChannel::Saturation),
            ..Props::default()
        });

        for ev in [
            Event::Focus { is_keyboard: true },
            Event::Increment,
            Event::Decrement,
            Event::IncrementLarge,
            Event::DecrementLarge,
            Event::IncrementToMax,
            Event::DecrementToMin,
            Event::SetValue(ColorValue::from_hsl(200.0, 0.3, 0.4)),
            Event::SetInvalid(true),
            Event::SetInvalid(false),
            Event::Commit,
            Event::Blur,
        ] {
            drop(svc.send(ev));
        }

        let api = svc.connect(&|_| {});

        for p in [
            Part::Root,
            Part::Label,
            Part::Input,
            Part::Description,
            Part::ErrorMessage,
            Part::HiddenInput,
        ] {
            let _attrs = api.part_attrs(p);
        }

        let _dbg = format!("{api:?}");
        let _focused = api.is_focused();
        let _text = api.input_text().to_string();

        // Dispatch helpers route to the right events via a capturing closure.
        let cap = core::cell::RefCell::new(Vec::new());
        let send = |event: Event| cap.borrow_mut().push(event);

        let dapi = svc.connect(&send);

        dapi.on_input_focus(false);
        dapi.on_input_blur();
        dapi.on_input_change("#abcdef".into());
        dapi.on_composition_start();
        dapi.on_composition_end();

        let evs = cap.borrow();

        assert!(matches!(evs[0], Event::Focus { .. }));
        assert!(matches!(evs[1], Event::Blur));
        assert!(matches!(evs[2], Event::Change(_)));
        assert!(matches!(evs[3], Event::CompositionStart));
        assert!(matches!(evs[4], Event::CompositionEnd));

        // IME-composing branch: only Change / CompositionEnd are processed.
        let mut ime = service(Props::default());

        drop(ime.send(Event::CompositionStart));
        drop(ime.send(Event::Change("rgb(1,2,3)".into())));
        drop(ime.send(Event::Increment)); // suppressed while composing
        drop(ime.send(Event::CompositionEnd));

        assert!(!ime.connect(&|_| {}).is_focused());

        // Readonly blocks Commit and channel adjustments.
        let mut ro = service(Props {
            readonly: true,
            channel: Some(ColorChannel::Hue),
            value: Some(ColorValue::from_hsl(10.0, 1.0, 0.5)),
            ..Props::default()
        });

        drop(ro.send(Event::Commit));
        drop(ro.send(Event::Increment));

        assert!((ro.connect(&|_| {}).value().unwrap().hue - 10.0).abs() < 1e-9);
    }

    #[test]
    fn connect_attrs_cover_both_flag_arms() {
        // All flags ON, channel mode, focused, value + name present.
        let mut on = service(Props {
            channel: Some(ColorChannel::Hue),
            value: Some(ColorValue::from_hsl(180.0, 1.0, 0.5)),
            disabled: true,
            readonly: true,
            invalid: true,
            required: true,
            name: Some("c".to_string()),
            ..Props::default()
        });
        drop(on.send(Event::Focus { is_keyboard: true })); // focused + focus_visible
        let on_api = on.connect(&|_| {});
        for part in [
            Part::Root,
            Part::Label,
            Part::Input,
            Part::Description,
            Part::ErrorMessage,
            Part::HiddenInput,
        ] {
            let _attrs = on_api.part_attrs(part);
        }
        // describedby references the error message when invalid.
        assert!(
            on_api
                .input_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::DescribedBy))
                .is_some()
        );

        // `has_description` true arm + describedby referencing the description.
        let mut described = service(Props::default());
        drop(described.send(Event::SetHasDescription(true)));
        let described_input = described.connect(&|_| {}).input_attrs();
        let describedby = described_input
            .get(&HtmlAttr::Aria(AriaAttr::DescribedBy))
            .expect("description is referenced");
        assert!(describedby.contains("description"));

        // All flags OFF, whole-color mode, empty value, no name.
        let off = service(Props::default());
        let off_api = off.connect(&|_| {});
        for part in [
            Part::Root,
            Part::Label,
            Part::Input,
            Part::Description,
            Part::ErrorMessage,
            Part::HiddenInput,
        ] {
            let _attrs = off_api.part_attrs(part);
        }
        // Empty describedby is not emitted.
        assert!(
            !off_api
                .input_attrs()
                .contains(&HtmlAttr::Aria(AriaAttr::DescribedBy))
        );
        // Whole-color hidden input without a name omits the name attribute.
        assert!(!off_api.hidden_input_attrs().contains(&HtmlAttr::Name));
    }

    #[test]
    fn keydown_covers_every_key_in_both_channel_modes() {
        let keys = [
            KeyboardKey::ArrowUp,
            KeyboardKey::ArrowDown,
            KeyboardKey::PageUp,
            KeyboardKey::PageDown,
            KeyboardKey::Home,
            KeyboardKey::End,
            KeyboardKey::Enter,
            KeyboardKey::Tab, // unhandled
        ];
        // Both the channel-mode (guards true) and whole-color-mode (guards false) paths.
        for channel in [Some(ColorChannel::Hue), None] {
            let svc = service(Props {
                channel,
                value: Some(ColorValue::from_hsl(10.0, 1.0, 0.5)),
                ..Props::default()
            });
            let captured = core::cell::RefCell::new(Vec::new());
            let send = |event: Event| captured.borrow_mut().push(event);
            let api = svc.connect(&send);
            for pressed in keys {
                api.on_input_keydown(&key(pressed));
            }
        }
        // While composing, keydown is fully suppressed (the early return).
        let mut composing = service(Props {
            channel: Some(ColorChannel::Hue),
            ..Props::default()
        });
        drop(composing.send(Event::CompositionStart));
        let captured = core::cell::RefCell::new(Vec::new());
        let send = |event: Event| captured.borrow_mut().push(event);
        composing
            .connect(&send)
            .on_input_keydown(&key(KeyboardKey::Enter));
        assert!(captured.borrow().is_empty());
    }

    #[test]
    fn channel_event_guards_cover_readonly_and_whole_color() {
        let channel_events = [
            Event::Increment,
            Event::Decrement,
            Event::IncrementLarge,
            Event::DecrementLarge,
            Event::IncrementToMax,
            Event::DecrementToMin,
        ];
        // Read-only channel field: every channel event is guarded out.
        let mut readonly = service(Props {
            channel: Some(ColorChannel::Hue),
            readonly: true,
            value: Some(ColorValue::from_hsl(10.0, 1.0, 0.5)),
            ..Props::default()
        });
        for event in channel_events.clone() {
            drop(readonly.send(event));
        }
        assert!((readonly.connect(&|_| {}).value().unwrap().hue - 10.0).abs() < 1e-9);

        // Whole-color field (channel is None): the `channel.is_none()` guard trips.
        let mut whole_color = service(Props {
            value: Some(ColorValue::from_hsl(10.0, 1.0, 0.5)),
            ..Props::default()
        });
        for event in channel_events {
            drop(whole_color.send(event));
        }
        // Commit while read-only is also guarded out.
        let mut readonly_commit = service(Props {
            readonly: true,
            ..Props::default()
        });
        drop(readonly_commit.send(Event::Commit));
    }
}
