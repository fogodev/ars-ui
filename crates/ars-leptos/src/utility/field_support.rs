//! Shared Leptos form-field context support for adapter controls.

use ars_forms::validation::Error;
use leptos::prelude::*;

/// Merged field state inherited from explicit props, Form, and Fieldset.
#[derive(Clone, Copy)]
pub(crate) struct FieldSupport {
    pub(crate) disabled: Signal<bool>,
    pub(crate) invalid: Signal<bool>,
    pub(crate) readonly: Signal<bool>,
    pub(crate) errors: Signal<Vec<Error>>,
}

pub(crate) fn use_field_support(
    disabled: Signal<bool>,
    invalid: Signal<bool>,
    readonly: Signal<bool>,
    errors: Signal<Vec<Error>>,
    name: Option<Oco<'static, str>>,
) -> FieldSupport {
    let form_context = use_context::<super::form::FormContext>();
    let fieldset_context = use_context::<super::fieldset::InheritedFieldsetContext>();

    let merged_errors = Signal::derive(move || {
        merged_validation_errors(errors.get(), name.as_deref(), form_context)
    });

    FieldSupport {
        disabled: Signal::derive(move || {
            disabled.get() || fieldset_context.is_some_and(|ctx| ctx.disabled.get())
        }),
        invalid: Signal::derive(move || {
            invalid.get()
                || fieldset_context.is_some_and(|ctx| ctx.invalid.get())
                || !merged_errors.get().is_empty()
        }),
        readonly: Signal::derive(move || {
            readonly.get() || fieldset_context.is_some_and(|ctx| ctx.readonly.get())
        }),
        errors: merged_errors,
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
        let _ = form_context.machine.context_version.get();

        if let Some(form_errors) = form_context
            .machine
            .service
            .read_value()
            .context()
            .validation_errors
            .get(name)
            && !form_errors.is_empty()
        {
            errors.reserve(form_errors.len());
            for error in form_errors {
                errors.push(error.clone());
            }
        }
    }

    errors
}
