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
            <h1>"Leptos Button Widgets"</h1>
            <p>"A compact gallery for variant, size, loading, disabled, and form behaviors."</p>
            <section aria-labelledby="variants">
                <h2 id="variants">"Variants"</h2>
                <p>"Hover each button to inspect transitions."</p>
                <div class="button-row">
                    <Button id="leptos-default">"Default"</Button>
                    <Button id="leptos-primary" variant=button::Variant::Primary>
                        "Primary"
                    </Button>
                    <Button id="leptos-secondary" variant=button::Variant::Secondary>
                        "Secondary"
                    </Button>
                    <Button id="leptos-destructive" variant=button::Variant::Destructive>
                        "Destructive"
                    </Button>
                    <Button id="leptos-outline" variant=button::Variant::Outline>
                        "Outline"
                    </Button>
                    <Button id="leptos-ghost" variant=button::Variant::Ghost>
                        "Ghost"
                    </Button>
                    <Button id="leptos-link" variant=button::Variant::Link>
                        "Link"
                    </Button>
                </div>
            </section>
            <section aria-labelledby="sizes">
                <h2 id="sizes">"Sizes"</h2>
                <p>"Enum-driven sizing."</p>
                <div class="button-row">
                    <Button id="leptos-sm" size=button::Size::Sm>
                        "Small"
                    </Button>
                    <Button id="leptos-md" size=button::Size::Md>
                        "Medium"
                    </Button>
                    <Button id="leptos-lg" size=button::Size::Lg>
                        "Large"
                    </Button>
                    <Button id="leptos-icon" size=button::Size::Icon>
                        "R"
                    </Button>
                </div>
            </section>
            <section aria-labelledby="states">
                <h2 id="states">"States"</h2>
                <p>"Disabled and busy controls."</p>
                <div class="button-row">
                    <Button id="leptos-disabled" disabled=true>
                        "Disabled"
                    </Button>
                    <Button id="leptos-loading" loading=true>
                        "Loading"
                    </Button>
                </div>
            </section>
            <section aria-labelledby="loading">
                <h2 id="loading">"Loading indicator"</h2>
                <p>"Spinner part styling."</p>
                <div class="button-row">
                    <Button
                        id="leptos-loading-primary"
                        variant=button::Variant::Primary
                        loading=true
                    >
                        "Saving"
                    </Button>
                    <Button
                        id="leptos-loading-destructive"
                        variant=button::Variant::Destructive
                        loading=true
                    >
                        "Deleting"
                    </Button>
                </div>
            </section>
            <section aria-labelledby="as-child">
                <h2 id="as-child">"As child"</h2>
                <p>"Button attrs on consumer-owned anchors."</p>
                <div class="button-row">
                    <ButtonAsChild id="leptos-as-child-docs" variant=button::Variant::Link>
                        <a href="#forms">"Docs link root"</a>
                    </ButtonAsChild>
                    <ButtonAsChild id="leptos-as-child-primary" variant=button::Variant::Primary>
                        <a href="#variants">"Anchor as primary"</a>
                    </ButtonAsChild>
                </div>
            </section>
            <section aria-labelledby="forms">
                <h2 id="forms">"Forms"</h2>
                <p>"Submit/reset and form overrides."</p>
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
                            "Submit override"
                        </Button>
                        <Button id="leptos-reset" r#type=button::Type::Reset>
                            "Reset"
                        </Button>
                    </div>
                </form>
            </section>
            <section aria-labelledby="dismissable">
                <h2 id="dismissable">"Dismissable primitive"</h2>
                <p>
                    "Outside pointer/focus, Escape, and hidden dismiss buttons share one primitive."
                </p>
                <dismissable::Region props=dismiss_props dismiss_label="Dismiss example region">
                    <div>
                        <h3>"Plain dismissable region"</h3>
                        <p>
                            "The primitive owns outside pointer, outside focus, Escape, and paired dismiss-button behavior."
                        </p>
                    </div>
                </dismissable::Region>
                <p>{move || dismiss_status.get()}</p>
            </section>
            <section aria-labelledby="errors">
                <h2 id="errors">"Error boundary"</h2>
                <p>"Healthy and captured child output."</p>
                <div class="button-row">
                    <Boundary>
                        <p>"Healthy child rendered inside the boundary."</p>
                    </Boundary>
                    <Boundary>{example_error()}</Boundary>
                </div>
            </section>
        </main>
    }
}
