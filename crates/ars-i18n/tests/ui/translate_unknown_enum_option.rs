use ars_i18n::Translate;

#[derive(Translate)]
#[translate(fallback = "en", domain = "widgets")]
enum BadText {
    #[translate(en = "Hello")]
    Hello,
}

fn main() {}
