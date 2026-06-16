//! Dioxus Checkbox adapter.
//!
//! This module renders the framework-agnostic Checkbox machine as Dioxus RSX,
//! preserving tri-state ARIA state, label wiring, and native form
//! participation through the hidden input part.

use ars_components::input::checkbox::Machine;
pub use ars_components::input::checkbox::{Event, Messages, Part, Props, State};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use ars_core::HtmlAttr;
use ars_forms::validation::Error;
use ars_interactions::KeyboardEventData;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use dioxus::events::MountedData;
use dioxus::{
    dioxus_core::{AttributeValue, DynamicNode, ListenerCallback, TemplateNode},
    prelude::*,
};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use web_sys::wasm_bindgen::JsCast as _;

use crate::{
    attr_map_to_dioxus_inline_attrs, event_mapping::dioxus_key_to_keyboard_key, merge_dioxus_attrs,
    use_machine_with_reactive_props, use_stable_id,
};

#[derive(Clone)]
struct CheckboxContext {
    machine: crate::UseMachineReturn<Machine>,
    on_checked_change: Option<EventHandler<State>>,
    last_pointer: Signal<bool>,
}

// adapter-context-glue: framework context lookup for compound checkbox parts.
fn checkbox_context() -> CheckboxContext {
    try_use_context::<CheckboxContext>()
        .expect("Checkbox subcomponents must be rendered inside <checkbox::Root/>")
}

const fn api_is_interactive(api: &ars_components::input::checkbox::Api<'_>) -> bool {
    api.is_interactive()
}

fn element_contains_component(element: &Element, component_path_suffix: &str) -> bool {
    element
        .as_ref()
        .is_ok_and(|node| vnode_contains_component(node, component_path_suffix))
}

fn vnode_contains_component(node: &VNode, component_path_suffix: &str) -> bool {
    node.template
        .roots
        .iter()
        .any(|root| template_node_contains_component(root, node, component_path_suffix))
}

fn template_node_contains_component(
    template_node: &TemplateNode,
    node: &VNode,
    component_path_suffix: &str,
) -> bool {
    match template_node {
        TemplateNode::Element { children, .. } => children
            .iter()
            .any(|child| template_node_contains_component(child, node, component_path_suffix)),
        TemplateNode::Dynamic { id } => {
            dynamic_node_contains_component(&node.dynamic_nodes[*id], component_path_suffix)
        }
        TemplateNode::Text { .. } => false,
    }
}

fn dynamic_node_contains_component(node: &DynamicNode, component_path_suffix: &str) -> bool {
    match node {
        DynamicNode::Component(component) => component.name.ends_with(component_path_suffix),
        DynamicNode::Fragment(nodes) => nodes
            .iter()
            .any(|node| vnode_contains_component(node, component_path_suffix)),
        DynamicNode::Text(_) | DynamicNode::Placeholder(_) => false,
    }
}

/// Props for the Dioxus [`Root`] compound checkbox part.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct RootProps {
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
    pub errors: Vec<Error>,

    /// Native form field name.
    #[props(optional, into)]
    pub name: Option<String>,

    /// Submitted value when checked. Defaults to `"on"`.
    #[props(optional, into)]
    pub value: Option<String>,

    /// Associated native form owner ID.
    #[props(optional, into)]
    pub form: Option<String>,

    /// Whether a description part is rendered in the initial tree.
    #[props(default = false)]
    pub has_description: bool,

    /// Whether an error message part is rendered in the initial tree.
    #[props(default = false)]
    pub has_error_message: bool,

    /// Fires after user intent requests a new checked state.
    #[props(optional, into)]
    pub on_checked_change: Option<EventHandler<State>>,

    /// Global HTML attributes forwarded onto the rendered root.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,

    /// Checkbox anatomy children.
    pub children: Element,
}

