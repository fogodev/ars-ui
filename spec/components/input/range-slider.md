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
    /// Synchronize the externally controlled value prop.
    SyncValue(Option<[f64; 2]>),
    /// Synchronize output-affecting props stored in context.
    SetProps,
    /// Track whether a Description part is rendered.
    SetHasDescription(bool),
    /// Track whether a Label part is rendered.
    SetHasLabel(bool),
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
    /// When true, dragging past the opposite thumb swaps active thumb.
    pub allow_thumb_swap: bool,
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
    /// Pending drag value used for controlled commit callbacks.
    pub drag_value: Option<[f64; 2]>,
    /// Whether the active drag changed the effective value.
    pub drag_changed: bool,
    /// How the thumbs align with the track boundaries.
    pub thumb_alignment: ThumbAlignment,
    /// The name attribute for form submission.
    pub name: Option<String>,
    /// The ID of the form element the hidden inputs are associated with.
    pub form: Option<String>,
    /// Whether the start thumb is individually disabled.
    pub start_disabled: bool,
    /// Whether the end thumb is individually disabled.
    pub end_disabled: bool,
    /// Whether a Description part is rendered (gates aria-describedby).
    pub has_description: bool,
    /// Whether a Label part is rendered (gates aria-labelledby).
    pub has_label: bool,
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
    pub format_value: Option<Callback<dyn Fn((f64, f64)) -> String + Send + Sync>>,
    /// How the thumbs align with the track ends. See `slider::ThumbAlignment`.
    pub thumb_alignment: ThumbAlignment,
    /// Callback fired when value-changing user intent requests a new range.
    pub on_value_change: Option<Callback<dyn Fn([f64; 2]) + Send + Sync>>,
    /// Callback fired when a drag interaction ends (pointerup), as opposed to
    /// continuous change callbacks. Receives the final `[start, end]` value pair.
    pub on_value_change_end: Option<Callback<dyn Fn([f64; 2]) + Send + Sync>>,
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
            on_value_change: None,
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

The RangeSlider maintains these invariants at all times:

- Values are finite. Non-finite event values are rejected; non-finite props
  normalize to the nearest safe bounded value.
- Bounds are normalized before value math, so reversed `min`/`max` props behave
  like their sorted pair.
- Values snap to the step grid when `step` is finite and positive.
- `start <= end`.
- `min_steps_between` is enforced when the requested gap is representable within
  the normalized bounds.
- Whole-component `disabled` and `readonly` block value-changing events.
- Per-thumb disabled blocks value-changing events for that thumb only.
- `allow_thumb_swap` can change the active thumb only during pointer drag.

### 1.6 Drag-Past Behavior

When the user drags a thumb past the other during a pointer interaction:

- **Clamp (default)**: The dragged thumb is clamped so it cannot exceed the other thumb's position (minus `min_steps_between`). The user must release and grab the other thumb.
- **Swap (opt-in)**: When `allow_thumb_swap: true`, dragging past causes the active thumb identity to swap. The previously-dragged thumb stays at the crossover point and the other thumb becomes the drag target.

In both modes, the machine fires `on_value_change` with the corrected `[start, end]`
values when the effective range changes, maintaining `start <= end`. The
`on_value_change_end` callback fires for committed keyboard/programmatic changes
and at pointer-up only when the drag changed the effective value.

### 1.7 Machine Contract

```rust
/// Machine for the RangeSlider component.
#[derive(Debug)]
pub struct Machine;

/// Typed identifier for side effects emitted by the range-slider machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Invoke `Props::on_value_change`.
    ValueChange,
    /// Invoke `Props::on_value_change_end`.
    ValueChangeEnd,
}
```

The machine initializes from `value` when controlled and from `default_value`
otherwise, normalizing the selected pair before storing it in context.

Transition rules:

- `Focus` is ignored when the whole component is disabled; otherwise it records
  `focused_thumb` and `focus_visible`.
- `Blur` clears focus-visible state and clears any active drag bookkeeping.
- `PointerDown` and `PointerMove` require a finite adapter-supplied value and a
  value-change-enabled thumb.
- `PointerMove` is ignored unless a thumb is currently dragging.
- `PointerUp` returns to the focused thumb state when present, otherwise idle.
  It emits `Effect::ValueChangeEnd` only when the drag changed the effective
  value, using `drag_value` so controlled drags commit the pending value rather
  than the stale controlled prop.
- Keyboard and programmatic value events snap, clamp, sort, and enforce the
  minimum representable gap without swapping thumb identity.
- `SetValues` normalizes both values and emits both change and change-end
  effects only when the effective value changes.
- `SyncValue` syncs controlled mode and normalizes finite controlled values;
  non-finite controlled values fall back to the current bounded value.
- `SetProps` updates output-affecting context fields, resnaps the current value,
  and preserves active drag bookkeeping across prop changes.
- `SetHasDescription` and `SetHasLabel` gate `aria-describedby` and
  `aria-labelledby`.

### 1.8 Connect / API

