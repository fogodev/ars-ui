---
component: ColorWheel
category: specialized
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: [color-picker]
references:
    react-aria: ColorWheel
---

# ColorWheel

A circular hue selector that adjusts the hue channel of a `ColorValue`. The user
drags a thumb around a circular track, where the angular position maps to hue
(0-360). `ColorWheel` is a 1D control like `ColorSlider` but with circular
geometry. It can be composed with `ColorArea` to build a full color picker
where the wheel controls hue and the area controls saturation/lightness.

`ColorWheel` reuses `ColorValue`, `ColorChannel`, and the shared helpers
(`channel_value`, `with_channel`, `channel_range`, `channel_step_default`).
The core machine operates exclusively on the `Hue` channel. Circular geometry
(outer radius, inner radius, atan2 conversion) is an adapter rendering concern
and does not appear in the core types.

## 1. State Machine

### 1.1 States

```rust
/// The states for the `ColorWheel` component.
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
/// The events for the `ColorWheel` component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// User started dragging (normalized position 0..1 along the track).
    DragStart {
        /// The normalized position of the drag.
        position: f64,
    },
    /// User is moving while dragging.
    DragMove {
        /// The normalized position of the drag.
        position: f64,
    },
    /// User released the drag.
    DragEnd,
    /// Increment channel by step.
    Increment {
        /// The step.
        step: f64,
    },
    /// Decrement channel by step.
    Decrement {
        /// The step.
        step: f64,
    },
    /// Snap channel to its minimum.
    /// Home -> 0 degrees
    SetToMin,
    /// Snap channel to its maximum.
    /// End -> 360 degrees
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
```

### 1.3 Context

```rust
/// The context for the `ColorWheel` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The value of the color wheel.
    pub value: Bindable<ColorValue>,
    /// The hue value in degrees, kept *unwrapped*.
    ///
    /// Source of truth for `aria-valuenow`, the value text, and the thumb angle.
    /// [`ColorValue`] normalizes hue into `[0, 360)` (so the stored color never
    /// holds `360`), but the End/`SetToMax` endpoint exposes `360°` here so the
    /// slider value can reach `aria-valuemax`. The derived color stays normalized.
    pub hue_value: f64,
    /// Whether the color wheel is disabled.
    pub disabled: bool,
    /// Whether the color wheel is readonly.
    pub readonly: bool,
    /// Whether the color wheel is focused.
    pub focused: bool,
    /// Whether the color wheel is focused via keyboard.
    pub focus_visible: bool,
    /// The step of the color wheel. (Default: 1.0 degrees)
    pub step: f64,
    /// The large step of the color wheel. (Default: 10.0 degrees)
    pub large_step: f64,
    /// Text direction for RTL-aware keyboard navigation (arrow keys swap direction).
    pub dir: Direction,
    /// Locale for internationalized messages.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// The component IDs.
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
/// The props for the `ColorWheel` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The id of the color wheel.
    pub id: String,
    /// The value of the color wheel.
    pub value: Option<ColorValue>,
    /// The default value of the color wheel.
    pub default_value: ColorValue,
    /// The step of the color wheel. (Default: 1.0 degrees)
    pub step: f64,
    /// The large step of the color wheel. (Default: 10.0 degrees)
    pub large_step: f64,
    /// Whether the color wheel is disabled.
    pub disabled: bool,
    /// Whether the color wheel is readonly.
    pub readonly: bool,
    /// Text direction for RTL-aware keyboard navigation.
    pub dir: Direction,
    /// The name of the color wheel.
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
// `f64::rem_euclid` is std-only; use the libm-backed `core_maths` version so the
// module compiles under `#![no_std]` (matching `ars-core::color`).
use core_maths::CoreFloat;

/// Apply a normalized angle (0..1) to the hue value.
fn apply_wheel_angle(ctx: &mut Context, angle: f64) {
    // A full revolution returns to the top, so drags stay in `[0, 360)`.
    let hue = (angle.clamp(0.0, 1.0) * 360.0) % 360.0;
    set_hue(ctx, hue);
}

