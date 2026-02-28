---
component: DateRangeField
category: date-time
tier: stateful
foundation_deps: [architecture, accessibility, i18n, interactions, forms]
shared_deps: [date-time-types]
related: [date-field, date-range-picker]
references:
  react-aria: DateRangePicker
---

# DateRangeField

Inline two-field range input **without a popover**. Wraps two DateField instances with shared validation.

## 1. State Machine

### 1.1 States

```rust
/// States for the DateRangeField component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// The component is idle.
    Idle,
    /// The start field is focused.
    StartFocused,
    /// The end field is focused.
    EndFocused,
}
```

### 1.2 Events

```rust
/// Events for the DateRangeField component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Focus moved to the start field.
    FocusStart,
    /// Focus moved to the end field.
    FocusEnd,
    /// Focus left the entire component.
    BlurAll,
    /// Set the full range programmatically.
    SetRange(Option<DateRange>),
    /// Start field value changed.
    StartValueChange(Option<CalendarDate>),
    /// End field value changed.
    EndValueChange(Option<CalendarDate>),
}
```

### 1.3 Context

```rust
/// Context for the DateRangeField component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    pub value: Bindable<Option<DateRange>>,
    /// The active field.
    pub active_field: Option<ActiveField>,
    /// The minimum date.
    pub min: Option<CalendarDate>,
    /// The maximum date.
    pub max: Option<CalendarDate>,
    /// The locale.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is readonly.
    pub readonly: bool,
    /// Whether the component is required.
    pub required: bool,
    /// The name of the component.
    pub name: Option<String>,
    /// Separate form name for start date.
    pub start_name: Option<String>,
    /// Separate form name for end date.
    pub end_name: Option<String>,
    /// Component IDs.
    pub ids: ComponentIds,
    /// Whether the Description part is rendered.
    pub has_description: bool,
    /// Whether the ErrorMessage part is rendered.
    pub has_error_message: bool,
    /// When true, all numeric segments in child DateField instances display
    /// with leading zeros (e.g., "03" instead of "3").
    pub force_leading_zeros: bool,
}

// Reuses ActiveField from DateRangePicker:
// pub enum ActiveField { Start, End }
```

### 1.4 Props

```rust
/// Props for the DateRangeField component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The ID of the component.
    pub id: String,
    /// The value of the component.
    pub value: Option<Option<DateRange>>,
    /// The default value of the component.
    pub default_value: Option<DateRange>,
    /// The minimum date.
    pub min: Option<CalendarDate>,
    /// The maximum date.
    pub max: Option<CalendarDate>,
    /// The locale.
    pub locale: Option<Locale>,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is readonly.
    pub readonly: bool,
    /// Whether the component is required.
    pub required: bool,
    /// The name of the component (single hidden input with range value).
    pub name: Option<String>,
    /// Form field name for the start date (alternative to `name`).
    /// When set, submits the start date as a separate form field.
    pub start_name: Option<String>,
    /// Form field name for the end date (alternative to `name`).
    /// When set, submits the end date as a separate form field.
    pub end_name: Option<String>,
    /// Internationalized messages. When `None`, resolved via `resolve_messages()`.
    pub messages: Option<Messages>,
    /// When true, all numeric segments in both start and end fields display
    /// with leading zeros (e.g., "03" instead of "3"). Defaults to false,
    /// which uses locale-aware formatting. Passed through to child DateField
    /// instances.
    pub force_leading_zeros: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            min: None,
            max: None,
            locale: None,
            disabled: false,
            readonly: false,
            required: false,
            name: None,
            start_name: None,
            end_name: None,
            messages: None,
            force_leading_zeros: false,
        }
    }
}
```

### 1.5 Guards

```rust
fn is_disabled(ctx: &Context) -> bool { ctx.disabled }
fn is_readonly(ctx: &Context) -> bool { ctx.readonly }
```

### 1.6 Full Machine Implementation

