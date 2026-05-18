use std::fmt::{self, Display};

use ars_leptos::{
    navigation::tabs::{Tab, Tabs},
    prelude::{Orientation, t},
    utility::{
        button::{self, Button, ButtonAsChild},
        dismissable,
        error_boundary::Boundary,
        separator::{Separator, SeparatorAsChild},
        visually_hidden::{VisuallyHidden, VisuallyHiddenAsChild},
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
        <section class="showcase-panel wide empty-category">
            <p>{t(message)}</p>
        </section>
    }
}

#[component]
pub(crate) fn NavigationPanel() -> impl IntoView {
    view! {
        <section class="showcase-panel wide">
            <div class="panel-heading">
                <h2>{t(WidgetsText::TabsHeading)}</h2>
                <p class="panel-note">{t(WidgetsText::TabsDemoSummary)}</p>
            </div>
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
        <div class="gallery-grid">
            <section class="showcase-panel wide" aria-labelledby="variants">
                <div class="panel-heading">
                    <h2 id="variants">{t(WidgetsText::ButtonVariants)}</h2>
                    <p class="panel-note">{t(WidgetsText::ButtonVariantsNote)}</p>
                </div>
                <div class="button-row">
                    <Button id="leptos-css-default">{t(WidgetsText::DefaultButton)}</Button>
                    <Button id="leptos-css-primary" variant=button::Variant::Primary>
                        {t(WidgetsText::PrimaryButton)}
                    </Button>
                    <Button id="leptos-css-secondary" variant=button::Variant::Secondary>
                        {t(WidgetsText::SecondaryButton)}
                    </Button>
                    <Button id="leptos-css-destructive" variant=button::Variant::Destructive>
                        {t(WidgetsText::DestructiveButton)}
                    </Button>
                    <Button id="leptos-css-outline" variant=button::Variant::Outline>
                        {t(WidgetsText::OutlineButton)}
                    </Button>
                    <Button id="leptos-css-ghost" variant=button::Variant::Ghost>
                        {t(WidgetsText::GhostButton)}
                    </Button>
                    <Button id="leptos-css-link" variant=button::Variant::Link>
                        {t(WidgetsText::LinkButton)}
                    </Button>
                </div>
            </section>
            <section class="showcase-panel" aria-labelledby="sizes">
                <div class="panel-heading">
                    <h2 id="sizes">{t(WidgetsText::ButtonSizes)}</h2>
                    <p class="panel-note">{t(WidgetsText::ButtonSizesNote)}</p>
                </div>
                <div class="button-row">
                    <Button id="leptos-css-sm" size=button::Size::Sm>
                        {t(WidgetsText::SmallButton)}
                    </Button>
                    <Button id="leptos-css-md" size=button::Size::Md>
                        {t(WidgetsText::MediumButton)}
                    </Button>
                    <Button id="leptos-css-lg" size=button::Size::Lg>
                        {t(WidgetsText::LargeButton)}
                    </Button>
                    <Button id="leptos-css-icon" size=button::Size::Icon>
                        {t(WidgetsText::IconButton)}
                    </Button>
                </div>
            </section>
            <section class="showcase-panel" aria-labelledby="states">
                <div class="panel-heading">
                    <h2 id="states">{t(WidgetsText::ButtonStates)}</h2>
                    <p class="panel-note">{t(WidgetsText::ButtonStatesNote)}</p>
                </div>
                <div class="button-row">
                    <Button id="leptos-css-disabled" disabled=true>
                        {t(WidgetsText::DisabledButton)}
                    </Button>
                    <Button id="leptos-css-loading" loading=true>
                        {t(WidgetsText::LoadingButton)}
                    </Button>
                </div>
            </section>
            <section class="showcase-panel" aria-labelledby="as-child">
                <div class="panel-heading">
                    <h2 id="as-child">{t(WidgetsText::AsChild)}</h2>
                    <p class="panel-note">{t(WidgetsText::AsChildNote)}</p>
                </div>
                <div class="button-row">
                    <ButtonAsChild id="leptos-css-as-child-docs" variant=button::Variant::Link>
                        <a href="#variants">{t(WidgetsText::DocsLinkRoot)}</a>
                    </ButtonAsChild>
                    <ButtonAsChild
                        id="leptos-css-as-child-primary"
                        variant=button::Variant::Primary
                    >
                        <a href="#variants">{t(WidgetsText::AnchorAsPrimary)}</a>
                    </ButtonAsChild>
                </div>
            </section>
            <section class="showcase-panel" aria-labelledby="forms">
                <div class="panel-heading">
                    <h2 id="forms">{t(WidgetsText::Forms)}</h2>
                    <p class="panel-note">{t(WidgetsText::FormsNote)}</p>
                </div>
                <form id="leptos-css-example-form">
                    <div class="button-row">
                        <Button
                            id="leptos-css-submit"
                            r#type=button::Type::Submit
                            form="leptos-css-example-form"
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
                        <Button id="leptos-css-reset" r#type=button::Type::Reset>
                            {t(WidgetsText::Reset)}
                        </Button>
                    </div>
                </form>
            </section>
            <section class="showcase-panel" aria-labelledby="visually-hidden">
                <div class="panel-heading">
                    <h2 id="visually-hidden">{t(WidgetsText::VisuallyHidden)}</h2>
                    <p class="panel-note">{t(WidgetsText::VisuallyHiddenDescription)}</p>
                </div>
                <p>
                    <VisuallyHidden id="leptos-css-visually-hidden-label">
                        {t(WidgetsText::VisuallyHiddenLabel)}
                    </VisuallyHidden>
                    {t(WidgetsText::VisuallyHiddenDescription)}
                </p>
                <p>
                    <VisuallyHidden id="leptos-css-focusable-skip" is_focusable=true>
                        <a href="#variants">{t(WidgetsText::FocusableSkipLink)}</a>
                    </VisuallyHidden>
                </p>
                <VisuallyHiddenAsChild id="leptos-css-visually-hidden-as-child">
                    <span>{t(WidgetsText::AsChildHiddenLabel)}</span>
                </VisuallyHiddenAsChild>
            </section>
            <section class="showcase-panel" aria-labelledby="separator">
                <div class="panel-heading">
                    <h2 id="separator">{t(WidgetsText::SeparatorPrimitive)}</h2>
                    <p class="panel-note">{t(WidgetsText::SeparatorDescription)}</p>
                </div>
                <Separator id="leptos-css-separator-horizontal" />
                <div class="separator-demo-row">
                    <span>{t(WidgetsText::HorizontalSeparator)}</span>
                    <Separator
                        id="leptos-css-separator-vertical"
                        orientation=Orientation::Vertical
                    />
                    <span>{t(WidgetsText::VerticalSeparator)}</span>
                </div>
                <Separator id="leptos-css-separator-decorative" decorative=true />
                <p class="panel-note">{t(WidgetsText::DecorativeSeparator)}</p>
                <SeparatorAsChild
                    id="leptos-css-separator-as-child"
                    orientation=Orientation::Vertical
                >
                    <div class="separator-as-child"></div>
                </SeparatorAsChild>
                <p class="panel-note">{t(WidgetsText::AsChildSeparator)}</p>
            </section>
            <section class="showcase-panel wide" aria-labelledby="dismissable">
                <div class="panel-heading">
                    <h2 id="dismissable">{t(WidgetsText::DismissablePrimitive)}</h2>
                    <p class="panel-note">{t(WidgetsText::DismissableNote)}</p>
                </div>
                <dismissable::Region props=dismiss_props>
                    <div class="dismissable-card">
                        <h3>{t(WidgetsText::CssDismissableRegion)}</h3>
                        <p>{t(WidgetsText::DismissableCompositionDescription)}</p>
                    </div>
                </dismissable::Region>
                <p class="dismissable-status">{t(dismiss_status)}</p>
            </section>
            <section class="showcase-panel wide" aria-labelledby="errors">
                <div class="panel-heading">
                    <h2 id="errors">{t(WidgetsText::ErrorBoundary)}</h2>
                    <p class="panel-note">{t(WidgetsText::ErrorBoundaryNote)}</p>
                </div>
                <div class="error-grid">
                    <Boundary>
                        <p class="healthy-boundary">{t(WidgetsText::HealthyChild)}</p>
                    </Boundary>
                    <Boundary>{example_error(error_message)}</Boundary>
                </div>
            </section>
        </div>
    }
}
