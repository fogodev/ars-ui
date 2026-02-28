---
component: RatingGroup
category: data-display
tier: stateful
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
  ark-ui: RatingGroup
---

# RatingGroup

An interactive or read-only star rating widget. Supports whole-number and half-star
increments. When interactive, it behaves as a composite keyboard widget (radio group or
slider).

## 1. State Machine

### 1.1 States

| State                       | Description                               |
| --------------------------- | ----------------------------------------- |
| `Idle`                      | No hover or keyboard focus on any item.   |
| `Focused { index: usize }`  | Keyboard focus is on the item at `index`. |
| `Hovering { index: usize }` | Pointer is hovering over item at `index`. |

### 1.2 Events

| Event             | Payload                           | Description                                                    |
| ----------------- | --------------------------------- | -------------------------------------------------------------- |
| `Rate`            | `f64`                             | Commit a rating value.                                         |
| `HoverItem`       | `usize`                           | Pointer entered star at index.                                 |
| `UnHover`         | —                                 | Pointer left the rating control.                               |
| `Focus`           | `index: usize, is_keyboard: bool` | Focus moved to a specific star.                                |
| `Blur`            | —                                 | Focus left the control.                                        |
| `IncrementRating` | —                                 | Increase value by `props.step` (keyboard ArrowRight/ArrowUp).  |
| `DecrementRating` | —                                 | Decrease value by `props.step` (keyboard ArrowLeft/ArrowDown). |
| `ClearRating`     | —                                 | Reset to 0 (keyboard Home).                                    |

### 1.3 Context

```rust
/// Context for the RatingGroup component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Committed rating value.
    pub value: Bindable<f64>,
    /// Value suggested by hover; None when not hovering.
    pub hovered_value: Option<f64>,
    /// Index of focused star; None when not focused.
    pub focused_index: Option<usize>,
    /// True when focus is keyboard-initiated (drives visible focus ring).
    pub focus_visible: bool,
    /// Number of stars (default 5).
    pub count: NonZero<u32>,
    /// Allow 0.5 increments.
    pub allow_half: bool,
    /// Read-only display mode.
    pub readonly: bool,
    /// Disabled display mode.
    pub disabled: bool,
    /// Active locale for message formatting.
    pub locale: Locale,
    /// Resolved messages for the rating group.
    pub messages: Messages,
}

/// Messages for the RatingGroup component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Returns the label for a given value, e.g. "1 star", "2 stars", "2.5 stars".
    pub item_label: MessageFn<dyn Fn(f64, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            item_label: MessageFn::new(|v, _locale| {
                if (v - 1.0).abs() < f64::EPSILON { format!("{v} star") }
                else { format!("{v} stars") }
            }),
        }
    }
}

impl ComponentMessages for Messages {}
```

### 1.4 Props

