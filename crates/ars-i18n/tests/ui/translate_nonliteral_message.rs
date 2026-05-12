use ars_i18n::Translate;

const HELLO: &str = "Hello";

#[derive(Translate)]
#[translate(fallback = "en")]
enum BadText {
    #[translate(en = HELLO)]
    Hello,
}

fn main() {}