/// Set the hue value (kept unwrapped) and derive the normalized stored color.
///
/// [`ColorValue::new`] normalizes the hue into `[0, 360)`, so the stored color
/// never violates that invariant even when `hue` is the `360°` endpoint.
fn set_hue(ctx: &mut Context, hue: f64) {
    ctx.hue_value = hue;
    let color = *ctx.value.get();
    ctx.value.set(ColorValue::new(hue, color.saturation, color.lightness, color.alpha));
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

/// The machine for the `ColorWheel` component.
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

        let hue_value = value.get().hue;
        let ids = ComponentIds::from_id(&props.id);
        let locale = env.locale.clone();
        let messages = messages.clone();

        (State::Idle, Context {
            value,
            hue_value,
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
        // Focus/Blur and parent-driven prop syncs always pass through regardless
        // of disabled/readonly (a disabled wheel must still be re-enableable).
        match event {
            Event::Focus { is_keyboard } => {
                let ik = *is_keyboard;
                return Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = ik;
                }));
            }
            Event::Blur => {
                return Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }));
            }
            Event::SyncValue(value) => {
                let value = *value;
                return Some(TransitionPlan::context_only(move |ctx| match value {
                    Some(color) => {
                        // Keep the cached hue when the parent echoes the value
                        // we emitted, so a 360° endpoint isn't re-derived to 0°
                        // (the stored color normalizes 360° -> 0°).
                        let echoes_pending = color == *ctx.value.pending();
                        ctx.value.set(color);
                        ctx.value.sync_controlled(Some(color));
                        if !echoes_pending {
                            ctx.hue_value = color.hue;
                        }
                    }
                    None => ctx.value.sync_controlled(None),
                }));
            }
            Event::SetProps => {
                let props = props.clone();
                return Some(TransitionPlan::context_only(move |ctx| {
                    ctx.step = props.step;
                    ctx.large_step = props.large_step;
                    ctx.disabled = props.disabled;
                    ctx.readonly = props.readonly;
                    ctx.dir = props.dir;
                }));
            }
            _ => {}
        }

        // Disabled and read-only both block value-changing events, except
        // `DragEnd`: a drag in flight when the control was disabled must still
        // be able to terminate cleanly (exit `Dragging`, fire change-end).
        if (ctx.disabled || ctx.readonly) && !matches!(event, Event::DragEnd) {
            return None;
        }

        match (state, event) {
            (State::Idle, Event::DragStart { position }) => {
                let pos = *position;
                Some(TransitionPlan::to(State::Dragging).apply(move |ctx| {
                    apply_wheel_angle(ctx, pos);
                }))
            }

            (State::Dragging, Event::DragMove { position }) => {
                let pos = *position;
                Some(TransitionPlan::context_only(move |ctx| {
                    apply_wheel_angle(ctx, pos);
                }))
            }

            (State::Dragging, Event::DragEnd) => {
                Some(TransitionPlan::to(State::Idle).with_effect(change_end_effect()))
            }

            (_, Event::Increment { step }) => {
                let s = *step;
                Some(TransitionPlan::context_only(move |ctx| {
                    // `rem_euclid` keeps the hue non-negative for custom steps > 360.
                    set_hue(ctx, CoreFloat::rem_euclid(ctx.hue_value + s, 360.0));
                }))
            }

            (_, Event::Decrement { step }) => {
                let s = *step;
                Some(TransitionPlan::context_only(move |ctx| {
                    // `rem_euclid` keeps the hue non-negative for custom steps > 360.
                    set_hue(ctx, CoreFloat::rem_euclid(ctx.hue_value - s, 360.0));
                }))
            }

            (_, Event::SetToMin) => {
                Some(TransitionPlan::context_only(|ctx| {
                    set_hue(ctx, 0.0);
                }))
            }

            (_, Event::SetToMax) => {
                // 360° is the same ring position as 0°, but exposing it keeps
                // aria-valuenow able to reach aria-valuemax; the stored color
                // hue is normalized to 0° by `set_hue`.
                Some(TransitionPlan::context_only(|ctx| {
                    set_hue(ctx, 360.0);
                }))
            }

            _ => None,
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        assert_eq!(
            old.id, new.id,
            "color_wheel::Props.id must remain stable after init"
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
    (old.step - new.step).abs() > f64::EPSILON
        || (old.large_step - new.large_step).abs() > f64::EPSILON
        || old.disabled != new.disabled
        || old.readonly != new.readonly
        || old.dir != new.dir
}
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "color-wheel"]
pub enum Part {
    Root,
    Track,
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

