---
component: ColorArea
category: specialized
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: [color-picker, color-slider]
references:
    ark-ui: ColorPicker
    react-aria: ColorArea
---

# ColorArea

A standalone 2D color area that adjusts two channels simultaneously. The user
specifies `x_channel` and `y_channel` (e.g., Saturation/Lightness for HSL, or
Saturation/Brightness for HSB). Can be used independently outside of ColorPicker.
Reuses `ColorValue`, `ColorChannel`, and the shared helpers (`channel_value`,
`with_channel`, `channel_range`, `channel_step_default`) from the color types.

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
    /// User started dragging (normalized x/y in 0..1 relative to the area).
    DragStart { x: f64, y: f64 },
    /// User is moving while dragging.
    DragMove { x: f64, y: f64 },
    /// User released the drag.
    DragEnd,
    /// Increment x_channel by step.
    IncrementX { step: f64 },
    /// Decrement x_channel by step.
    DecrementX { step: f64 },
    /// Increment y_channel by step.
    IncrementY { step: f64 },
    /// Decrement y_channel by step.
    DecrementY { step: f64 },
    /// Snap x_channel to its minimum.
    SetXToMin,
    /// Snap x_channel to its maximum.
    SetXToMax,
    /// Snap y_channel to its minimum.
    SetYToMin,
    /// Snap y_channel to its maximum.
    SetYToMax,
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
    /// Whether focus was via keyboard (for focus-visible ring).
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
    pub on_change_end: Option<Callback<dyn Fn(ColorValue) + Send + Sync>>,
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
```

### 1.5 Full Machine Implementation

```rust,no_check
/// Apply normalized (x, y) coordinates to both channels.
fn apply_area_position(ctx: &mut Context, x: f64, y: f64) {
    let color = *ctx.value.get();
    let (x_min, x_max) = channel_range(ctx.x_channel);
    let (y_min, y_max) = channel_range(ctx.y_channel);
    let x_val = x_min + x.clamp(0.0, 1.0) * (x_max - x_min);
    // y is inverted: top=max, bottom=min
    let y_val = y_max - y.clamp(0.0, 1.0) * (y_max - y_min);
    let updated = with_channel(&color, ctx.x_channel, x_val);
    ctx.value.set(with_channel(&updated, ctx.y_channel, y_val));
}

