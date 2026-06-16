use ars_dioxus::prelude::*;
use ars_dioxus_components::input::checkbox::tailwind::{Checkbox, State};

#[derive(Clone, Debug, Translate, PartialEq)]
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
pub(crate) fn InputPanel() -> Element {
    let mut controlled = use_signal(|| State::Indeterminate);
    let mut invalid_checked = use_signal(|| State::Unchecked);
    let mut form_newsletter = use_signal(|| State::Unchecked);
    let mut form_required = use_signal(|| State::Checked);
    let mut form_submit_attempted = use_signal(|| false);
    let mut form_status = use_signal(|| InputText::FormStatusEmpty);

    let required_invalid = form_submit_attempted() && form_required() != State::Checked;

    rsx! {
        section { class: "showcase-panel wide",
            h2 { {t(InputText::Checkbox)} }
            div { class: "showcase-grid",
                div { class: "showcase-card",
                    h3 { {t(InputText::CheckboxStates)} }
                    Checkbox { id: "checkbox-unchecked", name: "unchecked", {t(InputText::Unchecked)} }
                    Checkbox {
                        id: "checkbox-checked",
                        default_checked: State::Checked,
                        name: "checked",
                        {t(InputText::Checked)}
                    }
                    Checkbox {
                        id: "checkbox-indeterminate",
                        default_checked: State::Indeterminate,
                        name: "indeterminate",
                        {t(InputText::Indeterminate)}
                    }
                    Checkbox { id: "checkbox-disabled", disabled: true, {t(InputText::Disabled)} }
                    Checkbox {
                        id: "checkbox-readonly",
                        readonly: true,
                        default_checked: State::Checked,
                        {t(InputText::Readonly)}
                    }
                    Checkbox { id: "checkbox-required", required: true, {t(InputText::Required)} }
                    Checkbox {
                        id: "checkbox-invalid",
                        checked: invalid_checked(),
                        invalid: invalid_checked() != State::Checked,
                        description: rsx! {
                            {t(InputText::InvalidDescription)}
                        },
                        error_message: (invalid_checked() != State::Checked).then(|| rsx! {
                            {t(InputText::InvalidError)}
                        }),
                        on_checked_change: move |next| invalid_checked.set(next),
                        {t(InputText::InvalidWithHelpText)}
                    }
                }
                div { class: "showcase-card",
                    h3 { {t(InputText::Controlled)} }
                    Checkbox {
                        id: "checkbox-controlled",
                        checked: controlled(),
                        on_checked_change: move |next| controlled.set(next),
                        {t(InputText::ControlledCheckbox)}
                    }
                    p { "{t(InputText::CurrentState)}: {controlled():?}" }
                }
                form::Root {
                    class: "showcase-card",
                    id: "checkbox-demo-form",
                    on_submit: move |_| {
                        form_submit_attempted.set(true);
                        let next_status = if form_required() != State::Checked {
                            InputText::SubmitError
                        } else if form_newsletter() == State::Checked {
                            InputText::SubmittedWeekly
                        } else {
                            InputText::SubmittedNone
                        };

                        form_status.set(next_status);
                    },
                    on_reset: move |_| {
                        form_newsletter.set(State::Unchecked);
                        form_required.set(State::Checked);
                        form_submit_attempted.set(false);
                        form_status.set(InputText::FormReset);
                    },
                    h3 { {t(InputText::Forms)} }
                    Checkbox {
                        id: "checkbox-form-uncontrolled",
                        name: "newsletter",
                        value: "weekly",
                        checked: form_newsletter(),
                        on_checked_change: move |next| form_newsletter.set(next),
                        {t(InputText::OptionalNewsletter)}
                    }
                    Checkbox {
                        id: "checkbox-form-checked",
                        name: "terms",
                        checked: form_required(),
                        default_checked: State::Checked,
                        invalid: required_invalid,
                        error_message: required_invalid.then(|| rsx! {
                            {t(InputText::SubmitError)}
                        }),
                        on_checked_change: move |next| form_required.set(next),
                        {t(InputText::RequiredCheckedValue)}
                    }
                    div { class: "mt-2 flex flex-wrap items-center gap-3",
                        Button {
                            r#type: button::Type::Submit,
                            size: button::Size::Sm,
                            {t(InputText::Submit)}
                        }
                        Button {
                            r#type: button::Type::Reset,
                            size: button::Size::Sm,
                            {t(InputText::Reset)}
                        }
                    }
                    p { {t(form_status)} }
                }
            }
        }
    }
}
