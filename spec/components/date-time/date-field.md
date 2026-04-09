---
component: DateField
category: date-time
tier: stateful
foundation_deps: [architecture, accessibility, i18n, interactions, forms]
shared_deps: [date-time-types]
related: [date-picker, date-range-field, time-field]
references:
    react-aria: DateField
---

# DateField

`DateField` is a segmented date input where each field (month, day, year, era, etc.) is individually editable via keyboard. It is the foundational input component and is embedded inside `DatePicker`, `DateRangePicker`, and `DateRangeField`.

Unlike a plain `<input type="date">`, `DateField` exposes fine-grained control over every segment, supports arbitrary calendar systems, locale-aware segment ordering, accessible spin-button semantics per segment, and integrates with ars-ui's `Bindable<T>` for controlled/uncontrolled usage.

## 1. State Machine

### 1.1 States

```rust
// ars-core/src/components/date_field/machine.rs

use crate::{bindable::Bindable, machine::Machine};
use ars_core::{TransitionPlan, PendingEffect, AttrMap};
use ars_i18n::calendar::{types::*, segment_order::*};
use super::segment::*;

/// States for the DateField component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// No segment has focus; the field as a whole is unfocused.
    Idle,
    /// A specific segment has keyboard focus.
    Focused(DateSegmentKind),
}
```

### 1.2 Events

```rust
/// Events for the DateField component.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// User focused a segment (click or tab).
    FocusSegment(DateSegmentKind),
    /// Focus left the entire field group.
    BlurAll,
    /// ArrowUp on the focused segment: increment by 1, wrapping.
    IncrementSegment(DateSegmentKind),
    /// ArrowDown on the focused segment: decrement by 1, wrapping.
    DecrementSegment(DateSegmentKind),
    /// A printable character was typed while a numeric segment was focused.
    TypeIntoSegment(DateSegmentKind, char),
    /// The type-ahead buffer timer fired; commit whatever digits are buffered.
    TypeBufferCommit(DateSegmentKind),
    /// Backspace or Delete: clear the focused segment.
    ClearSegment(DateSegmentKind),
    /// Escape or programmatic clear: reset the entire field.
    ClearAll,
    /// Programmatic value update.
    SetValue(Option<CalendarDate>),
    /// Tab or ArrowRight: advance focus to next editable segment.
    FocusNextSegment,
    /// Shift+Tab or ArrowLeft: move focus to previous editable segment.
    FocusPrevSegment,
}
```

### 1.3 Context

```rust
/// Context for the DateField component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Controlled/uncontrolled date value.
    pub value: Bindable<Option<CalendarDate>>,
    /// Segments in display order (rebuilt on value/locale/granularity change).
    pub segments: Vec<DateSegment>,
    /// The segment currently holding keyboard focus.
    pub focused_segment: Option<DateSegmentKind>,
    /// Accumulated digit characters for the focused numeric segment.
    /// Typing "1" then "2" into Month produces "12" -> month 12.
    pub type_buffer: String,
    /// The locale.
    pub locale: Locale,
    /// ICU data provider for locale-dependent formatting.
    pub provider: ArsRc<dyn IcuProvider>,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// The calendar system.
    pub calendar: CalendarSystem,
    /// The granularity.
    pub granularity: DateGranularity,
    /// Whether the field is disabled.
    pub disabled: bool,
    /// Whether the field is readonly.
    pub readonly: bool,
    /// The minimum value.
    pub min_value: Option<CalendarDate>,
    /// The maximum value.
    pub max_value: Option<CalendarDate>,
    /// Component id for aria- attribute correlation.
    pub id: String,
    /// Derived element IDs (root, label, description, error, etc.).
    pub ids: ComponentIds,
    /// Whether the field is in an invalid/error state.
    pub invalid: bool,
    /// The `id` of the field-level error message element. When `invalid` is
    /// `true`, every editable segment's `aria-describedby` points to this ID
    /// so that the error is announced **once per field**, not per segment.
    /// Computed as `ids.part("error-message")` during init.
    pub error_message_id: String,
    /// When true, all numeric segments display with leading zeros (e.g., "03"
    /// instead of "3"). Defaults to false, which uses locale-aware formatting.
    pub force_leading_zeros: bool,
}

impl Context {
    /// Recompute segments from current value + locale + granularity.
    pub fn rebuild_segments(&mut self) {
        let kinds = segments_for_locale(&self.locale, self.granularity, self.calendar);
        let separators = LocaleSeparators::for_locale(&self.locale);
        let value = self.value.get().clone();
        self.segments = build_segments(
            &kinds,
            value.as_ref(),
            &self.locale,
            self.calendar,
            &separators,
            self.force_leading_zeros,
        );
    }

    /// Get the index of a segment.
    pub fn segment_index(&self, kind: DateSegmentKind) -> Option<usize> {
        self.segments.iter().position(|s| s.kind == kind)
    }

    /// Get the first editable segment.
    pub fn first_editable(&self) -> Option<DateSegmentKind> {
        self.segments.iter().find(|s| s.is_editable).map(|s| s.kind)
    }

    /// Get the next editable segment after a given segment.
    pub fn next_editable_after(&self, kind: DateSegmentKind) -> Option<DateSegmentKind> {
        let idx = self.segment_index(kind)?;
        self.segments[idx + 1..].iter().find(|s| s.is_editable).map(|s| s.kind)
    }

    /// Get the previous editable segment before a given segment.
    pub fn prev_editable_before(&self, kind: DateSegmentKind) -> Option<DateSegmentKind> {
        let idx = self.segment_index(kind)?;
        self.segments[..idx].iter().rev().find(|s| s.is_editable).map(|s| s.kind)
    }

    /// Check if the field is complete.
    pub fn is_complete(&self) -> bool {
        self.segments.iter()
            .filter(|s| s.is_editable && s.kind != DateSegmentKind::Literal)
            .all(|s| s.value.is_some())
    }

    /// Assemble a CalendarDate from current segment values.
    pub fn assemble_date(&self) -> Option<CalendarDate> {
        let year  = self.get_segment_value(DateSegmentKind::Year)?;
        let month = self.get_segment_value(DateSegmentKind::Month).unwrap_or(1) as u8;
        let day   = self.get_segment_value(DateSegmentKind::Day).unwrap_or(1) as u8;
        Some(CalendarDate::new_gregorian(
            year as i32,
            NonZero::new(month).expect("parsed month is 1-based"),
            NonZero::new(day).expect("parsed day is 1-based"),
        ))
    }

    /// Get the value of a segment.
    pub fn get_segment_value(&self, kind: DateSegmentKind) -> Option<i32> {
        self.segments.iter().find(|s| s.kind == kind)?.value
    }

    /// Get a mutable reference to a segment.
    pub fn segment_mut(&mut self, kind: DateSegmentKind) -> Option<&mut DateSegment> {
        self.segments.iter_mut().find(|s| s.kind == kind)
    }

    /// Set the value of a segment.
    ///
    /// When `self.force_leading_zeros` is true, numeric segments are always
    /// zero-padded to their maximum display width (month/day: 2, year: 4,
    /// hour/minute/second: 2). When false, padding delegates to
    /// `format_segment_digits` which uses locale-aware formatting (some
    /// locales like `ja-JP` omit leading zeros by default).
    pub fn set_segment_value(&mut self, kind: DateSegmentKind, raw: i32) {
        if let Some(seg) = self.segment_mut(kind) {
            let v = raw.clamp(seg.min, seg.max);
            seg.value = Some(v);
            seg.text = match kind {
                DateSegmentKind::DayPeriod => day_period_label(&*self.provider, v == 1, &self.locale),
                _ if self.force_leading_zeros => {
                    let width = match kind {
                        DateSegmentKind::Year => 4,
                        _ => 2, // Month, Day, Hour, Minute, Second
                    };
                    format!("{:0>width$}", v, width = width)
                }
                DateSegmentKind::Year => format_segment_digits(&*self.provider, v, 4, &self.locale),
                _ => format_segment_digits(&*self.provider, v, 2, &self.locale),
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
            let next = if seg.value.unwrap_or(seg.min) >= seg.max {
                seg.min
            } else {
                seg.value.unwrap_or(seg.min) + 1
            };
            self.set_segment_value(kind, next);
        }
    }

    /// Decrement the value of a segment.
    pub fn decrement_segment(&mut self, kind: DateSegmentKind) {
        if let Some(seg) = self.segments.iter().find(|s| s.kind == kind).cloned() {
            let next = if seg.value.unwrap_or(seg.max) <= seg.min {
                seg.max
            } else {
                seg.value.unwrap_or(seg.max) - 1
            };
            self.set_segment_value(kind, next);
        }
    }
}
```

### 1.4 Props

```rust
/// Props for the DateField component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The ID of the field.
    pub id: String,
    /// The value of the field.
    pub value: Option<CalendarDate>,
    /// The default value of the field.
    pub default_value: Option<CalendarDate>,
    /// The calendar system of the field.
    pub calendar: CalendarSystem,
    /// The granularity of the field.
    pub granularity: DateGranularity,
    /// The minimum value of the field.
    pub min_value: Option<CalendarDate>,
    /// The maximum value of the field.
    pub max_value: Option<CalendarDate>,
    /// Whether the field is disabled.
    pub disabled: bool,
    /// Whether the field is readonly.
    pub readonly: bool,
    /// Whether the field is required.
    pub required: bool,
    /// Whether the field should auto-focus.
    pub auto_focus: bool,
    /// The label of the field.
    pub label: String,
    /// The ARIA label of the field.
    pub aria_label: Option<String>,
    /// The ARIA labelledby of the field.
    pub aria_labelledby: Option<String>,
    /// The ARIA describedby of the field.
    pub aria_describedby: Option<String>,
    /// The description of the field.
    pub description: Option<String>,
    /// The error message of the field.
    pub error_message: Option<String>,
    /// Whether the field is invalid.
    pub invalid: bool,
    /// The name of the field.
    pub name: Option<String>,
    /// Optional override for segment display order. When `None`, the order is
    /// derived from the locale's date format pattern.
    pub segment_order: Option<Vec<DateSegmentKind>>,
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
            calendar: CalendarSystem::Gregorian,
            granularity: DateGranularity::Day,
            min_value: None,
            max_value: None,
            disabled: false,
            readonly: false,
            required: false,
            auto_focus: false,
            label: String::new(),
            aria_label: None,
            aria_labelledby: None,
            aria_describedby: None,
            description: None,
            error_message: None,
            invalid: false,
            name: None,
            segment_order: None,
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

### 1.6 Date Segment Types

```rust
// ars-core/src/components/date_field/segment.rs

