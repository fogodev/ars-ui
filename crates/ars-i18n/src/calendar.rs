//! Calendar and date/time primitives shared across ars-ui.

mod date;
mod helpers;
mod system;
mod typed;

#[cfg(feature = "icu4x")]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "issue #128 lays the internal calendar foundation; production consumers arrive in follow-up issues #129 and #138"
    )
)]
pub(crate) mod internal;

#[cfg(test)]
#[path = "../tests/unit/calendar.rs"]
mod tests;

pub use date::CalendarDate;
pub use helpers::DateRange;
#[cfg(feature = "icu4x")]
pub(crate) use helpers::platform_today_iso;
pub(crate) use helpers::{
    bounded_days_in_month, bounded_months_in_year, coptic_like_days_in_month, default_era_for,
    gregorian_days_in_month, minimum_day_in_month, minimum_month_in_year, years_in_era,
};
#[cfg(any(test, feature = "icu4x"))]
pub(crate) use helpers::{epoch_days_to_iso, iso_to_epoch_days};
pub use system::{
    CalendarConversionError, CalendarError, CalendarMetadata, CalendarSystem, DateError, Era,
    HourCycle, JapaneseEra, Time, WeekInfo,
};
pub use typed::{
    Buddhist, CalendarKind, CalendarTypeError, Chinese, Coptic, Dangi, DirectDayArithmetic,
    DirectWeekdayComputation, Ethiopic, EthiopicAmeteAlem, Gregorian, Hebrew, Indian, Islamic,
    IslamicCivil, IslamicUmmAlQura, Japanese, Month, Persian, Roc, TypedCalendarDate,
};
