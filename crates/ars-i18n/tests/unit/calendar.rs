use alloc::string::{String, ToString};
use core::{cmp::Ordering, num::NonZero};

#[cfg(feature = "icu4x")]
use super::internal::CalendarDate as InternalCalendarDate;
use super::{
    CalendarConversionError, CalendarDate, CalendarError, CalendarSystem, CalendarTypeError,
    DateError, DateRange, Era, Gregorian, HourCycle, Japanese, JapaneseEra, Month, Time,
    TypedCalendarDate, WeekInfo, bounded_days_in_month, bounded_months_in_year, default_era_for,
    epoch_days_to_iso, gregorian_days_in_month, iso_to_epoch_days, minimum_day_in_month,
    minimum_month_in_year, years_in_era,
};
use crate::{IcuProvider, Locale, StubIcuProvider, Weekday};

#[cfg(feature = "icu4x")]
#[test]
fn internal_calendar_date_from_iso_exposes_components() {
    let date = InternalCalendarDate::from_iso(2024, 3, 15).expect("date should be valid");

    assert_eq!(date.year(), 2024);
    assert_eq!(date.month(), 3);
    assert_eq!(date.day(), 15);
    assert_eq!(date.weekday(), Weekday::Friday);
}

#[cfg(feature = "icu4x")]
#[test]
fn internal_calendar_date_from_calendar_accepts_gregorian_fields() {
    let date = InternalCalendarDate::from_calendar(2024, 3, 15, CalendarSystem::Gregorian)
        .expect("gregorian conversion should succeed");

    assert_eq!(date.year(), 2024);
    assert_eq!(date.month(), 3);
    assert_eq!(date.day(), 15);
}

#[cfg(feature = "icu4x")]
#[test]
fn internal_calendar_date_from_calendar_uses_ordinal_months_for_chinese() {
    let date = InternalCalendarDate::from_calendar(2023, 3, 1, CalendarSystem::Chinese)
        .expect("Chinese ordinal month should construct");

    assert_eq!(date.month(), 3);
}

#[cfg(feature = "icu4x")]
#[test]
fn internal_calendar_date_from_calendar_defaults_japanese_to_current_era() {
    let date = InternalCalendarDate::from_calendar(6, 3, 15, CalendarSystem::Japanese)
        .expect("japanese current-era conversion should succeed");
    let gregorian = date.to_calendar(CalendarSystem::Gregorian);

    assert_eq!(date.era(), Some(String::from("reiwa")));
    assert_eq!(gregorian.year(), 2024);
    assert_eq!(gregorian.month(), 3);
    assert_eq!(gregorian.day(), 15);
}

#[cfg(feature = "icu4x")]
#[test]
fn stub_provider_uses_year_dependent_chinese_month_counts() {
    let provider = StubIcuProvider;

    let leap_year = (2020..=2030)
        .find(|year| {
            InternalCalendarDate::from_calendar(*year, 13, 1, CalendarSystem::Chinese).is_ok()
        })
        .expect("fixture range should include a Chinese leap-month year");
    let common_year = (2020..=2030)
        .find(|year| {
            InternalCalendarDate::from_calendar(*year, 13, 1, CalendarSystem::Chinese).is_err()
        })
        .expect("fixture range should include a Chinese common year");

    assert_eq!(
        provider.max_months_in_year(&CalendarSystem::Chinese, leap_year, None),
        13
    );
    assert_eq!(
        provider.max_months_in_year(&CalendarSystem::Chinese, common_year, None),
        12
    );
    assert!(
        CalendarDate::new(&provider, CalendarSystem::Chinese, None, leap_year, 13, 1).is_some()
    );
    assert!(
        CalendarDate::new(&provider, CalendarSystem::Chinese, None, common_year, 13, 1).is_none()
    );
}

#[test]
fn stub_provider_rejects_invalid_coptic_and_ethiopic_epagomenal_days() {
    let provider = StubIcuProvider;

    assert!(
        CalendarDate::new(&provider, CalendarSystem::Coptic, None, 1738, 13, 6).is_none(),
        "common Coptic year month 13 only has five days"
    );
    assert!(
        CalendarDate::new(&provider, CalendarSystem::Coptic, None, 1739, 13, 6).is_some(),
        "leap Coptic year month 13 has six days"
    );
    assert!(
        CalendarDate::new(&provider, CalendarSystem::Ethiopic, None, 2016, 13, 6).is_none(),
        "common Ethiopic year month 13 only has five days"
    );
    assert!(
        CalendarDate::new(&provider, CalendarSystem::Ethiopic, None, 2015, 13, 6).is_some(),
        "leap Ethiopic year month 13 has six days"
    );
}

#[cfg(feature = "icu4x")]
#[test]
fn internal_calendar_date_from_calendar_with_era_preserves_japanese_era() {
    let date =
        InternalCalendarDate::from_calendar_with_era("heisei", 31, 4, 30, CalendarSystem::Japanese)
            .expect("japanese era-aware conversion should succeed");
    let gregorian = date.to_calendar(CalendarSystem::Gregorian);

    assert_eq!(date.era(), Some(String::from("heisei")));
    assert_eq!(gregorian.year(), 2019);
    assert_eq!(gregorian.month(), 4);
    assert_eq!(gregorian.day(), 30);
}

