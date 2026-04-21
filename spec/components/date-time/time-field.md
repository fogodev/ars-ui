---
component: TimeField
category: date-time
tier: stateful
foundation_deps: [architecture, accessibility, i18n, interactions, forms]
shared_deps: [date-time-types]
related: [date-field, date-time-picker]
references:
  react-aria: TimeField
---

# TimeField

`TimeField` is the time-only counterpart to `DateField`. Its segments are `Hour`, `Minute`, optional `Second`, and `DayPeriod` (AM/PM) for 12-hour locales. It shares the same type-ahead, auto-advance, and spinbutton ARIA pattern as `DateField` but has time-specific constraints: hour cycle, 24 vs 12 hour display, and no calendar system dependency.

## 1. State Machine

### 1.1 States

```rust
/// States for the TimeField component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// The component is idle.
    Idle,
    /// The component is focused.
    Focused(DateSegmentKind),
}
```

### 1.2 Events

```rust
/// Events for the TimeField component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Focus moved to a segment.
    FocusSegment { kind: DateSegmentKind },
    /// Focus left the entire field group.
    BlurAll,
    /// ArrowUp on the focused segment: increment by 1, wrapping.
    IncrementSegment { kind: DateSegmentKind },
    /// ArrowDown on the focused segment: decrement by 1, wrapping.
    DecrementSegment { kind: DateSegmentKind },
    /// A printable character was typed while a numeric segment was focused.
    TypeIntoSegment { kind: DateSegmentKind, ch: char },
    /// The type-ahead buffer timer fired; commit whatever digits are buffered.
    TypeBufferCommit { kind: DateSegmentKind },
    /// Clear the value of a segment.
    ClearSegment { kind: DateSegmentKind },
    /// Clear the value of all segments.
    ClearAll,
    /// Set the value of the component.
    SetValue(Option<Time>),
    /// Focus the next segment.
    FocusNextSegment,
    /// Focus the previous segment.
    FocusPrevSegment,
}
```

### 1.3 Context

