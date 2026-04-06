//! Form context, submission data, cross-field validation, and focus helpers.

mod context;

pub use context::{AnyValidator, Context, CrossFieldValidator, Data, Mode};
use indexmap::IndexMap;

use crate::field::Descriptors;

/// Find the first invalid field and return its input ID.
///
/// Iterates fields in DOM order (which is registration order in the
/// [`IndexMap`]) and finds the first with a validation error. Returns
/// `None` if all fields are valid or the invalid field has no entry
/// in `field_descriptors`.
///
/// Must be called after all async validators have settled (i.e., after
/// `form_submit::Machine` enters `ValidationFailed` state, not while in
/// `Validating`).
#[must_use]
pub fn first_invalid_field_id(
    form: &Context,
    field_descriptors: &IndexMap<String, Descriptors>,
) -> Option<String> {
    form.fields
        .iter()
        .find(|(_, state)| state.validation.is_err())
        .and_then(|(name, _)| field_descriptors.get(name))
        .map(|d| d.input_id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        field::Value,
        validation::{Error, ErrorCode, Errors},
    };

    fn make_form_with_fields(names: &[&str]) -> Context {
        let mut form = Context::new(Mode::on_submit());
        for name in names {
            form.register(*name, Value::Text(String::new()), None, None);
        }
        form
    }

    fn make_descriptors(form_id: &str, names: &[&str]) -> IndexMap<String, Descriptors> {
        names
            .iter()
            .map(|name| ((*name).to_string(), Descriptors::new(form_id, name)))
            .collect()
    }

    fn set_field_invalid(form: &mut Context, name: &str) {
        if let Some(field) = form.fields.get_mut(name) {
            field.validation = Err(Errors(vec![Error {
                code: ErrorCode::Required,
                message: "required".into(),
            }]));
        }
    }

    #[test]
    fn returns_none_when_all_valid() {
        let form = make_form_with_fields(&["email", "name"]);
        let descriptors = make_descriptors("form", &["email", "name"]);
        assert_eq!(first_invalid_field_id(&form, &descriptors), None);
    }

    #[test]
    fn returns_first_invalid_field_input_id() {
        let mut form = make_form_with_fields(&["email", "name"]);
        set_field_invalid(&mut form, "email");
        let descriptors = make_descriptors("form", &["email", "name"]);
        assert_eq!(
            first_invalid_field_id(&form, &descriptors),
            Some("form-email-input".into())
        );
    }

    #[test]
    fn returns_second_field_when_first_is_valid() {
        let mut form = make_form_with_fields(&["email", "name"]);
        set_field_invalid(&mut form, "name");
        let descriptors = make_descriptors("form", &["email", "name"]);
        assert_eq!(
            first_invalid_field_id(&form, &descriptors),
            Some("form-name-input".into())
        );
    }

    #[test]
    fn returns_none_when_invalid_field_not_in_descriptors() {
        let mut form = make_form_with_fields(&["email", "name"]);
        set_field_invalid(&mut form, "email");
        // Descriptors only have "name", not "email"
        let descriptors = make_descriptors("form", &["name"]);
        assert_eq!(first_invalid_field_id(&form, &descriptors), None);
    }

    #[test]
    fn respects_insertion_order_for_multiple_invalid() {
        let mut form = make_form_with_fields(&["email", "name", "phone"]);
        set_field_invalid(&mut form, "name");
        set_field_invalid(&mut form, "email");
        let descriptors = make_descriptors("form", &["email", "name", "phone"]);
        // email was registered first, so it's first in iteration order
        assert_eq!(
            first_invalid_field_id(&form, &descriptors),
            Some("form-email-input".into())
        );
    }

    #[test]
    fn checks_validation_not_touched() {
        // Field is invalid but NOT touched — function still returns it
        // because it checks validation, not show_error()
        let mut form = make_form_with_fields(&["email"]);
        set_field_invalid(&mut form, "email");
        assert!(!form.fields.get("email").unwrap().touched);
        let descriptors = make_descriptors("form", &["email"]);
        assert_eq!(
            first_invalid_field_id(&form, &descriptors),
            Some("form-email-input".into())
        );
    }
}
