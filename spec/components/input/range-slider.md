---
component: RangeSlider
category: input
tier: complex
foundation_deps: [architecture, accessibility, interactions, forms]
shared_deps: []
related: [slider]
references:
    ark-ui: Slider
    react-aria: Slider
---

# RangeSlider

A dual-thumb slider for selecting a range (start and end values). Extends the Slider
architecture with two thumbs that cannot cross each other.

## 1. State Machine

### 1.1 States

```rust
/// The state of the RangeSlider component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// The component is in an idle state.
    Idle,
    /// A thumb is focused.
    Focused { thumb: ThumbIndex },
    /// A thumb is being dragged.
    Dragging { thumb: ThumbIndex },
}

/// Identifies which thumb.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ThumbIndex {
    /// The start (lower) thumb.
    Start,
    /// The end (upper) thumb.
    End,
}
```

### 1.2 Events

```rust
/// The events for the RangeSlider component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Focus on a specific thumb.
    Focus { thumb: ThumbIndex, is_keyboard: bool },
    /// Blur from a specific thumb.
    Blur { thumb: ThumbIndex },
    /// Pointer down on a specific thumb.
    PointerDown { thumb: ThumbIndex, value: f64 },
    /// Pointer move during drag.
    PointerMove { value: f64 },
    /// Pointer released.
    PointerUp,
    /// Increment a specific thumb.
    Increment { thumb: ThumbIndex },
    /// Decrement a specific thumb.
    Decrement { thumb: ThumbIndex },
    /// Increment a specific thumb by large step.
    IncrementLarge { thumb: ThumbIndex },
    /// Decrement a specific thumb by large step.
    DecrementLarge { thumb: ThumbIndex },
    /// Set a thumb to minimum.
    SetToMin { thumb: ThumbIndex },
    /// Set a thumb to maximum.
    SetToMax { thumb: ThumbIndex },
    /// Set both values.
    SetValues([f64; 2]),
}
```

### 1.3 Context

```rust
/// The context of the RangeSlider component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The range value as `[start, end]` — controlled or uncontrolled.
    pub value: Bindable<[f64; 2]>,
    /// The minimum value of the track.
    pub min: f64,
    /// The maximum value of the track.
    pub max: f64,
    /// The step value.
    pub step: f64,
    /// The large step value (PageUp/PageDown).
    pub large_step: Option<f64>,
    /// Minimum number of steps between the thumbs.
    pub min_steps_between: u32,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is readonly.
    pub readonly: bool,
    /// Whether the component is invalid.
    pub invalid: bool,
    /// The orientation of the track.
    pub orientation: Orientation,
    /// Text direction for RTL support.
    pub dir: Direction,
    /// The focused thumb.
    pub focused_thumb: Option<ThumbIndex>,
    /// Whether focus is visible.
    pub focus_visible: bool,
    /// The thumb being dragged.
    pub dragging_thumb: Option<ThumbIndex>,
    /// How the thumbs align with the track boundaries.
    pub thumb_alignment: ThumbAlignment,
    /// The name attribute for form submission.
    pub name: Option<String>,
    /// Whether the start thumb is individually disabled.
    pub start_disabled: bool,
    /// Whether the end thumb is individually disabled.
    pub end_disabled: bool,
    /// Whether a Description part is rendered (gates aria-describedby).
    pub has_description: bool,
    /// Resolved locale for i18n.
    pub locale: Locale,
    /// Resolved messages for the range slider.
    pub messages: Messages,
    /// Component IDs for part identification.
    pub ids: ComponentIds,
}
```

### 1.4 Props

