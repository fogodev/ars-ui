//! Calendar and date/time primitives shared across ars-ui.

mod date;
mod helpers;
pub mod parse;
pub mod queries;
mod system;
mod typed;

#[cfg(any(feature = "icu4x", all(feature = "web-intl", target_arch = "wasm32")))]
pub(crate) mod internal;

#[cfg(test)]
#[path = "../../tests/unit/calendar.rs"]
mod tests;

#[cfg(feature = "std")]
pub use date::ZonedDateTime;
#[cfg(all(feature = "web-intl", target_arch = "wasm32"))]
pub(crate) use date::build_from_iso_parts;
pub use date::{
    CalendarDate, CalendarDateFields, CalendarDateTime, CycleOptions, CycleTimeOptions,
    DateDuration, DateField, DateTimeDuration, DateTimeField, DateValue, Disambiguation, Time,
    TimeDuration, TimeField, TimeFields, to_calendar_date_time,
};
#[cfg(feature = "std")]
pub use date::{to_zoned, to_zoned_date_time};
pub use helpers::DateRange;
#[cfg(any(feature = "icu4x", all(feature = "web-intl", target_arch = "wasm32")))]
pub(crate) use helpers::default_era_for;
#[cfg(all(feature = "web-intl", target_arch = "wasm32"))]
pub(crate) use helpers::gregorian_days_in_month;
pub(crate) use helpers::{bounded_days_in_month, bounded_months_in_year};
pub use system::{
    CalendarConversionError, CalendarError, CalendarMetadata, CalendarSystem, DateError, Era,
    HourCycle, JapaneseEra, MonthCode, TimeZoneId, WeekInfo,
};
pub(crate) use system::{canonical_era, infer_public_era, month_code_from_temporal};
pub use typed::{
    Buddhist, CalendarKind, CalendarTypeError, Chinese, Coptic, Dangi, DirectDayArithmetic,
    DirectWeekdayComputation, Ethiopic, EthiopicAmeteAlem, Gregorian, Hebrew, Indian, IslamicCivil,
    IslamicUmmAlQura, Japanese, Month, Persian, Roc, TypedCalendarDate,
};