    /// Current hue formatted for display.
    pub fn formatted_value(&self) -> String {
        (self.ctx.messages.value_text)(self.ctx.hue_value, &self.ctx.locale)
    }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        attrs.set(HtmlAttr::Role, "group");
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.readonly { attrs.set_bool(HtmlAttr::Data("ars-readonly"), true); }
        if self.is_dragging() { attrs.set_bool(HtmlAttr::Data("ars-dragging"), true); }
        attrs
    }

    pub fn track_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Track.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set_style(CssProperty::Custom("ars-color-wheel-track-bg"),
            "conic-gradient(hsl(0,100%,50%), hsl(60,100%,50%), hsl(120,100%,50%), \
             hsl(180,100%,50%), hsl(240,100%,50%), hsl(300,100%,50%), hsl(360,100%,50%))");
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

        // Unwrapped hue so the 360° endpoint reaches aria-valuemax (the stored
        // color is normalized; 360° and 0° are the same ring position).
        let hue = self.ctx.hue_value;
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), format!("{:.0}", hue));
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), "0");
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), "360");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.label)(&self.ctx.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), self.formatted_value());

        attrs.set_style(CssProperty::Custom("ars-color-wheel-thumb-angle"),
            format!("{}deg", hue));

        if self.ctx.focus_visible { attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true); }
        if self.is_dragging() { attrs.set_bool(HtmlAttr::Data("ars-dragging"), true); }
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
        // Pending color so the submitted value matches the in-progress drag in
        // controlled mode (the thumb already tracks the pending hue_value).
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
            KeyboardKey::ArrowRight | KeyboardKey::ArrowUp =>
                (self.send)(Event::Increment { step }),
            KeyboardKey::ArrowLeft | KeyboardKey::ArrowDown =>
                (self.send)(Event::Decrement { step }),
            KeyboardKey::Home => (self.send)(Event::SetToMin),
            KeyboardKey::End => (self.send)(Event::SetToMax),
            KeyboardKey::PageUp =>
                (self.send)(Event::Increment { step: self.ctx.large_step }),
            KeyboardKey::PageDown =>
                (self.send)(Event::Decrement { step: self.ctx.large_step }),
            _ => {}
        }
    }

    pub fn on_track_pointer_down(&self, angle: f64) {
        (self.send)(Event::DragStart { position: angle });
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Track => self.track_attrs(),
            Part::Thumb => self.thumb_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
ColorWheel
├── Root          (required -- <div>, role="group")
├── Track         (required -- <div>, conic-gradient background)
│   └── Thumb     (required -- <div>, role="slider", positioned by angle)
└── HiddenInput   (optional -- <input type="hidden">)
```

| Part        | Element   | Key Attributes                                                                                                           |
| ----------- | --------- | ------------------------------------------------------------------------------------------------------------------------ |
| Root        | `<div>`   | `role="group"`, `data-ars-disabled`, `data-ars-readonly`                                                                 |
| Track       | `<div>`   | conic-gradient background via CSS custom property                                                                        |
| Thumb       | `<div>`   | `role="slider"`, `aria-valuenow/min/max`, `tabindex` (`"-1"` when disabled, else `"0"`), `aria-disabled` (when disabled) |
| HiddenInput | `<input>` | `type="hidden"`, `name`, `value` (hex color), `disabled` (when disabled -- omitted from form submission)                 |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Attribute                         | Element | Value                            |
| --------------------------------- | ------- | -------------------------------- |
| `role="group"`                    | Root    | Groups wheel components          |
| `role="slider"`                   | Thumb   | Standard 1D ARIA slider          |
| `aria-valuenow`                   | Thumb   | Current hue (0-360)              |
| `aria-valuemin` / `aria-valuemax` | Thumb   | `"0"` / `"360"`                  |
| `aria-label`                      | Thumb   | From messages (default: "Hue")   |
| `aria-valuetext`                  | Thumb   | Formatted hue (e.g., "180")      |
| `tabindex`                        | Thumb   | `"-1"` when disabled, else `"0"` |
| `aria-disabled`                   | Thumb   | `"true"` when disabled           |
| `data-ars-focus-visible`          | Thumb   | When keyboard-focused            |

No `aria-orientation` -- circular geometry has no h/v distinction.
Arrow keys do NOT flip for RTL -- angular direction is universal.

### 3.2 Keyboard Interaction

| Key                   | Action                  |
| --------------------- | ----------------------- |
| ArrowRight / ArrowUp  | Increment hue by step   |
| ArrowLeft / ArrowDown | Decrement hue by step   |
| Shift+Arrow           | Use large_step          |
| Home                  | Set to 0 degrees        |
| End                   | Set to 360 degrees      |
| PageUp                | Increment by large_step |
| PageDown              | Decrement by large_step |

## 4. Internationalization

### 4.1 Messages

```rust
/// The messages for the `ColorWheel` component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// The label for the color wheel. (Default: "Hue")
    pub label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// The value text for the color wheel. (Default: "180")
    pub value_text: MessageFn<dyn Fn(f64, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            label: MessageFn::static_str("Hue"),
            value_text: MessageFn::new(|hue, _locale| format!("{hue:.0}\u{00b0}")),
        }
    }
}