```rust
/// The props for the RangeSlider component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    pub id: String,
    /// Controlled value. When Some, component is controlled.
    pub value: Option<[f64; 2]>,
    /// Default value for uncontrolled mode.
    pub default_value: [f64; 2],
    /// The minimum value.
    pub min: f64,
    /// The maximum value.
    pub max: f64,
    /// The step size.
    pub step: f64,
    /// The large step size (PageUp/PageDown).
    pub large_step: Option<f64>,
    /// Minimum number of steps between thumbs.
    pub min_steps_between: u32,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is readonly.
    pub readonly: bool,
    /// Whether the component is invalid.
    pub invalid: bool,
    /// The orientation.
    pub orientation: Orientation,
    /// Text direction.
    pub dir: Direction,
    /// The name attribute for form submission.
    pub name: Option<String>,
    /// The ID of the form element the input is associated with.
    pub form: Option<String>,
    /// When true, dragging past the opposite thumb swaps active thumb.
    pub allow_thumb_swap: bool,
    /// Whether the start thumb is individually disabled.
    pub start_disabled: bool,
    /// Whether the end thumb is individually disabled.
    pub end_disabled: bool,
    /// Formatter for `aria-valuetext`. Receives `(this_value, other_value)`.
    pub format_value: Option<Callback<(f64, f64), String>>,
    /// How the thumbs align with the track ends. See `slider::ThumbAlignment`.
    pub thumb_alignment: ThumbAlignment,
    /// Callback fired when a drag interaction ends (pointerup), as opposed to
    /// continuous change callbacks. Receives the final `[start, end]` value pair.
    pub on_value_change_end: Option<Callback<[f64; 2]>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: [0.0, 100.0],
            min: 0.0, max: 100.0, step: 1.0, large_step: None,
            min_steps_between: 0,
            disabled: false, readonly: false, invalid: false,
            orientation: Orientation::Horizontal,
            dir: Direction::Ltr,
            name: None,
            form: None,
            allow_thumb_swap: false,
            start_disabled: false,
            end_disabled: false,
            format_value: None,
            thumb_alignment: ThumbAlignment::Contain,
            on_value_change_end: None,
        }
    }
}

/// Messages for the RangeSlider component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Label for the start thumb. Default: `"Range start"`.
    pub start_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the end thumb. Default: `"Range end"`.
    pub end_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            start_label: MessageFn::static_str("Range start"),
            end_label: MessageFn::static_str("Range end"),
        }
    }
}

impl ComponentMessages for Messages {}
```

### 1.5 Guards

The RangeSlider maintains the invariant `start_value <= end_value` at all times:

```rust
fn is_disabled(ctx: &Context) -> bool { ctx.disabled }
fn is_readonly(ctx: &Context) -> bool { ctx.readonly }
fn is_thumb_disabled(ctx: &Context, thumb: ThumbIndex) -> bool {
    ctx.disabled || match thumb {
        ThumbIndex::Start => ctx.start_disabled,
        ThumbIndex::End => ctx.end_disabled,
    }
}

/// Set the value of a specific thumb, enforcing the non-crossing invariant.
fn set_thumb_value(ctx: &mut Context, thumb: ThumbIndex, raw: f64) {
    let snapped = snap_to_step(raw, ctx.min, ctx.max, ctx.step);
    let min_gap = ctx.min_steps_between as f64 * ctx.step;
    let [start, end] = *ctx.value.get();

    let new_values = match thumb {
        ThumbIndex::Start => {
            let clamped = snapped.clamp(ctx.min, end - min_gap);
            [clamped, end]
        }
        ThumbIndex::End => {
            let clamped = snapped.clamp(start + min_gap, ctx.max);
            [start, clamped]
        }
    };
    ctx.value.set(new_values);
}
```

### 1.6 Drag-Past Behavior

When the user drags a thumb past the other during a pointer interaction:

- **Clamp (default)**: The dragged thumb is clamped so it cannot exceed the other thumb's position (minus `min_steps_between`). The user must release and grab the other thumb.
- **Swap (opt-in)**: When `allow_thumb_swap: true`, dragging past causes the active thumb identity to swap. The previously-dragged thumb stays at the crossover point and the other thumb becomes the drag target.

In both modes, the machine fires `on_change` with the corrected `[start, end]` values, maintaining `start <= end`.

### 1.7 Full Machine Implementation

