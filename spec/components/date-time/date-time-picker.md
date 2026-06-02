---
component: DateTimePicker
category: date-time
tier: complex
foundation_deps: [architecture, accessibility, i18n, interactions, forms]
shared_deps: [date-time-types]
related: [date-picker, time-field, calendar]
references:
    ark-ui: DatePicker
---

# DateTimePicker

`DateTimePicker` is a composite component that combines a `DatePicker` (segmented date input with calendar popover) and a `TimeField` (segmented time input) into a single control for selecting both date and time. The date segments and time segments render in a unified control area, separated by a visual separator. The calendar popover handles date selection; time segments are edited inline via the spinbutton pattern.

> **Spec-vs-implementation reconciliation.** The code blocks below describe the
> agnostic implementation at `crates/ars-components/src/date_time/date_time_picker/`,
> which is the authoritative contract. Key conventions, shared with the sibling
> `date_picker`/`time_field` machines:
>
> - **Combined value is `ars_i18n::CalendarDateTime`** (`CalendarDateTime::new(date, time)`,
>   accessors `.date()`/`.time()`). It has no `Ord`, so range checks compare the
>   date via `CalendarDate::compare` and then the `Time` via `Ord`.
> - **Focus is adapter-driven via named effects.** The machine declares a typed
>   `Effect` enum and emits [`PendingEffect::named`] intents (`FocusCalendar`,
>   `RestoreFocusToTrigger`, `FocusFirstTimeSegment`, `TypeBufferCommit`); the
>   adapter performs the live, element-handle-based focus / timer. The core never
>   calls `use_platform_effects()`/`focus_element_by_id` (matches the element/ref
>   handling note in issue #292).
> - **Controlled-prop sync.** `on_props_changed` emits `Event::SyncProps(Box<Props>)`,
>   which reconciles the controlled `value` and cached scalar fields and flows
>   through even when `disabled` so a parent can lift the disabled state.
> - **Segment helpers are shared.** Hour-cycle math (`display_hour`,
>   `display_hour_to_24`, `display_hour_range`, `has_day_period`, `is_pm`) and the
>   `digits_needed` type-ahead helper live in `date_time::hour_cycle`, shared with
>   `time_field`. Date segment order comes from `ars_i18n::date_order` and the
>   inter-segment literal from `ars_i18n::date_field_separator` (both CLDR-backed,
>   shared with `date_field`/`date_picker`). Segment ARIA labels resolve from the
>   per-segment fields on this component's `Messages`.
> - **Real `CalendarDate`/`Time` APIs.** `CalendarDate::new_gregorian(year: i32,
month: u8, day: u8)` and `Time::new(h, m, s, millis)` are fallible; segment
>   values pass through `.ok()`. Segment kinds expose `data_name()` (not `as_str()`).

## 1. State Machine

### 1.1 States

```rust
/// States for the DateTimePicker component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// The component is idle; no segment has focus and the popover is closed.
    Idle,
    /// A segment (date or time) has focus; the popover is closed.
    Focused,
    /// The calendar popover is open.
    Open,
}
```

### 1.2 Events

```rust
/// Events for the DateTimePicker component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Open the calendar popover.
    Open,
    /// Close the calendar popover.
    Close,
    /// Toggle the calendar popover.
    Toggle,
    /// A segment value changed via spinbutton interaction (ArrowUp/Down or type-ahead).
    SegmentChange {
        /// The segment that changed.
        segment: DateSegmentKind,
        /// The new value.
        value: i32,
    },
    /// Commit (or clear) the full combined date-time value (Enter, blur, programmatic).
    ValueCommit(Option<CalendarDateTime>),
    /// Focus moved to a specific segment.
    FocusSegment(DateSegmentKind),
    /// Focus moved to the next segment.
    FocusNextSegment,
    /// Focus moved to the previous segment.
    FocusPrevSegment,
    /// Increment the focused segment value by 1, wrapping.
    IncrementSegment { segment: DateSegmentKind },
    /// Decrement the focused segment value by 1, wrapping.
    DecrementSegment { segment: DateSegmentKind },
    /// A printable character was typed into a numeric segment.
    TypeIntoSegment { segment: DateSegmentKind, ch: char },
    /// The type-ahead buffer timer fired; commit buffered digits.
    TypeBufferCommit { segment: DateSegmentKind },
    /// A segment value changed programmatically.
    SegmentChange { segment: DateSegmentKind, value: i32 },
    /// Clear the value of a segment.
    ClearSegment { segment: DateSegmentKind },
    /// Clear the entire date-time value.
    ClearAll,
    /// A date was selected from the calendar popover.
    CalendarSelectDate(CalendarDate),
    /// Focus entered the component (any segment or trigger).
    FocusIn,
    /// Focus left the component entirely.
    FocusOut,
    /// Keyboard event on a segment, trigger, or popover.
    KeyDown { key: KeyboardKey },
    /// Synchronize context from a new props snapshot.
    SyncProps(Box<Props>),
}
```

> Calendar month navigation (next/previous) is delegated entirely to the embedded
> `Calendar` machine; the adapter wires those `Calendar` events directly, so the
> `DateTimePicker` machine has no `CalendarNext`/`CalendarPrev` events.

### 1.3 Context

```rust
/// Context for the DateTimePicker component.
///
/// `Debug` and `PartialEq` are hand-written to skip the `intl_backend` trait
/// object (every other field participates), mirroring `time_field::Context`.
#[derive(Clone)]
pub struct Context {
    /// The combined date-time value (controlled/uncontrolled).
    pub value: Bindable<Option<CalendarDateTime>>,
    /// The date portion of the value (synced from value or calendar selection).
    pub date_value: Option<CalendarDate>,
    /// The time portion of the value (synced from value or time segment edits).
    pub time_value: Option<Time>,
    /// The currently focused segment (date or time).
    pub focused_segment: Option<DateSegmentKind>,
    /// Whether the calendar popover is open.
    pub open: bool,
    /// Date segments (Year, Month, Day) in locale order, with literals.
    pub date_segments: Vec<DateSegment>,
    /// Time segments (Hour, Minute, optional Second, optional DayPeriod), with literals.
    pub time_segments: Vec<DateSegment>,
    /// The type-ahead buffer for numeric segment editing.
    pub type_buffer: String,
    /// The locale for formatting.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// Backend used for locale-dependent hour-cycle resolution.
    pub intl_backend: Arc<dyn IntlBackend>,
    /// The time granularity (Hour, Minute, Second).
    pub granularity: TimeGranularity,
    /// The resolved hour cycle.
    pub hour_cycle: HourCycle,
    /// The calendar system used by the embedded calendar.
    pub calendar: CalendarSystem,
    /// Minimum allowed date-time.
    pub min_value: Option<CalendarDateTime>,
    /// Maximum allowed date-time.
    pub max_value: Option<CalendarDateTime>,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is read-only.
    pub readonly: bool,
    /// Whether the field has been interacted with.
    pub is_touched: bool,
    /// Whether the field is required.
    pub required: bool,
    /// Whether the field is invalid.
    pub invalid: bool,
    /// Form field name.
    pub name: Option<String>,
    /// Right-to-left layout.
    pub is_rtl: bool,
    /// The adapter-injected current date forwarded to the calendar.
    pub today: CalendarDate,
    /// Number of months shown in the calendar popover.
    pub visible_months: usize,
    /// Component IDs.
    pub ids: ComponentIds,
}

impl Context {
    /// All segments in display order: date segments, then time segments.
    pub fn all_segments(&self) -> impl Iterator<Item = &DateSegment> {
        self.date_segments.iter().chain(self.time_segments.iter())
    }

    /// Returns the segment with the given kind, if present in either group.
    pub fn segment(&self, kind: DateSegmentKind) -> Option<&DateSegment> {
        self.all_segments().find(|s| s.kind == kind)
    }

    /// Returns the current numeric value for a segment.
    pub fn segment_value(&self, kind: DateSegmentKind) -> Option<i32> {
        self.segment(kind)?.value
    }

    /// Find the first editable segment across date and time.
    pub fn first_editable(&self) -> Option<DateSegmentKind> {
        self.all_segments().find(|s| s.is_editable).map(|s| s.kind)
    }

    /// Find the next editable segment after the given segment, crossing the
    /// date-to-time boundary when the current segment is the last date segment.
    pub fn next_editable_after(&self, kind: DateSegmentKind) -> Option<DateSegmentKind> {
        let all = self.all_segments().collect::<Vec<_>>();
        let idx = all.iter().position(|s| s.kind == kind)?;
        all[idx + 1..].iter().find(|s| s.is_editable).map(|s| s.kind)
    }

    /// Find the previous editable segment before the given segment, crossing
    /// the time-to-date boundary when the current segment is the first time segment.
    pub fn prev_editable_before(&self, kind: DateSegmentKind) -> Option<DateSegmentKind> {
        let all = self.all_segments().collect::<Vec<_>>();
        let idx = all.iter().position(|s| s.kind == kind)?;
        all[..idx].iter().rev().find(|s| s.is_editable).map(|s| s.kind)
    }

    /// Whether a segment belongs to the date group.
    pub fn is_date_segment(&self, kind: DateSegmentKind) -> bool {
        self.date_segments.iter().any(|s| s.kind == kind)
    }

    /// Whether a segment belongs to the time group.
    pub fn is_time_segment(&self, kind: DateSegmentKind) -> bool {
        self.time_segments.iter().any(|s| s.kind == kind)
    }

    /// Set a segment's value (clamped to its range), formatting the display text
    /// via `format_segment_text` (year is 4-wide, day-period is AM/PM, the rest
    /// are zero-padded to two digits).
    pub fn set_segment_value(&mut self, kind: DateSegmentKind, raw: i32) {
        let seg = self.date_segments.iter_mut()
            .chain(self.time_segments.iter_mut())
            .find(|s| s.kind == kind && s.is_editable);
        if let Some(seg) = seg {
            let v = raw.clamp(seg.min, seg.max);
            seg.value = Some(v);
            seg.text = format_segment_text(kind, v);
        }
    }

    /// Clear a segment's value.
    pub fn clear_segment_value(&mut self, kind: DateSegmentKind) { /* clears value + text */ }

    /// Increment / decrement a segment value, wrapping within its range
    /// (`rem_euclid`-based, identical to `time_field`).
    pub fn increment_segment(&mut self, kind: DateSegmentKind) { /* step_segment_value(+1) */ }
    pub fn decrement_segment(&mut self, kind: DateSegmentKind) { /* step_segment_value(-1) */ }

    /// Check if all editable segments across both date and time have values.
    pub fn is_complete(&self) -> bool {
        self.all_segments().filter(|s| s.is_editable).all(|s| s.value.is_some())
    }

    /// Assemble a `CalendarDate` from current date segment values, using the
    /// configured `calendar` system and carrying the current era (matching
    /// `date_field`) so non-Gregorian values commit dates in that calendar. The
    /// era is `current_era()` — the value's era, else the calendar's default —
    /// because `DateTimePicker` has no Era segment (so e.g. Japanese Reiwa 6
    /// stays Reiwa rather than ISO year 0006).
    pub fn assemble_date(&self) -> Option<CalendarDate> {
        let year = self.segment_value(DateSegmentKind::Year)?;
        let month = u8::try_from(self.segment_value(DateSegmentKind::Month)?).ok()?;
        let day = u8::try_from(self.segment_value(DateSegmentKind::Day)?).ok()?;
        CalendarDate::new(self.calendar, &CalendarDateFields {
            era: self.current_era(), year: Some(year), month: Some(month), day: Some(day),
            ..CalendarDateFields::default()
        }).ok()
    }

    /// Assemble a `Time` from current time segment values, honouring granularity
    /// and converting the displayed hour back to 24-hour via `display_hour_to_24`.
    pub fn assemble_time(&self) -> Option<Time> {
        let raw_hour = u8::try_from(self.segment_value(DateSegmentKind::Hour)?).ok()?;
        let minute = if self.granularity >= TimeGranularity::Minute {
            u8::try_from(self.segment_value(DateSegmentKind::Minute)?).ok()?
        } else { 0 };
        let second = if self.granularity >= TimeGranularity::Second {
            u8::try_from(self.segment_value(DateSegmentKind::Second)?).ok()?
        } else { 0 };
        let day_period = self.segment_value(DateSegmentKind::DayPeriod);
        let hour = display_hour_to_24(raw_hour, day_period, self.hour_cycle)?;
        Time::new(hour, minute, second, 0).ok()
    }

    /// Assemble a `CalendarDateTime` from the current date and time segment values.
    pub fn assemble_datetime(&self) -> Option<CalendarDateTime> {
        Some(CalendarDateTime::new(self.assemble_date()?, self.assemble_time()?))
    }
}
```

### 1.4 Props

```rust
/// Props for the DateTimePicker component.
///
/// Mirrored by chainable builder methods (`Props::new().id(..).granularity(..)`),
/// matching the `time_field`/`date_picker` builder convention.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The ID of the date-time picker.
    pub id: String,
    /// Controlled date-time value. `Some(v)` = controlled, `None` = uncontrolled.
    pub value: Option<Option<CalendarDateTime>>,
    /// Default date-time for uncontrolled mode.
    pub default_value: Option<CalendarDateTime>,
    /// Minimum allowed date-time.
    pub min_value: Option<CalendarDateTime>,
    /// Maximum allowed date-time.
    pub max_value: Option<CalendarDateTime>,
    /// Finest time segment to display.
    pub granularity: TimeGranularity,
    /// Whether the component is non-interactive.
    pub disabled: bool,
    /// Whether the component allows viewing but not editing.
    pub readonly: bool,
    /// Form field name for hidden input submission.
    pub name: Option<String>,
    /// Calendar system (Gregorian, Buddhist, etc.).
    pub calendar: CalendarSystem,
    /// Hour cycle override. `None` uses the locale default.
    pub hour_cycle: Option<HourCycle>,
    /// Whether the field is required.
    pub required: bool,
    /// Label text.
    pub label: String,
    /// Description/help text.
    pub description: Option<String>,
    /// Error message.
    pub error_message: Option<String>,
    /// Whether the field is in an invalid state.
    pub invalid: bool,
    /// Right-to-left layout direction.
    pub is_rtl: bool,
    /// Number of months to display in the calendar popover. Default: `1`.
    pub visible_months: usize,
    /// The "today" date, injected by the adapter for testability and SSR. It is
    /// forwarded to the embedded calendar so the popover opens on the current
    /// month and marks the correct day.
    pub today: CalendarDate,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            min_value: None,
            max_value: None,
            granularity: TimeGranularity::Minute,
            disabled: false,
            readonly: false,
            name: None,
            calendar: CalendarSystem::Gregorian,
            hour_cycle: None,
            required: false,
            label: String::new(),
            description: None,
            error_message: None,
            invalid: false,
            is_rtl: false,
            visible_months: 1,
            today: CalendarDate::new_gregorian(2025, 1, 1)
                .expect("2025-01-01 is a valid Gregorian date"),
        }
    }
}
```

### 1.5 Guards

Guards are inline field checks inside `transition`, not separate functions:

- `ctx.disabled` blocks every event except `SyncProps` (which flows through so a
  parent can lift the disabled state).
- `ctx.readonly` blocks the editing/open paths (`Open`, `CalendarSelectDate`,
  `Increment/DecrementSegment`, `TypeIntoSegment`, `SegmentChange`, `ValueCommit`,
  `ClearSegment`, `ClearAll`) while still allowing focus navigation.

### 1.6 Composition Strategy

The DateTimePicker does not embed separate `DatePicker` and `TimeField` machine instances. Instead, it manages a single unified state machine that owns both date segments and time segments. This design avoids cross-machine synchronization complexity and provides seamless segment navigation across the date-to-time boundary.

**Segment layout**: Date segments (Year, Month, Day in locale order) appear first, followed by a visual Separator, then time segments (Hour, Minute, optional Second, optional DayPeriod). The segments are grouped into a `DateSegmentGroup` and a `TimeSegmentGroup` for ARIA labeling, but navigation treats them as a single linear sequence.

**Value composition**: The combined `DateTime` value is assembled from individual segment values via `Context::assemble_datetime()`. When a date is selected from the calendar, only the date segments update; time segments retain their current values. When a time segment changes, only the time portion updates. The combined value is committed to `ctx.value` whenever all editable segments are complete.

**Calendar popover**: The `Content` part renders an embedded `Calendar` component. The adapter creates a `Calendar` `Service` using `api.calendar_props()` and wires the Calendar's `SelectDate` event back to the DateTimePicker as `Event::CalendarSelectDate(date)`. When a date is selected, the popover closes and focus returns to the first time segment (so the user can confirm or adjust the time).

**Segment navigation**: `FocusNextSegment` and `FocusPrevSegment` traverse all editable segments in linear order, crossing the date-to-time boundary transparently. Tab from the last date segment moves focus to the first time segment. Shift+Tab from the first time segment moves focus to the last date segment.

### 1.7 Full Machine Implementation

```rust
/// Typed identifier for every named effect intent the machine emits. The
/// adapter dispatches the real focus / timer operation on each name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Move focus into the embedded calendar grid.
    FocusCalendar,
    /// Restore focus to the trigger button.
    RestoreFocusToTrigger,
    /// Move focus to the first editable time segment (after calendar selection).
    FocusFirstTimeSegment,
    /// Adapter starts the type-buffer commit timer.
    TypeBufferCommit,
}

pub struct Machine;

impl Machine {
    /// If all segments are complete, assemble, clamp, and commit the value.
    fn maybe_publish(ctx: &mut Context) {
        if !ctx.is_complete() { return; }
        let Some(datetime) = ctx.assemble_datetime() else { return };
        let clamped = clamp_datetime(datetime, ctx.min_value.as_ref(), ctx.max_value.as_ref());
        ctx.date_value = Some(clamped.date().clone());
        ctx.time_value = Some(*clamped.time());
        ctx.date_segments = build_date_segments(&ctx.locale, ctx.date_value.as_ref());
        ctx.time_segments = build_time_segments(ctx.hour_cycle, ctx.granularity, ctx.time_value.as_ref());
        ctx.value.set(Some(clamped));
    }
}

impl ars_core::Machine for Machine {
    type State    = State;
    type Event    = Event;
    type Context  = Context;
    type Props    = Props;
    type Messages = Messages;
    type Effect   = Effect;
    type Api<'a>  = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let value = if let Some(controlled) = &props.value {
            Bindable::controlled(controlled.clone())
        } else {
            Bindable::uncontrolled(props.default_value.clone())
        };

        let locale = env.locale.clone();
        let date_value = value.get().as_ref().map(|dt| dt.date().clone());
        let time_value = value.get().as_ref().map(|dt| *dt.time());
        let hour_cycle = props.hour_cycle
            .unwrap_or_else(|| locale.hour_cycle(env.intl_backend.as_ref()));

        let date_segments = build_date_segments(&locale, date_value.as_ref());
        let time_segments = build_time_segments(hour_cycle, props.granularity, time_value.as_ref());

        let ctx = Context {
            value, date_value, time_value,
            focused_segment: None, open: false,
            date_segments, time_segments,
            type_buffer: String::new(),
            locale,
            messages: messages.clone(),
            intl_backend: Arc::clone(&env.intl_backend),
            granularity: props.granularity,
            hour_cycle,
            calendar: props.calendar,
            min_value: props.min_value.clone(),
            max_value: props.max_value.clone(),
            disabled: props.disabled, readonly: props.readonly,
            is_touched: false, required: props.required, invalid: props.invalid,
            name: props.name.clone(), is_rtl: props.is_rtl,
            today: props.today.clone(), visible_months: props.visible_months,
            ids: ComponentIds::from_id(&props.id),
        };

        (State::Idle, ctx)
    }

    /// Emits `SyncProps` when props change so controlled value / scalar fields reconcile.
    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        if old == new { Vec::new() } else { vec![Event::SyncProps(Box::new(new.clone()))] }
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        // `SyncProps` flows through even when disabled.
        if let Event::SyncProps(new_props) = event {
            let new_props = new_props.as_ref().clone();
            let mut probe = ctx.clone();
            sync_props(&mut probe, &new_props);
            let next_state = reconcile_state_after_sync(state, &probe);
            let clear_focus = next_state != *state && next_state == State::Idle;
            return Some(TransitionPlan::to(next_state).apply(move |ctx| {
                sync_props(ctx, &new_props);
                if clear_focus { ctx.focused_segment = None; ctx.type_buffer.clear(); }
            }));
        }

        if ctx.disabled { return None; }

        match event {
            // ── Popover open / close ──────────────────────────────────────
            // `open_plan`/`close_plan` are small `impl Machine` helpers:
            //   open_plan  → None if already open or read-only, else
            //                to(Open),    set ctx.open=true,  effect FocusCalendar.
            //   close_plan → None if not open, else
            //                to(Focused), set ctx.open=false, effect RestoreFocusToTrigger.
            Event::Open => Self::open_plan(state, ctx),

            Event::Close => Self::close_plan(state),

            Event::Toggle => match state {
                State::Open => Self::close_plan(state),
                _           => Self::open_plan(state, ctx),
            },

            // ── Calendar date selection ───────────────────────────────────
            Event::CalendarSelectDate(date) => {
                if ctx.readonly { return None; }
                // Defense-in-depth: reject a stale/scripted selection outside the
                // min/max date bounds before it is stored and displayed (the
                // embedded calendar should never offer one). Compares the date
                // against the date portion of the min/max `CalendarDateTime`.
                let in_range = ctx.min_value.as_ref()
                        .is_none_or(|min| date.compare(min.date()) != Ordering::Less)
                    && ctx.max_value.as_ref()
                        .is_none_or(|max| date.compare(max.date()) != Ordering::Greater);
                if !in_range { return None; }
                let date = date.clone();
                Some(TransitionPlan::to(State::Focused)
                    .apply(move |ctx| {
                        ctx.date_value = Some(date.clone());
                        ctx.open = false;
                        ctx.is_touched = true;
                        ctx.date_segments = build_date_segments(&ctx.locale, Some(&date));
                        Machine::maybe_publish(ctx);
                    })
                    .with_effect(PendingEffect::named(Effect::FocusFirstTimeSegment)))
            }

            // ── Segment focus / editing ───────────────────────────────────
            // Focus traversal crosses the date↔time boundary via `all_segments()`,
            // committing any pending type buffer on the way.
            Event::FocusSegment(kind) => {
                let kind = *kind;
                if !ctx.segment(kind).is_some_and(|seg| seg.is_editable) { return None; }
                Some(TransitionPlan::to(State::Focused)
                    .apply(move |ctx| {
                        commit_type_buffer(ctx);
                        ctx.focused_segment = Some(kind);
                        ctx.type_buffer.clear();
                    })
                    .cancel_effect(Effect::TypeBufferCommit))
            }

            Event::FocusNextSegment => match ctx.focused_segment {
                Some(current) => {
                    let next = ctx.next_editable_after(current);
                    Some(TransitionPlan::to(State::Focused)
                        .apply(move |ctx| {
                            commit_type_buffer(ctx);
                            ctx.type_buffer.clear();
                            if let Some(next) = next { ctx.focused_segment = Some(next); }
                        })
                        .cancel_effect(Effect::TypeBufferCommit))
                }
                None => {
                    let first = ctx.first_editable()?;
                    Some(TransitionPlan::to(State::Focused)
                        .apply(move |ctx| { ctx.focused_segment = Some(first); }))
                }
            },

            Event::FocusPrevSegment => {
                let current = ctx.focused_segment?;
                let previous = ctx.prev_editable_before(current)?;
                Some(TransitionPlan::to(State::Focused)
                    .apply(move |ctx| {
                        commit_type_buffer(ctx);
                        ctx.type_buffer.clear();
                        ctx.focused_segment = Some(previous);
                    })
                    .cancel_effect(Effect::TypeBufferCommit))
            }

            // Stepping wraps within the (calendar/month-aware) segment range.
            Event::IncrementSegment { segment } => {
                if ctx.readonly { return None; }
                let kind = *segment;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.type_buffer.clear();
                    ctx.increment_segment(kind);
                    ctx.is_touched = true;
                    sync_sub_value(ctx, kind);
                    Machine::maybe_publish(ctx);
                }).cancel_effect(Effect::TypeBufferCommit))
            }

            Event::DecrementSegment { segment } => {
                if ctx.readonly { return None; }
                let kind = *segment;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.type_buffer.clear();
                    ctx.decrement_segment(kind);
                    ctx.is_touched = true;
                    sync_sub_value(ctx, kind);
                    Machine::maybe_publish(ctx);
                }).cancel_effect(Effect::TypeBufferCommit))
            }

            // Type-ahead: numeric digits buffer + auto-advance; the day-period
            // segment accepts `a`/`p`. See `type_into_segment` below.
            Event::TypeIntoSegment { segment, ch } => {
                if ctx.readonly { return None; }
                type_into_segment(ctx, *segment, *ch)
            }

            Event::TypeBufferCommit { segment } => {
                if ctx.readonly { return None; }
                let kind = *segment;
                Some(TransitionPlan::context_only(move |ctx| commit_buffer_for_kind(ctx, kind)))
            }

            Event::SegmentChange { segment, value } => {
                if ctx.readonly { return None; }
                let kind = *segment;
                let value = *value;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.set_segment_value(kind, value);
                    ctx.is_touched = true;
                    sync_sub_value(ctx, kind);
                    Machine::maybe_publish(ctx);
                }))
            }

            Event::ValueCommit(datetime) => {
                if ctx.readonly { return None; }
                let datetime = datetime.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    apply_value(ctx, datetime.clone()); // clamps + rebuilds both groups
                    ctx.is_touched = true;
                    ctx.type_buffer.clear();
                }))
            }

            Event::ClearSegment { segment } => {
                if ctx.readonly { return None; }
                let kind = *segment;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.type_buffer.clear();
                    ctx.clear_segment_value(kind);
                    sync_sub_value(ctx, kind);
                    ctx.value.set(None);
                }).cancel_effect(Effect::TypeBufferCommit))
            }

            Event::ClearAll => {
                if ctx.readonly { return None; }
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    let editable = ctx.all_segments()
                        .filter(|seg| seg.is_editable).map(|seg| seg.kind).collect::<Vec<_>>();
                    for kind in editable { ctx.clear_segment_value(kind); }
                    ctx.date_value = None;
                    ctx.time_value = None;
                    ctx.value.set(None);
                    ctx.focused_segment = None;
                    ctx.type_buffer.clear();
                }).cancel_effect(Effect::TypeBufferCommit))
            }

            // ── Focus in / out ────────────────────────────────────────────
            Event::FocusIn => (*state == State::Idle).then(|| TransitionPlan::to(State::Focused)),

            Event::FocusOut => {
                let was_open = ctx.open;
                Some(TransitionPlan::to(State::Idle)
                    .apply(move |ctx| {
                        commit_type_buffer(ctx);
                        ctx.focused_segment = None;
                        ctx.type_buffer.clear();
                        if was_open { ctx.open = false; }
                    })
                    .cancel_effect(Effect::TypeBufferCommit))
            }

            // ── Keyboard shortcuts ────────────────────────────────────────
            Event::KeyDown { key } => match key {
                KeyboardKey::Escape if *state == State::Open => Self::close_plan(state),
                KeyboardKey::ArrowDown if *state != State::Open => Self::open_plan(state, ctx),
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
```

> The supporting free helpers referenced above — `build_date_segments` (locale
> order via `ars_i18n::date_order`, separator via `ars_i18n::date_field_separator`),
> `build_time_segments` (via `date_time::hour_cycle`), `type_into_segment`,
> `commit_type_buffer`/`commit_buffer_for_kind`, `sync_sub_value`, `apply_value`,
> `sync_props`, `reconcile_state_after_sync`, `clamp_datetime` (date-then-time
> `compare`), `format_segment_text`, `format_iso8601`, and `format_announcement` —
> live alongside the machine in that module. `Context::refresh_date_ranges`
> keeps the month/day segment maxima in sync with the year/month/era/calendar
> (via the intl backend's `max_months_in_year`/`days_in_month`), and
> `project_date` reprojects clamped/committed dates back into `ctx.calendar` so
> the displayed segments and a subsequent edit always agree on the ISO date.

### 1.8 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "date-time-picker"]
pub enum Part {
    Root,
    Label,
    Control,
    DateSegmentGroup,
    TimeSegmentGroup,
    Segment { kind: DateSegmentKind },
    Literal { index: usize },
    Separator,
    Trigger,
    ClearTrigger,
    Positioner,
    Content,
    Description,
    ErrorMessage,
    HiddenInput,
}

/// API for the DateTimePicker component.
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

    /// Attributes for the root element.
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
        if self.props.invalid {
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
        }
        if self.ctx.is_rtl {
            attrs.set(HtmlAttr::Dir, "rtl");
        }
        // Announce the selected datetime when both date and time are complete.
        // `format_announcement` builds a locale-ordered date plus an
        // hour-cycle-aware time string from `date_value`/`time_value`.
        if let Some(announcement) = format_announcement(self.ctx) {
            attrs.set(HtmlAttr::Aria(AriaAttr::Description),
                (self.ctx.messages.selected_datetime_label)(&announcement, &self.ctx.locale));
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
        attrs.set(HtmlAttr::For, self.ctx.ids.part("control"));
        attrs
    }

    /// Attributes for the control wrapper element.
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("control"));
        attrs.set(HtmlAttr::Role, "group");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));
        // Chain Description + ErrorMessage so AT announces help/validation when
        // focus enters the segment group (the hidden input cannot be described).
        let mut described_by = Vec::new();
        if self.props.description.is_some() {
            described_by.push(self.ctx.ids.part("description"));
        }
        if self.ctx.invalid && self.props.error_message.is_some() {
            described_by.push(self.ctx.ids.part("error-message"));
        }
        if !described_by.is_empty() {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), described_by.join(" "));
        }
        attrs
    }

    /// Attributes for the date segment group.
    pub fn date_segment_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::DateSegmentGroup.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "group");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.date_group_label)(&self.ctx.locale));
        if self.ctx.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }
        if self.props.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }
        attrs
    }

    /// Attributes for the time segment group.
    pub fn time_segment_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::TimeSegmentGroup.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "group");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.time_group_label)(&self.ctx.locale));
        attrs
    }

    /// Attributes for a segment element (date or time).
    pub fn segment_attrs(&self, kind: &DateSegmentKind) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Segment { kind: *kind }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        // Find the segment in either date or time segments.
        let Some(seg) = self.ctx.segment(*kind) else { return AttrMap::new() };

        attrs.set(HtmlAttr::Id, self.ctx.ids.item("segment", &seg.kind.data_name()));
        attrs.set(HtmlAttr::Role, "spinbutton");
        attrs.set(HtmlAttr::TabIndex, if self.ctx.disabled { "-1" } else { "0" });
        attrs.set(HtmlAttr::Data("ars-segment"), seg.kind.data_name());
        // Segment ARIA labels resolve from this component's per-segment Messages
        // fields (see §4.1), not from a borrowed `date_field::Messages`.
        attrs.set(HtmlAttr::Aria(AriaAttr::Label),
            segment_aria_label(seg.kind, &self.ctx.messages, &self.ctx.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), seg.min.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), seg.max.to_string());

        // `inputmode="numeric"` only on numeric segments (not the day-period).
        if seg.kind.is_numeric() {
            attrs.set(HtmlAttr::InputMode, "numeric");
        }

        if let Some(v) = seg.value {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), v.to_string());
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), &seg.text);
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), &seg.placeholder);
        }

        if self.ctx.focused_segment == Some(seg.kind) {
            attrs.set_bool(HtmlAttr::Data("ars-focused"), true);
        }
        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        if self.ctx.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }
        attrs
    }

    /// Attributes for a literal separator element (colon, slash, space).
    pub fn literal_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Literal { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attributes for the separator between date and time groups.
    pub fn separator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Separator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attributes for the trigger button.
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("trigger"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.trigger_label)(&self.ctx.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded),
            if self.is_open() { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), self.ctx.ids.part("content"));
        // Explicit `type="button"` so activating the trigger never submits a
        // surrounding form (the HTML default button type is `submit`).
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::TabIndex, "0");
        // `readonly` blocks opening the popover, so the trigger is disabled in
        // both states (an operable-looking no-op button would mislead users).
        if self.ctx.disabled || self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Disabled, true);
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        attrs
    }

    /// Attributes for the clear trigger button.
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ClearTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("clear-trigger"));
        attrs.set(HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.clear_label)(&self.ctx.locale));
        // Explicit `type="button"` so clearing never submits a surrounding form.
        attrs.set(HtmlAttr::Type, "button");
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

    /// Attributes for the positioner wrapper.
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

    /// Attributes for the popover content.
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("content"));
        attrs.set(HtmlAttr::Role, "dialog");
        attrs.set(HtmlAttr::Data("ars-state"), self.state_name());
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
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        attrs
    }

    /// Attributes for the hidden input element.
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::HiddenInput.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "hidden");
        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name);
        }
        // ISO 8601 datetime: YYYY-MM-DDTHH:MM:SS (always 24-hour, regardless of
        // the visible hour cycle). `format_iso8601` serializes the date from the
        // canonical ISO calendar slots (`CalendarDate::to_iso8601`), so a
        // non-Gregorian selected value still submits the ISO datetime.
        if let Some(dt) = self.ctx.value.get() {
            attrs.set(HtmlAttr::Value, format_iso8601(dt));
        }
        attrs
    }

    // ── Event handlers ──────────────────────────────────────────────────

    /// Handle click on the trigger button.
    pub fn on_trigger_click(&self) {
        (self.send)(Event::Toggle);
    }

    /// Handle keydown on the trigger button.
    pub fn on_trigger_keydown(&self, data: &KeyboardEventData) {
        match data.key {
            KeyboardKey::Enter | KeyboardKey::Space => (self.send)(Event::Toggle),
            KeyboardKey::ArrowDown => (self.send)(Event::Open),
            _ => {}
        }
    }

    /// Handle click on the clear trigger button.
    pub fn on_clear_trigger_click(&self) {
        (self.send)(Event::ClearAll);
    }

    /// Handle keydown on a segment (date or time).
    ///
    /// Typed characters arrive via `KeyboardEventData::character` (there is no
    /// `KeyboardKey::Char` variant). `dir` flips Arrow Left/Right for RTL, and
    /// `Alt+ArrowDown` opens the popover.
    pub fn on_segment_keydown(&self, kind: DateSegmentKind, data: &KeyboardEventData, dir: Direction) {
        let is_rtl = dir.is_rtl();
        match data.key {
            KeyboardKey::ArrowUp => (self.send)(Event::IncrementSegment { segment: kind }),
            KeyboardKey::ArrowDown if data.alt_key => (self.send)(Event::Open),
            KeyboardKey::ArrowDown => (self.send)(Event::DecrementSegment { segment: kind }),
            KeyboardKey::ArrowRight if is_rtl => (self.send)(Event::FocusPrevSegment),
            KeyboardKey::ArrowLeft if is_rtl => (self.send)(Event::FocusNextSegment),
            KeyboardKey::ArrowLeft => (self.send)(Event::FocusPrevSegment),
            KeyboardKey::Tab if data.shift_key => (self.send)(Event::FocusPrevSegment),
            KeyboardKey::ArrowRight | KeyboardKey::Tab => (self.send)(Event::FocusNextSegment),
            KeyboardKey::Backspace | KeyboardKey::Delete => {
                (self.send)(Event::ClearSegment { segment: kind })
            }
            KeyboardKey::Escape => (self.send)(Event::KeyDown { key: data.key }),
            _ if data.character.is_some() => {
                if let Some(ch) = data.character {
                    (self.send)(Event::TypeIntoSegment { segment: kind, ch });
                }
            }
            _ => {}
        }
    }

    /// Handle focus on a segment.
    pub fn on_segment_focus(&self, kind: DateSegmentKind) {
        (self.send)(Event::FocusSegment(kind));
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

    // ── Calendar composition ────────────────────────────────────────────

    /// Build Calendar Props from the DateTimePicker's current state.
    /// The adapter creates a Calendar machine with these props inside the
    /// Content part and wires its SelectDate event back to this machine.
    pub fn calendar_props(&self) -> calendar::Props {
        calendar::Props {
            id: format!("{}-calendar", self.ctx.ids.id()),
            value: Some(self.ctx.date_value.clone()),
            min: self.ctx.min_value.as_ref().map(|dt| dt.date().clone()),
            max: self.ctx.max_value.as_ref().map(|dt| dt.date().clone()),
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            is_rtl: self.ctx.is_rtl,
            visible_months: self.ctx.visible_months,
            today: self.ctx.today.clone(),
            ..calendar::Props::default()
        }
    }

    // ── Computed state accessors ─────────────────────────────────────────

    /// Whether the popover is currently open.
    pub fn is_open(&self) -> bool {
        *self.state == State::Open
    }

    /// The currently selected date-time value.
    pub fn selected_value(&self) -> Option<&DateTime> {
        self.ctx.value.get().as_ref()
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
    fn state_name(&self) -> &'static str {
        match self.state {
            State::Idle    => "idle",
            State::Focused => "focused",
            State::Open    => "open",
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
            Part::DateSegmentGroup => self.date_segment_group_attrs(),
            Part::TimeSegmentGroup => self.time_segment_group_attrs(),
            Part::Segment { kind } => self.segment_attrs(&kind),
            Part::Literal { index } => self.literal_attrs(index),
            Part::Separator => self.separator_attrs(),
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
DateTimePicker (en-US, closed, Minute granularity, H12)
└── Root                                data-ars-scope="date-time-picker"  data-ars-state="idle"
    ├── Label                           <label for="{id}-control">
    ├── Control                         role="group"  aria-labelledby="{id}-label"
    │   ├── DateSegmentGroup            role="group"  aria-label="Date"
    │   │   ├── Segment (Month)         role="spinbutton"  aria-valuemin="1"  aria-valuemax="12"
    │   │   ├── Literal "/"             aria-hidden="true"
    │   │   ├── Segment (Day)           role="spinbutton"  aria-valuemin="1"  aria-valuemax="31"
    │   │   ├── Literal "/"             aria-hidden="true"
    │   │   └── Segment (Year)          role="spinbutton"  aria-valuemin="1"  aria-valuemax="9999"
    │   ├── Separator                   aria-hidden="true"  (visual: ", " or " ")
    │   ├── TimeSegmentGroup            role="group"  aria-label="Time"
    │   │   ├── Segment (Hour)          role="spinbutton"  aria-valuemin="1"  aria-valuemax="12"
    │   │   ├── Literal ":"             aria-hidden="true"
    │   │   ├── Segment (Minute)        role="spinbutton"  aria-valuemin="0"  aria-valuemax="59"
    │   │   ├── Literal "\u{00A0}"      aria-hidden="true"
    │   │   └── Segment (DayPeriod)     role="spinbutton"  aria-label="AM/PM"
    │   ├── ClearTrigger                aria-label="Clear date and time"  [hidden when empty]
    │   └── Trigger                     aria-label="Open date and time picker"  aria-expanded="false"
    ├── Positioner                      [hidden when closed]
    │   └── Content                     role="dialog"
    │       └── <Calendar />            Embedded Calendar component
    ├── Description                     optional help text
    ├── ErrorMessage                    role="alert"  aria-live="polite"
    └── HiddenInput                     type="hidden"  name="..."  value="2024-03-15T14:30:00"
```

| Part               | HTML Element            | Required | Key Attributes                                               |
| ------------------ | ----------------------- | -------- | ------------------------------------------------------------ |
| `Root`             | `<div>`                 | yes      | `data-ars-scope`, `data-ars-state`                           |
| `Label`            | `<label>`               | yes      | `for` pointing to Control                                    |
| `Control`          | `<div>`                 | yes      | `role="group"`, `aria-labelledby`                            |
| `DateSegmentGroup` | `<div>`                 | yes      | `role="group"`, `aria-label="Date"`                          |
| `TimeSegmentGroup` | `<div>`                 | yes      | `role="group"`, `aria-label="Time"`                          |
| `Segment`          | `<div>`                 | yes      | `role="spinbutton"`, `tabindex="0"`, `aria-valuenow/min/max` |
| `Literal`          | `<span>`                | yes      | `aria-hidden="true"`                                         |
| `Separator`        | `<span>`                | yes      | `aria-hidden="true"`                                         |
| `Trigger`          | `<button>`              | yes      | `aria-label`, `aria-expanded`, `aria-controls`               |
| `ClearTrigger`     | `<button>`              | optional | `aria-label`, hidden when no value                           |
| `Positioner`       | `<div>`                 | yes      | Hidden when popover closed                                   |
| `Content`          | `<div role="dialog">`   | yes      | Contains embedded Calendar                                   |
| `Description`      | `<div>`                 | optional | Help text                                                    |
| `ErrorMessage`     | `<div role="alert">`    | optional | Validation error; `aria-live="polite"`                       |
| `HiddenInput`      | `<input type="hidden">` | yes      | ISO 8601 datetime string for form submission                 |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Element            | Role/Attribute          | Details                                                                           |
| ------------------ | ----------------------- | --------------------------------------------------------------------------------- |
| `Control`          | `role="group"`          | `aria-labelledby` → Label; `aria-describedby` → Description + ErrorMessage IDs    |
| `DateSegmentGroup` | `role="group"`          | `aria-label="Date"`; groups date segments                                         |
| `TimeSegmentGroup` | `role="group"`          | `aria-label="Time"`; groups time segments                                         |
| `Segment`          | `role="spinbutton"`     | `aria-valuenow`, `aria-valuemin`, `aria-valuemax`, `aria-valuetext`, `aria-label` |
| `Trigger`          | `<button>`              | `aria-label="Open date and time picker"`, `aria-expanded`, `aria-controls`        |
| `ClearTrigger`     | `<button>`              | `aria-label="Clear date and time"`                                                |
| `Content`          | `role="dialog"`         | Contains the Calendar; announced when opened                                      |
| `ErrorMessage`     | `role="alert"`          | `aria-live="polite"`; announced immediately on render                             |
| `Literal`          | `<span>`                | `aria-hidden="true"`; decorative separators                                       |
| `Separator`        | `<span>`                | `aria-hidden="true"`; decorative separator between date and time                  |
| `HiddenInput`      | `<input type="hidden">` | Carries the ISO 8601 value for form submission                                    |

When `disabled=true`: all segments receive `aria-disabled="true"`, trigger and clear buttons are disabled.
When `readonly=true`: all segments receive `aria-readonly="true"`, trigger and clear buttons are disabled.
When `required=true`: the DateSegmentGroup receives `aria-required="true"`.
When `invalid=true`: the DateSegmentGroup receives `aria-invalid="true"`, and the ErrorMessage part is rendered.

### 3.2 Keyboard Interaction

| Key                    | Context                            | Action                                                     |
| ---------------------- | ---------------------------------- | ---------------------------------------------------------- |
| `ArrowUp`              | Segment focused                    | Increment the focused segment value by 1, wrapping         |
| `ArrowDown`            | Segment focused                    | Decrement the focused segment value by 1, wrapping         |
| `ArrowLeft`            | Segment focused                    | Move focus to the previous editable segment (date or time) |
| `ArrowRight`           | Segment focused                    | Move focus to the next editable segment (date or time)     |
| `Tab`                  | Last date segment focused          | Move focus to the first time segment                       |
| `Shift+Tab`            | First time segment focused         | Move focus to the last date segment                        |
| `Tab`                  | Last time segment focused          | Move focus to the trigger button                           |
| `0`-`9`                | Numeric segment focused            | Type-ahead entry; auto-advances to next segment when full  |
| `a` / `p`              | DayPeriod segment focused          | Set AM / PM                                                |
| `Backspace` / `Delete` | Segment focused                    | Clear the focused segment value                            |
| `Enter` / `Space`      | Trigger focused                    | Toggle the popover open/closed                             |
| `ArrowDown`            | Trigger focused or segment focused | Open the calendar popover (Alt+ArrowDown at adapter level) |
| `Escape`               | Popover open                       | Close the popover; return focus to the trigger             |

> RTL: `ArrowLeft` and `ArrowRight` swap direction for segment navigation per `03-accessibility.md` section 4.1.

### 3.3 Focus Management

1. User tabs to the first date segment or the trigger button.
2. Arrow keys navigate between segments within the date and time groups. Navigation crosses the date-to-time boundary seamlessly.
3. Clicking the trigger or pressing Enter/Space on it opens the calendar popover.
4. When the popover opens, focus moves into the Calendar grid (first selected date, or today).
5. Selecting a date in the calendar closes the popover and moves focus to the first time segment, so the user can confirm or adjust the time.
6. Pressing Escape in the popover closes it and returns focus to the trigger.
7. Tab from inside the calendar closes the popover and moves focus to the next focusable element after the DateTimePicker.
8. When focus leaves the component entirely (`FocusOut`), any pending type-ahead buffer is committed and the popover closes.

## 4. Internationalization

### 4.1 Messages

```rust
/// Messages for the DateTimePicker component.
///
/// Carries the five composite labels (trigger, clear, the two segment groups,
/// and the selected-value announcement) plus its own per-segment spinbutton
/// labels. The segment labels live here — rather than being borrowed from
/// `date_field::Messages` — so a `DateTimePicker` owns a complete, independently
/// localizable label set.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Trigger button label (default: "Open date and time picker").
    pub trigger_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Clear button label (default: "Clear date and time").
    pub clear_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Date segment group label (default: "Date").
    pub date_group_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Time segment group label (default: "Time").
    pub time_group_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Announces the selected date-time (default: "Selected: {value}").
    pub selected_datetime_label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
    /// Label for the year segment (default: "Year").
    pub year_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the month segment (default: "Month").
    pub month_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the day segment (default: "Day").
    pub day_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the hour segment (default: "Hour").
    pub hour_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the minute segment (default: "Minute").
    pub minute_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the second segment (default: "Second").
    pub second_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the day-period segment (default: "AM/PM").
    pub day_period_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            trigger_label: MessageFn::static_str("Open date and time picker"),
            clear_label: MessageFn::static_str("Clear date and time"),
            date_group_label: MessageFn::static_str("Date"),
            time_group_label: MessageFn::static_str("Time"),
            selected_datetime_label: MessageFn::new(|value: &str, _locale: &Locale| {
                format!("Selected: {value}")
            }),
            year_label: MessageFn::static_str("Year"),
            month_label: MessageFn::static_str("Month"),
            day_label: MessageFn::static_str("Day"),
            hour_label: MessageFn::static_str("Hour"),
            minute_label: MessageFn::static_str("Minute"),
            second_label: MessageFn::static_str("Second"),
            day_period_label: MessageFn::static_str("AM/PM"),
        }
    }
}

