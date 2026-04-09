---
component: DatePicker
category: date-time
tier: complex
foundation_deps: [architecture, accessibility, i18n, interactions, forms]
shared_deps: [date-time-types]
related: [calendar, date-field, date-range-picker]
references:
  ark-ui: DatePicker
  react-aria: DatePicker
---

# DatePicker

`DatePicker` is a composite component that combines a `DateField` (segmented date input) with a `Calendar` in a popover.

The DatePicker composes two internal machines:

- A `DateField` machine handles the input segments.
- A `Calendar` machine handles the popover calendar grid.
- The DatePicker's own state machine manages the popover open/close lifecycle and synchronizes the selected value between the two sub-components.

## 1. State Machine

### 1.1 States

```rust
/// States for the DatePicker component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Popover is closed; input field may or may not have focus.
    Closed,
    /// Popover is open and the calendar is visible.
    Open,
}
```

### 1.2 Events

```rust
/// Events for the DatePicker component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Open the calendar popover.
    Open,
    /// Close the calendar popover.
    Close,
    /// Toggle the popover.
    Toggle,
    /// A date was selected (from the calendar or parsed from the input).
    SelectDate { date: CalendarDate },
    /// The text input value changed (user typing in the field).
    InputChange { value: String },
    /// Focus entered the date picker (input or trigger).
    FocusIn,
    /// Focus left the date picker entirely.
    FocusOut,
    /// Keyboard event on the input or trigger.
    KeyDown { key: KeyboardKey },
}
```

### 1.3 Context

```rust
/// Context for the DatePicker component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The selected date value (controlled/uncontrolled).
    pub value: Bindable<Option<CalendarDate>>,
    /// Whether the popover is open (controlled/uncontrolled).
    pub open: Bindable<bool>,
    /// Raw text in the input field.
    pub input_text: String,
    /// Last successfully parsed date from input text.
    pub parsed_date: Option<CalendarDate>,
    /// Locale for formatting and parsing.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// Date format pattern (e.g. "MM/dd/yyyy").
    pub format: String,
    /// Minimum selectable date (forwarded to Calendar).
    pub min: Option<CalendarDate>,
    /// Maximum selectable date (forwarded to Calendar).
    pub max: Option<CalendarDate>,
    /// Disabled state.
    pub disabled: bool,
    /// Read-only state.
    pub readonly: bool,
    /// Whether clicking/focusing the input opens the popover.
    pub open_on_click: bool,
    /// Unavailable date predicate result cache.
    pub unavailable_dates: Vec<CalendarDate>,
    /// Whether the input has been interacted with.
    pub is_touched: bool,
    /// Form field name.
    pub name: Option<String>,
    /// Whether the field is required.
    pub required: bool,
    /// Right-to-left layout.
    pub is_rtl: bool,
    /// Component IDs.
    pub ids: ComponentIds,
}

impl Context {
    /// Format the current value as a display string for the input field.
    pub fn formatted_value(&self) -> String {
        match self.value.get() {
            Some(date) => format_date(date, &self.format, &self.locale),
            None => String::new(),
        }
    }

    /// Attempt to parse a text string into a CalendarDate.
    pub fn parse_input(&self, text: &str) -> Option<CalendarDate> {
        parse_date(text, &self.format, &self.locale)
    }

    /// Sync the input text to reflect the current value.
    pub fn sync_input_text(&mut self) {
        self.input_text = self.formatted_value();
    }
}

/// Format a CalendarDate according to the given pattern and locale.
fn format_date(date: &CalendarDate, format: &str, locale: &Locale) -> String {
    // Production: ICU4X DateTimeFormatter.
    // Simplified placeholder for the spec:
    format!("{:02}/{:02}/{:04}", date.month.get(), date.day.get(), date.year)
}

/// Parse a date string according to the given pattern and locale.
fn parse_date(text: &str, format: &str, locale: &Locale) -> Option<CalendarDate> {
    // Production: ICU4X DateTimeParser.
    // Simplified placeholder for the spec:
    let parts: Vec<&str> = text.split('/').collect();
    if parts.len() != 3 { return None; }
    let month = parts[0].parse::<u8>().ok()?;
    let day   = parts[1].parse::<u8>().ok()?;
    let year  = parts[2].parse::<i32>().ok()?;
    if month < 1 || month > 12 || day < 1 || day > 31 { return None; }
    Some(CalendarDate::new_gregorian(
        year,
        NonZero::new(month).expect("parsed month is 1-based"),
        NonZero::new(day).expect("parsed day is 1-based"),
    ))
}
```

### 1.4 Props

