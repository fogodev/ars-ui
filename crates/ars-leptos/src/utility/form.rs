//! Leptos Form adapter.

use std::collections::BTreeMap;

use ars_components::utility::form;
pub use ars_components::utility::form::{Part, Props, ValidationBehavior};
use ars_core::{AriaAttr, AttrMap, AttrValue, HtmlAttr};
use ars_forms::validation::{Error, merge_error_map};
#[cfg(target_arch = "wasm32")]
use ars_forms::validation::{NativeInputType, NativeValidity};
use leptos::{children::TypedChildren, context::Provider, html, prelude::*};
#[cfg(target_arch = "wasm32")]
use leptos::{
    wasm_bindgen::{JsCast as _, JsValue},
    web_sys,
};

use crate::{
    apply_part_attrs, attr_map_to_leptos_inline_attrs, callbacks, use_id,
    use_machine_with_reactive_props, use_messages_and_locale,
};

#[derive(Clone, Copy)]
pub(crate) struct FormContext {
    pub(crate) machine: crate::UseMachineReturn<form::Machine>,
    status_region_count: RwSignal<usize>,
}

fn form_context() -> FormContext {
    use_context::<FormContext>().expect("Form subcomponents must be rendered inside <form::Root/>")
}

/// Leptos Form root component.
#[component]
pub fn Root<T: 'static>(
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

    /// Controlled status text shown in the form live region.
    #[prop(optional, into)]
    status_message: Signal<Option<String>>,

    /// Consumer class tokens appended to the form.
    #[prop(optional, into)]
    class: Option<TextProp>,

    /// Consumer inline style text applied to the form.
    #[prop(optional, into)]
    style: Option<TextProp>,

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
        status_message,
    ));

    let form_messages = use_messages_and_locale::<ars_forms::form::Messages>(None, None);

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

    let status_region_count = RwSignal::new(0_usize);

    view! {
        <Provider value=FormContext {
            machine,
            status_region_count,
        }>
            <form
                {..attrs}
                node_ref=form_ref
                on:submit:capture=move |event| {
                    let skip_validation = submitter_skips_validation(&event);
                    if validation_behavior.get_untracked() == ValidationBehavior::Aria
                        || on_submit.is_some()
                    {
                        event.prevent_default();
                    }
                    if validation_behavior.get_untracked() == ValidationBehavior::Aria
                        && !skip_validation && !form_is_valid(form_ref)
                    {
                        let (messages, locale) = form_messages.get_untracked();
                        let mut errors = validation_errors.get_untracked();
                        let native_errors = invalid_control_errors(form_ref, &messages, &locale);
                        let error_count = native_errors
                            .values()
                            .map(Vec::len)
                            .sum::<usize>()
                            .max(1);
                        merge_error_map(&mut errors, native_errors);
                        machine.send.run(form::Event::SetValidationErrors(errors));
                        machine
                            .send
                            .run(
                                form::Event::SetStatusMessage(
                                    Some((messages.submit_error_count)(error_count, &locale)),
                                ),
                            );
                        return;
                    }
                    if validation_behavior.get_untracked() == ValidationBehavior::Aria {
                        machine
                            .send
                            .run(
                                form::Event::SetValidationErrors(validation_errors.get_untracked()),
                            );
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
                {move || {
                    (status_region_count.get() == 0)
                        .then(|| status_region_view(machine, None, None))
                }}
            </form>
        </Provider>
    }
}

fn form_props_signal(
    props: Props,
    validation_behavior: Signal<ValidationBehavior>,
    validation_errors: Signal<BTreeMap<String, Vec<Error>>>,
    status_message: Signal<Option<String>>,
) -> Signal<Props> {
    Signal::derive(move || {
        props
            .clone()
            .validation_behavior(validation_behavior.get())
            .validation_errors(validation_errors.get())
            .maybe_status_message(status_message.get())
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
///
/// Rendering this part inside [`Root`] styles or repositions the form-owned
/// live region while preserving the core status-region attributes and message
/// source. If omitted, [`Root`] renders an unstyled fallback status region.
#[component]
pub fn StatusRegion(
    /// Consumer class tokens appended to the status region.
    #[prop(optional, into)]
    class: Option<TextProp>,

    /// Consumer inline style text applied to the status region.
    #[prop(optional, into)]
    style: Option<TextProp>,
) -> impl IntoView {
    let FormContext {
        machine,
        status_region_count,
    } = form_context();

    status_region_count.update(|status_region_count| *status_region_count += 1);

    on_cleanup(move || {
        status_region_count.update(|status_region_count| {
            *status_region_count = status_region_count.saturating_sub(1);
        });
    });

    status_region_view(machine, class, style)
}

fn status_region_view(
    machine: crate::UseMachineReturn<form::Machine>,
    class: Option<TextProp>,
    style: Option<TextProp>,
) -> impl IntoView {
    let status_message = machine.derive(|api| api.status_message().map(str::to_owned));

    let attrs =
        machine.with_api_snapshot(|api| apply_part_attrs(api.status_region_attrs(), class, style));

    view! { <div {..attrs}>{status_message}</div> }
}

#[cfg(target_arch = "wasm32")]
fn submitter_skips_validation(event: &web_sys::SubmitEvent) -> bool {
    event
        .submitter()
        .and_then(|submitter| submitter.dyn_into::<web_sys::Element>().ok())
        .is_some_and(|submitter| submitter.has_attribute("formnovalidate"))
}

#[cfg(not(target_arch = "wasm32"))]
const fn submitter_skips_validation<T>(event: &T) -> bool {
    let _ = event;

    false
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

#[expect(
    clippy::missing_const_for_fn,
    reason = "The wasm implementation reads live form controls through querySelectorAll."
)]
fn invalid_control_errors(
    form_ref: NodeRef<html::Form>,
    messages: &ars_forms::form::Messages,
    locale: &ars_i18n::Locale,
) -> BTreeMap<String, Vec<Error>> {
    #[cfg(target_arch = "wasm32")]
    {
        let mut errors = BTreeMap::new();

        if let Some(form) = form_ref.get()
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

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = form_ref;
        let _ = messages;
        let _ = locale;

        BTreeMap::new()
    }
}

#[cfg(target_arch = "wasm32")]
fn native_validation_error(
    element: &web_sys::Element,
    messages: &ars_forms::form::Messages,
    locale: &ars_i18n::Locale,
) -> Error {
    native_validity(element).to_error(messages, locale)
}

// adapter-rendering-glue: extracts browser DOM validity facts for the shared forms helper.
#[cfg(target_arch = "wasm32")]
fn native_validity(element: &web_sys::Element) -> NativeValidity {
    NativeValidity {
        value_missing: validity_flag(element, "valueMissing"),
        type_mismatch: type_mismatch_input_type(element),
        pattern_mismatch: validity_flag(element, "patternMismatch")
            .then(|| element.get_attribute("pattern"))
            .flatten(),
        too_short: flagged_parsed_attr(element, "tooShort", "minlength"),
        too_long: flagged_parsed_attr(element, "tooLong", "maxlength"),
        range_underflow: flagged_parsed_attr(element, "rangeUnderflow", "min"),
        range_overflow: flagged_parsed_attr(element, "rangeOverflow", "max"),
        step_mismatch: flagged_parsed_attr(element, "stepMismatch", "step"),
    }
}

#[cfg(target_arch = "wasm32")]
fn type_mismatch_input_type(element: &web_sys::Element) -> Option<NativeInputType> {
    if validity_flag(element, "typeMismatch") {
        Some(match element.get_attribute("type").as_deref() {
            Some("email") => NativeInputType::Email,
            Some("url") => NativeInputType::Url,
            _ => NativeInputType::Other,
        })
    } else {
        None
    }
}

#[cfg(target_arch = "wasm32")]
fn flagged_parsed_attr<T: std::str::FromStr>(
    element: &web_sys::Element,
    flag: &str,
    attr: &str,
) -> Option<T> {
    validity_flag(element, flag)
        .then(|| parsed_attr(element, attr))
        .flatten()
}

#[cfg(target_arch = "wasm32")]
fn parsed_attr<T: std::str::FromStr>(element: &web_sys::Element, attr: &str) -> Option<T> {
    element
        .get_attribute(attr)
        .and_then(|value| value.parse().ok())
}

#[cfg(target_arch = "wasm32")]
fn validity_flag(element: &web_sys::Element, flag: &str) -> bool {
    js_sys::Reflect::get(element.as_ref(), &JsValue::from_str("validity"))
        .ok()
        .and_then(|validity| js_sys::Reflect::get(&validity, &JsValue::from_str(flag)).ok())
        .and_then(|flag| flag.as_bool())
        .unwrap_or(false)
}

#[cfg(all(test, target_arch = "wasm32"))]
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
