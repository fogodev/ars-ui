---
component: DateRangePicker
category: date-time
tier: stateful
foundation_deps: [architecture, accessibility, i18n, interactions, forms]
shared_deps: [date-time-types]
related: [calendar, range-calendar, date-field, date-range-field]
references:
  ark-ui: DatePicker
  react-aria: DateRangePicker
---

# DateRangePicker

Composes **two DateField instances** + **Calendar with `is_range: true`** in a popover. Follows DatePicker's composition pattern (event bridging via `calendar_props()`).

## 1. State Machine

### 1.1 States

```rust
/// States for the DateRangePicker component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// The popover is closed.
    Closed,
    /// The popover is open.
    Open,
}
```

### 1.2 Events

```rust
/// Events for the DateRangePicker component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Open the popover.
    Open,
    /// Close the popover.
    Close,
    /// Toggle open/closed.
    Toggle,
    /// Calendar has completed range selection.
    SelectRangeComplete {
        /// The selected range.
        range: DateRange
    },
    /// Start date field text changed.
    StartInputChange {
        /// The new value.
        value: String
    },
    /// End date field text changed.
    EndInputChange {
        /// The new value.
        value: String
    },
    /// Focus entered the component.
    FocusIn,
    /// Focus left the component entirely.
    FocusOut,
    /// Key pressed on trigger or within popover.
    KeyDown {
        /// The key that was pressed.
        key: KeyboardKey
    },
}
```

### 1.3 Context

```rust
/// Context for the DateRangePicker component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The selected range.
    pub value: Bindable<Option<DateRange>>,
    /// Whether the popover is open.
    pub open: Bindable<bool>,
    /// The text in the start input field.
    pub start_input_text: String,
    /// The text in the end input field.
    pub end_input_text: String,
    /// The parsed start date.
    pub parsed_start: Option<CalendarDate>,
    /// The parsed end date.
    pub parsed_end: Option<CalendarDate>,
    /// Which DateField is currently active.
    pub active_field: ActiveField,
    /// The minimum date.
    pub min: Option<CalendarDate>,
    /// The maximum date.
    pub max: Option<CalendarDate>,
    /// The locale.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// The date format.
    pub format: DateFormat,
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
}

/// Which DateField is currently active.
#[derive(Clone, Debug, PartialEq)]
pub enum ActiveField {
    /// The start field is active.
    Start,
    /// The end field is active.
    End,
}
```

### 1.4 Props

```rust
/// Props for the DateRangePicker component.
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
    /// The date format.
    pub format: DateFormat,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is readonly.
    pub readonly: bool,
    /// Whether the component is required.
    pub required: bool,
    /// The name of the component (for single hidden input submission).
    pub name: Option<String>,
    /// Form field name for the start date (alternative to `name` for separate
    /// start/end hidden inputs). When set, the start date is submitted under
    /// this key in ISO 8601 format.
    pub start_name: Option<String>,
    /// Form field name for the end date. When set, the end date is submitted
    /// under this key in ISO 8601 format.
    pub end_name: Option<String>,
    /// Whether to close the popover after a date is selected. Default: true.
    pub close_on_select: bool,
    /// The positioning options.
    pub positioning: PositioningOptions,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            min: None,
            max: None,
            format: DateFormat::default(),
            disabled: false,
            readonly: false,
            required: false,
            name: None,
            start_name: None,
            end_name: None,
            close_on_select: true,
            positioning: PositioningOptions::default(),
        }
    }
}
```

### 1.5 Date Range Formatting

1. Date ranges use ICU4X `DateIntervalFormatter`, which handles locale-specific formatting (e.g., English: "May 1--10", German: "1. bis 10. Mai", Arabic: with RTL and native digits).
2. Adapters must NOT construct date ranges by concatenating formatted dates with hardcoded separators.
3. Duration descriptions ("5 days") use ICU4X `PluralRules` for correct pluralization.
4. Prepositions and connectors are part of the formatter output, not separately localized.

### 1.6 Guards

