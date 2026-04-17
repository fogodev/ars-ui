//! [`WebIntlProvider`] — browser-backed provider for WASM client builds.
//!
//! Implements the [`IcuProvider`](crate::IcuProvider) contract using the
//! browser's `Intl.*` APIs. See spec §9.5.4. Available only on `wasm32`
//! targets with the `web-intl` feature.

use alloc::string::{String, ToString};
use core::num::NonZero;

use js_sys::{Array, Function, Intl, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue};

use crate::{
    CalendarDate, CalendarSystem, Era, HourCycle, IcuProvider, Locale, Weekday,
    calendar::{
        bounded_days_in_month, bounded_months_in_year, default_era_for, gregorian_days_in_month,
        minimum_day_in_month, minimum_month_in_year, years_in_era,
    },
};

/// Browser-backed provider for WASM client builds with the `web-intl`
/// feature.
///
/// Delegates locale-sensitive operations to the browser's built-in
/// `Intl.DateTimeFormat`, `Intl.NumberFormat`, and `Intl.Locale` APIs.
/// Calendar arithmetic and era-boundary queries are served by shared
/// Rust-side helpers because browsers do not expose those directly.
#[derive(Clone, Copy, Debug, Default)]
pub struct WebIntlProvider;

impl WebIntlProvider {
    /// Formats a reference date through `Intl.DateTimeFormat` using the
    /// given locale and options bag, returning the formatted string.
    fn format_date_part(locale: &Locale, date: &js_sys::Date, opts: &Object) -> String {
        let locales = Array::of1(&JsValue::from_str(&locale.to_bcp47()));
        let formatter = Intl::DateTimeFormat::new(&locales, opts);
        let format_fn: Function = formatter.format();
        format_fn
            .call1(&JsValue::UNDEFINED, date.as_ref())
            .ok()
            .and_then(|value| value.as_string())
            .unwrap_or_default()
    }

    /// Builds a JS `Date` whose weekday matches `weekday` in January 2024.
    ///
    /// January 1, 2024 is a Monday, so Monday..=Sunday map to day-of-month
    /// 1..=7. Mirrors the reference dates used by [`Icu4xProvider`].
    fn date_for_weekday(weekday: Weekday) -> js_sys::Date {
        let day = match weekday {
            Weekday::Monday => 1,
            Weekday::Tuesday => 2,
            Weekday::Wednesday => 3,
            Weekday::Thursday => 4,
            Weekday::Friday => 5,
            Weekday::Saturday => 6,
            Weekday::Sunday => 7,
        };

        js_sys::Date::new_with_year_month_day(2024, 0, day)
    }

    /// Builds a JS `Date` on the 15th of the given 1-based month in 2024.
    fn date_for_month(month: u8) -> js_sys::Date {
        js_sys::Date::new_with_year_month_day(2024, i32::from(month.saturating_sub(1)), 15)
    }

    /// Returns the integer-valued option bag entry for the given key, or
    /// `None` if the key is missing or not a string.
    fn string_property(object: &JsValue, key: &str) -> Option<String> {
        Reflect::get(object, &JsValue::from_str(key))
            .ok()
            .and_then(|value| value.as_string())
    }
}

impl IcuProvider for WebIntlProvider {
    fn weekday_short_label(&self, weekday: Weekday, locale: &Locale) -> String {
        let opts = Object::new();

        Reflect::set(&opts, &"weekday".into(), &JsValue::from_str("short"))
            .expect("Reflect::set on a fresh Object never fails");

        Self::format_date_part(locale, &Self::date_for_weekday(weekday), &opts)
    }

    fn weekday_long_label(&self, weekday: Weekday, locale: &Locale) -> String {
        let opts = Object::new();

        Reflect::set(&opts, &"weekday".into(), &JsValue::from_str("long"))
            .expect("Reflect::set on a fresh Object never fails");

        Self::format_date_part(locale, &Self::date_for_weekday(weekday), &opts)
    }

