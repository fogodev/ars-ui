//! Leptos Fieldset adapter.

use ars_components::utility::fieldset;
pub use ars_components::utility::fieldset::{Part, Props};
use ars_core::Direction;
use leptos::{children::TypedChildren, context::Provider, prelude::*};

use crate::{attr_map_to_leptos_inline_attrs, use_id, use_machine};

#[derive(Clone, Copy)]
struct FieldsetContext {
    machine: crate::UseMachineReturn<fieldset::Machine>,
}

#[derive(Clone, Copy)]
pub(crate) struct InheritedFieldsetContext {
    pub(crate) disabled: Signal<bool>,
    pub(crate) invalid: Signal<bool>,
    pub(crate) readonly: Signal<bool>,
}

fn fieldset_context() -> FieldsetContext {
    use_context::<FieldsetContext>()
        .expect("Fieldset subcomponents must be rendered inside <Fieldset/>")
}

/// Leptos Fieldset root component.
#[component]
pub fn Fieldset<T: 'static>(
    /// Optional component instance ID.
    #[prop(optional, into)]
    id: Option<Oco<'static, str>>,

    /// Whether every descendant form control is disabled.
    #[prop(optional, into)]
    disabled: Signal<bool>,

    /// Whether the fieldset is invalid.
    #[prop(optional, into)]
    invalid: Signal<bool>,

    /// Whether the fieldset is read-only.
    #[prop(optional, into)]
    readonly: Signal<bool>,

    /// Optional text direction override.
    #[prop(optional)]
    dir: Option<Direction>,

    /// Consumer class tokens appended to the fieldset.
    #[prop(optional, into)]
    class: Option<TextProp>,

    /// Fieldset anatomy children.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let id = id.map_or_else(|| use_id("fieldset"), Oco::into_owned);

    let mut props = Props::new().id(&id);

    if let Some(dir) = dir {
        props = props.dir(dir);
    }

    let initial_props = props
        .disabled(disabled.get_untracked())
        .invalid(invalid.get_untracked())
        .readonly(readonly.get_untracked());

    let machine = use_machine::<fieldset::Machine>(initial_props);

    let inherited = InheritedFieldsetContext {
        disabled,
        invalid,
        readonly,
    };

    let attrs = machine.with_api_snapshot(|api| {
        let mut attrs = api.root_attrs();

        crate::merge_consumer_class_prop_into(&mut attrs, class);
        add_dynamic_root_attrs(&mut attrs, disabled, machine);

        attr_map_to_leptos_inline_attrs(attrs)
    });

    view! {
        <Provider value=FieldsetContext { machine }>
            <Provider value=inherited>
                <fieldset {..attrs}>{children.into_inner()()}</fieldset>
            </Provider>
        </Provider>
    }
}

/// Leptos Fieldset legend part.
#[component]
pub fn Legend<T>(
    /// Legend content.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let attrs = fieldset_context()
        .machine
        .with_api_snapshot(|api| attr_map_to_leptos_inline_attrs(api.legend_attrs()));

    view! { <legend {..attrs}>{children.into_inner()()}</legend> }
}

/// Leptos Fieldset description part.
#[component]
pub fn Description<T>(
    /// Description content.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let machine = fieldset_context().machine;

    machine.send.run(fieldset::Event::SetHasDescription(true));

    on_cleanup(move || {
        machine.send.run(fieldset::Event::SetHasDescription(false));
    });

    let attrs = fieldset_context()
        .machine
        .with_api_snapshot(|api| attr_map_to_leptos_inline_attrs(api.description_attrs()));

    view! { <div {..attrs}>{children.into_inner()()}</div> }
}

fn add_dynamic_root_attrs(
    attrs: &mut ars_core::AttrMap,
    disabled: Signal<bool>,
    machine: crate::UseMachineReturn<fieldset::Machine>,
) {
    let described_by = machine.derive(|api| {
        api.root_attrs()
            .get(&ars_core::HtmlAttr::Aria(ars_core::AriaAttr::DescribedBy))
            .map(str::to_owned)
    });

    attrs
        .set(
            ars_core::HtmlAttr::Disabled,
            ars_core::AttrValue::reactive_bool(move || disabled.get()),
        )
        .set(
            ars_core::HtmlAttr::Aria(ars_core::AriaAttr::DescribedBy),
            ars_core::AttrValue::reactive_optional(move || described_by.get()),
        );
}

/// Leptos Fieldset error message part.
#[component]
pub fn ErrorMessage<T>(
    /// Error message content.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let attrs = fieldset_context()
        .machine
        .with_api_snapshot(|api| attr_map_to_leptos_inline_attrs(api.error_message_attrs()));

    view! { <div {..attrs}>{children.into_inner()()}</div> }
}

/// Leptos Fieldset content part.
#[component]
pub fn Content<T>(
    /// Descendant form controls.
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    let attrs = fieldset_context()
        .machine
        .with_api_snapshot(|api| attr_map_to_leptos_inline_attrs(api.content_attrs()));

    view! { <div {..attrs}>{children.into_inner()()}</div> }
}