```rust
/// Props for the RatingGroup component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Committed rating value.
    pub value: Option<f64>,
    /// Default rating value.
    pub default_value: f64,
    /// Number of stars (default 5).
    pub count: NonZero<u32>,
    /// Whether to allow half-star ratings. Sugar for `step: 0.5`.
    /// If both `allow_half` and `step` are set, `step` takes precedence.
    pub allow_half: bool, // default: false
    /// Step increment for keyboard navigation and click resolution.
    /// Default: `1.0`. Setting `allow_half: true` is sugar for `step: 0.5`.
    /// Allows fractional ratings (e.g., `step: 0.25` for quarter-star ratings).
    pub step: f64, // default: 1.0
    /// Read-only display mode.
    pub readonly: bool,
    /// Disabled display mode.
    pub disabled: bool,
    /// Whether the rating group is required.
    pub required: bool,
    /// The name for form submission.
    pub name: Option<String>,
    /// The ID of the form element the component is associated with.
    pub form: Option<String>,
    /// Optional locale override. When `None`, resolved from the nearest `ArsProvider` context.
    pub locale: Option<Locale>,
    /// Localizable messages for the rating group.
    pub messages: Option<Messages>,
    // on_value_change callback is registered in the adapter layer, not in Props.
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: 0.0,
            count: NonZero::new(5).expect("non-zero"),
            allow_half: false,
            step: 1.0,
            readonly: false,
            disabled: false,
            required: false,
            name: None,
            form: None,
            locale: None,
            messages: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, ComponentIds, AttrMap, Bindable};

/// States for the RatingGroup component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// No hover or keyboard focus on any item.
    Idle,
    /// Keyboard focus is on the item at `index`.
    Focused { index: usize },
    /// Pointer is hovering over item at `index`.
    Hovering { index: usize },
}

/// Events for the RatingGroup component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Commit a rating value.
    Rate(f64),
    /// Pointer entered star at index.
    HoverItem(usize),
    /// Pointer left the rating control.
    UnHover,
    /// Focus moved to a specific star.
    Focus { index: usize, is_keyboard: bool },
    /// Focus left the control.
    Blur,
    /// Increase value by `props.step` (keyboard ArrowRight/ArrowUp).
    IncrementRating,
    /// Decrease value by `props.step` (keyboard ArrowLeft/ArrowDown).
    DecrementRating,
    /// Reset to 0 (keyboard Home).
    ClearRating,
}

/// Machine for the RatingGroup component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        (State::Idle, Context {
            value: match props.value {
                Some(v) => Bindable::controlled(v),
                None    => Bindable::uncontrolled(props.default_value),
            },
            hovered_value:  None,
            focused_index:  None,
            focus_visible:  false,
            count:          props.count,
            allow_half:     props.allow_half,
            readonly:       props.readonly,
            disabled:       props.disabled,
            locale,
            messages,
        })
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx:   &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.readonly || ctx.disabled {
            // Read-only/disabled: only allow Focus/Blur for AT
            return match event {
                Event::Focus { index, is_keyboard } if !ctx.disabled => {
                    let idx = *index;
                    let kb  = *is_keyboard;
                    Some(TransitionPlan::to(State::Focused { index: idx }).apply(move |ctx| {
                        ctx.focused_index = Some(idx);
                        ctx.focus_visible = kb;
                    }))
                }
                Event::Blur => {
                    Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                        ctx.focused_index = None;
                        ctx.focus_visible = false;
                    }))
                }
                _ => None,
            };
        }

        match event {
            // ── Commit rating ─────────────────────────────────────────────
            Event::Rate(new_val) => {
                let v = new_val.clamp(0.0, ctx.count.get() as f64);
                let v = if ctx.allow_half { (v * 2.0).round() / 2.0 } else { v.round() };
                Some(TransitionPlan::to(State::Idle).apply(move |ctx| {
                    ctx.value.set(v);
                }))
                // Value change notification is handled by the adapter layer
                // (which observes Bindable changes), not by the machine.
            }

            // ── Hover ─────────────────────────────────────────────────────
            Event::HoverItem(index) => {
                let idx = *index;
                // Hover value: 1-based star index
                let hv = (idx + 1) as f64;
                Some(TransitionPlan::to(State::Hovering { index: idx }).apply(move |ctx| {
                    ctx.hovered_value = Some(hv);
                }))
            }

            Event::UnHover => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.hovered_value = None;
                }))
            }

            // ── Focus ─────────────────────────────────────────────────────
            Event::Focus { index, is_keyboard } => {
                let idx = *index;
                let kb  = *is_keyboard;
                Some(TransitionPlan::to(State::Focused { index: idx }).apply(move |ctx| {
                    ctx.focused_index = Some(idx);
                    ctx.focus_visible = kb;
                }))
            }

            Event::Blur => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.focused_index = None;
                    ctx.focus_visible = false;
                    ctx.hovered_value = None;
                }))
            }

            // ── Keyboard navigation ───────────────────────────────────────
            Event::IncrementRating => {
                // Use props.step; allow_half is sugar for step: 0.5
                let step = if props.step != 1.0 { props.step } else if ctx.allow_half { 0.5 } else { 1.0 };
                let current = *ctx.value.get();
                let max_val = ctx.count.get() as f64;
                let new_val = (current + step).min(max_val);
                Self::transition(state, &Event::Rate(new_val), ctx, props)
            }

            Event::DecrementRating => {
                // Use props.step; allow_half is sugar for step: 0.5
                let step = if props.step != 1.0 { props.step } else if ctx.allow_half { 0.5 } else { 1.0 };
                let current = *ctx.value.get();
                let new_val = (current - step).max(0.0);
                Self::transition(state, &Event::Rate(new_val), ctx, props)
            }

            Event::ClearRating => {
                Self::transition(state, &Event::Rate(0.0), ctx, props)
            }
        }
    }

    fn connect<'a>(
        state: &'a State,
        ctx:   &'a Context,
        props: &'a Props,
        send:  &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "rating-group"]
pub enum Part {
    Root,
    Label,
    Control,
    Item { index: usize },
    HiddenInput,
}

/// API for the RatingGroup component.
pub struct Api<'a> {
    /// Current state of the rating group.
    state: &'a State,
    /// Current context of the rating group.
    ctx:   &'a Context,
    /// Current props of the rating group.
    props: &'a Props,
    /// Send event to the rating group.
    send:  &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// The display value: hovered (preview) or committed.
    pub fn display_value(&self) -> f64 {
        self.ctx.hovered_value.unwrap_or_else(|| *self.ctx.value.get())
    }

    /// Returns true if the item at `index` is highlighted.
    pub fn is_item_highlighted(&self, index: usize) -> bool {
        let star_val = (index + 1) as f64;
        self.display_value() >= star_val
    }

    /// Returns true if the item at `index` is selected.
    pub fn is_item_selected(&self, index: usize) -> bool {
        let star_val = (index + 1) as f64;
        *self.ctx.value.get() >= star_val
    }

    /// Returns the label attributes for the rating group.
    pub fn label_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p
    }

    /// Returns the root attributes for the rating group.
    pub fn root_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        if self.ctx.disabled { p.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.readonly { p.set_bool(HtmlAttr::Data("ars-readonly"), true); }
        p
    }

    /// Returns the control attributes for the rating group.
    pub fn control_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        // Use slider pattern for half-ratings; radio group for whole-number only
        if self.ctx.allow_half {
            p.set(HtmlAttr::Role, "slider");
            p.set(HtmlAttr::Aria(AriaAttr::ValueMin), "0");
            p.set(HtmlAttr::Aria(AriaAttr::ValueMax), self.ctx.count.get().to_string());
            p.set(HtmlAttr::Aria(AriaAttr::ValueNow), self.ctx.value.get().to_string());
            let label = (self.ctx.messages.item_label)(*self.ctx.value.get(), &self.ctx.locale);
            p.set(HtmlAttr::Aria(AriaAttr::ValueText), label);
            p.set(HtmlAttr::TabIndex, if self.ctx.disabled { "-1" } else { "0" });
            // Event handlers (keydown for arrow key navigation) are typed methods on the Api struct.
        } else {
            p.set(HtmlAttr::Role, "radiogroup");
        }
        if self.ctx.disabled {
            p.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        // Event handlers (mouseleave for unhover) are typed methods on the Api struct.
        p
    }

    /// Returns the item attributes for the rating group.
    pub fn item_attrs(&self, index: usize) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item { index }.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Data("ars-index"), index.to_string());
        let highlighted = self.is_item_highlighted(index);
        let selected    = self.is_item_selected(index);
        if highlighted { p.set_bool(HtmlAttr::Data("ars-highlighted"), true); }
        if selected { p.set_bool(HtmlAttr::Data("ars-selected"), true); }

        if !self.ctx.allow_half {
            // Each star is a radio button
            let star_val = (index + 1) as f64;
            p.set(HtmlAttr::Role, "radio");
            let label = (self.ctx.messages.item_label)(star_val, &self.ctx.locale);
            p.set(HtmlAttr::Aria(AriaAttr::Label), label);
            p.set(HtmlAttr::Aria(AriaAttr::Checked), if selected { "true" } else { "false" });
            p.set(HtmlAttr::TabIndex, if selected || (index == 0 && *self.ctx.value.get() == 0.0) {
                "0"
            } else { "-1" });

            // Event handlers (click, hover, focus, blur, keydown) are typed methods on the Api struct.
        }
        p
    }

    /// Returns the hidden input attributes for the rating group.
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Type, "hidden");
        if let Some(ref name) = self.props.name {
            p.set(HtmlAttr::Name, name);
        }
        p.set(HtmlAttr::Value, self.ctx.value.get().to_string());
        if self.props.required { p.set_bool(HtmlAttr::Required, true); }
        if let Some(ref form) = self.props.form {
            p.set(HtmlAttr::Form, form);
        }
        p
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Control => self.control_attrs(),
            Part::Item { index } => self.item_attrs(index),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
RatingGroup
├── Root           (container; data-ars-scope="rating-group" data-ars-part="root")
├── Label          (visible label text)
├── Control        (role="radiogroup" or role="slider")
│   ├── Item       (each star slot; data-ars-index, data-ars-highlighted, data-ars-selected)
│   │   ├── ItemHalf  (left half — half-rating mode only)
│   │   └── ItemFull  (full star icon)
│   └── ...
└── HiddenInput    (<input type="hidden"> for form submission)
```

