//! Leptos Button adapter.
//!
//! This module renders the framework-agnostic Button machine as Leptos views,
//! preserving the core root/loading/content anatomy and the adapter-local
//! `as_child` root reassignment contract.

use ars_components::utility::button::{self, Api};
pub use ars_components::utility::button::{
    FormEncType, FormMethod, FormTarget, Size, Type, Variant,
};
use ars_core::{AriaAttr, AttrMap, AttrValue, HtmlAttr, KeyModifiers, PointerType, SharedFlag};
pub use ars_core::{SafeUrl, UnsafeUrlError};
pub use ars_interactions::{PressEvent, PressEventType};
use leptos::{children::TypedChildren, prelude::*, tachys::view::add_attr::AddAnyAttr};

use crate::{
    LeptosAttribute, as_child::AsChildAttrs, attr_map_to_leptos_inline_attrs, attrs::string_attr,
    use_id, use_machine_with_reactive_props,
};

fn root_attrs(api: &Api<'_>) -> AttrMap {
    api.root_attrs()
}

fn loading_indicator_attrs(api: &Api<'_>) -> AttrMap {
    api.loading_indicator_attrs()
}

fn content_attrs(api: &Api<'_>) -> AttrMap {
    api.content_attrs()
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct UserRootAttrs {
    class: Option<String>,
    style: Option<String>,
    aria_label: Option<String>,
    aria_labelledby: Option<String>,
}

/// Leptos Button component rendered as a native `<button>` root.
#[component]
pub fn Button<T>(
    /// Optional component instance ID.
    #[prop(optional, into)]
    id: Option<String>,

    /// Whether the button is disabled.
    #[prop(optional, into)]
    disabled: Signal<bool>,

    /// Whether the button is in loading state.
    #[prop(optional, into)]
    loading: Signal<bool>,

    /// Visual style variant.
    #[prop(optional, into)]
    variant: Option<Variant>,

    /// Visual size token.
    #[prop(optional, into)]
    size: Option<Size>,

    /// Native button type.
    #[prop(optional, into)]
    r#type: Option<Type>,

    /// Associated form owner ID.
    #[prop(optional, into)]
    form: Option<String>,

    /// Submitted form name.
    #[prop(optional, into)]
    name: Option<String>,

    /// Submitted form value.
    #[prop(optional, into)]
    value: Option<String>,

    /// Form action override.
    #[prop(optional, into)]
    form_action: Option<SafeUrl>,

    /// Form method override.
    #[prop(optional, into)]
    form_method: Option<FormMethod>,

    /// Form encoding override.
    #[prop(optional, into)]
    form_enc_type: Option<FormEncType>,

    /// Form target override.
    #[prop(optional, into)]
    form_target: Option<FormTarget>,

    /// Whether native form validation is bypassed.
    #[prop(optional)]
    form_no_validate: bool,

    /// Whether the root receives focus on mount.
    #[prop(optional)]
    auto_focus: bool,

    /// Whether pointer press should suppress focus movement.
    #[prop(optional)]
    prevent_focus_on_press: bool,

    /// Whether to remove the root from sequential tab navigation.
    #[prop(optional)]
    exclude_from_tab_order: bool,

    /// Consumer class tokens appended to the root.
    #[prop(optional, into)]
    class: Option<String>,

    /// Consumer inline style text applied to the root.
    #[prop(optional, into)]
    style: Option<String>,

    /// Accessible label applied to the root.
    #[prop(optional, into)]
    aria_label: Option<String>,

    /// Accessible label relationship applied to the root.
    #[prop(optional, into)]
    aria_labelledby: Option<String>,

    /// Fires when a press starts.
    #[prop(optional)]
    on_press_start: Option<Callback<PressEvent>>,

    /// Fires when a press ends.
    #[prop(optional)]
    on_press_end: Option<Callback<PressEvent>>,

    /// Fires when the button activates.
    #[prop(optional)]
    on_press: Option<Callback<PressEvent>>,

    /// Fires when pressed state changes.
    #[prop(optional)]
    on_press_change: Option<Callback<bool>>,

    /// Fires when pointer/key release occurs.
    #[prop(optional)]
    on_press_up: Option<Callback<PressEvent>>,

    /// Visible button content.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let id = id.unwrap_or_else(|| use_id("button"));

    let mut props = button::Props::new()
        .id(&id)
        .variant(variant.unwrap_or_default())
        .size(size.unwrap_or_default())
        .button_type(r#type.unwrap_or_default())
        .exclude_from_tab_order(exclude_from_tab_order)
        .form_no_validate(form_no_validate)
        .auto_focus(auto_focus)
        .prevent_focus_on_press(prevent_focus_on_press);

    if let Some(form) = form {
        props = props.form(form);
    }

    if let Some(name) = name {
        props = props.name(name);
    }

    if let Some(value) = value {
        props = props.value(value);
    }

    if let Some(form_action) = form_action {
        props = props.form_action(form_action);
    }

    if let Some(form_method) = form_method {
        props = props.form_method(form_method);
    }

    if let Some(form_enc_type) = form_enc_type {
        props = props.form_enc_type(form_enc_type);
    }

    if let Some(form_target) = form_target {
        props = props.form_target(form_target);
    }

    let user_attrs = UserRootAttrs {
        class,
        style,
        aria_label,
        aria_labelledby,
    };

    let callbacks = PressCallbacks {
        on_press_start,
        on_press_end,
        on_press,
        on_press_change,
        on_press_up,
    };

    let machine =
        use_machine_with_reactive_props::<button::Machine>(props_signal(props, disabled, loading));

    let root_attrs = leptos_root_attrs(machine, user_attrs, false);
    let content_attrs = leptos_content_attrs(machine);

    let is_loading = machine.derive(Api::is_loading);

    let last_pointer = StoredValue::new(false);
    let last_pointer_type = StoredValue::new(None::<PointerType>);

    view! {
        <button
            {..root_attrs}
            on:pointerdown=move |ev| {
                last_pointer.set_value(true);
                let event = press_event_from_pointer(
                    pointer_type_from_leptos(&ev.pointer_type()),
                    PressEventType::PressStart,
                    Some(f64::from(ev.client_x())),
                    Some(f64::from(ev.client_y())),
                    key_modifiers_from_leptos_pointer(&ev),
                );
                last_pointer_type.set_value(Some(event.pointer_type));
                if !machine.with_api_snapshot(Api::is_disabled) {
                    emit_press(callbacks.on_press_start, event);
                    emit_bool(callbacks.on_press_change, true);
                }
                machine.send.run(button::Event::Press);
                if machine.with_api_snapshot(Api::should_prevent_focus_on_press) {
                    ev.prevent_default();
                }
            }

            on:pointerup=move |ev| {
                let was_pressed = machine.with_api_snapshot(Api::is_pressed);
                let event = press_event_from_pointer(
                    pointer_type_from_leptos(&ev.pointer_type()),
                    PressEventType::PressEnd,
                    Some(f64::from(ev.client_x())),
                    Some(f64::from(ev.client_y())),
                    key_modifiers_from_leptos_pointer(&ev),
                );
                machine.send.run(button::Event::Release);
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
            }

            on:focus=move |_| {
                let is_keyboard = !last_pointer.get_value();
                last_pointer.set_value(false);
                machine
                    .send
                    .run(button::Event::Focus {
                        is_keyboard,
                    });
            }

            on:blur=move |_| machine.send.run(button::Event::Blur)

            on:click=move |ev| {
                if machine.with_api_snapshot(should_prevent_activation_default) {
                    ev.prevent_default();
                }
                let interactive = !machine.with_api_snapshot(Api::is_disabled);
                let pointer_type = last_pointer_type.get_value();
                machine.send.run(button::Event::Click);
                if interactive {
                    emit_press(
                        callbacks.on_press,
                        press_event_from_click(&ev, pointer_type.unwrap_or(PointerType::Virtual)),
                    );
                }
                last_pointer_type.set_value(None);
            }
        >
            {move || {
                is_loading
                    .get()
                    .then(|| {
                        let loading_attrs = attr_map_to_leptos_inline_attrs(
                            machine.with_api_snapshot(loading_indicator_attrs),
                        );

                        view! { <span {..loading_attrs}></span> }
                    })
            }}
            <span {..content_attrs}>{children.into_inner()()}</span>
        </button>
    }
}

/// Leptos Button component that forwards root attrs to a typed consumer child.
#[component]
pub fn ButtonAsChild<T>(
    /// Optional component instance ID.
    #[prop(optional, into)]
    id: Option<String>,

    /// Whether the button is disabled.
    #[prop(optional, into)]
    disabled: Signal<bool>,

    /// Whether the button is in loading state.
    #[prop(optional, into)]
    loading: Signal<bool>,

    /// Visual style variant.
    #[prop(optional, into)]
    variant: Option<Variant>,

    /// Visual size token.
    #[prop(optional, into)]
    size: Option<Size>,

    /// Whether to remove the root from sequential tab navigation.
    #[prop(optional)]
    exclude_from_tab_order: bool,

    /// Consumer class tokens appended to the root.
    #[prop(optional, into)]
    class: Option<String>,

    /// Consumer inline style text applied to the root.
    #[prop(optional, into)]
    style: Option<String>,

    /// Accessible label applied to the root.
    #[prop(optional, into)]
    aria_label: Option<String>,

    /// Accessible label relationship applied to the root.
    #[prop(optional, into)]
    aria_labelledby: Option<String>,

    /// Typed child root that receives Button root attrs.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: AddAnyAttr,
    <View<T> as AddAnyAttr>::Output<Vec<LeptosAttribute>>: IntoView,
{
    let id = id.unwrap_or_else(|| use_id("button"));

    children.into_inner()().add_any_attr(
        AsChildAttrs::from(leptos_root_attrs(
            use_machine_with_reactive_props::<button::Machine>(props_signal(
                button::Props::new()
                    .id(&id)
                    .variant(variant.unwrap_or_default())
                    .size(size.unwrap_or_default())
                    .as_child(true)
                    .exclude_from_tab_order(exclude_from_tab_order),
                disabled,
                loading,
            )),
            UserRootAttrs {
                class,
                style,
                aria_label,
                aria_labelledby,
            },
            true,
        ))
        .into_inner(),
    )
}

fn props_signal(
    props: button::Props,
    disabled: Signal<bool>,
    loading: Signal<bool>,
) -> Signal<button::Props> {
    Signal::derive(move || {
        props
            .clone()
            .disabled(disabled.get())
            .loading(loading.get())
    })
}

#[derive(Clone, Copy, Debug, Default)]
struct PressCallbacks {
    on_press_start: Option<Callback<PressEvent>>,
    on_press_end: Option<Callback<PressEvent>>,
    on_press: Option<Callback<PressEvent>>,
    on_press_change: Option<Callback<bool>>,
    on_press_up: Option<Callback<PressEvent>>,
}

fn leptos_root_attrs(
    machine: crate::UseMachineReturn<button::Machine>,
    user_attrs: UserRootAttrs,
    filter_native: bool,
) -> Vec<LeptosAttribute> {
    let id = machine.service.with_value(|svc| svc.props().id.clone());

    let mut attrs = machine.with_api_snapshot(root_attrs);

    attrs.set(HtmlAttr::Id, id);

    apply_user_root_attrs(&mut attrs, &user_attrs);
    strip_dynamic_root_attrs(&mut attrs);
    add_dynamic_root_attrs(&mut attrs, machine);

    if filter_native {
        filter_native_button_attrs(&mut attrs);
    }

    let mut leptos_attrs = attr_map_to_leptos_inline_attrs(attrs);

    if let Some(style) = user_attrs.style {
        leptos_attrs.push(string_attr(String::from("style"), style));
    }

    leptos_attrs
}

fn leptos_content_attrs(machine: crate::UseMachineReturn<button::Machine>) -> Vec<LeptosAttribute> {
    let mut attrs = machine.with_api_snapshot(content_attrs);

    let loading = machine.derive(|api| {
        api.content_attrs()
            .get(&HtmlAttr::Data("ars-loading"))
            .unwrap_or("false")
            .to_owned()
    });

    attrs.set(
        HtmlAttr::Data("ars-loading"),
        AttrValue::reactive(move || loading.get()),
    );

    attr_map_to_leptos_inline_attrs(attrs)
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

fn strip_dynamic_root_attrs(attrs: &mut AttrMap) {
    for attr in [
        HtmlAttr::Data("ars-state"),
        HtmlAttr::Data("ars-loading"),
        HtmlAttr::Data("ars-disabled"),
        HtmlAttr::Data("ars-focus-visible"),
        HtmlAttr::Data("ars-pressed"),
        HtmlAttr::Disabled,
        HtmlAttr::Aria(AriaAttr::Busy),
        HtmlAttr::Aria(AriaAttr::Disabled),
    ] {
        attrs.set(attr, AttrValue::None);
    }
}

fn add_dynamic_root_attrs(attrs: &mut AttrMap, machine: crate::UseMachineReturn<button::Machine>) {
    let state = root_attr_string_memo(machine, HtmlAttr::Data("ars-state"));
    let loading = root_attr_bool_memo(machine, HtmlAttr::Data("ars-loading"));
    let disabled = root_attr_bool_memo(machine, HtmlAttr::Data("ars-disabled"));
    let focus_visible = root_attr_bool_memo(machine, HtmlAttr::Data("ars-focus-visible"));
    let pressed = root_attr_bool_memo(machine, HtmlAttr::Data("ars-pressed"));
    let html_disabled = root_attr_bool_memo(machine, HtmlAttr::Disabled);
    let busy = root_attr_bool_memo(machine, HtmlAttr::Aria(AriaAttr::Busy));
    let aria_disabled = root_attr_bool_memo(machine, HtmlAttr::Aria(AriaAttr::Disabled));

    attrs
        .set(
            HtmlAttr::Data("ars-state"),
            AttrValue::reactive(move || state.get()),
        )
        .set(
            HtmlAttr::Data("ars-loading"),
            AttrValue::reactive_bool(move || loading.get()),
        )
        .set(
            HtmlAttr::Data("ars-disabled"),
            AttrValue::reactive_bool(move || disabled.get()),
        )
        .set(
            HtmlAttr::Data("ars-focus-visible"),
            AttrValue::reactive_bool(move || focus_visible.get()),
        )
        .set(
            HtmlAttr::Data("ars-pressed"),
            AttrValue::reactive_bool(move || pressed.get()),
        )
        .set(
            HtmlAttr::Disabled,
            AttrValue::reactive_bool(move || html_disabled.get()),
        )
        .set(
            HtmlAttr::Aria(AriaAttr::Busy),
            AttrValue::reactive_bool(move || busy.get()),
        )
        .set(
            HtmlAttr::Aria(AriaAttr::Disabled),
            AttrValue::reactive_bool(move || aria_disabled.get()),
        );
}

fn root_attr_string_memo(
    machine: crate::UseMachineReturn<button::Machine>,
    attr: HtmlAttr,
) -> Memo<String> {
    machine.derive(move |api| {
        api.root_attrs()
            .get(&attr)
            .map(str::to_owned)
            .unwrap_or_default()
    })
}

fn root_attr_bool_memo(
    machine: crate::UseMachineReturn<button::Machine>,
    attr: HtmlAttr,
) -> Memo<bool> {
    machine.derive(move |api| api.root_attrs().contains(&attr))
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

fn emit_press(callback: Option<Callback<PressEvent>>, event: PressEvent) {
    if let Some(callback) = callback {
        callback.run(event);
    }
}

fn emit_bool(callback: Option<Callback<bool>>, value: bool) {
    if let Some(callback) = callback {
        callback.run(value);
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

fn pointer_type_from_leptos(pointer_type: &str) -> PointerType {
    match pointer_type {
        "mouse" => PointerType::Mouse,
        "touch" => PointerType::Touch,
        "pen" => PointerType::Pen,
        _ => PointerType::Virtual,
    }
}

fn key_modifiers_from_leptos_pointer(ev: &leptos::ev::PointerEvent) -> KeyModifiers {
    KeyModifiers {
        shift: ev.shift_key(),
        ctrl: ev.ctrl_key(),
        alt: ev.alt_key(),
        meta: ev.meta_key(),
    }
}

fn key_modifiers_from_leptos_mouse(ev: &leptos::ev::MouseEvent) -> KeyModifiers {
    KeyModifiers {
        shift: ev.shift_key(),
        ctrl: ev.ctrl_key(),
        alt: ev.alt_key(),
        meta: ev.meta_key(),
    }
}

fn press_event_from_click(ev: &leptos::ev::MouseEvent, pointer_type: PointerType) -> PressEvent {
    let (client_x, client_y) = if matches!(pointer_type, PointerType::Virtual) {
        (None, None)
    } else {
        (
            Some(f64::from(ev.client_x())),
            Some(f64::from(ev.client_y())),
        )
    };

    press_event_from_pointer(
        pointer_type,
        PressEventType::Press,
        client_x,
        client_y,
        key_modifiers_from_leptos_mouse(ev),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use ars_core::{Env, Service};

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
        assert!(!event.modifiers.ctrl);
        assert!(event.modifiers.alt);
        assert!(!event.modifiers.meta);
    }

    #[test]
    fn pointer_type_tokens_map_to_press_pointer_types() {
        assert_eq!(pointer_type_from_leptos("mouse"), PointerType::Mouse);
        assert_eq!(pointer_type_from_leptos("touch"), PointerType::Touch);
        assert_eq!(pointer_type_from_leptos("pen"), PointerType::Pen);
        assert_eq!(pointer_type_from_leptos(""), PointerType::Virtual);
        assert_eq!(pointer_type_from_leptos("unknown"), PointerType::Virtual);
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
        let press_count = Arc::new(AtomicUsize::new(0));
        let bool_count = Arc::new(AtomicUsize::new(0));

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

        assert_eq!(press_count.load(Ordering::SeqCst), 0);
        assert_eq!(bool_count.load(Ordering::SeqCst), 0);

        emit_press(
            Some(Callback::new({
                let press_count = Arc::clone(&press_count);
                move |event: PressEvent| {
                    assert_eq!(event.pointer_type, PointerType::Mouse);
                    assert_eq!(event.event_type, PressEventType::Press);
                    press_count.fetch_add(1, Ordering::SeqCst);
                }
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
            Some(Callback::new({
                let bool_count = Arc::clone(&bool_count);
                move |pressed: bool| {
                    assert!(pressed);
                    bool_count.fetch_add(1, Ordering::SeqCst);
                }
            })),
            true,
        );

        assert_eq!(press_count.load(Ordering::SeqCst), 1);
        assert_eq!(bool_count.load(Ordering::SeqCst), 1);
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
