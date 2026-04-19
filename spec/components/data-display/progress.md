---
component: Progress
category: data-display
tier: stateful
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
    ark-ui: Progress
    radix-ui: Progress
    react-aria: ProgressBar
---

# Progress

A progress bar or spinner showing task completion. Supports both determinate (known percent)
and indeterminate (unknown duration) modes.

## 1. State Machine

### 1.1 States

| State      | Description                                               |
| ---------- | --------------------------------------------------------- |
| `Idle`     | Value is set; progress is at rest (e.g. before starting). |
| `Loading`  | Indeterminate or active progress underway.                |
| `Complete` | Value has reached max.                                    |

### 1.2 Events

| Event      | Payload       | Description                           |
| ---------- | ------------- | ------------------------------------- |
| `SetValue` | `Option<f64>` | Update value; `None` = indeterminate. |
| `SetMax`   | `f64`         | Update the maximum bound.             |
| `Complete` | —             | Jump to complete state.               |
| `Reset`    | —             | Return to initial/idle state.         |

### 1.3 Context

```rust
/// Context for the Progress component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current value. None = indeterminate.
    pub value: Bindable<Option<f64>>,
    /// Lower bound (default 0).
    pub min: f64,
    /// Upper bound (default 100).
    pub max: f64,
    /// Layout orientation (horizontal or vertical).
    pub orientation: Orientation,
    /// Derived: value.is_none().
    pub indeterminate: bool,
    /// Derived: ((value - min) / (max - min)) * 100, clamped 0–100.
    pub percent: f64,
    /// Locale for aria-valuetext formatting.
    pub locale: Locale,
    /// Resolved messages for screen-reader announcements.
    pub messages: Messages,
}
```

### 1.4 Props

```rust
/// Layout orientation.
#[derive(Clone, Debug, PartialEq)]
pub enum Orientation {
    /// Horizontal layout (default).
    Horizontal,
    /// Vertical layout.
    Vertical,
}

/// Props for the Progress component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Controlled value.
    pub value: Option<Option<f64>>,
    /// Uncontrolled initial value.
    pub default_value: Option<f64>,
    /// Lower bound (default 0.0).
    pub min: f64,
    /// Upper bound (default 100.0).
    pub max: f64,
    /// Layout orientation. Controls `aria-orientation` and `data-ars-orientation`.
    pub orientation: Orientation,
    /// Formatting options passed to ars-i18n NumberFormatter.
    pub format_options: Option<NumberFormatOptions>,
    // on_value_change callback is registered in the adapter layer, not in Props.
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            min: 0.0,
            max: 100.0,
            orientation: Orientation::Horizontal,
            format_options: None,
        }
    }
}
```

### 1.5 Derived Values

```rust
impl Context {
    /// Computes the percent value from the given value, min, and max.
    pub fn compute_percent(value: Option<f64>, min: f64, max: f64) -> f64 {
        match value {
            None    => 0.0, // indeterminate; percent is meaningless
            Some(v) => ((v - min) / (max - min) * 100.0).clamp(0.0, 100.0),
        }
    }
}
```

### 1.6 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, ComponentIds, AttrMap, Bindable};

/// States for the Progress component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// Value is set; progress is at rest (e.g. before starting).
    Idle,
    /// Indeterminate or active progress underway.
    Loading,
    /// Value has reached max.
    Complete,
}

/// Events for the Progress component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Update value; `None` = indeterminate.
    SetValue(Option<f64>),
    /// Update the maximum bound.
    SetMax(f64),
    /// Jump to complete state.
    Complete,
    /// Return to initial/idle state.
    Reset,
}

