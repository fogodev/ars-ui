//! Dioxus Form adapter.

use std::{collections::BTreeMap, rc::Rc};

use ars_components::utility::form;
pub use ars_components::utility::form::{Part, Props, ValidationBehavior};
use ars_forms::validation::Error;
use dioxus::{events::MountedData, prelude::*};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use web_sys::wasm_bindgen::{JsCast as _, JsValue};

use crate::{
    attr_map_to_dioxus_inline_attrs, callbacks, merge_dioxus_attrs, use_machine,
    use_messages_and_locale, use_stable_id,
};

#[derive(Clone, Copy)]
pub(crate) struct FormContext {
    pub(crate) machine: crate::UseMachineReturn<form::Machine>,
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

    /// Controlled status text shown in the form live region.
    #[props(optional, into)]
    pub status_message: Option<String>,

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
    let mut form_ref = use_signal(|| None::<Rc<MountedData>>);

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

    core_props = core_props
        .validation_errors(props.validation_errors.clone())
        .maybe_status_message(props.status_message.clone());

    let validation_behavior = core_props.validation_behavior;
    let controlled_validation_errors = core_props.validation_errors.clone();
    let should_prevent_native_submit =
        validation_behavior == ValidationBehavior::Aria || props.on_submit.is_some();

    let machine = use_machine::<form::Machine>(core_props);
    let (form_messages, form_locale) =
        use_messages_and_locale::<ars_forms::form::Messages>(None, None);

    use_context_provider(|| FormContext { machine });

    let component_attrs = machine.derive(|api| attr_map_to_dioxus_inline_attrs(api.root_attrs()))();
    let attrs = strip_form_event_attrs(merge_dioxus_attrs(props.attrs, component_attrs));
    let status_message =
        machine.derive(|api| api.status_message().map(str::to_owned).unwrap_or_default())();
    let status_attrs =
        machine.derive(|api| attr_map_to_dioxus_inline_attrs(api.status_region_attrs()))();

    rsx! {
        form {
            onmounted: move |event| {
                form_ref.set(Some(event.data()));
            },
            onsubmit: move |event| {
                let skip_validation = submitter_skips_validation(&event);

                if should_prevent_native_submit {
                    prevent_native_default(&event);
                    event.prevent_default();
                }

                if validation_behavior == ValidationBehavior::Aria
                    && !skip_validation
                    && !form_is_valid(&event, form_ref())
                {
                    let current_form_ref = form_ref();
                    let mut errors = controlled_validation_errors.clone();
                    let native_errors = invalid_control_errors(
                        &event,
                        current_form_ref,
                        &form_messages,
                        &form_locale,
                    );
                    let error_count = native_errors.values().map(Vec::len).sum::<usize>().max(1);
                    merge_validation_errors(&mut errors, native_errors);
                    machine.send.call(form::Event::SetValidationErrors(errors));
                    machine
                        .send
                        .call(
                            form::Event::SetStatusMessage(
                                Some(
                                    (form_messages.submit_error_count)(error_count, &form_locale),
                                ),
                            ),
                        );
                    return;
                }
                if validation_behavior == ValidationBehavior::Aria {
                    machine
                        .send
                        .call(
                            form::Event::SetValidationErrors(
                                controlled_validation_errors.clone(),
                            ),
                        );
                }
                machine.send.call(form::Event::Submit);
                callbacks::call(props.on_submit.as_ref());
                machine
                    .send
                    .call(form::Event::SubmitComplete {
                        success: true,
                    });
            },
            onreset: move |_event| {
                machine.send.call(form::Event::Reset);
                callbacks::call(props.on_reset.as_ref());
            },
            ..attrs,
            {props.children}
            div { ..status_attrs,{status_message} }
        }
    }
}

fn strip_form_event_attrs(mut attrs: Vec<Attribute>) -> Vec<Attribute> {
    attrs.retain(|attr| !matches!(attr.name, "onsubmit" | "onreset" | "onmounted"));
    attrs
}

