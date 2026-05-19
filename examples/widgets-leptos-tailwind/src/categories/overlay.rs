use ars_leptos::prelude::{t, Translate};
use leptos::prelude::*;

#[derive(Clone, Debug, Translate)]
#[translate(fallback = "en-US")]
pub(crate) enum OverlayText {
    #[translate(
        en_US = "Overlay components - dialog, popover, tooltip, toast, presence, etc. Coming soon.",
        pt_BR = "Componentes de sobreposição - dialog, popover, tooltip, toast, presence etc. Em breve."
    )]
    OverlayPanel,
}

#[component]
pub(crate) fn OverlayPanel() -> impl IntoView {
    view! {
        <section class="p-5 mt-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10">
            <p class="text-sm text-slate-600">{t(OverlayText::OverlayPanel)}</p>
        </section>
    }
}
