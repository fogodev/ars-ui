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
    /// The full date-time value was committed (Enter, blur, or calendar cell click).
    ValueCommit(Option<DateTime>),
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
    /// Clear the value of a segment.
    ClearSegment { segment: DateSegmentKind },
    /// Clear the entire date-time value.
    ClearAll,
    /// A date was selected from the calendar popover.
    CalendarSelectDate(CalendarDate),
    /// Navigate to the next month in the calendar.
    CalendarNext,
    /// Navigate to the previous month in the calendar.
    CalendarPrev,
    /// Focus entered the component (any segment or trigger).
    FocusIn,
    /// Focus left the component entirely.
    FocusOut,
    /// Keyboard event on a segment or trigger.
    KeyDown { key: KeyboardKey },
}
```

### 1.3 Context

```rust
/// Context for the DateTimePicker component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The combined date-time value (controlled/uncontrolled).
    pub value: Bindable<Option<DateTime>>,
    /// The date portion of the value (synced from value or calendar selection).
    pub date_value: Option<CalendarDate>,
    /// The time portion of the value (synced from value or time segment edits).
    pub time_value: Option<Time>,
    /// The currently focused segment (date or time).
    pub focused_segment: Option<DateSegmentKind>,
    /// Whether the calendar popover is open.
    pub open: bool,
    /// Date segments (Year, Month, Day) in locale order.
    pub date_segments: Vec<DateSegment>,
    /// Time segments (Hour, Minute, optional Second, optional DayPeriod).
    pub time_segments: Vec<DateSegment>,
    /// The type-ahead buffer for numeric segment editing.
    pub type_buffer: String,
    /// The locale for formatting.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// The time granularity (Hour, Minute, Second).
    pub granularity: TimeGranularity,
    /// The resolved hour cycle.
    pub hour_cycle: HourCycle,
    /// Minimum allowed date-time.
    pub min_value: Option<DateTime>,
    /// Maximum allowed date-time.
    pub max_value: Option<DateTime>,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is read-only.
    pub readonly: bool,
    /// Whether the field has been interacted with.
    pub is_touched: bool,
    /// Whether the field is required.
    pub required: bool,
    /// Form field name.
    pub name: Option<String>,
    /// Right-to-left layout.
    pub is_rtl: bool,
    /// Component IDs.
    pub ids: ComponentIds,
}

impl Context {
    /// All segments in display order: date segments, then time segments.
    pub fn all_segments(&self) -> impl Iterator<Item = &DateSegment> {
        self.date_segments.iter().chain(self.time_segments.iter())
    }

    /// Find the first editable segment across date and time.
    pub fn first_editable(&self) -> Option<DateSegmentKind> {
        self.all_segments().find(|s| s.is_editable).map(|s| s.kind)
    }

    /// Find the next editable segment after the given segment, crossing the
    /// date-to-time boundary when the current segment is the last date segment.
    pub fn next_editable_after(&self, kind: DateSegmentKind) -> Option<DateSegmentKind> {
        let all: Vec<_> = self.all_segments().collect();
        let idx = all.iter().position(|s| s.kind == kind)?;
        all[idx + 1..].iter().find(|s| s.is_editable).map(|s| s.kind)
    }