#[cfg(feature = "icu4x")]
#[test]
fn internal_calendar_date_add_days_and_ordering_work() {
    let start = InternalCalendarDate::from_iso(2024, 3, 15).expect("date should be valid");
    let end = start.add_days(3).expect("date arithmetic should succeed");

    assert_eq!(start.days_until(&end), Ok(3));
    assert_eq!(start.is_before(&end), Ok(true));
    assert_eq!(end.day(), 18);
}

#[cfg(feature = "icu4x")]
#[test]
fn internal_calendar_date_conversion_and_era_helpers_are_callable() {
    let gregorian = InternalCalendarDate::from_iso(2024, 3, 15).expect("date should be valid");
    let converted = gregorian.to_calendar(CalendarSystem::Gregorian);

    assert_eq!(converted.year(), 2024);
    assert_eq!(converted.era(), Some(String::from("ce")));
}

#[cfg(all(feature = "icu4x", feature = "std"))]
#[test]
fn internal_calendar_date_today_returns_a_valid_date() {
    let today =
        InternalCalendarDate::today(CalendarSystem::Gregorian).expect("today should resolve");

    assert!((1..=12).contains(&today.month()));
    assert!((1..=31).contains(&today.day()));
}

#[test]
fn calendar_system_from_bcp47_and_locale_cover_spec_examples() {
    assert_eq!(
        CalendarSystem::from_bcp47("islamic-civil"),
        Some(CalendarSystem::IslamicCivil)
    );

    let locale =
        Locale::parse("en-US-u-ca-japanese").expect("locale should include calendar extension");

    assert_eq!(
        CalendarSystem::from_locale(&locale),
        CalendarSystem::Japanese
    );

    let fallback = Locale::parse("en-US").expect("locale should parse");

    assert_eq!(
        CalendarSystem::from_locale(&fallback),
        CalendarSystem::Gregorian
    );
}

#[test]
fn supported_calendars_include_hebrew_and_japanese_metadata() {
    let calendars = CalendarSystem::supported_calendars();

    let hebrew = calendars
        .iter()
        .find(|metadata| metadata.system == CalendarSystem::Hebrew)
        .expect("hebrew metadata should exist");

    let japanese = calendars
        .iter()
        .find(|metadata| metadata.system == CalendarSystem::Japanese)
        .expect("japanese metadata should exist");

    assert_eq!(hebrew.month_range, 1..=13);
    assert!(hebrew.has_leap_months);
    assert!(japanese.era_required);
}

#[test]
fn calendar_system_helpers_cover_remaining_mappings() {
    for (tag, expected) in [
        ("gregory", CalendarSystem::Gregorian),
        ("gregorian", CalendarSystem::Gregorian),
        ("buddhist", CalendarSystem::Buddhist),
        ("hebrew", CalendarSystem::Hebrew),
        ("islamic", CalendarSystem::Islamic),
    ] {
        assert_eq!(CalendarSystem::from_bcp47(tag), Some(expected));
    }

    assert_eq!(
        CalendarSystem::from_bcp47("japanext"),
        Some(CalendarSystem::Japanese)
    );
    assert_eq!(
        CalendarSystem::from_bcp47("islamic-umalqura"),
        Some(CalendarSystem::IslamicUmmAlQura)
    );
    assert_eq!(
        CalendarSystem::from_bcp47("ethioaa"),
        Some(CalendarSystem::EthiopicAmeteAlem)
    );
    assert_eq!(CalendarSystem::from_bcp47("unknown"), None);
    assert!(CalendarSystem::Japanese.has_custom_eras());
    assert!(CalendarSystem::Roc.has_custom_eras());
    assert!(!CalendarSystem::Gregorian.has_custom_eras());
    assert!(!CalendarSystem::Chinese.has_custom_eras());
    assert_eq!(CalendarSystem::supported_calendars().len(), 15);
}

#[test]
fn japanese_eras_localize_for_japanese_and_non_japanese_locales() {
    let reiwa = JapaneseEra {
        name: "Reiwa",
        start_year: 2019,
    };

    let custom = JapaneseEra {
        name: "Custom",
        start_year: 2030,
    };

    let ja = Locale::parse("ja-JP").expect("locale should parse");

    let en = Locale::parse("en-US").expect("locale should parse");

    assert_eq!(reiwa.localized_name(&ja), "令和");
    assert_eq!(reiwa.localized_name(&en), "Reiwa");
    assert_eq!(custom.localized_name(&ja), "Custom");
    assert_eq!(reiwa.romanized_name(), "Reiwa");
    assert_eq!(CalendarSystem::japanese_eras().last().copied(), Some(reiwa));
}

