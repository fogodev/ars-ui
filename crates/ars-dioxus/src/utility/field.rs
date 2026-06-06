//! Dioxus Field adapter.

use ars_components::utility::field;
pub use ars_components::utility::field::{InputType, Part, Props};
use ars_core::{AriaAttr, AttrMap, Direction, HtmlAttr};
use ars_forms::validation::Error;
use dioxus::prelude::*;

use crate::{
    as_child::merge_dioxus_attrs, attr_map_to_dioxus_inline_attrs, callbacks, use_machine,
    use_stable_id,
};

#[derive(Clone, Copy)]
struct FieldContext {
    machine: crate::UseMachineReturn<field::Machine>,
}

fn field_context() -> FieldContext {
    try_use_context::<FieldContext>().expect("Field subcomponents must be rendered inside <Field/>")
}

/// Props for the Dioxus [`Field`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct FieldProps {
    /// Optional component instance ID.
    #[props(optional, into)]
    pub id: Option<String>,

    /// Whether the field is required.
    #[props(default = false)]
    pub required: bool,

    /// Whether the field is disabled.
    #[props(default = false)]
    pub disabled: bool,

    /// Whether the field is read-only.
    #[props(default = false)]
    pub readonly: bool,

    /// Whether the field is invalid.
    #[props(default = false)]
    pub invalid: bool,

    /// Field name used to consume matching form-level validation errors.
    #[props(optional, into)]
    pub name: Option<String>,

    /// Field-level validation errors.
    #[props(default)]
    pub errors: Vec<Error>,

    /// Optional text direction override.
    #[props(optional)]
    pub dir: Option<Direction>,

    /// Global HTML attributes forwarded onto the root.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,

    /// Field anatomy children.
    pub children: Element,
}

/// Dioxus Field root component.
#[component]
pub fn Field(props: FieldProps) -> Element {
    let generated_id = use_stable_id("field");
    let id = props.id.unwrap_or(generated_id);
    let form_context = try_use_context::<super::form::FormContext>();
    let fieldset_context = try_use_context::<super::fieldset::InheritedFieldsetContext>();

    let errors = merged_validation_errors(props.errors, props.name.as_deref(), form_context);
    let inherited_disabled = fieldset_context.is_some_and(|ctx| (ctx.disabled)());
    let inherited_readonly = fieldset_context.is_some_and(|ctx| (ctx.readonly)());
    let inherited_invalid = fieldset_context.is_some_and(|ctx| (ctx.invalid)());

    let mut core_props = Props::new()
        .id(&id)
        .required(props.required)
        .disabled(props.disabled || inherited_disabled)
        .readonly(props.readonly || inherited_readonly)
        .invalid(props.invalid || inherited_invalid)
        .errors(errors);

    if let Some(dir) = props.dir {
        core_props = core_props.dir(dir);
    }

    let machine = use_machine::<field::Machine>(core_props);

    use_context_provider(|| FieldContext { machine });

    let component_attrs = machine.derive(|api| attr_map_to_dioxus_inline_attrs(api.root_attrs()));
    let attrs = merge_dioxus_attrs(props.attrs, component_attrs());

    rsx! {
        div { ..attrs,{props.children} }
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
/// Props for the Dioxus [`Label`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct LabelProps {
    /// Label content.
    pub children: Element,
}

/// Dioxus Field label part.
#[component]
pub fn Label(props: LabelProps) -> Element {
    let attrs = field_context()
        .machine
        .derive(|api| attr_map_to_dioxus_inline_attrs(api.label_attrs()))();

    rsx! {
        label { ..attrs,{props.children} }
    }
}

/// Props for the Dioxus [`Input`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct InputProps {
    /// Native input type.
    #[props(optional)]
    pub r#type: Option<InputType>,

    /// Native form field name.
    #[props(optional, into)]
    pub name: Option<String>,

    /// Placeholder text.
    #[props(optional, into)]
    pub placeholder: Option<String>,

    /// Current input value.
    #[props(optional, into)]
    pub value: Option<String>,

    /// Fires with the current value when the native input event runs.
    #[props(optional, into)]
    pub on_value_input: Option<EventHandler<String>>,

    /// Global HTML attributes forwarded onto the input.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,
}

