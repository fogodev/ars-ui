//! Tailwind styled Leptos Checkbox.

use ars_leptos::prelude::*;
pub use checkbox::State;

/// Leptos Checkbox component styled with Tailwind utility classes.
#[component]
pub fn Checkbox<T>(
    /// Optional component instance ID.
    #[prop(optional, into)]
    id: Option<Oco<'static, str>>,

    /// Controlled checked state.
    #[prop(optional, into)]
    checked: Option<Signal<State>>,

    /// Initial checked state for uncontrolled usage.
    #[prop(optional, default = State::Unchecked)]
    default_checked: State,

    /// Whether the checkbox is disabled.
    #[prop(optional, into)]
    disabled: Signal<bool>,

    /// Whether the checkbox is readonly.
    #[prop(optional, into)]
    readonly: Signal<bool>,

    /// Whether the checkbox is required for form submission.
    #[prop(optional, into)]
    required: Signal<bool>,

    /// Whether the checkbox is invalid.
    #[prop(optional, into)]
    invalid: Signal<bool>,

    /// Validation errors associated with the checkbox.
    #[prop(optional, into)]
    errors: Signal<Vec<ValidationError>>,

    /// Native form field name.
    #[prop(optional, into)]
    name: Option<Oco<'static, str>>,

    /// Submitted value when checked. Defaults to `"on"`.
    #[prop(optional, into)]
    value: Option<Oco<'static, str>>,

    /// Associated native form owner ID.
    #[prop(optional, into)]
    form: Option<Oco<'static, str>>,

    /// Consumer class tokens appended to the root.
    #[prop(optional, into)]
    class: Option<TextProp>,

    /// Consumer inline style text applied to the root.
    #[prop(optional, into)]
    style: Option<TextProp>,

    /// Optional descriptive content.
    #[prop(optional, into)]
    description: Option<ViewFn>,

    /// Optional validation error content.
    #[prop(optional, into)]
    error_message: Option<ViewFn>,

    /// Fires after user intent requests a new checked state.
    #[prop(optional, into, default = Callback::new(|_| ()))]
    on_checked_change: Callback<State>,

    /// Visible label content.
    children: TypedChildren<T>,
) -> impl IntoView
where
    T: IntoView + 'static,
{
    let label = children.into_inner();

    view! {
        <checkbox::Root
            id=id.unwrap_or_else(|| use_id("checkbox").into())
            checked
            default_checked
            disabled
            readonly
            required
            invalid
            errors
            name=name.unwrap_or_default()
            value=value.unwrap_or(Oco::Borrowed("on"))
            form=form.unwrap_or_default()
            has_description=description.is_some()
            has_error_message=error_message.is_some()
            class=root_class(
                "group my-2 grid grid-cols-[1.125rem_minmax(0,1fr)] items-center gap-x-2.5 gap-y-1 data-ars-disabled:opacity-50",
                class,
            )
            style=style.unwrap_or_default()
            on_checked_change
        >
            <checkbox::Label class="col-start-2 cursor-pointer">{label()}</checkbox::Label>
            <checkbox::Control class="inline-flex col-start-1 row-start-1 justify-center items-center text-white bg-white rounded border-2 box-border h-4.5 w-4.5 border-slate-500 group-data-ars-invalid:border-red-600 group-data-ars-disabled:opacity-50 group-data-[ars-state=checked]:border-blue-600 group-data-[ars-state=checked]:bg-blue-600 group-data-[ars-state=indeterminate]:border-blue-600 group-data-[ars-state=indeterminate]:bg-blue-600 group-data-ars-invalid:group-data-[ars-state=checked]:border-red-600 group-data-ars-invalid:group-data-[ars-state=checked]:bg-red-600 group-data-ars-invalid:group-data-[ars-state=indeterminate]:border-red-600 group-data-ars-invalid:group-data-[ars-state=indeterminate]:bg-red-600">
                <checkbox::Indicator class="after:block group-data-[ars-state=checked]:after:h-[0.65rem] group-data-[ars-state=checked]:after:w-[0.35rem] group-data-[ars-state=checked]:after:rotate-45 group-data-[ars-state=checked]:after:-translate-x-px group-data-[ars-state=checked]:after:-translate-y-px group-data-[ars-state=checked]:after:border-b-2 group-data-[ars-state=checked]:after:border-r-2 group-data-[ars-state=checked]:after:border-current group-data-[ars-state=indeterminate]:after:h-0.5 group-data-[ars-state=indeterminate]:after:w-[0.65rem] group-data-[ars-state=indeterminate]:after:rounded-full group-data-[ars-state=indeterminate]:after:bg-current" />
            </checkbox::Control>
            <checkbox::HiddenInput />
            {description
                .map(|description| {
                    view! {
                        <checkbox::Description class="col-start-2 text-[0.9rem]">
                            {description.run()}
                        </checkbox::Description>
                    }
                })}
            {error_message
                .map(|error_message| {
                    view! {
                        <checkbox::ErrorMessage class="col-start-2 text-red-700 text-[0.9rem]">
                            {error_message.run()}
                        </checkbox::ErrorMessage>
                    }
                })}
        </checkbox::Root>
    }
}