/// The logical kind of a date or time segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DateSegmentKind {
    /// The year segment.
    Year,
    /// The month segment.
    Month,
    /// The day segment.
    Day,
    /// The hour segment.
    Hour,
    /// The minute segment.
    Minute,
    /// The second segment.
    Second,
    /// AM/PM indicator.
    DayPeriod,
    /// The weekday segment.
    /// Display only; not editable in standard DateField
    Weekday,
    /// The era segment.
    /// Japanese calendar eras: Reiwa, Heisei, Showa, etc.
    Era,
    /// Separator: "/" "-" "." ":" " " "年" "月" "日"
    Literal,
    /// The time zone name segment.
    /// Display only, e.g. "EST"
    TimeZoneName,
}

impl DateSegmentKind {
    /// Checks if the segment is editable.
    pub fn is_editable(&self) -> bool {
        !matches!(
            self,
            DateSegmentKind::Literal
            | DateSegmentKind::Weekday
            | DateSegmentKind::TimeZoneName
        )
    }

    /// Checks if the segment is numeric.
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            DateSegmentKind::Year
            | DateSegmentKind::Month
            | DateSegmentKind::Day
            | DateSegmentKind::Hour
            | DateSegmentKind::Minute
            | DateSegmentKind::Second
        )
    }

    /// ARIA label for this segment, read from the Messages struct.
    pub fn aria_label(&self, messages: &Messages, locale: &Locale) -> String {
        match self {
            DateSegmentKind::Year        => (messages.year_label)(locale),
            DateSegmentKind::Month       => (messages.month_label)(locale),
            DateSegmentKind::Day         => (messages.day_label)(locale),
            DateSegmentKind::Hour        => (messages.hour_label)(locale),
            DateSegmentKind::Minute      => (messages.minute_label)(locale),
            DateSegmentKind::Second      => (messages.second_label)(locale),
            DateSegmentKind::DayPeriod   => (messages.day_period_label)(locale),
            DateSegmentKind::Era         => (messages.era_label)(locale),
            DateSegmentKind::Weekday     => (messages.weekday_label)(locale),
            DateSegmentKind::Literal     => String::new(),
            DateSegmentKind::TimeZoneName => (messages.timezone_label)(locale),
        }
    }
}

/// A single segment within a DateField or TimeField.
#[derive(Clone, Debug, PartialEq)]
pub struct DateSegment {
    /// The kind of segment.
    pub kind: DateSegmentKind,
    /// Current numeric value; None if the user has not yet entered a value.
    pub value: Option<i32>,
    /// Minimum valid value (1 for months, 0 for H23 hours, etc.).
    pub min: i32,
    /// Maximum valid value (12 for months, 23 for H23 hours, etc.).
    pub max: i32,
    /// Formatted display text (locale-aware). Empty when value is None.
    pub text: String,
    /// Placeholder shown when value is None: "yyyy", "mm", "dd", "hh".
    pub placeholder: String,
    /// For Literal segments: the literal character(s).
    pub literal: Option<String>,
    /// Whether this segment accepts keyboard input.
    pub is_editable: bool,
}

impl DateSegment {
    /// Creates a new numeric segment.
    pub fn new_numeric(kind: DateSegmentKind, min: i32, max: i32, placeholder: &str) -> Self {
        Self {
            kind,
            value: None,
            min,
            max,
            text: String::new(),
            placeholder: placeholder.to_string(),
            literal: None,
            is_editable: true,
        }
    }

    /// Creates a new literal segment.
    pub fn new_literal(ch: &str) -> Self {
        Self {
            kind: DateSegmentKind::Literal,
            value: None,
            min: 0,
            max: 0,
            text: ch.to_string(),
            placeholder: ch.to_string(),
            literal: Some(ch.to_string()),
            is_editable: false,
        }
    }

    /// Text to render: formatted value if set, otherwise the placeholder.
    pub fn display_text(&self) -> &str {
        if self.value.is_some() && !self.text.is_empty() {
            &self.text
        } else {
            &self.placeholder
        }
    }

    /// aria-valuetext: human-readable string (e.g., "March" instead of "3").
    pub fn aria_value_text(&self, provider: &dyn IcuProvider, locale: &Locale) -> Option<String> {
        let v = self.value?;
        match self.kind {
            DateSegmentKind::Month => Some(month_long_name(provider, v as u8, locale)),
            DateSegmentKind::DayPeriod => {
                Some(day_period_label(provider, v == 1, locale))
            }
            _ => Some(v.to_string()),
        }
    }
}

/// Format a numeric value with locale-appropriate numerals and zero-padding.
///
/// Delegates to `IcuProvider::format_segment_digits()`.
///
/// **Locale-aware digit formatting.** Some locales use native digit systems:
/// - Arabic (ar): Arabic-Indic ٠١٢٣٤٥٦٧٨٩
/// - Persian/Dari (fa): Extended Arabic-Indic ۰۱۲۳۴۵۶۷۸۹
/// - Bengali (bn): ০১২৩৪৫৬৭৮৯
/// - Myanmar (my): ၀၁၂၃၄၅၆၇၈၉
/// - Most others: Western Arabic 0123456789
///
/// Production (`Icu4xProvider`): uses ICU4X `DecimalFormatter` with the
/// locale's default numbering system for automatic native digit substitution.
/// Tests (`StubIcuProvider`): returns zero-padded Western Arabic digits.
pub fn format_segment_digits(provider: &dyn IcuProvider, value: u32, min_digits: NonZero<u8>, locale: &Locale) -> String {
    provider.format_segment_digits(value, min_digits, locale)
}

/// Returns the full month name for the given locale.
///
/// Delegates to `IcuProvider::month_long_name()`.
/// Production (`Icu4xProvider`): uses ICU4X `DateSymbols::month_names(FieldLength::Wide)`
/// for comprehensive locale coverage. Examples: en->"January", fr->"janvier",
/// ar->"يناير", ja->"1月", he->"תשרי" (Hebrew calendar).
/// Tests (`StubIcuProvider`): returns English month names.
pub fn month_long_name(provider: &dyn IcuProvider, month: u8, locale: &Locale) -> String {
    provider.month_long_name(month, locale)
}

/// Map typed character(s) to a day period value (0=AM, 1=PM) for the given locale.
/// Returns `None` if the input doesn't match any day period key.
///
/// Delegates to `IcuProvider::day_period_from_input()`.
/// Production (`Icu4xProvider`): matches against locale-specific AM/PM strings
/// from ICU4X `DayPeriodNames`.
///
/// ### CJK Locale Handling
///
/// CJK locales use multi-character day period labels where the first character
/// alone is ambiguous or shared between AM and PM:
///
/// | Locale | AM        | PM        | Match Strategy                         |
/// |--------|-----------|-----------|----------------------------------------|
/// | `ja`   | 午前      | 午後      | First char '午' is shared; match on second char: '前'->AM, '後'->PM |
/// | `zh`   | 上午      | 下午      | First char differs: '上'->AM, '下'->PM  |
/// | `ko`   | 오전      | 오후      | First char '오' is shared; match on second char: '전'->AM, '후'->PM |
/// | `ar`   | ص         | م         | Single char: 'ص'->AM, 'م'->PM           |
/// | `en`   | AM        | PM        | Single char: 'a'->AM, 'p'->PM           |
///
/// For locales where the first character is shared (ja, ko), the type-ahead
/// buffer accumulates characters and matches against the full prefix. The
/// `type_buffer` in `Context` is used (same as numeric segments) -- the segment
/// commits once a unique match is found.
///
/// ### Single-Char to Multi-Char Transition Logic
///
/// DayPeriod CJK matching uses a progressive disambiguation strategy:
///
/// 1. **Unique first character** -- If the first typed character uniquely identifies
///    a day period label, the segment resolves immediately and commits. Example:
///    in English, `'A'` -> AM, `'P'` -> PM; in Chinese, `'上'` -> 上午 (AM),
///    `'下'` -> 下午 (PM).
///
/// 2. **Ambiguous first character** -- If the first character is shared by multiple
///    labels, the segment enters buffer accumulation mode. The character is pushed
///    onto `ctx.type_buffer` and the segment waits for additional input. Example:
///    in Japanese, `'午'` is shared by 午前 (AM) and 午後 (PM); in Korean, `'오'`
///    is shared by 오전 (AM) and 오후 (PM).
///
/// 3. **Second character resolves** -- When the next character arrives, the full
///    buffer is passed to `day_period_from_buffer(provider, &buffer, locale)`,
///    which matches against the locale's AM/PM labels and commits. Example:
///    buffer `"午前"` -> AM, buffer `"午後"` -> PM.
///
/// 4. **Timeout with ambiguous buffer** -- If the `TYPE_BUFFER_TIMEOUT` (same as
///    numeric segments, typically 1000ms) expires while the buffer contains only
///    the ambiguous first character, the buffer is cleared with no commit. The
///    segment remains in its previous state.
///
/// **Japanese example walkthrough:**
/// - User types `'午'` -> buffer = `"午"`, ambiguous (午前 / 午後), wait.
/// - User types `'前'` within timeout -> buffer = `"午前"`,
///   `day_period_from_buffer()` returns `Some(0)` -> commit AM.
/// - Alternatively, timeout fires with buffer = `"午"` -> clear buffer, no commit.
///
/// ### IME Composition Integration
///
/// When `ctx.is_composing` is `true` (set by the adapter's `compositionstart`
/// / `compositionend` event handlers), the DayPeriod segment MUST NOT attempt
/// character-by-character matching. Instead, it waits for the `compositionend`
/// event, then matches the composed string against the locale's AM/PM labels.
/// This prevents premature matching on intermediate IME candidates.
pub fn day_period_from_char(provider: &dyn IcuProvider, ch: char, locale: &Locale) -> Option<i32> {
    provider.day_period_from_char(ch, locale).map(|is_pm| if is_pm { 1 } else { 0 })
}

