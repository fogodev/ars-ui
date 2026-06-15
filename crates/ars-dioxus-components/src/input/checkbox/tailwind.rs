//! Tailwind styled Dioxus Checkbox.

use ars_dioxus::prelude::*;
pub use checkbox::State;

/// Props for the Tailwind-styled Dioxus [`Checkbox`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct CheckboxProps {
    /// Optional component instance ID. When absent, the adapter generates one.
    #[props(optional, into)]
    pub id: Option<String>,

    /// Controlled checked state.
    #[props(optional)]
    pub checked: Option<State>,

    /// Initial checked state for uncontrolled usage.
    #[props(default = State::Unchecked)]
    pub default_checked: State,

    /// Whether the checkbox is disabled.
    #[props(default = false)]
    pub disabled: bool,

    /// Whether the checkbox is readonly.
    #[props(default = false)]
    pub readonly: bool,

    /// Whether the checkbox is required for form submission.
    #[props(default = false)]
    pub required: bool,

    /// Whether the checkbox is invalid.
    #[props(default = false)]
    pub invalid: bool,

    /// Validation errors associated with the checkbox.
    #[props(default)]
    pub errors: Vec<ValidationError>,

    /// Native form field name.
    #[props(optional, into)]
    pub name: Option<String>,

    /// Submitted value when checked. Defaults to `"on"`.
    #[props(optional, into)]
    pub value: Option<String>,

    /// Associated native form owner ID.
    #[props(optional, into)]
    pub form: Option<String>,

    /// Optional descriptive content.
    #[props(optional, into)]
    pub description: Option<Element>,

    /// Optional validation error content.
    #[props(optional, into)]
    pub error_message: Option<Element>,

    /// Fires after user intent requests a new checked state.
    #[props(optional, into)]
    pub on_checked_change: Option<EventHandler<State>>,

    /// Global HTML attributes forwarded onto the rendered root.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,

    /// Visible label content.
    pub children: Element,
}

/// Dioxus Checkbox component styled with Tailwind utility classes.
#[component]
pub fn Checkbox(props: CheckboxProps) -> Element {
    let attrs = root_class_attrs(
        props.attrs,
        "group my-2 grid grid-cols-[1.125rem_minmax(0,1fr)] items-center gap-x-2.5 gap-y-1 data-ars-disabled:opacity-50",
    );

    rsx! {
        checkbox::Root {
            id: props.id,
            checked: props.checked,
            default_checked: props.default_checked,
            disabled: props.disabled,
            readonly: props.readonly,
            required: props.required,
            invalid: props.invalid,
            errors: props.errors,
            name: props.name,
            value: props.value,
            form: props.form,
            has_description: props.description.is_some(),
            has_error_message: props.error_message.is_some(),
            on_checked_change: props.on_checked_change,
            attrs,
            checkbox::Label { class: "col-start-2 cursor-pointer", {props.children} }
            checkbox::Control { class: "col-start-1 row-start-1 box-border inline-flex h-4.5 w-4.5 items-center justify-center rounded border-2 border-slate-500 bg-white text-white group-data-ars-invalid:border-red-600 group-data-ars-disabled:opacity-50 group-data-[ars-state=checked]:border-blue-600 group-data-[ars-state=checked]:bg-blue-600 group-data-[ars-state=indeterminate]:border-blue-600 group-data-[ars-state=indeterminate]:bg-blue-600 group-data-ars-invalid:group-data-[ars-state=checked]:border-red-600 group-data-ars-invalid:group-data-[ars-state=checked]:bg-red-600 group-data-ars-invalid:group-data-[ars-state=indeterminate]:border-red-600 group-data-ars-invalid:group-data-[ars-state=indeterminate]:bg-red-600",
                checkbox::Indicator { class: "after:block group-data-[ars-state=checked]:after:h-[0.65rem] group-data-[ars-state=checked]:after:w-[0.35rem] group-data-[ars-state=checked]:after:rotate-45 group-data-[ars-state=checked]:after:-translate-x-px group-data-[ars-state=checked]:after:-translate-y-px group-data-[ars-state=checked]:after:border-b-2 group-data-[ars-state=checked]:after:border-r-2 group-data-[ars-state=checked]:after:border-current group-data-[ars-state=indeterminate]:after:h-0.5 group-data-[ars-state=indeterminate]:after:w-[0.65rem] group-data-[ars-state=indeterminate]:after:rounded-full group-data-[ars-state=indeterminate]:after:bg-current" }
            }
            checkbox::HiddenInput {}
            if let Some(description) = props.description {
                checkbox::Description { class: "col-start-2 text-[0.9rem]", {description} }
            }
            if let Some(error_message) = props.error_message {
                checkbox::ErrorMessage { class: "col-start-2 text-[0.9rem] text-red-700", {error_message} }
            }
        }
    }
}