/// Typed identifier for side effects emitted by the machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Invoke `Props::on_change_end`.
    ChangeEnd,
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
        let ids = ComponentIds::from_id(&props.id);
        let locale = env.locale.clone();
        let messages = messages.clone();

        (State::Idle, Context {
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
        // A disabled area ignores value-changing input but still tracks focus
        // and accepts parent-driven prop syncs (so it can be re-enabled).
        if ctx.disabled {
            match event {
                Event::DragStart { .. }
                | Event::DragMove { .. }
                | Event::DragEnd
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
            // The adapter resolves normalized (x, y) and drives DragMove/DragEnd
            // from its own pointer listeners; the core only updates the value.
            (State::Idle, Event::DragStart { x, y }) => {
                if ctx.readonly { return None; }
                let x = *x;
                let y = *y;
                Some(TransitionPlan::to(State::Dragging).apply(move |ctx| {
                    apply_area_position(ctx, x, y);
                }))
            }

            (State::Dragging, Event::DragMove { x, y }) => {
                let x = *x;
                let y = *y;
                Some(TransitionPlan::context_only(move |ctx| {
                    apply_area_position(ctx, x, y);
                }))
            }

            (State::Dragging, Event::DragEnd) => {
                Some(TransitionPlan::to(State::Idle).with_effect(change_end_effect()))
            }

            (_, Event::IncrementX { step }) => {
                if ctx.readonly { return None; }
                let step = *step;
                Some(TransitionPlan::context_only(move |ctx| {
                    let color = ctx.value.get();
                    let current = channel_value(color, ctx.x_channel);
                    let (_, max) = channel_range(ctx.x_channel);
                    ctx.value.set(with_channel(color, ctx.x_channel, (current + step).min(max)));
                }))
            }

            (_, Event::DecrementX { step }) => {
                if ctx.readonly { return None; }
                let step = *step;
                Some(TransitionPlan::context_only(move |ctx| {
                    let color = ctx.value.get();
                    let current = channel_value(color, ctx.x_channel);
                    let (min, _) = channel_range(ctx.x_channel);
                    ctx.value.set(with_channel(color, ctx.x_channel, (current - step).max(min)));
                }))
            }

            (_, Event::IncrementY { step }) => {
                if ctx.readonly { return None; }
                let step = *step;
                Some(TransitionPlan::context_only(move |ctx| {
                    let color = ctx.value.get();
                    let current = channel_value(color, ctx.y_channel);
                    let (_, max) = channel_range(ctx.y_channel);
                    ctx.value.set(with_channel(color, ctx.y_channel, (current + step).min(max)));
                }))
            }

            (_, Event::DecrementY { step }) => {
                if ctx.readonly { return None; }
                let step = *step;
                Some(TransitionPlan::context_only(move |ctx| {
                    let color = ctx.value.get();
                    let current = channel_value(color, ctx.y_channel);
                    let (min, _) = channel_range(ctx.y_channel);
                    ctx.value.set(with_channel(color, ctx.y_channel, (current - step).max(min)));
                }))
            }

            (_, Event::SetXToMin) => {
                if ctx.readonly { return None; }
                Some(TransitionPlan::context_only(|ctx| {
                    let color = ctx.value.get();
                    let (min, _) = channel_range(ctx.x_channel);
                    ctx.value.set(with_channel(color, ctx.x_channel, min));
                }))
            }

            (_, Event::SetXToMax) => {
                if ctx.readonly { return None; }
                Some(TransitionPlan::context_only(|ctx| {
                    let color = ctx.value.get();
                    let (_, max) = channel_range(ctx.x_channel);
                    ctx.value.set(with_channel(color, ctx.x_channel, max));
                }))
            }

            (_, Event::SetYToMin) => {
                if ctx.readonly { return None; }
                Some(TransitionPlan::context_only(|ctx| {
                    let color = ctx.value.get();
                    let (min, _) = channel_range(ctx.y_channel);
                    ctx.value.set(with_channel(color, ctx.y_channel, min));
                }))
            }

            (_, Event::SetYToMax) => {
                if ctx.readonly { return None; }
                Some(TransitionPlan::context_only(|ctx| {
                    let color = ctx.value.get();
                    let (_, max) = channel_range(ctx.y_channel);
                    ctx.value.set(with_channel(color, ctx.y_channel, max));
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
                        ctx.value.set(color);
                        ctx.value.sync_controlled(Some(color));
                    }
                    None => ctx.value.sync_controlled(None),
                }))
            }

            (_, Event::SetProps) => {
                let props = props.clone();
                Some(TransitionPlan::context_only(move |ctx| {
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
        Api { state, ctx, props, send }
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
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "color-area"]
pub enum Part {
    Root,
    Background,
    Thumb,
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

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "group");
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

    pub fn background_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Background.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let color = self.ctx.value.get();
        let bg = format!("hsl({:.0}, 100%, 50%)", color.hue);
        attrs.set_style(CssProperty::Custom("ars-color-area-bg"), bg);
        attrs
    }

    pub fn thumb_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Thumb.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("thumb"));
        attrs.set(HtmlAttr::Role, "application");
        attrs.set(HtmlAttr::Aria(AriaAttr::RoleDescription), (self.ctx.messages.role_description)(&self.ctx.locale));
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.label)(&self.ctx.locale));

        let color = self.ctx.value.get();
        let x_val = channel_value(color, self.ctx.x_channel);
        let y_val = channel_value(color, self.ctx.y_channel);
        let (x_min, x_max) = channel_range(self.ctx.x_channel);
        let (y_min, y_max) = channel_range(self.ctx.y_channel);

        let x_name = format!("{:?}", self.ctx.x_channel).to_lowercase();
        let y_name = format!("{:?}", self.ctx.y_channel).to_lowercase();
        let color_name = color.color_name_en();
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueText),
            (self.ctx.messages.value_text)(x_val, y_val, &x_name, &y_name, &color_name, &self.ctx.locale));

        let x_pct = if (x_max - x_min).abs() > f64::EPSILON {
            (x_val - x_min) / (x_max - x_min) * 100.0
        } else { 0.0 };
        let y_pct = if (y_max - y_min).abs() > f64::EPSILON {
            (1.0 - (y_val - y_min) / (y_max - y_min)) * 100.0
        } else { 0.0 };
        attrs.set_style(CssProperty::Custom("ars-color-area-thumb-x"), format!("{:.1}%", x_pct));
        attrs.set_style(CssProperty::Custom("ars-color-area-thumb-y"), format!("{:.1}%", y_pct));
        attrs.set_style(CssProperty::BackgroundColor, color.to_css_hsl());

        attrs.set(HtmlAttr::Aria(AriaAttr::KeyShortcuts), "ArrowUp ArrowDown ArrowLeft ArrowRight");
        if self.is_dragging() {
            attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
        }
        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
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
        // A disabled control must be omitted from form submission.
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        attrs
    }

    pub fn on_thumb_keydown(&self, data: &KeyboardEventData, shift: bool) {
        let step = if shift { self.ctx.large_step } else { self.ctx.step };
        match data.key {
            KeyboardKey::ArrowRight => (self.send)(Event::IncrementX { step }),
            KeyboardKey::ArrowLeft => (self.send)(Event::DecrementX { step }),
            KeyboardKey::ArrowUp => (self.send)(Event::IncrementY { step }),
            KeyboardKey::ArrowDown => (self.send)(Event::DecrementY { step }),
            KeyboardKey::Home => (self.send)(Event::SetXToMin),
            KeyboardKey::End => (self.send)(Event::SetXToMax),
            KeyboardKey::PageUp => (self.send)(Event::SetYToMax),
            KeyboardKey::PageDown => (self.send)(Event::SetYToMin),
            _ => {}
        }
    }

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
```

## 2. Anatomy

```text
ColorArea
├── Root         (required — <div>, role="group")
├── Background   (required — <div>, gradient background)
├── Thumb        (required — <div>, role="application", focusable)
└── HiddenInput  (optional — <input type="hidden">, form submission)
```

| Part        | Element   | Key Attributes                                                                 |
| ----------- | --------- | ------------------------------------------------------------------------------ |
| Root        | `<div>`   | `role="group"`, `data-ars-disabled`, `data-ars-dragging`                       |
| Background  | `<div>`   | gradient background via CSS custom property                                    |
| Thumb       | `<div>`   | `role="application"`, `aria-roledescription`, `aria-valuetext`, `tabindex="0"` |
| HiddenInput | `<input>` | `type="hidden"`, `name`, `value` (hex color), `disabled` (when disabled — omitted from form submission) |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part  | Role          | Properties                                                                                    |
| ----- | ------------- | --------------------------------------------------------------------------------------------- |
| Root  | `group`       | groups area components                                                                        |
| Thumb | `application` | `aria-roledescription="2d color picker"`, `aria-valuetext`, `aria-label`, `aria-keyshortcuts` |

`aria-valuetext` MUST include a human-readable color name from `color_name_parts()`, not raw numeric values (e.g., `"dark vibrant blue, saturation 80%, lightness 50%"`).

### 3.2 Keyboard Interaction

| Key         | Action               |
| ----------- | -------------------- |
| ArrowRight  | Increment x_channel  |
| ArrowLeft   | Decrement x_channel  |
| ArrowUp     | Increment y_channel  |
| ArrowDown   | Decrement y_channel  |
| Shift+Arrow | Use large_step       |
| Home        | Set x_channel to min |
| End         | Set x_channel to max |
| PageUp      | Set y_channel to max |
| PageDown    | Set y_channel to min |

> RTL: ArrowLeft increments, ArrowRight decrements on the x-axis.

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Label for the area thumb. Default: `"Color area"`.
    pub label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Role description for screen readers. Default: `"2d color picker"`.
    pub role_description: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Formats both channel values plus the perceptual color name for
    /// `aria-valuetext`. Arguments: `x_value`, `y_value`, `x_channel_name`,
    /// `y_channel_name`, `color_name`, `locale`.
    pub value_text: MessageFn<dyn Fn(f64, f64, &str, &str, &str, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            label: MessageFn::static_str("Color area"),
            role_description: MessageFn::static_str("2d color picker"),
            value_text: MessageFn::new(
                |x_value: f64,
                 y_value: f64,
                 x_name: &str,
                 y_name: &str,
                 color_name: &str,
                 _locale: &Locale| {
                    format!(
                        "{}, {} {:.0}%, {} {:.0}%",
                        color_name,
                        x_name,
                        x_value * 100.0,
                        y_name,
                        y_value * 100.0
                    )
                },
            ),
        }
    }
}

impl ComponentMessages for Messages {}
```

