//! Leptos Checkbox adapter.
//!
//! This module renders the framework-agnostic Checkbox machine as Leptos
//! views while preserving tri-state ARIA state, label wiring, and native form
//! participation through the hidden input part.

use ars_components::input::checkbox::Machine;
pub use ars_components::input::checkbox::{Event, Messages, Part, Props, State};
use ars_core::{AriaAttr, AttrMap, AttrValue, HtmlAttr};
use ars_forms::validation::Error;
use ars_interactions::KeyboardEventData;
use leptos::{children::TypedChildren, context::Provider, prelude::*};

use crate::{
    apply_part_attrs, attr_map_to_leptos_inline_attrs, event_mapping::leptos_key_to_keyboard_key,
    use_id, use_machine_with_reactive_props,
};

#[derive(Clone, Copy)]
struct CheckboxContext {
    machine: crate::UseMachineReturn<Machine>,
    on_checked_change: Option<Callback<State>>,
    last_pointer: StoredValue<bool>,
}

fn checkbox_context() -> CheckboxContext {
    use_context::<CheckboxContext>()
        .expect("Checkbox subcomponents must be rendered inside <checkbox::Root/>")
}

/// Leptos compound checkbox root.
#[component]
#[expect(
    clippy::too_many_arguments,
    reason = "Component props mirror the Checkbox convenience component and framework builder API."
)]
pub fn Root<T: 'static>(
    /// Optional component instance ID.
    #[prop(optional, into)]
    id: Option<Oco<'static, str>>,

    /// Controlled checked state.
    #[prop(into, default = None)]
    checked: Option<Signal<State>>,

    /// Initial checked state for uncontrolled usage.
    #[prop(optional, default = State::Unchecked)]
    default_checked: State,

    /// Whether the checkbox is disabled.
    #[prop(optional, into)]
    disabled: Signal<bool>,

    /// Whether the checkbox is readonly.
    #[prop(optional, into)]
    readonly: Signal<bool>,

    /// Whether the checkbox is required for form submission.
    #[prop(optional, into)]
    required: Signal<bool>,

    /// Whether the checkbox is invalid.
    #[prop(optional, into)]
    invalid: Signal<bool>,

    /// Validation errors associated with the checkbox.
    #[prop(optional, into)]
    errors: Signal<Vec<Error>>,

    /// Native form field name.
    #[prop(optional, into)]
    name: Option<Oco<'static, str>>,

    /// Submitted value when checked. Defaults to `"on"`.
    #[prop(optional, into, default = Some(Oco::Borrowed("on")))]
    value: Option<Oco<'static, str>>,

    /// Associated native form owner ID.
    #[prop(optional, into)]
    form: Option<Oco<'static, str>>,

    /// Whether a description part is rendered in the initial tree.
    #[prop(optional)]
    has_description: bool,

    /// Whether an error message part is rendered in the initial tree.
    #[prop(optional)]
    has_error_message: bool,

    /// Consumer class tokens appended to the root.
    #[prop(optional, into)]
    class: Option<TextProp>,

    /// Consumer inline style text applied to the root.
    #[prop(optional, into)]
    style: Option<TextProp>,

    /// Fires after user intent requests a new checked state.
    #[prop(optional, into)]
    on_checked_change: Option<Callback<State>>,

    /// Checkbox anatomy children.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let id = id.map_or_else(|| use_id("checkbox"), Oco::into_owned);

    let base_props = build_base_props(id, default_checked, name.clone(), value, form);

    let field_support =
        crate::utility::field_support::use_field_support(disabled, invalid, readonly, errors, name);

    let machine = use_machine_with_reactive_props::<Machine>(Signal::derive(move || {
        let mut props = base_props
            .clone()
            .disabled(field_support.disabled.get())
            .readonly(field_support.readonly.get())
            .required(required.get())
            .invalid(field_support.invalid.get())
            .errors(field_support.errors.get());

        if let Some(checked) = checked {
            props = props.checked(checked.get());
        }

        props
    }));

    machine.send.run(Event::SetHasDescription(has_description));

    machine
        .send
        .run(Event::SetHasErrorMessage(has_error_message));

    let attrs = machine.with_api_snapshot(|api| {
        let mut attrs = api.root_attrs();

        crate::merge_consumer_class_prop_into(&mut attrs, class);

        add_dynamic_root_attrs(&mut attrs, machine);

        let mut attrs = attr_map_to_leptos_inline_attrs(attrs);

        if let Some(style) = crate::consumer_style_prop_to_leptos_attr(style) {
            attrs.push(style);
        }

        attrs
    });

    let last_pointer = StoredValue::new(false);

    view! {
        <Provider value=CheckboxContext {
            machine,
            on_checked_change,
            last_pointer,
        }>
            <div {..attrs}>{children.into_inner()()}</div>
        </Provider>
    }
}