#[expect(
    clippy::missing_const_for_fn,
    reason = "the wasm web path downcasts event data and calls Event::prevent_default"
)]
fn prevent_native_default(event: &Event<FormData>) {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        if let Some(event) = event.data().downcast::<web_sys::Event>() {
            event.prevent_default();
        }
    }

    #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
    {
        let _ = event;
    }
}

fn merge_validation_errors(
    errors: &mut BTreeMap<String, Vec<Error>>,
    additional_errors: BTreeMap<String, Vec<Error>>,
) {
    additional_errors
        .into_iter()
        .for_each(|(name, mut field_errors)| {
            errors.entry(name).or_default().append(&mut field_errors);
        });
}

fn form_is_valid(event: &Event<FormData>, form_ref: Option<Rc<MountedData>>) -> bool {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        form_element(event, form_ref).is_none_or(|form| form.check_validity())
    }

    #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
    {
        let _ = event;
        drop(form_ref);
        true
    }
}

fn invalid_control_errors(
    event: &Event<FormData>,
    form_ref: Option<Rc<MountedData>>,
    messages: &ars_forms::form::Messages,
    locale: &ars_i18n::Locale,
) -> BTreeMap<String, Vec<Error>> {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        let mut errors = BTreeMap::new();

        if let Some(form) = form_element(event, form_ref)
            && let Ok(nodes) = form.query_selector_all(":invalid")
        {
            for index in 0..nodes.length() {
                let Some(node) = nodes.item(index) else {
                    continue;
                };

                let Ok(element) = node.dyn_into::<web_sys::Element>() else {
                    continue;
                };

                let Some(name) = element
                    .get_attribute("name")
                    .filter(|name| !name.is_empty())
                else {
                    continue;
                };

                errors
                    .entry(name)
                    .or_insert_with(Vec::new)
                    .push(native_validation_error(&element, messages, locale));
            }
        }

        errors
    }

    #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
    {
        let _ = event;
        drop(form_ref);
        let _ = messages;
        let _ = locale;
        BTreeMap::new()
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn native_validation_error(
    element: &web_sys::Element,
    messages: &ars_forms::form::Messages,
    locale: &ars_i18n::Locale,
) -> Error {
    if validity_flag(element, "valueMissing") {
        return Error::required(messages, locale);
    }

    if validity_flag(element, "typeMismatch")
        && let Some(input_type) = element.get_attribute("type")
    {
        return match input_type.as_str() {
            "email" => Error::email(messages, locale),
            "url" => Error::url(messages, locale),
            _ => Error::custom("native", (messages.pattern_error)(locale)),
        };
    }

    if validity_flag(element, "patternMismatch")
        && let Some(pattern) = element.get_attribute("pattern")
    {
        return Error::pattern(pattern, messages, locale);
    }

    if validity_flag(element, "tooShort")
        && let Some(min_length) = element
            .get_attribute("minlength")
            .and_then(|value| value.parse::<usize>().ok())
    {
        return Error::min_length(min_length, messages, locale);
    }

    if validity_flag(element, "tooLong")
        && let Some(max_length) = element
            .get_attribute("maxlength")
            .and_then(|value| value.parse::<usize>().ok())
    {
        return Error::max_length(max_length, messages, locale);
    }

    if validity_flag(element, "rangeUnderflow")
        && let Some(min) = element
            .get_attribute("min")
            .and_then(|value| value.parse::<f64>().ok())
    {
        return Error::min(min, messages, locale);
    }

    if validity_flag(element, "rangeOverflow")
        && let Some(max) = element
            .get_attribute("max")
            .and_then(|value| value.parse::<f64>().ok())
    {
        return Error::max(max, messages, locale);
    }

    if validity_flag(element, "stepMismatch")
        && let Some(step) = element
            .get_attribute("step")
            .and_then(|value| value.parse::<f64>().ok())
    {
        return Error::step(step, messages, locale);
    }

    Error::custom("native", (messages.pattern_error)(locale))
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn validity_flag(element: &web_sys::Element, flag: &str) -> bool {
    js_sys::Reflect::get(element.as_ref(), &JsValue::from_str("validity"))
        .ok()
        .and_then(|validity| js_sys::Reflect::get(&validity, &JsValue::from_str(flag)).ok())
        .and_then(|flag| flag.as_bool())
        .unwrap_or(false)
}

#[expect(
    clippy::missing_const_for_fn,
    reason = "The wasm implementation reflects on the live submit event's submitter property."
)]
fn submitter_skips_validation(event: &Event<FormData>) -> bool {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        event
            .data()
            .downcast::<web_sys::Event>()
            .and_then(|event| {
                js_sys::Reflect::get(event.as_ref(), &JsValue::from_str("submitter")).ok()
            })
            .and_then(|submitter| submitter.dyn_into::<web_sys::Element>().ok())
            .is_some_and(|submitter| submitter.has_attribute("formnovalidate"))
    }

    #[cfg(not(all(feature = "web", target_arch = "wasm32")))]
    {
        let _ = event;
        false
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn form_element(
    event: &Event<FormData>,
    form_ref: Option<Rc<MountedData>>,
) -> Option<web_sys::HtmlFormElement> {
    event
        .data()
        .downcast::<web_sys::Event>()
        .and_then(web_sys::Event::target)
        .and_then(|target| target.dyn_into::<web_sys::HtmlFormElement>().ok())
        .or_else(|| form_ref.and_then(|form| form.downcast::<web_sys::HtmlFormElement>().cloned()))
}

#[cfg(all(test, feature = "web", target_arch = "wasm32"))]
mod wasm_tests {
    use ars_forms::validation::ErrorCode;

    use super::*;

    fn input_element() -> web_sys::Element {
        web_sys::window()
            .and_then(|window| window.document())
            .expect("document should exist")
            .create_element("input")
            .expect("input element should be created")
    }

    fn messages_and_locale() -> (ars_forms::form::Messages, ars_i18n::Locale) {
        (
            ars_forms::form::Messages::default(),
            ars_i18n::Locale::parse("en-US").expect("test locale should parse"),
        )
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn native_validation_error_prefers_required_for_empty_required_inputs() {
        let element = input_element();
        element
            .set_attribute("required", "")
            .expect("required attribute should set");

        let (messages, locale) = messages_and_locale();

        assert_eq!(
            native_validation_error(&element, &messages, &locale).code,
            ErrorCode::Required
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn native_validation_error_prefers_email_for_nonempty_required_email_inputs() {
        let element = input_element();
        element
            .set_attribute("required", "")
            .expect("required attribute should set");
        element
            .set_attribute("type", "email")
            .expect("type attribute should set");
        js_sys::Reflect::set(
            element.as_ref(),
            &JsValue::from_str("value"),
            &JsValue::from_str("not-an-email"),
        )
        .expect("value property should set");

        let (messages, locale) = messages_and_locale();

        assert_eq!(
            native_validation_error(&element, &messages, &locale).code,
            ErrorCode::Email
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn native_validation_error_uses_range_overflow_before_min_attribute() {
        let element = input_element();
        element
            .set_attribute("type", "number")
            .expect("type attribute should set");
        element
            .set_attribute("min", "0")
            .expect("min attribute should set");
        element
            .set_attribute("max", "10")
            .expect("max attribute should set");
        js_sys::Reflect::set(
            element.as_ref(),
            &JsValue::from_str("value"),
            &JsValue::from_str("12"),
        )
        .expect("value property should set");

        let (messages, locale) = messages_and_locale();

        assert_eq!(
            native_validation_error(&element, &messages, &locale).code,
            ErrorCode::Max(10.0)
        );
    }
}
