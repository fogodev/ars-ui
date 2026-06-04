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

Composes **two segmented DateField inputs** (start and end) + a **[RangeCalendar](range-calendar.md)** shown in a popover, with optional preset shortcuts. Follows DatePicker's composition pattern: the picker owns the canonical range and popover state and exposes child-props builders (`start_field_props()`, `end_field_props()`, `range_calendar_props()`) that the adapter feeds to the child machines, bridging their events back.

> **Composition note.** A `DateField` is a _segmented_ input that emits an `Option<CalendarDate>` (it has no format string); range selection lives in the dedicated `RangeCalendar` component (`Calendar` has no range mode). The two-field value coordination mirrors [DateRangeField](date-range-field.md) (value-based events + `DateRange::normalized`), and the popover lifecycle mirrors [DatePicker](date-picker.md). Live focus, popover positioning, and return-focus are adapter concerns driven from element handles; the agnostic core emits no effects (`type Effect = NoEffect`).

## 1. State Machine

### 1.1 States

```rust
/// States for the DateRangePicker component.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// Open the calendar popover.
    Open,
    /// Close the calendar popover.
    Close,
    /// Toggle the popover open/closed.
    Toggle,
    /// The embedded calendar completed a range selection.
    SelectRangeComplete {
        /// The selected range.
        range: DateRange,
    },
    /// The start field's value changed. A segmented `DateField` emits an
    /// `Option<CalendarDate>`, not raw text.
    StartValueChange(Option<CalendarDate>),
    /// The end field's value changed.
    EndValueChange(Option<CalendarDate>),
    /// A preset range was chosen by its index into `Props::presets`.
    SelectPreset {
        /// Zero-based index into the configured presets.
        index: usize,
    },
    /// Clear the selected range (the clear trigger).
    Clear,
    /// Focus entered the component.
    FocusIn,
    /// Focus left the component entirely.
    FocusOut,
    /// A key was pressed on the trigger or within the popover.
    KeyDown {
        /// The key that was pressed.
        key: KeyboardKey,
    },
    /// Re-apply context-backed prop fields after a props change. Emitted by
    /// `Machine::on_props_changed` so a controlled `value` and the cached
    /// `min`/`max`/`today`/`presets`/flags follow parent-driven prop updates.
    SyncProps(Box<Props>),
}
```

### 1.3 Context

```rust
/// Context for the DateRangePicker component.
///
/// `intl_backend` is not `PartialEq`/`Debug`-derivable, so `Context` provides a
/// hand-written `Debug` and is not `PartialEq` (matching `DateRangeField`).
#[derive(Clone)]
pub struct Context {
    /// The canonical selected range. `Some` only when both `start_date` and
    /// `end_date` are set; always normalized so `start <= end`.
    pub value: Bindable<Option<DateRange>>,
    /// Whether the popover is open (always uncontrolled).
    pub open: Bindable<bool>,
    /// The start field's current value, tracked independently so a range can be
    /// assembled incrementally as each field changes.
    pub start_date: Option<CalendarDate>,
    /// The end field's current value, tracked independently.
    pub end_date: Option<CalendarDate>,
    /// Which field most recently received a value change.
    pub active_field: ActiveField,
    /// The minimum selectable date.
    pub min: Option<CalendarDate>,
    /// The maximum selectable date.
    pub max: Option<CalendarDate>,
    /// The adapter-injected "today" date, forwarded to the embedded calendar.
    pub today: CalendarDate,
    /// Named preset ranges offered as one-click shortcuts.
    pub presets: Vec<Preset>,
    /// Number of months displayed side-by-side, forwarded to the calendar.
    pub visible_months: usize,
    /// Right-to-left layout direction, forwarded to the calendar.
    pub is_rtl: bool,
    /// The resolved locale.
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
    /// When `true`, numeric segments in both child fields display with leading
    /// zeros.
    pub force_leading_zeros: bool,
    /// Whether the `Description` part is rendered.
    pub has_description: bool,
    /// Whether the `ErrorMessage` part is rendered.
    pub has_error_message: bool,
    /// Form field name for a single hidden input carrying the range value.
    pub name: Option<String>,
    /// Separate form name for the start date.
    pub start_name: Option<String>,
    /// Separate form name for the end date.
    pub end_name: Option<String>,
    /// Component IDs.
    pub ids: ComponentIds,
}
```

