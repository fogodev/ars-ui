use std::fmt::{self, Display};

use ars_leptos::utility::{
    button::{self, Button, ButtonAsChild},
    dismissable,
    error_boundary::Boundary,
};
use leptos::{mount::mount_to_body, prelude::*};

#[derive(Debug)]
struct ExampleError(&'static str);

impl Display for ExampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for ExampleError {}

const fn example_error() -> Result<&'static str, ExampleError> {
    Err(ExampleError("Example child failed while rendering."))
}

fn main() {
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let (dismiss_status, set_dismiss_status) = signal(String::from(
        "Click outside the region, press Escape, or tab to a hidden dismiss button.",
    ));
    let dismiss_props = dismissable::Props::new().on_dismiss(move |reason| {
        set_dismiss_status.set(format!("Last dismiss reason: {reason:?}"));
    });

    view! {
        <main class="widgets-page">
            <p class="page-kicker">"CSS styling"</p>
            <h1>"Leptos Button Widgets"</h1>
            <p class="page-summary">
                "A compact gallery for variant, size, loading, disabled, and form behaviors."
            </p>
            <div class="gallery-grid">
                <section class="showcase-panel wide" aria-labelledby="variants">
                    <div class="panel-heading">
                        <h2 id="variants">"Variants"</h2>
                        <p class="panel-note">"Hover each button to inspect transitions."</p>
                    </div>
                    <div class="button-row">
                        <Button id="leptos-css-default">"Default"</Button>
                        <Button id="leptos-css-primary" variant=button::Variant::Primary>
                            "Primary"
                        </Button>
                        <Button id="leptos-css-secondary" variant=button::Variant::Secondary>
                            "Secondary"
                        </Button>
                        <Button id="leptos-css-destructive" variant=button::Variant::Destructive>
                            "Destructive"
                        </Button>
                        <Button id="leptos-css-outline" variant=button::Variant::Outline>
                            "Outline"
                        </Button>
                        <Button id="leptos-css-ghost" variant=button::Variant::Ghost>
                            "Ghost"
                        </Button>
                        <Button id="leptos-css-link" variant=button::Variant::Link>
                            "Link"
                        </Button>
                    </div>
                </section>
                <section class="showcase-panel" aria-labelledby="sizes">
                    <div class="panel-heading">
                        <h2 id="sizes">"Sizes"</h2>
                        <p class="panel-note">"Enum-driven sizing."</p>
                    </div>
                    <div class="button-row">
                        <Button id="leptos-css-sm" size=button::Size::Sm>
                            "Small"
                        </Button>
                        <Button id="leptos-css-md" size=button::Size::Md>
                            "Medium"
                        </Button>
                        <Button id="leptos-css-lg" size=button::Size::Lg>
                            "Large"
                        </Button>
                        <Button id="leptos-css-icon" size=button::Size::Icon>
                            "R"
                        </Button>
                    </div>
                </section>
                <section class="showcase-panel" aria-labelledby="states">
                    <div class="panel-heading">
                        <h2 id="states">"States"</h2>
                        <p class="panel-note">"Disabled and busy controls."</p>
                    </div>
                    <div class="button-row">
                        <Button id="leptos-css-disabled" disabled=true>
                            "Disabled"
                        </Button>
                        <Button id="leptos-css-loading" loading=true>
                            "Loading"
                        </Button>
                    </div>
                </section>
                <section class="showcase-panel" aria-labelledby="loading">
                    <div class="panel-heading">
                        <h2 id="loading">"Loading indicator"</h2>
                        <p class="panel-note">"Spinner part styling."</p>
                    </div>
                    <div class="button-row">
                        <Button
                            id="leptos-css-loading-primary"
                            variant=button::Variant::Primary
                            loading=true
                        >
                            "Saving"
                        </Button>
                        <Button
                            id="leptos-css-loading-destructive"
                            variant=button::Variant::Destructive
                            loading=true
                        >
                            "Deleting"
                        </Button>
                    </div>
                </section>
                <section class="showcase-panel" aria-labelledby="as-child">
                    <div class="panel-heading">
                        <h2 id="as-child">"As child"</h2>
                        <p class="panel-note">"Button attrs on consumer-owned anchors."</p>
                    </div>
                    <div class="button-row">
                        <ButtonAsChild id="leptos-css-as-child-docs" variant=button::Variant::Link>
                            <a href="#forms">"Docs link root"</a>
                        </ButtonAsChild>
                        <ButtonAsChild
                            id="leptos-css-as-child-primary"
                            variant=button::Variant::Primary
                        >
                            <a href="#variants">"Anchor as primary"</a>
                        </ButtonAsChild>
                    </div>
                </section>
                <section class="showcase-panel" aria-labelledby="forms">
                    <div class="panel-heading">
                        <h2 id="forms">"Forms"</h2>
                        <p class="panel-note">"Submit/reset and form overrides."</p>
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
                                "Submit override"
                            </Button>
                            <Button id="leptos-css-reset" r#type=button::Type::Reset>
                                "Reset"
                            </Button>
                        </div>
                    </form>
                </section>
                <section class="showcase-panel wide" aria-labelledby="dismissable">
                    <div class="panel-heading">
                        <h2 id="dismissable">"Dismissable primitive"</h2>
                        <p class="panel-note">
                            "Outside pointer/focus, Escape, and hidden dismiss buttons share one primitive."
                        </p>
                    </div>
                    <dismissable::Region props=dismiss_props dismiss_label="Dismiss example region">
                        <div class="dismissable-card">
                            <h3>"CSS dismissable region"</h3>
                            <p>
                                "This standalone primitive is the behavior layer future overlays will compose."
                            </p>
                        </div>
                    </dismissable::Region>
                    <p class="dismissable-status">{move || dismiss_status.get()}</p>
                </section>
                <section class="showcase-panel wide" aria-labelledby="errors">
                    <div class="panel-heading">
                        <h2 id="errors">"Error boundary"</h2>
                        <p class="panel-note">"Healthy and captured child output."</p>
                    </div>
                    <div class="error-grid">
                        <Boundary>
                            <p class="healthy-boundary">
                                "Healthy child rendered inside the boundary."
                            </p>
                        </Boundary>
                        <Boundary>{example_error()}</Boundary>
                    </div>
                </section>
            </div>
        </main>
    }
}