```rust
/// Machine for the DateRangeField component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props) -> (Self::State, Self::Context) {
        let locale = resolve_locale(props.locale.as_ref());
        let ctx = Context {
            value: match &props.value {
                Some(v) => Bindable::controlled(v.clone()),
                None => Bindable::uncontrolled(props.default_value.clone()),
            },
            active_field: None,
            min: props.min.clone(),
            max: props.max.clone(),
            messages: resolve_messages::<Messages>(props.messages.as_ref(), &locale),
            locale,
            disabled: props.disabled,
            readonly: props.readonly,
            required: props.required,
            name: props.name.clone(),
            start_name: props.start_name.clone(),
            end_name: props.end_name.clone(),
            ids: ComponentIds::from_id(&props.id),
            has_description: false,
            has_error_message: false,
            force_leading_zeros: props.force_leading_zeros,
        };
        (State::Idle, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if is_disabled(ctx) { return None; }

        match event {
            Event::FocusStart => {
                Some(TransitionPlan::to(State::StartFocused).apply(|ctx| {
                    ctx.active_field = Some(ActiveField::Start);
                }))
            }

            Event::FocusEnd => {
                Some(TransitionPlan::to(State::EndFocused).apply(|ctx| {
                    ctx.active_field = Some(ActiveField::End);
                }))
            }

            Event::BlurAll => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.active_field = None;
                }))
            }

            Event::SetRange(range) => {
                let range = range.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(range);
                }))
            }

            Event::StartValueChange(date) => {
                let date = date.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    match (&date, ctx.value.get().as_ref().map(|r| &r.end)) {
                        (Some(start), Some(end)) => {
                            ctx.value.set(Some(DateRange::normalized(
                                start.clone(), end.clone(),
                            )));
                        }
                        (Some(start), None) => {
                            // Only start set; store partial (no complete range yet)
                            // Range will complete when end is also set.
                        }
                        (None, _) => {
                            ctx.value.set(None);
                        }
                    }
                }))
            }

            Event::EndValueChange(date) => {
                let date = date.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    match (ctx.value.get().as_ref().map(|r| &r.start), &date) {
                        (Some(start), Some(end)) => {
                            ctx.value.set(Some(DateRange::normalized(
                                start.clone(), end.clone(),
                            )));
                        }
                        (None, Some(end)) => {
                            // Only end set; store partial.
                        }
                        (_, None) => {
                            ctx.value.set(None);
                        }
                    }
                }))
            }
        }
    }
}
```

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "date-range-field"]
pub enum Part {
    Root,
    Label,
    StartField,
    Separator,
    EndField,
    Description,
    ErrorMessage,
    HiddenInput,
}

