---
component: AngleSlider
category: specialized
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: [color-picker]
references:
    ark-ui: AngleSlider
---

# AngleSlider

A circular/radial slider for selecting angle values (0-360 degrees), commonly
used in color pickers, rotation controls, and compass-style inputs. The user
drags a thumb around a circular track, where the angular position maps directly
to the output value. Composes with `use_move` (Interactions) for drag tracking.

## 1. State Machine

### 1.1 States

```rust
/// The state of the AngleSlider component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The component is in an idle state.
    Idle,
    /// The component is in a dragging state.
    Dragging,
    /// The component is in a focused state.
    Focused,
}
```

### 1.2 Events

```rust
/// The events for the AngleSlider component.
#[derive(Clone, Debug)]
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
    /// Increase value by step.
    Increment,
    /// Decrease value by step.
    Decrement,
    /// Set value to a specific angle.
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
    /// Keyboard event on the thumb.
    KeyDown {
        /// The key that was pressed.
        key: KeyboardKey,
    },
}
```

### 1.3 Context

```rust
/// The context of the AngleSlider component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current angle value (0.0-360.0).
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
```

### 1.4 Props

```rust
/// The props of the AngleSlider component.
#[derive(Clone, Debug, PartialEq, HasId)]
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
    pub on_change_end: Option<Callback<f64>>,
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
```

### 1.5 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, ComponentIds, AttrMap, Bindable};

/// Compute angle from pointer position relative to the center of the track.
/// Returns degrees with 0 degrees at the top (12 o'clock), increasing clockwise.
fn compute_angle(center: (f64, f64), pointer: (f64, f64)) -> f64 {
    let dx = pointer.0 - center.0;
    let dy = pointer.1 - center.1;
    let radians = dy.atan2(dx);
    let degrees = radians.to_degrees();
    // Normalize to 0..360, with 0 at top (12 o'clock)
    (degrees + 90.0).rem_euclid(360.0)
}

/// Snap an angle to the nearest step.
fn snap_to_step(angle: f64, step: f64) -> f64 {
    (angle / step).round() * step
}

/// Wrap a value into the range [min, max).
fn wrap_value(value: f64, min: f64, max: f64) -> f64 {
    let range = max - min;
    ((value - min).rem_euclid(range)) + min
}