| Part          | Element                 | Key Attributes                                                                                              |
| ------------- | ----------------------- | ----------------------------------------------------------------------------------------------------------- |
| `Root`        | `<div>`                 | `data-ars-disabled`, `data-ars-readonly`                                                                    |
| `Label`       | `<label>`               | `data-ars-scope="rating-group"` `data-ars-part="label"`                                                     |
| `Control`     | `<div>`                 | `role="radiogroup"` or `role="slider"`                                                                      |
| `Item`        | `<span>`                | `role="radio"`, `aria-label`, `aria-checked`, `data-ars-index`, `data-ars-highlighted`, `data-ars-selected` |
| `ItemHalf`    | `<span>`                | Half-star hit area (half-rating mode)                                                                       |
| `ItemFull`    | `<span>`                | Full-star hit area                                                                                          |
| `HiddenInput` | `<input type="hidden">` | Form submission value                                                                                       |

RatingGroup items in the half-filled state emit `data-ars-half` (presence attribute) for CSS targeting of half-star styling.

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

RatingGroup uses one of two ARIA patterns depending on whether half-ratings are enabled:

#### 3.1.1 Radio group pattern (whole numbers)

- `Control`: `role="radiogroup"`.
- Each `Item`: `role="radio"`, `aria-label="{N} stars"`, `aria-checked`.
- Roving tabindex: only the currently selected (or first, if none) item has `tabindex="0"`.
- Arrow keys (Left/Right or Up/Down) move selection.
- Home clears rating (tabindex moves to first item).
- End sets maximum rating.

