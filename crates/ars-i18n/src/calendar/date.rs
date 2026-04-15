use alloc::{format, string::String};
use core::{cmp::Ordering, num::NonZero};

use super::{
    CalendarSystem, Era,
    helpers::{
        GREGORIAN_JDN_OFFSET, days_in_month_for_calendar, epoch_days_to_iso, era_code_is_valid,
        gregorian_days_in_month, iso_to_epoch_days, max_months_in_year,
    },
};
use crate::{IcuProvider, Weekday};

/// A date in an arbitrary calendar system.
///
/// Raw field-order comparison is not sound across different calendar systems or
/// across differing named eras, so this type intentionally does not implement
/// ordering traits.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CalendarDate {
    /// The calendar system that gives semantic meaning to the date fields.
    pub calendar: CalendarSystem,

    /// The named era, when the calendar uses one.
    pub era: Option<Era>,

    /// The display year in the date's calendar system.
    pub year: i32,

    /// The 1-based month ordinal.
    pub month: NonZero<u8>,

    /// The 1-based day of month.
    pub day: NonZero<u8>,
}

impl CalendarDate {
    /// Creates a Gregorian date from validated month and day components.
    #[must_use]
    pub fn new_gregorian(year: i32, month: NonZero<u8>, day: NonZero<u8>) -> Self {
        let month_value = month.get();
        let day_value = day.get();

        assert!(year >= 1, "Gregorian dates require a year of at least 1");
        assert!(
            (1..=12).contains(&month_value),
            "Gregorian dates require a month in the range 1..=12",
        );
        assert!(
            day_value <= gregorian_days_in_month(year, month_value),
            "Gregorian dates require a valid day for the given month",
        );

        Self {
            calendar: CalendarSystem::Gregorian,
            era: None,
            year,
            month,
            day,
        }
    }

    /// Creates a validated date in the requested calendar system.
    #[must_use]
    pub fn new(
        provider: &dyn IcuProvider,
        calendar: CalendarSystem,
        era: Option<Era>,
        year: i32,
        month: u8,
        day: u8,
    ) -> Option<Self> {
        let era = era.or_else(|| provider.default_era(&calendar));
        let era_code = era.as_ref().map(|value| value.code.as_str());
        let validated_month = NonZero::new(month)?;

        if year < 1 || !era_code_is_valid(calendar, era_code) {
            return None;
        }

        let max_month = max_months_in_year(provider, calendar, year, era_code);

        if !(1..=max_month).contains(&month) {
            return None;
        }

        let provisional = Self {
            calendar,
            era: era.clone(),
            year,
            month: validated_month,
            day: NonZero::new(1).expect("one is non-zero"),
        };

        if let Some(max_year) = provider.years_in_era(&provisional) {
            if provisional.year > max_year {
                return None;
            }
        }

        if month < provider.minimum_month_in_year(&provisional) {
            return None;
        }

        let max_day = days_in_month_for_calendar(provider, calendar, year, month, era_code);

        if !(1..=max_day).contains(&day) {
            return None;
        }

        let candidate = Self {
            calendar,
            era,
            year,
            month: validated_month,
            day: NonZero::new(day).expect("validated day is 1-based"),
        };

        if day < provider.minimum_day_in_month(&candidate) {
            return None;
        }

        Some(candidate)
    }

    /// Compares two dates only when raw field comparison is meaningful.
    ///
    /// Returns `None` when the dates use different calendar systems or carry
    /// incompatible era values.
    #[must_use]
    pub fn compare_within_calendar(&self, other: &Self) -> Option<Ordering> {
        if self.calendar != other.calendar {
            return None;
        }

        if let (Some(left), Some(right)) = (&self.era, &other.era) {
            if left.code != right.code {
                return None;
            }
        }

        Some(
            self.year
                .cmp(&other.year)
                .then(self.month.cmp(&other.month))
                .then(self.day.cmp(&other.day)),
        )
    }

