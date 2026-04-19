//! WASM `NumberFormatter` tests for the `web-intl` backend.
//!
//! Run with:
//! `wasm-pack test --headless --chrome crates/ars-i18n --no-default-features --features std,web-intl`.

use alloc::{collections::BTreeSet, format, string::String};
use core::num::NonZeroU8;

use js_sys::{
    Intl::{NumberFormat, NumberFormatOptions, SupportedValuesKey, supported_values_of},
    Reflect,
};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

use crate::{
    CurrencyCode, Locale, NumberFormatOptions as ArsNumberFormatOptions, NumberFormatter,
    NumberStyle, RoundingMode, SignDisplay, UnitDisplay,
};

wasm_bindgen_test_configure!(run_in_browser);

fn locale(tag: &str) -> Locale {
    Locale::parse(tag).expect("test locale should parse")
}

fn style_options(style: NumberStyle) -> ArsNumberFormatOptions {
    ArsNumberFormatOptions {
        style,
        ..ArsNumberFormatOptions::default()
    }
}

fn unsupported_browser_measure_unit() -> (crate::MeasureUnit, String) {
    let supported_units = supported_values_of(SupportedValuesKey::Unit)
        .iter()
        .filter_map(|value| value.as_string())
        .collect::<BTreeSet<_>>();

    for (candidate, _) in super::CLDR_IDS_TRIE.iter() {
        if supported_units.contains(candidate.as_str()) {
            continue;
        }

        let Ok(unit) = crate::MeasureUnit::try_from_str(&candidate) else {
            continue;
        };

        return (unit, candidate);
    }

    panic!("expected a CLDR unit that Intl.NumberFormat does not sanction");
}

struct SupportedValuesOfGuard {
    original: JsValue,
}

impl SupportedValuesOfGuard {
    fn remove() -> Self {
        let intl = Reflect::get(&js_sys::global(), &JsValue::from_str("Intl"))
            .expect("global Intl should exist");

        let original = Reflect::get(&intl, &JsValue::from_str("supportedValuesOf"))
            .expect("reading Intl.supportedValuesOf should succeed");

        Reflect::set(
            &intl,
            &JsValue::from_str("supportedValuesOf"),
            &JsValue::UNDEFINED,
        )
        .expect("overriding Intl.supportedValuesOf should succeed");

        Self { original }
    }
}

impl Drop for SupportedValuesOfGuard {
    fn drop(&mut self) {
        let intl = Reflect::get(&js_sys::global(), &JsValue::from_str("Intl"))
            .expect("global Intl should exist");

        Reflect::set(
            &intl,
            &JsValue::from_str("supportedValuesOf"),
            &self.original,
        )
        .expect("restoring Intl.supportedValuesOf should succeed");
    }
}

struct NegativeSignDisplaySupportGuard {
    original: Option<bool>,
}

impl NegativeSignDisplaySupportGuard {
    fn unsupported() -> Self {
        Self {
            original: super::web_intl::replace_negative_sign_display_support_override(Some(false)),
        }
    }
}

impl Drop for NegativeSignDisplaySupportGuard {
    fn drop(&mut self) {
        super::web_intl::replace_negative_sign_display_support_override(self.original);
    }
}

#[wasm_bindgen_test]
fn web_intl_decimal_formats_en_us_with_grouping_and_decimal_separator() {
    let formatter = NumberFormatter::new(&locale("en-US"), ArsNumberFormatOptions::default());

    assert_eq!(formatter.format(1234.56), "1,234.56");
    assert_eq!(formatter.decimal_separator(), '.');
    assert_eq!(formatter.grouping_separator(), Some(','));
}

#[wasm_bindgen_test]
fn web_intl_decimal_formats_de_de_with_locale_separators() {
    let formatter = NumberFormatter::new(&locale("de-DE"), ArsNumberFormatOptions::default());

    assert_eq!(formatter.format(1234.56), "1.234,56");
    assert_eq!(formatter.decimal_separator(), ',');
    assert_eq!(formatter.grouping_separator(), Some('.'));
}