    fn month_long_name(&self, month: u8, locale: &Locale) -> String {
        if !(1..=12).contains(&month) {
            return String::from("Unknown");
        }

        let opts = Object::new();

        Reflect::set(&opts, &"month".into(), &JsValue::from_str("long"))
            .expect("Reflect::set on a fresh Object never fails");

        Self::format_date_part(locale, &Self::date_for_month(month), &opts)
    }

    fn day_period_label(&self, is_pm: bool, locale: &Locale) -> String {
        let locales = Array::of1(&JsValue::from_str(&locale.to_bcp47()));

        let opts = Object::new();

        Reflect::set(&opts, &"hour".into(), &JsValue::from_str("numeric"))
            .expect("Reflect::set on a fresh Object never fails");

        Reflect::set(&opts, &"hour12".into(), &JsValue::TRUE)
            .expect("Reflect::set on a fresh Object never fails");

        let formatter = Intl::DateTimeFormat::new(&locales, &opts);

        let hour = if is_pm { 18 } else { 6 };

        let date = js_sys::Date::new_with_year_month_day_hr_min(2024, 0, 1, hour, 0);

        let parts = formatter.format_to_parts(&date);

        for i in 0..parts.length() {
            let part = parts.get(i);

            if Self::string_property(&part, "type").as_deref() == Some("dayPeriod") {
                if let Some(value) = Self::string_property(&part, "value") {
                    return value;
                }
            }
        }

        if is_pm {
            String::from("PM")
        } else {
            String::from("AM")
        }
    }

    fn day_period_from_char(&self, ch: char, locale: &Locale) -> Option<bool> {
        let am_label = self.day_period_label(false, locale);

        let pm_label = self.day_period_label(true, locale);

        let ch_lower = ch.to_lowercase().next().unwrap_or(ch);

        let am_first = am_label
            .chars()
            .next()
            .and_then(|c| c.to_lowercase().next());

        let pm_first = pm_label
            .chars()
            .next()
            .and_then(|c| c.to_lowercase().next());

        if am_first == Some(ch_lower) {
            Some(false)
        } else if pm_first == Some(ch_lower) {
            Some(true)
        } else {
            None
        }
    }

    fn format_segment_digits(
        &self,
        value: u32,
        min_digits: NonZero<u8>,
        locale: &Locale,
    ) -> String {
        let locales = Array::of1(&JsValue::from_str(&locale.to_bcp47()));

        let opts = Object::new();

        Reflect::set(
            &opts,
            &"minimumIntegerDigits".into(),
            &JsValue::from_f64(f64::from(min_digits.get())),
        )
        .expect("Reflect::set on a fresh Object never fails");

        Reflect::set(&opts, &"useGrouping".into(), &JsValue::FALSE)
            .expect("Reflect::set on a fresh Object never fails");

        let formatter = Intl::NumberFormat::new(&locales, &opts);

        let format_fn: Function = formatter.format();

        format_fn
            .call1(&JsValue::UNDEFINED, &JsValue::from_f64(f64::from(value)))
            .ok()
            .and_then(|value| value.as_string())
            .unwrap_or_default()
    }

    fn max_months_in_year(&self, calendar: &CalendarSystem, year: i32, era: Option<&str>) -> u8 {
        if let Some(months) = bounded_months_in_year(*calendar, year, era) {
            return months;
        }

        match calendar {
            CalendarSystem::Hebrew => {
                let cycle_year = year.rem_euclid(19);

                if [0, 3, 6, 8, 11, 14, 17].contains(&cycle_year) {
                    13
                } else {
                    12
                }
            }

            CalendarSystem::Ethiopic
            | CalendarSystem::EthiopicAmeteAlem
            | CalendarSystem::Coptic => 13,

            _ => 12,
        }
    }

