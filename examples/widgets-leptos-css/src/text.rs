use ars_leptos::prelude::{TabKey, Translate};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey, Translate)]
#[tab_key(ordinal)]
#[translate(fallback = "en-US")]
pub(crate) enum CategoryTab {
    #[translate(en_US = "Input", pt_BR = "Entrada")]
    Input,

    #[translate(en_US = "Selection", pt_BR = "Seleção")]
    Selection,

    #[translate(en_US = "Overlay", pt_BR = "Sobreposição")]
    Overlay,

    #[translate(en_US = "Navigation", pt_BR = "Navegação")]
    Navigation,

    #[translate(en_US = "Date & time", pt_BR = "Data e hora")]
    DateTime,

    #[translate(en_US = "Data display", pt_BR = "Exibição de dados")]
    DataDisplay,

    #[translate(en_US = "Layout", pt_BR = "Layout")]
    Layout,

    #[translate(en_US = "Specialized", pt_BR = "Especializados")]
    Specialized,

    #[translate(en_US = "Utility", pt_BR = "Utilitários")]
    Utility,
}

#[derive(Clone, Debug, Translate)]
#[translate(fallback = "en-US")]
pub(crate) enum WidgetsText {
    #[translate(en_US = "CSS styling", pt_BR = "Estilo CSS")]
    CssStyling,

    #[translate(
        en_US = "Leptos Component Gallery",
        pt_BR = "Galeria de Componentes Leptos"
    )]
    LeptosTitle,

    #[translate(
        en_US = "One tab per spec category. Browse implemented components, or peek at categories that are still empty.",
        pt_BR = "Uma aba por categoria da especificação. Navegue pelos componentes implementados ou veja categorias que ainda estão vazias."
    )]
    PageSummary,

    #[translate(en_US = "Locale", pt_BR = "Idioma")]
    LocaleLabel,

    #[translate(en_US = "en-US", pt_BR = "en-US")]
    LocaleEnglish,

    #[translate(en_US = "pt-BR", pt_BR = "pt-BR")]
    LocalePortuguese,
}