/// Machine for the Progress component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let locale = env.locale.clone();
        let messages = messages.clone();
        let val = props.value.unwrap_or(props.default_value);
        let percent = Context::compute_percent(val, props.min, props.max);
        let state = match val {
            None    => State::Loading,
            Some(v) if v >= props.max => State::Complete,
            _       => State::Idle,
        };
        (state, Context {
            value: match props.value {
                Some(v) => Bindable::controlled(v),
                None    => Bindable::uncontrolled(props.default_value),
            },
            min: props.min,
            max: props.max,
            orientation: props.orientation.clone(),
            indeterminate: val.is_none(),
            percent,
            locale,
            messages,
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx:   &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::SetValue(new_val) => {
                let v = *new_val;
                let min = ctx.min;
                let max = ctx.max;
                let new_state = match v {
                    None    => State::Loading,
                    Some(n) if n >= max => State::Complete,
                    _       => State::Idle,
                };
                Some(TransitionPlan::to(new_state).apply(move |ctx| {
                    ctx.value.set(v);
                    ctx.indeterminate = v.is_none();
                    ctx.percent = Context::compute_percent(v, min, max);
                }))
            }
            Event::SetMax(new_max) => {
                let m = *new_max;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.max = m;
                    ctx.percent = Context::compute_percent(*ctx.value.get(), ctx.min, m);
                }))
            }
            Event::Complete => {
                Some(TransitionPlan::to(State::Complete).apply(|ctx| {
                    ctx.value.set(Some(ctx.max));
                    ctx.indeterminate = false;
                    ctx.percent = 100.0;
                }))
            }
            Event::Reset => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.value.set(None);
                    ctx.indeterminate = true;
                    ctx.percent = 0.0;
                }))
            }
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx:   &'a Self::Context,
        props: &'a Self::Props,
        send:  &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "progress"]
pub enum Part {
    Root,
    Label,
    Track,
    Range,
    ValueText,
    CircleTrack,
    CircleRange { radius: f64 },
}

/// API for the Progress component.
pub struct Api<'a> {
    /// Current state of the progress.
    state: &'a State,
    /// Current context of the progress.
    ctx:   &'a Context,
    /// Current props of the progress.
    props: &'a Props,
    /// Send event to the progress.
    send:  &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Returns true if the progress is indeterminate.
    pub fn is_indeterminate(&self) -> bool { self.ctx.indeterminate }
    /// Returns true if the progress is complete.
    pub fn is_complete(&self)      -> bool { *self.state == State::Complete }
    /// Returns the current percent value.
    pub fn percent(&self)          -> f64  { self.ctx.percent }

    /// Returns a locale-formatted value text for aria-valuetext.
    pub fn value_text(&self) -> String {
        if self.ctx.indeterminate {
            return (self.ctx.messages.loading)(&self.ctx.locale);
        }
        if self.is_complete() {
            return (self.ctx.messages.complete)(&self.ctx.locale);
        }
        let fmt = NumberFormatter::new(
            &self.ctx.locale,
            self.props.format_options.clone().unwrap_or_default(),
        );
        format!("{}% complete", fmt.format_percent(self.ctx.percent / 100.0))
    }

    /// Returns the root attributes for the progress.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "progressbar");
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), match self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical   => "vertical",
        });
        attrs.set(HtmlAttr::Data("ars-orientation"), match self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical   => "vertical",
        });
        attrs.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Idle     => "idle",
            State::Loading  => "loading",
            State::Complete => "complete",
        });
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), self.ctx.min.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), self.ctx.max.to_string());
        if !self.ctx.indeterminate {
            if let Some(v) = self.ctx.value.get() {
                attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), v.to_string());
            }
        }
        // When indeterminate, set aria-valuetext to the indeterminate message
        // so screen readers announce a meaningful status (e.g., "Loading…").
        if self.ctx.indeterminate {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), (self.ctx.messages.loading)(&self.ctx.locale));
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), self.value_text());
        }
        attrs
    }

    /// Returns the label attributes for the progress.
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Returns the track attributes for the progress.
    pub fn track_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Track.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Returns the range attributes for the progress.
    pub fn range_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Range.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let pct = if self.ctx.indeterminate { 0.0 } else { self.ctx.percent };
        attrs.set_style(CssProperty::Width, format!("{}%", pct));
        attrs.set(HtmlAttr::Data("ars-indeterminate"), self.ctx.indeterminate.to_string());
        attrs
    }

    /// Returns the value text attributes for the progress.
    pub fn value_text_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ValueText.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    // ── Circular variant ──────────────────────────────────────────────────

    /// Stroke-dashoffset for an SVG circle with the given radius.
    pub fn circle_stroke_dashoffset(&self, radius: f64) -> f64 {
        let circumference = 2.0 * std::f64::consts::PI * radius;
        let pct = if self.ctx.indeterminate { 0.0 } else { self.ctx.percent / 100.0 };
        circumference * (1.0 - pct)
    }

    /// Returns the circle track attributes for the progress.
    pub fn circle_track_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CircleTrack.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Returns the circle range attributes for the progress.
    pub fn circle_range_attrs(&self, radius: f64) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CircleRange.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let offset = self.circle_stroke_dashoffset(radius);
        let circumference = 2.0 * std::f64::consts::PI * radius;
        attrs.set(HtmlAttr::Data("stroke-dasharray"), circumference.to_string());
        attrs.set(HtmlAttr::Data("stroke-dashoffset"), offset.to_string());
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
            Part::ValueText => self.value_text_attrs(),
            Part::CircleTrack => self.circle_track_attrs(),
            Part::CircleRange { radius } => self.circle_range_attrs(radius),
        }
    }
}