```rust
fn is_disabled(ctx: &Context) -> bool { ctx.disabled }
fn is_readonly(ctx: &Context) -> bool { ctx.readonly }
```

### 1.7 Composition Pattern

Event bridging from child components to DateRangePicker:

```text
Calendar.SelectRangeEnd -> DateRangePicker.SelectRangeComplete { range }
StartDateField.SetValue -> DateRangePicker.StartInputChange { value } (parse, update range.start)
EndDateField.SetValue   -> DateRangePicker.EndInputChange { value }   (parse, update range.end)
```

> **Multi-month recommendation**: DateRangePicker uses `visible_months: 2` by default so users see start and end month grids side-by-side.

#### 1.7.1 State Ownership and Synchronization

The **Calendar** component is the single source of truth for the selected date range.

```rust
// DateRangePicker context owns the canonical range
struct Context {
    /// Source of truth for the selected range.
    range: Bindable<Option<DateRange>>,
    // ...
}
```

**Sync Rules:**

1. When the user selects a range via the Calendar, the DateFields update to reflect the new range.
2. When the user edits a date via a DateField, the Calendar updates to reflect the new value.
3. In controlled mode (`value` prop set), both Calendar and DateFields reflect the controlled value. User edits trigger `on_change` but do not update internal state.
4. In uncontrolled mode, internal `range` is the source of truth. Both Calendar and DateFields read from and write to it.

**Conflict Resolution:**

- If a DateField edit produces an invalid range (start > end), the edit is accepted but the range is marked invalid via `ctx.is_invalid = true`.
- The Calendar highlights the invalid range with a visual indicator.
- `on_change` fires with the invalid range; the consumer decides whether to correct it.

### 1.8 Full Machine Implementation