/// Dioxus compound checkbox root.
#[component]
pub fn Root(props: RootProps) -> Element {
    let generated_id = use_stable_id("checkbox");
    let id = props.id.unwrap_or(generated_id);

    let machine = use_checkbox_machine(build_core_props(CorePropsInput {
        id,
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
    }));

    let last_pointer = use_signal(|| false);

    let mut seeded_presence = use_signal(|| false);

    let has_description = props.has_description
        || element_contains_component(&props.children, "input::checkbox::Description");
    let has_error_message = props.has_error_message
        || element_contains_component(&props.children, "input::checkbox::ErrorMessage");

    if !*seeded_presence.peek() {
        machine.send.call(Event::SetHasDescription(has_description));

        machine
            .send
            .call(Event::SetHasErrorMessage(has_error_message));

        seeded_presence.set(true);
    }

    use_context_provider(|| CheckboxContext {
        machine,
        on_checked_change: props.on_checked_change,
        last_pointer,
    });

    let component_attrs = machine.derive(|api| attr_map_to_dioxus_inline_attrs(api.root_attrs()));
    let attrs = merge_dioxus_attrs(props.attrs, component_attrs());

    rsx! {
        div { ..attrs,{props.children} }
    }
}

/// Props for the Dioxus [`Label`] compound checkbox part.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct LabelProps {
    /// Global HTML attributes forwarded onto the rendered label.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,

    /// Label content.
    pub children: Element,
}

/// Dioxus compound checkbox label.
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Api method references are not general enough for UseMachineReturn snapshot callbacks."
)]
#[component]
pub fn Label(props: LabelProps) -> Element {
    let attrs = checkbox_context()
        .machine
        .part_attrs(props.attrs, |api| api.label_attrs());

    rsx! {
        label { ..attrs,{props.children} }
    }
}

/// Props for the Dioxus [`Control`] compound checkbox part.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct ControlProps {
    /// Global HTML attributes forwarded onto the rendered control.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,

    /// Control content.
    pub children: Element,
}

/// Dioxus compound checkbox control.
#[expect(
    unused_qualifications,
    reason = "rsx! macro expansion currently reports event-handler closures as unnecessary qualifications."
)]
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Api method references are not general enough for UseMachineReturn snapshot callbacks."
)]
#[component]
pub fn Control(props: ControlProps) -> Element {
    let CheckboxContext {
        machine,
        on_checked_change,
        mut last_pointer,
    } = checkbox_context();

    let mut attrs = machine.part_attrs(props.attrs, |api| api.control_attrs());
    let onclick_listeners = take_event_listeners(&mut attrs, "onclick");
    let onkeydown_listeners = take_event_listeners(&mut attrs, "onkeydown");
    let onpointerdown_listeners = take_event_listeners(&mut attrs, "onpointerdown");
    let onfocus_listeners = take_event_listeners(&mut attrs, "onfocus");
    let onblur_listeners = take_event_listeners(&mut attrs, "onblur");

    rsx! {
        div {
            onclick: move |ev| {
                let next = machine.with_api_snapshot(|api| api.next_toggle_state());
                let interactive = machine.with_api_snapshot(api_is_interactive);
                machine.send.call(Event::Toggle);
                if interactive && let Some(callback) = on_checked_change {
                    callback.call(next);
                }
                call_event_listeners(&onclick_listeners, &ev);
            },
            onkeydown: move |ev| {
                let (key, character) = dioxus_key_to_keyboard_key(&ev.key());
                let data = KeyboardEventData {
                    key,
                    character,
                    code: ev.code().to_string(),
                    shift_key: ev.modifiers().shift(),
                    ctrl_key: ev.modifiers().ctrl(),
                    alt_key: ev.modifiers().alt(),
                    meta_key: ev.modifiers().meta(),
                    repeat: ev.is_auto_repeating(),
                    is_composing: ev.is_composing(),
                };

                if data.key == ars_interactions::KeyboardKey::Space
                    && !data.repeat
                {
                    ev.prevent_default();

                    let next = machine.with_api_snapshot(|api| api.next_toggle_state());
                    let interactive = machine.with_api_snapshot(api_is_interactive);

                    machine.send.call(Event::Toggle);

                    if interactive && let Some(callback) = on_checked_change {
                        callback.call(next);
                    }
                }

                call_event_listeners(&onkeydown_listeners, &ev);
            },
            onpointerdown: move |ev| {
                last_pointer.set(true);
                call_event_listeners(&onpointerdown_listeners, &ev);
            },
            onfocus: move |ev| {
                let is_keyboard = !last_pointer();

                last_pointer.set(false);

                machine
                    .send
                    .call(Event::Focus {
                        is_keyboard,
                    });
                call_event_listeners(&onfocus_listeners, &ev);
            },
            onblur: move |ev| {
                last_pointer.set(false);
                machine.send.call(Event::Blur);
                call_event_listeners(&onblur_listeners, &ev);
            },
            ..attrs,
            {props.children}
        }
    }
}

