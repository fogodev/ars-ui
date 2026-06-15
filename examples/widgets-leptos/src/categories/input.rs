use ars_leptos::prelude::*;

#[derive(Clone, Debug, Translate)]
#[translate(fallback = "en-US")]
pub(crate) enum InputText {
    #[translate(en_US = "", pt_BR = "")]
    FormStatusEmpty,

    #[translate(en_US = "Checkbox", pt_BR = "Checkbox")]
    Checkbox,

    #[translate(en_US = "Checkbox states", pt_BR = "Estados de checkbox")]
    CheckboxStates,

    #[translate(en_US = "Forms", pt_BR = "Formulários")]
    Forms,

    #[translate(en_US = "Unchecked", pt_BR = "Desmarcado")]
    Unchecked,

    #[translate(en_US = "Checked", pt_BR = "Marcado")]
    Checked,

    #[translate(en_US = "Indeterminate", pt_BR = "Indeterminado")]
    Indeterminate,

    #[translate(en_US = "Disabled", pt_BR = "Desabilitado")]
    Disabled,

    #[translate(en_US = "Readonly", pt_BR = "Somente leitura")]
    Readonly,

    #[translate(en_US = "Required", pt_BR = "Obrigatório")]
    Required,

    #[translate(
        en_US = "Invalid with help text",
        pt_BR = "Inválido com texto de ajuda"
    )]
    InvalidWithHelpText,

    #[translate(
        en_US = "Used for notifications and billing alerts.",
        pt_BR = "Usado para notificações e alertas de cobrança."
    )]
    InvalidDescription,

    #[translate(
        en_US = "Choose this option before continuing.",
        pt_BR = "Escolha esta opção antes de continuar."
    )]
    InvalidError,

    #[translate(en_US = "Controlled", pt_BR = "Controlado")]
    Controlled,

    #[translate(en_US = "Controlled checkbox", pt_BR = "Checkbox controlado")]
    ControlledCheckbox,

    #[translate(en_US = "Current state", pt_BR = "Estado atual")]
    CurrentState,

    #[translate(
        en_US = "Optional newsletter value",
        pt_BR = "Valor opcional de newsletter"
    )]
    OptionalNewsletter,

    #[translate(en_US = "Required checked value", pt_BR = "Valor marcado obrigatório")]
    RequiredCheckedValue,

    #[translate(en_US = "Submit", pt_BR = "Enviar")]
    Submit,

    #[translate(en_US = "Reset", pt_BR = "Redefinir")]
    Reset,

    #[translate(
        en_US = "Select the required value before submitting.",
        pt_BR = "Selecione o valor obrigatório antes de enviar."
    )]
    SubmitError,

    #[translate(
        en_US = "Submitted: newsletter=weekly; terms=accepted",
        pt_BR = "Enviado: newsletter=semanal; termos=aceitos"
    )]
    SubmittedWeekly,

    #[translate(
        en_US = "Submitted: newsletter=none; terms=accepted",
        pt_BR = "Enviado: newsletter=nenhuma; termos=aceitos"
    )]
    SubmittedNone,

    #[translate(en_US = "Form reset.", pt_BR = "Formulário redefinido.")]
    FormReset,
}

#[component]
fn Checkbox<T>(
    #[prop(optional, into)] id: Option<Oco<'static, str>>,
    #[prop(optional, into)] checked: Option<Signal<checkbox::State>>,
    #[prop(optional, default = checkbox::State::Unchecked)] default_checked: checkbox::State,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] readonly: Signal<bool>,
    #[prop(optional, into)] required: Signal<bool>,
    #[prop(optional, into)] invalid: Signal<bool>,
    #[prop(optional, into)] name: Option<Oco<'static, str>>,
    #[prop(optional, into)] value: Option<Oco<'static, str>>,
    #[prop(optional, into)] description: Option<ViewFn>,
    #[prop(optional, into)] error_message: Option<ViewFn>,
    #[prop(optional, into, default = Callback::new(|_| ()))] on_checked_change: Callback<
        checkbox::State,
    >,
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
            name=name.unwrap_or_default()
            value=value.unwrap_or(Oco::Borrowed("on"))
            has_description=description.is_some()
            has_error_message=error_message.is_some()
            on_checked_change=on_checked_change
        >
            <checkbox::Label>{label()}</checkbox::Label>
            <checkbox::Control>
                <checkbox::Indicator />
            </checkbox::Control>
            <checkbox::HiddenInput />
            {description
                .map(|description| {
                    view! { <checkbox::Description>{description.run()}</checkbox::Description> }
                })}
            {error_message
                .map(|error_message| {
                    view! { <checkbox::ErrorMessage>{error_message.run()}</checkbox::ErrorMessage> }
                })}
        </checkbox::Root>
    }
}

