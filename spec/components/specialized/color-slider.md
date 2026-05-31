---
component: ColorSlider
category: specialized
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: [color-picker, color-area]
references:
    ark-ui: ColorPicker
    react-aria: ColorSlider
---

# ColorSlider

A standalone 1D color channel slider that adjusts a single channel of a `ColorValue`.
Can render as horizontal or vertical. Used independently or composed inside ColorPicker.
Reuses `ColorValue`, `ColorChannel`, `Orientation`, and the shared helpers
(`channel_value`, `with_channel`, `channel_range`, `channel_step_default`) from the color types.

## 1. State Machine

### 1.1 States

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// No interaction in progress.
    Idle,
    /// User is dragging the thumb.
    Dragging,
}
```

### 1.2 Events

```rust
#[derive(Clone, Copy, Debug)]
pub enum Event {
    /// User started dragging (normalized position 0..1 along the track).
    DragStart { position: f64 },
    /// User is moving while dragging.
    DragMove { position: f64 },
    /// User released the drag.
    DragEnd,
    /// Increment channel by step.
    Increment { step: f64 },
    /// Decrement channel by step.
    Decrement { step: f64 },
    /// Snap channel to its minimum.
    SetToMin,
    /// Snap channel to its maximum.
    SetToMax,
    /// Focus entered the thumb.
    Focus { is_keyboard: bool },
    /// Focus left the thumb.
    Blur,
    /// Controlled-value sync from the parent after `Service::set_props`.
    SyncValue(Option<ColorValue>),
    /// Refresh cached output props after `Service::set_props`.
    SetProps,
}
```

### 1.3 Context

```rust
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
    /// Whether focus was via keyboard (for focus-visible ring).
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
```

### 1.4 Props

```rust
#[derive(Clone, Debug, PartialEq, HasId)]
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
    /// Large step size for Shift+Arrow / PageUp/PageDown.
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
    pub on_change_end: Option<Callback<dyn Fn(ColorValue) + Send + Sync>>,
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
```

### 1.5 Full Machine Implementation

```rust,no_check
/// Apply a normalized position (0..1) to the channel value.
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

