use ars_dioxus::prelude::{Translate, t};
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
        section { class: "empty-category",
            p { {t(DataDisplayText::DataDisplayPanel)} }
        }
    }
}
