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
        <main class="py-10 px-5 mx-auto max-w-6xl min-h-screen md:px-8">
            <p class="mb-2 text-xs font-extrabold tracking-wider text-blue-700 uppercase">
                "Tailwind styling"
            </p>
            <h1 class="max-w-3xl text-4xl font-extrabold leading-tight text-slate-950">
                "Leptos Button Widgets"
            </h1>
            <p class="mt-3 max-w-3xl text-base leading-7 text-slate-600">
                "A compact gallery for variant, size, loading, disabled, and form behaviors."
            </p>
            <div class="grid gap-5 mt-8 lg:grid-cols-2">
                <section
                    class="p-5 rounded-lg border shadow-xl lg:col-span-2 border-slate-200 bg-white/85 shadow-slate-900/10"
                    aria-labelledby="variants"
                >
                    <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                        <h2 id="variants" class="text-base font-bold text-slate-950">
                            "Variants"
                        </h2>
                        <p class="text-sm text-slate-500">
                            "Hover each button to inspect transitions."
                        </p>
                    </div>
                    <div class="flex flex-wrap gap-3">
                        <Button id="leptos-tw-default">"Default"</Button>
                        <Button id="leptos-tw-primary" variant=button::Variant::Primary>
                            "Primary"
                        </Button>
                        <Button id="leptos-tw-secondary" variant=button::Variant::Secondary>
                            "Secondary"
                        </Button>
                        <Button id="leptos-tw-destructive" variant=button::Variant::Destructive>
                            "Destructive"
                        </Button>
                        <Button id="leptos-tw-outline" variant=button::Variant::Outline>
                            "Outline"
                        </Button>
                        <Button id="leptos-tw-ghost" variant=button::Variant::Ghost>
                            "Ghost"
                        </Button>
                        <Button id="leptos-tw-link" variant=button::Variant::Link>
                            "Link"
                        </Button>
                    </div>
                </section>
                <section
                    class="p-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10"
                    aria-labelledby="sizes"
                >
                    <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                        <h2 id="sizes" class="text-base font-bold text-slate-950">
                            "Sizes"
                        </h2>
                        <p class="text-sm text-slate-500">"sm, md, lg, icon"</p>
                    </div>
                    <div class="flex flex-wrap gap-3">
                        <Button id="leptos-tw-sm" size=button::Size::Sm>
                            "Small"
                        </Button>
                        <Button id="leptos-tw-md" size=button::Size::Md>
                            "Medium"
                        </Button>
                        <Button id="leptos-tw-lg" size=button::Size::Lg>
                            "Large"
                        </Button>
                        <Button id="leptos-tw-icon" size=button::Size::Icon>
                            "R"
                        </Button>
                    </div>
                </section>
                <section
                    class="p-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10"
                    aria-labelledby="states"
                >
                    <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                        <h2 id="states" class="text-base font-bold text-slate-950">
                            "States"
                        </h2>
                        <p class="text-sm text-slate-500">"Disabled and busy controls."</p>
                    </div>
                    <div class="flex flex-wrap gap-3">
                        <Button id="leptos-tw-disabled" disabled=true>
                            "Disabled"
                        </Button>
                        <Button id="leptos-tw-loading" loading=true>
                            "Loading"
                        </Button>
                    </div>
                </section>
                <section
                    class="p-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10"
                    aria-labelledby="loading"
                >
                    <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                        <h2 id="loading" class="text-base font-bold text-slate-950">
                            "Loading indicator"
                        </h2>
                        <p class="text-sm text-slate-500">"Spinner part styling."</p>
                    </div>
                    <div class="flex flex-wrap gap-3">
                        <Button
                            id="leptos-tw-loading-primary"
                            variant=button::Variant::Primary
                            loading=true
                        >
                            "Saving"
                        </Button>
                        <Button
                            id="leptos-tw-loading-destructive"
                            variant=button::Variant::Destructive
                            loading=true
                        >
                            "Deleting"
                        </Button>
                    </div>
                </section>
                <section
                    class="p-5 rounded-lg border shadow-lg transition hover:shadow-xl hover:-translate-y-0.5 border-slate-200 bg-white/85 shadow-slate-900/10 hover:shadow-slate-900/15"
                    aria-labelledby="as-child"
                >
                    <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                        <h2 id="as-child" class="text-base font-bold text-slate-950">
                            "As child"
                        </h2>
                        <p class="text-sm text-slate-500">
                            "Button attrs on consumer-owned anchors."
                        </p>
                    </div>
                    <div class="flex flex-wrap gap-3">
                        <ButtonAsChild
                            id="leptos-tw-as-child-docs"
                            variant=button::Variant::Link
                            class="group"
                        >
                            <a href="#forms">"Docs link root"</a>
                        </ButtonAsChild>
                        <ButtonAsChild
                            id="leptos-tw-as-child-primary"
                            variant=button::Variant::Primary
                        >
                            <a href="#variants">"Anchor as primary"</a>
                        </ButtonAsChild>
                    </div>
                </section>
                <section
                    class="p-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10"
                    aria-labelledby="forms"
                >
                    <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                        <h2 id="forms" class="text-base font-bold text-slate-950">
                            "Forms"
                        </h2>
                        <p class="text-sm text-slate-500">"Submit/reset and form overrides."</p>
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
                                "Submit override"
                            </Button>
                            <Button id="leptos-tw-reset" r#type=button::Type::Reset>
                                "Reset"
                            </Button>
                        </div>
                    </form>
                </section>
                <section
                    class="p-5 rounded-lg border shadow-lg transition lg:col-span-2 hover:shadow-xl hover:-translate-y-0.5 border-slate-200 bg-white/85 shadow-slate-900/10 hover:shadow-slate-900/15"
                    aria-labelledby="dismissable"
                >
                    <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                        <h2 id="dismissable" class="text-base font-bold text-slate-950">
                            "Dismissable primitive"
                        </h2>
                        <p class="text-sm text-slate-500">
                            "Outside pointer/focus, Escape, and hidden dismiss buttons share one primitive."
                        </p>
                    </div>
                    <dismissable::Region props=dismiss_props dismiss_label="Dismiss example region">
                        <div>
                            <h3 class="text-sm font-bold text-blue-950">
                                "Tailwind dismissable region"
                            </h3>
                            <p class="mt-2 max-w-2xl text-sm leading-6 text-blue-900">
                                "This standalone primitive is the behavior layer future overlays will compose."
                            </p>
                        </div>
                    </dismissable::Region>
                    <p class="py-2 px-3 mt-3 text-sm font-medium text-white rounded-md shadow-sm bg-slate-950">
                        {move || dismiss_status.get()}
                    </p>
                </section>
                <section
                    class="p-5 rounded-lg border shadow-lg lg:col-span-2 border-slate-200 bg-white/85 shadow-slate-900/10"
                    aria-labelledby="errors"
                >
                    <div class="flex flex-wrap gap-3 justify-between items-center mb-4">
                        <h2 id="errors" class="text-base font-bold text-slate-950">
                            "Error boundary"
                        </h2>
                        <p class="text-sm text-slate-500">"Healthy and captured child output."</p>
                    </div>
                    <div class="grid gap-4 md:grid-cols-2">
                        <Boundary>
                            <p class="p-4 text-sm font-medium text-emerald-900 bg-emerald-50 rounded-lg border border-emerald-200 shadow-sm">
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