/// Typed identifier for side effects emitted by the machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Invoke `Props::on_change_end`.
    ChangeEnd,
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
        let value = match &props.value {
            Some(v) => Bindable::controlled(v.clone()),
            None => Bindable::uncontrolled(props.default_value.clone()),
        };
        let slider_value = channel_value(value.get(), props.channel);
        let ids = ComponentIds::from_id(&props.id);
        let locale = env.locale.clone();
        let messages = messages.clone();

        (State::Idle, Context {
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
            locale,
            messages,
            ids,
        })
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
            // The adapter resolves the normalized position and drives
            // DragMove/DragEnd from its own pointer listeners.
            (State::Idle, Event::DragStart { position }) => {
                if ctx.readonly { return None; }
                let pos = *position;
                Some(TransitionPlan::to(State::Dragging).apply(move |ctx| {
                    apply_slider_position(ctx, pos);
                }))
            }

            (State::Dragging, Event::DragMove { position }) => {
                // Readonly toggled mid-drag must stop further value changes
                // (disabled is already handled by the guard above); DragEnd
                // still terminates the drag.
                if ctx.readonly { return None; }

                let pos = *position;
                Some(TransitionPlan::context_only(move |ctx| {
                    apply_slider_position(ctx, pos);
                }))
            }

            (State::Dragging, Event::DragEnd) => {
                Some(TransitionPlan::to(State::Idle).with_effect(change_end_effect()))
            }

            (_, Event::Increment { step }) => {
                if ctx.readonly { return None; }
                let step = *step;
                Some(TransitionPlan::context_only(move |ctx| {
                    let (_, max) = channel_range(ctx.channel);
                    set_channel_value(ctx, (ctx.slider_value + step).min(max));
                }))
            }

            (_, Event::Decrement { step }) => {
                if ctx.readonly { return None; }
                let step = *step;
                Some(TransitionPlan::context_only(move |ctx| {
                    let (min, _) = channel_range(ctx.channel);
                    set_channel_value(ctx, (ctx.slider_value - step).max(min));
                }))
            }

            (_, Event::SetToMin) => {
                if ctx.readonly { return None; }
                Some(TransitionPlan::context_only(|ctx| {
                    let (min, _) = channel_range(ctx.channel);
                    set_channel_value(ctx, min);
                }))
            }

            (_, Event::SetToMax) => {
                if ctx.readonly { return None; }
                Some(TransitionPlan::context_only(|ctx| {
                    let (_, max) = channel_range(ctx.channel);
                    set_channel_value(ctx, max);
                }))
            }

            (_, Event::Focus { is_keyboard }) => {
                let kb = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = kb;
                }))
            }

            (_, Event::Blur) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }))
            }

            (_, Event::SyncValue(value)) => {
                let value = *value;
                Some(TransitionPlan::context_only(move |ctx| match value {
                    Some(color) => {
                        // If the parent is echoing the value we just emitted,
                        // keep the cached slider value so a hue 360° endpoint
                        // isn't re-derived back to 0° (the color normalizes
                        // 360° -> 0°). Only re-derive for a genuinely new color.
                        let echoes_pending = color == *ctx.value.pending();
                        ctx.value.set(color);
                        ctx.value.sync_controlled(Some(color));
                        if !echoes_pending {
                            ctx.slider_value = channel_value(&color, ctx.channel);
                        }
                    }
                    None => ctx.value.sync_controlled(None),
                }))
            }

            (_, Event::SetProps) => {
                let props = props.clone();
                Some(TransitionPlan::context_only(move |ctx| {
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
        Api { state, ctx, props, send }
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
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "color-slider"]
pub enum Part {
    Root,
    Label,
    Track,
    Thumb,
    Output,
    HiddenInput,
}

pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    pub fn is_dragging(&self) -> bool { matches!(self.state, State::Dragging) }
    pub fn value(&self) -> &ColorValue { self.ctx.value.get() }

    /// Current channel value formatted for display.
    pub fn formatted_value(&self) -> String {
        // Use the unwrapped slider value so the hue endpoint reads "360°".
        let val = self.ctx.slider_value;
        match self.ctx.channel {
            ColorChannel::Hue => format!("{:.0}°", val),
            ColorChannel::Red | ColorChannel::Green | ColorChannel::Blue => format!("{:.0}", val),
            _ => format!("{:.0}%", val * 100.0),
        }
    }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "group");
        attrs.set(HtmlAttr::Data("ars-channel"), format!("{:?}", self.ctx.channel).to_lowercase());
        attrs.set(HtmlAttr::Data("ars-orientation"),
            if self.ctx.orientation == Orientation::Vertical { "vertical" } else { "horizontal" });
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.readonly { attrs.set_bool(HtmlAttr::Data("ars-readonly"), true); }
        if self.is_dragging() { attrs.set_bool(HtmlAttr::Data("ars-dragging"), true); }
        attrs
    }

    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("label"));
        attrs.set(HtmlAttr::For, self.ctx.ids.part("thumb"));
        attrs
    }

    pub fn track_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Track.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        // Use the pending color so the gradient matches the in-progress drag
        // position in controlled mode (where `get()` returns the stale prop).
        let color = self.ctx.value.pending();
        // A horizontal RTL slider renders the minimum on the right, so the
        // gradient runs `to left` to keep the visible ramp aligned with the
        // value selected by dragging/clicking.
        let to_edge = if self.ctx.orientation == Orientation::Horizontal && self.ctx.dir == Direction::Rtl {
            "to left"
        } else {
            "to right"
        };
        let gradient = match self.ctx.channel {
            ColorChannel::Hue => format!(
                "linear-gradient({to_edge}, \
                hsl(0,100%,50%), hsl(60,100%,50%), hsl(120,100%,50%), \
                hsl(180,100%,50%), hsl(240,100%,50%), hsl(300,100%,50%), \
                hsl(360,100%,50%))"
            ),
            ColorChannel::Alpha => format!(
                // Fade from the *same* color at alpha 0 to alpha 1, so the track
                // previews only opacity. `transparent` is transparent black and
                // would make non-black colors fade through gray.
                "linear-gradient({to_edge}, {}, {})",
                ColorValue::new(color.hue, color.saturation, color.lightness, 0.0).to_css_hsl(),
                ColorValue::new(color.hue, color.saturation, color.lightness, 1.0).to_css_hsl()
            ),
            _ => {
                let (min, max) = channel_range(self.ctx.channel);
                let start = with_channel(color, self.ctx.channel, min);
                let end = with_channel(color, self.ctx.channel, max);
                format!("linear-gradient({to_edge}, {}, {})", start.to_css_hsl(), end.to_css_hsl())
            }
        };
        attrs.set_style(CssProperty::Custom("ars-color-slider-track-bg"), gradient);
        attrs
    }

    pub fn thumb_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Thumb.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("thumb"));
        attrs.set(HtmlAttr::Role, "slider");
        // A disabled control must stay out of the tab order.
        attrs.set(HtmlAttr::TabIndex, if self.ctx.disabled { "-1" } else { "0" });
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

        attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), format!("{:.2}", val));
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), format!("{:.2}", min));
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), format!("{:.2}", max));
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation),
            if self.ctx.orientation == Orientation::Vertical { "vertical" } else { "horizontal" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.label)(&self.ctx.locale));
        // Channel-aware reading ("hue 180°") plus the perceptual color name, as
        // required by spec §3.1 (e.g. "hue 180°, dark vibrant blue").
        let channel_name = format!("{:?}", self.ctx.channel).to_lowercase();
        let reading = format!("{channel_name} {}", self.formatted_value());
        let color_name = color.color_name_en();
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), (self.ctx.messages.value_text)(&reading, &color_name, &self.ctx.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));

        let mut pct = if (max - min).abs() > f64::EPSILON { (val - min) / (max - min) * 100.0 } else { 0.0 };
        // A horizontal RTL slider flips its axis (min on the right), matching the
        // mirrored arrow-key handling in `on_thumb_keydown`.
        if self.ctx.orientation == Orientation::Horizontal && self.ctx.dir == Direction::Rtl {
            pct = 100.0 - pct;
        }
        attrs.set_style(CssProperty::Custom("ars-color-slider-thumb-position"), format!("{:.1}%", pct));
        attrs.set_style(CssProperty::BackgroundColor, color.to_css_hsl());

        if self.is_dragging() { attrs.set_bool(HtmlAttr::Data("ars-dragging"), true); }
        if self.ctx.focus_visible { attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true); }
        attrs
    }

    pub fn output_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Output.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::For, self.ctx.ids.part("thumb"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "off");
        attrs
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
        attrs.set(HtmlAttr::Value, self.ctx.value.pending().to_hex(true));
        // A disabled control must be omitted from form submission.
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        attrs
    }

    pub fn on_thumb_keydown(&self, data: &KeyboardEventData, shift: bool) {
        let step = if shift { self.ctx.large_step } else { self.ctx.step };
        match data.key {
            KeyboardKey::ArrowRight | KeyboardKey::ArrowUp => (self.send)(Event::Increment { step }),
            KeyboardKey::ArrowLeft | KeyboardKey::ArrowDown => (self.send)(Event::Decrement { step }),
            KeyboardKey::Home => (self.send)(Event::SetToMin),
            KeyboardKey::End => (self.send)(Event::SetToMax),
            KeyboardKey::PageUp => (self.send)(Event::Increment { step: self.ctx.large_step }),
            KeyboardKey::PageDown => (self.send)(Event::Decrement { step: self.ctx.large_step }),
            _ => {}
        }
    }

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
```

## 2. Anatomy

```text
ColorSlider
├── Root          (required — <div>, role="group")
├── Label         (required — <label>)
├── Track         (required — <div>, gradient background)
│   └── Thumb     (required — <div>, role="slider", focusable)
├── Output        (optional — <output>, displays current value)
└── HiddenInput   (optional — <input type="hidden">, form submission)
```

| Part        | Element    | Key Attributes                                                                                                                               |
| ----------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| Root        | `<div>`    | `role="group"`, `data-ars-channel`, `data-ars-orientation`                                                                                   |
| Label       | `<label>`  | `id`, `for` (thumb id)                                                                                                                       |
| Track       | `<div>`    | gradient background via CSS custom property                                                                                                  |
| Thumb       | `<div>`    | `role="slider"`, `aria-valuenow/min/max`, `aria-orientation`, `tabindex` (`"-1"` when disabled, else `"0"`), `aria-disabled` (when disabled) |
| Output      | `<output>` | `for` (thumb id), `aria-live="off"`                                                                                                          |
| HiddenInput | `<input>`  | `type="hidden"`, `name`, `value` (hex color), `disabled` (when disabled — omitted from form submission)                                      |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part   | Role     | Properties                                                                                                                                                |
| ------ | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Root   | `group`  | groups slider components                                                                                                                                  |
| Thumb  | `slider` | `aria-valuenow`, `aria-valuemin`, `aria-valuemax`, `aria-orientation`, `aria-label`, `aria-valuetext`, `aria-labelledby`, `aria-disabled` (when disabled) |
| Output | —        | `aria-live="off"` (prevents double-announcement with valuetext)                                                                                           |

`aria-valuetext` MUST include a human-readable color name from `color_name_parts()`, not raw numeric values (e.g., `"hue 180°, dark vibrant blue"`).

### 3.2 Keyboard Interaction

| Key              | Action                  |
| ---------------- | ----------------------- |
| ArrowRight / Up  | Increment by step       |
| ArrowLeft / Down | Decrement by step       |
| Shift+Arrow      | Use large_step          |
| Home             | Set to min              |
| End              | Set to max              |
| PageUp           | Increment by large_step |
| PageDown         | Decrement by large_step |

> RTL (horizontal): ArrowLeft increments, ArrowRight decrements. Vertical: ArrowUp always increments.

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Label for the slider. Default: `"Color channel"`.
    pub label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Formats the `aria-valuetext`. Arguments: `reading` (channel-aware, e.g.
    /// `"hue 180°"`) and `color_name` (the perceptual color name), plus `locale`.
    pub value_text: MessageFn<dyn Fn(&str, &str, &Locale) -> String + Send + Sync>,
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
```

