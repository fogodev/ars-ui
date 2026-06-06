//! Leptos Form adapter.

use std::collections::BTreeMap;

use ars_components::utility::form;
pub use ars_components::utility::form::{Part, Props, ValidationBehavior};
use ars_core::{AriaAttr, AttrMap, AttrValue, HtmlAttr};
use ars_forms::validation::Error;
use leptos::{children::TypedChildren, context::Provider, html, prelude::*};

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
    validation_behavior: Signal<ValidationBehavior>,

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
    let form_ref = NodeRef::<html::Form>::new();

    let mut props = Props::new().id(&id);

    if let Some(action) = action {
        props = props.action(action.into_owned());
    }

    if let Some(role) = role {
        props = props.role(role.into_owned());
    }

    let machine = use_machine_with_reactive_props::<form::Machine>(form_props_signal(
        props,
        validation_behavior,
        validation_errors,
    ));

    let attrs = machine.with_api_snapshot(|api| {
        let mut attrs = api.root_attrs();

        crate::merge_consumer_class_prop_into(&mut attrs, class);
        add_dynamic_root_attrs(&mut attrs, machine);

        attr_map_to_leptos_inline_attrs(attrs)
    });

    view! {
        <Provider value=FormContext { machine }>
            <form
                {..attrs}
                node_ref=form_ref
                on:submit:capture=move |event| {
                    if validation_behavior.get_untracked() == ValidationBehavior::Aria
                        || on_submit.is_some()
                    {
                        event.prevent_default();
                    }
                    if validation_behavior.get_untracked() == ValidationBehavior::Aria
                        && !form_is_valid(form_ref)
                    {
                        machine
                            .send
                            .run(
                                form::Event::SetStatusMessage(
                                    Some(String::from("Please correct the highlighted fields.")),
                                ),
                            );
                        return;
                    }
                    machine.send.run(form::Event::Submit);
                    callbacks::call(on_submit.as_ref());
                    machine
                        .send
                        .run(form::Event::SubmitComplete {
                            success: true,
                        });
                }
                on:reset:capture=move |_event| {
                    machine.send.run(form::Event::Reset);
                    callbacks::call(on_reset.as_ref());
                }
            >
                {children.into_inner()()}
                {status_region(machine, None)}
            </form>
        </Provider>
    }
}

fn form_props_signal(
    props: Props,
    validation_behavior: Signal<ValidationBehavior>,
    validation_errors: Signal<BTreeMap<String, Vec<Error>>>,
) -> Signal<Props> {
    Signal::derive(move || {
        props
            .clone()
            .validation_behavior(validation_behavior.get())
            .validation_errors(validation_errors.get())
    })
}

fn add_dynamic_root_attrs(attrs: &mut AttrMap, machine: crate::UseMachineReturn<form::Machine>) {
    let state = machine.derive(|api| {
        api.root_attrs()
            .get(&HtmlAttr::Data("ars-state"))
            .map(str::to_owned)
    });
    let busy = machine.derive(|api| api.root_attrs().contains(&HtmlAttr::Aria(AriaAttr::Busy)));
    let no_validate = machine.derive(|api| api.root_attrs().contains(&HtmlAttr::NoValidate));

    attrs
        .set(
            HtmlAttr::Data("ars-state"),
            AttrValue::reactive_optional(move || state.get()),
        )
        .set(
            HtmlAttr::Aria(AriaAttr::Busy),
            AttrValue::reactive_bool(move || busy.get()),
        )
        .set(
            HtmlAttr::NoValidate,
            AttrValue::reactive_bool(move || no_validate.get()),
        );
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
    let machine = form_context().machine;

    status_region(machine, Some(children.into_inner()().into_any()))
}

fn status_region(
    machine: crate::UseMachineReturn<form::Machine>,
    children: Option<AnyView>,
) -> impl IntoView {
    let status_message = machine.derive(|api| api.status_message().map(str::to_owned));

    let attrs =
        machine.with_api_snapshot(|api| attr_map_to_leptos_inline_attrs(api.status_region_attrs()));

    view! { <div {..attrs}>{children} {status_message}</div> }
}

#[expect(
    clippy::missing_const_for_fn,
    reason = "The wasm implementation reads the live form NodeRef and calls DOM constraint validation."
)]
fn form_is_valid(form_ref: NodeRef<html::Form>) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        form_ref.get().is_none_or(|form| form.check_validity())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = form_ref;
        true
    }
}
