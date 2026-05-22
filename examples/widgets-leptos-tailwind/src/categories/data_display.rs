use ars_leptos::prelude::{Translate, t};
use leptos::prelude::*;

#[derive(Clone, Debug, Translate)]
#[translate(fallback = "en-US")]
pub(crate) enum DataDisplayText {
    #[translate(
        en_US = "Data display components - table, avatar, progress, meter, badge, etc. Coming soon.",
        pt_BR = "Componentes de exibição de dados - table, avatar, progress, meter, badge etc. Em breve."
    )]
    DataDisplayPanel,
}

#[component]
pub(crate) fn DataDisplayPanel() -> impl IntoView {
    view! {
        <section class="p-5 mt-5 rounded-lg border shadow-lg border-slate-200 bg-white/85 shadow-slate-900/10">
            <p class="text-sm text-slate-600">{t(DataDisplayText::DataDisplayPanel)}</p>
        </section>
    }
}