#### 3.1.2 Slider pattern (half-ratings)

- `Control`: `role="slider"`, `aria-valuemin="0"`, `aria-valuemax="{count}"`,
  `aria-valuenow="{value}"`, `aria-valuetext="{N} of {count} stars"`.
- Single focusable element; arrow keys adjust the value.

#### 3.1.3 Disabled / Read-only

- `aria-disabled="true"` on `Control` when disabled.
- Items receive `tabindex="-1"` when disabled.
- In readonly mode items are not interactive but remain focusable for AT.

## 4. Internationalization

- Star labels use `PluralCategory` from `ars-i18n` for correct singular/plural forms:

```rust
use ars_i18n::{PluralRules, PluralCategory};

/// Returns the default item label for the rating group.
fn default_item_label(locale: &Locale) -> MessageFn<dyn Fn(f64) -> String + Send + Sync> {
    let rules = PluralRules::new(locale, PluralType::Cardinal);
    MessageFn::new(move |value: f64| {
        let category = rules.category_for(value);
        match category {
            PluralCategory::One => format!("{} star", value),
            _                   => format!("{} stars", value),
        }
    })
}
```

- In RTL locales the star order is visually reversed with CSS `direction: rtl`; keyboard
  arrow semantics remain LTR (ArrowRight = higher value) unless overridden by the host.