#[wasm_bindgen_test]
fn web_intl_decimal_formats_arabic_indic_digits() {
    let formatter = NumberFormatter::new(&locale("ar-EG"), ArsNumberFormatOptions::default());

    let formatted = formatter.format(1234.56);

    assert_eq!(formatted, "١٬٢٣٤٫٥٦");
    assert_eq!(formatter.decimal_separator(), '٫');
    assert_eq!(formatter.grouping_separator(), Some('٬'));
}

#[wasm_bindgen_test]
fn web_intl_percent_preserves_fractional_input_semantics() {
    let formatter = NumberFormatter::new(&locale("en-US"), style_options(NumberStyle::Percent));

    assert_eq!(formatter.format(0.47), "47%");
    assert_eq!(formatter.format_percent(0.47, None), "47%");
    assert_eq!(formatter.format_percent(0.475, Some(1)), "47.5%");
    assert_eq!(formatter.parse("47%"), Some(0.47));
}

#[wasm_bindgen_test]
fn web_intl_currency_uses_iso_minor_unit_defaults() {
    let en = NumberFormatter::new(&locale("en-US"), ArsNumberFormatOptions::default());

    let ja = NumberFormatter::new(&locale("ja-JP"), ArsNumberFormatOptions::default());

    assert_eq!(en.format_currency(1234.5, "USD"), "$1,234.50");
    assert_eq!(ja.format_currency(1234.5, "JPY"), "￥1,234");
}

#[wasm_bindgen_test]
fn web_intl_currency_and_decimal_output_round_trip_through_parse() {
    let de = NumberFormatter::new(&locale("de-DE"), ArsNumberFormatOptions::default());

    let currency = NumberFormatter::new(&locale("en-US"), ArsNumberFormatOptions::default());

    let decimal_text = de.format(1234.56);

    let currency_text = currency.format_currency(1234.5, "USD");

    assert_eq!(de.parse(&decimal_text), Some(1234.56));
    assert_eq!(currency.parse(&currency_text), Some(1234.5));
}

#[wasm_bindgen_test]
fn web_intl_parse_accepts_unicode_minus_from_browser_shaped_output() {
    let formatter = NumberFormatter::new(&locale("sv-SE"), ArsNumberFormatOptions::default());

    let positive = formatter.format(1234.5);
    let negative = formatter.format(-1234.5);
    let browser_shaped_negative = if negative.contains('\u{2212}') {
        negative
    } else {
        format!("−{positive}")
    };

    assert_eq!(formatter.parse(&browser_shaped_negative), Some(-1234.5));
}

#[wasm_bindgen_test]
fn web_intl_unit_formatting_uses_browser_backend() {
    let unit = crate::MeasureUnit::try_from_str("kilogram").expect("kilogram should parse");

    let formatter = NumberFormatter::new(
        &locale("en-US"),
        ArsNumberFormatOptions {
            style: NumberStyle::Unit(unit),
            unit_display: UnitDisplay::Short,
            ..ArsNumberFormatOptions::default()
        },
    );

    let formatted = formatter.format(5.0);

    assert!(formatted.contains("kg"), "expected kg in {formatted:?}");
}

#[wasm_bindgen_test]
fn web_intl_unsupported_units_fall_back_to_decimal_formatting() {
    let (unit, unit_id) = unsupported_browser_measure_unit();

    let decimal = NumberFormatter::new(&locale("en-US"), ArsNumberFormatOptions::default());

    let formatter = NumberFormatter::new(
        &locale("en-US"),
        ArsNumberFormatOptions {
            style: NumberStyle::Unit(unit),
            unit_display: UnitDisplay::Short,
            ..ArsNumberFormatOptions::default()
        },
    );

    assert_eq!(
        formatter.format(5.0),
        decimal.format(5.0),
        "expected unsupported unit {unit_id:?} to fall back to decimal formatting"
    );
}

