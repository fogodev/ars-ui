---
component: Slider
category: input
tier: complex
foundation_deps: [architecture, accessibility, interactions, forms]
shared_deps: []
related: [range-slider]
references:
    ark-ui: Slider
    radix-ui: Slider
    react-aria: Slider
---

# Slider

A slider lets the user select a value from a continuous range by dragging a thumb along a
track.

## 1. State Machine

### 1.1 States

```rust
/// The state of the Slider component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The component is in an idle state.
    Idle,
    /// The component is in a focused state.
    Focused,
    /// The component is in a dragging state.
    Dragging,
}
```

### 1.2 Events

```rust
/// The events for the Slider component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// The component received focus.
    Focus {
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// The component lost focus.
    Blur,
    /// The component received a pointer down event.
    PointerDown {
        /// The value of the pointer down event.
        value: f64,
    },
    /// The component received a pointer move event.
    PointerMove {
        /// The value of the pointer move event.
        value: f64,
    },
    /// The component received a pointer up event.
    PointerUp,
    /// The component received an increment event.
    Increment,
    /// The component received a decrement event.
    Decrement,
    /// The component received an increment large event.
    IncrementLarge,
    /// The component received a decrement large event.
    DecrementLarge,
    /// The component received a set to minimum event.
    SetToMin,
    /// The component received a set to maximum event.
    SetToMax,
    /// The component received a set value event.
    SetValue(f64),
}
```

### 1.3 Context

```rust
use ars_i18n::{Direction, Orientation};
use ars_core::Bindable;

/// The context of the Slider component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The value of the slider.
    pub value: Bindable<f64>,
    /// The minimum value of the slider.
    pub min: f64,
    /// The maximum value of the slider.
    pub max: f64,
    /// The step size of the slider.
    pub step: f64,
    /// The large step size of the slider.
    pub large_step: Option<f64>,
    /// Whether the slider is disabled.
    pub disabled: bool,
    /// Whether the slider is read-only.
    pub readonly: bool,
    /// Whether the slider is in an invalid state.
    pub invalid: bool,
    /// The orientation of the slider.
    pub orientation: Orientation,
    /// The direction of the slider.
    pub dir: Direction,
    /// Whether the slider is focused.
    pub focused: bool,
    /// Whether the focus is visible.
    pub focus_visible: bool,
    /// Whether the slider is being dragged.
    pub dragging: bool,
    /// How the thumb aligns with the track boundaries.
    pub thumb_alignment: ThumbAlignment,
    /// The name of the slider.
    pub name: Option<String>,
    /// Whether a Description part is rendered (gates aria-describedby).
    pub has_description: bool,
    /// Resolved locale for i18n.
    pub locale: Locale,
    /// Resolved messages for the slider.
    pub messages: Messages,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}
```

### 1.4 Props

Slider Props includes `origin: Origin` (default `Start`):

```rust
/// The origin of the slider.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Origin {
    /// Fill from min → thumb (default)
    Start,
    /// Fill from center → thumb (bidirectional)
    Center,
    /// Fill from thumb → max (useful for volume "reduction" controls)
    End,
}

/// The props for the Slider component.
#[derive(Clone, Debug, PartialEq, HasId)]
// Manual Debug/PartialEq impls needed (Callback fields)
pub struct Props {
    /// The id of the slider.
    pub id: String,
    /// The value of the slider.
    pub value: Option<f64>,
    /// The default value of the slider.
    pub default_value: f64,
    /// The minimum value of the slider.
    pub min: f64,
    /// The maximum value of the slider.
    pub max: f64,
    /// The step size of the slider.
    pub step: f64,
    /// The large step size of the slider.
    pub large_step: Option<f64>,
    /// Whether the slider is disabled.
    pub disabled: bool,
    /// Whether the slider is read-only.
    pub readonly: bool,
    /// Whether the slider is in an invalid state.
    pub invalid: bool,
    /// The orientation of the slider.
    pub orientation: Orientation,
    /// The direction of the slider.
    pub dir: Direction,
    /// The origin of the slider.
    pub origin: Origin,
    /// The name of the slider.
    pub name: Option<String>,
    /// The ID of the form element the input is associated with.
    pub form: Option<String>,
    /// The marks of the slider.
    pub marks: Vec<Mark>,
    /// When set, tick mark labels use this formatter instead of raw numeric values.
    pub tick_format: Option<Callback<dyn Fn(f64) -> String>>,
    /// Formatter for the current value display and `aria-valuetext`.
    /// When `None`, the raw numeric value is used as a string.
    /// Example: `Some(Callback::new(|v| format!("{:.0}%", v)))` renders "50%" instead of "50".
    pub value_format: Option<Callback<f64, String>>,
    /// Formatter for the current value used specifically for `aria-valuetext` on the thumb element.
    /// When `Some`, the returned string is set as `aria-valuetext`, enabling screen readers to
    /// announce a human-readable representation (e.g., "$50" or "50 percent") instead of a raw number.
    /// Integrates with `NumberFormatter` for locale-aware display: pass the value through
    /// `NumberFormatter::format()` inside the callback to get locale-appropriate grouping and
    /// decimal separators (e.g., "1.234,56" in de-DE).
    /// When `None`, falls back to `value_format` if set, otherwise uses the raw numeric value.
    pub format_value: Option<Callback<f64, String>>,
    /// How the thumb aligns with the track ends. `Center` means the thumb center
    /// aligns with the track min/max; `Contain` means the thumb edge stays within
    /// the track bounds. Default: `ThumbAlignment::Contain`.
    pub thumb_alignment: ThumbAlignment,
    /// Callback fired when the user finishes a drag interaction (pointerup) or
    /// keyboard adjustment, as opposed to `on_value_change` which fires continuously.
    /// Use this for expensive operations like network requests.
    pub on_value_change_end: Option<Callback<f64>>,
    // Change callbacks provided by the adapter layer
}

/// How the thumb aligns with the track boundaries.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum ThumbAlignment {
    /// The thumb center aligns with the track min/max values. The thumb may
    /// visually overflow the track at the edges.
    Center,
    /// The thumb edge stays within the track bounds. The thumb is fully
    /// contained at both extremes.
    #[default]
    Contain,
}

/// The mark of the slider.
#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    /// The value of the mark.
    pub value: f64,
    /// The label of the mark.
    pub label: Option<String>,
}

// --- Discrete vs Continuous Semantics ---
// See "Discrete vs Continuous Slider Semantics" subsection below.

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None, default_value: 0.0,
            min: 0.0, max: 100.0, step: 1.0, large_step: None,
            disabled: false, readonly: false, invalid: false,
            orientation: Orientation::Horizontal,
            dir: Direction::Ltr,
            origin: Origin::Start,
            name: None,
            form: None,
            marks: Vec::new(),
            tick_format: None,
            value_format: None,
            format_value: None,
            thumb_alignment: ThumbAlignment::Contain,
            on_value_change_end: None,
        }
    }
}
```

