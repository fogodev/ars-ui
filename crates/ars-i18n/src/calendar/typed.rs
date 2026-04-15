use core::{
    cmp::Ordering,
    fmt::{self, Display},
    marker::PhantomData,
    num::NonZero,
};

use super::{CalendarDate, CalendarSystem, Era};
use crate::{IcuProvider, Weekday};

mod sealed {
    pub trait Sealed {}
}

/// Marker trait for a statically-known calendar system.
///
/// `CalendarDate` remains the dynamic, data-driven boundary type used by
/// provider APIs and serialization. `CalendarKind` powers the typed calendar
/// overlay that exposes calendar-specific methods only on compatible dates.
pub trait CalendarKind: sealed::Sealed + Copy + Clone + Default + 'static {
    /// The dynamic calendar system represented by this marker type.
    const SYSTEM: CalendarSystem;
}

/// Capability trait for typed calendars that support direct day arithmetic
/// without an [`IcuProvider`].
pub trait DirectDayArithmetic: CalendarKind {}

/// Capability trait for typed calendars that support direct weekday
/// computation without an [`IcuProvider`].
pub trait DirectWeekdayComputation: CalendarKind {}

/// Zero-sized marker for the Gregorian calendar system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Gregorian;

impl sealed::Sealed for Gregorian {}

impl CalendarKind for Gregorian {
    const SYSTEM: CalendarSystem = CalendarSystem::Gregorian;
}

impl DirectDayArithmetic for Gregorian {}

impl DirectWeekdayComputation for Gregorian {}

/// Zero-sized marker for the Buddhist calendar system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Buddhist;

impl sealed::Sealed for Buddhist {}

impl CalendarKind for Buddhist {
    const SYSTEM: CalendarSystem = CalendarSystem::Buddhist;
}

/// Zero-sized marker for the Japanese imperial calendar system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Japanese;

impl sealed::Sealed for Japanese {}

impl CalendarKind for Japanese {
    const SYSTEM: CalendarSystem = CalendarSystem::Japanese;
}

/// Zero-sized marker for the Hebrew calendar system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Hebrew;

impl sealed::Sealed for Hebrew {}

impl CalendarKind for Hebrew {
    const SYSTEM: CalendarSystem = CalendarSystem::Hebrew;
}

/// Zero-sized marker for the astronomical Hijri calendar system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Islamic;

impl sealed::Sealed for Islamic {}

impl CalendarKind for Islamic {
    const SYSTEM: CalendarSystem = CalendarSystem::Islamic;
}

/// Zero-sized marker for the tabular Hijri civil calendar system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct IslamicCivil;

impl sealed::Sealed for IslamicCivil {}

impl CalendarKind for IslamicCivil {
    const SYSTEM: CalendarSystem = CalendarSystem::IslamicCivil;
}

/// Zero-sized marker for the Umm al-Qura Hijri calendar system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct IslamicUmmAlQura;

impl sealed::Sealed for IslamicUmmAlQura {}

impl CalendarKind for IslamicUmmAlQura {
    const SYSTEM: CalendarSystem = CalendarSystem::IslamicUmmAlQura;
}

/// Zero-sized marker for the Persian Solar Hijri calendar system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Persian;

impl sealed::Sealed for Persian {}

impl CalendarKind for Persian {
    const SYSTEM: CalendarSystem = CalendarSystem::Persian;
}

/// Zero-sized marker for the Indian national calendar system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Indian;

impl sealed::Sealed for Indian {}

impl CalendarKind for Indian {
    const SYSTEM: CalendarSystem = CalendarSystem::Indian;
}

/// Zero-sized marker for the Chinese lunisolar calendar system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Chinese;

impl sealed::Sealed for Chinese {}

impl CalendarKind for Chinese {
    const SYSTEM: CalendarSystem = CalendarSystem::Chinese;
}

/// Zero-sized marker for the Coptic calendar system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Coptic;

impl sealed::Sealed for Coptic {}

impl CalendarKind for Coptic {
    const SYSTEM: CalendarSystem = CalendarSystem::Coptic;
}

/// Zero-sized marker for the Korean Dangi lunisolar calendar system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Dangi;

impl sealed::Sealed for Dangi {}

impl CalendarKind for Dangi {
    const SYSTEM: CalendarSystem = CalendarSystem::Dangi;
}

/// Zero-sized marker for the Ethiopic calendar system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Ethiopic;

impl sealed::Sealed for Ethiopic {}

impl CalendarKind for Ethiopic {
    const SYSTEM: CalendarSystem = CalendarSystem::Ethiopic;
}

/// Zero-sized marker for the Ethiopic Amete Alem calendar system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct EthiopicAmeteAlem;

impl sealed::Sealed for EthiopicAmeteAlem {}

impl CalendarKind for EthiopicAmeteAlem {
    const SYSTEM: CalendarSystem = CalendarSystem::EthiopicAmeteAlem;
}

/// Zero-sized marker for the Republic of China (Minguo) calendar system.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Roc;

impl sealed::Sealed for Roc {}

impl CalendarKind for Roc {
    const SYSTEM: CalendarSystem = CalendarSystem::Roc;
}

/// Error returned when a dynamic [`CalendarDate`] is wrapped in the wrong typed
/// calendar marker.
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

/// A typed calendar date view whose available methods are constrained by the
/// calendar marker `C`.
///
/// This sits on top of the dynamic [`CalendarDate`] boundary type. Use it when
/// the calendar system is already known statically and calendar-specific
/// operations should be enforced by the type system instead of runtime checks.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TypedCalendarDate<C: CalendarKind> {
    raw: CalendarDate,
    marker: PhantomData<C>,
}