```rust
/// Context for the TimeField component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The current value of the component.
    pub value: Bindable<Option<Time>>,
    /// The segments of the component.
    pub segments: Vec<DateSegment>,
    /// The currently focused segment.
    pub focused_segment: Option<DateSegmentKind>,
    /// The type-ahead buffer.
    pub type_buffer: String,
    /// The locale of the component.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// Intl backend for locale-dependent formatting (day period labels, etc.).
    pub intl_backend: Arc<dyn IntlBackend>,
    /// The granularity of the component.
    pub granularity: TimeGranularity,
    /// The hour cycle of the component.
    pub hour_cycle: HourCycle,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is readonly.
    pub readonly: bool,
    /// The minimum value of the component.
    pub min_value: Option<Time>,
    /// The maximum value of the component.
    pub max_value: Option<Time>,
    /// Component IDs.
    pub ids: ComponentIds,
    /// When true, all numeric segments display with leading zeros (e.g., "03"
    /// instead of "3"). Defaults to false, which uses locale-aware formatting.
    pub force_leading_zeros: bool,
}

impl Context {
    /// Build segments from hour_cycle, granularity, and current value.
    ///
    /// When `self.force_leading_zeros` is true, numeric segments (hour,
    /// minute, second) are always zero-padded to 2 digits. When false,
    /// formatting uses locale-aware defaults.
    pub fn rebuild_segments(&mut self) {
        let cycle   = self.hour_cycle;
        let value   = self.value.get().clone();
        let pad     = self.force_leading_zeros;
        let mut segs = Vec::new();

        // ── Hour ─────────────────────────────────────────────────────────────
        let (h_min, h_max) = cycle.display_hour_range();
        let mut hour_seg = DateSegment::new_numeric(DateSegmentKind::Hour, h_min as i32, h_max as i32, "hh");
        if let Some(t) = &value {
            let display = match cycle {
                HourCycle::H12 => t.hour_12() as i32,
                HourCycle::H11 => (t.hour % 12) as i32,
                _              => t.hour as i32,
            };
            hour_seg.value = Some(display);
            hour_seg.text  = if pad { format!("{:02}", display) } else { format!("{}", display) };
        }
        segs.push(hour_seg);

        // ── Colon literal ─────────────────────────────────────────────────────
        segs.push(DateSegment::new_literal(":"));

        // ── Minute ───────────────────────────────────────────────────────────
        let mut min_seg = DateSegment::new_numeric(DateSegmentKind::Minute, 0, 59, "mm");
        if let Some(t) = &value {
            min_seg.value = Some(t.minute as i32);
            min_seg.text  = if pad { format!("{:02}", t.minute) } else { format!("{}", t.minute) };
        }
        segs.push(min_seg);

        // ── Second (optional) ────────────────────────────────────────────────
        if self.granularity >= TimeGranularity::Second {
            segs.push(DateSegment::new_literal(":"));
            let mut sec_seg = DateSegment::new_numeric(DateSegmentKind::Second, 0, 59, "ss");
            if let Some(t) = &value {
                sec_seg.value = Some(t.second as i32);
                sec_seg.text  = if pad { format!("{:02}", t.second) } else { format!("{}", t.second) };
            }
            segs.push(sec_seg);
        }

        // ── DayPeriod (12-hour only) ──────────────────────────────────────────
        if cycle.has_day_period() {
            segs.push(DateSegment::new_literal("\u{00A0}")); // non-breaking space
            let mut period_seg = DateSegment {
                kind: DateSegmentKind::DayPeriod,
                value: None,
                min: 0,
                max: 1,
                text: String::new(),
                placeholder: day_period_label(&*self.intl_backend, false, &self.locale),
                literal: None,
                is_editable: true,
            };
            if let Some(t) = &value {
                let is_pm = t.is_pm();
                period_seg.value = Some(if is_pm { 1 } else { 0 });
                period_seg.text  = day_period_label(&*self.intl_backend, is_pm, &self.locale);
            }
            segs.push(period_seg);
        }

        // When hide_time_zone is true, filter out TimeZoneName segments.
        // This is checked via the Props (accessed through the machine's init).
        // The adapter passes hide_time_zone into Context at init time.
        // if hide_time_zone { segs.retain(|s| s.kind != DateSegmentKind::TimeZoneName); }

        self.segments = segs;
    }

    /// Assemble a Time from current segment values.
    pub fn assemble_time(&self) -> Option<Time> {
        let cycle      = self.hour_cycle;
        let raw_hour   = self.get_seg(DateSegmentKind::Hour)? as u8;
        let minute     = self.get_seg(DateSegmentKind::Minute)? as u8;
        let second     = self.get_seg(DateSegmentKind::Second).unwrap_or(0) as u8;

        let hour = if cycle.has_day_period() {
            let is_pm = self.get_seg(DateSegmentKind::DayPeriod).unwrap_or(0) == 1;
            match cycle {
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

    /// Get the value of a segment.
    fn get_seg(&self, kind: DateSegmentKind) -> Option<i32> {
        self.segments.iter().find(|s| s.kind == kind)?.value
    }

    /// Get a mutable reference to a segment.
    pub fn segment_mut(&mut self, kind: DateSegmentKind) -> Option<&mut DateSegment> {
        self.segments.iter_mut().find(|s| s.kind == kind)
    }

    /// Set the value of a segment.
    ///
    /// When `self.force_leading_zeros` is true, numeric segments are always
    /// zero-padded to 2 digits (hour, minute, second). When false, formatting
    /// uses locale-aware defaults.
    pub fn set_segment_value(&mut self, kind: DateSegmentKind, raw: i32) {
        if let Some(seg) = self.segment_mut(kind) {
            let v = raw.clamp(seg.min, seg.max);
            seg.value = Some(v);
            seg.text  = match kind {
                DateSegmentKind::DayPeriod => day_period_label(&*self.intl_backend, v == 1, &self.locale),
                _ if self.force_leading_zeros => format!("{:02}", v),
                _ => format!("{}", v),
            };
        }
    }

    /// Clear the value of a segment.
    pub fn clear_segment_value(&mut self, kind: DateSegmentKind) {
        if let Some(seg) = self.segment_mut(kind) {
            seg.value = None;
            seg.text  = String::new();
        }
    }

    /// Increment the value of a segment.
    pub fn increment_segment(&mut self, kind: DateSegmentKind) {
        if let Some(seg) = self.segments.iter().find(|s| s.kind == kind).cloned() {
            let cur  = seg.value.unwrap_or(seg.min);
            let next = if cur >= seg.max { seg.min } else { cur + 1 };
            self.set_segment_value(kind, next);
        }
    }

    /// Decrement the value of a segment.
    pub fn decrement_segment(&mut self, kind: DateSegmentKind) {
        if let Some(seg) = self.segments.iter().find(|s| s.kind == kind).cloned() {
            let cur  = seg.value.unwrap_or(seg.max);
            let next = if cur <= seg.min { seg.max } else { cur - 1 };
            self.set_segment_value(kind, next);
        }
    }

    /// Get the first editable segment.
    pub fn first_editable(&self) -> Option<DateSegmentKind> {
        self.segments.iter().find(|s| s.is_editable).map(|s| s.kind)
    }

    /// Get the next editable segment after the given segment.
    pub fn next_editable_after(&self, kind: DateSegmentKind) -> Option<DateSegmentKind> {
        let idx = self.segments.iter().position(|s| s.kind == kind)?;
        self.segments[idx + 1..].iter().find(|s| s.is_editable).map(|s| s.kind)
    }

    /// Get the previous editable segment before the given segment.
    pub fn prev_editable_before(&self, kind: DateSegmentKind) -> Option<DateSegmentKind> {
        let idx = self.segments.iter().position(|s| s.kind == kind)?;
        self.segments[..idx].iter().rev().find(|s| s.is_editable).map(|s| s.kind)
    }

    /// Check if the component is complete.
    pub fn is_complete(&self) -> bool {
        self.segments.iter()
            .filter(|s| s.is_editable)
            .all(|s| s.value.is_some())
    }
}
```

### 1.4 Props