    /// Returns this date as a raw `YYYY-MM-DD` field string.
    ///
    /// For non-Gregorian calendars the output preserves the date's own calendar
    /// fields rather than converting to Gregorian ISO 8601 semantics.
    #[must_use]
    pub fn to_iso8601(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}",
            self.year,
            self.month.get(),
            self.day.get()
        )
    }

    /// Returns `true` if this date falls within the inclusive `[start, end]`
    /// range when raw field comparison is valid.
    #[must_use]
    pub fn is_between(&self, start: &CalendarDate, end: &CalendarDate) -> bool {
        if let (Some(lower), Some(upper)) = (
            self.compare_within_calendar(start),
            self.compare_within_calendar(end),
        ) {
            lower != Ordering::Less && upper != Ordering::Greater
        } else {
            false
        }
    }

    /// Returns the number of days in this date's month.
    #[must_use]
    pub fn days_in_month(&self, provider: &dyn IcuProvider) -> u8 {
        let era_code = self.era.as_ref().map(|value| value.code.as_str());

        days_in_month_for_calendar(
            provider,
            self.calendar,
            self.year,
            self.month.get(),
            era_code,
        )
    }

    /// Adds whole months within the same calendar system.
    #[must_use]
    pub fn add_months(&self, provider: &dyn IcuProvider, month_delta: i32) -> Option<CalendarDate> {
        let era = self
            .era
            .clone()
            .or_else(|| provider.default_era(&self.calendar));

        let era_code = era.as_ref().map(|value| value.code.as_str());
        let (year, month) = if let Some(months_per_year) = fixed_months_per_year(self.calendar) {
            let total_month_index = i64::from(self.year - 1)
                .checked_mul(i64::from(months_per_year))?
                .checked_add(i64::from(self.month.get() - 1))?
                .checked_add(i64::from(month_delta))?;
            let normalized_year = total_month_index
                .div_euclid(i64::from(months_per_year))
                .checked_add(1)?;
            let normalized_month = total_month_index.rem_euclid(i64::from(months_per_year));

            if normalized_year < 1 {
                return None;
            }

            (
                i32::try_from(normalized_year).ok()?,
                u8::try_from(normalized_month.checked_add(1)?).ok()?,
            )
        } else {
            let mut year = i64::from(self.year);
            let mut month = i64::from(self.month.get()).checked_add(i64::from(month_delta))?;

            loop {
                let current_year = i32::try_from(year).ok()?;
                let max =
                    i64::from(provider.max_months_in_year(&self.calendar, current_year, era_code));

                if month <= max {
                    break;
                }

                month -= max;
                year = year.checked_add(1)?;
            }

            while month < 1 {
                year = year.checked_sub(1)?;
                let current_year = i32::try_from(year).ok()?;
                let max =
                    i64::from(provider.max_months_in_year(&self.calendar, current_year, era_code));

                month += max;
            }

            (
                i32::try_from(year).ok()?,
                u8::try_from(month).expect("validated month count fits in u8"),
            )
        };

        let mut candidate = Self {
            calendar: self.calendar,
            era: era.clone(),
            year,
            month: NonZero::new(month).expect("validated month is 1-based"),
            day: self.day,
        };

        let min_month = provider.minimum_month_in_year(&candidate);

        if month < min_month {
            return None;
        }

        let max_day = days_in_month_for_calendar(provider, self.calendar, year, month, era_code);

        let mut day = self.day.get().min(max_day);

        candidate.day = NonZero::new(day).expect("clamped day is 1-based");

        let min_day = provider.minimum_day_in_month(&candidate);

        if day < min_day {
            day = min_day;
        }

        Self::new(provider, self.calendar, era, year, month, day)
    }

    /// Adds whole days for Gregorian dates.
    #[must_use]
    pub fn add_days(&self, day_delta: i32) -> Option<CalendarDate> {
        if self.calendar != CalendarSystem::Gregorian {
            return None;
        }

        let shifted = Self::from_jdn(self.to_jdn() + i64::from(day_delta), self.calendar);

        (shifted.year >= 1).then_some(shifted)
    }

    /// Adds whole days, delegating non-Gregorian conversion to the provider.
    #[must_use]
    pub fn add_days_with_provider(
        &self,
        provider: &dyn IcuProvider,
        day_delta: i32,
    ) -> CalendarDate {
        if self.calendar == CalendarSystem::Gregorian {
            return self
                .add_days(day_delta)
                .expect("Gregorian day arithmetic must remain within the supported year range");
        }

        let gregorian = provider.convert_date(self, CalendarSystem::Gregorian);
        let shifted = gregorian
            .add_days(day_delta)
            .expect("provider-backed day arithmetic must remain within the supported year range");

        provider.convert_date(&shifted, self.calendar)
    }

    /// Converts this date to a different calendar system.
    #[must_use]
    pub fn to_calendar(&self, provider: &dyn IcuProvider, target: CalendarSystem) -> CalendarDate {
        if self.calendar == target {
            return self.clone();
        }

        provider.convert_date(self, target)
    }

    /// Returns the weekday for Gregorian dates.
    #[must_use]
    pub fn weekday(&self) -> Weekday {
        debug_assert!(
            self.calendar == CalendarSystem::Gregorian,
            "weekday() is Gregorian-only"
        );

        let year = if self.month.get() < 3 {
            self.year - 1
        } else {
            self.year
        };

        let month = usize::from(self.month.get() - 1);
        let day = i32::from(self.day.get());

        // Sakamoto month offsets for a Sunday-zero Gregorian weekday
        // calculation. Jan/Feb use the previous year, which is why the branch
        // above adjusts `year` first.
        const OFFSETS: [i64; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];

        let year = i64::from(year);
        let day = i64::from(day);

        let weekday = year + year.div_euclid(4) - year.div_euclid(100)
            + year.div_euclid(400)
            + OFFSETS[month]
            + day;

        Weekday::from_sunday_zero(weekday.rem_euclid(7) as u8)
    }

    fn to_jdn(&self) -> i64 {
        debug_assert!(
            self.calendar == CalendarSystem::Gregorian,
            "to_jdn is Gregorian-only"
        );

        // `iso_to_epoch_days` returns a `0001-01-01`-relative day count; add
        // the fixed offset to move into Julian day numbering for arithmetic.
        iso_to_epoch_days(self.year, self.month.get(), self.day.get()) + GREGORIAN_JDN_OFFSET
    }

    fn from_jdn(jdn: i64, calendar: CalendarSystem) -> CalendarDate {
        debug_assert!(
            calendar == CalendarSystem::Gregorian,
            "from_jdn is Gregorian-only"
        );

        // Undo the Julian-day offset before converting back into Gregorian
        // year-month-day fields.
        let (year, month, day) = epoch_days_to_iso(jdn - GREGORIAN_JDN_OFFSET);

        CalendarDate {
            calendar,
            era: None,
            year,
            month: NonZero::new(month).expect("JDN yields a 1-based month"),
            day: NonZero::new(day).expect("JDN yields a 1-based day"),
        }
    }
}

const fn fixed_months_per_year(calendar: CalendarSystem) -> Option<u8> {
    match calendar {
        CalendarSystem::Gregorian
        | CalendarSystem::Buddhist
        | CalendarSystem::Japanese
        | CalendarSystem::Islamic
        | CalendarSystem::IslamicCivil
        | CalendarSystem::IslamicUmmAlQura
        | CalendarSystem::Persian
        | CalendarSystem::Indian
        | CalendarSystem::Roc => Some(12),
        CalendarSystem::Coptic | CalendarSystem::Ethiopic | CalendarSystem::EthiopicAmeteAlem => {
            Some(13)
        }
        CalendarSystem::Hebrew | CalendarSystem::Chinese | CalendarSystem::Dangi => None,
    }
}
