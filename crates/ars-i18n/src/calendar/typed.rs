use alloc::string::ToString;
use core::{
    cmp::Ordering,
    fmt::{self, Display},
    marker::PhantomData,
};

use super::{CalendarConversionError, CalendarDate, CalendarDateFields, CalendarSystem, Era};

mod sealed {
    pub trait Sealed {}
}

/// Marker trait for a statically-known calendar system.
pub trait CalendarKind: sealed::Sealed + Copy + Clone + Default + 'static {
    /// The dynamic calendar system represented by this marker type.
    const SYSTEM: CalendarSystem;
}

/// Capability trait for typed calendars that support direct day arithmetic.
pub trait DirectDayArithmetic: CalendarKind {}

/// Capability trait for typed calendars that support direct weekday computation.
pub trait DirectWeekdayComputation: CalendarKind {}

macro_rules! calendar_kind {
    ($name:ident, $system:expr) => {
        #[doc = concat!("Statically-known marker for the `", stringify!($system), "` calendar.")]
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        pub struct $name;

        impl sealed::Sealed for $name {}

        impl CalendarKind for $name {
            const SYSTEM: CalendarSystem = $system;
        }
    };
}

macro_rules! direct_day_arithmetic {
    ($name:ident) => {
        impl DirectDayArithmetic for $name {}
    };
}

macro_rules! direct_weekday_computation {
    ($name:ident) => {
        impl DirectWeekdayComputation for $name {}
    };
}

calendar_kind!(Gregorian, CalendarSystem::Gregorian);
calendar_kind!(Buddhist, CalendarSystem::Buddhist);
calendar_kind!(Japanese, CalendarSystem::Japanese);
calendar_kind!(Hebrew, CalendarSystem::Hebrew);
calendar_kind!(IslamicCivil, CalendarSystem::IslamicCivil);
calendar_kind!(IslamicUmmAlQura, CalendarSystem::IslamicUmmAlQura);
calendar_kind!(Persian, CalendarSystem::Persian);
calendar_kind!(Indian, CalendarSystem::Indian);
calendar_kind!(Chinese, CalendarSystem::Chinese);
calendar_kind!(Coptic, CalendarSystem::Coptic);
calendar_kind!(Dangi, CalendarSystem::Dangi);
calendar_kind!(Ethiopic, CalendarSystem::Ethiopic);
calendar_kind!(EthiopicAmeteAlem, CalendarSystem::EthiopicAmeteAlem);
calendar_kind!(Roc, CalendarSystem::Roc);

direct_day_arithmetic!(Gregorian);
direct_weekday_computation!(Gregorian);

/// Error returned when a dynamic [`CalendarDate`] is wrapped in the wrong typed calendar marker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarTypeError {
    /// The statically requested calendar system.
    pub expected: CalendarSystem,
    /// The actual runtime calendar system carried by the date.
    pub found: CalendarSystem,
}

impl Display for CalendarTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "calendar type mismatch: expected {:?}, found {:?}",
            self.expected, self.found
        )
    }
}

impl core::error::Error for CalendarTypeError {}

/// A typed calendar-date view constrained by a marker type.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypedCalendarDate<C: CalendarKind> {
    raw: CalendarDate,
    marker: PhantomData<C>,
}

impl CalendarDate {
    /// Wraps this date in a typed calendar view when the marker matches.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested marker does not match the runtime
    /// calendar carried by this date.
    pub fn typed<C: CalendarKind>(&self) -> Result<TypedCalendarDate<C>, CalendarTypeError> {
        TypedCalendarDate::try_from(self.clone())
    }

    /// Consumes this date and wraps it in a typed calendar view when the marker matches.
    ///
    /// # Errors
    ///
    /// Returns an error if the requested marker does not match the runtime
    /// calendar carried by this date.
    pub fn into_typed<C: CalendarKind>(self) -> Result<TypedCalendarDate<C>, CalendarTypeError> {
        TypedCalendarDate::try_from(self)
    }

    /// Converts this date into a statically-known calendar type.
    ///
    /// # Errors
    ///
    /// Returns an error if calendar conversion fails or if the converted date
    /// cannot be wrapped in the requested marker type.
    pub fn to_calendar_type<C: CalendarKind>(
        &self,
    ) -> Result<TypedCalendarDate<C>, CalendarConversionError> {
        self.to_calendar(C::SYSTEM).and_then(|date| {
            TypedCalendarDate::from_raw(date)
                .map_err(|error| CalendarConversionError::Icu(error.to_string()))
        })
    }
}