```rust
/// Props for the TimeField component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The ID of the component.
    pub id: String,
    /// The current value of the component.
    pub value: Option<Time>,
    /// The default value of the component.
    pub default_value: Option<Time>,
    /// The granularity of the component.
    pub granularity: TimeGranularity,
    /// Hour cycle override. When `Some`, overrides the locale's default hour cycle.
    /// `H12` forces 12-hour display with AM/PM; `H23` forces 24-hour display.
    /// When this field is `None`, the locale's preferred hour cycle is used
    /// (e.g., `H12` for `en-US`, `H23` for `de-DE`).
    pub hour_cycle: Option<HourCycle>,
    /// When `true`, the time zone segment (if present) is omitted from the rendered segments.
    /// Useful when the time zone is displayed elsewhere in the UI or is not relevant.
    /// Default: `false`.
    pub hide_time_zone: bool,
    /// Whether the component is disabled.
    pub disabled: bool,
    /// Whether the component is readonly.
    pub readonly: bool,
    /// Whether the component is required.
    pub required: bool,
    /// The minimum value of the component.
    pub min_value: Option<Time>,
    /// The maximum value of the component.
    pub max_value: Option<Time>,
    /// The label of the component.
    pub label: String,
    /// The ARIA label of the component.
    pub aria_label: Option<String>,
    /// The ARIA describedby of the component.
    pub aria_describedby: Option<String>,
    /// The description of the component.
    pub description: Option<String>,
    /// The error message of the component.
    pub error_message: Option<String>,
    /// Whether the component is invalid.
    pub invalid: bool,
    /// The name of the component.
    pub name: Option<String>,
    /// When true, all numeric segments display with leading zeros (e.g., "03"
    /// instead of "3"). Defaults to false, which uses locale-aware formatting.
    pub force_leading_zeros: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            granularity: TimeGranularity::Minute,
            hour_cycle: None,
            hide_time_zone: false,
            disabled: false,
            readonly: false,
            required: false,
            min_value: None,
            max_value: None,
            label: String::new(),
            aria_label: None,
            aria_describedby: None,
            description: None,
            error_message: None,
            invalid: false,
            name: None,
            force_leading_zeros: false,
        }
    }
}
```

### 1.5 Guards

```rust
fn is_disabled(ctx: &Context) -> bool {
    ctx.disabled
}

fn is_readonly(ctx: &Context) -> bool {
    ctx.readonly
}
```

### 1.6 Locale-Specific DayPeriod Parsing

The `day_period_from_input()` implementation for CJK locales is incomplete when only a single ambiguous character has been typed. For example, in Japanese typing only `'午'` does not distinguish between 午前 (AM) and 午後 (PM). The following disambiguation algorithm addresses this gap.

**Exhaustive CJK Day Period Lookup Table**:

| Locale                          | AM Label | PM Label | Ambiguous First Char                 | Disambiguating Char      |
| ------------------------------- | -------- | -------- | ------------------------------------ | ------------------------ |
| Japanese (`ja`)                 | 午前     | 午後     | `'午'`                               | `'前'` → AM, `'後'` → PM |
| Chinese Simplified (`zh-Hans`)  | 上午     | 下午     | — (unique: `'上'` → AM, `'下'` → PM) | —                        |
| Chinese Traditional (`zh-Hant`) | 上午     | 下午     | — (unique: `'上'` → AM, `'下'` → PM) | —                        |
| Korean (`ko`)                   | 오전     | 오후     | `'오'`                               | `'전'` → AM, `'후'` → PM |

**Disambiguation Algorithm**:

1. On first character input, check the locale's day period label table. If the character uniquely identifies a day period (e.g., Chinese `'上'` → AM), commit immediately.
2. If the first character is shared across AM and PM labels (e.g., Japanese `'午'`, Korean `'오'`), push it onto `ctx.type_buffer` and wait for the next character before resolving.
3. When the second character arrives, match the full two-character buffer against the locale's AM/PM labels and commit.
4. **Timeout fallback**: If only one ambiguous character has been typed and no further input arrives within 500ms (the `TypeBufferCommit` effect fires), use the current hour value as context to disambiguate — if the current hour is < 12, resolve to AM; otherwise resolve to PM.
5. **Abbreviated forms and tone mark variations**: Normalize input by stripping Unicode combining marks (NFD decomposition, remove `\p{M}` codepoints) before matching. This handles Korean Jamo composition variants and any accidental tone marks on CJK characters.

```rust
/// CJK day period label entry used for progressive disambiguation.
struct CjkDayPeriodEntry {
    am_label: &'static str,
    pm_label: &'static str,
}

/// Returns the CJK day period labels for supported locales, or None
/// for locales that use single-character (non-ambiguous) matching.
fn cjk_day_period_table(locale: &Locale) -> Option<CjkDayPeriodEntry> {
    match locale.language.as_str() {
        "ja" => Some(CjkDayPeriodEntry { am_label: "午前", pm_label: "午後" }),
        "ko" => Some(CjkDayPeriodEntry { am_label: "오전", pm_label: "오후" }),
        // zh-Hans and zh-Hant have unique first characters; no ambiguity.
        _ => None,
    }
}

/// Attempt to resolve a day period value from the type buffer contents
/// for CJK locales with ambiguous first characters.
///
/// Returns `Some(0)` for AM, `Some(1)` for PM, or `None` if the buffer
/// is still ambiguous and more input is needed.
fn day_period_from_cjk_buffer(
    buffer: &str,
    locale: &Locale,
    current_hour: Option<u8>,
) -> Option<i32> {
    let entry = match cjk_day_period_table(locale) {
        Some(e) => e,
        None => return None,
    };

    // Normalize: strip combining marks for tone-mark resilience.
    let normalized = buffer
        .nfd()
        .filter(|c| !c.is_combining_mark())
        .collect::<String>();

    // Full or prefix match against AM/PM labels.
    if entry.am_label.starts_with(&normalized) && !entry.pm_label.starts_with(&normalized) {
        return Some(0); // Unique AM match
    }
    if entry.pm_label.starts_with(&normalized) && !entry.am_label.starts_with(&normalized) {
        return Some(1); // Unique PM match
    }

    // Both labels still match the prefix — check if we have enough chars.
    if normalized.chars().count() >= 2 {
        // Two characters should always disambiguate in our supported locales.
        if entry.am_label.starts_with(&normalized) {
            return Some(0);
        }
        if entry.pm_label.starts_with(&normalized) {
            return Some(1);
        }
    }

    // Still ambiguous (single shared character). If this is a timeout
    // fallback, use the current hour as context.
    if let Some(hour) = current_hour {
        return Some(if hour < 12 { 0 } else { 1 });
    }

    None // Need more input
}
```