#[test]
fn week_info_for_locale_uses_region_defaults_and_fw_override() {
    let en_us = Locale::parse("en-US").expect("locale should parse");

    let de_de = Locale::parse("de-DE").expect("locale should parse");

    let ar_sa = Locale::parse("ar-SA").expect("locale should parse");

    let override_locale =
        Locale::parse("en-US-u-fw-mon").expect("locale should parse with fw extension");

    assert_eq!(
        WeekInfo::for_locale(&en_us),
        WeekInfo {
            first_day: Weekday::Sunday,
            min_days_in_first_week: 1,
        }
    );
    assert_eq!(
        WeekInfo::for_locale(&de_de),
        WeekInfo {
            first_day: Weekday::Monday,
            min_days_in_first_week: 4,
        }
    );
    assert_eq!(
        WeekInfo::for_locale(&ar_sa),
        WeekInfo {
            first_day: Weekday::Saturday,
            min_days_in_first_week: 1,
        }
    );
    assert_eq!(
        WeekInfo::for_locale(&override_locale),
        WeekInfo {
            first_day: Weekday::Monday,
            min_days_in_first_week: 1,
        }
    );
}

#[test]
fn ordered_weekdays_rotates_from_first_day() {
    let sunday = WeekInfo {
        first_day: Weekday::Sunday,
        min_days_in_first_week: 1,
    };

    let saturday = WeekInfo {
        first_day: Weekday::Saturday,
        min_days_in_first_week: 1,
    };

    assert_eq!(
        sunday.ordered_weekdays(),
        [
            Weekday::Sunday,
            Weekday::Monday,
            Weekday::Tuesday,
            Weekday::Wednesday,
            Weekday::Thursday,
            Weekday::Friday,
            Weekday::Saturday,
        ]
    );
    assert_eq!(
        saturday.ordered_weekdays(),
        [
            Weekday::Saturday,
            Weekday::Sunday,
            Weekday::Monday,
            Weekday::Tuesday,
            Weekday::Wednesday,
            Weekday::Thursday,
            Weekday::Friday,
        ]
    );
}

#[test]
fn public_calendar_date_replaces_placeholder_shape() {
    let date = CalendarDate::new_gregorian(
        2024,
        NonZero::new(3).expect("month is non-zero"),
        NonZero::new(15).expect("day is non-zero"),
    );

    assert_eq!(date.calendar, CalendarSystem::Gregorian);
    assert_eq!(date.era, None);
    assert_eq!(date.to_iso8601(), "2024-03-15");
}

#[test]
fn typed_calendar_date_wraps_matching_dynamic_date() {
    let raw = CalendarDate::new_gregorian(
        2024,
        NonZero::new(3).expect("month is non-zero"),
        NonZero::new(15).expect("day is non-zero"),
    );

    let typed = raw
        .typed::<Gregorian>()
        .expect("calendar marker should match");

    assert_eq!(typed.year(), 2024);
    assert_eq!(typed.month().get(), 3);
    assert_eq!(typed.day().get(), 15);
    assert_eq!(typed.as_raw().calendar, CalendarSystem::Gregorian);
    assert_eq!(
        TypedCalendarDate::<Gregorian>::calendar_system(),
        CalendarSystem::Gregorian
    );
}

#[test]
fn typed_calendar_date_rejects_mismatched_calendar_marker() {
    let raw = CalendarDate {
        calendar: CalendarSystem::Japanese,
        era: Some(Era {
            code: String::from("reiwa"),
            display_name: String::from("Reiwa"),
        }),
        year: 6,
        month: NonZero::new(3).expect("month is non-zero"),
        day: NonZero::new(15).expect("day is non-zero"),
    };

    let error = raw
        .into_typed::<Gregorian>()
        .expect_err("marker mismatch should fail");

    assert_eq!(
        error,
        CalendarTypeError {
            expected: CalendarSystem::Gregorian,
            found: CalendarSystem::Japanese,
        }
    );
}

#[test]
fn typed_gregorian_date_exposes_compile_time_gated_methods() {
    let date = TypedCalendarDate::<Gregorian>::new(
        2024,
        NonZero::new(3).expect("month is non-zero"),
        NonZero::new(15).expect("day is non-zero"),
    );

    let shifted = date.add_days(10);

    assert_eq!(date.weekday(), Weekday::Friday);
    assert_eq!(shifted.into_raw().to_iso8601(), "2024-03-25");
}

#[test]
fn typed_calendar_date_converts_between_static_calendar_markers() {
    #[derive(Debug)]
    struct TestProvider;

    impl IcuProvider for TestProvider {
        fn convert_date(&self, date: &CalendarDate, target: CalendarSystem) -> CalendarDate {
            match (date.calendar, target) {
                (CalendarSystem::Japanese, CalendarSystem::Gregorian) => {
                    CalendarDate::new_gregorian(
                        2024,
                        NonZero::new(3).expect("month should be non-zero"),
                        NonZero::new(15).expect("day should be non-zero"),
                    )
                }
                _ => date.clone(),
            }
        }
    }

    let provider = TestProvider;

    let typed = TypedCalendarDate::<Japanese>::from_raw(CalendarDate {
        calendar: CalendarSystem::Japanese,
        era: Some(Era {
            code: String::from("reiwa"),
            display_name: String::from("Reiwa"),
        }),
        year: 6,
        month: NonZero::new(3).expect("month is non-zero"),
        day: NonZero::new(15).expect("day is non-zero"),
    })
    .expect("calendar marker should match");

    let gregorian = typed.to_calendar::<Gregorian>(&provider);

    assert_eq!(gregorian.as_raw().calendar, CalendarSystem::Gregorian);
    assert_eq!(gregorian.into_raw().to_iso8601(), "2024-03-15");
}