/// Extended day period matching for multi-character input (CJK locales).
/// Called when `type_buffer` contains more than one character.
/// Returns `Some(0)` for AM, `Some(1)` for PM, `None` if no match.
pub fn day_period_from_buffer(provider: &dyn IcuProvider, buffer: &str, locale: &Locale) -> Option<i32> {
    provider.day_period_from_buffer(buffer, locale).map(|is_pm| if is_pm { 1 } else { 0 })
}

/// Locale-aware AM/PM label.
///
/// Delegates to `IcuProvider::day_period_label()`.
/// Production (`Icu4xProvider`): uses ICU4X `DayPeriodNames` with
/// `FieldLength::Abbreviated` for comprehensive locale coverage.
/// Examples: en->"AM"/"PM", ja->"午前"/"午後", ko->"오전"/"오후", ar->"ص"/"م".
/// Tests (`StubIcuProvider`): returns English "AM"/"PM".
pub fn day_period_label(provider: &dyn IcuProvider, is_pm: bool, locale: &Locale) -> String {
    provider.day_period_label(is_pm, locale)
}
```

### 1.7 Locale-Driven Segment Ordering

DateField segment order MUST be derived from the resolved locale's date pattern, not hardcoded per region. The resolution algorithm:

1. **Check locale calendar extension** (`-u-ca-`): If present (e.g., `th-TH-u-ca-buddhist`), use the calendar-specific pattern from ICU4X data.
2. **Check language + region**: Look up the standard date pattern for the locale (e.g., `en-US` -> `MM/dd/yyyy`, `de-DE` -> `dd.MM.yyyy`, `ja-JP` -> `yyyy/M/d`).
3. **Fallback to language only**: If the exact region is not available (e.g., `en-XX`), fall back to the language's default pattern (e.g., `en` -> `MM/dd/yyyy`).
4. **Final fallback**: Gregorian calendar with ISO 8601 order (`yyyy-MM-dd`).

**Locale-Specific Patterns**:

| Locale        | Pattern       | Segment Order               | Calendar        | Notes                            |
| ------------- | ------------- | --------------------------- | --------------- | -------------------------------- |
| `en-US`       | `MM/dd/yyyy`  | Month -> Day -> Year        | Gregorian       | US convention                    |
| `en-GB`       | `dd/MM/yyyy`  | Day -> Month -> Year        | Gregorian       | UK/EU convention                 |
| `de-DE`       | `dd.MM.yyyy`  | Day -> Month -> Year        | Gregorian       | Dot separators                   |
| `ja-JP`       | `yyyy/M/d`    | Year -> Month -> Day        | Gregorian       | No leading zeros                 |
| `ja-JP` (era) | `Gy年M月d日`  | Era -> Year -> Month -> Day | Japanese        | Era-prefixed with Kanji suffixes |
| `th-TH`       | `d/M/yyyy`    | Day -> Month -> Year        | Buddhist (BE)   | Year = Gregorian + 543           |
| `zh-Hans`     | `yyyy/M/d`    | Year -> Month -> Day        | Gregorian       | Simplified Chinese               |
| `zh-Hant`     | `yyyy/M/d`    | Year -> Month -> Day        | Gregorian       | Traditional Chinese              |
| `ar-SA`       | `d/M/yyyy`    | Day -> Month -> Year        | Islamic (Hijri) | RTL display, LTR digit order     |
| `ko-KR`       | `yyyy. M. d.` | Year -> Month -> Day        | Gregorian       | Dot-space separators             |

**Segment Reordering Algorithm**: When constructing the DateField UI:

1. Parse the locale's date pattern to extract segment types and literal separators
2. Create `DateSegment` instances in pattern order (not a fixed Month-Day-Year order)
3. Render segments left-to-right in the extracted order (RTL handled by CSS `direction`)
4. Tab navigation follows the visual (rendered) order

**Locale-Aware Segment Navigation**: Segment order derives from the locale's
`DateTimePatternGenerator` skeleton. ArrowRight/ArrowLeft navigate in **visual order**
(the order segments appear on screen), which matches the locale's date pattern order.

- In **LTR** locales: ArrowRight moves to the next segment in visual order (e.g., Month -> Day -> Year in en-US).
- In **RTL** locales: ArrowRight moves to the logically **earlier** segment because the visual right side contains earlier segments in RTL date formats (e.g., in `ar-SA` with pattern `d/M/yyyy`, ArrowRight moves from Month toward Day because Day is visually to the right).
- ArrowLeft is always the inverse of ArrowRight.

```rust
/// Returns the segment types in visual (rendering) order for the given locale.
/// This order is used for ArrowLeft/ArrowRight keyboard navigation.
/// The order is derived from the locale's DateTimePatternGenerator skeleton,
/// not hardcoded per region.
///
/// Example returns:
///   en-US -> [Month, Day, Year]
///   de-DE -> [Day, Month, Year]
///   ja-JP -> [Year, Month, Day]
///   ar-SA -> [Day, Month, Year] (visual order; RTL rendering handled by CSS)
pub fn segment_order(locale: &Locale) -> Vec<DateSegmentKind> {
    // Implementation: parse locale's date pattern skeleton via ICU4X
    // DateTimePatternGenerator and extract segment kinds in pattern order.
    // Falls back to ISO 8601 [Year, Month, Day] if pattern unavailable.
    todo!()
}
```

**Calendar System Impact**: When a non-Gregorian calendar is active (via locale extension or `calendar_system` prop), additional segments may appear (e.g., Era segment for Japanese calendar) and month values may differ (e.g., month 13 in Hebrew leap year). The segment ordering is derived from the calendar-specific pattern, not the Gregorian pattern.

### 1.8 Calendar-System Segment Mapping

The `segments_for_calendar` method returns the appropriate segment kinds and order for a given calendar system. The adapter uses this to render the correct number and sequence of editable segments. The calendar system is determined by the locale (e.g., `ja-JP-u-ca-japanese`) or an explicit `calendar` prop.

```rust
// ars-i18n/src/calendar/segment_order.rs

/// Returns the default segment order for a given calendar system.
/// The adapter calls this when no locale-specific override is available,
/// or to validate that a locale's pattern matches the calendar's requirements.
pub fn segments_for_calendar(calendar: CalendarSystem) -> Vec<DateSegmentKind> {
    match calendar {
        // Gregorian (default): Month, Day, Year (US) or Day, Month, Year (EU)
        // -- actual order is locale-dependent; this returns the canonical segments.
        CalendarSystem::Gregorian => vec![
            DateSegmentKind::Month,
            DateSegmentKind::Day,
            DateSegmentKind::Year,
        ],
        // Japanese: Era is required (e.g., 令和5年3月8日)
        CalendarSystem::Japanese => vec![
            DateSegmentKind::Era,
            DateSegmentKind::Year,
            DateSegmentKind::Month,
            DateSegmentKind::Day,
        ],
        // Buddhist: Common Thai format is Day/Month/Year (with BE year)
        CalendarSystem::Buddhist => vec![
            DateSegmentKind::Day,
            DateSegmentKind::Month,
            DateSegmentKind::Year,
        ],
        // Other calendar systems follow their CLDR-defined segment patterns.
        // The ICU4X pattern generator provides the authoritative order.
        _ => vec![
            DateSegmentKind::Year,
            DateSegmentKind::Month,
            DateSegmentKind::Day,
        ],
    }
}
```

The `segments_for_locale` function takes precedence when a full locale is available, as it accounts for regional formatting preferences within a calendar system. `segments_for_calendar` serves as the fallback and is also used to determine which segment _kinds_ are required (e.g., ensuring the Era segment is always present for Japanese calendar regardless of locale variant).

### 1.9 Month Name Segment Parsing

When parsing abbreviated month names typed into a month segment:

1. **Prefix Matching**: Typing "Ja" in English matches "January" (not "June" or "July") because it's a unique prefix. "Ju" is ambiguous -- the segment waits for a third character ("Jun" vs "Jul") before committing.
2. **Disambiguation Timeout**: If no further input arrives within 500ms after an ambiguous prefix, the first alphabetical match is selected (e.g., "Ju" -> "June"). The user can continue typing to override.
3. **Locale-Specific Abbreviations**: Month abbreviation patterns are locale-dependent. In French, "j" is ambiguous across "janvier", "juin", "juillet". The parser uses `Intl.DateTimeFormat` month names for the active locale to build the prefix tree.
4. **Calendar System**: Non-Gregorian calendars (e.g., Hebrew, Islamic) have different month names and counts. The parser adapts to the active calendar system's month list.

### 1.10 Controlled Value Sync

When a DatePicker or TimePicker is in controlled mode (`value` prop is bound) and the parent updates the value while the user is actively editing a segment:

1. **Parent Value is Authoritative**: The parent's prop update overwrites any in-progress segment edit immediately. The segment display updates to reflect the new controlled value.
2. **Cursor Position Reset**: The cursor position resets to the start of the affected segment, allowing the user to begin editing from a known state.
3. **Synchronous Sync**: The value synchronization happens synchronously within the same microtask as the prop change event -- there is no intermediate frame where stale segment data is visible.

**Priority Rules**:

The rules above describe the simple case but leave a race condition unspecified: what happens when a parent prop update arrives while the user has an active `type_buffer` (mid-keystroke) in a focused segment? The following priority rules resolve this.

**Problem**: DateField and TimeField allow controlled per-segment values. When the parent updates the controlled `value` prop during active segment editing (focus + non-empty `type_buffer`), the immediate overwrite described above would discard partially typed input, causing a jarring experience.

1. **Active editing defers prop sync**: If a segment currently has focus AND `type_buffer` is non-empty, the incoming parent prop update is stored as a pending value but NOT applied to the segment display. The segment continues to show the user's in-progress input.
2. **Segment blur applies pending update**: When the actively edited segment loses focus (blur or navigation to another segment via Tab/Arrow), any pending parent prop update is applied immediately before the blur completes. The segment display updates to the controlled value.
3. **No active segment -- immediate update**: If no segment has focus (the field is in `Idle` state), parent prop updates are applied to all segment displays immediately, exactly as described in the base rules above.
4. **`on_change` only on user edits**: The `on_change` callback MUST fire only after user-initiated edits (keystrokes, increment/decrement, click). Prop synchronization from the parent MUST NOT emit `on_change`, preventing infinite update loops in two-way binding scenarios.

**Cursor Position Preservation**: When a deferred prop update is finally applied on segment blur, the cursor position within the target segment is preserved at its current offset rather than reset to the start. This avoids disorienting the user if they immediately re-focus the same segment.

```rust
/// Guard logic for controlled prop sync. Called from the `sync_props`
/// handler whenever the parent provides a new controlled value.
fn apply_controlled_value_update(ctx: &mut Context, new_value: Option<Date>) {
    // Check whether any segment is actively being edited.
    let has_active_edit = ctx.focused_segment.is_some()
        && !ctx.type_buffer.is_empty();

    if has_active_edit {
        // Rule 1: Defer -- store the pending value but do not touch segments.
        ctx.pending_controlled_value = Some(new_value);
        return;
    }

    // Rule 3: No active edit -- apply immediately to all segments.
    ctx.value.set(new_value);
    ctx.rebuild_segments();
    // Do NOT emit on_change (Rule 4).
}