The `TypeIntoSegment` handler for `DateSegmentKind::DayPeriod` in the TimeField state machine MUST delegate to this logic for CJK locales instead of the current ASCII-only `'a'`/`'p'` check. When the buffer is ambiguous after the first character, the handler schedules a `TypeBufferCommit` effect (500ms) exactly as numeric segments do. On `TypeBufferCommit` for a `DayPeriod` segment, call `day_period_from_cjk_buffer` with `current_hour` set to the current hour segment value to apply the timeout fallback.

### 1.7 Timezone Handling

The `TimeZoneName` segment in `DateSegmentKind` is defined but timezone support is
**intentionally scoped out** of `TimeField` and `DatePicker` for v1. Rationale:

- Timezone selection requires a separate UI (dropdown or searchable combobox) that
  is orthogonal to the spinbutton-based segment editing model.
- Timezone-aware values require `DateTime<Tz>` semantics (offset + zone ID), which
  is a superset of the current `Time` / `CalendarDate` value types.
- DST transitions create ambiguous and non-existent local times that require
  disambiguation UI (e.g., "Did you mean 1:30 AM EST or 1:30 AM EDT?").

**v1 behavior:**

- `TimeZoneName` is retained in the `DateSegmentKind` enum for forward compatibility.
- If a component encounters a `TimeZoneName` segment, it renders as a **read-only
  display label** (e.g., "EST") and is skipped during keyboard navigation.
- The `Time` struct stores wall-clock time only (no offset).
- Form submission via hidden input uses `HH:MM:SS` (no timezone suffix).

#### 1.7.1 Optional Timezone Detection

When `infer_timezone: true` is set in Props, the adapter MAY detect the user's timezone automatically:

- Use `Intl.DateTimeFormat().resolvedOptions().timeZone` to obtain the IANA timezone identifier (e.g., `"America/New_York"`). Do **NOT** use the Geolocation API — it requires user permission, is asynchronous, and provides geographic coordinates rather than timezone identifiers.
- **Cache** the detected timezone per session (store in a module-level `OnceLock` or equivalent). Timezone detection is deterministic within a session and does not need to be repeated.
- **Fallback**: If `Intl.DateTimeFormat` is unavailable (e.g., SSR context), fall back to `"UTC"`.
- The detected timezone is used only for **display formatting** of the read-only `TimeZoneName` segment. It does not affect the stored `Time` value, which remains wall-clock time without offset.

#### 1.7.2 TimeField Semantic Clarification

**TimeField is for time-of-day entry, not instant-in-time.** The `Time` struct represents a wall-clock time (e.g., "2:30 PM") without any date or timezone context. It is NOT suitable for representing a specific moment in time (instant).

- **Internal storage**: Always ISO 8601 format (`HH:MM:SS` or `HH:MM:SS.sss`) for form submission and serialization.
- **Display**: Uses the locale's preferred hour cycle and numeral system. Timezone display name (if shown) is derived from the session's detected timezone or an explicitly provided timezone prop.
- **DST responsibility**: DST handling is the **application's responsibility**. TimeField does not validate whether a given wall-clock time is valid in a specific timezone on a specific date (e.g., 2:30 AM during spring-forward). Applications that need instant-in-time semantics should compose TimeField with a DateField and a timezone selector.

#### 1.7.3 Timezone Display

1. Default display uses locale-specific timezone name via ICU4X `TimeZoneFormatter` (e.g., "Eastern Standard Time" or "EST" depending on locale).
2. When timezone abbreviation is ambiguous (e.g., "CST" = US Central or China Standard), the formatter includes the offset (e.g., "CST (UTC-6)").
3. DST gap handling: if a user enters a time in a DST gap (e.g., 2:30 AM during spring-forward), the component adjusts to the next valid time and shows a warning via `aria-live`.

#### 1.7.4 DST Transition Disambiguation

When a TimeField is composed with a DateField and timezone context (i.e., the application is constructing an instant), the following disambiguation rules apply:

**Spring-Forward Gaps (invalid local times):**

- If the user enters a wall-clock time that does not exist due to a spring-forward transition (e.g., 2:30 AM when clocks jump from 2:00 AM → 3:00 AM), the component MUST adjust to the next valid local time (e.g., 3:00 AM).
- An `aria-live="polite"` announcement MUST be emitted: _"Time adjusted: 2:30 AM does not exist on this date, set to 3:00 AM"_.
- The adjusted time is committed to the internal value; the original invalid time is not stored.

**Fall-Back Overlaps (ambiguous local times):**

- If the user enters a wall-clock time that occurs twice due to a fall-back transition (e.g., 1:30 AM occurs in both EDT and EST), the component MUST resolve to the **first occurrence** (standard time / the earlier offset) by default.
- No user-facing ambiguity prompt is needed because the component stores instants internally — the resolved instant is unambiguous.
- Applications requiring the second occurrence (daylight time) can override via an `offset_disambiguation: OffsetDisambiguation` prop with values `FirstOccurrence` (default) and `SecondOccurrence`.

### 1.8 Full Machine Implementation