impl CalendarDate {
    /// Wraps this dynamic date in a typed calendar view when the calendar
    /// marker matches the stored calendar system.
    ///
    /// # Errors
    ///
    /// Returns [`CalendarTypeError`] when `self.calendar` does not match
    /// `C::SYSTEM`.
    pub fn typed<C: CalendarKind>(&self) -> Result<TypedCalendarDate<C>, CalendarTypeError> {
        TypedCalendarDate::try_from(self)
    }

    /// Consumes this dynamic date and wraps it in a typed calendar view when
    /// the calendar marker matches the stored calendar system.
    ///
    /// # Errors
    ///
    /// Returns [`CalendarTypeError`] when `self.calendar` does not match
    /// `C::SYSTEM`.
    pub fn into_typed<C: CalendarKind>(self) -> Result<TypedCalendarDate<C>, CalendarTypeError> {
        TypedCalendarDate::try_from(self)
    }

    /// Converts this date through the provider into a statically-known target
    /// calendar type.
    #[must_use]
    pub fn to_calendar_type<C: CalendarKind>(
        &self,
        provider: &dyn IcuProvider,
    ) -> TypedCalendarDate<C> {
        let converted = self.to_calendar(provider, C::SYSTEM);

        debug_assert_eq!(converted.calendar, C::SYSTEM);

        TypedCalendarDate::from_raw_unchecked(converted)
    }
}

impl<C: CalendarKind> TypedCalendarDate<C> {
    const fn from_raw_unchecked(raw: CalendarDate) -> Self {
        Self {
            raw,
            marker: PhantomData,
        }
    }

    /// Creates a typed calendar date from a dynamic date when the marker and
    /// stored calendar agree.
    ///
    /// # Errors
    ///
    /// Returns [`CalendarTypeError`] when `raw.calendar` does not match
    /// `C::SYSTEM`.
    pub fn from_raw(raw: CalendarDate) -> Result<Self, CalendarTypeError> {
        if raw.calendar == C::SYSTEM {
            Ok(Self::from_raw_unchecked(raw))
        } else {
            Err(CalendarTypeError {
                expected: C::SYSTEM,
                found: raw.calendar,
            })
        }
    }

    /// Returns the statically-known calendar system for this typed date.
    #[must_use]
    pub const fn calendar_system() -> CalendarSystem {
        C::SYSTEM
    }

    /// Returns the underlying dynamic date.
    #[must_use]
    pub const fn as_raw(&self) -> &CalendarDate {
        &self.raw
    }

    /// Consumes the typed wrapper and returns the underlying dynamic date.
    #[must_use]
    pub fn into_raw(self) -> CalendarDate {
        self.raw
    }

    /// Returns the date's era when one is present.
    #[must_use]
    pub const fn era(&self) -> Option<&Era> {
        self.raw.era.as_ref()
    }

    /// Returns the display year in the typed calendar.
    #[must_use]
    pub const fn year(&self) -> i32 {
        self.raw.year
    }

    /// Returns the 1-based month ordinal.
    #[must_use]
    pub const fn month(&self) -> NonZero<u8> {
        self.raw.month
    }

    /// Returns the 1-based day of month.
    #[must_use]
    pub const fn day(&self) -> NonZero<u8> {
        self.raw.day
    }

    /// Returns the number of days in the current month.
    #[must_use]
    pub fn days_in_month(&self, provider: &dyn IcuProvider) -> u8 {
        self.raw.days_in_month(provider)
    }

    /// Adds whole months within the same typed calendar.
    #[must_use]
    pub fn add_months(&self, provider: &dyn IcuProvider, month_delta: i32) -> Option<Self> {
        self.raw
            .add_months(provider, month_delta)
            .map(Self::from_raw_unchecked)
    }

    /// Adds whole days using provider-backed cross-calendar conversion when
    /// necessary.
    #[must_use]
    pub fn add_days_with_provider(&self, provider: &dyn IcuProvider, day_delta: i32) -> Self {
        Self::from_raw_unchecked(self.raw.add_days_with_provider(provider, day_delta))
    }

    /// Converts this typed date into another statically-known calendar type.
    #[must_use]
    pub fn to_calendar<T: CalendarKind>(&self, provider: &dyn IcuProvider) -> TypedCalendarDate<T> {
        let converted = self.raw.to_calendar(provider, T::SYSTEM);

        debug_assert_eq!(converted.calendar, T::SYSTEM);

        TypedCalendarDate::from_raw_unchecked(converted)
    }

    /// Compares two typed dates when their raw field ordering is meaningful.
    #[must_use]
    pub fn compare_within_calendar(&self, other: &Self) -> Option<Ordering> {
        self.raw.compare_within_calendar(&other.raw)
    }
}

impl<C: DirectDayArithmetic> TypedCalendarDate<C> {
    /// Adds whole days using direct arithmetic supported by this calendar kind.
    #[must_use]
    pub fn add_days(&self, day_delta: i32) -> Self {
        let shifted = self
            .raw
            .add_days(day_delta)
            .expect("typed direct day arithmetic is only implemented for compatible calendars");

        Self::from_raw_unchecked(shifted)
    }
}

impl<C: DirectWeekdayComputation> TypedCalendarDate<C> {
    /// Returns the weekday using direct calendar-specific computation.
    #[must_use]
    pub fn weekday(&self) -> Weekday {
        self.raw.weekday()
    }
}

impl TypedCalendarDate<Gregorian> {
    /// Creates a typed Gregorian date from validated month and day components.
    #[must_use]
    pub const fn new(year: i32, month: NonZero<u8>, day: NonZero<u8>) -> Self {
        Self {
            raw: CalendarDate::new_gregorian(year, month, day),
            marker: PhantomData,
        }
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