| Key                       | Default (en-US)                                                    | Purpose              |
| ------------------------- | ------------------------------------------------------------------ | -------------------- |
| `color_slider.label`      | `"Color channel"` (per-instance)                                   | Thumb aria-label     |
| `color_slider.value_text` | `"{reading}, {color_name}"` (e.g. `"hue 180°, dark vibrant blue"`) | Thumb aria-valuetext |

- **RTL**: Horizontal slider direction flips; ArrowLeft increments, ArrowRight decrements.
- **Number formatting**: Channel values respect locale decimal separators.
- **Vertical orientation**: ArrowUp increments, ArrowDown decrements (consistent in LTR and RTL).

## 5. Library Parity

> Compared against: Ark UI (`ColorPicker` channel slider sub-parts), React Aria (`ColorSlider`).

### 5.1 Props

| Feature                  | ars-ui                    | Ark UI                    | React Aria               | Notes                                          |
| ------------------------ | ------------------------- | ------------------------- | ------------------------ | ---------------------------------------------- |
| `channel`                | `channel`                 | `channel`                 | `channel`                | Equivalent                                     |
| `value` / `defaultValue` | `value` / `default_value` | (root-level)              | `value` / `defaultValue` | Equivalent                                     |
| `colorSpace`             | (via channel)             | --                        | `colorSpace`             | React Aria infers channels; ars-ui is explicit |
| `orientation`            | `orientation`             | `orientation`             | `orientation`            | Equivalent                                     |
| `isDisabled`             | `disabled`                | `disabled` (root)         | `isDisabled`             | Equivalent                                     |
| `isReadOnly`             | `readonly`                | `readOnly` (root)         | --                       | ars-ui has readonly                            |
| `name`                   | `name`                    | --                        | `name`                   | Equivalent                                     |
| `on_change_end`          | `on_change_end`           | `onValueChangeEnd` (root) | `onChangeEnd`            | Equivalent                                     |

