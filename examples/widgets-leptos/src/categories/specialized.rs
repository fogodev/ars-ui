use ars_leptos::prelude::{t, Translate};
use leptos::prelude::*;

#[derive(Clone, Debug, Translate)]
#[translate(fallback = "en-US")]
pub(crate) enum SpecializedText {
    #[translate(
        en_US = "Specialized components - color-picker, file-upload, signature-pad, qr-code, etc. Coming soon.",
        pt_BR = "Componentes especializados - color-picker, file-upload, signature-pad, qr-code etc. Em breve."
    )]
    SpecializedPanel,
}

#[component]
pub(crate) fn SpecializedPanel() -> impl IntoView {
    view! {
        <section class="empty-category">
            <p>{t(SpecializedText::SpecializedPanel)}</p>
        </section>
    }
}
