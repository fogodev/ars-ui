use ars_leptos::prelude::{Translate, t};
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
        <section class="p-5 mt-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10">
            <p class="text-sm text-slate-600">{t(SpecializedText::SpecializedPanel)}</p>
        </section>
    }
}