`ActiveField` (`Start` / `End`) is owned by [DateRangeField](date-range-field.md) as the first range component to need it; `DateRangePicker` re-exports and reuses that enum rather than redefining it.

### 1.4 Props

```rust
/// Props for the DateRangePicker component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The stable DOM id for the component.
    pub id: String,
    /// Controlled range value. `None` = uncontrolled, `Some(None)` =
    /// controlled-and-empty, `Some(Some(range))` = controlled with a range.
    pub value: Option<Option<DateRange>>,
    /// The initial range used when the component is uncontrolled.
    pub default_value: Option<DateRange>,
    /// The minimum selectable date (forwarded to both fields and the calendar).
    pub min: Option<CalendarDate>,
    /// The maximum selectable date (forwarded to both fields and the calendar).
    pub max: Option<CalendarDate>,
    /// The "today" date, injected by the adapter for testability and SSR
    /// determinism. Forwarded to the embedded calendar. Defaults to a fixed
    /// date; adapters inject the real today.
    pub today: CalendarDate,
    /// Named preset ranges offered as one-click shortcuts (rendered as
    /// `PresetTrigger` items inside the popover).
    pub presets: Vec<Preset>,
    /// Number of months displayed side-by-side in the calendar popover.
    /// Default: `2`. Forwarded to the embedded calendar's `visible_months`.
    pub visible_months: usize,
    /// Right-to-left layout direction (forwarded to the embedded calendar).
    pub is_rtl: bool,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is readonly (viewing allowed, editing blocked).
    pub readonly: bool,
    /// Whether the component is required.
    pub required: bool,
    /// When `true`, numeric segments in both child fields display with leading
    /// zeros. Defaults to `false`, which uses locale-aware formatting.
    pub force_leading_zeros: bool,
    /// Whether a `Description` element is rendered (wires the control's
    /// `aria-describedby`).
    pub has_description: bool,
    /// Whether an `ErrorMessage` element is rendered (wires the control's
    /// `aria-describedby`).
    pub has_error_message: bool,
    /// Form field name for a single hidden input carrying the range value as the
    /// ISO 8601 interval `YYYY-MM-DD/YYYY-MM-DD`.
    pub name: Option<String>,
    /// Form field name for a separate hidden input carrying the start date
    /// (alternative to `name`). Submitted in ISO 8601 format.
    pub start_name: Option<String>,
    /// Form field name for a separate hidden input carrying the end date.
    pub end_name: Option<String>,
    /// Whether to close the popover after a range is completed. Default: `true`.
    pub close_on_select: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            min: None,
            max: None,
            today: CalendarDate::new_gregorian(2025, 1, 1)
                .expect("2025-01-01 is a valid Gregorian date"),
            presets: Vec::new(),
            visible_months: 2,
            is_rtl: false,
            disabled: false,
            readonly: false,
            required: false,
            force_leading_zeros: false,
            has_description: false,
            has_error_message: false,
            name: None,
            start_name: None,
            end_name: None,
            close_on_select: true,
        }
    }
}
```

### 1.4.1 Preset