    /// Find the previous editable segment before the given segment, crossing
    /// the time-to-date boundary when the current segment is the first time segment.
    pub fn prev_editable_before(&self, kind: DateSegmentKind) -> Option<DateSegmentKind> {
        let all: Vec<_> = self.all_segments().collect();
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

    /// Set a segment's value, looking in both date and time segment lists.
    pub fn set_segment_value(&mut self, kind: DateSegmentKind, raw: i32) {
        let seg = self.date_segments.iter_mut()
            .chain(self.time_segments.iter_mut())
            .find(|s| s.kind == kind);
        if let Some(seg) = seg {
            let v = raw.clamp(seg.min, seg.max);
            seg.value = Some(v);
            seg.text = format!("{:02}", v);
        }
    }

    /// Clear a segment's value.
    pub fn clear_segment_value(&mut self, kind: DateSegmentKind) {
        let seg = self.date_segments.iter_mut()
            .chain(self.time_segments.iter_mut())
            .find(|s| s.kind == kind);
        if let Some(seg) = seg {
            seg.value = None;
            seg.text = String::new();
        }
    }

    /// Increment a segment's value, wrapping around.
    pub fn increment_segment(&mut self, kind: DateSegmentKind) {
        let seg = self.date_segments.iter()
            .chain(self.time_segments.iter())
            .find(|s| s.kind == kind)
            .cloned();
        if let Some(seg) = seg {
            let cur  = seg.value.unwrap_or(seg.min);
            let next = if cur >= seg.max { seg.min } else { cur + 1 };
            self.set_segment_value(kind, next);
        }
    }

    /// Decrement a segment's value, wrapping around.
    pub fn decrement_segment(&mut self, kind: DateSegmentKind) {
        let seg = self.date_segments.iter()
            .chain(self.time_segments.iter())
            .find(|s| s.kind == kind)
            .cloned();
        if let Some(seg) = seg {
            let cur  = seg.value.unwrap_or(seg.max);
            let next = if cur <= seg.min { seg.max } else { cur - 1 };
            self.set_segment_value(kind, next);
        }
    }

    /// Check if all editable segments across both date and time have values.
    pub fn is_complete(&self) -> bool {
        self.all_segments()
            .filter(|s| s.is_editable)
            .all(|s| s.value.is_some())
    }

    /// Assemble a CalendarDate from current date segment values.
    pub fn assemble_date(&self) -> Option<CalendarDate> {
        let year  = self.date_segments.iter().find(|s| s.kind == DateSegmentKind::Year)?.value?;
        let month = self.date_segments.iter().find(|s| s.kind == DateSegmentKind::Month)?.value? as u8;
        let day   = self.date_segments.iter().find(|s| s.kind == DateSegmentKind::Day)?.value? as u8;
        Some(CalendarDate::new_gregorian(
            year,
            NonZero::new(month).expect("month is 1-based from segment constraints"),
            NonZero::new(day).expect("day is 1-based from segment constraints"),
        ))
    }

    /// Assemble a Time from current time segment values.
    pub fn assemble_time(&self) -> Option<Time> {
        let raw_hour = self.time_segments.iter().find(|s| s.kind == DateSegmentKind::Hour)?.value? as u8;
        let minute   = self.time_segments.iter().find(|s| s.kind == DateSegmentKind::Minute)?.value? as u8;
        let second   = self.time_segments.iter()
            .find(|s| s.kind == DateSegmentKind::Second)
            .and_then(|s| s.value)
            .unwrap_or(0) as u8;

        let hour = if self.hour_cycle.has_day_period() {
            let is_pm = self.time_segments.iter()
                .find(|s| s.kind == DateSegmentKind::DayPeriod)
                .and_then(|s| s.value)
                .unwrap_or(0) == 1;
            match self.hour_cycle {
                HourCycle::H12 => {
                    if is_pm { if raw_hour == 12 { 12 } else { raw_hour + 12 } }
                    else     { if raw_hour == 12 { 0  } else { raw_hour       } }
                }
                HourCycle::H11 => if is_pm { raw_hour + 12 } else { raw_hour },
                _ => raw_hour,
            }
        } else {
            raw_hour
        };

        Some(Time::new(hour, minute, second))
    }

    /// Assemble a DateTime from the current date and time segment values.
    pub fn assemble_datetime(&self) -> Option<DateTime> {
        let date = self.assemble_date()?;
        let time = self.assemble_time()?;
        Some(DateTime { date, time })
    }
}
```

### 1.4 Props

```rust
/// Props for the DateTimePicker component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The ID of the date-time picker.
    pub id: String,
    /// Controlled date-time value. `Some(v)` = controlled, `None` = uncontrolled.
    pub value: Option<Option<DateTime>>,
    /// Default date-time for uncontrolled mode.
    pub default_value: Option<DateTime>,
    /// Minimum allowed date-time.
    pub min_value: Option<DateTime>,
    /// Maximum allowed date-time.
    pub max_value: Option<DateTime>,
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
    /// Hour cycle override. `HourCycle::Auto` uses the locale default.
    pub hour_cycle: HourCycle,
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
            hour_cycle: HourCycle::Auto,
            required: false,
            label: String::new(),
            description: None,
            error_message: None,
            invalid: false,
            is_rtl: false,
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

### 1.6 Composition Strategy

The DateTimePicker does not embed separate `DatePicker` and `TimeField` machine instances. Instead, it manages a single unified state machine that owns both date segments and time segments. This design avoids cross-machine synchronization complexity and provides seamless segment navigation across the date-to-time boundary.

**Segment layout**: Date segments (Year, Month, Day in locale order) appear first, followed by a visual Separator, then time segments (Hour, Minute, optional Second, optional DayPeriod). The segments are grouped into a `DateSegmentGroup` and a `TimeSegmentGroup` for ARIA labeling, but navigation treats them as a single linear sequence.

**Value composition**: The combined `DateTime` value is assembled from individual segment values via `Context::assemble_datetime()`. When a date is selected from the calendar, only the date segments update; time segments retain their current values. When a time segment changes, only the time portion updates. The combined value is committed to `ctx.value` whenever all editable segments are complete.

**Calendar popover**: The `Content` part renders an embedded `Calendar` component. The adapter creates a `Calendar` `Service` using `api.calendar_props()` and wires the Calendar's `SelectDate` event back to the DateTimePicker as `Event::CalendarSelectDate(date)`. When a date is selected, the popover closes and focus returns to the first time segment (so the user can confirm or adjust the time).

**Segment navigation**: `FocusNextSegment` and `FocusPrevSegment` traverse all editable segments in linear order, crossing the date-to-time boundary transparently. Tab from the last date segment moves focus to the first time segment. Shift+Tab from the first time segment moves focus to the last date segment.

### 1.7 Full Machine Implementation

```rust
pub struct Machine;

impl Machine {
    /// If all segments are complete, assemble and commit the DateTime value.
    fn maybe_publish(ctx: &mut Context) {
        if !ctx.is_complete() { return; }
        let Some(dt) = ctx.assemble_datetime() else { return };
        let clamped = match (&ctx.min_value, &ctx.max_value) {
            (Some(min), _) if dt < *min => min.clone(),
            (_, Some(max)) if dt > *max => max.clone(),
            _ => dt,
        };
        ctx.value.set(Some(clamped));
    }

    /// Build date segments for the given locale and current date value.
    fn build_date_segments(locale: &Locale, date: &Option<CalendarDate>) -> Vec<DateSegment> {
        let mut segs = Vec::new();

        let mut year_seg = DateSegment::new_numeric(DateSegmentKind::Year, 1, 9999, "yyyy");
        let mut month_seg = DateSegment::new_numeric(DateSegmentKind::Month, 1, 12, "mm");
        let mut day_seg = DateSegment::new_numeric(DateSegmentKind::Day, 1, 31, "dd");

        if let Some(d) = date {
            year_seg.value  = Some(d.year);
            year_seg.text   = format!("{:04}", d.year);
            month_seg.value = Some(d.month.get() as i32);
            month_seg.text  = format!("{:02}", d.month.get());
            day_seg.value   = Some(d.day.get() as i32);
            day_seg.text    = format!("{:02}", d.day.get());
        }

        // Locale-dependent segment order.
        match locale.as_str() {
            "ja-JP" | "zh-CN" | "zh-TW" | "ko-KR" => {
                segs.push(year_seg);
                segs.push(DateSegment::new_literal("/"));
                segs.push(month_seg);
                segs.push(DateSegment::new_literal("/"));
                segs.push(day_seg);
            }
            "en-GB" | "de-DE" | "fr-FR" | "es-ES" | "it-IT" | "ru-RU" => {
                segs.push(day_seg);
                segs.push(DateSegment::new_literal("/"));
                segs.push(month_seg);
                segs.push(DateSegment::new_literal("/"));
                segs.push(year_seg);
            }
            _ => {
                // en-US and fallback: MM/DD/YYYY
                segs.push(month_seg);
                segs.push(DateSegment::new_literal("/"));
                segs.push(day_seg);
                segs.push(DateSegment::new_literal("/"));
                segs.push(year_seg);
            }
        }
        segs
    }

    /// Build time segments for the given hour cycle, granularity, and current time value.
    fn build_time_segments(
        hour_cycle: HourCycle,
        granularity: TimeGranularity,
        time: &Option<Time>,
    ) -> Vec<DateSegment> {
        let mut segs = Vec::new();

        let (h_min, h_max) = hour_cycle.display_hour_range();
        let mut hour_seg = DateSegment::new_numeric(
            DateSegmentKind::Hour, h_min as i32, h_max as i32, "hh",
        );
        if let Some(t) = time {
            let display = match hour_cycle {
                HourCycle::H12 => t.hour_12() as i32,
                HourCycle::H11 => (t.hour % 12) as i32,
                _              => t.hour as i32,
            };
            hour_seg.value = Some(display);
            hour_seg.text  = format!("{:02}", display);
        }
        segs.push(hour_seg);

        segs.push(DateSegment::new_literal(":"));

        let mut min_seg = DateSegment::new_numeric(DateSegmentKind::Minute, 0, 59, "mm");
        if let Some(t) = time {
            min_seg.value = Some(t.minute as i32);
            min_seg.text  = format!("{:02}", t.minute);
        }
        segs.push(min_seg);

        if granularity >= TimeGranularity::Second {
            segs.push(DateSegment::new_literal(":"));
            let mut sec_seg = DateSegment::new_numeric(DateSegmentKind::Second, 0, 59, "ss");
            if let Some(t) = time {
                sec_seg.value = Some(t.second as i32);
                sec_seg.text  = format!("{:02}", t.second);
            }
            segs.push(sec_seg);
        }

        if hour_cycle.has_day_period() {
            segs.push(DateSegment::new_literal("\u{00A0}")); // non-breaking space
            let mut period_seg = DateSegment {
                kind: DateSegmentKind::DayPeriod,
                value: None,
                min: 0,
                max: 1,
                text: String::new(),
                placeholder: "AM".to_string(),
                literal: None,
                is_editable: true,
            };
            if let Some(t) = time {
                let is_pm = t.is_pm();
                period_seg.value = Some(if is_pm { 1 } else { 0 });
                period_seg.text  = if is_pm { "PM".to_string() } else { "AM".to_string() };
            }
            segs.push(period_seg);
        }

        segs
    }
}

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

        let date_value = value.get().as_ref().map(|dt| dt.date.clone());
        let time_value = value.get().as_ref().map(|dt| dt.time);

        let resolved_cycle = match props.hour_cycle {
            HourCycle::Auto => locale.hour_cycle(),
            explicit => explicit,
        };

        let date_segments = Machine::build_date_segments(&locale, &date_value);
        let time_segments = Machine::build_time_segments(
            resolved_cycle, props.granularity, &time_value,
        );

        let ctx = Context {
            value,
            date_value,
            time_value,
            focused_segment: None,
            open: false,
            date_segments,
            time_segments,
            type_buffer: String::new(),
            locale,
            messages,
            granularity: props.granularity,
            hour_cycle: resolved_cycle,
            min_value: props.min_value.clone(),
            max_value: props.max_value.clone(),
            disabled: props.disabled,
            readonly: props.readonly,
            is_touched: false,
            required: props.required,
            name: props.name.clone(),
            is_rtl: props.is_rtl,
            ids: ComponentIds::from_id(&props.id),
        };

        (State::Idle, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if is_disabled(ctx) { return None; }

        match event {
            // ── Popover open / close ──────────────────────────────────────
            Event::Open => {
                if *state == State::Open || is_readonly(ctx) { return None; }
                Some(TransitionPlan::to(State::Open)
                    .apply(|ctx| {
                        ctx.open = true;
                    })
                    .with_effect(PendingEffect::new("focus-calendar", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        let grid_id = ctx.ids.part("calendar-grid");
                        platform.focus_element_by_id(&grid_id);
                        no_cleanup()
                    })))
            }

            Event::Close => {
                if *state != State::Open { return None; }
                Some(TransitionPlan::to(State::Focused)
                    .apply(|ctx| {
                        ctx.open = false;
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
                    State::Open => Self::transition(state, &Event::Close, ctx, props),
                    _           => Self::transition(state, &Event::Open, ctx, props),
                }
            }

            // ── Calendar date selection ───────────────────────────────────
            Event::CalendarSelectDate(date) => {
                if is_readonly(ctx) { return None; }
                let date = date.clone();
                Some(TransitionPlan::to(State::Focused)
                    .apply(move |ctx| {
                        ctx.date_value = Some(date.clone());
                        ctx.open = false;
                        ctx.is_touched = true;
                        // Update date segments to reflect the selected date.
                        ctx.date_segments = Machine::build_date_segments(
                            &ctx.locale, &Some(date),
                        );
                        Machine::maybe_publish(ctx);
                    })
                    .with_effect(PendingEffect::new("focus-first-time-segment", |ctx, _props, _send| {
                        // After calendar selection, focus the first editable time segment
                        // so the user can confirm or adjust the time.
                        if let Some(seg) = ctx.time_segments.iter().find(|s| s.is_editable) {
                            let platform = use_platform_effects();
                            let seg_id = ctx.ids.item("segment", &seg.kind.as_str());
                            platform.focus_element_by_id(&seg_id);
                        }
                        no_cleanup()
                    })))
            }

            Event::CalendarNext | Event::CalendarPrev => {
                // Delegated to the embedded Calendar machine via the adapter.
                // No state change in the DateTimePicker itself.
                None
            }

            // ── Segment focus management ─────────────────────────────────
            Event::FocusSegment(kind) => {
                let k = *kind;
                Some(TransitionPlan::to(State::Focused)
                    .apply(move |ctx| {
                        ctx.focused_segment = Some(k);
                        ctx.type_buffer.clear();
                    }))
            }

            Event::FocusNextSegment => {
                match ctx.focused_segment {
                    Some(cur) => {
                        let has_buffer = !ctx.type_buffer.is_empty();
                        let next = ctx.next_editable_after(cur);
                        Some(TransitionPlan::to(State::Focused)
                            .apply(move |ctx| {
                                if has_buffer {
                                    if let Ok(v) = ctx.type_buffer.parse::<i32>() {
                                        ctx.set_segment_value(cur, v);
                                        Machine::maybe_publish(ctx);
                                    }
                                    ctx.type_buffer.clear();
                                }
                                ctx.focused_segment = next;
                            }))
                    }
                    None => {
                        let first = ctx.first_editable()?;
                        Some(TransitionPlan::to(State::Focused)
                            .apply(move |ctx| {
                                ctx.focused_segment = Some(first);
                            }))
                    }
                }
            }

            Event::FocusPrevSegment => {
                let cur = ctx.focused_segment?;
                if !ctx.type_buffer.is_empty() {
                    return Some(TransitionPlan::context_only(|ctx| {
                        ctx.type_buffer.clear();
                    }));
                }
                match ctx.prev_editable_before(cur) {
                    Some(k) => Some(TransitionPlan::to(State::Focused)
                        .apply(move |ctx| {
                            ctx.focused_segment = Some(k);
                        })),
                    None => None,
                }
            }

            // ── Segment value editing ────────────────────────────────────
            Event::IncrementSegment { segment } => {
                if is_readonly(ctx) { return None; }
                let k = *segment;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.type_buffer.clear();
                    ctx.increment_segment(k);
                    ctx.is_touched = true;
                    Machine::maybe_publish(ctx);
                }))
            }

            Event::DecrementSegment { segment } => {
                if is_readonly(ctx) { return None; }
                let k = *segment;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.type_buffer.clear();
                    ctx.decrement_segment(k);
                    ctx.is_touched = true;
                    Machine::maybe_publish(ctx);
                }))
            }

            Event::TypeIntoSegment { segment, ch } => {
                if is_readonly(ctx) { return None; }
                let ch = *ch;
                let k = *segment;
                match k {
                    DateSegmentKind::DayPeriod => {
                        let lower = ch.to_ascii_lowercase();
                        if lower != 'a' && lower != 'p' { return None; }
                        Some(TransitionPlan::context_only(move |ctx| {
                            match lower {
                                'a' => ctx.set_segment_value(DateSegmentKind::DayPeriod, 0),
                                'p' => ctx.set_segment_value(DateSegmentKind::DayPeriod, 1),
                                _ => {}
                            }
                            ctx.is_touched = true;
                            Machine::maybe_publish(ctx);
                        }))
                    }
                    k2 if k2.is_numeric() => {
                        if !ch.is_ascii_digit() { return None; }
                        let mut new_buffer = ctx.type_buffer.clone();
                        new_buffer.push(ch);
                        let buffered: i32 = new_buffer.parse().unwrap_or(0);
                        let (seg_min, seg_max) = ctx.all_segments()
                            .find(|s| s.kind == k2)
                            .map(|s| (s.min, s.max))
                            .unwrap_or((0, 59));
                        let max_digits = digits_needed(seg_max);
                        let should_advance = new_buffer.len() >= max_digits
                            || buffered * 10 > seg_max;
                        let valid = buffered >= seg_min && buffered <= seg_max;
                        let next_seg = if should_advance { ctx.next_editable_after(k2) } else { None };

                        let mut plan = TransitionPlan::to(State::Focused)
                            .apply(move |ctx| {
                                ctx.type_buffer.push(ch);
                                if valid {
                                    ctx.set_segment_value(k2, buffered);
                                    ctx.is_touched = true;
                                    Machine::maybe_publish(ctx);
                                }
                                if should_advance {
                                    ctx.type_buffer.clear();
                                    if let Some(nk) = next_seg {
                                        ctx.focused_segment = Some(nk);
                                    }
                                }
                            });

                        if !should_advance {
                            plan = plan.with_effect(PendingEffect::new(
                                "type-buffer-commit",
                                move |_ctx, _props, send| {
                                    let send = send.clone();
                                    Box::new(move || {
                                        send(Event::TypeBufferCommit { segment: k2 });
                                    })
                                },
                            ));
                        }
                        Some(plan)
                    }
                    _ => None,
                }
            }

            Event::TypeBufferCommit { segment } => {
                let k = *segment;
                Some(TransitionPlan::context_only(move |ctx| {
                    if let Ok(v) = ctx.type_buffer.parse::<i32>() {
                        ctx.set_segment_value(k, v);
                        Machine::maybe_publish(ctx);
                    }
                    ctx.type_buffer.clear();
                }))
            }

            Event::SegmentChange { segment, value } => {
                if is_readonly(ctx) { return None; }
                let k = *segment;
                let v = *value;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.set_segment_value(k, v);
                    ctx.is_touched = true;
                    // Update the date/time sub-values.
                    if ctx.is_date_segment(k) {
                        ctx.date_value = ctx.assemble_date();
                    } else if ctx.is_time_segment(k) {
                        ctx.time_value = ctx.assemble_time();
                    }
                    Machine::maybe_publish(ctx);
                }))
            }

            Event::ValueCommit(dt) => {
                if is_readonly(ctx) { return None; }
                let dt = dt.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(dt.clone());
                    ctx.is_touched = true;
                    if let Some(ref dt) = dt {
                        ctx.date_value = Some(dt.date.clone());
                        ctx.time_value = Some(dt.time);
                        ctx.date_segments = Machine::build_date_segments(
                            &ctx.locale, &Some(dt.date.clone()),
                        );
                        ctx.time_segments = Machine::build_time_segments(
                            ctx.hour_cycle, ctx.granularity, &Some(dt.time),
                        );
                    }
                }))
            }

            Event::ClearSegment { segment } => {
                if is_readonly(ctx) { return None; }
                let kind = *segment;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.type_buffer.clear();
                    ctx.clear_segment_value(kind);
                    ctx.value.set(None);
                }))
            }

            Event::ClearAll => {
                if is_readonly(ctx) { return None; }
                Some(TransitionPlan::to(State::Idle)
                    .apply(|ctx| {
                        let date_editable: Vec<_> = ctx.date_segments.iter()
                            .filter(|s| s.is_editable).map(|s| s.kind).collect();
                        for k in date_editable { ctx.clear_segment_value(k); }
                        let time_editable: Vec<_> = ctx.time_segments.iter()
                            .filter(|s| s.is_editable).map(|s| s.kind).collect();
                        for k in time_editable { ctx.clear_segment_value(k); }
                        ctx.date_value = None;
                        ctx.time_value = None;
                        ctx.value.set(None);
                        ctx.type_buffer.clear();
                    }))
            }

            // ── Focus management ─────────────────────────────────────────
            Event::FocusIn => {
                if *state == State::Idle {
                    Some(TransitionPlan::to(State::Focused)
                        .apply(|_ctx| {}))
                } else {
                    None
                }
            }

            Event::FocusOut => {
                // Commit any pending type buffer, close popover if open.
                let has_buffer = !ctx.type_buffer.is_empty();
                let focused = ctx.focused_segment;
                let was_open = ctx.open;
                Some(TransitionPlan::to(State::Idle)
                    .apply(move |ctx| {
                        if has_buffer {
                            if let (Some(k), Ok(v)) = (focused, ctx.type_buffer.parse::<i32>()) {
                                ctx.set_segment_value(k, v);
                                Machine::maybe_publish(ctx);
                            }
                        }
                        ctx.focused_segment = None;
                        ctx.type_buffer.clear();
                        if was_open {
                            ctx.open = false;
                        }
                    }))
            }

            // ── Keyboard shortcuts ───────────────────────────────────────
            Event::KeyDown { key } => {
                match key {
                    KeyboardKey::Escape if *state == State::Open => {
                        Self::transition(state, &Event::Close, ctx, props)
                    }
                    KeyboardKey::ArrowDown if *state != State::Open => {
                        // Alt+ArrowDown opens the picker (modifier check at adapter level).
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
        if let (Some(date), Some(time)) = (&self.ctx.parsed_date, self.ctx.parsed_time.as_ref()) {
            let formatted = format!("{} {}", format_date(date, &self.ctx.format, &self.ctx.locale), format_time(time, &self.ctx.locale));
            attrs.set(HtmlAttr::Aria(AriaAttr::Description),
                (self.ctx.messages.selected_datetime_label)(&formatted, &self.ctx.locale));
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
        attrs.set(HtmlAttr::Id, self.ctx.ids.item("segment", kind.as_str()));
        attrs.set(HtmlAttr::Role, "spinbutton");
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs.set(HtmlAttr::InputMode, "numeric");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), kind.aria_label());

        // Find the segment in either date or time segments.
        let seg = self.ctx.date_segments.iter()
            .chain(self.ctx.time_segments.iter())
            .find(|s| s.kind == *kind);

        if let Some(seg) = seg {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), seg.min.to_string());
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), seg.max.to_string());
            if let Some(v) = &seg.value {
                attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), v.to_string());
                attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), &seg.text);
            }
        }

        let is_focused = self.ctx.focused_segment.as_ref() == Some(kind);
        if is_focused {
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
        attrs.set(HtmlAttr::TabIndex, "0");
        if self.ctx.disabled {
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
        // ISO 8601 datetime: YYYY-MM-DDTHH:MM:SS
        if let Some(dt) = &self.ctx.value.get() {
            let iso = format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
                dt.date.year, dt.date.month.get(), dt.date.day.get(),
                dt.time.hour, dt.time.minute, dt.time.second,
            );
            attrs.set(HtmlAttr::Value, iso);
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
    pub fn on_segment_keydown(&self, kind: DateSegmentKind, key: KeyboardKey) {
        match key {
            KeyboardKey::ArrowUp   => (self.send)(Event::IncrementSegment { segment: kind }),
            KeyboardKey::ArrowDown => (self.send)(Event::DecrementSegment { segment: kind }),
            KeyboardKey::ArrowLeft => (self.send)(Event::FocusPrevSegment),
            KeyboardKey::ArrowRight => (self.send)(Event::FocusNextSegment),
            KeyboardKey::Backspace | KeyboardKey::Delete => {
                (self.send)(Event::ClearSegment { segment: kind })
            }
            KeyboardKey::Char(ch) => {
                (self.send)(Event::TypeIntoSegment { segment: kind, ch })
            }
            KeyboardKey::Escape => (self.send)(Event::KeyDown { key }),
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
            id: format!("{}-calendar", self.ctx.ids.base_id),
            value: Some(self.ctx.date_value.clone()),
            default_value: None,
            min: self.ctx.min_value.as_ref().map(|dt| dt.date.clone()),
            max: self.ctx.max_value.as_ref().map(|dt| dt.date.clone()),
            disabled: self.ctx.disabled,
            readonly: self.ctx.readonly,
            is_rtl: self.ctx.is_rtl,
            visible_months: self.props.visible_months,
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

| Element            | Role/Attribute          | Details                                                                                |
| ------------------ | ----------------------- | -------------------------------------------------------------------------------------- |
| `Control`          | `role="group"`          | Groups all segments; `aria-labelledby` points to Label                                 |
| `DateSegmentGroup` | `role="group"`          | `aria-label="Date"`; groups date segments                                              |
| `TimeSegmentGroup` | `role="group"`          | `aria-label="Time"`; groups time segments                                              |
| `Segment`          | `role="spinbutton"`     | `aria-valuenow`, `aria-valuemin`, `aria-valuemax`, `aria-valuetext`, `aria-label`      |
| `Trigger`          | `<button>`              | `aria-label="Open date and time picker"`, `aria-expanded`, `aria-controls`             |
| `ClearTrigger`     | `<button>`              | `aria-label="Clear date and time"`                                                     |
| `Content`          | `role="dialog"`         | Contains the Calendar; announced when opened                                           |
| `ErrorMessage`     | `role="alert"`          | `aria-live="polite"`; announced immediately on render                                  |
| `Literal`          | `<span>`                | `aria-hidden="true"`; decorative separators                                            |
| `Separator`        | `<span>`                | `aria-hidden="true"`; decorative separator between date and time                       |
| `HiddenInput`      | `<input type="hidden">` | Carries the ISO 8601 value; `aria-describedby` chains Description and ErrorMessage IDs |

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
#[derive(Clone, Debug)]
pub struct Messages {
    /// Trigger button label (default: "Open date and time picker").
    pub trigger_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Clear button label (default: "Clear date and time").
    pub clear_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Date segment group label (default: "Date").
    pub date_group_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Time segment group label (default: "Time").
    pub time_group_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Announces the selected date-time (e.g., "Selected: March 15, 2025 2:30 PM").
    pub selected_datetime_label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            trigger_label: MessageFn::static_str("Open date and time picker"),
            clear_label: MessageFn::static_str("Clear date and time"),
            date_group_label: MessageFn::static_str("Date"),
            time_group_label: MessageFn::static_str("Time"),
            selected_datetime_label: MessageFn::new(|value, _locale| {
                format!("Selected: {}", value)
            }),
        }
    }
}

impl ComponentMessages for Messages {}
```

All user-visible text is provided via the `Messages` struct. Individual segment labels (e.g., "Year", "Hour", "AM/PM") are inherited from the `DateSegmentKind::aria_label()` method defined in date-field's segment types. Date segment order and separators are locale-dependent (see `Machine::build_date_segments()`). Time segment display uses the locale's preferred hour cycle when `HourCycle::Auto` is set.

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