/// Props for the Dioxus [`Indicator`] compound checkbox part.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct IndicatorProps {
    /// Global HTML attributes forwarded onto the rendered indicator.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,

    /// Optional indicator content.
    #[props(default)]
    pub children: Element,
}

/// Dioxus compound checkbox indicator.
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Api method references are not general enough for UseMachineReturn snapshot callbacks."
)]
#[component]
pub fn Indicator(props: IndicatorProps) -> Element {
    let attrs = checkbox_context()
        .machine
        .part_attrs(props.attrs, |api| api.indicator_attrs());

    rsx! {
        div { ..attrs,{props.children} }
    }
}

/// Props for the Dioxus [`HiddenInput`] compound checkbox part.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct HiddenInputProps {
    /// Global HTML attributes forwarded onto the rendered input.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,
}

/// Dioxus compound checkbox hidden input.
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Api method references are not general enough for UseMachineReturn snapshot callbacks."
)]
#[component]
pub fn HiddenInput(props: HiddenInputProps) -> Element {
    let CheckboxContext {
        machine,
        on_checked_change,
        ..
    } = checkbox_context();

    let attrs = strip_hidden_input_event_attrs(
        machine.part_attrs(props.attrs, |api| api.hidden_input_attrs()),
    );

    render_hidden_input(attrs, machine, on_checked_change)
}

fn strip_hidden_input_event_attrs(mut attrs: Vec<Attribute>) -> Vec<Attribute> {
    attrs.retain(|attr| !matches!(attr.name, "onchange" | "onmounted"));
    attrs
}

fn take_event_listeners(attrs: &mut Vec<Attribute>, name: &str) -> Vec<ListenerCallback> {
    let mut listeners = Vec::new();

    attrs.retain(|attr| {
        if attr.name == name
            && let AttributeValue::Listener(listener) = &attr.value
        {
            listeners.push(listener.clone());

            false
        } else {
            true
        }
    });

    listeners
}

fn call_event_listeners<T: 'static>(
    listeners: &[ListenerCallback],
    event: &dioxus::prelude::Event<T>,
) {
    for listener in listeners {
        listener.call(event.clone().into_any());
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
#[expect(
    unused_qualifications,
    reason = "rsx! macro expansion currently reports event-handler closures as unnecessary qualifications."
)]
fn render_hidden_input(
    attrs: Vec<Attribute>,
    machine: crate::UseMachineReturn<Machine>,
    on_checked_change: Option<EventHandler<State>>,
) -> Element {
    let mut form_reset_target = use_signal(|| None::<web_sys::EventTarget>);

    crate::use_safe_event_listener(form_reset_target, "reset", move |_| {
        let reset_request = machine.with_api_snapshot(|api| {
            (api.is_checked_controlled() && api.checked() != api.default_checked())
                .then(|| api.default_checked())
        });

        machine.send.call(Event::Reset);

        if let (Some(callback), Some(next)) = (on_checked_change, reset_request) {
            callback.call(next);
        }
    });

    rsx! {
        input {
            onmounted: move |event| {
                set_form_reset_target(&mut form_reset_target, event.data());
            },
            onchange: move |ev| {
                let checked = ev.checked();

                let next = State::from_checked_bool(checked);
                let interactive = machine.with_api_snapshot(api_is_interactive);

                machine.send.call(if checked { Event::Check } else { Event::Uncheck });
                sync_hidden_input_checked(machine);

                if interactive && let Some(callback) = on_checked_change {
                    callback.call(next);
                }
            },
            ..attrs,
        }
    }
}

