//! Input category fixture panel.

use ars_leptos::{I18nRegistries, utility::form::Form};
use ars_leptos_components::input::checkbox::css::{Checkbox, State};
use leptos::prelude::*;

/// Registers input-category localized messages.
pub(crate) fn register_messages(_registries: &mut I18nRegistries) {}

/// Input category panel.
#[component]
pub fn InputPanel() -> impl IntoView {
    let (controlled, set_controlled) = signal(State::Indeterminate);
    let (form_value, set_form_value) = signal(State::Checked);
    let (form_status, set_form_status) = signal(String::new());

    view! {
        <section id="leptos-input-panel" data-fixture-category="input">
            <h2>"Checkbox"</h2>
            <Checkbox id="leptos-fixture-checkbox-unchecked" name="unchecked">
                "Unchecked"
            </Checkbox>
            <Checkbox
                id="leptos-fixture-checkbox-checked"
                default_checked=State::Checked
                name="checked"
            >
                "Checked"
            </Checkbox>
            <Checkbox
                id="leptos-fixture-checkbox-indeterminate"
                default_checked=State::Indeterminate
                name="mixed"
            >
                "Indeterminate"
            </Checkbox>
            <Checkbox id="leptos-fixture-checkbox-disabled" disabled=true>
                "Disabled"
            </Checkbox>
            <Checkbox
                id="leptos-fixture-checkbox-readonly"
                readonly=true
                default_checked=State::Checked
            >
                "Readonly"
            </Checkbox>
            <Checkbox id="leptos-fixture-checkbox-required" required=true>
                "Required"
            </Checkbox>
            <Checkbox
                id="leptos-fixture-checkbox-invalid"
                invalid=true
                description=|| view! { "Additional checkbox help." }
                error_message=|| view! { "Checkbox selection is required." }
            >
                "Invalid"
            </Checkbox>
            <Checkbox
                id="leptos-fixture-checkbox-controlled"
                checked=controlled.into()
                on_checked_change=Callback::new(move |next| set_controlled.set(next))
            >
                "Controlled"
            </Checkbox>
            <Form
                id="leptos-fixture-checkbox-form"
                on_submit=Callback::new(move |_| {
                    let status = if form_value.get() == State::Checked {
                        "submitted notifications=email"
                    } else {
                        "submitted notifications=none"
                    };
                    set_form_status.set(status.to_string());
                })
                on_reset=Callback::new(move |_| {
                    set_form_value.set(State::Checked);
                    set_form_status.set("reset notifications=email".to_string());
                })
            >
                <Checkbox
                    id="leptos-fixture-checkbox-form-value"
                    name="notifications"
                    value="email"
                    checked=form_value.into()
                    on_checked_change=Callback::new(move |next| set_form_value.set(next))
                >
                    "Form value"
                </Checkbox>
                <button type="reset" id="leptos-fixture-checkbox-reset">
                    "Reset"
                </button>
                <button type="submit" id="leptos-fixture-checkbox-submit">
                    "Submit"
                </button>
                <p id="leptos-fixture-checkbox-form-status">{move || form_status.get()}</p>
            </Form>
        </section>
    }
}