```rust
/// A named preset range offered as a one-click shortcut (e.g. "Last 7 days").
///
/// The consumer supplies an already-localized `label` and a concrete `range`.
/// Relative presets are computed by the consumer against the same `today`
/// injected into `Props::today`, keeping the agnostic core deterministic and
/// free of closures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preset {
    /// The pre-localized label shown on the preset trigger.
    pub label: String,
    /// The concrete range applied when this preset is selected.
    pub range: DateRange,
}

impl Preset {
    /// Creates a preset from a label and a concrete range.
    pub fn new(label: impl Into<String>, range: DateRange) -> Self {
        Self { label: label.into(), range }
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

The adapter bridges child-component events back to DateRangePicker. The children
are configured via the picker's `*_props()` builders; the picker runs no child
machines itself.

```text
RangeCalendar value completed -> DateRangePicker.SelectRangeComplete { range }
StartDateField value change   -> DateRangePicker.StartValueChange(Option<CalendarDate>)
EndDateField value change     -> DateRangePicker.EndValueChange(Option<CalendarDate>)
PresetTrigger click           -> DateRangePicker.SelectPreset { index }
```

> **Multi-month recommendation**: DateRangePicker uses `visible_months: 2` by default so users see start and end month grids side-by-side.

#### 1.7.1 State Ownership and Synchronization

The DateRangePicker context is the single source of truth for the selected
range. It tracks the two field values independently (`start_date`/`end_date`)
and derives the canonical normalized `value` from them.

```rust
struct Context {
    /// Canonical, normalized range derived from the two field values.
    value: Bindable<Option<DateRange>>,
    /// Per-field working values.
    start_date: Option<CalendarDate>,
    end_date: Option<CalendarDate>,
    // ...
}
```

**Sync Rules:**

1. When the user completes a range via the calendar (`SelectRangeComplete`), both field values and the canonical `value` are updated.
2. When the user edits a date via a field (`StartValueChange`/`EndValueChange`), the changed field is stored and the canonical range is recomputed via `DateRange::normalized`; the adapter feeds the updated `value` back to the calendar through `range_calendar_props()`.
3. In controlled mode (`value` prop set), `get()` returns the parent's value; field edits update the per-field working values and the internal (pending) value but the override stands until the parent reconciles via the next `SyncProps`. The internal value is kept in lockstep with the override so a later controlled→uncontrolled switch reveals a value consistent with the fields.
4. In uncontrolled mode, the internal `value` is the source of truth; both the fields and the calendar read from and write to it.

**Normalization & invalid ranges:**

- Field edits are normalized by `DateRange::normalized` so the stored range always satisfies `start <= end` (an out-of-order edit swaps the endpoints rather than being rejected).
- A range whose endpoints fall outside `[min, max]` is _valid in order but out of bounds_; `Root` then carries `data-ars-invalid` and each child field reports its own `invalid`. Each child field is bounded only by the global `min`/`max` (never by the opposite endpoint), so an out-of-order edit reaches the parent for the normalizing swap instead of being clamped away by the child.

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
    type Effect = NoEffect;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let value = if let Some(value) = &props.value {
            Bindable::controlled(value.clone())
        } else {
            Bindable::uncontrolled(props.default_value.clone())
        };

        let initial_range = value.get().clone();
        let start_date = initial_range.as_ref().map(|range| range.start.clone());
        let end_date = initial_range.as_ref().map(|range| range.end.clone());

        let ctx = Context {
            value,
            open: Bindable::uncontrolled(false),
            start_date,
            end_date,
            active_field: ActiveField::Start,
            min: props.min.clone(),
            max: props.max.clone(),
            today: props.today.clone(),
            presets: props.presets.clone(),
            visible_months: props.visible_months,
            is_rtl: props.is_rtl,
            locale: env.locale.clone(),
            intl_backend: Arc::clone(&env.intl_backend),
            messages: messages.clone(),
            disabled: props.disabled,
            readonly: props.readonly,
            required: props.required,
            force_leading_zeros: props.force_leading_zeros,
            has_description: props.has_description,
            has_error_message: props.has_error_message,
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
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // `SyncProps` must process even when disabled so adapter-driven prop
        // updates (including re-enabling) still reach the context.
        if ctx.disabled && !matches!(event, Event::SyncProps(_)) {
            return None;
        }

        match (state, event) {
            (_, Event::SyncProps(next)) => {
                let next = next.as_ref().clone();
                // Disabling an open picker dismisses it: while disabled the guard
                // above blocks Close/Escape/FocusOut, so it could never reopen.
                if next.disabled && *state == State::Open {
                    return Some(TransitionPlan::to(State::Closed).apply(move |ctx| {
                        sync_props(ctx, &next);
                        ctx.open.set(false);
                    }));
                }
                Some(TransitionPlan::context_only(move |ctx| sync_props(ctx, &next)))
            }

            (State::Closed, Event::Open | Event::Toggle) => {
                Some(TransitionPlan::to(State::Open).apply(|ctx| ctx.open.set(true)))
            }

            (State::Open, Event::Close | Event::Toggle) => {
                Some(TransitionPlan::to(State::Closed).apply(|ctx| ctx.open.set(false)))
            }

            // Accept the completed range in any state: a browser may fire
            // `FocusOut` (closing the popover) before the calendar reports the
            // cell click. The close side-effect only applies when it was open.
            (_, Event::SelectRangeComplete { range }) => {
                if ctx.readonly { return None; }
                let range = range.clone();
                let should_close = props.close_on_select && *state == State::Open;
                let next_state = if should_close { State::Closed } else { state.clone() };
                Some(TransitionPlan::to(next_state)
                    .apply(move |ctx| apply_complete_range(ctx, range, should_close)))
            }

            (_, Event::SelectPreset { index }) => {
                if ctx.readonly { return None; }
                let range = ctx.presets.get(*index)?.range.clone();
                let should_close = props.close_on_select && *state == State::Open;
                let next_state = if should_close { State::Closed } else { state.clone() };
                Some(TransitionPlan::to(next_state)
                    .apply(move |ctx| apply_complete_range(ctx, range, should_close)))
            }

            (_, Event::StartValueChange(date)) => {
                if ctx.readonly { return None; }
                let date = date.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.active_field = ActiveField::Start;
                    ctx.start_date = date;
                    recompute_range(ctx);
                }))
            }

            (_, Event::EndValueChange(date)) => {
                if ctx.readonly { return None; }
                let date = date.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.active_field = ActiveField::End;
                    ctx.end_date = date;
                    recompute_range(ctx);
                }))
            }

            (_, Event::Clear) => {
                if ctx.readonly { return None; }
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.start_date = None;
                    ctx.end_date = None;
                    ctx.value.set(None);
                }))
            }

            (State::Open, Event::KeyDown { key }) if *key == KeyboardKey::Escape => {
                Self::transition(state, &Event::Close, ctx, props)
            }

            (State::Closed, Event::KeyDown { key }) if *key == KeyboardKey::ArrowDown => {
                Self::transition(state, &Event::Open, ctx, props)
            }

            (State::Open, Event::FocusOut) => Self::transition(state, &Event::Close, ctx, props),

            _ => None,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        if old == new { Vec::new() } else { vec![Event::SyncProps(Box::new(new.clone()))] }
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

/// Re-derives mutable context fields from changed props. `open` is always
/// uncontrolled and is left untouched.
fn sync_props(ctx: &mut Context, props: &Props) {
    ctx.min = props.min.clone();
    ctx.max = props.max.clone();
    ctx.today = props.today.clone();
    ctx.presets = props.presets.clone();
    ctx.visible_months = props.visible_months;
    ctx.is_rtl = props.is_rtl;
    ctx.disabled = props.disabled;
    ctx.readonly = props.readonly;
    ctx.required = props.required;
    ctx.force_leading_zeros = props.force_leading_zeros;
    ctx.has_description = props.has_description;
    ctx.has_error_message = props.has_error_message;
    ctx.name = props.name.clone();
    ctx.start_name = props.start_name.clone();
    ctx.end_name = props.end_name.clone();

    if let Some(controlled) = &props.value {
        ctx.value.sync_controlled(Some(controlled.clone()));
        // Keep the internal value in lockstep so a later controlled→uncontrolled
        // switch reveals a value consistent with the per-field values.
        ctx.value.set(controlled.clone());
        ctx.start_date = controlled.as_ref().map(|range| range.start.clone());
        ctx.end_date = controlled.as_ref().map(|range| range.end.clone());
    } else {
        // Genuinely uncontrolled: drop any stale override but keep the internal
        // working state and per-field values so in-progress edits survive.
        ctx.value.sync_controlled(None);
    }
}

/// Applies a completed range to the context (fields, canonical value, and — when
/// `close` — the open state).
fn apply_complete_range(ctx: &mut Context, range: DateRange, close: bool) {
    ctx.start_date = Some(range.start.clone());
    ctx.end_date = Some(range.end.clone());
    ctx.value.set(Some(range));
    if close { ctx.open.set(false); }
}

/// Recomputes the canonical range from the two field values, normalizing so
/// `start <= end`. Clears the range when either field is empty or the dates are
/// not comparable.
fn recompute_range(ctx: &mut Context) {
    match (ctx.start_date.clone(), ctx.end_date.clone()) {
        (Some(start), Some(end)) => match DateRange::normalized(start, end) {
            Some(range) => {
                ctx.start_date = Some(range.start.clone());
                ctx.end_date = Some(range.end.clone());
                ctx.value.set(Some(range));
            }
            None => ctx.value.set(None),
        },
        _ => ctx.value.set(None),
    }
}
```

