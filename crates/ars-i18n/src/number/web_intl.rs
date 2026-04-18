//! Browser-backed `NumberFormatter` helpers for `web-intl` builds on `wasm32`.

use alloc::{string::String, vec::Vec};

use js_sys::{
    Array, Function,
    Intl::{
        CurrencyDisplay, CurrencySign, NumberFormat, NumberFormatOptions as JsNumberFormatOptions,
        NumberFormatPart, NumberFormatPartType, NumberFormatStyle, RoundingMode as JsRoundingMode,
        SignDisplay as JsSignDisplay, UnitDisplay as JsUnitDisplay, UseGrouping,
    },
    Reflect,
};
use wasm_bindgen::{JsCast, JsValue};

use super::{
    NumberFormatOptions, NumberStyle, RoundingMode, SignDisplay, UnitDisplay,
    resolve_measure_unit_id,
};
use crate::Locale;

/// Browser-backed formatter for the public `NumberFormatter` API.
#[derive(Clone, Debug)]
pub(super) struct WebIntlNumberFormatter {
    inner: NumberFormat,
}

impl WebIntlNumberFormatter {
    /// Construct a browser-backed formatter using `Intl.NumberFormat`.
    pub(super) fn new(locale: &Locale, options: &NumberFormatOptions) -> Self {
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
    let formatter = WebIntlNumberFormatter::new(locale, &NumberFormatOptions::default());

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

fn build_options(options: &NumberFormatOptions) -> JsNumberFormatOptions {
    let js_options = JsNumberFormatOptions::new();

    js_options.set_minimum_integer_digits(options.min_integer_digits.get());

    js_options.set_minimum_fraction_digits(options.min_fraction_digits);

    js_options.set_maximum_fraction_digits(options.max_fraction_digits);

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
    } else {
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
        NumberStyle::Decimal => {
            js_options.set_style(NumberFormatStyle::Decimal);
        }

        NumberStyle::Percent => {
            js_options.set_style(NumberFormatStyle::Percent);
        }

        NumberStyle::Currency(code) => {
            js_options.set_style(NumberFormatStyle::Currency);

            js_options.set_currency(code.as_str());

            js_options.set_currency_display(CurrencyDisplay::Symbol);

            js_options.set_currency_sign(CurrencySign::Standard);
        }

        NumberStyle::Unit(unit) => {
            let unit_id = resolve_measure_unit_id(unit)
                .expect("unit formatter requires a resolvable CLDR unit id");

            js_options.set_style(NumberFormatStyle::Unit);

            js_options.set_unit(&unit_id);

            js_options.set_unit_display(options.unit_display.into_js());
        }
    }

    js_options
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