| Key                           | Default (en-US)                        | Purpose                    |
| ----------------------------- | -------------------------------------- | -------------------------- |
| `color_area.label`            | `"Color area"`                         | Thumb aria-label           |
| `color_area.role_description` | `"2d color picker"`                    | Thumb aria-roledescription |
| `color_area.value_text`       | `"{color_name}, {x_channel} {x}%, {y_channel} {y}%"` | Thumb aria-valuetext       |

- **RTL**: x-axis gradient flips horizontally; ArrowLeft increments, ArrowRight decrements.
- **Number formatting**: Channel values respect locale decimal separators.

## 5. Library Parity

> Compared against: Ark UI (`ColorPicker` area sub-parts), React Aria (`ColorArea`).

### 5.1 Props

| Feature                  | ars-ui                    | Ark UI                    | React Aria               | Notes                                                            |
| ------------------------ | ------------------------- | ------------------------- | ------------------------ | ---------------------------------------------------------------- |
| `value` / `defaultValue` | `value` / `default_value` | (root-level)              | `value` / `defaultValue` | Equivalent                                                       |
| `xChannel` / `yChannel`  | `x_channel` / `y_channel` | (implicit S/L)            | `xChannel` / `yChannel`  | Equivalent; ars-ui configurable                                  |
| `colorSpace`             | (via channel selection)   | --                        | `colorSpace`             | React Aria infers from colorSpace; ars-ui uses explicit channels |
| `isDisabled`             | `disabled`                | `disabled` (root)         | `isDisabled`             | Equivalent                                                       |
| `isReadOnly`             | `readonly`                | `readOnly` (root)         | --                       | ars-ui has readonly                                              |
| `name`                   | `name`                    | --                        | `xName` / `yName`        | React Aria has per-axis names                                    |
| `form`                   | --                        | --                        | `form`                   | React Aria has form ID                                           |
| `on_change_end`          | `on_change_end`           | `onValueChangeEnd` (root) | `onChangeEnd`            | Equivalent                                                       |