/// Leptos compound checkbox label.
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Api method references are not general enough for part attr callbacks."
)]
#[component]
pub fn Label<T: 'static>(
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
    let attrs = checkbox_context()
        .machine
        .part_attrs(|api| api.label_attrs(), class, style);

    view! { <label {..attrs}>{children.into_inner()()}</label> }
}

/// Leptos compound checkbox control.
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Api method references are not general enough for UseMachineReturn snapshot callbacks."
)]
#[component]
pub fn Control<T: 'static>(
    /// Consumer class tokens appended to the control.
    #[prop(optional, into)]
    class: Option<TextProp>,

    /// Consumer inline style text applied to the control.
    #[prop(optional, into)]
    style: Option<TextProp>,

    /// Control content.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let CheckboxContext {
        machine,
        on_checked_change,
        last_pointer,
    } = checkbox_context();

    let attrs = machine.with_api_snapshot(|api| {
        let mut attrs = api.control_attrs();

        add_dynamic_control_attrs(&mut attrs, machine);

        apply_part_attrs(attrs, class, style)
    });

    view! {
        <div
            {..attrs}
            on:click=move |_| {
                let next = machine.with_api_snapshot(|api| api.next_toggle_state());
                let interactive = machine.with_api_snapshot(|api| api.is_interactive());
                machine.send.run(Event::Toggle);
                if interactive && let Some(callback) = on_checked_change {
                    callback.run(next);
                }
            }
            on:keydown=move |ev| {
                let (key, character) = leptos_key_to_keyboard_key(&ev);
                let data = KeyboardEventData {
                    key,
                    character,
                    code: ev.code(),
                    shift_key: ev.shift_key(),
                    ctrl_key: ev.ctrl_key(),
                    alt_key: ev.alt_key(),
                    meta_key: ev.meta_key(),
                    repeat: ev.repeat(),
                    is_composing: ev.is_composing(),
                };
                if data.key == ars_interactions::KeyboardKey::Space && !data.repeat {
                    ev.prevent_default();
                    let next = machine.with_api_snapshot(|api| api.next_toggle_state());
                    let interactive = machine.with_api_snapshot(|api| api.is_interactive());
                    machine.send.run(Event::Toggle);
                    if interactive && let Some(callback) = on_checked_change {
                        callback.run(next);
                    }
                }
            }
            on:pointerdown=move |_| {
                last_pointer.set_value(true);
            }
            on:focus=move |_| {
                let is_keyboard = !last_pointer.get_value();
                last_pointer.set_value(false);
                machine.send.run(Event::Focus { is_keyboard });
            }
            on:blur=move |_| machine.send.run(Event::Blur)
        >
            {children.into_inner()()}
        </div>
    }
}