/// Messages for the Progress component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Text announced by screen readers when the progress is loading.
    /// Default (en): `"Loading…"`.
    pub loading: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Text announced by screen readers when the progress is complete.
    /// Default (en): `"Complete"`.
    pub complete: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}
impl Default for Messages {
    fn default() -> Self {
        Self {
            loading: MessageFn::static_str("Loading…"),
            complete: MessageFn::static_str("Complete"),
        }
    }
}
impl ComponentMessages for Messages {}

// Note: Percentage values MUST be formatted using `NumberFormatter` with
// `style: Percent` for locale-aware display (e.g. "47 %" in French,
// "47%" in English). See `04-internationalization.md` §NumberFormatter.
```

## 2. Anatomy

```text
Progress
├── Root        (role="progressbar"; data-ars-scope="progress" data-ars-part="root")
├── Label       (associated label text)
├── Track       (background bar container)
├── Range       (filled portion; width = percent%)
├── ValueText   (formatted percentage/value display)
└── [Circular variant]
    ├── Circle       (<svg> wrapper)
    ├── CircleTrack  (<circle> background)
    └── CircleRange  (<circle> filled arc via stroke-dashoffset)
```

| Part          | Element              | Key Attributes                                                                                                                                          |
| ------------- | -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Root`        | `<div>`              | `role="progressbar"`, `aria-valuenow`, `aria-valuemin`, `aria-valuemax`, `aria-valuetext`, `aria-orientation`, `data-ars-state`, `data-ars-orientation` |
| `Label`       | `<label>` / `<span>` | Associated via `aria-labelledby` on Root                                                                                                                |
| `Track`       | `<div>`              | Visual background                                                                                                                                       |
| `Range`       | `<div>`              | `style="width: N%"`, `data-ars-indeterminate`                                                                                                           |
| `ValueText`   | `<span>`             | Human-readable percentage                                                                                                                               |
| `Circle`      | `<svg>`              | SVG wrapper for circular variant                                                                                                                        |
| `CircleTrack` | `<circle>`           | Background arc                                                                                                                                          |
| `CircleRange` | `<circle>`           | Foreground arc; `stroke-dashoffset` drives fill                                                                                                         |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- `role="progressbar"` on Root.
- `aria-valuenow` omitted when `indeterminate=true` (value is unknown).
- `aria-valuetext` is locale-formatted: "47% complete", "Loading…", or "Complete".
- `aria-orientation` set to `"horizontal"` or `"vertical"` reflecting the `orientation` prop.
- Associate a visible label with `aria-label` or `aria-labelledby`.

## 4. Internationalization

- `aria-valuetext` uses `NumberFormatter::format_percent()` from `ars-i18n` for locale-aware
  percentage formatting (e.g. "47 %" in French, "47%" in English). When locale
  is inherited from `ArsProvider`, adapters should derive the formatter through
  `use_number_formatter(...)`.
- "Loading…" and "Complete" strings come from `Messages` and should be supplied
  by the host application from a message catalog.

> **No Hardcoded English Default Strings.** All default message strings
> displayed to users or announced to screen readers MUST come from locale-aware message
> catalogs rather than hardcoded English. This applies to:
>
> - `aria-valuetext` content (e.g., "47% complete", "Loading…", "Complete")
> - `aria-label` content on interactive elements
> - `aria-describedby` content
> - `role="alert"` text announcements
>
> **Canonical pattern:** Every component's `XyzMessages` struct provides English defaults
> via `Default` impl for zero-config usage, but adapters MUST allow overriding these
> per-locale via the `ComponentMessages` trait and `ArsProvider` context. The
> `Messages` struct is the reference pattern — all data-display components
> (Badge, Stat, TagGroup, RatingGroup, Table, GridList) follow the same approach.
>
> **Audit checklist for data-display components:**
>
> - ProgressBar: `"Loading…"`, `"Complete"` in `aria-valuetext` → `Messages`
> - Badge: overflow label `"99+"` → `BadgeMessages::overflow_label`
> - Stat: `"increase"`, `"decrease"`, `"no change"` → `StatMessages::change_label`
> - TagGroup: `"Remove"` button label → `TagGroupMessages::remove_label`
> - Table: `"Sort ascending"`, `"Sort descending"` → `TableMessages`
> - RatingGroup: `"{n} of {max} stars"` → `RatingGroupMessages`

