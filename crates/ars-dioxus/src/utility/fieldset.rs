//! Dioxus Fieldset adapter.

use ars_components::utility::fieldset;
pub use ars_components::utility::fieldset::{Part, Props};
use ars_core::{Direction, HtmlAttr};
use ars_forms::validation::Error;
use dioxus::prelude::*;

use crate::{attr_map_to_dioxus_inline_attrs, merge_dioxus_attrs, use_machine, use_stable_id};

#[derive(Clone, Copy)]
struct FieldsetContext {
    machine: crate::UseMachineReturn<fieldset::Machine>,
}

#[derive(Clone, Copy)]
pub(crate) struct InheritedFieldsetContext {
    pub(crate) disabled: Memo<bool>,
    pub(crate) invalid: Memo<bool>,
    pub(crate) readonly: Memo<bool>,
}

fn fieldset_context() -> FieldsetContext {
    try_use_context::<FieldsetContext>()
        .expect("Fieldset subcomponents must be rendered inside <Fieldset/>")
}

/// Props for the Dioxus [`Fieldset`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct FieldsetProps {
    /// Optional component instance ID.
    #[props(optional, into)]
    pub id: Option<String>,

    /// Whether every descendant form control is disabled.
    #[props(default = false)]
    pub disabled: bool,

    /// Whether the fieldset is invalid.
    #[props(default = false)]
    pub invalid: bool,

    /// Whether the fieldset is read-only.
    #[props(default = false)]
    pub readonly: bool,

    /// Fieldset-level validation errors.
    #[props(default)]
    pub errors: Vec<Error>,

    /// Optional text direction override.
    #[props(optional)]
    pub dir: Option<Direction>,

    /// Global HTML attributes forwarded onto the fieldset.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,

    /// Fieldset anatomy children.
    pub children: Element,
}

/// Dioxus Fieldset root component.
#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "fieldset::Api method items are not lifetime-general enough for derive()."
)]
#[component]
pub fn Fieldset(props: FieldsetProps) -> Element {
    let generated_id = use_stable_id("fieldset");
    let id = props.id.unwrap_or(generated_id);

    let mut core_props = Props::new()
        .id(&id)
        .disabled(props.disabled)
        .invalid(props.invalid)
        .readonly(props.readonly)
        .errors(props.errors);

    if let Some(dir) = props.dir {
        core_props = core_props.dir(dir);
    }

    let machine = use_machine::<fieldset::Machine>(core_props);

    let inherited_disabled = machine.derive(|api| api.root_attrs().contains(&HtmlAttr::Disabled));
    let inherited_invalid = machine.derive(|api| api.is_invalid());
    let inherited_readonly = machine.derive(|api| api.is_readonly());

    use_context_provider(|| FieldsetContext { machine });
    use_context_provider(|| InheritedFieldsetContext {
        disabled: inherited_disabled,
        invalid: inherited_invalid,
        readonly: inherited_readonly,
    });

    let component_attrs = machine.derive(|api| attr_map_to_dioxus_inline_attrs(api.root_attrs()));
    let attrs = merge_dioxus_attrs(props.attrs, component_attrs());

    rsx! {
        fieldset { ..attrs,{props.children} }
    }
}

/// Props for the Dioxus [`Legend`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct LegendProps {
    /// Legend content.
    pub children: Element,
}

/// Dioxus Fieldset legend part.
#[component]
pub fn Legend(props: LegendProps) -> Element {
    let attrs = fieldset_context()
        .machine
        .derive(|api| attr_map_to_dioxus_inline_attrs(api.legend_attrs()))();

    rsx! {
        legend { ..attrs,{props.children} }
    }
}

/// Props for the Dioxus [`Description`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct DescriptionProps {
    /// Description content.
    pub children: Element,
}

/// Dioxus Fieldset description part.
#[component]
pub fn Description(props: DescriptionProps) -> Element {
    let machine = fieldset_context().machine;
    let mut registered = use_signal(|| false);

    if !*registered.peek() {
        machine.send.call(fieldset::Event::SetHasDescription(true));
        registered.set(true);
    }

    use_drop(move || {
        machine.send.call(fieldset::Event::SetHasDescription(false));
    });

    let attrs = machine.derive(|api| attr_map_to_dioxus_inline_attrs(api.description_attrs()))();

    rsx! {
        div { ..attrs,{props.children} }
    }
}

/// Props for the Dioxus [`ErrorMessage`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct ErrorMessageProps {
    /// Error message content.
    pub children: Element,
}

/// Dioxus Fieldset error message part.
#[component]
pub fn ErrorMessage(props: ErrorMessageProps) -> Element {
    let attrs = fieldset_context()
        .machine
        .derive(|api| attr_map_to_dioxus_inline_attrs(api.error_message_attrs()))();

    rsx! {
        div { ..attrs,{props.children} }
    }
}

/// Props for the Dioxus [`Content`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct ContentProps {
    /// Descendant form controls.
    pub children: Element,
}

/// Dioxus Fieldset content part.
#[component]
pub fn Content(props: ContentProps) -> Element {
    let attrs = fieldset_context()
        .machine
        .derive(|api| attr_map_to_dioxus_inline_attrs(api.content_attrs()))();

    rsx! {
        div { ..attrs,{props.children} }
    }
}