```rust
/// Machine for the TimeField component.
pub struct Machine;

impl Machine {
    fn maybe_publish(ctx: &mut Context) {
        if !ctx.is_complete() { return; }
        let Some(time) = ctx.assemble_time() else { return };
        let clamped = match (&ctx.min_value, &ctx.max_value) {
            (Some(min), _) if time < *min => *min,
            (_, Some(max)) if time > *max => *max,
            _ => time,
        };
        ctx.value.set(Some(clamped));
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
        let locale = env.locale.clone();
        let messages = messages.clone();
        let provider = env.intl_backend.clone();

        let resolved_cycle = props
            .hour_cycle
            .unwrap_or_else(|| locale.hour_cycle(&*provider));
        let mut ctx = Context {
            value: match &props.value {
                Some(v) => Bindable::controlled(*v),
                None    => Bindable::uncontrolled(props.default_value),
            },
            segments: Vec::new(),
            focused_segment: None,
            type_buffer: String::new(),
            locale,
            messages,
            provider,
            granularity: props.granularity,
            hour_cycle: resolved_cycle,
            disabled: props.disabled,
            readonly: props.readonly,
            min_value: props.min_value,
            max_value: props.max_value,
            ids: ComponentIds::from_id(&props.id),
            force_leading_zeros: props.force_leading_zeros,
        };
        ctx.rebuild_segments();
        // When hide_time_zone is true, remove any TimeZoneName segments.
        if props.hide_time_zone {
            ctx.segments.retain(|s| s.kind != DateSegmentKind::TimeZoneName);
        }
        (State::Idle, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled { return None; }

        match event {
            Event::FocusSegment { kind } => {
                if !kind.is_editable() { return None; }
                let k = *kind;
                Some(TransitionPlan::to(State::Focused(k))
                    .apply(move |ctx| {
                        ctx.focused_segment = Some(k);
                        ctx.type_buffer.clear();
                    }))
            }

            Event::BlurAll => {
                let has_buffer = !ctx.type_buffer.is_empty();
                let focused = ctx.focused_segment;
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
                    }))
            }

            Event::FocusNextSegment => {
                match state {
                    State::Focused(cur) => {
                        let cur = *cur;
                        let has_buffer = !ctx.type_buffer.is_empty();
                        let next = ctx.next_editable_after(cur);
                        let target = match next {
                            Some(k) => State::Focused(k),
                            None    => State::Idle,
                        };
                        Some(TransitionPlan::to(target)
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
                    _ => {
                        let first = ctx.first_editable()?;
                        Some(TransitionPlan::to(State::Focused(first))
                            .apply(move |ctx| {
                                ctx.focused_segment = Some(first);
                            }))
                    }
                }
            }

            Event::FocusPrevSegment => {
                if let State::Focused(cur) = state {
                    let cur = *cur;
                    if !ctx.type_buffer.is_empty() {
                        return Some(TransitionPlan::to(state.clone())
                            .apply(|ctx| { ctx.type_buffer.clear(); }));
                    }
                    match ctx.prev_editable_before(cur) {
                        Some(k) => Some(TransitionPlan::to(State::Focused(k))
                            .apply(move |ctx| { ctx.focused_segment = Some(k); })),
                        None => None,
                    }
                } else { None }
            }

            Event::IncrementSegment { kind } => {
                if ctx.readonly { return None; }
                let k = *kind;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.type_buffer.clear();
                    ctx.increment_segment(k);
                    Machine::maybe_publish(ctx);
                }))
            }

            Event::DecrementSegment { kind } => {
                if ctx.readonly { return None; }
                let k = *kind;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.type_buffer.clear();
                    ctx.decrement_segment(k);
                    Machine::maybe_publish(ctx);
                }))
            }

            Event::TypeIntoSegment { kind, ch } => {
                if ctx.readonly { return None; }
                let ch = *ch;
                let k = *kind;
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
                            Machine::maybe_publish(ctx);
                        }))
                    }
                    k2 if k2.is_numeric() => {
                        if !ch.is_ascii_digit() { return None; }
                        let mut new_buffer = ctx.type_buffer.clone();
                        new_buffer.push(ch);
                        let buffered: i32 = new_buffer.parse().unwrap_or(0);
                        let (seg_min, seg_max) = ctx.segments.iter()
                            .find(|s| s.kind == k2)
                            .map(|s| (s.min, s.max))
                            .unwrap_or((0, 59));
                        let max_digits = digits_needed(seg_max);
                        let should_advance = new_buffer.len() >= max_digits
                            || buffered * 10 > seg_max;
                        let valid = buffered >= seg_min && buffered <= seg_max;
                        let next_seg = if should_advance { ctx.next_editable_after(k2) } else { None };

                        let target = if should_advance {
                            match next_seg {
                                Some(nk) => State::Focused(nk),
                                None => state.clone(),
                            }
                        } else {
                            state.clone()
                        };

                        let mut plan = TransitionPlan::to(target)
                            .apply(move |ctx| {
                                ctx.type_buffer.push(ch);
                                if valid {
                                    ctx.set_segment_value(k2, buffered);
                                    Machine::maybe_publish(ctx);
                                }
                                if should_advance {
                                    ctx.type_buffer.clear();
                                    if let Some(nk) = next_seg {
                                        ctx.focused_segment = Some(nk);
                                    }
                                }
                            });

                        // Schedule a delayed commit effect when the buffer is not
                        // immediately consumed. Replaces the old TimerId-based timer
                        // with a PendingEffect that the adapter converts into a
                        // setTimeout / spawn_local timer.
                        if !should_advance {
                            plan = plan.with_effect(PendingEffect::new(
                                "type-buffer-commit",
                                move |_ctx, _props, send| {
                                    let send = send.clone();
                                    Box::new(move || {
                                        send(Event::TypeBufferCommit { kind: k2 });
                                    })
                                },
                            ));
                        }
                        Some(plan)
                    }
                    _ => None,
                }
            }

            Event::TypeBufferCommit { kind } => {
                let k = *kind;
                Some(TransitionPlan::context_only(move |ctx| {
                    if let Ok(v) = ctx.type_buffer.parse::<i32>() {
                        ctx.set_segment_value(k, v);
                        Machine::maybe_publish(ctx);
                    }
                    ctx.type_buffer.clear();
                }))
            }

            Event::ClearSegment { kind } => {
                if ctx.readonly { return None; }
                let kind = *kind;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.type_buffer.clear();
                    ctx.clear_segment_value(kind);
                    ctx.value.set(None);
                }))
            }

            Event::ClearAll => {
                if ctx.readonly { return None; }
                Some(TransitionPlan::to(State::Idle)
                    .apply(|ctx| {
                        let editable = ctx.segments.iter()
                            .filter(|s| s.is_editable)
                            .map(|s| s.kind)
                            .collect::<Vec<_>>();
                        for k in editable { ctx.clear_segment_value(k); }
                        ctx.value.set(None);
                        ctx.type_buffer.clear();
                    }))
            }

            Event::SetValue(v) => {
                let v = v.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(v);
                    ctx.rebuild_segments();
                }))
            }
        }
    }
}
```