### 1.5 Locale-Aware Value Display

Slider and RangeSlider MUST format all user-visible numeric values using the resolved locale's `NumberFormatter`:

- **Value labels**: The current value displayed near the thumb (e.g., tooltip or adjacent text) MUST be formatted with locale-appropriate decimal and grouping separators.
- **Tick mark labels**: If `marks` are provided with numeric labels, each label MUST be formatted through `NumberFormatter`.
- **Min/max labels**: If displayed, min and max values MUST use locale formatting.
- **`aria-valuetext`**: When a custom `value_label` function is not provided, `aria-valuetext` MUST contain the locale-formatted value (e.g., `"1.234,56"` in de-DE).

**Locale Resolution**: Same inheritance chain as NumberInput (prop → FormContext → ArsProvider → `en-US` fallback).

The track fill percentage calculation in `connect()` adjusts based on origin:

- `Start`: fill = `(value - min) / (max - min)`
- `Center`: fill from `50%` to `value%` (both directions)
- `End`: fill = `(max - value) / (max - min)`

### 1.6 RTL Direction Handling

When the `Slider`'s containing element has `dir="rtl"`, the following behaviors MUST be reversed for horizontal sliders:

**Arrow Key Reversal**:

- `ArrowRight` → **decrease** value (opposite of LTR)
- `ArrowLeft` → **increase** value (opposite of LTR)
- `ArrowUp` / `ArrowDown` → unchanged (increase / decrease respectively)
- `Home` / `End` → unchanged (min / max respectively)

**Visual Layout**:

- Track fill extends from the **right** side (min is on the right in RTL)
- Thumb position uses `right: {percent}%` instead of `left: {percent}%`
- Min label appears on the right; max label appears on the left

> **Text Rendering Constraint**: Components that measure element widths (spinner buttons, slider thumbs) must round `getBoundingClientRect()` values to the nearest integer pixel.

**RangeSlider in RTL**:

- Both thumbs flip their visual positions
- `start_value` thumb appears on the right (semantically "start" is still the lower value)
- `end_value` thumb appears on the left
- Keyboard behavior for each thumb follows the same arrow reversal rules

**Vertical Slider**: RTL has NO effect on vertical sliders. Up always increases, down always decreases, regardless of text direction.

**`aria-orientation`**: Always set to `"horizontal"` or `"vertical"` regardless of `dir`. Orientation is spatial, not directional.

### 1.7 Full Machine Implementation