impl ComponentMessages for Messages {}
```

| Key                      | Default (en-US)   | Purpose              |
| ------------------------ | ----------------- | -------------------- |
| `color_wheel.label`      | `"Hue"`           | Thumb aria-label     |
| `color_wheel.value_text` | `"{hue}\u{00b0}"` | Thumb aria-valuetext |

## 5. Library Parity

> Compared against: React Aria (`ColorWheel`).

### 5.1 Props

| Feature                       | ars-ui                    | React Aria                    | Notes                                     |
| ----------------------------- | ------------------------- | ----------------------------- | ----------------------------------------- |
| `value` / `defaultValue`      | `value` / `default_value` | `value` / `defaultValue`      | Equivalent                                |
| `outerRadius` / `innerRadius` | --                        | `outerRadius` / `innerRadius` | Rendering concern; handled by adapter CSS |
| `isDisabled`                  | `disabled`                | `isDisabled`                  | Equivalent                                |
| `name`                        | `name`                    | `name`                        | Equivalent                                |
| `form`                        | --                        | `form`                        | React Aria has form ID association        |
| `on_change_end`               | `on_change_end`           | `onChangeEnd`                 | Equivalent                                |

**Gaps:** None worth adopting. `outerRadius`/`innerRadius` are rendering dimensions, not state machine concerns.

### 5.2 Anatomy

| Part        | ars-ui        | React Aria        | Notes                 |
| ----------- | ------------- | ----------------- | --------------------- |
| Root        | `Root`        | `ColorWheel`      | Equivalent            |
| Track       | `Track`       | `ColorWheelTrack` | Equivalent            |
| Thumb       | `Thumb`       | `ColorThumb`      | Equivalent            |
| HiddenInput | `HiddenInput` | --                | ars-ui has form input |

**Gaps:** None.

### 5.3 Events

| Callback         | ars-ui                | React Aria    | Notes      |
| ---------------- | --------------------- | ------------- | ---------- |
| Value change     | `Bindable` reactivity | `onChange`    | Equivalent |
| Value change end | `on_change_end`       | `onChangeEnd` | Equivalent |

**Gaps:** None.

### 5.4 Features

| Feature                 | ars-ui     | React Aria |
| ----------------------- | ---------- | ---------- |
| Keyboard navigation     | Yes (full) | Yes        |
| Circular drag           | Yes        | Yes        |
| RTL-agnostic (circular) | Yes        | Yes        |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** React Aria requires explicit `outerRadius`/`innerRadius` props for sizing; ars-ui treats these as adapter CSS concerns.
- **Recommended additions:** None.
