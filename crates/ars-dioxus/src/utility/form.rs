//! Dioxus Form adapter.

use std::collections::BTreeMap;

use ars_components::utility::form;
pub use ars_components::utility::form::{Part, Props, ValidationBehavior};
use ars_forms::validation::Error;
use dioxus::prelude::*;

use crate::{
    as_child::merge_dioxus_attrs, attr_map_to_dioxus_inline_attrs, callbacks, use_machine,
    use_stable_id,
};

#[derive(Clone, Copy)]
pub(crate) struct FormContext {
    pub(crate) machine: crate::UseMachineReturn<form::Machine>,
}

fn form_context() -> FormContext {
    try_use_context::<FormContext>().expect("Form subcomponents must be rendered inside <Form/>")
}

/// Props for the Dioxus [`Form`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct FormProps {
    /// Optional component instance ID.
    #[props(optional, into)]
    pub id: Option<String>,

    /// URL the browser submits the form to.
    #[props(optional, into)]
    pub action: Option<String>,

    /// Optional explicit form role.
    #[props(optional, into)]
    pub role: Option<String>,

    /// Validation display behavior.
    #[props(optional)]
    pub validation_behavior: Option<ValidationBehavior>,

    /// Validation errors keyed by field name.
    #[props(default, into)]
    pub validation_errors: BTreeMap<String, Vec<Error>>,

    /// Fires when the form submit event runs.
    #[props(optional, into)]
    pub on_submit: Option<EventHandler>,

    /// Fires when the form reset event runs.
    #[props(optional, into)]
    pub on_reset: Option<EventHandler>,

    /// Global HTML attributes forwarded onto the form.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,

    /// Form content.
    pub children: Element,
}

/// Dioxus Form root component.
#[expect(
    unused_qualifications,
    reason = "Dioxus rsx event attributes are reported as unnecessary qualifications"
)]
#[component]
pub fn Form(props: FormProps) -> Element {
    let generated_id = use_stable_id("form");
    let id = props.id.unwrap_or(generated_id);

    let mut core_props = Props::new().id(&id);

    if let Some(action) = props.action {
        core_props = core_props.action(action);
    }

    if let Some(role) = props.role {
        core_props = core_props.role(role);
    }

    if let Some(validation_behavior) = props.validation_behavior {
        core_props = core_props.validation_behavior(validation_behavior);
    }

    core_props = core_props.validation_errors(props.validation_errors);

    let machine = use_machine::<form::Machine>(core_props);

    use_context_provider(|| FormContext { machine });

    let component_attrs = machine.derive(|api| attr_map_to_dioxus_inline_attrs(api.root_attrs()));
    let attrs = merge_dioxus_attrs(props.attrs, component_attrs());

    rsx! {
        form {
            onsubmit: move |event| {
                if props.on_submit.is_some() {
                    event.prevent_default();
                }

                machine.send.call(form::Event::Submit);
                callbacks::call(props.on_submit.as_ref());
            },
            onreset: move |_event| {
                machine.send.call(form::Event::Reset);
                callbacks::call(props.on_reset.as_ref());
            },
            ..attrs,
            {props.children}
        }
    }
}

/// Props for the Dioxus [`StatusRegion`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct StatusRegionProps {
    /// Status region content.
    pub children: Element,
}

/// Dioxus Form status live-region part.
#[component]
pub fn StatusRegion(props: StatusRegionProps) -> Element {
    let attrs = form_context()
        .machine
        .derive(|api| attr_map_to_dioxus_inline_attrs(api.status_region_attrs()))();

    rsx! {
        div { ..attrs,{props.children} }
    }
}