```rust
/// Machine for the RangeSlider component.
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
            min_steps_between: props.min_steps_between,
            disabled: props.disabled, readonly: props.readonly, invalid: props.invalid,
            orientation: props.orientation, dir: props.dir,
            focused_thumb: None, focus_visible: false, dragging_thumb: None, thumb_alignment: props.thumb_alignment,
            name: props.name.clone(),
            start_disabled: props.start_disabled,
            end_disabled: props.end_disabled,
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
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if is_disabled(ctx) || is_readonly(ctx) {
            match event {
                Event::PointerDown { .. } | Event::PointerMove { .. }
                | Event::Increment { .. } | Event::Decrement { .. }
                | Event::IncrementLarge { .. } | Event::DecrementLarge { .. }
                | Event::SetToMin { .. } | Event::SetToMax { .. }
                | Event::SetValues(_) => return None,
                _ => {}
            }
        }

        match event {
            Event::Focus { thumb, is_keyboard } => {
                let thumb = *thumb;
                let is_kb = *is_keyboard;
                Some(TransitionPlan::to(State::Focused { thumb }).apply(move |ctx| {
                    ctx.focused_thumb = Some(thumb);
                    ctx.focus_visible = is_kb;
                }))
            }
            Event::Blur { .. } => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.focused_thumb = None;
                    ctx.focus_visible = false;
                    ctx.dragging_thumb = None;
                }))
            }
            Event::PointerDown { thumb, value } => {
                if is_thumb_disabled(ctx, *thumb) { return None; }
                let thumb = *thumb;
                let value = *value;
                Some(TransitionPlan::to(State::Dragging { thumb }).apply(move |ctx| {
                    ctx.dragging_thumb = Some(thumb);
                    set_thumb_value(ctx, thumb, value);
                }))
            }
            Event::PointerMove { value } => {
                let thumb = match ctx.dragging_thumb {
                    Some(t) => t,
                    None => return None,
                };
                let value = *value;
                Some(TransitionPlan::context_only(move |ctx| {
                    set_thumb_value(ctx, thumb, value);
                }))
            }
            Event::PointerUp => {
                if ctx.dragging_thumb.is_none() { return None; }
                let focused = ctx.focused_thumb;
                let final_value = *ctx.value.get();
                Some(TransitionPlan::to(match focused {
                        Some(t) => State::Focused { thumb: t },
                        None => State::Idle,
                    }).apply(|ctx| {
                        ctx.dragging_thumb = None;
                    }).with_effect(PendingEffect::new("value-change-end", move |_ctx, props, _send| {
                        if let Some(ref cb) = props.on_value_change_end {
                            cb.call(final_value);
                        }
                        no_cleanup()
                    }))
                )
            }
            Event::Increment { thumb } => {
                if is_thumb_disabled(ctx, *thumb) { return None; }
                let thumb = *thumb;
                let current = match thumb {
                    ThumbIndex::Start => ctx.value.get()[0],
                    ThumbIndex::End => ctx.value.get()[1],
                };
                let next = current + ctx.step;
                Some(TransitionPlan::context_only(move |ctx| {
                    set_thumb_value(ctx, thumb, next);
                }))
            }
            Event::Decrement { thumb } => {
                if is_thumb_disabled(ctx, *thumb) { return None; }
                let thumb = *thumb;
                let current = match thumb {
                    ThumbIndex::Start => ctx.value.get()[0],
                    ThumbIndex::End => ctx.value.get()[1],
                };
                let prev = current - ctx.step;
                Some(TransitionPlan::context_only(move |ctx| {
                    set_thumb_value(ctx, thumb, prev);
                }))
            }
            Event::IncrementLarge { thumb } => {
                if is_thumb_disabled(ctx, *thumb) { return None; }
                let thumb = *thumb;
                let step = ctx.large_step.unwrap_or(ctx.step * 10.0);
                let current = match thumb {
                    ThumbIndex::Start => ctx.value.get()[0],
                    ThumbIndex::End => ctx.value.get()[1],
                };
                let next = current + step;
                Some(TransitionPlan::context_only(move |ctx| {
                    set_thumb_value(ctx, thumb, next);
                }))
            }
            Event::DecrementLarge { thumb } => {
                if is_thumb_disabled(ctx, *thumb) { return None; }
                let thumb = *thumb;
                let step = ctx.large_step.unwrap_or(ctx.step * 10.0);
                let current = match thumb {
                    ThumbIndex::Start => ctx.value.get()[0],
                    ThumbIndex::End => ctx.value.get()[1],
                };
                let prev = current - step;
                Some(TransitionPlan::context_only(move |ctx| {
                    set_thumb_value(ctx, thumb, prev);
                }))
            }
            Event::SetToMin { thumb } => {
                if is_thumb_disabled(ctx, *thumb) { return None; }
                let thumb = *thumb;
                let min = ctx.min;
                Some(TransitionPlan::context_only(move |ctx| {
                    set_thumb_value(ctx, thumb, min);
                }))
            }
            Event::SetToMax { thumb } => {
                if is_thumb_disabled(ctx, *thumb) { return None; }
                let thumb = *thumb;
                let max = ctx.max;
                Some(TransitionPlan::context_only(move |ctx| {
                    set_thumb_value(ctx, thumb, max);
                }))
            }
            Event::SetValues([start, end]) => {
                let s = start.min(*end);
                let e = start.max(*end);
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set([s, e]);
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

fn snap_to_step(value: f64, min: f64, max: f64, step: f64) -> f64 {
    let clamped = value.clamp(min, max);
    let steps_from_min = ((clamped - min) / step).round();
    (min + steps_from_min * step).clamp(min, max)
}
```