### 1.9 Connect / API

`ActiveField` is re-exported from [DateRangeField](date-range-field.md).

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
    /// A preset shortcut button; the index selects into `Props::presets`.
    PresetTrigger { index: usize },
    Positioner,
    Content,
    Description,
    ErrorMessage,
    HiddenInput,
}

/// API for the DateRangePicker component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Root element. Carries scope/part/id/state plus `data-ars-disabled`,
    /// `data-ars-readonly`, `data-ars-required`, and `data-ars-invalid` flags.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(HtmlAttr::Data("ars-state"), self.state_name());
        if self.ctx.disabled { attrs.set_bool(HtmlAttr::Data("ars-disabled"), true); }
        if self.ctx.readonly { attrs.set_bool(HtmlAttr::Data("ars-readonly"), true); }
        if self.ctx.required { attrs.set_bool(HtmlAttr::Data("ars-required"), true); }
        if self.ctx.is_invalid() { attrs.set_bool(HtmlAttr::Data("ars-invalid"), true); }
        attrs
    }

    /// Label element: `id={id}-label`, `for={id}-start-input`.
    pub fn label_attrs(&self) -> AttrMap { /* scope/part + id + For(start-input) */ }

    /// Control group: `role="group"`, `aria-labelledby={id}-label`, and
    /// `aria-describedby` chaining the description/error ids when present.
    pub fn control_attrs(&self) -> AttrMap { /* ... */ }

    /// Marker attributes for the start/end input wrappers (scope/part only); the
    /// child `DateField` is configured through `start_field_props`/`end_field_props`.
    pub fn start_input_attrs(&self) -> AttrMap { /* scope/part */ }
    pub fn end_input_attrs(&self) -> AttrMap { /* scope/part */ }

    /// Separator element: `aria-hidden="true"`.
    pub fn separator_attrs(&self) -> AttrMap { /* ... */ }

    /// Builds the child `DateField` props for the start input. Bounded only by
    /// the global `min`/`max` (never by the opposite endpoint); `invalid`
    /// reflects whether this field's own value is out of bounds.
    pub fn start_field_props(&self) -> date_field::Props {
        date_field::Props {
            id: self.ctx.ids.part("start-input"),
            value: Some(self.ctx.start_date.clone()),
            min_value: self.ctx.min.clone(),
            max_value: self.ctx.max.clone(),
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            required: self.ctx.required,
            invalid: date_out_of_bounds(self.ctx.start_date.as_ref(), self.ctx.min.as_ref(), self.ctx.max.as_ref()),
            aria_label: Some((self.ctx.messages.start_label)(&self.ctx.locale)),
            force_leading_zeros: self.ctx.force_leading_zeros,
            ..date_field::Props::default()
        }
    }

    /// Builds the child `DateField` props for the end input (mirror of start).
    pub fn end_field_props(&self) -> date_field::Props { /* id = {id}-end-input, value = end_date, ... */ }

    /// Trigger: `aria-haspopup="dialog"`, `aria-expanded`, `aria-controls={id}-content`,
    /// `aria-label` from `messages.trigger_label`; `disabled` when disabled.
    pub fn trigger_attrs(&self) -> AttrMap { /* ... */ }

    /// Clear trigger: `aria-label` from `messages.clear_label`; `disabled` when
    /// the component is disabled or read-only, or no range is selected (matching
    /// the machine, which rejects `Event::Clear` in those states).
    pub fn clear_trigger_attrs(&self) -> AttrMap { /* ... */ }

    /// Preset trigger at `index`: `data-ars-index={index}`, `type="button"`;
    /// `disabled` when the component is disabled or read-only, or the index is
    /// out of range (matching the machine, which rejects `Event::SelectPreset`).
    pub fn preset_trigger_attrs(&self, index: usize) -> AttrMap { /* ... */ }

    /// Positioner: scope/part only (positioning is performed by the adapter).
    pub fn positioner_attrs(&self) -> AttrMap { /* ... */ }

    /// Content: `role="dialog"`, `aria-modal="false"`, `aria-labelledby={id}-label`,
    /// `id={id}-content`.
    pub fn content_attrs(&self) -> AttrMap { /* ... */ }

    pub fn description_attrs(&self) -> AttrMap { /* id={id}-description */ }
    pub fn error_message_attrs(&self) -> AttrMap { /* id={id}-error-message, role="alert" */ }

    /// Combined hidden input: `value = range.to_iso8601()` (the `start/end`
    /// interval), or empty. `name` from props; `disabled` mirrors the component.
    pub fn hidden_input_attrs(&self) -> AttrMap { /* ... */ }

    /// Separate hidden inputs carrying each endpoint as `CalendarDate::to_iso8601`.
    /// Used when `start_name`/`end_name` are set.
    pub fn start_hidden_input_attrs(&self) -> AttrMap { /* name from start_name, value = start_date.to_iso8601() */ }
    pub fn end_hidden_input_attrs(&self) -> AttrMap { /* name from end_name, value = end_date.to_iso8601() */ }

    /// Builds the embedded `RangeCalendar` props. The calendar reflects the
    /// picker's canonical range as a controlled value and inherits the picker's
    /// bounds, today, visible-month count, and layout direction.
    pub fn range_calendar_props(&self) -> range_calendar::Props {
        range_calendar::Props {
            id: self.ctx.ids.part("calendar"),
            value: Some(self.ctx.value.get().clone()),
            min: self.ctx.min.clone(),
            max: self.ctx.max.clone(),
            today: self.ctx.today.clone(),
            visible_months: self.ctx.visible_months,
            is_rtl: self.ctx.is_rtl,
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            ..range_calendar::Props::default()
        }
    }

    /// Screen-reader description of the full range (e.g. "March 1, 2025 to
    /// March 15, 2025"), or `None` if no complete range is selected.
    pub fn range_description(&self) -> Option<String> { /* via shared format_date_label + messages.range_description */ }

    // Imperative dispatch + getters
    pub fn open(&self)   { (self.send)(Event::Open); }
    pub fn close(&self)  { (self.send)(Event::Close); }
    pub fn toggle(&self) { (self.send)(Event::Toggle); }
    pub fn clear(&self)  { (self.send)(Event::Clear); }
    pub fn select_range(&self, range: DateRange) { (self.send)(Event::SelectRangeComplete { range }); }
    pub fn select_preset(&self, index: usize)    { (self.send)(Event::SelectPreset { index }); }
    pub fn set_start_value(&self, date: Option<CalendarDate>) { (self.send)(Event::StartValueChange(date)); }
    pub fn set_end_value(&self, date: Option<CalendarDate>)   { (self.send)(Event::EndValueChange(date)); }
    pub fn focus_in(&self)  { (self.send)(Event::FocusIn); }
    pub fn focus_out(&self) { (self.send)(Event::FocusOut); }
    pub fn on_key_down(&self, key: KeyboardKey) { (self.send)(Event::KeyDown { key }); }

    pub fn is_open(&self) -> bool { matches!(self.state, State::Open) }
    pub fn selected_range(&self) -> Option<&DateRange> { self.ctx.value.get().as_ref() }
    pub fn active_field(&self) -> ActiveField { self.ctx.active_field }
    pub fn presets(&self) -> &[Preset] { &self.ctx.presets }
    pub fn preset_label(&self, index: usize) -> Option<&str> {
        self.ctx.presets.get(index).map(|preset| preset.label.as_str())
    }
    pub fn is_invalid(&self) -> bool { self.ctx.is_invalid() }

    fn state_name(&self) -> &'static str {
        match self.state { State::Closed => "closed", State::Open => "open" }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Control => self.control_attrs(),
            Part::StartInput => self.start_input_attrs(),
            Part::Separator => self.separator_attrs(),
            Part::EndInput => self.end_input_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::ClearTrigger => self.clear_trigger_attrs(),
            Part::PresetTrigger { index } => self.preset_trigger_attrs(index),
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
|   |   +-- StartInput                    (DateField wrapper)
|   |   +-- Separator                     aria-hidden="true"  "--"
|   |   +-- EndInput                      (DateField wrapper)
|   |   +-- Trigger                       aria-haspopup="dialog" aria-expanded
|   |   +-- ClearTrigger                  aria-label="Clear date range"
|   +-- Positioner
|   |   +-- Content                       role="dialog" aria-labelledby="{id}-label"
|   |       +-- PresetTrigger*            data-ars-index   (one per Props::presets entry)
|   |       +-- (RangeCalendar)           visible_months=2
|   +-- Description                       help text
|   +-- ErrorMessage                      role="alert"
|   +-- HiddenInput                       type="hidden" value="2024-01-10/2024-01-20"
```

| Part            | Element               | Key Attributes                                                                          |
| --------------- | --------------------- | --------------------------------------------------------------------------------------- |
| `Root`          | `<div>`               | `data-ars-scope="date-range-picker"`, `data-ars-part="root"`, state, `data-ars-invalid` |
| `Label`         | `<label>`             | `for="{id}-start-input"`                                                                |
| `Control`       | `<div>`               | `role="group"`, `aria-labelledby`, `aria-describedby`                                   |
| `StartInput`    | `<div>` _(DateField)_ | Start date field wrapper (child configured via `start_field_props`)                     |
| `Separator`     | `<span>`              | `aria-hidden="true"`                                                                    |
| `EndInput`      | `<div>` _(DateField)_ | End date field wrapper (child configured via `end_field_props`)                         |
| `Trigger`       | `<button>`            | `aria-haspopup="dialog"`, `aria-expanded`, `aria-controls`                              |
| `ClearTrigger`  | `<button>`            | `aria-label="Clear date range"`, disabled when empty, disabled, or read-only            |
| `PresetTrigger` | `<button>`            | `data-ars-index`, one per `Props::presets` entry; disabled when out of range/read-only  |
| `Positioner`    | `<div>`               | Floating positioner (positioned by the adapter)                                         |
| `Content`       | `<div>`               | `role="dialog"`, `aria-modal="false"`, `aria-labelledby`                                |
| `Description`   | `<div>`               | Help text, wired via the control's `aria-describedby`                                   |
| `ErrorMessage`  | `<div>`               | `role="alert"`, validation error                                                        |
| `HiddenInput`   | `<input>`             | `type="hidden"`, ISO 8601 range value `start/end`                                       |

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
#[derive(Clone, Debug, PartialEq)]
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

| Feature               | ars-ui                   | Ark UI                | React Aria                  | Notes                                                   |
| --------------------- | ------------------------ | --------------------- | --------------------------- | ------------------------------------------------------- |
| Controlled range      | `value`                  | `value` (DateValue[]) | `value` (RangeValue)        | Equivalent                                              |
| Default range         | `default_value`          | `defaultValue`        | `defaultValue`              | Equivalent                                              |
| Min/max               | `min`, `max`             | `min`, `max`          | `minValue`, `maxValue`      | Equivalent                                              |
| Disabled              | `disabled`               | `disabled`            | `isDisabled`                | Equivalent                                              |
| Read-only             | `readonly`               | `readOnly`            | `isReadOnly`                | Equivalent                                              |
| Required              | `required`               | `required`            | `isRequired`                | Equivalent                                              |
| Close on select       | `close_on_select`        | `closeOnSelect`       | `shouldCloseOnSelect`       | Equivalent                                              |
| Locale                | (context)                | `locale`              | -- (context)                | Resolved from `Env`, not a prop                         |
| Today (injected)      | `today`                  | --                    | `todayValue` (context)      | Adapter-injected for determinism; forwarded to calendar |
| Presets               | `presets`                | `presetTrigger`       | (userland)                  | ars-ui takes pre-localized concrete-range presets       |
| Name                  | `name`                   | `name`                | `startName`, `endName`      | React Aria has separate form names                      |
| Allows non-contiguous | --                       | --                    | `allowsNonContiguousRanges` | React Aria feature                                      |
| Max visible months    | `visible_months`         | `numOfMonths`         | `maxVisibleMonths`          | ars-ui default `2`, forwarded to RangeCalendar          |
| Start/end form names  | `start_name`, `end_name` | --                    | `startName`, `endName`      | ars-ui matches React Aria's separate form names         |

**Gaps:**

- `allowsNonContiguousRanges`: React Aria allows selecting ranges that span unavailable dates. Currently ars-ui's RangeCalendar blocks selection of unavailable dates. This is a niche feature; not adopting.
- The date _format_ is owned by each child `DateField` (segment order / leading zeros), not a picker-level `format` prop; popover _positioning_ is an adapter concern (no `positioning` prop on the agnostic core).

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

| Feature               | ars-ui                        | Ark UI                | React Aria            |
| --------------------- | ----------------------------- | --------------------- | --------------------- |
| Two-field input       | Yes (StartInput + EndInput)   | Yes (Input index)     | Yes (start/end slots) |
| Calendar popover      | Yes                           | Yes                   | Yes                   |
| Range normalization   | Yes                           | Yes                   | Yes                   |
| Close on select       | Yes                           | Yes                   | Yes                   |
| Clear button          | Yes                           | Yes                   | No                    |
| Multi-month calendar  | Yes (default 2)               | Yes                   | Yes                   |
| Preset shortcuts      | Yes (`presets`)               | Yes (`presetTrigger`) | Userland              |
| Non-contiguous ranges | No                            | No                    | Yes                   |
| Separate form names   | Yes (`start_name`/`end_name`) | No                    | Yes                   |
| Hidden form input     | Yes                           | No                    | No                    |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity.
- **Divergences:** React Aria supports `allowsNonContiguousRanges`, a niche feature not adopted. ars-ui submits ranges via a single hidden input (`name`) and additionally supports separate `start_name`/`end_name` inputs (matching React Aria). Presets are taken as pre-localized concrete-range values (mirroring Ark UI's `presetTrigger`) rather than relative tokens, keeping the agnostic core deterministic.
- **Recommended additions:** None.

## Appendix: Testing

The full unit, ARIA, snapshot, spec-conformance, and proptest suites live in
`crates/ars-components/src/date_time/date_range_picker/` (`tests.rs` +
`snapshots/`),
`crates/ars-components/tests/spec_conformance/date_time/date_range_picker.rs`,
and
`crates/ars-components/tests/proptest_state_machines/date_time/date_range_picker.rs`.
The illustrative cases below drive the machine through a `Service`.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ars_core::Service;
    use ars_i18n::{CalendarDate, DateRange, locales::en_us};

    fn date(y: i32, m: u8, d: u8) -> CalendarDate {
        CalendarDate::new_gregorian(y, m, d).expect("valid date")
    }

    fn service() -> Service<Machine> {
        Service::<Machine>::new(
            Props { id: "range".into(), ..Props::default() },
            &Env::default(),
            &Messages::default(),
        )
    }

    #[test]
    fn initial_state_is_closed() {
        let svc = service();
        assert_eq!(*svc.state(), State::Closed);
        assert!(svc.context().value.get().is_none());
    }

    #[test]
    fn open_and_close() {
        let mut svc = service();
        drop(svc.send(Event::Open));
        assert_eq!(*svc.state(), State::Open);
        drop(svc.send(Event::Close));
        assert_eq!(*svc.state(), State::Closed);
    }

    #[test]
    fn select_range_complete_closes_and_sets_value() {
        let mut svc = service();
        drop(svc.send(Event::Open));
        let selected = DateRange::new(date(2025, 3, 1), date(2025, 3, 15)).unwrap();
        drop(svc.send(Event::SelectRangeComplete { range: selected.clone() }));
        assert_eq!(*svc.state(), State::Closed);
        assert_eq!(*svc.context().value.get(), Some(selected));
    }

    #[test]
    fn field_edits_assemble_and_normalize_range() {
        let mut svc = service();
        // A segmented DateField emits an Option<CalendarDate>, not text.
        drop(svc.send(Event::StartValueChange(Some(date(2025, 6, 25)))));
        drop(svc.send(Event::EndValueChange(Some(date(2025, 6, 1)))));
        assert_eq!(
            *svc.context().value.get(),
            DateRange::new(date(2025, 6, 1), date(2025, 6, 25)).ok(),
        );
    }

    #[test]
    fn select_preset_applies_range() {
        let preset_range = DateRange::new(date(2025, 5, 26), date(2025, 6, 1)).unwrap();
        let mut svc = Service::<Machine>::new(
            Props {
                id: "range".into(),
                presets: vec![Preset { label: "Last 7 days".into(), range: preset_range.clone() }],
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );
        drop(svc.send(Event::SelectPreset { index: 0 }));
        assert_eq!(*svc.context().value.get(), Some(preset_range));
    }

    #[test]
    fn escape_closes_and_arrow_down_opens() {
        let mut svc = service();
        drop(svc.send(Event::KeyDown { key: KeyboardKey::ArrowDown }));
        assert_eq!(*svc.state(), State::Open);
        drop(svc.send(Event::KeyDown { key: KeyboardKey::Escape }));
        assert_eq!(*svc.state(), State::Closed);
    }

    #[test]
    fn disabled_ignores_events() {
        let mut svc = Service::<Machine>::new(
            Props { id: "range".into(), disabled: true, ..Props::default() },
            &Env::default(),
            &Messages::default(),
        );
        assert!(!svc.send(Event::Open).state_changed);
    }
}
```