```rust,no_check
/// Anatomy parts emitted by the RangeSlider connect API.
#[derive(Clone, Copy, Debug)]
pub enum Part {
    Root,
    Label,
    Track,
    Range,
    Thumb { thumb: ThumbIndex },
    Output,
    MarkerGroup,
    Marker { value: f64 },
    HiddenInput { thumb: ThumbIndex },
    DraggingIndicator,
    Description,
    ErrorMessage,
}

/// API for the RangeSlider component.
#[derive(Clone, Copy, Debug)]
pub struct Api<'a> {
    pub state: &'a State,
    pub ctx: &'a Context,
    pub props: &'a Props,
    pub send: &'a dyn Fn(Event),
}

impl Api<'_> {
    pub fn root_attrs(&self) -> AttrMap;
    pub fn label_attrs(&self) -> AttrMap;
    pub fn track_attrs(&self) -> AttrMap;
    pub fn range_attrs(&self) -> AttrMap;
    pub fn thumb_attrs(&self, thumb: ThumbIndex) -> AttrMap;
    pub fn output_attrs(&self) -> AttrMap;
    pub fn marker_group_attrs(&self) -> AttrMap;
    pub fn marker_attrs(&self, value: f64) -> AttrMap;
    pub fn hidden_input_attrs(&self, thumb: ThumbIndex) -> AttrMap;
    pub fn description_attrs(&self) -> AttrMap;
    pub fn error_message_attrs(&self) -> AttrMap;
    pub fn dragging_indicator_attrs(&self) -> AttrMap;

    pub fn on_thumb_focus(&self, thumb: ThumbIndex, is_keyboard: bool);
    pub fn on_thumb_blur(&self, thumb: ThumbIndex);
    pub fn on_thumb_keydown(&self, thumb: ThumbIndex, key: KeyboardKey, shift: bool);
    pub fn on_track_pointerdown(&self, thumb: ThumbIndex, value: f64);
    pub fn on_track_pointermove(&self, value: f64);
    pub fn on_pointerup(&self);
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap;
}
```

`Part` is implemented manually rather than with `derive(ComponentPart)` because
`Marker { value: f64 }` must compare and hash by `to_bits()` so distinct marker
values and NaN payloads remain stable in attribute dispatch.

The range and thumb styles mirror in horizontal RTL. Vertical orientation always
uses bottom-to-top geometry and is not affected by `dir`.

```rust,no_check
pub fn value_from_pointer(
    pointer: slider::SliderPointer,
    track: Rect,
    ctx: &Context,
) -> Option<f64>;
```

`value_from_pointer` maps adapter-supplied pointer and track geometry into a
snapped logical value. It returns `None` when the relevant pointer coordinate,
track origin, or track size is non-finite, or when the relevant track size is
zero or negative.

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

Disabled state:

- Whole-component `disabled` removes both thumbs from keyboard interaction and
  blocks all value-changing events.
- Per-thumb disabled state (`start_disabled` / `end_disabled`) sets
  `aria-disabled="true"` on that thumb and blocks value-changing pointer and
  keyboard events for that thumb.
- Per-thumb disabled thumbs remain focusable when adapter focus reaches them so
  assistive technology can discover the disabled state.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property            | Element           | Value                                                        |
| ------------------- | ----------------- | ------------------------------------------------------------ |
| `role`              | Each Thumb        | `slider`                                                     |
| `aria-valuenow`     | Each Thumb        | Current thumb value                                          |
| `aria-valuemin`     | Start Thumb       | `min`; End Thumb: `start_value + effective gap`              |
| `aria-valuemax`     | Start Thumb       | `end_value - effective gap`; End Thumb: `max`                |
| `aria-valuetext`    | Each Thumb        | Formatted value (via `format_value`)                         |
| `aria-label`        | Each Thumb        | From `messages.start_label` / `messages.end_label`           |
| `aria-orientation`  | Each Thumb        | `"horizontal"` or `"vertical"`                               |
| `aria-disabled`     | Each Thumb        | When the specific thumb is disabled                          |
| `aria-invalid`      | Each Thumb        | `"true"` when `invalid=true`                                 |
| `aria-errormessage` | Each Thumb        | Points to ErrorMessage id when `invalid=true`                |
| `aria-hidden`       | DraggingIndicator | `"true"` — purely decorative visual feedback during drag     |
| `hidden`            | DraggingIndicator | Present when not dragging (indicator is invisible when idle) |

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
- The agnostic core stores focus intent and active thumb identity only; adapters
  resolve live element handles and perform any framework-specific focus calls.

### 3.4 Thumb Focus Announcement

Each thumb has a distinct `aria-label` identifying its role. When `aria-valuetext` is present, screen readers announce the formatted value. Adapters may throttle `aria-label` updates during drag to at most one per 150ms.

## 4. Internationalization

- Same locale resolution as Slider.
- Thumb labels ("Range start", "Range end") localized via `Messages`.
- Output format: "50 – 80" — en-dash and number formatting per locale.
- RTL: Arrow keys reverse for horizontal orientation. Thumb positions mirror visually.

## 5. Form Integration

- **Hidden inputs**: Two hidden `<input type="hidden">` elements are rendered — one per thumb. They carry `name[0]` and `name[1]` with the start and end values.
- **Validation states**: when `invalid=true`, each Thumb exposes `aria-invalid="true"` and `aria-errormessage` pointing to the ErrorMessage id. A surrounding `Field` may additionally mark the Root invalid.
- **Reset behavior**: On form reset, the adapter restores values to `default_value`.
- **Disabled propagation**: When inside a `Field` or `Fieldset`, the adapter merges `disabled` from `FieldCtx` per `07-forms.md` §12.6.
- **Geometry boundary**: Core pointer helpers accept adapter-supplied pointer and
  track geometry. The core never reads DOM layout, resolves DOM IDs to elements,
  or performs direct focus.

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
| On value change    | `on_value_change`               | `onValueChange`          | `onChange`            | Full parity                 |
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

| Callback     | ars-ui                | Ark UI             | React Aria    | Notes       |
| ------------ | --------------------- | ------------------ | ------------- | ----------- |
| Value change | `on_value_change`     | `onValueChange`    | `onChange`    | Full parity |
| Change end   | `on_value_change_end` | `onValueChangeEnd` | `onChangeEnd` | Full parity |

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
