//! Field association IDs for ARIA.
//!
//! [`Descriptors`] generates the element IDs needed to link a form field's
//! label, input, description, and error elements via ARIA attributes.
//! [`InputAria`] bundles the computed ARIA attributes for an input element.
//!
//! **Note:** `Descriptors` is a lower-level utility for manual ARIA wiring.
//! When using the Field component (§13), `Descriptors` is not needed —
//! the Field component handles all ID generation and ARIA linkage internally.

use super::state::State;

/// IDs for all elements of a form field.
///
/// Generated from a form ID and field name, these IDs are used to link
/// label, input, description, and error elements via ARIA attributes.
#[derive(Clone, Debug)]
pub struct Descriptors {
    /// The root element ID (`{form_id}-{field_name}`).
    pub root_id: String,

    /// The label element ID (`{form_id}-{field_name}-label`).
    pub label_id: String,

    /// The input element ID (`{form_id}-{field_name}-input`).
    pub input_id: String,

    /// The description element ID (`{form_id}-{field_name}-desc`).
    pub description_id: String,

    /// The error message element ID (`{form_id}-{field_name}-error`).
    pub error_id: String,
}

impl Descriptors {
    /// Generate IDs from a form ID and field name.
    #[must_use]
    pub fn new(form_id: &str, field_name: &str) -> Self {
        let base = format!("{form_id}-{field_name}");
        Self {
            root_id: base.clone(),
            label_id: format!("{base}-label"),
            input_id: format!("{base}-input"),
            description_id: format!("{base}-desc"),
            error_id: format!("{base}-error"),
        }
    }

    /// Compute `aria-describedby` for the input element.
    ///
    /// Includes `description_id` when a description is present; includes
    /// `error_id` only when the field has an error to show.
    #[must_use]
    pub fn aria_describedby(&self, field: &State, has_description: bool) -> Option<String> {
        let mut ids = Vec::new();

        if has_description {
            ids.push(self.description_id.clone());
        }

        if field.show_error() {
            ids.push(self.error_id.clone());
        }

        if ids.is_empty() {
            None
        } else {
            Some(ids.join(" "))
        }
    }

    /// Compute all ARIA attributes for the input element.
    ///
    /// This is the canonical touch-gated ARIA system: `aria-invalid` and
    /// `aria-errormessage` are only set when [`State::show_error()`]
    /// returns `true` (i.e., the field is both touched and invalid).
    ///
    /// When `aria_invalid` is `true`, this sets BOTH `aria-describedby`
    /// (general description + error message ID) AND `aria-errormessage`
    /// (error-message ID only). `aria-errormessage` is the WAI-ARIA 1.2
    /// recommended attribute for pointing to the error message element.
    /// `aria-describedby` is retained for backwards compatibility with
    /// older assistive technologies.
    #[must_use]
    pub fn input_aria(&self, field: &State, required: bool, has_description: bool) -> InputAria {
        let aria_errormessage = if field.show_error() {
            Some(self.error_id.clone())
        } else {
            None
        };

        InputAria {
            id: self.input_id.clone(),
            aria_labelledby: self.label_id.clone(),
            aria_describedby: self.aria_describedby(field, has_description),
            aria_invalid: if field.show_error() { Some(true) } else { None },
            aria_required: if required { Some(true) } else { None },
            aria_busy: if field.validating { Some(true) } else { None },
            aria_errormessage,
        }
    }
}

/// ARIA attributes to spread onto an input element.
#[derive(Clone, Debug)]
pub struct InputAria {
    /// The `id` attribute for the input element.
    pub id: String,

    /// The `aria-labelledby` attribute pointing to the label element.
    pub aria_labelledby: String,

    /// The `aria-describedby` attribute pointing to description and/or error elements.
    pub aria_describedby: Option<String>,

    /// The `aria-invalid` attribute (set only when touched and invalid).
    pub aria_invalid: Option<bool>,

    /// The `aria-required` attribute.
    pub aria_required: Option<bool>,

    /// The `aria-busy` attribute (set when async validation is running).
    pub aria_busy: Option<bool>,

