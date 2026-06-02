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

`Context` derives only `Clone` and implements `Debug` manually, because
`intl_backend` is an `Arc<dyn IntlBackend>` that cannot derive `Debug`/`PartialEq`
(the `Machine` trait requires `Context: Clone + Debug` only).

The public `value` is the _derived complete_ range — `Some` only when both
`start_date` and `end_date` are set, always normalized so `start <= end`. The two
fields are tracked independently so a range can be assembled incrementally as each
child field changes (an `Option<DateRange>` alone cannot hold a partial, one-sided
value).

```rust
/// Context for the DateRangeField component.
#[derive(Clone)]
pub struct Context {
    /// The derived complete range (Some only when both fields are set, always normalized).
    pub value: Bindable<Option<DateRange>>,
    /// The start field's current value, tracked independently.
    pub start_date: Option<CalendarDate>,
    /// The end field's current value, tracked independently.
    pub end_date: Option<CalendarDate>,
    /// The active field.
    pub active_field: Option<ActiveField>,
    /// The minimum date.
    pub min: Option<CalendarDate>,
    /// The maximum date.
    pub max: Option<CalendarDate>,
    /// The locale.
    pub locale: Locale,
    /// Backend used for locale-dependent labels in range descriptions.
    pub intl_backend: Arc<dyn IntlBackend>,
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

impl Context {
    /// Returns `true` when the current range violates the min/max bounds.
    /// The stored range is always normalized (start <= end), so the only source
    /// of invalidity is a start earlier than `min` or an end later than `max`.
    pub fn is_invalid(&self) -> bool {
        let Some(range) = self.value.get() else { return false };
        let below_min = self.min.as_ref().is_some_and(|min| {
            matches!(range.start.compare_within_calendar(min), Some(Ordering::Less))
        });
        let above_max = self.max.as_ref().is_some_and(|max| {
            matches!(range.end.compare_within_calendar(max), Some(Ordering::Greater))
        });
        below_min || above_max
    }
}

/// Identifies which of the two child fields is currently active.
///
/// Owned by `DateRangeField` as the first range component to require it; later
/// range components (e.g., `DateRangePicker`) reuse this enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveField {
    /// The start field is active.
    Start,
    /// The end field is active.
    End,
}
```

### 1.4 Props

```rust
/// Props for the DateRangeField component.
#[derive(Clone, Debug, Default, PartialEq, HasId)]
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
    /// When true, all numeric segments in both start and end fields display
    /// with leading zeros (e.g., "03" instead of "3"). Defaults to false,
    /// which uses locale-aware formatting. Passed through to child DateField
    /// instances.
    pub force_leading_zeros: bool,
    /// Whether a Description element is rendered. When true, the root's
    /// `aria-describedby` references the description part id.
    pub has_description: bool,
    /// Whether an ErrorMessage element is rendered. When true, the root's
    /// `aria-describedby` references the error-message part id.
    pub has_error_message: bool,
}
```

`Props` derives `Default` (all fields are themselves `Default`). Each field also
has a `#[must_use]` builder method following the workspace convention.

### 1.5 Guards

Guards are inlined in `transition`: a disabled component ignores every event
(`if ctx.disabled { return None }`), and a read-only component ignores the
value-mutating events (`SetRange`, `StartValueChange`, `EndValueChange`) while
still allowing focus tracking.

### 1.6 Full Machine Implementation

