//! Locale-aware date formatting helpers.
//!
//! The public API stays stable across ICU4X, browser `Intl`, and fallback
//! builds. Backend selection is internal to this module.

#[cfg(any(
    feature = "icu4x",
    all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")),
    not(any(feature = "icu4x", feature = "web-intl")),
    all(
        feature = "web-intl",
        not(target_arch = "wasm32"),
        not(feature = "icu4x")
    )
))]
use alloc::format;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::{self, Debug};

#[cfg(feature = "icu4x")]
use icu::datetime::{
    DateTimeFormatter as IcuDateTimeFormatter, DateTimeFormatterPreferences,
    fieldsets::{T, YMD, YMDE},
};
use unicode_segmentation::UnicodeSegmentation;
#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
use {
    js_sys::{
        Array, Date as JsDate, Function,
        Intl::{DateTimeFormat as JsDateTimeFormat, DateTimeFormatOptions, DateTimeStyle},
    },
    wasm_bindgen::JsValue,
};

#[cfg(feature = "std")]
use crate::CalendarDateFields;
#[cfg(feature = "std")]
use crate::calendar::ZonedDateTime;
#[cfg(any(feature = "icu4x", all(feature = "web-intl", target_arch = "wasm32")))]
use crate::calendar::internal::CalendarDate as InternalCalendarDate;
#[cfg(feature = "std")]
use crate::to_zoned_date_time;
use crate::{
    CalendarDate, CalendarDateTime, CalendarSystem, Era, HourCycle, IcuProvider, Locale, Time,
    TimeZoneId, Weekday, default_provider,
};

/// Length of the formatted date/time string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FormatLength {
    /// Full date with weekday.
    Full,

    /// Long month name date.
    Long,

    /// Medium abbreviated date.
    #[default]
    Medium,

    /// Short numeric date.
    Short,
}

impl FormatLength {
    /// Returns the ICU4X field set for date-only formatting.
    #[cfg(feature = "icu4x")]
    fn to_icu_date_field_set(self) -> YMD {
        debug_assert!(
            !self.is_full(),
            "full format must use to_icu_full_date_field_set"
        );

        match self {
            Self::Full | Self::Long => YMD::long(),
            Self::Medium => YMD::medium(),
            Self::Short => YMD::short(),
        }
    }

    /// Returns the ICU4X field set for weekday-inclusive full dates.
    #[cfg(feature = "icu4x")]
    const fn to_icu_full_date_field_set(self) -> YMDE {
        YMDE::long()
    }

    /// Returns `true` when this format length includes the weekday.
    #[must_use]
    pub const fn is_full(self) -> bool {
        matches!(self, Self::Full)
    }

    /// Returns the ICU4X field set for time-only formatting.
    #[cfg(feature = "icu4x")]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "time field-set support is part of the public formatter contract even though date-only formatting is implemented in this task"
        )
    )]
    fn to_icu_time_field_set(self) -> T {
        match self {
            Self::Full | Self::Long => T::hms(),
            Self::Medium | Self::Short => T::hm(),
        }
    }

    #[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
    const fn to_js_date_style(self) -> DateTimeStyle {
        match self {
            Self::Full => DateTimeStyle::Full,
            Self::Long => DateTimeStyle::Long,
            Self::Medium => DateTimeStyle::Medium,
            Self::Short => DateTimeStyle::Short,
        }
    }
}

/// Width used for textual date fields such as weekdays or eras.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextWidth {
    /// A single-character or compact stand-alone form when available.
    Narrow,

    /// An abbreviated textual form.
    Short,

    /// A fully spelled-out textual form.
    Long,
}

/// Width used for numeric date or time fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericWidth {
    /// Render the field without zero padding.
    Numeric,

    /// Render the field with two-digit zero padding.
    TwoDigit,
}

/// Month rendering style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonthFormat {
    /// Render the month as an unpadded number.
    Numeric,

    /// Render the month as a two-digit number.
    TwoDigit,

    /// Render the month using a narrow textual label.
    Narrow,

    /// Render the month using an abbreviated textual label.
    Short,

    /// Render the month using a long textual label.
    Long,
}

/// Time-zone label style for formatter output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeZoneNameFormat {
    /// Use a compact human-readable time-zone label.
    Short,

    /// Use a long human-readable time-zone label.
    Long,

    /// Use a compact numeric UTC offset label.
    ShortOffset,

    /// Use a long numeric UTC offset label.
    LongOffset,
}

/// Formatter options shared across ICU4X, browser `Intl`, and fallback builds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DateFormatterOptions {
    /// Style-based date formatting shortcut.
    pub date_style: Option<FormatLength>,

    /// Style-based time formatting shortcut.
    pub time_style: Option<FormatLength>,

    /// Explicit weekday formatting width.
    pub weekday: Option<TextWidth>,

    /// Explicit era formatting width.
    pub era: Option<TextWidth>,

    /// Explicit year formatting width.
    pub year: Option<NumericWidth>,

    /// Explicit month formatting style.
    pub month: Option<MonthFormat>,

    /// Explicit day-of-month formatting width.
    pub day: Option<NumericWidth>,

    /// Explicit hour formatting width.
    pub hour: Option<NumericWidth>,

    /// Explicit minute formatting width.
    pub minute: Option<NumericWidth>,

    /// Explicit second formatting width.
    pub second: Option<NumericWidth>,

    /// Explicit hour-cycle override.
    pub hour_cycle: Option<HourCycle>,

    /// Explicit display time-zone override.
    pub time_zone: Option<TimeZoneId>,

    /// Explicit time-zone label style.
    pub time_zone_name: Option<TimeZoneNameFormat>,

    /// Explicit display calendar override.
    pub calendar: Option<CalendarSystem>,

    /// Explicit numbering-system identifier.
    pub numbering_system: Option<String>,
}

impl DateFormatterOptions {
    fn simple_date_style_only(&self) -> Option<FormatLength> {
        (self.time_style.is_none()
            && self.weekday.is_none()
            && self.era.is_none()
            && self.year.is_none()
            && self.month.is_none()
            && self.day.is_none()
            && self.hour.is_none()
            && self.minute.is_none()
            && self.second.is_none()
            && self.hour_cycle.is_none()
            && self.time_zone.is_none()
            && self.time_zone_name.is_none()
            && self.calendar.is_none()
            && self.numbering_system.is_none())
        .then_some(self.date_style)
        .flatten()
    }
}

/// Semantic kind for one formatted date/time part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateFormatterPartKind {
    /// An era label.
    Era,

    /// A year field.
    Year,

    /// A month field.
    Month,

    /// A day-of-month field.
    Day,

    /// A weekday field.
    Weekday,

    /// An AM/PM or related day-period field.
    DayPeriod,

    /// An hour field.
    Hour,

    /// A minute field.
    Minute,

    /// A second field.
    Second,

    /// A fractional-second field.
    FractionalSecond,

    /// A time-zone name or offset field.
    TimeZoneName,

    /// A literal separator or punctuation segment.
    Literal,
}

/// One formatted date/time part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateFormatterPart {
    /// The semantic kind of this part.
    pub kind: DateFormatterPartKind,

    /// The rendered text for this part.
    pub value: String,
}

/// Source classification for one formatted range part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateRangePartSource {
    /// The part is shared by both the start and end of the range.
    Shared,

    /// The part belongs only to the start value.
    Start,

    /// The part belongs only to the end value.
    End,
}

/// One formatted date/time range part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateRangeFormatterPart {
    /// The semantic kind of this part.
    pub kind: DateFormatterPartKind,

    /// The rendered text for this part.
    pub value: String,

    /// Whether the part came from the start, end, or shared range context.
    pub source: DateRangePartSource,
}

/// The effective formatter configuration after locale defaults and explicit overrides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedDateFormatterOptions {
    /// The formatter locale.
    pub locale: Locale,

    /// The effective display calendar.
    pub calendar: CalendarSystem,

    /// The effective numbering-system identifier, when one is available.
    pub numbering_system: Option<String>,

    /// The effective display time zone, when one is configured.
    pub time_zone: Option<TimeZoneId>,

    /// The effective hour cycle, when time formatting is in use.
    pub hour_cycle: Option<HourCycle>,

    /// The effective date style shortcut, when one is configured.
    pub date_style: Option<FormatLength>,

    /// The effective time style shortcut, when one is configured.
    pub time_style: Option<FormatLength>,
}

/// Internal ICU4X formatter storage.
#[cfg(feature = "icu4x")]
#[derive(Debug)]
enum DateFormatterInner {
    Ymd(IcuDateTimeFormatter<YMD>),
    Ymde(IcuDateTimeFormatter<YMDE>),
}

/// A locale-aware date formatter.
pub struct DateFormatter {
    locale: Locale,
    options: DateFormatterOptions,
    resolved: ResolvedDateFormatterOptions,
    simple_length: Option<FormatLength>,

    #[cfg(feature = "icu4x")]
    inner: Option<DateFormatterInner>,

    #[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
    inner: Option<JsDateTimeFormat>,
}

impl Debug for DateFormatter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DateFormatter")
            .field("locale", &self.locale)
            .field("options", &self.options)
            .field("resolved", &self.resolved)
            .finish()
    }
}

impl DateFormatter {
    /// Creates a new locale-aware date formatter.
    #[must_use]
    pub fn new(locale: &Locale, length: FormatLength) -> Self {
        Self::new_with_options(
            locale,
            DateFormatterOptions {
                date_style: Some(length),
                ..DateFormatterOptions::default()
            },
        )
    }

    /// Creates a new locale-aware formatter using explicit date/time field options.
    #[must_use]
    pub fn new_with_options(locale: &Locale, options: DateFormatterOptions) -> Self {
        let simple_length = options.simple_date_style_only();

        Self {
            locale: locale.clone(),
            resolved: resolve_options(locale, &options),
            options,
            simple_length,
            #[cfg(feature = "icu4x")]
            inner: simple_length.map(|length| build_icu_date_formatter(locale, length)),
            #[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
            inner: simple_length.map(|length| build_js_date_formatter(locale, length)),
        }
    }