/// Leptos compound checkbox indicator.
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Api method references are not general enough for part attr callbacks."
)]
#[component]
pub fn Indicator(
    /// Consumer class tokens appended to the indicator.
    #[prop(optional, into)]
    class: Option<TextProp>,

    /// Consumer inline style text applied to the indicator.
    #[prop(optional, into)]
    style: Option<TextProp>,
) -> impl IntoView {
    let attrs = checkbox_context()
        .machine
        .part_attrs(|api| api.indicator_attrs(), class, style);

    view! { <div {..attrs}></div> }
}

/// Leptos compound checkbox hidden input.
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Api method references are not general enough for UseMachineReturn snapshot callbacks."
)]
#[component]
pub fn HiddenInput(
    /// Consumer class tokens appended to the hidden input.
    #[prop(optional, into)]
    class: Option<TextProp>,

    /// Consumer inline style text applied to the hidden input.
    #[prop(optional, into)]
    style: Option<TextProp>,
) -> impl IntoView {
    let CheckboxContext {
        machine,
        on_checked_change,
        ..
    } = checkbox_context();

    let attrs = machine.with_api_snapshot(|api| {
        let mut attrs = api.hidden_input_attrs();

        add_dynamic_hidden_input_attrs(&mut attrs, machine);

        apply_part_attrs(attrs, class, style)
    });

    view! {
        <input
            {..attrs}
            on:change=move |ev| {
                let checked = event_target_checked(&ev);
                let next = State::from_checked_bool(checked);
                let interactive = machine.with_api_snapshot(|api| api.is_interactive());
                if checked {
                    machine.send.run(Event::Check);
                } else {
                    machine.send.run(Event::Uncheck);
                }
                if interactive && let Some(callback) = on_checked_change {
                    callback.run(next);
                }
            }
        />
    }
}

/// Leptos compound checkbox description.
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Api method references are not general enough for part attr callbacks."
)]
#[component]
pub fn Description<T: 'static>(
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
    let machine = checkbox_context().machine;

    machine.send.run(Event::SetHasDescription(true));

    on_cleanup(move || machine.send.run(Event::SetHasDescription(false)));

    let attrs = machine.part_attrs(|api| api.description_attrs(), class, style);

    view! { <div {..attrs}>{children.into_inner()()}</div> }
}

/// Leptos compound checkbox error message.
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Api method references are not general enough for part attr callbacks."
)]
#[component]
pub fn ErrorMessage<T: 'static>(
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
    let machine = checkbox_context().machine;

    machine.send.run(Event::SetHasErrorMessage(true));

    on_cleanup(move || machine.send.run(Event::SetHasErrorMessage(false)));

    let attrs = machine.part_attrs(|api| api.error_message_attrs(), class, style);

    view! { <div {..attrs}>{children.into_inner()()}</div> }
}

fn build_base_props(
    id: String,
    default_checked: State,
    name: Option<Oco<'static, str>>,
    value: Option<Oco<'static, str>>,
    form: Option<Oco<'static, str>>,
) -> Props {
    let mut props = Props::new().id(id).default_checked(default_checked);

    if let Some(name) = name {
        props = props.name(name);
    }

    if let Some(value) = value {
        props = props.value(value);
    }

    if let Some(form) = form {
        props = props.form(form);
    }

    props
}

#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Api method references are not general enough for reactive attr snapshot callbacks."
)]
fn add_dynamic_root_attrs(attrs: &mut AttrMap, machine: crate::UseMachineReturn<Machine>) {
    let state = machine.attr_string_memo(|api| api.root_attrs(), HtmlAttr::Data("ars-state"));
    let disabled =
        machine.attr_presence_memo(|api| api.root_attrs(), HtmlAttr::Data("ars-disabled"));
    let invalid = machine.attr_presence_memo(|api| api.root_attrs(), HtmlAttr::Data("ars-invalid"));
    let readonly =
        machine.attr_presence_memo(|api| api.root_attrs(), HtmlAttr::Data("ars-readonly"));
    let focus_visible =
        machine.attr_presence_memo(|api| api.root_attrs(), HtmlAttr::Data("ars-focus-visible"));

    attrs
        .set(
            HtmlAttr::Data("ars-state"),
            AttrValue::reactive(move || state.get()),
        )
        .set(
            HtmlAttr::Data("ars-disabled"),
            AttrValue::reactive_bool(move || disabled.get()),
        )
        .set(
            HtmlAttr::Data("ars-invalid"),
            AttrValue::reactive_bool(move || invalid.get()),
        )
        .set(
            HtmlAttr::Data("ars-readonly"),
            AttrValue::reactive_bool(move || readonly.get()),
        )
        .set(
            HtmlAttr::Data("ars-focus-visible"),
            AttrValue::reactive_bool(move || focus_visible.get()),
        );
}

