//! Shared Dioxus form-field context support for adapter controls.

use ars_forms::validation::Error;
use dioxus::prelude::*;

/// Merged field state inherited from explicit props, Form, and Fieldset.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FieldSupport {
    pub(crate) disabled: bool,
    pub(crate) invalid: bool,
    pub(crate) readonly: bool,
    pub(crate) errors: Vec<Error>,
}

pub(crate) fn use_field_support(
    disabled: bool,
    invalid: bool,
    readonly: bool,
    errors: Vec<Error>,
    name: Option<&str>,
) -> FieldSupport {
    let form_context = try_use_context::<super::form::FormContext>();
    let fieldset_context = try_use_context::<super::fieldset::InheritedFieldsetContext>();

    let errors = merged_validation_errors(errors, name, form_context);

    FieldSupport {
        disabled: disabled || fieldset_context.is_some_and(|ctx| (ctx.disabled)()),
        invalid: invalid
            || fieldset_context.is_some_and(|ctx| (ctx.invalid)())
            || !errors.is_empty(),
        readonly: readonly || fieldset_context.is_some_and(|ctx| (ctx.readonly)()),
        errors,
    }
}

fn merged_validation_errors(
    mut errors: Vec<Error>,
    name: Option<&str>,
    form_context: Option<super::form::FormContext>,
) -> Vec<Error> {
    if let Some(name) = name
        && let Some(form_context) = form_context
    {
        let _ = &*form_context.machine.context_version.read();

        if let Some(form_errors) = form_context
            .machine
            .service
            .peek()
            .context()
            .validation_errors
            .get(name)
            && !form_errors.is_empty()
        {
            errors.reserve(form_errors.len());
            form_errors
                .iter()
                .for_each(|error| errors.push(error.clone()));
        }
    }

    errors
}