impl<C: CalendarKind> TypedCalendarDate<C> {
    const fn from_raw_unchecked(raw: CalendarDate) -> Self {
        Self {
            raw,
            marker: PhantomData,
        }
    }

    /// Creates a typed date from a dynamic date when the marker and runtime calendar agree.
    ///
    /// # Errors
    ///
    /// Returns an error if the marker and runtime calendar differ.
    pub fn from_raw(raw: CalendarDate) -> Result<Self, CalendarTypeError> {
        if raw.calendar() == C::SYSTEM {
            Ok(Self::from_raw_unchecked(raw))
        } else {
            Err(CalendarTypeError {
                expected: C::SYSTEM,
                found: raw.calendar(),
            })
        }
    }

    /// Returns the statically-known calendar system.
    #[must_use]
    pub const fn calendar_system() -> CalendarSystem {
        C::SYSTEM
    }

    /// Returns the underlying dynamic date.
    #[must_use]
    pub const fn as_raw(&self) -> &CalendarDate {
        &self.raw
    }

    /// Consumes the typed wrapper and returns the raw date.
    #[must_use]
    pub fn into_raw(self) -> CalendarDate {
        self.raw
    }

    /// Returns the stored era.
    #[must_use]
    pub const fn era(&self) -> Option<&Era> {
        self.raw.era()
    }

    /// Returns the display year.
    #[must_use]
    pub const fn year(&self) -> i32 {
        self.raw.year()
    }

    /// Returns the display month.
    #[must_use]
    pub const fn month(&self) -> u8 {
        self.raw.month()
    }

    /// Returns the display day.
    #[must_use]
    pub const fn day(&self) -> u8 {
        self.raw.day()
    }

    /// Returns the number of days in the current month.
    #[must_use]
    pub fn days_in_month(&self) -> u8 {
        self.raw.days_in_month()
    }

    /// Adds whole months.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying dynamic date arithmetic fails.
    pub fn add_months(&self, month_delta: i32) -> Result<Self, super::CalendarError> {
        self.raw
            .add(super::DateDuration {
                months: month_delta,
                ..super::DateDuration::default()
            })
            .map(Self::from_raw_unchecked)
    }

    /// Converts this typed date into another statically-known calendar type.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying calendar conversion fails.
    pub fn to_calendar<T: CalendarKind>(
        &self,
    ) -> Result<TypedCalendarDate<T>, CalendarConversionError> {
        self.raw.to_calendar_type::<T>()
    }

    /// Compares two typed dates within the same calendar.
    #[must_use]
    pub fn compare_within_calendar(&self, other: &Self) -> Option<Ordering> {
        self.raw.compare_within_calendar(&other.raw)
    }
}

impl<C: DirectDayArithmetic> TypedCalendarDate<C> {
    /// Adds whole days.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying dynamic date arithmetic fails.
    pub fn add_days(&self, day_delta: i32) -> Result<Self, super::CalendarError> {
        self.raw.add_days(day_delta).map(Self::from_raw_unchecked)
    }
}

impl TypedCalendarDate<Gregorian> {
    /// Creates a typed Gregorian date from ISO-compatible fields.
    ///
    /// # Errors
    ///
    /// Returns an error if the supplied fields do not form a valid Gregorian
    /// date.
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, super::CalendarError> {
        CalendarDate::new(
            CalendarSystem::Gregorian,
            &CalendarDateFields {
                year: Some(year),
                month: Some(month),
                day: Some(day),
                ..CalendarDateFields::default()
            },
        )
        .map(Self::from_raw_unchecked)
    }
}

impl<C: CalendarKind> AsRef<CalendarDate> for TypedCalendarDate<C> {
    fn as_ref(&self) -> &CalendarDate {
        &self.raw
    }
}

impl<C: CalendarKind> From<TypedCalendarDate<C>> for CalendarDate {
    fn from(value: TypedCalendarDate<C>) -> Self {
        value.raw
    }
}

impl<C: CalendarKind> TryFrom<CalendarDate> for TypedCalendarDate<C> {
    type Error = CalendarTypeError;

