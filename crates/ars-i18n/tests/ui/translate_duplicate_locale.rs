use ars_i18n::Translate;

#[derive(Translate)]
#[translate(fallback = "en")]
enum Text {
    #[translate(pt_BR = "Perfil")]
    #[translate(locale = "pt-BR", text = "Outro")]
    Profile,
}

fn main() {}