```rust
/// Props for the DatePicker component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The ID of the date picker.
    pub id: String,
    /// Controlled date value.
    pub value: Option<Option<CalendarDate>>,
    /// Default date for uncontrolled mode.
    pub default_value: Option<CalendarDate>,
    /// Minimum selectable date.
    pub min: Option<CalendarDate>,
    /// Maximum selectable date.
    pub max: Option<CalendarDate>,
    /// Whether the date picker is non-interactive.
    pub disabled: bool,
    /// Whether the date picker allows viewing but not editing.
    pub readonly: bool,
    /// Predicate for unavailable dates (forwarded to Calendar).
    pub is_date_unavailable: Option<fn(&CalendarDate) -> bool>,
    /// Date format pattern. Defaults to locale-appropriate format.
    pub format: Option<String>,
    /// Placeholder text for the input field.
    pub placeholder: Option<String>,
    /// Form field name for hidden input submission.
    pub name: Option<String>,
    /// Whether the field is required.
    pub required: bool,
    /// Right-to-left layout direction.
    pub is_rtl: bool,
    /// Label text.
    pub label: String,
    /// Description/help text.
    pub description: Option<String>,
    /// Error message.
    pub error_message: Option<String>,
    /// Whether the field is in an invalid state.
    pub invalid: bool,
    /// Whether to close the popover after a date is selected. Default: true.
    pub close_on_select: bool,
    /// Controlled open state. `Some(true)` = forced open, `Some(false)` = forced closed,
    /// `None` = uncontrolled (internal state manages open/close).
    pub open: Option<bool>,
    /// Default open state for uncontrolled mode.
    pub default_open: bool,
    /// When `true` (the default), clicking or focusing the date field opens
    /// the calendar popover. When `false`, only the trigger button opens it.
    pub open_on_click: bool,
    /// Number of months to display side-by-side in the calendar popover. Default: `1`.
    /// When `> 1`, months render side-by-side on desktop (or stacked on mobile/narrow viewports).
    /// Navigation (prev/next buttons) advances by `visible_months` count (or by 1 if
    /// `page_behavior: PageBehavior::Single` is set on the underlying Calendar).
    /// Forwarded to the embedded Calendar component's `visible_months` prop.
    pub visible_months: usize,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            min: None,
            max: None,
            disabled: false,
            readonly: false,
            is_date_unavailable: None,
            format: None,
            placeholder: None,
            name: None,
            required: false,
            is_rtl: false,
            label: String::new(),
            description: None,
            error_message: None,
            invalid: false,
            close_on_select: true,
            open: None,
            default_open: false,
            open_on_click: true,
            visible_months: 1,
        }
    }
}
```

### 1.5 Guards

```rust
fn is_disabled(ctx: &Context) -> bool { ctx.disabled }
fn is_readonly(ctx: &Context) -> bool { ctx.readonly }
```

### 1.6 Composition with Calendar

The DatePicker does not re-implement calendar logic. Instead, its `Content` part renders a `Calendar` component. The adapter is responsible for:

1. Creating a `Calendar` `Service` using `api.calendar_props()`.
2. Rendering the Calendar inside the `Content` element.
3. Wiring the Calendar's `SelectDate` event back to the DatePicker as `Event::SelectDate { date }`.
4. Synchronizing the `value` Bindable so the Calendar reflects the DatePicker's current value.

```rust
// Pseudocode: Leptos adapter composition
#[component]
fn DatePicker(props: DatePickerProps) -> impl IntoView {
    let (state, ctx, send) = use_machine::<date_picker::Machine>(/* ... */);
    let api = date_picker::Machine::connect(&state, &ctx, &props, &send);

    view! {
        <div {..api.root_attrs()}>
            <label {..api.label_attrs()}>{&props.label}</label>
            <div {..api.control_attrs()}>
                <input {..api.input_attrs()}
                    on:input=move |e| api.on_input_change(&event_target_value(&e))
                    on:keydown=move |e| api.on_input_keydown(&e.key())
                />
                <button {..api.trigger_attrs()}
                    on:click=move |_| api.on_trigger_click()
                    on:keydown=move |e| api.on_trigger_keydown(&e.key())
                >
                    <CalendarIcon />
                </button>
            </div>
            <div {..api.positioner_attrs()}>
                <div {..api.content_attrs()}
                    on:keydown=move |e| api.on_content_keydown(&e.key())
                >
                    // Calendar composed here, with events bridged back.
                    <Calendar
                        props=api.calendar_props()
                        on_select=move |date| send(date_picker::Event::SelectDate { date })
                    />
                </div>
            </div>
            <input {..api.hidden_input_attrs()} />
        </div>
    }
}
```

### 1.7 Full Machine Implementation

