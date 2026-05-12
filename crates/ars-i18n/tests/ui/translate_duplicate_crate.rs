use ars_i18n::Translate;

#[derive(Translate)]
#[translate(fallback = "en", crate = ars_i18n, crate = ars_i18n)]
enum BadText {
    #[translate(en = "Hello")]
    Hello,
}

fn main() {}
