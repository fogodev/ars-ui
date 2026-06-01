---
component: ColorField
category: specialized
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: [color-picker]
references:
    ark-ui: ColorPicker
    react-aria: ColorField
---

# ColorField

A text input for typing and editing color values. Operates in two modes:

1. **Whole-color mode** (default, `channel: None`): Accepts full color strings
   (`#ff0000`, `rgb(255,0,0)`, `hsl(0,100%,50%)`). Parsed on commit (blur/Enter)
   via `parse_color_string` from §1.1.
2. **Channel mode** (`channel: Some(ch)`): Numeric spinbutton for a single channel.
   Arrow keys increment/decrement by step using `channel_value`, `with_channel`,
   and `channel_range` from §1.1.

`ColorField` reuses `ColorValue`, `ColorChannel`, `ColorFormat`, and the shared helpers
(`channel_value`, `with_channel`, `channel_range`, `channel_step_default`,
`parse_color_string`, `format_color_string`) from the color-picker shared types (§1.1 of `color-picker.md`).

## 1. State Machine

### 1.1 States

```rust
/// The states for the ColorField component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// Input is not focused.
    Idle,
    /// Input is focused, user may be editing text.
    Focused,
}
```

### 1.2 Events

```rust
/// The events for the ColorField component.
#[derive(Clone, Debug)]
pub enum Event {
    /// Input received focus.
    Focus {
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// Input lost focus — triggers commit.
    Blur,
    /// Raw text changed (keystroke or paste). No parsing until commit.
    Change(String),
    /// Enter key — parse and commit without leaving Focused.
    Commit,
    /// Programmatic value update from parent.
    SetValue(ColorValue),
    /// Programmatic invalid state.
    SetInvalid(bool),
    /// Adapter signal that a description part is (or is no longer) rendered,
    /// toggling whether the input's `aria-describedby` references it.
    SetHasDescription(bool),
    /// Channel mode: increment by step (ArrowUp).
    Increment,
    /// Channel mode: decrement by step (ArrowDown).
    Decrement,
    /// Channel mode: increment by large_step (PageUp).
    IncrementLarge,
    /// Channel mode: decrement by large_step (PageDown).
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
```

### 1.3 Context

```rust
/// The context for the ColorField component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The current color value (controlled or uncontrolled). `None` when empty.
    pub value: Bindable<Option<ColorValue>>,
    /// Raw text in the input. Diverges from `value` while editing.
    pub input_text: String,
    /// If `Some`, the field edits a single channel (numeric spinbutton).
    /// If `None`, the field accepts whole color strings.
    pub channel: Option<ColorChannel>,
    /// Display format for formatting value → text. Default: Hex.
    pub color_format: ColorFormat,
    /// Step size for channel-mode keyboard adjustment.
    pub step: f64,
    /// Large step size for channel-mode PageUp/PageDown.
    pub large_step: f64,
    /// Whether the input is focused.
    pub focused: bool,
    /// Whether focus was via keyboard (for focus-visible ring).
    pub focus_visible: bool,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is read-only.
    pub readonly: bool,
    /// Whether the value is invalid per *external* validation (the `invalid`
    /// prop / `SetInvalid`). Kept separate from `parse_error` so a prop refresh
    /// cannot clear a parser-derived error.
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
```

### 1.4 Props

```rust
/// The props for the ColorField component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Controlled value. When `Some`, the component is controlled.
    pub value: Option<ColorValue>,
    /// Default value for uncontrolled mode.
    pub default_value: Option<ColorValue>,
    /// If `Some`, the field edits a single channel (numeric spinbutton).
    /// If `None`, the field accepts whole color strings.
    pub channel: Option<ColorChannel>,
    /// Display format for whole-color mode. Default: Hex.
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
    /// Default: `channel_step_default(ch)` when channel is set.
    pub step: Option<f64>,
    /// Large step size for channel-mode PageUp/PageDown.
    /// Default: `step * 10`.
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
```

### 1.5 Full Machine Implementation

