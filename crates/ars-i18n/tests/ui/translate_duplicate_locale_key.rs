use ars_i18n::Translate;

#[derive(Translate)]
#[translate(fallback = "en")]
enum BadText {
    #[translate(en = "Hello")]
    #[translate(locale = "en", text = "Hi")]
    Hello,
}

fn main() {}
