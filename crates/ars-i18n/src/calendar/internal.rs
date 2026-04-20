#[cfg(any(
    test,
    all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x"))
))]
use alloc::string::String;
use alloc::string::ToString;

#[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
use icu::calendar::AnyCalendarKind;
use icu::calendar::{
    AnyCalendar, Date, Iso,
    cal::{
        Buddhist, ChineseTraditional, Coptic, Ethiopian, EthiopianEraStyle, Hebrew, Hijri, Indian,
        Japanese, KoreanTraditional, Persian, Roc,
        hijri::{self},
    },
    error::DateFromFieldsError,
    types::{DateFields, YearInput},
};
#[cfg(test)]
use {super::CalendarError, crate::Weekday};

use super::{CalendarConversionError, CalendarSystem, DateError};

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
        let date = date_from_ordinal_fields(year, month, day, calendar)
            .map_err(|error| CalendarConversionError::Icu(error.to_string()))?;

        Ok(Self { inner: date })
    }

    /// Converts the date into a different calendar system.
    #[must_use]
    #[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
    pub(crate) fn to_calendar(&self, calendar: CalendarSystem) -> Self {
        Self {
            inner: self.inner.to_calendar(any_calendar_for(calendar)),
        }
    }

    /// Returns the public year for the date's calendar.
    ///
    /// Gregorian public dates use astronomical year numbering even though ICU
    /// exposes CE/BCE era years for Gregorian dates.
    #[must_use]
    #[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
    pub(crate) fn year(&self) -> i32 {
        if self.inner.calendar().kind() == AnyCalendarKind::Gregorian {
            self.inner.year().extended_year()
        } else {
            self.inner.year().era_year_or_related_iso()
        }
    }

    /// Returns the 1-based month ordinal.
    #[must_use]
    #[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
    pub(crate) fn month(&self) -> u8 {
        self.inner.month().ordinal
    }

    /// Returns the 1-based day of month.
    #[must_use]
    #[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
    pub(crate) fn day(&self) -> u8 {
        self.inner.day_of_month().0
    }

    /// Returns the ISO weekday.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn weekday(&self) -> Weekday {
        Weekday::from_icu_weekday(self.inner.weekday())
    }

    /// Returns the era code when one is present.
    #[must_use]
    #[cfg(all(feature = "web-intl", target_arch = "wasm32", not(feature = "icu4x")))]
    pub(crate) fn era(&self) -> Option<String> {
        self.inner.year().era().map(|era| era.era.to_string())
    }

    /// Adds a whole-day offset to the date.
    #[cfg(test)]
    pub(crate) fn add_days(&self, days: i32) -> Result<Self, CalendarError> {
        use super::helpers::{epoch_days_to_iso, iso_to_epoch_days};

        let iso = self.inner.to_calendar(Iso);

        let epoch_days = iso_to_epoch_days(
            iso.year().era_year_or_related_iso(),
            iso.month().ordinal,
            iso.day_of_month().0,
        );

        let (year, month, day) = epoch_days_to_iso(epoch_days + i64::from(days));

        let next = Date::try_new_iso(year, month, day)
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

        Ok(Self {
            inner: next.to_any().to_calendar(self.inner.calendar().clone()),
        })
    }

    /// Returns the day offset to `other`.
    #[cfg(test)]
    pub(crate) fn days_until(&self, other: &Self) -> Result<i32, CalendarError> {
        use super::helpers::iso_to_epoch_days;

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

        i32::try_from(diff)
            .map_err(|_| CalendarError::Arithmetic("date difference exceeds i32 range".to_string()))
    }

    /// Returns `true` when `self` is chronologically before `other`.
    #[cfg(test)]
    pub(crate) fn is_before(&self, other: &Self) -> Result<bool, CalendarError> {
        self.days_until(other).map(|diff| diff > 0)
    }

    /// Returns today's date in the requested calendar system.
    #[cfg(all(test, feature = "std", feature = "icu4x"))]
    pub(crate) fn today(calendar: CalendarSystem) -> Result<Self, CalendarError> {
        use super::helpers::platform_today_iso;

        let (year, month, day) = platform_today_iso().map_err(CalendarError::Arithmetic)?;

        let iso = Date::<Iso>::try_new_iso(year, month, day)
            .map_err(|error| CalendarError::Arithmetic(error.to_string()))?;

        Ok(Self {
            inner: iso.to_calendar(any_calendar_for(calendar)),
        })
    }
}