```rust
/// Machine for the DateRangeField component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let value = match &props.value {
            Some(v) => Bindable::controlled(v.clone()),
            None => Bindable::uncontrolled(props.default_value.clone()),
        };
        let initial_range = value.get().clone();
        let ctx = Context {
            start_date: initial_range.as_ref().map(|r| r.start.clone()),
            end_date: initial_range.as_ref().map(|r| r.end.clone()),
            value,
            active_field: None,
            min: props.min.clone(),
            max: props.max.clone(),
            locale: env.locale.clone(),
            intl_backend: Arc::clone(&env.intl_backend),
            messages: messages.clone(),
            disabled: props.disabled,
            readonly: props.readonly,
            required: props.required,
            name: props.name.clone(),
            start_name: props.start_name.clone(),
            end_name: props.end_name.clone(),
            ids: ComponentIds::from_id(&props.id),
            has_description: props.has_description,
            has_error_message: props.has_error_message,
            force_leading_zeros: props.force_leading_zeros,
        };
        (State::Idle, ctx)
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled { return None; }

        match event {
            Event::FocusStart => Some(TransitionPlan::to(State::StartFocused)
                .apply(|ctx: &mut Context| ctx.active_field = Some(ActiveField::Start))),

            Event::FocusEnd => Some(TransitionPlan::to(State::EndFocused)
                .apply(|ctx: &mut Context| ctx.active_field = Some(ActiveField::End))),

            Event::BlurAll => Some(TransitionPlan::to(State::Idle)
                .apply(|ctx: &mut Context| ctx.active_field = None)),

            Event::SetRange(range) => {
                if ctx.readonly { return None; }
                let range = range.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.start_date = range.as_ref().map(|r| r.start.clone());
                    ctx.end_date = range.as_ref().map(|r| r.end.clone());
                    ctx.value.set(range);
                }))
            }

            Event::StartValueChange(date) => {
                if ctx.readonly { return None; }
                let date = date.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.start_date = date;
                    recompute_range(ctx);
                }))
            }

            Event::EndValueChange(date) => {
                if ctx.readonly { return None; }
                let date = date.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.end_date = date;
                    recompute_range(ctx);
                }))
            }
        }
    }
}

/// Recomputes the derived complete range from the two field values. When both
/// fields hold a value the range is normalized (swapping if needed so
/// `start <= end`) and the normalized order is reflected back into the fields.
/// When either field is empty the range is incomplete and the value is cleared.
fn recompute_range(ctx: &mut Context) {
    match (ctx.start_date.clone(), ctx.end_date.clone()) {
        (Some(start), Some(end)) => {
            if let Some(range) = DateRange::normalized(start, end) {
                ctx.start_date = Some(range.start.clone());
                ctx.end_date = Some(range.end.clone());
                ctx.value.set(Some(range));
            }
        }
        _ => ctx.value.set(None),
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
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }
        if self.ctx.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }
        if self.ctx.is_invalid() {
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
        }
        attrs
    }

    /// Returns `true` when the current range violates the min/max bounds.
    pub fn is_invalid(&self) -> bool { self.ctx.is_invalid() }

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

    /// Marker attributes (scope + part data hooks) for the start-field wrapper.
    /// The embedded child DateField is configured via `start_field_props`.
    pub fn start_field_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::StartField.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Marker attributes (scope + part data hooks) for the end-field wrapper.
    pub fn end_field_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::EndField.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Screen reader description of the full date range (e.g., "March 1, 2025 to
    /// March 15, 2025"). Returns `None` if no complete range is selected.
    /// The adapter can use this as an `aria-description` on the root or as a live announcement.
    pub fn range_description(&self) -> Option<String> {
        let range = self.ctx.value.get().as_ref()?;
        let start = format_date_label(&range.start, self.ctx.intl_backend.as_ref(), &self.ctx.locale);
        let end = format_date_label(&range.end, self.ctx.intl_backend.as_ref(), &self.ctx.locale);
        Some((self.ctx.messages.range_description)(&start, &end, &self.ctx.locale))
    }

    /// Build DateField Props for the start date input. The start field's lower
    /// bound is the global `min`; its upper bound is the current end date when
    /// set, otherwise the global `max`, so both global bounds and cross-field
    /// coordination apply.
    pub fn start_field_props(&self) -> date_field::Props {
        date_field::Props {
            id: self.ctx.ids.part("start"),
            value: Some(self.ctx.start_date.clone()),
            min_value: self.ctx.min.clone(),
            max_value: self.ctx.end_date.clone().or_else(|| self.ctx.max.clone()),
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            required: self.ctx.required,
            aria_label: Some((self.ctx.messages.start_label)(&self.ctx.locale)),
            force_leading_zeros: self.ctx.force_leading_zeros,
            ..Default::default()
        }
    }

    /// Build DateField Props for the end date input. The end field's upper bound
    /// is the global `max`; its lower bound is the current start date when set,
    /// otherwise the global `min`.
    pub fn end_field_props(&self) -> date_field::Props {
        date_field::Props {
            id: self.ctx.ids.part("end"),
            value: Some(self.ctx.end_date.clone()),
            min_value: self.ctx.start_date.clone().or_else(|| self.ctx.min.clone()),
            max_value: self.ctx.max.clone(),
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            required: self.ctx.required,
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
        // `DateRange::to_iso8601` already emits the `start/end` interval form.
        let iso_value = match self.ctx.value.get() {
            Some(range) => range.to_iso8601(),
            None => String::new(),
        };
        attrs.set(HtmlAttr::Value, iso_value);
        attrs
    }

    /// Attributes for a separate hidden input carrying the start date.
    /// Only meaningful when `start_name` is set on Props.
    pub fn start_hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(name) = &self.ctx.start_name {
            attrs.set(HtmlAttr::Name, name);
        }
        let value = match self.ctx.value.get() {
            Some(range) => range.start.to_iso8601(),
            None => String::new(),
        };
        attrs.set(HtmlAttr::Value, value);
        attrs
    }

    /// Attributes for a separate hidden input carrying the end date.
    /// Only meaningful when `end_name` is set on Props.
    pub fn end_hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(name) = &self.ctx.end_name {
            attrs.set(HtmlAttr::Name, name);
        }
        let value = match self.ctx.value.get() {
            Some(range) => range.end.to_iso8601(),
            None => String::new(),
        };
        attrs.set(HtmlAttr::Value, value);
        attrs
    }

    // Event dispatch helpers called by adapters: `focus_start`, `focus_end`,
    // `blur`, `set_range`, `set_start_value`, `set_end_value` — each forwards
    // the matching `Event` through `send`.

    // ── Convenience getters ─────────────────────────────────────────────

    /// Get the selected range.
    pub fn selected_range(&self) -> Option<&DateRange> {
        self.ctx.value.get().as_ref()
    }

    /// The active field, if either currently holds focus.
    pub fn active_field(&self) -> Option<ActiveField> { self.ctx.active_field }

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
            Part::StartField => self.start_field_attrs(), // wrapper markers; child via start_field_props
            Part::Separator => self.separator_attrs(),
            Part::EndField => self.end_field_attrs(), // wrapper markers; child via end_field_props
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}

/// Formats a single date as a human-readable label for screen-reader range
/// descriptions (e.g., "March 1, 2025").
fn format_date_label(date: &CalendarDate, backend: &dyn IntlBackend, locale: &Locale) -> String {
    format!("{} {}, {}", backend.month_long_name(date.month(), locale), date.day(), date.year())
}
```