#[test]
fn typed_calendar_date_forwarders_and_conversions_cover_wrapper_paths() {
    let provider = StubIcuProvider;

    let raw = CalendarDate::new_gregorian(
        2024,
        NonZero::new(1).expect("month should be non-zero"),
        NonZero::new(31).expect("day should be non-zero"),
    );

    let typed = raw
        .clone()
        .typed::<Gregorian>()
        .expect("calendar marker should match");

    let shifted_month = typed
        .add_months(&provider, 1)
        .expect("Gregorian month arithmetic should succeed");

    let shifted_day = typed.add_days_with_provider(&provider, 1);

    let converted = raw.to_calendar_type::<Gregorian>(&provider);

    let via_from = CalendarDate::from(typed.clone());

    assert_eq!(typed.days_in_month(&provider), 31);
    assert_eq!(shifted_month.as_raw().to_iso8601(), "2024-02-29");
    assert_eq!(shifted_day.as_raw().to_iso8601(), "2024-02-01");
    assert_eq!(
        typed.compare_within_calendar(&converted),
        Some(Ordering::Equal)
    );
    assert_eq!(typed.as_ref().to_iso8601(), "2024-01-31");
    assert_eq!(via_from.to_iso8601(), "2024-01-31");
}

#[test]
fn typed_calendar_date_helpers_expose_era_and_mismatch_display() {
    let japanese = TypedCalendarDate::<Japanese>::from_raw(CalendarDate {
        calendar: CalendarSystem::Japanese,
        era: Some(Era {
            code: String::from("reiwa"),
            display_name: String::from("Reiwa"),
        }),
        year: 6,
        month: NonZero::new(3).expect("month is non-zero"),
        day: NonZero::new(15).expect("day is non-zero"),
    })
    .expect("calendar marker should match");

    assert_eq!(japanese.era().map(|era| era.code.as_str()), Some("reiwa"));
    assert_eq!(
        CalendarTypeError {
            expected: CalendarSystem::Gregorian,
            found: CalendarSystem::Japanese,
        }
        .to_string(),
        "calendar type mismatch: expected Gregorian, found Japanese"
    );
}

#[test]
fn public_calendar_date_compare_within_calendar_respects_era_and_calendar() {
    let base = CalendarDate::new_gregorian(
        2024,
        NonZero::new(3).expect("month is non-zero"),
        NonZero::new(15).expect("day is non-zero"),
    );

    let later = CalendarDate::new_gregorian(
        2024,
        NonZero::new(3).expect("month is non-zero"),
        NonZero::new(16).expect("day is non-zero"),
    );

    let japanese = CalendarDate {
        calendar: CalendarSystem::Japanese,
        era: Some(Era {
            code: String::from("reiwa"),
            display_name: String::from("Reiwa"),
        }),
        year: 6,
        month: NonZero::new(3).expect("month is non-zero"),
        day: NonZero::new(15).expect("day is non-zero"),
    };

    assert_eq!(base.compare_within_calendar(&later), Some(Ordering::Less));
    assert_eq!(base.compare_within_calendar(&japanese), None);

    let heisei = CalendarDate {
        calendar: CalendarSystem::Japanese,
        era: Some(Era {
            code: String::from("heisei"),
            display_name: String::from("Heisei"),
        }),
        year: 31,
        month: NonZero::new(4).expect("month is non-zero"),
        day: NonZero::new(30).expect("day is non-zero"),
    };

    assert_eq!(japanese.compare_within_calendar(&heisei), None);
    assert_eq!(
        japanese.compare_within_calendar(&japanese.clone()),
        Some(Ordering::Equal)
    );
}