/// Called when the focused segment loses focus (blur or navigation).
/// Flushes any deferred controlled value that arrived during editing.
fn flush_pending_controlled_value(ctx: &mut Context) {
    if let Some(pending) = ctx.pending_controlled_value.take() {
        // Rule 2: Apply the deferred value now that editing is complete.
        ctx.value.set(pending);
        ctx.rebuild_segments();
        // Do NOT emit on_change (Rule 4).
    }
}
```

The `pending_controlled_value` field MUST be added to the DateField and TimeField `Context` structs as `Option<Option<Date>>` (or the corresponding `Option<Option<Time>>` for TimeField). It defaults to `None` and is cleared after each flush.

**Locale-Driven Segment Ordering for Controlled Value Sync**:

The order of date segments (M/D/Y vs D.M.Y vs Y-M-D) is determined entirely by the locale. In production this is resolved by parsing ICU4X DateTimeFormatter skeleton patterns for the locale; the function below encodes a representative lookup table covering the most common locales.

```rust
// ars-i18n/src/calendar/segment_order.rs

/// Granularity of a date field: how many segments to include.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DateGranularity {
    /// The year segment.
    Year,
    /// The month segment.
    Month,
    /// The day segment.
    Day,
    /// The hour segment.
    Hour,
    /// The minute segment.
    Minute,
    /// The second segment.
    Second,
}

/// Granularity of a time-only field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimeGranularity {
    /// The hour segment.
    Hour,
    /// The minute segment.
    Minute,
    /// The second segment.
    Second,
    /// The millisecond segment.
    Millisecond,
}

/// Produce the ordered list of segment kinds for a date field in the given locale.
///
/// Segment ordering is determined by ICU4X `DateTimePatternGenerator` for any CLDR-supported
/// locale. The `segments_for_locale(locale)` function queries the pattern generator to derive
/// the correct field order (year/month/day, separators, etc.) rather than maintaining a
/// hardcoded mapping of locale -> segment order.
///
/// ### Locale Fallback Chain
///
/// When ICU4X does not have a pattern for the exact locale tag, the following
/// fallback chain is applied (matching ICU4X `LocaleFallbacker` behavior):
///
/// 1. **Exact locale tag** -- e.g., `zh-Hant-TW`
/// 2. **Language + script** -- e.g., `zh-Hant` (dropping region)
/// 3. **Language only** -- e.g., `zh`
/// 4. **Gregorian default** -- ISO Y-M-D order
///
/// This ensures that `zh-Hans` (Simplified Chinese, Y/M/D) and `zh-Hant`
/// (Traditional Chinese, Y/M/D) resolve correctly despite having the same
/// language code, and that script variants like `sr-Latn` vs `sr-Cyrl` are
/// handled properly.
///
/// ### Locale-Specific Test Coverage
///
/// The following locales MUST be covered by unit tests for segment order correctness:
///
/// | Locale    | Expected Order | Notes |
/// |-----------|----------------|-------|
/// | `zh-Hans` | Y/M/D          | Simplified Chinese |
/// | `zh-Hant` | Y/M/D          | Traditional Chinese |
/// | `ja-JP`   | [Era] Y年M月D日 | Japanese with era for Japanese calendar |
/// | `he-IL`   | D/M/Y          | Hebrew; RTL |
/// | `ar-SA`   | D/M/Y          | Arabic; RTL |
/// | `fa-IR`   | Y/M/D          | Persian; RTL with Solar Hijri calendar |
/// | `th-TH`   | D/M/Y          | Thailand; Buddhist Era year display |
/// | `ko-KR`   | Y년 M월 D일    | Korean with ideographic suffixes |
///
/// Examples:
/// - `en-US`, Day, Gregorian   -> [Month, Literal, Day, Literal, Year]
/// - `de-DE`, Day, Gregorian   -> [Day, Literal, Month, Literal, Year]
/// - `ja-JP`, Day, Japanese    -> [Era, Year, Literal, Month, Literal, Day, Literal]
/// - `zh-CN`, Day, Gregorian   -> [Year, Literal, Month, Literal, Day]
/// - `ar-SA`, Day, Gregorian   -> [Day, Literal, Month, Literal, Year]
pub fn segments_for_locale(
    locale: &Locale,
    granularity: DateGranularity,
    calendar: CalendarSystem,
) -> Vec<DateSegmentKind> {
    let requires_era = matches!(calendar, CalendarSystem::Japanese);

    // Language-prefix matching ensures that locale tags without a region
    // (e.g., "de", "fr") and uncommon region variants (e.g., "en-ZA") get
    // appropriate segment orders rather than falling through to the ISO default.
    let locale_str = locale.as_str();
    let lang = locale.language();

    let base: Vec<DateSegmentKind> = if locale_str == "en-US" || locale_str == "en-PH" || locale_str == "en-CA" || locale_str == "es-MX" {
        // United States / Canada / Philippines: M/D/Y
        vec![
            DateSegmentKind::Month,
            DateSegmentKind::Literal,
            DateSegmentKind::Day,
            DateSegmentKind::Literal,
            DateSegmentKind::Year,
        ]
    } else if lang == "ja" {
        // Japanese: [Era] Y年M月D日
        let mut segs = Vec::new();
        if requires_era { segs.push(DateSegmentKind::Era); }
        segs.extend([
            DateSegmentKind::Year,
            DateSegmentKind::Literal,
            DateSegmentKind::Month,
            DateSegmentKind::Literal,
            DateSegmentKind::Day,
            DateSegmentKind::Literal,
        ]);
        return trim_to_granularity(segs, granularity);
    } else if lang == "zh" || lang == "ko" || lang == "sv" || lang == "fi"
           || lang == "hu" || lang == "lt" || lang == "lv" || lang == "et" {
        // Chinese, Korean, Scandinavian, Baltic, Hungary: Y/M/D or Y.M.D
        vec![
            DateSegmentKind::Year,
            DateSegmentKind::Literal,
            DateSegmentKind::Month,
            DateSegmentKind::Literal,
            DateSegmentKind::Day,
        ]
    } else if lang == "ar" || lang == "fa" || lang == "ur" {
        // Arabic, Farsi, Urdu: D/M/Y (RTL handled by CSS direction)
        vec![
            DateSegmentKind::Day,
            DateSegmentKind::Literal,
            DateSegmentKind::Month,
            DateSegmentKind::Literal,
            DateSegmentKind::Year,
        ]
    } else if lang == "en" || lang == "de" || lang == "nl" || lang == "fr"
           || lang == "it" || lang == "es" || lang == "pt" || lang == "pl"
           || lang == "ru" || lang == "uk" || lang == "el" || lang == "tr"
           || lang == "ro" || lang == "cs" || lang == "sk" || lang == "hr"
           || lang == "bg" || lang == "id" || lang == "ms" || lang == "vi" {
        // Most of Europe, Latin America, and remaining English regions: D/M/Y or D.M.Y
        vec![
            DateSegmentKind::Day,
            DateSegmentKind::Literal,
            DateSegmentKind::Month,
            DateSegmentKind::Literal,
            DateSegmentKind::Year,
        ]
    } else {
        // Default: ISO Y-M-D
        vec![
            DateSegmentKind::Year,
            DateSegmentKind::Literal,
            DateSegmentKind::Month,
            DateSegmentKind::Literal,
            DateSegmentKind::Day,
        ]
    };

    trim_to_granularity(base, granularity)
}

fn trim_to_granularity(
    mut segs: Vec<DateSegmentKind>,
    granularity: DateGranularity,
) -> Vec<DateSegmentKind> {
    segs.retain(|s| match s {
        DateSegmentKind::Day     => granularity >= DateGranularity::Day,
        DateSegmentKind::Month   => granularity >= DateGranularity::Month,
        DateSegmentKind::Year | DateSegmentKind::Era => true,
        DateSegmentKind::Literal => true, // cleaned below
        _ => false,
    });
    // Remove trailing literals
    while segs.last() == Some(&DateSegmentKind::Literal) {
        segs.pop();
    }
    segs
}

/// Locale-specific separator strings, in order of appearance.
pub struct LocaleSeparators {
    /// The separators.
    pub separators: Vec<String>,
}

impl LocaleSeparators {
    pub fn for_locale(locale: &Locale) -> Self {
        // Production: ICU4X pattern parsing extracts exact separator strings.
        let seps: Vec<String> = match locale.as_str() {
            "de-DE" | "de-AT" | "nl-NL" | "pl-PL" | "ru-RU" | "cs-CZ"
            | "sk-SK" | "hr-HR" | "bg-BG" | "ro-RO" =>
                vec![".".into(), ".".into()],
            "ja-JP" =>
                vec!["\u{5E74}".into(), "\u{6708}".into(), "\u{65E5}".into()],
            // Korean uses year/month/day ideographs as primary format.
            // The formal ". " variant is an alternative used in some contexts.
            "ko-KR" =>
                vec!["\u{B144} ".into(), "\u{C6D4} ".into(), "\u{C77C}".into()],
            "zh-CN" | "zh-TW" | "zh-SG" =>
                vec!["/".into(), "/".into()],
            _ =>
                vec!["/".into(), "/".into()],
        };
        Self { separators: seps }
    }