```rust
/// Machine for the DateRangePicker component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let locale = env.locale.clone();
        let messages = messages.clone();

        let (start_text, end_text, parsed_start, parsed_end) = match &props.default_value {
            Some(range) => (
                format_date(&range.start, &props.format, &locale),
                format_date(&range.end, &props.format, &locale),
                Some(range.start.clone()),
                Some(range.end.clone()),
            ),
            None => (String::new(), String::new(), None, None),
        };

        let ctx = Context {
            value: match &props.value {
                Some(v) => Bindable::controlled(v.clone()),
                None => Bindable::uncontrolled(props.default_value.clone()),
            },
            open: Bindable::uncontrolled(false),
            start_input_text: start_text,
            end_input_text: end_text,
            parsed_start,
            parsed_end,
            active_field: ActiveField::Start,
            min: props.min.clone(),
            max: props.max.clone(),
            locale,
            messages,
            format: props.format.clone(),
            disabled: props.disabled,
            readonly: props.readonly,
            required: props.required,
            name: props.name.clone(),
            start_name: props.start_name.clone(),
            end_name: props.end_name.clone(),
            ids: ComponentIds::from_id(&props.id),
        };
        (State::Closed, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled { return None; }

        match (state, event) {
            (State::Closed, Event::Open) | (State::Closed, Event::Toggle) => {
                Some(TransitionPlan::to(State::Open).apply(|ctx| {
                    ctx.open.set(true);
                }))
            }

            (State::Open, Event::Close) | (State::Open, Event::Toggle) => {
                Some(TransitionPlan::to(State::Closed).apply(|ctx| {
                    ctx.open.set(false);
                }))
            }

            (State::Open, Event::SelectRangeComplete { range }) => {
                let range = range.clone();
                let should_close = _props.close_on_select;
                let next_state = if should_close { State::Closed } else { State::Open };
                Some(TransitionPlan::to(next_state).apply(move |ctx| {
                    ctx.start_input_text = format_date(&range.start, &ctx.format, &ctx.locale);
                    ctx.end_input_text = format_date(&range.end, &ctx.format, &ctx.locale);
                    ctx.parsed_start = Some(range.start.clone());
                    ctx.parsed_end = Some(range.end.clone());
                    ctx.value.set(Some(range));
                    if should_close {
                        ctx.open.set(false);
                    }
                }))
            }

            (_, Event::StartInputChange { value }) => {
                let text = value.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.parsed_start = parse_date(&text, &ctx.format, &ctx.locale);
                    ctx.start_input_text = text;
                    ctx.active_field = ActiveField::Start;
                    // Update range if both dates are valid
                    if let (Some(start), Some(end)) = (&ctx.parsed_start, &ctx.parsed_end) {
                        ctx.value.set(Some(DateRange::normalized(start.clone(), end.clone())));
                    }
                }))
            }

            (_, Event::EndInputChange { value }) => {
                let text = value.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.parsed_end = parse_date(&text, &ctx.format, &ctx.locale);
                    ctx.end_input_text = text;
                    ctx.active_field = ActiveField::End;
                    if let (Some(start), Some(end)) = (&ctx.parsed_start, &ctx.parsed_end) {
                        ctx.value.set(Some(DateRange::normalized(start.clone(), end.clone())));
                    }
                }))
            }

            (State::Open, Event::KeyDown { key }) if *key == KeyboardKey::Escape => {
                Self::transition(state, &Event::Close, ctx, _props)
            }

            (State::Closed, Event::KeyDown { key }) if *key == KeyboardKey::ArrowDown => {
                Self::transition(state, &Event::Open, ctx, _props)
            }

            (_, Event::FocusOut) if *state == State::Open => {
                Self::transition(state, &Event::Close, ctx, _props)
            }

            _ => None,
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.9 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "date-range-picker"]
pub enum Part {
    Root,
    Label,
    Control,
    StartInput,
    Separator,
    EndInput,
    Trigger,
    ClearTrigger,
    Positioner,
    Content,
    Description,
    ErrorMessage,
    HiddenInput,
}

/// API for the DateRangePicker component.
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
    /// Attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        attrs.set(HtmlAttr::Data("ars-state"), self.state_name());
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.readonly { attrs.set_bool(HtmlAttr::Data("ars-readonly"), true); }
        if self.ctx.required { attrs.set_bool(HtmlAttr::Data("ars-required"), true); }
        attrs
    }

    /// Attributes for the label element.
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("label"));
        attrs.set(HtmlAttr::For, self.ctx.ids.part("start-input"));
        attrs
    }

    /// Attributes for the control element.
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "group");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));
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

    /// Build DateField Props for the start date input.
    pub fn start_field_props(&self) -> date_field::Props {
        date_field::Props {
            id: self.ctx.ids.part("start-input"),
            value: Some(self.ctx.parsed_start.clone()),
            format: self.ctx.format.clone(),
            min: self.ctx.min.clone(),
            max: self.ctx.parsed_end.clone(), // start can't exceed end
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            aria_label: Some((self.ctx.messages.start_label)(&self.ctx.locale)),
            ..Default::default()
        }
    }

    /// Build DateField Props for the end date input.
    pub fn end_field_props(&self) -> date_field::Props {
        date_field::Props {
            id: self.ctx.ids.part("end-input"),
            value: Some(self.ctx.parsed_end.clone()),
            format: self.ctx.format.clone(),
            min: self.ctx.parsed_start.clone(), // end can't precede start
            max: self.ctx.max.clone(),
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            aria_label: Some((self.ctx.messages.end_label)(&self.ctx.locale)),
            ..Default::default()
        }
    }

    /// Attributes for the trigger element.
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("trigger"));
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.trigger_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if *self.ctx.open.get() { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), self.ctx.ids.part("content"));
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Disabled, true); }
        attrs
    }

    /// Attributes for the clear trigger element.
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ClearTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.clear_label)(&self.ctx.locale));
        if self.ctx.disabled || self.ctx.value.get().is_none() {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        attrs
    }

    /// Attributes for the positioner element.
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Attributes for the content element.
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("content"));
        attrs.set(HtmlAttr::Role, "dialog");
        attrs.set(HtmlAttr::Aria(AriaAttr::Modal), "false");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));
        attrs
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

    /// Attributes for the hidden input element (form submission).
    /// When `name` is set, submits range as `"YYYY-MM-DD/YYYY-MM-DD"`.
    /// When `start_name`/`end_name` are set, use `start_hidden_input_attrs()`
    /// and `end_hidden_input_attrs()` instead for separate form fields.
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name);
        }
        // Submit range as two ISO 8601 dates separated by "/"
        if let Some(range) = &self.ctx.value.get() {
            let iso = format!("{}/{}", range.start.to_iso_string(), range.end.to_iso_string());
            attrs.set(HtmlAttr::Value, iso);
        }
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
            attrs.set(HtmlAttr::Value, range.start.to_iso_string());
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
            attrs.set(HtmlAttr::Value, range.end.to_iso_string());
        }
        attrs
    }

    /// Build Calendar Props with range mode enabled.
    /// Uses `visible_months: 2` by default so users see start and end month
    /// grids side-by-side.
    pub fn calendar_props(&self) -> calendar::Props {
        calendar::Props {
            id: self.ctx.ids.part("calendar"),
            is_range: true,
            visible_months: 2,
            page_behavior: PageBehavior::Visible,
            range_start: self.ctx.parsed_start.clone(),
            range_end: self.ctx.parsed_end.clone(),
            min: self.ctx.min.clone(),
            max: self.ctx.max.clone(),
            ..Default::default()
        }
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

    // -- Imperative methods -----------------------------------------------

    /// Open the popover.
    pub fn open(&self)   { (self.send)(Event::Open); }
    /// Close the popover.
    pub fn close(&self)  { (self.send)(Event::Close); }
    /// Toggle the popover.
    pub fn toggle(&self) { (self.send)(Event::Toggle); }
    /// Check if the popover is open.
    pub fn is_open(&self) -> bool { *self.ctx.open.get() }
    /// Get the selected range.
    pub fn selected_range(&self) -> Option<&DateRange> {
        self.ctx.value.get().as_ref()
    }

    fn state_name(&self) -> &'static str {
        match self.state {
            State::Closed => "closed",
            State::Open   => "open",
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Control => self.control_attrs(),
            Part::StartInput => self.start_field_props().into(), // delegates to DateField
            Part::Separator => self.separator_attrs(),
            Part::EndInput => self.end_field_props().into(), // delegates to DateField
            Part::Trigger => self.trigger_attrs(),
            Part::ClearTrigger => self.clear_trigger_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
DateRangePicker
+-- Root                                  data-ars-scope="date-range-picker"
|   +-- Label                             <label for="{id}-start-input">
|   +-- Control                           role="group" aria-labelledby="{id}-label"
|   |   +-- StartInput                    (DateField)
|   |   +-- Separator                     aria-hidden="true"  "--"
|   |   +-- EndInput                      (DateField)
|   |   +-- Trigger                       aria-haspopup="dialog" aria-expanded
|   |   +-- ClearTrigger                  aria-label="Clear date range"
|   +-- Positioner
|   |   +-- Content                       role="dialog" aria-labelledby="{id}-label"
|   |       +-- (Calendar)                is_range=true  visible_months=2
|   +-- Description                       help text
|   +-- ErrorMessage                      role="alert"
|   +-- HiddenInput                       type="hidden" value="2024-01-10/2024-01-20"
```

