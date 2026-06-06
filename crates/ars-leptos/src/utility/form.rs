//! Leptos Form adapter.

use std::collections::BTreeMap;

use ars_components::utility::form;
pub use ars_components::utility::form::{Part, Props, ValidationBehavior};
use ars_forms::validation::Error;
use leptos::{children::TypedChildren, context::Provider, prelude::*};

use crate::{attr_map_to_leptos_inline_attrs, callbacks, use_id, use_machine_with_reactive_props};

#[derive(Clone, Copy)]
pub(crate) struct FormContext {
    pub(crate) machine: crate::UseMachineReturn<form::Machine>,
}

fn form_context() -> FormContext {
    use_context::<FormContext>().expect("Form subcomponents must be rendered inside <Form/>")
}

/// Leptos Form root component.
#[component]
pub fn Form<T: 'static>(
    /// Optional component instance ID.
    #[prop(optional, into)]
    id: Option<Oco<'static, str>>,

    /// URL the browser submits the form to.
    #[prop(optional, into)]
    action: Option<Oco<'static, str>>,

    /// Optional explicit form role.
    #[prop(optional, into)]
    role: Option<Oco<'static, str>>,

    /// Validation display behavior.
    #[prop(optional, into)]
    validation_behavior: Option<ValidationBehavior>,

    /// Validation errors keyed by field name.
    #[prop(optional, into)]
    validation_errors: Signal<BTreeMap<String, Vec<Error>>>,

    /// Consumer class tokens appended to the form.
    #[prop(optional, into)]
    class: Option<TextProp>,

    /// Fires when the form submit event runs.
    #[prop(optional, into)]
    on_submit: Option<Callback<()>>,

    /// Fires when the form reset event runs.
    #[prop(optional, into)]
    on_reset: Option<Callback<()>>,

    /// Form content.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let id = id.map_or_else(|| use_id("form"), Oco::into_owned);

    let mut props = Props::new().id(&id);

    if let Some(action) = action {
        props = props.action(action.into_owned());
    }

    if let Some(role) = role {
        props = props.role(role.into_owned());
    }

    if let Some(validation_behavior) = validation_behavior {
        props = props.validation_behavior(validation_behavior);
    }

    let machine = use_machine_with_reactive_props::<form::Machine>(form_props_signal(
        props,
        validation_errors,
    ));

    let attrs = machine.with_api_snapshot(|api| {
        let mut attrs = api.root_attrs();

        crate::merge_consumer_class_prop_into(&mut attrs, class);

        attr_map_to_leptos_inline_attrs(attrs)
    });

    view! {
        <Provider value=FormContext { machine }>
            <form
                {..attrs}
                on:submit:capture=move |event| {
                    if on_submit.is_some() {
                        event.prevent_default();
                    }
                    machine.send.run(form::Event::Submit);
                    callbacks::call(on_submit.as_ref());
                }
                on:reset:capture=move |_event| {
                    machine.send.run(form::Event::Reset);
                    callbacks::call(on_reset.as_ref());
                }
            >
                {children.into_inner()()}
            </form>
        </Provider>
    }
}

fn form_props_signal(
    props: Props,
    validation_errors: Signal<BTreeMap<String, Vec<Error>>>,
) -> Signal<Props> {
    Signal::derive(move || props.clone().validation_errors(validation_errors.get()))
}

/// Leptos Form status live-region part.
#[component]
pub fn StatusRegion<T>(
    /// Status region content.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let attrs = form_context()
        .machine
        .with_api_snapshot(|api| attr_map_to_leptos_inline_attrs(api.status_region_attrs()));

    view! { <div {..attrs}>{children.into_inner()()}</div> }
}