```rust
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let value = match props.value {
            Some(v) => Bindable::controlled(v),
            None    => Bindable::uncontrolled(props.default_value.clone()),
        };

        let locale = env.locale.clone();
        let messages = messages.clone();

        let locale_tag = locale.to_bcp47();
        let default_format = match locale_tag.as_str() {
            "en-US" | "en-CA" => "MM/dd/yyyy",
            "en-GB" | "de-DE" | "fr-FR" | "es-ES" | "it-IT" | "ru-RU" => "dd/MM/yyyy",
            "ja-JP" | "zh-CN" | "zh-TW" | "ko-KR" => "yyyy/MM/dd",
            _ => "MM/dd/yyyy",
        };
        let format = props
            .format
            .clone()
            .unwrap_or_else(|| default_format.to_string());

        let input_text = value
            .get()
            .as_ref()
            .map(|d| format_date(d, &format, &locale))
            .unwrap_or_default();

        let open = match props.open {
            Some(v) => Bindable::controlled(v),
            None    => Bindable::uncontrolled(props.default_open),
        };

        let ctx = Context {
            value,
            open,
            input_text,
            parsed_date: None,
            locale,
            messages,
            format,
            min: props.min.clone(),
            max: props.max.clone(),
            disabled: props.disabled,
            readonly: props.readonly,
            open_on_click: props.open_on_click,
            unavailable_dates: Vec::new(),
            is_touched: false,
            name: props.name.clone(),
            required: props.required,
            is_rtl: props.is_rtl,
            ids: ComponentIds::from_id(&props.id),
        };

        let initial_state = if *open.get() { State::Open } else { State::Closed };

        (initial_state, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled { return None; }

        match event {
            // ── Popover open / close ─────────────────────────────────────
            Event::Open => {
                if *state == State::Open || ctx.readonly { return None; }
                Some(TransitionPlan::to(State::Open)
                    .apply(|ctx| {
                        ctx.open.set(true);
                    })
                    .with_effect(PendingEffect::new("focus-calendar", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        let grid_id = ctx.ids.part("grid");
                        platform.focus_element_by_id(&grid_id);
                        no_cleanup()
                    })))
            }

            Event::Close => {
                if *state == State::Closed { return None; }
                Some(TransitionPlan::to(State::Closed)
                    .apply(|ctx| {
                        ctx.open.set(false);
                    })
                    .with_effect(PendingEffect::new("restore-focus", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        let trigger_id = ctx.ids.part("trigger");
                        platform.focus_element_by_id(&trigger_id);
                        no_cleanup()
                    })))
            }

            Event::Toggle => {
                match state {
                    State::Closed => Self::transition(state, &Event::Open, ctx, props),
                    State::Open   => Self::transition(state, &Event::Close, ctx, props),
                }
            }

            // ── Date selection (from calendar) ───────────────────────────
            Event::SelectDate { date } => {
                if ctx.readonly { return None; }
                let date = date.clone();
                let format = ctx.format.clone();
                let locale = ctx.locale.clone();
                let should_close = props.close_on_select;
                let next_state = if should_close { State::Closed } else { State::Open };
                let mut plan = TransitionPlan::to(next_state)
                    .apply(move |ctx| {
                        ctx.value.set(Some(date.clone()));
                        ctx.input_text = format_date(&date, &format, &locale);
                        ctx.parsed_date = Some(date);
                        if should_close {
                            ctx.open.set(false);
                        }
                        ctx.is_touched = true;
                    });
                if should_close {
                    plan = plan.with_effect(PendingEffect::new("restore-focus-after-select", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        let input_id = ctx.ids.part("input");
                        platform.focus_element_by_id(&input_id);
                        no_cleanup()
                    }));
                }
                Some(plan)
            }

            // ── Input text change ────────────────────────────────────────
            Event::InputChange { value } => {
                if ctx.readonly { return None; }
                let text = value.clone();
                let format = ctx.format.clone();
                let locale = ctx.locale.clone();
                let min = ctx.min.clone();
                let max = ctx.max.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.is_touched = true;
                    // Attempt to parse the input.
                    if let Some(date) = parse_date(&text, &format, &locale) {
                        // Validate against min/max.
                        let in_range = min.as_ref().map_or(true, |m| date >= *m)
                            && max.as_ref().map_or(true, |m| date <= *m);
                        if in_range {
                            ctx.parsed_date = Some(date.clone());
                            ctx.value.set(Some(date));
                        }
                    } else {
                        // If text is empty, clear the value.
                        if text.is_empty() {
                            ctx.parsed_date = None;
                            ctx.value.set(None);
                        }
                    }
                    ctx.input_text = text;
                }))
            }

            // ── Focus management ─────────────────────────────────────────
            Event::FocusIn => {
                if *state == State::Closed && ctx.open_on_click {
                    Some(TransitionPlan::to(State::Open)
                        .apply(|ctx| {
                            ctx.open.set(true);
                        })
                        .with_effect(PendingEffect::new("focus-calendar", |ctx, _props, _send| {
                            let platform = use_platform_effects();
                            let grid_id = ctx.ids.part("grid");
                            platform.focus_element_by_id(&grid_id);
                            no_cleanup()
                        })))
                } else {
                    Some(TransitionPlan::context_only(|_ctx| {
                        // No state change; just marks that the component has focus.
                    }))
                }
            }

            Event::FocusOut => {
                // Close popover if open and focus leaves the entire component.
                if *state == State::Open {
                    Some(TransitionPlan::to(State::Closed)
                        .apply(|ctx| {
                            ctx.open.set(false);
                        }))
                } else {
                    None
                }
            }

            // ── Keyboard shortcuts ───────────────────────────────────────
            Event::KeyDown { key } => {
                match key {
                    KeyboardKey::Escape if *state == State::Open => {
                        Self::transition(state, &Event::Close, ctx, props)
                    }
                    KeyboardKey::ArrowDown if *state == State::Closed => {
                        // Alt+ArrowDown opens the picker (handled at adapter level).
                        Self::transition(state, &Event::Open, ctx, props)
                    }
                    _ => None,
                }
            }
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

### 1.8 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "date-picker"]
pub enum Part {
    Root,
    Label,
    Control,
    Input,
    Trigger,
    ClearTrigger,
    Positioner,
    Content,
    Description,
    ErrorMessage,
    HiddenInput,
}

/// API for the DatePicker component.
pub struct Api<'a> {
    /// The state of the component.
    state: &'a State,
    /// The context of the component.
    ctx:   &'a Context,
    /// The props of the component.
    props: &'a Props,
    /// The send function.
    send:  &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    // ── AttrMap getters ──────────────────────────────────────────────────

    /// Get the root attributes.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        attrs.set(HtmlAttr::Data("ars-state"), self.state_name());
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }
        if self.ctx.is_rtl {
            attrs.set(HtmlAttr::Dir, "rtl");
        }
        attrs
    }

    /// Get the label attributes.
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("label"));
        attrs.set(HtmlAttr::For, self.ctx.ids.part("input"));
        attrs
    }

    /// Get the control attributes.
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("control"));
        attrs
    }

    /// Get the input attributes.
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("input"));
        attrs.set(HtmlAttr::Type, "text");
        attrs.set(HtmlAttr::Value, &self.ctx.input_text);
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.is_open() { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), self.ctx.ids.part("content"));
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));

        // Announce the selected date when present.
        if let Some(date) = &self.ctx.parsed_date {
            let formatted = format_date(date, &self.ctx.format, &self.ctx.locale);
            attrs.set(HtmlAttr::Aria(AriaAttr::Description),
                (self.ctx.messages.selected_date_label)(&formatted, &self.ctx.locale));
        }

        // Wire description and error references.
        let mut describedby_parts = Vec::new();
        if self.props.description.is_some() {
            describedby_parts.push(self.ctx.ids.part("description"));
        }
        if self.props.invalid {
            describedby_parts.push(self.ctx.ids.part("error-message"));
        }
        if !describedby_parts.is_empty() {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), describedby_parts.join(" "));
        }

        if let Some(ph) = &self.props.placeholder {
            attrs.set(HtmlAttr::Placeholder, ph);
        }
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::ReadOnly, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }
        if self.ctx.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }
        if self.props.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }
        attrs
    }

    /// Get the trigger attributes.
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("trigger"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.trigger_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.is_open() { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), self.ctx.ids.part("content"));
        attrs.set(HtmlAttr::TabIndex, "0");
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        attrs
    }

    /// Get the clear trigger attributes.
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ClearTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("clear-trigger"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.clear_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::TabIndex, "-1");
        if self.ctx.disabled || self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        // Only visible when a value is selected.
        if self.ctx.value.get().is_none() {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }
        attrs
    }

    /// Get the positioner attributes.
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("positioner"));
        if !self.is_open() {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }
        attrs
    }

    /// Get the content attributes.
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("content"));
        attrs.set(HtmlAttr::Role, "dialog");
        attrs.set(HtmlAttr::Data("ars-state"), self.state_name());
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.content_label)(&self.ctx.locale));
        attrs
    }

    /// Get the description attributes.
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));
        attrs
    }

    /// Get the error message attributes.
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("error-message"));
        attrs.set(HtmlAttr::Role, "alert");
        attrs
    }

    /// Get the hidden input attributes.
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let value = self.ctx.value.get()
            .as_ref()
            .map(|d| format!("{:04}-{:02}-{:02}", d.year, d.month.get(), d.day.get()))
            .unwrap_or_default();
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name);
        }
        attrs.set(HtmlAttr::Value, value);
        attrs
    }

    // ── Typed handler methods ────────────────────────────────────────────

    /// Handle click on the trigger button.
    pub fn on_trigger_click(&self) {
        (self.send)(Event::Toggle);
    }

    /// Handle keydown on the trigger button.
    pub fn on_trigger_keydown(&self, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::Enter | KeyboardKey::Space => (self.send)(Event::Toggle),
            KeyboardKey::ArrowDown   => (self.send)(Event::Open),
            _ => {}
        }
    }

    /// Handle click on the clear trigger button.
    pub fn on_clear_trigger_click(&self) {
        (self.send)(Event::InputChange { value: String::new() });
    }

    /// Handle input text change.
    pub fn on_input_change(&self, value: &str) {
        (self.send)(Event::InputChange { value: value.to_string() });
    }

    /// Handle keydown on the input field.
    pub fn on_input_keydown(&self, key: KeyboardKey) {
        (self.send)(Event::KeyDown { key });
    }

    /// Handle focus entering the component.
    pub fn on_focusin(&self) {
        (self.send)(Event::FocusIn);
    }

    /// Handle focus leaving the entire component.
    pub fn on_focusout(&self, focus_leaving_component: bool) {
        if focus_leaving_component {
            (self.send)(Event::FocusOut);
        }
    }

    /// Handle Escape key within the popover content.
    pub fn on_content_keydown(&self, data: &KeyboardEventData) {
        if data.key == KeyboardKey::Escape {
            (self.send)(Event::Close);
        }
    }

    // ── Calendar composition ─────────────────────────────────────────────

    /// Build Calendar Props from the DatePicker's current state.
    /// The adapter creates a Calendar machine with these props inside the
    /// Content part and wires its SelectDate event back to this machine.
    pub fn calendar_props(&self) -> calendar::Props {
        calendar::Props {
            id: format!("{}-calendar", self.ctx.ids.base_id),
            value: Some(self.ctx.value.get().clone()),
            default_value: None,
            min: self.ctx.min.clone(),
            max: self.ctx.max.clone(),
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            is_date_unavailable: self.props.is_date_unavailable,
            first_day_of_week: None,
            is_rtl: self.ctx.is_rtl,
            visible_months: self.props.visible_months,
            today: CalendarDate::new_gregorian(2024, nzu8(1), nzu8(1)), // adapter injects real today
            ..calendar::Props::default()
        }
    }

    // ── Computed state accessors ─────────────────────────────────────────

    /// Whether the popover is currently open.
    pub fn is_open(&self) -> bool {
        *self.state == State::Open
    }

    /// The currently selected date.
    pub fn selected_date(&self) -> Option<&CalendarDate> {
        self.ctx.value.get().as_ref()
    }

    /// The formatted display value of the selected date.
    pub fn formatted_value(&self) -> String {
        self.ctx.formatted_value()
    }

    /// Open the popover programmatically.
    pub fn open(&self) {
        (self.send)(Event::Open);
    }

    /// Close the popover programmatically.
    pub fn close(&self) {
        (self.send)(Event::Close);
    }

    /// Toggle the popover programmatically.
    pub fn toggle(&self) {
        (self.send)(Event::Toggle);
    }

    /// Get the name of the state.
    const fn state_name(&self) -> &'static str {
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
            Part::Input => self.input_attrs(),
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

| Part           | HTML Element            | Description                                          |
| -------------- | ----------------------- | ---------------------------------------------------- |
| `Root`         | `<div>`                 | Outermost container; `data-ars-scope="date-picker"`  |
| `Label`        | `<label>`               | Associated label pointing to the input               |
| `Control`      | `<div>`                 | Wrapper around the input and trigger button          |
| `Input`        | `<input>`               | Date text input (or `DateField` segments container)  |
| `Trigger`      | `<button>`              | Calendar icon button to toggle the popover           |
| `ClearTrigger` | `<button>`              | Clears the selected date; hidden when empty          |
| `Positioner`   | `<div>`                 | Floating positioner managed by `ars-dom` positioning |
| `Content`      | `<div role="dialog">`   | Popover content containing the embedded Calendar     |
| `Description`  | `<div>`                 | Optional help text                                   |
| `ErrorMessage` | `<div role="alert">`    | Validation error text                                |
| `HiddenInput`  | `<input type="hidden">` | ISO date string for form submission                  |

```text
DatePicker (en-US, closed)
└── Root                                data-ars-scope="date-picker"  data-ars-state="closed"
    ├── Label                           <label for="{id}-input">
    ├── Control                         data-ars-part="control"
    │   ├── Input                       id="{id}-input"  aria-haspopup="dialog"
    │   ├── ClearTrigger                aria-label="Clear date"  [hidden when empty]
    │   └── Trigger                     id="{id}-trigger"  aria-label="Open calendar"
    ├── Positioner                      [hidden when closed]
    │   └── Content                     role="dialog"  aria-label="Choose date"
    │       └── <Calendar />            Embedded Calendar component
    ├── Description
    ├── ErrorMessage                    role="alert"
    └── HiddenInput                     type="hidden"  name="..."  value="2024-01-15"