    fn try_from(value: CalendarDate) -> Result<Self, Self::Error> {
        Self::from_raw(value)
    }
}

impl<C: CalendarKind> TryFrom<&CalendarDate> for TypedCalendarDate<C> {
    type Error = CalendarTypeError;

    fn try_from(value: &CalendarDate) -> Result<Self, Self::Error> {
        Self::from_raw(value.clone())
    }
}

/// Validated month value (1..=13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Month(u8);

impl Month {
    /// Creates a month value in the library's supported 1..=13 range.
    #[must_use]
    pub const fn new(month: u8) -> Option<Self> {
        if month >= 1 && month <= 13 {
            Some(Self(month))
        } else {
            None
        }
    }

    /// Returns the underlying 1-based month ordinal.
    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Month {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(value)
    }
}

#[cfg(test)]
mod tests {
    use alloc::format;

    use super::*;
    use crate::CalendarError;

    fn buddhist_fixture() -> CalendarDate {
        CalendarDate::new(
            CalendarSystem::Buddhist,
            &CalendarDateFields {
                year: Some(2567),
                month: Some(3),
                day: Some(15),
                ..CalendarDateFields::default()
            },
        )
        .expect("Buddhist fixture should validate")
    }

    #[test]
    fn typed_calendar_date_roundtrips_and_exposes_raw_fields() {
        let raw = CalendarDate::new_gregorian(2024, 3, 15).expect("Gregorian fixture");
        let typed = raw.typed::<Gregorian>().expect("marker should match");

        assert_eq!(
            TypedCalendarDate::<Gregorian>::calendar_system(),
            CalendarSystem::Gregorian
        );
        assert_eq!(typed.as_raw(), &raw);
        assert_eq!(typed.year(), 2024);
        assert_eq!(typed.month(), 3);
        assert_eq!(typed.day(), 15);
        assert_eq!(typed.clone().into_raw(), raw);
        assert_eq!(CalendarDate::from(typed.clone()), raw);
    }

    #[test]
    fn typed_calendar_date_reports_mismatched_marker_systems() {
        let raw = buddhist_fixture();

        let error = TypedCalendarDate::<Gregorian>::from_raw(raw.clone())
            .expect_err("Buddhist date must not wrap as Gregorian");

        assert_eq!(error.expected, CalendarSystem::Gregorian);
        assert_eq!(error.found, CalendarSystem::Buddhist);
        assert_eq!(
            format!("{error}"),
            "calendar type mismatch: expected Gregorian, found Buddhist"
        );
        assert_eq!(raw.to_calendar_type::<Gregorian>().unwrap().year(), 2024);
    }

    #[test]
    fn typed_calendar_date_arithmetic_and_conversion_delegate_to_dynamic_api() {
        let january = TypedCalendarDate::<Gregorian>::new(2024, 1, 31).expect("Gregorian fixture");

        let february = january
            .add_months(1)
            .expect("month arithmetic should constrain");

        let next_day = january.add_days(1).expect("day arithmetic should work");

        let buddhist = january
            .to_calendar::<Buddhist>()
            .expect("calendar conversion should succeed");

        assert_eq!(february.month(), 2);
        assert_eq!(february.day(), 29);
        assert_eq!(next_day.month(), 2);
        assert_eq!(next_day.day(), 1);
        assert_eq!(buddhist.as_raw().calendar(), CalendarSystem::Buddhist);
        assert_eq!(
            january.compare_within_calendar(&next_day),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn gregorian_typed_constructor_and_month_bounds_validate_inputs() {
        assert_eq!(
            TypedCalendarDate::<Gregorian>::new(2024, 2, 29)
                .expect("leap-day Gregorian fixture")
                .days_in_month(),
            29
        );
        assert!(matches!(
            TypedCalendarDate::<Gregorian>::new(2023, 2, 29),
            Err(CalendarError::Arithmetic(_))
        ));

        assert_eq!(Month::new(0), None);
        assert_eq!(Month::new(1).map(Month::get), Some(1));
        assert_eq!(Month::new(13).map(Month::get), Some(13));
        assert_eq!(Month::new(14), None);
        assert_eq!(Month::try_from(13).map(Month::get), Ok(13));
        assert_eq!(Month::try_from(14), Err(14));
    }
}
