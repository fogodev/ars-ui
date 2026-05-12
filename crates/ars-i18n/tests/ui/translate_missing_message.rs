use ars_i18n::Translate;

#[derive(Translate)]
#[translate(fallback = "en")]
enum BadText {
    Hello,
}

fn main() {}
