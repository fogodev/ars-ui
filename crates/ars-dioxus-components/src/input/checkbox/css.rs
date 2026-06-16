//! CSS-class styled Dioxus Checkbox.

use ars_dioxus::prelude::*;
pub use checkbox::State;

/// Stylesheet for the CSS Checkbox variant.
pub const STYLES: &str = include_str!("checkbox.css");

/// Props for the CSS-styled Dioxus [`Checkbox`] component.
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

/// Dioxus Checkbox component styled with stable CSS classes.
#[component]
pub fn Checkbox(props: CheckboxProps) -> Element {
    let attrs = root_class_attrs(props.attrs, "ars-checkbox");

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
            checkbox::Label { class: "ars-checkbox__label", {props.children} }
            checkbox::Control { class: "ars-checkbox__control",
                checkbox::Indicator { class: "ars-checkbox__indicator" }
            }
            checkbox::HiddenInput {}

            if let Some(description) = props.description {
                checkbox::Description { class: "ars-checkbox__description", {description} }
            }

            if let Some(error_message) = props.error_message {
                checkbox::ErrorMessage { class: "ars-checkbox__error-message", {error_message} }
            }
        }
    }
}