    /// Formats a calendar date for the formatter locale.
    #[must_use]
    pub fn format(&self, date: &CalendarDate) -> String {
        if let Some(length) = self.simple_length {
            return self.format_simple(date, length);
        }

        self.format_date(date)
    }

    /// Formats a calendar date using the option-based formatter contract.
    #[must_use]
    pub fn format_date(&self, value: &CalendarDate) -> String {
        if let Some(length) = self.simple_length {
            return self.format_simple(value, length);
        }

        join_parts(&self.format_date_to_parts(value))
    }

    /// Formats a calendar date-time using the option-based formatter contract.
    #[must_use]
    pub fn format_date_time(&self, value: &CalendarDateTime) -> String {
        join_parts(&self.format_date_time_to_parts(value))
    }

    /// Formats a wall-clock time using the option-based formatter contract.
    #[must_use]
    pub fn format_time(&self, value: &Time) -> String {
        join_parts(&self.format_time_to_parts(value))
    }

    /// Formats a zoned date-time using the option-based formatter contract.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn format_zoned(&self, value: &ZonedDateTime) -> String {
        join_parts(&self.format_zoned_to_parts(value))
    }

    /// Formats a calendar date range using the option-based formatter contract.
    #[must_use]
    pub fn format_date_range(&self, start: &CalendarDate, end: &CalendarDate) -> String {
        join_range_parts(&self.format_date_range_to_parts(start, end))
    }

    /// Formats a calendar date-time range using the option-based formatter contract.
    #[must_use]
    pub fn format_date_time_range(
        &self,
        start: &CalendarDateTime,
        end: &CalendarDateTime,
    ) -> String {
        join_range_parts(&self.format_date_time_range_to_parts(start, end))
    }

    /// Formats a zoned date-time range using the option-based formatter contract.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn format_zoned_range(&self, start: &ZonedDateTime, end: &ZonedDateTime) -> String {
        join_range_parts(&self.format_zoned_range_to_parts(start, end))
    }

    /// Formats a calendar date into semantic parts.
    #[must_use]
    pub fn format_date_to_parts(&self, value: &CalendarDate) -> Vec<DateFormatterPart> {
        let provider = default_provider();

        let date = self.convert_date_for_display(value);

        render_date_parts(
            provider.as_ref(),
            &self.locale,
            &date,
            self.effective_date_fields(),
        )
    }

    /// Formats a calendar date-time into semantic parts.
    #[must_use]
    pub fn format_date_time_to_parts(&self, value: &CalendarDateTime) -> Vec<DateFormatterPart> {
        let provider = default_provider();

        let date = self.convert_date_for_display(value.date());

        let mut parts = render_date_parts(
            provider.as_ref(),
            &self.locale,
            &date,
            self.effective_date_fields(),
        );

        let time_parts = render_time_parts(
            provider.as_ref(),
            &self.locale,
            value.time(),
            self.effective_time_fields(),
            self.resolved.time_zone.as_ref(),
            #[cfg(feature = "std")]
            self.resolved.time_zone.as_ref().and_then(|time_zone| {
                to_zoned_date_time(value, time_zone, crate::Disambiguation::Compatible)
                    .ok()
                    .map(|zoned| zoned.offset_minutes())
            }),
            #[cfg(not(feature = "std"))]
            None,
        );

        if !parts.is_empty() && !time_parts.is_empty() {
            parts.push(literal_part(", "));
        }

        parts.extend(time_parts);

        parts
    }

    /// Formats a wall-clock time into semantic parts.
    #[must_use]
    pub fn format_time_to_parts(&self, value: &Time) -> Vec<DateFormatterPart> {
        let provider = default_provider();

        render_time_parts(
            provider.as_ref(),
            &self.locale,
            value,
            self.effective_time_fields(),
            self.resolved.time_zone.as_ref(),
            None,
        )
    }

    /// Formats a zoned date-time into semantic parts.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn format_zoned_to_parts(&self, value: &ZonedDateTime) -> Vec<DateFormatterPart> {
        let provider = default_provider();

        let zoned = self.convert_zoned_for_display(value);

        let date = date_from_zoned(&zoned);

        let time = time_from_zoned(&zoned);

        let mut parts = render_date_parts(
            provider.as_ref(),
            &self.locale,
            &date,
            self.effective_date_fields(),
        );

        let time_parts = render_time_parts(
            provider.as_ref(),
            &self.locale,
            &time,
            self.effective_time_fields(),
            Some(zoned.time_zone()),
            Some(zoned.offset_minutes()),
        );

        if !parts.is_empty() && !time_parts.is_empty() {
            parts.push(literal_part(", "));
        }

        parts.extend(time_parts);

        parts
    }

    /// Formats a calendar date range into semantic parts with source metadata.
    #[must_use]
    pub fn format_date_range_to_parts(
        &self,
        start: &CalendarDate,
        end: &CalendarDate,
    ) -> Vec<DateRangeFormatterPart> {
        let start_parts = self.format_date_to_parts(start);

        let end_parts = self.format_date_to_parts(end);

        range_parts_from_parts(&start_parts, &end_parts)
    }

    /// Formats a calendar date-time range into semantic parts with source metadata.
    #[must_use]
    pub fn format_date_time_range_to_parts(
        &self,
        start: &CalendarDateTime,
        end: &CalendarDateTime,
    ) -> Vec<DateRangeFormatterPart> {
        let start_parts = self.format_date_time_to_parts(start);

        let end_parts = self.format_date_time_to_parts(end);

        range_parts_from_parts(&start_parts, &end_parts)
    }

    /// Formats a zoned date-time range into semantic parts with source metadata.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn format_zoned_range_to_parts(
        &self,
        start: &ZonedDateTime,
        end: &ZonedDateTime,
    ) -> Vec<DateRangeFormatterPart> {
        let start_parts = self.format_zoned_to_parts(start);

        let end_parts = self.format_zoned_to_parts(end);

        range_parts_from_parts(&start_parts, &end_parts)
    }

    /// Returns the effective formatter configuration after locale resolution.
    #[must_use]
    pub fn resolved_options(&self) -> ResolvedDateFormatterOptions {
        self.resolved.clone()
    }

    fn effective_date_fields(&self) -> EffectiveDateFields {
        let mut fields = self
            .options
            .date_style
            .map(date_fields_from_style)
            .unwrap_or_default();

        if let Some(weekday) = self.options.weekday {
            fields.weekday = Some(weekday);
        }

        if let Some(era) = self.options.era {
            fields.era = Some(era);
        }

        if let Some(year) = self.options.year {
            fields.year = Some(year);
        }

        if let Some(month) = self.options.month {
            fields.month = Some(month);
        }

        if let Some(day) = self.options.day {
            fields.day = Some(day);
        }

        fields
    }

    fn effective_time_fields(&self) -> EffectiveTimeFields {
        let hour_cycle = self.resolved.hour_cycle.unwrap_or(HourCycle::H23);

        let mut fields = self.options.time_style.map_or(
            EffectiveTimeFields {
                hour_cycle,
                ..EffectiveTimeFields::default()
            },
            |style| time_fields_from_style(style, hour_cycle),
        );

        if let Some(hour) = self.options.hour {
            fields.hour = Some(hour);
        }

        if let Some(minute) = self.options.minute {
            fields.minute = Some(minute);
        }

        if let Some(second) = self.options.second {
            fields.second = Some(second);
        }

        if let Some(time_zone_name) = self.options.time_zone_name {
            fields.time_zone_name = Some(time_zone_name);
        }

        fields.hour_cycle = hour_cycle;

        fields
    }

    fn convert_date_for_display(&self, value: &CalendarDate) -> CalendarDate {
        if value.calendar() == self.resolved.calendar {
            value.clone()
        } else {
            value
                .to_calendar(self.resolved.calendar)
                .unwrap_or_else(|_| value.clone())
        }
    }

    #[cfg(feature = "std")]
    fn convert_zoned_for_display(&self, value: &ZonedDateTime) -> ZonedDateTime {
        let with_zone = if let Some(time_zone) = &self.resolved.time_zone {
            value
                .with_time_zone(time_zone.clone())
                .unwrap_or_else(|_| value.clone())
        } else {
            value.clone()
        };

        if with_zone.calendar == self.resolved.calendar {
            with_zone
        } else {
            with_zone
                .to_calendar(self.resolved.calendar)
                .unwrap_or(with_zone)
        }
    }

    fn format_simple(&self, date: &CalendarDate, length: FormatLength) -> String {
        #[cfg(feature = "icu4x")]
        {
            if let Some(inner) = &self.inner {
                return format_icu_date(&self.locale, length, inner, date);
            }
        }

        #[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
        {
            if let Some(inner) = &self.inner {
                return format_js_date(&self.locale, length, inner, date);
            }
        }

        fallback_format_date(&self.locale, length, date)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct EffectiveDateFields {
    weekday: Option<TextWidth>,
    era: Option<TextWidth>,
    year: Option<NumericWidth>,
    month: Option<MonthFormat>,
    day: Option<NumericWidth>,
}

#[derive(Clone, Copy, Debug, Default)]
struct EffectiveTimeFields {
    hour: Option<NumericWidth>,
    minute: Option<NumericWidth>,
    second: Option<NumericWidth>,
    time_zone_name: Option<TimeZoneNameFormat>,
    hour_cycle: HourCycle,
}

fn resolve_options(
    locale: &Locale,
    options: &DateFormatterOptions,
) -> ResolvedDateFormatterOptions {
    let provider = default_provider();

    let uses_time = options.time_style.is_some()
        || options.hour.is_some()
        || options.minute.is_some()
        || options.second.is_some()
        || options.time_zone_name.is_some()
        || options.time_zone.is_some();

    ResolvedDateFormatterOptions {
        locale: locale.clone(),
        calendar: options
            .calendar
            .unwrap_or_else(|| CalendarSystem::from_locale(locale)),
        numbering_system: options.numbering_system.clone(),
        time_zone: options.time_zone.clone(),
        hour_cycle: uses_time.then(|| {
            options
                .hour_cycle
                .unwrap_or_else(|| locale.hour_cycle(provider.as_ref()))
        }),
        date_style: options.date_style,
        time_style: options.time_style,
    }
}

fn date_fields_from_style(style: FormatLength) -> EffectiveDateFields {
    match style {
        FormatLength::Full => EffectiveDateFields {
            weekday: Some(TextWidth::Long),
            year: Some(NumericWidth::Numeric),
            month: Some(MonthFormat::Long),
            day: Some(NumericWidth::Numeric),
            ..EffectiveDateFields::default()
        },

        FormatLength::Long => EffectiveDateFields {
            year: Some(NumericWidth::Numeric),
            month: Some(MonthFormat::Long),
            day: Some(NumericWidth::Numeric),
            ..EffectiveDateFields::default()
        },

        FormatLength::Medium => EffectiveDateFields {
            year: Some(NumericWidth::Numeric),
            month: Some(MonthFormat::Short),
            day: Some(NumericWidth::Numeric),
            ..EffectiveDateFields::default()
        },

        FormatLength::Short => EffectiveDateFields {
            year: Some(NumericWidth::TwoDigit),
            month: Some(MonthFormat::Numeric),
            day: Some(NumericWidth::Numeric),
            ..EffectiveDateFields::default()
        },
    }
}

fn time_fields_from_style(style: FormatLength, hour_cycle: HourCycle) -> EffectiveTimeFields {
    match style {
        FormatLength::Full => EffectiveTimeFields {
            hour: Some(NumericWidth::Numeric),
            minute: Some(NumericWidth::TwoDigit),
            second: Some(NumericWidth::TwoDigit),
            time_zone_name: Some(TimeZoneNameFormat::Long),
            hour_cycle,
        },

        FormatLength::Long => EffectiveTimeFields {
            hour: Some(NumericWidth::Numeric),
            minute: Some(NumericWidth::TwoDigit),
            second: Some(NumericWidth::TwoDigit),
            time_zone_name: Some(TimeZoneNameFormat::Short),
            hour_cycle,
        },

        FormatLength::Medium => EffectiveTimeFields {
            hour: Some(NumericWidth::Numeric),
            minute: Some(NumericWidth::TwoDigit),
            second: Some(NumericWidth::TwoDigit),
            hour_cycle,
            ..EffectiveTimeFields::default()
        },

        FormatLength::Short => EffectiveTimeFields {
            hour: Some(NumericWidth::Numeric),
            minute: Some(NumericWidth::TwoDigit),
            hour_cycle,
            ..EffectiveTimeFields::default()
        },
    }
}

#[cfg(feature = "icu4x")]
fn build_icu_date_formatter(locale: &Locale, length: FormatLength) -> DateFormatterInner {
    let prefs = DateTimeFormatterPreferences::from(locale.as_icu());

    if length.is_full() {
        DateFormatterInner::Ymde(
            IcuDateTimeFormatter::try_new(prefs, length.to_icu_full_date_field_set())
                .expect("compiled_data guarantees date formatter data for all locales"),
        )
    } else {
        DateFormatterInner::Ymd(
            IcuDateTimeFormatter::try_new(prefs, length.to_icu_date_field_set())
                .expect("compiled_data guarantees date formatter data for all locales"),
        )
    }
}

#[cfg(feature = "icu4x")]
fn format_icu_date(
    locale: &Locale,
    length: FormatLength,
    inner: &DateFormatterInner,
    date: &CalendarDate,
) -> String {
    let Ok(internal) = InternalCalendarDate::try_from(date) else {
        return fallback_format_date(locale, length, date);
    };

    match inner {
        DateFormatterInner::Ymd(formatter) => formatter.format(&internal.inner).to_string(),
        DateFormatterInner::Ymde(formatter) => formatter.format(&internal.inner).to_string(),
    }
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
fn build_js_date_formatter(locale: &Locale, length: FormatLength) -> JsDateTimeFormat {
    let locales = Array::of1(&JsValue::from_str(&locale.to_bcp47()));

    let options = DateTimeFormatOptions::new();

    options.set_date_style(length.to_js_date_style());
    options.set_time_zone("UTC");

    JsDateTimeFormat::new(&locales, options.as_ref())
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
fn format_js_date(
    locale: &Locale,
    length: FormatLength,
    inner: &JsDateTimeFormat,
    date: &CalendarDate,
) -> String {
    let Some(js_date) = js_date_from_calendar(date) else {
        return fallback_format_date(locale, length, date);
    };

    let format: Function = inner.format();

    format
        .call1(&JsValue::UNDEFINED, js_date.as_ref())
        .expect("Intl.DateTimeFormat.format should not throw for a valid Date")
        .as_string()
        .unwrap_or_default()
}

fn render_date_parts(
    provider: &dyn IcuProvider,
    locale: &Locale,
    date: &CalendarDate,
    fields: EffectiveDateFields,
) -> Vec<DateFormatterPart> {
    let mut parts = Vec::new();

    if let Some(weekday_width) = fields.weekday {
        parts.push(DateFormatterPart {
            kind: DateFormatterPartKind::Weekday,
            value: weekday_label(provider, date.weekday(), locale, weekday_width),
        });

        if fields.era.is_some()
            || fields.year.is_some()
            || fields.month.is_some()
            || fields.day.is_some()
        {
            parts.push(literal_part(", "));
        }
    }

    let mut ordered = Vec::new();

    if let Some(era_width) = fields.era
        && let Some(era) = date.era()
    {
        ordered.push(DateFormatterPart {
            kind: DateFormatterPartKind::Era,
            value: era_label(era, era_width),
        });
    }

    match date_order(locale) {
        DateOrder::MonthDayYear => {
            push_date_field(
                &mut ordered,
                fields.month.map(|month| DateFormatterPart {
                    kind: DateFormatterPartKind::Month,
                    value: format_month(provider, locale, date.month(), month),
                }),
                " ",
            );

            push_date_field(
                &mut ordered,
                fields.day.map(|day| DateFormatterPart {
                    kind: DateFormatterPartKind::Day,
                    value: format_numeric(i32::from(date.day()), day),
                }),
                " ",
            );

            push_date_field(
                &mut ordered,
                fields.year.map(|year| DateFormatterPart {
                    kind: DateFormatterPartKind::Year,
                    value: format_year(date.year(), year),
                }),
                ", ",
            );
        }

        DateOrder::DayMonthYear => {
            push_date_field(
                &mut ordered,
                fields.day.map(|day| DateFormatterPart {
                    kind: DateFormatterPartKind::Day,
                    value: format_numeric(i32::from(date.day()), day),
                }),
                " ",
            );

            push_date_field(
                &mut ordered,
                fields.month.map(|month| DateFormatterPart {
                    kind: DateFormatterPartKind::Month,
                    value: format_month(provider, locale, date.month(), month),
                }),
                " ",
            );

            push_date_field(
                &mut ordered,
                fields.year.map(|year| DateFormatterPart {
                    kind: DateFormatterPartKind::Year,
                    value: format_year(date.year(), year),
                }),
                " ",
            );
        }

        DateOrder::YearMonthDay => {
            push_date_field(
                &mut ordered,
                fields.year.map(|year| DateFormatterPart {
                    kind: DateFormatterPartKind::Year,
                    value: format_year(date.year(), year),
                }),
                " ",
            );

            push_date_field(
                &mut ordered,
                fields.month.map(|month| DateFormatterPart {
                    kind: DateFormatterPartKind::Month,
                    value: format_month(provider, locale, date.month(), month),
                }),
                " ",
            );

            push_date_field(
                &mut ordered,
                fields.day.map(|day| DateFormatterPart {
                    kind: DateFormatterPartKind::Day,
                    value: format_numeric(i32::from(date.day()), day),
                }),
                " ",
            );
        }
    }

    parts.extend(ordered);

    parts
}

fn render_time_parts(
    provider: &dyn IcuProvider,
    locale: &Locale,
    time: &Time,
    fields: EffectiveTimeFields,
    time_zone: Option<&TimeZoneId>,
    utc_offset_minutes: Option<i32>,
) -> Vec<DateFormatterPart> {
    let mut parts = Vec::new();

    if let Some(hour) = fields.hour {
        parts.push(DateFormatterPart {
            kind: DateFormatterPartKind::Hour,
            value: format_hour(time.hour(), hour, fields.hour_cycle),
        });
    }

    if let Some(minute) = fields.minute {
        if !parts.is_empty() {
            parts.push(literal_part(":"));
        }

        parts.push(DateFormatterPart {
            kind: DateFormatterPartKind::Minute,
            value: format_numeric(i32::from(time.minute()), minute),
        });
    }

    if let Some(second) = fields.second {
        if !parts.is_empty() {
            parts.push(literal_part(":"));
        }

        parts.push(DateFormatterPart {
            kind: DateFormatterPartKind::Second,
            value: format_numeric(i32::from(time.second()), second),
        });
    }
    if matches!(fields.hour_cycle, HourCycle::H11 | HourCycle::H12) && fields.hour.is_some() {
        parts.push(literal_part(" "));

        parts.push(DateFormatterPart {
            kind: DateFormatterPartKind::DayPeriod,
            value: provider.day_period_label(time.hour() >= 12, locale),
        });
    }
    if let Some(zone_format) = fields.time_zone_name
        && let Some(time_zone) = time_zone
    {
        parts.push(literal_part(" "));

        parts.push(DateFormatterPart {
            kind: DateFormatterPartKind::TimeZoneName,
            value: format_time_zone_name(time_zone, zone_format, utc_offset_minutes),
        });
    }

    parts
}

fn join_parts(parts: &[DateFormatterPart]) -> String {
    parts.iter().fold(String::new(), |mut output, part| {
        output.push_str(&part.value);
        output
    })
}

fn join_range_parts(parts: &[DateRangeFormatterPart]) -> String {
    parts.iter().fold(String::new(), |mut output, part| {
        output.push_str(&part.value);
        output
    })
}

fn range_parts_from_parts(
    start: &[DateFormatterPart],
    end: &[DateFormatterPart],
) -> Vec<DateRangeFormatterPart> {
    if start == end {
        return start
            .iter()
            .cloned()
            .map(|part| DateRangeFormatterPart {
                kind: part.kind,
                value: part.value,
                source: DateRangePartSource::Shared,
            })
            .collect();
    }

    let prefix_len = start
        .iter()
        .zip(end.iter())
        .take_while(|(lhs, rhs)| lhs == rhs)
        .count();

    let start_tail = &start[prefix_len..];

    let end_tail = &end[prefix_len..];

    let suffix_len = start_tail
        .iter()
        .rev()
        .zip(end_tail.iter().rev())
        .take_while(|(lhs, rhs)| lhs == rhs)
        .count();

    let start_mid_end = start.len().saturating_sub(suffix_len);

    let end_mid_end = end.len().saturating_sub(suffix_len);

    let mut parts = Vec::new();

    parts.extend(
        start[..prefix_len]
            .iter()
            .cloned()
            .map(|part| range_part(part, DateRangePartSource::Shared)),
    );

    parts.extend(
        start[prefix_len..start_mid_end]
            .iter()
            .cloned()
            .map(|part| range_part(part, DateRangePartSource::Start)),
    );

    if prefix_len < start_mid_end || prefix_len < end_mid_end {
        parts.push(DateRangeFormatterPart {
            kind: DateFormatterPartKind::Literal,
            value: String::from(" - "),
            source: DateRangePartSource::Shared,
        });
    }

    parts.extend(
        end[prefix_len..end_mid_end]
            .iter()
            .cloned()
            .map(|part| range_part(part, DateRangePartSource::End)),
    );

    parts.extend(
        start[start_mid_end..]
            .iter()
            .cloned()
            .map(|part| range_part(part, DateRangePartSource::Shared)),
    );

    parts
}

fn range_part(part: DateFormatterPart, source: DateRangePartSource) -> DateRangeFormatterPart {
    DateRangeFormatterPart {
        kind: part.kind,
        value: part.value,
        source,
    }
}

fn literal_part(value: &str) -> DateFormatterPart {
    DateFormatterPart {
        kind: DateFormatterPartKind::Literal,
        value: String::from(value),
    }
}

fn push_date_field(
    output: &mut Vec<DateFormatterPart>,
    part: Option<DateFormatterPart>,
    separator: &str,
) {
    if let Some(part) = part {
        if !output.is_empty() {
            output.push(literal_part(separator));
        }

        output.push(part);
    }
}

#[derive(Clone, Copy)]
enum DateOrder {
    MonthDayYear,
    DayMonthYear,
    YearMonthDay,
}

fn date_order(locale: &Locale) -> DateOrder {
    match locale.language() {
        "ja" | "zh" | "ko" => DateOrder::YearMonthDay,

        _ if matches!(locale.region(), Some("US" | "FM" | "PW")) => DateOrder::MonthDayYear,

        _ => DateOrder::DayMonthYear,
    }
}

fn format_year(year: i32, width: NumericWidth) -> String {
    match width {
        NumericWidth::Numeric => year.to_string(),
        NumericWidth::TwoDigit => format!("{:02}", year.rem_euclid(100)),
    }
}

fn format_numeric(value: i32, width: NumericWidth) -> String {
    match width {
        NumericWidth::Numeric => value.to_string(),
        NumericWidth::TwoDigit => format!("{value:02}"),
    }
}

fn format_hour(hour: u8, width: NumericWidth, cycle: HourCycle) -> String {
    let display = match cycle {
        HourCycle::H11 => hour % 12,

        HourCycle::H12 => {
            let modulo = hour % 12;
            if modulo == 0 { 12 } else { modulo }
        }

        HourCycle::H23 => hour,

        HourCycle::H24 => {
            if hour == 0 {
                24
            } else {
                hour
            }
        }
    };

    format_numeric(i32::from(display), width)
}

fn format_month(
    provider: &dyn IcuProvider,
    locale: &Locale,
    month: u8,
    format_kind: MonthFormat,
) -> String {
    match format_kind {
        MonthFormat::Numeric => month.to_string(),
        MonthFormat::TwoDigit => format!("{month:02}"),
        MonthFormat::Long => provider.month_long_name(month, locale),
        MonthFormat::Short => truncate_graphemes(&provider.month_long_name(month, locale), 3),
        MonthFormat::Narrow => truncate_graphemes(&provider.month_long_name(month, locale), 1),
    }
}

fn weekday_label(
    provider: &dyn IcuProvider,
    weekday: Weekday,
    locale: &Locale,
    width: TextWidth,
) -> String {
    match width {
        TextWidth::Long => provider.weekday_long_label(weekday, locale),
        TextWidth::Short => provider.weekday_short_label(weekday, locale),
        TextWidth::Narrow => truncate_graphemes(&provider.weekday_short_label(weekday, locale), 1),
    }
}

fn era_label(era: &Era, width: TextWidth) -> String {
    match width {
        TextWidth::Long | TextWidth::Short => era.display_name.clone(),
        TextWidth::Narrow => truncate_graphemes(&era.display_name, 1),
    }
}

fn truncate_graphemes(value: &str, count: usize) -> String {
    value.graphemes(true).take(count).collect()
}

fn format_time_zone_name(
    time_zone: &TimeZoneId,
    format_kind: TimeZoneNameFormat,
    utc_offset_minutes: Option<i32>,
) -> String {
    match format_kind {
        TimeZoneNameFormat::Short | TimeZoneNameFormat::Long => String::from(time_zone.as_str()),
        TimeZoneNameFormat::ShortOffset => utc_offset_minutes.map_or_else(
            || format!("GMT {}", time_zone.as_str()),
            |minutes| format_utc_offset(minutes, false),
        ),
        TimeZoneNameFormat::LongOffset => utc_offset_minutes.map_or_else(
            || format!("GMT offset {}", time_zone.as_str()),
            |minutes| format_utc_offset(minutes, true),
        ),
    }
}

fn format_utc_offset(utc_offset_minutes: i32, long: bool) -> String {
    if utc_offset_minutes == 0 {
        return String::from("GMT");
    }

    let sign = if utc_offset_minutes.is_negative() {
        '-'
    } else {
        '+'
    };
    let absolute_minutes = utc_offset_minutes.abs();
    let hours = absolute_minutes / 60;
    let minutes = absolute_minutes % 60;

    if long {
        return format!("GMT{sign}{hours:02}:{minutes:02}");
    }

    if minutes == 0 {
        return format!("GMT{sign}{hours}");
    }

    format!("GMT{sign}{hours}:{minutes:02}")
}

#[cfg(feature = "std")]
fn date_from_zoned(value: &ZonedDateTime) -> CalendarDate {
    let era = value.inner.era().map(|code| {
        let code = code.to_string();

        Era {
            display_name: title_case_ascii(&code),
            code,
        }
    });

    CalendarDate::new(
        value.calendar,
        &CalendarDateFields {
            era,
            year: Some(value.inner.era_year().unwrap_or_else(|| value.inner.year())),
            month: Some(value.inner.month()),
            day: Some(value.inner.day()),
            ..CalendarDateFields::default()
        },
    )
    .unwrap_or_else(|_| {
        CalendarDate::new_gregorian(value.inner.year(), value.inner.month(), value.inner.day())
            .expect("temporal zoned date fields should always form a valid Gregorian fallback")
    })
}

#[cfg(feature = "std")]
fn time_from_zoned(value: &ZonedDateTime) -> Time {
    Time::new(
        value.inner.hour(),
        value.inner.minute(),
        value.inner.second(),
        value.inner.millisecond(),
    )
    .expect("temporal zoned time fields should always be valid")
}

#[cfg(feature = "std")]
fn title_case_ascii(value: &str) -> String {
    let mut chars = value.chars();

    let Some(first) = chars.next() else {
        return String::new();
    };

    let mut result = String::new();

    result.push(first.to_ascii_uppercase());

    result.extend(chars);

    result
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
fn js_date_from_calendar(date: &CalendarDate) -> Option<JsDate> {
    if date.calendar == CalendarSystem::Gregorian {
        let js_date = js_date_from_ymd(date.iso_year, date.iso_month, date.iso_day);

        return js_date_is_valid(&js_date).then_some(js_date);
    }

    let internal = InternalCalendarDate::try_from(date).ok()?;

    let gregorian = internal.to_calendar(CalendarSystem::Gregorian);

    let js_date = js_date_from_ymd(gregorian.year(), gregorian.month(), gregorian.day());

    js_date_is_valid(&js_date).then_some(js_date)
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
fn js_date_from_ymd(year: i32, month: u8, day: u8) -> JsDate {
    let iso = format!("{}-{month:02}-{day:02}T12:00:00.000Z", js_iso_year(year),);

    JsDate::new(&JsValue::from_str(&iso))
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
fn js_date_is_valid(date: &JsDate) -> bool {
    !date.get_time().is_nan()
}

#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
fn js_iso_year(year: i32) -> String {
    if (0..=9_999).contains(&year) {
        format!("{year:04}")
    } else if year < 0 {
        format!("-{:06}", year.unsigned_abs())
    } else {
        format!("+{year:06}")
    }
}

fn fallback_format_date(_locale: &Locale, length: FormatLength, date: &CalendarDate) -> String {
    if date.calendar != CalendarSystem::Gregorian {
        return date.to_iso8601();
    }

    let year = date.year;

    let month = date.month;

    let day = date.day;

    match length {
        FormatLength::Short => format!("{month}/{day}/{:02}", year.rem_euclid(100)),

        FormatLength::Medium => {
            format!("{} {day}, {year}", english_month_short(month))
        }

        FormatLength::Long => {
            format!("{} {day}, {year}", english_month_long(month))
        }

        FormatLength::Full => {
            format!(
                "{}, {} {day}, {year}",
                english_weekday_long(date.weekday()),
                english_month_long(month)
            )
        }
    }
}

const fn english_month_short(month: u8) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "???",
    }
}

const fn english_month_long(month: u8) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}

const fn english_weekday_long(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Sunday => "Sunday",
        Weekday::Monday => "Monday",
        Weekday::Tuesday => "Tuesday",
        Weekday::Wednesday => "Wednesday",
        Weekday::Thursday => "Thursday",
        Weekday::Friday => "Friday",
        Weekday::Saturday => "Saturday",
    }
}

#[cfg(test)]
mod tests {
    #[cfg(any(
        feature = "icu4x",
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    use alloc::format;
    #[cfg(any(
        feature = "icu4x",
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    use alloc::string::String;

    #[cfg(feature = "icu4x")]
    use icu::datetime::fieldsets::{T, YMD, YMDE};

    #[cfg(any(
        feature = "icu4x",
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    use super::{
        DateFormatter, FormatLength, english_month_long, english_month_short, english_weekday_long,
        fallback_format_date,
    };
    #[cfg(feature = "icu4x")]
    use super::{
        DateFormatterOptions, DateFormatterPart, DateFormatterPartKind, DateRangePartSource,
        EffectiveDateFields, EffectiveTimeFields, MonthFormat, NumericWidth, TextWidth,
        TimeZoneNameFormat, date_fields_from_style, join_parts, join_range_parts,
        range_parts_from_parts, render_date_parts, render_time_parts, resolve_options,
        time_fields_from_style,
    };
    #[cfg(all(feature = "std", feature = "icu4x"))]
    use super::{
        DateOrder, date_from_zoned, date_order, era_label, format_hour, format_month,
        format_numeric, format_time_zone_name, format_utc_offset, format_year, literal_part,
        push_date_field, time_from_zoned, title_case_ascii, truncate_graphemes, weekday_label,
    };
    #[cfg(feature = "icu4x")]
    use crate::StubIcuProvider;
    #[cfg(any(
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    use crate::Weekday;
    #[cfg(all(feature = "std", feature = "icu4x"))]
    use crate::calendar::ZonedDateTime;
    #[cfg(any(
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    use crate::locales;
    #[cfg(any(
        feature = "icu4x",
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    use crate::{CalendarDate, CalendarSystem, Era};
    #[cfg(feature = "icu4x")]
    use crate::{
        CalendarDateTime, CalendarSystem as PublicCalendarSystem, HourCycle, Locale, Time,
        TimeZoneId, Weekday, locales,
    };

    #[cfg(feature = "icu4x")]
    fn march_2024() -> CalendarDate {
        CalendarDate::new_gregorian(2024, 3, 15).expect("fixture should validate")
    }

    #[cfg(any(
        feature = "icu4x",
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    fn japanese_reiwa_date() -> CalendarDate {
        CalendarDate::new(
            CalendarSystem::Japanese,
            &crate::CalendarDateFields {
                era: Some(Era {
                    code: String::from("reiwa"),
                    display_name: String::from("Reiwa"),
                }),
                year: Some(1),
                month: Some(5),
                day: Some(1),
                ..crate::CalendarDateFields::default()
            },
        )
        .expect("fixture should validate")
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn short_date_formatter_formats_en_us_date() {
        let formatter = DateFormatter::new(&locales::en_us(), FormatLength::Short);

        assert_eq!(formatter.format(&march_2024()), "3/15/24");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn format_length_icu_field_sets_cover_time_and_full_variants() {
        assert_eq!(
            FormatLength::Full.to_icu_full_date_field_set(),
            YMDE::long()
        );
        assert_eq!(FormatLength::Long.to_icu_date_field_set(), YMD::long());
        assert_eq!(FormatLength::Medium.to_icu_date_field_set(), YMD::medium());
        assert_eq!(FormatLength::Short.to_icu_date_field_set(), YMD::short());
        assert_eq!(FormatLength::Full.to_icu_time_field_set(), T::hms());
        assert_eq!(FormatLength::Long.to_icu_time_field_set(), T::hms());
        assert_eq!(FormatLength::Medium.to_icu_time_field_set(), T::hm());
        assert_eq!(FormatLength::Short.to_icu_time_field_set(), T::hm());
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn full_date_formatter_includes_weekday_name() {
        let formatter = DateFormatter::new(&locales::en_us(), FormatLength::Full);

        let formatted = formatter.format(&march_2024());

        assert!(formatted.contains("Friday"));
        assert!(formatted.contains("March"));
        assert!(formatted.contains("2024"));
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn date_formatter_supports_representative_locales() {
        let english = DateFormatter::new(&locales::en_us(), FormatLength::Medium);

        let german = DateFormatter::new(
            &Locale::parse("de-DE").expect("locale should parse"),
            FormatLength::Medium,
        );

        let arabic = DateFormatter::new(
            &Locale::parse("ar-SA").expect("locale should parse"),
            FormatLength::Medium,
        );

        let japanese = DateFormatter::new(
            &Locale::parse("ja-JP").expect("locale should parse"),
            FormatLength::Medium,
        );

        assert_ne!(english.format(&march_2024()), german.format(&march_2024()));
        assert_ne!(english.format(&march_2024()), arabic.format(&march_2024()));
        assert_ne!(
            english.format(&march_2024()),
            japanese.format(&march_2024())
        );
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn non_gregorian_japanese_dates_format_with_era_year() {
        let formatter = DateFormatter::new(
            &Locale::parse("en-US-u-ca-japanese").expect("locale should parse"),
            FormatLength::Long,
        );

        let date = japanese_reiwa_date();

        let formatted = formatter.format(&date);

        assert!(formatted.contains("Reiwa"));
        assert!(formatted.contains("May"));
        assert_ne!(formatted, date.to_iso8601());
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn formatter_gracefully_formats_large_gregorian_years() {
        let formatter = DateFormatter::new(&locales::en_us(), FormatLength::Long);

        let date = CalendarDate::new_gregorian(10_000, 1, 1).expect("fixture should validate");

        assert_eq!(formatter.format(&date), "January 1, 10000");
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn legacy_style_constructor_keeps_existing_output_for_format_date() {
        let formatter = DateFormatter::new(&locales::en_us(), FormatLength::Long);

        assert_eq!(
            formatter.format(&march_2024()),
            formatter.format_date(&march_2024())
        );
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn options_date_formatter_emits_structured_parts_for_representative_locales() {
        for locale_name in ["en-US", "de-DE", "ar-SA", "ja-JP"] {
            let locale = Locale::parse(locale_name).expect("locale should parse");

            let formatter = DateFormatter::new_with_options(
                &locale,
                DateFormatterOptions {
                    weekday: Some(TextWidth::Long),
                    year: Some(NumericWidth::Numeric),
                    month: Some(MonthFormat::Long),
                    day: Some(NumericWidth::Numeric),
                    ..DateFormatterOptions::default()
                },
            );

            let parts = formatter.format_date_to_parts(&march_2024());

            assert!(
                parts
                    .iter()
                    .any(|part| part.kind == DateFormatterPartKind::Weekday)
            );
            assert!(
                parts
                    .iter()
                    .any(|part| part.kind == DateFormatterPartKind::Month)
            );
            assert!(
                parts
                    .iter()
                    .any(|part| part.kind == DateFormatterPartKind::Day)
            );
            assert!(
                parts
                    .iter()
                    .any(|part| part.kind == DateFormatterPartKind::Year)
            );
            assert!(
                parts
                    .iter()
                    .any(|part| part.kind == DateFormatterPartKind::Literal)
            );

            let rebuilt = parts
                .iter()
                .map(|part| part.value.as_str())
                .collect::<String>();

            assert_eq!(rebuilt, formatter.format_date(&march_2024()));

            let month_idx = parts
                .iter()
                .position(|part| part.kind == DateFormatterPartKind::Month)
                .expect("month part should exist");

            let day_idx = parts
                .iter()
                .position(|part| part.kind == DateFormatterPartKind::Day)
                .expect("day part should exist");

            let year_idx = parts
                .iter()
                .position(|part| part.kind == DateFormatterPartKind::Year)
                .expect("year part should exist");

            match locale_name {
                "en-US" => assert!(month_idx < day_idx && day_idx < year_idx),

                "de-DE" | "ar-SA" => assert!(day_idx < month_idx && month_idx < year_idx),

                "ja-JP" => assert!(year_idx < month_idx && month_idx < day_idx),

                _ => unreachable!("representative locale fixture should be covered"),
            }
        }
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn format_time_to_parts_respects_hour_cycle() {
        let time = Time::new(17, 5, 9, 0).expect("fixture should validate");

        let twelve_hour = DateFormatter::new_with_options(
            &locales::en_us(),
            DateFormatterOptions {
                hour: Some(NumericWidth::Numeric),
                minute: Some(NumericWidth::TwoDigit),
                hour_cycle: Some(HourCycle::H12),
                ..DateFormatterOptions::default()
            },
        );

        let twenty_four_hour = DateFormatter::new_with_options(
            &locales::en_us(),
            DateFormatterOptions {
                hour: Some(NumericWidth::Numeric),
                minute: Some(NumericWidth::TwoDigit),
                hour_cycle: Some(HourCycle::H23),
                ..DateFormatterOptions::default()
            },
        );

        let twelve_parts = twelve_hour.format_time_to_parts(&time);

        let twenty_four_parts = twenty_four_hour.format_time_to_parts(&time);

        assert!(
            twelve_parts
                .iter()
                .any(|part| part.kind == DateFormatterPartKind::DayPeriod)
        );
        assert!(
            !twenty_four_parts
                .iter()
                .any(|part| part.kind == DateFormatterPartKind::DayPeriod)
        );
        assert_eq!(
            twelve_parts
                .iter()
                .find(|part| part.kind == DateFormatterPartKind::Hour)
                .expect("12-hour formatter should emit an hour part")
                .value,
            "5"
        );
        assert_eq!(
            twenty_four_parts
                .iter()
                .find(|part| part.kind == DateFormatterPartKind::Hour)
                .expect("24-hour formatter should emit an hour part")
                .value,
            "17"
        );
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn resolved_options_reflect_effective_configuration() {
        let formatter = DateFormatter::new_with_options(
            &locales::en_us(),
            DateFormatterOptions {
                date_style: Some(FormatLength::Long),
                time_style: Some(FormatLength::Short),
                hour_cycle: Some(HourCycle::H12),
                time_zone: Some(
                    TimeZoneId::new("America/New_York").expect("time zone should parse"),
                ),
                calendar: Some(PublicCalendarSystem::Japanese),
                numbering_system: Some(String::from("latn")),
                ..DateFormatterOptions::default()
            },
        );

        let resolved = formatter.resolved_options();

        assert_eq!(resolved.locale, locales::en_us());
        assert_eq!(resolved.calendar, PublicCalendarSystem::Japanese);
        assert_eq!(
            resolved.time_zone,
            Some(TimeZoneId::new("America/New_York").expect("time zone should parse"))
        );
        assert_eq!(resolved.hour_cycle, Some(HourCycle::H12));
        assert_eq!(resolved.date_style, Some(FormatLength::Long));
        assert_eq!(resolved.time_style, Some(FormatLength::Short));
        assert_eq!(resolved.numbering_system.as_deref(), Some("latn"));
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn calendar_option_overrides_locale_calendar_when_requested() {
        let formatter = DateFormatter::new_with_options(
            &locales::en_us(),
            DateFormatterOptions {
                calendar: Some(PublicCalendarSystem::Japanese),
                era: Some(TextWidth::Long),
                year: Some(NumericWidth::Numeric),
                month: Some(MonthFormat::Long),
                day: Some(NumericWidth::Numeric),
                ..DateFormatterOptions::default()
            },
        );

        let date = CalendarDate::new_gregorian(2019, 5, 1).expect("fixture should validate");

        let parts = formatter.format_date_to_parts(&date);

        assert_eq!(
            formatter.resolved_options().calendar,
            PublicCalendarSystem::Japanese
        );
        assert!(
            parts
                .iter()
                .any(|part| part.kind == DateFormatterPartKind::Era)
        );
        assert_eq!(
            parts
                .iter()
                .find(|part| part.kind == DateFormatterPartKind::Year)
                .expect("calendar override should emit a year part")
                .value,
            "1"
        );
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn format_date_range_collapses_shared_tokens_in_en_us() {
        let formatter = DateFormatter::new_with_options(
            &locales::en_us(),
            DateFormatterOptions {
                month: Some(MonthFormat::Long),
                day: Some(NumericWidth::Numeric),
                year: Some(NumericWidth::Numeric),
                ..DateFormatterOptions::default()
            },
        );

        let start = CalendarDate::new_gregorian(2024, 3, 15).expect("fixture should validate");

        let end = CalendarDate::new_gregorian(2024, 3, 20).expect("fixture should validate");

        assert_eq!(
            formatter.format_date_range(&start, &end),
            "March 15 - 20, 2024"
        );
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn format_date_range_to_parts_marks_shared_start_and_end_segments() {
        let formatter = DateFormatter::new_with_options(
            &locales::en_us(),
            DateFormatterOptions {
                month: Some(MonthFormat::Long),
                day: Some(NumericWidth::Numeric),
                year: Some(NumericWidth::Numeric),
                ..DateFormatterOptions::default()
            },
        );

        let start = CalendarDate::new_gregorian(2024, 3, 15).expect("fixture should validate");

        let end = CalendarDate::new_gregorian(2024, 3, 20).expect("fixture should validate");

        let parts = formatter.format_date_range_to_parts(&start, &end);

        assert!(parts.iter().any(|part| {
            part.kind == DateFormatterPartKind::Month && part.source == DateRangePartSource::Shared
        }));
        assert!(parts.iter().any(|part| {
            part.kind == DateFormatterPartKind::Day
                && part.source == DateRangePartSource::Start
                && part.value == "15"
        }));
        assert!(parts.iter().any(|part| {
            part.kind == DateFormatterPartKind::Day
                && part.source == DateRangePartSource::End
                && part.value == "20"
        }));
        assert!(parts.iter().any(|part| {
            part.kind == DateFormatterPartKind::Year
                && part.source == DateRangePartSource::Shared
                && part.value == "2024"
        }));
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn format_date_time_range_preserves_time_fields() {
        let formatter = DateFormatter::new_with_options(
            &locales::en_us(),
            DateFormatterOptions {
                month: Some(MonthFormat::Long),
                day: Some(NumericWidth::Numeric),
                year: Some(NumericWidth::Numeric),
                hour: Some(NumericWidth::Numeric),
                minute: Some(NumericWidth::TwoDigit),
                hour_cycle: Some(HourCycle::H12),
                ..DateFormatterOptions::default()
            },
        );

        let start = CalendarDateTime::new(
            CalendarDate::new_gregorian(2024, 3, 15).expect("fixture should validate"),
            Time::new(13, 30, 0, 0).expect("time should validate"),
        );

        let end = CalendarDateTime::new(
            CalendarDate::new_gregorian(2024, 3, 15).expect("fixture should validate"),
            Time::new(17, 45, 0, 0).expect("time should validate"),
        );

        let formatted = formatter.format_date_time_range(&start, &end);

        assert!(formatted.contains("March"));
        assert!(formatted.contains("1:30"));
        assert!(formatted.contains("5:45 PM"));
    }

    #[cfg(all(feature = "std", feature = "icu4x"))]
    #[test]
    fn format_zoned_range_preserves_zone_output() {
        let formatter = DateFormatter::new_with_options(
            &locales::en_us(),
            DateFormatterOptions {
                month: Some(MonthFormat::Long),
                day: Some(NumericWidth::Numeric),
                year: Some(NumericWidth::Numeric),
                hour: Some(NumericWidth::Numeric),
                minute: Some(NumericWidth::TwoDigit),
                time_zone_name: Some(TimeZoneNameFormat::Short),
                ..DateFormatterOptions::default()
            },
        );

        let time_zone = TimeZoneId::new("America/New_York").expect("time zone should parse");

        let start_local = CalendarDateTime::new(
            CalendarDate::new_gregorian(2024, 3, 15).expect("fixture should validate"),
            Time::new(9, 0, 0, 0).expect("time should validate"),
        );
        let start = ZonedDateTime::new(
            &start_local,
            time_zone.clone(),
            crate::Disambiguation::Compatible,
        )
        .expect("zoned start should validate");

        let end_local = CalendarDateTime::new(
            CalendarDate::new_gregorian(2024, 3, 16).expect("fixture should validate"),
            Time::new(11, 30, 0, 0).expect("time should validate"),
        );
        let end = ZonedDateTime::new(&end_local, time_zone, crate::Disambiguation::Compatible)
            .expect("zoned end should validate");

        let formatted = formatter.format_zoned_range(&start, &end);

        assert!(formatted.contains("America/New_York"));
        assert!(formatted.contains("9:00"));
        assert!(formatted.contains("11:30"));
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn formatter_option_resolution_and_style_helpers_cover_default_paths() {
        let locale = Locale::parse("ja-JP-u-ca-japanese")
            .expect("locale with calendar extension should parse");

        let date_only = resolve_options(
            &locale,
            &DateFormatterOptions {
                date_style: Some(FormatLength::Short),
                ..DateFormatterOptions::default()
            },
        );

        let time_only = resolve_options(
            &locales::en_us(),
            &DateFormatterOptions {
                time_style: Some(FormatLength::Long),
                ..DateFormatterOptions::default()
            },
        );

        let full_date = date_fields_from_style(FormatLength::Full);

        let short_time = time_fields_from_style(FormatLength::Short, HourCycle::H24);

        assert_eq!(date_only.calendar, PublicCalendarSystem::Japanese);
        assert_eq!(date_only.hour_cycle, None);
        assert_eq!(time_only.hour_cycle, Some(HourCycle::H12));
        assert_eq!(full_date.weekday, Some(TextWidth::Long));
        assert_eq!(full_date.month, Some(MonthFormat::Long));
        assert_eq!(short_time.hour, Some(NumericWidth::Numeric));
        assert_eq!(short_time.minute, Some(NumericWidth::TwoDigit));
        assert_eq!(short_time.second, None);
        assert_eq!(short_time.time_zone_name, None);
        assert_eq!(short_time.hour_cycle, HourCycle::H24);
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn formatter_style_helpers_cover_remaining_named_styles() {
        let long_date = date_fields_from_style(FormatLength::Long);

        let medium_date = date_fields_from_style(FormatLength::Medium);

        let short_date = date_fields_from_style(FormatLength::Short);

        let full_time = time_fields_from_style(FormatLength::Full, HourCycle::H11);

        let long_time = time_fields_from_style(FormatLength::Long, HourCycle::H12);

        let medium_time = time_fields_from_style(FormatLength::Medium, HourCycle::H23);

        assert_eq!(long_date.month, Some(MonthFormat::Long));
        assert_eq!(long_date.day, Some(NumericWidth::Numeric));
        assert_eq!(medium_date.month, Some(MonthFormat::Short));
        assert_eq!(short_date.year, Some(NumericWidth::TwoDigit));
        assert_eq!(short_date.month, Some(MonthFormat::Numeric));

        assert_eq!(full_time.hour, Some(NumericWidth::Numeric));
        assert_eq!(full_time.second, Some(NumericWidth::TwoDigit));
        assert_eq!(full_time.time_zone_name, Some(TimeZoneNameFormat::Long));
        assert_eq!(full_time.hour_cycle, HourCycle::H11);

        assert_eq!(long_time.time_zone_name, Some(TimeZoneNameFormat::Short));
        assert_eq!(long_time.hour_cycle, HourCycle::H12);

        assert_eq!(medium_time.hour, Some(NumericWidth::Numeric));
        assert_eq!(medium_time.minute, Some(NumericWidth::TwoDigit));
        assert_eq!(medium_time.second, Some(NumericWidth::TwoDigit));
        assert_eq!(medium_time.time_zone_name, None);
        assert_eq!(medium_time.hour_cycle, HourCycle::H23);
    }

    #[cfg(feature = "icu4x")]
    #[test]
    fn low_level_part_rendering_helpers_cover_shared_and_distinct_ranges() {
        let provider = StubIcuProvider;

        let locale = locales::en_us();

        let date = march_2024();

        let time = Time::new(17, 5, 9, 0).expect("time should validate");

        let date_parts = render_date_parts(
            &provider,
            &locale,
            &date,
            EffectiveDateFields {
                weekday: Some(TextWidth::Short),
                year: Some(NumericWidth::Numeric),
                month: Some(MonthFormat::Short),
                day: Some(NumericWidth::Numeric),
                ..EffectiveDateFields::default()
            },
        );

        let time_parts = render_time_parts(
            &provider,
            &locale,
            &time,
            EffectiveTimeFields {
                hour: Some(NumericWidth::Numeric),
                minute: Some(NumericWidth::TwoDigit),
                second: Some(NumericWidth::TwoDigit),
                time_zone_name: Some(TimeZoneNameFormat::ShortOffset),
                hour_cycle: HourCycle::H12,
            },
            Some(&TimeZoneId::new("America/New_York").expect("time zone should parse")),
            Some(-4 * 60),
        );

        let identical_range = range_parts_from_parts(&date_parts, &date_parts);

        let distinct_range = range_parts_from_parts(
            &date_parts,
            &[DateFormatterPart {
                kind: DateFormatterPartKind::Month,
                value: String::from("Apr"),
            }],
        );

        assert!(join_parts(&date_parts).contains("Mar"));
        assert!(join_parts(&time_parts).contains("PM"));
        assert!(join_parts(&time_parts).contains("GMT-4"));
        assert!(
            identical_range
                .iter()
                .all(|part| part.source == DateRangePartSource::Shared)
        );
        assert!(
            distinct_range
                .iter()
                .any(|part| part.kind == DateFormatterPartKind::Literal && part.value == " - ")
        );
        assert!(join_range_parts(&distinct_range).contains("Apr"));
    }

    #[cfg(all(feature = "std", feature = "icu4x"))]
    #[test]
    fn formatter_public_entrypoints_cover_time_date_time_and_zoned_variants() {
        let formatter = DateFormatter::new_with_options(
            &locales::en_us(),
            DateFormatterOptions {
                month: Some(MonthFormat::Long),
                day: Some(NumericWidth::Numeric),
                year: Some(NumericWidth::Numeric),
                hour: Some(NumericWidth::Numeric),
                minute: Some(NumericWidth::TwoDigit),
                second: Some(NumericWidth::TwoDigit),
                hour_cycle: Some(HourCycle::H12),
                time_zone_name: Some(TimeZoneNameFormat::Short),
                ..DateFormatterOptions::default()
            },
        );

        let time = Time::new(17, 5, 9, 0).expect("time should validate");

        let date_time = CalendarDateTime::new(march_2024(), time);

        let zoned = ZonedDateTime::new(
            &date_time,
            TimeZoneId::new("America/New_York").expect("time zone should parse"),
            crate::Disambiguation::Compatible,
        )
        .expect("zoned date-time should validate");

        let formatted_date_time = formatter.format_date_time(&date_time);

        let formatted_time = formatter.format_time(&time);

        let formatted_zoned = formatter.format_zoned(&zoned);

        assert!(formatted_date_time.contains("March"));
        assert!(formatted_date_time.contains("5:05:09 PM"));
        assert_eq!(formatted_time, "5:05:09 PM");
        assert!(formatted_zoned.contains("America/New_York"));
    }

    #[cfg(all(feature = "std", feature = "icu4x"))]
    #[test]
    fn formatter_public_entrypoints_render_numeric_offset_zone_labels() {
        let formatter = DateFormatter::new_with_options(
            &locales::en_us(),
            DateFormatterOptions {
                hour: Some(NumericWidth::Numeric),
                minute: Some(NumericWidth::TwoDigit),
                hour_cycle: Some(HourCycle::H12),
                time_zone: Some(
                    TimeZoneId::new("America/New_York").expect("time zone should parse"),
                ),
                time_zone_name: Some(TimeZoneNameFormat::ShortOffset),
                ..DateFormatterOptions::default()
            },
        );

        let date_time = CalendarDateTime::new(
            march_2024(),
            Time::new(17, 5, 9, 0).expect("time should validate"),
        );

        let zoned = ZonedDateTime::new(
            &date_time,
            TimeZoneId::new("America/New_York").expect("time zone should parse"),
            crate::Disambiguation::Compatible,
        )
        .expect("zoned date-time should validate");

        assert!(formatter.format_date_time(&date_time).contains("GMT-4"));
        assert_eq!(
            format_time_zone_name(
                zoned.time_zone(),
                TimeZoneNameFormat::LongOffset,
                Some(zoned.offset_minutes())
            ),
            "GMT-04:00"
        );
    }

    #[cfg(all(feature = "std", feature = "icu4x"))]
    #[test]
    fn formatter_primitives_cover_numeric_and_projection_helpers() {
        let provider = StubIcuProvider;

        let locale = locales::en_us();

        let era = Era {
            code: String::from("reiwa"),
            display_name: String::from("Reiwa"),
        };

        let local = CalendarDateTime::new(
            CalendarDate::new_gregorian(2024, 3, 15).expect("fixture should validate"),
            Time::new(17, 5, 9, 250).expect("time should validate"),
        );
        let zoned = ZonedDateTime::new(
            &local,
            TimeZoneId::new("America/New_York").expect("time zone should parse"),
            crate::Disambiguation::Compatible,
        )
        .expect("zoned date-time should validate");

        let mut pushed = Vec::new();

        push_date_field(&mut pushed, Some(literal_part("month")), " / ");
        push_date_field(&mut pushed, None, " / ");
        push_date_field(&mut pushed, Some(literal_part("day")), " / ");

        assert!(matches!(
            date_order(&Locale::parse("en-US").expect("locale should parse")),
            DateOrder::MonthDayYear
        ));
        assert!(matches!(
            date_order(&Locale::parse("de-DE").expect("locale should parse")),
            DateOrder::DayMonthYear
        ));
        assert!(matches!(
            date_order(&Locale::parse("ja-JP").expect("locale should parse")),
            DateOrder::YearMonthDay
        ));
        assert_eq!(format_year(2024, NumericWidth::TwoDigit), "24");
        assert_eq!(format_numeric(5, NumericWidth::TwoDigit), "05");
        assert_eq!(format_hour(0, NumericWidth::Numeric, HourCycle::H11), "0");
        assert_eq!(format_hour(0, NumericWidth::Numeric, HourCycle::H12), "12");
        assert_eq!(format_hour(17, NumericWidth::Numeric, HourCycle::H23), "17");
        assert_eq!(format_hour(0, NumericWidth::Numeric, HourCycle::H24), "24");
        assert_eq!(
            format_month(&provider, &locale, 3, MonthFormat::Numeric),
            "3"
        );
        assert_eq!(
            format_month(&provider, &locale, 3, MonthFormat::TwoDigit),
            "03"
        );
        assert_eq!(
            format_month(&provider, &locale, 3, MonthFormat::Long),
            "March"
        );
        assert_eq!(
            format_month(&provider, &locale, 3, MonthFormat::Short),
            "Mar"
        );
        assert_eq!(
            format_month(&provider, &locale, 3, MonthFormat::Narrow),
            "M"
        );
        assert_eq!(
            weekday_label(&provider, Weekday::Friday, &locale, TextWidth::Narrow),
            "F"
        );
        assert_eq!(era_label(&era, TextWidth::Narrow), "R");
        assert_eq!(truncate_graphemes("école", 2), "éc");
        assert_eq!(
            format_time_zone_name(
                zoned.time_zone(),
                TimeZoneNameFormat::LongOffset,
                Some(zoned.offset_minutes())
            ),
            "GMT-04:00"
        );
        assert_eq!(format_utc_offset(-4 * 60, false), "GMT-4");
        assert_eq!(format_utc_offset(5 * 60 + 30, false), "GMT+5:30");
        assert_eq!(format_utc_offset(-4 * 60, true), "GMT-04:00");
        assert_eq!(title_case_ascii("reiwa"), "Reiwa");
        assert_eq!(title_case_ascii(""), "");
        assert_eq!(date_from_zoned(&zoned).to_iso8601(), "2024-03-15");
        assert_eq!(time_from_zoned(&zoned).millisecond(), 250);
        assert_eq!(join_parts(&pushed), "month / day");
    }

    #[cfg(any(
        feature = "icu4x",
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    #[test]
    fn date_formatter_debug_includes_locale_and_length() {
        let formatter = DateFormatter::new(&locales::en_us(), FormatLength::Long);

        let debug = format!("{formatter:?}");

        assert!(debug.contains("DateFormatter"));
        assert!(debug.contains("en-US"));
        assert!(debug.contains("Long"));
    }

    #[cfg(any(
        feature = "icu4x",
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    #[test]
    fn fallback_format_date_covers_all_lengths_and_non_gregorian_passthrough() {
        let locale = locales::en_us();

        let gregorian = CalendarDate::new_gregorian(2024, 3, 15).expect("fixture should validate");

        assert_eq!(
            fallback_format_date(&locale, FormatLength::Short, &gregorian),
            "3/15/24"
        );
        assert_eq!(
            fallback_format_date(&locale, FormatLength::Medium, &gregorian),
            "Mar 15, 2024"
        );
        assert_eq!(
            fallback_format_date(&locale, FormatLength::Long, &gregorian),
            "March 15, 2024"
        );
        assert_eq!(
            fallback_format_date(&locale, FormatLength::Full, &gregorian),
            "Friday, March 15, 2024"
        );

        let japanese = japanese_reiwa_date();

        assert_eq!(
            fallback_format_date(&locale, FormatLength::Long, &japanese),
            japanese.to_iso8601()
        );
    }

    #[cfg(any(
        feature = "icu4x",
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    #[test]
    fn english_fallback_helpers_cover_valid_and_invalid_inputs() {
        assert_eq!(english_month_short(1), "Jan");
        assert_eq!(english_month_short(12), "Dec");
        assert_eq!(english_month_short(0), "???");
        assert_eq!(english_month_short(13), "???");

        assert_eq!(english_month_long(1), "January");
        assert_eq!(english_month_long(12), "December");
        assert_eq!(english_month_long(0), "Unknown");
        assert_eq!(english_month_long(13), "Unknown");

        assert_eq!(english_weekday_long(Weekday::Sunday), "Sunday");
        assert_eq!(english_weekday_long(Weekday::Monday), "Monday");
        assert_eq!(english_weekday_long(Weekday::Tuesday), "Tuesday");
        assert_eq!(english_weekday_long(Weekday::Wednesday), "Wednesday");
        assert_eq!(english_weekday_long(Weekday::Thursday), "Thursday");
        assert_eq!(english_weekday_long(Weekday::Friday), "Friday");
        assert_eq!(english_weekday_long(Weekday::Saturday), "Saturday");
    }

    #[cfg(any(
        not(any(feature = "icu4x", feature = "web-intl")),
        all(
            feature = "web-intl",
            not(target_arch = "wasm32"),
            not(feature = "icu4x")
        )
    ))]
    #[test]
    fn fallback_date_formatter_keeps_api_available() {
        let formatter = DateFormatter::new(&locales::en_us(), FormatLength::Full);

        let formatted = formatter
            .format(&CalendarDate::new_gregorian(2024, 3, 15).expect("fixture should validate"));

        assert!(formatted.contains("Friday"));
        assert!(formatted.contains("March"));
    }
}

#[cfg(all(
    test,
    feature = "web-intl",
    target_arch = "wasm32",
    not(feature = "icu4x")
))]
mod web_intl_tests {
    use alloc::string::String;

    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

    use super::{
        DateFormatter, FormatLength, js_date_from_calendar, js_date_from_ymd, js_date_is_valid,
        js_iso_year,
    };
    use crate::{CalendarDate, CalendarSystem, Locale, locales};

    wasm_bindgen_test_configure!(run_in_browser);

    fn march_2024() -> CalendarDate {
        CalendarDate::new_gregorian(2024, 3, 15).expect("fixture should validate")
    }

    #[wasm_bindgen_test]
    fn web_intl_date_formatter_formats_gregorian_dates() {
        let formatter = DateFormatter::new(&locales::en_us(), FormatLength::Short);

        assert_eq!(formatter.format(&march_2024()), "3/15/24");
    }

    #[wasm_bindgen_test]
    fn web_intl_date_formatter_tracks_locale_shape() {
        let english = DateFormatter::new(&locales::en_us(), FormatLength::Medium);

        let german = DateFormatter::new(
            &Locale::parse("de-DE").expect("locale should parse"),
            FormatLength::Medium,
        );

        assert_ne!(english.format(&march_2024()), german.format(&march_2024()));
    }

    #[wasm_bindgen_test]
    fn web_intl_date_formatter_formats_non_gregorian_dates_via_browser_calendar() {
        let locale = Locale::parse("en-US-u-ca-japanese").expect("locale should parse");

        let formatter = DateFormatter::new(&locale, FormatLength::Long);

        let date = CalendarDate::new(
            CalendarSystem::Japanese,
            &crate::CalendarDateFields {
                era: Some(crate::Era {
                    code: String::from("reiwa"),
                    display_name: String::from("Reiwa"),
                }),
                year: Some(1),
                month: Some(5),
                day: Some(1),
                ..crate::CalendarDateFields::default()
            },
        )
        .expect("fixture should validate");

        let direct = direct_browser_format(&locale, FormatLength::Long, &date);

        let formatted = formatter.format(&date);

        assert_eq!(formatted, direct);
        assert!(formatted.contains("Reiwa"));
        assert_ne!(formatted, date.to_iso8601());
    }

    #[wasm_bindgen_test]
    fn web_intl_date_helpers_preserve_low_gregorian_years() {
        let date = CalendarDate::new_gregorian(44, 3, 15).expect("fixture should validate");

        let js_date = js_date_from_ymd(44, 3, 15);

        let converted =
            js_date_from_calendar(&date).expect("Gregorian test fixture should map to a Date");

        assert_eq!(js_date.get_utc_full_year(), 44);
        assert_eq!(js_date.get_utc_month(), 2);
        assert_eq!(js_date.get_utc_date(), 15);
        assert_eq!(converted.get_utc_full_year(), 44);
        assert_eq!(converted.get_utc_month(), 2);
        assert_eq!(converted.get_utc_date(), 15);
    }

    #[wasm_bindgen_test]
    fn web_intl_date_formatter_gracefully_formats_large_gregorian_years() {
        let formatter = DateFormatter::new(&locales::en_us(), FormatLength::Long);

        let date = CalendarDate::new_gregorian(10_000, 1, 1).expect("fixture should validate");

        let formatted = formatter.format(&date);

        assert_eq!(formatted, "January 1, 10000");
    }

    #[wasm_bindgen_test]
    fn web_intl_date_helpers_support_pre_ce_gregorian_dates() {
        let js_date = js_date_from_ymd(-1, 1, 1);

        assert_eq!(js_date.to_iso_string(), "-000001-01-01T12:00:00.000Z");
        assert_eq!(js_iso_year(-1), "-000001");
        assert_eq!(js_iso_year(0), "0000");
        assert_eq!(js_iso_year(44), "0044");
        assert_eq!(js_iso_year(10_000), "+010000");
    }

    #[wasm_bindgen_test]
    fn web_intl_date_formatter_preserves_bce_gregorian_dates() {
        let locale = locales::en_us();
        let formatter = DateFormatter::new(&locale, FormatLength::Long);
        let date = CalendarDate::new(
            CalendarSystem::Gregorian,
            &crate::CalendarDateFields {
                era: Some(crate::Era {
                    code: String::from("bc"),
                    display_name: String::from("BC"),
                }),
                year: Some(1),
                month: Some(1),
                day: Some(1),
                ..crate::CalendarDateFields::default()
            },
        )
        .expect("BCE Gregorian fixture should validate");

        let converted =
            js_date_from_calendar(&date).expect("BCE Gregorian fixture should map to a Date");
        let expected = direct_browser_format_for_js_date(
            &locale,
            FormatLength::Long,
            &js_date_from_ymd(0, 1, 1),
        );

        assert_eq!(converted.to_iso_string(), "0000-01-01T12:00:00.000Z");
        assert_eq!(formatter.format(&date), expected);
    }

    #[wasm_bindgen_test]
    fn web_intl_date_helpers_preserve_astronomical_gregorian_years_from_non_gregorian_input() {
        let buddhist = CalendarDate::new(
            CalendarSystem::Buddhist,
            &crate::CalendarDateFields {
                year: Some(1),
                month: Some(1),
                day: Some(1),
                ..crate::CalendarDateFields::default()
            },
        )
        .expect("Buddhist date should validate");

        let converted = js_date_from_calendar(&buddhist)
            .expect("Buddhist fixture should map to a browser Date");

        assert_eq!(converted.to_iso_string(), "-000542-01-01T12:00:00.000Z");
    }

    #[wasm_bindgen_test]
    fn web_intl_public_date_validation_rejects_gregorian_years_beyond_supported_range() {
        assert!(js_date_is_valid(&js_date_from_ymd(10_000, 1, 1)));
        assert!(!js_date_is_valid(&js_date_from_ymd(1_000_000, 1, 1)));
        assert!(CalendarDate::new_gregorian(1_000_000, 1, 1).is_err());
    }

    fn direct_browser_format(locale: &Locale, length: FormatLength, date: &CalendarDate) -> String {
        let js_date =
            js_date_from_calendar(date).expect("test fixtures should map to a browser Date");

        direct_browser_format_for_js_date(locale, length, &js_date)
    }

    fn direct_browser_format_for_js_date(
        locale: &Locale,
        length: FormatLength,
        js_date: &js_sys::Date,
    ) -> String {
        use js_sys::{
            Array, Function,
            Intl::{DateTimeFormat as JsDateTimeFormat, DateTimeFormatOptions},
        };
        use wasm_bindgen::JsValue;

        let locales = Array::of1(&JsValue::from_str(&locale.to_bcp47()));

        let options = DateTimeFormatOptions::new();

        options.set_date_style(length.to_js_date_style());
        options.set_time_zone("UTC");

        let formatter = JsDateTimeFormat::new(&locales, options.as_ref());

        let format: Function = formatter.format();

        format
            .call1(&JsValue::UNDEFINED, js_date.as_ref())
            .expect("Intl.DateTimeFormat.format should not throw for a valid Date")
            .as_string()
            .unwrap_or_default()
    }
}
