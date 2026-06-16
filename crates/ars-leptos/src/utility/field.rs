//! Leptos Field adapter.

use ars_components::utility::field;
pub use ars_components::utility::field::{InputType, Part, Props};
use ars_core::{AriaAttr, AttrMap, AttrValue, Direction, HtmlAttr};
use ars_forms::validation::Error;
use leptos::{children::TypedChildren, context::Provider, either::Either, prelude::*};

use crate::{
    apply_part_attrs, attr_map_to_leptos_inline_attrs, callbacks, use_id,
    use_machine_with_reactive_props,
};

#[derive(Clone, Copy)]
struct FieldContext {
    machine: crate::UseMachineReturn<field::Machine>,
}

fn field_context() -> FieldContext {
    use_context::<FieldContext>()
        .expect("Field subcomponents must be rendered inside <field::Root/>")
}

/// Leptos Field root component.
#[component]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Leptos component props are owned builder inputs; borrowing the Oco class prop avoids allocating with Oco::into_owned just to satisfy Clippy."
)]
pub fn Root<T: 'static>(
    /// Optional component instance ID.
    #[prop(optional, into)]
    id: Option<Oco<'static, str>>,

    /// Whether the field is required.
    #[prop(optional, into)]
    required: Signal<bool>,

    /// Whether the field is disabled.
    #[prop(optional, into)]
    disabled: Signal<bool>,

    /// Whether the field is read-only.
    #[prop(optional, into)]
    readonly: Signal<bool>,

    /// Whether the field is invalid.
    #[prop(optional, into)]
    invalid: Signal<bool>,

    /// Field name used to consume matching form-level validation errors.
    #[prop(optional, into)]
    name: Option<Oco<'static, str>>,

    /// Field-level validation errors.
    #[prop(optional, into)]
    errors: Signal<Vec<Error>>,

    /// Optional text direction override.
    #[prop(optional)]
    dir: Option<Direction>,

    /// Consumer class tokens appended to the root.
    #[prop(optional, into)]
    class: Option<TextProp>,

    /// Field anatomy children.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let id = id.map_or_else(|| use_id("field"), Oco::into_owned);

    let mut props = Props::new().id(id);

    if let Some(dir) = dir {
        props = props.dir(dir);
    }

    let machine = use_machine_with_reactive_props::<field::Machine>(field_props_signal(
        props,
        FieldReactiveProps {
            required,
            disabled,
            readonly,
            invalid,
            errors,
            name,
        },
    ));

    let attrs = machine.with_api_snapshot(|api| {
        let mut attrs = api.root_attrs();

        crate::merge_consumer_class_prop_into(&mut attrs, class.clone());

        add_dynamic_root_attrs(&mut attrs, machine);

        attr_map_to_leptos_inline_attrs(attrs)
    });

    view! {
        <Provider value=FieldContext { machine }>
            <div {..attrs}>{children.into_inner()()}</div>
        </Provider>
    }
}

struct FieldReactiveProps {
    required: Signal<bool>,
    disabled: Signal<bool>,
    readonly: Signal<bool>,
    invalid: Signal<bool>,
    errors: Signal<Vec<Error>>,
    name: Option<Oco<'static, str>>,
}

fn field_props_signal(
    props: Props,
    FieldReactiveProps {
        required,
        disabled,
        readonly,
        invalid,
        errors,
        name,
    }: FieldReactiveProps,
) -> Signal<Props> {
    let support =
        super::field_support::use_field_support(disabled, invalid, readonly, errors, name);

    Signal::derive(move || {
        props
            .clone()
            .required(required.get())
            .disabled(support.disabled.get())
            .readonly(support.readonly.get())
            .invalid(support.invalid.get())
            .errors(support.errors.get())
    })
}

/// Leptos Field label part.
#[component]
pub fn Label<T>(
    /// Consumer class tokens appended to the label.
    #[prop(optional, into)]
    class: Option<TextProp>,

    /// Consumer inline style text applied to the label.
    #[prop(optional, into)]
    style: Option<TextProp>,

    /// Label content.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let attrs = field_context()
        .machine
        .with_api_snapshot(|api| apply_part_attrs(api.label_attrs(), class, style));

    view! { <label {..attrs}>{children.into_inner()()}</label> }
}