#[wasm_bindgen_test]
fn web_intl_units_fall_back_to_decimal_when_supported_values_of_is_unavailable() {
    let _guard = SupportedValuesOfGuard::remove();

    let unit = crate::MeasureUnit::try_from_str("kilogram").expect("kilogram should parse");

    let decimal = NumberFormatter::new(&locale("en-US"), ArsNumberFormatOptions::default());

    let formatter = NumberFormatter::new(
        &locale("en-US"),
        ArsNumberFormatOptions {
            style: NumberStyle::Unit(unit),
            unit_display: UnitDisplay::Short,
            ..ArsNumberFormatOptions::default()
        },
    );

    assert_eq!(
        formatter.format(5.0),
        decimal.format(5.0),
        "expected missing Intl.supportedValuesOf to fall back to decimal formatting"
    );
}

#[wasm_bindgen_test]
fn web_intl_rounding_modes_cover_remaining_browser_mappings() {
    let cases = [
        (RoundingMode::HalfUp, 1.25, "1.3"),
        (RoundingMode::HalfDown, 1.25, "1.2"),
        (RoundingMode::Ceiling, 1.21, "1.3"),
        (RoundingMode::Floor, 1.29, "1.2"),
        (RoundingMode::Truncate, 1.29, "1.2"),
    ];

    for (rounding_mode, value, expected) in cases {
        let formatter = NumberFormatter::new(
            &locale("en-US"),
            ArsNumberFormatOptions {
                max_fraction_digits: 1,
                rounding_mode,
                ..ArsNumberFormatOptions::default()
            },
        );

        assert_eq!(formatter.format(value), expected, "case {rounding_mode:?}");
    }
}

#[wasm_bindgen_test]
fn web_intl_grouping_flag_is_reflected_in_output() {
    let formatter = NumberFormatter::new(
        &locale("en-US"),
        ArsNumberFormatOptions {
            use_grouping: false,
            ..ArsNumberFormatOptions::default()
        },
    );

    assert_eq!(formatter.format(1234.56), "1234.56");
    assert_eq!(formatter.grouping_separator(), Some(','));
}

#[wasm_bindgen_test]
fn web_intl_normalizes_inverted_fraction_digit_bounds_before_formatting() {
    let formatter = NumberFormatter::new(
        &locale("en-US"),
        ArsNumberFormatOptions {
            min_fraction_digits: 3,
            max_fraction_digits: 1,
            ..ArsNumberFormatOptions::default()
        },
    );

    assert_eq!(formatter.format(1.2), "1.200");
}

#[wasm_bindgen_test]
fn web_intl_clamps_out_of_range_digit_bounds_before_formatting() {
    let actual = NumberFormatter::new(
        &locale("en-US"),
        ArsNumberFormatOptions {
            min_integer_digits: NonZeroU8::new(42).expect("42 should be non-zero"),
            min_fraction_digits: 150,
            max_fraction_digits: 200,
            ..ArsNumberFormatOptions::default()
        },
    );

    let expected = NumberFormatter::new(
        &locale("en-US"),
        ArsNumberFormatOptions {
            min_integer_digits: NonZeroU8::new(21).expect("21 should be non-zero"),
            min_fraction_digits: 100,
            max_fraction_digits: 100,
            ..ArsNumberFormatOptions::default()
        },
    );

    assert_eq!(actual.format(1.5), expected.format(1.5));
}

#[wasm_bindgen_test]
fn web_intl_sign_display_is_reflected_in_output() {
    let formatter = NumberFormatter::new(
        &locale("en-US"),
        ArsNumberFormatOptions {
            sign_display: SignDisplay::Always,
            ..ArsNumberFormatOptions::default()
        },
    );

    assert_eq!(formatter.format(12.0), "+12");
}

