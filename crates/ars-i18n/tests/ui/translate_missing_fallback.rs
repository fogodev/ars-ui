use ars_i18n::Translate;

#[derive(Translate)]
#[translate(fallback = "en")]
enum Text {
    #[translate(pt = "Perfil")]
    Profile,
}

fn main() {}