#[test]
fn date_range_and_time_iso_helpers_are_non_placeholder() {
    let start = CalendarDate::new_gregorian(
        2024,
        NonZero::new(3).expect("month is non-zero"),
        NonZero::new(15).expect("day is non-zero"),
    );

    let end = CalendarDate::new_gregorian(
        2024,
        NonZero::new(3).expect("month is non-zero"),
        NonZero::new(20).expect("day is non-zero"),
    );

    let range = DateRange::new(start.clone(), end.clone()).expect("range should be ordered");

    let time = Time::new(14, 30, 0);

    let precise = Time {
        hour: 14,
        minute: 30,
        second: 0,
        millisecond: 125,
    };

    assert!(range.contains(&start));
    assert!(!range.contains(&CalendarDate::new_gregorian(
        2024,
        NonZero::new(3).expect("month is non-zero"),
        NonZero::new(21).expect("day is non-zero"),
    )));
    assert!(!japanese_range_example().is_between(&start, &end));
    assert_eq!(
        DateRange::normalized(end.clone(), start.clone()),
        Some(range.clone())
    );
    assert_eq!(
        DateRange::normalized(end.clone(), end.clone()),
        Some(DateRange {
            start: end.clone(),
            end: end.clone(),
        })
    );
    assert_eq!(range.to_iso8601(), "2024-03-15/2024-03-20");
    assert_eq!(time.to_iso8601(), "14:30:00");
    assert_eq!(precise.to_iso8601(), "14:30:00.125");
    assert_eq!(time.hour_12(), 2);
    assert!(time.is_pm());
    assert_eq!(HourCycle::H11.display_hour_range(), (0, 11));
    assert_eq!(HourCycle::H12.display_hour_range(), (1, 12));
    assert!(HourCycle::H12.has_day_period());
    assert!(!HourCycle::H23.has_day_period());
    assert_eq!(HourCycle::H24.display_hour_range(), (1, 24));
    assert_eq!(DateRange::new(end.clone(), start.clone()), None);
    assert_eq!(DateRange::normalized(start, japanese_range_example()), None);
}

#[test]
fn public_calendar_date_validated_constructor_uses_provider_rules() {
    let provider = StubIcuProvider;

    let gregorian = CalendarDate::new(&provider, CalendarSystem::Gregorian, None, 2024, 2, 29);

    let invalid = CalendarDate::new(&provider, CalendarSystem::Gregorian, None, 2024, 2, 30);

    let invalid_year = CalendarDate::new(&provider, CalendarSystem::Gregorian, None, 0, 2, 29);

    let hebrew_leap = CalendarDate::new(&provider, CalendarSystem::Hebrew, None, 5784, 13, 1);

    let hebrew_common = CalendarDate::new(&provider, CalendarSystem::Hebrew, None, 5785, 13, 1);

    assert!(gregorian.is_some());
    assert!(invalid.is_none());
    assert!(invalid_year.is_none());
    assert!(hebrew_leap.is_some());
    assert!(hebrew_common.is_none());
}

#[test]
fn public_calendar_date_new_defaults_japanese_to_current_era() {
    let provider = StubIcuProvider;

    let date = CalendarDate::new(&provider, CalendarSystem::Japanese, None, 6, 3, 15)
        .expect("missing Japanese era should default to the current era");

    assert_eq!(date.calendar, CalendarSystem::Japanese);
    assert_eq!(
        date.era,
        Some(Era {
            code: String::from("reiwa"),
            display_name: String::from("Reiwa"),
        })
    );
    assert_eq!(date.year, 6);
    assert_eq!(date.month.get(), 3);
    assert_eq!(date.day.get(), 15);
}

#[test]
fn public_calendar_date_new_rejects_japanese_dates_outside_era_bounds() {
    let provider = StubIcuProvider;

    let heisei = Some(Era {
        code: String::from("heisei"),
        display_name: String::from("Heisei"),
    });

    let reiwa = Some(Era {
        code: String::from("reiwa"),
        display_name: String::from("Reiwa"),
    });

    assert!(
        CalendarDate::new(&provider, CalendarSystem::Japanese, heisei.clone(), 1, 1, 7,).is_none()
    );
    assert!(
        CalendarDate::new(&provider, CalendarSystem::Japanese, reiwa.clone(), 1, 4, 30,).is_none()
    );
    assert!(CalendarDate::new(&provider, CalendarSystem::Japanese, heisei, 31, 5, 1,).is_none());
    assert!(CalendarDate::new(&provider, CalendarSystem::Japanese, reiwa, 1, 5, 1).is_some());
}

#[test]
fn public_calendar_date_arithmetic_helpers_match_shared_spec_shape() {
    let provider = StubIcuProvider;

    let date = CalendarDate::new_gregorian(
        2024,
        NonZero::new(1).expect("month should be non-zero"),
        NonZero::new(31).expect("day should be non-zero"),
    );

    assert_eq!(date.days_in_month(&provider), 31);
    assert_eq!(
        date.add_months(&provider, 1),
        Some(CalendarDate::new_gregorian(
            2024,
            NonZero::new(2).expect("month should be non-zero"),
            NonZero::new(29).expect("day should be non-zero"),
        ))
    );
    assert_eq!(
        date.add_days(1),
        Some(CalendarDate::new_gregorian(
            2024,
            NonZero::new(2).expect("month should be non-zero"),
            NonZero::new(1).expect("day should be non-zero"),
        ))
    );
    assert_eq!(
        date.add_days_with_provider(&provider, 1),
        CalendarDate::new_gregorian(
            2024,
            NonZero::new(2).expect("month should be non-zero"),
            NonZero::new(1).expect("day should be non-zero"),
        )
    );
    assert_eq!(date.to_calendar(&provider, CalendarSystem::Gregorian), date);
}

