//! Dioxus Button adapter.
//!
//! This module renders the framework-agnostic Button machine as Dioxus RSX,
//! preserving native button semantics and the callback-based `as_child`
//! reassignment contract.

use ars_components::utility::button::{self, Api};
pub use ars_components::utility::button::{
    FormEncType, FormMethod, FormTarget, Size, Type, Variant,
};
use ars_core::{AriaAttr, AttrMap, AttrValue, HtmlAttr, KeyModifiers, PointerType, SharedFlag};
pub use ars_core::{SafeUrl, UnsafeUrlError};
pub use ars_interactions::{PressEvent, PressEventType};
use dioxus::{dioxus_core::AttributeValue, prelude::*};

use crate::{
    as_child::AsChildRenderProps, attr_map_to_dioxus_inline_attrs, use_machine, use_stable_id,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct UserRootAttrs {
    class: Option<String>,
    style: Option<String>,
    aria_label: Option<String>,
    aria_labelledby: Option<String>,
}

/// Dioxus prop input for an optional Button form action override.
///
/// The wrapper keeps the public RSX API ergonomic for hardcoded URLs
/// (`form_action: "/submit"`) while still normalizing to the core
/// [`SafeUrl`] boundary before attributes are rendered.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormAction(Option<SafeUrl>);

impl FormAction {
    /// Return the validated URL value.
    #[must_use]
    pub fn into_safe_url(self) -> Option<SafeUrl> {
        self.0
    }
}

impl From<SafeUrl> for FormAction {
    fn from(value: SafeUrl) -> Self {
        Self(Some(value))
    }
}

impl From<&'static str> for FormAction {
    fn from(value: &'static str) -> Self {
        Self(Some(SafeUrl::from_static(value)))
    }
}

/// Props for the native Dioxus [`Button`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct ButtonProps {
    /// Optional component instance ID. When absent, the adapter generates one.
    #[props(optional, into)]
    pub id: Option<String>,

    /// Whether the button is disabled.
    #[props(default = false)]
    pub disabled: bool,

    /// Whether the button is in loading state.
    #[props(default = false)]
    pub loading: bool,

    /// Visual style variant.
    #[props(optional, into)]
    pub variant: Option<Variant>,

    /// Visual size token.
    #[props(optional, into)]
    pub size: Option<Size>,

    /// Native button type.
    #[props(optional, into)]
    pub r#type: Option<Type>,

    /// Associated form owner ID.
    #[props(optional, into)]
    pub form: Option<String>,

    /// Submitted form name.
    #[props(optional, into)]
    pub name: Option<String>,

    /// Submitted form value.
    #[props(optional, into)]
    pub value: Option<String>,

    /// Whether to remove the root from sequential tab navigation.
    #[props(default = false)]
    pub exclude_from_tab_order: bool,

    /// Form action override.
    #[props(default, into)]
    pub form_action: FormAction,

    /// Form method override.
    #[props(optional, into)]
    pub form_method: Option<FormMethod>,

    /// Form encoding override.
    #[props(optional, into)]
    pub form_enc_type: Option<FormEncType>,

    /// Form target override.
    #[props(optional, into)]
    pub form_target: Option<FormTarget>,

    /// Whether native form validation is bypassed.
    #[props(default = false)]
    pub form_no_validate: bool,

    /// Whether the root receives focus on mount.
    #[props(default = false)]
    pub auto_focus: bool,

    /// Whether pointer press should suppress focus movement.
    #[props(default = false)]
    pub prevent_focus_on_press: bool,

    /// Consumer class tokens appended to the root.
    #[props(optional, into)]
    pub class: Option<String>,

    /// Consumer inline style text applied to the root.
    #[props(optional, into)]
    pub style: Option<String>,

    /// Accessible label applied to the root.
    #[props(optional, into)]
    pub aria_label: Option<String>,

    /// Accessible label relationship applied to the root.
    #[props(optional, into)]
    pub aria_labelledby: Option<String>,

    /// Fires when a press starts.
    #[props(optional, into)]
    pub on_press_start: Option<EventHandler<PressEvent>>,

    /// Fires when a press ends.
    #[props(optional, into)]
    pub on_press_end: Option<EventHandler<PressEvent>>,

    /// Fires when the button activates.
    #[props(optional, into)]
    pub on_press: Option<EventHandler<PressEvent>>,

    /// Fires when pressed state changes.
    #[props(optional, into)]
    pub on_press_change: Option<EventHandler<bool>>,

    /// Fires when pointer/key release occurs.
    #[props(optional, into)]
    pub on_press_up: Option<EventHandler<PressEvent>>,

    /// Visible button content.
    pub children: Element,
}

