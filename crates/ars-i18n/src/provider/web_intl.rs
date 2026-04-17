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

    /// Resolves a named month label emitted by `Intl.DateTimeFormat`
    /// (e.g., Hebrew `"Adar II"`) to its 1-based ordinal in the target
    /// calendar's year numbering.
    ///
    /// Some runtimes (notably Node.js under `--features web-intl` without
    /// `icu4x`) emit calendar-specific names from
    /// `Intl.DateTimeFormat({ calendar, month: "numeric" })` when the
    /// calendar uses non-numeric month labelling — Hebrew leap years
    /// are the canonical case, where civil-order ordinal 6 is rendered
    /// as `Adar I` and ordinal 7 as `Adar II`.
    ///
    /// We resolve the label by sweeping probe Gregorian days through
    /// target-calendar years of both leap-cycle flavours and asking two
    /// formatters for the same instant: `month: "long"` gives us the
    /// label to match against the input, and `month: "2-digit"` gives
    /// us the target calendar's numeric ordinal directly. Returning the
    /// iteration index (as the previous version did) silently corrupted
    /// Hebrew months because the index tracked the Gregorian probe
    /// month, not the target calendar's civil-order ordinal.
    pub(crate) fn resolve_named_month(
        target: CalendarSystem,
        target_year: i32,
        label: &str,
    ) -> Option<u8> {
        let locales = Array::of1(&JsValue::from_str("en-US"));
        let long_formatter = month_label_formatter(&locales, target, "long")?;
        let numeric_formatter = month_label_formatter(&locales, target, "2-digit")?;

        // Hebrew, Chinese, Dangi, Ethiopic, and other 13-month
        // calendars surface different month labels in leap vs. common
        // years — and in Chinese/Dangi the *specific* leap month
        // varies each cycle (leap 4th in 2020, leap 2nd in 2023, leap
        // 6th in 2025, …). Probing only `[2024, 2025]` couldn't match
        // labels like `"Second Monthbis"` (2023) or `"Fourth Monthbis"`
        // (2020), so `convert_date` fell back to `date.clone()` for
        // those dates on runtimes that emit spelled-out leap labels.
        //
        // `SWEEP_YEARS` covers the full 19-year Metonic-style cycle
        // plus recent Chinese/Dangi leap-month positions. We start
        // with a Hebrew-aware fast path (matching round-5 behaviour
        // for the common Hebrew case) and then sweep the wider list
        // with an early exit on first match, so the typical Hebrew
        // call still resolves on the first probe pair.
        const SWEEP_YEARS: [u32; 19] = [
            2023, 2020, 2028, 2021, 2017, 2014, 2031, 2019, 2022, 2026, 2027, 2029, 2030, 2015,
            2016, 2018, 2032, 2033, 2034,
        ];
        let prefer_leap = matches!(
            target,
            CalendarSystem::Hebrew | CalendarSystem::Ethiopic | CalendarSystem::EthiopicAmeteAlem
        ) && is_hebrew_leap_year(target_year);
        let primary_years: [u32; 2] = if prefer_leap {
            [2024, 2025]
        } else {
            [2025, 2024]
        };
        let probe_years = primary_years.iter().chain(SWEEP_YEARS.iter()).copied();

        for probe_year in probe_years {
            for probe_month in 0_i32..12 {
                let probe = noon_utc_js_date(probe_year, probe_month, 15);

                let long_label = month_part_value(&long_formatter.format_to_parts(&probe));
                if long_label != label {
                    continue;
                }

                // Long label matches this probe instant. Ask the
                // numeric formatter for the ordinal of the same
                // instant — that's the target calendar's civil-order
                // month number. When the numeric formatter also
                // falls back to a name (extremely rare), we can't
                // resolve the label so the caller's fallback kicks in.
                let numeric_value = month_part_value(&numeric_formatter.format_to_parts(&probe));
                let leading: String = numeric_value
                    .chars()
                    .take_while(|c| c.is_numeric())
                    .collect();
                if let Ok(ordinal) = leading.parse::<u8>()
                    && (1..=13).contains(&ordinal)
                {
                    return Some(ordinal);
                }
                return None;
            }
        }

        None
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

        // When the labels share their first character, a single
        // character can't disambiguate AM from PM — Japanese
        // `午前` / `午後` is the canonical example. Per the trait
        // contract, return `None` so CJK-style input fails safely
        // instead of defaulting to AM because it's checked first.
        if am_first.is_some() && am_first == pm_first {
            return None;
        }

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

        // The browser-backed path expects a Gregorian civil day as its
        // JS Date input; `Intl.DateTimeFormat({calendar: …})` only
        // relabels a Gregorian moment into the target calendar. For
        // non-Gregorian sources we'd need to resolve the source fields
        // to Gregorian first, which requires calendar-aware code we
        // don't have in the pure `web-intl` build. Return the source
        // date unchanged when we can't safely convert — callers who need
        // cross-calendar conversion for non-Gregorian sources must
        // enable the `icu4x` feature alongside `web-intl` so the
        // internal bridge handles that case (see the cfg block above).
        if date.calendar != CalendarSystem::Gregorian {
            return date.clone();
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

        // Pin the formatter to UTC so the browser doesn't reinterpret
        // the probe day through the runtime's local timezone. Without
        // this, a probe whose UTC time lands near midnight can be
        // rendered into the previous or next local day, flipping the
        // returned month/day against the caller's expectation.
        Reflect::set(&opts, &"timeZone".into(), &JsValue::from_str("UTC"))
            .expect("Reflect::set on a fresh Object never fails");

        let formatter = Intl::DateTimeFormat::new(&locales, &opts);

        // `Date::new_with_year_month_day` takes a positive `u32` year; for
        // pre-CE Gregorian dates we fall back to returning the source date
        // unchanged, matching `StubIcuProvider`'s behavior outside the
        // supported range.
        let Ok(year_u32) = u32::try_from(date.year) else {
            return date.clone();
        };

        let js_date = noon_utc_js_date(
            year_u32,
            i32::from(date.month.get().saturating_sub(1)),
            i32::from(date.day.get()),
        );

        let parts = formatter.format_to_parts(&js_date);

        let mut year = 0_i32;
        let mut month: u8 = 0;
        let mut day: u8 = 0;
        let mut era_label: Option<String> = None;
        let mut month_label: Option<String> = None;

        for i in 0..parts.length() {
            let part = parts.get(i);

            let part_type = Self::string_property(&part, "type").unwrap_or_default();

            let value = Self::string_property(&part, "value").unwrap_or_default();

            match part_type.as_str() {
                "year" | "relatedYear" => year = value.parse().unwrap_or(0),
                "month" => {
                    month = value.parse().unwrap_or(0);
                    if month == 0 {
                        // Chinese and Dangi leap months surface as
                        // `"6bis"` / `"06bis"` / `"06L"` from
                        // `Intl.DateTimeFormat({ month: "numeric" })` on
                        // ICU-backed runtimes — a leading numeric run
                        // followed by a leap-month marker. Strip the
                        // trailing marker and parse what remains so we
                        // return a usable civil-order ordinal instead
                        // of a source clone. The leap-vs-regular
                        // distinction is lost at this layer (our
                        // `CalendarDate::month: NonZero<u8>` has no slot
                        // for it); callers that need exact leap-month
                        // precision must enable the `icu4x` feature,
                        // which routes through the internal bridge
                        // above.
                        let leading: String =
                            value.chars().take_while(|c| c.is_numeric()).collect();
                        if !leading.is_empty() {
                            month = leading.parse().unwrap_or(0);
                        }
                    }
                    if month == 0 {
                        // Non-numeric month label (e.g., Hebrew
                        // `"Adar II"`). Keep the label so we can
                        // resolve it against the long-form probe.
                        month_label = Some(value);
                    }
                }
                "day" => day = value.parse().unwrap_or(0),
                "era" => era_label = Some(value),
                _ => {}
            }
        }

        // If the browser returned a named month, probe the target calendar
        // to resolve it to an ordinal. This matters for Hebrew leap years
        // where `Intl.DateTimeFormat('en-US', { calendar: 'hebrew',
        // month: 'numeric' })` can still emit `Adar I`/`Adar II` instead
        // of a number.
        if month == 0 {
            if let Some(label) = month_label {
                month = Self::resolve_named_month(target, year, &label).unwrap_or(0);
            }
        }

        let (Some(month_nz), Some(day_nz)) = (NonZero::new(month), NonZero::new(day)) else {
            return date.clone();
        };

        // Map the browser's long era label back to the CLDR era code.
        // Several `Intl.DateTimeFormat` implementations emit macronized
        // long names for Japanese eras — `Shōwa`, `Taishō` — whose
        // ASCII-lowercase forms (`shōwa`, `taishō`) do NOT match the
        // canonical CLDR codes (`showa`, `taisho`). `canonical_era_code`
        // normalizes known macrons (ō/ū/ā/ē/ī) to their ASCII base
        // letters before lowercasing so downstream era-aware helpers
        // see the same codes the stub provider and `Icu4xProvider`
        // would emit. If the browser suppressed the era (no token in
        // parts) or returned an unfamiliar form, fall back to the
        // target's default era.
        let era = if target.has_custom_eras() {
            era_label
                .as_deref()
                .map(|label| Era {
                    code: canonical_era_code(label),
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

/// Builds a JS `Date` pinned to 12:00:00 UTC on the given Gregorian
/// civil day, bypassing the legacy `new Date(year, month)` century
/// quirk and the `new Date()` wall-clock seeding.
///
/// The noon anchor keeps the instant away from midnight so any
/// formatter that, for whatever reason, doesn't honour `timeZone:
/// "UTC"` still lands on the intended day in the browser's local
/// timezone. `year` is a `u32` because `Date::set_utc_full_year_*`
/// rejects negative inputs; callers that need BCE support handle the
/// out-of-range case before reaching this helper.
pub(crate) fn noon_utc_js_date(year: u32, month: i32, day: i32) -> js_sys::Date {
    let js_date = js_sys::Date::new_0();
    js_date.set_utc_full_year_with_month_date(year, month, day);
    js_date.set_utc_hours(12);
    js_date.set_utc_minutes(0);
    js_date.set_utc_seconds(0);
    js_date.set_utc_milliseconds(0);
    js_date
}

/// Returns `true` if the given Hebrew year is a leap year of the 19-year
/// Metonic cycle (years 3, 6, 8, 11, 14, 17, and 19 of the cycle).
///
/// Scoped to Hebrew so `resolve_named_month` can bias its probe years
/// without pulling in a full calendar library; other calendars with
/// conditional month labels should grow their own predicate here.
/// Builds an `Intl.DateTimeFormat` that renders the given target
/// calendar's `month` field with the supplied option value (e.g.,
/// `"long"`, `"2-digit"`). Returns `None` when `Reflect::set` on a
/// fresh `Object` fails — which can't happen on a valid runtime but
/// is preserved as the error path per the surrounding helpers.
fn month_label_formatter(
    locales: &Array,
    target: CalendarSystem,
    month_style: &str,
) -> Option<Intl::DateTimeFormat> {
    let opts = Object::new();
    Reflect::set(
        &opts,
        &"calendar".into(),
        &JsValue::from_str(target.to_bcp47_value()),
    )
    .ok()?;
    Reflect::set(&opts, &"month".into(), &JsValue::from_str(month_style)).ok()?;
    Reflect::set(&opts, &"timeZone".into(), &JsValue::from_str("UTC")).ok()?;
    Some(Intl::DateTimeFormat::new(locales, &opts))
}

/// Extracts the `value` string of the first `month` part from a
/// `formatToParts` result. Returns an empty string if no month part is
/// present (e.g., options that suppress month emission).
fn month_part_value(parts: &Array) -> String {
    for i in 0..parts.length() {
        let part = parts.get(i);
        if Reflect::get(&part, &JsValue::from_str("type"))
            .ok()
            .and_then(|v| v.as_string())
            .as_deref()
            == Some("month")
        {
            return Reflect::get(&part, &JsValue::from_str("value"))
                .ok()
                .and_then(|v| v.as_string())
                .unwrap_or_default();
        }
    }
    String::new()
}

/// Converts a browser-emitted era long label into its canonical CLDR
/// era code.
///
/// `Intl.DateTimeFormat('en-US', { calendar: 'japanese', era: 'long' })`
/// emits labels with macrons (`Shōwa`, `Taishō`) on every major
/// browser. The canonical CLDR era codes drop the macrons
/// (`showa`, `taisho`), so a plain `to_ascii_lowercase()` call leaves
/// the code mismatched against every downstream era lookup.
///
/// The mapping here replaces the handful of Unicode macron letters that
/// appear in Japanese (and transliterated Ryukyuan/Ainu) era labels
/// with their ASCII base letters, then lowercases. Non-macronized
/// labels (`Heisei`, `Reiwa`, `Meiji`, CE/BCE for Gregorian, `AH` for
/// Hijri, etc.) round-trip through the function unchanged apart from
/// the lowercasing.
pub(crate) fn canonical_era_code(label: &str) -> String {
    let mut buf = String::with_capacity(label.len());
    for ch in label.chars() {
        let replacement = match ch {
            'ā' | 'Ā' => 'a',
            'ē' | 'Ē' => 'e',
            'ī' | 'Ī' => 'i',
            'ō' | 'Ō' => 'o',
            'ū' | 'Ū' => 'u',
            other => other,
        };
        for lower in replacement.to_lowercase() {
            buf.push(lower);
        }
    }
    buf
}

pub(crate) const fn is_hebrew_leap_year(year: i32) -> bool {
    let cycle_year = year.rem_euclid(19);
    matches!(cycle_year, 3 | 6 | 8 | 11 | 14 | 17 | 0)
}