pub(crate) fn months_in_year(year: i32, calendar: CalendarSystem, era: Option<&str>) -> Option<u8> {
    let date = date_from_ordinal_fields(year_input_from_parts(calendar, era, year), 1, 1, calendar)
        .ok()?;

    Some(date.months_in_year())
}

pub(crate) fn days_in_month(
    year: i32,
    month: u8,
    calendar: CalendarSystem,
    era: Option<&str>,
) -> Option<u8> {
    let date = date_from_ordinal_fields(
        year_input_from_parts(calendar, era, year),
        month,
        1,
        calendar,
    )
    .ok()?;

    Some(date.days_in_month())
}

impl TryFrom<&super::CalendarDate> for CalendarDate {
    type Error = CalendarConversionError;

    fn try_from(value: &super::CalendarDate) -> Result<Self, Self::Error> {
        if let Some(era) = &value.era {
            return Self::from_calendar_with_era(
                &era.code,
                value.year,
                value.month,
                value.day,
                value.calendar,
            );
        }

        if value.calendar == CalendarSystem::Gregorian {
            return Self::from_iso(value.year, value.month, value.day).map_err(
                |error| match error {
                    DateError::InvalidDate => CalendarConversionError::InvalidDate,
                    DateError::OutOfRange | DateError::CalendarError(_) => {
                        CalendarConversionError::Icu(error.to_string())
                    }
                },
            );
        }

        Self::from_calendar(value.year, value.month, value.day, value.calendar)
    }
}

fn date_from_ordinal_fields(
    year: YearInput<'_>,
    ordinal_month: u8,
    day: u8,
    calendar: CalendarSystem,
) -> Result<Date<AnyCalendar>, DateFromFieldsError> {
    let mut fields = DateFields::default();

    assign_year_fields(&mut fields, year);

    fields.ordinal_month = Some(ordinal_month);
    fields.day = Some(day);

    Date::try_from_fields(fields, Default::default(), any_calendar_for(calendar))
}

fn assign_year_fields<'a>(fields: &mut DateFields<'a>, year: YearInput<'a>) {
    match year {
        YearInput::Extended(extended_year) => {
            fields.extended_year = Some(extended_year);
        }

        YearInput::EraYear(era, era_year) => {
            fields.era = Some(era.as_bytes());
            fields.era_year = Some(era_year);
        }

        _ => unreachable!("ICU4X YearInput currently only supports extended and era-year forms"),
    }
}

const fn default_year_input(calendar: CalendarSystem, year: i32) -> YearInput<'static> {
    if let Some(era) = default_era_code(calendar) {
        YearInput::EraYear(era, year)
    } else {
        YearInput::Extended(year)
    }
}

fn year_input_from_parts<'a>(
    calendar: CalendarSystem,
    era: Option<&'a str>,
    year: i32,
) -> YearInput<'a> {
    match era {
        Some("bce") if matches!(calendar, CalendarSystem::Coptic) => YearInput::Extended(1 - year),

        Some("ce") if matches!(calendar, CalendarSystem::Coptic) => YearInput::Extended(year),

        Some(era_code) => YearInput::EraYear(era_code, year),

        None => default_year_input(calendar, year),
    }
}

const fn default_era_code(calendar: CalendarSystem) -> Option<&'static str> {
    match calendar {
        CalendarSystem::Buddhist => Some("be"),

        CalendarSystem::Japanese => Some("reiwa"),

        CalendarSystem::Hebrew | CalendarSystem::Coptic | CalendarSystem::Ethiopic => Some("am"),

        CalendarSystem::IslamicCivil | CalendarSystem::IslamicUmmAlQura => Some("ah"),

        CalendarSystem::Persian => Some("ap"),

        CalendarSystem::Indian => Some("shaka"),

        CalendarSystem::EthiopicAmeteAlem => Some("aa"),

        CalendarSystem::Roc => Some("roc"),

        _ => None,
    }
}