/// Props for the Dioxus [`ButtonAsChild`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct ButtonAsChildProps {
    /// Optional component instance ID. When absent, the adapter generates one.
    #[props(optional, into)]
    pub id: Option<String>,

    /// Whether the button is disabled.
    #[props(default = false)]
    pub disabled: bool,

    /// Whether the button is in loading state.
    #[props(default = false)]
    pub loading: bool,

    /// Visual style variant.
    #[props(optional, into)]
    pub variant: Option<Variant>,

    /// Visual size token.
    #[props(optional, into)]
    pub size: Option<Size>,

    /// Whether to remove the root from sequential tab navigation.
    #[props(default = false)]
    pub exclude_from_tab_order: bool,

    /// Consumer class tokens appended to the root.
    #[props(optional, into)]
    pub class: Option<String>,

    /// Consumer inline style text applied to the root.
    #[props(optional, into)]
    pub style: Option<String>,

    /// Accessible label applied to the root.
    #[props(optional, into)]
    pub aria_label: Option<String>,

    /// Accessible label relationship applied to the root.
    #[props(optional, into)]
    pub aria_labelledby: Option<String>,

    /// Render callback that owns the child root and spreads Button attrs.
    pub render: Callback<AsChildRenderProps, Element>,
}

/// Dioxus Button component rendered as a native `<button>` root.
#[expect(
    unused_qualifications,
    reason = "rsx! macro expansion currently reports event-handler closures as unnecessary qualifications."
)]
#[component]
pub fn Button(props: ButtonProps) -> Element {
    let ButtonProps {
        id,
        disabled,
        loading,
        variant,
        size,
        r#type,
        form,
        name,
        value,
        exclude_from_tab_order,
        form_action,
        form_method,
        form_enc_type,
        form_target,
        form_no_validate,
        auto_focus,
        prevent_focus_on_press,
        class,
        style,
        aria_label,
        aria_labelledby,
        on_press_start,
        on_press_end,
        on_press,
        on_press_change,
        on_press_up,
        children,
    } = props;

    let generated_id = use_stable_id("button");
    let id = id.unwrap_or(generated_id);

    let mut core_props = button::Props::new()
        .id(&id)
        .disabled(disabled)
        .loading(loading)
        .variant(variant.unwrap_or_default())
        .size(size.unwrap_or_default())
        .button_type(r#type.unwrap_or_default())
        .exclude_from_tab_order(exclude_from_tab_order)
        .form_no_validate(form_no_validate)
        .auto_focus(auto_focus)
        .prevent_focus_on_press(prevent_focus_on_press);

    if let Some(form) = form {
        core_props = core_props.form(form);
    }

    if let Some(name) = name {
        core_props = core_props.name(name);
    }

    if let Some(value) = value {
        core_props = core_props.value(value);
    }

    if let Some(form_action) = form_action.into_safe_url() {
        core_props = core_props.form_action(form_action);
    }

    if let Some(form_method) = form_method {
        core_props = core_props.form_method(form_method);
    }

    if let Some(form_enc_type) = form_enc_type {
        core_props = core_props.form_enc_type(form_enc_type);
    }

    if let Some(form_target) = form_target {
        core_props = core_props.form_target(form_target);
    }

    let machine = use_machine::<button::Machine>(core_props);

    let user_attrs = UserRootAttrs {
        class,
        style,
        aria_label,
        aria_labelledby,
    };

    let root_attrs = machine.derive(move |api| dioxus_root_attrs(api, &id, &user_attrs, false));

    let loading_attrs =
        machine.derive(|api| attr_map_to_dioxus_inline_attrs(api.loading_indicator_attrs()));

    let content_attrs = machine.derive(|api| attr_map_to_dioxus_inline_attrs(api.content_attrs()));

    let is_loading = machine.derive(Api::is_loading);

    let mut last_pointer = use_signal(|| false);
    let mut last_pointer_type = use_signal(|| None::<PointerType>);

    let callbacks = PressCallbacks {
        on_press_start,
        on_press_end,
        on_press,
        on_press_change,
        on_press_up,
    };

    rsx! {
        button {
            onpointerdown: move |ev| {
                last_pointer.set(true);

                let data = ev.data();

                let event = press_event_from_pointer(
                    pointer_type_from_dioxus(&data.pointer_type()),
                    PressEventType::PressStart,
                    Some(data.client_coordinates().x),
                    Some(data.client_coordinates().y),
                    key_modifiers_from_dioxus(data.modifiers()),
                );

                last_pointer_type.set(Some(event.pointer_type));

                if !machine.with_api_snapshot(Api::is_disabled) {
                    emit_press(callbacks.on_press_start, event);

                    emit_bool(callbacks.on_press_change, true);
                }

                machine.send.call(button::Event::Press);

                if machine.with_api_snapshot(Api::should_prevent_focus_on_press) {
                    ev.prevent_default();
                }
            },

            onpointerup: move |ev| {
                let was_pressed = machine.with_api_snapshot(Api::is_pressed);

                let data = ev.data();

                let event = press_event_from_pointer(
                    pointer_type_from_dioxus(&data.pointer_type()),
                    PressEventType::PressEnd,
                    Some(data.client_coordinates().x),
                    Some(data.client_coordinates().y),
                    key_modifiers_from_dioxus(data.modifiers()),
                );

                machine.send.call(button::Event::Release);

                if was_pressed {
                    emit_press(callbacks.on_press_end, event.clone());

                    emit_press(
                        callbacks.on_press_up,
                        PressEvent {
                            event_type: PressEventType::PressUp,
                            ..event
                        },
                    );

                    emit_bool(callbacks.on_press_change, false);
                }
            },

            onfocus: move |_| {
                let is_keyboard = !last_pointer();

                last_pointer.set(false);

                machine
                    .send
                    .call(button::Event::Focus {
                        is_keyboard,
                    });
            },

            onblur: move |_| machine.send.call(button::Event::Blur),

            onclick: move |ev| {
                if machine.with_api_snapshot(should_prevent_activation_default) {
                    ev.prevent_default();
                }

                let interactive = !machine.with_api_snapshot(Api::is_disabled);
                let pointer_type = last_pointer_type();

                machine.send.call(button::Event::Click);

                if interactive {
                    emit_press(callbacks.on_press, press_event_from_click(&ev, pointer_type));
                }

                last_pointer_type.set(None);
            },
            ..root_attrs(),

            if is_loading() {
                span { ..loading_attrs() }
            }

            span { ..content_attrs(),{children} }
        }
    }
}

/// Dioxus Button component that forwards root attrs to a callback-owned child.
#[component]
pub fn ButtonAsChild(props: ButtonAsChildProps) -> Element {
    let ButtonAsChildProps {
        id,
        disabled,
        loading,
        variant,
        size,
        exclude_from_tab_order,
        class,
        style,
        aria_label,
        aria_labelledby,
        render,
    } = props;

    let generated_id = use_stable_id("button");
    let id = id.unwrap_or(generated_id);

    let core_props = button::Props::new()
        .id(&id)
        .disabled(disabled)
        .loading(loading)
        .variant(variant.unwrap_or_default())
        .size(size.unwrap_or_default())
        .as_child(true)
        .exclude_from_tab_order(exclude_from_tab_order);

    let machine = use_machine::<button::Machine>(core_props);

    let user_attrs = UserRootAttrs {
        class,
        style,
        aria_label,
        aria_labelledby,
    };

    let attrs = machine.derive(move |api| dioxus_root_attrs(api, &id, &user_attrs, true));

    render.call(AsChildRenderProps { attrs: attrs() })
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct PressCallbacks {
    on_press_start: Option<EventHandler<PressEvent>>,
    on_press_end: Option<EventHandler<PressEvent>>,
    on_press: Option<EventHandler<PressEvent>>,
    on_press_change: Option<EventHandler<bool>>,
    on_press_up: Option<EventHandler<PressEvent>>,
}

fn dioxus_root_attrs(
    api: &Api<'_>,
    id: &str,
    user_attrs: &UserRootAttrs,
    filter_native: bool,
) -> Vec<Attribute> {
    let mut attrs = api.root_attrs();

    attrs.set(HtmlAttr::Id, id.to_owned());

    apply_user_root_attrs(&mut attrs, user_attrs);

    if filter_native {
        filter_native_button_attrs(&mut attrs);
    }

    let mut dioxus_attrs = attr_map_to_dioxus_inline_attrs(attrs);

    if let Some(style) = &user_attrs.style {
        dioxus_attrs.push(Attribute::new(
            "style",
            AttributeValue::Text(style.clone()),
            None,
            false,
        ));
    }

    dioxus_attrs
}

fn apply_user_root_attrs(attrs: &mut AttrMap, user_attrs: &UserRootAttrs) {
    if let Some(class) = &user_attrs.class {
        attrs.set(HtmlAttr::Class, class);
    }

    if let Some(label) = &user_attrs.aria_label {
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
    }

    if let Some(labelledby) = &user_attrs.aria_labelledby {
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), labelledby);
    }
}

