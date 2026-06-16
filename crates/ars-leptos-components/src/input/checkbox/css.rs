//! CSS-class styled Leptos Checkbox.

use ars_leptos::prelude::*;
pub use checkbox::State;

/// Stylesheet for the CSS Checkbox variant.
pub const STYLES: &str = include_str!("checkbox.css");

/// Leptos Checkbox component styled with stable CSS classes.
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
            class=root_class("ars-checkbox", class)
            style=style.unwrap_or_default()
            on_checked_change
        >
            <checkbox::Label class="ars-checkbox__label">{label()}</checkbox::Label>
            <checkbox::Control class="ars-checkbox__control">
                <checkbox::Indicator class="ars-checkbox__indicator" />
            </checkbox::Control>
            <checkbox::HiddenInput />
            {description
                .map(|description| {
                    view! {
                        <checkbox::Description class="ars-checkbox__description">
                            {description.run()}
                        </checkbox::Description>
                    }
                })}
            {error_message
                .map(|error_message| {
                    view! {
                        <checkbox::ErrorMessage class="ars-checkbox__error-message">
                            {error_message.run()}
                        </checkbox::ErrorMessage>
                    }
                })}
        </checkbox::Root>
    }
}
