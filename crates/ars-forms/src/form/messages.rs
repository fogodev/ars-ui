//! Localizable messages for form submission and validation errors.
//!
//! This module defines [`Messages`], the form-domain i18n bundle used by
//! validators, form submission announcements, and adapter-owned messaging
//! helpers.
//!
//! It is intentionally distinct from the form-machine `Messages` type in
//! `ars_components::utility::form`. Adapters resolve [`Messages`] separately
//! and use it when formatting validation and status text around the machine.

use alloc::{format, string::String};

use ars_core::{ComponentMessages, MessageFn};
use ars_i18n::Locale;

type LocaleMessage = dyn Fn(&Locale) -> String + Send + Sync;
type CountLocaleMessage = dyn Fn(usize, &Locale) -> String + Send + Sync;
type FloatLocaleMessage = dyn Fn(f64, &Locale) -> String + Send + Sync;

/// Localizable messages for form submission announcements and validator errors.
///
/// This type follows the shared `ComponentMessages` pattern so adapters can
/// provide a single locale-aware message bundle to all form-related logic in a
/// subtree while keeping English defaults available for zero-config usage.
///
/// This is a domain-level message bundle, not the associated `Machine::Messages`
/// type for `ars_components::utility::form::Machine`. The form machine keeps
/// `type Messages = ()`; adapters and validator helpers consume [`Messages`]
/// separately when generating localized strings.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Message announced via a status region on successful submission.
    pub submit_success: MessageFn<LocaleMessage>,

    /// Message announced via a status region when validation fails.
    ///
    /// Receives the number of errors found. Production adapters should use
    /// locale-aware plural rules for this message; the built-in default is an
    /// English-only fallback.
    pub submit_error_count: MessageFn<CountLocaleMessage>,

    /// Message used for required-field validation failures.
    pub required_error: MessageFn<LocaleMessage>,

    /// Message used when a value is shorter than the minimum length.
    pub min_length_error: MessageFn<CountLocaleMessage>,

    /// Message used when a value exceeds the maximum length.
    pub max_length_error: MessageFn<CountLocaleMessage>,

    /// Message used when a value does not match the expected pattern.
    pub pattern_error: MessageFn<LocaleMessage>,

    /// Message used when a numeric value is below the minimum.
    pub min_error: MessageFn<FloatLocaleMessage>,

    /// Message used when a numeric value exceeds the maximum.
    pub max_error: MessageFn<FloatLocaleMessage>,

    /// Message used when a value is not a valid email address.
    pub email_error: MessageFn<LocaleMessage>,

    /// Message used when a numeric value does not match the required step.
    pub step_error: MessageFn<FloatLocaleMessage>,

    /// Message used when a value is not a valid URL.
    pub url_error: MessageFn<LocaleMessage>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            submit_success: MessageFn::new(|_locale: &Locale| {
                "Form submitted successfully.".into()
            }),

            submit_error_count: MessageFn::new(|count: usize, _locale: &Locale| {
                if count == 1 {
                    "1 error found. Please correct the highlighted field.".into()
                } else {
                    format!("{count} errors found. Please correct the highlighted fields.")
                }
            }),

            required_error: MessageFn::new(|_locale: &Locale| "This field is required".into()),

            min_length_error: MessageFn::new(|min: usize, _locale: &Locale| {
                format!("Must be at least {min} characters")
            }),

            max_length_error: MessageFn::new(|max: usize, _locale: &Locale| {
                format!("Must be at most {max} characters")
            }),

            pattern_error: MessageFn::new(|_locale: &Locale| "Invalid format".into()),

            min_error: MessageFn::new(|min: f64, _locale: &Locale| {
                format!("Must be at least {min}")
            }),

            max_error: MessageFn::new(|max: f64, _locale: &Locale| {
                format!("Must be at most {max}")
            }),

            email_error: MessageFn::new(|_locale: &Locale| "Must be a valid email address".into()),

            step_error: MessageFn::new(|step: f64, _locale: &Locale| {
                format!(
                    "Please enter a valid value. The nearest allowed value is a multiple of {step}."
                )
            }),

            url_error: MessageFn::new(|_locale: &Locale| "Please enter a valid URL.".into()),
        }
    }
}

impl ComponentMessages for Messages {}

#[cfg(test)]
mod tests {
    use ars_i18n::locales;

    use super::Messages;

    #[test]
    fn form_messages_default_required() {
        let messages = Messages::default();

        assert_eq!(
            (messages.required_error)(&locales::en()),
            "This field is required"
        );
    }

    #[test]
    fn form_messages_default_submit_error_count_singular() {
        let messages = Messages::default();

        assert_eq!(
            (messages.submit_error_count)(1, &locales::en()),
            "1 error found. Please correct the highlighted field."
        );
    }

    #[test]
    fn form_messages_default_submit_error_count_plural() {
        let messages = Messages::default();

        assert_eq!(
            (messages.submit_error_count)(3, &locales::en()),
            "3 errors found. Please correct the highlighted fields."
        );
    }

    #[test]
    fn form_messages_default_submit_success() {
        let messages = Messages::default();

        assert_eq!(
            (messages.submit_success)(&locales::en()),
            "Form submitted successfully."
        );
    }

    #[test]
    fn form_messages_default_max_length() {
        let messages = Messages::default();

        assert_eq!(
            (messages.max_length_error)(8, &locales::en()),
            "Must be at most 8 characters"
        );
    }

    #[test]
    fn form_messages_default_pattern() {
        let messages = Messages::default();

        assert_eq!((messages.pattern_error)(&locales::en()), "Invalid format");
    }

    #[test]
    fn form_messages_default_min() {
        let messages = Messages::default();

        assert_eq!(
            (messages.min_error)(2.5, &locales::en()),
            "Must be at least 2.5"
        );
    }

    #[test]
    fn form_messages_default_max() {
        let messages = Messages::default();

        assert_eq!(
            (messages.max_error)(9.5, &locales::en()),
            "Must be at most 9.5"
        );
    }

    #[test]
    fn form_messages_default_email() {
        let messages = Messages::default();

        assert_eq!(
            (messages.email_error)(&locales::en()),
            "Must be a valid email address"
        );
    }

    #[test]
    fn form_messages_default_step() {
        let messages = Messages::default();

        assert_eq!(
            (messages.step_error)(0.25, &locales::en()),
            "Please enter a valid value. The nearest allowed value is a multiple of 0.25."
        );
    }

    #[test]
    fn form_messages_default_url() {
        let messages = Messages::default();

        assert_eq!(
            (messages.url_error)(&locales::en()),
            "Please enter a valid URL."
        );
    }
}