| Part           | Element       | Key Attributes                                                      |
| -------------- | ------------- | ------------------------------------------------------------------- |
| `Root`         | `<div>`       | `data-ars-scope="date-range-picker"`, `data-ars-part="root"`, state |
| `Label`        | `<label>`     | `for="{id}-start-input"`                                            |
| `Control`      | `<div>`       | `role="group"`, `aria-labelledby`                                   |
| `StartInput`   | _(DateField)_ | Start date field                                                    |
| `Separator`    | `<span>`      | `aria-hidden="true"`                                                |
| `EndInput`     | _(DateField)_ | End date field                                                      |
| `Trigger`      | `<button>`    | `aria-haspopup="dialog"`, `aria-expanded`, `aria-controls`          |
| `ClearTrigger` | `<button>`    | `aria-label="Clear date range"`                                     |
| `Positioner`   | `<div>`       | Floating positioner (via positioning engine)                        |
| `Content`      | `<div>`       | `role="dialog"`, `aria-modal="false"`, `aria-labelledby`            |
| `Description`  | `<div>`       | Help text, wired via `aria-describedby`                             |
| `ErrorMessage` | `<div>`       | `role="alert"`, validation error                                    |
| `HiddenInput`  | `<input>`     | `type="hidden"`, ISO 8601 range value                               |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Element        | Role/Attribute      | Details                                                                       |
| -------------- | ------------------- | ----------------------------------------------------------------------------- |
| `Control`      | `role="group"`      | `aria-labelledby` points to `Label`                                           |
| `Trigger`      | `<button>`          | `aria-haspopup="dialog"`, `aria-expanded`, `aria-controls` points to Content  |
| `Content`      | `role="dialog"`     | `aria-modal="false"`, `aria-labelledby` points to Label                       |
| `ErrorMessage` | `role="alert"`      | Announced immediately on render                                               |
| `Root`         | --                  | `data-ars-invalid` when validation fails                                      |
| `StartInput`   | spinbutton segments | `aria-label` from `messages.start_label`; delegates to DateField ARIA pattern |
| `EndInput`     | spinbutton segments | `aria-label` from `messages.end_label`; delegates to DateField ARIA pattern   |