fn filter_native_button_attrs(attrs: &mut AttrMap) {
    for attr in [
        HtmlAttr::Type,
        HtmlAttr::Form,
        HtmlAttr::FormAction,
        HtmlAttr::FormMethod,
        HtmlAttr::FormEncType,
        HtmlAttr::FormTarget,
        HtmlAttr::FormNoValidate,
        HtmlAttr::Name,
        HtmlAttr::Value,
        HtmlAttr::Disabled,
        HtmlAttr::AutoFocus,
    ] {
        attrs.set(attr, AttrValue::None);
    }
}

const fn should_prevent_activation_default(api: &Api<'_>) -> bool {
    Api::is_loading(api) && matches!(Api::button_type(api), Type::Submit | Type::Reset)
}

fn emit_press(callback: Option<EventHandler<PressEvent>>, event: PressEvent) {
    if let Some(callback) = callback {
        callback.call(event);
    }
}

fn emit_bool(callback: Option<EventHandler<bool>>, value: bool) {
    if let Some(callback) = callback {
        callback.call(value);
    }
}

fn press_event_from_pointer(
    pointer_type: PointerType,
    event_type: PressEventType,
    client_x: Option<f64>,
    client_y: Option<f64>,
    modifiers: KeyModifiers,
) -> PressEvent {
    PressEvent {
        pointer_type,
        event_type,
        client_x,
        client_y,
        modifiers,
        is_within_element: true,
        continue_propagation: SharedFlag::new(false),
    }
}