```rust
/// Whether a channel is shown and entered as a percentage (`0-100`) rather than
/// its underlying `0..=1` fractional range.
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
        ctx.value.set(Some(new_color.clone()));
        ctx.input_text = format_value(&new_color, ctx.channel, ctx.color_format);
        ctx.parse_error = false;
    }
}

/// The machine for the `ColorField` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    // ColorField emits no named effects (value changes flow through `Bindable`).
    type Effect = ars_core::NoEffect;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let value = match &props.value {
            Some(v) => Bindable::controlled(Some(v.clone())),
            None => {
                // A channel field is an ARIA spinbutton: it needs a base color to
                // expose a valid value and to accept keyboard steps / commits.
                // Seed an initial color in channel mode when none was supplied.
                let seed = props
                    .default_value
                    .or_else(|| props.channel.map(|_| ColorValue::default()));
                Bindable::uncontrolled(seed)
            }
        };

        let step = props.step.unwrap_or_else(|| {
            props.channel.map(channel_step_default).unwrap_or(1.0)
        });

        let large_step = props.large_step.unwrap_or(step * 10.0);

        let input_text = match value.get() {
            Some(c) => format_value(c, props.channel, props.color_format),
            None => String::new(),
        };

        let ids = ComponentIds::from_id(&props.id);
        let locale = env.locale.clone();
        let messages = messages.clone();

        (State::Idle, Context {
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
            locale,
            messages,
            ids,
        })
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

                            // A synced value clears only the parser-derived
                            // error; the external `invalid` prop is owned by the
                            // parent (via `SetProps`/`SetInvalid`) and must not be
                            // cleared here, or a value update that keeps
                            // `invalid: true` would wrongly drop the invalid state.
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

                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = kb;
                }))
            }

            Event::Blur => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    // A read-only field must never mutate its value on blur.
                    if !ctx.readonly {
                        commit_input(ctx);
                    }
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }))
            }

            Event::Change(text) => {
                // Read-only fields ignore edits so input text cannot diverge from
                // the committed value (and cannot be committed later on blur).
                if ctx.readonly {
                    return None;
                }

                let t = text.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.input_text = t;
                }))
            }

            Event::Commit => {
                if ctx.readonly { return None; }

                Some(TransitionPlan::context_only(|ctx| {
                    commit_input(ctx);
                }))
            }

            Event::SetValue(color) => {
                let c = color.clone();

                Some(TransitionPlan::context_only(move |ctx| {
                    if !ctx.focused {
                        ctx.input_text = format_value(&c, ctx.channel, ctx.color_format);
                    }
                    ctx.value.set(Some(c));
                    // Clear only the parser-derived error; external `invalid` is
                    // controlled by the parent (`SetInvalid`/the `invalid` prop).
                    ctx.parse_error = false;
                }))
            }

            Event::SetInvalid(inv) => {
                let inv = *inv;

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.invalid = inv;
                }))
            }

            Event::SetHasDescription(has_description) => {
                let has_description = *has_description;

                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.has_description = has_description;
                }))
            }

            Event::Increment => {
                if ctx.readonly || ctx.channel.is_none() { return None; }

                let step = ctx.step;

                Some(TransitionPlan::context_only(move |ctx| {
                    adjust_channel(ctx, step);
                }))
            }

            Event::Decrement => {
                if ctx.readonly || ctx.channel.is_none() { return None; }

                let step = -ctx.step;

                Some(TransitionPlan::context_only(move |ctx| {
                    adjust_channel(ctx, step);
                }))
            }

            Event::IncrementLarge => {
                if ctx.readonly || ctx.channel.is_none() { return None; }

                let step = ctx.large_step;

                Some(TransitionPlan::context_only(move |ctx| {
                    adjust_channel(ctx, step);
                }))
            }

            Event::DecrementLarge => {
                if ctx.readonly || ctx.channel.is_none() { return None; }

                let step = -ctx.large_step;

                Some(TransitionPlan::context_only(move |ctx| {
                    adjust_channel(ctx, step);
                }))
            }

            Event::IncrementToMax => {
                if ctx.readonly || ctx.channel.is_none() { return None; }

                Some(TransitionPlan::context_only(|ctx| {
                    // Read the pending color so the snap builds on the staged value.
                    if let (Some(ch), Some(color)) = (ctx.channel, *ctx.value.pending()) {
                        let color = &color;
                        let (_, max) = channel_range(ch);
                        let new_color = with_channel(color, ch, max);
                        ctx.value.set(Some(new_color.clone()));
                        ctx.input_text = format_value(&new_color, ctx.channel, ctx.color_format);
                        ctx.parse_error = false;
                    }
                }))
            }

            Event::DecrementToMin => {
                if ctx.readonly || ctx.channel.is_none() { return None; }

                Some(TransitionPlan::context_only(|ctx| {
                    // Read the pending color so the snap builds on the staged value.
                    if let (Some(ch), Some(color)) = (ctx.channel, *ctx.value.pending()) {
                        let color = &color;
                        let (min, _) = channel_range(ch);
                        let new_color = with_channel(color, ch, min);
                        ctx.value.set(Some(new_color.clone()));
                        ctx.input_text = format_value(&new_color, ctx.channel, ctx.color_format);
                        ctx.parse_error = false;
                    }
                }))
            }

            Event::CompositionStart => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.is_composing = true;
                }))
            }

            Event::CompositionEnd => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.is_composing = false;
                }))
            }
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
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
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
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "color-field"]
pub enum Part {
    Root,
    Label,
    Input,
    Description,
    ErrorMessage,
    HiddenInput,
}

/// The connect API for the `ColorField` component.
pub struct Api<'a> {
    /// The current state of the component.
    state: &'a State,
    /// The context of the component.
    ctx: &'a Context,
    /// The props of the component.
    props: &'a Props,
    /// The send function to send events to the component.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Whether the component is currently focused.
    pub fn is_focused(&self) -> bool {
        matches!(self.state, State::Focused)
    }

    /// The effective invalid state: external validation (`invalid` prop /
    /// `SetInvalid`) OR a parser-derived error from the last commit.
    pub const fn is_invalid(&self) -> bool {
        self.ctx.invalid || self.ctx.parse_error
    }

    /// The current value of the component.
    ///
    /// Reports the *pending* value so a committed edit is reflected consistently
    /// (matching the displayed text, channel ARIA, and the hidden input) even in
    /// controlled mode, where the controlled prop only updates via `SyncValue`.
    pub const fn value(&self) -> Option<&ColorValue> {
        self.ctx.value.pending().as_ref()
    }

    /// The current input text of the component.
    pub fn input_text(&self) -> &str {
        &self.ctx.input_text
    }

    /// The attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
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
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let label_id = self.ctx.ids.part("label");
        attrs.set(HtmlAttr::Id, label_id);
        attrs.set(HtmlAttr::For, self.ctx.ids.part("input"));
        attrs
    }

    /// The attributes for the input element.
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let input_id = self.ctx.ids.part("input");
        attrs.set(HtmlAttr::Id, input_id);
        attrs.set(HtmlAttr::Type, "text");
        attrs.set(HtmlAttr::Value, &self.ctx.input_text);

        // Mode-specific ARIA
        match self.ctx.channel {
            Some(ch) => {
                // Channel mode: numeric spinbutton
                attrs.set(HtmlAttr::Role, "spinbutton");
                attrs.set(HtmlAttr::InputMode, "numeric");
                if let Some(color) = self.ctx.value.pending() {
                    let val = channel_value(color, ch);
                    let (min, max) = channel_range(ch);
                    // Percentage channels display/commit as 0-100, so the spinbutton
                    // ARIA range must match (the stored channel range is 0..=1).
                    let scale = if channel_is_percentage(ch) { 100.0 } else { 1.0 };
                    attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), format!("{:.2}", val * scale));
                    attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), format!("{:.2}", min * scale));
                    attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), format!("{:.2}", max * scale));
                    attrs.set(HtmlAttr::Aria(AriaAttr::ValueText),
                        (self.ctx.messages.channel_value_text)(ch, val, &self.ctx.locale));
                }
                attrs.set(HtmlAttr::Aria(AriaAttr::Label),
                    (self.ctx.messages.channel_label)(ch, &self.ctx.locale));
            }
            None => {
                // Whole-color mode: standard text input
                attrs.set(HtmlAttr::InputMode, "text");
                attrs.set(HtmlAttr::Aria(AriaAttr::Label),
                    (self.ctx.messages.color_label)(&self.ctx.locale));
            }
        }

        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));

        if self.is_invalid() {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }
        if self.ctx.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }
        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::ReadOnly, true);
            attrs.set_bool(HtmlAttr::Aria(AriaAttr::ReadOnly), true);
        }
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set_bool(HtmlAttr::Aria(AriaAttr::Disabled), true);
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
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));
        attrs
    }

    /// Returns the error message text when color parsing fails.
    /// The adapter renders this inside the `ErrorMessage` part.
    pub fn invalid_message(&self) -> String {
        (self.ctx.messages.invalid_message)(&self.ctx.locale)
    }

    /// The attributes for the error message element.
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("error-message"));
        attrs.set(HtmlAttr::Role, "alert");
        attrs
    }

    /// The attributes for the hidden input element.
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(ref name) = self.ctx.name {
            attrs.set(HtmlAttr::Name, name);
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

    /// The on input focus handler.
    pub fn on_input_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// The on input blur handler.
    pub fn on_input_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// The on input change handler.
    pub fn on_input_change(&self, text: String) {
        (self.send)(Event::Change(text));
    }

    /// Handle keydown on the input element.
    ///
    /// Shortcuts are suppressed while an IME composition is in progress — both
    /// when the machine already knows (`ctx.is_composing`) and when the event
    /// itself reports composing (`data.is_composing`), in case the keydown
    /// arrives before the `CompositionStart` event reaches the machine.
    pub fn on_input_keydown(&self, data: &KeyboardEventData) {
        if self.ctx.is_composing || data.is_composing { return; }
        match data.key {
            KeyboardKey::Enter => (self.send)(Event::Commit),
            KeyboardKey::ArrowUp if self.ctx.channel.is_some() => (self.send)(Event::Increment),
            KeyboardKey::ArrowDown if self.ctx.channel.is_some() => (self.send)(Event::Decrement),
            KeyboardKey::PageUp if self.ctx.channel.is_some() => (self.send)(Event::IncrementLarge),
            KeyboardKey::PageDown if self.ctx.channel.is_some() => (self.send)(Event::DecrementLarge),
            KeyboardKey::Home if self.ctx.channel.is_some() => (self.send)(Event::DecrementToMin),
            KeyboardKey::End if self.ctx.channel.is_some() => (self.send)(Event::IncrementToMax),
            _ => {}
        }
    }

    /// The on composition start handler.
    pub fn on_composition_start(&self) {
        (self.send)(Event::CompositionStart);
    }

    /// The on composition end handler.
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
```

