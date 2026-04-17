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
        // English ordinal labels (`"First Month"`, `"Seventh Monthbis"`,
        // `"Twelfth Monthbis"`) have a fixed, year-independent mapping
        // to civil-order ordinal — regardless of whether a year-based
        // probe happens to land inside a cycle where that leap month
        // appears. Parse them directly so labels unreachable from any
        // finite probe list (First Monthbis, Twelfth Monthbis, and
        // the rarer Tenth/Eleventh positions) still resolve.
        //
        // Leap-vs-regular precision is lost at this layer (our
        // `CalendarDate::month: NonZero<u8>` has no slot for it), same
        // caveat as the `"6bis"` numeric path.
        if let Some(ordinal) = parse_english_ordinal_month_label(label) {
            return Some(ordinal);
        }

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
        //
        // The sweep list is curated to hit every Chinese leap-month
        // position observed in the modern era (1900..=2100):
        //   leap 1  — (extremely rare; none within our probe range)
        //   leap 2  — 2023, 2042
        //   leap 3  — 2031, 2050
        //   leap 4  — 2020, 2039
        //   leap 5  — 2028, 2047
        //   leap 6  — 2025, 2044, 2063
        //   leap 7  — 1987, 2006, 2044 (shared with leap 6 at year
        //                boundary), 1968
        //   leap 8  — 1995, 2014, 2052, 1957
        //   leap 9  — 1900-era; 2014 covers via adjacent month window
        //   leap 10 — 2033, 2099
        //   leap 11 — 2033
        //   leap 12 — (extremely rare)
        // Hebrew leap cycle is already covered by the 2024/2025 fast
        // path plus a scattering of the sweep years below.
        const SWEEP_YEARS: [u32; 30] = [
            // Hebrew Metonic cycle coverage + nearby common years
            2023, 2020, 2028, 2021, 2017, 2031, 2019, 2022, 2026, 2027, 2029, 2030, 2015, 2016,
            2018, 2032, 2034, // Chinese leap-month coverage for 7/8/10/11
            1987, 2006, 2044, 1995, 2014, 2052, 2033, 2099, 1968, 1957, 2042, 2063, 2039,
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
                // month number.
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

                // The `2-digit` formatter also returned a non-numeric
                // label (observed on Node/ICU Hebrew: both `long` and
                // `2-digit` emit names like `"Adar II"`). Previously
                // this branch returned `None`, which surfaced up
                // through `convert_date` and demoted the conversion to
                // `date.clone()` — silently losing the civil-order
                // ordinal for valid Hebrew leap-month labels.
                //
                // Resolve the ordinal through the shared ICU4X
                // calendar-arithmetic bridge instead. We already know
                // the Gregorian probe `(probe_year, probe_month, 15)`
                // falls inside the target-calendar month whose long
                // label matches the input, so converting that Gregorian
                // instant through the bridge and reading
                // `CalendarDate::month()` yields the correct civil-
                // order ordinal. This path requires no CLDR data and
                // compiles under pure `--features web-intl`.
                if let Ok(probe_year_i32) = i32::try_from(probe_year)
                    && let Ok(probe_month_u8) = u8::try_from(probe_month + 1)
                    && let Ok(internal) = crate::calendar::internal::CalendarDate::from_iso(
                        probe_year_i32,
                        probe_month_u8,
                        15,
                    )
                {
                    let ordinal = internal.to_calendar(target).month();
                    if (1..=13).contains(&ordinal) {
                        return Some(ordinal);
                    }
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

        // Delegate to the shared ICU4X calendar-arithmetic bridge
        // instead of fabricating a per-calendar constant. The previous
        // implementation hard-coded 12 months for every calendar
        // outside `Hebrew/Ethiopic/Coptic`, which rejected valid
        // Chinese/Dangi leap-month dates at the `CalendarDate::new`
        // validation edge and normalised civil-ordinal 13 inputs into
        // the next year on `add_months(0)`. The bridge produces the
        // correct per-year month count including leap-cycle widenings.
        // Hebrew and the fixed-13 calendars are bounded via the
        // `bounded_*` fast path above, so the bridge handles the
        // remaining lunisolar/luni-solar calendars where the year-
        // specific answer varies.
        crate::calendar::internal::months_in_year(year, *calendar, era).unwrap_or(12)
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
                // Delegate to the shared ICU4X calendar-arithmetic
                // bridge (available whenever `web-intl` is on — the
                // `calendar::internal` module is gated on
                // `any(feature = "icu4x", feature = "web-intl")` and
                // does not require CLDR formatter data). The previous
                // implementation returned a flat 30 for every non-
                // Gregorian month outside the `bounded_*` table, which
                // both accepted impossible day 30 inputs on 29-day
                // months and rejected correct days on 31-day months
                // during `CalendarDate::new` validation.
                crate::calendar::internal::days_in_month(year, month, *calendar, era)
                    .unwrap_or_else(|| gregorian_days_in_month(year, month))
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

        // Two Intl.Locale shapes deliver week metadata:
        //
        // 1. `getWeekInfo()` — the original TC39 proposal shape, still
        //    the canonical method on Chrome, Safari, and Node.
        // 2. `weekInfo` property — the newer getter shape that some
        //    engines (including Firefox in recent releases) expose
        //    directly without a method call.
        //
        // The previous implementation only tried form (1), so every
        // engine on form (2) silently fell through to the region
        // table below and got wrong answers for locales the table
        // doesn't cover (e.g., `pt-BR` → Monday instead of Sunday).
        // We now probe both shapes in order.
        if let Ok(js_locale) = Intl::Locale::new(&locale.to_bcp47()) {
            let js_value: JsValue = js_locale.into();

            if let Some(day) = read_week_info_first_day(&js_value) {
                return weekday_from_iso_index(day);
            }
        }

        // Fallback to the shared region-based table for older engines
        // that expose neither shape.
        crate::WeekInfo::for_locale(locale).first_day
    }

    fn convert_date(&self, date: &CalendarDate, target: CalendarSystem) -> CalendarDate {
        if date.calendar == target {
            return date.clone();
        }

        // Non-Gregorian sources must route through the shared ICU4X
        // calendar-arithmetic bridge: the browser's
        // `Intl.DateTimeFormat({calendar: …})` path only *relabels* a
        // Gregorian instant, so feeding it a non-Gregorian source would
        // reinterpret the raw year/month/day fields as Gregorian and
        // corrupt the result. The `calendar::internal` module is
        // compiled whenever either the `icu4x` or `web-intl` feature is
        // on (see `calendar.rs`), so the bridge is always available
        // under the same feature gate that compiles this provider and
        // does not require the `compiled_data` CLDR payload.
        //
        // Contract reminder: `IcuProvider::convert_date` returns a
        // [`CalendarDate`] in `target`. Silently returning the source
        // calendar for non-Gregorian inputs violates that contract,
        // panics in `CalendarDate::add_days_with_provider` (Gregorian-
        // only arithmetic) and lets `TypedCalendarDate::to_calendar`
        // wrap the wrong calendar in release builds (only guarded by
        // `debug_assert`). The bridge eliminates that footgun.
        if date.calendar != CalendarSystem::Gregorian {
            return bridge_convert(date, target).unwrap_or_else(|| date.clone());
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

        // `js_sys::Date`'s `setUTCFullYear` accepts any integer year,
        // but the browser path only works when the formatted parts
        // ordering matches our positive-year assumptions. For BCE
        // Gregorian inputs (`date.year < 0`) the safest option is the
        // shared ICU4X calendar-arithmetic bridge: it handles negative
        // year arithmetic directly and produces the target calendar's
        // correct civil-order fields without going through
        // `Intl.DateTimeFormat` at all. Previously we returned the
        // source date unchanged here, which violated the
        // `IcuProvider::convert_date` contract by handing the caller
        // back a date in the *source* calendar.
        let Ok(year_u32) = u32::try_from(date.year) else {
            return bridge_convert(date, target).unwrap_or_else(|| date.clone());
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
                        // followed by a leap-month marker. The numeric
                        // prefix is the underlying *lunar* month, and
                        // the leap month always sits in the civil-
                        // ordinal slot immediately after it (e.g., leap
                        // 6 sits between regular 6 and regular 7 in
                        // year ordering), so the civil ordinal that
                        // matches `DateFields.ordinal_month` is
                        // `prefix + 1`. Without this shift `"6bis"`
                        // would collide with regular 6 and feed the
                        // wrong month into downstream validation.
                        let leading: String =
                            value.chars().take_while(|c| c.is_numeric()).collect();
                        let had_leap_marker =
                            !leading.is_empty() && leading.len() < value.trim().len();
                        if !leading.is_empty() {
                            if let Ok(base) = leading.parse::<u8>() {
                                month = if had_leap_marker {
                                    base.saturating_add(1).min(13)
                                } else {
                                    base
                                };
                            }
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

        // Map the browser's long era label back to the CLDR era code
        // that ICU4X's `Date::try_from_fields` expects. Two sharp
        // edges to handle:
        //
        // 1. Japanese: `Intl.DateTimeFormat` emits macronized names
        //    (`Shōwa`, `Taishō`) whose plain-lowercase forms (`shōwa`)
        //    do NOT match the ASCII CLDR codes (`showa`).
        //
        // 2. ROC: `Intl.DateTimeFormat('en-US', { calendar: 'roc',
        //    era: 'long' })` emits `Minguo` / `Before R.O.C.` /
        //    `B.R.O.C.`, which a plain-lowercase pass persists as
        //    `minguo` / `b.r.o.c.` — neither is a CLDR code. ICU4X
        //    expects `roc` (post-1912) and `broc` (before).
        //
        // `era_code_for_calendar` covers only the well-known modern
        // allow-list. For labels outside it — notably Japanese
        // historical eras like `Kansei (1789–1801)`, `Meiwa`,
        // `Bunsei`, `Tenpō`, etc. — silently defaulting to
        // `default_era_for(target)` (= Reiwa for Japanese) would
        // rewrite the date to the wrong era and corrupt downstream
        // era-boundary behaviour (e.g., a 1800 date becoming
        // `Reiwa 12`). Route the unmapped label through the shared
        // ICU4X calendar-arithmetic bridge, which knows the full
        // CLDR era vocabulary for every calendar we handle.
        //
        // Precedence:
        // 1. Allow-list hit → persist the browser's label as
        //    `display_name` and the mapped CLDR code.
        // 2. Label present but not in allow-list → bridge fallback
        //    on the whole date; refuses silently on bridge
        //    rejection by returning `date.clone()`.
        // 3. Label absent (browser suppressed the era part) →
        //    `default_era_for(target)`; only safe because the
        //    browser told us the date actually lives in the target
        //    calendar's default era.
        let era = if target.has_custom_eras() {
            match era_label.as_deref() {
                Some(label) => match era_code_for_calendar(target, label) {
                    Some(code) => Some(Era {
                        code,
                        display_name: label.to_string(),
                    }),
                    None => {
                        return bridge_convert(date, target).unwrap_or_else(|| date.clone());
                    }
                },
                None => default_era_for(target),
            }
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

/// Parses an English ordinal month label of the form
/// `"<Ordinal> Month"` or `"<Ordinal> Monthbis"` into its 1..=13
/// civil-order ordinal (the same representation this crate's
/// `CalendarDate::month` carries, matching ICU4X
/// `DateFields.ordinal_month`).
///
/// These labels come from `Intl.DateTimeFormat('en-US',
/// { calendar: 'chinese', month: 'long' })` on ICU-backed runtimes
/// and are the only way some leap-month positions (`First Monthbis`,
/// `Tenth Monthbis`, `Twelfth Monthbis`) can be resolved without
/// maintaining an exhaustive calendar-year probe list covering every
/// possible leap-cycle position in history.
///
/// The `bis` suffix signals a leap month, which always sits in the
/// civil-order slot immediately after its base lunar month. We bump
/// the base ordinal by 1 for leap labels — so `"Sixth Monthbis"`
/// resolves to 7 (leap 6 in civil order), matching the numeric-token
/// path's handling of `"6bis"` → 7.
pub(crate) fn parse_english_ordinal_month_label(label: &str) -> Option<u8> {
    // Detect and strip the leap-month marker, remembering whether it
    // was present so we can bump the ordinal by 1 below.
    let (core, is_leap) = match label.strip_suffix("bis") {
        Some(rest) => (rest, true),
        None => (label, false),
    };
    // Strip " Month" suffix; accept both with and without to stay
    // tolerant of future CLDR wording tweaks.
    let core = core.trim();
    let core = core.strip_suffix(" Month").unwrap_or(core);
    let base = match core.trim() {
        "First" => Some(1_u8),
        "Second" => Some(2),
        "Third" => Some(3),
        "Fourth" => Some(4),
        "Fifth" => Some(5),
        "Sixth" => Some(6),
        "Seventh" => Some(7),
        "Eighth" => Some(8),
        "Ninth" => Some(9),
        "Tenth" => Some(10),
        "Eleventh" => Some(11),
        "Twelfth" => Some(12),
        "Thirteenth" => Some(13),
        _ => None,
    }?;
    if is_leap {
        Some(base.saturating_add(1).min(13))
    } else {
        Some(base)
    }
}

/// Reads the `firstDay` field from an `Intl.Locale` instance by
/// probing both known shapes: the `getWeekInfo()` method form and
/// the `weekInfo` property form. Returns the 1..=7 ISO day index
/// (Monday=1, Sunday=7) if either shape delivers a number, or
/// `None` if the engine exposes neither.
pub(crate) fn read_week_info_first_day(js_locale_value: &JsValue) -> Option<u8> {
    // Form 1: `getWeekInfo()` method (Chrome, Safari, Node).
    if let Ok(get_week_info) = Reflect::get(js_locale_value, &JsValue::from_str("getWeekInfo")) {
        if get_week_info.is_function() {
            if let Ok(func) = get_week_info.dyn_into::<Function>() {
                if let Ok(info) = func.call0(js_locale_value) {
                    if let Some(day) = read_first_day(&info) {
                        return Some(day);
                    }
                }
            }
        }
    }

    // Form 2: `weekInfo` getter/property (Firefox and other engines
    // that implement the TC39 proposal's property shape).
    if let Ok(info) = Reflect::get(js_locale_value, &JsValue::from_str("weekInfo")) {
        if !info.is_undefined() && !info.is_null() {
            if let Some(day) = read_first_day(&info) {
                return Some(day);
            }
        }
    }

    None
}

fn read_first_day(info: &JsValue) -> Option<u8> {
    Reflect::get(info, &JsValue::from_str("firstDay"))
        .ok()?
        .as_f64()
        .map(|day| day as u8)
}

/// Maps an ISO 8601 weekday index (1=Monday .. 7=Sunday) to a
/// [`Weekday`] variant. Out-of-range inputs collapse to Monday,
/// matching `getWeekInfo()`'s documented default.
pub(crate) const fn weekday_from_iso_index(day: u8) -> Weekday {
    match day {
        2 => Weekday::Tuesday,
        3 => Weekday::Wednesday,
        4 => Weekday::Thursday,
        5 => Weekday::Friday,
        6 => Weekday::Saturday,
        7 => Weekday::Sunday,
        // `getWeekInfo().firstDay` is documented as 1..=7 with
        // Monday=1; treat 1 and any unexpected value as Monday
        // (the ISO 8601 default).
        _ => Weekday::Monday,
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
/// Runs the shared ICU4X calendar-arithmetic bridge on `date` and
/// converts it into `target`. Returns `None` when the bridge rejects
/// the source (e.g., invalid era/year/month/day combination) so the
/// caller can fall back to `date.clone()` rather than fabricate a
/// result.
///
/// The `calendar::internal` module is compiled whenever either the
/// `icu4x` or `web-intl` feature is on (see `calendar.rs`), so this
/// path is always available under the same feature gate as the
/// provider itself.
///
/// Used in two places in [`WebIntlProvider::convert_date`]:
/// - Non-Gregorian sources (the browser path only relabels Gregorian
///   instants, so it cannot convert a non-Gregorian source directly).
/// - Gregorian sources whose browser era label falls outside the
///   [`era_code_for_calendar`] allow-list (Japanese historical eras
///   like `Kansei`, `Meiwa`, `Bunsei`, `Tenpō`, etc.).
pub(crate) fn bridge_convert(date: &CalendarDate, target: CalendarSystem) -> Option<CalendarDate> {
    let internal = crate::calendar::internal::CalendarDate::try_from(date).ok()?;
    let converted = internal.to_calendar(target);

    let month_nz = NonZero::new(converted.month())?;
    let day_nz = NonZero::new(converted.day())?;

    Some(CalendarDate {
        calendar: target,
        era: converted
            .era()
            .filter(|_| target.has_custom_eras())
            .map(|code| Era {
                code: code.clone(),
                display_name: code,
            }),
        year: converted.year(),
        month: month_nz,
        day: day_nz,
    })
}

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

/// Maps a browser-emitted `era: "long"` label onto the CLDR era code
/// ICU4X accepts for `target`'s calendar, returning `None` for labels
/// we cannot confidently map.
///
/// Unlike [`canonical_era_code`] — which only ASCII-folds macrons and
/// lowercases — this function also strips separator characters
/// (`.`, whitespace) and validates the normalised value against a
/// per-calendar allow-list. ICU4X's `Date::try_from_fields` rejects
/// any era code outside its vocabulary, so persisting an unmapped
/// label (`minguo`, `b.r.o.c.`, `anno mundi`, …) would break the
/// internal-bridge round-trip for `calendar::internal::CalendarDate::
/// try_from(&ars_i18n::CalendarDate)`.
///
/// Calendars covered here are restricted to those with
/// well-documented ICU4X era vocabularies:
///
/// - [`CalendarSystem::Japanese`]: `reiwa` / `heisei` / `showa` /
///   `taisho` / `meiji` (macron variants accepted).
/// - [`CalendarSystem::Roc`]: `roc` for post-1912 (`Minguo`), `broc`
///   for before (`Before R.O.C.`, `B.R.O.C.`).
///
/// For every other custom-era calendar the browser-emitted label is
/// too varied to map reliably (Hebrew `"Anno Mundi"` vs `"AM"`,
/// Ethiopic `"Amete Mihret"` / `"ERA0"` / `"Incarnation Era"`, Islamic
/// `"AH"` vs `"Anno Hegirae"`, …), so we return `None`. The caller's
/// downstream handling then drops the era entirely rather than
/// persisting display text as a CLDR code.
pub(crate) fn era_code_for_calendar(target: CalendarSystem, label: &str) -> Option<String> {
    // Normalise: macron-fold + lowercase + strip separators so a
    // label like `"Before R.O.C."` collapses to `"beforeroc"` and
    // `"Anno Mundi"` to `"annomundi"`.
    let normalized: String = canonical_era_code(label)
        .chars()
        .filter(|c| !c.is_ascii_whitespace() && *c != '.')
        .collect();

    match target {
        CalendarSystem::Japanese => match normalized.as_str() {
            "reiwa" | "heisei" | "showa" | "taisho" | "meiji" => Some(normalized),
            _ => None,
        },
        CalendarSystem::Roc => match normalized.as_str() {
            // Post-1912: Intl emits `"Minguo"` (en-US) or the
            // abbreviation `"ROC"`.
            "minguo" | "roc" => Some(String::from("roc")),
            // Before ROC: observed forms include
            // `"Before R.O.C."` → `"beforeroc"` after normalisation,
            // the abbreviation `"B.R.O.C."` → `"broc"`, and
            // variants like `"Before Minguo"` → `"beforeminguo"`.
            "broc" | "beforeroc" | "beforeminguo" => Some(String::from("broc")),
            _ => None,
        },
        // Other custom-era calendars have Intl long labels that vary
        // too much to map without test coverage against real browser
        // output. Drop the era rather than persist garbage.
        _ => None,
    }
}

pub(crate) const fn is_hebrew_leap_year(year: i32) -> bool {
    let cycle_year = year.rem_euclid(19);
    matches!(cycle_year, 3 | 6 | 8 | 11 | 14 | 17 | 0)
}