**Gaps:** None.

### 5.2 Anatomy

| Part        | ars-ui        | Ark UI                   | React Aria     | Notes                 |
| ----------- | ------------- | ------------------------ | -------------- | --------------------- |
| Root        | `Root`        | (slider within picker)   | `ColorSlider`  | Equivalent            |
| Label       | `Label`       | `ChannelSliderLabel`     | `Label`        | Equivalent            |
| Track       | `Track`       | `ChannelSliderTrack`     | `SliderTrack`  | Equivalent            |
| Thumb       | `Thumb`       | `ChannelSliderThumb`     | `ColorThumb`   | Equivalent            |
| Output      | `Output`      | `ChannelSliderValueText` | `SliderOutput` | Equivalent            |
| HiddenInput | `HiddenInput` | --                       | --             | ars-ui has form input |

**Gaps:** None.

### 5.3 Events

| Callback         | ars-ui                | Ark UI                    | React Aria    | Notes      |
| ---------------- | --------------------- | ------------------------- | ------------- | ---------- |
| Value change     | `Bindable` reactivity | `onValueChange` (root)    | `onChange`    | Equivalent |
| Value change end | `on_change_end`       | `onValueChangeEnd` (root) | `onChangeEnd` | Equivalent |

**Gaps:** None.

### 5.4 Features

| Feature              | ars-ui         | Ark UI | React Aria |
| -------------------- | -------------- | ------ | ---------- |
| Vertical orientation | Yes            | Yes    | Yes        |
| Keyboard navigation  | Yes (full)     | Yes    | Yes        |
| RTL support          | Yes            | Yes    | Yes        |
| Gradient track       | Yes (computed) | Yes    | Yes        |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** Same pattern as ColorArea -- Ark embeds within ColorPicker, ars-ui/React Aria standalone.
- **Recommended additions:** None.