#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Api method references are not general enough for reactive attr snapshot callbacks."
)]
fn add_dynamic_control_attrs(attrs: &mut AttrMap, machine: crate::UseMachineReturn<Machine>) {
    let checked =
        machine.attr_string_memo(|api| api.control_attrs(), HtmlAttr::Aria(AriaAttr::Checked));
    let required = machine.attr_presence_memo(
        |api| api.control_attrs(),
        HtmlAttr::Aria(AriaAttr::Required),
    );
    let invalid =
        machine.attr_presence_memo(|api| api.control_attrs(), HtmlAttr::Aria(AriaAttr::Invalid));
    let errormessage = machine.attr_optional_string_memo(
        |api| api.control_attrs(),
        HtmlAttr::Aria(AriaAttr::ErrorMessage),
    );
    let disabled = machine.attr_presence_memo(
        |api| api.control_attrs(),
        HtmlAttr::Aria(AriaAttr::Disabled),
    );
    let readonly = machine.attr_presence_memo(
        |api| api.control_attrs(),
        HtmlAttr::Aria(AriaAttr::ReadOnly),
    );
    let describedby = machine.attr_optional_string_memo(
        |api| api.control_attrs(),
        HtmlAttr::Aria(AriaAttr::DescribedBy),
    );

    attrs
        .set(
            HtmlAttr::Aria(AriaAttr::Checked),
            AttrValue::reactive(move || checked.get()),
        )
        .set(
            HtmlAttr::Aria(AriaAttr::Required),
            AttrValue::reactive_bool(move || required.get()),
        )
        .set(
            HtmlAttr::Aria(AriaAttr::Invalid),
            AttrValue::reactive_bool(move || invalid.get()),
        )
        .set(
            HtmlAttr::Aria(AriaAttr::ErrorMessage),
            AttrValue::reactive_optional(move || errormessage.get()),
        )
        .set(
            HtmlAttr::Aria(AriaAttr::Disabled),
            AttrValue::reactive_bool(move || disabled.get()),
        )
        .set(
            HtmlAttr::Aria(AriaAttr::ReadOnly),
            AttrValue::reactive_bool(move || readonly.get()),
        )
        .set(
            HtmlAttr::Aria(AriaAttr::DescribedBy),
            AttrValue::reactive_optional(move || describedby.get()),
        );
}

#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Api method references are not general enough for reactive attr snapshot callbacks."
)]
fn add_dynamic_hidden_input_attrs(attrs: &mut AttrMap, machine: crate::UseMachineReturn<Machine>) {
    let checked = machine.attr_presence_memo(|api| api.hidden_input_attrs(), HtmlAttr::Checked);
    let disabled = machine.attr_presence_memo(|api| api.hidden_input_attrs(), HtmlAttr::Disabled);
    let required = machine.attr_presence_memo(|api| api.hidden_input_attrs(), HtmlAttr::Required);

    attrs
        .set(
            HtmlAttr::Checked,
            AttrValue::reactive_bool(move || checked.get()),
        )
        .set(
            HtmlAttr::Disabled,
            AttrValue::reactive_bool(move || disabled.get()),
        )
        .set(
            HtmlAttr::Required,
            AttrValue::reactive_bool(move || required.get()),
        );
}