## 2. Anatomy

```text
ColorField
├── Root             (<div>)        (required)
├── Label            (<label>)      (required)
├── Input            (<input>)      (required — type="text")
├── Description      (<div>)        (optional)
├── ErrorMessage     (<div>)        (optional — role="alert")
└── HiddenInput      (<input>)      (required — type="hidden", submits hex)
```

| Part         | Element   | Key Attributes                                                                                                                                                                                                                                              |
| ------------ | --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Root         | `<div>`   | `data-ars-disabled`, `data-ars-readonly`, `data-ars-invalid` (effective invalid: external `invalid` prop **or** parser-derived `parse_error`; a prop refresh preserves the parser error), `data-ars-focused`                                                |
| Label        | `<label>` | `for` pointing to Input                                                                                                                                                                                                                                     |
| Input        | `<input>` | `type="text"`, `aria-labelledby`, `aria-invalid`, `aria-required`, `aria-describedby`, native `readonly` (when read-only)                                                                                                                                   |
| Description  | `<div>`   | Referenced by Input `aria-describedby`                                                                                                                                                                                                                      |
| ErrorMessage | `<div>`   | `role="alert"`, referenced by Input `aria-describedby`                                                                                                                                                                                                      |
| HiddenInput  | `<input>` | `type="hidden"`, `name`, `value` (hex; omitted while invalid so a stale last-valid color is not submitted), `disabled` (when disabled **or invalid** — omitted from form submission so an invalid field round-trips as absent rather than an empty `name=`) |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Attribute / Behaviour             | Element                  | Value                                                                                                                                                                                                                                                                     |
| --------------------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `role="spinbutton"`               | Input (channel mode)     | ARIA spinbutton pattern                                                                                                                                                                                                                                                   |
| `inputmode="numeric"`             | Input (channel mode)     | Numeric keyboard on mobile                                                                                                                                                                                                                                                |
| `inputmode="text"`                | Input (whole-color mode) | Text keyboard on mobile                                                                                                                                                                                                                                                   |
| `aria-valuenow`                   | Input (channel mode)     | Current channel value, scaled to the visible 0-100 range for percentage channels                                                                                                                                                                                          |
| `aria-valuemin` / `aria-valuemax` | Input (channel mode)     | From `channel_range(channel)`, scaled to 0-100 for percentage channels (saturation/lightness/brightness/alpha)                                                                                                                                                            |
| `aria-valuetext`                  | Input (channel mode)     | Localized formatted channel value                                                                                                                                                                                                                                         |
| `aria-label`                      | Input (channel mode)     | Channel name (from messages)                                                                                                                                                                                                                                              |
| `aria-labelledby`                 | Input                    | Label element ID                                                                                                                                                                                                                                                          |
| `aria-invalid`                    | Input                    | `"true"` for the effective invalid state — the external `invalid` prop / `SetInvalid` **or** the parser-derived `parse_error` (`invalid \|\| parse_error`). A prop refresh (`SetProps`) updates only the external part and preserves a parser error from the last commit. |
| `aria-required`                   | Input                    | `"true"` when required                                                                                                                                                                                                                                                    |
| `aria-readonly` / `readonly`      | Input                    | When read-only (both the native attribute and `aria-readonly` are set)                                                                                                                                                                                                    |
| `aria-disabled` / `disabled`      | Input                    | When disabled                                                                                                                                                                                                                                                             |
| `aria-describedby`                | Input                    | Description + ErrorMessage IDs (kept in sync via `SetHasDescription`, which is honored even while the field is disabled or mid-IME-composition)                                                                                                                           |
| `role="alert"`                    | ErrorMessage             | Live error announcement                                                                                                                                                                                                                                                   |