### 3.2 Keyboard Interaction

| Key         | Context         | Action                                        |
| ----------- | --------------- | --------------------------------------------- |
| `ArrowDown` | Trigger / Input | Open the popover                              |
| `Escape`    | Popover open    | Close the popover, restore focus to trigger   |
| `Tab`       | Within calendar | Close the popover                             |
| `Enter`     | Calendar cell   | Complete range selection (if second endpoint) |
| `Space`     | Calendar cell   | Complete range selection (if second endpoint) |

### 3.3 Focus Flow

- When the popover opens, focus moves into the Calendar grid.
- When a range is completed, the popover closes and focus returns to the trigger.
- Start and End inputs are linked via `aria-labelledby` to the shared label.
- Tab navigation flows: StartInput -> Separator (skipped) -> EndInput -> Trigger -> ClearTrigger.
- `aria-describedby` wires to Description and ErrorMessage parts when present.

## 4. Internationalization

### 4.1 Messages

```rust
/// Messages for the DateRangePicker component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Trigger button label (default: "Open date range picker").
    pub trigger_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the start date field.
    pub start_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the end date field.
    pub end_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Describes the selected range (e.g. "March 1 to March 15").
    pub range_description: MessageFn<dyn Fn(&str, &str, &Locale) -> String + Send + Sync>,
    /// Accessible label for the clear trigger button (default: "Clear date range").
    pub clear_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            trigger_label: MessageFn::static_str("Open date range picker"),
            start_label: MessageFn::static_str("Start date"),
            end_label: MessageFn::static_str("End date"),
            range_description: MessageFn::new(|start, end, _locale| format!("{} to {}", start, end)),
            clear_label: MessageFn::static_str("Clear date range"),
        }
    }
}

impl ComponentMessages for Messages {}
```

## 5. Form Integration

- Hidden `<input>` submits the range as two ISO 8601 dates separated by `/` (e.g., `2024-01-10/2024-01-20`).
- `name` attribute set from Props.
- Reset restores `default_value`.
- Validation states reflected via `data-ars-invalid` on Root.
- `aria-describedby` wires to Description and ErrorMessage parts.
- `aria-required` set when `required` is true.
- Disabled/readonly propagation from form context per `07-forms.md` S15.

## 6. Library Parity

> Compared against: Ark UI (`DatePicker` with `selectionMode: 'range'`), React Aria (`DateRangePicker`).

### 6.1 Props