    pub fn get(&self, idx: usize) -> &str {
        self.separators.get(idx).map(String::as_str).unwrap_or("/")
    }
}

/// Build a fully populated Vec<DateSegment> from an ordered kind list.
///
/// When `force_leading_zeros` is true, numeric segments are always
/// zero-padded to their maximum display width regardless of locale
/// (month/day: 2 digits, year: 4 digits).
pub fn build_segments(
    kinds: &[DateSegmentKind],
    value: Option<&CalendarDate>,
    locale: &Locale,
    calendar: CalendarSystem,
    separators: &LocaleSeparators,
    force_leading_zeros: bool,
) -> Vec<DateSegment> {
    let mut literal_idx = 0usize;

    kinds.iter().map(|kind| {
        match kind {
            DateSegmentKind::Literal => {
                let sep = separators.get(literal_idx);
                literal_idx += 1;
                DateSegment::new_literal(sep)
            }
            DateSegmentKind::Year => {
                let mut seg = DateSegment::new_numeric(*kind, 1, 9999, "yyyy");
                if let Some(d) = value {
                    seg.value = Some(d.year);
                    seg.text  = format!("{:04}", d.year);
                }
                seg
            }
            DateSegmentKind::Month => {
                let mut seg = DateSegment::new_numeric(*kind, 1, 12, "mm");
                if let Some(d) = value {
                    seg.value = Some(d.month.get() as i32);
                    seg.text  = if force_leading_zeros {
                        format!("{:02}", d.month.get())
                    } else {
                        format!("{}", d.month.get())
                    };
                }
                seg
            }
            DateSegmentKind::Day => {
                let max_day = value.map(|d| d.days_in_month() as i32).unwrap_or(31);
                let mut seg = DateSegment::new_numeric(*kind, 1, max_day, "dd");
                if let Some(d) = value {
                    seg.value = Some(d.day.get() as i32);
                    seg.text  = if force_leading_zeros {
                        format!("{:02}", d.day.get())
                    } else {
                        format!("{}", d.day.get())
                    };
                }
                seg
            }
            DateSegmentKind::Era => {
                let text = value
                    .and_then(|d| d.era.as_ref())
                    .map(|e| e.display_name.clone())
                    .unwrap_or_default();
                DateSegment {
                    kind: *kind,
                    value: None,
                    min: 0,
                    max: 0,
                    text,
                    placeholder: "era".to_string(),
                    literal: None,
                    is_editable: true,
                }
            }
            _ => DateSegment::new_literal(""),
        }
    }).collect()
}
```

### 1.11 Segment Order Override, Auto-Advance & IME

**Localized segment order** is derived from the locale's `dateFormat` pattern
(see `segments_for_locale` above). Common orderings:

| Region | Format         | Segment order                                |
| ------ | -------------- | -------------------------------------------- |
| US     | MM/DD/YYYY     | Month, Day, Year                             |
| EU     | DD/MM/YYYY     | Day, Month, Year                             |
| ISO    | YYYY-MM-DD     | Year, Month, Day                             |
| Japan  | YYYY年MM月DD日 | Year, Month, Day (with ideographic literals) |

**`segment_order` prop override**: Consumers can override the locale-derived
segment order by supplying an explicit `segment_order: Option<Vec<DateSegmentKind>>`
prop. When set, the field uses this order instead of the locale default. This is
useful for forms that must enforce a specific date entry order regardless of locale.

**Auto-advance semantics**: When a numeric segment's value is fully determined
(i.e., the typed digits form a complete value that cannot be extended further),
focus automatically advances to the next editable segment. For example:

- Month: typing `1` waits (could be 10, 11, 12); typing `2` after `1` commits
  `12` and advances. Typing `2` alone commits `02` and advances.
- Day: typing `3` followed by `1` commits `31` and advances. Typing `4` alone
  commits `04` and advances (no 2-digit day starts with 4).
- Year: advances after 4 digits are typed.

**IME handling**: When an Input Method Editor is active (detected via
`compositionstart`/`compositionend` events), segment type-ahead is suspended
until the composition completes. The DayPeriod segment ignores IME input
entirely -- only direct `a`/`p` key presses are accepted. The adapter must:

1. Set a `composing: bool` flag on `compositionstart`.
2. Suppress `TypeIntoSegment` events while `composing` is `true`.
3. On `compositionend`, process the composed result if applicable.

### 1.12 Enhanced Type-Ahead for Date Segments

DateField segment type-ahead MUST support numeric input, abbreviated name matching, and IME composition handling beyond the basic single-character matching.

**Numeric Type-Ahead** (Month, Day, Year, Hour, Minute, Second segments):

- Typing digits auto-fills the segment left-to-right
- When the entered digits form an unambiguous valid value, auto-advance to the next segment:
  - Month: typing `2` waits (could be 2 or 20+); typing `3`-`9` auto-advances (only single-digit months 3-9 possible); typing `1` then `2` = December, auto-advance
  - Day: typing `4`-`9` auto-advances (single-digit); `1`-`3` waits for second digit
  - Year: waits until 4 digits entered (or 2 for 2-digit year formats), then auto-advances
  - Hour (12h): typing `2`-`9` auto-advances; `1` waits (could be 10-12)
  - Hour (24h): typing `3`-`9` auto-advances; `1`-`2` waits (could be 10-23)
  - Minute/Second: typing `6`-`9` auto-advances; `0`-`5` waits for second digit

**Abbreviated Month Name Matching** (Month segment with named format):

- Typing characters matches against locale-aware abbreviated month names
- Example (en-US): typing "J" highlights January; "Ju" narrows to June/July; "Jul" selects July
- Example (de-DE): typing "M" highlights Marz; "Ma" narrows to Marz/Mai
- Matching is case-insensitive and uses locale-appropriate collation
- After 1 second of inactivity, the type-ahead buffer resets

**IME Composition Handling**:

- During IME composition (`event.isComposing === true` or between `compositionstart` / `compositionend`), type-ahead MUST NOT process individual keystrokes
- After `compositionend`, the accumulated composed text is used for matching
- This prevents CJK input from triggering premature segment advancement (e.g., typing Japanese month names)

**DayPeriod Type-Ahead** (AM/PM segment):

- Single character: "A" -> AM, "P" -> PM (English)
- CJK locales: match against full period names (e.g., Japanese "午前"/"午後", Chinese "上午"/"下午")
- CJK matching waits for `compositionend` before processing

**Enhanced CJK Day Period Handling:**

CJK day period matching MUST support full period name matching for the following locales:

| Locale | AM (romanized) | PM (romanized) | Match Strategy                                            |
| ------ | -------------- | -------------- | --------------------------------------------------------- |
| `ja`   | gozen (午前)   | gogo (午後)    | Buffer-match: accumulate until unique prefix found        |
| `zh`   | shangwu (上午) | xiawu (下午)   | First char differs: '上'->AM, '下'->PM (single-char OK)   |
| `ko`   | ojeon (오전)   | ohu (오후)     | Buffer-match: '오' shared, second char '전'->AM, '후'->PM |

The DateField/TimeField `Context` MUST include an `is_composing: bool` field, set to `true` on `compositionstart` and `false` on `compositionend`. While `is_composing` is `true`:

- The DayPeriod segment MUST NOT process individual keystrokes.
- The type-ahead buffer accumulates IME candidates silently.
- On `compositionend`, the full composed string is passed to `day_period_from_buffer()` for matching.
- After matching (or no match), the buffer resets and normal input resumes.

### 1.13 Full Machine Implementation

```rust
/// Machine for the DateField component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let ids = ComponentIds::from_id(&props.id);
        let error_message_id = ids.part("error-message");
        let locale = env.locale.clone();
        let messages = messages.clone();
        let mut ctx = Context {
            value: match &props.value {
                Some(v) => Bindable::controlled(v.clone()),
                None    => Bindable::uncontrolled(props.default_value.clone()),
            },
            segments: Vec::new(),
            focused_segment: None,
            type_buffer: String::new(),
            locale,
            provider: env.icu_provider.clone(),
            messages,
            calendar: props.calendar,
            granularity: props.granularity,
            disabled: props.disabled,
            readonly: props.readonly,
            min_value: props.min_value.clone(),
            max_value: props.max_value.clone(),
            id: props.id.clone(),
            ids,
            invalid: props.invalid,
            error_message_id,
            force_leading_zeros: props.force_leading_zeros,
        };
        ctx.rebuild_segments();
        (State::Idle, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if ctx.disabled {
            match event {
                Event::SetValue(_) => {},
                _ => return None,
            }
        }

        match event {
            // -- Focus management --------------------------------------------
            Event::FocusSegment(kind) => {
                if !kind.is_editable() { return None; }
                let kind = *kind;
                Some(TransitionPlan::to(State::Focused(kind))
                    .apply(move |ctx| {
                        ctx.focused_segment = Some(kind);
                        ctx.type_buffer.clear();
                    }))
            }

            Event::BlurAll => {
                let has_buffer = !ctx.type_buffer.is_empty();
                let focused = ctx.focused_segment;
                Some(TransitionPlan::to(State::Idle)
                    .apply(move |ctx| {
                        // Commit any digits sitting in the buffer.
                        if has_buffer {
                            if let Some(focused) = focused {
                                if let Ok(v) = ctx.type_buffer.parse::<i32>() {
                                    ctx.set_segment_value(focused, v);
                                    Machine::maybe_publish(ctx);
                                }
                            }
                        }
                        ctx.focused_segment = None;
                        ctx.type_buffer.clear();
                    }))
            }

            Event::FocusNextSegment => {
                if let State::Focused(current) = state {
                    let current = *current;
                    let has_buffer = !ctx.type_buffer.is_empty();
                    let next = ctx.next_editable_after(current);
                    let target = match next {
                        Some(k) => State::Focused(k),
                        None    => State::Idle,
                    };
                    Some(TransitionPlan::to(target)
                        .apply(move |ctx| {
                            if has_buffer {
                                if let Ok(v) = ctx.type_buffer.parse::<i32>() {
                                    ctx.set_segment_value(current, v);
                                    Machine::maybe_publish(ctx);
                                }
                                ctx.type_buffer.clear();
                            }
                            ctx.focused_segment = next;
                        }))
                } else {
                    let first = ctx.first_editable()?;
                    Some(TransitionPlan::to(State::Focused(first))
                        .apply(move |ctx| {
                            ctx.focused_segment = Some(first);
                        }))
                }
            }

            Event::FocusPrevSegment => {
                if let State::Focused(current) = state {
                    let current = *current;
                    if !ctx.type_buffer.is_empty() {
                        // Cancel partial input; stay on current segment.
                        return Some(TransitionPlan::to(state.clone())
                            .apply(|ctx| {
                                ctx.type_buffer.clear();
                            }));
                    }
                    match ctx.prev_editable_before(current) {
                        Some(k) => {
                            Some(TransitionPlan::to(State::Focused(k))
                                .apply(move |ctx| {
                                    ctx.focused_segment = Some(k);
                                }))
                        }
                        None => None, // already on first segment
                    }
                } else {
                    None
                }
            }

            // -- Spin button increment / decrement ---------------------------
            Event::IncrementSegment(kind) => {
                if ctx.readonly { return None; }
                let kind = *kind;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.type_buffer.clear();
                    ctx.increment_segment(kind);
                    Machine::maybe_publish(ctx);
                }))
            }

            Event::DecrementSegment(kind) => {
                if ctx.readonly { return None; }
                let kind = *kind;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.type_buffer.clear();
                    ctx.decrement_segment(kind);
                    Machine::maybe_publish(ctx);
                }))
            }

            // -- Type-ahead --------------------------------------------------
            Event::TypeIntoSegment(kind, ch) => {
                if ctx.readonly { return None; }
                if !matches!(state, State::Focused(_)) { return None; }
                let ch = *ch;
                let kind = *kind;

                match kind {
                    DateSegmentKind::DayPeriod => {
                        // Locale-aware AM/PM type-ahead:
                        // - Latin: 'a' for AM, 'p' for PM
                        // - Arabic: 'ص' (Sad) for AM, 'م' (Mim) for PM
                        // - Japanese: '午' starts both 午前/午後 -- further chars disambiguate
                        // - Korean: '오' starts both 오전/오후
                        // Support Latin + Arabic. CJK handled via ArrowUp/Down only.
                        let period_value = day_period_from_char(&*ctx.provider, ch, &ctx.locale);
                        let period_value = match period_value {
                            Some(v) => v,
                            None => return None,
                        };
                        let next_seg = ctx.next_editable_after(DateSegmentKind::DayPeriod);
                        Some(TransitionPlan::to(match next_seg {
                            Some(nk) => State::Focused(nk),
                            None => state.clone(),
                        }).apply(move |ctx| {
                            ctx.set_segment_value(DateSegmentKind::DayPeriod, period_value);
                            Machine::maybe_publish(ctx);
                            if let Some(nk) = next_seg {
                                ctx.focused_segment = Some(nk);
                            }
                        }))
                    }
                    k if k.is_numeric() => {
                        if !ch.is_ascii_digit() { return None; }
                        // We need to compute what will happen after buffer push
                        let mut new_buffer = ctx.type_buffer.clone();
                        new_buffer.push(ch);
                        let buffered: u32 = new_buffer.parse().unwrap_or(0);

                        let (seg_min, seg_max) = ctx.segments.iter()
                            .find(|s| s.kind == k)
                            .map(|s| (s.min, s.max))
                            .unwrap_or((0, 99));

                        let max_digits = digits_needed(seg_max);
                        let should_advance = new_buffer.len() >= max_digits
                            || buffered * 10 > seg_max;
                        let valid = buffered >= seg_min && buffered <= seg_max;
                        let next_seg = if should_advance { ctx.next_editable_after(k) } else { None };

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
                                    ctx.set_segment_value(k, buffered);
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
                        // immediately consumed. This replaces the old TimerId-based
                        // approach with a PendingEffect that the adapter converts
                        // into a setTimeout / spawn_local timer.
                        if !should_advance {
                            plan = plan.with_effect(PendingEffect::new(
                                "type-buffer-commit",
                                move |_ctx, _props, send| {
                                    let send = send.clone();
                                    // Adapter schedules ~1s timeout, then sends commit.
                                    Box::new(move || {
                                        send(Event::TypeBufferCommit(k));
                                    })
                                },
                            ));
                        }
                        Some(plan)
                    }
                    DateSegmentKind::Era => {
                        // Era navigation is locale-specific; arrow keys handle it.
                        None
                    }
                    _ => None,
                }
            }

            Event::TypeBufferCommit(kind) => {
                if ctx.readonly { return None; }
                let kind = *kind;
                Some(TransitionPlan::context_only(move |ctx| {
                    if let Ok(v) = ctx.type_buffer.parse::<i32>() {
                        ctx.set_segment_value(kind, v);
                        Machine::maybe_publish(ctx);
                    }
                    ctx.type_buffer.clear();
                }))
            }

            // -- Clear -------------------------------------------------------
            Event::ClearSegment(kind) => {
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
                        let editable: Vec<_> = ctx.segments.iter()
                            .filter(|s| s.is_editable)
                            .map(|s| s.kind)
                            .collect();
                        for k in editable { ctx.clear_segment_value(k); }
                        ctx.value.set(None);
                        ctx.type_buffer.clear();
                    }))
            }

            // -- Programmatic set --------------------------------------------
            Event::SetValue(v) => {
                let v = v.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.value.set(v);
                    ctx.rebuild_segments();
                }))
            }
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}