### 3.2 Keyboard Interaction

| Key       | Context              | Action                  |
| --------- | -------------------- | ----------------------- |
| Enter     | Input                | Commit (parse value)    |
| ArrowUp   | Input (channel mode) | Increment by step       |
| ArrowDown | Input (channel mode) | Decrement by step       |
| PageUp    | Input (channel mode) | Increment by large_step |
| PageDown  | Input (channel mode) | Decrement by large_step |
| Home      | Input (channel mode) | Set to min              |
| End       | Input (channel mode) | Set to max              |

IME: All keyboard shortcuts are suppressed while `is_composing` is true.

## 4. Internationalization

### 4.1 Messages

```rust
/// The messages for the `ColorField` component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Label for a channel input. Default: channel name (e.g., `"Hue"`).
    pub channel_label: MessageFn<dyn Fn(ColorChannel, &Locale) -> String + Send + Sync>,
    /// Formatted channel value for aria-valuetext.
    pub channel_value_text: MessageFn<dyn Fn(ColorChannel, f64, &Locale) -> String + Send + Sync>,
    /// Label for whole-color mode. Default: `"Color value"`.
    pub color_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Message shown when parsing fails. Default: `"Invalid color value"`.
    pub invalid_message: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            channel_label: MessageFn::new(|ch, _locale| format!("{ch:?}")),
            channel_value_text: MessageFn::new(|ch, val, _locale| match ch {
                ColorChannel::Hue => format!("{val:.0}°"),
                ColorChannel::Red | ColorChannel::Green | ColorChannel::Blue => format!("{val:.0}"),
                _ => format!("{:.0}%", val * 100.0),
            }),
            color_label: MessageFn::static_str("Color value"),
            invalid_message: MessageFn::static_str("Invalid color value"),
        }
    }
}

impl ComponentMessages for Messages {}
```

