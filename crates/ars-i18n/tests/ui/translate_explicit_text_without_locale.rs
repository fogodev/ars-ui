use ars_i18n::Translate;

#[derive(Translate)]
#[translate(fallback = "en")]
enum BadText {
    #[translate(text = "Hello")]
    Hello,
}

fn main() {}