impl ComponentMessages for Messages {}
```

All user-visible text is provided via the `Messages` struct, including the
per-segment spinbutton labels resolved by `segment_aria_label`. Date segment
order and separators are locale-dependent (via `ars_i18n::date_order`; see
`build_date_segments`). Time segment display uses the locale's preferred hour
cycle when `hour_cycle` is `None`.

## 5. Form Integration

The DateTimePicker participates in HTML form submission via a hidden `<input type="hidden">` element rendered by the `HiddenInput` part. The hidden input carries the combined date-time value in ISO 8601 format.

- **ISO value**: The hidden input's `value` is always `YYYY-MM-DDTHH:MM:SS` (e.g., `2024-03-15T14:30:00`), regardless of the display format or locale used in the visible segments.
- **`name` prop**: When set, the hidden input includes `name="{value}"` so the date-time is submitted with the form under that key.
- **Required validation**: When `required=true`, the DateSegmentGroup carries `aria-required="true"`. Form validation treats an empty value (no date-time selected) as invalid.
- **Reset**: On form reset, the adapter restores the initial value (the `default_value` from Props or `None`) and rebuilds all segments accordingly.
- **`aria-describedby`**: The Control group's `aria-describedby` chains the Description and ErrorMessage part IDs so assistive technology announces help text and validation errors in context.
- **Disabled/readonly propagation**: When the DateTimePicker is nested inside a form context that sets `disabled` or `readonly`, those values propagate per `07-forms.md` section 15.

## 6. Library Parity

> Compared against: Ark UI (`DatePicker`).

Ark UI does not have a dedicated DateTimePicker component. Its `DatePicker` supports time selection only indirectly (time segments are not part of its anatomy). ars-ui's DateTimePicker is a novel composite that unifies date segments, time segments, and a calendar popover into a single state machine, providing a more integrated experience than Ark UI's date-only picker.

### 6.1 Props

| Feature          | ars-ui                              | Ark UI                   | Notes                                      |
| ---------------- | ----------------------------------- | ------------------------ | ------------------------------------------ |
| Controlled value | `value` (DateTime)                  | `value` (DateValue[])    | Ark UI is date-only                        |
| Min/max          | `min_value`, `max_value` (DateTime) | `min`, `max` (DateValue) | ars-ui includes time constraints           |
| Granularity      | `granularity` (TimeGranularity)     | --                       | ars-ui adds time granularity               |
| Hour cycle       | `hour_cycle`                        | --                       | ars-ui adds 12/24h support                 |
| Calendar system  | `calendar`                          | --                       | ars-ui supports non-Gregorian              |
| Disabled         | `disabled`                          | `disabled`               | Equivalent                                 |
| Read-only        | `readonly`                          | `readOnly`               | Equivalent                                 |
| Required         | `required`                          | `required`               | Equivalent                                 |
| Invalid          | `invalid`                           | `invalid`                | Equivalent                                 |
| Close on select  | --                                  | `closeOnSelect`          | ars-ui always closes after calendar select |
| Locale           | `locale`                            | `locale`                 | Equivalent                                 |
| Name             | `name`                              | `name`                   | Equivalent                                 |
| Visible months   | `visible_months`                    | `numOfMonths`            | Equivalent                                 |

**Gaps:** None. ars-ui is a superset with time-specific features.

### 6.2 Anatomy

| Part          | ars-ui                         | Ark UI         | Notes                            |
| ------------- | ------------------------------ | -------------- | -------------------------------- |
| Root          | `Root`                         | `Root`         | Equivalent                       |
| Label         | `Label`                        | `Label`        | Equivalent                       |
| Control       | `Control`                      | `Control`      | Equivalent                       |
| Date segments | `DateSegmentGroup` + `Segment` | `Input`        | ars-ui has grouped date segments |
| Time segments | `TimeSegmentGroup` + `Segment` | --             | Novel in ars-ui                  |
| Separator     | `Separator`                    | --             | Between date and time groups     |
| Trigger       | `Trigger`                      | `Trigger`      | Equivalent                       |
| Clear trigger | `ClearTrigger`                 | `ClearTrigger` | Equivalent                       |
| Positioner    | `Positioner`                   | `Positioner`   | Equivalent                       |
| Content       | `Content`                      | `Content`      | Equivalent                       |
| Description   | `Description`                  | --             | ars-ui form integration          |
| Error message | `ErrorMessage`                 | --             | ars-ui form integration          |
| Hidden input  | `HiddenInput`                  | --             | ars-ui form integration          |
| Literal       | `Literal`                      | --             | Separators between segments      |

**Gaps:** None. ars-ui has a richer anatomy.

### 6.3 Events

| Callback        | ars-ui                                       | Ark UI          | Notes                                  |
| --------------- | -------------------------------------------- | --------------- | -------------------------------------- |
| Value change    | `ValueCommit` / `SegmentChange`              | `onValueChange` | Equivalent                             |
| Open change     | `Open` / `Close`                             | `onOpenChange`  | Equivalent                             |
| Segment editing | `IncrementSegment`, `DecrementSegment`, etc. | --              | ars-ui has fine-grained segment events |

**Gaps:** None.

### 6.4 Features

| Feature                           | ars-ui | Ark UI |
| --------------------------------- | ------ | ------ |
| Date + time input                 | Yes    | No     |
| Calendar popover                  | Yes    | Yes    |
| Segmented editing (spinbutton)    | Yes    | No     |
| Type-ahead numeric entry          | Yes    | No     |
| AM/PM segment                     | Yes    | No     |
| Cross-boundary segment navigation | Yes    | No     |
| Hidden form input (ISO 8601)      | Yes    | No     |

**Gaps:** None. ars-ui is a superset.

### 6.5 Summary

- **Overall:** ars-ui exceeds Ark UI's capabilities since Ark UI has no dedicated date-time picker.
- **Divergences:** ars-ui combines date and time segments in a single machine with a unified calendar popover. Ark UI would require composing separate DatePicker and time input components.
- **Recommended additions:** None.