| Key                              | Default (en-US)                              | Purpose                         |
| -------------------------------- | -------------------------------------------- | ------------------------------- |
| `color_field.channel_label`      | Channel name (e.g., `"Hue"`)                 | Channel-mode `aria-label`       |
| `color_field.channel_value_text` | Channel-specific formatting (e.g., `"180°"`) | Channel-mode `aria-valuetext`   |
| `color_field.color_label`        | `"Color value"`                              | Whole-color mode label fallback |
| `color_field.invalid_message`    | `"Invalid color value"`                      | Error message text              |

- **Color format strings** (hex/rgb/hsl CSS syntax) are locale-independent.
- **Channel `aria-valuetext`** uses localized number formatting via message closures.
- **RTL**: No special layout handling needed (single text input).

## 5. Library Parity

> Compared against: Ark UI (`ColorPicker.ChannelInput`), React Aria (`ColorField`).

### 5.1 Props

| Feature                  | ars-ui                    | Ark UI            | React Aria               | Notes                                                 |
| ------------------------ | ------------------------- | ----------------- | ------------------------ | ----------------------------------------------------- |
| `value` / `defaultValue` | `value` / `default_value` | (root-level)      | `value` / `defaultValue` | Equivalent                                            |
| `channel`                | `channel`                 | `channel`         | `channel`                | Equivalent                                            |
| `colorSpace`             | --                        | --                | `colorSpace`             | React Aria has colorSpace; ars-ui uses `color_format` |
| `isDisabled`             | `disabled`                | `disabled` (root) | `isDisabled`             | Equivalent                                            |
| `isReadOnly`             | `readonly`                | `readOnly` (root) | `isReadOnly`             | Equivalent                                            |
| `isRequired`             | `required`                | `required` (root) | `isRequired`             | Equivalent                                            |
| `isInvalid`              | `invalid`                 | `invalid` (root)  | `isInvalid`              | Equivalent                                            |
| `isWheelDisabled`        | --                        | --                | `isWheelDisabled`        | React Aria disables scroll wheel changes              |
| `placeholder`            | --                        | --                | `placeholder`            | React Aria has placeholder text                       |
| `name`                   | `name`                    | --                | `name`                   | Equivalent                                            |
| `validate`               | --                        | --                | `validate`               | React Aria has custom validation function             |
| `validationBehavior`     | --                        | --                | `validationBehavior`     | React Aria native/aria validation                     |