## 5. Library Parity

> Compared against: Ark UI (`RatingGroup`).

### 5.1 Props

| Feature                     | ars-ui                | Ark UI              | Notes                                  |
| --------------------------- | --------------------- | ------------------- | -------------------------------------- |
| `value` / `default_value`   | `Option<f64>` / `f64` | `number` / `number` | Equivalent                             |
| `count`                     | `NonZero<u32>`        | `number` (5)        | Equivalent                             |
| `allow_half`                | `bool`                | `boolean`           | Equivalent                             |
| `step`                      | `f64`                 | --                  | ars-ui original for fractional ratings |
| `disabled`                  | `bool`                | `boolean`           | Equivalent                             |
| `readonly`                  | `bool`                | `boolean`           | Equivalent                             |
| `required`                  | `bool`                | `boolean`           | Equivalent                             |
| `name`                      | `Option<String>`      | `string`            | Equivalent                             |
| `form`                      | `Option<String>`      | `string`            | Equivalent                             |
| `autoFocus`                 | --                    | `boolean`           | Omitted; adapter-level concern         |
| `translations` / `messages` | `Messages`            | `IntlTranslations`  | Equivalent                             |

**Gaps:** None.

### 5.2 Anatomy

| Part        | ars-ui        | Ark UI        | Notes                                         |
| ----------- | ------------- | ------------- | --------------------------------------------- |
| Root        | `Root`        | `Root`        | --                                            |
| Label       | `Label`       | `Label`       | Added to Part enum                            |
| Control     | `Control`     | `Control`     | --                                            |
| Item        | `Item`        | `Item`        | --                                            |
| HiddenInput | `HiddenInput` | `HiddenInput` | --                                            |
| ItemContext | --            | `ItemContext` | Ark UI provider pattern; not needed in ars-ui |

**Gaps:** None.

### 5.3 Events

| Callback          | ars-ui                               | Ark UI          | Notes                                                                 |
| ----------------- | ------------------------------------ | --------------- | --------------------------------------------------------------------- |
| `on_value_change` | Adapter layer (Bindable observation) | `onValueChange` | Equivalent                                                            |
| `on_hover_change` | `HoverItem`/`UnHover` events         | `onHoverChange` | ars-ui exposes via state transitions; adapter can derive the callback |

**Gaps:** None.

### 5.4 Features

| Feature                     | ars-ui                    | Ark UI |
| --------------------------- | ------------------------- | ------ |
| Whole-number rating         | Yes (radio group pattern) | Yes    |
| Half-star rating            | Yes (slider pattern)      | Yes    |
| Fractional ratings (`step`) | Yes                       | No     |
| Read-only                   | Yes                       | Yes    |
| Disabled                    | Yes                       | Yes    |
| Form submission             | Yes (`HiddenInput`)       | Yes    |
| Hover preview               | Yes                       | Yes    |
| Keyboard navigation         | Yes                       | Yes    |

**Gaps:** None.

### 5.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui adds a `step` prop for arbitrary fractional ratings beyond half-star. Ark UI's `autoFocus` is omitted as an adapter-level concern. Ark UI's `ItemContext` provider pattern is not needed in ars-ui's architecture.
- **Recommended additions:** None.