### 1.8 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "range-slider"]
pub enum Part {
    Root,
    Label,
    Track,
    Range,
    Thumb { thumb: ThumbIndex },       // which thumb
    Output,
    MarkerGroup,
    Marker { value: f64 },             // mark value
    HiddenInput { thumb: ThumbIndex }, // which thumb
    DraggingIndicator,
    Description,
    ErrorMessage,
}

/// API for the RangeSlider component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    fn percent(&self, value: f64) -> f64 {
        ((value - self.ctx.min) / (self.ctx.max - self.ctx.min) * 100.0).clamp(0.0, 100.0)
    }

    /// Attributes for the root container.
    pub fn root_attrs(&self) -> AttrMap {
        let is_horizontal = self.ctx.orientation == Orientation::Horizontal;
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-orientation"), if is_horizontal { "horizontal" } else { "vertical" });
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.dragging_thumb.is_some() { attrs.set_bool(HtmlAttr::Data("ars-dragging"), true); }
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

    /// Attributes for the range (filled portion between thumbs).
    pub fn range_attrs(&self) -> AttrMap {
        let [start, end] = *self.ctx.value.get();
        let start_pct = self.percent(start);
        let end_pct = self.percent(end);
        let is_horizontal = self.ctx.orientation == Orientation::Horizontal;

        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Range.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set_style(
            if is_horizontal { CssProperty::Left } else { CssProperty::Bottom },
            format!("{}%", start_pct),
        );
        attrs.set_style(
            if is_horizontal { CssProperty::Width } else { CssProperty::Height },
            format!("{}%", end_pct - start_pct),
        );
        attrs
    }

    /// Attributes for a specific thumb.
    pub fn thumb_attrs(&self, thumb: ThumbIndex) -> AttrMap {
        let [start, end] = *self.ctx.value.get();
        let (value, other) = match thumb {
            ThumbIndex::Start => (start, end),
            ThumbIndex::End => (end, start),
        };
        let pct = self.percent(value);
        let is_horizontal = self.ctx.orientation == Orientation::Horizontal;
        let min_gap = self.ctx.min_steps_between as f64 * self.ctx.step;

        let value_text = self.props.format_value.as_ref()
            .map(|f| f((value, other)))
            .unwrap_or_else(|| format!("{}", value));

        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Thumb { thumb }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "slider");
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), value.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), value_text);
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation),
            if is_horizontal { "horizontal" } else { "vertical" });
        match thumb {
            ThumbIndex::Start => {
                attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), self.ctx.min.to_string());
                attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), (end - min_gap).to_string());
                attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.start_label)(&self.ctx.locale));
            }
            ThumbIndex::End => {
                attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), (start + min_gap).to_string());
                attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), self.ctx.max.to_string());
                attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.end_label)(&self.ctx.locale));
            }
        }
        let is_thumb_disabled = is_thumb_disabled(self.ctx, thumb);
        if is_thumb_disabled { attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true"); }
        let is_focused = self.ctx.focused_thumb == Some(thumb);
        attrs.set(HtmlAttr::TabIndex, if is_focused && !is_thumb_disabled { "0" } else { "-1" });
        if self.ctx.focus_visible && is_focused {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        attrs.set(HtmlAttr::Data("ars-index"), match thumb {
            ThumbIndex::Start => "0",
            ThumbIndex::End => "1",
        });
        attrs.set_style(
            if is_horizontal { CssProperty::Left } else { CssProperty::Bottom },
            format!("{}%", pct),
        );
        attrs.set(HtmlAttr::Class, "ars-touch-none");
        attrs
    }

    /// Attributes for the output display.
    pub fn output_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Output.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "off");
        attrs
    }

    /// Attributes for the marker group.
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
        let [start, end] = *self.ctx.value.get();
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Marker { value }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if value >= start && value <= end {
            attrs.set_bool(HtmlAttr::Data("ars-in-range"), true);
        }
        attrs
    }

    /// Attributes for a hidden input (form submission).
    pub fn hidden_input_attrs(&self, thumb: ThumbIndex) -> AttrMap {
        let value = match thumb {
            ThumbIndex::Start => self.ctx.value.get()[0],
            ThumbIndex::End => self.ctx.value.get()[1],
        };
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput { thumb }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(ref name) = self.ctx.name {
            let suffix = match thumb { ThumbIndex::Start => "[0]", ThumbIndex::End => "[1]" };
            attrs.set(HtmlAttr::Name, format!("{name}{suffix}"));
        }
        if let Some(ref form) = self.props.form {
            attrs.set(HtmlAttr::Form, form);
        }
        attrs.set(HtmlAttr::Value, value.to_string());
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
        let is_dragging = self.ctx.dragging_thumb.is_some();
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DraggingIndicator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if is_dragging { "dragging" } else { "idle" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        if !is_dragging {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }
        attrs
    }

    pub fn on_thumb_focus(&self, thumb: ThumbIndex, is_keyboard: bool) {
        (self.send)(Event::Focus { thumb, is_keyboard });
    }
    pub fn on_thumb_blur(&self, thumb: ThumbIndex) {
        (self.send)(Event::Blur { thumb });
    }
    pub fn on_thumb_keydown(&self, thumb: ThumbIndex, data: &KeyboardEventData) {
        let is_rtl = self.ctx.dir == Direction::Rtl;
        let is_horizontal = self.ctx.orientation == Orientation::Horizontal;
        match data.key {
            KeyboardKey::ArrowRight => {
                if is_horizontal && is_rtl { (self.send)(Event::Decrement { thumb }) }
                else { (self.send)(Event::Increment { thumb }) }
            }
            KeyboardKey::ArrowLeft => {
                if is_horizontal && is_rtl { (self.send)(Event::Increment { thumb }) }
                else { (self.send)(Event::Decrement { thumb }) }
            }
            KeyboardKey::ArrowUp => (self.send)(Event::Increment { thumb }),
            KeyboardKey::ArrowDown => (self.send)(Event::Decrement { thumb }),
            KeyboardKey::PageUp => (self.send)(Event::IncrementLarge { thumb }),
            KeyboardKey::PageDown => (self.send)(Event::DecrementLarge { thumb }),
            KeyboardKey::Home => (self.send)(Event::SetToMin { thumb }),
            KeyboardKey::End => (self.send)(Event::SetToMax { thumb }),
            _ => {}
        }
    }
    pub fn on_track_pointerdown(&self, thumb: ThumbIndex, value: f64) {
        (self.send)(Event::PointerDown { thumb, value });
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
            Part::Thumb { thumb } => self.thumb_attrs(thumb),
            Part::Output => self.output_attrs(),
            Part::MarkerGroup => self.marker_group_attrs(),
            Part::Marker { value } => self.marker_attrs(value),
            Part::HiddenInput { thumb } => self.hidden_input_attrs(thumb),
            Part::DraggingIndicator => self.dragging_indicator_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
RangeSlider
├── Root               <div>     data-ars-scope="range-slider" data-ars-part="root"
├── Label              <label>   data-ars-part="label"
├── Track              <div>     data-ars-part="track"
│   ├── Range          <div>     data-ars-part="range" (filled between thumbs)
│   ├── Thumb (Start)  <div>     data-ars-part="thumb" data-ars-index="0" (role="slider")
│   └── Thumb (End)    <div>     data-ars-part="thumb" data-ars-index="1" (role="slider")
├── DraggingIndicator  <div>     data-ars-part="dragging-indicator" (optional, aria-hidden)
├── Output             <output>  data-ars-part="output" (optional)
├── MarkerGroup        <div>     data-ars-part="marker-group" (optional)
│   └── Marker (×N)    <span>    data-ars-part="marker"
├── HiddenInput (×2)   <input>   data-ars-part="hidden-input" (type="hidden")
├── Description        <div>     data-ars-part="description" (optional)
└── ErrorMessage       <div>     data-ars-part="error-message" (optional)
```

| Part              | Element    | Key Attributes                                          |
| ----------------- | ---------- | ------------------------------------------------------- |
| Root              | `<div>`    | `data-ars-scope="range-slider"`, `data-ars-orientation` |
| Label             | `<label>`  | Group label                                             |
| Track             | `<div>`    | Pointer interaction target                              |
| Range             | `<div>`    | Filled region between thumbs                            |
| Thumb             | `<div>`    | `role="slider"`, `aria-valuenow/min/max/text` (×2)      |
| DraggingIndicator | `<div>`    | `aria-hidden`, `data-ars-state` (optional)              |
| Output            | `<output>` | Value display (optional)                                |
| MarkerGroup       | `<div>`    | `role="presentation"` (optional)                        |
| Marker            | `<span>`   | `data-ars-in-range` when between thumbs (optional)      |
| HiddenInput       | `<input>`  | `type="hidden"`, `name[0]`/`name[1]` (×2)               |
| Description       | `<div>`    | Help text; linked via `aria-describedby` (optional)     |
| ErrorMessage      | `<div>`    | Validation error (optional)                             |

Per-thumb disabled state: When `start_disabled` or `end_disabled` is true, the respective thumb sets `aria-disabled="true"` but can still receive focus (for discoverability). Pointer interactions are ignored.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property           | Element           | Value                                                        |
| ------------------ | ----------------- | ------------------------------------------------------------ |
| `role`             | Each Thumb        | `slider`                                                     |
| `aria-valuenow`    | Each Thumb        | Current thumb value                                          |
| `aria-valuemin`    | Start Thumb       | `min`; End Thumb: `start_value + min_gap`                    |
| `aria-valuemax`    | Start Thumb       | `end_value - min_gap`; End Thumb: `max`                      |
| `aria-valuetext`   | Each Thumb        | Formatted value (via `format_value`)                         |
| `aria-label`       | Each Thumb        | From `messages.start_label` / `messages.end_label`           |
| `aria-orientation` | Each Thumb        | `"horizontal"` or `"vertical"`                               |
| `aria-disabled`    | Each Thumb        | When the specific thumb is disabled                          |
| `aria-hidden`      | DraggingIndicator | `"true"` — purely decorative visual feedback during drag     |
| `hidden`           | DraggingIndicator | Present when not dragging (indicator is invisible when idle) |

### 3.2 Keyboard Interaction

Same as Slider, applied to the focused thumb. RTL arrow reversal applies for horizontal orientation.

| Key              | Action                    |
| ---------------- | ------------------------- |
| ArrowRight / Up  | Increment focused thumb   |
| ArrowLeft / Down | Decrement focused thumb   |
| PageUp           | Increment by large step   |
| PageDown         | Decrement by large step   |
| Home             | Set thumb to minimum      |
| End              | Set thumb to maximum      |
| Tab              | Move focus between thumbs |

### 3.3 Focus Management

- Roving tabindex: only the focused thumb has `tabindex="0"`.
- Tab moves focus between thumbs, then out of the slider.
- `touch-action: none` on each thumb prevents scroll interference.

### 3.4 Thumb Focus Announcement

Each thumb has a distinct `aria-label` identifying its role. When `aria-valuetext` is present, screen readers announce the formatted value. Adapters may throttle `aria-label` updates during drag to at most one per 150ms.

## 4. Internationalization

- Same locale resolution as Slider.
- Thumb labels ("Range start", "Range end") localized via `Messages`.
- Output format: "50 – 80" — en-dash and number formatting per locale.
- RTL: Arrow keys reverse for horizontal orientation. Thumb positions mirror visually.

## 5. Form Integration

- **Hidden inputs**: Two hidden `<input type="hidden">` elements are rendered — one per thumb. They carry `name[0]` and `name[1]` with the start and end values.
- **Validation states**: `aria-invalid` can be set on the Root if needed by wrapping in a Field.
- **Reset behavior**: On form reset, the adapter restores values to `default_value`.
- **Disabled propagation**: When inside a `Field` or `Fieldset`, the adapter merges `disabled` from `FieldCtx` per `07-forms.md` §12.6.

## 6. Variant: N-Thumb

The RangeSlider generalizes from 2 thumbs to N thumbs for use cases requiring multiple value points (e.g., audio equalizer, multi-range price filters, color gradient stops).

### 6.1 Additional Props

```rust
pub struct MultiThumbSliderProps {
    /// Current values for each thumb, in sorted order. Length determines thumb count.
    pub values: Vec<f64>,
    /// Per-thumb step values (optional; falls back to shared `step`).
    pub steps: Option<Vec<f64>>,
    /// Per-thumb min constraints.
    pub min_values: Option<Vec<f64>>,
    /// Per-thumb max constraints.
    pub max_values: Option<Vec<f64>>,
}
```

### 6.2 Additional Context

```rust
pub struct MultiThumbContext {
    pub values: Vec<f64>,
    pub dragging_thumb: Option<usize>,
    pub focused_thumb: Option<usize>,
}
```

### 6.3 Behavior

Thumb values must remain in non-descending order. Two modes:

- **Push**: Dragging past an adjacent thumb pushes it along.
- **Block** (default for 2-thumb): Thumbs stop at the adjacent position.

```rust
#[derive(Clone, Debug, PartialEq, Default)]
pub enum ThumbCrossingMode {
    #[default]
    Push,
    Block,
}
```

### 6.4 Additional Events

```rust,no_check
ThumbChange { index: usize, value: f64 },
ThumbDragStart { index: usize },
ThumbDragEnd { index: usize },
ValuesCommit(Vec<f64>),
```

### 6.5 Anatomy Additions

Each thumb is rendered with `data-ars-index="{i}"`. Range segments between consecutive thumbs also carry `data-ars-index`:

| Part        | Multiplicity | Key Attributes                          |
| ----------- | ------------ | --------------------------------------- |
| Thumb       | N            | `data-ars-index="{i}"`, `role="slider"` |
| Range       | N−1          | Between thumb[i] and thumb[i+1]         |
| HiddenInput | N            | `name="{name}[{i}]"`                    |

### 6.6 Accessibility

Each thumb is an independent `role="slider"` with its own `aria-valuenow`, `aria-valuemin` (clamped to neighbour), and `aria-valuemax` (clamped to neighbour). `aria-label` defaults to `"Value {i+1} of {n}"` — localized via the i18n catalog.

## 7. Library Parity

> Compared against: Ark UI (`Slider`), React Aria (`Slider`).
>
> Note: Radix UI `Slider` supports multi-thumb via `value: number[]` but is otherwise documented under the single Slider entry. ars-ui splits single and range into separate components for clarity.

### 7.1 Props

| Feature            | ars-ui                          | Ark UI                   | React Aria            | Notes                       |
| ------------------ | ------------------------------- | ------------------------ | --------------------- | --------------------------- |
| Controlled value   | `value: Option<[f64; 2]>`       | `value: number[]`        | `value: number[]`     | Full parity (typed as pair) |
| Default value      | `default_value: [f64; 2]`       | `defaultValue`           | `defaultValue`        | Full parity                 |
| Min/Max            | `min`/`max`                     | `min`/`max`              | `minValue`/`maxValue` | Full parity                 |
| Step               | `step: f64`                     | `step`                   | `step`                | Full parity                 |
| Min steps between  | `min_steps_between: u32`        | `minStepsBetweenThumbs`  | --                    | Ark parity                  |
| Disabled           | `disabled: bool`                | `disabled`               | `isDisabled`          | Full parity                 |
| Read-only          | `readonly: bool`                | `readOnly`               | --                    | Ark parity                  |
| Invalid            | `invalid: bool`                 | `invalid`                | --                    | Ark parity                  |
| Orientation        | `orientation`                   | `orientation`            | `orientation`         | Full parity                 |
| Direction          | `dir`                           | --                       | --                    | ars-ui specific             |
| Form name          | `name`                          | `name`                   | --                    | Ark parity                  |
| Thumb swap         | `allow_thumb_swap: bool`        | `thumbCollisionBehavior` | --                    | Ark parity (swap mode)      |
| Per-thumb disabled | `start_disabled`/`end_disabled` | --                       | --                    | ars-ui enhancement          |
| Value format       | `format_value`                  | `getAriaValueText`       | `formatOptions`       | Full parity                 |
| On change end      | `on_value_change_end`           | `onValueChangeEnd`       | `onChangeEnd`         | Full parity                 |

**Gaps:** None.

### 7.2 Anatomy

| Part         | ars-ui                              | Ark UI            | React Aria              | Notes                  |
| ------------ | ----------------------------------- | ----------------- | ----------------------- | ---------------------- |
| Root         | `Root`                              | `Root`            | `Slider`                | Full parity            |
| Label        | `Label`                             | `Label`           | `Label`                 | Full parity            |
| Track        | `Track`                             | `Track`           | `SliderTrack`           | Full parity            |
| Range        | `Range`                             | `Range`           | --                      | Ark parity             |
| StartThumb   | `StartThumb`                        | `Thumb` (index 0) | `SliderThumb` (index 0) | Full parity            |
| EndThumb     | `EndThumb`                          | `Thumb` (index 1) | `SliderThumb` (index 1) | Full parity            |
| Output       | `Output`                            | `ValueText`       | `SliderOutput`          | Full parity            |
| HiddenInput  | `StartHiddenInput`/`EndHiddenInput` | `HiddenInput`     | (built-in)              | Full parity            |
| Description  | `Description`                       | --                | --                      | ars-ui form-field part |
| ErrorMessage | `ErrorMessage`                      | --                | --                      | ars-ui form-field part |

**Gaps:** None.

### 7.3 Events

| Callback     | ars-ui                                              | Ark UI             | React Aria    | Notes       |
| ------------ | --------------------------------------------------- | ------------------ | ------------- | ----------- |
| Value change | `Increment`/`Decrement`/`PointerDown`/`PointerMove` | `onValueChange`    | `onChange`    | Full parity |
| Change end   | `on_value_change_end`                               | `onValueChangeEnd` | `onChangeEnd` | Full parity |

**Gaps:** None.

### 7.4 Features

| Feature                | ars-ui                   | Ark UI                                 | React Aria         |
| ---------------------- | ------------------------ | -------------------------------------- | ------------------ |
| Non-crossing invariant | Yes                      | Yes                                    | Yes                |
| Thumb swap on cross    | Yes (`allow_thumb_swap`) | Yes (`thumbCollisionBehavior: 'swap'`) | --                 |
| Per-thumb keyboard     | Yes                      | Yes                                    | Yes                |
| RTL support            | Yes                      | --                                     | --                 |
| N-thumb generalization | Yes (section 6)          | Yes (array values)                     | Yes (array values) |

**Gaps:** None.

### 7.5 Summary

- **Overall:** Full parity with both reference libraries.
- **Divergences:** ars-ui uses a typed `[f64; 2]` pair instead of `number[]`, and splits range into a separate component for type clarity. ars-ui adds `start_disabled`/`end_disabled` for per-thumb disable control.
- **Recommended additions:** None.