```rust
/// The machine for the Slider component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;
    type Messages = Messages;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let locale = env.locale.clone();
        let messages = messages.clone();
        let state = State::Idle;
        let ctx = Context {
            value: match props.value {
                Some(v) => Bindable::controlled(v),
                None => Bindable::uncontrolled(props.default_value),
            },
            min: props.min, max: props.max, step: props.step,
            large_step: props.large_step,
            disabled: props.disabled, readonly: props.readonly, invalid: props.invalid,
            orientation: props.orientation,
            dir: props.dir,
            focused: false, focus_visible: false, dragging: false,
            thumb_alignment: props.thumb_alignment,
            name: props.name.clone(),
            has_description: false,
            locale,
            messages,
            ids: ComponentIds::from_id(&props.id),
        };
        (state, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled || ctx.readonly {
            match event {
                Event::PointerDown { .. }
                | Event::PointerMove { .. }
                | Event::Increment
                | Event::Decrement
                | Event::IncrementLarge
                | Event::DecrementLarge
                | Event::SetToMin
                | Event::SetToMax
                | Event::SetValue(_) => return None,
                _ => {}
            }
        }

        match event {
            Event::Focus { is_keyboard } => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::to(State::Focused).apply(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = is_kb;
                }))
            }
            Event::Blur => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                    ctx.dragging = false;
                }))
            }
            Event::PointerDown { value } => {
                let snapped = snap_to_step(*value, ctx.min, ctx.max, ctx.step);
                Some(TransitionPlan::to(State::Dragging).apply(move |ctx| {
                    ctx.value.set(snapped);
                    ctx.dragging = true;
                }))
            }
            Event::PointerMove { value } => {
                if !ctx.dragging { return None; }
                let snapped = snap_to_step(*value, ctx.min, ctx.max, ctx.step);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(snapped);
                }))
            }
            Event::PointerUp => {
                // Guard: ignore orphaned PointerUp without a preceding PointerDown.
                if !ctx.dragging { return None; }
                let is_focused = ctx.focused;
                let final_value = *ctx.value.get();
                Some(TransitionPlan::to(if is_focused {
                        State::Focused
                    } else {
                        State::Idle
                    }).apply(|ctx| {
                        ctx.dragging = false;
                    })with_effect(PendingEffect::new("value-change-end", move |_ctx, props, _send| {
                        // Fire on_value_change_end with the final value after drag completes.
                        if let Some(ref cb) = props.on_value_change_end {
                            cb.call(final_value);
                        }
                        no_cleanup()
                    }))
                )
            }
            Event::Increment => {
                let next = (*ctx.value.get() + ctx.step).min(ctx.max);
                let snapped = snap_to_step(next, ctx.min, ctx.max, ctx.step);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(snapped);
                }))
            }
            Event::Decrement => {
                let prev = (*ctx.value.get() - ctx.step).max(ctx.min);
                let snapped = snap_to_step(prev, ctx.min, ctx.max, ctx.step);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(snapped);
                }))
            }
            Event::IncrementLarge => {
                let step = ctx.large_step.unwrap_or(ctx.step * 10.0);
                let next = (*ctx.value.get() + step).min(ctx.max);
                let snapped = snap_to_step(next, ctx.min, ctx.max, ctx.step);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(snapped);
                }))
            }
            Event::DecrementLarge => {
                let step = ctx.large_step.unwrap_or(ctx.step * 10.0);
                let prev = (*ctx.value.get() - step).max(ctx.min);
                let snapped = snap_to_step(prev, ctx.min, ctx.max, ctx.step);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(snapped);
                }))
            }
            Event::SetToMin => {
                let min = ctx.min;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(min);
                }))
            }
            Event::SetToMax => {
                let max = ctx.max;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(max);
                }))
            }
            Event::SetValue(val) => {
                let snapped = snap_to_step(*val, ctx.min, ctx.max, ctx.step);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(snapped);
                }))
            }
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

/// The API for the Slider component.
pub struct Api<'a> {
    /// The state of the Slider component.
    state: &'a State,
    /// The context of the Slider component.
    ctx: &'a Context,
    /// The props of the Slider component.
    props: &'a Props,
    /// The send callback for the Slider component.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Focus the thumb of the Slider component.
    pub fn on_thumb_focus(&self, is_keyboard: bool) { (self.send)(Event::Focus { is_keyboard }); }

    /// Blur the thumb of the Slider component.
    pub fn on_thumb_blur(&self) { (self.send)(Event::Blur); }

    /// Handle a keydown event on the thumb of the Slider component.
    pub fn on_thumb_keydown(&self, data: &KeyboardEventData, _shift: bool) {
        // RTL-aware arrow key mapping for horizontal sliders
        let is_rtl = self.ctx.dir == Direction::Rtl;
        let is_horizontal = self.ctx.orientation == Orientation::Horizontal;
        match data.key {
            KeyboardKey::ArrowRight => {
                if is_horizontal && is_rtl {
                    (self.send)(Event::Decrement)
                } else {
                    (self.send)(Event::Increment)
                }
            }
            KeyboardKey::ArrowLeft => {
                if is_horizontal && is_rtl {
                    (self.send)(Event::Increment)
                } else {
                    (self.send)(Event::Decrement)
                }
            }
            KeyboardKey::ArrowUp => (self.send)(Event::Increment),
            KeyboardKey::ArrowDown => (self.send)(Event::Decrement),
            KeyboardKey::PageUp => (self.send)(Event::IncrementLarge),
            KeyboardKey::PageDown => (self.send)(Event::DecrementLarge),
            KeyboardKey::Home => (self.send)(Event::SetToMin),
            KeyboardKey::End => (self.send)(Event::SetToMax),
            _ => {}
        }
    }
}

/// Snap a value to the nearest step.
fn snap_to_step(value: f64, min: f64, max: f64, step: f64) -> f64 {
    let clamped = value.clamp(min, max);
    let steps_from_min = ((clamped - min) / step).round();
    (min + steps_from_min * step).clamp(min, max)
}
```