The `StartField` and `EndField` parts carry only the wrapper scope/part data
hooks. The embedded child `DateField` components are configured through the
`start_field_props` / `end_field_props` methods, whose `role="group"` sub-group
and per-field `aria-label` come from the child machine — keeping semantic data
separate from the rendered child view per the API design standards. Live segment
focus and caret behavior are resolved by the adapter against native handles keyed
by logical segment, not by these string ids.

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
/// Closure type for the range description message (formatted start, formatted
/// end, locale). A type alias keeps the multi-arg `MessageFn` within clippy's
/// `type_complexity` budget, matching the convention used by color components.
type RangeDescriptionFn = dyn Fn(&str, &str, &Locale) -> String + Send + Sync;

#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Label for the start date field.
    pub start_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the end date field.
    pub end_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Text displayed between start and end fields.
    pub separator_text: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Description of the range for screen readers.
    pub range_description: MessageFn<RangeDescriptionFn>,
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

`CalendarDate::new_gregorian(year, month, day)` takes plain `u8` month/day and
returns `Result`; tests use a `date(y, m, d)` helper that `.expect`s the result.
`DateRange::new`/`DateRange::normalized` return `Option`. Snapshot tests prefixed
`snapshot_` use `insta::assert_snapshot!`. The full test suite lives in
`crates/ars-components/src/date_time/date_range_field/tests.rs` (unit + snapshot),
`tests/spec_conformance/date_time.rs` (anatomy), and
`tests/proptest_state_machines/date_time.rs` (normalized-range invariants).

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u8, day: u8) -> CalendarDate {
        CalendarDate::new_gregorian(year, month, day).expect("valid test date")
    }
    fn range(start: CalendarDate, end: CalendarDate) -> DateRange {
        DateRange::new(start, end).expect("ordered test range")
    }

    #[test]
    fn focus_tracking() {
        let props = Props::default();
        let (state, ctx) = Machine::init(&props, &Env::default(), &Default::default());
        let plan = Machine::transition(&state, &Event::FocusStart, &ctx, &props).unwrap();
        assert_eq!(plan.target, Some(State::StartFocused));
        let plan = Machine::transition(&State::EndFocused, &Event::BlurAll, &ctx, &props).unwrap();
        assert_eq!(plan.target, Some(State::Idle));
    }

    #[test]
    fn out_of_order_change_normalizes() {
        // Setting start after the existing end swaps so start <= end always holds.
        let props = Props { default_value: Some(range(date(2025, 6, 10), date(2025, 6, 20))), ..Props::default() };
        let mut svc = Service::<Machine>::new(props, &Env::default(), &Messages::default());
        drop(svc.send(Event::StartValueChange(Some(date(2025, 6, 25)))));
        let stored = svc.context().value.get().clone().unwrap();
        assert_eq!((stored.start, stored.end), (date(2025, 6, 20), date(2025, 6, 25)));
    }

    #[test]
    fn disabled_ignores_events() {
        let props = Props { disabled: true, ..Props::default() };
        let (state, ctx) = Machine::init(&props, &Env::default(), &Default::default());
        assert!(Machine::transition(&state, &Event::FocusStart, &ctx, &props).is_none());
    }
}
```