impl Machine {
    /// Maybe publish the date.
    fn maybe_publish(ctx: &mut Context) {
        if !ctx.is_complete() { return; }
        let Some(date) = ctx.assemble_date() else { return };
        let clamped = match (&ctx.min_value, &ctx.max_value) {
            (Some(min), _) if date < *min => min.clone(),
            (_, Some(max)) if date > *max => max.clone(),
            _ => date,
        };
        ctx.value.set(Some(clamped));
    }
}

/// Get the number of digits needed for a number.
fn digits_needed(n: u32) -> usize {
    if n == 0 { return 1; }
    let digits = n.ilog10() as usize;
    if digits == 0 { 1 } else { digits + 1 }
}
```

### 1.14 Connect / API

```rust
// ars-core/src/components/date_field/connect.rs

#[derive(ComponentPart)]
#[scope = "date-field"]
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

/// API for the DateField component.
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
    /// Create a new API.
    pub fn new(
        state: &'a State,
        ctx:   &'a Context,
        props: &'a Props,
        send:  &'a dyn Fn(Event),
    ) -> Self { Self { state, ctx, props, send } }

    // -- AttrMap getters (data only, SSR-safe) --

    /// Attrs for the outermost `<div>` wrapper.
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
        if self.ctx.invalid {
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
        }
        attrs
    }

    /// Attrs for the `<label>` element.
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("label"));
        attrs.set(HtmlAttr::For, self.ctx.ids.part("field-group"));
        attrs
    }

    /// Attrs for the `<div role="group">` containing all segments.
    pub fn field_group_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::FieldGroup.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("field-group"));
        attrs.set(HtmlAttr::Role, "group");
        if let Some(ref lbl) = self.props.aria_labelledby {
            attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), lbl);
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.part("label"));
        }
        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }
        if self.ctx.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }
        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }
        if self.props.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }
        // Wire description and error message via aria-describedby
        let mut described_by = Vec::new();
        if self.props.description.is_some() {
            described_by.push(self.ctx.ids.part("description"));
        }
        if self.ctx.invalid && self.props.error_message.is_some() {
            described_by.push(self.ctx.ids.part("error-message"));
        }
        if let Some(ref extra) = self.props.aria_describedby {
            described_by.push(extra.clone());
        }
        if !described_by.is_empty() {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), described_by.join(" "));
        }
        attrs
    }

    /// Attrs for a segment `<div>`. Handles both editable (spinbutton) and literal segments.
    pub fn segment_attrs(&self, kind: &DateSegmentKind) -> AttrMap {
        let segment = match self.ctx.segments.iter().find(|s| s.kind == *kind) {
            Some(s) => s,
            None => return AttrMap::new(),
        };
        let mut attrs = AttrMap::new();

        if !segment.is_editable {
            let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Literal { index: 0 }.data_attrs();
            attrs.set(scope_attr, scope_val);
            attrs.set(part_attr, part_val);
            attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
            attrs.set(HtmlAttr::TabIndex, "-1");
            return attrs;
        }

        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Segment { kind: *kind }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);

        let is_focused = self.ctx.focused_segment == Some(segment.kind);

        attrs.set(HtmlAttr::Role, "spinbutton");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), segment.kind.aria_label(&self.ctx.messages, &self.ctx.locale));
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), segment.min.to_string());
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), segment.max.to_string());
        attrs.set(HtmlAttr::Data("ars-segment"), format!("{:?}", segment.kind).to_lowercase());
        attrs.set(HtmlAttr::TabIndex,
            if is_focused || (self.ctx.focused_segment.is_none()
                && self.ctx.first_editable() == Some(segment.kind)) {
                "0"
            } else {
                "-1"
            }
        );

        // aria-valuenow and aria-valuetext
        if let Some(v) = segment.value {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueNow), v.to_string());
            if let Some(text) = segment.aria_value_text(&self.ctx.locale) {
                attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), text);
            }
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::ValueText), &segment.placeholder);
        }

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        // Link every editable segment to the single field-level error message
        // when invalid. This ensures the error is announced once per field focus,
        // not redundantly per segment.
        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), &self.ctx.error_message_id);
        }

        attrs
    }

    /// Attrs for a literal separator segment.
    pub fn literal_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Literal { index }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs
    }

    /// Attrs for the description element.
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));
        attrs
    }

    /// Attrs for the error message element.
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("error-message"));
        attrs.set(HtmlAttr::Role, "alert");
        attrs
    }

    /// Attrs for the hidden `<input type="hidden">` used in form submission.
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
        if let Some(name) = &self.props.name {
            attrs.set(HtmlAttr::Name, name);
        }
        attrs.set(HtmlAttr::Value, value);
        attrs
    }

    // -- Typed handler methods (adapters wire to native events) --

    /// Handle focusout on field group -- only blur if focus leaves the entire group.
    pub fn on_field_group_focusout(&self, focus_leaving_group: bool) {
        if focus_leaving_group {
            (self.send)(Event::BlurAll);
        }
    }

    /// Handle keydown on a segment.
    ///
    /// `dir` determines RTL-aware arrow key behavior: in RTL locales,
    /// ArrowRight moves to the previous segment (visually left = logically previous)
    /// and ArrowLeft moves to the next segment. This matches React Aria behavior.
    pub fn on_segment_keydown(&self, kind: DateSegmentKind, data: &KeyboardEventData, shift: bool, dir: Direction) {
        let is_rtl = dir.is_rtl();
        match data.key {
            KeyboardKey::ArrowUp   => (self.send)(Event::IncrementSegment(kind)),
            KeyboardKey::ArrowDown => (self.send)(Event::DecrementSegment(kind)),
            KeyboardKey::ArrowRight => {
                if is_rtl { (self.send)(Event::FocusPrevSegment) }
                else      { (self.send)(Event::FocusNextSegment) }
            }
            KeyboardKey::ArrowLeft => {
                if is_rtl { (self.send)(Event::FocusNextSegment) }
                else      { (self.send)(Event::FocusPrevSegment) }
            }
            KeyboardKey::Tab if !shift => (self.send)(Event::FocusNextSegment),
            KeyboardKey::Tab if shift  => (self.send)(Event::FocusPrevSegment),
            KeyboardKey::Backspace | KeyboardKey::Delete => (self.send)(Event::ClearSegment(kind)),
            KeyboardKey::Escape => (self.send)(Event::ClearAll),
            _ if let Some(ch) = data.character => {
                (self.send)(Event::TypeIntoSegment(kind, ch));
            }
            _ => {}
        }
    }

    /// Handle focus on a segment.
    pub fn on_segment_focus(&self, kind: DateSegmentKind) {
        (self.send)(Event::FocusSegment(kind));
    }

    /// Handle click on a segment.
    pub fn on_segment_click(&self, kind: DateSegmentKind) {
        (self.send)(Event::FocusSegment(kind));
    }

    // -- Computed state accessors --

    /// Get the segments of the component.
    pub fn segments(&self) -> &[DateSegment] { &self.ctx.segments }
    /// Get the value of the component.
    pub fn value(&self) -> Option<&CalendarDate> { self.ctx.value.get().as_ref() }
    /// Check if the component is focused.
    pub fn is_focused(&self) -> bool { !matches!(self.state, State::Idle) }

    /// Get the name of the state.
    const fn state_name(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::Focused(_) => "focused",
        }
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

