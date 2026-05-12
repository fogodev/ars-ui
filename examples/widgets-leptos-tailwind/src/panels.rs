use std::fmt::{self, Display};

use ars_leptos::{
    navigation::tabs::{Tab, Tabs},
    prelude::t,
    utility::{
        button::{self, Button, ButtonAsChild},
        dismissable,
        error_boundary::Boundary,
    },
};
use leptos::prelude::*;

use crate::text::{NavigationTab, WidgetsText};

#[derive(Debug)]
struct ExampleError(Signal<String>);

impl Display for ExampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0.get())
    }
}

impl std::error::Error for ExampleError {}

fn example_error(message: Signal<String>) -> Result<&'static str, ExampleError> {
    Err(ExampleError(message))
}

#[component]
pub(crate) fn EmptyPanel(message: WidgetsText) -> impl IntoView {
    view! {
        <section class="p-5 mt-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10">
            <p class="text-sm text-slate-600">{t(message)}</p>
        </section>
    }
}

#[component]
pub(crate) fn NavigationPanel() -> impl IntoView {
    view! {
        <section class="p-5 mt-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10">
            <div class="mb-4">
                <h2 class="text-base font-bold text-slate-950">{t(WidgetsText::TabsHeading)}</h2>
                <p class="mt-1 text-sm text-slate-500">{t(WidgetsText::TabsDemoSummary)}</p>
            </div>
            <Tabs
                default_value=NavigationTab::Overview
                tabs=[
                    Tab::new(
                        NavigationTab::Overview,
                        || {
                            view! {
                                <p class="text-sm leading-6 text-slate-600">
                                    {t(WidgetsText::TabsOverview)}
                                </p>
                            }
                        },
                    ),
                    Tab::new(
                            NavigationTab::Keyboard,
                            || {
                                view! {
                                    <ul class="text-sm leading-6 list-disc list-inside text-slate-600">
                                        <li>{t(WidgetsText::KeyboardArrowKeys)}</li>
                                        <li>{t(WidgetsText::KeyboardHomeEnd)}</li>
                                        <li>{t(WidgetsText::KeyboardManualActivation)}</li>
                                        <li>{t(WidgetsText::KeyboardReorder)}</li>
                                        <li>{t(WidgetsText::KeyboardClosable)}</li>
                                    </ul>
                                }
                            },
                        )
                        .closable(true),
                    Tab::new(
                            NavigationTab::Closable,
                            || {
                                view! {
                                    <p class="text-sm leading-6 text-slate-600">
                                        {t(WidgetsText::ClosablePanel)}
                                    </p>
                                }
                            },
                        )
                        .closable(true),
                    Tab::new(
                            NavigationTab::Disabled,
                            || {
                                view! {
                                    <p class="text-sm leading-6 text-slate-600">
                                        {t(WidgetsText::DisabledPanel)}
                                    </p>
                                }
                            },
                        )
                        .disabled(true),
                ]
                reorderable=true
            />
        </section>
    }
}