**Gaps:** None worth adopting. Per-axis form names (`xName`/`yName`) and `form` ID are React Aria specifics that can be handled by the adapter.

### 5.2 Anatomy

| Part        | ars-ui        | Ark UI               | React Aria    | Notes      |
| ----------- | ------------- | -------------------- | ------------- | ---------- |
| Root        | `Root`        | (area within picker) | `ColorArea`   | Equivalent |
| Background  | `Background`  | `AreaBackground`     | --            | Equivalent |
| Thumb       | `Thumb`       | `AreaThumb`          | `ColorThumb`  | Equivalent |
| HiddenInput | `HiddenInput` | --                   | (form inputs) | Equivalent |

**Gaps:** None.

### 5.3 Events

| Callback         | ars-ui                | Ark UI                    | React Aria    | Notes      |
| ---------------- | --------------------- | ------------------------- | ------------- | ---------- |
| Value change     | `Bindable` reactivity | `onValueChange` (root)    | `onChange`    | Equivalent |
| Value change end | `on_change_end`       | `onValueChangeEnd` (root) | `onChangeEnd` | Equivalent |

**Gaps:** None.

### 5.4 Features

| Feature               | ars-ui                              | Ark UI         | React Aria |
| --------------------- | ----------------------------------- | -------------- | ---------- |
| Configurable channels | Yes                                 | No (fixed S/L) | Yes        |
| Keyboard navigation   | Yes (arrows, Home/End, PageUp/Down) | Yes            | Yes        |
| RTL support           | Yes                                 | Yes            | Yes        |
| Pointer drag          | Yes                                 | Yes            | Yes        |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** Ark UI embeds the area within a monolithic ColorPicker; ars-ui and React Aria treat it as a standalone component. React Aria uses `colorSpace` prop; ars-ui uses explicit `x_channel`/`y_channel` for more flexibility.
- **Recommended additions:** None.
