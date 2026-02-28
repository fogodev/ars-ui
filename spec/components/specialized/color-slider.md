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
}
```

### 1.3 Context

```rust
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
    /// Locale override. When `None`, resolved via `resolve_locale()`.
    pub locale: Option<Locale>,
    /// Translatable messages. When `None`, resolved via `resolve_messages()`.
    pub messages: Option<Messages>,
    /// Fired on `Event::DragEnd` / pointer release.
    pub on_change_end: Option<Callback<ColorValue>>,
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
            locale: None,
            messages: None,
            on_change_end: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
/// Apply a normalized position (0..1) to the channel value.
fn apply_slider_position(ctx: &mut Context, position: f64) {
    let color = ctx.value.get();
    let (min, max) = channel_range(ctx.channel);
    let value = min + position.clamp(0.0, 1.0) * (max - min);
    ctx.value.set(with_channel(color, ctx.channel, value));
}

pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let value = match &props.value {
            Some(v) => Bindable::controlled(v.clone()),
            None => Bindable::uncontrolled(props.default_value.clone()),
        };
        let ids = ComponentIds::from_id(&props.id);
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);

        (State::Idle, Context {
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
            locale,
            messages,
            ids,
        })
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        _props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled {
            return match event {
                Event::Focus { is_keyboard } => {
                    let kb = *is_keyboard;
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.focused = true;
                        ctx.focus_visible = kb;
                    }))
                }
                Event::Blur => Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                })),
                _ => None,
            };
        }

        match (state, event) {
            (State::Idle, Event::DragStart { position }) => {
                if ctx.readonly { return None; }
                let pos = *position;
                Some(TransitionPlan::to(State::Dragging).apply(move |ctx| {
                    apply_slider_position(ctx, pos);
                }).with_named_effect("drag-listeners", move |_ctx, _props, send| {
                    let platform = use_platform_effects();
                    let send_move = send.clone();
                    let send_up = send.clone();
                    platform.track_pointer_drag(
                        Box::new(move |x, y| { send_move.call_if_alive(Event::DragMove { position: x }); }),
                        Box::new(move || { send_up.call_if_alive(Event::DragEnd); }),
                    )
                }))
            }

            (State::Dragging, Event::DragMove { position }) => {
                let pos = *position;
                Some(TransitionPlan::context_only(move |ctx| {
                    apply_slider_position(ctx, pos);
                }))
            }

            (State::Dragging, Event::DragEnd) => {
                Some(TransitionPlan::to(State::Idle))
            }

            (_, Event::Increment { step }) => {
                if ctx.readonly { return None; }
                let step = *step;
                Some(TransitionPlan::context_only(move |ctx| {
                    let color = ctx.value.get();
                    let current = channel_value(color, ctx.channel);
                    let (_, max) = channel_range(ctx.channel);
                    ctx.value.set(with_channel(color, ctx.channel, (current + step).min(max)));
                }))
            }

            (_, Event::Decrement { step }) => {
                if ctx.readonly { return None; }
                let step = *step;
                Some(TransitionPlan::context_only(move |ctx| {
                    let color = ctx.value.get();
                    let current = channel_value(color, ctx.channel);
                    let (min, _) = channel_range(ctx.channel);
                    ctx.value.set(with_channel(color, ctx.channel, (current - step).max(min)));
                }))
            }

            (_, Event::SetToMin) => {
                if ctx.readonly { return None; }
                Some(TransitionPlan::context_only(|ctx| {
                    let color = ctx.value.get();
                    let (min, _) = channel_range(ctx.channel);
                    ctx.value.set(with_channel(color, ctx.channel, min));
                }))
            }

            (_, Event::SetToMax) => {
                if ctx.readonly { return None; }
                Some(TransitionPlan::context_only(|ctx| {
                    let color = ctx.value.get();
                    let (_, max) = channel_range(ctx.channel);
                    ctx.value.set(with_channel(color, ctx.channel, max));
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

            _ => None,
        }
    }

    fn connect<'a>(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api { state, ctx, props, send }
    }
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
        let color = self.ctx.value.get();
        let val = channel_value(color, self.ctx.channel);
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
        let color = self.ctx.value.get();
        let gradient = match self.ctx.channel {
            ColorChannel::Hue => "linear-gradient(to right, \
                hsl(0,100%,50%), hsl(60,100%,50%), hsl(120,100%,50%), \
                hsl(180,100%,50%), hsl(240,100%,50%), hsl(300,100%,50%), \
                hsl(360,100%,50%))".to_string(),
            ColorChannel::Alpha => format!(
                "linear-gradient(to right, transparent, {})",
                ColorValue::new(color.hue, color.saturation, color.lightness, 1.0).to_css_hsl()
            ),
            _ => {
                let (min, max) = channel_range(self.ctx.channel);
                let start = with_channel(color, self.ctx.channel, min);
                let end = with_channel(color, self.ctx.channel, max);
                format!("linear-gradient(to right, {}, {})", start.to_css_hsl(), end.to_css_hsl())
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
        attrs.set(HtmlAttr::TabIndex, "0");

        let color = self.ctx.value.get();
        let val = channel_value(color, self.ctx.channel);
        let (min, max) = channel_range(self.ctx.channel);

        attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), format!("{:.2}", val));
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), format!("{:.2}", min));
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), format!("{:.2}", max));
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation),
            if self.ctx.orientation == Orientation::Vertical { "vertical" } else { "horizontal" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.label)(&self.ctx.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), (self.ctx.messages.value_text)(val, &self.ctx.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));

        let pct = if (max - min).abs() > f64::EPSILON { (val - min) / (max - min) * 100.0 } else { 0.0 };
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
        attrs.set(HtmlAttr::Value, self.ctx.value.get().to_hex(true));
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

| Part        | Element    | Key Attributes                                                               |
| ----------- | ---------- | ---------------------------------------------------------------------------- |
| Root        | `<div>`    | `role="group"`, `data-ars-channel`, `data-ars-orientation`                   |
| Label       | `<label>`  | `id`, `for` (thumb id)                                                       |
| Track       | `<div>`    | gradient background via CSS custom property                                  |
| Thumb       | `<div>`    | `role="slider"`, `aria-valuenow/min/max`, `aria-orientation`, `tabindex="0"` |
| Output      | `<output>` | `for` (thumb id), `aria-live="off"`                                          |
| HiddenInput | `<input>`  | `type="hidden"`, `name`, `value` (hex color)                                 |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part   | Role     | Properties                                                                                                               |
| ------ | -------- | ------------------------------------------------------------------------------------------------------------------------ |
| Root   | `group`  | groups slider components                                                                                                 |
| Thumb  | `slider` | `aria-valuenow`, `aria-valuemin`, `aria-valuemax`, `aria-orientation`, `aria-label`, `aria-valuetext`, `aria-labelledby` |
| Output | —        | `aria-live="off"` (prevents double-announcement with valuetext)                                                          |

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
    /// Formats the channel value for aria-valuetext.
    pub value_text: MessageFn<dyn Fn(f64, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            label: MessageFn::static_str("Color channel"),
            value_text: MessageFn::new(|val, _locale| format!("{val:.0}")),
        }
    }
}

impl ComponentMessages for Messages {}
```

| Key                       | Default (en-US)                  | Purpose              |
| ------------------------- | -------------------------------- | -------------------- |
| `color_slider.label`      | `"Color channel"` (per-instance) | Thumb aria-label     |
| `color_slider.value_text` | Channel-specific formatting      | Thumb aria-valuetext |

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