fn any_calendar_for(calendar: CalendarSystem) -> AnyCalendar {
    match calendar {
        CalendarSystem::Iso8601 => AnyCalendar::Iso(Iso),

        CalendarSystem::Gregorian => AnyCalendar::Gregorian(icu::calendar::Gregorian),

        CalendarSystem::Buddhist => AnyCalendar::Buddhist(Buddhist),

        CalendarSystem::Japanese => AnyCalendar::Japanese(Japanese::default()),

        CalendarSystem::Hebrew => AnyCalendar::Hebrew(Hebrew),

        CalendarSystem::IslamicCivil => AnyCalendar::HijriTabular(Hijri::new_tabular(
            hijri::TabularAlgorithmLeapYears::TypeII,
            hijri::TabularAlgorithmEpoch::Friday,
        )),

        CalendarSystem::IslamicUmmAlQura => AnyCalendar::HijriUmmAlQura(Hijri::new_umm_al_qura()),

        CalendarSystem::Persian => AnyCalendar::Persian(Persian),

        CalendarSystem::Indian => AnyCalendar::Indian(Indian),

        CalendarSystem::Chinese => AnyCalendar::Chinese(ChineseTraditional::new()),

        CalendarSystem::Coptic => AnyCalendar::Coptic(Coptic),

        CalendarSystem::Dangi => AnyCalendar::Dangi(KoreanTraditional::new()),

        CalendarSystem::Ethiopic => AnyCalendar::Ethiopian(Ethiopian::new_with_era_style(
            EthiopianEraStyle::AmeteMihret,
        )),

        CalendarSystem::EthiopicAmeteAlem => {
            AnyCalendar::Ethiopian(Ethiopian::new_with_era_style(EthiopianEraStyle::AmeteAlem))
        }

        CalendarSystem::Roc => AnyCalendar::Roc(Roc),
    }
}

#[cfg(test)]
mod tests {
    use icu::calendar::types::YearInput;

    use super::*;
    use crate::{CalendarDate as PublicCalendarDate, CalendarDateFields, Era};

    #[test]
    fn internal_year_input_helpers_cover_default_and_explicit_era_paths() {
        assert!(matches!(
            default_year_input(CalendarSystem::Gregorian, 2024),
            YearInput::Extended(2024)
        ));
        assert!(matches!(
            default_year_input(CalendarSystem::Buddhist, 2567),
            YearInput::EraYear("be", 2567)
        ));
        assert!(matches!(
            year_input_from_parts(CalendarSystem::Coptic, Some("bce"), 5),
            YearInput::Extended(-4)
        ));
        assert!(matches!(
            year_input_from_parts(CalendarSystem::Coptic, Some("ce"), 5),
            YearInput::Extended(5)
        ));
        assert!(matches!(
            year_input_from_parts(CalendarSystem::Japanese, Some("heisei"), 2),
            YearInput::EraYear("heisei", 2)
        ));
        assert_eq!(default_era_code(CalendarSystem::Roc), Some("roc"));
        assert_eq!(default_era_code(CalendarSystem::Gregorian), None);
    }

    #[test]
    fn internal_calendar_date_supports_roundtrip_and_day_arithmetic() {
        let date = CalendarDate::from_iso(2024, 3, 15).expect("Gregorian fixture");

        let next = date.add_days(1).expect("day arithmetic should succeed");

        assert_eq!(date.weekday(), Weekday::Friday);
        assert_eq!(date.days_until(&next).expect("difference should fit"), 1);
        assert!(date.is_before(&next).expect("ordering should succeed"));
    }

    #[test]
    fn internal_calendar_conversion_helpers_cover_public_dates_and_month_queries() {
        let gregorian = PublicCalendarDate::new_gregorian(2024, 3, 15).expect("Gregorian fixture");

        let internal = CalendarDate::try_from(&gregorian).expect("Gregorian conversion");

        assert_eq!(internal.weekday(), Weekday::Friday);

        let japanese = PublicCalendarDate::new(
            CalendarSystem::Japanese,
            &CalendarDateFields {
                era: Some(Era {
                    code: String::from("heisei"),
                    display_name: String::from("Heisei"),
                }),
                year: Some(31),
                month: Some(4),
                day: Some(30),
                ..CalendarDateFields::default()
            },
        )
        .expect("Japanese fixture should validate");

        assert!(CalendarDate::try_from(&japanese).is_ok());
        assert_eq!(months_in_year(5784, CalendarSystem::Hebrew, None), Some(13));
        assert_eq!(
            days_in_month(31, 4, CalendarSystem::Japanese, Some("heisei")),
            Some(30)
        );
        assert_eq!(
            days_in_month(2024, 2, CalendarSystem::Gregorian, None),
            Some(29)
        );
    }