#[test]
fn public_calendar_date_add_months_respects_era_boundaries() {
    let provider = StubIcuProvider;

    let heisei = Era {
        code: String::from("heisei"),
        display_name: String::from("Heisei"),
    };

    let same_era_rollover = CalendarDate::new(
        &provider,
        CalendarSystem::Japanese,
        Some(heisei.clone()),
        1,
        12,
        15,
    )
    .expect("Heisei year 1 December should be valid");

    let cross_era_rollover = CalendarDate::new(
        &provider,
        CalendarSystem::Japanese,
        Some(heisei.clone()),
        31,
        4,
        15,
    )
    .expect("Heisei year 31 April should be valid");

    let negative_rollover = CalendarDate::new(
        &provider,
        CalendarSystem::Japanese,
        Some(heisei.clone()),
        2,
        1,
        15,
    )
    .expect("Heisei year 2 January should be valid");

    let boundary_clamp = CalendarDate::new(
        &provider,
        CalendarSystem::Japanese,
        Some(heisei.clone()),
        2,
        1,
        1,
    )
    .expect("Heisei year 2 January should be valid");

    assert_eq!(
        same_era_rollover.add_months(&provider, 1),
        Some(CalendarDate {
            calendar: CalendarSystem::Japanese,
            era: Some(heisei.clone()),
            year: 2,
            month: NonZero::new(1).expect("month should be non-zero"),
            day: NonZero::new(15).expect("day should be non-zero"),
        })
    );
    assert_eq!(
        negative_rollover.add_months(&provider, -1),
        Some(CalendarDate {
            calendar: CalendarSystem::Japanese,
            era: Some(heisei.clone()),
            year: 1,
            month: NonZero::new(12).expect("month should be non-zero"),
            day: NonZero::new(15).expect("day should be non-zero"),
        })
    );
    assert_eq!(
        boundary_clamp.add_months(&provider, -12),
        Some(CalendarDate {
            calendar: CalendarSystem::Japanese,
            era: Some(heisei),
            year: 1,
            month: NonZero::new(1).expect("month should be non-zero"),
            day: NonZero::new(8).expect("day should be non-zero"),
        })
    );
    assert_eq!(cross_era_rollover.add_months(&provider, 1), None);
}

#[test]
fn public_calendar_date_non_gregorian_provider_paths_delegate() {
    #[derive(Debug)]
    struct TestProvider;

    impl IcuProvider for TestProvider {
        fn convert_date(&self, date: &CalendarDate, target: CalendarSystem) -> CalendarDate {
            match (date.calendar, target) {
                (CalendarSystem::Japanese, CalendarSystem::Gregorian) => {
                    CalendarDate::new_gregorian(
                        2024,
                        NonZero::new(3).expect("month should be non-zero"),
                        NonZero::new(15).expect("day should be non-zero"),
                    )
                }

                (CalendarSystem::Gregorian, CalendarSystem::Japanese) => CalendarDate {
                    calendar: CalendarSystem::Japanese,
                    era: Some(Era {
                        code: String::from("reiwa"),
                        display_name: String::from("Reiwa"),
                    }),
                    year: 6,
                    month: NonZero::new(3).expect("month should be non-zero"),
                    day: date.day,
                },

                _ => date.clone(),
            }
        }
    }

    let provider = TestProvider;

    let japanese = CalendarDate {
        calendar: CalendarSystem::Japanese,
        era: Some(Era {
            code: String::from("reiwa"),
            display_name: String::from("Reiwa"),
        }),
        year: 6,
        month: NonZero::new(3).expect("month should be non-zero"),
        day: NonZero::new(15).expect("day should be non-zero"),
    };

    assert_eq!(
        japanese.to_calendar(&provider, CalendarSystem::Gregorian),
        CalendarDate::new_gregorian(
            2024,
            NonZero::new(3).expect("month should be non-zero"),
            NonZero::new(15).expect("day should be non-zero"),
        )
    );
    assert_eq!(japanese.add_days(1), None);
    assert_eq!(
        japanese.add_days_with_provider(&provider, 1),
        CalendarDate {
            calendar: CalendarSystem::Japanese,
            era: Some(Era {
                code: String::from("reiwa"),
                display_name: String::from("Reiwa"),
            }),
            year: 6,
            month: NonZero::new(3).expect("month should be non-zero"),
            day: NonZero::new(16).expect("day should be non-zero"),
        }
    );
}

#[cfg(feature = "icu4x")]
#[test]
fn public_calendar_date_to_internal_preserves_explicit_japanese_era() {
    let heisei = CalendarDate {
        calendar: CalendarSystem::Japanese,
        era: Some(Era {
            code: String::from("heisei"),
            display_name: String::from("Heisei"),
        }),
        year: 31,
        month: NonZero::new(4).expect("month should be non-zero"),
        day: NonZero::new(30).expect("day should be non-zero"),
    };

    let internal = InternalCalendarDate::try_from(&heisei)
        .expect("era-aware public conversion should succeed");

    let gregorian = internal.to_calendar(CalendarSystem::Gregorian);

    assert_eq!(internal.era(), Some(String::from("heisei")));
    assert_eq!(gregorian.year(), 2019);
    assert_eq!(gregorian.month(), 4);
    assert_eq!(gregorian.day(), 30);
}