fn pointer_type_from_dioxus(pointer_type: &str) -> PointerType {
    match pointer_type {
        "mouse" => PointerType::Mouse,
        "touch" => PointerType::Touch,
        "pen" => PointerType::Pen,
        _ => PointerType::Virtual,
    }
}

fn key_modifiers_from_dioxus(modifiers: Modifiers) -> KeyModifiers {
    KeyModifiers {
        shift: modifiers.shift(),
        ctrl: modifiers.ctrl(),
        alt: modifiers.alt(),
        meta: modifiers.meta(),
    }
}

fn press_event_from_click(ev: &Event<MouseData>, pointer_type: Option<PointerType>) -> PressEvent {
    let data = ev.data();
    let pointer_type = pointer_type.unwrap_or(PointerType::Virtual);
    let (client_x, client_y) = if matches!(pointer_type, PointerType::Virtual) {
        (None, None)
    } else {
        (
            Some(data.client_coordinates().x),
            Some(data.client_coordinates().y),
        )
    };

    press_event_from_pointer(
        pointer_type,
        PressEventType::Press,
        client_x,
        client_y,
        key_modifiers_from_dioxus(data.modifiers()),
    )
}

#[cfg(test)]
mod tests {
    use ars_core::{Env, Service};
    use dioxus::prelude::Modifiers;

    use super::*;

    fn api_for(props: button::Props) -> bool {
        let service =
            Service::<button::Machine>::new(props, &Env::default(), &button::Messages::default());

        let api = service.connect(&|_| {});

        should_prevent_activation_default(&api)
    }

