use ars_dioxus::prelude::{t, Translate};
use dioxus::prelude::*;

#[derive(Clone, Debug, Translate, PartialEq)]
#[translate(fallback = "en-US")]
pub(crate) enum DataDisplayText {
    #[translate(
        en_US = "Data display components - table, avatar, progress, meter, badge, etc. Coming soon.",
        pt_BR = "Componentes de exibição de dados - table, avatar, progress, meter, badge etc. Em breve."
    )]
    DataDisplayPanel,
}

#[component]
pub(crate) fn DataDisplayPanel() -> Element {
    rsx! {
        section { class: "mt-5 rounded-lg border border-slate-200 bg-white/85 p-5 shadow-lg shadow-slate-900/10",
            p { class: "text-sm text-slate-600", {t(DataDisplayText::DataDisplayPanel)} }
        }
    }
}