## 5. Library Parity

> Compared against: Ark UI (`Progress`), Radix UI (`Progress`), React Aria (`ProgressBar`).

### 5.1 Props

| Feature          | ars-ui                    | Ark UI                       | Radix UI         | React Aria                 | Notes                                                        |
| ---------------- | ------------------------- | ---------------------------- | ---------------- | -------------------------- | ------------------------------------------------------------ |
| `value`          | `Option<Option<f64>>`     | `number`                     | `number \| null` | `number`                   | ars-ui wraps in double-option for controlled + indeterminate |
| `default_value`  | `Option<f64>`             | `number` (50)                | --               | --                         | Only Ark UI has a default value                              |
| `min` / `max`    | `f64`                     | `number`                     | `max` only       | `minValue` / `maxValue`    | ars-ui and Ark UI have both bounds                           |
| `indeterminate`  | Derived from `value=None` | --                           | `value=null`     | `isIndeterminate`          | Different representations, same behavior                     |
| `format_options` | `NumberFormatOptions`     | `NumberFormatOptions`        | --               | `Intl.NumberFormatOptions` | ars-ui and Ark UI match                                      |
| `orientation`    | `Orientation`             | `'horizontal' \| 'vertical'` | --               | --                         | Added from Ark UI                                            |
| `locale`         | `Option<Locale>`          | `string`                     | --               | --                         | ars-ui and Ark UI both support locale                        |
| `getValueLabel`  | --                        | --                           | `fn(value, max)` | `valueLabel: ReactNode`    | ars-ui covers via `Messages` + `value_text()`                |

**Gaps:** None.

### 5.2 Anatomy

| Part              | ars-ui                       | Ark UI                                 | Radix UI    | React Aria    | Notes                                  |
| ----------------- | ---------------------------- | -------------------------------------- | ----------- | ------------- | -------------------------------------- |
| Root              | `Root`                       | `Root`                                 | `Root`      | `ProgressBar` | --                                     |
| Label             | `Label`                      | `Label`                                | --          | `Label`       | Radix has no explicit label part       |
| Track             | `Track`                      | `Track`                                | --          | --            | ars-ui and Ark UI match                |
| Range / Indicator | `Range`                      | `Range`                                | `Indicator` | --            | Same concept, different names          |
| ValueText         | `ValueText`                  | `ValueText`                            | --          | --            | ars-ui and Ark UI match                |
| Circle\*          | `CircleTrack`, `CircleRange` | `Circle`, `CircleTrack`, `CircleRange` | --          | --            | ars-ui matches Ark UI circular variant |

**Gaps:** None.

### 5.3 Events

| Callback          | ars-ui        | Ark UI          | Radix UI | React Aria | Notes                                              |
| ----------------- | ------------- | --------------- | -------- | ---------- | -------------------------------------------------- |
| `on_value_change` | Adapter layer | `onValueChange` | --       | --         | ars-ui handles via Bindable observation in adapter |

**Gaps:** None.

### 5.4 Features

| Feature                  | ars-ui | Ark UI | Radix UI           | React Aria              |
| ------------------------ | ------ | ------ | ------------------ | ----------------------- |
| Determinate              | Yes    | Yes    | Yes                | Yes                     |
| Indeterminate            | Yes    | Yes    | Yes (`value=null`) | Yes (`isIndeterminate`) |
| Circular variant         | Yes    | Yes    | No                 | No                      |
| Locale-aware formatting  | Yes    | Yes    | No                 | Yes                     |
| Orientation (horiz/vert) | Yes    | Yes    | No                 | No                      |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** Radix UI uses `Indicator` instead of `Track`+`Range`; ars-ui follows Ark UI's more granular anatomy. Radix `getValueLabel` callback is covered by ars-ui's `Messages` pattern.
- **Recommended additions:** None.