    #[test]
    fn press_event_preserves_pointer_coordinates_and_modifiers() {
        let event = press_event_from_pointer(
            PointerType::Mouse,
            PressEventType::PressStart,
            Some(12.0),
            Some(34.0),
            KeyModifiers {
                shift: true,
                ctrl: false,
                alt: true,
                meta: false,
            },
        );

        assert_eq!(event.pointer_type, PointerType::Mouse);
        assert_eq!(event.event_type, PressEventType::PressStart);
        assert_eq!(event.client_x, Some(12.0));
        assert_eq!(event.client_y, Some(34.0));
        assert!(event.modifiers.shift);
        assert!(event.modifiers.alt);
        assert!(!event.modifiers.ctrl);
        assert!(!event.modifiers.meta);
    }

    #[test]
    fn dioxus_modifiers_map_to_press_modifiers() {
        let modifiers = key_modifiers_from_dioxus(Modifiers::SHIFT | Modifiers::CONTROL);

        assert!(modifiers.shift);
        assert!(modifiers.ctrl);
        assert!(!modifiers.alt);
        assert!(!modifiers.meta);
    }

    #[test]
    fn pointer_type_tokens_map_to_press_pointer_types() {
        assert_eq!(pointer_type_from_dioxus("mouse"), PointerType::Mouse);
        assert_eq!(pointer_type_from_dioxus("touch"), PointerType::Touch);
        assert_eq!(pointer_type_from_dioxus("pen"), PointerType::Pen);
        assert_eq!(pointer_type_from_dioxus(""), PointerType::Virtual);
        assert_eq!(pointer_type_from_dioxus("unknown"), PointerType::Virtual);
    }

    #[test]
    fn should_prevent_activation_default_only_for_loading_submit_or_reset() {
        assert!(!api_for(button::Props::new().id("button")));
        assert!(!api_for(button::Props::new().id("button").loading(true)));
        assert!(!api_for(
            button::Props::new().id("button").button_type(Type::Submit)
        ));
        assert!(api_for(
            button::Props::new()
                .id("button")
                .loading(true)
                .button_type(Type::Submit)
        ));
        assert!(api_for(
            button::Props::new()
                .id("button")
                .loading(true)
                .button_type(Type::Reset)
        ));
    }

    #[test]
    fn callback_emitters_invoke_present_callbacks_and_ignore_missing_callbacks() {
        fn app() -> Element {
            let captured = use_signal(Vec::<String>::new);

            let mut press_captured = captured;
            let mut bool_captured = captured;

            emit_press(
                None,
                press_event_from_pointer(
                    PointerType::Mouse,
                    PressEventType::Press,
                    None,
                    None,
                    KeyModifiers::default(),
                ),
            );
            emit_bool(None, true);

            assert!(captured.peek().is_empty());

            emit_press(
                Some(EventHandler::new(move |event: PressEvent| {
                    press_captured
                        .write()
                        .push(format!("{:?}:{:?}", event.pointer_type, event.event_type));
                })),
                press_event_from_pointer(
                    PointerType::Mouse,
                    PressEventType::Press,
                    None,
                    None,
                    KeyModifiers::default(),
                ),
            );
            emit_bool(
                Some(EventHandler::new(move |pressed: bool| {
                    bool_captured.write().push(format!("pressed:{pressed}"));
                })),
                true,
            );

            assert_eq!(
                captured.peek().as_slice(),
                &[String::from("Mouse:Press"), String::from("pressed:true")]
            );

            rsx! {
                div {}
            }
        }

        let mut dom = VirtualDom::new(app);

        dom.rebuild_in_place();
    }