| Part           | HTML Element                | Purpose                                                                     |
| -------------- | --------------------------- | --------------------------------------------------------------------------- |
| `Root`         | `<div>`                     | Outermost container; `data-ars-scope="date-field"`                          |
| `Label`        | `<label>`                   | Visible label; `for` points to the first focusable segment                  |
| `FieldGroup`   | `<div role="group">`        | Groups all segments + receives the accessible name                          |
| `Segment`      | `<div role="spinbutton">`   | One per editable segment (Year, Month, Day, Era)                            |
| `Literal`      | `<span aria-hidden="true">` | Non-interactive separator characters                                        |
| `Description`  | `<div>`                     | Optional help text; referenced by `aria-describedby`                        |
| `ErrorMessage` | `<div role="alert">`        | Validation error text; announced immediately. `id` = `ctx.error_message_id` |
| `HiddenInput`  | `<input type="hidden">`     | ISO date string for HTML form submission                                    |

> **Note**: Error messages and descriptions are associated at the **field level**
> via `aria-describedby` on each editable segment pointing to `ctx.error_message_id`.
> This ensures the error is announced **once per field** (not per segment).
> This matches React Aria's approach. **Test**: when `invalid` is `true`, assert
> that every spinbutton segment's `aria-describedby` equals the single
> `error_message_id`, and that the error is announced exactly once when the user
> focuses any segment.

```text
DateField (en-US, Day granularity)
└── Root                            data-ars-scope="date-field"
    ├── Label                       <label>
    ├── FieldGroup                  role="group"  aria-label="Date"
    │   ├── Segment (Month)         role="spinbutton"  aria-label="Month"
    │   ├── Literal "/"             aria-hidden="true"
    │   ├── Segment (Day)           role="spinbutton"  aria-label="Day"
    │   ├── Literal "/"             aria-hidden="true"
    │   └── Segment (Year)          role="spinbutton"  aria-label="Year"
    ├── Description
    ├── ErrorMessage                role="alert"
    └── HiddenInput                 type="hidden"

DateField (ja-JP, Day granularity, Japanese calendar)
└── Root
    ├── Label
    ├── FieldGroup                  role="group"  aria-label="日付"
    │   ├── Segment (Era)           role="spinbutton"  aria-label="Era"
    │   ├── Segment (Year)          role="spinbutton"  aria-label="Year"
    │   ├── Literal "年"            aria-hidden="true"
    │   ├── Segment (Month)         role="spinbutton"  aria-label="Month"
    │   ├── Literal "月"            aria-hidden="true"
    │   ├── Segment (Day)           role="spinbutton"  aria-label="Day"
    │   └── Literal "日"            aria-hidden="true"
    └── HiddenInput
```

## 3. Accessibility

### 3.1 ARIA Roles and Attributes

| Element             | Role         | Required Attributes                                                               |
| ------------------- | ------------ | --------------------------------------------------------------------------------- |
| `FieldGroup`        | `group`      | `aria-label` or `aria-labelledby`                                                 |
| `Segment` (numeric) | `spinbutton` | `aria-label`, `aria-valuenow`, `aria-valuemin`, `aria-valuemax`, `aria-valuetext` |
| `Literal`           | --           | `aria-hidden="true"`                                                              |
| `ErrorMessage`      | `alert`      | -- (auto-announced on render)                                                     |

**`aria-valuetext` values by segment:**

| Segment           | `aria-valuenow` | `aria-valuetext`                |
| ----------------- | --------------- | ------------------------------- |
| Month (value=3)   | `3`             | "March"                         |
| Day (value=15)    | `15`            | "15"                            |
| Year (value=2024) | `2024`          | "2024"                          |
| DayPeriod (PM)    | `1`             | "PM" (or "午後" in ja-JP)       |
| Unset segment     | omitted         | Placeholder: "mm", "dd", "yyyy" |

### 3.2 Keyboard Interaction

| Key                    | Action                                                                  |
| ---------------------- | ----------------------------------------------------------------------- |
| `ArrowUp`              | Increment focused segment by 1, wrapping at max                         |
| `ArrowDown`            | Decrement focused segment by 1, wrapping at min                         |
| `ArrowRight`           | Move focus to next editable segment                                     |
| `ArrowLeft`            | Move focus to previous editable segment                                 |
| `Tab`                  | Move to next editable segment; Tab past last exits the field            |
| `Shift+Tab`            | Move to previous editable segment                                       |
| `0-9`                  | Type-ahead for numeric segments; auto-advances when value is determined |
| `a` or `p`             | Set AM / PM in DayPeriod segment                                        |
| `Backspace` / `Delete` | Clear the focused segment's value                                       |
| `Escape`               | Clear all segment values                                                |

### 3.3 Screen Reader Announcement Examples

When user changes the Month segment from 2 to 3:

- NVDA/JAWS: "March, Month, spinbutton, minimum 1, maximum 12"
- VoiceOver: "March"

When a segment has no value:

- NVDA/JAWS: "mm, Month, spinbutton, minimum 1, maximum 12"

## 4. Internationalization

| Aspect                        | Details                                                                                        |
| ----------------------------- | ---------------------------------------------------------------------------------------------- |
| Segment order                 | CLDR-derived: M/D/Y (en-US), D.M.Y (de-DE), Y-M-D (ISO/zh-CN), Era+Y年M月D日 (ja-JP)           |
| Separator literals            | Locale-specific: "/" (en-US), "." (de-DE), "年月日" (ja-JP), "/" (zh-CN)                       |
| Month names in aria-valuetext | ICU4X MonthNames (English: "January"..."December"; French: "janvier"..."decembre")             |
| Numerals                      | Arabic-Indic for `ar` locale; Persian-Indic for `fa-IR` (via ICU4X numeral formatting)         |
| Era names                     | Japanese eras shown in kanji: 令和 (Reiwa), 平成 (Heisei), 昭和 (Showa)                        |
| RTL layout                    | `dir="rtl"` on FieldGroup for RTL locales; segment DOM order LTR (CSS handles visual reversal) |
| Calendar system               | Determines available segments (Era in Japanese), day counts, year range, and month names       |

### 4.1 Messages

```rust
/// Localized labels for DateField/TimeField segments and announcements.
#[derive(Clone, Debug)]
pub struct Messages {
    /// The label for the year segment.
    pub year_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// The label for the month segment.
    pub month_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// The label for the day segment.
    pub day_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// The label for the hour segment.
    pub hour_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// The label for the minute segment.
    pub minute_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// The label for the second segment.
    pub second_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// The label for the day period segment.
    pub day_period_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// The label for the era segment.
    pub era_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// The label for the weekday segment.
    pub weekday_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// The label for the time zone name segment.
    pub timezone_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            year_label: MessageFn::static_str("Year"),
            month_label: MessageFn::static_str("Month"),
            day_label: MessageFn::static_str("Day"),
            hour_label: MessageFn::static_str("Hour"),
            minute_label: MessageFn::static_str("Minute"),
            second_label: MessageFn::static_str("Second"),
            day_period_label: MessageFn::static_str("AM/PM"),
            era_label: MessageFn::static_str("Era"),
            weekday_label: MessageFn::static_str("Day of week"),
            timezone_label: MessageFn::static_str("Time zone"),
        }
    }
}

impl ComponentMessages for Messages {}
```

