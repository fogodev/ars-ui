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
        <section class="empty-category">
            <p>{t(OverlayText::OverlayPanel)}</p>
        </section>
    }
}