/// Leptos Field input part.
#[component]
pub fn Input(
    /// Native input type.
    #[prop(optional, into)]
    r#type: Option<Signal<InputType>>,

    /// Native form field name.
    #[prop(optional, into)]
    name: Option<Oco<'static, str>>,

    /// Placeholder text.
    #[prop(optional, into)]
    placeholder: Option<TextProp>,

    /// Current input value.
    #[prop(optional, into)]
    value: Option<Signal<String>>,

    /// Consumer class tokens appended to the input.
    #[prop(optional, into)]
    class: Option<TextProp>,

    /// Consumer inline style text applied to the input.
    #[prop(optional, into)]
    style: Option<TextProp>,

    /// Fires with the current value when the native input event runs.
    #[prop(optional)]
    on_value_input: Option<Callback<String>>,
) -> impl IntoView {
    let machine = field_context().machine;

    let attrs = machine.with_api_snapshot(|api| {
        let mut attrs = api.input_attrs();

        add_dynamic_input_attrs(&mut attrs, machine);

        apply_input_attrs(&mut attrs, r#type, name, placeholder);

        apply_part_attrs(attrs, class, style)
    });

    if let Some(value) = value {
        Either::Left(view! {
            <input
                {..attrs}
                prop:value=move || value.get()
                on:input:target=move |event| {
                    callbacks::emit(on_value_input.as_ref(), event.target().value());
                }
            />
        })
    } else {
        Either::Right(view! {
            <input
                {..attrs}
                on:input:target=move |event| {
                    callbacks::emit(on_value_input.as_ref(), event.target().value());
                }
            />
        })
    }
}

/// Leptos Field description part.
#[component]
pub fn Description<T>(
    /// Consumer class tokens appended to the description.
    #[prop(optional, into)]
    class: Option<TextProp>,

    /// Consumer inline style text applied to the description.
    #[prop(optional, into)]
    style: Option<TextProp>,

    /// Description content.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let machine = field_context().machine;

    machine.send.run(field::Event::SetHasDescription(true));

    on_cleanup(move || machine.send.run(field::Event::SetHasDescription(false)));

    let attrs =
        machine.with_api_snapshot(|api| apply_part_attrs(api.description_attrs(), class, style));

    view! { <div {..attrs}>{children.into_inner()()}</div> }
}

/// Leptos Field error message part.
#[component]
pub fn ErrorMessage<T>(
    /// Consumer class tokens appended to the error message.
    #[prop(optional, into)]
    class: Option<TextProp>,

    /// Consumer inline style text applied to the error message.
    #[prop(optional, into)]
    style: Option<TextProp>,

    /// Error message content.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let machine = field_context().machine;

    let hidden = machine.derive(|api| api.error_message_attrs().contains(&HtmlAttr::Hidden));

    let attrs = machine.with_api_snapshot(|api| {
        let mut attrs = api.error_message_attrs();

        attrs.set(
            HtmlAttr::Hidden,
            AttrValue::reactive_bool(move || hidden.get()),
        );

        apply_part_attrs(attrs, class, style)
    });

    view! { <div {..attrs}>{children.into_inner()()}</div> }
}

#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Api method references are not general enough for part attr callbacks."
)]
fn add_dynamic_input_attrs(attrs: &mut AttrMap, machine: crate::UseMachineReturn<field::Machine>) {
    let described_by = machine.attr_optional_string_memo(
        |api| api.input_attrs(),
        HtmlAttr::Aria(AriaAttr::DescribedBy),
    );
    let aria_invalid =
        machine.attr_presence_memo(|api| api.input_attrs(), HtmlAttr::Aria(AriaAttr::Invalid));
    let error_message = machine.attr_optional_string_memo(
        |api| api.input_attrs(),
        HtmlAttr::Aria(AriaAttr::ErrorMessage),
    );
    let aria_required =
        machine.attr_presence_memo(|api| api.input_attrs(), HtmlAttr::Aria(AriaAttr::Required));
    let aria_disabled =
        machine.attr_presence_memo(|api| api.input_attrs(), HtmlAttr::Aria(AriaAttr::Disabled));
    let aria_readonly =
        machine.attr_presence_memo(|api| api.input_attrs(), HtmlAttr::Aria(AriaAttr::ReadOnly));

    attrs
        .set(
            HtmlAttr::Aria(AriaAttr::DescribedBy),
            AttrValue::reactive_optional(move || described_by.get()),
        )
        .set(
            HtmlAttr::Aria(AriaAttr::Invalid),
            AttrValue::reactive_bool(move || aria_invalid.get()),
        )
        .set(
            HtmlAttr::Aria(AriaAttr::ErrorMessage),
            AttrValue::reactive_optional(move || error_message.get()),
        )
        .set(
            HtmlAttr::Aria(AriaAttr::Required),
            AttrValue::reactive_bool(move || aria_required.get()),
        )
        .set(
            HtmlAttr::Required,
            AttrValue::reactive_bool(move || aria_required.get()),
        )
        .set(
            HtmlAttr::Aria(AriaAttr::Disabled),
            AttrValue::reactive_bool(move || aria_disabled.get()),
        )
        .set(
            HtmlAttr::Disabled,
            AttrValue::reactive_bool(move || aria_disabled.get()),
        )
        .set(
            HtmlAttr::Aria(AriaAttr::ReadOnly),
            AttrValue::reactive_bool(move || aria_readonly.get()),
        )
        .set(
            HtmlAttr::ReadOnly,
            AttrValue::reactive_bool(move || aria_readonly.get()),
        );
}

fn add_dynamic_root_attrs(attrs: &mut AttrMap, machine: crate::UseMachineReturn<field::Machine>) {
    let invalid = machine.derive(|api| api.root_attrs().contains(&HtmlAttr::Data("ars-invalid")));

    attrs.set(
        HtmlAttr::Data("ars-invalid"),
        AttrValue::reactive_bool(move || invalid.get()),
    );
}

fn apply_input_attrs(
    attrs: &mut AttrMap,
    r#type: Option<Signal<InputType>>,
    name: Option<Oco<'static, str>>,
    placeholder: Option<TextProp>,
) {
    if let Some(input_type) = r#type {
        attrs.set(
            HtmlAttr::Type,
            AttrValue::reactive(move || input_type.get().as_str().to_owned()),
        );
    }

    if let Some(name) = name {
        attrs.set(HtmlAttr::Name, name.into_owned());
    }

    if let Some(placeholder) = placeholder {
        attrs.set(
            HtmlAttr::Placeholder,
            AttrValue::reactive(move || placeholder.get().into_owned()),
        );
    }
}