#[test]
fn public_calendar_date_weekday_handles_january_and_february() {
    let january = CalendarDate::new_gregorian(
        2024,
        NonZero::new(1).expect("month should be non-zero"),
        NonZero::new(1).expect("day should be non-zero"),
    );

    let february = CalendarDate::new_gregorian(
        2024,
        NonZero::new(2).expect("month should be non-zero"),
        NonZero::new(29).expect("day should be non-zero"),
    );

    assert_eq!(january.weekday(), Weekday::Monday);
    assert_eq!(february.weekday(), Weekday::Thursday);
}

#[test]
fn month_type_validates_shared_range() {
    assert_eq!(Month::new(1).map(Month::get), Some(1));
    assert_eq!(Month::new(13).map(Month::get), Some(13));
    assert_eq!(Month::new(0), None);
    assert_eq!(Month::new(14), None);
    assert_eq!(Month::try_from(7).ok().map(Month::get), Some(7));
    assert!(Month::try_from(0).is_err());
}

#[test]
fn hour_cycle_auto_resolves_via_locale_provider() {
    let provider = StubIcuProvider;

    let en_us = Locale::parse("en-US").expect("locale should parse");

    let de_de = Locale::parse("de-DE").expect("locale should parse");

    assert_eq!(HourCycle::Auto.resolve(&provider, &en_us), HourCycle::H12);
    assert_eq!(HourCycle::Auto.resolve(&provider, &de_de), HourCycle::H23);
    assert_eq!(en_us.hour_cycle(&provider), HourCycle::H12);
}

#[test]
fn epoch_day_helpers_use_the_spec_epoch() {
    assert_eq!(iso_to_epoch_days(1, 1, 1), 0);
    assert_eq!(epoch_days_to_iso(0), (1, 1, 1));
    assert_eq!(iso_to_epoch_days(1970, 1, 1), 719_162);
    assert_eq!(epoch_days_to_iso(719_162), (1970, 1, 1));
    assert_eq!(gregorian_days_in_month(2024, 2), 29);
    assert_eq!(gregorian_days_in_month(2023, 2), 28);
    assert_eq!(gregorian_days_in_month(2024, 0), 30);
    assert_eq!(gregorian_days_in_month(2024, 13), 30);
}

#[test]
fn date_and_calendar_errors_format_as_specified() {
    assert_eq!(DateError::InvalidDate.to_string(), "invalid date");
    assert_eq!(
        DateError::CalendarError(String::from("bad month")).to_string(),
        "calendar error: bad month"
    );
    assert_eq!(
        CalendarError::Arithmetic(String::from("overflow")).to_string(),
        "calendar arithmetic error: overflow"
    );
    assert_eq!(
        CalendarConversionError::InvalidDate.to_string(),
        "invalid date for target calendar"
    );
    assert_eq!(
        CalendarConversionError::Icu(String::from("failed")).to_string(),
        "ICU4X calendar conversion error: failed"
    );
}

#[test]
fn japanese_era_helper_functions_cover_bounded_and_fallback_paths() {
    let reiwa = CalendarDate {
        calendar: CalendarSystem::Japanese,
        era: Some(Era {
            code: String::from("reiwa"),
            display_name: String::from("Reiwa"),
        }),
        year: 1,
        month: NonZero::new(5).expect("month should be non-zero"),
        day: NonZero::new(1).expect("day should be non-zero"),
    };

    let heisei = CalendarDate {
        calendar: CalendarSystem::Japanese,
        era: Some(Era {
            code: String::from("heisei"),
            display_name: String::from("Heisei"),
        }),
        year: 31,
        month: NonZero::new(4).expect("month should be non-zero"),
        day: NonZero::new(30).expect("day should be non-zero"),
    };

    let unknown_era = CalendarDate {
        calendar: CalendarSystem::Japanese,
        era: Some(Era {
            code: String::from("unknown"),
            display_name: String::from("Unknown"),
        }),
        year: 1,
        month: NonZero::new(1).expect("month should be non-zero"),
        day: NonZero::new(1).expect("day should be non-zero"),
    };

    assert_eq!(
        default_era_for(CalendarSystem::Japanese).map(|era| era.code),
        Some(String::from("reiwa"))
    );
    assert_eq!(default_era_for(CalendarSystem::Gregorian), None);
    assert_eq!(years_in_era(&heisei), Some(31));
    assert_eq!(years_in_era(&unknown_era), None);
    assert_eq!(minimum_month_in_year(&reiwa), 5);
    assert_eq!(minimum_day_in_month(&reiwa), 1);
    assert_eq!(minimum_month_in_year(&unknown_era), 1);
    assert_eq!(minimum_day_in_month(&unknown_era), 1);
    assert_eq!(reiwa.days_in_month(&StubIcuProvider), 31);
    assert_eq!(
        bounded_months_in_year(CalendarSystem::Japanese, 31, Some("heisei")),
        Some(4)
    );
    assert_eq!(
        bounded_months_in_year(CalendarSystem::Japanese, 1, None),
        None
    );
    assert_eq!(
        bounded_days_in_month(CalendarSystem::Japanese, 31, 4, Some("heisei")),
        Some(30)
    );
    assert_eq!(
        bounded_days_in_month(CalendarSystem::Gregorian, 2024, 2, None),
        None
    );
}

