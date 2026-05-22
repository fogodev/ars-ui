use ars_leptos::prelude::{Translate, t};
use leptos::prelude::*;

#[derive(Clone, Debug, Translate)]
#[translate(fallback = "en-US")]
pub(crate) enum LayoutText {
    #[translate(
        en_US = "Layout components - splitter, scroll-area, carousel, portal, toolbar, etc. Coming soon.",
        pt_BR = "Componentes de layout - splitter, scroll-area, carousel, portal, toolbar etc. Em breve."
    )]
    LayoutPanel,
}

#[component]
pub(crate) fn LayoutPanel() -> impl IntoView {
    view! {
        <section class="showcase-panel wide empty-category">
            <p>{t(LayoutText::LayoutPanel)}</p>
        </section>
    }
}