    fn days_in_month(
        &self,
        calendar: &CalendarSystem,
        year: i32,
        month: u8,
        era: Option<&str>,
    ) -> u8 {
        if let Some(days) = bounded_days_in_month(*calendar, year, month, era) {
            return days;
        }

        match calendar {
            CalendarSystem::Gregorian => gregorian_days_in_month(year, month),

            _ => {
                // Other calendars would require probing `Intl.DateTimeFormat`
                // with the target calendar option. Browsers expose this but
                // the probing loop is non-trivial and is tracked in #545 as
                // part of the public era-aware calendar operations API.
                // Conservative fallback that matches the spec §9.5.4 sketch.
                30
            }
        }
    }

    fn default_era(&self, calendar: &CalendarSystem) -> Option<Era> {
        default_era_for(*calendar)
    }

    fn years_in_era(&self, date: &CalendarDate) -> Option<i32> {
        years_in_era(date)
    }

    fn minimum_month_in_year(&self, date: &CalendarDate) -> u8 {
        minimum_month_in_year(date)
    }

    fn minimum_day_in_month(&self, date: &CalendarDate) -> u8 {
        minimum_day_in_month(date)
    }

    fn hour_cycle(&self, locale: &Locale) -> HourCycle {
        let locales = Array::of1(&JsValue::from_str(&locale.to_bcp47()));

        // `resolvedOptions().hourCycle` is only populated when `hour` is
        // requested in the options bag — otherwise the browser leaves it
        // undefined. Requesting `hour: "numeric"` forces the engine to
        // materialize the locale's preferred cycle.
        let opts = Object::new();

        Reflect::set(&opts, &"hour".into(), &JsValue::from_str("numeric"))
            .expect("Reflect::set on a fresh Object never fails");

        let formatter = Intl::DateTimeFormat::new(&locales, &opts);

        let resolved = formatter.resolved_options();

        match Self::string_property(&resolved, "hourCycle")
            .as_deref()
            .unwrap_or("")
        {
            "h11" => HourCycle::H11,
            "h12" => HourCycle::H12,
            "h24" => HourCycle::H24,
            // "h23" and unknown both fall back to 24-hour without day-period.
            _ => HourCycle::H23,
        }
    }

    fn first_day_of_week(&self, locale: &Locale) -> Weekday {
        if let Some(weekday) = locale.first_day_of_week_extension() {
            return weekday;
        }

        // Prefer `Intl.Locale#getWeekInfo()` when available. Not every
        // browser supports it yet, so we feature-detect at the JS level.
        if let Ok(js_locale) = Intl::Locale::new(&locale.to_bcp47()) {
            let js_value: JsValue = js_locale.into();

            if let Ok(get_week_info) = Reflect::get(&js_value, &JsValue::from_str("getWeekInfo"))
                && get_week_info.is_function()
                && let Ok(func) = get_week_info.dyn_into::<Function>()
                && let Ok(info) = func.call0(&js_value)
                && let Ok(first_day) = Reflect::get(&info, &JsValue::from_str("firstDay"))
                && let Some(day) = first_day.as_f64()
            {
                return match day as u8 {
                    2 => Weekday::Tuesday,
                    3 => Weekday::Wednesday,
                    4 => Weekday::Thursday,
                    5 => Weekday::Friday,
                    6 => Weekday::Saturday,
                    7 => Weekday::Sunday,

                    // `getWeekInfo().firstDay` is
                    // documented as 1..=7 with Monday=1;
                    // treat 1 and any unexpected value
                    // as Monday (the ISO 8601 default).
                    _ => Weekday::Monday,
                };
            }
        }

        // Fallback to the shared region-based table for older browsers.
        crate::WeekInfo::for_locale(locale).first_day
    }

