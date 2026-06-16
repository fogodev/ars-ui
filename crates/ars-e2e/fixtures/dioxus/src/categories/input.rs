//! Input category fixture panel.

use ars_dioxus::{I18nRegistries, utility::form::Form};
use ars_dioxus_components::input::checkbox::css::{Checkbox, STYLES as CHECKBOX_STYLES, State};
use dioxus::prelude::*;

/// Registers input-category localized messages.
pub(crate) fn register_messages(_registries: &mut I18nRegistries) {}

/// Input category panel.
#[component]
pub fn InputPanel() -> Element {
    let mut controlled = use_signal(|| State::Indeterminate);
    let mut form_value = use_signal(|| State::Checked);
    let mut form_status = use_signal(String::new);

    rsx! {
        section { id: "dioxus-input-panel", "data-fixture-category": "input",
            style { "{CHECKBOX_STYLES}" }
            h2 { "Checkbox" }
            Checkbox { id: "dioxus-fixture-checkbox-unchecked", name: "unchecked", "Unchecked" }
            Checkbox {
                id: "dioxus-fixture-checkbox-checked",
                default_checked: State::Checked,
                name: "checked",
                "Checked"
            }
            Checkbox {
                id: "dioxus-fixture-checkbox-indeterminate",
                default_checked: State::Indeterminate,
                name: "mixed",
                "Indeterminate"
            }
            Checkbox { id: "dioxus-fixture-checkbox-disabled", disabled: true, "Disabled" }
            Checkbox {
                id: "dioxus-fixture-checkbox-readonly",
                readonly: true,
                default_checked: State::Checked,
                "Readonly"
            }
            Checkbox { id: "dioxus-fixture-checkbox-required", required: true, "Required" }
            Checkbox {
                id: "dioxus-fixture-checkbox-invalid",
                invalid: true,
                description: rsx! { "Additional checkbox help." },
                error_message: rsx! { "Checkbox selection is required." },
                "Invalid"
            }
            Checkbox {
                id: "dioxus-fixture-checkbox-controlled",
                checked: controlled(),
                on_checked_change: move |next| controlled.set(next),
                "Controlled"
            }
            Form {
                id: "dioxus-fixture-checkbox-form",
                on_submit: move |_| {
                    let status = if form_value() == State::Checked {
                        "submitted notifications=email"
                    } else {
                        "submitted notifications=none"
                    };

                    form_status.set(status.to_string());
                },
                on_reset: move |_| {
                    form_value.set(State::Checked);
                    form_status.set("reset notifications=email".to_string());
                },
                Checkbox {
                    id: "dioxus-fixture-checkbox-form-value",
                    name: "notifications",
                    value: "email",
                    checked: form_value(),
                    default_checked: State::Checked,
                    on_checked_change: move |next| form_value.set(next),
                    "Form value"
                }
                button { id: "dioxus-fixture-checkbox-reset", r#type: "reset", "Reset" }
                button { id: "dioxus-fixture-checkbox-submit", r#type: "submit", "Submit" }
                p { id: "dioxus-fixture-checkbox-form-status", "{form_status}" }
            }
        }
    }
}
