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

`DatePicker` is a composite component that combines a date text input with a `Calendar` in a popover.

- The DatePicker machine owns the popover open/close lifecycle, the selected
  value, input text parsing/validation, and the ARIA/`data-ars-*` surface.
- The `Calendar` is **composed by the adapter** from [`Api::calendar_props`](#18-connect--api):
  the adapter instantiates a `Calendar` machine inside the `Content` part and
  bridges its `SelectDate` event back as [`Event::SelectDate`](#12-events). The
  DatePicker does not embed a `Calendar` `Service` in its own context.

> **Spec-vs-implementation reconciliation.** The §1.7/§1.8 code blocks predate
> the current core engine and sibling components. The agnostic implementation
> (`crates/ars-components/src/date_time/date_picker/`) keeps the semantics below
> while using the real APIs, and these sections have been updated to match:
>
> - **Open/closed is owned by [`State`](#11-states).** There is no parallel
>   `open: Bindable<bool>` context field — the connect API renders from the
>   state. When `open` is **uncontrolled**, user events mutate `State` directly.
>   When `open` is **controlled** (`props.open.is_some()`), user events do **not**
>   mutate `State`: the request is recorded in `Context::requested_open` and
>   signalled via `Effect::OpenChange`, and the parent reconciles by updating the
>   `open` prop (applied by `SyncProps`). The adapter invokes
>   `Props::on_open_change` with `requested_open` on every `OpenChange`.
>   `OpenChange` is a user-interaction signal only — it is not emitted by
>   `SyncProps` (a controlled prop change is the parent's own doing) or at mount.
> - **Focus is adapter-driven via named effects.** The machine declares a typed
>   `Effect` enum and emits [`PendingEffect::named`] intents (`OpenChange`,
>   `ValueChange`, `FocusCalendar`, `RestoreFocusToTrigger`, `RestoreFocusToInput`);
>   the adapter performs the live, element-handle-based focus. The core never calls
>   `use_platform_effects()`/`focus_element_by_id` (matches `popover`/`dialog`
>   and the element/ref handling note in issue #289).
> - **Real `CalendarDate` API.** `year()`/`month()`/`day()` are methods returning
>   plain integers, `new_gregorian` is fallible, and `CalendarDate` has no `Ord`
>   so range checks use `CalendarDate::compare`.
> - **`is_date_unavailable` reuses `calendar::IsDateUnavailableFn`** so it
>   forwards cleanly into `Api::calendar_props` and keeps `Props: Clone + PartialEq`.
> - **Controlled-prop sync.** A `SyncProps(Box<Props>)` event plus
>   `on_props_changed`/`initial_effects` keep controlled `value`/`open`/`min`/
>   `max`/`disabled` live, matching Calendar/DateField/Popover.
> - **Format-aware parse/format.** `format_date`/`parse_date` honour the resolved
>   `format` pattern (field order + separator), so the locale-default `format` is
>   not dead code.

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
    /// Focus entered the input field. With `open_on_click`, opens the calendar.
    /// Adapters dispatch it from input focus only — not trigger focus (the
    /// trigger's own click/keydown handlers manage opening).
    FocusIn,
    /// Focus left the date picker entirely.
    FocusOut,
    /// Keyboard event on the input or trigger.
    KeyDown { key: KeyboardKey },
    /// Re-apply context-backed prop fields after a props change. Emitted by
    /// [`on_props_changed`](#17-full-machine-implementation) so controlled
    /// `value`/`open` and the cached `min`/`max`/`disabled`/`format` fields
    /// follow parent-driven prop updates.
    SyncProps(Box<Props>),
}
```

### 1.3 Context

```rust
/// Context for the DatePicker component.
///
/// Only `Clone + Debug` are required by the `Machine` trait. `PartialEq` is not
/// derived — nothing compares whole contexts, and `Messages` holds `MessageFn`
/// closures that have no structural equality.
#[derive(Clone, Debug)]
pub struct Context {
    /// The selected date value (controlled/uncontrolled).
    pub value: Bindable<Option<CalendarDate>>,
    /// Raw text in the input field.
    pub input_text: String,
    /// Last successfully parsed date from input text.
    pub parsed_date: Option<CalendarDate>,
    /// Most recently requested value, carried to the adapter's value-change
    /// wiring via `Effect::ValueChange`. Distinct from `value`: when `value` is
    /// controlled, `value.get()` returns the parent's committed value, so the
    /// requested date would otherwise be invisible. `None` is a requested clear.
    pub requested_value: Option<CalendarDate>,
    /// Most recently requested open state, carried to the adapter's
    /// `on_open_change` wiring via `Effect::OpenChange`. When `open` is
    /// controlled the machine does not mutate `State` on user events; it records
    /// the request here and the parent reconciles by updating the `open` prop.
    /// The adapter reads this (not `is_open()`) when an `OpenChange` fires.
    pub requested_open: bool,
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
    /// Adapter-injected "today" date, forwarded to the embedded Calendar.
    pub today: CalendarDate,
    /// Disabled state.
    pub disabled: bool,
    /// Read-only state.
    pub readonly: bool,
    /// Whether clicking/focusing the input opens the popover.
    pub open_on_click: bool,
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
            Some(date) => format_date(date, &self.format),
            None => String::new(),
        }
    }

    /// Attempt to parse a text string into a CalendarDate.
    pub fn parse_input(&self, text: &str) -> Option<CalendarDate> {
        parse_date(text, &self.format)
    }

    /// Sync the input text to reflect the current value.
    pub fn sync_input_text(&mut self) {
        self.input_text = self.formatted_value();
    }
}

/// One of the three ordered fields in a date format pattern.
enum DateField { Year, Month, Day }

/// Locale-appropriate default date format pattern (used when `Props::format` is
/// `None`). Field order comes from [`ars_i18n::date_order`], which uses real
/// locale data — CLDR via ICU4X, or the browser `Intl` API on wasm — falling
/// back to a `(language, region)` heuristic, so every locale gets the correct
/// month/day/year order and the order matches the sibling `date_field`. The
/// separator stays a small locale heuristic (`.` for German/Korean, `/` else).
fn default_format_for_locale(locale: &Locale) -> String {
    let separator = match (locale.language(), locale.region()) {
        ("de", Some("DE")) | ("ko", Some("KR")) => '.',
        _ => '/',
    };
    let order: [&str; 3] = match date_order(locale) {
        DateOrder::MonthDayYear => ["MM", "dd", "yyyy"],
        DateOrder::DayMonthYear => ["dd", "MM", "yyyy"],
        DateOrder::YearMonthDay => ["yyyy", "MM", "dd"],
    };
    format!("{}{separator}{}{separator}{}", order[0], order[1], order[2])
}

/// Parse a format pattern into its separator and ordered fields. The separator
/// is the first non-alphabetic character; each token's leading letter selects
/// the field (`y`/`Y` → year, `d`/`D` → day, otherwise month).
fn parse_format(format: &str) -> (char, [DateField; 3]) {
    let sep = format.chars().find(|c| !c.is_ascii_alphabetic()).unwrap_or('/');
    let mut order = [DateField::Month, DateField::Day, DateField::Year];
    let tokens: Vec<&str> = format.split(sep).collect();
    if tokens.len() == 3 {
        for (index, token) in tokens.iter().enumerate() {
            order[index] = match token.chars().next() {
                Some('y' | 'Y') => DateField::Year,
                Some('d' | 'D') => DateField::Day,
                _ => DateField::Month,
            };
        }
    }
    (sep, order)
}

/// Format a `CalendarDate` in the pattern's field order (month/day zero-padded
/// to two digits, year to four). Uses the real `year()`/`month()`/`day()`
/// accessor methods.
fn format_date(date: &CalendarDate, format: &str) -> String {
    let (sep, order) = parse_format(format);
    let mut out = String::new();
    for (index, field) in order.iter().enumerate() {
        if index > 0 { out.push(sep); }
        match field {
            DateField::Year => out.push_str(&format!("{:04}", date.year())),
            DateField::Month => out.push_str(&format!("{:02}", date.month())),
            DateField::Day => out.push_str(&format!("{:02}", date.day())),
        }
    }
    out
}

/// Parse a date string in the pattern's field order. `new_gregorian` is fallible
/// (validates the date), so it is mapped to `Option` via `.ok()`.
fn parse_date(text: &str, format: &str) -> Option<CalendarDate> {
    let (sep, order) = parse_format(format);
    let tokens: Vec<&str> = text.split(sep).collect();
    if tokens.len() != 3 { return None; }
    let (mut year, mut month, mut day) = (None, None, None);
    for (index, field) in order.iter().enumerate() {
        let parsed: i64 = tokens[index].trim().parse().ok()?;
        match field {
            DateField::Year => year = Some(parsed),
            DateField::Month => month = Some(parsed),
            DateField::Day => day = Some(parsed),
        }
    }
    let year = i32::try_from(year?).ok()?;
    let month = u8::try_from(month?).ok()?;
    let day = u8::try_from(day?).ok()?;
    CalendarDate::new_gregorian(year, month, day).ok()
}

/// Classification of input text for `Event::InputChange`.
enum InputClass {
    /// A complete, valid date.
    Valid(CalendarDate),
    /// Three numeric fields that do not form a valid date (e.g. `02/30/2024`) —
    /// treated as a rejected commit, not in-progress text.
    CompleteInvalid,
    /// In-progress / non-numeric text still being edited.
    Partial,
}

/// A "complete" entry is exactly three numeric fields in the format separator;
/// only those can be rejected as invalid. Anything else is `Partial`.
fn classify_input(text: &str, format: &str) -> InputClass {
    let (sep, _order) = parse_format(format);
    let tokens: Vec<&str> = text.split(sep).collect();
    let complete_numeric = tokens.len() == 3
        && tokens.iter().all(|t| {
            let t = t.trim();
            !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit())
        });
    if !complete_numeric { return InputClass::Partial; }
    match parse_date(text, format) {
        Some(date) => InputClass::Valid(date),
        None => InputClass::CompleteInvalid,
    }
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
    /// Predicate for unavailable dates (forwarded to Calendar). Reuses
    /// `calendar::IsDateUnavailableFn` (an `Arc`-backed `Callback`) so it
    /// forwards directly into `Api::calendar_props` and keeps `Props: PartialEq`.
    pub is_date_unavailable: Option<calendar::IsDateUnavailableFn>,
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
    /// The "today" date, injected by the adapter for testability/SSR. Forwarded
    /// to the embedded Calendar's `today` so an empty picker opens on the current
    /// month and marks the correct day. Defaults to a fixed date (matching
    /// `calendar::Props::default().today`); adapters inject the real today.
    pub today: CalendarDate,
    /// Called whenever the open state changes. Fired by the adapter from the
    /// [`Effect::OpenChange`] intent with the new open value; a controlled-`open`
    /// parent uses it to reconcile its state after a user-driven open/close
    /// (mirrors `popover`/`dialog`).
    pub on_open_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,
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
            today: CalendarDate::new_gregorian(2025, 1, 1)
                .expect("2025-01-01 is a valid Gregorian date"),
            on_open_change: None,
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
/// Typed identifier for every named effect the machine emits. The adapter
/// performs the live, element-handle-based work from these intents (the core
/// never touches the DOM).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Notify the consumer that an open/close was requested. The adapter reads
    /// `Context::requested_open` (not `is_open()`, which still reflects the
    /// committed state) and invokes `Props::on_open_change`. Emitted only from
    /// user events — never from `SyncProps` (a controlled prop change is the
    /// parent's own doing) or at mount. When `open` is controlled the `State` is
    /// not mutated; the parent reconciles via the `open` prop.
    OpenChange,
    /// Notify the consumer that the selected value changed. The adapter reads
    /// `Context::requested_value` (the requested date — not `value.get()`, which
    /// a controlled bindable holds at the parent's value) and forwards it.
    /// Emitted on a selection, an accepted typed entry, or a clear. Mirrors the
    /// `requested_value` + `ValueChange` convention of other value components.
    ValueChange,
    /// Move focus into the embedded calendar. Emitted when the popover opens via
    /// the trigger / `ArrowDown` (an explicit `Open`), not when opened by input
    /// focus (`FocusIn`), which keeps focus in the input for typing.
    FocusCalendar,
    /// Return focus to the trigger. Emitted on `Close`/Escape.
    RestoreFocusToTrigger,
    /// Return focus to the input. Emitted when a selection closes the popover.
    RestoreFocusToInput,
}

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

        let locale = env.locale.clone();
        let format = props
            .format
            .clone()
            .unwrap_or_else(|| default_format_for_locale(&locale));

        let input_text = value
            .get()
            .as_ref()
            .map(|d| format_date(d, &format))
            .unwrap_or_default();

        // `State` is the single source of truth for open/closed; controlled
        // `open`/`default_open` only seed the initial state.
        let open = props.open.unwrap_or(props.default_open);
        let initial_state = if open { State::Open } else { State::Closed };

        // The `requested_*` mirrors seed from the initial state; they are only
        // read by the adapter on an `Effect::ValueChange` / `Effect::OpenChange`,
        // neither of which fires at init.
        let requested_value = value.get().clone();
        let requested_open = open;

        let ctx = Context {
            value,
            input_text,
            parsed_date: None,
            requested_value,
            requested_open,
            locale,
            messages: messages.clone(),
            format,
            min: props.min.clone(),
            max: props.max.clone(),
            today: props.today.clone(),
            disabled: props.disabled,
            readonly: props.readonly,
            open_on_click: props.open_on_click,
            is_touched: false,
            name: props.name.clone(),
            required: props.required,
            is_rtl: props.is_rtl,
            ids: ComponentIds::from_id(&props.id),
        };

        (initial_state, ctx)
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        debug_assert_eq!(old.id, new.id, "DatePicker id must remain stable after init");
        if old == new {
            Vec::new()
        } else {
            vec![Event::SyncProps(Box::new(new.clone()))]
        }
    }

    fn initial_effects(
        state: &Self::State,
        _ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Vec<PendingEffect<Self>> {
        // A picker that boots `Open` focuses the calendar. It does NOT emit
        // `OpenChange`: that is a user-interaction signal, and the initial open
        // state is the parent's own configuration (no `onOpenChange` on mount).
        if *state == State::Open {
            vec![PendingEffect::named(Effect::FocusCalendar)]
        } else {
            Vec::new()
        }
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // `SyncProps` flows through even when disabled so a parent can take the
        // picker *out of* the disabled state. Controlled `open` forces the
        // matching state; uncontrolled (`None`) preserves the current state.
        if let Event::SyncProps(new_props) = event {
            let new_props = new_props.as_ref().clone();
            let was_open = *state == State::Open;
            let target = match new_props.open {
                Some(true) => Some(State::Open),
                Some(false) => Some(State::Closed),
                None => None,
            };
            let opening = !was_open && target.as_ref() == Some(&State::Open);
            let closing = was_open && target.as_ref() == Some(&State::Closed);
            let mut plan = match target {
                Some(next) => TransitionPlan::to(next),
                None => TransitionPlan::new(),
            };
            plan = plan.apply(move |ctx| {
                if let Some(open) = new_props.open { ctx.requested_open = open; }
                sync_props_into_ctx(ctx, &new_props);
            });
            // No `OpenChange` here: a controlled `open` prop change is the parent's
            // own doing. Focus still follows the open/close that lands.
            if opening {
                plan = plan.with_effect(PendingEffect::named(Effect::FocusCalendar));
            } else if closing {
                plan = plan.with_effect(PendingEffect::named(Effect::RestoreFocusToTrigger));
            }
            return Some(plan);
        }

        if ctx.disabled { return None; }

        match event {
            // ── Popover open / close ──────────────────────────────────────
            // `open_request` honors controlled `open`: when controlled it records
            // the request + emits `OpenChange` without mutating `State`; when
            // uncontrolled it transitions and fires the focus effect.
            Event::Open => {
                if ctx.readonly { return None; }
                open_request(state, props, true, Some(PendingEffect::named(Effect::FocusCalendar)))
            }

            Event::Close => {
                open_request(state, props, false, Some(PendingEffect::named(Effect::RestoreFocusToTrigger)))
            }

            Event::Toggle => match state {
                State::Closed => Self::transition(state, &Event::Open, ctx, props),
                State::Open => Self::transition(state, &Event::Close, ctx, props),
            },

            // ── Date selection (from calendar) ───────────────────────────
            Event::SelectDate { date } => {
                if ctx.readonly { return None; }
                // Defense-in-depth: reject a selection the picker disallows (the
                // calendar should never offer it, but a scripted/stale/buggy
                // event might), mirroring the typed-input constraints.
                let in_range = ctx.min.as_ref().is_none_or(|m| date.compare(m) != Ordering::Less)
                    && ctx.max.as_ref().is_none_or(|m| date.compare(m) != Ordering::Greater);
                let available = props.is_date_unavailable.as_ref().is_none_or(|p| !p(date));
                if !(in_range && available) { return None; }
                let date = date.clone();
                let should_close = props.close_on_select;
                let open_controlled = props.open.is_some();
                // Commit the close only when open is uncontrolled; otherwise just
                // request it and let the parent reconcile.
                let commit_close = should_close && !open_controlled;
                // Suppress a no-op `ValueChange` when the date is already selected.
                let value_changes = ctx.value.get().as_ref() != Some(&date);
                let next_state = if commit_close { State::Closed } else { State::Open };
                let mut plan = TransitionPlan::to(next_state).apply(move |ctx| {
                    ctx.requested_value = Some(date.clone());
                    ctx.value.set(Some(date));
                    // Derive the visible text and `parsed_date` from the value the
                    // bindable actually exposes — for a controlled `value` that is
                    // still the parent's value, so the display never optimistically
                    // diverges from `selected_date()` / the hidden input / the
                    // composed calendar props.
                    ctx.parsed_date = ctx.value.get().clone();
                    ctx.input_text = ctx.formatted_value();
                    ctx.is_touched = true;
                    if should_close { ctx.requested_open = false; }
                });
                if value_changes {
                    plan = plan.with_effect(PendingEffect::named(Effect::ValueChange));
                }
                if should_close {
                    plan = plan.with_effect(PendingEffect::named(Effect::OpenChange));
                    if commit_close {
                        plan = plan.with_effect(PendingEffect::named(Effect::RestoreFocusToInput));
                    }
                }
                Some(plan)
            }

            // ── Input text change ────────────────────────────────────────
            Event::InputChange { value } => {
                if ctx.readonly { return None; }
                let text = value.clone();
                // Decide the committed-value outcome up front so `ValueChange`
                // fires exactly when the value changes. A complete date that is
                // rejected (out of range, unavailable, or not a real date such as
                // `02/30/2024`) CLEARS the value, so the hidden input / calendar
                // never submit a stale date contradicting the field; partial /
                // non-numeric text leaves the value untouched.
                let (committed_value, value_changed): (Option<CalendarDate>, bool) =
                    match classify_input(&text, &ctx.format) {
                        InputClass::Valid(date) => {
                            let in_range = ctx.min.as_ref().is_none_or(|m| date.compare(m) != Ordering::Less)
                                && ctx.max.as_ref().is_none_or(|m| date.compare(m) != Ordering::Greater);
                            let available = props.is_date_unavailable.as_ref().is_none_or(|p| !p(&date));
                            if in_range && available { (Some(date), true) } else { (None, true) }
                        }
                        InputClass::CompleteInvalid => (None, true),
                        InputClass::Partial if text.is_empty() => (None, true),
                        InputClass::Partial => (None, false),
                    };
                // Suppress a no-op `ValueChange` when the value would not change.
                let value_changes = value_changed && committed_value.as_ref() != ctx.value.get().as_ref();
                let mut plan = TransitionPlan::context_only(move |ctx| {
                    ctx.is_touched = true;
                    if value_changed {
                        let accepted = committed_value.is_some();
                        ctx.parsed_date = committed_value.clone();
                        ctx.requested_value = committed_value.clone();
                        ctx.value.set(committed_value);
                        // For an accepted date, reflect the bindable's value so a
                        // controlled `value` the parent hasn't echoed yet does not
                        // diverge from the hidden input / calendar (the typed-input
                        // analog of `SelectDate`). A rejected complete date or
                        // explicit clear keeps the text the user sees.
                        ctx.input_text = if accepted { ctx.formatted_value() } else { text };
                    } else {
                        // In-progress (partial/unparseable) typing is preserved.
                        ctx.input_text = text;
                    }
                });
                if value_changes {
                    plan = plan.with_effect(PendingEffect::named(Effect::ValueChange));
                }
                Some(plan)
            }

            // ── Focus management ─────────────────────────────────────────
            Event::FocusIn => {
                // `readonly` blocks opening through the focus path just as it
                // blocks the explicit `Open` event. Open via focus passes no focus
                // effect — the user focused the input to type, so focus stays in
                // the field (only trigger/ArrowDown opens move it into the grid).
                if *state == State::Closed && ctx.open_on_click && !ctx.readonly {
                    open_request(state, props, true, None)
                } else {
                    None
                }
            }

            Event::FocusOut => open_request(state, props, false, None),

            // ── Keyboard shortcuts ───────────────────────────────────────
            Event::KeyDown { key } => match key {
                KeyboardKey::Escape if *state == State::Open => {
                    Self::transition(state, &Event::Close, ctx, props)
                }
                KeyboardKey::ArrowDown if *state == State::Closed => {
                    Self::transition(state, &Event::Open, ctx, props)
                }
                _ => None,
            },

            // Handled above the disabled guard.
            Event::SyncProps(_) => None,
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

/// Builds the plan for a user-driven request to open or close.
///
/// When `open` is **controlled** (`props.open.is_some()`), `State` is not mutated:
/// the request is recorded in `requested_open` and signalled via `OpenChange`, and
/// the parent reconciles by updating the `open` prop. No focus effect fires
/// (nothing visibly changed). When **uncontrolled**, `State` transitions and the
/// supplied `focus` effect (if any) fires. Returns `None` when already in the
/// requested open state.
fn open_request(
    state: &State,
    props: &Props,
    target_open: bool,
    focus: Option<PendingEffect<Machine>>,
) -> Option<TransitionPlan<Machine>> {
    if (*state == State::Open) == target_open { return None; }
    let controlled = props.open.is_some();
    let mut plan = if controlled {
        TransitionPlan::new()
    } else {
        TransitionPlan::to(if target_open { State::Open } else { State::Closed })
    };
    plan = plan
        .apply(move |ctx| ctx.requested_open = target_open)
        .with_effect(PendingEffect::named(Effect::OpenChange));
    if !controlled && let Some(focus) = focus {
        plan = plan.with_effect(focus);
    }
    Some(plan)
}

/// Re-applies the controlled value and cached scalar prop fields onto `ctx`.
/// Open/closed is owned by `State` (the `SyncProps` transition derives the
/// target state from `props.open`), so there is no `open` field to refresh.
///
/// The input text is re-synced **only when the displayed value or format
/// actually changed** — an unrelated prop change (e.g. `invalid`, `description`,
/// `disabled`) must not clobber a partial/invalid date the user is typing.
fn sync_props_into_ctx(ctx: &mut Context, props: &Props) {
    let previous_display = ctx.formatted_value();
    ctx.value.sync_controlled(props.value.clone());
    ctx.min = props.min.clone();
    ctx.max = props.max.clone();
    ctx.today = props.today.clone();
    ctx.disabled = props.disabled;
    ctx.readonly = props.readonly;
    ctx.open_on_click = props.open_on_click;
    ctx.name = props.name.clone();
    ctx.required = props.required;
    ctx.is_rtl = props.is_rtl;
    ctx.format = props
        .format
        .clone()
        .unwrap_or_else(|| default_format_for_locale(&ctx.locale));
    let next_display = ctx.formatted_value();
    if next_display != previous_display {
        ctx.input_text = next_display;
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

        // Announce the selected date from the live value, so the description is
        // present for `default_value` and controlled values too — not only for
        // dates typed or picked during this session.
        if let Some(date) = self.ctx.value.get() {
            let formatted = format_date(date, &self.ctx.format);
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
            // Native `required` on the visible control drives browser constraint
            // validation (ARIA alone does not); the hidden input is `type=hidden`
            // and cannot validate.
            attrs.set_bool(HtmlAttr::Required, true);
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
        // Explicit `type="button"` so activating the trigger never submits a
        // surrounding form (the HTML default button type is `submit`).
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.trigger_label)(&self.ctx.locale));
        // The trigger advertises the popup it opens (matches the Popover trigger
        // convention); the Input carries the same `aria-haspopup`.
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.is_open() { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), self.ctx.ids.part("content"));
        attrs.set(HtmlAttr::TabIndex, "0");
        // `readonly` blocks every opening path, so the trigger advertises it is
        // non-actionable rather than rendering enabled-but-inert (matches the
        // input and clear-trigger, which already expose readonly/disabled).
        if self.ctx.disabled || self.ctx.readonly {
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
        // Explicit `type="button"` so clearing never submits a surrounding form.
        attrs.set(HtmlAttr::Type, "button");
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
        // Canonical ISO 8601 via `to_iso8601()` (the date's ISO slots), so a
        // non-Gregorian value still submits the correct calendar-independent
        // date rather than its display-calendar `year()/month()/day()` fields.
        let value = self.ctx.value.get()
            .as_ref()
            .map(CalendarDate::to_iso8601)
            .unwrap_or_default();
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name);
        }
        // A disabled control is excluded from form submission — mirror the
        // disabled visible input/buttons so a disabled picker submits nothing.
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
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
    ///
    /// Returns `true` when the key was handled so adapters can prevent the
    /// follow-up native button-activation click — without this, Enter/Space
    /// would toggle once on keydown and again on the synthesized click, leaving
    /// the popover back in its original state (mirrors
    /// `collapsible::Api::on_trigger_keydown`).
    pub fn on_trigger_keydown(&self, data: &KeyboardEventData) -> bool {
        match data.key {
            KeyboardKey::Enter | KeyboardKey::Space => { (self.send)(Event::Toggle); true }
            KeyboardKey::ArrowDown => { (self.send)(Event::Open); true }
            _ => false,
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

    /// Handle focus entering the **input field**. Wire to the `Input` only — not
    /// the `Trigger` (which has its own click/keydown handlers); wiring it on the
    /// trigger would open on trigger focus and the activating click would
    /// immediately toggle it closed.
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
            id: format!("{}-calendar", self.ctx.ids.id()),
            value: Some(self.ctx.value.get().clone()),
            min: self.ctx.min.clone(),
            max: self.ctx.max.clone(),
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            is_date_unavailable: self.props.is_date_unavailable.clone(),
            is_rtl: self.ctx.is_rtl,
            visible_months: self.props.visible_months,
            // Forward the adapter-injected "today" so the calendar opens on the
            // current month and marks the correct day (otherwise it falls back to
            // the fixed `calendar::Props::default().today`).
            today: self.ctx.today.clone(),
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
| `Trigger`      | `<button type="button">` | Calendar icon button to toggle the popover (non-submit) |
| `ClearTrigger` | `<button type="button">` | Clears the selected date; hidden when empty (non-submit) |
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

| Element        | Role/Attribute        | Details                                                                                  |
| -------------- | --------------------- | ---------------------------------------------------------------------------------------- |
| `Input`        | `<input type="text">` | `aria-haspopup="dialog"`, `aria-expanded`, `aria-controls` points to Content             |
| `Trigger`      | `<button type="button">` | `aria-label="Open calendar"`, `aria-haspopup="dialog"`, `aria-expanded`, `aria-controls` |
| `Content`      | `role="dialog"`       | `aria-label="Choose date"`                                                               |
| `ErrorMessage` | `role="alert"`        | Announced immediately on render                                                          |
| `Label`        | `<label>`             | `for` attribute points to Input                                                          |

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
/// `MessageFn` carrying a locale-only label closure.
pub type LocaleLabelFn = dyn Fn(&Locale) -> String + Send + Sync;

/// `MessageFn` carrying a formatted-date plus locale label closure.
pub type SelectedDateLabelFn = dyn Fn(&str, &Locale) -> String + Send + Sync;

/// Messages for the DatePicker component.
///
/// Only `Clone` is derived; `Debug` and `PartialEq` are implemented manually
/// because `MessageFn` wraps a closure (no structural `Debug`/`Eq`).
#[derive(Clone)]
pub struct Messages {
    /// Trigger button label (default: "Open calendar").
    pub trigger_label: MessageFn<LocaleLabelFn>,
    /// Clear button label (default: "Clear date").
    pub clear_label: MessageFn<LocaleLabelFn>,
    /// Content dialog label (default: "Choose date").
    pub content_label: MessageFn<LocaleLabelFn>,
    /// Announces the selected date (e.g. "Selected date: March 15, 2025").
    /// Used as `aria-description` on the input field when a date is selected.
    pub selected_date_label: MessageFn<SelectedDateLabelFn>,
}
// Calendar navigation labels (prev/next month, today, unavailable) belong to
// the embedded Calendar component's Messages — not duplicated here.

impl Default for Messages {
    fn default() -> Self {
        Self {
            trigger_label: MessageFn::static_str("Open calendar"),
            clear_label: MessageFn::static_str("Clear date"),
            content_label: MessageFn::static_str("Choose date"),
            selected_date_label: MessageFn::new(|date: &str, _locale: &Locale| {
                format!("Selected date: {date}")
            }),
        }
    }
}

impl ComponentMessages for Messages {}
```

## 5. Form Integration

The DatePicker participates in HTML form submission via a hidden `<input type="hidden">` element rendered by the `HiddenInput` part. The hidden input carries the selected date in ISO 8601 format (`YYYY-MM-DD`).

- **`name` prop**: When set, the hidden input includes `name="{value}"` so the date is submitted with the form under that key.
- **Required validation**: When `required` is `true`, the visible input carries both the native `required` attribute and `aria-required="true"`, so the browser enforces constraint validation (ARIA alone does not) and an empty value is treated as invalid.
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
| Default open          | `default_open`        | `defaultOpen`                | `defaultOpen`          | Uncontrolled initial open state                     |
| Controlled open       | `open`                | `open`                       | `isOpen`               | Controlled open; pair with `on_open_change`         |
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
| Value change         | `requested_value` + `Effect::ValueChange` | `onValueChange`            | `onChange`                           | Equivalent (adapter forwards `requested_value`) |
| Open change          | `on_open_change` callback    | `onOpenChange`             | `onOpenChange`                       | Equivalent (fired via `Effect::OpenChange`) |
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
    use ars_core::{Env, Service};

    fn date(year: i32, month: u8, day: u8) -> CalendarDate {
        CalendarDate::new_gregorian(year, month, day).expect("valid test date")
    }

    fn make_service() -> Service<Machine> {
        Service::new(
            Props {
                id: "test-dp".into(),
                label: "Date".into(),
                placeholder: Some("MM/DD/YYYY".into()),
                name: Some("date".into()),
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        )
    }

    #[test]
    fn initial_state_is_closed() {
        let svc = make_service();
        assert_eq!(*svc.state(), State::Closed);
    }

    #[test]
    fn toggle_opens_popover() {
        let mut svc = make_service();
        drop(svc.send(Event::Toggle));
        assert_eq!(*svc.state(), State::Open);
    }

    #[test]
    fn toggle_twice_closes_popover() {
        let mut svc = make_service();
        drop(svc.send(Event::Toggle));
        drop(svc.send(Event::Toggle));
        assert_eq!(*svc.state(), State::Closed);
    }

    #[test]
    fn select_date_closes_and_updates_value() {
        let mut svc = make_service();
        drop(svc.send(Event::Open));
        let chosen = date(2024, 3, 15);
        drop(svc.send(Event::SelectDate { date: chosen.clone() }));
        assert_eq!(*svc.state(), State::Closed);
        assert_eq!(*svc.context().value.get(), Some(chosen));
        assert_eq!(svc.context().input_text, "03/15/2024");
    }

    #[test]
    fn input_change_parses_valid_date() {
        let mut svc = make_service();
        drop(svc.send(Event::InputChange { value: "06/20/2024".into() }));
        assert_eq!(*svc.context().value.get(), Some(date(2024, 6, 20)));
    }

    #[test]
    fn input_change_ignores_invalid_text() {
        let mut svc = make_service();
        drop(svc.send(Event::InputChange { value: "not-a-date".into() }));
        assert_eq!(*svc.context().value.get(), None);
    }

    #[test]
    fn escape_closes_popover() {
        let mut svc = make_service();
        drop(svc.send(Event::Open));
        drop(svc.send(Event::KeyDown { key: KeyboardKey::Escape }));
        assert_eq!(*svc.state(), State::Closed);
    }

    #[test]
    fn disabled_picker_ignores_events() {
        let mut svc = Service::new(
            Props { id: "test-dp".into(), disabled: true, ..Props::default() },
            &Env::default(),
            &Messages::default(),
        );
        drop(svc.send(Event::Open));
        assert_eq!(*svc.state(), State::Closed);
    }

    #[test]
    fn hidden_input_has_iso_value() {
        let mut svc = make_service();
        drop(svc.send(Event::SelectDate { date: date(2024, 12, 25) }));
        let api = svc.connect(&|_| {});
        assert_eq!(api.hidden_input_attrs().get(&HtmlAttr::Value), Some("2024-12-25"));
    }

    #[test]
    fn focusout_closes_open_popover() {
        let mut svc = make_service();
        drop(svc.send(Event::Open));
        drop(svc.send(Event::FocusOut));
        assert_eq!(*svc.state(), State::Closed);
    }
}
```