### 1.9 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "time-field"]
pub enum Part {
    Root,
    Label,
    FieldGroup,
    Segment { kind: DateSegmentKind },
    Literal { index: usize },
    Description,
    ErrorMessage,
    HiddenInput,
}

/// API for the TimeField component.
pub struct Api<'a> {
    /// The state of the time field.
    state: &'a State,
    /// The context of the time field.
    ctx: &'a Context,
    /// The props of the time field.
    props: &'a Props,
    /// The send function for the time field.
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
        attrs.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Idle => "idle",
            State::Focused(_) => "focused",
        });
        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }
        if self.props.invalid {
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
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
        attrs.set(HtmlAttr::For, self.ctx.ids.part("field-group"));
        attrs
    }

    /// Attributes for the field group element.
    pub fn field_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::FieldGroup.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("field-group"));
        attrs.set(HtmlAttr::Role, "group");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));
        if self.props.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }
        if self.props.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }
        // Wire aria-describedby to description and error message parts.
        let mut described_by = Vec::new();
        if self.props.description.is_some() {
            described_by.push(self.ctx.ids.part("description"));
        }
        if self.props.invalid && self.props.error_message.is_some() {
            described_by.push(self.ctx.ids.part("error-message"));
        }
        if !described_by.is_empty() {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), described_by.join(" "));
        }
        attrs
    }

    /// Attributes for a segment element.
    pub fn segment_attrs(&self, kind: &DateSegmentKind) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Segment { kind: kind.clone() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "spinbutton");
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs.set(HtmlAttr::InputMode, "numeric");
        let segment_label = match kind {
            DateSegmentKind::Hour      => (self.ctx.messages.hour_label)(&self.ctx.locale),
            DateSegmentKind::Minute    => (self.ctx.messages.minute_label)(&self.ctx.locale),
            DateSegmentKind::Second    => (self.ctx.messages.second_label)(&self.ctx.locale),
            DateSegmentKind::DayPeriod => (self.ctx.messages.day_period_label)(&self.ctx.locale),
            other                      => format!("{:?}", other),
        };
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), segment_label);

        // Value range for spinbutton semantics.
        if let Some(seg) = self.ctx.segments.iter().find(|s| s.kind == *kind) {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), seg.min.to_string());
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), seg.max.to_string());
            if let Some(v) = &seg.value {
                attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), v.to_string());
                attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), &seg.text);
            }
        }

        // Focused segment tracking.
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

    /// Attributes for a literal separator element.
    pub fn literal_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Literal { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
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
        if let Some(name) = &self.props.name {
            attrs.set(HtmlAttr::Name, name);
        }
        if let Some(value) = &self.ctx.value.get() {
            attrs.set(HtmlAttr::Value, value.to_iso8601());
        }
        attrs
    }

    // ── Event handlers ───────────────────────────────────────────────

    /// Handle keydown on a segment.
    pub fn on_segment_keydown(&self, kind: DateSegmentKind, key: KeyboardKey) {
        match key {
            KeyboardKey::ArrowUp => (self.send)(Event::IncrementSegment { kind }),
            KeyboardKey::ArrowDown => (self.send)(Event::DecrementSegment { kind }),
            KeyboardKey::ArrowLeft => (self.send)(Event::FocusPrevSegment),
            KeyboardKey::ArrowRight => (self.send)(Event::FocusNextSegment),
            KeyboardKey::Backspace | KeyboardKey::Delete => {
                (self.send)(Event::ClearSegment { kind })
            }
            KeyboardKey::Char(ch) => {
                (self.send)(Event::TypeIntoSegment { kind, ch })
            }
            _ => {}
        }
    }

    /// Handle focus on a segment.
    pub fn on_segment_focus(&self, kind: DateSegmentKind) {
        (self.send)(Event::FocusSegment { kind });
    }

    /// Handle focus leaving the field group.
    pub fn on_field_group_focusout(&self) {
        (self.send)(Event::BlurAll);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::FieldGroup => self.field_group_attrs(),
            Part::Segment { kind } => self.segment_attrs(&kind),
            Part::Literal { index } => self.literal_attrs(index),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::HiddenInput => self.hidden_input_attrs(),
        }
    }
}
```

## 2. Anatomy

| Part           | HTML Element                | Purpose                                       |
| -------------- | --------------------------- | --------------------------------------------- |
| `Root`         | `<div>`                     | Outermost container                           |
| `Label`        | `<label>`                   | Visible label                                 |
| `FieldGroup`   | `<div role="group">`        | Groups all segments; receives accessible name |
| `Segment`      | `<div role="spinbutton">`   | Hour, Minute, optional Second, DayPeriod      |
| `Literal`      | `<span aria-hidden="true">` | Colon separators, non-breaking space          |
| `Description`  | `<div>`                     | Help text                                     |
| `ErrorMessage` | `<div role="alert">`        | Validation error                              |
| `HiddenInput`  | `<input type="hidden">`     | ISO 8601 time string for form submission      |

```text
TimeField (en-US, H12, Minute granularity)
└── Root
    ├── Label
    ├── FieldGroup (role="group"  aria-label="Time")
    │   ├── Segment (Hour)        role="spinbutton"  aria-valuemin="1"  aria-valuemax="12"
    │   ├── Literal ":"           aria-hidden="true"
    │   ├── Segment (Minute)      role="spinbutton"  aria-valuemin="0"  aria-valuemax="59"
    │   ├── Literal "\u{00A0}"    aria-hidden="true"
    │   └── Segment (DayPeriod)   role="spinbutton"  aria-label="AM/PM"
    ├── Description
    ├── ErrorMessage
    └── HiddenInput               value="14:30" (ISO 24h for form submission)

