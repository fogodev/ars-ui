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
        <section class="empty-category">
            <p>{t(message)}</p>
        </section>
    }
}

#[component]
pub(crate) fn NavigationPanel() -> impl IntoView {
    view! {
        <section>
            <h3>{t(WidgetsText::TabsHeading)}</h3>
            <p>{t(WidgetsText::TabsDemoSummary)}</p>
            <Tabs
                default_value=NavigationTab::Overview
                tabs=[
                    Tab::new(
                        NavigationTab::Overview,
                        || {
                            view! { <p>{t(WidgetsText::TabsOverview)}</p> }
                        },
                    ),
                    Tab::new(
                            NavigationTab::Keyboard,
                            || {
                                view! {
                                    <ul>
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
                                view! { <p>{t(WidgetsText::ClosablePanel)}</p> }
                            },
                        )
                        .closable(true),
                    Tab::new(
                            NavigationTab::Disabled,
                            || {
                                view! { <p>{t(WidgetsText::DisabledPanel)}</p> }
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
        <div class="utility-grid">
            <section aria-labelledby="variants">
                <h3 id="variants">{t(WidgetsText::ButtonVariants)}</h3>
                <div class="button-row">
                    <Button id="leptos-default">{t(WidgetsText::DefaultButton)}</Button>
                    <Button id="leptos-primary" variant=button::Variant::Primary>
                        {t(WidgetsText::PrimaryButton)}
                    </Button>
                    <Button id="leptos-secondary" variant=button::Variant::Secondary>
                        {t(WidgetsText::SecondaryButton)}
                    </Button>
                    <Button id="leptos-destructive" variant=button::Variant::Destructive>
                        {t(WidgetsText::DestructiveButton)}
                    </Button>
                    <Button id="leptos-outline" variant=button::Variant::Outline>
                        {t(WidgetsText::OutlineButton)}
                    </Button>
                    <Button id="leptos-ghost" variant=button::Variant::Ghost>
                        {t(WidgetsText::GhostButton)}
                    </Button>
                    <Button id="leptos-link" variant=button::Variant::Link>
                        {t(WidgetsText::LinkButton)}
                    </Button>
                </div>
            </section>
            <section aria-labelledby="sizes">
                <h3 id="sizes">{t(WidgetsText::ButtonSizes)}</h3>
                <div class="button-row">
                    <Button id="leptos-sm" size=button::Size::Sm>
                        {t(WidgetsText::SmallButton)}
                    </Button>
                    <Button id="leptos-md" size=button::Size::Md>
                        {t(WidgetsText::MediumButton)}
                    </Button>
                    <Button id="leptos-lg" size=button::Size::Lg>
                        {t(WidgetsText::LargeButton)}
                    </Button>
                    <Button id="leptos-icon" size=button::Size::Icon>
                        {t(WidgetsText::IconButton)}
                    </Button>
                </div>
            </section>
            <section aria-labelledby="states">
                <h3 id="states">{t(WidgetsText::ButtonStates)}</h3>
                <div class="button-row">
                    <Button id="leptos-disabled" disabled=true>
                        {t(WidgetsText::DisabledButton)}
                    </Button>
                    <Button id="leptos-loading" loading=true>
                        {t(WidgetsText::LoadingButton)}
                    </Button>
                </div>
            </section>
            <section aria-labelledby="as-child">
                <h3 id="as-child">{t(WidgetsText::AsChild)}</h3>
                <div class="button-row">
                    <ButtonAsChild id="leptos-as-child-docs" variant=button::Variant::Link>
                        <a href="#variants">{t(WidgetsText::DocsLinkRoot)}</a>
                    </ButtonAsChild>
                    <ButtonAsChild id="leptos-as-child-primary" variant=button::Variant::Primary>
                        <a href="#variants">{t(WidgetsText::AnchorAsPrimary)}</a>
                    </ButtonAsChild>
                </div>
            </section>
            <section aria-labelledby="forms">
                <h3 id="forms">{t(WidgetsText::Forms)}</h3>
                <form id="leptos-example-form">
                    <div class="button-row">
                        <Button
                            id="leptos-submit"
                            r#type=button::Type::Submit
                            form="leptos-example-form"
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
                        <Button id="leptos-reset" r#type=button::Type::Reset>
                            {t(WidgetsText::Reset)}
                        </Button>
                    </div>
                </form>
            </section>
            <section aria-labelledby="dismissable">
                <h3 id="dismissable">{t(WidgetsText::DismissablePrimitive)}</h3>
                <dismissable::Region props=dismiss_props>
                    <div>
                        <h4>{t(WidgetsText::PlainDismissableRegion)}</h4>
                        <p>{t(WidgetsText::DismissableDescription)}</p>
                    </div>
                </dismissable::Region>
                <p>{t(dismiss_status)}</p>
            </section>
            <section aria-labelledby="errors">
                <h3 id="errors">{t(WidgetsText::ErrorBoundary)}</h3>
                <div class="button-row">
                    <Boundary>
                        <p>{t(WidgetsText::HealthyChild)}</p>
                    </Boundary>
                    <Boundary>{example_error(error_message)}</Boundary>
                </div>
            </section>
        </div>
    }
}