/// The machine for the AngleSlider component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let ids = ComponentIds::from_id(&props.id);
        let locale = env.locale.clone();
        let messages = messages.clone();
        let value = match props.value {
            Some(v) => Bindable::controlled(v),
            None    => Bindable::uncontrolled(props.default_value),
        };

        (State::Idle, Context {
            value,
            step: props.step,
            min: props.min,
            max: props.max,
            disabled: props.disabled,
            readonly: props.readonly,
            focused: false,
            focus_visible: false,
            locale,
            messages,
            ids,
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // Disabled blocks all except Focus/Blur.
        if ctx.disabled {
            match event {
                Event::Focus { is_keyboard } => {
                    let ik = *is_keyboard;
                    return Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                        ctx.focused = true;
                        ctx.focus_visible = ik;
                    }));
                }
                Event::Blur => {
                    return Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                        ctx.focused = false;
                        ctx.focus_visible = false;
                    }));
                }
                _ => return None,
            }
        }

        // Readonly blocks drag and value changes.
        if ctx.readonly {
            match event {
                Event::Focus { is_keyboard } => {
                    let ik = *is_keyboard;
                    return Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                        ctx.focused = true;
                        ctx.focus_visible = ik;
                    }));
                }
                Event::Blur => {
                    return Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                        ctx.focused = false;
                        ctx.focus_visible = false;
                    }));
                }
                _ => return None,
            }
        }

        match (state, event) {
            // Drag lifecycle
            (State::Idle | State::Focused, Event::DragStart { angle }) => {
                let snapped = snap_to_step(*angle, ctx.step).clamp(ctx.min, ctx.max);
                Some(TransitionPlan::to(State::Dragging).apply(move |ctx| {
                    ctx.value.set(snapped);
                }))
            }
            (State::Dragging, Event::DragMove { angle }) => {
                let snapped = snap_to_step(*angle, ctx.step).clamp(ctx.min, ctx.max);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(snapped);
                }))
            }
            (State::Dragging, Event::DragEnd) => {
                let final_value = *ctx.value.get();
                let next_state = if ctx.focused { State::Focused } else { State::Idle };
                Some(TransitionPlan::to(next_state)
                    .with_effect(PendingEffect::new("on-change-end", move |_ctx, props, _send| {
                        if let Some(ref cb) = props.on_change_end {
                            cb.call(final_value);
                        }
                        no_cleanup()
                    })))
            }

            // Focus lifecycle
            (_, Event::Focus { is_keyboard }) => {
                let ik = *is_keyboard;
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = ik;
                }))
            }
            (_, Event::Blur) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }))
            }

            // Value adjustments
            (_, Event::Increment) => {
                let new_val = wrap_value(ctx.value.get() + ctx.step, ctx.min, ctx.max);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(new_val);
                }))
            }
            (_, Event::Decrement) => {
                let new_val = wrap_value(ctx.value.get() - ctx.step, ctx.min, ctx.max);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(new_val);
                }))
            }
            (_, Event::SetValue { angle }) => {
                let clamped = angle.clamp(ctx.min, ctx.max);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(clamped);
                }))
            }

            // Keyboard
            (State::Focused, Event::KeyDown { key }) => {
                match key {
                    KeyboardKey::ArrowRight | KeyboardKey::ArrowUp => {
                        let new_val = wrap_value(ctx.value.get() + ctx.step, ctx.min, ctx.max);
                        Some(TransitionPlan::context_only(move |ctx| {
                            ctx.value.set(new_val);
                        }))
                    }
                    KeyboardKey::ArrowLeft | KeyboardKey::ArrowDown => {
                        let new_val = wrap_value(ctx.value.get() - ctx.step, ctx.min, ctx.max);
                        Some(TransitionPlan::context_only(move |ctx| {
                            ctx.value.set(new_val);
                        }))
                    }
                    KeyboardKey::Home => {
                        Some(TransitionPlan::context_only(|ctx| {
                            ctx.value.set(ctx.min);
                        }))
                    }
                    KeyboardKey::End => {
                        Some(TransitionPlan::context_only(|ctx| {
                            ctx.value.set(ctx.max);
                        }))
                    }
                    KeyboardKey::PageUp => {
                        let large_step = ctx.step * 10.0;
                        let new_val = wrap_value(ctx.value.get() + large_step, ctx.min, ctx.max);
                        Some(TransitionPlan::context_only(move |ctx| {
                            ctx.value.set(new_val);
                        }))
                    }
                    KeyboardKey::PageDown => {
                        let large_step = ctx.step * 10.0;
                        let new_val = wrap_value(ctx.value.get() - large_step, ctx.min, ctx.max);
                        Some(TransitionPlan::context_only(move |ctx| {
                            ctx.value.set(new_val);
                        }))
                    }
                    _ => None,
                }
            }

            _ => None,
        }
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
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "angle-slider"]
pub enum Part {
    Root,
    Control,
    Track,
    Range,
    Thumb,
    ValueText,
    MarkerGroup,
    Marker { value: f64 },
    HiddenInput,
}

pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Current angle value.
    pub fn value(&self) -> f64 {
        self.ctx.value.get()
    }

    /// Set the angle value programmatically.
    pub fn set_value(&self, angle: f64) {
        (self.send)(Event::SetValue { angle });
    }

    /// Whether the slider is currently being dragged.
    pub fn is_dragging(&self) -> bool {
        *self.state == State::Dragging
    }

    /// Whether the slider is focused.
    pub fn is_focused(&self) -> bool {
        self.ctx.focused
    }

    /// Formatted value text (e.g., "45 degrees").
    pub fn formatted_value(&self) -> String {
        (self.ctx.messages.value_text)(self.value(), &self.ctx.locale)
    }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        attrs.set(HtmlAttr::Role, "group");
        let state_str = match self.state {
            State::Idle     => "idle",
            State::Dragging => "dragging",
            State::Focused  => "focused",
        };
        attrs.set(HtmlAttr::Data("ars-state"), state_str);
        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
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

    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn track_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Track.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn range_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Range.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn thumb_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Thumb.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "slider");
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), self.value().to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), self.ctx.min.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), self.ctx.max.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), self.formatted_value());
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.label)(&self.ctx.locale));
        // CSS custom property for thumb rotation.
        attrs.set_style(CssProperty::Custom("ars-angle-value"), format!("{}", self.value()));
        attrs.set_style(CssProperty::Custom("ars-angle-thumb-rotation"),
            format!("{}deg", self.value()));
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

    pub fn value_text_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ValueText.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "off");
        attrs
    }

    pub fn marker_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::MarkerGroup.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "presentation");
        attrs
    }

    pub fn marker_attrs(&self, value: f64) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Marker { value }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set_style(
            CssProperty::Custom("ars-angle-marker-rotation"),
            format!("{value}deg")
        );
        attrs
    }

    /// Handle keydown events on the thumb element.
    pub fn on_thumb_keydown(&self, key: KeyboardKey) {
        (self.send)(Event::KeyDown { key });
    }

    /// Handle pointer down on the track. `angle` is computed by the adapter
    /// from pointer coordinates via `compute_angle()`.
    pub fn on_track_pointer_down(&self, angle: f64) {
        (self.send)(Event::DragStart { angle });
    }

    /// Attributes for the hidden input element (form submission).
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(ref name) = self.props.name {
            attrs.set(HtmlAttr::Name, name);
        }
        attrs.set(HtmlAttr::Value, self.ctx.value.get().to_string());
        if let Some(ref form) = self.props.form {
            attrs.set(HtmlAttr::Form, form);
        }
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
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
```

## 2. Anatomy

```text
AngleSlider
├── Root             (required -- <div>, role="group")
├── Control          (required -- <div>, wraps track + thumb)
│   ├── Track        (required -- <div>, circular background)
│   ├── Range        (optional -- <div>, filled arc indicator)
│   └── Thumb        (required -- <div>, role="slider", positioned by angle)
├── ValueText        (optional -- <output>, current angle display)
├── MarkerGroup      (optional -- <div>)
│   └── Marker*      (optional -- <div>, positioned by angle)
└── HiddenInput      (optional -- <input type="hidden">, form submission)
```

| Part        | Element                 | Key Attributes                                           |
| ----------- | ----------------------- | -------------------------------------------------------- |
| Root        | `<div>`                 | `role="group"`, `data-ars-state`, `data-ars-disabled`    |
| Control     | `<div>`                 | Wraps track and thumb                                    |
| Track       | `<div>`                 | Circular track background                                |
| Range       | `<div>`                 | Filled arc indicator showing current value               |
| Thumb       | `<div>`                 | `role="slider"`, `aria-valuenow/min/max`, `tabindex="0"` |
| ValueText   | `<output>`              | `aria-live="off"`, displays formatted angle              |
| MarkerGroup | `<div>`                 | `role="presentation"`, container for markers             |
| Marker      | `<div>`                 | Positioned by angle via CSS custom property              |
| HiddenInput | `<input type="hidden">` | Form submission value                                    |

**9 parts total.**

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Attribute                         | Element | Value                                |
| --------------------------------- | ------- | ------------------------------------ |
| `role="group"`                    | Root    | Groups slider components             |
| `role="slider"`                   | Thumb   | Standard 1D ARIA slider              |
| `aria-valuenow`                   | Thumb   | Current angle (0-360)                |
| `aria-valuemin` / `aria-valuemax` | Thumb   | `"0"` / `"360"`                      |
| `aria-valuetext`                  | Thumb   | Formatted angle (e.g., "45 degrees") |
| `aria-label`                      | Thumb   | From messages (default: "Angle")     |
| `tabindex="0"`                    | Thumb   | Focusable                            |
| `data-ars-focus-visible`          | Thumb   | When keyboard-focused                |

No `aria-orientation` -- circular geometry has no h/v distinction.
Arrow keys do NOT flip for RTL -- angular direction is universal.

### 3.2 Keyboard Interaction

| Key                   | Action                   |
| --------------------- | ------------------------ |
| ArrowRight / ArrowUp  | Increment angle by step  |
| ArrowLeft / ArrowDown | Decrement angle by step  |
| Home                  | Set to min (0 degrees)   |
| End                   | Set to max (360 degrees) |
| PageUp                | Increment by step x 10   |
| PageDown              | Decrement by step x 10   |

## 4. Internationalization

### 4.1 Messages

```rust
/// The messages for the AngleSlider component.
#[derive(Clone)]
pub struct Messages {
    /// Value text formatter. Receives the current angle as f64.
    pub value_text: MessageFn<dyn Fn(f64, &Locale) -> String + Send + Sync>,
    /// Accessible label for the slider.
    pub label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            value_text: MessageFn::new(|v, _locale| format!("{} degrees", v as i32)),
            label: MessageFn::static_str("Angle"),
        }
    }
}