TimeField (de-DE, H23, Second granularity)
└── Root
    ├── Label
    ├── FieldGroup (role="group"  aria-label="Uhrzeit")
    │   ├── Segment (Hour)        role="spinbutton"  aria-valuemin="0"  aria-valuemax="23"
    │   ├── Literal ":"
    │   ├── Segment (Minute)      role="spinbutton"
    │   ├── Literal ":"
    │   └── Segment (Second)      role="spinbutton"
    └── HiddenInput
```

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

Follows the same spinbutton pattern as DateField with these time-specific notes:

- `aria-valuemin` on Hour: 0 (H23/H11) or 1 (H12/H24).
- `aria-valuemax` on Hour: 11 (H11), 12 (H12), 23 (H23), 24 (H24).
- `aria-valuetext` on DayPeriod: locale-aware AM/PM string.

| Element      | Role / Attribute                   | Value                                  |
| ------------ | ---------------------------------- | -------------------------------------- |
| `Root`       | `data-ars-state`                   | `"idle"` or `"focused"`                |
| `FieldGroup` | `role="group"`                     | Groups all segments                    |
| `FieldGroup` | `aria-labelledby`                  | Points to Label ID                     |
| `FieldGroup` | `aria-required`                    | Set when `required` is true            |
| `FieldGroup` | `aria-invalid`                     | Set when `invalid` is true             |
| `FieldGroup` | `aria-describedby`                 | Points to Description and ErrorMessage |
| `Segment`    | `role="spinbutton"`                | Each editable segment                  |
| `Segment`    | `aria-valuemin` / `aria-valuemax`  | Segment numeric range                  |
| `Segment`    | `aria-valuenow` / `aria-valuetext` | Current value and display text         |
| `Segment`    | `aria-label`                       | Segment-specific label (e.g., "Hour")  |
| `Segment`    | `aria-disabled` / `aria-readonly`  | Mirrors component state                |
| `Literal`    | `aria-hidden="true"`               | Hidden from screen readers             |

### 3.2 Keyboard Interaction

| Key                    | Action                                         |
| ---------------------- | ---------------------------------------------- |
| `ArrowUp`              | Increment focused segment by 1, wrapping       |
| `ArrowDown`            | Decrement focused segment by 1, wrapping       |
| `ArrowLeft`            | Move focus to previous segment                 |
| `ArrowRight`           | Move focus to next segment                     |
| `0-9`                  | Type-ahead into numeric segments               |
| `Backspace` / `Delete` | Clear focused segment value                    |
| `Tab`                  | Move focus to next segment or out of component |

**DayPeriod Keyboard Interaction:**

| Key         | Action          |
| ----------- | --------------- |
| `a`         | Set AM          |
| `p`         | Set PM          |
| `ArrowUp`   | Toggle AM -> PM |
| `ArrowDown` | Toggle PM -> AM |

## 4. Internationalization

| Aspect         | Details                                                                     |
| -------------- | --------------------------------------------------------------------------- |
| Hour cycle     | H12 (en-US, zh-TW, ko-KR), H23 (de-DE, fr-FR, en-GB, ru-RU)                 |
| AM/PM labels   | "AM/PM" (en), "午前/午後" (ja/zh), "오전/오후" (ko), "ص/م" (ar)             |
| Hidden input   | Always outputs ISO 8601 24-hour format (HH:MM:SS) for form submission       |
| Numeral system | Locale numeral digits for displayed text (same ICU4X pipeline as DateField) |

### 4.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Label for the hour segment.
    pub hour_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the minute segment.
    pub minute_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the second segment.
    pub second_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Label for the day period segment (AM/PM).
    pub day_period_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            hour_label: MessageFn::static_str("Hour"),
            minute_label: MessageFn::static_str("Minute"),
            second_label: MessageFn::static_str("Second"),
            day_period_label: MessageFn::static_str("AM/PM"),
        }
    }
}

impl ComponentMessages for Messages {}
```