#[component]
pub(crate) fn UtilityPanel() -> impl IntoView {
    let (dismiss_status, set_dismiss_status) = signal(WidgetsText::DismissInitial);
    let dismiss_props = dismissable::Props::new().on_dismiss(move |reason| {
        set_dismiss_status.set(WidgetsText::DismissReason {
            reason: format!("{reason:?}"),
        });
    });
    let error_message = t(WidgetsText::ExampleChildError);

    view! {
        <div class="grid gap-5 mt-5 lg:grid-cols-2">
            <section
                class="p-5 rounded-lg border shadow-xl lg:col-span-2 border-slate-200 bg-white/85 shadow-slate-900/10"
                aria-labelledby="variants"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="variants" class="text-base font-bold text-slate-950">
                        {t(WidgetsText::ButtonVariants)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(WidgetsText::ButtonVariantsNote)}</p>
                </div>
                <div class="flex flex-wrap gap-3">
                    <Button id="leptos-tw-default">{t(WidgetsText::DefaultButton)}</Button>
                    <Button id="leptos-tw-primary" variant=button::Variant::Primary>
                        {t(WidgetsText::PrimaryButton)}
                    </Button>
                    <Button id="leptos-tw-secondary" variant=button::Variant::Secondary>
                        {t(WidgetsText::SecondaryButton)}
                    </Button>
                    <Button id="leptos-tw-destructive" variant=button::Variant::Destructive>
                        {t(WidgetsText::DestructiveButton)}
                    </Button>
                    <Button id="leptos-tw-outline" variant=button::Variant::Outline>
                        {t(WidgetsText::OutlineButton)}
                    </Button>
                    <Button id="leptos-tw-ghost" variant=button::Variant::Ghost>
                        {t(WidgetsText::GhostButton)}
                    </Button>
                    <Button id="leptos-tw-link" variant=button::Variant::Link>
                        {t(WidgetsText::LinkButton)}
                    </Button>
                </div>
            </section>
            <section
                class="p-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10"
                aria-labelledby="sizes"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="sizes" class="text-base font-bold text-slate-950">
                        {t(WidgetsText::ButtonSizes)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(WidgetsText::ButtonSizeTokens)}</p>
                </div>
                <div class="flex flex-wrap gap-3">
                    <Button id="leptos-tw-sm" size=button::Size::Sm>
                        {t(WidgetsText::SmallButton)}
                    </Button>
                    <Button id="leptos-tw-md" size=button::Size::Md>
                        {t(WidgetsText::MediumButton)}
                    </Button>
                    <Button id="leptos-tw-lg" size=button::Size::Lg>
                        {t(WidgetsText::LargeButton)}
                    </Button>
                    <Button id="leptos-tw-icon" size=button::Size::Icon>
                        {t(WidgetsText::IconButton)}
                    </Button>
                </div>
            </section>
            <section
                class="p-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10"
                aria-labelledby="states"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="states" class="text-base font-bold text-slate-950">
                        {t(WidgetsText::ButtonStates)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(WidgetsText::ButtonStatesNote)}</p>
                </div>
                <div class="flex flex-wrap gap-3">
                    <Button id="leptos-tw-disabled" disabled=true>
                        {t(WidgetsText::DisabledButton)}
                    </Button>
                    <Button id="leptos-tw-loading" loading=true>
                        {t(WidgetsText::LoadingButton)}
                    </Button>
                </div>
            </section>
            <section
                class="p-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10"
                aria-labelledby="as-child"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="as-child" class="text-base font-bold text-slate-950">
                        {t(WidgetsText::AsChild)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(WidgetsText::AsChildNote)}</p>
                </div>
                <div class="flex flex-wrap gap-3">
                    <ButtonAsChild
                        id="leptos-tw-as-child-docs"
                        variant=button::Variant::Link
                        class="group"
                    >
                        <a href="#variants">{t(WidgetsText::DocsLinkRoot)}</a>
                    </ButtonAsChild>
                    <ButtonAsChild id="leptos-tw-as-child-primary" variant=button::Variant::Primary>
                        <a href="#variants">{t(WidgetsText::AnchorAsPrimary)}</a>
                    </ButtonAsChild>
                </div>
            </section>
            <section
                class="p-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10"
                aria-labelledby="forms"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="forms" class="text-base font-bold text-slate-950">
                        {t(WidgetsText::Forms)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(WidgetsText::FormsNote)}</p>
                </div>
                <form id="leptos-tw-example-form">
                    <div class="flex flex-wrap gap-3">
                        <Button
                            id="leptos-tw-submit"
                            r#type=button::Type::Submit
                            form="leptos-tw-example-form"
                            name="intent"
                            value="save"
                            form_action="/submit"
                            form_method=button::FormMethod::Post
                            form_enc_type=button::FormEncType::UrlEncoded
                            form_target=button::FormTarget::Self_
                            form_no_validate=true
                        >
                            {t(WidgetsText::SubmitOverride)}
                        </Button>
                        <Button id="leptos-tw-reset" r#type=button::Type::Reset>
                            {t(WidgetsText::Reset)}
                        </Button>
                    </div>
                </form>
            </section>
            <section
                class="p-5 rounded-lg border shadow-lg lg:col-span-2 border-slate-200 bg-white/85 shadow-slate-900/10"
                aria-labelledby="dismissable"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="dismissable" class="text-base font-bold text-slate-950">
                        {t(WidgetsText::DismissablePrimitive)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(WidgetsText::DismissableNote)}</p>
                </div>
                <dismissable::Region props=dismiss_props>
                    <div>
                        <h4 class="text-sm font-bold text-blue-950">
                            {t(WidgetsText::TailwindDismissableRegion)}
                        </h4>
                        <p class="mt-2 max-w-2xl text-sm leading-6 text-blue-900">
                            {t(WidgetsText::DismissableCompositionDescription)}
                        </p>
                    </div>
                </dismissable::Region>
                <p class="py-2 px-3 mt-3 text-sm font-medium text-white rounded-md shadow-sm bg-slate-950">
                    {t(dismiss_status)}
                </p>
            </section>
            <section
                class="p-5 rounded-lg border shadow-lg lg:col-span-2 border-slate-200 bg-white/85 shadow-slate-900/10"
                aria-labelledby="errors"
            >
                <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                    <h2 id="errors" class="text-base font-bold text-slate-950">
                        {t(WidgetsText::ErrorBoundary)}
                    </h2>
                    <p class="text-sm text-slate-500">{t(WidgetsText::ErrorBoundaryNote)}</p>
                </div>
                <div class="grid gap-4 md:grid-cols-2">
                    <Boundary>
                        <p class="p-4 text-sm font-medium text-emerald-900 bg-emerald-50 rounded-lg border border-emerald-200 shadow-sm">
                            {t(WidgetsText::HealthyChild)}
                        </p>
                    </Boundary>
                    <Boundary>{example_error(error_message)}</Boundary>
                </div>
            </section>
        </div>
    }
}
