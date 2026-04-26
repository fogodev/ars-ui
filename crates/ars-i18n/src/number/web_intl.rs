//! Browser-backed `number::Formatter` helpers for `web-intl` builds on `wasm32`.

use alloc::{string::String, vec::Vec};
#[cfg(test)]
use core::sync::atomic::{AtomicI8, Ordering};

use js_sys::{
    Array, Function,
    Intl::{
        CurrencyDisplay, CurrencySign, NumberFormat, NumberFormatOptions as JsNumberFormatOptions,
        NumberFormatPart, NumberFormatPartType, NumberFormatStyle, RoundingMode as JsRoundingMode,
        SignDisplay as JsSignDisplay, UnitDisplay as JsUnitDisplay, UseGrouping,
    },
    Object, Reflect,
};
use wasm_bindgen::{JsCast, JsValue};

use super::{RoundingMode, SignDisplay, UnitDisplay, resolve_measure_unit_id};
use crate::Locale;

#[cfg(test)]
static NEGATIVE_SIGN_DISPLAY_SUPPORT_OVERRIDE: AtomicI8 = AtomicI8::new(-1);

/// Browser-backed formatter for the public `number::Formatter` API.
#[derive(Clone, Debug)]
pub(super) struct WebIntlNumberFormatter {
    inner: NumberFormat,
}

impl WebIntlNumberFormatter {
    /// Construct a browser-backed formatter using `Intl.NumberFormat`.
    pub(super) fn new(locale: &Locale, options: &super::FormatOptions) -> Self {
        let locales = Array::of1(&JsValue::from_str(&locale.to_bcp47()));

        let js_options = build_options(options);

        let inner = NumberFormat::new(&locales, &js_options);

        Self { inner }
    }

    /// Format the given numeric value through the browser formatter.
    pub(super) fn format(&self, value: f64) -> String {
        let format_fn: Function = self.inner.format();

        format_fn
            .call1(&JsValue::UNDEFINED, &JsValue::from_f64(value))
            .ok()
            .and_then(|value| value.as_string())
            .unwrap_or_default()
    }

    fn format_to_parts(&self, value: f64) -> Vec<NumberFormatPart> {
        self.inner
            .format_to_parts(value)
            .iter()
            .map(JsCast::unchecked_into::<NumberFormatPart>)
            .collect()
    }
}

/// Extract decimal and grouping separators from `Intl.NumberFormat::formatToParts`.
#[must_use]
pub(super) fn decimal_and_group_separators(locale: &Locale) -> (char, char) {
    let formatter = WebIntlNumberFormatter::new(locale, &super::FormatOptions::default());

    let mut decimal_separator = '.';

    let mut grouping_separator = None;

    for part in formatter.format_to_parts(12345.6) {
        match part.type_() {
            NumberFormatPartType::Decimal => {
                decimal_separator = first_char(&part.value()).unwrap_or('.');
            }

            NumberFormatPartType::Group if grouping_separator.is_none() => {
                grouping_separator = first_char(&part.value());
            }

            _ => {}
        }
    }

    let fallback_group = match locale.language() {
        "de" | "pt" => '.',
        "fr" => ' ',
        "ar" => '٬',
        _ => ',',
    };

    (
        decimal_separator,
        grouping_separator.unwrap_or(fallback_group),
    )
}

fn build_options(options: &super::FormatOptions) -> JsNumberFormatOptions {
    let js_options = JsNumberFormatOptions::new();

    let min_integer_digits = clamped_minimum_integer_digits(options);

    let (min_fraction_digits, max_fraction_digits) = clamped_fraction_digit_bounds(options);

    js_options.set_minimum_integer_digits(min_integer_digits);

    js_options.set_minimum_fraction_digits(min_fraction_digits);

    js_options.set_maximum_fraction_digits(max_fraction_digits);

    js_options.set_use_grouping(if options.use_grouping {
        UseGrouping::True
    } else {
        UseGrouping::False
    });

    Reflect::set(
        js_options.as_ref(),
        &JsValue::from_str("useGrouping"),
        &JsValue::from_bool(options.use_grouping),
    )
    .expect("setting Intl.NumberFormat useGrouping should succeed");

    js_options.set_rounding_mode(options.rounding_mode.into_js());

    if let Some(sign_display) = options.sign_display.into_js() {
        js_options.set_sign_display(sign_display);
    } else if browser_supports_negative_sign_display() {
        // `js-sys` does not expose the Intl `"negative"` variant yet, so
        // set it through the raw options bag to preserve the spec-level
        // distinction from `"auto"` for cases like negative zero.
        Reflect::set(
            js_options.as_ref(),
            &JsValue::from_str("signDisplay"),
            &JsValue::from_str("negative"),
        )
        .expect("setting Intl.NumberFormat signDisplay should succeed");
    }

    match &options.style {
        super::Style::Decimal => {
            js_options.set_style(NumberFormatStyle::Decimal);
        }

        super::Style::Percent => {
            js_options.set_style(NumberFormatStyle::Percent);
        }

        super::Style::Currency(code) => {
            js_options.set_style(NumberFormatStyle::Currency);

            js_options.set_currency(code.as_str());

            js_options.set_currency_display(CurrencyDisplay::Symbol);

            js_options.set_currency_sign(CurrencySign::Standard);
        }

        super::Style::Unit(unit) => {
            let unit_id = resolve_measure_unit_id(unit)
                .expect("unit formatter requires a resolvable CLDR unit id");

            if browser_supports_unit(&unit_id) {
                js_options.set_style(NumberFormatStyle::Unit);

                js_options.set_unit(&unit_id);

                js_options.set_unit_display(options.unit_display.into_js());
            } else {
                js_options.set_style(NumberFormatStyle::Decimal);
            }
        }
    }

    js_options
}