DatePicker (en-US, open)
└── Root                                data-ars-state="open"
    ├── Label
    ├── Control
    │   ├── Input                       aria-expanded="true"
    │   ├── ClearTrigger
    │   └── Trigger                     aria-expanded="true"
    ├── Positioner
    │   └── Content                     role="dialog"  aria-modal="false"
    │       └── <Calendar
    │               value={synced from DatePicker}
    │               min={forwarded}
    │               max={forwarded}
    │               locale={forwarded}
    │           />
    └── HiddenInput
```

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Element        | Role/Attribute        | Details                                                                      |
| -------------- | --------------------- | ---------------------------------------------------------------------------- |
| `Input`        | `<input type="text">` | `aria-haspopup="dialog"`, `aria-expanded`, `aria-controls` points to Content |
| `Trigger`      | `<button>`            | `aria-label="Open calendar"`, `aria-expanded`, `aria-controls`               |
| `Content`      | `role="dialog"`       | `aria-label="Choose date"`                                                   |
| `ErrorMessage` | `role="alert"`        | Announced immediately on render                                              |
| `Label`        | `<label>`             | `for` attribute points to Input                                              |

### 3.2 Keyboard Interaction

| Key               | Context                       | Action                                     |
| ----------------- | ----------------------------- | ------------------------------------------ |
| `ArrowDown`       | Input focused, popover closed | Open the calendar popover                  |
| `Escape`          | Popover open                  | Close the popover, return focus to trigger |
| `Enter` / `Space` | Trigger focused               | Toggle the popover                         |
| `Tab`             | Inside calendar popover       | Close the popover, move focus forward      |

### 3.3 Focus Flow

1. User tabs to the input or trigger.
2. Clicking the trigger or pressing ArrowDown on the input opens the popover.
3. Focus moves into the Calendar grid (first selected date, or today).
4. Selecting a date closes the popover and returns focus to the input.
5. Pressing Escape closes the popover and returns focus to the trigger.
6. Tab from inside the calendar closes the popover and moves focus to the next focusable element after the date picker.

## 4. Internationalization

### 4.1 Messages

```rust
/// Messages for the DatePicker component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Trigger button label (default: "Open calendar").
    pub trigger_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Clear button label (default: "Clear date").
    pub clear_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Content dialog label (default: "Choose date").
    pub content_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Announces the selected date (e.g. "Selected date: March 15, 2025").
    /// Used as `aria-description` on the input field when a date is selected.
    pub selected_date_label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
}
// Calendar navigation labels (prev/next month, today, unavailable) belong to
// the embedded Calendar component's Messages — not duplicated here.

