//! Native browser constraint-validation mapping.
//!
//! Adapters read browser-specific `ValidityState` and DOM attributes, then pass
//! the framework-neutral facts here so validation precedence stays shared.

use alloc::{collections::BTreeMap, string::String, vec::Vec};

use ars_i18n::Locale;

use super::Error;
use crate::form::Messages;

/// Input type associated with a native browser `typeMismatch` validity flag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeInputType {
    /// Native `<input type="email">`.
    Email,

    /// Native `<input type="url">`.
    Url,

    /// Any other input type that reports `typeMismatch`.
    Other,
}

/// Framework-neutral native browser validity facts for one form control.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NativeValidity {
    /// Whether the browser reports `ValidityState.valueMissing`.
    pub value_missing: bool,

    /// Input type when the browser reports `ValidityState.typeMismatch`.
    pub type_mismatch: Option<NativeInputType>,

    /// Pattern attribute when the browser reports `ValidityState.patternMismatch`.
    pub pattern_mismatch: Option<String>,

    /// `minlength` attribute when the browser reports `ValidityState.tooShort`.
    pub too_short: Option<usize>,

    /// `maxlength` attribute when the browser reports `ValidityState.tooLong`.
    pub too_long: Option<usize>,

    /// `min` attribute when the browser reports `ValidityState.rangeUnderflow`.
    pub range_underflow: Option<f64>,

    /// `max` attribute when the browser reports `ValidityState.rangeOverflow`.
    pub range_overflow: Option<f64>,

    /// `step` attribute when the browser reports `ValidityState.stepMismatch`.
    pub step_mismatch: Option<f64>,
}

impl NativeValidity {
    /// Converts native browser validity facts into the first matching validation error.
    ///
    /// The precedence mirrors browser constraint-validation reporting and the
    /// Field/Form adapter contract: required errors win, followed by type,
    /// pattern, length, range, and step errors. Unknown native validity states
    /// fall back to a generic `native` custom error using the pattern message.
    #[must_use]
    pub fn to_error(&self, messages: &Messages, locale: &Locale) -> Error {
        if self.value_missing {
            return Error::required(messages, locale);
        }

        if let Some(input_type) = self.type_mismatch {
            return match input_type {
                NativeInputType::Email => Error::email(messages, locale),
                NativeInputType::Url => Error::url(messages, locale),
                NativeInputType::Other => Error::custom("native", (messages.pattern_error)(locale)),
            };
        }

        if let Some(pattern) = &self.pattern_mismatch {
            return Error::pattern(pattern.clone(), messages, locale);
        }

        if let Some(min_length) = self.too_short {
            return Error::min_length(min_length, messages, locale);
        }

        if let Some(max_length) = self.too_long {
            return Error::max_length(max_length, messages, locale);
        }

        if let Some(min) = self.range_underflow {
            return Error::min(min, messages, locale);
        }

        if let Some(max) = self.range_overflow {
            return Error::max(max, messages, locale);
        }

        if let Some(step) = self.step_mismatch {
            return Error::step(step, messages, locale);
        }

        Error::custom("native", (messages.pattern_error)(locale))
    }
}

/// Merges additional field errors into an existing name-keyed error map.
pub fn merge_error_map(
    errors: &mut BTreeMap<String, Vec<Error>>,
    additional_errors: BTreeMap<String, Vec<Error>>,
) {
    additional_errors
        .into_iter()
        .for_each(|(name, mut field_errors)| {
            errors.entry(name).or_default().append(&mut field_errors);
        });
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeMap, string::ToString, vec};

    use ars_i18n::locales;

    use super::*;
    use crate::validation::ErrorCode;

    #[test]
    fn native_validity_prefers_required_over_email_type_mismatch() {
        let validity = NativeValidity {
            value_missing: true,
            type_mismatch: Some(NativeInputType::Email),
            ..NativeValidity::default()
        };

        assert_eq!(
            validity.to_error(&Messages::default(), &locales::en()).code,
            ErrorCode::Required
        );
    }

    #[test]
    fn native_validity_maps_typed_mismatches() {
        let messages = Messages::default();
        let locale = locales::en();

        assert_eq!(
            NativeValidity {
                type_mismatch: Some(NativeInputType::Email),
                ..NativeValidity::default()
            }
            .to_error(&messages, &locale)
            .code,
            ErrorCode::Email
        );

        assert_eq!(
            NativeValidity {
                type_mismatch: Some(NativeInputType::Url),
                ..NativeValidity::default()
            }
            .to_error(&messages, &locale)
            .code,
            ErrorCode::Url
        );
    }

    #[test]
    fn native_validity_maps_other_type_mismatch_to_native_error() {
        let error = NativeValidity {
            type_mismatch: Some(NativeInputType::Other),
            ..NativeValidity::default()
        }
        .to_error(&Messages::default(), &locales::en());

        assert_eq!(error.code, ErrorCode::Custom("native".to_string()));
    }

    #[test]
    fn native_validity_maps_pattern_length_range_and_step() {
        let messages = Messages::default();
        let locale = locales::en();

        for (validity, expected) in [
            (
                NativeValidity {
                    pattern_mismatch: Some("[a-z]+".to_string()),
                    ..NativeValidity::default()
                },
                ErrorCode::Pattern("[a-z]+".to_string()),
            ),
            (
                NativeValidity {
                    too_short: Some(3),
                    ..NativeValidity::default()
                },
                ErrorCode::MinLength(3),
            ),
            (
                NativeValidity {
                    too_long: Some(12),
                    ..NativeValidity::default()
                },
                ErrorCode::MaxLength(12),
            ),
            (
                NativeValidity {
                    range_underflow: Some(2.5),
                    ..NativeValidity::default()
                },
                ErrorCode::Min(2.5),
            ),
            (
                NativeValidity {
                    range_overflow: Some(9.5),
                    ..NativeValidity::default()
                },
                ErrorCode::Max(9.5),
            ),
            (
                NativeValidity {
                    step_mismatch: Some(0.25),
                    ..NativeValidity::default()
                },
                ErrorCode::Step(0.25),
            ),
        ] {
            assert_eq!(validity.to_error(&messages, &locale).code, expected);
        }
    }

    #[test]
    fn native_validity_falls_back_to_native_custom_error() {
        let error = NativeValidity::default().to_error(&Messages::default(), &locales::en());

        assert_eq!(error.code, ErrorCode::Custom("native".to_string()));
    }

    #[test]
    fn merge_error_map_appends_errors_by_field_name() {
        let mut errors = BTreeMap::from([(
            "email".to_string(),
            vec![Error::server("Already registered")],
        )]);
        let additional_errors = BTreeMap::from([
            (
                "email".to_string(),
                vec![Error::custom("native", "Invalid")],
            ),
            (
                "name".to_string(),
                vec![Error::required(&Messages::default(), &locales::en())],
            ),
        ]);

        merge_error_map(&mut errors, additional_errors);

        assert_eq!(errors["email"].len(), 2);
        assert_eq!(errors["name"].len(), 1);
    }
}