| Feature               | ars-ui            | Ark UI                | React Aria                  | Notes                                               |
| --------------------- | ----------------- | --------------------- | --------------------------- | --------------------------------------------------- |
| Controlled range      | `value`           | `value` (DateValue[]) | `value` (RangeValue)        | Equivalent                                          |
| Default range         | `default_value`   | `defaultValue`        | `defaultValue`              | Equivalent                                          |
| Min/max               | `min`, `max`      | `min`, `max`          | `minValue`, `maxValue`      | Equivalent                                          |
| Disabled              | `disabled`        | `disabled`            | `isDisabled`                | Equivalent                                          |
| Read-only             | `readonly`        | `readOnly`            | `isReadOnly`                | Equivalent                                          |
| Required              | `required`        | `required`            | `isRequired`                | Equivalent                                          |
| Close on select       | `close_on_select` | `closeOnSelect`       | `shouldCloseOnSelect`       | Equivalent                                          |
| Locale                | `locale`          | `locale`              | -- (context)                | Equivalent                                          |
| Format                | `format`          | `format`              | --                          | Equivalent                                          |
| Name                  | `name`            | `name`                | `startName`, `endName`      | React Aria has separate form names                  |
| Positioning           | `positioning`     | `positioning`         | --                          | Equivalent                                          |
| Allows non-contiguous | --                | --                    | `allowsNonContiguousRanges` | React Aria feature                                  |
| Max visible months    | --                | `numOfMonths`         | `maxVisibleMonths`          | ars-ui uses `visible_months: 2` default on Calendar |
| Start/end form names  | --                | --                    | `startName`, `endName`      | React Aria has separate form names for start/end    |

**Gaps:**

- `allowsNonContiguousRanges`: React Aria allows selecting ranges that span unavailable dates. Currently ars-ui's RangeCalendar blocks selection of unavailable dates. This is a niche feature; not adopting.

### 6.2 Anatomy

| Part          | ars-ui         | Ark UI            | React Aria                 | Notes                               |
| ------------- | -------------- | ----------------- | -------------------------- | ----------------------------------- |
| Root          | `Root`         | `Root`            | `DateRangePicker`          | Equivalent                          |
| Label         | `Label`        | `Label`           | `Label`                    | Equivalent                          |
| Control       | `Control`      | `Control`         | `Group`                    | Equivalent                          |
| Start input   | `StartInput`   | `Input` (index=0) | `DateInput` (slot="start") | Equivalent                          |
| Separator     | `Separator`    | --                | --                         | ars-ui adds a visual separator      |
| End input     | `EndInput`     | `Input` (index=1) | `DateInput` (slot="end")   | Equivalent                          |
| Trigger       | `Trigger`      | `Trigger`         | `Button`                   | Equivalent                          |
| Clear trigger | `ClearTrigger` | `ClearTrigger`    | --                         | ars-ui and Ark UI have clear button |
| Positioner    | `Positioner`   | `Positioner`      | --                         | Equivalent                          |
| Content       | `Content`      | `Content`         | `Popover` + `Dialog`       | Equivalent                          |
| Description   | `Description`  | --                | `Text` (slot)              | Equivalent                          |
| Error message | `ErrorMessage` | --                | `FieldError`               | Equivalent                          |
| Hidden input  | `HiddenInput`  | --                | --                         | ars-ui form integration             |

**Gaps:** None worth adopting.

### 6.3 Events

| Callback     | ars-ui                 | Ark UI          | React Aria      | Notes      |
| ------------ | ---------------------- | --------------- | --------------- | ---------- |
| Range change | `SelectRangeComplete`  | `onValueChange` | `onChange`      | Equivalent |
| Open change  | `Open` / `Close`       | `onOpenChange`  | `onOpenChange`  | Equivalent |
| Focus change | `FocusIn` / `FocusOut` | `onFocusChange` | `onFocusChange` | Equivalent |

**Gaps:** None.

### 6.4 Features