## 5. Form Integration

- Hidden `<input>` submits the current time value in ISO 8601 format (`HH:MM:SS`).
- `name` attribute is set from Props.
- Reset restores `default_value`.
- Validation states reflected via `data-ars-invalid` on Root and `aria-invalid` on FieldGroup.
- `aria-describedby` wires to Description and ErrorMessage parts.
- `aria-required` set when `required` is true.
- Disabled/readonly propagation from form context per `07-forms.md` §15.

## 6. Library Parity

> Compared against: React Aria (`TimeField`).

### 6.1 Props

| Feature             | ars-ui                         | React Aria                 | Notes                                   |
| ------------------- | ------------------------------ | -------------------------- | --------------------------------------- |
| Controlled value    | `value` (Time)                 | `value` (TimeValue)        | Equivalent                              |
| Default value       | `default_value`                | `defaultValue`             | Equivalent                              |
| Min/max             | `min_value`, `max_value`       | `minValue`, `maxValue`     | Equivalent                              |
| Granularity         | `granularity`                  | `granularity`              | Equivalent                              |
| Hour cycle          | `hour_cycle`                   | `hourCycle`                | Equivalent                              |
| Hide time zone      | `hide_time_zone`               | `hideTimeZone`             | Equivalent                              |
| Force leading zeros | `force_leading_zeros`          | `shouldForceLeadingZeros`  | Equivalent                              |
| Placeholder value   | --                             | `placeholderValue`         | React Aria uses a TimeValue placeholder |
| Disabled            | `disabled`                     | `isDisabled`               | Equivalent                              |
| Read-only           | `readonly`                     | `isReadOnly`               | Equivalent                              |
| Required            | `required`                     | `isRequired`               | Equivalent                              |
| Invalid             | `invalid`                      | `isInvalid`                | Equivalent                              |
| Auto-focus          | --                             | `autoFocus`                | Adapter concern                         |
| Name                | `name`                         | `name`                     | Equivalent                              |
| Validate            | --                             | `validate`                 | React Aria custom validation            |
| Validation behavior | --                             | `validationBehavior`       | React Aria native/aria mode             |
| Form                | --                             | `form`                     | React Aria associated form ID           |
| Auto-complete       | --                             | `autoComplete`             | React Aria auto-fill hint               |
| Label/aria-label    | `label`, `aria_label`          | Label sub-component        | Equivalent                              |
| Description/error   | `description`, `error_message` | `Text`, `FieldError` slots | Equivalent                              |

**Gaps:**

- `placeholderValue`: React Aria allows a TimeValue to define placeholder format. ars-ui derives this from the `granularity` and `hour_cycle` props, which is equivalent.
- `validate`/`validationBehavior`: Form-framework-specific. ars-ui uses `invalid` + `error_message`.

None worth adopting.

### 6.2 Anatomy

| Part          | ars-ui         | React Aria                     | Notes                   |
| ------------- | -------------- | ------------------------------ | ----------------------- |
| Root          | `Root`         | `TimeField`                    | Equivalent              |
| Label         | `Label`        | `Label`                        | Equivalent              |
| Field group   | `FieldGroup`   | `DateInput`                    | Equivalent              |
| Segment       | `Segment`      | `DateSegment`                  | Equivalent              |
| Literal       | `Literal`      | `DateSegment` (type="literal") | Equivalent              |
| Description   | `Description`  | `Text` (slot)                  | Equivalent              |
| Error message | `ErrorMessage` | `FieldError`                   | Equivalent              |
| Hidden input  | `HiddenInput`  | --                             | ars-ui form integration |

**Gaps:** None.

### 6.3 Events

| Callback     | ars-ui                     | React Aria             | Notes                             |
| ------------ | -------------------------- | ---------------------- | --------------------------------- |
| Value change | `SetValue` / segment edits | `onChange`             | Equivalent                        |
| Focus        | `FocusSegment`             | `onFocus`              | Equivalent                        |
| Blur         | `BlurAll`                  | `onBlur`               | Equivalent                        |
| Focus change | --                         | `onFocusChange`        | React Aria boolean focus callback |
| Key events   | Segment keydown handlers   | `onKeyDown`, `onKeyUp` | Equivalent                        |

**Gaps:** None.

### 6.4 Features

| Feature                           | ars-ui | React Aria |
| --------------------------------- | ------ | ---------- |
| Segmented spinbutton input        | Yes    | Yes        |
| Hour/minute/second granularity    | Yes    | Yes        |
| 12-hour / 24-hour cycle           | Yes    | Yes        |
| AM/PM segment                     | Yes    | Yes        |
| CJK day period support            | Yes    | Yes        |
| Type-ahead numeric entry          | Yes    | Yes        |
| Auto-advance on complete          | Yes    | Yes        |
| Increment/decrement with wrapping | Yes    | Yes        |
| Hide time zone                    | Yes    | Yes        |
| Force leading zeros               | Yes    | Yes        |
| Hidden form input (ISO)           | Yes    | No         |

**Gaps:** None. ars-ui matches React Aria and adds hidden form input.

### 6.5 Summary

- **Overall:** Full parity.
- **Divergences:** React Aria uses `DateInput` + `DateSegment` components shared with DateField; ars-ui has its own `FieldGroup` + `Segment` parts. ars-ui adds CJK day period disambiguation and hidden form input.
- **Recommended additions:** None.