### 4.2 Month/Weekday Display

Segment display format (abbreviated vs. full) is locale-dependent and configured via ICU4X `DateTimeFormatter` pattern. Some locales have no standard abbreviation (Arabic months) -- in these cases, the full name is used. Typeahead matching is case-insensitive and matches against both abbreviated and full names when both exist.

## 5. Form Integration

- Hidden `<input>` submits the current date value in ISO 8601 format (`YYYY-MM-DD`).
- `name` attribute is set from Props.
- Reset restores `default_value`.
- Validation states (`valid`, `invalid`) reflected via `data-ars-invalid` on Root and `aria-invalid` on FieldGroup.
- `aria-describedby` wires to Description and ErrorMessage parts.
- `aria-required` set when `required` is true.
- Disabled/readonly propagation from form context per `07-forms.md` S15.

## 6. Library Parity

> Compared against: React Aria (`DateField`).

### 6.1 Props

| Feature               | ars-ui                   | React Aria                | Notes                                               |
| --------------------- | ------------------------ | ------------------------- | --------------------------------------------------- |
| Controlled value      | `value`                  | `value`                   | Equivalent                                          |
| Default value         | `default_value`          | `defaultValue`            | Equivalent                                          |
| Min/max               | `min_value`, `max_value` | `minValue`, `maxValue`    | Equivalent                                          |
| Unavailable predicate | --                       | `isDateUnavailable`       | React Aria has it; ars-ui relies on min/max         |
| Granularity           | `granularity`            | `granularity`             | Equivalent                                          |
| Calendar system       | `calendar`               | `createCalendar`          | Equivalent concept, different API                   |
| Disabled              | `disabled`               | `isDisabled`              | Equivalent                                          |
| Read-only             | `readonly`               | `isReadOnly`              | Equivalent                                          |
| Required              | `required`               | `isRequired`              | Equivalent                                          |
| Invalid               | `invalid`                | `isInvalid`               | Equivalent                                          |
| Placeholder value     | --                       | `placeholderValue`        | React Aria uses a DateValue as placeholder template |
| Force leading zeros   | `force_leading_zeros`    | `shouldForceLeadingZeros` | Equivalent                                          |
| Hour cycle            | --                       | `hourCycle`               | React Aria DateField can include time segments      |
| Hide time zone        | --                       | `hideTimeZone`            | React Aria DateField can include time zone          |
| Auto-focus            | `auto_focus`             | `autoFocus`               | Equivalent                                          |
| Name                  | `name`                   | `name`                    | Equivalent                                          |
| Validate              | --                       | `validate`                | React Aria custom validation function               |
| Validation behavior   | --                       | `validationBehavior`      | React Aria native/aria validation mode              |
| Form                  | --                       | `form`                    | React Aria associated form ID                       |
| Auto-complete         | --                       | `autoComplete`            | React Aria auto-fill hint                           |
| Segment order         | `segment_order`          | --                        | ars-ui allows explicit override                     |
| Label/aria-label      | `label`, `aria_label`    | Label sub-component       | Equivalent                                          |

**Gaps:**

- `placeholderValue`: React Aria allows specifying a DateValue that determines placeholder format (e.g., which calendar to show). ars-ui derives this from the `calendar` and `locale` props, which is equivalent.
- `isDateUnavailable`: React Aria supports this on DateField for visual marking. ars-ui only uses this on Calendar. Not critical for an inline field.
- `validate`/`validationBehavior`: React Aria's custom validation is form-framework-specific. ars-ui uses `invalid` + `error_message` which is simpler and sufficient.

None worth adopting.

### 6.2 Anatomy

| Part          | ars-ui         | React Aria                     | Notes                   |
| ------------- | -------------- | ------------------------------ | ----------------------- |
| Root          | `Root`         | `DateField`                    | Equivalent              |
| Label         | `Label`        | `Label`                        | Equivalent              |
| Field group   | `FieldGroup`   | `DateInput`                    | Equivalent              |
| Segment       | `Segment`      | `DateSegment`                  | Equivalent              |
| Literal       | `Literal`      | `DateSegment` (type="literal") | Equivalent              |
| Description   | `Description`  | `Text` (slot="description")    | Equivalent              |
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

| Feature                           | ars-ui                        | React Aria           |
| --------------------------------- | ----------------------------- | -------------------- |
| Segmented spinbutton input        | Yes                           | Yes                  |
| Locale-driven segment ordering    | Yes                           | Yes                  |
| Type-ahead numeric entry          | Yes                           | Yes                  |
| Auto-advance on complete          | Yes                           | Yes                  |
| Increment/decrement with wrapping | Yes                           | Yes                  |
| Calendar system support           | Yes (CalendarSystem enum)     | Yes (createCalendar) |
| Era segment (Japanese calendar)   | Yes                           | Yes                  |
| Force leading zeros               | Yes                           | Yes                  |
| Controlled value sync             | Yes (with deferred prop sync) | Yes                  |
| Hidden form input                 | Yes                           | No                   |
| Custom segment order              | Yes                           | No                   |
| Month name prefix matching        | Yes                           | No                   |

**Gaps:** None. ars-ui is a superset with custom segment order and month name matching.

### 6.5 Summary

- **Overall:** Full parity.
- **Divergences:** React Aria's DateField can include time segments via `granularity`; ars-ui separates time into TimeField. React Aria supports `placeholderValue` as a DateValue; ars-ui uses locale and calendar props to derive the same behavior.
- **Recommended additions:** None.

## Appendix: Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ars_core::service::Service;

    fn make_service(granularity: DateGranularity) -> Service<DateField> {
        Service::new(Props {
            value: None,
            default_value: None,
            on_change: None,
            calendar: CalendarSystem::Gregorian,
            granularity,
            min_value: None,
            max_value: None,
            disabled: false,
            readonly: false,
            required: false,
            auto_focus: false,
            id: Some("test-date-field".into()),
            label: "Date".into(),
            aria_label: None,
            aria_labelledby: None,
            aria_describedby: None,
            description: None,
            error_message: None,
            invalid: false,
            name: None,
            force_leading_zeros: false,
        }, Env::default(), Default::default())
    }

    #[test]
    fn initial_state_is_idle() {
        let svc = make_service(DateGranularity::Day);
        assert_eq!(*svc.state(), State::Idle);
    }

    #[test]
    fn focus_segment_transitions_to_focused() {
        let mut svc = make_service(DateGranularity::Day);
        svc.send(Event::FocusSegment(DateSegmentKind::Month));
        assert_eq!(*svc.state(), State::Focused(DateSegmentKind::Month));
    }

    #[test]
    fn blur_all_returns_to_idle() {
        let mut svc = make_service(DateGranularity::Day);
        svc.send(Event::FocusSegment(DateSegmentKind::Day));
        svc.send(Event::BlurAll);
        assert_eq!(*svc.state(), State::Idle);
    }

    #[test]
    fn typing_month_12_auto_advances_to_day() {
        let mut svc = make_service(DateGranularity::Day);
        svc.send(Event::FocusSegment(DateSegmentKind::Month));
        svc.send(Event::TypeIntoSegment(DateSegmentKind::Month, '1'));
        // "1" alone is ambiguous (could be 1, 10, 11, 12) -- stays on month
        assert_eq!(*svc.state(), State::Focused(DateSegmentKind::Month));
        svc.send(Event::TypeIntoSegment(DateSegmentKind::Month, '2'));
        // "12" is complete -- auto-advances to Day
        assert_eq!(*svc.state(), State::Focused(DateSegmentKind::Day));
        assert_eq!(svc.context().get_segment_value(DateSegmentKind::Month), Some(12));
    }

    #[test]
    fn increment_wraps_month_from_12_to_1() {
        let mut svc = make_service(DateGranularity::Day);
        svc.send(Event::FocusSegment(DateSegmentKind::Month));
        // Set month to 12 first
        svc.context_mut().set_segment_value(DateSegmentKind::Month, 12);
        svc.send(Event::IncrementSegment(DateSegmentKind::Month));
        assert_eq!(svc.context().get_segment_value(DateSegmentKind::Month), Some(1));
    }

    #[test]
    fn en_us_segment_order_is_month_day_year() {
        let svc = make_service(DateGranularity::Day);
        let kinds: Vec<_> = svc.context().segments.iter().map(|s| s.kind).collect();
        assert_eq!(kinds, vec![
            DateSegmentKind::Month,
            DateSegmentKind::Literal,
            DateSegmentKind::Day,
            DateSegmentKind::Literal,
            DateSegmentKind::Year,
        ]);
    }

    #[test]
    fn de_de_segment_order_is_day_month_year() {
        let env = Env { locale: Locale::parse("de-DE").expect("valid locale"), ..Env::default() };
        let svc = Service::new(Props { granularity: DateGranularity::Day, ..Props::default() }, env, Default::default());
        let kinds: Vec<_> = svc.context().segments.iter().map(|s| s.kind).collect();
        assert_eq!(kinds, vec![
            DateSegmentKind::Day,
            DateSegmentKind::Literal,
            DateSegmentKind::Month,
            DateSegmentKind::Literal,
            DateSegmentKind::Year,
        ]);
    }

    #[test]
    fn disabled_field_ignores_events() {
        let mut props = Props { disabled: true, ..default_props() };
        let mut svc = Service::new(props, Env::default(), Default::default());
        svc.send(Event::FocusSegment(DateSegmentKind::Month));
        assert_eq!(*svc.state(), State::Idle);
    }

    #[test]
    fn set_value_rebuilds_segments() {
        let mut svc = make_service(DateGranularity::Day);
        let date = CalendarDate::new_gregorian(2024, nzu8(3), nzu8(15));
        svc.send(Event::SetValue(Some(date.clone())));
        assert_eq!(svc.context().get_segment_value(DateSegmentKind::Year),  Some(2024));
        assert_eq!(svc.context().get_segment_value(DateSegmentKind::Month), Some(3));
        assert_eq!(svc.context().get_segment_value(DateSegmentKind::Day),   Some(15));
    }
}
```
