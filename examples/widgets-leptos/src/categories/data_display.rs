use ars_leptos::prelude::{t, Translate};
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
        <section class="empty-category">
            <p>{t(DataDisplayText::DataDisplayPanel)}</p>
        </section>
    }
}