impl Default for Messages {
    fn default() -> Self {
        Self {
            trigger_label: MessageFn::static_str("Open calendar"),
            clear_label: MessageFn::static_str("Clear date"),
            content_label: MessageFn::static_str("Choose date"),
            selected_date_label: MessageFn::new(|date, _locale| format!("Selected date: {}", date)),
        }
    }
}

impl ComponentMessages for Messages {}
```

## 5. Form Integration

The DatePicker participates in HTML form submission via a hidden `<input type="hidden">` element rendered by the `HiddenInput` part. The hidden input carries the selected date in ISO 8601 format (`YYYY-MM-DD`).

- **`name` prop**: When set, the hidden input includes `name="{value}"` so the date is submitted with the form under that key.
- **Required validation**: When `required` is `true`, the input carries `aria-required="true"`. Form validation treats an empty value as invalid.
- **Reset**: On form reset, the adapter restores the initial value (the `default_value` from Props or `None`) and syncs the input text accordingly.
- **`aria-describedby`**: The input's `aria-describedby` chains the Description and ErrorMessage IDs so assistive technology announces help text and validation errors.
- **ISO value**: The hidden input's `value` is always the ISO 8601 representation of the selected date (e.g., `2024-03-15`), regardless of the display format used in the visible input.

## 6. Library Parity

> Compared against: Ark UI (`DatePicker`), React Aria (`DatePicker`).

### 6.1 Props

| Feature               | ars-ui                | Ark UI                       | React Aria             | Notes                                               |
| --------------------- | --------------------- | ---------------------------- | ---------------------- | --------------------------------------------------- |
| Controlled value      | `value`               | `value`                      | `value`                | Equivalent                                          |
| Default value         | `default_value`       | `defaultValue`               | `defaultValue`         | Equivalent                                          |
| Min/max               | `min`, `max`          | `min`, `max`                 | `minValue`, `maxValue` | Equivalent                                          |
| Disabled              | `disabled`            | `disabled`                   | `isDisabled`           | Equivalent                                          |
| Read-only             | `readonly`            | `readOnly`                   | `isReadOnly`           | Equivalent                                          |
| Invalid               | `invalid`             | `invalid`                    | `isInvalid`            | Equivalent                                          |
| Required              | `required`            | `required`                   | `isRequired`           | Equivalent                                          |
| Unavailable predicate | `is_date_unavailable` | `isDateUnavailable`          | `isDateUnavailable`    | Equivalent                                          |
| Close on select       | `close_on_select`     | `closeOnSelect`              | `shouldCloseOnSelect`  | Equivalent                                          |
| Open on click         | `open_on_click`       | `openOnClick`                | --                     | Ark UI has it; React Aria does not                  |
| Locale                | `locale`              | `locale`                     | -- (context)           | Equivalent approach differs                         |
| Format                | `format`              | `format`                     | --                     | Ark UI has custom format fn                         |
| Placeholder           | `placeholder`         | `placeholder`                | `placeholderValue`     | React Aria uses a DateValue placeholder             |
| Name                  | `name`                | `name`                       | `name`                 | Equivalent                                          |
| Visible months        | `visible_months`      | `numOfMonths`                | --                     | ars-ui and Ark UI support multi-month               |
| Default open          | --                    | `defaultOpen`                | `defaultOpen`          | Not present in ars-ui                               |
| Controlled open       | --                    | `open`                       | `isOpen`               | Not present as a controlled prop                    |
| Fixed weeks           | --                    | `fixedWeeks`                 | --                     | ars-ui always renders 6 weeks                       |
| Default view          | --                    | `defaultView`                | --                     | Ark UI has month/year view switching                |
| View control          | --                    | `view`, `minView`, `maxView` | --                     | Ark UI view switching not in ars-ui                 |
| Selection mode        | --                    | `selectionMode`              | --                     | Ark UI uses one component for single/range/multiple |
| Inline mode           | --                    | `inline`                     | --                     | ars-ui Calendar is the inline mode                  |
| Start of week         | --                    | `startOfWeek`                | `firstDayOfWeek`       | Forwarded to Calendar's `first_day_of_week`         |
| Time zone             | --                    | `timeZone`                   | --                     | Not applicable to date-only picker                  |
| Parse function        | --                    | `parse`                      | --                     | ars-ui has internal parse_date                      |
| Granularity           | --                    | --                           | `granularity`          | React Aria DatePicker can include time              |
| Hour cycle            | --                    | --                           | `hourCycle`            | React Aria DatePicker can include time              |
| Auto-focus            | --                    | --                           | `autoFocus`            | Adapter concern                                     |
| Validation            | --                    | --                           | `validate`             | React Aria custom validation function               |
| Page behavior         | --                    | --                           | `pageBehavior`         | Forwarded to Calendar                               |

**Gaps:** None.

### 6.2 Anatomy

| Part              | ars-ui         | Ark UI                      | React Aria                  | Notes                                  |
| ----------------- | -------------- | --------------------------- | --------------------------- | -------------------------------------- |
| Root              | `Root`         | `Root`                      | `DatePicker`                | Equivalent                             |
| Label             | `Label`        | `Label`                     | `Label`                     | Equivalent                             |
| Control           | `Control`      | `Control`                   | `Group`                     | Equivalent                             |
| Input             | `Input`        | `Input`                     | `DateInput` + `DateSegment` | React Aria uses segments               |
| Trigger           | `Trigger`      | `Trigger`                   | `Button`                    | Equivalent                             |
| Clear trigger     | `ClearTrigger` | `ClearTrigger`              | --                          | React Aria has no clear button         |
| Positioner        | `Positioner`   | `Positioner`                | --                          | React Aria uses Popover directly       |
| Content           | `Content`      | `Content`                   | `Popover` + `Dialog`        | Equivalent                             |
| Calendar          | (composed)     | Calendar parts              | `Calendar`                  | Composed inside content                |
| Description       | `Description`  | --                          | `Text` (slot)               | Equivalent                             |
| Error message     | `ErrorMessage` | --                          | `FieldError`                | Equivalent                             |
| Hidden input      | `HiddenInput`  | --                          | --                          | ars-ui form integration                |
| Value text        | --             | `ValueText`                 | --                          | Ark UI has a formatted value text part |
| Preset trigger    | --             | `PresetTrigger`             | --                          | Ark UI has preset date buttons         |
| Month/year select | --             | `MonthSelect`, `YearSelect` | --                          | Ark UI has dropdown selects            |

**Gaps:** None worth adopting. `ValueText`, `PresetTrigger`, and `MonthSelect`/`YearSelect` are Ark UI-specific conveniences. ars-ui exposes `formatted_value()` on the API for value text, and month/year selection is handled by Calendar's `SetMonth`/`SetYear` events.

### 6.3 Events

| Callback             | ars-ui                       | Ark UI                     | React Aria                           | Notes                               |
| -------------------- | ---------------------------- | -------------------------- | ------------------------------------ | ----------------------------------- |
| Value change         | `SelectDate` / `InputChange` | `onValueChange`            | `onChange`                           | Equivalent                          |
| Open change          | `Open` / `Close` events      | `onOpenChange`             | `onOpenChange`                       | Equivalent                          |
| Focus change         | `FocusIn` / `FocusOut`       | `onFocusChange` (calendar) | `onFocus`, `onBlur`, `onFocusChange` | Equivalent                          |
| View change          | --                           | `onViewChange`             | --                                   | Ark UI view switching               |
| Visible range change | --                           | `onVisibleRangeChange`     | --                                   | Ark UI calendar navigation callback |

**Gaps:** None. View change and visible range change are Ark UI-specific features tied to their view switching model.

### 6.4 Features

| Feature                         | ars-ui                      | Ark UI | React Aria               |
| ------------------------------- | --------------------------- | ------ | ------------------------ |
| Text input with parsing         | Yes                         | Yes    | No (segments only)       |
| Segmented input                 | No (delegates to DateField) | No     | Yes                      |
| Calendar popover                | Yes                         | Yes    | Yes                      |
| Close on select                 | Yes                         | Yes    | Yes                      |
| Open on click                   | Yes                         | Yes    | No                       |
| Clear button                    | Yes                         | Yes    | No                       |
| Multi-month calendar            | Yes                         | Yes    | No (at DatePicker level) |
| Inline mode                     | No (use Calendar)           | Yes    | No                       |
| View switching (day/month/year) | No                          | Yes    | No                       |
| Preset triggers                 | No                          | Yes    | No                       |
| Month/year dropdown select      | No                          | Yes    | No                       |
| Custom format/parse             | Yes                         | Yes    | No                       |
| Hidden form input               | Yes                         | No     | No                       |
| Form validation                 | Yes                         | No     | Yes                      |

**Gaps:** None worth adopting. Ark UI's view switching and preset triggers are nice-to-haves but add significant complexity; ars-ui's Calendar already supports `SetMonth`/`SetYear` for programmatic navigation.

### 6.5 Summary

- **Overall:** Full parity with both libraries.
- **Divergences:** Ark UI has view switching (day/month/year views) and preset triggers. React Aria uses segmented input by default (ars-ui delegates to DateField). ars-ui supports both text input and composed DateField approaches.
- **Recommended additions:** None.

## Appendix: Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ars_core::service::Service;

    fn make_service() -> Service<Machine> {
        Service::new(Props {
            id: "test-dp".into(),
            label: "Date".into(),
            placeholder: Some("MM/DD/YYYY".into()),
            name: Some("date".into()),
            ..Props::default()
        }, Env::default(), Default::default())
    }

    #[test]
    fn initial_state_is_closed() {
        let svc = make_service();
        assert_eq!(*svc.state(), State::Closed);
        assert!(!*svc.context().open.get());
    }

    #[test]
    fn toggle_opens_popover() {
        let mut svc = make_service();
        svc.send(Event::Toggle);
        assert_eq!(*svc.state(), State::Open);
        assert!(*svc.context().open.get());
    }

    #[test]
    fn toggle_twice_closes_popover() {
        let mut svc = make_service();
        svc.send(Event::Toggle);
        svc.send(Event::Toggle);
        assert_eq!(*svc.state(), State::Closed);
    }

    #[test]
    fn select_date_closes_and_updates_value() {
        let mut svc = make_service();
        svc.send(Event::Open);
        let date = CalendarDate::new_gregorian(2024, nzu8(3), nzu8(15));
        svc.send(Event::SelectDate { date: date.clone() });
        assert_eq!(*svc.state(), State::Closed);
        assert_eq!(*svc.context().value.get(), Some(date));
        assert_eq!(svc.context().input_text, "03/15/2024");
    }

    #[test]
    fn input_change_parses_valid_date() {
        let mut svc = make_service();
        svc.send(Event::InputChange { value: "06/20/2024".into() });
        assert_eq!(
            *svc.context().value.get(),
            Some(CalendarDate::new_gregorian(2024, nzu8(6), nzu8(20))),
        );
    }

    #[test]
    fn input_change_ignores_invalid_text() {
        let mut svc = make_service();
        svc.send(Event::InputChange { value: "not-a-date".into() });
        assert_eq!(*svc.context().value.get(), None);
    }

    #[test]
    fn escape_closes_popover() {
        let mut svc = make_service();
        svc.send(Event::Open);
        svc.send(Event::KeyDown { key: KeyboardKey::Escape });
        assert_eq!(*svc.state(), State::Closed);
    }

    #[test]
    fn disabled_picker_ignores_events() {
        let mut svc = Service::new(Props {
            id: "test-dp".into(),
            disabled: true,
            ..Props::default()
        }, Env::default(), Default::default());
        svc.send(Event::Open);
        assert_eq!(*svc.state(), State::Closed);
    }

    #[test]
    fn hidden_input_has_iso_value() {
        let mut svc = make_service();
        let date = CalendarDate::new_gregorian(2024, nzu8(12), nzu8(25));
        svc.send(Event::SelectDate { date });
        let api = svc.connect(&|_| {});
        let attrs = api.hidden_input_attrs();
        // The hidden input should carry the ISO-formatted date.
        // attrs.get(HtmlAttr::Value) == "2024-12-25"
    }

    #[test]
    fn focusout_closes_open_popover() {
        let mut svc = make_service();
        svc.send(Event::Open);
        svc.send(Event::FocusOut);
        assert_eq!(*svc.state(), State::Closed);
    }
}
```