    #[test]
    fn user_root_attrs_are_applied_and_as_child_native_attrs_are_filtered() {
        let service = Service::<button::Machine>::new(
            button::Props::new()
                .id("button")
                .disabled(true)
                .button_type(Type::Submit)
                .form("account-form")
                .name("intent")
                .value("save")
                .form_action(SafeUrl::from_static("/submit"))
                .form_method(FormMethod::Post)
                .form_enc_type(FormEncType::MultipartFormData)
                .form_target(FormTarget::Self_)
                .form_no_validate(true)
                .auto_focus(true),
            &Env::default(),
            &button::Messages::default(),
        );

        let api = service.connect(&|_| {});

        let mut attrs = api.root_attrs();

        apply_user_root_attrs(
            &mut attrs,
            &UserRootAttrs {
                class: Some(String::from("app-button")),
                style: None,
                aria_label: Some(String::from("Save account")),
                aria_labelledby: Some(String::from("save-label")),
            },
        );

        assert_eq!(attrs.get(&HtmlAttr::Class), Some("app-button"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Save account")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("save-label")
        );

        filter_native_button_attrs(&mut attrs);

        for attr in [
            HtmlAttr::Type,
            HtmlAttr::Form,
            HtmlAttr::FormAction,
            HtmlAttr::FormMethod,
            HtmlAttr::FormEncType,
            HtmlAttr::FormTarget,
            HtmlAttr::FormNoValidate,
            HtmlAttr::Name,
            HtmlAttr::Value,
            HtmlAttr::Disabled,
            HtmlAttr::AutoFocus,
        ] {
            assert!(
                !attrs.contains(&attr),
                "as-child filtering should remove {attr:?}"
            );
        }

        assert_eq!(attrs.get(&HtmlAttr::Class), Some("app-button"));
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    use wasm_bindgen::{JsCast, closure::Closure};
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
    use web_sys::{Document, Element as WebElementHandle, HtmlElement};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    thread_local! {
        static PRESS_LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn clear_press_log() {
        PRESS_LOG.with_borrow_mut(Vec::clear);
    }

    fn press_log() -> Vec<String> {
        PRESS_LOG.with_borrow(Clone::clone)
    }

    fn push_press(label: &str, event: &PressEvent) {
        PRESS_LOG.with_borrow_mut(|log| {
            log.push(format!(
                "{label}:{:?}:{:?}:x={:?}:y={:?}:shift={}:alt={}",
                event.pointer_type,
                event.event_type,
                event.client_x,
                event.client_y,
                event.modifiers.shift,
                event.modifiers.alt,
            ));
        });
    }

    fn dispatch_mouse_pointer(root: &WebElementHandle, event_type: &str) {
        let init = web_sys::PointerEventInit::new();

        init.set_bubbles(true);
        init.set_cancelable(true);
        init.set_pointer_type("mouse");
        init.set_client_x(12);
        init.set_client_y(34);

        let event = web_sys::PointerEvent::new_with_event_init_dict(event_type, &init)
            .expect("pointer event should construct");

        assert!(
            root.dispatch_event(&event)
                .expect("pointer event should dispatch"),
            "{event_type} should not be canceled",
        );
    }

    fn dispatch_mouse_click(root: &WebElementHandle) {
        let event = web_sys::MouseEvent::new("click").expect("click should construct");
        event.init_mouse_event_with_can_bubble_arg_and_cancelable_arg_and_view_arg_and_detail_arg_and_screen_x_arg_and_screen_y_arg_and_client_x_arg_and_client_y_arg_and_ctrl_key_arg_and_alt_key_arg_and_shift_key_arg_and_meta_key_arg(
            "click",
            true,
            true,
            web_sys::window().as_ref(),
            0,
            0,
            0,
            56,
            78,
            false,
            true,
            true,
            false,
        );

        assert!(
            root.dispatch_event(&event).expect("click should dispatch"),
            "click should not be canceled",
        );
    }

    fn document() -> Document {
        web_sys::window()
            .and_then(|window| window.document())
            .expect("browser document should exist")
    }

    fn with_container() -> WebElementHandle {
        let container = document()
            .create_element("div")
            .expect("create_element should succeed");

        document()
            .body()
            .expect("body should exist")
            .append_child(&container)
            .expect("append_child should succeed");

        container
    }

    async fn animation_frame_turn() {
        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            let resolve = resolve.clone();

            let callback = Closure::once_into_js(move || {
                drop(resolve.call0(&wasm_bindgen::JsValue::UNDEFINED));
            });

            web_sys::window()
                .expect("window should exist")
                .request_animation_frame(callback.unchecked_ref())
                .expect("requestAnimationFrame should succeed");
        });

        drop(wasm_bindgen_futures::JsFuture::from(promise).await);
    }

    async fn microtask_turn() {
        drop(
            wasm_bindgen_futures::JsFuture::from(js_sys::Promise::resolve(
                &wasm_bindgen::JsValue::UNDEFINED,
            ))
            .await,
        );
    }

    async fn sleep_ms(ms: i32) {
        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            web_sys::window()
                .expect("window should exist")
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms)
                .expect("setTimeout should succeed");
        });

        drop(wasm_bindgen_futures::JsFuture::from(promise).await);
    }

    async fn flush() {
        for _ in 0..3 {
            animation_frame_turn().await;

            microtask_turn().await;
        }

        sleep_ms(100).await;

        for _ in 0..3 {
            animation_frame_turn().await;

            microtask_turn().await;
        }
    }

    fn launch(container: &WebElementHandle, app: fn() -> Element) {
        let dom = VirtualDom::new(app);

        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootelement(container.clone()),
        );
    }

    #[expect(
        unused_qualifications,
        reason = "rsx! macro expansion currently reports event-handler closures as unnecessary qualifications."
    )]
    fn reactive_loading_fixture() -> Element {
        let mut loading = use_signal(|| false);

        rsx! {
            Button { id: "reactive-button", loading: loading(), "Save" }
            button { id: "toggle-loading", onclick: move |_| loading.set(true), "toggle" }
        }
    }

    #[wasm_bindgen_test]
    async fn button_reactive_loading_updates_root_and_parts_on_wasm() {
        let container = with_container();

        launch(&container, reactive_loading_fixture);

        flush().await;

        let root = container
            .query_selector("#reactive-button")
            .expect("query_selector should succeed")
            .expect("button should exist");

        assert_eq!(root.get_attribute("aria-busy"), None);
        assert!(
            root.query_selector("[data-ars-part='loading-indicator']")
                .expect("query_selector should succeed")
                .is_none(),
            "idle button must not render a loading indicator",
        );

        let trigger = container
            .query_selector("#toggle-loading")
            .expect("query_selector should succeed")
            .expect("toggle trigger should exist");

        let trigger: HtmlElement = trigger.unchecked_into();

        trigger.click();

        flush().await;

        assert_eq!(root.get_attribute("aria-busy").as_deref(), Some("true"));
        assert_eq!(root.get_attribute("aria-disabled").as_deref(), Some("true"));

        let indicator = root
            .query_selector("[data-ars-part='loading-indicator']")
            .expect("query_selector should succeed")
            .expect("loading indicator should exist");

        assert_eq!(indicator.get_attribute("role").as_deref(), Some("status"));
        assert_eq!(
            indicator.get_attribute("aria-live").as_deref(),
            Some("polite")
        );
        assert_eq!(
            indicator.get_attribute("aria-label").as_deref(),
            Some("Loading")
        );

        container.remove();
    }

    fn callback_fixture() -> Element {
        rsx! {
            Button {
                id: "callback-button",
                on_press_start: move |event: PressEvent| {
                    push_press("start", &event);
                },
                on_press_end: move |event: PressEvent| {
                    push_press("end", &event);
                },
                on_press: move |event: PressEvent| {
                    push_press("press", &event);
                },
                on_press_change: move |pressed: bool| {
                    PRESS_LOG.with_borrow_mut(|log| log.push(format!("change:{pressed}")));
                },
                on_press_up: move |event: PressEvent| {
                    push_press("up", &event);
                },
                "Save"
            }
        }
    }

    #[wasm_bindgen_test]
    async fn button_press_callbacks_fire_in_native_event_order_on_wasm() {
        clear_press_log();

        let container = with_container();

        launch(&container, callback_fixture);

        flush().await;

        let root = container
            .query_selector("#callback-button")
            .expect("query_selector should succeed")
            .expect("button should exist");

        dispatch_mouse_pointer(&root, "pointerdown");
        dispatch_mouse_pointer(&root, "pointerup");

        dispatch_mouse_click(&root);

        flush().await;

        assert_eq!(
            press_log(),
            [
                String::from(
                    "start:Mouse:PressStart:x=Some(12.0):y=Some(34.0):shift=false:alt=false"
                ),
                String::from("change:true"),
                String::from("end:Mouse:PressEnd:x=Some(12.0):y=Some(34.0):shift=false:alt=false"),
                String::from("up:Mouse:PressUp:x=Some(12.0):y=Some(34.0):shift=false:alt=false"),
                String::from("change:false"),
                String::from("press:Mouse:Press:x=Some(56.0):y=Some(78.0):shift=true:alt=true"),
            ]
        );

        container.remove();
    }

    fn disabled_callback_fixture() -> Element {
        rsx! {
            Button {
                id: "disabled-callback-button",
                disabled: true,
                on_press_start: move |event: PressEvent| {
                    push_press("start", &event);
                },
                on_press_end: move |event: PressEvent| {
                    push_press("end", &event);
                },
                on_press: move |event: PressEvent| {
                    push_press("press", &event);
                },
                on_press_change: move |pressed: bool| {
                    PRESS_LOG.with_borrow_mut(|log| log.push(format!("change:{pressed}")));
                },
                on_press_up: move |event: PressEvent| {
                    push_press("up", &event);
                },
                "Save"
            }
        }
    }

    #[wasm_bindgen_test]
    async fn disabled_button_suppresses_press_callbacks_on_wasm() {
        clear_press_log();

        let container = with_container();

        launch(&container, disabled_callback_fixture);

        flush().await;

        let root = container
            .query_selector("#disabled-callback-button")
            .expect("query_selector should succeed")
            .expect("button should exist");

        dispatch_mouse_pointer(&root, "pointerdown");
        dispatch_mouse_pointer(&root, "pointerup");

        dispatch_mouse_click(&root);

        flush().await;

        assert!(
            press_log().is_empty(),
            "disabled buttons must not emit press callbacks",
        );

        container.remove();
    }

    fn prevent_focus_fixture() -> Element {
        rsx! {
            Button { id: "prevent-focus-button", prevent_focus_on_press: true, "Open" }
        }
    }

    #[wasm_bindgen_test]
    async fn button_prevent_focus_on_press_cancels_pointerdown_default_on_wasm() {
        let container = with_container();

        launch(&container, prevent_focus_fixture);
        flush().await;

        let root = container
            .query_selector("#prevent-focus-button")
            .expect("query_selector should succeed")
            .expect("button should exist");

        let init = web_sys::PointerEventInit::new();

        init.set_bubbles(true);
        init.set_cancelable(true);
        init.set_pointer_type("mouse");

        let event = web_sys::PointerEvent::new_with_event_init_dict("pointerdown", &init)
            .expect("pointerdown should construct");

        assert!(
            !root
                .dispatch_event(&event)
                .expect("pointerdown should dispatch"),
            "prevent_focus_on_press should cancel pointerdown default",
        );

        container.remove();
    }

    fn loading_submit_fixture() -> Element {
        rsx! {
            form { id: "loading-form",
                Button {
                    id: "loading-submit",
                    loading: true,
                    r#type: Type::Submit,
                    "Save"
                }
            }
        }
    }

    #[wasm_bindgen_test]
    async fn loading_submit_button_cancels_native_activation_on_wasm() {
        let container = with_container();

        launch(&container, loading_submit_fixture);

        flush().await;

        let submits = Rc::new(Cell::new(0usize));
        let submit_count = Rc::clone(&submits);
        let on_submit = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
            submit_count.set(submit_count.get() + 1);
            event.prevent_default();
        });

        let form = container
            .query_selector("#loading-form")
            .expect("query_selector should succeed")
            .expect("form should exist");

        form.add_event_listener_with_callback("submit", on_submit.as_ref().unchecked_ref())
            .expect("submit listener should attach");

        let root = container
            .query_selector("#loading-submit")
            .expect("query_selector should succeed")
            .expect("button should exist");

        let root: HtmlElement = root.unchecked_into();

        root.click();

        flush().await;

        assert_eq!(
            submits.get(),
            0,
            "loading submit button should prevent native form submission",
        );

        form.remove_event_listener_with_callback("submit", on_submit.as_ref().unchecked_ref())
            .expect("submit listener should detach");

        drop(on_submit);

        container.remove();
    }
}