### 1.8 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "slider"]
pub enum Part {
    Root,
    Label,
    Track,
    Range,
    Thumb,
    Output,
    MarkerGroup,
    Marker { value: f64 },    // mark value
    HiddenInput,
    DraggingIndicator,
    Description,
    ErrorMessage,
}

impl<'a> Api<'a> {
    /// Attributes for the root container.
    pub fn root_attrs(&self) -> AttrMap {
        let is_horizontal = self.ctx.orientation == Orientation::Horizontal;
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-orientation"), if is_horizontal { "horizontal" } else { "vertical" });
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.invalid { attrs.set_bool(HtmlAttr::Data("ars-invalid"), true); }
        if self.ctx.dragging { attrs.set_bool(HtmlAttr::Data("ars-dragging"), true); }
        attrs
    }

    /// Attributes for the label element.
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("label"));
        attrs
    }

    /// Attributes for the track element.
    pub fn track_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Track.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn on_track_pointerdown(&self, pointer_value: f64) {
        (self.send)(Event::PointerDown { value: pointer_value });
    }

    /// Attributes for the range (filled portion of track).
    pub fn range_attrs(&self) -> AttrMap {
        let percent = ((self.ctx.value.get() - self.ctx.min) / (self.ctx.max - self.ctx.min) * 100.0).clamp(0.0, 100.0);
        let is_horizontal = self.ctx.orientation == Orientation::Horizontal;
        let is_rtl = self.ctx.dir == Direction::Rtl;

        let range_start = if is_horizontal && is_rtl {
            format!("{}%", 100.0 - percent)
        } else {
            "0%".to_string()
        };
        let range_size = format!("{}%", percent);

        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Range.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set_style(if is_horizontal { CssProperty::Left } else { CssProperty::Bottom }, range_start);
        attrs.set_style(if is_horizontal { CssProperty::Width } else { CssProperty::Height }, range_size);
        attrs
    }

    /// Attributes for the thumb element.
    pub fn thumb_attrs(&self) -> AttrMap {
        let raw_percent = ((self.ctx.value.get() - self.ctx.min) / (self.ctx.max - self.ctx.min) * 100.0).clamp(0.0, 100.0);
        // ThumbAlignment::Center: thumb center sits at raw_percent (may overflow track).
        // ThumbAlignment::Contain: compress the range so the thumb edge stays within
        // the track. The adapter sets `--ars-thumb-size` CSS var; the position formula
        // becomes `calc(raw% * (1 - thumb-size/track-size))`. We output the raw percent
        // and a data attribute; the adapter applies the containment offset in CSS.
        let percent = raw_percent;
        let is_horizontal = self.ctx.orientation == Orientation::Horizontal;
        let is_rtl = self.ctx.dir == Direction::Rtl;

        let value_text = self.props.format_value.as_ref()
            .map(|f| f(*self.ctx.value.get()))
            .or_else(|| self.props.value_format.as_ref().map(|f| f(*self.ctx.value.get())))
            .unwrap_or_else(|| format!("{}", self.ctx.value.get()));

        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Thumb.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "slider");
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), self.ctx.value.get().to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), self.ctx.min.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), self.ctx.max.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), value_text);
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), if is_horizontal { "horizontal" } else { "vertical" });
        // aria-label from messages provides the accessible name.
        // When the adapter renders a visible <label> element, it sets aria-labelledby
        // on the thumb which takes precedence over aria-label per WAI-ARIA.
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.thumb_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));
        if self.ctx.disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        if self.ctx.readonly { attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true"); }
        if self.ctx.invalid { attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true"); }
        if self.ctx.has_description {
            let mut describedby_parts = Vec::new();
            describedby_parts.push(self.ctx.ids.part("description"));
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), describedby_parts.join(" "));
        }
        attrs.set(HtmlAttr::TabIndex, if self.ctx.disabled { "-1" } else { "0" });
        if self.ctx.focus_visible { attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true); }
        attrs.set_style(
            if is_horizontal { CssProperty::Left } else { CssProperty::Bottom },
            format!("{}%", if is_horizontal && is_rtl { 100.0 - percent } else { percent }),
        );
        attrs.set_bool(HtmlAttr::Data("ars-thumb-alignment"), match self.ctx.thumb_alignment {
            ThumbAlignment::Center => false,  // default CSS behavior
            ThumbAlignment::Contain => true,  // adapter applies containment offset
        });
        attrs.set(HtmlAttr::Class, "ars-touch-none");
        attrs
    }

    /// Attributes for the output display element.
    pub fn output_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Output.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::For, self.ctx.ids.part("thumb"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "off");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));
        attrs
    }

    /// Attributes for the marker group container.
    pub fn marker_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::MarkerGroup.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "presentation");
        attrs
    }

    /// Attributes for a single marker.
    pub fn marker_attrs(&self, value: f64) -> AttrMap {
        let current = *self.ctx.value.get();
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Marker { value }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if value <= current {
            attrs.set_bool(HtmlAttr::Data("ars-in-range"), true);
        }
        attrs
    }

    /// Attributes for the hidden input (form submission).
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name);
        }
        if let Some(ref form) = self.props.form {
            attrs.set(HtmlAttr::Form, form);
        }
        attrs.set(HtmlAttr::Value, self.ctx.value.get().to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attributes for the description/help text.
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));
        attrs
    }

    /// Attributes for the error message element.
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("error-message"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        attrs
    }

    /// Attributes for the dragging indicator element.
    /// A purely decorative visual element shown only during thumb drag operations.
    pub fn dragging_indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DraggingIndicator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if self.ctx.dragging { "dragging" } else { "idle" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        if !self.ctx.dragging {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }
        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Track => self.track_attrs(),
            Part::Range => self.range_attrs(),
            Part::Thumb => self.thumb_attrs(),
            Part::Output => self.output_attrs(),
            Part::MarkerGroup => self.marker_group_attrs(),
            Part::Marker { value } => self.marker_attrs(value),
            Part::HiddenInput => self.hidden_input_attrs(),
            Part::DraggingIndicator => self.dragging_indicator_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}