#[cfg(feature = "icu4x")]
#[test]
fn internal_calendar_date_invalid_construction_paths_return_errors() {
    assert_eq!(
        InternalCalendarDate::from_iso(2024, 2, 30),
        Err(DateError::InvalidDate)
    );
    assert!(
        InternalCalendarDate::from_calendar_with_era("bogus", 1, 1, 1, CalendarSystem::Japanese)
            .is_err()
    );
    assert!(
        InternalCalendarDate::try_from(&CalendarDate {
            calendar: CalendarSystem::Japanese,
            era: Some(Era {
                code: String::from("bogus"),
                display_name: String::from("Bogus"),
            }),
            year: 1,
            month: NonZero::new(1).expect("month should be non-zero"),
            day: NonZero::new(1).expect("day should be non-zero"),
        })
        .is_err()
    );
}

#[cfg(all(feature = "icu4x", feature = "std"))]
#[test]
fn internal_calendar_date_additional_paths_cover_identity_and_ordering() {
    let start = InternalCalendarDate::from_iso(2024, 3, 15).expect("date should be valid");

    let same = start.to_calendar(CalendarSystem::Gregorian);

    let previous = start.add_days(-3).expect("date arithmetic should succeed");

    let japanese_today =
        InternalCalendarDate::today(CalendarSystem::Japanese).expect("today should resolve");

    assert_eq!(same.year(), 2024);
    assert_eq!(start.days_until(&previous), Ok(-3));
    assert_eq!(start.is_before(&previous), Ok(false));
    assert!(japanese_today.era().is_some());
}

#[test]
fn weekday_helpers_cover_remaining_variants() {
    assert_eq!(Weekday::from_sunday_zero(0), Weekday::Sunday);
    assert_eq!(Weekday::from_sunday_zero(8), Weekday::Monday);

    assert_eq!(Weekday::from_iso_8601(1), Some(Weekday::Monday));
    assert_eq!(Weekday::from_iso_8601(2), Some(Weekday::Tuesday));
    assert_eq!(Weekday::from_iso_8601(3), Some(Weekday::Wednesday));
    assert_eq!(Weekday::from_iso_8601(4), Some(Weekday::Thursday));
    assert_eq!(Weekday::from_iso_8601(5), Some(Weekday::Friday));
    assert_eq!(Weekday::from_iso_8601(6), Some(Weekday::Saturday));
    assert_eq!(Weekday::from_iso_8601(7), Some(Weekday::Sunday));
    assert_eq!(Weekday::from_iso_8601(0), None);
    assert_eq!(Weekday::from_iso_8601(8), None);

    assert_eq!(Weekday::from_icu_str("mon"), Some(Weekday::Monday));
    assert_eq!(Weekday::from_icu_str("tue"), Some(Weekday::Tuesday));
    assert_eq!(Weekday::from_icu_str("wed"), Some(Weekday::Wednesday));
    assert_eq!(Weekday::from_icu_str("thu"), Some(Weekday::Thursday));
    assert_eq!(Weekday::from_icu_str("fri"), Some(Weekday::Friday));
    assert_eq!(Weekday::from_icu_str("sat"), Some(Weekday::Saturday));
    assert_eq!(Weekday::from_icu_str("sun"), Some(Weekday::Sunday));
    assert_eq!(Weekday::from_icu_str("bad"), None);

    assert_eq!(Weekday::from_bcp47_fw("sun"), Some(Weekday::Sunday));
    assert_eq!(Weekday::from_bcp47_fw("mon"), Some(Weekday::Monday));

    assert_eq!(
        Weekday::from_icu_weekday(icu::calendar::types::Weekday::Monday),
        Weekday::Monday
    );
    assert_eq!(
        Weekday::from_icu_weekday(icu::calendar::types::Weekday::Tuesday),
        Weekday::Tuesday
    );
    assert_eq!(
        Weekday::from_icu_weekday(icu::calendar::types::Weekday::Wednesday),
        Weekday::Wednesday
    );
    assert_eq!(
        Weekday::from_icu_weekday(icu::calendar::types::Weekday::Thursday),
        Weekday::Thursday
    );
    assert_eq!(
        Weekday::from_icu_weekday(icu::calendar::types::Weekday::Friday),
        Weekday::Friday
    );
    assert_eq!(
        Weekday::from_icu_weekday(icu::calendar::types::Weekday::Saturday),
        Weekday::Saturday
    );
    assert_eq!(
        Weekday::from_icu_weekday(icu::calendar::types::Weekday::Sunday),
        Weekday::Sunday
    );
}

fn japanese_range_example() -> CalendarDate {
    CalendarDate {
        calendar: CalendarSystem::Japanese,
        era: Some(Era {
            code: String::from("reiwa"),
            display_name: String::from("Reiwa"),
        }),
        year: 6,
        month: NonZero::new(3).expect("month should be non-zero"),
        day: NonZero::new(15).expect("day should be non-zero"),
    }
}