    fn convert_date(&self, date: &CalendarDate, target: CalendarSystem) -> CalendarDate {
        if date.calendar == target {
            return date.clone();
        }

        // Use the same ICU4X-backed internal bridge as `Icu4xProvider` when
        // the `icu4x` feature is also on. Otherwise fall back to formatting
        // through `Intl.DateTimeFormat({calendar})` and reparsing the parts
        // — browsers ship full CLDR calendar data without WASM payload.
        #[cfg(feature = "icu4x")]
        {
            if let Ok(internal) = crate::calendar::internal::CalendarDate::try_from(date) {
                let converted = internal.to_calendar(target);

                return CalendarDate {
                    calendar: target,
                    era: converted
                        .era()
                        .filter(|_| target.has_custom_eras())
                        .map(|code| Era {
                            code: code.clone(),
                            display_name: code,
                        }),
                    year: converted.year(),
                    month: NonZero::new(converted.month())
                        .expect("internal calendar conversion yields a 1-based month"),
                    day: NonZero::new(converted.day())
                        .expect("internal calendar conversion yields a 1-based day"),
                };
            }
        }

        let locales = Array::of1(&JsValue::from_str("en-US"));

        let opts = Object::new();

        Reflect::set(
            &opts,
            &"calendar".into(),
            &JsValue::from_str(target.to_bcp47_value()),
        )
        .expect("Reflect::set on a fresh Object never fails");

        Reflect::set(&opts, &"year".into(), &JsValue::from_str("numeric"))
            .expect("Reflect::set on a fresh Object never fails");

        Reflect::set(&opts, &"month".into(), &JsValue::from_str("numeric"))
            .expect("Reflect::set on a fresh Object never fails");

        Reflect::set(&opts, &"day".into(), &JsValue::from_str("numeric"))
            .expect("Reflect::set on a fresh Object never fails");

        // Request the era part too so `formatToParts` emits it. Without
        // `era` in the options, browsers drop the era token entirely and
        // we lose the Japanese era boundary information that downstream
        // validation depends on (e.g., Gregorian 1990 → Heisei 2, not
        // Reiwa 2).
        Reflect::set(&opts, &"era".into(), &JsValue::from_str("long"))
            .expect("Reflect::set on a fresh Object never fails");

        let formatter = Intl::DateTimeFormat::new(&locales, &opts);

        // `Date::new_with_year_month_day` takes a positive `u32` year; for
        // pre-CE Gregorian dates we fall back to returning the source date
        // unchanged, matching `StubIcuProvider`'s behavior outside the
        // supported range.
        let Ok(year_u32) = u32::try_from(date.year) else {
            return date.clone();
        };

        let js_date = js_sys::Date::new_with_year_month_day(
            year_u32,
            i32::from(date.month.get().saturating_sub(1)),
            i32::from(date.day.get()),
        );

        let parts = formatter.format_to_parts(&js_date);

        let mut year = 0_i32;
        let mut month: u8 = 0;
        let mut day: u8 = 0;
        let mut era_label: Option<String> = None;

        for i in 0..parts.length() {
            let part = parts.get(i);

            let part_type = Self::string_property(&part, "type").unwrap_or_default();

            let value = Self::string_property(&part, "value").unwrap_or_default();

            match part_type.as_str() {
                "year" | "relatedYear" => year = value.parse().unwrap_or(0),
                "month" => month = value.parse().unwrap_or(0),
                "day" => day = value.parse().unwrap_or(0),
                "era" => era_label = Some(value),
                _ => {}
            }
        }

        let (Some(month_nz), Some(day_nz)) = (NonZero::new(month), NonZero::new(day)) else {
            return date.clone();
        };

        // Map the browser's long era label back to the CLDR era code. The
        // `era: "long"` option with the en-US formatting locale yields
        // "Reiwa", "Heisei", "Showa", … for Japanese, which `to_ascii_lowercase`
        // turns into the canonical codes ("reiwa", "heisei", "showa", …).
        // If the browser suppressed the era (no `era` token in parts) or
        // returned an unfamiliar form, fall back to the target's default era
        // so downstream consumers still see a sensible value.
        let era = if target.has_custom_eras() {
            era_label
                .as_deref()
                .map(|label| Era {
                    code: label.to_ascii_lowercase(),
                    display_name: label.to_string(),
                })
                .or_else(|| default_era_for(target))
        } else {
            None
        };

        CalendarDate {
            calendar: target,
            era,
            year,
            month: month_nz,
            day: day_nz,
        }
    }
}
