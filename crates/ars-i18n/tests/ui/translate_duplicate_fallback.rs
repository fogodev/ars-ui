use ars_i18n::Translate;

#[derive(Translate)]
#[translate(fallback = "en", fallback = "pt")]
enum BadText {
    #[translate(en = "Hello")]
    Hello,
}

fn main() {}