/// Dioxus Field input part.
#[expect(
    unused_qualifications,
    reason = "Dioxus rsx event attributes are reported as unnecessary qualifications"
)]
#[component]
pub fn Input(props: InputProps) -> Element {
    let machine = field_context().machine;

    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "field::Api method items are not lifetime-general enough for derive()."
    )]
    let mut component_attrs = machine.derive(|api| api.input_attrs())();

    machine.with_api_snapshot(|api| add_description_relationship(&mut component_attrs, api));

    apply_input_attrs(
        &mut component_attrs,
        props.r#type,
        props.name,
        props.placeholder,
    );

    let attrs = merge_dioxus_attrs(
        props.attrs,
        attr_map_to_dioxus_inline_attrs(component_attrs),
    );

    if let Some(value) = props.value {
        rsx! {
            input {
                value,
                oninput: move |event| callbacks::emit(props.on_value_input.as_ref(), event.value()),
                ..attrs,
            }
        }
    } else {
        rsx! {
            input {
                oninput: move |event| callbacks::emit(props.on_value_input.as_ref(), event.value()),
                ..attrs,
            }
        }
    }
}

/// Props for the Dioxus [`Description`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct DescriptionProps {
    /// Description content.
    pub children: Element,
}

/// Dioxus Field description part.
#[component]
pub fn Description(props: DescriptionProps) -> Element {
    let machine = field_context().machine;
    let mut registered = use_signal(|| false);

    if !*registered.peek() {
        machine.send.call(field::Event::SetHasDescription(true));
        registered.set(true);
    }

    use_drop(move || {
        machine.send.call(field::Event::SetHasDescription(false));
    });

    let attrs = field_context()
        .machine
        .derive(|api| attr_map_to_dioxus_inline_attrs(api.description_attrs()))();

    rsx! {
        div { ..attrs,{props.children} }
    }
}

/// Props for the Dioxus [`ErrorMessage`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct ErrorMessageProps {
    /// Error message content.
    pub children: Element,
}

/// Dioxus Field error message part.
#[component]
pub fn ErrorMessage(props: ErrorMessageProps) -> Element {
    let attrs = field_context()
        .machine
        .derive(|api| attr_map_to_dioxus_inline_attrs(api.error_message_attrs()))();

    rsx! {
        div { ..attrs,{props.children} }
    }
}

fn apply_input_attrs(
    attrs: &mut AttrMap,
    r#type: Option<InputType>,
    name: Option<String>,
    placeholder: Option<String>,
) {
    if let Some(input_type) = r#type {
        attrs.set(HtmlAttr::Type, input_type.as_str());
    }

    if let Some(name) = name {
        attrs.set(HtmlAttr::Name, name);
    }

    if let Some(placeholder) = placeholder {
        attrs.set(HtmlAttr::Placeholder, placeholder);
    }

    if attrs.contains(&HtmlAttr::Aria(AriaAttr::Disabled)) {
        attrs.set_bool(HtmlAttr::Disabled, true);
    }
}

fn add_description_relationship(attrs: &mut AttrMap, api: &field::Api<'_>) {
    let Some(description_id) = api.description_attrs().take(&HtmlAttr::Id) else {
        return;
    };

    let mut described_by = Vec::new();

    if let Some(description_id) = description_id.materialize_string()
        && !description_id.is_empty()
    {
        described_by.push(description_id);
    }

    if let Some(existing) = attrs.take(&HtmlAttr::Aria(AriaAttr::DescribedBy))
        && let Some(existing) = existing.materialize_string()
        && !existing.is_empty()
    {
        described_by.extend(existing.split_whitespace().map(str::to_owned));
    }

    if !described_by.is_empty() {
        described_by.dedup();
        attrs.set(
            HtmlAttr::Aria(AriaAttr::DescribedBy),
            described_by.join(" "),
        );
    }
}