impl ComponentMessages for Messages {}
```

| Key                       | Default (en-US)     | Purpose              |
| ------------------------- | ------------------- | -------------------- |
| `angle_slider.label`      | `"Angle"`           | Thumb aria-label     |
| `angle_slider.value_text` | `"{value} degrees"` | Thumb aria-valuetext |

## 5. Library Parity

> Compared against: Ark UI (`AngleSlider`).

### 5.1 Props

| Feature                  | ars-ui                    | Ark UI                   | Notes                        |
| ------------------------ | ------------------------- | ------------------------ | ---------------------------- |
| `value` / `defaultValue` | `value` / `default_value` | `value` / `defaultValue` | Equivalent                   |
| `step`                   | `step`                    | `step`                   | Equivalent                   |
| `disabled`               | `disabled`                | `disabled`               | Equivalent                   |
| `readOnly`               | `readonly`                | `readOnly`               | Equivalent                   |
| `invalid`                | --                        | `invalid`                | Ark-only; form-level concern |
| `name`                   | `name`                    | `name`                   | Equivalent                   |
| `form`                   | `form`                    | --                       | ars-ui has form association  |
| `on_change_end`          | `on_change_end`           | `onValueChangeEnd`       | Equivalent                   |
| `min` / `max`            | `min` / `max`             | --                       | ars-ui supports custom range |

**Gaps:** None. `invalid` is a form-level concern.

### 5.2 Anatomy

| Part        | ars-ui        | Ark UI        | Notes                             |
| ----------- | ------------- | ------------- | --------------------------------- |
| Root        | `Root`        | `Root`        | Equivalent                        |
| Control     | `Control`     | `Control`     | Equivalent                        |
| Track       | `Track`       | --            | ars-ui has explicit track         |
| Range       | `Range`       | --            | ars-ui has filled range indicator |
| Thumb       | `Thumb`       | `Thumb`       | Equivalent                        |
| ValueText   | `ValueText`   | `ValueText`   | Equivalent                        |
| MarkerGroup | `MarkerGroup` | `MarkerGroup` | Equivalent                        |
| Marker      | `Marker`      | `Marker`      | Equivalent                        |
| HiddenInput | `HiddenInput` | `HiddenInput` | Equivalent                        |
| Label       | --            | `Label`       | Ark has label part                |

**Gaps:** None. ars-ui has a `Root` that serves as the group container; label text is provided by the adapter.

### 5.3 Events

| Callback         | ars-ui                | Ark UI             | Notes      |
| ---------------- | --------------------- | ------------------ | ---------- |
| Value change     | `Bindable` reactivity | `onValueChange`    | Equivalent |
| Value change end | `on_change_end`       | `onValueChangeEnd` | Equivalent |

**Gaps:** None.

### 5.4 Features

| Feature                | ars-ui     | Ark UI           |
| ---------------------- | ---------- | ---------------- |
| Circular drag          | Yes        | Yes              |
| Keyboard navigation    | Yes (full) | Yes              |
| Markers                | Yes        | Yes              |
| Step snapping          | Yes        | Yes              |
| Value wrapping         | Yes        | Yes              |
| Custom range (min/max) | Yes        | No (fixed 0-360) |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui supports custom `min`/`max` range beyond 0-360. Ark UI has a `Label` part; ars-ui treats label text as adapter-provided content.
- **Recommended additions:** None.