**Gaps:** None worth adopting. `isWheelDisabled` is niche (prevents accidental scroll-wheel changes). Custom `validate` is handled by `invalid` prop + application-level validation. `placeholder` can be set by the adapter.

### 5.2 Anatomy

| Part         | ars-ui         | Ark UI         | React Aria                  | Notes                 |
| ------------ | -------------- | -------------- | --------------------------- | --------------------- |
| Root         | `Root`         | --             | `ColorField`                | Equivalent            |
| Label        | `Label`        | --             | `Label`                     | Equivalent            |
| Input        | `Input`        | `ChannelInput` | `Input`                     | Equivalent            |
| Description  | `Description`  | --             | `Text` (slot="description") | Equivalent            |
| ErrorMessage | `ErrorMessage` | --             | `FieldError`                | Equivalent            |
| HiddenInput  | `HiddenInput`  | --             | --                          | ars-ui has form input |

**Gaps:** None.

### 5.3 Events

| Callback     | ars-ui                         | Ark UI                 | React Aria                             | Notes      |
| ------------ | ------------------------------ | ---------------------- | -------------------------------------- | ---------- |
| Value change | `Bindable` reactivity          | `onValueChange` (root) | `onChange`                             | Equivalent |
| Focus/Blur   | `Event::Focus` / `Event::Blur` | --                     | `onFocus` / `onBlur` / `onFocusChange` | Equivalent |

**Gaps:** None.

### 5.4 Features

| Feature                      | ars-ui                 | Ark UI            | React Aria            |
| ---------------------------- | ---------------------- | ----------------- | --------------------- |
| Whole-color mode             | Yes                    | No (channel only) | Yes                   |
| Channel mode (spinbutton)    | Yes                    | Yes               | Yes                   |
| IME composition handling     | Yes                    | --                | --                    |
| Keyboard increment/decrement | Yes                    | Yes               | Yes                   |
| Validation                   | Yes (external + parse) | --                | Yes (custom + native) |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui supports both whole-color and channel modes in a single component. Ark UI only has channel inputs as part of ColorPicker. React Aria has a dedicated ColorField with both modes.
- **Recommended additions:** None.