/// Compute slider value from a pointer event's position relative to the track.
/// `track` is a platform-agnostic `Rect` (see `01-architecture.md` §2.2.7).
fn compute_value_from_pointer(event: &PointerEvent, track: &Rect, ctx: &Context) -> f64 {
    let percent = if ctx.orientation == Orientation::Horizontal {
        let offset = event.client_x() as f64 - track.x;
        (offset / track.width).clamp(0.0, 1.0)
    } else {
        let offset = (track.y + track.height) - event.client_y() as f64;
        (offset / track.height).clamp(0.0, 1.0)
    };

    let percent = if ctx.dir == Direction::Rtl && ctx.orientation == Orientation::Horizontal {
        1.0 - percent
    } else {
        percent
    };

    ctx.min + percent * (ctx.max - ctx.min)
}

/// The messages for the Slider component.
/// Range-specific labels (start/end thumb) are in `range-slider.md`'s Messages.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible label for the thumb. Default: `"Value"`.
    pub thumb_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            thumb_label: MessageFn::static_str("Value"),
        }
    }
}

impl ComponentMessages for Messages {}
```

> **Mark Accessibility Labels**: Each slider mark element should receive an `aria-label` describing its semantic meaning (e.g., "Low", "Medium", "High") when `tick_labels` are provided. The `tick_format` callback produces the visual label; the same text is used as `aria-label` on the mark element. Marks without labels receive no `aria-label` (they are decorative).
> **Message override pattern:** All components with a `Messages` struct follow a consistent
> override chain:
>
> 1. **`props.messages`** — Component-level override. When provided, replaces all defaults.
> 2. **Context locale provider** — If `props.messages` is `None` (or uses defaults), messages
>    are resolved from the nearest `ArsProvider` context, which supplies locale-appropriate
>    defaults for all `ComponentMessages` types.
> 3. **Built-in English defaults** — The `Default` impl on each `Messages` struct provides
>    English fallback text.
>
> To override a single message while keeping defaults for the rest:
>
> ```rust
> let messages = Messages {
>     slider_roledescription: "volume control".into(),
>     ..Messages::default()  // keep mark_label etc. as default
> };
> ```
>
> This pattern applies to all `Messages` structs: `Messages`, `EditableMessages`,
> `PasswordInputMessages`, `FileTriggerMessages`, and their equivalents in other component
> files.

## 2. Anatomy

```text
Slider
├── Root               <div>     data-ars-scope="slider" data-ars-part="root"
├── Label              <label>   data-ars-part="label"
├── Track              <div>     data-ars-part="track"
│   ├── Range          <div>     data-ars-part="range" (filled portion)
│   └── Thumb     [A]  <div>     data-ars-part="thumb" (role="slider")
├── DraggingIndicator  <div>     data-ars-part="dragging-indicator" (optional, aria-hidden)
├── Output             <output>  data-ars-part="output" (optional)
├── MarkerGroup        <div>     data-ars-part="marker-group" (optional)
│   └── Marker (×N)    <span>    data-ars-part="marker"
├── HiddenInput        <input>   data-ars-part="hidden-input" (type="hidden")
├── Description        <div>     data-ars-part="description" (optional)
└── ErrorMessage       <div>     data-ars-part="error-message" (optional)
```

| Part                | Selector                                                        | Element    |
| ------------------- | --------------------------------------------------------------- | ---------- |
| `Root`              | `[data-ars-scope="slider"][data-ars-part="root"]`               | `<div>`    |
| `Label`             | `[data-ars-scope="slider"][data-ars-part="label"]`              | `<label>`  |
| `Track`             | `[data-ars-scope="slider"][data-ars-part="track"]`              | `<div>`    |
| `Range`             | `[data-ars-scope="slider"][data-ars-part="range"]`              | `<div>`    |
| `Thumb`             | `[data-ars-scope="slider"][data-ars-part="thumb"]`              | `<div>`    |
| `DraggingIndicator` | `[data-ars-scope="slider"][data-ars-part="dragging-indicator"]` | `<div>`    |
| `Output`            | `[data-ars-scope="slider"][data-ars-part="output"]`             | `<output>` |
| `MarkerGroup`       | `[data-ars-scope="slider"][data-ars-part="marker-group"]`       | `<div>`    |
| `Marker`            | `[data-ars-scope="slider"][data-ars-part="marker"]`             | `<span>`   |
| `HiddenInput`       | `[data-ars-scope="slider"][data-ars-part="hidden-input"]`       | `<input>`  |
| `Description`       | `[data-ars-scope="slider"][data-ars-part="description"]`        | `<div>`    |
| `ErrorMessage`      | `[data-ars-scope="slider"][data-ars-part="error-message"]`      | `<div>`    |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Element           | Value                                                         |
| ------------------ | ----------------- | ------------------------------------------------------------- |
| `role`             | Thumb             | `slider`                                                      |
| `aria-valuenow`    | Thumb             | Current value                                                 |
| `aria-valuemin`    | Thumb             | Minimum value                                                 |
| `aria-valuemax`    | Thumb             | Maximum value                                                 |
| `aria-valuetext`   | Thumb             | Formatted value text (via `format_value` or locale formatter) |
| `aria-orientation` | Thumb             | `"horizontal"` or `"vertical"`                                |
| `aria-disabled`    | Thumb             | Present when `disabled=true`                                  |
| `aria-readonly`    | Thumb             | Present when `readonly=true`                                  |
| `aria-labelledby`  | Thumb             | Points to Label id                                            |
| `aria-hidden`      | DraggingIndicator | `"true"` — purely decorative visual feedback during drag      |
| `hidden`           | DraggingIndicator | Present when not dragging (indicator is invisible when idle)  |

### 3.2 Keyboard Interaction

| Key                               | Action                                     |
| --------------------------------- | ------------------------------------------ |
| ArrowRight / ArrowUp              | Increment by `props.step` (default 1)      |
| ArrowLeft / ArrowDown             | Decrement by `props.step` (default 1)      |
| Shift+ArrowRight / Shift+ArrowUp  | Increment by 10% of range                  |
| Shift+ArrowLeft / Shift+ArrowDown | Decrement by 10% of range                  |
| PageUp                            | Increment by large step (default 10 steps) |
| PageDown                          | Decrement by large step (default 10 steps) |
| Home                              | Set to minimum                             |
| End                               | Set to maximum                             |

## 4. Internationalization

- **Locale resolution**: The effective locale is `props.locale.unwrap_or_else(|| context_locale())`.
- **RTL**: Arrow keys swap direction for horizontal sliders. Thumb and range positions mirror.
- **`aria-valuetext`**: Uses `NumberFormatter` to format value per the resolved locale.
- **Output display**: Formatted per the resolved locale.
- **Marks labels**: User-provided, localized by consumer.
- **Percentage display**: Formatted per locale in the Output part.

### 4.1 Discrete vs Continuous Slider Semantics

Sliders operate in one of two modes, controlled by the `discrete` prop:

```rust
/// When true, the slider snaps to defined steps and reports discrete labels.
pub discrete: bool,  // default: false
/// Optional labels for each discrete step. Length must match the number of steps.
/// When provided, `aria-valuetext` uses these labels instead of raw numbers.
pub value_labels: Option<Vec<String>>,
```

**`aria-valuetext` generation:**

- **Discrete with `value_labels`**: `aria-valuetext` is set to `"{label}"` with positional
  context — e.g., `"3 of 5 (Medium)"` where 3 is the current step index, 5 is total steps,
  and "Medium" is the label. Format: `"{index} of {total} ({label})"`.
- **Continuous with unit**: `aria-valuetext` is set to the formatted value with unit suffix —
  e.g., `"72 dB"`. The unit is provided via `Messages.unit: Option<String>`.
- **Continuous without unit**: `aria-valuetext` is omitted; `aria-valuenow` alone is
  sufficient for screen readers.

When `discrete` is true and `value_labels` is `None`, the slider uses the numeric value
formatted per locale as `aria-valuetext`.

### 4.2 `aria-valuetext` Localization

1. Slider accepts an optional `format_value_text: Box<dyn Fn(f64) -> String>` prop for
   custom labels (e.g., `'Low'`, `'Medium'`, `'High'`).
2. This function receives the current value and must return a localized string.
3. When provided, the adapter sets `aria-valuetext` to the function's output.
4. When not provided, `aria-valuetext` is omitted and screen readers use the raw numeric
   `aria-valuenow`.
5. For RangeSlider, each thumb has its own format function.

### 4.3 Keyboard Modifiers (Slider / RangeSlider)

1. `Shift+Arrow` moves by `large_step` (default: 10× step).
2. When both thumbs are at the same position, `ArrowRight` moves the upper thumb,
   `ArrowLeft` moves the lower thumb.
3. Thumbs cannot cross: if moving a thumb would pass the other, it stops at the other
   thumb's position (or the shared position, depending on the `allow_overlap` prop).

## 5. Form Integration

- **Hidden input**: A hidden `<input type="hidden">` is rendered via the `HiddenInput` part. It carries the `name` attribute from context and the current numeric value.
- **Validation states**: The Thumb carries `aria-disabled` and `aria-readonly` from context.
- **Reset behavior**: On form reset, the adapter restores `value` to `default_value`.
- **Disabled/readonly propagation**: When inside a `Field` or `Fieldset`, the adapter merges `disabled`/`readonly` from `FieldCtx` per `07-forms.md` §12.6.

## 6. Variant: N-Thumb

The single-value Slider can be generalized to support **N thumbs** for multi-point
value selection (e.g., color gradient stops, multi-range audio EQ). The `RangeSlider`
(§9) is the specific 2-thumb case; this section defines the general N-thumb contract.

### 6.1 Additional Props

```rust
/// N-thumb slider props — generalization of the single-thumb Slider.
/// When `values` has one element, this behaves identically to the single Slider.
/// When `values` has two elements, this is equivalent to RangeSlider.
pub values: Option<Vec<f64>>,
pub default_values: Vec<f64>,
```

### 6.2 Additional Context

```rust
/// Generalized multi-thumb context.
pub values: Bindable<Vec<f64>>,
/// Index of the thumb currently focused / being dragged.
pub active_thumb: Option<usize>,
```

### 6.3 Behavior

Thumb values **must** remain in non-descending order at all times. The machine enforces
this by clamping each thumb between its neighbours:

```rust
fn set_thumb_value(ctx: &mut Context, index: usize, raw: f64) {
    let snapped = snap_to_step(raw, ctx.min, ctx.max, ctx.step);
    let min_gap = ctx.min_steps_between as f64 * ctx.step;

    let values = ctx.values.get_mut_owned();
    let lower_bound = if index == 0 {
        ctx.min
    } else {
        values[index - 1] + min_gap
    };
    let upper_bound = if index == values.len() - 1 {
        ctx.max
    } else {
        values[index + 1] - min_gap
    };
    values[index] = snapped.clamp(lower_bound, upper_bound);
}
```

### 6.4 Additional Events

```rust
/// Generalized events for N-thumb slider.
/// `thumb` is the 0-based index of the target thumb.
Increment { thumb: usize },
Decrement { thumb: usize },
Focus { thumb: usize, is_keyboard: bool },
Blur { thumb: usize },
PointerDown { thumb: usize, value: f64 },
SetValues(Vec<f64>),
```

### 6.5 Anatomy Additions

Each thumb is rendered with `data-ars-index="{i}"`. The `Range` parts (filled regions
between consecutive thumbs) also carry `data-ars-index`:

| Part        | Multiplicity | Key Attributes                                   |
| ----------- | ------------ | ------------------------------------------------ |
| Thumb       | N            | `data-ars-index="{i}"`, `role="slider"`          |
| Range       | N−1          | `data-ars-index="{i}"` (between thumb i and i+1) |
| HiddenInput | N            | `name="{name}[{i}]"` for form submission         |

### 6.6 Accessibility

Each thumb is an independent `role="slider"` element with its own `aria-valuenow`,
`aria-valuemin` (clamped to neighbour), and `aria-valuemax` (clamped to neighbour).
`aria-label` defaults to `"Value {i+1} of {n}"` — localised via the i18n catalog.

## 7. Library Parity

> Compared against: Ark UI (`Slider`), Radix UI (`Slider`), React Aria (`Slider`).

### 7.1 Props

| Feature          | ars-ui                            | Ark UI             | Radix UI          | React Aria      | Notes                                              |
| ---------------- | --------------------------------- | ------------------ | ----------------- | --------------- | -------------------------------------------------- |
| Controlled value | `value: Option<f64>`              | `value: number[]`  | `value: number[]` | `value`         | Full parity (Ark/Radix use arrays for multi-thumb) |
| Default value    | `default_value: f64`              | `defaultValue`     | `defaultValue`    | `defaultValue`  | Full parity                                        |
| Min              | `min: f64`                        | `min`              | `min`             | `minValue`      | Full parity                                        |
| Max              | `max: f64`                        | `max`              | `max`             | `maxValue`      | Full parity                                        |
| Step             | `step: f64`                       | `step`             | `step`            | `step`          | Full parity                                        |
| Disabled         | `disabled: bool`                  | `disabled`         | `disabled`        | `isDisabled`    | Full parity                                        |
| Read-only        | `readonly: bool`                  | `readOnly`         | --                | --              | Ark parity                                         |
| Invalid          | `invalid: bool`                   | `invalid`          | --                | --              | Ark parity                                         |
| Orientation      | `orientation: Orientation`        | `orientation`      | `orientation`     | `orientation`   | Full parity                                        |
| Direction        | `dir: Direction`                  | --                 | `dir`             | --              | Radix parity                                       |
| Form name        | `name: Option<String>`            | `name`             | `name`            | --              | Ark+Radix parity                                   |
| Form ID          | `form: Option<String>`            | `form`             | `form`            | --              | Ark+Radix parity                                   |
| Origin           | `origin: Origin`                  | `origin`           | --                | --              | Ark parity                                         |
| Thumb alignment  | `thumb_alignment: ThumbAlignment` | `thumbAlignment`   | --                | --              | Ark parity                                         |
| Value format     | `format_value` / `value_format`   | `getAriaValueText` | --                | `formatOptions` | Full parity                                        |
| On change end    | `on_value_change_end`             | `onValueChangeEnd` | `onValueCommit`   | `onChangeEnd`   | Full parity                                        |
| Inverted         | --                                | --                 | `inverted`        | --              | Radix-only visual inversion                        |
| Marks            | `marks: Vec<Mark>`                | `Marker` part      | --                | --              | Ark parity                                         |
| Large step       | `large_step: Option<f64>`         | --                 | --                | --              | ars-ui enhancement for PageUp/PageDown             |

**Gaps:** None material. Radix `inverted` is a purely visual flag achievable via CSS with ars-ui's `origin: End`.

### 7.2 Anatomy

| Part              | ars-ui              | Ark UI              | Radix UI   | React Aria     | Notes                      |
| ----------------- | ------------------- | ------------------- | ---------- | -------------- | -------------------------- |
| Root              | `Root`              | `Root`              | `Root`     | `Slider`       | Full parity                |
| Label             | `Label`             | `Label`             | --         | `Label`        | Full parity                |
| Track             | `Track`             | `Track`             | `Track`    | `SliderTrack`  | Full parity                |
| Range             | `Range`             | `Range`             | `Range`    | --             | Full parity with Ark/Radix |
| Thumb             | `Thumb`             | `Thumb`             | `Thumb`    | `SliderThumb`  | Full parity                |
| Output            | `Output`            | `ValueText`         | --         | `SliderOutput` | Full parity                |
| MarkerGroup       | `MarkerGroup`       | `MarkerGroup`       | --         | --             | Ark parity                 |
| Marker            | `Marker`            | `Marker`            | --         | --             | Ark parity                 |
| HiddenInput       | `HiddenInput`       | `HiddenInput`       | (built-in) | (built-in)     | Full parity                |
| DraggingIndicator | `DraggingIndicator` | `DraggingIndicator` | --         | --             | Ark parity                 |
| Description       | `Description`       | --                  | --         | --             | ars-ui form-field part     |
| ErrorMessage      | `ErrorMessage`      | --                  | --         | --             | ars-ui form-field part     |

**Gaps:** None.

### 7.3 Events

| Callback     | ars-ui                             | Ark UI             | Radix UI        | React Aria    | Notes       |
| ------------ | ---------------------------------- | ------------------ | --------------- | ------------- | ----------- |
| Value change | `SetValue`/`Increment`/`Decrement` | `onValueChange`    | `onValueChange` | `onChange`    | Full parity |
| Change end   | `on_value_change_end`              | `onValueChangeEnd` | `onValueCommit` | `onChangeEnd` | Full parity |
| Focus change | `Focus`/`Blur`                     | `onFocusChange`    | --              | --            | Ark parity  |

**Gaps:** None.

### 7.4 Features

| Feature                | ars-ui                                  | Ark UI             | Radix UI           | React Aria         |
| ---------------------- | --------------------------------------- | ------------------ | ------------------ | ------------------ |
| Keyboard navigation    | Yes (arrows, Home/End, PageUp/PageDown) | Yes                | Yes                | Yes                |
| RTL support            | Yes                                     | --                 | Yes                | --                 |
| Drag interaction       | Yes                                     | Yes                | Yes                | Yes                |
| Track click jump       | Yes                                     | Yes                | Yes                | Yes                |
| Marks/ticks            | Yes                                     | Yes                | --                 | --                 |
| Form integration       | Yes                                     | Yes                | Built-in           | --                 |
| N-thumb generalization | Yes (section 6)                         | Yes (array values) | Yes (array values) | Yes (array values) |

**Gaps:** None.

### 7.5 Summary

- **Overall:** Full parity across all three reference libraries.
- **Divergences:** ars-ui uses a single `f64` for single-thumb and `[f64; 2]` for range (see RangeSlider), while Ark/Radix always use `number[]`. ars-ui adds `large_step` for PageUp/PageDown granularity and `Origin` for fill direction.
- **Recommended additions:** None.