#[cfg(not(all(feature = "web", target_arch = "wasm32")))]
#[expect(
    unused_qualifications,
    reason = "rsx! macro expansion currently reports event-handler closures as unnecessary qualifications."
)]
fn render_hidden_input(
    attrs: Vec<Attribute>,
    machine: crate::UseMachineReturn<Machine>,
    on_checked_change: Option<EventHandler<State>>,
) -> Element {
    rsx! {
        input {
            onchange: move |ev| {
                let checked = ev.checked();

                let next = State::from_checked_bool(checked);
                let interactive = machine.with_api_snapshot(api_is_interactive);

                machine.send.call(if checked { Event::Check } else { Event::Uncheck });

                if interactive && let Some(callback) = on_checked_change {
                    callback.call(next);
                }
            },
            ..attrs,
        }
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn set_form_reset_target(
    form_reset_target: &mut Signal<Option<web_sys::EventTarget>>,
    mounted: std::rc::Rc<MountedData>,
) {
    let target = mounted
        .downcast::<web_sys::HtmlInputElement>()
        .and_then(|input| input.form())
        .map(|form| form.unchecked_into::<web_sys::EventTarget>());

    form_reset_target.set(target);
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn sync_hidden_input_checked(machine: crate::UseMachineReturn<Machine>) {
    let (input_id, committed) = machine.with_api_snapshot(|api| {
        (
            api.hidden_input_attrs()
                .get(&HtmlAttr::Id)
                .map(str::to_owned),
            api.checked() == State::Checked,
        )
    });

    if let Some(input) = input_id
        .and_then(|id| web_sys::window()?.document()?.get_element_by_id(&id))
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
    {
        input.set_checked(committed);
    }
}

/// Props for the Dioxus [`Description`] compound checkbox part.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct DescriptionProps {
    /// Global HTML attributes forwarded onto the rendered description.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,

    /// Description content.
    pub children: Element,
}

/// Dioxus compound checkbox description.
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Api method references are not general enough for part attr callbacks."
)]
#[component]
pub fn Description(props: DescriptionProps) -> Element {
    let machine = checkbox_context().machine;
    let mut registered = use_signal(|| false);

    if !*registered.peek() {
        machine.send.call(Event::SetHasDescription(true));

        registered.set(true);
    }

    use_drop(move || {
        machine.send.call(Event::SetHasDescription(false));
    });

    let attrs = machine.part_attrs(props.attrs, |api| api.description_attrs());

    rsx! {
        div { ..attrs,{props.children} }
    }
}

/// Props for the Dioxus [`ErrorMessage`] compound checkbox part.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct ErrorMessageProps {
    /// Global HTML attributes forwarded onto the rendered error message.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,

    /// Error message content.
    pub children: Element,
}

/// Dioxus compound checkbox error message.
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "Api method references are not general enough for part attr callbacks."
)]
#[component]
pub fn ErrorMessage(props: ErrorMessageProps) -> Element {
    let CheckboxContext { machine, .. } = checkbox_context();

    let mut registered = use_signal(|| false);

    if !*registered.peek() {
        machine.send.call(Event::SetHasErrorMessage(true));

        registered.set(true);
    }

    use_drop(move || {
        machine.send.call(Event::SetHasErrorMessage(false));
    });

    let attrs = machine.part_attrs(props.attrs, |api| api.error_message_attrs());

    rsx! {
        div { ..attrs,{props.children} }
    }
}

struct CorePropsInput {
    id: String,
    checked: Option<State>,
    default_checked: State,
    disabled: bool,
    readonly: bool,
    required: bool,
    invalid: bool,
    errors: Vec<Error>,
    name: Option<String>,
    value: Option<String>,
    form: Option<String>,
}

fn build_core_props(
    CorePropsInput {
        id,
        checked,
        default_checked,
        disabled,
        readonly,
        required,
        invalid,
        errors,
        name,
        value,
        form,
    }: CorePropsInput,
) -> Props {
    let field_support = crate::utility::field_support::use_field_support(
        disabled,
        invalid,
        readonly,
        errors,
        name.as_deref(),
    );

    let mut core_props = Props::new()
        .id(id)
        .default_checked(default_checked)
        .disabled(field_support.disabled)
        .readonly(field_support.readonly)
        .required(required)
        .invalid(field_support.invalid)
        .errors(field_support.errors);

    if let Some(checked) = checked {
        core_props = core_props.checked(checked);
    }

    if let Some(name) = name {
        core_props = core_props.name(name);
    }

    if let Some(value) = value {
        core_props = core_props.value(value);
    }

    if let Some(form) = form {
        core_props = core_props.form(form);
    }

    core_props
}

fn use_checkbox_machine(core_props: Props) -> crate::UseMachineReturn<Machine> {
    let mut props_signal = use_signal(|| core_props.clone());

    if *props_signal.peek() != core_props {
        props_signal.set(core_props);
    }

    use_machine_with_reactive_props::<Machine>(props_signal)
}