#[wasm_bindgen_test]
fn web_intl_other_sign_display_variants_are_preserved() {
    let never = NumberFormatter::new(
        &locale("en-US"),
        ArsNumberFormatOptions {
            sign_display: SignDisplay::Never,
            ..ArsNumberFormatOptions::default()
        },
    );

    let except_zero = NumberFormatter::new(
        &locale("en-US"),
        ArsNumberFormatOptions {
            sign_display: SignDisplay::ExceptZero,
            ..ArsNumberFormatOptions::default()
        },
    );

    assert_eq!(never.format(-12.0), "12");
    assert_eq!(except_zero.format(12.0), "+12");
    assert_eq!(except_zero.format(0.0), "0");
}

#[wasm_bindgen_test]
fn web_intl_negative_sign_display_differs_from_auto_for_negative_zero() {
    let auto = NumberFormatter::new(
        &locale("en-US"),
        ArsNumberFormatOptions {
            sign_display: SignDisplay::Auto,
            ..ArsNumberFormatOptions::default()
        },
    );

    let negative = NumberFormatter::new(
        &locale("en-US"),
        ArsNumberFormatOptions {
            sign_display: SignDisplay::Negative,
            ..ArsNumberFormatOptions::default()
        },
    );

    assert_eq!(auto.format(-0.0), "-0");
    assert_eq!(negative.format(-0.0), "0");
}

#[wasm_bindgen_test]
fn web_intl_negative_sign_display_falls_back_when_browser_rejects_it() {
    let _guard = NegativeSignDisplaySupportGuard::unsupported();

    let formatter = NumberFormatter::new(
        &locale("en-US"),
        ArsNumberFormatOptions {
            sign_display: SignDisplay::Negative,
            ..ArsNumberFormatOptions::default()
        },
    );

    assert_eq!(formatter.format(-0.0), "-0");
}

#[wasm_bindgen_test]
fn web_intl_rounding_mode_half_even_matches_browser_support() {
    let formatter = NumberFormatter::new(
        &locale("en-US"),
        ArsNumberFormatOptions {
            max_fraction_digits: 1,
            rounding_mode: RoundingMode::HalfEven,
            ..ArsNumberFormatOptions::default()
        },
    );

    assert_eq!(formatter.format(1.25), "1.2");
    assert_eq!(formatter.format(1.35), "1.4");
}

#[wasm_bindgen_test]
fn web_intl_unit_display_variants_cover_browser_option_mapping() {
    let unit = crate::MeasureUnit::try_from_str("kilogram").expect("kilogram should parse");

    let long = NumberFormatter::new(
        &locale("en-US"),
        ArsNumberFormatOptions {
            style: NumberStyle::Unit(unit.clone()),
            unit_display: UnitDisplay::Long,
            ..ArsNumberFormatOptions::default()
        },
    );

    let narrow = NumberFormatter::new(
        &locale("en-US"),
        ArsNumberFormatOptions {
            style: NumberStyle::Unit(unit),
            unit_display: UnitDisplay::Narrow,
            ..ArsNumberFormatOptions::default()
        },
    );

    let long_formatted = long.format(5.0);

    let narrow_formatted = narrow.format(5.0);

    assert!(
        long_formatted.contains("kilogram"),
        "expected long unit name in {long_formatted:?}"
    );
    assert!(
        narrow_formatted.contains("kg"),
        "expected narrow unit abbreviation in {narrow_formatted:?}"
    );
}

#[wasm_bindgen_test]
fn web_intl_style_specific_formatters_resolve_expected_browser_options() {
    let opts = NumberFormatOptions::new();

    opts.set_style(js_sys::Intl::NumberFormatStyle::Currency);
    opts.set_currency(CurrencyCode::USD.as_str());

    let locales = js_sys::Array::of1(&JsValue::from_str("en-US"));

    let formatter = NumberFormat::new(&locales, opts.as_ref());

    let resolved = formatter.resolved_options();

    let currency = Reflect::get(resolved.as_ref(), &"currency".into())
        .ok()
        .and_then(|value| value.as_string());

    assert_eq!(currency.as_deref(), Some("USD"));
}