/// API for the DateRangeField component.
pub struct Api<'a> {
    /// The state of the component.
    state: &'a State,
    /// The context of the component.
    ctx: &'a Context,
    /// The props of the component.
    props: &'a Props,
    /// The send function.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    // ── AttrMap getters ──────────────────────────────────────────────────

    /// Attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        attrs.set(HtmlAttr::Role, "group");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));
        attrs.set(HtmlAttr::Data("ars-state"), self.state_name());
        let mut described_by = Vec::new();
        if self.ctx.has_description { described_by.push(self.ctx.ids.part("description")); }
        if self.ctx.has_error_message { described_by.push(self.ctx.ids.part("error-message")); }
        if !described_by.is_empty() {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), described_by.join(" "));
        }
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }
        if self.ctx.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }
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

    /// Attributes for the separator element.
    pub fn separator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Separator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// The separator text between start and end fields (e.g., " – ").
    /// The adapter renders this as the text content of the Separator element.
    pub fn separator_text(&self) -> String {
        (self.ctx.messages.separator_text)(&self.ctx.locale)
    }

    /// Screen reader description of the full date range (e.g., "March 1 to March 15").
    /// Returns `None` if no complete range is selected.
    /// The adapter can use this as an `aria-description` on the root or as a live announcement.
    pub fn range_description(&self) -> Option<String> {
        let range = self.ctx.value.get().as_ref()?;
        let start = format_date(&range.start, &self.ctx.format, &self.ctx.locale);
        let end = format_date(&range.end, &self.ctx.format, &self.ctx.locale);
        Some((self.ctx.messages.range_description)(&start, &end, &self.ctx.locale))
    }

    /// Build DateField Props for the start date input.
    pub fn start_field_props(&self) -> date_field::Props {
        date_field::Props {
            id: self.ctx.ids.part("start"),
            value: self.ctx.value.get().as_ref().map(|r| Some(r.start.clone())),
            locale: self.ctx.locale.clone(),
            min: self.ctx.min.clone(),
            max: self.ctx.value.get().as_ref().map(|r| r.end.clone()),
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            aria_label: Some((self.ctx.messages.start_label)(&self.ctx.locale)),
            force_leading_zeros: self.ctx.force_leading_zeros,
            ..Default::default()
        }
    }

    /// Build DateField Props for the end date input.
    pub fn end_field_props(&self) -> date_field::Props {
        date_field::Props {
            id: self.ctx.ids.part("end"),
            value: self.ctx.value.get().as_ref().map(|r| Some(r.end.clone())),
            locale: self.ctx.locale.clone(),
            min: self.ctx.value.get().as_ref().map(|r| r.start.clone()),
            max: self.ctx.max.clone(),
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            aria_label: Some((self.ctx.messages.end_label)(&self.ctx.locale)),
            force_leading_zeros: self.ctx.force_leading_zeros,
            ..Default::default()
        }
    }

    /// Attributes for the description element.
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
        attrs.set(HtmlAttr::Role, "alert");
        attrs
    }

    /// Attributes for the hidden input element.
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(ref name) = self.ctx.name {
            attrs.set(HtmlAttr::Name, name);
        }
        let iso_value = match self.ctx.value.get() {
            Some(range) => format!(
                "{}/{}",
                format_iso(&range.start),
                format_iso(&range.end),
            ),
            None => String::new(),
        };
        attrs.set(HtmlAttr::Value, iso_value);
        attrs
    }

    /// Attributes for a separate hidden input carrying the start date.
    /// Only rendered when `start_name` is set on Props.
    pub fn start_hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(name) = &self.ctx.start_name {
            attrs.set(HtmlAttr::Name, name);
        }
        if let Some(range) = &self.ctx.value.get() {
            attrs.set(HtmlAttr::Value, format_iso(&range.start));
        }
        attrs
    }

    /// Attributes for a separate hidden input carrying the end date.
    /// Only rendered when `end_name` is set on Props.
    pub fn end_hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(name) = &self.ctx.end_name {
            attrs.set(HtmlAttr::Name, name);
        }
        if let Some(range) = &self.ctx.value.get() {
            attrs.set(HtmlAttr::Value, format_iso(&range.end));
        }
        attrs
    }

    // ── Convenience getters ─────────────────────────────────────────────

    /// Get the selected range.
    pub fn selected_range(&self) -> Option<&DateRange> {
        self.ctx.value.get().as_ref()
    }

    /// Whether either field is currently focused.
    pub fn is_focused(&self) -> bool {
        !matches!(self.state, State::Idle)
    }

    /// Get the name of the current state.
    fn state_name(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::StartFocused => "start-focused",
            State::EndFocused => "end-focused",
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::StartField => self.start_field_props().into(), // delegated to DateField
            Part::Separator => self.separator_attrs(),
            Part::EndField => self.end_field_props().into(), // delegated to DateField
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
DateRangeField (en-US)
└── Root                                  role="group"  data-ars-scope="date-range-field"
    ├── Label                             "Trip dates"
    ├── StartField (DateField)            role="group"  aria-label="Start date"
    │   ├── Segment (Month)               role="spinbutton"  aria-label="Month"
    │   ├── Literal "/"                   aria-hidden="true"
    │   ├── Segment (Day)                 role="spinbutton"  aria-label="Day"
    │   ├── Literal "/"                   aria-hidden="true"
    │   └── Segment (Year)               role="spinbutton"  aria-label="Year"
    ├── Separator                         " – "  aria-hidden="true"
    ├── EndField (DateField)              role="group"  aria-label="End date"
    │   ├── Segment (Month)               role="spinbutton"  aria-label="Month"
    │   ├── Literal "/"                   aria-hidden="true"
    │   ├── Segment (Day)                 role="spinbutton"  aria-label="Day"
    │   ├── Literal "/"                   aria-hidden="true"
    │   └── Segment (Year)               role="spinbutton"  aria-label="Year"
    ├── Description                       "Select check-in and check-out dates"
    ├── ErrorMessage                      role="alert"
    └── HiddenInput                       type="hidden"  value="2025-06-01/2025-06-15"
```

| Part           | Element                 | Description                                                    |
| -------------- | ----------------------- | -------------------------------------------------------------- |
| `Root`         | `<div>`                 | `role="group"` — groups both fields; `aria-labelledby` → Label |
| `Label`        | `<label>`               | Shared label for the entire range                              |
| `StartField`   | _(DateField)_           | Start date segments (delegated to DateField)                   |
| `Separator`    | `<span>`                | `aria-hidden="true"` — visual separator (" -- " or "to")       |
| `EndField`     | _(DateField)_           | End date segments (delegated to DateField)                     |
| `Description`  | `<div>`                 | Optional help text; referenced by `aria-describedby`           |
| `ErrorMessage` | `<div role="alert">`    | Validation error text; announced immediately                   |
| `HiddenInput`  | `<input type="hidden">` | ISO 8601 range value for form submission                       |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part       | Role/Attribute     | Details                                                      |
| ---------- | ------------------ | ------------------------------------------------------------ |
| Root       | `role="group"`     | `aria-labelledby` points to Label                            |
| StartField | --                 | Delegates to DateField; each segment has `role="spinbutton"` |
| EndField   | --                 | Delegates to DateField; each segment has `role="spinbutton"` |
| Root       | `aria-disabled`    | Present when disabled                                        |
| Root       | `aria-readonly`    | Present when readonly                                        |
| Root       | `data-ars-invalid` | Present when validation fails                                |
| Root       | `aria-describedby` | Wired to Description and ErrorMessage part IDs               |
| Root       | `aria-required`    | Present when required                                        |

### 3.2 Keyboard Interaction

| Key        | Action                                                          |
| ---------- | --------------------------------------------------------------- |
| Tab        | Move focus from last segment of StartField to first of EndField |
| Shift+Tab  | Move focus from first segment of EndField to last of StartField |
| ArrowUp    | Increment focused segment value                                 |
| ArrowDown  | Decrement focused segment value                                 |
| ArrowLeft  | Move to previous segment (within current field)                 |
| ArrowRight | Move to next segment (within current field)                     |

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Label for the start date field.
    pub start_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the end date field.
    pub end_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Text displayed between start and end fields.
    pub separator_text: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Description of the range for screen readers.
    pub range_description: MessageFn<dyn Fn(&str, &str, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            start_label: MessageFn::static_str("Start date"),
            end_label: MessageFn::static_str("End date"),
            separator_text: MessageFn::static_str(" \u{2013} "),
            range_description: MessageFn::new(|start, end, _locale| {
                format!("{} to {}", start, end)
            }),
        }
    }
}

impl ComponentMessages for Messages {}
```

- Label text and separator are consumer-overridable via Messages.
- `data-ars-state` values are stable API tokens, not localized.
- RTL: segment ordering within each field follows locale conventions (delegated to DateField).

## 5. Form Integration

- Hidden `<input>` submits the current range in ISO 8601 interval format (`YYYY-MM-DD/YYYY-MM-DD`).
- `name` attribute is set from Props.
- Reset restores `default_value`.
- Validation states (`valid`, `invalid`) reflected via `data-ars-invalid` on Root.
- `aria-describedby` wires to Description and ErrorMessage parts.
- `aria-required` set when `required` is true.
- Disabled/readonly propagation from form context per `07-forms.md` S15.

## 6. Library Parity

> Compared against: React Aria (`DateRangePicker` field-only aspects).

React Aria does not have a standalone `DateRangeField` component separate from `DateRangePicker`. The closest equivalent is the inline portion of `DateRangePicker` (two `DateInput` slots without the popover). ars-ui separates the range field into its own component for composability.

### 6.1 Props

| Feature               | ars-ui                | React Aria                  | Notes                          |
| --------------------- | --------------------- | --------------------------- | ------------------------------ |
| Controlled range      | `value`               | `value` (RangeValue)        | Equivalent                     |
| Default range         | `default_value`       | `defaultValue`              | Equivalent                     |
| Min/max               | `min`, `max`          | `minValue`, `maxValue`      | Equivalent                     |
| Disabled              | `disabled`            | `isDisabled`                | Equivalent                     |
| Read-only             | `readonly`            | `isReadOnly`                | Equivalent                     |
| Required              | `required`            | `isRequired`                | Equivalent                     |
| Locale                | `locale`              | -- (context)                | Equivalent                     |
| Name                  | `name`                | `startName`, `endName`      | React Aria uses separate names |
| Force leading zeros   | `force_leading_zeros` | `shouldForceLeadingZeros`   | Equivalent                     |
| Allows non-contiguous | --                    | `allowsNonContiguousRanges` | React Aria feature             |
| Start/end names       | --                    | `startName`, `endName`      | Separate form field names      |

**Gaps:** None.

### 6.2 Anatomy

| Part          | ars-ui                   | React Aria                 | Notes                          |
| ------------- | ------------------------ | -------------------------- | ------------------------------ |
| Root          | `Root`                   | `DateRangePicker`          | Equivalent                     |
| Label         | `Label`                  | `Label`                    | Equivalent                     |
| Start field   | `StartField` (DateField) | `DateInput` (slot="start") | Equivalent                     |
| Separator     | `Separator`              | --                         | ars-ui explicit separator part |
| End field     | `EndField` (DateField)   | `DateInput` (slot="end")   | Equivalent                     |
| Description   | `Description`            | `Text` (slot)              | Equivalent                     |
| Error message | `ErrorMessage`           | `FieldError`               | Equivalent                     |
| Hidden input  | `HiddenInput`            | --                         | ars-ui form integration        |

**Gaps:** None.

### 6.3 Events

| Callback     | ars-ui                                | React Aria      | Notes      |
| ------------ | ------------------------------------- | --------------- | ---------- |
| Range change | `StartValueChange` / `EndValueChange` | `onChange`      | Equivalent |
| Focus        | `FocusStart` / `FocusEnd`             | `onFocusChange` | Equivalent |
| Blur         | `BlurAll`                             | `onBlur`        | Equivalent |

**Gaps:** None.

### 6.4 Features

| Feature                    | ars-ui | React Aria |
| -------------------------- | ------ | ---------- |
| Two-field range input      | Yes    | Yes        |
| Range normalization        | Yes    | Yes        |
| Cross-field tab navigation | Yes    | Yes        |
| Hidden form input          | Yes    | No         |
| Separate form names        | No     | Yes        |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity.
- **Divergences:** React Aria bundles the range field into `DateRangePicker`; ars-ui separates it for composability. ars-ui adds a `Separator` part and hidden form input.
- **Recommended additions:** None.

## Appendix: Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_idle() {
        let (state, ctx) = Machine::init(&Props::default());
        assert_eq!(state, State::Idle);
        assert!(ctx.active_field.is_none());
    }

    #[test]
    fn focus_tracking() {
        let props = Props::default();
        let (state, ctx) = Machine::init(&props);
        let plan = Machine::transition(&state, &Event::FocusStart, &ctx, &props).unwrap();
        assert_eq!(plan.target, Some(State::StartFocused));

        let plan = Machine::transition(&State::StartFocused, &Event::FocusEnd, &ctx, &props).unwrap();
        assert_eq!(plan.target, Some(State::EndFocused));

        let plan = Machine::transition(&State::EndFocused, &Event::BlurAll, &ctx, &props).unwrap();
        assert_eq!(plan.target, Some(State::Idle));
    }

    #[test]
    fn range_validation_normalizes() {
        let props = Props::default();
        let (state, mut ctx) = Machine::init(&props);
        // Set end first, then start after it — should normalize
        let end = CalendarDate::new_gregorian(2025, nzu8(1), nzu8(1));
        let start = CalendarDate::new_gregorian(2025, nzu8(3), nzu8(15));
        // Simulates setting start > end; normalized should swap
    }

    #[test]
    fn disabled_ignores_events() {
        let props = Props { disabled: true, ..Props::default() };
        let (state, ctx) = Machine::init(&props);
        assert!(Machine::transition(&state, &Event::FocusStart, &ctx, &props).is_none());
    }

    #[test]
    fn set_range_updates_value() {
        let props = Props::default();
        let (state, ctx) = Machine::init(&props);
        let range = DateRange::new(
            CalendarDate::new_gregorian(2025, nzu8(6), nzu8(1)),
            CalendarDate::new_gregorian(2025, nzu8(6), nzu8(30)),
        );
        let plan = Machine::transition(
            &state,
            &Event::SetRange(Some(range)),
            &ctx,
            &props,
        ).unwrap();
        assert!(plan.target.is_none()); // context-only
    }
}
```