    /// WAI-ARIA 1.2 §5.2.7.5: points to the error message element when invalid.
    ///
    /// Set only when `aria_invalid` is `true`. Assistive technologies that
    /// support `aria-errormessage` use this instead of `aria-describedby`
    /// for error announcements.
    pub aria_errormessage: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        field::Value,
        validation::{Error, ErrorCode, Errors},
    };

    fn make_descriptors() -> Descriptors {
        Descriptors::new("signup", "email")
    }

    fn make_clean_field() -> State {
        State::new(Value::Text(String::new()))
    }

    fn make_touched_invalid_field() -> State {
        let mut field = make_clean_field();

        field.touched = true;
        field.validation = Err(Errors(vec![Error {
            code: ErrorCode::Required,
            message: "required".to_string(),
        }]));

        field
    }

    #[test]
    fn generates_correct_ids() {
        let d = make_descriptors();

        assert_eq!(d.root_id, "signup-email");
        assert_eq!(d.label_id, "signup-email-label");
        assert_eq!(d.input_id, "signup-email-input");
        assert_eq!(d.description_id, "signup-email-desc");
        assert_eq!(d.error_id, "signup-email-error");
    }

    #[test]
    fn aria_describedby_with_description_and_error() {
        let d = make_descriptors();

        let field = make_touched_invalid_field();

        let result = d.aria_describedby(&field, true);

        assert_eq!(
            result,
            Some("signup-email-desc signup-email-error".to_string())
        );
    }

    #[test]
    fn aria_describedby_with_description_only() {
        let d = make_descriptors();

        let field = make_clean_field();

        let result = d.aria_describedby(&field, true);

        assert_eq!(result, Some("signup-email-desc".to_string()));
    }

    #[test]
    fn aria_describedby_with_error_only() {
        let d = make_descriptors();

        let field = make_touched_invalid_field();

        let result = d.aria_describedby(&field, false);

        assert_eq!(result, Some("signup-email-error".to_string()));
    }

    #[test]
    fn aria_describedby_none_when_empty() {
        let d = make_descriptors();

        let field = make_clean_field();

        let result = d.aria_describedby(&field, false);

        assert_eq!(result, None);
    }

    #[test]
    fn input_aria_invalid_only_when_touched() {
        let d = make_descriptors();

        // Not touched, invalid → aria_invalid is None
        let mut field = make_clean_field();

        field.validation = Err(Errors(vec![Error {
            code: ErrorCode::Required,
            message: "required".to_string(),
        }]));

        let aria = d.input_aria(&field, false, false);

        assert!(aria.aria_invalid.is_none());
        assert!(aria.aria_errormessage.is_none());

        // Touched and invalid → aria_invalid is Some(true)
        let field = make_touched_invalid_field();

        let aria = d.input_aria(&field, false, false);

        assert_eq!(aria.aria_invalid, Some(true));
        assert_eq!(
            aria.aria_errormessage,
            Some("signup-email-error".to_string())
        );
    }

    #[test]
    fn input_aria_required() {
        let d = make_descriptors();

        let field = make_clean_field();

        let aria = d.input_aria(&field, true, false);

        assert_eq!(aria.aria_required, Some(true));

        let aria = d.input_aria(&field, false, false);

        assert!(aria.aria_required.is_none());
    }

    #[test]
    fn input_aria_busy_when_validating() {
        let d = make_descriptors();

        let mut field = make_clean_field();

        field.validating = true;

        let aria = d.input_aria(&field, false, false);

        assert_eq!(aria.aria_busy, Some(true));
    }

    #[test]
    fn input_aria_busy_none_when_not_validating() {
        let d = make_descriptors();

        let field = make_clean_field();

        let aria = d.input_aria(&field, false, false);

        assert!(aria.aria_busy.is_none());
    }

    #[test]
    fn input_aria_required_and_invalid_simultaneously() {
        let d = make_descriptors();

        let field = make_touched_invalid_field();

        let aria = d.input_aria(&field, true, true);

        assert_eq!(aria.aria_required, Some(true));
        assert_eq!(aria.aria_invalid, Some(true));
        assert_eq!(
            aria.aria_errormessage,
            Some("signup-email-error".to_string())
        );
        assert_eq!(
            aria.aria_describedby,
            Some("signup-email-desc signup-email-error".to_string())
        );
    }

    #[test]
    fn input_aria_errormessage_set_when_showing_error() {
        let d = make_descriptors();

        let field = make_touched_invalid_field();

        let aria = d.input_aria(&field, false, true);

        assert_eq!(
            aria.aria_errormessage,
            Some("signup-email-error".to_string())
        );
        // aria-describedby includes both desc and error
        assert_eq!(
            aria.aria_describedby,
            Some("signup-email-desc signup-email-error".to_string())
        );
    }
}
