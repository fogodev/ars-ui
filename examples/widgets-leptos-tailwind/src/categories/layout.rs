use ars_leptos::prelude::{t, Translate};
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
        <section class="p-5 mt-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10">
            <p class="text-sm text-slate-600">{t(LayoutText::LayoutPanel)}</p>
        </section>
    }
}