fn normalized_fraction_digit_bounds(options: &super::FormatOptions) -> (u8, u8) {
    (
        options.min_fraction_digits,
        options.max_fraction_digits.max(options.min_fraction_digits),
    )
}

fn clamped_minimum_integer_digits(options: &super::FormatOptions) -> u8 {
    options.min_integer_digits.get().clamp(1, 21)
}

fn clamped_fraction_digit_bounds(options: &super::FormatOptions) -> (u8, u8) {
    let (min_fraction_digits, max_fraction_digits) = normalized_fraction_digit_bounds(options);

    (min_fraction_digits.min(100), max_fraction_digits.min(100))
}

fn browser_supports_unit(unit_id: &str) -> bool {
    let Ok(intl) = Reflect::get(&js_sys::global(), &JsValue::from_str("Intl")) else {
        return false;
    };

    let Ok(supported_values_of) = Reflect::get(&intl, &JsValue::from_str("supportedValuesOf"))
    else {
        return false;
    };

    let Some(supported_values_of) = supported_values_of.dyn_ref::<Function>() else {
        return false;
    };

    let Ok(supported_units) = supported_values_of.call1(&intl, &JsValue::from_str("unit")) else {
        return false;
    };

    Array::from(&supported_units)
        .iter()
        .filter_map(|value| value.as_string())
        .any(|supported| supported == unit_id)
}

fn browser_supports_negative_sign_display() -> bool {
    #[cfg(test)]
    if let Some(supported) = decode_negative_sign_display_support_override(
        NEGATIVE_SIGN_DISPLAY_SUPPORT_OVERRIDE.load(Ordering::Relaxed),
    ) {
        return supported;
    }

    let Ok(intl) = Reflect::get(&js_sys::global(), &JsValue::from_str("Intl")) else {
        return false;
    };

    let Ok(number_format) = Reflect::get(&intl, &JsValue::from_str("NumberFormat")) else {
        return false;
    };

    let Some(number_format) = number_format.dyn_ref::<Function>() else {
        return false;
    };

    let options = Object::new();

    if Reflect::set(
        &options,
        &JsValue::from_str("signDisplay"),
        &JsValue::from_str("negative"),
    )
    .is_err()
    {
        return false;
    }

    let args = Array::new();

    args.push(&Array::new());
    args.push(&options);

    Reflect::construct(number_format, &args).is_ok()
}

#[cfg(test)]
pub(super) fn replace_negative_sign_display_support_override(value: Option<bool>) -> Option<bool> {
    decode_negative_sign_display_support_override(NEGATIVE_SIGN_DISPLAY_SUPPORT_OVERRIDE.swap(
        encode_negative_sign_display_support_override(value),
        Ordering::Relaxed,
    ))
}

#[cfg(test)]
const fn encode_negative_sign_display_support_override(value: Option<bool>) -> i8 {
    match value {
        None => -1,
        Some(false) => 0,
        Some(true) => 1,
    }
}

#[cfg(test)]
const fn decode_negative_sign_display_support_override(value: i8) -> Option<bool> {
    match value {
        -1 => None,
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

fn first_char(value: &js_sys::JsString) -> Option<char> {
    String::from(value.clone()).chars().next()
}

impl RoundingMode {
    const fn into_js(self) -> JsRoundingMode {
        match self {
            Self::HalfEven => JsRoundingMode::HalfEven,
            Self::HalfUp => JsRoundingMode::HalfExpand,
            Self::HalfDown => JsRoundingMode::HalfTrunc,
            Self::Ceiling => JsRoundingMode::Ceil,
            Self::Floor => JsRoundingMode::Floor,
            Self::Truncate => JsRoundingMode::Trunc,
        }
    }
}

impl SignDisplay {
    const fn into_js(self) -> Option<JsSignDisplay> {
        match self {
            Self::Auto => Some(JsSignDisplay::Auto),
            Self::Always => Some(JsSignDisplay::Always),
            Self::Never => Some(JsSignDisplay::Never),
            Self::ExceptZero => Some(JsSignDisplay::ExceptZero),
            Self::Negative => None,
        }
    }
}

impl UnitDisplay {
    const fn into_js(self) -> JsUnitDisplay {
        match self {
            Self::Long => JsUnitDisplay::Long,
            Self::Short => JsUnitDisplay::Short,
            Self::Narrow => JsUnitDisplay::Narrow,
        }
    }
}