    #[test]
    fn internal_calendar_helpers_cover_invalid_inputs_and_remaining_variants() {
        let invalid =
            date_from_ordinal_fields(YearInput::Extended(2024), 13, 1, CalendarSystem::Gregorian);

        assert!(invalid.is_err());

        let buddhist = CalendarDate::from_calendar(2567, 1, 1, CalendarSystem::Buddhist)
            .expect("Buddhist date should validate");

        let roc = CalendarDate::from_calendar_with_era("roc", 113, 1, 1, CalendarSystem::Roc)
            .expect("ROC date with explicit era should validate");

        assert!(
            !buddhist
                .is_before(&buddhist)
                .expect("comparison should succeed")
        );
        assert_eq!(
            buddhist.days_until(&buddhist).expect("same-day difference"),
            0
        );
        assert_eq!(
            months_in_year(2024, CalendarSystem::Chinese, None),
            Some(12)
        );
        assert_eq!(
            days_in_month(113, 13, CalendarSystem::Roc, Some("roc")),
            None
        );
        assert_eq!(roc.weekday(), Weekday::Monday);
    }

    #[test]
    fn internal_try_from_maps_public_date_errors_and_today_projects_requested_calendar() {
        let invalid_gregorian = PublicCalendarDate::new(
            CalendarSystem::Gregorian,
            &CalendarDateFields {
                year: Some(2024),
                month: Some(2),
                day: Some(30),
                ..CalendarDateFields::default()
            },
        );

        assert!(invalid_gregorian.is_err());

        let bad_japanese = PublicCalendarDate::new(
            CalendarSystem::Japanese,
            &CalendarDateFields {
                era: Some(Era {
                    code: String::from("reiwa"),
                    display_name: String::from("Reiwa"),
                }),
                year: Some(1),
                month: Some(4),
                day: Some(30),
                ..CalendarDateFields::default()
            },
        );

        assert!(bad_japanese.is_err());

        #[cfg(all(feature = "std", feature = "icu4x"))]
        let today = CalendarDate::today(CalendarSystem::Persian)
            .expect("today should project into the requested calendar");

        #[cfg(all(feature = "std", feature = "icu4x"))]
        assert_eq!(
            today.inner.calendar().kind(),
            any_calendar_for(CalendarSystem::Persian).kind()
        );
    }

    #[test]
    fn internal_helpers_cover_remaining_calendar_variants_and_invalid_public_dates() {
        let invalid_public = PublicCalendarDate {
            calendar: CalendarSystem::Gregorian,
            era: None,
            year: 2024,
            month: 2,
            month_code: None,
            day: 30,
            iso_year: 2024,
            iso_month: 2,
            iso_day: 30,
        };

        let invalid = CalendarDate::try_from(&invalid_public)
            .expect_err("invalid Gregorian public dates should map to a conversion error");

        assert_eq!(invalid, CalendarConversionError::InvalidDate);

        let variants = [
            (CalendarSystem::IslamicCivil, 1445, 9, 1),
            (CalendarSystem::IslamicUmmAlQura, 1445, 9, 1),
            (CalendarSystem::Persian, 1403, 1, 1),
            (CalendarSystem::Indian, 1946, 1, 1),
            (CalendarSystem::Dangi, 2024, 1, 1),
            (CalendarSystem::Ethiopic, 2016, 1, 1),
            (CalendarSystem::EthiopicAmeteAlem, 7516, 1, 1),
        ];

        for (calendar, year, month, day) in variants {
            let date = CalendarDate::from_calendar(year, month, day, calendar)
                .expect("calendar variant fixture should validate");

            assert_eq!(
                date.inner.calendar().kind(),
                any_calendar_for(calendar).kind()
            );
        }

        assert_eq!(
            months_in_year(1403, CalendarSystem::Persian, Some("ap")),
            Some(12)
        );
        assert_eq!(
            days_in_month(1946, 1, CalendarSystem::Indian, Some("shaka")),
            Some(31)
        );
        assert_eq!(
            days_in_month(1445, 9, CalendarSystem::IslamicCivil, Some("ah")),
            Some(30)
        );
    }
}