#[component]
pub(crate) fn InputPanel() -> impl IntoView {
    let (controlled, set_controlled) = signal(checkbox::State::Indeterminate);
    let (invalid_checked, set_invalid_checked) = signal(checkbox::State::Unchecked);
    let (form_newsletter, set_form_newsletter) = signal(checkbox::State::Unchecked);
    let (form_required, set_form_required) = signal(checkbox::State::Checked);
    let (form_submit_attempted, set_form_submit_attempted) = signal(false);
    let (form_status, set_form_status) = signal(InputText::FormStatusEmpty);
    let form_status_text = t(form_status);
    let required_invalid =
        move || form_submit_attempted.get() && form_required.get() != checkbox::State::Checked;

    view! {
        <section class="showcase-panel wide">
            <h2>{t(InputText::Checkbox)}</h2>
            <div class="showcase-grid">
                <div class="showcase-card">
                    <h3>{t(InputText::CheckboxStates)}</h3>
                    <Checkbox id="checkbox-unchecked" name="unchecked">
                        {t(InputText::Unchecked)}
                    </Checkbox>
                    <Checkbox
                        id="checkbox-checked"
                        default_checked=checkbox::State::Checked
                        name="checked"
                    >
                        {t(InputText::Checked)}
                    </Checkbox>
                    <Checkbox
                        id="checkbox-indeterminate"
                        default_checked=checkbox::State::Indeterminate
                        name="indeterminate"
                    >
                        {t(InputText::Indeterminate)}
                    </Checkbox>
                    <Checkbox id="checkbox-disabled" disabled=true>
                        {t(InputText::Disabled)}
                    </Checkbox>
                    <Checkbox
                        id="checkbox-readonly"
                        readonly=true
                        default_checked=checkbox::State::Checked
                    >
                        {t(InputText::Readonly)}
                    </Checkbox>
                    <Checkbox id="checkbox-required" required=true>
                        {t(InputText::Required)}
                    </Checkbox>
                    <Checkbox
                        id="checkbox-invalid"
                        checked=invalid_checked
                        invalid=move || invalid_checked.get() != checkbox::State::Checked
                        description=|| view! { {t(InputText::InvalidDescription)} }
                        error_message=move || {
                            view! {
                                <Show when=move || {
                                    invalid_checked.get() != checkbox::State::Checked
                                }>{t(InputText::InvalidError)}</Show>
                            }
                        }
                        on_checked_change=move |next| set_invalid_checked.set(next)
                    >
                        {t(InputText::InvalidWithHelpText)}
                    </Checkbox>
                </div>
                <div class="showcase-card">
                    <h3>{t(InputText::Controlled)}</h3>
                    <Checkbox
                        id="checkbox-controlled"
                        checked=controlled
                        on_checked_change=move |next| set_controlled.set(next)
                    >
                        {t(InputText::ControlledCheckbox)}
                    </Checkbox>
                    <p>
                        {move || {
                            format!("{}: {:?}", t(InputText::CurrentState).get(), controlled.get())
                        }}
                    </p>
                </div>
                <Form
                    class="showcase-card"
                    id="checkbox-demo-form"
                    on_submit=move |()| {
                        set_form_submit_attempted.set(true);
                        set_form_status
                            .set(
                                if form_required.get() != checkbox::State::Checked {
                                    InputText::SubmitError
                                } else if form_newsletter.get() == checkbox::State::Checked {
                                    InputText::SubmittedWeekly
                                } else {
                                    InputText::SubmittedNone
                                },
                            );
                    }
                    on_reset=move |()| {
                        set_form_newsletter.set(checkbox::State::Unchecked);
                        set_form_required.set(checkbox::State::Checked);
                        set_form_submit_attempted.set(false);
                        set_form_status.set(InputText::FormReset);
                    }
                >
                    <h3>{t(InputText::Forms)}</h3>
                    <Checkbox
                        id="checkbox-form-uncontrolled"
                        name="newsletter"
                        value="weekly"
                        checked=form_newsletter
                        on_checked_change=move |next| set_form_newsletter.set(next)
                    >
                        {t(InputText::OptionalNewsletter)}
                    </Checkbox>
                    <Checkbox
                        id="checkbox-form-checked"
                        name="terms"
                        checked=form_required
                        invalid=required_invalid
                        error_message=move || {
                            view! { <Show when=required_invalid>{t(InputText::SubmitError)}</Show> }
                        }
                        on_checked_change=move |next| set_form_required.set(next)
                    >
                        {t(InputText::RequiredCheckedValue)}
                    </Checkbox>
                    <div
                        class="checkbox-form-actions"
                        style="display: flex; flex-wrap: wrap; align-items: center; gap: 0.75rem;"
                    >
                        <Button r#type=button::Type::Submit size=button::Size::Sm>
                            {t(InputText::Submit)}
                        </Button>
                        <Button r#type=button::Type::Reset size=button::Size::Sm>
                            {t(InputText::Reset)}
                        </Button>
                    </div>
                    <p>{form_status_text}</p>
                </Form>
            </div>
        </section>
    }
}
