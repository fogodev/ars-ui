use alloc::string::{String, ToString};

use icu::calendar::{
    AnyCalendar, Date, Iso,
    types::{Month as IcuMonth, YearInput},
};

use super::{
    CalendarConversionError, CalendarError, CalendarSystem, DateError, iso_to_epoch_days,
    platform_today_iso,
};
use crate::Weekday;

/// A calendar date backed by ICU4X runtime calendar data.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CalendarDate {
    /// The underlying ICU4X date.
    pub(crate) inner: Date<AnyCalendar>,
}

impl CalendarDate {
    /// Creates a Gregorian date from ISO year-month-day components.
    pub(crate) fn from_iso(year: i32, month: u8, day: u8) -> Result<Self, DateError> {
        let date = Date::try_new_gregorian(year, month, day).map_err(|_| DateError::InvalidDate)?;

        Ok(Self {
            inner: date.to_any(),
        })
    }

    /// Creates a date in the given calendar system from raw calendar fields.
    pub(crate) fn from_calendar(
        year: i32,
        month: u8,
        day: u8,
        calendar: CalendarSystem,
    ) -> Result<Self, CalendarConversionError> {
        Self::from_calendar_parts(default_year_input(calendar, year), month, day, calendar)
    }

    /// Creates a date in the given calendar system from an explicit era plus
    /// year-in-era.
    pub(crate) fn from_calendar_with_era(
        era: &str,
        year: i32,
        month: u8,
        day: u8,
        calendar: CalendarSystem,
    ) -> Result<Self, CalendarConversionError> {
        Self::from_calendar_parts(YearInput::EraYear(era, year), month, day, calendar)
    }

    fn from_calendar_parts(
        year: YearInput<'_>,
        month: u8,
        day: u8,
        calendar: CalendarSystem,
    ) -> Result<Self, CalendarConversionError> {
        let any_calendar = AnyCalendar::new(calendar.to_icu_kind());

        let date = Date::try_new(year, IcuMonth::new(month), day, any_calendar)
            .map_err(|error| CalendarConversionError::Icu(error.to_string()))?;

        Ok(Self { inner: date })
    }

    /// Converts the date into a different calendar system.
    #[must_use]
    pub(crate) fn to_calendar(&self, calendar: CalendarSystem) -> Self {
        Self {
            inner: self
                .inner
                .to_calendar(AnyCalendar::new(calendar.to_icu_kind())),
        }
    }

    /// Returns the display year for the date's calendar.
    #[must_use]
    pub(crate) fn year(&self) -> i32 {
        self.inner.year().era_year_or_related_iso()
    }

    /// Returns the 1-based month ordinal.
    #[must_use]
    pub(crate) fn month(&self) -> u8 {
        self.inner.month().ordinal
    }

    /// Returns the 1-based day of month.
    #[must_use]
    pub(crate) fn day(&self) -> u8 {
        self.inner.day_of_month().0
    }

    /// Returns the ISO weekday.
    #[must_use]
    pub(crate) fn weekday(&self) -> Weekday {
        Weekday::from_icu_weekday(self.inner.weekday())
    }

    /// Returns the era code when one is present.
    #[must_use]
    pub(crate) fn era(&self) -> Option<String> {
        self.inner.year().era().map(|era| era.era.to_string())
    }

    /// Adds a whole-day offset to the date.
    pub(crate) fn add_days(&self, days: i32) -> Result<Self, CalendarError> {
        let iso = self.inner.to_calendar(Iso);

        let epoch_days = iso_to_epoch_days(
            iso.year().era_year_or_related_iso(),
            iso.month().ordinal,
            iso.day_of_month().0,
        );

        let (year, month, day) = super::epoch_days_to_iso(epoch_days + i64::from(days));

        let next = Date::try_new_iso(year, month, day)
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

        Ok(Self {
            inner: next.to_any().to_calendar(self.inner.calendar().clone()),
        })
    }

    /// Returns the day offset to `other`.
    pub(crate) fn days_until(&self, other: &Self) -> Result<i32, CalendarError> {
        let lhs = self.inner.to_calendar(Iso);

        let rhs = other.inner.to_calendar(Iso);

        let lhs_epoch = iso_to_epoch_days(
            lhs.year().era_year_or_related_iso(),
            lhs.month().ordinal,
            lhs.day_of_month().0,
        );

        let rhs_epoch = iso_to_epoch_days(
            rhs.year().era_year_or_related_iso(),
            rhs.month().ordinal,
            rhs.day_of_month().0,
        );

        let diff = rhs_epoch - lhs_epoch;

        i32::try_from(diff).map_err(|_| {
            CalendarError::Arithmetic(String::from("date difference exceeds i32 range"))
        })
    }

    /// Returns `true` when `self` is chronologically before `other`.
    pub(crate) fn is_before(&self, other: &Self) -> Result<bool, CalendarError> {
        self.days_until(other).map(|diff| diff > 0)
    }

    /// Returns today's date in the requested calendar system.
    pub(crate) fn today(calendar: CalendarSystem) -> Result<Self, CalendarError> {
        let (year, month, day) = platform_today_iso().map_err(CalendarError::Arithmetic)?;

        let iso = Date::<Iso>::try_new_iso(year, month, day)
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

        Ok(Self {
            inner: iso.to_calendar(AnyCalendar::new(calendar.to_icu_kind())),
        })
    }
}

impl TryFrom<&super::CalendarDate> for CalendarDate {
    type Error = CalendarConversionError;

    fn try_from(value: &super::CalendarDate) -> Result<Self, Self::Error> {
        if let Some(era) = &value.era {
            return Self::from_calendar_with_era(
                &era.code,
                value.year,
                value.month.get(),
                value.day.get(),
                value.calendar,
            );
        }

        if value.calendar == CalendarSystem::Gregorian {
            return Self::from_iso(value.year, value.month.get(), value.day.get()).map_err(
                |error| match error {
                    DateError::InvalidDate => CalendarConversionError::InvalidDate,
                    DateError::OutOfRange | DateError::CalendarError(_) => {
                        CalendarConversionError::Icu(error.to_string())
                    }
                },
            );
        }

        Self::from_calendar(
            value.year,
            value.month.get(),
            value.day.get(),
            value.calendar,
        )
    }
}

const fn default_year_input(calendar: CalendarSystem, year: i32) -> YearInput<'static> {
    if let Some(era) = default_era_code(calendar) {
        YearInput::EraYear(era, year)
    } else {
        YearInput::Extended(year)
    }
}

const fn default_era_code(calendar: CalendarSystem) -> Option<&'static str> {
    match calendar {
        CalendarSystem::Japanese => Some("reiwa"),
        _ => None,
    }
}