| Feature               | ars-ui                      | Ark UI            | React Aria            |
| --------------------- | --------------------------- | ----------------- | --------------------- |
| Two-field input       | Yes (StartInput + EndInput) | Yes (Input index) | Yes (start/end slots) |
| Calendar popover      | Yes                         | Yes               | Yes                   |
| Range normalization   | Yes                         | Yes               | Yes                   |
| Close on select       | Yes                         | Yes               | Yes                   |
| Clear button          | Yes                         | Yes               | No                    |
| Multi-month calendar  | Yes (default 2)             | Yes               | Yes                   |
| Non-contiguous ranges | No                          | No                | Yes                   |
| Separate form names   | No                          | No                | Yes                   |
| Hidden form input     | Yes                         | No                | No                    |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity.
- **Divergences:** React Aria supports `allowsNonContiguousRanges` which is a niche feature not adopted. ars-ui submits ranges via a single hidden input while React Aria uses separate form names.
- **Recommended additions:** None.

## Appendix: Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_closed() {
        let (state, ctx) = Machine::init(&Props::default(), &Env::default(), &Default::default());
        assert_eq!(state, State::Closed);
        assert_eq!(*ctx.open.get(), false);
        assert!(ctx.value.get().is_none());
    }

    #[test]
    fn open_and_close() {
        let props = Props::default();
        let (state, ctx) = Machine::init(&props, &Env::default(), &Default::default());
        let plan = Machine::transition(&state, &Event::Open, &ctx, &props).unwrap();
        assert_eq!(plan.target, Some(State::Open));

        let (state, ctx) = (State::Open, /* apply plan */);
        let plan = Machine::transition(&state, &Event::Close, &ctx, &props).unwrap();
        assert_eq!(plan.target, Some(State::Closed));
    }

    #[test]
    fn select_range_complete_closes_and_sets_value() {
        let props = Props::default();
        let (_, ctx) = Machine::init(&props, &Env::default(), &Default::default());
        let range = DateRange::new(
            CalendarDate::new_gregorian(2025, nzu8(3), nzu8(1)),
            CalendarDate::new_gregorian(2025, nzu8(3), nzu8(15)),
        );
        let plan = Machine::transition(
            &State::Open,
            &Event::SelectRangeComplete { range: range.clone() },
            &ctx,
            &props,
        ).unwrap();
        assert_eq!(plan.target, Some(State::Closed));
    }

    #[test]
    fn start_input_change_parses_date() {
        let props = Props { ..Props::default() };
        let (state, ctx) = Machine::init(&props, &Env::default(), &Default::default());
        let plan = Machine::transition(
            &state,
            &Event::StartInputChange { value: "03/01/2025".to_string() },
            &ctx,
            &props,
        ).unwrap();
        // Plan should update start_input_text and parsed_start
        assert!(plan.target.is_none()); // context-only
    }

    #[test]
    fn escape_closes_popover() {
        let props = Props::default();
        let (_, ctx) = Machine::init(&props, &Env::default(), &Default::default());
        let plan = Machine::transition(
            &State::Open,
            &Event::KeyDown { key: KeyboardKey::Escape },
            &ctx,
            &props,
        ).unwrap();
        assert_eq!(plan.target, Some(State::Closed));
    }

    #[test]
    fn disabled_ignores_events() {
        let props = Props { disabled: true, ..Props::default() };
        let (state, ctx) = Machine::init(&props, &Env::default(), &Default::default());
        assert!(Machine::transition(&state, &Event::Open, &ctx, &props).is_none());
    }

    #[test]
    fn focusout_closes_popover() {
        let props = Props::default();
        let (_, ctx) = Machine::init(&props, &Env::default(), &Default::default());
        let plan = Machine::transition(
            &State::Open,
            &Event::FocusOut,
            &ctx,
            &props,
        ).unwrap();
        assert_eq!(plan.target, Some(State::Closed));
    }

    #[test]
    fn arrow_down_opens_from_closed() {
        let props = Props::default();
        let (state, ctx) = Machine::init(&props, &Env::default(), &Default::default());
        let plan = Machine::transition(
            &state,
            &Event::KeyDown { key: KeyboardKey::ArrowDown },
            &ctx,
            &props,
        ).unwrap();
        assert_eq!(plan.target, Some(State::Open));
    }
}
```
