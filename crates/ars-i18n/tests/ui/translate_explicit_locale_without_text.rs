use ars_i18n::Translate;

#[derive(Translate)]
#[translate(fallback = "en")]
enum BadText {
    #[translate(locale = "en")]
    Hello,
}

fn main() {}
